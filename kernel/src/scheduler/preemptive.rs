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

use crate::utils::spinlock::SpinLock;
use alloc::{collections::vec_deque::VecDeque, sync::Arc, vec::Vec};

use crate::{
    arch::{drivers::time::preferred_timer_ns, interrupts::StackFrame, system::lapic},
    drivers::fs,
    memory::vmm::{PAGEMAP, Pagemap},
    utils::asm::{halt_loop, mem::memcpy},
};

use super::thread::*;

pub static mut SCHEDULER: OnceCell<Scheduler> = OnceCell::new();

fn next_pid() -> u64 {
    unsafe {
        SCHEDULER
            .get_mut()
            .unwrap()
            .next_pid
            .fetch_add(1, Ordering::Relaxed)
    }
}

pub struct Scheduler {
    pub pid0: OnceCell<Arc<SpinLock<Process>>>,
    pub processes: Vec<Arc<SpinLock<Process>>>,
    pub current: Option<Arc<SpinLock<Thread>>>,
    queue: VecDeque<Arc<SpinLock<Thread>>>,
    timeslice: usize,
    next_pid: AtomicU64,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            pid0: OnceCell::new(),
            processes: Vec::new(),
            current: None,
            queue: VecDeque::new(),
            timeslice: 6_000_000,
            next_pid: AtomicU64::new(0),
        }
    }
}

pub struct Process {
    name: &'static str,
    pid: u64,
    next_tid: AtomicU64,
    cwd: fs::Path,
    _pagemap: &'static Arc<SpinLock<Pagemap>>,
    children: Vec<Arc<SpinLock<Thread>>>,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

impl Process {
    pub fn new(_pagemap: &'static Arc<SpinLock<Pagemap>>, name: &'static str) -> Self {
        let pid = next_pid();
        // debug!("spawning new process {}", _pid);
        Self {
            name,
            pid,
            next_tid: AtomicU64::new(1),
            cwd: fs::Path::new("/").join("home"),
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

    pub fn get_children(&self) -> &Vec<Arc<SpinLock<Thread>>> {
        &self.children
    }

    pub fn get_children_mut(&mut self) -> &mut Vec<Arc<SpinLock<Thread>>> {
        &mut self.children
    }

    pub fn get_cwd(&self) -> &fs::Path {
        &self.cwd
    }

    pub fn set_cwd(&mut self, path: fs::Path) {
        self.cwd = path;
    }

    pub fn next_tid(&mut self) -> u64 {
        self.next_tid.fetch_add(1, Ordering::Relaxed)
    }
}

pub fn schedule(regs: *mut StackFrame) {
    let mut next = None;

    if let Some(ct) = get_self() {
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

    let scheduler = get_scheduler();

    for _ in 0..scheduler.queue.len() {
        if let Some(thread) = next_thread() {
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

    if let Some(ct) = get_self() {
        enqueue(ct.clone());
    }

    scheduler.current = next.clone();

    let thread = next.unwrap();

    thread.lock().set_status(Status::Running);

    memcpy(
        regs as *mut c_void,
        &raw const thread.lock().regs as *const c_void,
        size_of::<StackFrame>(),
    );

    lapic::arm(scheduler.timeslice, 0xFF);
}

#[inline(always)]
pub fn enqueue(thread: Arc<SpinLock<Thread>>) {
    get_scheduler().queue.push_back(thread);
}

#[inline(always)]
pub fn next_thread() -> Option<Arc<SpinLock<Thread>>> {
    get_scheduler().queue.pop_front()
}

pub fn kill_process(pid: u64) -> bool {
    let scheduler = get_scheduler();
    if pid == 0 {
        crate::drivers::acpi::shutdown();
    }
    if let Some(pos) = scheduler.processes.iter().position(|p| p.lock().pid == pid) {
        let proc = scheduler.processes.get(pos).unwrap();

        for thread in proc.lock().children.iter() {
            thread.lock().set_status(Status::Terminated);
        }

        scheduler.processes.remove(pos);

        true
    } else {
        false
    }
}

pub fn spawn_process(pagemap: &'static Arc<SpinLock<Pagemap>>, name: &'static str) -> u64 {
    let scheduler = get_scheduler();
    let proc = Arc::new(SpinLock::new(Process::new(pagemap, name)));
    scheduler.processes.push(proc.clone());
    proc.lock().get_pid()
}

pub fn init() {
    unsafe { SCHEDULER.set(Scheduler::default()).ok() };
    let scheduler = get_scheduler();
    scheduler
        .pid0
        .set(Arc::new(SpinLock::new(Process::new(
            unsafe { PAGEMAP.get().unwrap() },
            "kernel",
        ))))
        .ok();
    scheduler
        .processes
        .push(scheduler.pid0.get().unwrap().clone());
}

pub fn start() -> ! {
    yld();
    halt_loop()
}

pub fn is_initialized() -> bool {
    unsafe { SCHEDULER.get().is_some() }
}

pub fn get_scheduler() -> &'static mut Scheduler {
    unsafe { SCHEDULER.get_mut().unwrap() }
}
