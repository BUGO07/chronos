/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{cell::OnceCell, sync::atomic::Ordering};

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use pc_keyboard::{DecodedKey, KeyCode};

use crate::{
    arch::drivers::time::{self, KernelTimer},
    print, print_fill, println,
    utils::logger::color,
};

use super::drivers::{keyboard::ScancodeStream, time::TimerFuture};

pub static mut SHELL: OnceCell<Shell> = OnceCell::new();

lazy_static::lazy_static! {
    pub static ref INVISIBLE_CHARS: Vec<u8> = (0u8..=255)
        .filter(|&b| {
            matches!(b, 0x00..=0x1F | 0x7F | 0xA0)
        })
        .collect();
}

pub async fn shell_task() {
    unsafe { SHELL.set(Shell::new()).ok() };
    print!("$ ");

    let mut visible = true;

    loop {
        if visible {
            print!("\x1b[?25l");
        } else {
            print!("\x1b[?25h");
        }
        visible = !visible;

        TimerFuture::new(700).await
    }
}

pub struct Shell {
    input: Vec<u8>,
    last_commands: Vec<String>,
    prev_commands: Vec<String>,
    color_fg: String,
    color_bg: String,
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

impl Shell {
    pub fn new() -> Self {
        Self {
            input: Vec::new(),
            last_commands: Vec::new(),
            prev_commands: Vec::new(),
            color_fg: color::RESET.to_string(),
            color_bg: color::RESET.to_string(),
        }
    }

    pub fn key_event(&mut self, dc: DecodedKey, scancodes: &ScancodeStream) {
        print!("\x1b[?25h");
        match scancodes.keys_down.as_slice() {
            [KeyCode::LControl, KeyCode::C] => {
                print!("^C\n$ ");
                self.input.clear();
                return;
            }
            _ => {}
        }
        match dc {
            DecodedKey::Unicode(character) => {
                // backspace
                if character == '\u{8}' {
                    if !self.input.is_empty() {
                        print!("\x08 \x08");
                        self.input.pop();
                    }
                }
                // enter
                else if character == '\n' || character == '\r' {
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
                    let args = Vec::from_iter(split_input);
                    run_command(cmd, args, self);
                    self.prev_commands.reverse();
                    self.last_commands.append(&mut self.prev_commands);
                    if !raw_input.is_empty() {
                        self.last_commands.push(raw_input);
                    }
                    print!("$ ");
                    self.input.clear();
                }
                // del, esc, tab, etc
                else if !INVISIBLE_CHARS.contains(&(character as u8)) {
                    self.input.push(character as u8);
                    print!("{}", character);
                }
            }
            DecodedKey::RawKey(key) => match key {
                KeyCode::ArrowUp => {
                    if !self.last_commands.is_empty() {
                        print_fill!("\x08 \x08", "", false);
                        if !self.input.is_empty() {
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
                    print_fill!("\x08 \x08", "", false);
                    if !self.input.is_empty() {
                        self.last_commands
                            .push(String::from_utf8(self.input.clone()).unwrap());
                    }
                    self.input.clear();
                    self.input.clear();
                    if !self.prev_commands.is_empty() {
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

pub fn run_command(cmd: &str, args: Vec<&str>, shell: &mut Shell) {
    match cmd {
        "help" | "?" => {
            println!(
                "\nlist of commands:\n\n\t\
            !! - replaced with the last command (like linux)\n\t\
            help (?) - provides help\n\t\
            time [digits] - current time given from all the timers\n\t\
            date [timezone] - current date given from the RTC\n\t\
            clear - clears the screen\n\t\
            color [x] changes the color of the terminal (like windows or use hex)\n\t\
            bg [x] changes the background of the terminal (like windows or use hex)\n\t\
            echo [what] - echoes the input\n\t\
            nooo - prints nooo\n\t\
            shutdown - shuts down the system\n\t\
            reboot - reboots the system\n\t\
            suspend - suspends the system\n\t\
            hibernate - hibernates the system\n\t\
            pagefault - pagefaults on purpose\n\t\
            panic [message] - panics on command with the command arguments as the panic info\n\t\
            "
            )
        }
        "time" => {
            let digits = if !args.is_empty() {
                args[0].parse().unwrap_or(5).clamp(0, 9)
            } else {
                5
            };
            println!("PIT - {}", time::pit::elapsed_pretty(digits));
            if time::TIMERS_INIT_STATE.load(Ordering::Relaxed) >= 4 {
                unsafe {
                    let kvm = time::kvm::KVM_TIMER
                        .get()
                        .expect("couldn't access kvm timer");
                    let tsc = time::tsc::TSC_TIMER
                        .get()
                        .expect("couldn't access tsc timer");
                    let hpet = time::hpet::HPET_TIMER
                        .get()
                        .expect("couldn't access hpet timer");
                    if kvm.is_supported() {
                        println!("KVM - {}", kvm.elapsed_pretty(digits));
                    }
                    if tsc.is_supported() {
                        println!("TSC - {}", tsc.elapsed_pretty(digits));
                    }
                    if hpet.is_supported() {
                        println!("HPET- {}", hpet.elapsed_pretty(digits));
                    }
                }
            }
        }
        "date" => {
            let zone = if args.is_empty() {
                // config.time.zone_offset
                crate::utils::config::ZONE_OFFSET
            } else {
                args[0].parse().unwrap_or(crate::utils::config::ZONE_OFFSET)
            };

            let rtc_time = crate::arch::drivers::time::rtc::RtcTime::current()
                .with_timezone_offset(zone)
                .adjusted_for_timezone();
            println!(
                "{} | {}",
                rtc_time.datetime_pretty(),
                rtc_time.timezone_pretty()
            )
        }
        "clear" => {
            print!("\x1b[2J\x1b[H");
        }
        "color" => {
            let fg = if !args.is_empty() {
                match args[0] {
                    "0" => color::BLACK.to_string(),
                    "1" => color::BLUE.to_string(),
                    "2" => color::GREEN.to_string(),
                    "3" => color::CYAN.to_string(),
                    "4" => color::RED.to_string(),
                    "5" => color::PURPLE.to_string(),
                    "6" => color::YELLOW.to_string(),
                    "7" => color::WHITE.to_string(),
                    "8" => color::GRAY.to_string(),
                    "9" => color::BLUE.to_string(),
                    //TODO: make lighter variants of the following
                    "a" => color::LIGHT_GREEN.to_string(),
                    "b" => color::CYAN.to_string(),
                    "c" => color::LIGHT_RED.to_string(),
                    "d" => color::PINK.to_string(),
                    "e" => color::YELLOW.to_string(),
                    "f" => color::WHITE_BRIGHT.to_string(),
                    x => {
                        let is_valid = |hex: &str| -> bool {
                            if !hex.starts_with("#") || hex.len() != 7 {
                                return false;
                            }
                            let is_valid_hex = |h: u8| -> bool {
                                h.is_ascii_digit()
                                    || (b'a'..=b'f').contains(&h)
                                    || (b'A'..=b'F').contains(&h)
                            };
                            let mut valid = true;
                            for i in 1..7 {
                                if !is_valid_hex(hex.as_bytes()[i]) {
                                    valid = false;
                                    break;
                                }
                            }
                            valid
                        };
                        if is_valid(x) {
                            let bytes = x.as_bytes();
                            color::rgb(bytes[1..3][0], bytes[3..5][0], bytes[5..7][0], false)
                        } else {
                            color::RESET.to_string()
                        }
                    }
                }
            } else {
                color::RESET.to_string()
            };

            shell.color_fg = fg;

            let mut bg = shell.color_bg.as_str();

            if bg == color::RESET {
                bg = "";
            }

            println!("{}{}", shell.color_fg, bg);
        }
        "bg" => {
            // repeated code ik
            let bg = if !args.is_empty() {
                match args[0] {
                    "0" => color::BLACK_BG.to_string(),
                    "1" => color::BLUE_BG.to_string(),
                    "2" => color::GREEN_BG.to_string(),
                    "3" => color::CYAN_BG.to_string(),
                    "4" => color::RED_BG.to_string(),
                    "5" => color::PURPLE_BG.to_string(),
                    "6" => color::YELLOW_BG.to_string(),
                    "7" => color::WHITE_BG.to_string(),
                    "8" => color::GRAY_BG.to_string(),
                    "9" => color::BLUE_BG.to_string(),
                    //TODO: make lighter variants of the following
                    "a" => color::LIGHT_GREEN_BG.to_string(),
                    "b" => color::CYAN_BG.to_string(),
                    "c" => color::LIGHT_RED_BG.to_string(),
                    "d" => color::PINK_BG.to_string(),
                    "e" => color::YELLOW_BG.to_string(),
                    "f" => color::WHITE_BRIGHT_BG.to_string(),
                    x => {
                        let is_valid = |hex: &str| -> bool {
                            if !hex.starts_with("#") || hex.len() != 7 {
                                return false;
                            }
                            let is_valid_hex = |h: u8| -> bool {
                                h.is_ascii_digit()
                                    || (b'a'..=b'f').contains(&h)
                                    || (b'A'..=b'F').contains(&h)
                            };
                            let mut valid = true;
                            for i in 1..7 {
                                if !is_valid_hex(hex.as_bytes()[i]) {
                                    valid = false;
                                    break;
                                }
                            }
                            valid
                        };
                        if is_valid(x) {
                            let bytes = x.as_bytes();
                            color::rgb(bytes[1..3][0], bytes[3..5][0], bytes[5..7][0], true)
                        } else {
                            color::RESET.to_string()
                        }
                    }
                }
            } else {
                color::RESET.to_string()
            };

            shell.color_bg = bg;

            let mut fg = shell.color_fg.as_str();

            if fg == color::RESET {
                fg = "";
            }

            println!("{}{}", shell.color_bg, fg);
        }
        "echo" => {
            println!("{}", args.join(" "));
        }
        "nooo" => {
            println!("\n{}\n", crate::NOOO);
        }
        "shutdown" => {
            crate::arch::drivers::acpi::perform_power_action(
                super::drivers::acpi::PowerAction::Shutdown,
            );
        }
        "reboot" => {
            crate::arch::drivers::acpi::perform_power_action(
                super::drivers::acpi::PowerAction::Reboot,
            );
        }
        "sleep" => {
            crate::arch::drivers::acpi::perform_power_action(
                super::drivers::acpi::PowerAction::Sleep,
            );
        }
        "hibernate" => {
            crate::arch::drivers::acpi::perform_power_action(
                super::drivers::acpi::PowerAction::Hibernate,
            );
        }
        "pagefault" => {
            unsafe { *(0xdeadbeef as *mut u8) = 42 };
        }
        "panic" => {
            panic!("{}", args.join(" ").as_str());
        }
        "" => {}
        x => {
            println!("command not found - '{x}'\nrun 'help' to see the list of available commands");
        }
    }
}
