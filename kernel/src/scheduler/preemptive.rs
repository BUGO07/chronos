/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    alloc::Layout,
    cell::OnceCell,
    sync::atomic::{AtomicI32, AtomicU64, Ordering},
};

use crate::{
    drivers::fs::FileDescriptor,
    memory::{KERNEL_STACK_SIZE, USER_STACK_SIZE},
    utils::{asm::without_ints, spinlock::Spin},
};
use alloc::{
    collections::{btree_map::BTreeMap, vec_deque::VecDeque},
    sync::Arc,
    vec::Vec,
};

use crate::{
    arch::{drivers::time::preferred_timer_ns, system::cpu::Registers, system::lapic},
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
    ppid: u64,
    next_tid: AtomicU64,
    next_stack_addr: u64,
    cwd: fs::Path,
    pub fdt: BTreeMap<i32, FileDescriptor>,
    pub next_fd: AtomicI32,
    pub pagemap: Arc<Spin<Pagemap>>,
    children: Vec<Arc<Spin<Thread>>>,
    exit_status: Option<i32>,
}

unsafe impl Send for Process {}

impl Process {
    pub fn new(pagemap: Arc<Spin<Pagemap>>, name: &'static str, ppid: u64) -> Self {
        let pid = next_pid();
        Self {
            name,
            pid,
            ppid,
            next_tid: AtomicU64::new(1),
            next_stack_addr: 0x0000_7FFF_FF00_0000,
            cwd: fs::Path::new("/"),
            fdt: BTreeMap::new(),
            next_fd: AtomicI32::new(3),
            pagemap,
            children: Vec::new(),
            exit_status: None,
        }
    }

    pub fn get_name(&self) -> &'static str {
        self.name
    }

    pub fn get_pid(&self) -> u64 {
        self.pid
    }

    pub fn get_ppid(&self) -> u64 {
        self.ppid
    }

    pub fn get_exit_status(&self) -> Option<i32> {
        self.exit_status
    }

    pub fn set_exit_status(&mut self, status: i32) {
        self.exit_status = Some(status);
    }

    pub fn get_pagemap(&self) -> &Arc<Spin<Pagemap>> {
        &self.pagemap
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

    pub fn set_next_stack_addr(&mut self, addr: u64) {
        self.next_stack_addr = addr;
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

pub fn schedule(regs: &mut Registers) {
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
        let t = ct.lock();
        let status = t.get_status();
        if status == Status::Terminated {
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
        } else if status != Status::Blocked {
            drop(t);
            scheduler.queue.push_back(ct);
        }
    }

    let thread = next.unwrap();
    {
        let mut t = thread.lock();
        t.set_status(Status::Running);
        *regs = t.regs;
        t.schedule_time = preferred_timer_ns();

        let cr3 = t.cr3;
        if cr3 != 0 {
            unsafe { core::arch::asm!("mov cr3, {}", in(reg) cr3, options(nostack)) };
        }

        let gs_base = crate::arch::system::syscall::kernel_gs_base();
        if gs_base != 0 {
            unsafe {
                let cpu_data = gs_base as *mut u64;
                *cpu_data.add(2) = t.kstack + KERNEL_STACK_SIZE as u64;
            }
            crate::utils::asm::regs::wrmsr(0xC0000102, gs_base);
        }
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
        if let Some(proc) = scheduler
            .processes
            .iter()
            .find(|p| p.lock().pid == pid)
            .cloned()
        {
            let mut lock = proc.lock();
            for thread in lock.children.iter() {
                thread.lock().set_status(Status::Terminated);
            }
            if lock.exit_status.is_none() {
                lock.set_exit_status(0);
            }
            true
        } else {
            false
        }
    })
}

pub fn reap_process(pid: u64) {
    without_ints(|| {
        let scheduler = get_scheduler();
        scheduler.processes.retain(|p| p.lock().pid != pid);
    });
}

pub fn spawn_process(pagemap: Arc<Spin<Pagemap>>, name: &'static str, ppid: u64) -> u64 {
    without_ints(|| {
        let scheduler = get_scheduler();
        let proc = Arc::new(Spin::new(Process::new(pagemap, name, ppid)));
        let pid = proc.lock().get_pid();
        scheduler.processes.push(proc);
        pid
    })
}

pub fn fork_process(parent_regs: &Registers) -> u64 {
    without_ints(|| {
        let scheduler = get_scheduler();
        let current = scheduler.current.as_ref().unwrap().clone();
        let parent_proc = current.lock().get_parent().upgrade().unwrap();
        let parent_lock = parent_proc.lock();

        let parent_pid = parent_lock.get_pid();
        let new_pagemap = Arc::new(Spin::new(parent_lock.pagemap.lock().clone_userspace()));

        let child_pid = next_pid();
        let mut child = Process {
            name: parent_lock.name,
            pid: child_pid,
            ppid: parent_pid,
            next_tid: AtomicU64::new(parent_lock.next_tid.load(Ordering::Relaxed)),
            next_stack_addr: parent_lock.next_stack_addr,
            cwd: parent_lock.cwd.clone(),
            fdt: BTreeMap::new(),
            next_fd: AtomicI32::new(parent_lock.next_fd.load(Ordering::Relaxed)),
            pagemap: new_pagemap,
            children: Vec::new(),
            exit_status: None,
        };

        for (&fd_num, fd) in &parent_lock.fdt {
            child.fdt.insert(fd_num, fd.dup());
        }

        drop(parent_lock);

        let child_arc = Arc::new(Spin::new(child));

        let mut child_regs = *parent_regs;
        child_regs.rax = 0;

        let kstack_ptr =
            unsafe { alloc::alloc::alloc(Layout::from_size_align(KERNEL_STACK_SIZE, 16).unwrap()) };
        assert!(!kstack_ptr.is_null(), "failed to allocate kernel stack");

        let child_thread = Arc::new(Spin::new(Thread::new_from_regs(
            &child_arc,
            "main",
            child_regs,
            kstack_ptr as u64,
        )));

        child_arc
            .lock()
            .get_children_mut()
            .push(child_thread.clone());
        scheduler.processes.push(child_arc);
        scheduler.queue.push_back(child_thread);

        child_pid
    })
}

pub fn init() {
    unsafe { SCHEDULER.set(Scheduler::default()).ok() };
    get_scheduler()
        .processes
        .push(Arc::new(Spin::new(Process::new(
            unsafe { PAGEMAP.get().unwrap() }.clone(),
            "kernel",
            0,
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
