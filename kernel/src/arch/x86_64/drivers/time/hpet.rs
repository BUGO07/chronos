/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{
    debug, info,
    memory::vmm::{flag, page_size},
    utils::{limine::get_hhdm_offset, time::KernelTimer},
};
use core::cell::OnceCell;
use uacpi_sys::*;

pub static mut HPET_TIMER: OnceCell<HpetTimer> = OnceCell::new();

static mut HPET_ADDRESS: u64 = 0;

pub struct HpetTimer {
    start: u64,
    tickrate: u64,
    offset_ns: u64,
    supported: bool,
}

impl HpetTimer {
    pub fn start(tickrate: u64) -> Self {
        HpetTimer {
            start: hpet_read(0xF0),
            tickrate,
            offset_ns: super::preferred_timer_ns(),
            supported: true,
        }
    }

    pub fn unsupported() -> Self {
        HpetTimer {
            start: 0,
            tickrate: 0,
            offset_ns: 0,
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
}

impl KernelTimer for HpetTimer {
    fn is_supported(&self) -> bool {
        self.supported
    }

    fn elapsed_ns(&self) -> u64 {
        if self.supported {
            (self.elapsed_cycles() as u128 * 1_000_000_000 / self.tickrate as u128) as u64
                + self.offset_ns // offset from main timer
        } else {
            0
        }
    }

    fn name(&self) -> &'static str {
        "HPET"
    }

    fn priority(&self) -> u8 {
        20
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

    debug!("mapping hpet: 0x{:X} -> 0x{:X}", paddr, address);
    unsafe {
        crate::memory::vmm::PAGEMAP.get_mut().unwrap().lock().map(
            paddr + get_hhdm_offset(),
            paddr,
            flag::RW,
            page_size::SMALL,
        )
    };

    let capabilities = hpet_read(0x000);

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
    info!("done");
}

fn supported() -> bool {
    unsafe {
        let mut table: uacpi_table = uacpi_table::default();
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

        true
    }
}
