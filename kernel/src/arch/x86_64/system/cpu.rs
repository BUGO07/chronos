/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::vec::Vec;
use limine::mp::Cpu;

use crate::{info, scheduler::cooperative, utils::limine::get_mp_response};

static mut PROCESSORS: Vec<&Cpu> = Vec::new();

pub fn init_bsp() {
    let mp = get_mp_response();
    let bsp_id = mp.bsp_lapic_id();

    for cpu in mp.cpus() {
        if bsp_id != cpu.lapic_id {
            continue;
        }

        info!("initializing bsp {}: lapic id: {}", cpu.id, cpu.lapic_id);

        unsafe { PROCESSORS.push(cpu) };
    }
}

pub fn init() {
    let mp = get_mp_response();
    let bsp_id = mp.bsp_lapic_id();

    for cpu in mp.cpus() {
        if bsp_id == cpu.lapic_id {
            continue;
        }

        info!("initializing cpu {}: lapic id: {}", cpu.id, cpu.lapic_id);

        unsafe { PROCESSORS.push(cpu) };
        cpu.goto_address.write(cpu_entry);
    }
}

extern "C" fn cpu_entry(_cpu: &limine::mp::Cpu) -> ! {
    // TODO: fix this
    let mut scheduler = cooperative::Scheduler::new();
    scheduler.run()
}
