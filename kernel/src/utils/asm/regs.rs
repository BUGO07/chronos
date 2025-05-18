/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::arch::asm;

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn rdmsr(msr: u32) -> u64 {
    let high: u32;
    let low: u32;
    unsafe {
        asm!(
            "rdmsr",
            in("ecx") msr,
            out("edx") high,
            out("eax") low,
        );
    }
    (high as u64) << 32 | low as u64
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn wrmsr(msr: u32, value: u64) {
    let high = (value >> 32) as u32;
    let low = value as u32;
    unsafe {
        asm!(
            "wrmsr",
            in("ecx") msr,
            in("edx") high,
            in("eax") low,
        );
    }
}

#[inline(always)]
#[cfg(target_arch = "aarch64")]
pub fn get_cntfrq() -> u64 {
    let freq: u64;
    unsafe {
        asm!("mrs {}, cntfrq_el0", out(reg) freq);
    }
    freq
}

#[inline(always)]
#[cfg(target_arch = "aarch64")]
pub fn get_cntpct() -> u64 {
    let cnt: u64;
    unsafe {
        asm!("mrs {}, cntpct_el0", out(reg) cnt);
    }
    cnt
}
