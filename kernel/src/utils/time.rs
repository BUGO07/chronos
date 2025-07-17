/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

pub struct Timer {
    pub name: &'static str,
    pub start: u64,
    pub frequency: u64,
    pub supported: bool,
    pub priority: u8,
    pub elapsed_ns: fn(&Self) -> u64,
    pub offset: u64,
}

impl Timer {
    pub fn new(
        name: &'static str,
        start: u64,
        frequency: u64,
        supported: bool,
        priority: u8,
        elapsed_ns: fn(&Self) -> u64,
        offset: u64,
    ) -> Self {
        Self {
            name,
            start,
            frequency,
            supported,
            priority,
            elapsed_ns,
            offset,
        }
    }
    pub fn name(&self) -> &'static str {
        self.name
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
    pub fn elapsed_pretty(&self, digits: u32) -> alloc::string::String {
        elapsed_time_pretty((self.elapsed_ns)(self), digits)
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

pub fn is_leap_year(year: u16) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}

pub fn days_in_month(year: u16, month: u8) -> u32 {
    match month {
        1 => 31,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        3 => 31,
        4 => 30,
        5 => 31,
        6 => 30,
        7 => 31,
        8 => 31,
        9 => 30,
        10 => 31,
        11 => 30,
        12 => 31,
        _ => 0,
    }
}
