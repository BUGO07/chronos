/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{
    arch::interrupts::StackFrame,
    error, info,
    utils::asm::port::{inb, outb},
};

pub static mut MOUSE: Mouse = Mouse::new();

#[derive(Debug)]
pub struct Mouse {
    pub x: u16,
    pub y: u16,
    pub left: bool,
    pub right: bool,
    pub middle: bool,
}

impl Default for Mouse {
    fn default() -> Self {
        Self::new()
    }
}

impl Mouse {
    pub const fn new() -> Mouse {
        Mouse {
            x: 0,
            y: 0,
            left: false,
            right: false,
            middle: false,
        }
    }
}

pub fn init() {
    crate::utils::asm::without_ints(|| {
        info!("initializing ps/2 mouse...");

        wait_for_write();
        outb(COMMAND_PORT, 0xA8);

        wait_for_write();
        outb(COMMAND_PORT, 0x20);
        wait_for_read();
        let status = inb(DATA_PORT) | 0x02;
        wait_for_write();
        outb(COMMAND_PORT, 0x60);
        wait_for_write();
        outb(DATA_PORT, status);

        send_mouse_command(0xF6);
        expect_ack();

        send_mouse_command(0xF4);
        expect_ack();

        flush_data_port();

        crate::arch::interrupts::install_interrupt(0x2c, mouse_interrupt_handler);
        info!("done...");
    });
}

pub fn mouse_interrupt_handler(_stack_frame: *mut StackFrame) {
    unsafe {
        let status = inb(STATUS_PORT);

        if (status & 0x20) == 0 {
            crate::arch::interrupts::pic::send_eoi(12);
            return;
        }

        let byte = inb(DATA_PORT);

        if PACKET_INDEX == 0 && (byte & 0x08) == 0 {
            PACKET_INDEX = 0;
            crate::arch::interrupts::pic::send_eoi(12);
            return;
        }

        MOUSE_PACKET[PACKET_INDEX] = byte;
        PACKET_INDEX += 1;

        if PACKET_INDEX == 3 {
            PACKET_INDEX = 0;

            let status_byte = MOUSE_PACKET[0];
            let dx = MOUSE_PACKET[1] as i8;
            let dy = MOUSE_PACKET[2] as i8;

            MOUSE.left = (status_byte & 0x01) != 0;
            MOUSE.right = (status_byte & 0x02) != 0;
            MOUSE.middle = (status_byte & 0x04) != 0;

            MOUSE.x = ((MOUSE.x as i32 + dx as i32).clamp(0, u16::MAX as i32)) as u16;
            MOUSE.y = ((MOUSE.y as i32 - dy as i32).clamp(0, u16::MAX as i32)) as u16;
        }
        crate::arch::interrupts::pic::send_eoi(12);
    }
}

const DATA_PORT: u16 = 0x60;
const STATUS_PORT: u16 = 0x64;
const COMMAND_PORT: u16 = 0x64;

static mut MOUSE_PACKET: [u8; 3] = [0; 3];
static mut PACKET_INDEX: usize = 0;

fn send_mouse_command(command: u8) {
    wait_for_write();
    outb(COMMAND_PORT, 0xD4);
    wait_for_write();
    outb(DATA_PORT, command);
}

fn expect_ack() {
    let ack = read_mouse_data();
    if ack != 0xFA {
        error!("mouse did not acknowledge (got {:X})", ack);
    }
}

fn read_mouse_data() -> u8 {
    wait_for_read();
    inb(DATA_PORT)
}

fn wait_for_read() {
    while (inb(STATUS_PORT) & 0x01) == 0 {
        core::hint::spin_loop();
    }
}

fn wait_for_write() {
    while (inb(STATUS_PORT) & 0x02) != 0 {
        core::hint::spin_loop();
    }
}

fn flush_data_port() {
    for _ in 0..10 {
        if (inb(STATUS_PORT) & 0x01) != 0 {
            let _ = inb(DATA_PORT);
        }
    }
}
