/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    alloc::Layout,
    cell::OnceCell,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    memory::{KERNEL_STACK_SIZE, USER_STACK_SIZE},
    utils::{asm::without_ints, spinlock::Spin},
};
use alloc::{collections::vec_deque::VecDeque, sync::Arc, vec::Vec};

use crate::{
    arch::{drivers::time::preferred_timer_ns, interrupts::StackFrame, system::lapic},
    drivers::fs,
    memory::vmm::{PAGEMAP, Pagemap},
    utils::asm::halt_loop,
};

use super::thread::*;

pub static mut SCHEDULER: OnceCell<Scheduler> = OnceCell::new();

fn next_pid() -> u64 {
    get_scheduler().next_pid.fetch_add(1, Ordering::Relaxed)
}

pub struct Scheduler {
    pub processes: Vec<Arc<Spin<Process>>>,
    pub current: Option<Arc<Spin<Thread>>>,
    pub queue: VecDeque<Arc<Spin<Thread>>>,
    timeslice: usize,
    next_pid: AtomicU64,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
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
    next_stack_addr: u64,
    cwd: fs::Path,
    pub pagemap: &'static Arc<Spin<Pagemap>>,
    children: Vec<Arc<Spin<Thread>>>,
}

impl Process {
    pub fn new(pagemap: &'static Arc<Spin<Pagemap>>, name: &'static str) -> Self {
        let pid = next_pid();
        Self {
            name,
            pid,
            next_tid: AtomicU64::new(1),
            next_stack_addr: 0x0000_7FFF_FF00_0000,
            cwd: fs::Path::new("/"),
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

    pub fn get_children(&self) -> &[Arc<Spin<Thread>>] {
        &self.children
    }

    pub fn get_children_mut(&mut self) -> &mut Vec<Arc<Spin<Thread>>> {
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
        assert!(
            self.next_stack_addr >= USER_STACK_SIZE as u64,
            "user stack address space exhausted"
        );
        self.next_stack_addr -= USER_STACK_SIZE as u64;
        self.next_stack_addr
    }
}

pub fn schedule(regs: &mut StackFrame) {
    let time = preferred_timer_ns();
    let scheduler = get_scheduler();

    if let Some(ref ct) = scheduler.current {
        let mut t = ct.lock();
        t.regs = *regs;
        if t.get_status() == Status::Running {
            t.set_status(Status::Ready);
        }
        t.runtime += time - t.schedule_time;
    }

    let mut next = None;
    let queue_len = scheduler.queue.len();

    for _ in 0..queue_len {
        if let Some(thread) = scheduler.queue.pop_front() {
            let mut t = thread.lock();
            match t.get_status() {
                Status::Ready => {
                    drop(t);
                    next = Some(thread);
                    break;
                }
                Status::Sleeping(when) => {
                    if preferred_timer_ns() >= when {
                        t.set_status(Status::Ready);
                        drop(t);
                        next = Some(thread);
                        break;
                    } else {
                        drop(t);
                        scheduler.queue.push_back(thread);
                    }
                }
                Status::Terminated => {
                    unsafe {
                        alloc::alloc::dealloc(
                            t.kstack_alloc as _,
                            Layout::from_size_align(KERNEL_STACK_SIZE, 16).unwrap(),
                        );
                        if t.ustack_alloc != 0 {
                            alloc::alloc::dealloc(
                                t.ustack_alloc as _,
                                Layout::from_size_align(USER_STACK_SIZE, 16).unwrap(),
                            );
                        }
                    }
                    // TODO: unmap user pages and remove from parent children
                }
                Status::Blocked => {
                    // wake() will re-add when unblocked
                }
                Status::Running => {
                    drop(t);
                    scheduler.queue.push_back(thread);
                }
            }
        }
    }

    if next.is_none() {
        next = Some(idle0().clone());
    }

    if let Some(ct) = scheduler.current.take() {
        let status = ct.lock().get_status();
        if status != Status::Terminated && status != Status::Blocked {
            scheduler.queue.push_back(ct);
        }
    }

    let thread = next.unwrap();
    {
        let mut t = thread.lock();
        t.set_status(Status::Running);
        *regs = t.regs;
        t.schedule_time = preferred_timer_ns();
    }

    scheduler.current = Some(thread);
    lapic::arm(scheduler.timeslice, 0xFE);
}

#[inline(always)]
pub fn enqueue(thread: Arc<Spin<Thread>>) {
    without_ints(|| {
        get_scheduler().queue.push_back(thread);
    });
}

pub fn kill_process(pid: u64) -> bool {
    if pid == 0 {
        crate::drivers::acpi::shutdown();
        return false;
    }
    without_ints(|| {
        let scheduler = get_scheduler();
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
    })
}

pub fn spawn_process(pagemap: &'static Arc<Spin<Pagemap>>, name: &'static str) -> u64 {
    without_ints(|| {
        let scheduler = get_scheduler();
        let proc = Arc::new(Spin::new(Process::new(pagemap, name)));
        let pid = proc.lock().get_pid();
        scheduler.processes.push(proc);
        pid
    })
}

pub fn init() {
    unsafe { SCHEDULER.set(Scheduler::default()).ok() };
    get_scheduler()
        .processes
        .push(Arc::new(Spin::new(Process::new(
            unsafe { PAGEMAP.get().unwrap() },
            "kernel",
        ))));
}

pub fn start() -> ! {
    yield_();
    halt_loop()
}

pub fn is_initialized() -> bool {
    unsafe { SCHEDULER.get().is_some() }
}

pub fn get_scheduler() -> &'static mut Scheduler {
    unsafe { SCHEDULER.get_mut().unwrap() }
}

pub fn get_scheduler_safe() -> Option<&'static mut Scheduler> {
    unsafe { SCHEDULER.get_mut() }
}

pub fn get_proc_by_pid(pid: u64) -> Option<Arc<Spin<Process>>> {
    get_scheduler_safe().and_then(|x| x.processes.iter().find(|p| p.lock().pid == pid).cloned())
}

pub fn get_proc_by_name(name: &str) -> Option<Arc<Spin<Process>>> {
    get_scheduler_safe().and_then(|x| x.processes.iter().find(|p| p.lock().name == name).cloned())
}

pub fn current_process() -> Option<Arc<Spin<Process>>> {
    get_scheduler_safe().and_then(|x| {
        x.current
            .as_ref()
            .and_then(|x| x.lock().get_parent().upgrade())
    })
}
