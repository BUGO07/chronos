/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{
    debug, info,
    memory::{
        get_hhdm_offset,
        vmm::{flag, page_size},
    },
};
use alloc::{format, string::String};
use core::{cell::OnceCell, ptr::null_mut};
use uacpi_sys::*;

static mut HPET_ADDRESS: u64 = 0;

pub static mut HPET_TIMER: OnceCell<HpetTimer> = OnceCell::new();

pub struct HpetTimer {
    start: u64,
    tickrate: u64,
    supported: bool,
}

impl HpetTimer {
    pub fn start(tickrate: u64) -> Self {
        HpetTimer {
            start: hpet_read(0xF0),
            tickrate,
            supported: true,
        }
    }

    pub fn unsupported() -> Self {
        HpetTimer {
            start: 0,
            tickrate: 0,
            supported: false,
        }
    }

    pub fn elapsed_cycles(&self) -> u64 {
        if self.supported {
            hpet_read(0xF0) - self.start
        } else {
            0
        }
    }

    pub fn elapsed_ns(&self) -> u64 {
        if self.supported {
            (self.elapsed_cycles() as u128 * 1_000_000_000 / self.tickrate as u128) as u64
        } else {
            0
        }
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
        self.supported
    }
}

fn hpet_read(offset: u64) -> u64 {
    unsafe { *((HPET_ADDRESS + get_hhdm_offset() + offset) as *const u64) }
}

fn hpet_write(offset: u64, value: u64) {
    unsafe { *((HPET_ADDRESS + get_hhdm_offset() + offset) as *mut u64) = value }
}

pub fn init() {
    info!("setting up...");

    if !supported() {
        unsafe {
            info!("hpet not supported");
            HPET_TIMER.set(HpetTimer::unsupported()).ok();
        }
        return;
    }

    let paddr = unsafe { HPET_ADDRESS };
    let address = paddr + get_hhdm_offset();

    info!("mapping hpet address: 0x{:X} -> 0x{:X}", paddr, address);
    crate::memory::vmm::PAGEMAP.lock().map(
        paddr + get_hhdm_offset(),
        paddr,
        flag::PRESENT | flag::WRITE,
        page_size::SMALL,
    );

    let capabilities = hpet_read(0x000) as u64;

    let mut config = hpet_read(0x010);
    config |= 1;
    hpet_write(0x010, config);

    unsafe {
        HPET_TIMER
            .set(HpetTimer::start(
                1_000_000_000_000_000 / (capabilities >> 32),
            ))
            .ok()
    };
    debug!("done");
}

fn supported() -> bool {
    unsafe {
        let mut table: uacpi_table = uacpi_table {
            index: 0,
            __bindgen_anon_1: uacpi_table__bindgen_ty_1 { ptr: null_mut() },
        };
        if uacpi_table_find_by_signature(c"HPET".as_ptr(), &raw mut table) != UACPI_STATUS_OK {
            info!("couldn't find hpet table");
            return false;
        }

        let hpet = *(table.__bindgen_anon_1.ptr as *const acpi_hpet);

        if hpet.address.address_space_id as u32 != UACPI_ADDRESS_SPACE_SYSTEM_MEMORY {
            uacpi_table_unref(&mut table as *mut uacpi_table);
            return false;
        }

        HPET_ADDRESS = hpet.address.address;

        uacpi_table_unref(&mut table as *mut uacpi_table);

        return true;
    }
}
