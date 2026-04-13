/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

core::arch::global_asm!(include_str!("isr.S"));

use core::{
    arch::asm,
    sync::atomic::{AtomicPtr, Ordering},
};

use crate::{arch::system::cpu::Registers, debug};

#[repr(C, packed)]
pub struct IdtPtr {
    limit: u16,
    base: u64,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct IdtEntry {
    offset0: u16,
    selector: u16,
    ist: u8,
    typeattr: u8,
    offset1: u16,
    offset2: u32,
    zero: u32,
}

impl IdtEntry {
    const fn new() -> Self {
        Self {
            offset0: 0,
            selector: 0,
            ist: 0,
            typeattr: 0,
            offset1: 0,
            offset2: 0,
            zero: 0,
        }
    }

    fn set(&mut self, isr: u64, typeattr: Option<u8>, ist: Option<u8>) {
        self.typeattr = typeattr.unwrap_or(0x8E);
        self.ist = ist.unwrap_or(0);

        let addr = isr;
        self.offset0 = (addr & 0xFFFF) as u16;
        self.offset1 = ((addr >> 16) & 0xFFFF) as u16;
        self.offset2 = (addr >> 32) as u32;

        unsafe {
            asm!("mov {0:x}, cs", out(reg) self.selector, options(nomem, nostack, preserves_flags));
        }
    }
}

static HANDLERS: [AtomicPtr<()>; 256] = [const { AtomicPtr::new(core::ptr::null_mut()) }; 256];
static mut IDT: [IdtEntry; 256] = [IdtEntry::new(); 256];
static mut IDTR: IdtPtr = IdtPtr {
    limit: (size_of::<IdtEntry>() * 256 - 1) as u16,
    base: 0,
};

const EXCEPTION_NAMES: [&str; 32] = [
    "divide by zero",
    "debug",
    "non-maskable interrupt",
    "breakpoint",
    "detected overflow",
    "out-of-bounds",
    "invalid opcode",
    "no coprocessor",
    "double fault",
    "coprocessor segment overrun",
    "bad TSS",
    "segment not present",
    "stack fault",
    "general protection fault",
    "page fault",
    "unknown interrupt",
    "coprocessor fault",
    "alignment check",
    "machine check",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
    "reserved",
];

#[unsafe(no_mangle)]
extern "C" fn isr_handler(regs: &mut Registers) {
    if regs.vector < 32 {
        match regs.vector {
            1..=3 => {
                debug!(
                    "exception: {},\n{:#018x?}",
                    EXCEPTION_NAMES[regs.vector as usize], regs
                );
                return;
            }
            _ => {
                panic!(
                    "exception: {},\n{:#018x?}",
                    EXCEPTION_NAMES[regs.vector as usize], regs
                );
            }
        }
    }

    if regs.vector == 0x80 {
        super::syscall::syscall_handler(regs);
        return;
    }

    let handler_ptr = HANDLERS[regs.vector as usize].load(Ordering::Acquire);
    if !handler_ptr.is_null() {
        let handler: fn(&mut Registers) = unsafe { core::mem::transmute(handler_ptr) };
        handler(regs);
    }
}

unsafe extern "C" {
    static isr_table: [u64; 256];
}

pub fn init() {
    unsafe {
        for (i, entry) in IDT.iter_mut().enumerate() {
            entry.set(
                isr_table[i],
                if i == 0x80 || i == 0xFE {
                    Some(0xEE)
                } else {
                    Some(0x8E)
                },
                None,
            );
        }
        IDTR.base = IDT.as_ptr() as u64;

        asm!("cli; lidt [{}]", in(reg) &IDTR, options(readonly, nostack, preserves_flags));
    }

    install_interrupt(
        0x20,
        crate::arch::drivers::time::pit::timer_interrupt_handler,
    );
    install_interrupt(
        0x21,
        crate::arch::drivers::keyboard::keyboard_interrupt_handler,
    );
}

pub fn install_interrupt(vector: u8, func: fn(&mut Registers)) {
    HANDLERS[vector as usize].store(func as _, Ordering::Release);
}

pub fn clear_interrupt(vector: u8) {
    HANDLERS[vector as usize].store(0 as _, Ordering::Release);
}
