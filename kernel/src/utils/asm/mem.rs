/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::ffi::{c_int, c_void};

#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    unsafe {
        core::arch::asm!(
            "rep movsb",
            inout("rdi") dest => _,
            inout("rsi") src => _,
            inout("rcx") n => _,
            options(nostack, preserves_flags)
        );
        dest
    }
}

#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub extern "C" fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    unsafe {
        let mut i = 0;
        let dest_ptr = dest as *mut u8;
        let src_ptr = src as *const u8;
        while i < n {
            *dest_ptr.add(i) = *src_ptr.add(i);
            i += 1;
        }
        dest
    }
}

#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub extern "C" fn memset(dest: *mut c_void, val: c_int, n: usize) -> *mut c_void {
    unsafe {
        core::arch::asm!(
            "rep stosb",
            inout("rdi") dest => _,
            in("al") val as u8,
            inout("rcx") n => _,
            options(nostack, preserves_flags)
        );
        dest
    }
}

#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub extern "C" fn memset(dest: *mut c_void, val: c_int, n: usize) -> *mut c_void {
    unsafe {
        let mut i = 0;
        let dest_ptr = dest as *mut u8;
        let value = val as u8;
        while i < n {
            *dest_ptr.add(i) = value;
            i += 1;
        }
        dest
    }
}

#[cfg(target_arch = "x86_64")]
#[unsafe(no_mangle)]
pub extern "C" fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    unsafe {
        let d = dest as usize;
        let s = src as usize;
        if d < s || d >= (s + n) {
            memcpy(dest, src, n)
        } else {
            core::arch::asm!(
                "std",
                "add rsi, rcx",
                "add rdi, rcx",
                "dec rsi",
                "dec rdi",
                "rep movsb",
                "cld",
                inout("rdi") dest => _,
                inout("rsi") src => _,
                inout("rcx") n => _,
                options(nostack, preserves_flags)
            );
            dest
        }
    }
}

#[cfg(target_arch = "aarch64")]
#[unsafe(no_mangle)]
pub extern "C" fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    unsafe {
        let d = dest as usize;
        let s = src as usize;
        if d < s || d >= (s + n) {
            memcpy(dest, src, n)
        } else {
            let mut i = n;
            let dest_ptr = dest as *mut u8;
            let src_ptr = src as *const u8;
            while i > 0 {
                i -= 1;
                *dest_ptr.add(i) = *src_ptr.add(i);
            }
            dest
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int {
    let mut i = 0;
    while i < n {
        let va = unsafe { *(a.add(i) as *const u8) };
        let vb = unsafe { *(b.add(i) as *const u8) };
        if va != vb {
            return va as c_int - vb as c_int;
        }
        i += 1;
    }
    0
}
