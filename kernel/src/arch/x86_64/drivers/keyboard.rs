/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::cell::UnsafeCell;

use alloc::{collections::vec_deque::VecDeque, vec::Vec};
use pc_keyboard::{HandleControl, KeyCode, Keyboard, ScancodeSet1, layouts::Us104Key};

use crate::{arch::system::cpu::Registers, utils::asm::port::inb};

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

pub fn keyboard_interrupt_handler(_stack_frame: &mut Registers) {
    unsafe { KEYBOARD_STATE.get_mut().scancodes.push_back(inb(0x60)) };
    crate::arch::system::pic::send_eoi(1);
}
