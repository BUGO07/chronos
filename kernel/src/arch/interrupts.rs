/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{
    arch::{drivers::pic, gdt},
    halt_loop, info, print, println,
};
use lazy_static::lazy_static;
use x86_64::{
    instructions::port::Port,
    structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode},
};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        idt[0x20].set_handler_fn(timer_interrupt_handler);
        idt[0x21].set_handler_fn(keyboard_interrupt_handler);
        idt[0xFF].set_handler_fn(lapic_oneshot_timer_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt
    };
}

pub fn init_idt() {
    info!("initializing idt");
    IDT.load();
}

extern "x86-interrupt" fn lapic_oneshot_timer_handler(
    _stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
    // for now
    print!("$ ");
}

extern "x86-interrupt" fn timer_interrupt_handler(
    _stack_frame: x86_64::structures::idt::InterruptStackFrame,
) {
    crate::task::timer::tick_timer();
    crate::arch::drivers::pic::send_eoi();
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);
    pic::send_eoi();
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
        "Accessed Address: {:?}",
        crate::arch::system::cpu::read_registers().cr2
    );
    println!("Error Code: {:?}", error_code);
    println!("{:#?}{}", stack_frame, crate::utils::logger::color::RESET);
    halt_loop();
}
