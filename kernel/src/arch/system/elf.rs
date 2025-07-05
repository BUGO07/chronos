/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{alloc::Layout, ffi::c_void};

use std::{
    align_down, align_up,
    asm::mem::memcpy,
    kernel::{
        bootloader::get_hhdm_offset,
        paging::{Pagemap, flag, page_size},
    },
    spinlock::SpinLock,
};

use alloc::sync::Arc;

#[cfg(target_arch = "x86_64")]
pub const EM_CURRENT: u16 = 0x3E;
#[cfg(target_arch = "aarch64")]
pub const EM_CURRENT: u16 = 0xB7;

pub const PT_NULL: u32 = 0x00000000;
pub const PT_LOAD: u32 = 0x00000001;
pub const PT_DYNAMIC: u32 = 0x00000002;
pub const PT_INTERP: u32 = 0x00000003;
pub const PT_NOTE: u32 = 0x00000004;
pub const PT_SHLIB: u32 = 0x00000005;
pub const PT_PHDR: u32 = 0x00000006;
pub const PT_TLS: u32 = 0x00000007;
pub const PT_MODVERSION: u32 = 0x60000001;
pub const PT_MODAUTHOR: u32 = 0x60000002;
pub const PT_MODDESC: u32 = 0x60000003;

pub const PF_EXECUTE: u32 = 0x01;
pub const PF_WRITE: u32 = 0x02;
pub const PF_READ: u32 = 0x04;

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ElfProgramHeader {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct ElfHeader {
    pub e_ident: [u8; 16],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

#[derive(Debug)]
pub enum ElfLoadError {
    InvalidMagic,
    InvalidEntrypoint,
    InvalidVersion,
    UnsupportedClass,
    UnsupportedDataEncoding,
    UnsupportedMachine,
    InvalidProgramHeaderOffset,
}

pub fn load_elf(binary: &[u8], pagemap: Arc<SpinLock<Pagemap>>) -> Result<u64, ElfLoadError> {
    let header = unsafe {
        if binary.len() < core::mem::size_of::<ElfHeader>() {
            return Err(ElfLoadError::InvalidMagic);
        }
        &*(binary.as_ptr() as *const ElfHeader)
    };

    if header.e_entry == 0 {
        return Err(ElfLoadError::InvalidEntrypoint);
    }

    if &header.e_ident[0..4] != b"\x7FELF" {
        return Err(ElfLoadError::InvalidMagic);
    }

    if header.e_version != 1 {
        return Err(ElfLoadError::InvalidVersion);
    }

    if header.e_ident[4] != 2 {
        return Err(ElfLoadError::UnsupportedClass);
    }

    if header.e_ident[5] != 1 {
        return Err(ElfLoadError::UnsupportedDataEncoding);
    }

    if header.e_machine != EM_CURRENT {
        return Err(ElfLoadError::UnsupportedMachine);
    }

    let mut pmap = pagemap.lock();

    for i in 0..header.e_phnum {
        let program_header_offset =
            header.e_phoff as usize + (i as usize * header.e_phentsize as usize);

        if program_header_offset + core::mem::size_of::<ElfProgramHeader>() > binary.len() {
            return Err(ElfLoadError::InvalidProgramHeaderOffset);
        }

        let ph =
            unsafe { &*(binary.as_ptr().add(program_header_offset) as *const ElfProgramHeader) };

        if ph.p_type != PT_LOAD {
            continue;
        }

        let virt_start = ph.p_vaddr;
        let virt_end = virt_start + ph.p_memsz;

        let start = align_down(virt_start, 0x1000);
        let end = align_up(virt_end, 0x1000);

        let mut vmm_flags = flag::RW | flag::USER;
        if (ph.p_flags & PF_EXECUTE) == 0 {
            vmm_flags |= flag::NO_EXEC;
        }

        for i in (0..(end - start)).step_by(0x1000) {
            let vaddr = start + i;

            let paddr = unsafe {
                alloc::alloc::alloc_zeroed(Layout::from_size_align(0x1000, 0x1000).unwrap())
            } as u64
                - get_hhdm_offset();

            pmap.map(vaddr, paddr, vmm_flags, page_size::SMALL);
        }

        unsafe {
            let dst_ptr = virt_start as *mut c_void;
            let src_ptr = binary.as_ptr().add(ph.p_offset as usize) as *mut c_void;

            if ph.p_filesz > 0 {
                if (ph.p_offset as usize + ph.p_filesz as usize) > binary.len() {
                    return Err(ElfLoadError::InvalidProgramHeaderOffset);
                }
                memcpy(dst_ptr, src_ptr, ph.p_filesz as usize);
            }

            if ph.p_memsz > ph.p_filesz {
                core::ptr::write_bytes(
                    dst_ptr.add(ph.p_filesz as usize),
                    0,
                    (ph.p_memsz - ph.p_filesz) as usize,
                );
            }
        }
    }

    Ok(header.e_entry)
}
