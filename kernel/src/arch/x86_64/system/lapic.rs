/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{
    arch::interrupts::StackFrame,
    debug, info,
    memory::vmm::{flag, page_size},
    utils::{
        asm::regs::{rdmsr, wrmsr},
        limine::get_hhdm_offset,
    },
};
use core::sync::atomic::{AtomicU64, Ordering, fence};

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
    pub const EOI: u32 = 0xB0;
}

static MMIO: AtomicU64 = AtomicU64::new(0);
static LAPIC_FREQUENCY: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    info!("setting up...");
    let mut val = rdmsr(reg::APIC_BASE);
    let phys_mmio = val & 0xFFFFF000;
    let mmio = phys_mmio + get_hhdm_offset();
    MMIO.store(mmio, Ordering::SeqCst);

    val |= 1 << 11;
    wrmsr(reg::APIC_BASE, val);

    debug!("mapping mmio: 0x{:X} -> 0x{:X}", phys_mmio, mmio);

    if unsafe {
        !crate::memory::vmm::PAGEMAP.get_mut().unwrap().lock().map(
            mmio,
            phys_mmio,
            flag::RW,
            page_size::SMALL,
        )
    } {
        panic!("could not map lapic mmio");
    }

    fence(Ordering::SeqCst);

    crate::arch::interrupts::install_interrupt(0xff, lapic_oneshot_timer_handler);

    mmio_write(reg::TPR, 0x00);
    mmio_write(reg::SIV, (1 << 8) | 0xFF);

    debug!("calibrating...");
    calibrate_timer();
    info!("done");
}

fn lapic_oneshot_timer_handler(stack_frame: *mut StackFrame) {
    crate::scheduler::schedule(stack_frame);
    mmio_write(reg::EOI, 0);
}

pub fn arm(ns: usize, vector: u8) {
    let freq = LAPIC_FREQUENCY.load(Ordering::SeqCst);

    let lvt_value = (vector as u32) & !(0b11 << 17);
    mmio_write(reg::LVT, lvt_value);

    let ticks = ((ns as u128 * freq as u128) / 1_000_000_000) as u32;
    mmio_write(reg::TIC, ticks);
}

fn calibrate_timer() {
    mmio_write(reg::TDC, 0b1011);

    let millis = 10;
    let times = 3;
    let mut freq_total: u64 = 0;

    for _ in 0..times {
        mmio_write(reg::TIC, 0xFFFFFFFF);

        crate::utils::time::busywait_ms(millis);

        let count = mmio_read(reg::TCC);
        mmio_write(reg::TIC, 0);

        let elapsed = 0xFFFFFFFFu64 - count as u64;
        freq_total += elapsed * 1000 / millis;
    }

    let avg = freq_total / times;
    LAPIC_FREQUENCY.store(avg, Ordering::SeqCst);

    debug!(
        "lapic frequency - {}.{:03}MHz",
        avg / 1_000_000,
        (avg % 1_000_000) / 1_000,
    );
}

#[inline(always)]
fn mmio_write(reg: u32, val: u32) {
    let addr = MMIO.load(Ordering::Relaxed) + reg as u64;
    crate::utils::asm::mmio::write(addr, val as u64, core::mem::size_of::<u32>());
}

#[inline(always)]
fn mmio_read(reg: u32) -> u32 {
    let addr = MMIO.load(Ordering::Relaxed) + reg as u64;
    crate::utils::asm::mmio::read(addr, core::mem::size_of::<u32>()) as u32
}
