/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{
    NOOO,
    asm::{halt_loop, stack_trace},
    info,
    memory::get_usable_memory,
    print, print_centered, print_fill, println,
    utils::bootloader::{get_bootloader_info, get_framebuffers},
};
use alloc::{format, string::ToString, vec::Vec};
use core::{
    panic::PanicInfo,
    sync::atomic::{AtomicU64, Ordering},
};
use drivers::keyboard::keyboard_thread;

use crate::arch::system::elf::load_elf;

pub mod drivers;
pub mod gdt;
pub mod interrupts;
pub mod pic;
pub mod sched;
pub mod system;

pub static CPU_FREQ: AtomicU64 = AtomicU64::new(0);

#[inline(always)]
pub fn _start() -> ! {
    println!("\n{NOOO}\n");
    info!("starting chronos...\n");

    crate::memory::init();
    crate::utils::term::init();
    self::system::cpu::init_bsp();
    self::gdt::init();
    self::interrupts::init();
    self::pic::init();

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
    let pci_devices = unsafe { &crate::device::pci::PCI_DEVICES };
    info!("Found {} PCI devices:", pci_devices.len());
    for device in pci_devices {
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
    crate::device::nvme::init();

    self::sched::init();
    self::thread::spawn(
        self::sched::get_proc_by_pid(0).unwrap(),
        main_thread as usize,
        "main",
        false,
    );
    self::sched::start();
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
            // crate::utils::config::get_config()
            //     .timezone_offset
            //     .to_int()
            //     .clamp(-720, 840) as i16,
            240,
        )
        .adjusted_for_timezone();
    info!(
        "{} | {}",
        rtc_time.datetime_pretty(),
        rtc_time.timezone_pretty(),
    );

    info!("rocking a(n) {}", std::asm::get_cpu());
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

    let pid0 = std::sched::get_proc_by_pid(0).unwrap();

    std::thread::spawn(pid0, keyboard_thread as usize, "keyboard", false);
    // scheduler::thread::spawn(pid0, serial_thread as usize, "serial", false);

    let shell_pid = std::sched::spawn_process("shell");
    let shell_proc = std::sched::get_proc_by_pid(shell_pid).unwrap();
    // scheduler::thread::spawn(shell_proc, shell_thread as usize, "main", false);
    // scheduler::thread::spawn(shell_proc, cursor_thread as usize, "cursor", false);

    // // let buffer = include_bytes!("../../res/userspace.elf");
    // // let entry_point = load_elf(buffer);

    let bytes = include_bytes!("../../../userspace/elfs/shell");

    info!("bytes - {:?}", bytes.len());

    // let elf = load_elf(bytes, shell_proc.lock().pagemap.clone()).unwrap() as usize;
    unsafe { shell_proc.force_unlock() };
    std::thread::spawn(
        shell_proc,
        load_elf(bytes, shell_proc.lock().pagemap.clone()).unwrap() as usize,
        "main",
        false,
    );

    halt_loop()
}

// static mut COUNT: u64 = 0;
// static mut SHICE: u64 = 0;

// extern "C" fn shice() -> ! {
//     loop {
//         unsafe {
//             COUNT += 1;
//             if COUNT < 2000 {
//                 std::thread::spawn(
//                     std::sched::get_proc_by_pid(std::sched::spawn_process("rice")).unwrap(),
//                     shice as usize,
//                     "main",
//                     false,
//                 );
//             }
//             SHICE += 1;
//         }
//     }
// }

#[inline(always)]
pub fn _panic(info: &PanicInfo) -> ! {
    std::asm::toggle_ints(false);
    print!("\x1b[2J{}", std::kernel::logger::color::RED);
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
    stack_trace();
    print_centered!("\nstopping code execution and dumping registers\n", "~");
    let registers = std::asm::dump_regs();
    print_centered!(
        format!(
            "r15:    0x{0:016x}  rsi:    0x{10:016x}\n\
                 r14:    0x{1:016x}  rdx:    0x{11:016x}\n\
                 r13:    0x{2:016x}  rcx:    0x{12:016x}\n\
                 r12:    0x{3:016x}  rbx:    0x{13:016x}\n\
                 r11:    0x{4:016x}  rax:    0x{14:016x}\n\
                 r10:    0x{5:016x}  rip:    0x{15:016x}\n\
                 r9:     0x{6:016x}  cs:     0x{16:016x}\n\
                 r8:     0x{7:016x}  rflags: 0x{17:016x}\n\
                 rbp:    0x{8:016x}  rsp:    0x{18:016x}\n\
                 rdi:    0x{9:016x}  ss:     0x{19:016x}\n\
                 cr2:    0x{20:016x}  cr3:    0x{21:016x}\n",
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
    print!("{}", std::kernel::logger::color::RESET);

    std::asm::halt_loop();
}
