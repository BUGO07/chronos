/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::vec::Vec;

use crate::{
    NOOO, error, info,
    memory::get_usable_memory,
    print_fill, println,
    task::scheduler::Scheduler,
    utils::{
        halt_loop,
        limine::{get_bootloader_info, get_framebuffers},
    },
};
use core::panic::PanicInfo;

pub mod device;
pub mod drivers;

pub fn _start() -> ! {
    println!("\n{NOOO}\n");
    info!("x86_64 kernel starting...\n");

    crate::memory::init();

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

    let memory_bytes = get_usable_memory();
    let gib = memory_bytes / (1024 * 1024 * 1024);
    let gremainder = memory_bytes % (1024 * 1024 * 1024);
    let gdecimal = (gremainder * 100) / (1024 * 1024 * 1024);

    info!("usable memory - {}.{:02}GiB", gib, gdecimal);

    info!("icl ts pmo ♥");
    let mut scheduler = Scheduler::new();
    scheduler.run();
}

pub fn _panic(info: &PanicInfo) -> ! {
    error!("{}", info);
    halt_loop();
}
