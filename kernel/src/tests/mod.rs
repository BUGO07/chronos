/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{print, println, utils::asm::halt_loop};

mod memory;
mod time;

trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        let string = &alloc::format!(
            "        {}...                        ",
            core::any::type_name::<T>().split("::").last().unwrap()
        )[0..35];
        print!("{}", string);
        self();
        println!("[ok]");
    }
}

fn test_runner(tests: &[&dyn Testable]) {
    println!("    Running {} test(s)", tests.len());
    for test in tests {
        test.run();
    }
}

pub fn init() {
    println!("Basic tests...");
    test_runner(&[&add, &bool_check, &basic_loop]);
    println!("\nMemory tests...");
    test_runner(&[
        &memory::simple_allocation,
        &memory::large_vec,
        &memory::many_boxes,
        &memory::malloc_test,
    ]);
    println!("\nTimer tests...");
    test_runner(&[&time::preferred_timer, &time::all_timers]);
    crate::drivers::acpi::shutdown();
    halt_loop()
}

fn add() {
    let x = 1 + 1;
    assert_eq!(x, 2);
}

fn bool_check() {
    let x = true;
    assert!(x);
}

fn basic_loop() {
    let mut i = 0;
    loop {
        i += 1;
        if i > 5 {
            break;
        }
    }
    assert_eq!(i, 6);
}
