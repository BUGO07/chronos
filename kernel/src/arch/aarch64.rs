/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::vec::Vec;

use crate::{
    NOOO, error, info,
    memory::get_usable_memory,
    print_fill, println,
    scheduler::{Scheduler, Task},
    utils::{
        asm::halt_loop,
        limine::{get_bootloader_info, get_framebuffers},
    },
};
use core::panic::PanicInfo;

pub mod drivers;
pub mod interrupts;

pub fn _start() -> ! {
    println!("\n{NOOO}\n");
    info!("aarch64 kernel starting...\n");
    self::drivers::time::early_init();
    crate::memory::init();
    self::interrupts::init();
    self::interrupts::gic::init();
    self::drivers::time::init();
    // crate::drivers::acpi::init(); // TODO: implement mmu for acpi to work

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

    info!("rocking a(n) {}", crate::utils::asm::get_cpu());

    let memory_bytes = get_usable_memory();
    let gib = memory_bytes / (1024 * 1024 * 1024);
    let gremainder = memory_bytes % (1024 * 1024 * 1024);
    let gdecimal = (gremainder * 100) / (1024 * 1024 * 1024);

    info!("usable memory - {}.{:02}GiB", gib, gdecimal);

    let mut scheduler = Scheduler::new();
    scheduler.spawn(Task::new(async {
        info!("icl ts pmo ♥");
    }));
    scheduler.run();
}

pub fn _panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    halt_loop();
}
