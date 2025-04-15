/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{print, println, warn};
use alloc::{
    string::{String, ToString},
    vec,
};
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
use pc_keyboard::{HandleControl, Keyboard, ScancodeSet1, layouts};

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
    // these shouldnt be here
    // keys_down: alloc::vec::Vec<KeyCode>,
    input: alloc::vec::Vec<u8>,
    last_commands: alloc::vec::Vec<String>,
}

impl ScancodeStream {
    pub fn new() -> Self {
        SCANCODE_QUEUE
            .try_init_once(|| ArrayQueue::new(100))
            .expect("ScancodeStream::new should only be called once");
        ScancodeStream {
            _private: (),
            // keys_down: vec![],
            input: vec![],
            last_commands: vec![],
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
            // if key_event.state == pc_keyboard::KeyState::Down {
            //     if !scancodes.keys_down.contains(&key_event.code) {
            //         scancodes.keys_down.push(key_event.code);
            //     }
            // } else {
            //     scancodes.keys_down.retain(|&x| x != key_event.code);
            // }

            // dont berate me this is temporary
            if let Some(dc) = keyboard.process_keyevent(key_event) {
                match dc {
                    pc_keyboard::DecodedKey::Unicode(character) => {
                        if character == '\u{8}' {
                            if scancodes.input.len() > 0 {
                                print!("\x08 \x08");
                                scancodes.input.pop();
                            }
                        } else if character == '\n' || character == '\r' {
                            println!();
                            let raw_input = core::str::from_utf8(&scancodes.input).unwrap().trim();
                            let mut split_input = raw_input.split(" ");
                            let cmd = split_input.next().unwrap();
                            let args = alloc::vec::Vec::from_iter(split_input.into_iter());
                            match cmd {
                                "help" | "?" => {
                                    println!(
                                        "\nlist of commands:\n\n    \
                                    help (?) - provides help\n    \
                                    time - current time given from the PIT\n    \
                                    last_commands - prints last used commands\n    \
                                    nooo - prints nooo\n    \
                                    panic - panics on command with the command arguments as the panic info\n    \
                                    "
                                    )
                                }
                                "time" => {
                                    let time = crate::arch::drivers::pit::time_ms();
                                    let hours = time / 3_600_000;
                                    let minutes = (time % 3_600_000) / 60_000;
                                    let seconds = (time % 60_000) / 1000;
                                    let millis = time % 1000;
                                    println!(
                                        "current time - {:02}:{:02}:{:02}.{:03}",
                                        hours, minutes, seconds, millis,
                                    )
                                }
                                "last_commands" => {
                                    println!("{:?}", scancodes.last_commands)
                                }
                                "nooo" => {
                                    println!("\n{}\n", crate::NOOO);
                                }
                                "panic" => {
                                    crate::call_panic(args.join(" ").as_str());
                                }
                                "" => {}
                                x => {
                                    println!(
                                        "command not found - '{x}'\nrun 'help' to see the list of available commands"
                                    );
                                }
                            }
                            if raw_input != "" {
                                scancodes.last_commands.push(raw_input.to_string());
                            }
                            print!("> ");
                            scancodes.input.clear();
                        } else {
                            scancodes.input.push(character as u8);
                            print!("{}", character);
                        }
                    }
                    _ => {}
                }
            }
        }

        // if vec![KeyCode::LShift, KeyCode::Spacebar]
        //     .iter()
        //     .all(|x: &KeyCode| scancodes.keys_down.contains(x))
        // {
        // unsafe { crate::arch::drivers::pit::TIME_MS += 40271000 }; // 11:11:11
        // TODO:
        // crate::utils::time::get_rtc();
        // }

        // if scancodes.keys_down.contains(&KeyCode::F1) {
        //     info!("keybord input works");
        // }
    }
}
