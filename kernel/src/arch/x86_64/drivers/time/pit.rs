/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::sync::atomic::{AtomicU64, Ordering};

use alloc::{format, string::String};

use crate::{
    arch::interrupts::StackFrame,
    info,
    utils::asm::port::{outb, outl},
};

pub const PIT_FREQUENCY: u32 = 1193182;
pub static ELAPSED_MS: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    info!("setting up at 1000hz...");
    outb(0x43, 0b00110100);
    outl(0x40, (PIT_FREQUENCY / 1000) & 0xFF);
    outl(0x40, (PIT_FREQUENCY / 1000) >> 8);
    info!("done");
}

pub fn timer_interrupt_handler(_stack_frame: *mut StackFrame) {
    pit_tick();
    crate::arch::interrupts::pic::send_eoi(0);
}

pub fn elapsed_pretty(digits: u32) -> String {
    let elapsed_ns = current_pit_ticks() * 1_000_000;
    let subsecond_ns = elapsed_ns % 1_000_000_000;

    let divisor = 10u64.pow(9 - digits);
    let subsecond = subsecond_ns / divisor;

    let elapsed_ms = elapsed_ns / 1_000_000;
    let seconds_total = elapsed_ms / 1000;
    let seconds = seconds_total % 60;
    let minutes_total = seconds_total / 60;
    let minutes = minutes_total % 60;
    let hours = minutes_total / 60;

    format!(
        "{:02}:{:02}:{:02}.{:0width$}",
        hours,
        minutes,
        seconds,
        subsecond,
        width = digits as usize
    )
}

pub fn pit_tick() {
    ELAPSED_MS.fetch_add(1, Ordering::Relaxed);
}

pub fn current_pit_ticks() -> u64 {
    ELAPSED_MS.load(Ordering::Relaxed)
}
