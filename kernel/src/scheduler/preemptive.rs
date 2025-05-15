/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    cell::OnceCell,
    ffi::c_void,
    mem::size_of,
    sync::atomic::{AtomicU64, Ordering},
};

use alloc::{collections::vec_deque::VecDeque, sync::Arc, vec::Vec};
use spin::Mutex;

use crate::{
    arch::{drivers::time::preferred_timer_ns, interrupts::StackFrame, system::lapic},
    memory::vmm::{PAGEMAP, Pagemap},
    utils::{
        asm::{halt_loop, mem::memcpy},
        wait_for_spinlock_arc,
    },
};

use super::thread::*;

pub static mut PID0: OnceCell<Arc<Mutex<Process>>> = OnceCell::new();
pub static mut PROCESSES: Vec<Arc<Mutex<Process>>> = Vec::new();

static NEXT_PID: AtomicU64 = AtomicU64::new(0);
static mut QUEUE: VecDeque<Arc<Mutex<Thread>>> = VecDeque::new();

const TIMESLICE: usize = 6;

fn next_pid() -> u64 {
    NEXT_PID.fetch_add(1, Ordering::Relaxed)
}

pub struct Process {
    name: &'static str,
    pid: u64,
    next_tid: AtomicU64,
    _pagemap: &'static Arc<Mutex<Pagemap>>,
    children: Vec<Arc<Mutex<Thread>>>,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

impl Process {
    pub fn new(_pagemap: &'static Arc<Mutex<Pagemap>>, name: &'static str) -> Self {
        let pid = next_pid();
        // debug!("spawning new process {}", _pid);
        Self {
            name,
            pid,
            next_tid: AtomicU64::new(1),
            _pagemap,
            children: Vec::new(),
        }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_pid(&self) -> u64 {
        self.pid
    }

    pub fn get_children(&self) -> &Vec<Arc<Mutex<Thread>>> {
        &self.children
    }

    pub fn get_children_mut(&mut self) -> &mut Vec<Arc<Mutex<Thread>>> {
        &mut self.children
    }

    pub fn next_tid(&mut self) -> u64 {
        self.next_tid.fetch_add(1, Ordering::Relaxed)
    }
}

pub fn schedule(regs: *mut StackFrame) {
    let mut next = None;

    if let Some(ct) = get_self() {
        wait_for_spinlock_arc(ct);
        let mut t = ct.lock();
        memcpy(
            &raw mut t.regs as *mut c_void,
            regs as *mut c_void,
            size_of::<StackFrame>(),
        );
        if t.get_status() == &Status::Running {
            t.set_status(Status::Ready);
        }
    }

    for _ in 0..unsafe { QUEUE.len() } {
        if let Some(thread) = unsafe { QUEUE.pop_front() } {
            wait_for_spinlock_arc(&thread);
            let mut t = thread.lock();
            match t.get_status() {
                Status::Ready => {
                    next = Some(thread.clone());
                    break;
                }
                Status::Sleeping(when) => {
                    if &preferred_timer_ns() >= when {
                        t.set_status(Status::Ready);
                    }
                    enqueue(thread.clone());
                }
                Status::Blocked | Status::Terminated => {
                    // ignored
                }
                _ => enqueue(thread.clone()),
            }
        }
    }

    if next.is_none() {
        next = Some(idle0().clone());
    }

    if let Some(n) = next {
        if let Some(ct) = get_self() {
            enqueue(ct.clone());
        }

        unsafe { CURRENT_THREAD = Some(n.clone()) };

        wait_for_spinlock_arc(&n);
        let mut t = n.lock();
        t.set_status(Status::Running);
        memcpy(
            regs as *mut c_void,
            &raw const t.regs as *const c_void,
            size_of::<StackFrame>(),
        );
    }

    lapic::arm(TIMESLICE * 1_000_000, 0xFF);
}

#[inline(always)]
pub fn enqueue(thread: Arc<Mutex<Thread>>) {
    unsafe { QUEUE.push_back(thread) };
}

pub fn kill_process(pid: u64) -> bool {
    unsafe {
        if pid == 0 {
            crate::drivers::acpi::shutdown();
        }
        if let Some(pos) = PROCESSES.iter().position(|p| p.lock().pid == pid) {
            let proc = PROCESSES.get(pos).unwrap();

            wait_for_spinlock_arc(proc);

            for thread in proc.lock().children.iter() {
                wait_for_spinlock_arc(thread);
                thread.lock().set_status(Status::Terminated);
            }

            PROCESSES.remove(pos);

            true
        } else {
            false
        }
    }
}

pub fn spawn_process(pagemap: &'static Arc<Mutex<Pagemap>>, name: &'static str) -> u64 {
    unsafe {
        let proc = Arc::new(Mutex::new(Process::new(pagemap, name)));
        PROCESSES.push(proc.clone());
        proc.lock().get_pid()
    }
}

pub fn init_pid0() {
    unsafe {
        PID0.set(Arc::new(Mutex::new(Process::new(
            PAGEMAP.get().unwrap(),
            "kernel",
        ))))
        .ok();
        PROCESSES.push(PID0.get().unwrap().clone());
    }
}

pub fn init() -> ! {
    yld();
    halt_loop();
}
