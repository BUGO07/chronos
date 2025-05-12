/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

pub trait KernelTimer {
    fn name(&self) -> &'static str;
    fn is_supported(&self) -> bool;
    fn priority(&self) -> u8; // unused as of now
    fn elapsed_ns(&self) -> u64;

    fn elapsed_pretty(&self, digits: u32) -> alloc::string::String {
        elapsed_time_pretty(self.elapsed_ns(), digits)
    }
}

pub fn elapsed_time_pretty(ns: u64, digits: u32) -> alloc::string::String {
    let subsecond_ns = ns % 1_000_000_000;

    let divisor = 10u64.pow(9 - digits);
    let subsecond = subsecond_ns / divisor;

    let elapsed_ms = ns / 1_000_000;
    let seconds_total = elapsed_ms / 1000;
    let seconds = seconds_total % 60;
    let minutes_total = seconds_total / 60;
    let minutes = minutes_total % 60;
    let hours = minutes_total / 60;

    alloc::format!(
        "{:02}:{:02}:{:02}.{:0width$}",
        hours,
        minutes,
        seconds,
        subsecond,
        width = digits as usize
    )
}

#[inline(always)]
pub fn busywait_ns(ns: u64) {
    let start = crate::arch::drivers::time::preferred_timer_ns();
    while crate::arch::drivers::time::preferred_timer_ns() - start < ns {
        core::hint::spin_loop();
    }
}

#[inline(always)]
pub fn busywait_ms(ms: u64) {
    busywait_ns(ms * 1_000_000);
}
