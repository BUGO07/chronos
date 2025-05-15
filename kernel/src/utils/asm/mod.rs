/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::arch::asm;

use alloc::string::String;

#[cfg(target_arch = "x86_64")]
pub mod port;

pub mod mem;
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
    return (r >> 7) & 1 == 0; // IRQs
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

pub fn get_cpu() -> String {
    #[cfg(target_arch = "x86_64")]
    {
        let part1 = _cpuid(0x80000002);
        let part2 = _cpuid(0x80000003);
        let part3 = _cpuid(0x80000004);

        let brand_raw = [
            part1.eax, part1.ebx, part1.ecx, part1.edx, part2.eax, part2.ebx, part2.ecx, part2.edx,
            part3.eax, part3.ebx, part3.ecx, part3.edx,
        ];

        String::from(
            brand_raw
                .iter()
                .flat_map(|reg| reg.to_le_bytes())
                .map(|b| b as char)
                .collect::<String>()
                .trim(),
        )
    }
    #[cfg(target_arch = "aarch64")]
    {
        let midr: u64;
        unsafe {
            core::arch::asm!("mrs {0}, MIDR_EL1", out(reg) midr);
        }
        let implementer = ((midr >> 24) & 0xFF) as u8;
        let variant = ((midr >> 20) & 0xF) as u8;
        let architecture = ((midr >> 16) & 0xF) as u8;
        let part_num = ((midr >> 4) & 0xFFF) as u16;
        let revision = (midr & 0xF) as u8;

        alloc::format!(
            "{} V{revision} [variant - {variant}, arch - 0x{architecture:X}]",
            match (implementer, part_num) {
                (0x41, 0xD03) => "ARM Cortex-A53",
                (0x41, 0xD07) => "ARM Cortex-A57",
                (0x41, 0xD08) => "ARM Cortex-A72",
                (0x41, 0xD09) => "ARM Cortex-A73",
                (0x41, 0xD0A) => "ARM Cortex-A75",
                // do we really gaf?
                _ => "Unknown CPU",
            }
        )
    }
}

#[cfg(target_arch = "aarch64")]
#[derive(Debug)]
pub struct Registers {
    pub x0: u64,
    pub x1: u64,
    pub x2: u64,
    pub x3: u64,
    pub x4: u64,
    pub x5: u64,
    pub x6: u64,
    pub x7: u64,
    pub x8: u64,
    pub x9: u64,
    pub x10: u64,
    pub x11: u64,
    pub x12: u64,
    pub x13: u64,
    pub x14: u64,
    pub x15: u64,
    pub x16: u64,
    pub x17: u64,
    pub x18: u64,
    pub x19: u64,
    pub x20: u64,
    pub x21: u64,
    pub x22: u64,
    pub x23: u64,
    pub x24: u64,
    pub x25: u64,
    pub x26: u64,
    pub x27: u64,
    pub x28: u64,
    pub fp: u64, // x29
    pub lr: u64, // x30
    pub sp: u64,
    pub pc: u64,
}

#[cfg(target_arch = "x86_64")]
#[derive(Debug)]
pub struct Registers {
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
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
    pub cr2: u64,
    pub cr3: u64,
}

#[inline(always)]
pub fn dump_regs() -> Registers {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let (
            x0,
            x1,
            x2,
            x3,
            x4,
            x5,
            x6,
            x7,
            x8,
            x9,
            x10,
            x11,
            x12,
            x13,
            x14,
            x15,
            x16,
            x17,
            x18,
            x19,
            x20,
            x21,
            x22,
            x23,
            x24,
            x25,
            x26,
            x27,
            x28,
            fp,
            lr,
            sp,
            pc,
        );

        asm!(
            "mov {}, x0",
            "mov {}, x1",
            "mov {}, x2",
            "mov {}, x3",
            "mov {}, x4",
            "mov {}, x5",
            "mov {}, x6",
            "mov {}, x7",
            "mov {}, x8",
            "mov {}, x9",
            "mov {}, x10",
            "mov {}, x11",
            "mov {}, x12",
            "mov {}, x13",
            "mov {}, x14",
            "mov {}, x15",
            "mov {}, x16",
            "mov {}, x17",
            "mov {}, x18",
            "mov {}, x19",
            out(reg) x0,
            out(reg) x1,
            out(reg) x2,
            out(reg) x3,
            out(reg) x4,
            out(reg) x5,
            out(reg) x6,
            out(reg) x7,
            out(reg) x8,
            out(reg) x9,
            out(reg) x10,
            out(reg) x11,
            out(reg) x12,
            out(reg) x13,
            out(reg) x14,
            out(reg) x15,
            out(reg) x16,
            out(reg) x17,
            out(reg) x18,
            out(reg) x19,
        );

        asm!(
            "mov {}, x20",
            "mov {}, x21",
            "mov {}, x22",
            "mov {}, x23",
            "mov {}, x24",
            "mov {}, x25",
            "mov {}, x26",
            "mov {}, x27",
            "mov {}, x28",
            "mov {}, x29",
            "mov {}, x30",
            "mov {}, sp",
            "adr {}, .", // pc
            out(reg) x20,
            out(reg) x21,
            out(reg) x22,
            out(reg) x23,
            out(reg) x24,
            out(reg) x25,
            out(reg) x26,
            out(reg) x27,
            out(reg) x28,
            out(reg) fp,
            out(reg) lr,
            out(reg) sp,
            out(reg) pc,
        );

        Registers {
            x0,
            x1,
            x2,
            x3,
            x4,
            x5,
            x6,
            x7,
            x8,
            x9,
            x10,
            x11,
            x12,
            x13,
            x14,
            x15,
            x16,
            x17,
            x18,
            x19,
            x20,
            x21,
            x22,
            x23,
            x24,
            x25,
            x26,
            x27,
            x28,
            fp,
            lr,
            sp,
            pc,
        }
    }
    #[cfg(target_arch = "x86_64")]
    unsafe {
        let (
            r15,
            r14,
            r13,
            r12,
            r11,
            r10,
            r9,
            r8,
            rbp,
            rdi,
            rsi,
            rdx,
            rcx,
            rbx,
            rax,
            rsp,
            cs,
            rflags,
            ss,
            rip,
            cr2,
            cr3,
        );

        asm!(
            "mov {}, r15",
            "mov {}, r14",
            "mov {}, r13",
            "mov {}, r12",
            "mov {}, r11",
            "mov {}, r10",
            "mov {}, r9",
            "mov {}, r8",
            "mov {}, rbp",
            "mov {}, rdi",
            out(reg) r15,
            out(reg) r14,
            out(reg) r13,
            out(reg) r12,
            out(reg) r11,
            out(reg) r10,
            out(reg) r9,
            out(reg) r8,
            out(reg) rbp,
            out(reg) rdi,
        );
        asm!(
            "mov {}, rsi",
            "mov {}, rdx",
            "mov {}, rcx",
            "mov {}, rbx",
            "mov {}, rax",
            "mov {}, rsp",
            "mov {}, cs",
            "pushfq; pop {}",
            "mov {}, ss",
            "lea {}, [rip]",
            out(reg) rsi,
            out(reg) rdx,
            out(reg) rcx,
            out(reg) rbx,
            out(reg) rax,
            out(reg) rsp,
            out(reg) cs,
            out(reg) rflags,
            out(reg) ss,
            out(reg) rip,
        );
        asm!(
            "mov {}, cr2",
            "mov {}, cr3",
            out(reg) cr2,
            out(reg) cr3,
            options(nomem, nostack, preserves_flags)
        );

        Registers {
            r15,
            r14,
            r13,
            r12,
            r11,
            r10,
            r9,
            r8,
            rbp,
            rdi,
            rsi,
            rdx,
            rcx,
            rbx,
            rax,
            rip,
            cs,
            rflags,
            rsp,
            ss,
            cr2,
            cr3,
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub fn kvm_base() -> u32 {
    if in_hypervisor() {
        use core::ffi::c_void;
        let mut signature: [u32; 3] = [0; 3];
        for base in (0x40000000..0x40010000).step_by(0x100) {
            let id = _cpuid(base);

            signature[0] = id.ebx;
            signature[1] = id.ecx;
            signature[2] = id.edx;

            let mut output: [u8; 12] = [0; 12];

            for (i, num) in signature.iter().enumerate() {
                let bytes = num.to_le_bytes();
                output[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
            }
            if crate::utils::asm::mem::memcmp(
                c"KVMKVMKVM".as_ptr() as *const c_void,
                output.as_ptr() as *const c_void,
                12,
            ) != 0
            {
                return base;
            }
        }
    }
    0
}

#[cfg(target_arch = "x86_64")]
pub fn in_hypervisor() -> bool {
    let id = _cpuid(1);

    (id.ecx & (1 << 31)) != 0
}
