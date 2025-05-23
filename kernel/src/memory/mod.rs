/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use talc::*;

use crate::{
    debug, info,
    utils::limine::{get_hhdm_offset, get_memory_map},
};

pub mod vmm;

pub const KERNEL_STACK_SIZE: usize = 64 * 1024;
pub static KERNEL_STACK: [u8; KERNEL_STACK_SIZE] = [0; KERNEL_STACK_SIZE];

pub const USER_STACK_SIZE: usize = 64 * 1024;
pub const USER_STACK_BASE: usize = 0x800000;

#[global_allocator]
pub static ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> =
    Talc::new(unsafe { ClaimOnOom::new(Span::from_array((&raw const KERNEL_STACK).cast_mut())) })
        .lock();

pub struct TotalMemory {
    usable_bytes: u64,
    reserved_bytes: u64,
}

pub static mut MEMORY_INIT_STAGE: u8 = 0;
pub static mut TOTAL_MEMORY: TotalMemory = TotalMemory {
    usable_bytes: 0,
    reserved_bytes: 0,
};

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
                    .claim(Span::from_base_size(
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
    vmm::init();
    unsafe { MEMORY_INIT_STAGE = 2 };
}

pub fn get_memory_init_stage() -> u8 {
    unsafe { MEMORY_INIT_STAGE }
}

pub fn get_usable_memory() -> u64 {
    unsafe { TOTAL_MEMORY.usable_bytes }
}

pub fn get_reserved_memory() -> u64 {
    unsafe { TOTAL_MEMORY.reserved_bytes }
}
