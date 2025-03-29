use core::fmt::Write;

use x86_64::instructions::interrupts;

use crate::{serial_print, utils::term::WRITER};

pub struct Color {
    pub reset: &'static str,
    pub black: &'static str,
    pub red: &'static str,
    pub green: &'static str,
    pub yellow: &'static str,
    pub blue: &'static str,
    pub purple: &'static str,
    pub cyan: &'static str,
    pub white: &'static str,
}

pub const COLOR: Color = Color {
    reset: "\x1b[0m",
    black: "\x1b[0;30m",
    red: "\x1b[0;31m",
    green: "\x1b[0;32m",
    yellow: "\x1b[0;33m",
    blue: "\x1b[0;34m",
    purple: "\x1b[0;35m",
    cyan: "\x1b[0;36m",
    white: "\x1b[0;37m",
};

// janky but whatever
pub fn log_message(level: &str, color: &str, mut module_path: &str, args: core::fmt::Arguments) {
    #[cfg(not(feature = "tests"))]
    {
        module_path = module_path.split("::").last().unwrap();
        interrupts::without_interrupts(|| {
            let mut writer = WRITER.lock();
            writer
                .write_fmt(format_args!(
                    "{}[{}]{} {}{}:{} {}\n",
                    color, level, COLOR.reset, COLOR.green, module_path, COLOR.reset, args
                ))
                .expect("Printing to WRITER failed");
        });

        serial_print!(
            "{}[{}]{} {}{}:{} {}\n",
            color,
            level,
            COLOR.reset,
            COLOR.green,
            module_path,
            COLOR.reset,
            args
        );
    }
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => ($crate::utils::logger::log_message("INFO", $crate::utils::logger::COLOR.cyan, module_path!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => ($crate::utils::logger::log_message("WARN", $crate::utils::logger::COLOR.yellow, module_path!(), format_args!($($arg)*)));
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => ($crate::utils::logger::log_message("ERROR", $crate::utils::logger::COLOR.red, module_path!(), format_args!($($arg)*)));
}
