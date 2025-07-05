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
    clippy::manual_dangling_ptr,
    clippy::not_unsafe_ptr_arg_deref,
    clippy::too_many_arguments,
    clippy::while_immutable_condition
)]

extern crate alloc;

use core::panic::PanicInfo;

pub mod arch;
pub mod device;
pub mod drivers;
pub mod utils;

#[unsafe(no_mangle)]
extern "C" fn kmain() -> ! {
    arch::_start()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    arch::_panic(info)
}
