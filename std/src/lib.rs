/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

#![no_std]
#![allow(
    static_mut_refs,
    clippy::not_unsafe_ptr_arg_deref,
    clippy::missing_safety_doc
)]

extern crate alloc;

pub mod asm;
pub mod fs;
pub mod heapless;
pub mod memory;
pub mod spinlock;
pub mod syscalls;
pub mod time;

#[cfg(not(feature = "kernel"))]
pub mod elf;

#[cfg(feature = "profiling")]
pub use embedded_profiling;

pub use lazy_static::lazy_static;

pub const NOOO: &str = include_str!("../res/nooo.txt");

#[inline(always)]
pub const fn align_up(addr: u64, align: u64) -> u64 {
    assert!(align.is_power_of_two());
    (addr + align - 1) & !(align - 1)
}

#[inline(always)]
pub const fn align_down(addr: u64, align: u64) -> u64 {
    assert!(align.is_power_of_two());
    addr & !(align - 1)
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

pub fn _print(args: core::fmt::Arguments) {
    #[cfg(feature = "kernel")]
    crate::kernel::term::_print(args);
    #[cfg(not(feature = "kernel"))]
    crate::elf::_print(args);
}

#[macro_export]
macro_rules! print_fill {
    ($what:expr) => {
        $crate::_print_fill($what, "", true)
    };
    ($what:expr, $with:expr) => {
        $crate::_print_fill($what, $with, true)
    };
    ($what:expr, $with:expr, $newline:expr) => {
        $crate::_print_fill($what, $with, $newline)
    };
}

#[macro_export]
macro_rules! print_centered {
    ($what:expr) => {
        $crate::_print_centered($what, "", true)
    };
    ($what:expr, $with:expr) => {
        $crate::_print_centered($what, $with, true)
    };
    ($what:expr, $with:expr, $newline:expr) => {
        $crate::_print_centered($what, $with, $newline)
    };
}

pub fn _print_fill(_what: &str, _with: &str, _newline: bool) {
    #[cfg(feature = "kernel")]
    crate::kernel::term::_print_fill(_what, _with, _newline);
    // #[cfg(feature = "elf")]
    // crate::elf::_print_fill(what, with, newline);
}
pub fn _print_centered(_what: &str, _with: &str, _newline: bool) {
    #[cfg(feature = "kernel")]
    crate::kernel::term::_print_centered(_what, _with, _newline);
    // #[cfg(feature = "elf")]
    // crate::elf::_print_centered(what, with, newline);
}

#[repr(C, packed)]
#[derive(Default, Clone, Copy, PartialEq, PartialOrd)]
pub struct StackFrame {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub vector: u64,
    pub ec: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

impl core::fmt::Debug for StackFrame {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        macro_rules! field {
            ($name:ident) => {
                let value = self.$name;
                let reg = stringify!($name);
                write!(f, "{}:{}0x{:016x}  ", reg, " ".repeat(7 - reg.len()), value)?
            };
        }

        field!(r15);
        field!(r14);
        writeln!(f)?;
        field!(r13);
        field!(r12);
        writeln!(f)?;
        field!(r11);
        field!(r10);
        writeln!(f)?;
        field!(r9);
        field!(r8);
        writeln!(f)?;
        field!(rbp);
        field!(rdi);
        writeln!(f)?;
        field!(rsi);
        field!(rdx);
        writeln!(f)?;
        field!(rcx);
        field!(rbx);
        writeln!(f)?;
        field!(rax);
        field!(vector);
        writeln!(f)?;
        field!(ec);
        field!(rip);
        writeln!(f)?;
        field!(cs);
        field!(rflags);
        writeln!(f)?;
        field!(rsp);
        field!(ss);
        Ok(())
    }
}

#[cfg(feature = "profiling")]
lazy_static::lazy_static! {
    pub static ref PROFILER: usize = {
        unsafe {
        embedded_profiling::set_profiler(&Profiler).unwrap();
    }
        0
    };
}

#[cfg(feature = "profiling")]
struct Profiler;

#[cfg(feature = "profiling")]
impl EmbeddedProfiler for Profiler {
    fn read_clock(&self) -> embedded_profiling::EPInstant {
        #[cfg(feature = "kernel")]
        {
            embedded_profiling::EPInstant::from_ticks(preferred_timer_ns() / 1000)
        }
        #[cfg(not(feature = "kernel"))]
        {
            embedded_profiling::EPInstant::from_ticks(0)
        }
    }
    fn log_snapshot(&self, _snapshot: &embedded_profiling::EPSnapshot) {
        println!("snapshot - {_snapshot}");
    }
}
