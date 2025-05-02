/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::cell::OnceCell;

use aarch64::regs::{CNTFRQ_EL0, CNTPCT_EL0, Readable};

use crate::info;

use super::KernelTimer;

pub static mut GENERIC_TIMER: OnceCell<GenericTimer> = OnceCell::new();

pub struct GenericTimer {
    start: u64,
    tickrate: u64,
    supported: bool,
}

impl GenericTimer {
    pub fn start(tickrate: u64) -> Self {
        GenericTimer {
            start: CNTPCT_EL0.get(),
            tickrate,
            supported: true,
        }
    }

    pub fn unsupported() -> Self {
        GenericTimer {
            start: 0,
            tickrate: 0,
            supported: false,
        }
    }

    pub fn elapsed_cycles(&self) -> u64 {
        if self.supported {
            CNTPCT_EL0.get() - self.start
        } else {
            0
        }
    }
}

impl KernelTimer for GenericTimer {
    fn is_supported(&self) -> bool {
        self.supported
    }

    fn elapsed_ns(&self) -> u64 {
        if self.supported {
            (self.elapsed_cycles() as u128 * 1_000_000_000 / self.tickrate as u128) as u64
        } else {
            0
        }
    }

    fn name(&self) -> &'static str {
        "GENERIC"
    }

    fn priority(&self) -> u8 {
        20
    }
}

pub fn init() {
    info!("initializing generic timer...");
    let tickrate = CNTFRQ_EL0.get();
    info!("tickrate - {}hz", tickrate);
    unsafe {
        GENERIC_TIMER
            .set(GenericTimer::start(CNTFRQ_EL0.get()))
            .ok();
    }
    info!("done");
}
