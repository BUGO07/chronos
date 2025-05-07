/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::arch::asm;

#[cfg(target_arch = "x86_64")]
pub mod mem;
#[cfg(target_arch = "x86_64")]
pub mod port;

pub mod mmio;
pub mod regs;

#[inline(always)]
pub fn halt() {
    unsafe {
        #[cfg(target_arch = "aarch64")]
        asm!("wfi");
        #[cfg(target_arch = "x86_64")]
        asm!("hlt");
    }
}

#[inline(always)]
pub fn halt_loop() -> ! {
    loop {
        halt();
    }
}

#[inline(always)]
pub fn halt_no_ints() {
    toggle_ints(false);
    halt();
}

#[inline(always)]
pub fn halt_with_ints() {
    toggle_ints(true);
    halt();
}

#[inline(always)]
pub fn toggle_ints(val: bool) {
    unsafe {
        #[cfg(target_arch = "aarch64")]
        if val {
            asm!("msr daifclr, #0b1111",);
        } else {
            asm!("msr daifset, #0b1111",);
        }
        #[cfg(target_arch = "x86_64")]
        if val {
            asm!("sti");
        } else {
            asm!("cli");
        }
    }
}

#[inline(always)]
pub fn int_status() -> bool {
    let r: u64;
    unsafe {
        #[cfg(target_arch = "aarch64")]
        asm!("mrs {}, daif", out(reg) r);
        #[cfg(target_arch = "x86_64")]
        asm!("pushfq; pop {}", out(reg) r);
    }
    #[cfg(target_arch = "aarch64")]
    return (r >> 7) & 1 != 0; // IRQs
    #[cfg(target_arch = "x86_64")]
    return (r & (1 << 9)) != 0;
}

#[inline(always)]
pub fn without_ints<F, R>(closure: F) -> R
where
    F: FnOnce() -> R,
{
    let enabled = int_status();
    if enabled {
        toggle_ints(false);
    }
    let ret = closure();
    if enabled {
        toggle_ints(true);
    }
    ret
}

#[cfg(target_arch = "x86_64")]
pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn _cpuid_count(leaf: u32, sub_leaf: u32) -> CpuidResult {
    let eax;
    let ebx;
    let ecx;
    let edx;

    unsafe {
        asm!(
            "mov {0:r}, rbx",
            "cpuid",
            "xchg {0:r}, rbx",
            out(reg) ebx,
            inout("eax") leaf => eax,
            inout("ecx") sub_leaf => ecx,
            out("edx") edx,
            options(nostack, preserves_flags),
        );
    }
    CpuidResult { eax, ebx, ecx, edx }
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn _cpuid(leaf: u32) -> CpuidResult {
    _cpuid_count(leaf, 0)
}

#[cfg(target_arch = "x86_64")]
#[inline(always)]
pub fn _rdtsc() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags),
        );
    }
    ((high as u64) << 32) | (low as u64)
}
