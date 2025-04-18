/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::arch::x86_64::{__cpuid, _rdtsc};

use alloc::{format, string::String, sync::Arc};
use spin::Mutex;

use crate::{info, memory::get_hhdm_offset};

lazy_static::lazy_static! {
    pub static ref KVM_TIMER: Mutex<KvmTimer> = Mutex::new(KvmTimer::start());
}

pub struct KvmTimer {
    supported: bool,
    table_pointer: Arc<PvClockVcpuTimeInfo>,
}

impl KvmTimer {
    pub fn start() -> Self {
        KvmTimer {
            supported: false,
            table_pointer: Arc::new(PvClockVcpuTimeInfo::default()),
        }
    }

    pub fn elapsed_ns(&self) -> u64 {
        let table = &self.table_pointer;
        let mut time: u128 = unsafe { _rdtsc() as u128 } - table.tsc_timestamp as u128;
        if table.tsc_shift >= 0 {
            time <<= table.tsc_shift;
        } else {
            time >>= -table.tsc_shift;
        }
        time = (time * table.tsc_to_system_mul as u128) >> 32;
        time = time + table.system_time as u128;

        return time as u64 - unsafe { OFFSET };
    }

    pub fn elapsed_pretty(&self, digits: u32) -> String {
        let elapsed_ns = self.elapsed_ns();
        let subsecond_ns = elapsed_ns % 1_000_000_000;

        let divisor = 10u64.pow(9 - digits);
        let subsecond = subsecond_ns / divisor;

        let elapsed_ms = elapsed_ns / 1_000_000;
        let seconds_total = elapsed_ms / 1000;
        let seconds = seconds_total % 60;
        let minutes_total = seconds_total / 60;
        let minutes = minutes_total % 60;
        let hours = minutes_total / 60;

        format!(
            "{:02}:{:02}:{:02}.{:0width$}",
            hours,
            minutes,
            seconds,
            subsecond,
            width = digits as usize
        )
    }

    pub fn is_supported(&self) -> bool {
        return self.supported;
    }
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
    let ptr = Arc::as_ptr(&KVM_TIMER.lock().table_pointer) as u64;
    let is_supported = supported();
    info!("kvm clock supported: {}", is_supported);
    if is_supported {
        KVM_TIMER.lock().supported = true;
        crate::arch::system::cpu::write_msr(0x4b564d01, (ptr - get_hhdm_offset()) | 1);
        unsafe {
            OFFSET =
                KVM_TIMER.lock().elapsed_ns() - (crate::task::timer::current_ticks() / 1_000_000)
        };
    }
}

pub fn supported() -> bool {
    let mut is_supported = false;
    let base = crate::arch::system::cpu::kvm_base();
    if base != 0 {
        let id = unsafe { __cpuid(0x40000001) };
        is_supported = (id.eax & (1 << 3)) != 0
    }
    return is_supported;
}

pub fn tsc_freq() -> u64 {
    let table = &KVM_TIMER.lock().table_pointer;
    let mut freq = (1_000_000_000u64 << 32) / table.tsc_to_system_mul as u64;
    if table.tsc_shift < 0 {
        freq <<= -table.tsc_shift;
    } else {
        freq >>= table.tsc_shift;
    }
    return freq;
}
