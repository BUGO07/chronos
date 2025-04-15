/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

// ! idk if this works, its temporary

use core::arch::asm;

#[derive(Debug)]
pub struct Registers {
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rbp: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

pub fn read_registers() -> Registers {
    let (
        r15,
        r14,
        r13,
        r12,
        r11,
        r10,
        r9,
        r8,
        rbp,
        rdi,
        rsi,
        rdx,
        rcx,
        rbx,
        rax,
        rsp,
        cs,
        rflags,
        ss,
        rip,
    );

    unsafe {
        asm!(
            "mov {}, r15", "mov {}, r14", "mov {}, r13", "mov {}, r12", "mov {}, r11",
            out(reg) r15, out(reg) r14, out(reg) r13, out(reg) r12, out(reg) r11,
        );
        asm!(
            "mov {}, r10", "mov {}, r9", "mov {}, r8", "mov {}, rbp", "mov {}, rdi",
            out(reg) r10, out(reg) r9, out(reg) r8, out(reg) rbp, out(reg) rdi,
        );
        asm!(
            "mov {}, rsi", "mov {}, rdx", "mov {}, rcx", "mov {}, rbx", "mov {}, rax",
            out(reg) rsi, out(reg) rdx, out(reg) rcx, out(reg) rbx, out(reg) rax,
        );
        asm!(
            "mov {}, rsp", "mov {}, cs", "pushfq; pop {}", "mov {}, ss", "lea {}, [rip]",
            out(reg) rsp, out(reg) cs, out(reg) rflags, out(reg) ss, out(reg) rip,
        );
    }

    Registers {
        r15,
        r14,
        r13,
        r12,
        r11,
        r10,
        r9,
        r8,
        rbp,
        rdi,
        rsi,
        rdx,
        rcx,
        rbx,
        rax,
        rip,
        cs,
        rflags,
        rsp,
        ss,
    }
}
