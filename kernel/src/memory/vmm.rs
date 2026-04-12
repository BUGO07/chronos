/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::sync::Arc;
use core::{alloc::Layout, cell::OnceCell};

use crate::{
    debug, info,
    utils::{
        limine::{get_executable_address, get_executable_file, get_hhdm_offset, get_memory_map},
        spinlock::Spin,
    },
};

pub mod page_size {
    pub const SMALL: u64 = 0x1000; // 4KiB
    pub const MEDIUM: u64 = 0x200000; // 2MiB
    pub const LARGE: u64 = 0x40000000; // 1GiB
}

pub mod flag {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITE: u64 = 1 << 1;
    pub const USER: u64 = 1 << 2;
    pub const LPAGES: u64 = 1 << 7;
    pub const NO_EXEC: u64 = 1 << 63;

    pub const RW: u64 = PRESENT | WRITE;

    pub const PADDR_MASK: u64 = 0x000F_FFFF_FFFF_F000;
    pub const TABLE_FLAGS: u64 = PRESENT | WRITE | USER;
}

pub static mut PAGEMAP: OnceCell<Arc<Spin<Pagemap>>> = OnceCell::new();

unsafe impl Send for Pagemap {}
unsafe impl Sync for Pagemap {}

#[derive(Clone)]
pub struct Pagemap {
    pub top_level: *mut u64,
}

impl Default for Pagemap {
    fn default() -> Self {
        Self::new()
    }
}

impl Pagemap {
    pub fn new() -> Pagemap {
        Pagemap {
            top_level: alloc_table(),
        }
    }

    pub fn map(
        &mut self,
        virt: u64,
        phys: u64,
        flags: u64,
        psize: u64,
    ) -> Result<(), &'static str> {
        let hhdm = get_hhdm_offset();

        let pml4_entry = (virt & (0x1ff << 39)) >> 39;
        let pml3_entry = (virt & (0x1ff << 30)) >> 30;
        let pml2_entry = (virt & (0x1ff << 21)) >> 21;
        let pml1_entry = (virt & (0x1ff << 12)) >> 12;

        let pml4 = (self.top_level as u64 + hhdm) as *mut u64;

        let pml3 = get_next_level(pml4, pml4_entry, psize, 3, virt, self)?;
        if psize == page_size::LARGE {
            unsafe {
                *pml3.add(pml3_entry as usize) = phys | flags | flag::LPAGES;
            }
            return Ok(());
        }

        let pml2 = get_next_level(pml3, pml3_entry, psize, 2, virt, self)?;
        if psize == page_size::MEDIUM {
            unsafe {
                *pml2.add(pml2_entry as usize) = phys | flags | flag::LPAGES;
            }
            return Ok(());
        }

        let pml1 = get_next_level(pml2, pml2_entry, psize, 1, virt, self)?;
        unsafe {
            *pml1.add(pml1_entry as usize) = phys | flags;
        }

        Ok(())
    }

    pub fn map_pages(
        &mut self,
        virt: u64,
        phys: u64,
        flags: u64,
        count: u64,
    ) -> Result<(), &'static str> {
        assert!(
            virt.is_multiple_of(page_size::SMALL)
                && phys.is_multiple_of(page_size::SMALL)
                && count.is_multiple_of(page_size::SMALL),
            "vmm: misaligned call to map_pages"
        );

        let mut i: u64 = 0;
        while i < count {
            if (phys + i) & (page_size::LARGE - 1) == 0
                && (virt + i) & (page_size::LARGE - 1) == 0
                && count - i >= page_size::LARGE
            {
                self.map(virt + i, phys + i, flags, page_size::LARGE)?;
                i += page_size::LARGE;
                continue;
            }
            if (phys + i) & (page_size::MEDIUM - 1) == 0
                && (virt + i) & (page_size::MEDIUM - 1) == 0
                && count - i >= page_size::MEDIUM
            {
                self.map(virt + i, phys + i, flags, page_size::MEDIUM)?;
                i += page_size::MEDIUM;
                continue;
            }
            self.map(virt + i, phys + i, flags, page_size::SMALL)?;
            i += page_size::SMALL;
        }

        Ok(())
    }
}

fn alloc_table() -> *mut u64 {
    unsafe {
        let ptr = alloc::alloc::alloc_zeroed(Layout::from_size_align(0x1000, 0x1000).unwrap());
        if ptr.is_null() {
            panic!("vmm: failed to allocate page table");
        }
        (ptr as u64 - get_hhdm_offset()) as _
    }
}

const PAGE_SIZES: [u64; 4] = [
    0, // unused (level 0)
    page_size::SMALL,
    page_size::MEDIUM,
    page_size::LARGE,
];

fn pte_to_flags(entry: u64) -> u64 {
    entry & (flag::WRITE | flag::USER | flag::NO_EXEC)
}

fn is_table(entry: u64) -> bool {
    (entry & (flag::PRESENT | flag::LPAGES)) == flag::PRESENT
}

fn is_large(entry: u64) -> bool {
    (entry & (flag::PRESENT | flag::LPAGES)) == (flag::PRESENT | flag::LPAGES)
}

fn get_next_level(
    current_level: *mut u64,
    idx: u64,
    _desired_psize: u64,
    level_idx: usize,
    virt: u64,
    pagemap: &mut Pagemap,
) -> Result<*mut u64, &'static str> {
    unsafe {
        let hhdm = get_hhdm_offset();
        let entry = &mut *current_level.add(idx as usize);

        if *entry & flag::PRESENT != 0 && is_table(*entry) {
            let addr = (*entry & flag::PADDR_MASK)
                .checked_add(hhdm)
                .ok_or("vmm: address overflow in get_next_level")?;
            return Ok(addr as _);
        }

        if is_large(*entry) {
            if level_idx == 0 || level_idx >= 3 {
                panic!("vmm: unexpected level {} in get_next_level", level_idx);
            }

            let old_page_size = PAGE_SIZES[level_idx + 1];
            let old_flags = pte_to_flags(*entry);
            let old_phys = *entry & flag::PADDR_MASK;
            let old_virt = virt & !(old_page_size - 1);

            let new_table = alloc_table() as u64;
            *entry = new_table | flag::TABLE_FLAGS;

            let split_size = PAGE_SIZES[level_idx];
            let split_flags = flag::PRESENT | old_flags;

            let mut offset = 0u64;
            while offset < old_page_size {
                pagemap.map(
                    old_virt + offset,
                    old_phys + offset,
                    split_flags,
                    split_size,
                )?;
                offset += split_size;
            }

            let entry_val = *current_level.add(idx as usize);
            let addr = (entry_val & flag::PADDR_MASK) + hhdm;
            return Ok(addr as _);
        }

        let next_level = alloc_table() as u64;
        if next_level == 0 {
            return Err("vmm: couldn't allocate page table");
        }

        *entry = next_level | flag::TABLE_FLAGS;

        Ok((next_level + hhdm) as _)
    }
}

pub fn init() {
    let mem_map = get_memory_map();
    let hhdm = get_hhdm_offset();
    info!("setting up the kernel pagemap...");
    debug!("hhdm offset is: 0x{:X}", hhdm);

    let mut pmap = Pagemap::new();

    for entry in mem_map {
        let etype = entry.type_;
        if etype != limine::memmap::MEMMAP_USABLE
            && etype != limine::memmap::MEMMAP_BOOTLOADER_RECLAIMABLE
            && etype != limine::memmap::MEMMAP_EXECUTABLE_AND_MODULES
            && etype != limine::memmap::MEMMAP_FRAMEBUFFER
        {
            continue;
        }

        let base = entry.base;
        let length = entry.length;

        debug!(
            "mapping 0x{:X}-0x{:X} ({} bytes)",
            base,
            base + length,
            length
        );

        if let Err(err) = pmap.map_pages(base + hhdm, base, flag::RW, length) {
            panic!(
                "couldn't map region 0x{:X}-0x{:X}: {}",
                base,
                base + length,
                err
            );
        }
    }

    let executable_address_response = get_executable_address();
    let phys_base = executable_address_response.physical_base;
    let virt_base = executable_address_response.virtual_base;

    let size = get_executable_file().data().len() as u64;

    debug!(
        "mapping kernel executable: virt 0x{:X}, phys 0x{:X}, size 0x{:X}",
        virt_base, phys_base, size
    );

    for i in (0..size).step_by(page_size::SMALL as usize) {
        if let Err(err) = pmap.map(virt_base + i, phys_base + i, flag::RW, page_size::SMALL) {
            panic!(
                "couldn't map kernel executable 0x{:X} -> 0x{:X}: {}",
                virt_base + i,
                phys_base + i,
                err
            );
        }
    }

    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) pmap.top_level as u64, options(nostack));
        PAGEMAP.set(Arc::new(Spin::new(pmap))).ok();
    }
    info!("done");
}
