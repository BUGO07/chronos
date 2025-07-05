/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::string::String;

use crate::{
    heapless::HeaplessVec,
    info,
    time::{TimerKind, elapsed_time_pretty},
};

pub static mut TIMERS: HeaplessVec<Timer, 10> = HeaplessVec::new();

pub struct Timer {
    pub kind: TimerKind,
    pub start: u64,
    pub frequency: u64,
    pub supported: bool,
    pub priority: u8,
    pub elapsed_ns: fn(&Self) -> u64,
    pub offset: u64,
}

impl Timer {
    pub fn new(
        kind: TimerKind,
        start: u64,
        frequency: u64,
        supported: bool,
        priority: u8,
        elapsed_ns: fn(&Self) -> u64,
        offset: u64,
    ) -> Self {
        Self {
            kind,
            start,
            frequency,
            supported,
            priority,
            elapsed_ns,
            offset,
        }
    }
    pub fn kind(&self) -> &TimerKind {
        &self.kind
    }
    pub fn name(&self) -> &'static str {
        self.kind.into()
    }
    pub fn is_supported(&self) -> bool {
        self.supported
    }
    pub fn priority(&self) -> u8 {
        self.priority
    } // unused as of now
    pub fn get_offset(&self) -> u64 {
        self.offset
    }
    pub fn set_offset(&mut self, offset: u64) {
        self.offset = offset;
    }
    pub fn elapsed(&self) -> u64 {
        (self.elapsed_ns)(self)
    }
    pub fn elapsed_pretty(&self, digits: u32) -> alloc::string::String {
        elapsed_time_pretty((self.elapsed_ns)(self), digits)
    }
}

pub fn register_timer(timer: Timer) {
    info!("registering timer - {} [{}]", timer.name(), timer.priority);
    unsafe { TIMERS.push(timer).ok() };
    get_timers().sort_by(|a, b| a.priority.cmp(&b.priority));
}

pub fn get_timers() -> &'static mut HeaplessVec<Timer, 10> {
    unsafe { &mut TIMERS }
}

pub fn get_timer(kind: &TimerKind) -> &mut Timer {
    get_timers().iter_mut().find(|x| x.kind() == kind).unwrap()
}

pub fn get_timer_by_name(name: &str) -> &mut Timer {
    get_timers().iter_mut().find(|x| x.name() == name).unwrap()
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

    0
}

#[inline(always)]
pub fn preferred_timer_pretty(digits: u32) -> String {
    crate::time::elapsed_time_pretty(preferred_timer_ns(), digits)
}
