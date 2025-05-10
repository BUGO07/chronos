/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::{collections::linked_list::LinkedList, vec::Vec};
use pc_keyboard::{HandleControl, KeyCode, Keyboard, ScancodeSet1, layouts::Us104Key};

use crate::{arch::interrupts::StackFrame, utils::asm::port::inb};

pub static mut KEYBOARD_STATE: KeyboardState = KeyboardState {
    scancodes: LinkedList::new(),
    keys_down: Vec::new(),
};

pub struct KeyboardState {
    pub scancodes: LinkedList<u8>,
    pub keys_down: Vec<KeyCode>,
}

pub fn keyboard_interrupt_handler(_stack_frame: *mut StackFrame) {
    unsafe { KEYBOARD_STATE.scancodes.push_back(inb(0x60)) };
    crate::arch::interrupts::pic::send_eoi(1);
}

pub fn keyboard_thread() -> ! {
    let mut keyboard = Keyboard::new(ScancodeSet1::new(), Us104Key, HandleControl::Ignore);
    let keyboard_state = unsafe { &mut KEYBOARD_STATE };

    loop {
        crate::utils::asm::halt();
        if keyboard_state.scancodes.is_empty() {
            continue;
        }

        let scancode = keyboard_state.scancodes.pop_front().unwrap();
        let keys_down = &mut keyboard_state.keys_down;
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if key_event.state == pc_keyboard::KeyState::Down {
                if !keys_down.contains(&key_event.code) {
                    keys_down.push(key_event.code);
                }
            } else {
                keys_down.retain(|&x| x != key_event.code);
            }

            if let Some(dc) = keyboard.process_keyevent(key_event) {
                unsafe {
                    crate::arch::shell::SHELL
                        .get_mut()
                        .unwrap()
                        .key_event(dc, keyboard_state);
                }
            }
        }
    }
}
