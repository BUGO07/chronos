/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::sync::Arc;

use crate::{
    info,
    utils::{
        asm::{_cpuid, _rdtsc, regs::wrmsr},
        limine::get_hhdm_offset,
        time::Timer,
    },
};

lazy_static::lazy_static! {
    static ref TABLE: Arc<PvClockVcpuTimeInfo> = Arc::new(PvClockVcpuTimeInfo::default());
}

#[repr(C, packed)]
#[derive(Default, Debug, Copy, Clone)]
pub struct PvClockVcpuTimeInfo {
    pub version: u32,
    pub pad0: u32,
    pub tsc_timestamp: u64,
    pub system_time: u64,
    pub tsc_to_system_mul: u32,
    pub tsc_shift: i8,
    pub flags: u8,
    pub pad: [u8; 2],
}

pub fn init() {
    let is_supported = supported();
    info!("kvm clock supported: {}", is_supported);
    if is_supported {
        let mut timer = Timer::new(
            "KVM",
            0,
            1,
            true,
            0,
            |timer: &Timer| {
                let table_ptr = Arc::as_ptr(&*TABLE);
                // Use raw pointer + read_unaligned for packed struct fields
                let version = unsafe { core::ptr::addr_of!((*table_ptr).version).read_unaligned() };
                let tsc_timestamp = unsafe { core::ptr::addr_of!((*table_ptr).tsc_timestamp).read_unaligned() };
                let system_time = unsafe { core::ptr::addr_of!((*table_ptr).system_time).read_unaligned() };
                let tsc_to_system_mul = unsafe { core::ptr::addr_of!((*table_ptr).tsc_to_system_mul).read_unaligned() };
                let tsc_shift = unsafe { core::ptr::addr_of!((*table_ptr).tsc_shift).read_unaligned() };
                let _ = version; // read for seqlock consistency

                let mut time: u128 = _rdtsc() as u128 - tsc_timestamp as u128;
                if tsc_shift >= 0 {
                    time <<= tsc_shift;
                } else {
                    time >>= -tsc_shift;
                }
                time = (time * tsc_to_system_mul as u128) >> 32;
                time += system_time as u128;

                time as u64 - timer.offset
            },
            0,
        );
        info!("setting up...");
        wrmsr(
            0x4b564d01,
            (Arc::as_ptr(&*TABLE) as u64 - get_hhdm_offset()) | 1,
        );
        timer
            .set_offset((timer.elapsed_ns)(&timer) - (super::pit::current_pit_ticks() * 1_000_000));
        info!("done");
        super::register_timer(timer);
    }
}

pub fn supported() -> bool {
    let mut is_supported = false;
    let base = crate::utils::asm::kvm_base();
    if base != 0 {
        let id = _cpuid(0x40000001);
        is_supported = (id.eax & (1 << 3)) != 0
    }
    is_supported
}

pub fn tsc_freq() -> u64 {
    let table_ptr = Arc::as_ptr(&*TABLE);
    let tsc_to_system_mul = unsafe { core::ptr::addr_of!((*table_ptr).tsc_to_system_mul).read_unaligned() };
    let tsc_shift = unsafe { core::ptr::addr_of!((*table_ptr).tsc_shift).read_unaligned() };
    let mut freq = (1_000_000_000u64 << 32) / tsc_to_system_mul as u64;
    if tsc_shift < 0 {
        freq <<= -tsc_shift;
    } else {
        freq >>= tsc_shift;
    }
    freq
}
