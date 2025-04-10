use core::fmt::Write;

use x86_64::instructions::port::Port;

pub mod logger;
pub mod term;
pub mod time;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum QemuExitCode {
    Success = 0x10,
    Failed = 0x11,
}

pub fn exit_qemu(exit_code: QemuExitCode) {
    unsafe {
        let mut port = Port::new(0xf4);
        port.write(exit_code as u32);
    }
}

pub fn halt_loop() -> ! {
    loop {
        x86_64::instructions::hlt();
    }
}

pub struct Buffer {
    data: [u8; 256],
    pos: usize,
}

impl Buffer {
    pub fn new() -> Self {
        Self {
            data: [0; 256],
            pos: 0,
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.pos]
    }
}

impl Write for Buffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let len = bytes.len();
        if self.pos + len > self.data.len() {
            return Err(core::fmt::Error);
        }
        self.data[self.pos..self.pos + len].copy_from_slice(bytes);
        self.pos += len;
        Ok(())
    }
}
