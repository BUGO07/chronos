/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    cell::OnceCell,
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(feature = "kernel")]
use alloc::boxed::Box;
use alloc::{
    collections::{binary_heap::BinaryHeap, vec_deque::VecDeque},
    sync::Arc,
    vec::Vec,
};

use crate::{
    asm::halt_loop,
    fs::Path,
    memory::USER_STACK_SIZE,
    spinlock::SpinLock,
    thread::{Thread, ThreadStatus, yld},
};

#[cfg(feature = "kernel")]
use crate::{
    fs::VfsNode,
    kernel::{
        bootloader::get_hhdm_offset,
        paging::{PAGEMAP, Pagemap, page_size},
        time::preferred_timer_ns,
    },
};

pub static mut SCHEDULER: OnceCell<Scheduler> = OnceCell::new();
pub static mut LAPIC_ARM: OnceCell<fn(ns: usize, vector: u8)> = OnceCell::new();
pub static mut NEXT_USTACK_ADDR: u64 = 0x0000_7FFF_FF00_0000;

fn next_pid() -> u64 {
    get_scheduler().next_pid.fetch_add(1, Ordering::Relaxed)
}

pub fn next_stack_address() -> u64 {
    unsafe {
        NEXT_USTACK_ADDR -= USER_STACK_SIZE as u64;
        NEXT_USTACK_ADDR
    }
}

pub struct Scheduler {
    pub processes: Vec<Arc<SpinLock<Process>>>,
    pub current: Option<Arc<SpinLock<Thread>>>,
    pub queue: BinaryHeap<Arc<SpinLock<Thread>>>,
    pub secondary_queue: VecDeque<Arc<SpinLock<Thread>>>,
    pub timeslice: usize,
    pub next_pid: AtomicU64,
}

impl Default for Scheduler {
    fn default() -> Self {
        Self {
            processes: Vec::new(),
            current: None,
            queue: BinaryHeap::new(),
            secondary_queue: VecDeque::new(),
            timeslice: 1_000_000,
            next_pid: AtomicU64::new(0),
        }
    }
}

pub struct Process {
    pub name: &'static str,
    pub pid: u64,
    pub next_tid: AtomicU64,
    pub cwd: Path,
    #[cfg(feature = "kernel")]
    pub fd_table: Vec<Option<Box<dyn VfsNode>>>,
    #[cfg(feature = "kernel")]
    pub pagemap: Arc<SpinLock<Pagemap>>,
    pub children: Vec<Arc<SpinLock<Thread>>>,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

impl Process {
    pub fn new(name: &'static str) -> Self {
        let pid = next_pid();
        // debug!("spawning new process {}", _pid);
        Self {
            name,
            pid,
            next_tid: AtomicU64::new(1),
            cwd: Path::new("/").join("home"),
            #[cfg(feature = "kernel")]
            fd_table: Vec::new(),
            #[cfg(feature = "kernel")]
            pagemap: unsafe { PAGEMAP.get().unwrap().clone() },
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

    pub fn get_cwd(&self) -> &Path {
        &self.cwd
    }

    pub fn set_cwd(&mut self, path: Path) {
        self.cwd = path;
    }

    pub fn next_tid(&mut self) -> u64 {
        self.next_tid.fetch_add(1, Ordering::Relaxed)
    }
}

#[cfg(feature = "kernel")]
#[cfg_attr(feature = "profiling", embedded_profiling::profile_function)]
pub fn schedule(regs: *mut crate::StackFrame) {
    use core::{alloc::Layout, ffi::c_void};

    use crate::{
        StackFrame,
        asm::mem::memcpy,
        memory::KERNEL_STACK_SIZE,
        thread::{current_thread, idle0},
    };

    let mut next = None;

    if let Some(ct) = current_thread() {
        let mut t = ct.lock();
        memcpy(
            &raw mut t.regs as *mut c_void,
            regs as *mut c_void,
            size_of::<StackFrame>(),
        );
        if t.get_status() == &ThreadStatus::Running {
            t.set_status(ThreadStatus::Ready);
        }
        t.runtime += preferred_timer_ns() - t.schedule_time;
    }

    let scheduler = get_scheduler();

    let mut count = scheduler.queue.len();
    while count > 0 {
        if let Some(thread) = next_thread() {
            count -= 1;
            let mut t = thread.lock();
            match t.get_status() {
                ThreadStatus::Ready => {
                    next = Some(thread.clone());
                    break;
                }
                ThreadStatus::Sleeping(when) => {
                    if &preferred_timer_ns() >= when {
                        t.set_status(ThreadStatus::Ready);
                        next = Some(thread.clone());
                        break;
                    } else {
                        scheduler.secondary_queue.push_back(thread.clone());
                    }
                }
                ThreadStatus::Terminated => {
                    unsafe {
                        let parent = t.get_parent().upgrade().unwrap();
                        let mut p = parent.lock();

                        alloc::alloc::dealloc(
                            t.kstack as *mut u8,
                            Layout::from_size_align(KERNEL_STACK_SIZE, 0x8).unwrap(),
                        );

                        if t.ustack != 0 {
                            for i in (0..USER_STACK_SIZE).step_by(page_size::SMALL as usize) {
                                p.pagemap
                                    .lock()
                                    .unmap(t.ustack + i as u64, page_size::SMALL);
                            }
                            alloc::alloc::dealloc(
                                (t.ustack_phys + get_hhdm_offset()) as *mut u8,
                                Layout::from_size_align(USER_STACK_SIZE, page_size::SMALL as usize)
                                    .unwrap(),
                            );
                        }

                        let tid = t.get_tid();
                        thread.force_unlock();
                        p.children.retain(|x| x.lock().get_tid() != tid);
                    };
                }
                ThreadStatus::Blocked => {
                    // ignored
                }
                _ => enqueue(thread.clone()),
            }
        }
    }

    for thread in scheduler.secondary_queue.drain(..) {
        match thread.lock().get_status() {
            ThreadStatus::Blocked => {
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

    t.set_status(ThreadStatus::Running);

    unsafe {
        // very weird ik
        core::arch::asm!("mov cr3, {}", in(reg) t.parent.upgrade().unwrap().lock().pagemap.lock().top_level as u64, options(nostack));
    }

    memcpy(
        regs as *mut c_void,
        &raw const t.regs as *const c_void,
        size_of::<StackFrame>(),
    );

    (unsafe { LAPIC_ARM.get().unwrap() })(scheduler.timeslice, 0xFF);
    t.schedule_time = preferred_timer_ns();
}

#[inline(always)]
pub fn enqueue(thread: Arc<SpinLock<Thread>>) {
    get_scheduler().queue.push(thread);
}

#[inline(always)]
pub fn next_thread() -> Option<Arc<SpinLock<Thread>>> {
    get_scheduler().queue.pop()
}

pub fn kill_process(pid: u64) -> bool {
    let scheduler = get_scheduler();
    if pid == 0 {
        // crate::drivers::acpi::shutdown();
    }
    if let Some(pos) = scheduler.processes.iter().position(|p| p.lock().pid == pid) {
        let proc = scheduler.processes.get(pos).unwrap();

        for thread in proc.lock().children.iter() {
            thread.lock().set_status(ThreadStatus::Terminated);
        }

        scheduler.processes.remove(pos);

        true
    } else {
        false
    }
}

pub fn spawn_process(name: &'static str) -> u64 {
    let scheduler = get_scheduler();
    let proc = Arc::new(SpinLock::new(Process::new(name)));
    scheduler.processes.push(proc.clone());
    proc.lock().get_pid()
}

pub fn init() {
    unsafe { SCHEDULER.set(Scheduler::default()).ok() };
    get_scheduler()
        .processes
        .push(Arc::new(SpinLock::new(Process::new("kernel"))));
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
    get_scheduler().processes.iter().find(|p| unsafe {
        p.force_unlock();
        p.lock().pid == pid
    })
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
