/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::fmt::Write;

use x86_64::instructions::interrupts;

use crate::{arch::drivers::pit, serial_print, utils::term::WRITER};

pub mod color {
    pub const RESET: &str = "\x1b[0m";
    pub const BLACK: &str = "\x1b[0;30m";
    pub const GRAY: &str = "\x1b[38;5;243m";
    pub const RED: &str = "\x1b[0;31m";
    pub const GREEN: &str = "\x1b[0;32m";
    pub const YELLOW: &str = "\x1b[38;5;226m";
    pub const BLUE: &str = "\x1b[38;5;69m";
    pub const PURPLE: &str = "\x1b[0;35m";
    pub const CYAN: &str = "\x1b[0;36m";
    pub const WHITE: &str = "\x1b[0;37m";
}

// janky but whatever
pub fn log_message(level: &str, color: &str, mut module_path: &str, args: core::fmt::Arguments) {
    #[cfg(not(feature = "tests"))]
    {
        if level == "DEBUG" && !cfg!(debug_assertions) {
            return;
        }
        module_path = module_path.split("::").last().unwrap();
        let time = pit::time_ms();
        let hours = time / 3_600_000;
        let minutes = (time % 3_600_000) / 60_000;
        let seconds = (time % 60_000) / 1000;
        let millis = time % 1000;
        let padding = if level.len() == 4 { " " } else { "" };

        if unsafe { crate::memory::FINISHED_INIT } {
            interrupts::without_interrupts(|| {
                let mut writer = WRITER.lock();
                writer
                    .write_fmt(format_args!(
                        "[{:02}:{:02}:{:02}.{:03}] [ {}{}{} {}] {}{}:{} {}\n",
                        hours,
                        minutes,
                        seconds,
                        millis,
                        color,
                        level,
                        color::RESET,
                        padding,
                        color::GRAY,
                        module_path,
                        color::RESET,
                        args
                    ))
                    .expect("Printing to WRITER failed");
            });
        }

        serial_print!(
            "[{:02}:{:02}:{:02}.{:03}] [ {}{}{} {}] {}{}:{} {}\n",
            hours,
            minutes,
            seconds,
            millis,
            color,
            level,
            color::RESET,
            padding,
            color::GRAY,
            module_path,
            color::RESET,
            args
        );
    }
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::utils::logger::log_message("info", $crate::utils::logger::color::GREEN, module_path!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => ($crate::utils::logger::log_message("debug", $crate::utils::logger::color::CYAN, module_path!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::utils::logger::log_message("warn", $crate::utils::logger::color::YELLOW, module_path!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::utils::logger::log_message("error", $crate::utils::logger::color::RED, module_path!(), format_args!($($arg)*)));
}
