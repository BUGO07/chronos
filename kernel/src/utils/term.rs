/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::utils::spinlock::Spin;
use crate::{memory::get_memory_init_stage, serial_print};
use alloc::vec::Vec;
use core::{
    alloc::Layout,
    fmt::{self, Write},
    ptr::null_mut,
    sync::atomic::{AtomicU64, Ordering},
};

use super::limine::get_framebuffers;

lazy_static::lazy_static! {
    pub static ref WRITERS: Spin<Vec<Writer>> = Spin::new(Writer::new());
}

pub struct Writer {
    ctx: *mut flanterm_sys::flanterm_context,
}

unsafe impl Send for Writer {}

extern "C" fn malloc(size: usize) -> *mut core::ffi::c_void {
    unsafe { alloc::alloc::alloc(Layout::from_size_align(size, 0x10).unwrap()) as _ }
}

extern "C" fn free(ptr: *mut core::ffi::c_void, size: usize) {
    unsafe { alloc::alloc::dealloc(ptr as _, Layout::from_size_align(size, 0x10).unwrap()) };
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
                            framebuffer.address() as _,
                            framebuffer.width as _,
                            framebuffer.height as _,
                            framebuffer.pitch as _,
                            framebuffer.red_mask_size,
                            framebuffer.red_mask_shift,
                            framebuffer.green_mask_size,
                            framebuffer.green_mask_shift,
                            framebuffer.blue_mask_size,
                            framebuffer.blue_mask_shift,
                            null_mut(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                            null_mut(),
                            FONT.as_ptr() as _,
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
        let buf = s.as_ptr();
        unsafe { flanterm_sys::flanterm_write(self.ctx, buf as _, s.len()) };
    }
}

static CURSOR_POS: AtomicU64 = AtomicU64::new(0);

extern "C" fn callback(
    _ctx: *mut flanterm_sys::flanterm_context,
    _second: u64,
    cursor_x: u64,
    _fourth: u64,
    _fifth: u64,
) {
    CURSOR_POS.store(cursor_x, Ordering::Relaxed);
}

pub fn get_cursor_pos() -> u64 {
    crate::print!("\x1b[6n");
    CURSOR_POS.load(Ordering::Relaxed)
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
    if get_memory_init_stage() > 0 {
        crate::utils::asm::without_ints(|| {
            for writer in WRITERS.lock().iter_mut() {
                writer.write_fmt(args).expect("Printing failed");
            }
        });
    }
    serial_print!("{}", args);
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
                    let width = get_framebuffers().nth(i).unwrap().width as usize;
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
                    let width = get_framebuffers().nth(i).unwrap().width as usize;
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
