#![no_std]
#![no_main]
#![allow(clippy::macro_metavars_in_unsafe)]

use core::fmt::Write;

use crate::syscalls::{
    sleep_ms, sys_close, sys_exit, sys_get_cwd, sys_mkdir, sys_open, sys_read, sys_write,
};

pub mod syscalls;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    let fd = sys_open(c"/src/main.rs".as_ptr() as _, 0, 0);
    if fd < 0 {
        println!("Failed to open /src/main.rs: error code {}", fd);
        sys_exit(1);
    }
    println!("Opened main.rs with fd {}", fd);
    let buf = [0u8; 128];
    let n = sys_read(fd, buf.as_ptr() as _, buf.len());
    if n < 0 {
        println!("Failed to read main.rs: error code {}", n);
        sys_exit(1);
    }
    println!(
        "Read {} bytes from main.rs:\n{}",
        n,
        core::str::from_utf8(&buf[..n as usize]).unwrap_or("<invalid utf-8>")
    );
    sys_mkdir(c"/tmp".as_ptr() as _);
    println!("Created directory /tmp");
    let cwd_buf = [0u8; 128];
    let cwd_n = sys_get_cwd(cwd_buf.as_ptr() as _, cwd_buf.len());
    if cwd_n < 0 {
        println!(
            "Failed to get current working directory: error code {}",
            cwd_n
        );
        sys_exit(1);
    }
    println!(
        "Current working directory: {}",
        core::str::from_utf8(&cwd_buf[..cwd_n as usize]).unwrap_or("<invalid utf-8>")
    );

    sys_close(fd);
    println!("Hello from the test process!");
    sleep_ms(1000);
    println!("Goodbye from the test process!");
    panic!("This is a test panic!");
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    sys_exit(1);
}

pub struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        sys_write(1, s.as_ptr(), s.len());
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    write!(Writer, "{args}").ok();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(
        concat!($fmt, "\n"), $($arg)*));
}
