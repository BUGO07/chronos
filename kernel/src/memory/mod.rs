/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::sync::atomic::{AtomicU8, AtomicU64, Ordering};

use talc::*;

use crate::{
    debug, info,
    utils::limine::{get_hhdm_offset, get_memory_map},
};

pub mod vmm;

#[global_allocator]
pub static ALLOCATOR: Talck<spin::Mutex<()>, ClaimOnOom> = Talc::new(unsafe {
    ClaimOnOom::new(Span::from_array(
        (&raw const crate::arch::gdt::STACK).cast_mut(),
    ))
})
.lock();

pub static MEMORY_INIT_STAGE: AtomicU8 = AtomicU8::new(0);
pub static TOTAL_USABLE_MEMORY: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    info!("setting up...");
    {
        debug!("requesting hhdm and memmap...");
        let hhdm_offset = get_hhdm_offset();
        let mem_map = get_memory_map();

        let mut allocator = ALLOCATOR.lock();

        let mut total_usable_bytes = 0;

        for entry in mem_map {
            if entry.entry_type == limine::memory_map::EntryType::USABLE {
                unsafe {
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
                    total_usable_bytes += entry.length;
                }
            }
        }

        TOTAL_USABLE_MEMORY.store(total_usable_bytes, Ordering::Relaxed);
    }
    debug!("done");
    MEMORY_INIT_STAGE.store(1, Ordering::Relaxed);
    vmm::init();
    MEMORY_INIT_STAGE.store(2, Ordering::Relaxed);
}
