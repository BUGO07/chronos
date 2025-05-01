/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use ps2_mouse::{Mouse, MouseState};
use spin::Mutex;
use x86_64::{instructions::port::PortReadOnly, structures::idt::InterruptStackFrame};

use crate::{arch::interrupts::IDT, debug, error, info};

lazy_static::lazy_static! {
    pub static ref DRIVER: Mutex<Mouse> = Mutex::new(Mouse::new());
    pub static ref MOUSE: Mutex<MouseInfo> = Mutex::new(MouseInfo::new());
}

pub struct MouseInfo {
    x: u16,
    y: u16,
    left: bool,
    right: bool,
    state: MouseState,
}

impl Default for MouseInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl MouseInfo {
    pub fn new() -> MouseInfo {
        MouseInfo {
            state: MouseState::new(),
            x: 0,
            y: 0,
            left: false,
            right: false,
        }
    }
}

pub fn init() {
    x86_64::instructions::interrupts::without_interrupts(|| {
        info!("initializing ps/2 mouse...");
        unsafe { IDT[0x2c].set_handler_fn(mouse_interrupt_handler) };
        if let Err(err) = DRIVER.lock().init() {
            error!("failed to initialize ps/2 mouse: {}", err);
        }
        DRIVER.lock().set_on_complete(on_complete);
        debug!("done");
    });
}

fn on_complete(mouse_state: MouseState) {
    let mut mouse = MOUSE.lock();
    if mouse_state.x_moved() {
        let x_movement = mouse_state.get_x();
        if x_movement > 0 {
            let added = mouse.x as u32 + x_movement.unsigned_abs() as u32;
            if added <= u16::MAX as u32 {
                mouse.x += x_movement.unsigned_abs();
            }
        } else if x_movement < 0 {
            let subtracted = mouse.x as i16 - x_movement.abs();
            if subtracted >= 0 {
                mouse.x -= x_movement.unsigned_abs();
            }
        }
    }

    if mouse_state.y_moved() {
        let y_movement = mouse_state.get_y();
        if y_movement > 0 {
            let added = mouse.y as u32 + y_movement.unsigned_abs() as u32;
            if added <= u16::MAX as u32 {
                mouse.y += y_movement.unsigned_abs();
            }
        } else if y_movement < 0 {
            let subtracted = mouse.y as i16 - y_movement.abs();
            if subtracted >= 0 {
                mouse.y -= y_movement.unsigned_abs();
            }
        }
    }

    mouse.left = mouse_state.left_button_down();

    mouse.right = mouse_state.right_button_down();

    mouse.state = mouse_state;
}

pub extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    let mut port = PortReadOnly::new(0x60);
    let packet = unsafe { port.read() };
    DRIVER.lock().process_packet(packet);

    crate::arch::interrupts::pic::send_eoi(12);
}
