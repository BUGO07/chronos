/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{
    info,
    utils::{
        asm::regs::{get_cntfrq, get_cntpct},
        time::Timer,
    },
};

pub fn init() {
    let tickrate = get_cntfrq();
    info!("setting up at {tickrate}hz...");
    super::register_timer(Timer::new(
        "Generic",
        get_cntpct(),
        tickrate,
        true,
        0,
        |timer: &Timer| {
            ((get_cntpct() - timer.start) as u128 * 1_000_000_000 / timer.frequency as u128) as u64
        },
        0,
    ));
    info!("done");
}
