use limine::request::{HhdmRequest, MemoryMapRequest};
use talc::*;

use crate::{debug, info};

pub mod vmm;

#[unsafe(link_section = ".requests")]
static MEMMAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[unsafe(link_section = ".requests")]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

pub static EARLY_MEMORY: [u8; 2048] = [0u8; 2048];

#[global_allocator]
pub static ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> = Talc::new(unsafe {
    ClaimOnOom::new(Span::from_array(
        core::ptr::addr_of!(EARLY_MEMORY).cast_mut(),
    ))
})
.lock();

pub fn init() {
    info!("setting up memory");
    {
        debug!("requesting hhdm and memmap");
        let hhdm_offset = get_hhdm_offset();
        let mem_map = get_mem_map();

        let mut allocator = ALLOCATOR.lock();

        for entry in mem_map {
            if entry.entry_type == limine::memory_map::EntryType::USABLE {
                unsafe {
                    debug!(
                        "claiming 0x{:X}-0x{:X}",
                        entry.base,
                        entry.base + hhdm_offset
                    );
                    allocator.claim(Span::from_base_size(
                        (entry.base + hhdm_offset) as *mut u8,
                        entry.length as usize,
                    ))
                };
            }
        }
    }
    vmm::init();
}

pub fn get_hhdm_offset() -> u64 {
    HHDM_REQUEST.get_response().unwrap().offset()
}
pub fn get_mem_map() -> &'static [&'static limine::memory_map::Entry] {
    MEMMAP_REQUEST.get_response().unwrap().entries()
}
