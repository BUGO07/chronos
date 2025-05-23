/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::cell::UnsafeCell;

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use pc_keyboard::{HandleControl, KeyCode, Keyboard, ScancodeSet1, layouts::Us104Key};

use crate::{arch::interrupts::StackFrame, utils::asm::port::inb};

pub static mut KEYBOARD_STATE: UnsafeCell<KeyboardState> = UnsafeCell::new(KeyboardState {
    keyboard: Keyboard::new(ScancodeSet1::new(), Us104Key, HandleControl::Ignore),
    scancodes: VecDeque::new(),
    keys_down: Vec::new(),
});

pub struct KeyboardState {
    pub keyboard: Keyboard<Us104Key, ScancodeSet1>,
    pub scancodes: VecDeque<u8>,
    pub keys_down: Vec<KeyCode>,
}

pub fn keyboard_interrupt_handler(_stack_frame: *mut StackFrame) {
    unsafe { KEYBOARD_STATE.get_mut().scancodes.push_back(inb(0x60)) };
    crate::arch::interrupts::pic::send_eoi(1);
}

pub fn keyboard_thread() -> ! {
    crate::arch::interrupts::pic::unmask(1);
    let keyboard_state = unsafe { KEYBOARD_STATE.get_mut() };
    loop {
        if !keyboard_state.scancodes.is_empty() {
            let scancode = keyboard_state.scancodes.pop_front().unwrap();
            let keys_down = &mut keyboard_state.keys_down;
            if let Ok(Some(key_event)) = keyboard_state.keyboard.add_byte(scancode) {
                if key_event.state == pc_keyboard::KeyState::Down {
                    if !keys_down.contains(&key_event.code) {
                        keys_down.push(key_event.code);
                    }
                } else {
                    keys_down.retain(|&x| x != key_event.code);
                }

                if let Some(dc) = keyboard_state.keyboard.process_keyevent(key_event) {
                    unsafe {
                        if let Some(shell) = crate::utils::shell::SHELL.get_mut() {
                            shell.event_queue.push_back((dc, KEYBOARD_STATE.get_mut()));
                        }
                    }
                }
            }
        }
    }
}
