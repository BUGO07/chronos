/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::arch::asm;

pub fn read(addr: u64, width: usize) -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        match width {
            1 => {
                let value: u8;
                unsafe {
                    asm!(
                        "mov {0}, byte ptr [{1:r}]",
                        out(reg_byte) value,
                        in(reg) addr,
                        options(nostack, readonly)
                    );
                }
                value as u64
            }
            2 => {
                let value: u16;
                unsafe {
                    asm!(
                        "mov {0:x}, word ptr [{1:r}]",
                        out(reg) value,
                        in(reg) addr,
                        options(nostack, readonly)
                    );
                }
                value as u64
            }
            4 => {
                let value: u32;
                unsafe {
                    asm!(
                        "mov {0:e}, dword ptr [{1:r}]",
                        out(reg) value,
                        in(reg) addr,
                        options(nostack, readonly)
                    );
                }
                value as u64
            }
            8 => {
                let value: u64;
                unsafe {
                    asm!(
                        "mov {0:r}, qword ptr [{1:r}]",
                        out(reg) value,
                        in(reg) addr,
                        options(nostack, readonly)
                    );
                }
                value
            }
            _ => panic!("mmio::read: invalid width {width}"),
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        match width {
            1 => {
                barrier();
                let value: u8;
                unsafe {
                    asm!(
                        "ldrb {0:w}, [{1:x}]",
                        out(reg) value,
                        in(reg) addr,
                        options(nostack, readonly)
                    );
                }
                barrier();
                value as u64
            }
            2 => {
                barrier();
                let value: u16;
                unsafe {
                    asm!(
                        "ldrh {0:w}, [{1:x}]",
                        out(reg) value,
                        in(reg) addr,
                        options(nostack, readonly)
                    );
                }
                barrier();
                value as u64
            }
            4 => {
                barrier();
                let value: u32;
                unsafe {
                    asm!(
                        "ldr {0:w}, [{1:x}]",
                        out(reg) value,
                        in(reg) addr,
                        options(nostack, readonly)
                    );
                }
                barrier();
                value as u64
            }
            8 => {
                barrier();
                let value: u64;
                unsafe {
                    asm!(
                        "ldr {0:x}, [{1:x}]",
                        out(reg) value,
                        in(reg) addr,
                        options(nostack, readonly)
                    );
                }
                barrier();
                value
            }
            _ => panic!("mmio::read: invalid width {width}"),
        }
    }
}

pub fn write(addr: u64, val: u64, width: usize) {
    #[cfg(target_arch = "x86_64")]
    match width {
        1 => {
            let val = val as u8;
            unsafe {
                asm!(
                    "mov byte ptr [{0:r}], {1}",
                    in(reg) addr,
                    in(reg_byte) val,
                    options(nostack)
                );
            }
        }
        2 => {
            let val = val as u16;
            unsafe {
                asm!(
                    "mov word ptr [{0:r}], {1:x}",
                    in(reg) addr,
                    in(reg) val,
                    options(nostack)
                );
            }
        }
        4 => {
            let val = val as u32;
            unsafe {
                asm!(
                    "mov dword ptr [{0:r}], {1:e}",
                    in(reg) addr,
                    in(reg) val,
                    options(nostack)
                );
            }
        }
        8 => unsafe {
            asm!(
                "mov qword ptr [{0:r}], {1:r}",
                in(reg) addr,
                in(reg) val,
                options(nostack)
            );
        },
        _ => panic!("mmio::write: invalid width {width}"),
    }

    #[cfg(target_arch = "aarch64")]
    match width {
        1 => {
            let val = val as u8;
            unsafe {
                asm!(
                    "strb {1:w}, [{0:x}]",
                    in(reg) addr,
                    in(reg) val,
                    options(nostack)
                );
            }
        }
        2 => {
            let val = val as u16;
            unsafe {
                asm!(
                    "strh {1:w}, [{0:x}]",
                    in(reg) addr,
                    in(reg) val,
                    options(nostack)
                );
            }
        }
        4 => {
            let val = val as u32;
            unsafe {
                asm!(
                    "str {1:w}, [{0:x}]",
                    in(reg) addr,
                    in(reg) val,
                    options(nostack)
                );
            }
        }
        8 => unsafe {
            asm!(
                "str {1:x}, [{0:x}]",
                in(reg) addr,
                in(reg) val,
                options(nostack)
            );
        },
        _ => panic!("mmio::write: invalid width {width}"),
    }

    #[cfg(target_arch = "aarch64")]
    barrier();
}

#[inline(always)]
#[cfg(target_arch = "aarch64")]
fn barrier() {
    unsafe {
        asm!("dsb sy", "isb", options(nomem, nostack, preserves_flags));
    }
}
