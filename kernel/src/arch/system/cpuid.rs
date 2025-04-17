/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::arch::x86_64::__cpuid;

use alloc::string::{String, ToString};

pub fn get_cpu() -> String {
    unsafe {
        let part1 = __cpuid(0x80000002);
        let part2 = __cpuid(0x80000003);
        let part3 = __cpuid(0x80000004);

        let brand_raw = [
            part1.eax, part1.ebx, part1.ecx, part1.edx, part2.eax, part2.ebx, part2.ecx, part2.edx,
            part3.eax, part3.ebx, part3.ecx, part3.edx,
        ];

        let brand = brand_raw
            .iter()
            .flat_map(|reg| reg.to_le_bytes())
            .map(|b| b as char)
            .collect::<String>()
            .trim()
            .to_string();

        brand
    }
}

pub fn get_freq() -> u64 {
    let freq = unsafe { __cpuid(0x15) };

    let a = freq.eax as u64;
    let b = freq.ebx as u64;
    let c = freq.ecx as u64;

    if a != 0 && b != 0 && c != 0 {
        return c * b / a;
    }
    0
}
