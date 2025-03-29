use limine::{
    memory_map::{Entry, EntryType},
    request::{HhdmRequest, MemoryMapRequest},
};
use x86_64::{
    PhysAddr, VirtAddr,
    registers::control::Cr3,
    structures::paging::{
        FrameAllocator, OffsetPageTable, PageTable, PhysFrame, Size4KiB, page_table::FrameError,
    },
};

use crate::info;

pub mod allocator;

#[unsafe(link_section = ".requests")]
static MEMMAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[unsafe(link_section = ".requests")]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

pub struct BootInfoFrameAllocator {
    memory_map: &'static [&'static Entry],
    next: usize,
}

impl BootInfoFrameAllocator {
    pub unsafe fn init(memory_map: &'static [&'static Entry]) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.entry_type == EntryType::USABLE);
        let addr_ranges = usable_regions.map(|r| r.base..r.length);
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

pub fn init() {
    info!("requesting hhdm and memmap");
    let hhdm_offset = HHDM_REQUEST.get_response().unwrap().offset();
    let entries = MEMMAP_REQUEST.get_response().unwrap().entries();

    info!("mapping physical to virtual memory");
    let physical_memory_offset = VirtAddr::new(hhdm_offset);

    let level_4_table = unsafe { active_level_4_table(physical_memory_offset) };
    let mut mapper = unsafe { OffsetPageTable::new(level_4_table, physical_memory_offset) };
    info!("initializing frame allocator");
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(entries) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");
}

unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    unsafe { &mut *page_table_ptr }
}

pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [
        addr.p4_index(),
        addr.p3_index(),
        addr.p2_index(),
        addr.p1_index(),
    ];
    let mut frame = level_4_table_frame;

    for &index in &table_indexes {
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };

        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }

    Some(frame.start_address() + u64::from(addr.page_offset()))
}
