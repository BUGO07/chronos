use x86_64::instructions::{interrupts, port::Port};

use core::fmt::Write;

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    interrupts::without_interrupts(|| {
        let mut port: Port<u8> = Port::new(0x3f8);
        let mut buf = crate::utils::Buffer::new();
        write!(buf, "{}", args);

        for byte in buf.as_slice() {
            unsafe { port.write(*byte) };
        }
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
