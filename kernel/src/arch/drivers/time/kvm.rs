/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::arch::x86_64::{__cpuid, _rdtsc};

use alloc::sync::Arc;

use crate::{info, memory::get_hhdm_offset};

const MSR_KVM_SYSTEM_TIME_NEW: u32 = 0x4b564d01;

lazy_static::lazy_static! {
    static ref KVM_TIMER: Arc<PvClockVcpuTimeInfo> = Arc::new(PvClockVcpuTimeInfo::default());
}

static mut OFFSET: u64 = 0;

#[repr(C)]
#[derive(Default, Debug, Copy, Clone)]
pub struct PvClockVcpuTimeInfo {
    pub version: u32,
    pub pad0: u32,
    pub tsc_timestamp: u64,
    pub system_time: u64,
    pub tsc_to_system_mul: u32,
    pub tsc_shift: i8,
    pub pad: [u8; 3],
}

pub fn init() {
    let ptr = Arc::as_ptr(&KVM_TIMER) as u64;
    let is_supported = supported();
    info!("kvm clock supported: {}", is_supported);
    if is_supported {
        crate::arch::system::cpu::write_msr(0x4B564D01, (ptr - get_hhdm_offset()) | 1);
        unsafe { OFFSET = time_ns() - (crate::task::timer::current_ticks() / 1_000_000) };
    }
}

pub fn time_ns() -> u64 {
    let mut time: u128 = unsafe { _rdtsc() as u128 } - KVM_TIMER.tsc_timestamp as u128;
    if KVM_TIMER.tsc_shift >= 0 {
        time <<= KVM_TIMER.tsc_shift;
    } else {
        time >>= -KVM_TIMER.tsc_shift;
    }
    time = (time * KVM_TIMER.tsc_to_system_mul as u128) >> 32;
    time = time + KVM_TIMER.system_time as u128;

    return time as u64 - unsafe { OFFSET };
}

pub fn supported() -> bool {
    let mut kvmclock = false;
    let base = crate::arch::system::cpu::kvm_base();
    if base != 0 {
        let id = unsafe { __cpuid(0x40000001) };
        kvmclock = (id.eax & (1 << 3)) != 0
    }
    return kvmclock;
}

pub fn tsc_freq() -> u64 {
    let mut freq = (1_000_000_000u64 << 32) / KVM_TIMER.tsc_to_system_mul as u64;
    if KVM_TIMER.tsc_shift < 0 {
        freq <<= -KVM_TIMER.tsc_shift;
    } else {
        freq >>= KVM_TIMER.tsc_shift;
    }
    return freq;
}
