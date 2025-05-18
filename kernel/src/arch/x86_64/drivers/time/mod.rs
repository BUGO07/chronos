/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::sync::atomic::{AtomicU8, Ordering};

use alloc::string::String;

use crate::utils::time::KernelTimer;

pub mod hpet;
pub mod kvm;
pub mod pit;
pub mod rtc;
pub mod tsc;

pub static TIMERS_INIT_STATE: AtomicU8 = AtomicU8::new(0);

pub fn init() {
    pit::init();
    TIMERS_INIT_STATE.store(1, Ordering::Relaxed);
    kvm::init();
    TIMERS_INIT_STATE.store(2, Ordering::Relaxed);
    tsc::init();
    TIMERS_INIT_STATE.store(3, Ordering::Relaxed);
    crate::arch::system::lapic::init();
    TIMERS_INIT_STATE.store(4, Ordering::Relaxed);
}

pub fn init_hpet() {
    hpet::init();
    TIMERS_INIT_STATE.store(5, Ordering::Relaxed);
}

#[inline(always)]
pub fn preferred_timer_ms() -> u64 {
    preferred_timer_ns() / 1_000_000
}

pub fn preferred_timer_ns() -> u64 {
    unsafe {
        let kvm = crate::arch::drivers::time::kvm::KVM_TIMER
            .get()
            .map(|t| t as &dyn KernelTimer);
        let tsc = crate::arch::drivers::time::tsc::TSC_TIMER
            .get()
            .map(|t| t as &dyn KernelTimer);
        let hpet = crate::arch::drivers::time::hpet::HPET_TIMER
            .get()
            .map(|t| t as &dyn KernelTimer);

        let timers: [&Option<&dyn KernelTimer>; 3] = match TIMERS_INIT_STATE.load(Ordering::Relaxed)
        {
            5 => [&kvm, &tsc, &hpet],
            3..=4 => [&kvm, &tsc, &None],
            2 => [&kvm, &None, &None],
            _ => [&None, &None, &None],
        };

        for timer in timers.into_iter().flatten() {
            if timer.is_supported() {
                return timer.elapsed_ns();
            }
        }

        self::pit::current_pit_ticks() * 1_000_000
    }
}

#[inline(always)]
pub fn preferred_timer_pretty(digits: u32) -> String {
    crate::utils::time::elapsed_time_pretty(preferred_timer_ns(), digits)
}
