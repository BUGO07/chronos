/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{alloc::Layout, arch::asm, cell::OnceCell, fmt::Debug};

use crate::{
    memory::{
        USER_STACK_SIZE,
        vmm::{flag, page_size},
    },
    utils::{limine::get_hhdm_offset, spinlock::SpinLock},
};
use alloc::sync::{Arc, Weak};

use crate::{
    arch::{drivers::time::preferred_timer_ns, interrupts::StackFrame},
    memory::KERNEL_STACK_SIZE,
    utils::asm::halt_loop,
};

use super::*;

#[derive(Debug, PartialEq, Eq)]
pub enum Status {
    Ready,
    Running,
    Sleeping(u64), // ns
    Blocked,
    Terminated,
}

pub struct Thread {
    name: &'static str,
    tid: u64,
    pub kstack: u64,
    pub ustack: u64,
    pub regs: StackFrame,
    parent: Weak<SpinLock<Process>>,
    status: Status,
    pub runtime: u64,
    pub schedule_time: u64,
}

impl Debug for Thread {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Thread")
            .field("name", &self.name)
            .field("tid", &self.tid)
            .field("status", &self.status)
            .field("runtime", &self.runtime)
            .finish()
    }
}

impl PartialEq for Thread {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

impl PartialOrd for Thread {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Thread {}

impl Ord for Thread {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        if self.runtime == 0 {
            return core::cmp::Ordering::Greater;
        }
        self.runtime.cmp(&other.runtime)
    }
}

unsafe impl Sync for Thread {}
unsafe impl Send for Thread {}

impl Thread {
    pub fn new(
        proc: &'static Arc<SpinLock<Process>>,
        func: usize,
        name: &'static str,
        user: bool,
    ) -> Self {
        Self::new_with_tid(proc, func, name, user, proc.lock().next_tid())
    }
    pub fn new_with_tid(
        proc: &'static Arc<SpinLock<Process>>,
        func: usize,
        name: &'static str,
        user: bool,
        tid: u64,
    ) -> Self {
        let kstack = unsafe {
            alloc::alloc::alloc(Layout::from_size_align(KERNEL_STACK_SIZE, 0x8).unwrap()) as u64
        };

        let mut ustack: u64 = 0;

        if user {
            let phys = unsafe {
                alloc::alloc::alloc(Layout::from_size_align(USER_STACK_SIZE, 0x8).unwrap()) as u64
                    - get_hhdm_offset()
            };
            unsafe { proc.force_unlock() };
            let mut locked = proc.lock();
            ustack = locked.next_stack_address();
            for i in (0..USER_STACK_SIZE).step_by(page_size::SMALL as usize) {
                locked
                    .pagemap
                    .lock()
                    .map(
                        ustack + i as u64,
                        phys + i as u64,
                        flag::RW | flag::USER,
                        page_size::SMALL,
                    )
                    .unwrap();
            }
        }

        Self {
            name,
            tid,
            kstack,
            ustack,
            regs: StackFrame {
                #[cfg(target_arch = "x86_64")]
                cs: if user { 0x20 | 0x03 } else { 0x08 },
                #[cfg(target_arch = "x86_64")]
                ss: if user { 0x28 | 0x03 } else { 0x10 },
                #[cfg(target_arch = "x86_64")]
                rsp: if user {
                    ustack + USER_STACK_SIZE as u64
                } else {
                    kstack + KERNEL_STACK_SIZE as u64
                },
                #[cfg(target_arch = "x86_64")]
                rip: func as u64,
                // #[cfg(target_arch = "x86_64")]
                // cs: crate::utils::asm::regs::get_cs_reg() as u64,
                // #[cfg(target_arch = "x86_64")]
                // ss: crate::utils::asm::regs::get_ss_reg() as u64,
                #[cfg(target_arch = "x86_64")]
                rflags: 0x202,

                // TODO: implement aarch64 properly
                #[cfg(target_arch = "aarch64")]
                sp: kstack + KERNEL_STACK_SIZE as u64,
                #[cfg(target_arch = "aarch64")]
                pc: func,
                #[cfg(target_arch = "aarch64")]
                pstate: 0,
                ..Default::default()
            },
            parent: Arc::downgrade(proc),
            status: Status::Ready,
            runtime: 0,
            schedule_time: 0,
        }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_tid(&self) -> u64 {
        self.tid
    }

    pub fn get_parent(&self) -> &Weak<SpinLock<Process>> {
        &self.parent
    }

    pub fn get_status(&self) -> &Status {
        &self.status
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }
}

pub fn spawn(
    proc: &'static Arc<SpinLock<Process>>,
    func: usize,
    name: &'static str,
    user: bool,
) -> u64 {
    let thread = Arc::new(SpinLock::new(Thread::new(proc, func, name, user)));
    proc.lock().get_children_mut().push(thread.clone());
    let tid = thread.lock().get_tid();
    enqueue(thread);
    tid
}

pub fn sleep(ns: u64) {
    if let Some(thread) = current_thread() {
        let mut t = thread.lock();
        t.set_status(Status::Sleeping(preferred_timer_ns() + ns));
    }
    yld();
}

#[inline(always)]
pub fn sleep_ms(ms: u64) {
    sleep(ms * 1_000_000);
}

pub fn yld() {
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!("int $0xFF");
        #[cfg(target_arch = "aarch64")]
        asm!("svc #0");
    }
}

pub fn block() {
    if let Some(thread) = current_thread() {
        thread.lock().set_status(Status::Blocked);
        yld();
    }
}

pub fn wake(thread: &Arc<SpinLock<Thread>>) {
    let mut t: crate::utils::spinlock::SpinLockGuard<'_, Thread> = thread.lock();
    if t.get_status() == &Status::Blocked {
        t.set_status(Status::Ready);
        enqueue(thread.clone());
    }
}

pub fn terminate() -> ! {
    if let Some(thread) = current_thread() {
        thread.lock().set_status(Status::Terminated);
        yld();
    }
    halt_loop()
}

pub fn idle0() -> &'static Arc<SpinLock<Thread>> {
    static mut IDLE: OnceCell<Arc<SpinLock<Thread>>> = OnceCell::new();
    unsafe {
        IDLE.get_or_init(|| {
            let t = Arc::new(SpinLock::new(Thread::new_with_tid(
                get_proc_by_pid(0).unwrap(),
                halt_loop as usize,
                "idle",
                false,
                99,
            )));
            t.lock().set_status(Status::Ready);
            t
        })
    }
}

pub fn current_thread() -> &'static mut Option<Arc<SpinLock<Thread>>> {
    &mut get_scheduler().current
}
