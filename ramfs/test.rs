#![no_std]
#![no_main]

use core::{arch::asm, fmt::Write};

macro_rules! syscall {
    ($id:expr) => {{
        let ret: u64;
        unsafe {
            asm!(
                "int 0x80",
                in("rax") $id as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
    ($id:expr, $a0:expr) => {{
        let ret: u64;
        unsafe {
            asm!(
                "int 0x80",
                in("rax") $id as u64,
                in("rdi") $a0 as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
    ($id:expr, $a0:expr, $a1:expr) => {{
        let ret: u64;
        unsafe {
            asm!(
                "int 0x80",
                in("rax") $id as u64,
                in("rdi") $a0 as u64,
                in("rsi") $a1 as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
    ($id:expr, $a0:expr, $a1:expr, $a2:expr) => {{
        let ret: u64;
        unsafe {
            asm!(
                "int 0x80",
                in("rax") $id as u64,
                in("rdi") $a0 as u64,
                in("rsi") $a1 as u64,
                in("rdx") $a2 as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
    ($id:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr) => {{
        let ret: u64;
        unsafe {
            asm!(
                "int 0x80",
                in("rax") $id as u64,
                in("rdi") $a0 as u64,
                in("rsi") $a1 as u64,
                in("rdx") $a2 as u64,
                in("r10") $a3 as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
    ($id:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {{
        let ret: u64;
        unsafe {
            asm!(
                "int 0x80",
                in("rax") $id as u64,
                in("rdi") $a0 as u64,
                in("rsi") $a1 as u64,
                in("rdx") $a2 as u64,
                in("r10") $a3 as u64,
                in("r8")  $a4 as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
    ($id:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr) => {{
        let ret: u64;
        unsafe {
            asm!(
                "int 0x80",
                in("rax") $id as u64,
                in("rdi") $a0 as u64,
                in("rsi") $a1 as u64,
                in("rdx") $a2 as u64,
                in("r10") $a3 as u64,
                in("r8")  $a4 as u64,
                in("r9")  $a5 as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
}

#[inline(always)]
fn exit(code: u64) -> ! {
    syscall!(60, code);
    sched_yield();
    unsafe { core::hint::unreachable_unchecked() }
}

#[inline(always)]
fn sleep(ns: u64) {
    syscall!(35, ns);
}

#[inline(always)]
fn sleep_us(us: u64) {
    sleep(us * 1_000);
}

#[inline(always)]
fn sleep_ms(ms: u64) {
    sleep(ms * 1_000_000);
}

#[inline(always)]
fn sched_yield() {
    syscall!(24);
}

pub struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        syscall!(1, 1, s.as_ptr(), s.len());
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

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Hello from the test process!");
    sleep(3_000_000_000);
    println!("Goodbye from the test process!");
    panic!("This is a test panic!");
    exit(0);
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    exit(1);
}
