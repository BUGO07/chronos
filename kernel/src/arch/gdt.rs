/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use x86_64::{
    VirtAddr,
    instructions::tables::load_tss,
    registers::segmentation::{CS, DS, ES, FS, GS, SS, Segment},
    structures::{
        gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector},
        tss::TaskStateSegment,
    },
};

use crate::{debug, info};

pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;
pub const STACK_SIZE: usize = 64 * 1024;
pub static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

lazy_static::lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let data_selector = gdt.append(Descriptor::kernel_data_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));
        (
            gdt,
            Selectors {
                code_selector,
                data_selector,
                tss_selector,
            },
        )
    };

    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {

            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            stack_start + STACK_SIZE as u64 // stack_end
        };
        tss
    };
}

pub fn init() {
    info!("loading gdt");
    GDT.0.load();
    unsafe {
        debug!("loading code segment");
        CS::set_reg(GDT.1.code_selector);

        debug!("loading tss");
        load_tss(GDT.1.tss_selector);

        debug!("loading data segments");
        SS::set_reg(GDT.1.data_selector);
        FS::set_reg(GDT.1.data_selector);
        GS::set_reg(GDT.1.data_selector);
        // unused in 64 bit but just in case i decide to go to 32
        DS::set_reg(GDT.1.data_selector);
        ES::set_reg(GDT.1.data_selector);
    }
}
