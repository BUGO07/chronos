/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::{format, string::ToString, vec::Vec};

use crate::{
    NOOO,
    device::serial::serial_task,
    info,
    memory::get_usable_memory,
    print, print_centered, print_fill, println,
    scheduler::{Scheduler, Task},
    utils::{
        asm::halt_loop,
        limine::{get_bootloader_info, get_device_tree, get_framebuffers},
        shell::shell_task,
    },
    warn,
};
use core::panic::PanicInfo;

pub mod drivers;
pub mod interrupts;

pub fn _start() -> ! {
    crate::device::serial::init();
    println!("\n{NOOO}\n");
    info!("aarch64 kernel starting...\n");
    self::drivers::time::early_init();
    crate::memory::init();
    self::interrupts::init();
    self::interrupts::gic::init();
    self::drivers::time::init();
    crate::drivers::fs::init();
    // crate::drivers::acpi::init(); // TODO: implement mmu for acpi to work

    if let Some(dtb) = get_device_tree() {
        info!("device tree found at address {:?}", dtb);
    } else {
        warn!("bootloader couldn't provide a device tree blob");
    }

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
    // reversed order lmao
    scheduler.spawn(Task::new(serial_task()));
    scheduler.spawn(Task::new(shell_task()));
    scheduler.spawn(Task::new(async {
        info!("icl ts pmo ♥");
    }));
    scheduler.run();
}

#[cfg(debug_assertions)]
#[repr(C)]
struct StackTrace {
    fp: *const StackTrace,
    lr: usize,
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
            let mut fp: *const StackTrace;
            unsafe {
                core::arch::asm!("mov {}, fp", out(reg) fp);
            }
            let mut i = 0;
            while let Some(frame) = unsafe { fp.as_ref() } {
                print_centered!(
                    format!("frame {}: lr = 0x{:016x}", i, frame.lr).as_str(),
                    "~"
                );
                fp = frame.fp;
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
                "x0:   0x{0:016X}  x16:  0x{16:016X}\n\
                 x1:   0x{1:016X}  x17:  0x{17:016X}\n\
                 x2:   0x{2:016X}  x18:  0x{18:016X}\n\
                 x3:   0x{3:016X}  x19:  0x{19:016X}\n\
                 x4:   0x{4:016X}  x20:  0x{20:016X}\n\
                 x5:   0x{5:016X}  x21:  0x{21:016X}\n\
                 x6:   0x{6:016X}  x22:  0x{22:016X}\n\
                 x7:   0x{7:016X}  x23:  0x{23:016X}\n\
                 x8:   0x{8:016X}  x24:  0x{24:016X}\n\
                 x9:   0x{9:016X}  x25:  0x{25:016X}\n\
                 x10:  0x{10:016X}  x26:  0x{26:016X}\n\
                 x11:  0x{11:016X}  x27:  0x{27:016X}\n\
                 x12:  0x{12:016X}  x28:  0x{28:016X}\n\
                 x13:  0x{13:016X}  fp:   0x{29:016X}\n\
                 x14:  0x{14:016X}  lr:   0x{30:016X}\n\
                 x15:  0x{15:016X}  sp:   0x{31:016X}\n\
                 pc:   0x{32:016X}\n",
                registers.x0,
                registers.x1,
                registers.x2,
                registers.x3,
                registers.x4,
                registers.x5,
                registers.x6,
                registers.x7,
                registers.x8,
                registers.x9,
                registers.x10,
                registers.x11,
                registers.x12,
                registers.x13,
                registers.x14,
                registers.x15,
                registers.x16,
                registers.x17,
                registers.x18,
                registers.x19,
                registers.x20,
                registers.x21,
                registers.x22,
                registers.x23,
                registers.x24,
                registers.x25,
                registers.x26,
                registers.x27,
                registers.x28,
                registers.fp,
                registers.lr,
                registers.sp,
                registers.pc
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
