/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{arch::x86_64::_rdtsc, cell::OnceCell, sync::atomic::Ordering};

use crate::{debug, info};

use super::{KernelTimer, TimerFuture, pit::current_pit_ticks};

pub static mut TSC_TIMER: OnceCell<TscTimer> = OnceCell::new();

pub struct TscTimer {
    start: u64,
    supported: bool,
}

impl TscTimer {
    pub fn start() -> Self {
        TscTimer {
            start: unsafe { _rdtsc() },
            supported: true,
        }
    }

    pub fn elapsed_cycles(&self) -> u64 {
        unsafe { _rdtsc() - self.start }
    }

    pub fn set_supported(&mut self, supported: bool) {
        self.supported = supported;
    }
}

impl KernelTimer for TscTimer {
    fn elapsed_ns(&self) -> u64 {
        unsafe {
            (self.elapsed_cycles() as u128 * 1_000_000_000
                / crate::CPU_FREQ.load(Ordering::Relaxed) as u128) as u64
        }
    }

    fn is_supported(&self) -> bool {
        self.supported
    }

    fn name(&self) -> &'static str {
        "TSC"
    }
}

pub async fn measure_cpu_frequency_async() -> u64 {
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

pub fn measure_cpu_frequency_bl() -> u64 {
    if super::kvm::supported() {
        return super::kvm::tsc_freq();
    }

    let start_cycles = unsafe { _rdtsc() };
    let start_ticks = current_pit_ticks();

    while start_ticks + 100 > current_pit_ticks() {}

    let end_cycles = unsafe { _rdtsc() };
    let end_ticks = current_pit_ticks();

    let elapsed_ticks = end_ticks - start_ticks;
    let elapsed_cycles = end_cycles - start_cycles;

    let cycles_per_tick = elapsed_cycles / elapsed_ticks;

    let cpu_freq_hz = cycles_per_tick * 1000;

    cpu_freq_hz
}

pub fn init() {
    unsafe {
        info!("setting up...");
        let freq = measure_cpu_frequency_bl();
        info!("cpu frequency - {}hz", freq);
        crate::CPU_FREQ.store(freq, Ordering::Relaxed);
        TSC_TIMER.set(TscTimer::start()).ok();
        debug!("done");
    }
}
