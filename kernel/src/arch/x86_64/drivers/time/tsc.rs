/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{cell::OnceCell, sync::atomic::Ordering};

use crate::{debug, info, utils::asm::_rdtsc};

use super::{KernelTimer, pit::current_pit_ticks};

pub static mut TSC_TIMER: OnceCell<TscTimer> = OnceCell::new();

pub struct TscTimer {
    start: u64,
    supported: bool,
}

impl TscTimer {
    pub fn start() -> Self {
        TscTimer {
            start: _rdtsc(),
            supported: false,
        }
    }

    pub fn elapsed_cycles(&self) -> u64 {
        _rdtsc() - self.start
    }

    pub fn set_supported(&mut self, supported: bool) {
        self.supported = supported;
    }
}

impl KernelTimer for TscTimer {
    fn is_supported(&self) -> bool {
        self.supported
    }

    fn elapsed_ns(&self) -> u64 {
        if self.supported {
            (self.elapsed_cycles() as u128 * 1_000_000_000
                / crate::arch::CPU_FREQ.load(Ordering::Relaxed) as u128) as u64
        } else {
            0
        }
    }

    fn name(&self) -> &'static str {
        "TSC"
    }

    fn priority(&self) -> u8 {
        10
    }
}

pub fn measure_cpu_frequency() -> u64 {
    if super::kvm::supported() {
        return super::kvm::tsc_freq();
    }

    let mut cpu_freq_hz = 0;

    for _ in 0..3 {
        let start_cycles = _rdtsc();
        let start_ticks = current_pit_ticks();

        while start_ticks + 50 > current_pit_ticks() {}

        let end_cycles = _rdtsc();
        let end_ticks = current_pit_ticks();

        let elapsed_ticks = end_ticks - start_ticks;
        let elapsed_cycles = end_cycles - start_cycles;

        let cycles_per_tick = elapsed_cycles / elapsed_ticks;

        cpu_freq_hz += cycles_per_tick * 1000;
    }

    cpu_freq_hz / 3
}

pub fn init() {
    unsafe {
        info!("setting up...");
        TSC_TIMER.set(TscTimer::start()).ok();
        let freq = measure_cpu_frequency();
        debug!("cpu frequency - {}hz", freq);
        crate::arch::x86_64::CPU_FREQ.store(freq, Ordering::Relaxed);
        TSC_TIMER.get_mut().unwrap().set_supported(true);
        info!("done");
    }
}
