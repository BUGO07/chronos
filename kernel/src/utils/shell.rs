/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::cell::OnceCell;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use crate::{arch::drivers::time, drivers::acpi, print, println, utils::logger::color};

#[cfg(target_arch = "x86_64")]
use alloc::collections::vec_deque::VecDeque;
#[cfg(target_arch = "x86_64")]
use pc_keyboard::{DecodedKey, KeyCode};

#[cfg(target_arch = "x86_64")]
use crate::{arch::drivers::keyboard::KeyboardState, drivers::fs};

pub static mut SHELL: OnceCell<Shell> = OnceCell::new();

lazy_static::lazy_static! {
    pub static ref INVISIBLE_CHARS: Vec<u8> = (0u8..=255)
        .filter(|&b| {
            matches!(b, 0x00..=0x1F | 0x7F | 0xA0)
        })
        .collect();
}

#[cfg(target_arch = "x86_64")]
pub fn shell_thread() -> ! {
    unsafe { SHELL.set(Shell::new()).ok() };
    print!("$ ");
    loop {
        let shell = unsafe { SHELL.get_mut().unwrap() };
        if let Some((dc, keyboard_state)) = shell.event_queue.pop_front() {
            shell.key_event(dc, keyboard_state);
        }
    }
}

#[cfg(target_arch = "x86_64")]
pub fn cursor_thread() -> ! {
    let mut visible = true;
    loop {
        if visible {
            print!("\x1b[?25h");
        } else {
            print!("\x1b[?25l");
        }
        visible = !visible && unsafe { !RUNNING_RTC };
        crate::scheduler::thread::sleep_ms(500);
    }
}

#[cfg(target_arch = "aarch64")]
pub async fn shell_task() {
    unsafe { SHELL.set(Shell::new()).ok() };
    print!("$ ");
}

pub struct Shell {
    input: Vec<u8>,
    last_commands: Vec<String>,
    prev_commands: Vec<String>,
    color_fg: String,
    color_bg: String,
    #[cfg(target_arch = "x86_64")]
    pub event_queue: VecDeque<(DecodedKey, &'static KeyboardState)>,
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
            #[cfg(target_arch = "x86_64")]
            event_queue: VecDeque::new(),
        }
    }

    #[cfg(target_arch = "x86_64")]
    pub fn key_event(&mut self, dc: DecodedKey, keyboard_state: &KeyboardState) {
        print!("\x1b[?25h"); // cursor high (this is how other shells do it when they have a blinking cursor idk)
        let slice = keyboard_state.keys_down.as_slice();
        if slice.contains(&KeyCode::LControl) && slice.contains(&KeyCode::C) {
            print!("^C\n$ ");
            self.input.clear();
            return;
        }
        if slice.contains(&KeyCode::F1) {
            print!("\ncursor pos: {}\n$ ", crate::utils::term::get_cursor_pos());
            self.input.clear();
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
                    let raw_input = str::from_utf8(&self.input).unwrap().trim().replace(
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
                        print!("\x1b[2K\x1b[1G");
                        if !self.input.is_empty() {
                            self.prev_commands
                                .push(String::from_utf8(self.input.clone()).unwrap());
                        }
                        self.input.clear();
                        let last_input = self.last_commands.pop().unwrap();
                        self.input = last_input.as_bytes().to_vec();
                        print!("$ {last_input}");
                    }
                }
                KeyCode::ArrowDown => {
                    print!("\x1b[2K\x1b[1G");
                    if !self.input.is_empty() {
                        self.last_commands
                            .push(String::from_utf8(self.input.clone()).unwrap());
                    }
                    self.input.clear();
                    self.input.clear();
                    if !self.prev_commands.is_empty() {
                        let last_input = self.prev_commands.pop().unwrap();
                        self.input = last_input.as_bytes().to_vec();
                        print!("$ {last_input}");
                    } else {
                        print!("$ ");
                    }
                }
                _ => {}
            },
        }
    }

    #[cfg(target_arch = "aarch64")]
    pub fn key_event(&mut self, character: char) {
        print!("\x1b[?25h");
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
            let raw_input = str::from_utf8(&self.input).unwrap().trim().replace(
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
        // DecodedKey::RawKey(key) => match key {
        //     KeyCode::ArrowUp => {
        //         if !self.last_commands.is_empty() {
        //             print!("\x1b[2K\x1b[1G");
        //             if !self.input.is_empty() {
        //                 self.prev_commands
        //                     .push(String::from_utf8(self.input.clone()).unwrap());
        //             }
        //             self.input.clear();
        //             let last_input = self.last_commands.pop().unwrap();
        //             self.input = last_input.as_bytes().to_vec();
        //             print!("$ {last_input}");
        //         }
        //     }
        //     KeyCode::ArrowDown => {
        //         print!("\x1b[2K\x1b[1G");
        //         if !self.input.is_empty() {
        //             self.last_commands
        //                 .push(String::from_utf8(self.input.clone()).unwrap());
        //         }
        //         self.input.clear();
        //         self.input.clear();
        //         if !self.prev_commands.is_empty() {
        //             let last_input = self.prev_commands.pop().unwrap();
        //             self.input = last_input.as_bytes().to_vec();
        //             print!("$ {last_input}");
        //         } else {
        //             print!("$ ");
        //         }
        //     }
        //     _ => {}
        // },
    }
}

#[cfg(target_arch = "x86_64")]
static mut RUNNING_RTC: bool = false;

pub fn run_command(cmd: &str, args: Vec<&str>, shell: &mut Shell) {
    match cmd {
        "help" | "?" => {
            println!(
                "\nlist of commands:\n    \
            !! - replaced with the last command (like linux)\n    \
            help (?) - provides help\n    \
            time [digits] - current time given from all the timers\n    \
            date [timezone] - current date given from the RTC\n    \
            rtc [timezone] - does the same as `date` but infinite loop (unless stopped by ESC)\n    \
            epoch [timezone] - does the same as `rtc` but gives epoch timestamp instead\n    \
            clear - clears the screen\n    \
            color [x] changes the color of the terminal (like windows or use hex)\n    \
            bg [x] changes the background of the terminal (like windows or use hex)\n    \
            echo [what] - echoes the input\n    \
            nooo - prints nooo\n    \
            tasks - goofy ahh task manager/system monitor\n    \
            kill [pid] - kill a process\n    \
            ls [path] - lists the current directory\n    \
            pwd - prints the current working directory\n    \
            shutdown - shuts the system down\n    \
            reboot - reboots the system\n    \
            suspend - suspends the system\n    \
            hibernate - hibernates the system\n    \
            pagefault - pagefaults on purpose\n    \
            panic [message] - panics on command with the command arguments as the panic info\n"
            )
        }
        "time" => {
            let digits = if !args.is_empty() {
                args[0].parse().unwrap_or(5).clamp(0, 9)
            } else {
                5
            };
            for timer in time::get_timers().iter() {
                if timer.is_supported() {
                    println!(
                        "{}{}- {}",
                        timer.name,
                        if timer.name.len() == 4 { "" } else { " " },
                        timer.elapsed_pretty(digits)
                    );
                }
            }
        }
        #[cfg(target_arch = "x86_64")]
        "date" => {
            let default_zone = crate::utils::config::get_config().timezone_offset.to_int();
            let zone = if args.is_empty() {
                default_zone
            } else {
                args[0].parse().unwrap_or(default_zone)
            };

            let rtc_time = crate::arch::drivers::time::rtc::RtcTime::current()
                .with_timezone_offset(zone.clamp(-720, 840) as i16)
                .adjusted_for_timezone();
            println!(
                "{} | {}",
                rtc_time.datetime_pretty(),
                rtc_time.timezone_pretty()
            )
        }
        "rtc" => {
            #[cfg(target_arch = "x86_64")]
            {
                unsafe { RUNNING_RTC = true };
                loop {
                    if let Some((dc, keyboard_state)) =
                        unsafe { SHELL.get_mut().unwrap().event_queue.pop_front() }
                    {
                        if dc == DecodedKey::Unicode(0x1B as char) {
                            println!();
                            unsafe { RUNNING_RTC = false };
                            break;
                        }
                        match keyboard_state.keys_down.as_slice() {
                            [KeyCode::LControl, KeyCode::C] | [KeyCode::Escape] => {
                                println!();
                                unsafe { RUNNING_RTC = false };
                                break;
                            }
                            _ => {}
                        }
                    }
                    let default_zone = crate::utils::config::get_config().timezone_offset.to_int();
                    let zone = if args.is_empty() {
                        default_zone
                    } else {
                        args[0].parse().unwrap_or(default_zone)
                    };

                    let rtc_time = crate::arch::drivers::time::rtc::RtcTime::current()
                        .with_timezone_offset(zone.clamp(-720, 840) as i16)
                        .adjusted_for_timezone();
                    print!(
                        "\x1b[2K\r{} | {}",
                        rtc_time.datetime_pretty(),
                        rtc_time.timezone_pretty()
                    );
                    crate::scheduler::thread::sleep_ms(100);
                    crate::utils::asm::halt();
                }
            }
            #[cfg(target_arch = "aarch64")]
            {
                todo!()
            }
        }
        "epoch" => {
            #[cfg(target_arch = "x86_64")]
            {
                unsafe { RUNNING_RTC = true };
                loop {
                    if let Some((dc, keyboard_state)) =
                        unsafe { SHELL.get_mut().unwrap().event_queue.pop_front() }
                    {
                        if dc == DecodedKey::Unicode(0x1B as char) {
                            println!();
                            unsafe { RUNNING_RTC = false };
                            break;
                        }
                        match keyboard_state.keys_down.as_slice() {
                            [KeyCode::LControl, KeyCode::C] | [KeyCode::Escape] => {
                                println!();
                                unsafe { RUNNING_RTC = false };
                                break;
                            }
                            _ => {}
                        }
                    }
                    let default_zone = crate::utils::config::get_config().timezone_offset.to_int();
                    let zone = if args.is_empty() {
                        default_zone
                    } else {
                        args[0].parse().unwrap_or(default_zone)
                    };

                    let rtc_time = crate::arch::drivers::time::rtc::RtcTime::current()
                        .with_timezone_offset(zone.clamp(-720, 840) as i16)
                        .adjusted_for_timezone();
                    print!(
                        "\x1b[2K\r{} | {}",
                        rtc_time.to_epoch().unwrap(),
                        rtc_time.timezone_pretty()
                    );
                    crate::scheduler::thread::sleep_ms(100);
                    crate::utils::asm::halt();
                }
            }
            #[cfg(target_arch = "aarch64")]
            {
                todo!()
            }
        }
        "clear" => {
            print!("\x1b[2J\x1b[H");
        }
        "lspci" => {
            crate::device::pci::pci_enumerate();
            for device in unsafe { &crate::device::pci::PCI_DEVICES } {
                println!(
                    "{:02x}:{:02x}:{} {} {:04X}:{:04X} [0x{:X}:0x{:X}:0x{:X}]",
                    device.address.bus,
                    device.address.device,
                    device.address.function,
                    device.name,
                    device.vendor_id,
                    device.device_id,
                    device.class_code,
                    device.subclass,
                    device.prog_if,
                );
            }
        }
        #[cfg(target_arch = "x86_64")]
        "pwd" => {
            let cwd = crate::scheduler::current_process()
                .unwrap()
                .lock()
                .get_cwd()
                .clone();
            println!("{cwd}");
        }
        #[cfg(target_arch = "x86_64")]
        "cd" => {
            let cwd = crate::scheduler::current_process()
                .unwrap()
                .lock()
                .get_cwd()
                .clone();
            let path = if args.is_empty() {
                fs::Path::new("/home")
            } else {
                let p = fs::Path::new(args[0]);
                if let Some(new_p) = fs::get_vfs()
                    .resolve_path(cwd.clone())
                    .unwrap()
                    .resolve_path(p.clone())
                {
                    new_p.get_path().clone()
                } else {
                    println!("cd: {}: No such file or directory", p);
                    return;
                }
            };
            crate::scheduler::current_process()
                .unwrap()
                .lock()
                .set_cwd(path);
        }
        #[cfg(target_arch = "x86_64")]
        "ls" => {
            let cwd = crate::scheduler::current_process()
                .unwrap()
                .lock()
                .get_cwd()
                .clone();
            let path = if args.is_empty() {
                cwd.clone()
            } else {
                let p = fs::Path::new(args[0]);
                if let Some(new_p) = fs::get_vfs()
                    .resolve_path(cwd.clone())
                    .unwrap()
                    .resolve_path(p.clone())
                {
                    new_p.get_path().clone()
                } else {
                    println!("cd: {}: No such file or directory", p);
                    return;
                }
            };
            let vfs = unsafe { fs::VFS.get_mut().unwrap() };
            for child in vfs
                .resolve_path(path)
                .unwrap_or(vfs.resolve_path(cwd).unwrap()) // goofy ahh
                .get_children()
            {
                print!(
                    "{}{}{} ",
                    if child.is_dir() {
                        color::BLUE
                    } else if child.get_permissions().execute {
                        color::GREEN
                    } else {
                        color::WHITE_BRIGHT
                    },
                    child.get_name(),
                    color::RESET
                );
            }
            println!();
        }
        #[cfg(target_arch = "x86_64")]
        "mkdir" => {
            if args.is_empty() {
                println!("mkdir: what name dumass");
                return;
            }
            for arg in args {
                unsafe {
                    fs::VFS
                        .get_mut()
                        .unwrap()
                        .resolve_path_mut(
                            crate::scheduler::current_process()
                                .unwrap()
                                .lock()
                                .get_cwd()
                                .clone(),
                        )
                        .unwrap()
                        .create_dir(arg)
                        .unwrap();
                }
            }
        }
        #[cfg(target_arch = "x86_64")]
        "touch" => {
            if args.is_empty() {
                println!("touch: what name dumass");
                return;
            }
            for arg in args {
                unsafe {
                    fs::VFS
                        .get_mut()
                        .unwrap()
                        .resolve_path_mut(
                            crate::scheduler::current_process()
                                .unwrap()
                                .lock()
                                .get_cwd()
                                .clone(),
                        )
                        .unwrap()
                        .create_file(arg)
                        .unwrap();
                }
            }
        }
        #[cfg(target_arch = "x86_64")]
        "cat" => {
            if args.is_empty() {
                println!("cat: what file dumass");
                return;
            }
            let path = fs::Path::new(args[0]);
            let vfs = unsafe { fs::VFS.get_mut().unwrap() };
            if let Some(file) = vfs
                .resolve_path(
                    crate::scheduler::current_process()
                        .unwrap()
                        .lock()
                        .get_cwd()
                        .clone(),
                )
                .unwrap()
                .resolve_path(path.clone())
            {
                println!("{}", str::from_utf8(file.read().unwrap()).unwrap());
            } else {
                println!("cat: no such file");
            }
        }
        #[cfg(target_arch = "x86_64")]
        "rm" => {
            if args.is_empty() {
                println!("rm: what dumass");
                return;
            }
            let path = fs::Path::new(args[0]);
            if let Some(node) = unsafe {
                fs::VFS
                    .get_mut()
                    .unwrap()
                    .resolve_path_mut(
                        crate::scheduler::current_process()
                            .unwrap()
                            .lock()
                            .get_cwd()
                            .clone(),
                    )
                    .unwrap()
                    .resolve_path_mut(path.clone())
            } {
                node.get_parent_mut()
                    .unwrap()
                    .get_children_mut()
                    .retain(|x| x.get_name() != path.get_name());
            } else {
                println!("rm: no such item");
            }
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
        "tasks" => {
            #[cfg(target_arch = "x86_64")]
            {
                crate::utils::asm::without_ints(|| {
                    let scheduler = crate::scheduler::get_scheduler();

                    println!("Processes running: {}", scheduler.processes.len());

                    for process in scheduler.processes.iter() {
                        let p = process.lock();

                        println!("Process [{}] '{}':", p.get_pid(), p.get_name());

                        for thread in p.get_children().iter() {
                            let t = thread.lock();
                            println!(
                                "  Thread [{}] '{}': {:?}",
                                t.get_tid(),
                                t.get_name(),
                                t.get_status()
                            );
                        }
                    }
                });
            }
            #[cfg(target_arch = "aarch64")]
            {
                todo!()
            }
        }
        "kill" => {
            #[cfg(target_arch = "x86_64")]
            {
                if let Ok(pid) = args[0].parse::<u64>() {
                    if pid == 0 {
                        println!("congrats dumass, you just killed pid 0, which is the kernel")
                    }
                    if crate::scheduler::kill_process(pid) {
                        println!("process {} terminated", pid);
                    } else {
                        println!("process {} not found", pid);
                    }
                } else {
                    println!("invalid pid");
                }
            }
            #[cfg(target_arch = "aarch64")]
            {
                todo!()
            }
        }
        "shutdown" => {
            #[cfg(target_arch = "x86_64")]
            acpi::perform_power_action(acpi::PowerAction::Shutdown);
            #[cfg(target_arch = "aarch64")]
            unsafe {
                core::arch::asm!("mov x0, {0:x}", "hvc #0", in(reg) 0x84000008u64);
            }
        }
        "reboot" => {
            acpi::perform_power_action(acpi::PowerAction::Reboot);
        }
        "sleep" => {
            acpi::perform_power_action(acpi::PowerAction::Sleep);
        }
        "hibernate" => {
            acpi::perform_power_action(acpi::PowerAction::Hibernate);
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
