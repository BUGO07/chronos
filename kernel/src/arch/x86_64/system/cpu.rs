/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::vec::Vec;
use limine::mp::MpInfo;

use crate::{
    info,
    scheduler::cooperative,
    utils::{limine::get_mp_response, spinlock::SpinLock},
};

static PROCESSORS: SpinLock<Vec<&MpInfo>> = SpinLock::new(Vec::new());

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
