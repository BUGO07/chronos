/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::alloc::Layout;

use crate::{
    debug,
    memory::vmm::{Pagemap, flag, page_size},
    utils::{asm::mem::memcpy, limine::get_hhdm_offset},
};

const ELF_MAGIC: [u8; 4] = [0x7F, b'E', b'L', b'F'];

const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const ET_EXEC: u16 = 2;
const EM_X86_64: u16 = 62;
const PT_LOAD: u32 = 1;

const PF_X: u32 = 1;
const PF_W: u32 = 2;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Elf64Ehdr {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Elf64Phdr {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

pub struct ElfInfo {
    pub entry: u64,
}

pub fn load_elf(data: &[u8], pagemap: &mut Pagemap) -> Result<ElfInfo, &'static str> {
    if data.len() < size_of::<Elf64Ehdr>() {
        return Err("ELF too small");
    }

    let ehdr = unsafe { core::ptr::read_unaligned(data.as_ptr() as *const Elf64Ehdr) };

    if ehdr.e_ident[0..4] != ELF_MAGIC {
        return Err("invalid ELF magic");
    }
    if ehdr.e_ident[4] != ELFCLASS64 {
        return Err("not ELF64");
    }
    if ehdr.e_ident[5] != ELFDATA2LSB {
        return Err("not little-endian");
    }
    if ehdr.e_type != ET_EXEC {
        return Err("not an executable");
    }
    if ehdr.e_machine != EM_X86_64 {
        return Err("not x86_64");
    }

    let ph_offset = ehdr.e_phoff as usize;
    let ph_size = ehdr.e_phentsize as usize;
    let ph_count = ehdr.e_phnum as usize;

    let ph_table_end = ph_size
        .checked_mul(ph_count)
        .and_then(|v| v.checked_add(ph_offset))
        .ok_or("program header table overflow")?;
    if ph_table_end > data.len() {
        return Err("program headers out of bounds");
    }

    let hhdm = get_hhdm_offset();

    for i in 0..ph_count {
        let phdr = unsafe {
            core::ptr::read_unaligned(data.as_ptr().add(ph_offset + i * ph_size) as *const Elf64Phdr)
        };

        if phdr.p_type != PT_LOAD {
            continue;
        }

        let seg_end = phdr
            .p_offset
            .checked_add(phdr.p_filesz)
            .ok_or("segment offset+filesz overflow")?;
        if seg_end > data.len() as u64 {
            return Err("segment data out of bounds");
        }

        let vaddr_top = phdr
            .p_vaddr
            .checked_add(phdr.p_memsz)
            .ok_or("segment vaddr+memsz overflow")?;

        let mut flags = flag::PRESENT | flag::USER;
        if phdr.p_flags & PF_W != 0 {
            flags |= flag::WRITE;
        }
        if phdr.p_flags & PF_X == 0 {
            flags |= flag::NO_EXEC;
        }

        let vaddr_base = crate::utils::align_down(phdr.p_vaddr, page_size::SMALL);
        let vaddr_end = crate::utils::align_up(vaddr_top, page_size::SMALL);
        let total_pages = (vaddr_end - vaddr_base) as usize;

        let alloc_ptr = unsafe {
            alloc::alloc::alloc_zeroed(
                Layout::from_size_align(total_pages, page_size::SMALL as usize).unwrap(),
            )
        };
        if alloc_ptr.is_null() {
            return Err("allocation failed");
        }
        let phys_base = alloc_ptr as u64 - hhdm;

        for offset in (0..total_pages as u64).step_by(page_size::SMALL as usize) {
            pagemap
                .map(
                    vaddr_base + offset,
                    phys_base + offset,
                    flags,
                    page_size::SMALL,
                )
                .map_err(|_| "failed to map ELF segment")?;
        }

        if phdr.p_filesz > 0 {
            memcpy(
                (alloc_ptr as u64 + (phdr.p_vaddr - vaddr_base)) as _,
                unsafe { data.as_ptr().add(phdr.p_offset as usize) } as _,
                phdr.p_filesz as usize,
            );
        }

        debug!(
            "elf: loaded segment vaddr=0x{:X} memsz=0x{:X} filesz=0x{:X} flags=0x{:X}",
            phdr.p_vaddr, phdr.p_memsz, phdr.p_filesz, phdr.p_flags
        );
    }

    Ok(ElfInfo {
        entry: ehdr.e_entry,
    })
}
