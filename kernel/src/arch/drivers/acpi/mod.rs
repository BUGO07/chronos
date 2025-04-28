/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{error, println};
use limine::{request::RsdpRequest, response::RsdpResponse};
use uacpi_sys::*;

pub mod uacpi;

#[used]
#[unsafe(link_section = ".requests")]
static RSDP_REQUEST: RsdpRequest = RsdpRequest::new();

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
    }
}

pub fn get_rsdp() -> &'static RsdpResponse {
    RSDP_REQUEST
        .get_response()
        .expect("couldn't get rsdp from limine")
}

pub fn shutdown() {
    unsafe {
        uacpi_prepare_for_sleep_state(UACPI_SLEEP_STATE_S5);
        uacpi_enter_sleep_state(UACPI_SLEEP_STATE_S5);
    }
}

pub fn reboot() {
    unsafe {
        uacpi_reboot();
    }
}

pub fn sleep() {
    unsafe {
        uacpi_prepare_for_sleep_state(UACPI_SLEEP_STATE_S3);
        uacpi_enter_sleep_state(UACPI_SLEEP_STATE_S3);
    }
}

pub enum PowerAction {
    Shutdown,
    Reboot,
    Sleep,
    Hibernate, // todo
}

pub fn perform_power_action(action: PowerAction) {
    match action {
        PowerAction::Shutdown => {
            println!("Shutting down...");
            shutdown();
            error!("Couldn't shutdown...");
        }
        PowerAction::Reboot => {
            println!("Rebooting...");
            reboot();
            error!("Couldn't reboot...");
        }
        PowerAction::Sleep => {
            println!("Sleeping...");
            sleep();
            error!("Couldn't sleep...");
        }
        _ => {}
    }
}
