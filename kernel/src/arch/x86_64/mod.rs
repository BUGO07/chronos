/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::{string::ToString, vec::Vec};
use core::{
    panic::PanicInfo,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    NOOO, debug, info,
    memory::get_usable_memory,
    print, print_fill, println,
    task::{Task, scheduler::Scheduler},
    utils::{
        halt_loop,
        limine::{get_bootloader_info, get_framebuffers},
    },
};

pub mod device;
pub mod drivers;
pub mod gdt;
pub mod interrupts;
pub mod shell;
pub mod system;

pub static CPU_FREQ: AtomicU64 = AtomicU64::new(0);

#[cfg(debug_assertions)]
#[repr(C)]
struct StackFrame {
    rbp: *const StackFrame,
    rip: usize,
}

pub fn _start() -> ! {
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

    #[cfg(feature = "uacpi_test")]
    crate::arch::drivers::acpi::shutdown();

    crate::arch::drivers::time::init();
    crate::arch::device::pci::pci_enumerate();

    crate::arch::drivers::mouse::init();

    println!();
    print_fill!("-");
    println!();

    info!("up and running");

    let bootloader_info = get_bootloader_info();
    info!(
        "bootloader info - {} {}",
        bootloader_info.name(),
        bootloader_info.version(),
    );

    let framebuffers = get_framebuffers().collect::<Vec<_>>();
    info!("found {} display(s):", framebuffers.len());
    for (i, fb) in framebuffers.iter().enumerate() {
        info!("display {}: size - {}x{}", i + 1, fb.width(), fb.height());
    }

    // ! icl ts pmo sm its causing sometimes gpf sometimes pagefault on reboot
    // let config = crate::utils::config::get_config();

    let rtc_time = crate::arch::drivers::time::rtc::RtcTime::current()
        .with_timezone_offset(crate::utils::config::ZONE_OFFSET) // change me
        .adjusted_for_timezone();

    info!(
        "{} | {}",
        rtc_time.datetime_pretty(),
        rtc_time.timezone_pretty()
    );

    #[cfg(feature = "tests")]
    crate::tests::init();

    // crate::arch::shell::run_command("clear", Vec::new());

    // info!("cpu freq - {}", crate::arch::system::cpuid::get_freq());

    info!("rocking a {}", crate::arch::system::cpuid::get_cpu());
    let cpu_freq = CPU_FREQ.load(Ordering::Relaxed);
    info!(
        "cpu frequency - {}.{:03}GHz",
        cpu_freq / 1_000_000_000,
        (cpu_freq % 1_000_000_000) / 1_000_000,
    );

    let memory_bytes = get_usable_memory();
    let gib = memory_bytes / (1024 * 1024 * 1024);
    let gremainder = memory_bytes % (1024 * 1024 * 1024);
    let gdecimal = (gremainder * 100) / (1024 * 1024 * 1024);

    info!("usable memory - {}.{:02}GiB", gib, gdecimal);

    // let reserved_bytes = get_reserved_memory();
    // let mib = reserved_bytes / (1024 * 1024);
    // let mremainder = reserved_bytes % (1024 * 1024);
    // let mdecimal = (mremainder * 100) / (1024 * 1024);
    // info!("reserved memory - {}.{:02}MiB", mib, mdecimal);

    info!("icl ts pmo ♥");

    let mut scheduler = Scheduler::new();
    scheduler.spawn(Task::new(
        crate::arch::drivers::keyboard::handle_keypresses(),
    ));
    scheduler.spawn(Task::new(crate::arch::shell::shell_task()));
    // scheduler.spawn(Task::new(async {
    //     let cpu_freq = measure_cpu_frequency_async().await;
    //     if cpu_freq == 0 {
    //         panic!("failed to measure cpu frequency");
    //     }
    //     info!(
    //         "cpu frequency - {}.{:03}GHz",
    //         cpu_freq / 1_000_000_000,
    //         (cpu_freq % 1_000_000_000) / 1_000_000,
    //     );
    // }));
    scheduler.run()
    // loop {}
}

pub fn _panic(info: &PanicInfo) -> ! {
    #[cfg(feature = "tests")]
    {
        println!("[failed]\n");
        println!("{info}\n");
        crate::arch::drivers::acpi::shutdown();
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
