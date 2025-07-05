/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::{alloc::Layout, cell::OnceCell, ffi::CStr, ptr::null_mut};

use crate::{error, kernel::bootloader::get_hhdm_offset, spinlock::SpinLock};

pub mod page_size {
    pub const SMALL: u64 = 0x1000; // 4KiB
    pub const MEDIUM: u64 = 0x200000; // 2MiB
    pub const LARGE: u64 = 0x40000000; // 1GiB
}

pub mod flag {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITE: u64 = 1 << 1;
    pub const USER: u64 = 1 << 2;
    pub const LPAGES: u64 = 1 << 7;
    pub const NO_EXEC: u64 = 1 << 63;

    pub const RW: u64 = PRESENT | WRITE;
}

pub static mut PAGEMAP: OnceCell<Arc<SpinLock<Pagemap>>> = OnceCell::new(); // wtf

unsafe impl Send for Pagemap {}
unsafe impl Sync for Pagemap {}

#[repr(C, packed)]
pub struct Table {
    pub entries: [u64; 512],
}

#[derive(Copy, Clone)]
pub struct Pagemap {
    pub top_level: *mut Table,
    pub used_pages: u64,
}

impl Default for Pagemap {
    fn default() -> Self {
        Self::new()
    }
}

impl Pagemap {
    pub fn new() -> Pagemap {
        Pagemap {
            top_level: alloc_table(),
            used_pages: 0,
        }
    }

    pub fn map(&mut self, virt: u64, phys: u64, mut flags: u64, psize: u64) -> bool {
        let hhdm = get_hhdm_offset();
        if !(phys <= !0 - hhdm || phys >= hhdm) {
            error!("illegal physical address: 0x{:X}", phys);
            return false;
        }
        let pml4_entry = (virt & (0x1ff << 39)) >> 39;
        let pml3_entry = (virt & (0x1ff << 30)) >> 30;
        let pml2_entry = (virt & (0x1ff << 21)) >> 21;
        let pml1_entry = (virt & (0x1ff << 12)) >> 12;

        let pml4 = (self.top_level as u64 + hhdm) as *mut Table;

        let pml3 = get_next_level(pml4, pml4_entry, true);
        if pml3.is_null() {
            return false;
        }
        if psize == page_size::LARGE {
            flags |= flag::LPAGES;
            unsafe {
                (*pml3).entries[pml3_entry as usize] = phys | flags;
            }
            return true;
        }

        let pml2 = get_next_level(pml3, pml3_entry, true);
        if pml2.is_null() {
            return false;
        }
        if psize == page_size::MEDIUM {
            flags |= flag::LPAGES;
            unsafe {
                (*pml2).entries[pml2_entry as usize] = phys | flags;
            }
            return true;
        }

        let pml1 = get_next_level(pml2, pml2_entry, true);
        if pml1.is_null() {
            return false;
        }

        unsafe {
            (*pml1).entries[pml1_entry as usize] = phys | flags;
            core::arch::asm!("invlpg [{}]", in(reg) virt as *const u8, options(nostack, preserves_flags));
            self.used_pages += psize / 0x1000;
        }

        true
    }

    pub fn unmap(&mut self, virt: u64, psize: u64) -> bool {
        let pml4_entry = (virt >> 39) & 0x1ff;
        let pml3_entry = (virt >> 30) & 0x1ff;
        let pml2_entry = (virt >> 21) & 0x1ff;
        let pml1_entry = (virt >> 12) & 0x1ff;

        let hhdm = get_hhdm_offset();
        let pml4 = (self.top_level as u64 + hhdm) as *mut Table;

        let pml3 = get_next_level(pml4, pml4_entry, false);
        if pml3.is_null() {
            return false;
        }

        if psize == page_size::LARGE {
            unsafe {
                (*pml3).entries[pml3_entry as usize] = 0;
            }
            return true;
        }

        let pml2 = get_next_level(pml3, pml3_entry, false);
        if pml2.is_null() {
            return false;
        }

        if psize == page_size::MEDIUM {
            unsafe {
                (*pml2).entries[pml2_entry as usize] = 0;
            }
            return true;
        }

        let pml1 = get_next_level(pml2, pml2_entry, false);
        if pml1.is_null() {
            return false;
        }

        unsafe {
            (*pml1).entries[pml1_entry as usize] = 0;
            core::arch::asm!("invlpg [{}]", in(reg) virt as *const u8, options(nostack, preserves_flags));
            self.used_pages = self.used_pages.saturating_sub(psize / 0x1000);
        }

        true
    }

    pub fn copy_kernel_map(self) -> Self {
        let src = unsafe { PAGEMAP.get().unwrap().clone() };
        let hhdm = get_hhdm_offset();
        let src_pml4 = (src.lock().top_level as u64 + hhdm) as *const Table;
        let dst_pml4 = (self.top_level as u64 + hhdm) as *mut Table;

        unsafe {
            for i in 256..512 {
                (*dst_pml4).entries[i] = (*src_pml4).entries[i];
            }
        }

        self
    }

    pub fn translate(&self, virt: u64) -> Option<u64> {
        let pml4_index = (virt >> 39) & 0x1ff;
        let pml3_index = (virt >> 30) & 0x1ff;
        let pml2_index = (virt >> 21) & 0x1ff;
        let pml1_index = (virt >> 12) & 0x1ff;
        let offset = virt & 0xfff;
        let hhdm = get_hhdm_offset();

        unsafe {
            let pml4 = (self.top_level as u64 + hhdm) as *const Table;
            let pml4e = (*pml4).entries[pml4_index as usize];
            if pml4e & flag::PRESENT == 0 {
                return None;
            }

            let pml3 = ((pml4e & 0x000FFFFFFFFFF000) + hhdm) as *const Table;
            let pml3e = (*pml3).entries[pml3_index as usize];
            if pml3e & flag::PRESENT == 0 {
                return None;
            }
            if pml3e & flag::LPAGES != 0 {
                let phys = (pml3e & 0x000FFFFFE00000) + (virt & 0x3FFFFFFF); // 1GiB offset
                return Some(phys);
            }

            let pml2 = ((pml3e & 0x000FFFFFFFFFF000) + hhdm) as *const Table;
            let pml2e = (*pml2).entries[pml2_index as usize];
            if pml2e & flag::PRESENT == 0 {
                return None;
            }
            if pml2e & flag::LPAGES != 0 {
                let phys = (pml2e & 0x000FFFFFFFE00000) + (virt & 0x1FFFFF); // 2MiB offset
                return Some(phys);
            }

            let pml1 = ((pml2e & 0x000FFFFFFFFFF000) + hhdm) as *const Table;
            let pml1e = (*pml1).entries[pml1_index as usize];
            if pml1e & flag::PRESENT == 0 {
                return None;
            }

            let phys = (pml1e & 0x000FFFFFFFFFF000) + offset;
            Some(phys)
        }
    }

    pub fn read_user_ptr(&self, user_ptr: u64) -> Result<u64, &'static str> {
        let mut bytes = [0u8; 8];
        #[allow(clippy::needless_range_loop)]
        for i in 0..8 {
            let va = user_ptr + i as u64;
            let phys = self.translate(va).ok_or("invalid user memory")?;
            bytes[i] = unsafe { *((get_hhdm_offset() + phys) as *const u8) };
        }
        Ok(u64::from_le_bytes(bytes))
    }

    pub fn read_user_bytes(&self, user_ptr: u64, len: usize) -> Result<Vec<u8>, &'static str> {
        let mut buf = Vec::with_capacity(len);
        for offset in 0..len {
            let va = user_ptr + offset as u64;
            let phys = self.translate(va).ok_or("invalid user memory")?;
            let byte = unsafe { *((get_hhdm_offset() + phys) as *const u8) };
            buf.push(byte);
        }
        Ok(buf)
    }

    pub fn read_user_c_string(
        &self,
        user_ptr: u64,
        max_len: usize,
    ) -> Result<String, &'static str> {
        let mut buf = Vec::with_capacity(max_len);

        for i in 0..max_len {
            let va = user_ptr + i as u64;
            let phys = self.translate(va).ok_or("invalid user memory")?;
            let byte = unsafe { *((get_hhdm_offset() + phys) as *const u8) };
            buf.push(byte);
            if byte == 0 {
                break;
            }
        }

        // If no null terminator found in max_len
        if *buf.last().unwrap_or(&1) != 0 {
            return Err("no null terminator");
        }

        let cstr = CStr::from_bytes_with_nul(&buf).map_err(|_| "invalid C string")?;
        Ok(cstr.to_string_lossy().to_string())
    }

    pub fn read_user_array<T: Copy>(
        &self,
        user_ptr: u64,
        count: usize,
    ) -> Result<Vec<T>, &'static str> {
        let mut result = Vec::with_capacity(count);
        let size = core::mem::size_of::<T>();

        for i in 0..count {
            let mut val_bytes = alloc::vec![0u8; size];
            #[allow(clippy::needless_range_loop)]
            for b in 0..size {
                let va = user_ptr + (i * size + b) as u64;
                let phys = self.translate(va).ok_or("invalid user memory")?;
                val_bytes[b] = unsafe { *((get_hhdm_offset() + phys) as *const u8) };
            }
            let val = unsafe { core::ptr::read_unaligned(val_bytes.as_ptr() as *const T) };
            result.push(val);
        }

        Ok(result)
    }

    pub fn write_user_byte(&self, user_ptr: u64, value: u8) -> Result<(), &'static str> {
        let phys = self.translate(user_ptr).ok_or("invalid user memory")?;
        unsafe {
            *((get_hhdm_offset() + phys) as *mut u8) = value;
        }
        Ok(())
    }

    pub fn write_user_bytes(&self, user_ptr: u64, data: &[u8]) -> Result<(), &'static str> {
        for (i, &byte) in data.iter().enumerate() {
            let va = user_ptr + i as u64;
            let phys = self.translate(va).ok_or("invalid user memory")?;
            unsafe {
                *((get_hhdm_offset() + phys) as *mut u8) = byte;
            }
        }
        Ok(())
    }

    pub fn write_user_ptr(&self, user_ptr: u64, value: u64) -> Result<(), &'static str> {
        let bytes = value.to_le_bytes();
        self.write_user_bytes(user_ptr, &bytes)
    }
}

pub fn alloc_table() -> *mut Table {
    unsafe {
        (alloc::alloc::alloc_zeroed(Layout::from_size_align(0x1000, 0x1000).unwrap()) as u64
            - get_hhdm_offset()) as *mut Table
    }
}

fn get_next_level(top_level: *mut Table, idx: u64, allocate: bool) -> *mut Table {
    unsafe {
        let hhdm = get_hhdm_offset();
        let entry = top_level.cast::<u64>().add(idx as usize);
        if !(*entry <= !0 - hhdm || *entry >= hhdm) {
            panic!("illegal entry: 0x{:X}", *entry);
        }
        if *entry & flag::PRESENT != 0 {
            return ((*entry & 0x000FFFFFFFFFF000) + hhdm) as *mut Table;
        }

        if !allocate {
            return null_mut();
        }

        let next_level = alloc_table() as u64;
        *entry = next_level | flag::RW | flag::USER;
        (next_level + hhdm) as *mut Table
    }
}
