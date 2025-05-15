/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

#[cfg(target_arch = "x86_64")]
pub mod preemptive;
#[cfg(target_arch = "x86_64")]
pub use preemptive::*;
#[cfg(target_arch = "x86_64")]
pub mod thread;

pub mod cooperative;
#[cfg(target_arch = "aarch64")]
pub use cooperative::*;
