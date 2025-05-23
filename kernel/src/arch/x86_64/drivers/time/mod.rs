/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::string::String;

use crate::{
    info,
    utils::{heapless::HeaplessVec, time::Timer},
};

pub mod hpet;
pub mod kvm;
pub mod pit;
pub mod rtc;
pub mod tsc;

pub static mut TIMERS: HeaplessVec<Timer, 4> = HeaplessVec::new();

pub fn init() {
    pit::init();
    kvm::init();
    tsc::init();
    crate::arch::system::lapic::init();
}

pub fn init_hpet() {
    hpet::init();
}

pub fn register_timer(timer: Timer) {
    info!("registering timer - {} [{}]", timer.name, timer.priority);
    unsafe { TIMERS.push(timer).ok() };
    get_timers().sort_by(|a, b| a.priority.cmp(&b.priority));
}

pub fn get_timers() -> &'static mut HeaplessVec<Timer, 4> {
    unsafe { &mut TIMERS }
}

pub fn get_timer(name: &str) -> &mut Timer {
    get_timers().iter_mut().find(|x| x.name == name).unwrap()
}

#[inline(always)]
pub fn preferred_timer_ms() -> u64 {
    preferred_timer_ns() / 1_000_000
}

pub fn preferred_timer_ns() -> u64 {
    for timer in get_timers().iter() {
        if timer.is_supported() {
            return (timer.elapsed_ns)(timer);
        }
    }

    self::pit::current_pit_ticks() * 1_000_000
}

#[inline(always)]
pub fn preferred_timer_pretty(digits: u32) -> String {
    crate::utils::time::elapsed_time_pretty(preferred_timer_ns(), digits)
}
