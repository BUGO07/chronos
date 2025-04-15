/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use crate::{memory::FINISHED_INIT, serial_print};
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
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer::new());
}

pub struct Writer {
    pub ctx: Option<*mut flanterm_bindings::flanterm_context>,
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

impl Writer {
    pub fn new() -> Writer {
        if let Some(framebuffer) = get_framebuffers().next() {
            unsafe {
                Self {
                    ctx: Some(flanterm_bindings::flanterm_fb_init(
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
                        include_bytes!("../../assets/font.bin").as_ptr() as *mut core::ffi::c_void,
                        8,
                        16,
                        1,
                        1,
                        1,
                        10,
                    )),
                }
            }
        } else {
            Self { ctx: None }
        }
    }

    pub fn write(&mut self, s: &str) {
        unsafe {
            flanterm_bindings::flanterm_write(self.ctx.unwrap(), s.as_ptr() as *const i8, s.len())
        };
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

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    serial_print!("{}", args);
    if unsafe { FINISHED_INIT } {
        interrupts::without_interrupts(|| {
            WRITER.lock().write_fmt(args).expect("Printing failed");
        });
    }
}
