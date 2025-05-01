/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{arch::gdt, info, println};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub mod pic;

pub static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::new();

pub fn init_idt() {
    info!("initializing idt");

    unsafe {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        idt.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        idt[0x20].set_handler_fn(crate::arch::drivers::time::pit::timer_interrupt_handler);
        idt[0x21].set_handler_fn(crate::arch::drivers::keyboard::keyboard_interrupt_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        IDT = idt;
        IDT.load();
    }
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT - {}\n{:#?}",
        error_code, stack_frame
    );
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    panic!(
        "EXCEPTION: DOUBLE FAULT - {}\n{:#?}",
        error_code, stack_frame
    );
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    println!("{}EXCEPTION: PAGE FAULT", crate::utils::logger::color::RED);
    println!(
        "Accessed Address: 0x{:X}",
        crate::arch::system::cpu::read_registers().cr2
    );
    println!("Error Code: {:?}", error_code);
    println!("{:#?}{}", stack_frame, crate::utils::logger::color::RESET);
    panic!("page faulted");
}
