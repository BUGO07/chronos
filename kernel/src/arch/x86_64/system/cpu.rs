/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    arch::{asm, x86_64::__cpuid},
    cell::OnceCell,
    ffi::c_void,
};

use crossbeam_queue::ArrayQueue;
use limine::mp::Cpu;

use crate::{info, scheduler::Scheduler, utils::limine::get_mp_response};

static mut CPU_COUNT: usize = 0;
static mut PROCESSORS: OnceCell<ArrayQueue<&Cpu>> = OnceCell::new();

pub fn init_bsp() {
    let mp = get_mp_response();
    let bsp_id = mp.bsp_lapic_id();
    let cpus = mp.cpus_mut();
    let cpu_count = cpus.len();
    unsafe { CPU_COUNT = cpu_count };

    unsafe { PROCESSORS.set(ArrayQueue::new(cpu_count)).unwrap() };

    for cpu in cpus {
        if bsp_id != cpu.lapic_id {
            continue;
        }

        info!("initializing bsp {}: lapic id: {}", cpu.id, cpu.lapic_id);

        unsafe { PROCESSORS.get().unwrap().push(cpu).ok() };
    }
}

pub fn init() {
    let mp = get_mp_response();
    let bsp_id = mp.bsp_lapic_id();

    let processors = unsafe { PROCESSORS.get().unwrap() };

    let mut idx = 0;
    for cpu in mp.cpus_mut() {
        idx += 1;
        if bsp_id == cpu.lapic_id {
            continue;
        }

        info!("initializing cpu {}: lapic id: {}", cpu.id, cpu.lapic_id);

        cpu.extra = idx;
        processors.push(cpu).ok();
        cpu.goto_address.write(cpu_entry);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn cpu_entry(_cpu: &limine::mp::Cpu) -> ! {
    // todo
    let mut scheduler = Scheduler::new();
    scheduler.run()
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

pub fn read_registers() -> Registers {
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

    unsafe {
        asm!(
            "mov {}, r15", "mov {}, r14", "mov {}, r13", "mov {}, r12", "mov {}, r11",
            out(reg) r15, out(reg) r14, out(reg) r13, out(reg) r12, out(reg) r11,
        );
        asm!(
            "mov {}, r10", "mov {}, r9", "mov {}, r8", "mov {}, rbp", "mov {}, rdi",
            out(reg) r10, out(reg) r9, out(reg) r8, out(reg) rbp, out(reg) rdi,
        );
        asm!(
            "mov {}, rsi", "mov {}, rdx", "mov {}, rcx", "mov {}, rbx", "mov {}, rax",
            out(reg) rsi, out(reg) rdx, out(reg) rcx, out(reg) rbx, out(reg) rax,
        );
        asm!(
            "mov {}, rsp", "mov {}, cs", "pushfq; pop {}", "mov {}, ss", "lea {}, [rip]",
            out(reg) rsp, out(reg) cs, out(reg) rflags, out(reg) ss, out(reg) rip,
        );

        asm!("mov {}, cr2", out(reg) cr2, options(nomem, nostack, preserves_flags));

        asm!("mov {}, cr3", out(reg) cr3, options(nomem, nostack, preserves_flags));
    }

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

pub fn kvm_base() -> u32 {
    unsafe {
        if in_hypervisor() {
            let mut signature: [u32; 3] = [0; 3];
            for base in (0x40000000..0x40010000).step_by(0x100) {
                let id = __cpuid(base);

                signature[0] = id.ebx;
                signature[1] = id.ecx;
                signature[2] = id.edx;

                let mut output: [u8; 12] = [0; 12];

                for (i, num) in signature.iter().enumerate() {
                    let bytes = num.to_le_bytes();
                    output[i * 4..(i + 1) * 4].copy_from_slice(&bytes);
                }
                if crate::utils::cmem::memcmp(
                    c"KVMKVMKVM".as_ptr() as *const c_void,
                    output.as_ptr() as *const c_void,
                    12,
                ) != 0
                {
                    return base;
                }
            }
        }
    }
    0
}

pub fn in_hypervisor() -> bool {
    let id = unsafe { __cpuid(1) };

    (id.ecx & (1 << 31)) != 0
}

pub fn read_msr(msr: u32) -> u64 {
    let high: u32;
    let low: u32;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("edx") high,
            out("eax") low,
        );
    }
    (high as u64) << 32 | low as u64
}

pub fn write_msr(msr: u32, value: u64) {
    let high = (value >> 32) as u32;
    let low = value as u32;
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("edx") high,
            in("eax") low,
        );
    }
}
