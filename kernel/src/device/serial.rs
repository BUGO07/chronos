/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::fmt::Write;

#[cfg(target_arch = "x86_64")]
use crate::utils::asm::port::{inb, outb};

#[cfg(target_arch = "x86_64")]
const COM1_BASE: u16 = 0x3F8;
#[cfg(target_arch = "x86_64")]
const COM1_DATA: u16 = COM1_BASE;
#[cfg(target_arch = "x86_64")]
const COM1_INTERRUPT_ENABLE: u16 = COM1_BASE + 1;
#[cfg(target_arch = "x86_64")]
const COM1_LINE_CONTROL: u16 = COM1_BASE + 3;
#[cfg(target_arch = "x86_64")]
const COM1_FIFO_CONTROL: u16 = COM1_BASE + 2;
#[cfg(target_arch = "x86_64")]
const COM1_MODEM_CONTROL: u16 = COM1_BASE + 4;
#[cfg(target_arch = "x86_64")]
const COM1_LINE_STATUS: u16 = COM1_BASE + 5;

#[cfg(target_arch = "aarch64")]
const UART0_BASE: usize = 0x0900_0000;
#[cfg(target_arch = "aarch64")]
const UART0_DR: *mut u32 = UART0_BASE as *mut u32;
#[cfg(target_arch = "aarch64")]
const UART0_FR: *const u32 = (UART0_BASE + 0x18) as *const u32;
#[cfg(target_arch = "aarch64")]
const UART0_CR: *mut u32 = (UART0_BASE + 0x30) as *mut u32;
#[cfg(target_arch = "aarch64")]
const UART0_IBRD: *mut u32 = (UART0_BASE + 0x24) as *mut u32;
#[cfg(target_arch = "aarch64")]
const UART0_FBRD: *mut u32 = (UART0_BASE + 0x28) as *mut u32;
#[cfg(target_arch = "aarch64")]
const UART0_LCRH: *mut u32 = (UART0_BASE + 0x2C) as *mut u32;

pub fn init() {
    #[cfg(target_arch = "x86_64")]
    {
        outb(COM1_INTERRUPT_ENABLE, 0x00);
        outb(COM1_LINE_CONTROL, 0x80);
        outb(COM1_DATA, 0x03);
        outb(COM1_INTERRUPT_ENABLE, 0x00);
        outb(COM1_LINE_CONTROL, 0x03);
        outb(COM1_FIFO_CONTROL, 0xC7);
        outb(COM1_MODEM_CONTROL, 0x0B);
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        UART0_CR.write_volatile(0);
        UART0_IBRD.write_volatile(1);
        UART0_FBRD.write_volatile(40);
        UART0_LCRH.write_volatile(3 << 5);
        UART0_CR.write_volatile((1 << 0) | (1 << 8) | (1 << 9));
    }
}

pub fn serial_write(byte: u8) {
    #[cfg(target_arch = "x86_64")]
    {
        while (inb(COM1_LINE_STATUS) & 0x20) == 0 {}
        outb(COM1_DATA, byte);
    }
    #[cfg(target_arch = "aarch64")]
    {
        unsafe {
            while UART0_FR.read_volatile() & (1 << 5) != 0 {}
            UART0_DR.write_volatile(byte as u32);
        }
    }
}

pub fn serial_read() -> u8 {
    #[cfg(target_arch = "x86_64")]
    {
        while (inb(COM1_LINE_STATUS) & 1) == 0 {}
        inb(COM1_DATA)
    }
    #[cfg(target_arch = "aarch64")]
    {
        unsafe {
            while UART0_FR.read_volatile() & (1 << 4) != 0 {}
            UART0_DR.read_volatile() as u8
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub fn serial_thread() -> ! {
    loop {
        unsafe {
            let mut input = crate::device::serial::serial_read(); // chat should i keep ts?
            // backspace is interpreted as del on serial for some reason
            if input == 0x7F {
                input = 0x8; // backspace
            }
            if let Some(shell) = crate::utils::shell::SHELL.get_mut() {
                shell.event_queue.push_back((
                    pc_keyboard::DecodedKey::Unicode(input as char),
                    crate::arch::drivers::keyboard::KEYBOARD_STATE.get_mut(),
                ));
            } else {
                crate::warn!("shell not initialized");
            }
        }
        crate::utils::asm::halt();
    }
}

#[cfg(target_arch = "aarch64")]
pub async fn serial_task() {
    loop {
        unsafe {
            let mut input = crate::device::serial::serial_read(); // chat should i keep ts?
            // backspace is interpreted as del on serial for some reason
            if input == 0x7F {
                input = 0x8; // backspace
            }
            if let Some(shell) = crate::utils::shell::SHELL.get_mut() {
                shell.key_event(input as char);
            }
        }
    }
}

pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.bytes() {
            serial_write(byte);
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
