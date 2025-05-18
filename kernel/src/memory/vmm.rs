/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

#[cfg(target_arch = "x86_64")]
pub use x86_64::*;

#[cfg(target_arch = "x86_64")]
mod x86_64 {
    use alloc::sync::Arc;
    use core::{alloc::Layout, cell::OnceCell, ptr::null_mut};
    use limine::memory_map::EntryType;

    use crate::{
        debug, error, info,
        utils::{
            align_down, align_up,
            limine::{
                get_executable_address, get_executable_file, get_hhdm_offset, get_memory_map,
            },
            spinlock::SpinLock,
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
    }

    pub static mut PAGEMAP: OnceCell<Arc<SpinLock<Pagemap>>> = OnceCell::new(); // wtf

    unsafe impl Send for Pagemap {}
    unsafe impl Sync for Pagemap {}

    #[repr(C)]
    #[repr(packed)]
    pub struct Table {
        entries: [u64; 512],
    }

    #[derive(Copy, Clone)]
    pub struct Pagemap {
        pub top_level: *mut Table,
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

        pub fn map(&mut self, virt: u64, phys: u64, mut flags: u64, psize: u64) -> bool {
            let hhdm = get_hhdm_offset();
            if !(phys <= !0 - hhdm || phys >= hhdm) {
                error!("illegal physical address: 0x{:X}", phys);
                return false;
            }
            let pml4_entry = (virt & (0x1ff << 39)) >> 39;
            let pml3_entry = (virt & (0x1ff << 30)) >> 30;
            let pml2_entry = (virt & (0x1ff << 21)) >> 21;
            let pml1_entry = (virt & (0x1ff << 12)) >> 12;

            let pml4 = (self.top_level as u64 + hhdm) as *mut Table;

            let pml3 = get_next_level(pml4, pml4_entry, true);
            if pml3.is_null() {
                return false;
            }
            if psize == page_size::LARGE {
                flags |= flag::LPAGES;
                unsafe {
                    (*pml3).entries[pml3_entry as usize] = phys | flags;
                }
                return true;
            }

            let pml2 = get_next_level(pml3, pml3_entry, true);
            if pml2.is_null() {
                return false;
            }
            if psize == page_size::MEDIUM {
                flags |= flag::LPAGES;
                unsafe {
                    (*pml2).entries[pml2_entry as usize] = phys | flags;
                }
                return true;
            }

            let pml1 = get_next_level(pml2, pml2_entry, true);
            if pml1.is_null() {
                return false;
            }

            unsafe {
                (*pml1).entries[pml1_entry as usize] = phys | flags;
            }

            true
        }
    }

    fn alloc_table() -> *mut Table {
        unsafe {
            (alloc::alloc::alloc_zeroed(Layout::from_size_align(0x1000, 0x1000).unwrap()) as u64
                - get_hhdm_offset()) as *mut Table
        }
    }

    fn get_next_level(top_level: *mut Table, idx: u64, allocate: bool) -> *mut Table {
        unsafe {
            let hhdm = get_hhdm_offset();
            let entry = top_level.cast::<u64>().add(idx as usize);
            if !(*entry <= !0 - hhdm || *entry >= hhdm) {
                panic!("illegal entry: 0x{:X}", *entry);
            }
            if *entry & flag::PRESENT != 0 {
                return ((*entry & 0x000FFFFFFFFFF000) + hhdm) as *mut Table;
            }

            if !allocate {
                return null_mut();
            }

            let next_level = alloc_table() as u64;
            *entry = next_level | flag::RW | flag::USER;
            (next_level + hhdm) as *mut Table
        }
    }

    pub fn init() {
        let mem_map = get_memory_map();
        let hhdm_offset = get_hhdm_offset();
        info!("setting up the kernel pagemap...");
        debug!("hhdm offset is: 0x{:X}", hhdm_offset);

        let mut pmap = Pagemap::new();

        for entry in mem_map {
            let etype = entry.entry_type;
            if etype != EntryType::USABLE
                && etype != EntryType::BOOTLOADER_RECLAIMABLE
                && etype != EntryType::EXECUTABLE_AND_MODULES
                && etype != EntryType::FRAMEBUFFER
            {
                continue;
            }

            // ! hard setting this to LARGE stops my laptop from crashing
            let psize = page_size::LARGE;

            let base = align_down(entry.base, psize);
            let end = align_up(entry.base + entry.length, psize);

            debug!(
                "size: {} bytes, 0x{:X} -> 0x{:X}",
                end - base,
                base,
                base + hhdm_offset
            );

            for i in (base..end).step_by(psize as usize) {
                if !(i <= !0 - hhdm_offset || i >= hhdm_offset) {
                    error!("illegal physical address: 0x{:X}", i);
                    continue;
                }
                if !pmap.map(i + hhdm_offset, i, flag::RW, psize) {
                    panic!("couldn't map 0x{:X} -> 0x{:X}", i, i + hhdm_offset);
                }
            }
        }

        let executable_address_response = get_executable_address();
        let phys_base = executable_address_response.physical_base();
        let virt_base = executable_address_response.virtual_base();

        let size = get_executable_file().size();

        for i in (0..size).step_by(page_size::SMALL as usize) {
            if !pmap.map(virt_base + i, phys_base + i, flag::RW, page_size::SMALL) {
                panic!(
                    "couldn't map kernel executable 0x{:X} -> 0x{:X}",
                    virt_base + i,
                    phys_base + i
                );
            }
        }

        unsafe {
            core::arch::asm!("mov cr3, {}", in(reg) pmap.top_level as u64, options(nostack));
            PAGEMAP.set(Arc::new(SpinLock::new(pmap))).ok();
        }
        info!("done");
    }
}

#[cfg(target_arch = "aarch64")]
mod aarch64 {
    // TODO: implement
    pub fn init() {}
}

#[cfg(target_arch = "aarch64")]
pub use aarch64::*;
