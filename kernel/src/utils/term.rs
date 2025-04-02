use crate::serial_print;
use core::{
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

impl Writer {
    pub fn new() -> Writer {
        if let Some(framebuffer_response) = FRAMEBUFFER_REQUEST.get_response() {
            if let Some(framebuffer) = framebuffer_response.framebuffers().next() {
                unsafe {
                    Self {
                        ctx: Some(flanterm_bindings::flanterm_fb_init(
                            None,
                            None,
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
                            null_mut(),
                            9,
                            18,
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
    interrupts::without_interrupts(|| {
        WRITER.lock().write_fmt(args).expect("Printing failed");
    });
}
