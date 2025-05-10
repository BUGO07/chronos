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
#[cfg(target_arch = "x86_64")]
pub fn get_cs_reg() -> u16 {
    let register: u16;
    unsafe {
        asm!("mov {0:x}, cs", out(reg) register, options(nomem, nostack, preserves_flags));
    }
    register
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn get_ss_reg() -> u16 {
    let register: u16;
    unsafe {
        asm!("mov {0:x}, ss", out(reg) register, options(nomem, nostack, preserves_flags));
    }
    register
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn set_cs_reg(selector: u16) {
    unsafe {
        asm!(
            "push {sel}",
            "lea {tmp}, [55f + rip]",
            "push {tmp}",
            "retfq",
            "55:",
            sel = in(reg) u64::from(selector),
            tmp = lateout(reg) _,
            options(preserves_flags),
        );
    }
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn set_ds_reg(selector: u16) {
    unsafe {
        asm!("mov ds, {0:x}", in(reg) selector, options(nostack, preserves_flags));
    }
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn set_es_reg(selector: u16) {
    unsafe {
        asm!("mov es, {0:x}", in(reg) selector, options(nostack, preserves_flags));
    }
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn set_fs_reg(selector: u16) {
    unsafe {
        asm!("mov fs, {0:x}", in(reg) selector, options(nostack, preserves_flags));
    }
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn set_gs_reg(selector: u16) {
    unsafe {
        asm!("mov gs, {0:x}", in(reg) selector, options(nostack, preserves_flags));
    }
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn set_ss_reg(selector: u16) {
    unsafe {
        asm!("mov ss, {0:x}", in(reg) selector, options(nostack, preserves_flags));
    }
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn load_gdt(gdt_ptr: &'static crate::arch::gdt::GdtPtr) {
    unsafe {
        asm!("lgdt [{}]", in(reg) gdt_ptr, options(readonly, nostack, preserves_flags));
    }
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn load_tss(tss_selector: u16) {
    unsafe {
        asm!("ltr {0:x}", in(reg) tss_selector, options(nostack, preserves_flags));
    }
}

#[inline(always)]
#[cfg(target_arch = "x86_64")]
pub fn load_idt(idt_ptr: &'static crate::arch::interrupts::IdtPtr) {
    unsafe {
        asm!("cli; lidt [{}]", in(reg) idt_ptr, options(readonly, nostack, preserves_flags));
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
