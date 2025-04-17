#![no_std]

use core::ptr::NonNull;

#[allow(nonstandard_style)]
pub mod sys;

#[repr(transparent)]
pub struct Context {
    raw: NonNull<sys::flanterm_context>,
}

impl Context {
    pub fn write(&mut self, buf: &[u8]) {
        unsafe { sys::flanterm_write(self.raw.as_ptr(), buf.as_ptr().cast(), buf.len()) }
    }
}

impl Drop for Context {
    fn drop(&mut self) {
        unsafe {
            sys::flanterm_deinit(self.raw.as_ptr(), None);
        }
    }
}
