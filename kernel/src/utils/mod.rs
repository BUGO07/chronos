/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

pub mod asm;
pub mod config;
pub mod limine;
pub mod logger;
pub mod term;
pub mod time;

#[inline(always)]
pub const fn align_up(addr: u64, align: u64) -> u64 {
    assert!(align.is_power_of_two());
    (addr + align - 1) & !(align - 1)
}

#[inline(always)]
pub const fn align_down(addr: u64, align: u64) -> u64 {
    assert!(align.is_power_of_two());
    addr & !(align - 1)
}
