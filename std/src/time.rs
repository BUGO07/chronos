/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerKind {
    KVM,
    TSC,
    HPET,
    PIT,
}

impl From<TimerKind> for &'static str {
    fn from(val: TimerKind) -> Self {
        match val {
            TimerKind::KVM => "kvm",
            TimerKind::TSC => "tsc",
            TimerKind::HPET => "hpet",
            TimerKind::PIT => "pit",
        }
    }
}

impl From<&'static str> for TimerKind {
    fn from(val: &'static str) -> Self {
        match val {
            "kvm" => TimerKind::KVM,
            "tsc" => TimerKind::TSC,
            "hpet" => TimerKind::HPET,
            "pit" => TimerKind::PIT,
            _ => TimerKind::PIT,
        }
    }
}

impl From<TimerKind> for u64 {
    fn from(val: TimerKind) -> Self {
        match val {
            TimerKind::KVM => 0,
            TimerKind::TSC => 1,
            TimerKind::HPET => 2,
            TimerKind::PIT => 3,
        }
    }
}

impl From<u64> for TimerKind {
    fn from(val: u64) -> Self {
        match val {
            0 => TimerKind::KVM,
            1 => TimerKind::TSC,
            2 => TimerKind::HPET,
            3 => TimerKind::PIT,
            _ => TimerKind::PIT,
        }
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

pub fn preferred_timer_ns() -> u64 {
    #[cfg(feature = "kernel")]
    return crate::kernel::time::preferred_timer_ns();
    #[cfg(not(feature = "kernel"))]
    return crate::elf::time_ns();
}

#[inline(always)]
pub fn busywait_ns(ns: u64) {
    let start = preferred_timer_ns();
    while preferred_timer_ns() - start < ns {
        core::hint::spin_loop();
    }
}

#[inline(always)]
pub fn busywait_ms(ms: u64) {
    busywait_ns(ms * 1_000_000);
}

pub fn is_leap_year(year: u16) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
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
