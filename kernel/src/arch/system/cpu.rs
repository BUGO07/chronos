/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use std::{info, kernel::bootloader::get_mp_response};

use alloc::vec::Vec;
use std::kernel::bootloader::limine::mp::Cpu;

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

extern "C" fn cpu_entry(_cpu: &Cpu) -> ! {
    // TODO: fix this
    std::asm::halt_loop();
}
