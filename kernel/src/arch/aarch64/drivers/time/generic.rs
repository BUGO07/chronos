/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::cell::OnceCell;

use crate::{
    info,
    utils::{
        asm::regs::{get_cntfrq, get_cntpct},
        time::KernelTimer,
    },
};

pub static mut GENERIC_TIMER: OnceCell<GenericTimer> = OnceCell::new();

pub struct GenericTimer {
    start: u64,
    tickrate: u64,
    supported: bool,
}

impl GenericTimer {
    pub fn start(tickrate: u64) -> Self {
        GenericTimer {
            start: get_cntpct(),
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
            get_cntpct() - self.start
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
        0
    }
}

pub fn init() {
    info!("initializing generic timer...");
    let tickrate = get_cntfrq();
    info!("tickrate - {}hz", tickrate);
    unsafe {
        GENERIC_TIMER.set(GenericTimer::start(tickrate)).ok();
    }
    info!("done");
}
