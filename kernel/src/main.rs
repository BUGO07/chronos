/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

#![no_std]
#![no_main]
#![allow(
    static_mut_refs,
    clippy::new_ret_no_self,
    clippy::missing_safety_doc,
    clippy::single_match,
    clippy::manual_dangling_ptr,
    clippy::empty_loop,
    clippy::not_unsafe_ptr_arg_deref
)]
#![cfg_attr(
    feature = "tests",
    allow(unused_imports, unused_variables, dead_code, unused_mut)
)]

extern crate alloc;

use core::panic::PanicInfo;

pub const NOOO: &str = include_str!("../res/nooo.txt");

pub mod arch;
pub mod device;
pub mod drivers;
pub mod memory;
pub mod scheduler;
pub mod utils;

#[cfg(feature = "tests")]
pub mod tests;

#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    arch::_start()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    arch::_panic(info)
}
