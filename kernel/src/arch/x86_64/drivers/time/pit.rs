/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::sync::atomic::{AtomicU64, Ordering};

use alloc::string::String;

use crate::{
    arch::interrupts::StackFrame,
    info,
    utils::{
        asm::port::{outb, outl},
        time::Timer,
    },
};

pub const PIT_FREQUENCY: u32 = 1193182;
pub static ELAPSED_MS: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    info!("setting up at 1000hz...");
    outb(0x43, 0b00110100);
    outl(0x40, (PIT_FREQUENCY / 1000) & 0xFF);
    outl(0x40, (PIT_FREQUENCY / 1000) >> 8);
    super::register_timer(Timer::new(
        "PIT",
        0,
        1000,
        true,
        !0,
        |_: &Timer| ELAPSED_MS.load(Ordering::Relaxed) * 1_000_000,
        0,
    ));
    crate::arch::interrupts::pic::unmask(0);
    info!("done");
}

pub fn timer_interrupt_handler(_stack_frame: *mut StackFrame) {
    pit_tick();
    crate::arch::interrupts::pic::send_eoi(0);
}

pub fn elapsed_pretty(digits: u32) -> String {
    crate::utils::time::elapsed_time_pretty(current_pit_ticks() * 1_000_000, digits)
}

pub fn pit_tick() {
    ELAPSED_MS.fetch_add(1, Ordering::Relaxed);
}

pub fn current_pit_ticks() -> u64 {
    ELAPSED_MS.load(Ordering::Relaxed)
}
