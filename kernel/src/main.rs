#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

pub mod arch;
pub mod memory;
pub mod task;
pub mod utils;

#[cfg(feature = "tests")]
pub mod tests;

extern crate alloc;

use core::panic::PanicInfo;
use limine::BaseRevision;
use limine::request::{DateAtBootRequest, RequestsEndMarker, RequestsStartMarker};
use task::Task;
use task::executor::Executor;
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

#[used]
#[unsafe(link_section = ".requests")]
static BOOT_DATE: DateAtBootRequest = DateAtBootRequest::new();

#[unsafe(no_mangle)]
unsafe extern "C" fn kmain() -> ! {
    assert!(BASE_REVISION.is_supported());

    info!("x86_64 kernel starting...\n");

    arch::gdt::init();
    arch::interrupts::init_idt();
    arch::drivers::pic::init();
    arch::drivers::pit::init();
    memory::init();

    println!("\n--------------------------------------\n");

    debug!("debug rahh");
    info!("up and running");

    let (years, months, days, hours, minutes, seconds) =
        utils::time::unix_to_date(BOOT_DATE.get_response().unwrap().timestamp());
    info!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        years, months, days, hours, minutes, seconds
    );

    #[cfg(feature = "tests")]
    tests::init();

    let mut executor = Executor::new();
    executor.spawn(Task::new(task::keyboard::handle_keypresses()));
    executor.run();

    // halt_loop();
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
