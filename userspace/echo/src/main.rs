#![no_std]
#![no_main]

use std::println;

use alloc::vec::Vec;

extern crate alloc;
extern crate std;

#[unsafe(no_mangle)]
fn main(args: Vec<&str>) -> u64 {
    println!("{}", args.join(" "));
    0
}
