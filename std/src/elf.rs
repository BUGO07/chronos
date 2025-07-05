/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::ffi::{CStr, c_char};

use alloc::{ffi::CString, string::String, vec::Vec};

use crate::{println, syscalls::*};

#[cfg(target_arch = "x86_64")]
pub fn _syscall(id: u64, arg0: u64, arg1: u64, arg2: u64, arg3: u64, arg4: u64, arg5: u64) -> u64 {
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "int $0x80",
            in("rax") id,
            in("rdi") arg0,
            in("rsi") arg1,
            in("rdx") arg2,
            in("r10") arg3,
            in("r8") arg4,
            in("r9") arg5,
            lateout("rax") ret,
            options(nomem, nostack, preserves_flags)
        );
    }
    ret
}

#[cfg(not(target_arch = "x86_64"))]
pub fn _syscall(
    _id: u64,
    _arg0: u64,
    _arg1: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
) -> u64 {
    0
}

#[macro_export]
macro_rules! syscall {
    ($id:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr, $arg5:expr) => {
        $crate::elf::_syscall($id, $arg0, $arg1, $arg2, $arg3, $arg4, $arg5)
    };
    ($id:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr, $arg4:expr) => {
        $crate::elf::_syscall($id, $arg0, $arg1, $arg2, $arg3, $arg4, 0)
    };
    ($id:expr, $arg0:expr, $arg1:expr, $arg2:expr, $arg3:expr) => {
        $crate::elf::_syscall($id, $arg0, $arg1, $arg2, $arg3, 0, 0)
    };
    ($id:expr, $arg0:expr, $arg1:expr, $arg2:expr) => {
        $crate::elf::_syscall($id, $arg0, $arg1, $arg2, 0, 0, 0)
    };
    ($id:expr, $arg0:expr, $arg1:expr) => {
        $crate::elf::_syscall($id, $arg0, $arg1, 0, 0, 0, 0)
    };
    ($id:expr, $arg0:expr) => {
        $crate::elf::_syscall($id, $arg0, 0, 0, 0, 0, 0)
    };
    ($id:expr) => {
        $crate::elf::_syscall($id, 0, 0, 0, 0, 0, 0)
    };
}

#[inline(always)]
pub fn read(fd: u64, buf: &mut [u8]) -> u64 {
    syscall!(READ, fd, buf.as_mut_ptr() as u64, buf.len() as u64)
}

#[inline(always)]
pub fn write(fd: u64, buf: &[u8]) -> u64 {
    syscall!(WRITE, fd, buf.as_ptr() as u64, buf.len() as u64)
}

#[inline(always)]
pub fn close(fd: u64) -> u64 {
    syscall!(CLOSE, fd)
}

#[inline(always)]
pub fn execve(path: &str, argv: &[&str], envp: &[&str]) -> u64 {
    let argv: Vec<CString> = argv.iter().map(|s| CString::new(*s).unwrap()).collect();
    let envp: Vec<CString> = envp.iter().map(|s| CString::new(*s).unwrap()).collect();
    syscall!(
        EXECVE,
        path.as_ptr() as u64,
        argv.as_ptr() as u64,
        envp.as_ptr() as u64
    )
}

#[inline(always)]
pub fn time_ns() -> u64 {
    syscall!(TIME)
}

#[inline(always)]
pub fn get_pid() -> u64 {
    syscall!(GETPID)
}

#[inline(always)]
pub fn get_tid() -> u64 {
    syscall!(GETTID)
}

#[inline(always)]
pub fn sleep_ns(ns: u64) {
    syscall!(NANOSLEEP, ns);
}

#[inline(always)]
pub fn spawn_process(name: &str) -> u64 {
    syscall!(303, name.as_ptr() as u64, name.len() as u64)
}

#[inline(always)]
pub fn spawn_thread(pid: u64, func: u64, name: &str, args: Vec<String>) -> u64 {
    let cstrs: Vec<CString> = args
        .iter()
        .map(|s| CString::new(s.as_bytes()).unwrap())
        .collect();
    let cstr_ptrs: Vec<*const c_char> = cstrs.iter().map(|s| s.as_ptr()).collect();

    syscall!(
        302,
        pid,
        func,
        name.as_ptr() as u64,
        name.len() as u64,
        cstr_ptrs.as_ptr() as u64,
        cstr_ptrs.len() as u64
    )
}

pub fn load_elf(name: &str) -> u64 {
    syscall!(307, name.as_ptr() as u64, name.len() as u64)
}

#[inline(always)]
pub fn printf(s: &str) {
    syscall!(WRITE, 1, s.as_ptr() as u64, s.len() as u64);
}

struct Writer;

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        printf(s);
        Ok(())
    }
}

pub fn _print(args: core::fmt::Arguments) {
    core::fmt::Write::write_fmt(&mut Writer, args).unwrap();
}

#[inline(always)]
pub fn exit(code: u64) -> ! {
    syscall!(EXIT, code);
    loop {
        core::hint::spin_loop();
    }
}

unsafe extern "Rust" {
    fn main(arg: Vec<&str>) -> u64;
}

#[unsafe(no_mangle)]
extern "C" fn _start(argc: u64, argv: *const *const c_char) -> ! {
    unsafe {
        let args = if argc != 0 && argv as u64 != 0 {
            core::slice::from_raw_parts(argv, argc as usize)
                .iter()
                .map(|x| CStr::from_ptr(*x).to_str().unwrap())
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        exit(main(args))
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    println!(
        "panicked on {}/{} with message: {}",
        get_pid(),
        get_tid(),
        _info.message()
    );
    exit(1);
}
