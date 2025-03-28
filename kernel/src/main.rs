#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(sync_unsafe_cell)]
#![allow(
    clippy::missing_safety_doc,
    clippy::new_without_default,
    clippy::zero_ptr
)]

// pub mod keyboard;
pub mod arch;
pub mod memory;
pub mod utils;

#[cfg(feature = "tests")]
pub mod tests;

extern crate alloc;

use core::panic::PanicInfo;
use limine::BaseRevision;
use limine::request::{HhdmRequest, MemoryMapRequest, RequestsEndMarker, RequestsStartMarker};
use x86_64::VirtAddr;

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();

#[unsafe(link_section = ".requests")]
static MEMMAP_REQUEST: MemoryMapRequest = MemoryMapRequest::new();

#[unsafe(link_section = ".requests")]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::new();

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    assert!(BASE_REVISION.is_supported());

    // println!("\n-----------------------------------\n");
    info!("Starting kernel...\n");
    // println!("\n-----------------------------------\n");

    info!("Initializing GDT and TSS...");
    arch::gdt::init();
    info!("Initialized GDT and TSS.");

    println!();
    info!("Setting up interrupts...\n");

    info!("Initializing IDT...");
    arch::interrupts::init_idt();
    info!("Initialized IDT.\n");

    info!("Initializing PIC driver...");
    unsafe { arch::drivers::pic::init() };
    info!("Initialized PIC driver.\n");

    info!("Enabling interrupts... (`sti` instruction)\n");
    x86_64::instructions::interrupts::enable();

    info!("Interrupts are set up.\n");

    info!("Initializing memory...");
    let hhdm_offset = HHDM_REQUEST.get_response().unwrap().offset();
    let entries = MEMMAP_REQUEST.get_response().unwrap().entries();
    let phys_mem_offset = VirtAddr::new(hhdm_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { memory::BootInfoFrameAllocator::init(entries) };

    memory::allocator::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
    info!("Memory initialized.");

    #[cfg(feature = "tests")]
    tests::init();

    println!("\n--------------------------------------\n");

    info!("Up and running!");

    halt_loop()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    #[cfg(feature = "tests")]
    {
        println!("[failed]\n");
        error!("{}\n", info);
        utils::exit_qemu(utils::QemuExitCode::Failed);
    }
    #[cfg(not(feature = "tests"))]
    {
        error!("Kernel panic: {}", info);
    }
    loop {
        x86_64::instructions::interrupts::disable();
        x86_64::instructions::hlt();
    }
}

pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}
