/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::{string::String, vec::Vec};
use pc_keyboard::{DecodedKey, KeyCode};
use spin::Mutex;

use crate::{arch::drivers::time, print, println, task::keyboard::ScancodeStream};

lazy_static::lazy_static! {
    pub static ref SHELL: Mutex<Shell> = Mutex::new(Shell::new());

    static ref invis: Vec<u8> = (0u8..=255)
        .filter(|&b| {
            match b {
                0x00..=0x1F | 0x7F => true,
                0xA0 => true,
                _ => false,
            }
        })
        .collect();
}

pub struct Shell {
    input: Vec<u8>,
    last_commands: Vec<String>,
    prev_commands: Vec<String>,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            input: Vec::new(),
            last_commands: Vec::new(),
            prev_commands: Vec::new(),
        }
    }

    pub fn key_event(&mut self, dc: DecodedKey, scancodes: &ScancodeStream) {
        if alloc::vec![KeyCode::LControl, KeyCode::C]
            .iter()
            .all(|x: &KeyCode| scancodes.keys_down.contains(x))
        {
            print!("^C\n$ ");
            self.input.clear();
            return;
        }
        match dc {
            DecodedKey::Unicode(character) => {
                if character == '\u{8}' {
                    if self.input.len() > 0 {
                        print!("\x08 \x08");
                        self.input.pop();
                    }
                } else if character == '\n' || character == '\r' {
                    println!();
                    let raw_input = core::str::from_utf8(&self.input).unwrap().trim().replace(
                        "!!",
                        self.last_commands
                            .last()
                            .map(|x| x.as_str())
                            .unwrap_or_default(),
                    );
                    let mut split_input = raw_input.split(" ");
                    let cmd = split_input.next().unwrap();
                    let args = Vec::from_iter(split_input.into_iter());
                    run_command(cmd, args);
                    if !raw_input.is_empty() {
                        self.last_commands.push(raw_input);
                    }
                    print!("$ ");
                    self.input.clear();
                } else if !invis.contains(&(character as u8)) {
                    self.input.push(character as u8);
                    print!("{}", character);
                }
            }
            DecodedKey::RawKey(key) => match key {
                KeyCode::ArrowUp => {
                    if self.last_commands.len() > 0 {
                        print!(
                            "{}",
                            "\x08 \x08".repeat(
                                crate::utils::term::get_framebuffers()
                                    .next()
                                    .unwrap()
                                    .width() as usize
                                    / 8
                            )
                        );
                        if self.input.len() > 0 {
                            self.prev_commands
                                .push(String::from_utf8(self.input.clone()).unwrap());
                        }
                        self.input.clear();
                        let last_input = self.last_commands.pop().unwrap();
                        self.input = last_input.as_bytes().to_vec();
                        print!("$ {}", last_input);
                    }
                }
                KeyCode::ArrowDown => {
                    print!(
                        "{}",
                        "\x08 \x08".repeat(
                            crate::utils::term::get_framebuffers()
                                .next()
                                .unwrap()
                                .width() as usize
                                / 8
                        )
                    );
                    if self.input.len() > 0 {
                        self.last_commands
                            .push(String::from_utf8(self.input.clone()).unwrap());
                    }
                    self.input.clear();
                    self.input.clear();
                    if self.prev_commands.len() > 0 {
                        let last_input = self.prev_commands.pop().unwrap();
                        self.input = last_input.as_bytes().to_vec();
                        print!("$ {}", last_input);
                    } else {
                        print!("$ ");
                    }
                }
                _ => {}
            },
        }
    }
}

pub fn run_command(cmd: &str, args: Vec<&str>) {
    match cmd {
        "help" | "?" => {
            println!(
                "\nlist of commands:\n\n    \
            !! - replaced with the last command (like linux)\n    \
            help (?) - provides help\n    \
            time - current time given from the PIT\n    \
            date - current date given from the RTC\n    \
            clear - clears the screen\n    \
            echo - echoes the input\n    \
            nooo - prints nooo\n    \
            shutdown - shuts down the system\n    \
            pagefault - pagefaults on purpose\n    \
            panic - panics on command with the command arguments as the panic info\n    \
            "
            )
        }
        "time" => {
            println!("PIT - {}", crate::task::timer::pit_time_pretty(5));
            println!("TSC - {}", time::tsc::TSC_TIMER.lock().elapsed_pretty(5));
            let kvm = time::kvm::KVM_TIMER.lock();
            if kvm.is_supported() {
                println!("KVM - {}", kvm.elapsed_pretty(5));
            }
            println!("HPET- {}", time::hpet::HPET_TIMER.lock().elapsed_pretty(5));
        }
        "date" => {
            let config = crate::utils::config::get_config();

            let rtc_time = crate::arch::drivers::time::rtc::RtcTime::current()
                .with_timezone_offset(config.time.zone_offset as i16)
                .adjusted_for_timezone()
                .datetime_pretty();
            println!("{}", rtc_time)
        }
        "clear" => {
            print!("\x1b[2J\x1b[H");
        }
        "echo" => {
            println!("{}", args.join(" "));
        }
        "nooo" => {
            println!("\n{}\n", crate::NOOO);
        }
        "shutdown" => {
            println!("shutting down");
            // exits qemu for now
            crate::utils::exit_qemu(crate::utils::QemuExitCode::Success);
        }
        "pagefault" => {
            unsafe { *(0xdeadbeef as *mut u8) = 42 };
        }
        "panic" => {
            panic!("{}", args.join(" ").as_str());
        }
        "x" => {
            let tsc = unsafe { core::arch::x86_64::_rdtsc() };
            println!("tsc: {tsc}");
        }
        "" => {}
        x => {
            println!("command not found - '{x}'\nrun 'help' to see the list of available commands");
        }
    }
}
