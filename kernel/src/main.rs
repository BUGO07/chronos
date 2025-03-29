#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]
#![feature(sync_unsafe_cell)]
#![allow(
    clippy::missing_safety_doc,
    clippy::new_without_default,
    clippy::zero_ptr
)]

pub mod arch;
pub mod memory;
pub mod task;
pub mod utils;

#[cfg(feature = "tests")]
pub mod tests;

extern crate alloc;

use arch::time::print_time;
use core::panic::PanicInfo;
use limine::BaseRevision;
use limine::request::{RequestsEndMarker, RequestsStartMarker};
// use task::executor::Executor;
use utils::halt_loop;

#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".requests_start_marker")]
static _START_MARKER: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".requests_end_marker")]
static _END_MARKER: RequestsEndMarker = RequestsEndMarker::new();
#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    assert!(BASE_REVISION.is_supported());

    info!("starting kernel...\n");

    arch::gdt::init();
    arch::interrupts::init_idt();
    arch::drivers::pic::init();
    memory::init();

    println!("\n--------------------------------------\n");

    info!("up and running");

    // let mut executor = Executor::new();
    // executor.run();

    #[cfg(feature = "tests")]
    tests::init();

    halt_loop();
}

async fn printstuff() {
    print_time();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    #[cfg(feature = "tests")]
    {
        println!("[failed]\n");
        println!("{}\n", info);
        utils::exit_qemu(utils::QemuExitCode::Failed);
    }
    #[cfg(not(feature = "tests"))]
    {
        error!("Kernel panic: {}", info);
    }
    x86_64::instructions::interrupts::disable();
    halt_loop()
}
