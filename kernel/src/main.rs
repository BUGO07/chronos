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

use crate::{arch::drivers::time::tsc::measure_cpu_frequency, task::Task, utils::halt_loop};

use alloc::{string::ToString, vec::Vec};
use core::{
    panic::PanicInfo,
    sync::atomic::{AtomicU64, Ordering},
};
use limine::{
    BaseRevision,
    request::{RequestsEndMarker, RequestsStartMarker},
};
use task::scheduler::Scheduler;

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

pub static mut CPU_FREQ: AtomicU64 = AtomicU64::new(0);

#[cfg(debug_assertions)]
#[repr(C)]
struct StackFrame {
    rbp: *const StackFrame,
    rip: usize,
}

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    assert!(BASE_REVISION.is_supported());

    println!("\n{NOOO}\n");
    info!("x86_64 kernel starting...\n");

    crate::memory::init();
    crate::arch::gdt::init();
    crate::arch::interrupts::init_idt();
    crate::arch::interrupts::pic::init();
    crate::arch::interrupts::pic::unmask_all(); // limine masks all IRQs by default // todo: fix ts

    debug!("enabling interrupts (sti)");
    x86_64::instructions::interrupts::enable();

    crate::arch::drivers::time::early_init();

    crate::arch::drivers::acpi::init();
    crate::arch::drivers::time::init();
    crate::arch::drivers::keyboard::init();
    crate::arch::drivers::mouse::init();

    println!();
    print_fill!("-");
    println!();

    info!("up and running");

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

    let mut scheduler = Scheduler::new();
    scheduler.spawn(Task::new(
        crate::arch::drivers::keyboard::handle_keypresses(),
    ));
    scheduler.spawn(Task::new(async move {
        let cpu_freq = measure_cpu_frequency().await;
        if cpu_freq == 0 {
            panic!("failed to measure cpu frequency");
        }
        unsafe {
            CPU_FREQ.store(cpu_freq, Ordering::Relaxed);
            crate::arch::drivers::time::tsc::TSC_TIMER
                .get_mut()
                .unwrap()
                .set_supported(true);
        }
        info!("rocking a {}", crate::arch::system::cpuid::get_cpu());
        info!(
            "cpu frequency - {}.{:03}GHz",
            cpu_freq / 1_000_000_000,
            (cpu_freq % 1_000_000_000) / 1_000_000,
        );

        // for now
        print!("$ ");
    }));
    scheduler.run();
    // loop {}
}

// async fn simple_task() {
//     for i in 0..5 {
//         info!("Task running iteration: {}", i);

//         crate::arch::drivers::time::TimerFuture::new(10).await;
//     }
//     info!("Task completed!");
// }

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
        // unnecessary but might change in the future
        let msg = info.message().to_string();
        if let Some(location) = info.location() {
            println!(
                "~\tERROR: panicked at {}:{}:{} {}{}\n~\t",
                location.file(),
                location.line(),
                location.column(),
                if msg.is_empty() {
                    "without a message."
                } else {
                    "with message: "
                },
                msg,
            );
        } else {
            println!("~\tERROR: panicked with message: {}\n~\t", info.message(),);
        }
        #[cfg(debug_assertions)]
        {
            let mut rbp: *const StackFrame;
            unsafe {
                core::arch::asm!("mov {}, rbp", out(reg) rbp);
            }
            let mut i = 0;
            while let Some(frame) = unsafe { rbp.as_ref() } {
                println!("~\tframe {}: rip = {:#x}", i, frame.rip);
                rbp = frame.rbp;
                i += 1;

                if i > 64 {
                    break;
                }
            }
        }
        println!("~\tstopping code execution and dumping registers\n~\t");
        let registers = crate::arch::system::cpu::read_registers();
        println!(
            "~\tr15:    0x{0:016X}  -  rsi:    0x{10:016X}\n\
             ~\tr14:    0x{1:016X}  -  rdx:    0x{11:016X}\n\
             ~\tr13:    0x{2:016X}  -  rcx:    0x{12:016X}\n\
             ~\tr12:    0x{3:016X}  -  rbx:    0x{13:016X}\n\
             ~\tr11:    0x{4:016X}  -  rax:    0x{14:016X}\n\
             ~\tr10:    0x{5:016X}  -  rip:    0x{15:016X}\n\
             ~\tr9:     0x{6:016X}  -  cs:     0x{16:016X}\n\
             ~\tr8:     0x{7:016X}  -  rflags: 0x{17:016X}\n\
             ~\trbp:    0x{8:016X}  -  rsp:    0x{18:016X}\n\
             ~\trdi:    0x{9:016X}  -  ss:     0x{19:016X}\n\
             ~\tcr2:    0x{20:016X}  -  cr3:    0x{21:016X}\n~",
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
