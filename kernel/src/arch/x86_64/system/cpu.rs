/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::vec::Vec;
use limine::mp::MpInfo;

use crate::{
    info,
    scheduler::cooperative,
    utils::{limine::get_mp_response, spinlock::Spin},
};

static PROCESSORS: Spin<Vec<&MpInfo>> = Spin::new(Vec::new());

#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, PartialOrd)]
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
    pub vector: u64,
    pub error_code: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

pub fn init_bsp() {
    let mp = get_mp_response();
    let bsp_id = mp.bsp_lapic_id;

    for cpu in mp.cpus() {
        if bsp_id != cpu.lapic_id {
            continue;
        }

        info!(
            "initializing bsp {}: lapic id: {}",
            cpu.processor_id, cpu.lapic_id
        );

        PROCESSORS.lock().push(cpu);
    }
    crate::arch::gdt::init();
    crate::arch::system::interrupts::init();
    crate::arch::system::pic::init();
    crate::arch::system::syscall::init();
}

pub fn init() {
    let mp = get_mp_response();
    let bsp_id = mp.bsp_lapic_id;

    for cpu in mp.cpus() {
        if bsp_id == cpu.lapic_id {
            continue;
        }

        info!(
            "initializing cpu {}: lapic id: {}",
            cpu.processor_id, cpu.lapic_id
        );

        PROCESSORS.lock().push(cpu);
        cpu.bootstrap(cpu_entry, 0);
    }
}

extern "C" fn cpu_entry(_cpu: &limine::mp::MpInfo) -> ! {
    // TODO: fix this
    let mut scheduler = cooperative::Scheduler::new();
    scheduler.run()
}
