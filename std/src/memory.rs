/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use talc::*;

#[cfg(feature = "kernel")]
use crate::{
    align_down, align_up,
    alloc::sync::Arc,
    debug, error, info,
    kernel::{
        bootloader::{
            get_executable_address, get_executable_file, get_hhdm_offset, get_memory_map,
            limine::memory_map::EntryType,
        },
        paging::{PAGEMAP, Pagemap, flag, page_size},
    },
    spinlock::SpinLock,
};

pub const KERNEL_STACK_SIZE: usize = 64 * 1024;
pub static KERNEL_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

pub const USER_STACK_SIZE: usize = 64 * 1024;

#[global_allocator]
pub static ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> =
    Talc::new(unsafe { ClaimOnOom::new(Span::from_array((&raw const KERNEL_STACK).cast_mut())) })
        .lock();

#[cfg(feature = "kernel")]
pub struct TotalMemory {
    pub usable_bytes: u64,
    pub reserved_bytes: u64,
}

#[cfg(feature = "kernel")]
pub static mut MEMORY_INIT_STAGE: u8 = 0;
#[cfg(feature = "kernel")]
pub static mut TOTAL_MEMORY: TotalMemory = TotalMemory {
    usable_bytes: 0,
    reserved_bytes: 0,
};

#[cfg(feature = "kernel")]
pub fn init() {
    info!("setting up...");
    unsafe {
        debug!("requesting hhdm and memmap...");
        let hhdm_offset = get_hhdm_offset();
        let mem_map = get_memory_map();

        let mut allocator = ALLOCATOR.lock();

        for entry in mem_map {
            if entry.entry_type == limine::memory_map::EntryType::USABLE {
                debug!(
                    "claiming 0x{:X}-0x{:X}...",
                    entry.base,
                    entry.base + hhdm_offset
                );
                allocator
                    .claim(talc::Span::from_base_size(
                        (entry.base + hhdm_offset) as *mut u8,
                        entry.length as usize,
                    ))
                    .ok();
                TOTAL_MEMORY.usable_bytes += entry.length;
            } else if entry.entry_type == limine::memory_map::EntryType::RESERVED {
                TOTAL_MEMORY.reserved_bytes += entry.length
            }
        }

        info!("done");
        MEMORY_INIT_STAGE = 1;
    }

    {
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
        info!("memory setup done");
    }
}

#[cfg(feature = "kernel")]
pub fn get_memory_init_stage() -> u8 {
    unsafe { MEMORY_INIT_STAGE }
}

#[cfg(feature = "kernel")]
pub fn get_usable_memory() -> u64 {
    unsafe { TOTAL_MEMORY.usable_bytes }
}

#[cfg(feature = "kernel")]
pub fn get_reserved_memory() -> u64 {
    unsafe { TOTAL_MEMORY.reserved_bytes }
}
