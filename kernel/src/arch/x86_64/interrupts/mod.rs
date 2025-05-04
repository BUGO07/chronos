/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::arch::{asm, global_asm};

use x86_64::registers::segmentation::{CS, Segment};

pub mod pic;

#[repr(C)]
#[repr(packed)]
#[derive(Debug, Default, Clone, Copy)]
pub struct StackFrame {
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

#[repr(C)]
#[repr(packed)]
struct IdtPtr {
    limit: u16,
    base: u64,
}

#[repr(C)]
#[repr(packed)]
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

        self.selector = CS::get_reg().0;
    }
}

global_asm! {
    r#"
.extern isr_handler
isr_common_stub:
    cld

    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    mov rdi, rsp
    call isr_handler

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    add rsp, 16

    iretq

.macro isr number
    isr_\number:
.if !(\number == 8 || (\number >= 10 && \number <= 14) || \number == 17 || \number == 21 || \number == 29 || \number == 30)
    push 0
.endif
    push \number
    jmp isr_common_stub
.endm

.altmacro
.macro isr_insert number
    .section .text
    isr \number

    .section .data
    .quad isr_\number
.endm

.section .data
.byte 1
.align 8
isr_table:
.set i, 0
.rept 256
    isr_insert %i
    .set i, i + 1
.endr
.global isr_table
    "#
}

type HandlerFn = fn(frame: *mut StackFrame);
static mut HANDLERS: [Option<HandlerFn>; 256] = [None; 256];
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
extern "C" fn isr_handler(regs: *mut StackFrame) {
    unsafe {
        let registers = &*regs;
        if registers.vector < 32 {
            panic!(
                "exception: {}, {:?}",
                EXCEPTION_NAMES[registers.vector as usize], registers
            );
        }

        if let Some(handler) = HANDLERS[registers.vector as usize] {
            handler(regs);
        }
    };
}

unsafe extern "C" {
    static isr_table: u64;
}

pub fn init_idt() {
    let table = &raw const isr_table;
    #[allow(clippy::needless_range_loop)]
    for i in 0..256 {
        unsafe {
            IDT[i].set(*table.add(i), None, None);
        }
    }
    #[allow(static_mut_refs)]
    unsafe {
        IDTR.base = IDT.as_ptr() as u64
    };

    unsafe {
        asm!("cli; lidt [{}]", in(reg) &raw const IDTR, options(nostack));
    }

    unsafe {
        HANDLERS[0x20] = Some(crate::arch::drivers::time::pit::timer_interrupt_handler);
        HANDLERS[0x21] = Some(crate::arch::drivers::keyboard::keyboard_interrupt_handler);
    }
}

pub fn install_interrupt(vector: u8, func: HandlerFn) {
    unsafe {
        HANDLERS[vector as usize] = Some(func);
    }
}

pub fn clear_interrupt(vector: u8) {
    unsafe {
        HANDLERS[vector as usize] = None;
    }
}
