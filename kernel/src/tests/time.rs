/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

pub fn basic_timer() {
    let time = crate::task::timer::current_ticks();
    loop {
        if time + 1000 < crate::task::timer::current_ticks() {
            break;
        }
    }
}
