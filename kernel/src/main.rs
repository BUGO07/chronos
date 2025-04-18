/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![allow(static_mut_refs)]

pub mod arch;
pub mod memory;
pub mod task;
pub mod utils;

#[cfg(feature = "tests")]
pub mod tests;

extern crate alloc;

use crate::{
    arch::drivers::time::tsc::measure_cpu_frequency,
    task::{Task, executor::Executor},
    utils::halt_loop,
};

use alloc::vec::Vec;
use core::panic::PanicInfo;
use limine::{
    BaseRevision,
    request::{RequestsEndMarker, RequestsStartMarker},
};

const NOOO: &str = include_str!("../res/nooo.txt");

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

// #[used]
// #[unsafe(link_section = ".requests")]
// static BOOT_DATE: DateAtBootRequest = DateAtBootRequest::new();

// #[used]
// #[unsafe(link_section = ".requests")]
// static MP_REQUEST: MpRequest = MpRequest::new();

pub static mut CPU_FREQ: u64 = 0;

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    assert!(BASE_REVISION.is_supported());

    println!("\n{NOOO}\n");

    info!("x86_64 kernel starting...\n");

    crate::memory::init();
    crate::arch::gdt::init();
    crate::arch::interrupts::init_idt();
    crate::arch::drivers::mouse::init();
    crate::arch::drivers::pic::init();
    crate::arch::drivers::time::init();

    println!();
    print_fill!("-");
    println!();

    info!("up and running");

    // let cpus = MP_REQUEST.get_response().unwrap().cpus();
    // info!("Found {} cpus", cpus.len());
    // for cpu in cpus {
    //     info!(
    //         "cpu {}: lapic id - {}, extra - 0x{:X} ",
    //         cpu.id, cpu.lapic_id, cpu.extra
    //     );
    // }

    let framebuffers = crate::utils::term::get_framebuffers().collect::<Vec<_>>();
    info!("found {} displays:", framebuffers.len());
    for (i, fb) in framebuffers.iter().enumerate() {
        info!("display {}: size - {}x{}", i + 1, fb.width(), fb.height());
    }

    let config = crate::utils::config::get_config();

    let rtc_time = crate::arch::drivers::time::rtc::RtcTime::current()
        .with_timezone_offset(config.time.zone_offset as i16)
        .adjusted_for_timezone();

    info!(
        "{} | {}",
        rtc_time.datetime_pretty(),
        rtc_time.timezone_pretty()
    );

    #[cfg(feature = "tests")]
    tests::init();

    // crate::arch::shell::run_command("clear", Vec::new());

    // info!("cpu freq - {}", crate::arch::system::cpuid::get_freq());

    let mut executor = Executor::new();
    executor.spawn(Task::new(crate::task::keyboard::handle_keypresses()));
    executor.spawn(Task::new(async move {
        let cpu_freq = measure_cpu_frequency().await;
        unsafe {
            CPU_FREQ = cpu_freq;
        }
        info!("rocking a {}", crate::arch::system::cpuid::get_cpu());
        info!(
            "cpu frequency - {}.{:03}GHz",
            cpu_freq / 1_000_000_000,
            (cpu_freq % 1_000_000_000) / 1_000_000,
        );
    }));
    executor.run();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    #[cfg(feature = "tests")]
    {
        println!("[failed]\n");
        println!("{info}\n");
        crate::utils::exit_qemu(crate::utils::QemuExitCode::Failed);
    }
    #[cfg(not(feature = "tests"))]
    {
        println!("\n{}{NOOO}\n", crate::utils::logger::color::RED);
        print_fill!("~", "Kernel Panic");
        println!("~");
        if let Some(location) = info.location() {
            println!(
                "~    ERROR: panicked at {}:{}:{} with message: {}\n~    ",
                location.file(),
                location.line(),
                location.column(),
                info.message(),
            );
        } else {
            println!(
                "~    ERROR: panicked with message: {}\n~    ",
                info.message(),
            );
        }
        println!("~    stopping code execution and dumping registers\n~    ");
        let registers = crate::arch::system::cpu::read_registers();
        println!(
            "~    r15:    0x{0:016X}  -  rsi:    0x{10:016X}\n\
             ~    r14:    0x{1:016X}  -  rdx:    0x{11:016X}\n\
             ~    r13:    0x{2:016X}  -  rcx:    0x{12:016X}\n\
             ~    r12:    0x{3:016X}  -  rbx:    0x{13:016X}\n\
             ~    r11:    0x{4:016X}  -  rax:    0x{14:016X}\n\
             ~    r10:    0x{5:016X}  -  rip:    0x{15:016X}\n\
             ~    r9:     0x{6:016X}  -  cs:     0x{16:016X}\n\
             ~    r8:     0x{7:016X}  -  rflags: 0x{17:016X}\n\
             ~    rbp:    0x{8:016X}  -  rsp:    0x{18:016X}\n\
             ~    rdi:    0x{9:016X}  -  ss:     0x{19:016X}\n\
             ~    cr2:    0x{20:016X}  -  cr3:    0x{21:016X}\n~",
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
            registers.ss,
            registers.cr2,
            registers.cr3
        );
        print_fill!("~");
        print!("{}", crate::utils::logger::color::RESET);
        x86_64::instructions::interrupts::disable();
    }
    halt_loop()
}
