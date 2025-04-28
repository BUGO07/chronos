/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{arch::x86_64::_rdtsc, cell::OnceCell, sync::atomic::Ordering};

use alloc::{format, string::String};

use super::{TimerFuture, pit::current_pit_ticks};

pub static mut TSC_TIMER: OnceCell<TscTimer> = OnceCell::new();

pub struct TscTimer {
    start: u64,
    supported: bool,
}

impl TscTimer {
    pub fn start() -> Self {
        TscTimer {
            start: unsafe { _rdtsc() },
            supported: false, // for now
        }
    }

    pub fn elapsed_cycles(&self) -> u64 {
        unsafe { _rdtsc() - self.start }
    }

    pub fn elapsed_ns(&self) -> u64 {
        unsafe {
            (self.elapsed_cycles() as u128 * 1_000_000_000
                / crate::CPU_FREQ.load(Ordering::Relaxed) as u128) as u64
        }
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

    pub fn is_supported(&self) -> bool {
        self.supported
    }

    pub fn set_supported(&mut self, supported: bool) {
        self.supported = supported;
    }
}

pub async fn measure_cpu_frequency() -> u64 {
    if super::kvm::supported() {
        return super::kvm::tsc_freq();
    }
    let start_cycles = unsafe { _rdtsc() };
    let start_ticks = current_pit_ticks();

    TimerFuture::new(100).await;

    let end_cycles = unsafe { _rdtsc() };
    let end_ticks = current_pit_ticks();

    let elapsed_ticks = end_ticks - start_ticks;
    let elapsed_cycles = end_cycles - start_cycles;

    let cycles_per_tick = elapsed_cycles / elapsed_ticks;

    let cpu_freq_hz = cycles_per_tick * 1000;

    cpu_freq_hz
}

pub fn init() {
    unsafe { TSC_TIMER.set(TscTimer::start()).ok() };
}
