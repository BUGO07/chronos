/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{
    arch::drivers::keyboard::keyboard_thread,
    print_centered,
    utils::shell::{cursor_thread, shell_thread},
};
use alloc::{format, vec::Vec};
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
    {
        let pci_devices = crate::device::pci::PCI_DEVICES.lock();
        info!("Found {} PCI devices:", pci_devices.len());
        for device in pci_devices.iter() {
            info!(
                "{:02x}:{:02x}:{} {} {:04X}:{:04X} [0x{:X}:0x{:X}:0x{:X}]",
                device.address.bus,
                device.address.device,
                device.address.function,
                device.name,
                device.vendor_id,
                device.device_id,
                device.class_code,
                device.subclass,
                device.prog_if,
            );
        }
    }
    crate::device::nvme::init();

    #[cfg(feature = "tests")]
    crate::tests::init();

    scheduler::init();
    scheduler::thread::spawn(
        scheduler::get_proc_by_pid(0).unwrap(),
        main_thread as *const (),
        "main",
        false,
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
        info!("display {}: size - {}x{}", i + 1, fb.width, fb.height);
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
        rtc_time.timezone_pretty(),
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

    let pid0 = scheduler::get_proc_by_pid(0).unwrap();

    scheduler::thread::spawn(pid0, keyboard_thread as *const (), "keyboard", false);
    // scheduler::thread::spawn(pid0, serial_thread as *const (), "serial", false);

    let shell_pid = scheduler::spawn_process(
        unsafe { crate::memory::vmm::PAGEMAP.get().unwrap() },
        "shell",
    );
    let shell_proc = scheduler::get_proc_by_pid(shell_pid).unwrap();
    scheduler::thread::spawn(shell_proc, shell_thread as *const (), "main", false);
    scheduler::thread::spawn(shell_proc, cursor_thread as *const (), "cursor", false);

    // * loading userspace ELF binary
    let elf_data = include_bytes!("../../res/test.elf");
    let elf_proc_pid = scheduler::spawn_process(
        unsafe { crate::memory::vmm::PAGEMAP.get().unwrap() },
        "test_elf",
    );
    let elf_proc = scheduler::get_proc_by_pid(elf_proc_pid).unwrap();
    let entry = {
        let pagemap_ref = unsafe { crate::memory::vmm::PAGEMAP.get().unwrap() };
        let mut pagemap = pagemap_ref.lock();
        match crate::utils::elf::load_elf(elf_data, &mut pagemap) {
            Ok(elf_info) => {
                info!("loaded ELF binary, entry=0x{:X}", elf_info.entry);
                elf_info.entry as usize
            }
            Err(e) => {
                info!("failed to load ELF: {}", e);
                0
            }
        }
    };
    if entry != 0 {
        scheduler::thread::spawn(elf_proc, entry as *const (), "elf_main", true);
    }

    halt_loop()
}

pub fn _panic(info: &PanicInfo) -> ! {
    crate::utils::asm::toggle_ints(false);
    #[cfg(feature = "tests")]
    {
        println!("[failed]\n");
        println!("{info}\n");
        crate::drivers::acpi::shutdown();
    }
    #[cfg(not(feature = "tests"))]
    {
        // print!("\x1b[2J{}", crate::utils::logger::color::RED);
        // print_centered!(NOOO);
        // println!();
        print_fill!("~", "Kernel Panic");
        print_centered!("", "~");
        print_centered!(format!("ERROR: {info}\n").as_str(), "~");
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
        println!();
        print_fill!("~", "", false);
        print!("{}", crate::utils::logger::color::RESET);
    }
    halt_loop()
}
