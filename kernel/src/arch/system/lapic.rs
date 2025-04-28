/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use mmio::{Allow, VolBox};

use crate::{
    arch::{drivers::time::pit::current_pit_ticks, interrupts::IDT},
    debug, info,
    memory::{
        get_hhdm_offset,
        vmm::{flag, page_size},
    },
};

#[allow(dead_code)]
mod reg {
    pub const APIC_BASE: u32 = 0x1B;
    pub const TPR: u32 = 0x80;
    pub const SIV: u32 = 0xF0;
    pub const ICRL: u32 = 0x300;
    pub const ICRH: u32 = 0x310;
    pub const LVT: u32 = 0x320;
    pub const TDC: u32 = 0x3E0;
    pub const TIC: u32 = 0x380;
    pub const TCC: u32 = 0x390;

    pub const DEADLINE: u32 = 0x6E0;
}

pub static mut MMIO: u64 = 0;
pub static mut LAPIC_FREQUENCY: u64 = 0;

pub fn init() {
    info!("setting up...");
    let mut val = super::cpu::read_msr(reg::APIC_BASE);
    let phys_mmio = val & 0xFFFFF000;

    val |= 1 << 11;
    super::cpu::write_msr(reg::APIC_BASE, val);

    unsafe { MMIO = phys_mmio + get_hhdm_offset() };

    debug!("mapping mmio: 0x{:X} -> 0x{:X}", phys_mmio, unsafe { MMIO });

    let psize = page_size::SMALL;

    if !crate::memory::vmm::PAGEMAP.lock().map(
        unsafe { MMIO },
        phys_mmio,
        flag::PRESENT | flag::WRITE,
        psize,
    ) {
        panic!("could not map lapic mmio");
    }

    unsafe {
        IDT[0xFF].set_handler_fn(lapic_oneshot_timer_handler);
        // pic::unmask(0xFF);
    }

    mmio_write(reg::TPR, 0x00);
    mmio_write(reg::SIV, (1 << 8) | 0xFF);

    debug!("calibrating...");
    calibrate_timer();
    arm(250_000_000, 0xFF);
}

extern "x86-interrupt" fn lapic_oneshot_timer_handler(
    _stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
}

pub fn arm(ns: usize, vector: u8) {
    mmio_write(reg::TIC, 0);
    mmio_write(reg::LVT, vector as u32);
    mmio_write(reg::TIC, unsafe {
        (ns as u128 * LAPIC_FREQUENCY as u128 / 1_000_000_000) as u32
    });
}

fn calibrate_timer() {
    mmio_write(reg::TDC, 0b1011);

    let millis = 10;
    let times = 3;
    let mut freq: u64 = 0;

    for _ in 0..times {
        mmio_write(reg::TIC, 0xFFFFFFFF);
        let target = current_pit_ticks() + millis;
        loop {
            if current_pit_ticks() >= target {
                break;
            }
        }
        let count = mmio_read(reg::TCC);
        mmio_write(reg::TIC, 0);

        freq += (0xFFFFFFFF - count as u64) * 100;
    }
    unsafe {
        LAPIC_FREQUENCY = freq / times;
        debug!(
            "lapic frequency - {}.{:03}MHz",
            LAPIC_FREQUENCY / 1_000_000,
            (LAPIC_FREQUENCY % 1_000_000) / 1_000,
        );
    };
}

fn mmio_write(reg: u32, val: u32) {
    unsafe {
        VolBox::<u32, Allow, Allow>::new((MMIO + reg as u64) as *mut u32).write(val);
    }
}

fn mmio_read(reg: u32) -> u32 {
    unsafe { VolBox::<u32, Allow, Allow>::new((MMIO + reg as u64) as *mut u32).read() }
}
