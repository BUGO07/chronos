/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

pub fn basic_timer() {
    let time = crate::arch::drivers::pit::time_ms();
    loop {
        if time + 1000 < crate::arch::drivers::pit::time_ms() {
            break;
        }
    }
}
