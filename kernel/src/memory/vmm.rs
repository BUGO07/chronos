/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{alloc::Layout, cell::OnceCell, ptr::null_mut};

use crate::utils::{align_down, align_up};
use alloc::sync::Arc;
use limine::memory_map::EntryType;
use spin::Mutex;

use crate::{
    debug, info,
    utils::limine::{get_executable_address, get_executable_file},
};

use super::{ALLOCATOR, get_hhdm_offset};

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
}

pub static mut PAGEMAP: OnceCell<Arc<Mutex<Pagemap>>> = OnceCell::new();

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
        let pml4_entry = (virt & (0x1ff << 39)) >> 39;
        let pml3_entry = (virt & (0x1ff << 30)) >> 30;
        let pml2_entry = (virt & (0x1ff << 21)) >> 21;
        let pml1_entry = (virt & (0x1ff << 12)) >> 12;

        let pml4 = (self.top_level as u64 + super::get_hhdm_offset()) as *mut Table;

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
    let ptr = unsafe {
        (ALLOCATOR
            .lock()
            .malloc(Layout::from_size_align(0x1000, 0x1000).unwrap())
            .unwrap()
            .as_ptr() as u64
            - super::get_hhdm_offset()) as *mut Table
    };
    unsafe {
        for i in 0..512 {
            (*((ptr as u64 + get_hhdm_offset()) as *mut Table)).entries[i] = 0;
        }
    }
    ptr
}

fn get_next_level(top_level: *mut Table, idx: u64, allocate: bool) -> *mut Table {
    unsafe {
        let entry = top_level.cast::<u64>().add(idx as usize);
        if *entry & flag::PRESENT != 0 {
            return ((*entry & 0x000FFFFFFFFFF000) + super::get_hhdm_offset()) as *mut Table;
        }

        if !allocate {
            return null_mut();
        }

        let next_level = alloc_table() as u64;
        *entry = next_level | flag::PRESENT | flag::WRITE | flag::USER;
        (next_level + super::get_hhdm_offset()) as *mut Table
    }
}

pub fn init() {
    let mem_map = super::get_memory_map();
    let hhdm_offset = super::get_hhdm_offset();
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
        let psize = if entry.length >= page_size::LARGE {
            page_size::LARGE
        } else if entry.length >= page_size::MEDIUM {
            page_size::MEDIUM
        } else {
            page_size::SMALL
        };

        let base = align_down(entry.base, psize);
        let end = align_up(entry.base + entry.length, psize);

        debug!(
            "size: {} bytes, 0x{:X} -> 0x{:X}",
            end - base,
            base,
            base + hhdm_offset
        );

        for i in (base..end).step_by(psize as usize) {
            pmap.map(i + hhdm_offset, i, flag::PRESENT | flag::WRITE, psize);
        }
    }

    let executable_address_response = get_executable_address();
    let phys_base = executable_address_response.physical_base();
    let virt_base = executable_address_response.virtual_base();

    let size = get_executable_file().size();

    for i in (0..size).step_by(page_size::SMALL as usize) {
        pmap.map(
            virt_base + i,
            phys_base + i,
            flag::PRESENT | flag::WRITE,
            page_size::SMALL,
        );
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("mov cr3, {}", in(reg) pmap.top_level as u64, options(nostack));
    }
    #[cfg(target_arch = "aarch64")]
    {
        // TODO:
    }
    unsafe { PAGEMAP.set(Arc::new(Mutex::new(pmap))).ok() };
    info!("done");
}
