/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::arch::asm;

use crate::{debug, info};

#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct SegmentSelector(pub u16);

impl SegmentSelector {
    pub const fn new(index: u16, table_indicator: u8) -> Self {
        SegmentSelector((index << 3) | ((table_indicator as u16) << 2))
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
    const fn missing() -> Self {
        GdtEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: 0,
            granularity: 0,
            base_high: 0,
        }
    }

    const fn kernel_code_segment() -> Self {
        const ACCESS_KERNEL_CODE: u8 = 0b1001_1010;
        const GRANULARITY_4KB: u8 = 0b0010_0000;

        GdtEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: ACCESS_KERNEL_CODE,
            granularity: GRANULARITY_4KB,
            base_high: 0,
        }
    }

    const fn kernel_data_segment() -> Self {
        const ACCESS_KERNEL_DATA: u8 = 0b1001_0010;
        const GRANULARITY_4KB: u8 = 0b0000_0000;

        GdtEntry {
            limit_low: 0,
            base_low: 0,
            base_middle: 0,
            access: ACCESS_KERNEL_DATA,
            granularity: GRANULARITY_4KB,
            base_high: 0,
        }
    }

    fn tss_segment(tss: &'static TaskStateSegment) -> Self {
        let base = &raw const *tss as u64;
        let limit = size_of::<TaskStateSegment>() as u32 - 1;
        const ACCESS_TSS: u8 = 0b1000_1001;
        const GRANULARITY_TSS: u8 = 0b0000_0000;

        GdtEntry {
            limit_low: limit as u16,
            base_low: base as u16,
            base_middle: (base >> 16) as u8,
            access: ACCESS_TSS,
            granularity: GRANULARITY_TSS,
            base_high: (base >> 24) as u8,
        }
    }
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
pub struct GlobalDescriptorTable {
    null: GdtEntry,
    kernel_code: GdtEntry,
    kernel_data: GdtEntry,
    tss_low: GdtEntry,
    tss_high: GdtEntry,
}

impl Default for GlobalDescriptorTable {
    fn default() -> Self {
        Self::new()
    }
}

impl GlobalDescriptorTable {
    pub const fn new() -> Self {
        GlobalDescriptorTable {
            null: GdtEntry::missing(),
            kernel_code: GdtEntry::kernel_code_segment(),
            kernel_data: GdtEntry::kernel_data_segment(),
            tss_low: GdtEntry::missing(),
            tss_high: GdtEntry::missing(),
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
    interrupt_stack_table: [u64; 7],
}

lazy_static::lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::default();
        tss.interrupt_stack_table[0] = {
            let stack_start = &raw const crate::memory::STACK as u64;
            stack_start + crate::memory::STACK_SIZE as u64
        };
        tss
    };

    static ref GDT: GlobalDescriptorTable = {
        let mut gdt = GlobalDescriptorTable::new();
        gdt.tss_low = GdtEntry::tss_segment(&TSS);
        gdt
    };

    static ref GDT_PTR: GdtPtr = GdtPtr {
        limit: (size_of::<GlobalDescriptorTable>() - 1) as u16,
        base: &raw const *GDT as u64,
    };

    static ref SELECTORS: Selectors = {
        Selectors {
            code_selector: SegmentSelector::new(1, 0),
            data_selector: SegmentSelector::new(2, 0),
            tss_selector: SegmentSelector::new(3, 0),
        }
    };
}

struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

pub fn init() {
    unsafe {
        info!("loading gdt");
        asm!("lgdt [{}]", in(reg) &raw const *GDT_PTR, options(readonly, nostack, preserves_flags));

        debug!("loading code segment");
        asm!(
            "push {sel:r}",
            "lea {tmp}, [55f + rip]",
            "push {tmp}",
            "retfq",
            "55:",
            sel = in(reg) SELECTORS.code_selector.0,
            tmp = lateout(reg) _,
            options(preserves_flags),
        );

        debug!("loading tss");
        asm!("ltr {0:x}", in(reg) SELECTORS.tss_selector.0, options(nostack, preserves_flags));

        debug!("loading data segments");
        let data = SELECTORS.data_selector.0;
        asm!(
            "mov ds, {0:x}",
            "mov es, {0:x}",
            "mov fs, {0:x}",
            "mov gs, {0:x}",
            "mov ss, {0:x}",
            in(reg) data,
            options(nostack, preserves_flags)
        );
    }
}
