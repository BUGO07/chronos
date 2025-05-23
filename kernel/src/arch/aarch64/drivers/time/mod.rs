/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::string::String;

use crate::{
    info,
    utils::{heapless::HeaplessVec, time::Timer},
};

pub mod generic;

pub static mut TIMERS: HeaplessVec<Timer, 1> = HeaplessVec::new();

pub fn early_init() {
    generic::init();
}

pub fn init() {} // TODO: implement more timers

pub fn register_timer(timer: Timer) {
    info!("registering timer - {} [{}]", timer.name, timer.priority);
    unsafe { TIMERS.push(timer).ok() };
    get_timers().sort_by(|a, b| a.priority.cmp(&b.priority));
}

pub fn get_timers() -> &'static mut HeaplessVec<Timer, 1> {
    unsafe { &mut TIMERS }
}

pub fn get_timer(name: &str) -> &mut Timer {
    get_timers().iter_mut().find(|x| x.name == name).unwrap()
}

pub fn preferred_timer_ns() -> u64 {
    for timer in get_timers().iter() {
        if timer.is_supported() {
            return (timer.elapsed_ns)(timer);
        }
    }

    0
}

#[inline(always)]
pub fn preferred_timer_ms() -> u64 {
    preferred_timer_ns() / 1_000_000
}

#[inline(always)]
pub fn preferred_timer_pretty(digits: u32) -> String {
    crate::utils::time::elapsed_time_pretty(preferred_timer_ns(), digits)
}
