/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{arch::drivers::time::pit::current_pit_ticks, print, utils::time::KernelTimer};

pub fn pit_timer() {
    let time = current_pit_ticks();
    loop {
        if time + 200 < current_pit_ticks() {
            break;
        }
    }
}

pub fn kvm_timer() {
    let kvm = unsafe {
        crate::arch::drivers::time::kvm::KVM_TIMER
            .get()
            .expect("couldnt get kvm")
    };
    if !kvm.is_supported() {
        return print!("[kvm not supported] ");
    }
    let time = kvm.elapsed_ns();
    loop {
        if time + 200_000_000 < kvm.elapsed_ns() {
            break;
        }
    }
}

pub fn tsc_timer() {
    let tsc = unsafe {
        crate::arch::drivers::time::tsc::TSC_TIMER
            .get()
            .expect("couldnt get tsc")
    };
    if !tsc.is_supported() {
        return print!("[tsc not supported] ");
    }
    let time = tsc.elapsed_ns();
    loop {
        if time + 200_000_000 < tsc.elapsed_ns() {
            break;
        }
    }
}

pub fn hpet_timer() {
    let hpet = unsafe {
        crate::arch::drivers::time::hpet::HPET_TIMER
            .get()
            .expect("couldnt get hpet")
    };
    if !hpet.is_supported() {
        return print!("[hpet not supported] ");
    }
    let time = hpet.elapsed_ns();
    loop {
        if time + 200_000_000 < hpet.elapsed_ns() {
            break;
        }
    }
}

pub fn preferred_timer() {
    let time = crate::arch::drivers::time::preferred_timer_ms();
    loop {
        if time + 200 < crate::arch::drivers::time::preferred_timer_ms() {
            break;
        }
    }
}
