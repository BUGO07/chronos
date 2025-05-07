/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{
    alloc::Layout,
    arch::asm,
    cell::OnceCell,
    ffi::c_void,
    sync::atomic::{AtomicU64, Ordering},
};

use alloc::{
    collections::linked_list::LinkedList,
    sync::{Arc, Weak},
    vec::Vec,
};
use spin::Mutex;

use crate::{
    arch::{interrupts::StackFrame, system::lapic},
    memory::{
        STACK_SIZE,
        vmm::{PAGEMAP, Pagemap},
    },
    utils::asm::{
        halt_loop,
        regs::{get_cs_reg, get_ss_reg},
    },
};

static NEXT_PID: AtomicU64 = AtomicU64::new(0);
static mut QUEUE: LinkedList<Arc<Mutex<Thread>>> = LinkedList::new();
static mut PROCESSES: Vec<Arc<Mutex<Process>>> = Vec::new();
static mut CURRENT_THREAD: Option<Arc<Mutex<Thread>>> = None;

const TIMESLICE: usize = 6;

fn next_pid() -> u64 {
    NEXT_PID.fetch_add(1, Ordering::Relaxed)
}

pub struct Process {
    _pid: u64,
    next_tid: AtomicU64,
    _pagemap: Arc<Mutex<Pagemap>>,
    children: Vec<Arc<Mutex<Thread>>>,
}

unsafe impl Send for Process {}
unsafe impl Sync for Process {}

impl Process {
    pub fn new(_pagemap: Arc<Mutex<Pagemap>>) -> Self {
        Self {
            _pid: next_pid(),
            next_tid: AtomicU64::new(1),
            _pagemap,
            children: Vec::new(),
        }
    }

    fn next_tid(&mut self) -> u64 {
        self.next_tid.fetch_add(1, Ordering::Relaxed)
    }
}

struct Thread {
    _tid: u64,
    _kstack: u64,
    regs: StackFrame,
    _parent: Weak<Mutex<Process>>,
}

unsafe impl Sync for Thread {}
unsafe impl Send for Thread {}

impl Thread {
    pub fn new(proc: Arc<Mutex<Process>>, func: u64) -> Self {
        let _kstack = unsafe {
            alloc::alloc::alloc(Layout::from_size_align(STACK_SIZE, 0x8).unwrap()) as u64
        };
        Self {
            _tid: proc.lock().next_tid(),
            _kstack,
            regs: StackFrame {
                rsp: _kstack + STACK_SIZE as u64,
                rip: func,
                cs: get_cs_reg() as u64,
                ss: get_ss_reg() as u64,
                rflags: 0x202,
                ..Default::default()
            },
            _parent: Arc::downgrade(&proc),
        }
    }
}

pub fn schedule(regs: *mut StackFrame) {
    let next = next_thread();
    if let Some(ct) = unsafe { CURRENT_THREAD.clone() } {
        crate::utils::asm::mem::memcpy(
            &raw mut ct.lock().regs as *mut c_void,
            regs as *mut c_void,
            size_of::<StackFrame>(),
        );
    }
    if let Some(n) = next {
        if let Some(ct) = unsafe { CURRENT_THREAD.clone() } {
            enqueue(ct)
        }

        unsafe { CURRENT_THREAD = Some(Arc::clone(&n)) };

        crate::utils::asm::mem::memcpy(
            regs as *mut c_void,
            &raw const n.lock().regs as *const c_void,
            size_of::<StackFrame>(),
        );
    }
    lapic::arm(TIMESLICE * 1_000_000, 0xFF);
}

fn next_thread() -> Option<Arc<Mutex<Thread>>> {
    unsafe { QUEUE.pop_back() }
}

fn enqueue(thread: Arc<Mutex<Thread>>) {
    unsafe { QUEUE.push_front(thread) };
}

pub static mut PID0: OnceCell<Arc<Mutex<Process>>> = OnceCell::new();

pub fn init_pid0() {
    unsafe {
        PID0.set(Arc::new(Mutex::new(Process::new(Arc::clone(
            PAGEMAP.get().unwrap(),
        )))))
        .ok()
    };
    unsafe { PROCESSES.push(Arc::clone(PID0.get().unwrap())) };
}

pub fn new_thread(proc: Arc<Mutex<Process>>, func: usize) {
    let thread = Arc::new(Mutex::new(Thread::new(proc.clone(), func as u64)));
    proc.lock().children.push(thread.clone());
    enqueue(thread);
}

pub fn init() -> ! {
    unsafe { asm!("int $0xFF") };
    halt_loop();
}
