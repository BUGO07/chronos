/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{alloc::Layout, arch::asm, cell::OnceCell};

use alloc::sync::{Arc, Weak};
use spin::Mutex;

use crate::{
    arch::{drivers::time::preferred_timer_ns, interrupts::StackFrame},
    memory::STACK_SIZE,
    scheduler::PID0,
    utils::{asm::halt_loop, wait_for_spinlock_arc},
};

use super::*;

pub static mut CURRENT_THREAD: Option<Arc<Mutex<Thread>>> = None;

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
    _kstack: u64,
    pub regs: StackFrame,
    parent: Weak<Mutex<Process>>,
    status: Status,
}

unsafe impl Sync for Thread {}
unsafe impl Send for Thread {}

impl Thread {
    pub fn new(proc: &'static Arc<Mutex<Process>>, func: usize, name: &'static str) -> Self {
        wait_for_spinlock_arc(proc);
        let tid = proc.lock().next_tid();
        // debug!("spawning new thread {}", _tid);
        let _kstack = unsafe {
            alloc::alloc::alloc(Layout::from_size_align(STACK_SIZE, 0x8).unwrap()) as u64
        };
        Self {
            name,
            tid,
            _kstack,
            regs: StackFrame {
                #[cfg(target_arch = "x86_64")]
                rsp: _kstack + STACK_SIZE as u64,
                #[cfg(target_arch = "x86_64")]
                rip: func as u64,
                #[cfg(target_arch = "x86_64")]
                cs: crate::utils::asm::regs::get_cs_reg() as u64,
                #[cfg(target_arch = "x86_64")]
                ss: crate::utils::asm::regs::get_ss_reg() as u64,
                #[cfg(target_arch = "x86_64")]
                rflags: 0x202,

                // TODO: implement aarch64 properly
                #[cfg(target_arch = "aarch64")]
                sp: _kstack + STACK_SIZE as u64,
                #[cfg(target_arch = "aarch64")]
                pc: func,
                #[cfg(target_arch = "aarch64")]
                pstate: 0,
                ..Default::default()
            },
            parent: Arc::downgrade(proc),
            status: Status::Ready,
        }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_tid(&self) -> u64 {
        self.tid
    }

    pub fn get_parent(&self) -> &Weak<Mutex<Process>> {
        &self.parent
    }

    pub fn get_status(&self) -> &Status {
        &self.status
    }

    pub fn set_status(&mut self, status: Status) {
        self.status = status;
    }
}

pub fn spawn(proc: &'static Arc<Mutex<Process>>, func: usize, name: &'static str) -> u64 {
    let thread = Arc::new(Mutex::new(Thread::new(proc, func, name)));
    wait_for_spinlock_arc(&thread);
    proc.lock().get_children_mut().push(thread.clone());
    let tid = thread.lock().get_tid();
    enqueue(thread);
    tid
}

pub fn sleep(ns: u64) {
    let wakeup_time = preferred_timer_ns() + ns;

    if let Some(thread) = get_self() {
        wait_for_spinlock_arc(thread);
        let mut t = thread.lock();
        t.set_status(Status::Sleeping(wakeup_time));
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
    if let Some(thread) = get_self() {
        wait_for_spinlock_arc(thread);
        thread.lock().set_status(Status::Blocked);
        yld();
    }
}

pub fn wake(thread: &Arc<Mutex<Thread>>) {
    wait_for_spinlock_arc(thread);
    let mut t = thread.lock();
    if t.get_status() == &Status::Blocked {
        t.set_status(Status::Ready);
        enqueue(thread.clone());
    }
}

pub fn terminate() -> ! {
    if let Some(thread) = get_self() {
        wait_for_spinlock_arc(thread);
        thread.lock().set_status(Status::Terminated);
        yld();
    }
    halt_loop();
}

pub fn idle0() -> &'static Arc<Mutex<Thread>> {
    static mut IDLE: OnceCell<Arc<Mutex<Thread>>> = OnceCell::new();
    unsafe {
        IDLE.get_or_init(|| {
            let proc = PID0.get().unwrap();
            let t = Arc::new(Mutex::new(Thread::new(proc, halt_loop as usize, "idle")));
            t.lock().set_status(Status::Ready);
            t
        })
    }
}

pub fn get_self() -> &'static mut Option<Arc<Mutex<Thread>>> {
    unsafe { &mut CURRENT_THREAD }
}
