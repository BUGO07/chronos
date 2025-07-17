/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::ffi::CStr;

#[cfg(target_arch = "x86_64")]
use crate::memory::vmm::{PAGEMAP, flag, page_size};
use crate::{debug, device::pci::MCFG_ADDRESS, println, utils::limine::get_hhdm_offset};

use uacpi_sys::*;

pub mod uacpi;

pub fn init() {
    unsafe {
        let mut ret = uacpi_initialize(0);
        if ret != UACPI_STATUS_OK {
            panic!("uacpi didn't initialize properly - {}", ret);
        }
        ret = uacpi_namespace_load();
        if ret != UACPI_STATUS_OK {
            panic!(
                "uacpi didn't initialize properly | namespace load - {}",
                ret
            );
        }
        ret = uacpi_namespace_initialize();
        if ret != UACPI_STATUS_OK {
            panic!(
                "uacpi didn't initialize properly | namespace init - {}",
                ret
            );
        }
        ret = uacpi_finalize_gpe_initialization();
        if ret != UACPI_STATUS_OK {
            panic!("uacpi didn't initialize properly | gpe init - {}", ret);
        }

        let mut table = uacpi_table::default();
        if uacpi_table_find_by_signature(c"MCFG".as_ptr(), &mut table) == UACPI_STATUS_OK {
            let mcfg = &mut *(*(table.__bindgen_anon_1.ptr as *mut acpi_mcfg)) // this is how not to use rust
                .entries
                .as_mut_ptr();

            let addr = mcfg.address & !0xFFF;
            let virt = addr + get_hhdm_offset();

            debug!("mapping mcfg: 0x{addr:X} -> 0x{virt:X}");

            #[cfg(target_arch = "x86_64")]
            for i in (0..(256 * 1024 * 1024)).step_by(page_size::MEDIUM as usize) {
                PAGEMAP
                    .get_mut()
                    .unwrap()
                    .lock()
                    .map(virt + i, addr + i, flag::RW | flag::USER, page_size::MEDIUM)
                    .ok();
            }

            MCFG_ADDRESS = virt;
        }

        uacpi_install_fixed_event_handler(
            UACPI_FIXED_EVENT_POWER_BUTTON,
            Some(uacpi_powerbtn_handler),
            core::ptr::null_mut(),
        );

        uacpi_install_fixed_event_handler(
            UACPI_FIXED_EVENT_SLEEP_BUTTON,
            Some(uacpi_sleepbtn_handler),
            core::ptr::null_mut(),
        );
    }
}

extern "C" fn uacpi_powerbtn_handler(_: uacpi_handle) -> uacpi_interrupt_ret {
    perform_power_action(PowerAction::Shutdown);
    UACPI_INTERRUPT_HANDLED
}

extern "C" fn uacpi_sleepbtn_handler(_: uacpi_handle) -> uacpi_interrupt_ret {
    perform_power_action(PowerAction::Sleep);
    UACPI_INTERRUPT_HANDLED
}

pub fn shutdown() -> uacpi_status {
    unsafe {
        let ret = uacpi_prepare_for_sleep_state(UACPI_SLEEP_STATE_S5);
        if ret != UACPI_STATUS_OK {
            return ret;
        }
        uacpi_enter_sleep_state(UACPI_SLEEP_STATE_S5)
    }
}

pub fn reboot() -> uacpi_status {
    unsafe { uacpi_reboot() }
}

pub fn sleep() -> uacpi_status {
    unsafe {
        let ret = uacpi_prepare_for_sleep_state(UACPI_SLEEP_STATE_S3);
        if ret != UACPI_STATUS_OK {
            return ret;
        }
        uacpi_enter_sleep_state(UACPI_SLEEP_STATE_S3)
    }
}

pub fn hibernate() -> uacpi_status {
    unsafe {
        let ret = uacpi_prepare_for_sleep_state(UACPI_SLEEP_STATE_S4);
        if ret != UACPI_STATUS_OK {
            return ret;
        }
        uacpi_enter_sleep_state(UACPI_SLEEP_STATE_S4)
    }
}

pub enum PowerAction {
    Shutdown,
    Reboot,
    Sleep,
    Hibernate,
}

pub fn perform_power_action(action: PowerAction) {
    match action {
        PowerAction::Shutdown => {
            println!("shutting down...");
            let ret = shutdown();
            if ret != UACPI_STATUS_OK {
                println!("[ error ] couldn't shutdown - {}", unsafe {
                    CStr::from_ptr(uacpi_status_to_string(ret)).to_string_lossy()
                });
            }
        }
        PowerAction::Reboot => {
            println!("rebooting...");
            let ret = reboot();
            if ret != UACPI_STATUS_OK {
                println!("[ error ] couldn't reboot - {}", unsafe {
                    CStr::from_ptr(uacpi_status_to_string(ret)).to_string_lossy()
                });
            }
        }
        PowerAction::Sleep => {
            println!("sleeping...");
            let ret = sleep();
            if ret != UACPI_STATUS_OK {
                println!("[ error ] couldn't sleep - {}", unsafe {
                    CStr::from_ptr(uacpi_status_to_string(ret)).to_string_lossy()
                });
            }
        }
        PowerAction::Hibernate => {
            println!("hibernating...");
            let ret = hibernate();
            if ret != UACPI_STATUS_OK {
                println!("[ error ] couldn't hibernate - {}", unsafe {
                    CStr::from_ptr(uacpi_status_to_string(ret)).to_string_lossy()
                });
            }
        }
    }
}
