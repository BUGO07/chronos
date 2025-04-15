/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
pub mod arch;
pub mod memory;
pub mod task;
pub mod utils;

#[cfg(feature = "tests")]
pub mod tests;

extern crate alloc;

use core::panic::PanicInfo;
use limine::BaseRevision;
use limine::request::{DateAtBootRequest, MpRequest, RequestsEndMarker, RequestsStartMarker};
use task::Task;
use task::executor::Executor;
use utils::halt_loop;

const NOOO: &str = include_str!("../assets/nooo.txt");

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[used]
#[unsafe(link_section = ".requests")]
static BOOT_DATE: DateAtBootRequest = DateAtBootRequest::new();

#[used]
#[unsafe(link_section = ".requests")]
static MP_REQUEST: MpRequest = MpRequest::new();

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    assert!(BASE_REVISION.is_supported());

    println!("\n{NOOO}\n");

    info!("x86_64 kernel starting...\n");

    arch::gdt::init();
    arch::interrupts::init_idt();
    arch::drivers::pic::init();
    arch::drivers::pit::init();

    memory::init();

    println!("\n--------------------------------------\n");

    debug!("debug rahh");
    info!("up and running");
    let cpus = MP_REQUEST.get_response().unwrap().cpus();
    info!("found {} cpus", cpus.len());
    for cpu in cpus {
        info!(
            "cpu {}: lapic id - {}, extra - 0x{:X} ",
            cpu.id, cpu.lapic_id, cpu.extra
        );
    }
    for (i, fb) in utils::term::get_framebuffers().enumerate() {
        info!("display {}: size - {}x{}", i + 1, fb.width(), fb.height());
    }

    let (years, months, days, hours, minutes, seconds) =
        utils::time::unix_to_date(BOOT_DATE.get_response().unwrap().timestamp());
    info!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        years, months, days, hours, minutes, seconds
    );

    #[cfg(feature = "tests")]
    tests::init();

    print!("\n> ");

    let mut executor = Executor::new();
    executor.spawn(Task::new(task::keyboard::handle_keypresses()));
    executor.run();

    // halt_loop();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    #[cfg(feature = "tests")]
    {
        println!("[failed]\n");
        println!("{info}\n");
        utils::exit_qemu(utils::QemuExitCode::Failed);
    }
    #[cfg(not(feature = "tests"))]
    {
        println!("\n{NOOO}\n");
        error!("{info}\n");
        error!("stopping code execution and dumping registers\n");
        let registers = arch::system::cpu::read_registers();
        println!(
            "r15:    0x{:016X}\n\
             r14:    0x{:016X}\n\
             r13:    0x{:016X}\n\
             r12:    0x{:016X}\n\
             r11:    0x{:016X}\n\
             r10:    0x{:016X}\n\
             r9:     0x{:016X}\n\
             r8:     0x{:016X}\n\
             rbp:    0x{:016X}\n\
             rdi:    0x{:016X}\n\
             rsi:    0x{:016X}\n\
             rdx:    0x{:016X}\n\
             rcx:    0x{:016X}\n\
             rbx:    0x{:016X}\n\
             rax:    0x{:016X}\n\
             rip:    0x{:016X}\n\
             cs:     0x{:016X}\n\
             rflags: 0x{:016X}\n\
             rsp:    0x{:016X}\n\
             ss:     0x{:016X}",
            registers.r15,
            registers.r14,
            registers.r13,
            registers.r12,
            registers.r11,
            registers.r10,
            registers.r9,
            registers.r8,
            registers.rbp,
            registers.rdi,
            registers.rsi,
            registers.rdx,
            registers.rcx,
            registers.rbx,
            registers.rax,
            registers.rip,
            registers.cs,
            registers.rflags,
            registers.rsp,
            registers.ss
        );
        x86_64::instructions::interrupts::disable();
    }
    halt_loop()
}

pub fn call_panic(reason: &str) {
    panic!("{reason}");
}
