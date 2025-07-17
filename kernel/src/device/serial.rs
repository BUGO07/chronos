/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::fmt::Write;

use crate::utils::asm::port::{inb, outb};

const COM1_BASE: u16 = 0x3F8;
const COM1_DATA: u16 = COM1_BASE;
const COM1_INTERRUPT_ENABLE: u16 = COM1_BASE + 1;
const COM1_LINE_CONTROL: u16 = COM1_BASE + 3;
const COM1_FIFO_CONTROL: u16 = COM1_BASE + 2;
const COM1_MODEM_CONTROL: u16 = COM1_BASE + 4;
const COM1_LINE_STATUS: u16 = COM1_BASE + 5;

pub fn init() {
    outb(COM1_INTERRUPT_ENABLE, 0x00);
    outb(COM1_LINE_CONTROL, 0x80);
    outb(COM1_DATA, 0x03);
    outb(COM1_INTERRUPT_ENABLE, 0x00);
    outb(COM1_LINE_CONTROL, 0x03);
    outb(COM1_FIFO_CONTROL, 0xC7);
    outb(COM1_MODEM_CONTROL, 0x0B);
}

pub fn serial_write(byte: u8) {
    while (inb(COM1_LINE_STATUS) & 0x20) == 0 {}
    outb(COM1_DATA, byte);
}

pub fn serial_read() -> u8 {
    while (inb(COM1_LINE_STATUS) & 1) == 0 {}
    inb(COM1_DATA) // garbage read on real hardware
}

// pub fn serial_thread() -> ! {
//     loop {
//         unsafe {
//             let mut input = crate::device::serial::serial_read(); // chat should i keep ts?
//             // backspace is interpreted as del on serial for some reason
//             if input == 0x7F {
//                 input = 0x8; // backspace
//             }
//             if let Some(shell) = crate::utils::shell::SHELL.get_mut() {
//                 shell.event_queue.push_back((
//                     pc_keyboard::DecodedKey::Unicode(input as char),
//                     crate::arch::drivers::keyboard::KEYBOARD_STATE.get_mut(),
//                 ));
//             } else {
//                 crate::warn!("shell not initialized");
//             }
//         }
//     }
// }

pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        if s != "\x1b[6n" {
            for byte in s.bytes() {
                serial_write(byte);
            }
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    write!(SerialWriter, "{args}").ok();
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::device::serial::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
