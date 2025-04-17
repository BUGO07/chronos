/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use x86_64::instructions::port::Port;

use crate::info;

pub const PIT_FREQUENCY: u32 = 1193182;

pub fn init() {
    info!("setting up with frequency 1000hz");
    unsafe {
        Port::new(0x43).write(0b00110100u8);
        Port::new(0x40).write((PIT_FREQUENCY / 1000) & 0xFF);
        Port::new(0x40).write((PIT_FREQUENCY / 1000) >> 8);
    }
}
