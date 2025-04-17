/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{memory::FINISHED_INIT, serial_print};
use alloc::vec::Vec;
use core::{
    alloc::Layout,
    ffi::c_void,
    fmt::{self, Write},
    ptr::null_mut,
};
use lazy_static::lazy_static;
use limine::request::FramebufferRequest;
use spin::Mutex;
use x86_64::instructions::interrupts;

lazy_static! {
    pub static ref WRITERS: Mutex<Vec<Writer>> = Mutex::new(Writer::new());
}

pub struct Writer {
    pub ctx: *mut flanterm::sys::flanterm_context,
}

unsafe impl Send for Writer {}
unsafe impl Sync for Writer {}

#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

unsafe extern "C" fn malloc(x: usize) -> *mut core::ffi::c_void {
    unsafe { alloc::alloc::alloc(Layout::from_size_align(x, 0x10).unwrap()) as *mut c_void }
}

unsafe extern "C" fn free(x: *mut core::ffi::c_void, y: usize) {
    unsafe {
        alloc::alloc::dealloc(x as *mut u8, Layout::from_size_align(y, 0x10).unwrap());
    }
}

pub fn get_framebuffers()
-> impl core::iter::Iterator<Item = limine::framebuffer::Framebuffer<'static>> {
    FRAMEBUFFER_REQUEST.get_response().unwrap().framebuffers()
}

const FONT_WIDTH: usize = 8;
const FONT_HEIGHT: usize = 16;
const FONT_SPACING: usize = 1;
const FONT_SCALE_X: usize = 1;
const FONT_SCALE_Y: usize = 1;
const MARGIN: usize = 10;

pub struct Cursor {
    pub row: usize,
    pub col: usize,
}

impl Cursor {
    pub fn new() -> Self {
        Self { row: 1, col: 1 }
    }

    pub fn move_to(&mut self, row: usize, col: usize) {
        crate::print!("\x1b[{};{}H", row, col);
    }

    pub fn move_right(&mut self, n: usize) {
        self.col += n;
        crate::print!("\x1b[{}C", n);
    }

    pub fn move_left(&mut self, n: usize) {
        self.col = self.col.saturating_sub(n);
        crate::print!("\x1b[{}D", n);
    }
}

impl Writer {
    pub fn new() -> Vec<Writer> {
        let mut flanterm_contexts = Vec::new();
        for framebuffer in get_framebuffers() {
            unsafe {
                flanterm_contexts.push(Writer {
                    ctx: flanterm::sys::flanterm_fb_init(
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
                        include_bytes!("../../res/font.bin").as_ptr() as *mut core::ffi::c_void,
                        FONT_WIDTH,
                        FONT_HEIGHT,
                        FONT_SPACING,
                        FONT_SCALE_X,
                        FONT_SCALE_Y,
                        MARGIN,
                    ),
                });
            }
        }
        return flanterm_contexts;
    }

    pub fn write(&mut self, s: &str) {
        unsafe { flanterm::sys::flanterm_write(self.ctx, s.as_ptr() as *const i8, s.len()) };
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
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
        $crate::utils::term::_print_fill($what, "")
    };
    ($what:expr, $with:expr) => {
        $crate::utils::term::_print_fill($what, $with)
    };
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    serial_print!("{}", args);
    if unsafe { FINISHED_INIT } {
        interrupts::without_interrupts(|| {
            for writer in WRITERS.lock().iter_mut() {
                writer.write_fmt(args).expect("Printing failed");
            }
        });
    }
}

#[doc(hidden)]
pub fn _print_fill(what: &str, with: &str) {
    if with.is_empty() {
        serial_print!("{}\n", what.repeat(65));
    } else {
        serial_print!("{} {} {}\n", what.repeat(25), with, what.repeat(25));
    }
    if unsafe { FINISHED_INIT } {
        interrupts::without_interrupts(|| {
            for (i, writer) in WRITERS.lock().iter_mut().enumerate() {
                let fbw = get_framebuffers().nth(i).unwrap().width() as usize;
                if with.is_empty() {
                    writer
                        .write_fmt(format_args!(
                            "{}\n",
                            what.repeat(fbw / (FONT_WIDTH * FONT_SCALE_X) - MARGIN * 2)
                        ))
                        .expect("Printing failed");
                } else {
                    let x = what.repeat(
                        fbw / (FONT_WIDTH * FONT_SCALE_X * 2)
                            - (MARGIN + with.len() / 2 + 1 + with.len() % 2),
                    );
                    writer
                        .write_fmt(format_args!("{} {} {}\n", x, with, x))
                        .expect("Printing failed");
                };
            }
        });
    }
}
