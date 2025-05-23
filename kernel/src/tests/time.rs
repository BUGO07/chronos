/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::println;

pub fn all_timers() {
    for timer in crate::arch::drivers::time::get_timers().iter() {
        if timer.supported {
            let time = (timer.elapsed_ns)(timer);
            loop {
                if time + 200 < (timer.elapsed_ns)(timer) {
                    break;
                }
            }
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
