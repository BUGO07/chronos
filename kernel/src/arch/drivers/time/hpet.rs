/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::{format, string::String};
use spin::Mutex;

use crate::memory::{
    get_hhdm_offset,
    vmm::{flag, page_size},
};

const HPET_BASE: u64 = 0xFED00000;
static mut HPET_TICKRATE: u64 = 0;

lazy_static::lazy_static! {
    pub static ref HPET_TIMER: Mutex<HpetTimer> = Mutex::new(HpetTimer::start());
}

pub struct HpetTimer {
    start: u64,
}

impl HpetTimer {
    pub fn start() -> Self {
        HpetTimer {
            start: hpet_read(0xF0),
        }
    }

    pub fn elapsed_cycles(&self) -> u64 {
        hpet_read(0xF0) - self.start
    }

    pub fn elapsed_ns(&self) -> u64 {
        self.elapsed_cycles() * 1_000_000_000 / unsafe { HPET_TICKRATE }
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

fn hpet_read(offset: u64) -> u64 {
    unsafe { *((HPET_BASE + get_hhdm_offset() + offset) as *const u64) }
}

fn hpet_write(offset: u64, value: u64) {
    unsafe { *((HPET_BASE + get_hhdm_offset() + offset) as *mut u64) = value }
}

pub fn init() {
    crate::memory::vmm::PAGEMAP.lock().map(
        HPET_BASE + get_hhdm_offset(),
        HPET_BASE,
        flag::PRESENT | flag::WRITE,
        page_size::SMALL,
    );

    let capabilities = hpet_read(0x000);
    let clock_period_fs = (capabilities >> 32) & 0xFFFFFFFF;

    let mut config = hpet_read(0x010);
    config |= 1;
    hpet_write(0x010, config);

    unsafe { HPET_TICKRATE = clock_period_fs as u64 * 10 };

    HPET_TIMER.lock();
}
