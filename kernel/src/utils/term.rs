/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::utils::spinlock::SpinLock;
use crate::{memory::get_memory_init_stage, serial_print};
use alloc::vec::Vec;
use core::{
    alloc::Layout,
    ffi::c_void,
    fmt::{self, Write},
    ptr::null_mut,
};

use super::limine::get_framebuffers;

lazy_static::lazy_static! {
    pub static ref WRITERS: SpinLock<Vec<Writer>> = SpinLock::new(Writer::new());
}

pub struct Writer {
    ctx: *mut flanterm_sys::flanterm_context,
}

unsafe impl Send for Writer {}
unsafe impl Sync for Writer {}

extern "C" fn malloc(size: usize) -> *mut core::ffi::c_void {
    unsafe { alloc::alloc::alloc(Layout::from_size_align(size, 0x10).unwrap()) as *mut c_void }
}

extern "C" fn free(ptr: *mut core::ffi::c_void, size: usize) {
    unsafe { alloc::alloc::dealloc(ptr as *mut u8, Layout::from_size_align(size, 0x10).unwrap()) };
}

const FONT: &[u8] = include_bytes!("../../res/font.bin");
const FONT_WIDTH: usize = 8;
const FONT_HEIGHT: usize = 16;
const FONT_SPACING: usize = 1;
const FONT_SCALE_X: usize = 1;
const FONT_SCALE_Y: usize = 1;
const MARGIN: usize = 10;

impl Writer {
    fn new() -> Vec<Writer> {
        let mut flanterm_contexts = Vec::new();
        #[cfg(not(feature = "uacpi_test"))]
        {
            for framebuffer in get_framebuffers() {
                unsafe {
                    let writer = Writer {
                        ctx: flanterm_sys::flanterm_fb_init(
                            Some(malloc),
                            Some(free),
                            framebuffer.addr() as *mut u32,
                            framebuffer.width() as usize,
                            framebuffer.height() as usize,
                            framebuffer.pitch() as usize,
                            framebuffer.red_mask_size(),
                            framebuffer.red_mask_shift(),
                            framebuffer.green_mask_size(),
                            framebuffer.green_mask_shift(),
                            framebuffer.blue_mask_size(),
                            framebuffer.blue_mask_shift(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                            FONT.as_ptr() as *mut core::ffi::c_void,
                            FONT_WIDTH,
                            FONT_HEIGHT,
                            FONT_SPACING,
                            FONT_SCALE_X,
                            FONT_SCALE_Y,
                            MARGIN,
                        ),
                    };
                    flanterm_sys::flanterm_set_callback(writer.ctx, Some(callback));

                    flanterm_contexts.push(writer);
                }
            }
        }
        flanterm_contexts
    }

    fn write(&mut self, s: &str) {
        #[cfg(target_arch = "x86_64")]
        let buf = s.as_ptr() as *const i8;
        #[cfg(target_arch = "aarch64")]
        let buf = s.as_ptr();
        unsafe { flanterm_sys::flanterm_write(self.ctx, buf, s.len()) };
    }
}

static mut CURSOR_POS: u64 = 0;

extern "C" fn callback(
    _ctx: *mut flanterm_sys::flanterm_context,
    _second: u64,
    cursor_x: u64,
    _fourth: u64,
    _fifth: u64,
) {
    unsafe { CURSOR_POS = cursor_x };
}

pub fn get_cursor_pos() -> u64 {
    crate::print!("\x1b[6n");
    unsafe { CURSOR_POS }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        #[cfg(not(feature = "uacpi_test"))]
        self.write(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::utils::term::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[macro_export]
macro_rules! print_fill {
    ($what:expr) => {
        $crate::utils::term::_print_fill($what, "", true)
    };
    ($what:expr, $with:expr) => {
        $crate::utils::term::_print_fill($what, $with, true)
    };
    ($what:expr, $with:expr, $newline:expr) => {
        $crate::utils::term::_print_fill($what, $with, $newline)
    };
}

#[macro_export]
macro_rules! print_centered {
    ($what:expr) => {
        $crate::utils::term::_print_centered($what, "", true)
    };
    ($what:expr, $with:expr) => {
        $crate::utils::term::_print_centered($what, $with, true)
    };
    ($what:expr, $with:expr, $newline:expr) => {
        $crate::utils::term::_print_centered($what, $with, $newline)
    };
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    serial_print!("{}", args);
    if get_memory_init_stage() > 0 {
        let closure = || {
            for writer in WRITERS.lock().iter_mut() {
                writer.write_fmt(args).expect("Printing failed");
            }
        };
        crate::utils::asm::without_ints(closure);
    }
}

#[doc(hidden)]
pub fn _print_fill(what: &str, with: &str, newline: bool) {
    #[cfg(not(feature = "uacpi_test"))]
    {
        if with.is_empty() {
            serial_print!("{}", what.repeat(65));
        } else {
            serial_print!("{} {} {}", what.repeat(25), with, what.repeat(25));
        }
        if newline {
            serial_print!("\n");
        }
        if get_memory_init_stage() > 0 {
            crate::utils::asm::without_ints(|| {
                for (i, writer) in WRITERS.lock().iter_mut().enumerate() {
                    let width = get_framebuffers().nth(i).unwrap().width() as usize;
                    let cols = (width - 2 * MARGIN) / (1 + FONT_WIDTH * FONT_SCALE_X);
                    if with.is_empty() {
                        writer
                            .write_fmt(format_args!("{}", what.repeat(cols)))
                            .expect("Printing failed");
                    } else {
                        let x = what.repeat(cols / 2 - with.len() / 2 - 1);
                        writer
                            .write_fmt(format_args!(
                                "{} {} {}{}",
                                x,
                                with,
                                x,
                                if !cols.is_multiple_of(2) { what } else { "" }
                            ))
                            .expect("Printing failed");
                    };
                    if newline {
                        writer.write("\n");
                    }
                }
            });
        }
    }
}

#[doc(hidden)]
pub fn _print_centered(what: &str, with: &str, newline: bool) {
    #[cfg(not(feature = "uacpi_test"))]
    {
        serial_print!("{}", what);
        if newline {
            serial_print!("\n");
        }
        if get_memory_init_stage() > 0 {
            crate::utils::asm::without_ints(|| {
                for (i, writer) in WRITERS.lock().iter_mut().enumerate() {
                    let width = get_framebuffers().nth(i).unwrap().width() as usize;
                    let cols = (width - 2 * MARGIN) / (1 + FONT_WIDTH * FONT_SCALE_X);
                    for line in what.split('\n') {
                        let space = " ".repeat((cols / 2).saturating_sub(line.len() / 2 + 3));
                        writer
                            .write_fmt(format_args!(
                                "{}{} {} {}{}{}",
                                with,
                                space,
                                line,
                                space,
                                if !cols.is_multiple_of(2) ^ !line.len().is_multiple_of(2) {
                                    " "
                                } else {
                                    "  "
                                },
                                with,
                            ))
                            .expect("Printing failed");
                        if newline {
                            writer.write("\n");
                        }
                    }
                }
            });
        }
    }
}
