/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    alloc::Layout,
    arch::asm,
    cell::OnceCell,
    fmt::Debug,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    memory::{
        USER_STACK_SIZE,
        vmm::{flag, page_size},
    },
    utils::{asm::without_ints, limine::get_hhdm_offset, spinlock::Spin},
};
use alloc::sync::{Arc, Weak};

use crate::{
    arch::{drivers::time::preferred_timer_ns, interrupts::StackFrame},
    memory::KERNEL_STACK_SIZE,
    utils::asm::halt_loop,
};

use super::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Status {
    Ready,
    Running,
    Sleeping(u64), // ns
    Blocked,
    Terminated,
}

static NEXT_GLOBAL_TID: AtomicU64 = AtomicU64::new(1);

pub struct Thread {
    name: &'static str,
    pub tid: u64,
    pub gtid: u64,
    pub kstack: u64,
    pub ustack: u64,
    pub kstack_alloc: u64,
    pub ustack_alloc: u64,
    pub regs: StackFrame,
    parent: Weak<Spin<Process>>,
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
    fn eq(&self, other: &Self) -> bool {
        self.gtid == other.gtid
    }
}

impl Eq for Thread {}

unsafe impl Sync for Thread {}
unsafe impl Send for Thread {}

impl Thread {
    pub fn new(proc: &Arc<Spin<Process>>, func: *const (), name: &'static str, user: bool) -> Self {
        let tid = proc.lock().next_tid();
        Self::new_with_tid(proc, func, name, user, tid)
    }
    pub fn new_with_tid(
        proc: &Arc<Spin<Process>>,
        func: *const (),
        name: &'static str,
        user: bool,
        tid: u64,
    ) -> Self {
        let kstack_ptr =
            unsafe { alloc::alloc::alloc(Layout::from_size_align(KERNEL_STACK_SIZE, 16).unwrap()) };
        assert!(!kstack_ptr.is_null(), "failed to allocate kernel stack");
        let kstack_alloc = kstack_ptr as u64;
        let kstack = kstack_alloc;

        let mut ustack: u64 = 0;
        let mut ustack_alloc: u64 = 0;

        if user {
            let alloc_ptr = unsafe {
                alloc::alloc::alloc(Layout::from_size_align(USER_STACK_SIZE, 16).unwrap())
            };
            assert!(!alloc_ptr.is_null(), "failed to allocate user stack");
            ustack_alloc = alloc_ptr as u64;
            let phys = ustack_alloc - get_hhdm_offset();
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
            gtid: NEXT_GLOBAL_TID.fetch_add(1, Ordering::Relaxed),
            kstack,
            ustack,
            kstack_alloc,
            ustack_alloc,
            regs: StackFrame {
                #[cfg(target_arch = "x86_64")]
                cs: if user { 0x28 | 0x03 } else { 0x08 },
                #[cfg(target_arch = "x86_64")]
                ss: if user { 0x20 | 0x03 } else { 0x10 },
                #[cfg(target_arch = "x86_64")]
                rsp: if user {
                    ustack + USER_STACK_SIZE as u64
                } else {
                    kstack + KERNEL_STACK_SIZE as u64
                },
                #[cfg(target_arch = "x86_64")]
                rip: func as u64,
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

    pub fn get_parent(&self) -> &Weak<Spin<Process>> {
        &self.parent
    }

    pub fn get_status(&self) -> Status {
        self.status
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }
}

pub fn spawn(proc: &Arc<Spin<Process>>, func: *const (), name: &'static str, user: bool) -> u64 {
    without_ints(|| {
        let thread = Arc::new(Spin::new(Thread::new(proc, func, name, user)));
        proc.lock().get_children_mut().push(thread.clone());
        let tid = thread.lock().tid;
        get_scheduler().queue.push_back(thread);
        tid
    })
}

pub fn sleep(ns: u64) {
    use crate::utils::asm::{int_status, toggle_ints};
    let was_enabled = int_status();
    toggle_ints(false);
    if let Some(thread) = current_thread() {
        thread
            .lock()
            .set_status(Status::Sleeping(preferred_timer_ns() + ns));
    }
    yield_();
    toggle_ints(was_enabled);
}

#[inline(always)]
pub fn sleep_ms(ms: u64) {
    sleep(ms * 1_000_000);
}

pub fn yield_() {
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!("int $0xFE");
        #[cfg(target_arch = "aarch64")]
        asm!("svc #0");
    }
}

pub fn block() {
    use crate::utils::asm::{int_status, toggle_ints};
    let was_enabled = int_status();
    toggle_ints(false);
    if let Some(thread) = current_thread() {
        thread.lock().set_status(Status::Blocked);
    }
    yield_();
    toggle_ints(was_enabled);
}

pub fn wake(thread: &Arc<Spin<Thread>>) {
    without_ints(|| {
        let mut t = thread.lock();
        if t.get_status() == Status::Blocked {
            t.set_status(Status::Ready);
            drop(t);
            get_scheduler().queue.push_back(thread.clone());
        }
    });
}

pub fn terminate() -> ! {
    crate::utils::asm::toggle_ints(false);
    if let Some(thread) = current_thread() {
        thread.lock().set_status(Status::Terminated);
    }
    yield_();
    halt_loop()
}

pub fn idle0() -> &'static Arc<Spin<Thread>> {
    static mut IDLE: OnceCell<Arc<Spin<Thread>>> = OnceCell::new();
    unsafe {
        IDLE.get_or_init(|| {
            let proc = get_proc_by_pid(0).unwrap();
            let t = Arc::new(Spin::new(Thread::new_with_tid(
                &proc,
                halt_loop as _,
                "idle",
                false,
                0,
            )));
            t.lock().set_status(Status::Ready);
            t
        })
    }
}

pub fn current_thread() -> Option<Arc<Spin<Thread>>> {
    get_scheduler_safe().and_then(|x| x.current.clone())
}
