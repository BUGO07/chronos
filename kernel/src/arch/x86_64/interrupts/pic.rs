/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{
    debug, info,
    utils::asm::port::{inb, outb},
};

const PIC_EOI: u8 = 0x20;
const ICW1_ICW4: u8 = 0x01;
const ICW4_8086: u8 = 0x01;
const ICW1_INIT: u8 = 0x10;
const PIC1_COMMAND: u16 = 0x20;
const PIC2_COMMAND: u16 = 0xA0;
const PIC1_DATA: u16 = 0x21;
const PIC2_DATA: u16 = 0xA1;

pub fn send_eoi(irq: u8) {
    if irq >= 8 {
        outb(PIC2_COMMAND, PIC_EOI);
    }
    outb(PIC1_COMMAND, PIC_EOI);
}

pub fn interrupts_enabled() -> bool {
    let rflags: u64;
    unsafe {
        core::arch::asm!("pushfq; pop {}", out(reg) rflags);
    }
    (rflags & (1 << 9)) != 0
}

pub fn unmask_all() {
    debug!("unmasking all irqs...");
    outb(PIC1_DATA, 0);
    outb(PIC2_DATA, 0);
    info!("done");
}

pub fn mask_all() {
    debug!("masking all irqs...");
    outb(PIC1_DATA, 0xff);
    outb(PIC2_DATA, 0xff);
    info!("done");
}

pub fn mask(mut irq: u8) {
    let port: u16;
    if irq < 8 {
        port = PIC1_DATA;
    } else {
        port = PIC2_DATA;
        irq -= 8;
    }
    outb(port, inb(port) | (1 << irq));
    // debug!("masked irq {}", irq);
}

pub fn unmask(mut irq: u8) {
    let port: u16;
    if irq < 8 {
        port = PIC1_DATA;
    } else {
        port = PIC2_DATA;
        irq -= 8;
    }
    outb(port, inb(port) & !(1 << irq));
    // debug!("unmasked irq {}", irq);
}

pub fn init() {
    info!("remapping...");

    let i1 = inb(PIC1_DATA);
    let i2 = inb(PIC2_DATA);

    outb(PIC1_COMMAND, ICW1_INIT | ICW1_ICW4);
    outb(PIC2_COMMAND, ICW1_INIT | ICW1_ICW4);

    outb(PIC1_DATA, 0x20);
    outb(PIC2_DATA, 0x28);

    outb(PIC1_DATA, 0x04);
    outb(PIC2_DATA, 0x02);

    outb(PIC1_DATA, ICW4_8086);
    outb(PIC2_DATA, ICW4_8086);

    outb(PIC1_DATA, i1);
    outb(PIC2_DATA, i2);
}
