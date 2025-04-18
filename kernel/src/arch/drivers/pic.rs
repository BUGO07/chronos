/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use x86_64::instructions::port::Port;

use crate::{debug, info};

const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;
const ICW1_INIT: u8 = 0x10;

pub fn send_eoi() {
    unsafe {
        Port::new(0xA0).write(0x20u8);
        Port::new(0x20).write(0x20u8);
    }
}

pub fn unmask() {
    debug!("unmasking all irqs");
    unsafe {
        Port::new(0x21).write(0u8);
        Port::new(0xA1).write(0u8);
    }
}

pub fn disable() {
    debug!("masking all irqs");
    unsafe {
        Port::new(0x21).write(0xffu8);
        Port::new(0xA1).write(0xffu8);
    }
}

pub fn init() {
    info!("remapping pic");

    let mut master_command: Port<u8> = Port::new(0x20);
    let mut master_data: Port<u8> = Port::new(0x21);
    let mut slave_command: Port<u8> = Port::new(0xA0);
    let mut slave_data: Port<u8> = Port::new(0xA1);

    unsafe {
        let i1 = master_data.read();
        let i2 = slave_data.read();

        master_command.write(ICW1_INIT | ICW1_ICW4);
        slave_command.write(ICW1_INIT | ICW1_ICW4);

        master_data.write(0x20);
        slave_data.write(0x28);

        master_data.write(0x04);
        slave_data.write(0x02);

        master_data.write(ICW4_8086);
        slave_data.write(ICW4_8086);

        master_data.write(i1);
        slave_data.write(i2);
    }

    // limine masks all irqs by default
    unmask();

    debug!("enabling interrupts (sti)");
    x86_64::instructions::interrupts::enable();
}
