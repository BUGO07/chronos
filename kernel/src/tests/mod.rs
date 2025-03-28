use crate::{print, println};

pub mod memory;

pub trait Testable {
    fn run(&self);
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("{}...\t", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}

fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} test(s)\n", tests.len());
    for test in tests {
        test.run();
    }
}

pub fn init() {
    println!("Basic tests...");
    test_runner(&[&add, &bool_check, &basic_loop]);
    println!("\nMemory tests");
    test_runner(&[
        &memory::simple_allocation,
        &memory::large_vec,
        &memory::many_boxes,
    ]);
    crate::utils::exit_qemu(crate::utils::QemuExitCode::Success);
}

fn add() {
    let x = 1 + 1;
    assert_eq!(x, 2);
}

fn bool_check() {
    let x = true;
    assert!(x)
}

fn basic_loop() {
    let mut i = 0;
    loop {
        i += 1;
        if i > 5 {
            break;
        }
    }
    assert_eq!(i, 6)
}
