/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::arch::asm;

use alloc::string::String;

pub mod mem;
pub mod mmio;
pub mod port;
pub mod regs;

#[inline(always)]
pub fn halt() {
    unsafe {
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
        asm!("pushfq; pop {}", out(reg) r);
    }
    (r & (1 << 9)) != 0
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

pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

#[inline(always)]
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
pub fn _cpuid(leaf: u32) -> CpuidResult {
    _cpuid_count(leaf, 0)
}

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
            if self::mem::memcmp(
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

pub fn in_hypervisor() -> bool {
    let id = _cpuid(1);

    (id.ecx & (1 << 31)) != 0
}

#[cfg(debug_assertions)]
#[repr(C)]
struct StackTrace {
    rbp: *const StackTrace,
    rip: usize,
}

pub fn stack_trace() {
    #[cfg(debug_assertions)]
    {
        let mut rbp: *const StackTrace;
        unsafe {
            core::arch::asm!("mov {}, rbp", out(reg) rbp);
        }
        let mut i = 0;
        while let Some(frame) = unsafe { rbp.as_ref() } {
            crate::print_centered!(
                alloc::format!("frame {}: rip = 0x{:016x}", i, frame.rip).as_str(),
                "~"
            );
            rbp = frame.rbp;
            i += 1;

            if i > 64 {
                break;
            }
        }
    }
}
