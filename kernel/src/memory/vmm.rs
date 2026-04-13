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

    pub fn is_mapped(&self, virt: u64) -> bool {
        let hhdm = get_hhdm_offset();

        let pml4_entry = (virt & (0x1ff << 39)) >> 39;
        let pml3_entry = (virt & (0x1ff << 30)) >> 30;
        let pml2_entry = (virt & (0x1ff << 21)) >> 21;
        let pml1_entry = (virt & (0x1ff << 12)) >> 12;

        let pml4 = (self.top_level as u64 + hhdm) as *const u64;

        unsafe {
            let pml4e = *pml4.add(pml4_entry as usize);
            if pml4e & flag::PRESENT == 0 {
                return false;
            }

            let pml3 = ((pml4e & flag::PADDR_MASK) + hhdm) as *const u64;
            let pml3e = *pml3.add(pml3_entry as usize);
            if pml3e & flag::PRESENT == 0 {
                return false;
            }
            if pml3e & flag::LPAGES != 0 {
                return true;
            }

            let pml2 = ((pml3e & flag::PADDR_MASK) + hhdm) as *const u64;
            let pml2e = *pml2.add(pml2_entry as usize);
            if pml2e & flag::PRESENT == 0 {
                return false;
            }
            if pml2e & flag::LPAGES != 0 {
                return true;
            }

            let pml1 = ((pml2e & flag::PADDR_MASK) + hhdm) as *const u64;
            let pml1e = *pml1.add(pml1_entry as usize);
            pml1e & flag::PRESENT != 0
        }
    }

    pub fn new_user() -> Pagemap {
        let hhdm = get_hhdm_offset();
        let new = Pagemap::new();
        let kernel_pml4 = unsafe {
            let kp = PAGEMAP.get().unwrap().lock();
            (kp.top_level as u64 + hhdm) as *const u64
        };
        let new_pml4 = (new.top_level as u64 + hhdm) as *mut u64;
        unsafe {
            core::ptr::copy_nonoverlapping(kernel_pml4.add(256), new_pml4.add(256), 256);
        }
        new
    }

    pub fn clone_userspace(&self) -> Pagemap {
        let hhdm = get_hhdm_offset();
        let new = Pagemap::new_user();
        let src_pml4 = (self.top_level as u64 + hhdm) as *const u64;
        let dst_pml4 = (new.top_level as u64 + hhdm) as *mut u64;

        for i4 in 0..256u64 {
            let pml4e = unsafe { *src_pml4.add(i4 as usize) };
            if pml4e & flag::PRESENT == 0 {
                continue;
            }

            let src_pml3 = ((pml4e & flag::PADDR_MASK) + hhdm) as *const u64;
            let new_pml3 = alloc_table();
            unsafe {
                *dst_pml4.add(i4 as usize) = (new_pml3 as u64) | (pml4e & !flag::PADDR_MASK);
            }
            let dst_pml3 = (new_pml3 as u64 + hhdm) as *mut u64;

            for i3 in 0..512u64 {
                let pml3e = unsafe { *src_pml3.add(i3 as usize) };
                if pml3e & flag::PRESENT == 0 {
                    continue;
                }
                if pml3e & flag::LPAGES != 0 {
                    let old_phys = pml3e & flag::PADDR_MASK;
                    let new_phys = alloc_pages(page_size::LARGE as usize);
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            (old_phys + hhdm) as *const u8,
                            (new_phys + hhdm) as *mut u8,
                            page_size::LARGE as usize,
                        );
                        *dst_pml3.add(i3 as usize) = new_phys | (pml3e & !flag::PADDR_MASK);
                    }
                    continue;
                }

                let src_pml2 = ((pml3e & flag::PADDR_MASK) + hhdm) as *const u64;
                let new_pml2 = alloc_table();
                unsafe {
                    *dst_pml3.add(i3 as usize) = (new_pml2 as u64) | (pml3e & !flag::PADDR_MASK);
                }
                let dst_pml2 = (new_pml2 as u64 + hhdm) as *mut u64;

                for i2 in 0..512u64 {
                    let pml2e = unsafe { *src_pml2.add(i2 as usize) };
                    if pml2e & flag::PRESENT == 0 {
                        continue;
                    }
                    if pml2e & flag::LPAGES != 0 {
                        let old_phys = pml2e & flag::PADDR_MASK;
                        let new_phys = alloc_pages(page_size::MEDIUM as usize);
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                (old_phys + hhdm) as *const u8,
                                (new_phys + hhdm) as *mut u8,
                                page_size::MEDIUM as usize,
                            );
                            *dst_pml2.add(i2 as usize) = new_phys | (pml2e & !flag::PADDR_MASK);
                        }
                        continue;
                    }

                    let src_pml1 = ((pml2e & flag::PADDR_MASK) + hhdm) as *const u64;
                    let new_pml1 = alloc_table();
                    unsafe {
                        *dst_pml2.add(i2 as usize) =
                            (new_pml1 as u64) | (pml2e & !flag::PADDR_MASK);
                    }
                    let dst_pml1 = (new_pml1 as u64 + hhdm) as *mut u64;

                    for i1 in 0..512u64 {
                        let pml1e = unsafe { *src_pml1.add(i1 as usize) };
                        if pml1e & flag::PRESENT == 0 {
                            continue;
                        }
                        let old_phys = pml1e & flag::PADDR_MASK;
                        let new_phys = alloc_pages(page_size::SMALL as usize);
                        unsafe {
                            core::ptr::copy_nonoverlapping(
                                (old_phys + hhdm) as *const u8,
                                (new_phys + hhdm) as *mut u8,
                                page_size::SMALL as usize,
                            );
                            *dst_pml1.add(i1 as usize) = new_phys | (pml1e & !flag::PADDR_MASK);
                        }
                    }
                }
            }
        }

        new
    }

    pub fn destroy_userspace(&mut self) {
        let hhdm = get_hhdm_offset();
        let pml4 = (self.top_level as u64 + hhdm) as *mut u64;

        for i4 in 0..256u64 {
            let pml4e = unsafe { *pml4.add(i4 as usize) };
            if pml4e & flag::PRESENT == 0 {
                continue;
            }

            let pml3 = ((pml4e & flag::PADDR_MASK) + hhdm) as *mut u64;

            for i3 in 0..512u64 {
                let pml3e = unsafe { *pml3.add(i3 as usize) };
                if pml3e & flag::PRESENT == 0 {
                    continue;
                }
                if pml3e & flag::LPAGES != 0 {
                    free_pages(pml3e & flag::PADDR_MASK, page_size::LARGE as usize);
                    continue;
                }

                let pml2 = ((pml3e & flag::PADDR_MASK) + hhdm) as *mut u64;

                for i2 in 0..512u64 {
                    let pml2e = unsafe { *pml2.add(i2 as usize) };
                    if pml2e & flag::PRESENT == 0 {
                        continue;
                    }
                    if pml2e & flag::LPAGES != 0 {
                        free_pages(pml2e & flag::PADDR_MASK, page_size::MEDIUM as usize);
                        continue;
                    }

                    let pml1 = ((pml2e & flag::PADDR_MASK) + hhdm) as *mut u64;

                    for i1 in 0..512u64 {
                        let pml1e = unsafe { *pml1.add(i1 as usize) };
                        if pml1e & flag::PRESENT == 0 {
                            continue;
                        }
                        free_pages(pml1e & flag::PADDR_MASK, page_size::SMALL as usize);
                    }

                    free_table(pml1);
                }

                free_table(pml2);
            }

            free_table(pml3);

            unsafe {
                *pml4.add(i4 as usize) = 0;
            }
        }
    }

    pub fn cr3(&self) -> u64 {
        self.top_level as u64
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

fn alloc_pages(size: usize) -> u64 {
    unsafe {
        let ptr =
            alloc::alloc::alloc_zeroed(Layout::from_size_align(size, size.min(0x1000)).unwrap());
        if ptr.is_null() {
            panic!("vmm: failed to allocate pages");
        }
        ptr as u64 - get_hhdm_offset()
    }
}

fn free_pages(phys: u64, size: usize) {
    let hhdm = get_hhdm_offset();
    unsafe {
        alloc::alloc::dealloc(
            (phys + hhdm) as *mut u8,
            Layout::from_size_align(size, size.min(0x1000)).unwrap(),
        );
    }
}

fn free_table(virt_ptr: *mut u64) {
    let hhdm = get_hhdm_offset();
    let phys = virt_ptr as u64 - hhdm;
    unsafe {
        alloc::alloc::dealloc(
            (phys + hhdm) as *mut u8,
            Layout::from_size_align(0x1000, 0x1000).unwrap(),
        );
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
