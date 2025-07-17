/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use x86_64::*;
