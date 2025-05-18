/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{
    arch::drivers::keyboard::keyboard_thread,
    device::serial::serial_thread,
    print_centered,
    utils::shell::{cursor_thread, shell_thread},
};
use alloc::{format, string::ToString, vec::Vec};
use core::{
    panic::PanicInfo,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    NOOO, info,
    memory::get_usable_memory,
    print, print_fill, println, scheduler,
    utils::{
        asm::halt_loop,
        limine::{get_bootloader_info, get_framebuffers},
    },
};

pub mod drivers;
pub mod gdt;
pub mod interrupts;
pub mod system;

pub static CPU_FREQ: AtomicU64 = AtomicU64::new(0);

#[cfg(debug_assertions)]
#[repr(C)]
struct StackTrace {
    rbp: *const StackTrace,
    rip: usize,
}

pub fn _start() -> ! {
    crate::device::serial::init();
    println!("\n{NOOO}\n");
    info!("x86_64 kernel starting...\n");

    crate::memory::init();
    self::system::cpu::init_bsp();
    self::gdt::init();
    self::interrupts::init();
    self::interrupts::pic::init();

    crate::utils::asm::toggle_ints(true);

    self::drivers::time::init();

    crate::drivers::fs::init();
    crate::drivers::acpi::init();

    #[cfg(feature = "uacpi_test")]
    crate::drivers::acpi::shutdown();

    self::drivers::time::init_hpet();
    self::system::cpu::init();
    self::drivers::mouse::init();
    crate::device::pci::pci_enumerate();

    #[cfg(feature = "tests")]
    crate::tests::init();

    scheduler::init();
    scheduler::thread::spawn(
        scheduler::get_scheduler().pid0.get().unwrap(),
        main_thread as usize,
        "main",
    );
    scheduler::start();
}

pub fn main_thread() -> ! {
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

    let rtc_time = self::drivers::time::rtc::RtcTime::current()
        .with_timezone_offset(
            crate::utils::config::get_config()
                .timezone_offset
                .to_int()
                .clamp(-720, 840) as i16,
        )
        .adjusted_for_timezone();

    info!(
        "{} | {}",
        rtc_time.datetime_pretty(),
        rtc_time.timezone_pretty()
    );

    info!("rocking a(n) {}", crate::utils::asm::get_cpu());
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

    let sched = scheduler::get_scheduler();

    let pid0 = sched.pid0.get().unwrap();

    scheduler::thread::spawn(pid0, keyboard_thread as usize, "keyboard");
    scheduler::thread::spawn(pid0, serial_thread as usize, "serial");

    let shell_pid = scheduler::spawn_process(
        unsafe { crate::memory::vmm::PAGEMAP.get().unwrap() },
        "shell",
    );
    let shell_proc = sched
        .processes
        .iter()
        .find(|p| p.lock().get_pid() == shell_pid)
        .unwrap();
    scheduler::thread::spawn(shell_proc, shell_thread as usize, "main");
    scheduler::thread::spawn(shell_proc, cursor_thread as usize, "cursor");

    halt_loop()
}

pub fn _panic(info: &PanicInfo) -> ! {
    #[cfg(feature = "tests")]
    {
        println!("[failed]\n");
        println!("{info}\n");
        crate::drivers::acpi::shutdown();
    }
    #[cfg(not(feature = "tests"))]
    {
        print!("{}", crate::utils::logger::color::RED);
        print_centered!(NOOO);
        println!();
        print_fill!("~", "Kernel Panic");
        print_centered!("", "~");
        // unnecessary but might change in the future
        let msg = info.message().to_string();
        if let Some(location) = info.location() {
            print_centered!(
                format!(
                    "ERROR: panicked at {}:{}:{} {}{}\n",
                    location.file(),
                    location.line(),
                    location.column(),
                    if msg.is_empty() {
                        "without a message."
                    } else {
                        "with message: "
                    },
                    msg,
                )
                .as_str(),
                "~"
            );
        } else {
            print_centered!(
                format!("ERROR: panicked with message: {}\n", info.message()).as_str(),
                "~"
            );
        }
        #[cfg(debug_assertions)]
        {
            let mut rbp: *const StackTrace;
            unsafe {
                core::arch::asm!("mov {}, rbp", out(reg) rbp);
            }
            let mut i = 0;
            while let Some(frame) = unsafe { rbp.as_ref() } {
                print_centered!(
                    format!("frame {}: rip = 0x{:016x}", i, frame.rip).as_str(),
                    "~"
                );
                rbp = frame.rbp;
                i += 1;

                if i > 64 {
                    break;
                }
            }
        }
        print_centered!("\nstopping code execution and dumping registers\n", "~");
        let registers = crate::utils::asm::dump_regs();
        print_centered!(
            format!(
                "r15:    0x{0:016X}  rsi:    0x{10:016X}\n\
                 r14:    0x{1:016X}  rdx:    0x{11:016X}\n\
                 r13:    0x{2:016X}  rcx:    0x{12:016X}\n\
                 r12:    0x{3:016X}  rbx:    0x{13:016X}\n\
                 r11:    0x{4:016X}  rax:    0x{14:016X}\n\
                 r10:    0x{5:016X}  rip:    0x{15:016X}\n\
                 r9:     0x{6:016X}  cs:     0x{16:016X}\n\
                 r8:     0x{7:016X}  rflags: 0x{17:016X}\n\
                 rbp:    0x{8:016X}  rsp:    0x{18:016X}\n\
                 rdi:    0x{9:016X}  ss:     0x{19:016X}\n\
                 cr2:    0x{20:016X}  cr3:    0x{21:016X}\n",
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
            )
            .as_str(),
            "~"
        );
        print_fill!("~", "", false);
        print!("{}", crate::utils::logger::color::RESET);
        crate::utils::asm::toggle_ints(false);
    }
    halt_loop()
}
