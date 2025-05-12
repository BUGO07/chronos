/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::{boxed::Box, vec::Vec};

pub fn simple_allocation() {
    let heap_value_1 = Box::new(41);
    let heap_value_2 = Box::new(13);
    assert_eq!(*heap_value_1, 41);
    assert_eq!(*heap_value_2, 13);
}

pub fn large_vec() {
    let n = 1000;
    let mut vec = Vec::new();
    for i in 0..n {
        vec.push(i);
    }
    assert_eq!(vec.iter().sum::<u64>(), (n - 1) * n / 2);
}

pub fn many_boxes() {
    let long_lived = Box::new(1);
    for i in 0..10000 {
        let x = Box::new(i);
        assert_eq!(*x, i);
    }
    assert_eq!(*long_lived, 1);
}

// stole it from someone in discord
pub fn malloc_test() {
    for i in 0..5000 {
        unsafe {
            let osize = 500000;
            let mut x =
                alloc::alloc::alloc(alloc::alloc::Layout::from_size_align(osize, 16).unwrap())
                    as *mut u8;
            for i in 0..osize {
                *x.cast::<usize>().add(i) = 2;
            }
        }
    }
}
