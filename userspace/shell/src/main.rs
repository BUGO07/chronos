#![no_std]
#![no_main]

use core::ffi::{CStr, c_char};
use std::{
    elf::{get_pid, get_tid, sleep_ns},
    heapless::HeaplessVec,
    println,
};

use alloc::vec::Vec;

extern crate alloc;
extern crate std;

#[unsafe(no_mangle)]
fn main(args: Vec<&str>) -> u64 {
    println!("shell bruh with args {args:?}");
    println!("clock reads at {}", std::elf::time_ns());
    println!("sleeping thread {}/{} for 500ms", get_pid(), get_tid());
    sleep_ns(500_000_000);
    println!("by my calculations, sleeping for another 500ms");
    for _ in 0..(500 / 6) {
        // 6ms timeslice
        std::syscall!(std::syscalls::SCHED_YIELD);
    }
    // execve("/bin/echo", &["tralalelo", "tralala"], &[]);
    println!("woke up from sleep");

    let args: &mut [*const c_char; 256] = &mut [core::ptr::null(); 256];
    args[0] = c"hello".as_ptr();
    args[1] = c"tralalelo".as_ptr();
    let env: &mut [*const c_char; 256] = &mut [core::ptr::null(); 256];
    env[0] = c"bye".as_ptr();
    env[1] = c"tralala".as_ptr();
    let file = c"/bin/echo";

    let pid = std::syscall!(std::syscalls::FORK);

    if pid == 0 {
        println!(
            "this is printed on the child thread, the current pid is {}",
            get_pid()
        );
    } else {
        println!("this is printed on the parent thread, child pid is {}", pid);
    }

    println!("shice...");
    println!("shice...");
    println!("shice...");
    println!("shice...");

    println!("sleeping thread {}/{} for 500ms", get_pid(), get_tid());
    sleep_ns(500_000_000);
    loop {}
    0
}

#[unsafe(no_mangle)]
extern "C" fn echo(argc: u64, argv: *const *const c_char) -> ! {
    let args = unsafe {
        core::slice::from_raw_parts(argv, argc as usize)
            .iter()
            .map(|x| CStr::from_ptr(*x).to_str().unwrap())
            .collect::<Vec<_>>()
    };
    println!("rahh - {}", args.join(" "));
    std::elf::exit(1)
}
