/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{alloc::Layout, arch::asm};

use crate::{info, memory::KERNEL_STACK_SIZE};

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SegmentSelector(pub u16);

impl SegmentSelector {
    pub const fn new(index: u16, table_indicator: u8, ring: u8) -> Self {
        SegmentSelector((index << 3) | ((table_indicator as u16) << 2) | (ring as u16))
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct GdtPtr {
    limit: u16,
    base: u64,
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct GdtEntry {
    limit_low: u16,
    base_low: u16,
    base_middle: u8,
    access: u8,
    granularity: u8,
    base_high: u8,
}

impl GdtEntry {
    const fn null() -> Self {
        GdtEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            granularity: 0,
            base_high: 0,
        }
    }
    const fn kernel_code() -> Self {
        GdtEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0b1001_1010,
            granularity: 0b0010_0000,
            base_high: 0,
        }
    }
    const fn kernel_data() -> Self {
        GdtEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0b1001_0010,
            granularity: 0b0010_0000,
            base_high: 0,
        }
    }
    const fn user_code_32bit() -> Self {
        GdtEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0b1111_1010,
            granularity: 0b0100_0000,
            base_high: 0,
        }
    }
    const fn user_code() -> Self {
        GdtEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0b1111_1010,
            granularity: 0b0010_0000,
            base_high: 0,
        }
    }
    const fn user_data() -> Self {
        GdtEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0b1111_0010,
            granularity: 0b0010_0000,
            base_high: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct TssEntry {
    limit_low: u16,
    base_low: u16,
    base_middle_low: u8,
    access: u8,
    granularity: u8,
    base_middle_high: u8,
    base_high: u32,
    reserved: u32,
}

impl TssEntry {
    const fn null() -> Self {
        TssEntry {
            limit_low: 0,
            base_low: 0,
            base_middle_low: 0,
            access: 0,
            granularity: 0,
            base_middle_high: 0,
            base_high: 0,
            reserved: 0,
        }
    }
    fn tss_segment(tss: &'static TaskStateSegment) -> Self {
        let base = &raw const *tss as u64;
        let limit = size_of::<TaskStateSegment>() as u32 - 1;

        TssEntry {
            limit_low: limit as u16,
            base_low: base as u16,
            base_middle_low: (base >> 16) as u8,
            access: 0b1000_1001,
            granularity: 0b0000_0000,
            base_middle_high: (base >> 24) as u8,
            base_high: (base >> 32) as u32,
            reserved: 0,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct GlobalDescriptorTable {
    null: GdtEntry,
    kernel_code: GdtEntry,
    kernel_data: GdtEntry,
    user_code_32bit: GdtEntry,
    user_code: GdtEntry,
    user_data: GdtEntry,
    tss: TssEntry,
}

impl Default for GlobalDescriptorTable {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalDescriptorTable {
    pub const fn new() -> Self {
        GlobalDescriptorTable {
            null: GdtEntry::null(),
            kernel_code: GdtEntry::kernel_code(),
            kernel_data: GdtEntry::kernel_data(),
            user_code_32bit: GdtEntry::user_code_32bit(),
            user_code: GdtEntry::user_code(),
            user_data: GdtEntry::user_data(),
            tss: TssEntry::null(),
        }
    }
}

#[derive(Debug, Default)]
#[repr(C, packed)]
pub struct TaskStateSegment {
    reserved0: u32,
    rsp0: u64,
    rsp1: u64,
    rsp2: u64,
    reserved1: u64,
    ist1: u64,
    ist2: u64,
    ist3: u64,
    ist4: u64,
    ist5: u64,
    ist6: u64,
    ist7: u64,
    reserved2: u64,
    reserved3: u16,
    io_map_base_address: u16,
}

lazy_static::lazy_static! {
    static ref TSS: TaskStateSegment = TaskStateSegment {
        rsp0: unsafe {
            alloc::alloc::alloc(Layout::from_size_align(KERNEL_STACK_SIZE, 0x8).unwrap()) as u64
                + crate::memory::KERNEL_STACK_SIZE as u64
        },
        ..Default::default()
    };

    static ref GDT: GlobalDescriptorTable = {
        let mut gdt = GlobalDescriptorTable::new();
        gdt.tss = TssEntry::tss_segment(&TSS);
        gdt
    };

    static ref GDT_PTR: GdtPtr = GdtPtr {
        limit: (size_of::<GlobalDescriptorTable>() - 1) as u16,
        base: &raw const *GDT as u64,
    };

    static ref SELECTORS: Selectors = {
        Selectors {
            kernel_code: SegmentSelector::new(1, 0, 0),
            kernel_data: SegmentSelector::new(2, 0, 0),
            _user_code_32bit: SegmentSelector::new(3, 0, 3),
            _user_code: SegmentSelector::new(4, 0, 3),
            _user_data: SegmentSelector::new(5, 0, 3),
            tss: SegmentSelector::new(6, 0, 0),
        }
    };
}

struct Selectors {
    kernel_code: SegmentSelector,
    kernel_data: SegmentSelector,
    _user_code_32bit: SegmentSelector,
    _user_code: SegmentSelector,
    _user_data: SegmentSelector,
    tss: SegmentSelector,
}

pub fn init() {
    unsafe {
        info!("loading gdt");
        asm!(
            "mov rdi, {ptr}",
            "lgdt [rdi]",
            "mov rsi, {data:r}",
            "mov ss, si",
            "mov si, 0",
            "mov ds, si",
            "mov fs, si",
            "mov es, si",
            "mov gs, si",
            "mov rdx, {code:r}",
            "push rdx",
            "lea rax, [rip + 55f]",
            "push rax",
            "retfq",
            "55:",
            ptr = in(reg) &raw const *GDT_PTR,
            data = in(reg) SELECTORS.kernel_data.0,
            code = in(reg) SELECTORS.kernel_code.0,
            options(nostack)
        );
        info!("loading tss");
        asm!(
            "ltr {tss:x}",
            tss = in(reg) SELECTORS.tss.0,
            options(nostack)
        )
    }
}
