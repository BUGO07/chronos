/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    alloc::Layout,
    cell::OnceCell,
    ffi::c_void,
    mem::size_of,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    memory::STACK_SIZE,
    utils::{limine::get_hhdm_offset, spinlock::SpinLock},
};
use alloc::{
    collections::{btree_set::BTreeSet, vec_deque::VecDeque},
    sync::Arc,
    vec::Vec,
};

use crate::{
    arch::{drivers::time::preferred_timer_ns, interrupts::StackFrame, system::lapic},
    drivers::fs,
    memory::vmm::{PAGEMAP, Pagemap},
    utils::asm::{halt_loop, mem::memcpy},
};

use super::thread::*;

pub static mut SCHEDULER: OnceCell<Scheduler> = OnceCell::new();

fn next_pid() -> u64 {
    get_scheduler().next_pid.fetch_add(1, Ordering::Relaxed)
}

pub struct Scheduler {
    pub processes: Vec<Arc<SpinLock<Process>>>,
    pub current: Option<Arc<SpinLock<Thread>>>,
    pub queue: BTreeSet<Arc<SpinLock<Thread>>>,
    pub secondary_queue: VecDeque<Arc<SpinLock<Thread>>>,
    timeslice: usize,
    next_pid: AtomicU64,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            processes: Vec::new(),
            current: None,
            queue: BTreeSet::new(),
            secondary_queue: VecDeque::new(),
            timeslice: 6_000_000,
            next_pid: AtomicU64::new(0),
        }
    }
}

pub struct Process {
    name: &'static str,
    pid: u64,
    next_tid: AtomicU64,
    next_stack_addr: u64,
    cwd: fs::Path,
    pub pagemap: &'static Arc<SpinLock<Pagemap>>,
    children: Vec<Arc<SpinLock<Thread>>>,
}

impl Process {
    pub fn new(pagemap: &'static Arc<SpinLock<Pagemap>>, name: &'static str) -> Self {
        let pid = next_pid();
        // debug!("spawning new process {}", _pid);
        Self {
            name,
            pid,
            next_tid: AtomicU64::new(1),
            next_stack_addr: 0x0000_7FFF_FF00_0000,
            cwd: fs::Path::new("/").join("home"),
            pagemap,
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

    pub fn next_stack_address(&mut self) -> u64 {
        self.next_stack_addr -= STACK_SIZE as u64;
        self.next_stack_addr
    }
}

pub fn schedule(regs: *mut StackFrame) {
    let mut next = None;
    let time = preferred_timer_ns();

    if let Some(ct) = current_thread() {
        let mut t = ct.lock();
        memcpy(
            &raw mut t.regs as *mut c_void,
            regs as *mut c_void,
            size_of::<StackFrame>(),
        );
        if t.get_status() == &Status::Running {
            t.set_status(Status::Ready);
        }
        t.runtime += time - t.schedule_time;
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
                        next = Some(thread.clone());
                        break;
                    } else {
                        scheduler.secondary_queue.push_back(thread.clone());
                    }
                }
                Status::Terminated => {
                    unsafe {
                        alloc::alloc::dealloc(
                            t.kstack as *mut u8,
                            Layout::from_size_align(STACK_SIZE, 0x8).unwrap(),
                        );
                        alloc::alloc::dealloc(
                            (t.ustack + get_hhdm_offset()) as *mut u8,
                            Layout::from_size_align(STACK_SIZE, 0x8).unwrap(),
                        );
                    };
                }
                Status::Blocked => {
                    // ignored
                }
                _ => enqueue(thread.clone()),
            }
        }
    }

    for thread in scheduler.secondary_queue.drain(..) {
        match thread.lock().get_status() {
            Status::Blocked => {
                get_scheduler().secondary_queue.push_back(thread.clone());
            }
            _ => enqueue(thread.clone()),
        }
    }

    if next.is_none() {
        next = Some(idle0().clone());
    }

    if let Some(ct) = current_thread() {
        enqueue(ct.clone());
    }

    scheduler.current = next.clone();

    let thread = next.unwrap();
    let mut t = thread.lock();

    t.set_status(Status::Running);

    memcpy(
        regs as *mut c_void,
        &raw const t.regs as *const c_void,
        size_of::<StackFrame>(),
    );

    lapic::arm(scheduler.timeslice, 0xFF);
    t.schedule_time = preferred_timer_ns();
}

#[inline(always)]
pub fn enqueue(thread: Arc<SpinLock<Thread>>) {
    get_scheduler().queue.insert(thread);
}

#[inline(always)]
pub fn next_thread() -> Option<Arc<SpinLock<Thread>>> {
    get_scheduler().queue.pop_first()
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
    get_scheduler()
        .processes
        .push(Arc::new(SpinLock::new(Process::new(
            unsafe { PAGEMAP.get().unwrap() },
            "kernel",
        ))));
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

pub fn get_proc_by_pid(pid: u64) -> Option<&'static Arc<SpinLock<Process>>> {
    get_scheduler()
        .processes
        .iter()
        .find(|p| p.lock().pid == pid)
}

pub fn get_proc_by_name(name: &str) -> Option<&'static Arc<SpinLock<Process>>> {
    get_scheduler()
        .processes
        .iter()
        .find(|p| p.lock().name == name)
}

pub fn current_process() -> Option<Arc<SpinLock<Process>>> {
    get_scheduler()
        .current
        .as_ref()
        .map(|x| x.lock().get_parent().upgrade().unwrap())
}
