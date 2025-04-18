/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::arch::x86_64::_rdtsc;

use alloc::{format, string::String};
use spin::Mutex;

use crate::task::timer::{TimerFuture, current_ticks};

lazy_static::lazy_static! {
    pub static ref TSC_TIMER: Mutex<TscTimer> = Mutex::new(TscTimer::start());
}

pub struct TscTimer {
    start: u64,
}

impl TscTimer {
    pub fn start() -> Self {
        unsafe { TscTimer { start: _rdtsc() } }
    }

    pub fn elapsed_cycles(&self) -> u64 {
        unsafe { _rdtsc() - self.start }
    }

    pub fn elapsed_ns(&self) -> u64 {
        (self.elapsed_cycles() as u128 * 1_000_000_000 / unsafe { crate::CPU_FREQ as u128 }) as u64 // u128s because it overflowed once
    }

    pub fn elapsed_pretty(&self, digits: u32) -> String {
        let elapsed_ns = self.elapsed_ns();
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
}

pub async fn measure_cpu_frequency() -> u64 {
    if super::kvm::supported() {
        return super::kvm::tsc_freq();
    }
    let start_cycles = unsafe { _rdtsc() };
    let start_ticks = current_ticks();

    TimerFuture::new(100).await;

    let end_cycles = unsafe { _rdtsc() };
    let end_ticks = current_ticks();

    let elapsed_ticks = end_ticks - start_ticks;
    let elapsed_cycles = end_cycles - start_cycles;

    let cycles_per_tick = elapsed_cycles / elapsed_ticks;

    let cpu_freq_hz = cycles_per_tick * 1000;

    cpu_freq_hz
}
