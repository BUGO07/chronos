/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::vec::Vec;
use conquer_once::spin::OnceCell;
use core::{
    pin::Pin,
    task::{Context, Poll},
};
use crossbeam_queue::ArrayQueue;
use futures_util::{
    stream::{Stream, StreamExt},
    task::AtomicWaker,
};
use pc_keyboard::{HandleControl, KeyCode, Keyboard, ScancodeSet1, layouts};
use x86_64::{instructions::port::Port, structures::idt::InterruptStackFrame};

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

#[non_exhaustive]
pub struct ScancodeStream {
    // this probably shouldnt be here
    pub keys_down: Vec<KeyCode>,
}

impl Default for ScancodeStream {
    fn default() -> Self {
        Self::new()
    }
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream {
            keys_down: Vec::new(),
        }
    }
}

impl Stream for ScancodeStream {
    type Item = u8;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<u8>> {
        let queue = SCANCODE_QUEUE
            .try_get()
            .expect("scancode queue not initialized");

        if let Some(scancode) = queue.pop() {
            return Poll::Ready(Some(scancode));
        }

        WAKER.register(cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
        }
    }
}

pub extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    add_scancode(unsafe { Port::new(0x60).read() });
    crate::arch::interrupts::pic::send_eoi(1);
}

pub fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if queue.push(scancode).is_err() {
            crate::warn!("scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    }
}

pub async fn handle_keypresses() {
    let mut scancodes = ScancodeStream::new();
    let mut keyboard = Keyboard::new(
        ScancodeSet1::new(),
        layouts::Us104Key,
        HandleControl::Ignore,
    );

    while let Some(scancode) = scancodes.next().await {
        if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
            if key_event.state == pc_keyboard::KeyState::Down {
                if !scancodes.keys_down.contains(&key_event.code) {
                    scancodes.keys_down.push(key_event.code);
                }
            } else {
                scancodes.keys_down.retain(|&x| x != key_event.code);
            }

            if let Some(dc) = keyboard.process_keyevent(key_event) {
                // dont berate me this is temporary
                unsafe {
                    crate::arch::shell::SHELL
                        .get_mut()
                        .unwrap()
                        .key_event(dc, &scancodes)
                };
            }
        }
    }
}
