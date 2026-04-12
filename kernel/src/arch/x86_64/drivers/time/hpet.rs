/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{
    debug, info,
    memory::vmm::{flag, page_size},
    utils::{limine::get_hhdm_offset, time::Timer},
};
use uacpi_sys::*;

use core::sync::atomic::{AtomicU64, Ordering};

static HPET_ADDRESS: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    let supported = supported();
    info!("hpet supported: {}", supported);
    if !supported {
        return;
    }

    info!("setting up...");

    let paddr = HPET_ADDRESS.load(Ordering::Relaxed);
    let address = paddr + get_hhdm_offset();

    debug!("mapping hpet: 0x{:X} -> 0x{:X}", paddr, address);
    unsafe {
        crate::memory::vmm::PAGEMAP
            .get()
            .unwrap()
            .lock()
            .map(paddr + get_hhdm_offset(), paddr, flag::RW, page_size::SMALL)
            .unwrap();
    };

    let capabilities = hpet_read(0x000);
    let period_fs = capabilities >> 32;
    if period_fs == 0 {
        info!("hpet period is zero, skipping");
        return;
    }

    let mut config = hpet_read(0x010);
    config |= 1;
    hpet_write(0x010, config);

    super::register_timer(Timer::new(
        "HPET",
        hpet_read(0xF0),
        1_000_000_000_000_000 / period_fs,
        true,
        20,
        |timer: &Timer| {
            ((((hpet_read(0xF0) - timer.start) as u128 * 1_000_000_000) / timer.frequency as u128)
                as u64)
                + timer.offset
        },
        super::pit::current_pit_ticks() * 1_000_000,
    ));
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
            uacpi_table_unref(&raw mut table);
            return false;
        }

        HPET_ADDRESS.store(hpet.address.address, Ordering::Relaxed);

        uacpi_table_unref(&raw mut table);

        true
    }
}

fn hpet_read(offset: u64) -> u64 {
    unsafe {
        core::ptr::read_volatile(
            (HPET_ADDRESS.load(Ordering::Relaxed) + get_hhdm_offset() + offset) as _,
        )
    }
}

fn hpet_write(offset: u64, value: u64) {
    unsafe {
        core::ptr::write_volatile(
            (HPET_ADDRESS.load(Ordering::Relaxed) + get_hhdm_offset() + offset) as _,
            value,
        )
    }
}
