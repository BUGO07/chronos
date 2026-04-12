/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use talc::{source::Claim, *};

use crate::{
    debug, info,
    utils::limine::{get_hhdm_offset, get_memory_map},
};

pub mod vmm;

pub const KERNEL_STACK_SIZE: usize = 64 * 1024;
pub const USER_STACK_SIZE: usize = 64 * 1024;

#[global_allocator]
pub static ALLOCATOR: TalcLock<spin::Mutex<()>, Claim> = TalcLock::new(unsafe {
    static mut INITIAL_HEAP: [u8; min_first_heap_size::<DefaultBinning>() + 128 * 1024] =
        [0; min_first_heap_size::<DefaultBinning>() + 128 * 1024];

    Claim::array(&raw mut INITIAL_HEAP)
});

pub static MEMORY_INIT_STAGE: AtomicU8 = AtomicU8::new(0);
static USABLE_MEMORY: AtomicU64 = AtomicU64::new(0);
static RESERVED_MEMORY: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    info!("setting up...");
    unsafe {
        debug!("requesting hhdm and memmap...");
        let hhdm_offset = get_hhdm_offset();
        let mem_map = get_memory_map();

        let mut allocator = ALLOCATOR.lock();

        for entry in mem_map {
            if entry.type_ == limine::memmap::MEMMAP_USABLE {
                debug!(
                    "claiming 0x{:X}-0x{:X}...",
                    entry.base,
                    entry.base + entry.length
                );
                allocator.claim((entry.base + hhdm_offset) as _, entry.length as usize);
                USABLE_MEMORY.fetch_add(entry.length, Ordering::Relaxed);
            } else if entry.type_ == limine::memmap::MEMMAP_RESERVED {
                RESERVED_MEMORY.fetch_add(entry.length, Ordering::Relaxed);
            }
        }

        info!("done");
        MEMORY_INIT_STAGE.store(1, Ordering::Release);
    }
    vmm::init();
    MEMORY_INIT_STAGE.store(2, Ordering::Release);
}

pub fn get_memory_init_stage() -> u8 {
    MEMORY_INIT_STAGE.load(Ordering::Acquire)
}

pub fn get_usable_memory() -> u64 {
    USABLE_MEMORY.load(Ordering::Relaxed)
}

pub fn get_reserved_memory() -> u64 {
    RESERVED_MEMORY.load(Ordering::Relaxed)
}
