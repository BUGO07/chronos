/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use x86_64::instructions::{interrupts, port::Port};

use core::fmt::Write;

pub struct SerialWriter;

impl Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for byte in s.chars() {
            unsafe { Port::new(0xe9).write(byte as u8) };
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    interrupts::without_interrupts(|| {
        write!(SerialWriter, "{}", args).ok();
    });
}

#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::arch::device::serial::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}
