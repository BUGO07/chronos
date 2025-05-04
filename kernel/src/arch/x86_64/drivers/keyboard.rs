/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::{collections::linked_list::LinkedList, vec::Vec};
use pc_keyboard::{HandleControl, KeyCode, Keyboard, ScancodeSet1, layouts::Us104Key};
use x86_64::instructions::port::Port;

static mut KEYBOARD_STATE: KeyboardState = KeyboardState {
    scancodes: LinkedList::new(),
    keys_down: Vec::new(),
};
pub struct KeyboardState {
    pub scancodes: LinkedList<u8>,
    pub keys_down: Vec<KeyCode>,
}

pub fn keyboard_interrupt_handler(_stack_frame: *mut crate::arch::x86_64::interrupts::StackFrame) {
    unsafe {
        KEYBOARD_STATE
            .scancodes
            .push_back(Port::<u8>::new(0x60).read())
    };
    crate::arch::interrupts::pic::send_eoi(1);
}

pub fn keyboard_thread() -> ! {
    let mut keyboard = Keyboard::new(ScancodeSet1::new(), Us104Key, HandleControl::Ignore);
    let keyboard_state = unsafe { &mut KEYBOARD_STATE };

    loop {
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
                        .key_event(dc, keyboard_state)
                };
            }
        }
    }
}
