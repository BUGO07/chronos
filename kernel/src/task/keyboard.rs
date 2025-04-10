use crate::{println, warn};
use alloc::vec;
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

static SCANCODE_QUEUE: OnceCell<ArrayQueue<u8>> = OnceCell::uninit();
static WAKER: AtomicWaker = AtomicWaker::new();

pub fn add_scancode(scancode: u8) {
    if let Ok(queue) = SCANCODE_QUEUE.try_get() {
        if let Err(_) = queue.push(scancode) {
            warn!("scancode queue full; dropping keyboard input");
        } else {
            WAKER.wake();
        }
    } else {
        warn!("scancode queue uninitialized");
    }
}

pub struct ScancodeStream {
    _private: (),
    keys_down: alloc::vec::Vec<KeyCode>,
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream {
            _private: (),
            keys_down: alloc::vec::Vec::new(),
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

        WAKER.register(&cx.waker());
        match queue.pop() {
            Some(scancode) => {
                WAKER.take();
                Poll::Ready(Some(scancode))
            }
            None => Poll::Pending,
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
        }

        if vec![KeyCode::LShift, KeyCode::Spacebar]
            .iter()
            .all(|x: &KeyCode| scancodes.keys_down.contains(x))
        {
            // TODO:
            // crate::utils::time::get_rtc();
        }

        if scancodes.keys_down.contains(&KeyCode::F1) {
            let ms = crate::arch::drivers::pit::time_ms();
            println!("time - {}.{:03}", ms / 1000, ms % 1000);
        }
    }
}
