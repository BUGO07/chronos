/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

#[cfg(target_arch = "x86_64")]
pub mod cmem;
pub mod config;
pub mod limine;
pub mod logger;
pub mod term;

pub fn halt_loop() -> ! {
    loop {
        #[cfg(target_arch = "x86_64")]
        x86_64::instructions::hlt();
        #[cfg(target_arch = "aarch64")]
        aarch64::instructions::halt();
    }
}

#[inline(always)]
pub const fn align_up(addr: u64, align: u64) -> u64 {
    debug_assert!(align.is_power_of_two());
    (addr + align - 1) & !(align - 1)
}

#[inline(always)]
pub const fn align_down(addr: u64, align: u64) -> u64 {
    debug_assert!(align.is_power_of_two());
    addr & !(align - 1)
}
