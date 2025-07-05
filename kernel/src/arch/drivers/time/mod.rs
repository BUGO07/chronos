/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

pub mod hpet;
pub mod kvm;
pub mod pit;
pub mod rtc;
pub mod tsc;

pub fn init() {
    pit::init();
    kvm::init();
    tsc::init();
    crate::arch::system::lapic::init();
}

pub fn init_hpet() {
    hpet::init();
}
