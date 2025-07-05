/*
    Copyright (C) 2025 bugo07
    Released under EUPL 1.2 License
*/

use core::{arch::asm, cell::OnceCell};

use alloc::{
    string::String,
    sync::{Arc, Weak},
    vec::Vec,
};

#[cfg(feature = "kernel")]
use crate::kernel::{bootloader::get_hhdm_offset, paging::page_size};
use crate::{
    StackFrame,
    asm::halt_loop,
    sched::{Process, enqueue, get_scheduler},
    spinlock::SpinLock,
    time::preferred_timer_ns,
};

#[derive(Debug, PartialEq, Eq)]
pub enum ThreadStatus {
    Ready,
    Running,
    Sleeping(u64), // ns
    Blocked,
    Terminated,
}

pub struct Thread {
    pub name: &'static str,
    pub tid: u64,
    pub args: Vec<String>,
    pub entry: u64,
    pub kstack: u64,
    pub ustack: u64,
    pub ustack_phys: u64,
    pub regs: StackFrame,
    pub parent: Weak<SpinLock<Process>>,
    pub status: ThreadStatus,
    pub runtime: u64,
    pub schedule_time: u64,
}

impl core::fmt::Debug for Thread {
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
            return core::cmp::Ordering::Less;
        }
        other.runtime.cmp(&self.runtime)
    }
}

unsafe impl Send for Thread {}
unsafe impl Sync for Thread {}

impl Thread {
    #[cfg(feature = "kernel")]
    pub fn new(
        proc: &'static Arc<SpinLock<Process>>,
        func: usize,
        name: &'static str,
        user: bool,
        args: Vec<String>,
    ) -> Self {
        Self::new_with_tid(proc, func, name, user, args, proc.lock().next_tid())
    }
    #[cfg(feature = "kernel")]
    pub fn new_with_tid(
        proc: &'static Arc<SpinLock<Process>>,
        func: usize,
        name: &'static str,
        user: bool,
        args: Vec<String>,
        tid: u64,
    ) -> Self {
        use crate::{
            asm::mem::memcpy,
            memory::{KERNEL_STACK_SIZE, USER_STACK_SIZE},
            sched::next_stack_address,
        };
        use core::{alloc::Layout, ffi::c_void};

        let kstack = unsafe {
            alloc::alloc::alloc(Layout::from_size_align(KERNEL_STACK_SIZE, 0x8).unwrap()) as u64
        };

        let mut ustack: u64 = 0;
        let mut ustack_phys = 0;
        let mut argv_ptrs: [u64; 64] = [0; 64];
        let mut argc = 0;

        if user {
            ustack_phys = unsafe {
                alloc::alloc::alloc(
                    Layout::from_size_align(USER_STACK_SIZE, page_size::SMALL as usize).unwrap(),
                ) as u64
                    - get_hhdm_offset()
            };

            unsafe { proc.force_unlock() };

            ustack = next_stack_address();

            for i in (0..USER_STACK_SIZE).step_by(page_size::SMALL as usize) {
                proc.lock().pagemap.lock().map(
                    ustack + i as u64,
                    ustack_phys + i as u64,
                    crate::kernel::paging::flag::RW | crate::kernel::paging::flag::USER,
                    page_size::SMALL,
                );
            }

            let mut stack = [0u8; 0x1000];
            let mut string_data_offset = 0x100;

            for arg in &args {
                let bytes = arg.as_bytes();
                let len = bytes.len();

                if string_data_offset + len + 1 > stack.len() {
                    panic!("Not enough stack space for argv strings");
                }

                let str_start = string_data_offset;
                stack[str_start..str_start + len].copy_from_slice(bytes);
                stack[str_start + len] = 0;

                argv_ptrs[argc] = ustack + str_start as u64;
                string_data_offset += len + 1;
                argc += 1;
            }

            argv_ptrs[argc] = 0;
            argc += 1;

            for (i, &ptr) in argv_ptrs[..argc].iter().enumerate() {
                let bytes = ptr.to_le_bytes();
                stack[i * 8..(i + 1) * 8].copy_from_slice(&bytes);
            }

            memcpy(
                ustack as *mut c_void,
                stack.as_ptr() as *const c_void,
                stack.len(),
            );
        }

        Self {
            name,
            tid,
            kstack,
            ustack,
            ustack_phys,
            args,
            entry: func as u64,
            regs: StackFrame {
                cs: if user { 0x20 | 0x03 } else { 0x08 },
                ss: if user { 0x28 | 0x03 } else { 0x10 },
                rsp: if user {
                    (ustack + USER_STACK_SIZE as u64) & !0xF
                } else {
                    kstack + KERNEL_STACK_SIZE as u64
                },
                rip: func as u64,
                rsi: ustack,
                rdi: argc.saturating_sub(1) as u64,
                rflags: 0x202,

                ..Default::default()
            },
            parent: Arc::downgrade(proc),
            status: ThreadStatus::Ready,
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

    pub fn get_status(&self) -> &ThreadStatus {
        &self.status
    }

    pub fn set_status(&mut self, status: ThreadStatus) {
        self.status = status;
    }

    pub fn is_user(&self) -> bool {
        self.ustack != 0
    }
}

#[cfg(feature = "kernel")]
pub fn spawn(
    proc: &'static Arc<SpinLock<Process>>,
    func: usize,
    name: &'static str,
    user: bool,
) -> u64 {
    spawn_with_args(proc, func, name, user, Vec::new())
}

#[cfg(feature = "kernel")]
pub fn spawn_with_args(
    proc: &'static Arc<SpinLock<Process>>,
    func: usize,
    name: &'static str,
    user: bool,
    args: Vec<String>,
) -> u64 {
    unsafe { proc.force_unlock() };
    let thread = Arc::new(SpinLock::new(Thread::new(proc, func, name, user, args)));
    unsafe { proc.force_unlock() };
    proc.lock().get_children_mut().push(thread.clone());
    unsafe { thread.force_unlock() };
    let tid = thread.lock().get_tid();
    unsafe { thread.force_unlock() };
    enqueue(thread);
    tid
}

pub fn sleep(ns: u64) {
    if let Some(thread) = current_thread() {
        let mut t = thread.lock();
        t.set_status(ThreadStatus::Sleeping(preferred_timer_ns() + ns));
    }
    yld();
}

#[inline(always)]
pub fn sleep_ms(ms: u64) {
    sleep(ms * 1_000_000);
}

pub fn yld() {
    unsafe {
        asm!("int $0xFF");
    }
}

pub fn block() {
    if let Some(thread) = current_thread() {
        thread.lock().set_status(ThreadStatus::Blocked);
        yld();
    }
}

pub fn wake(thread: &Arc<SpinLock<Thread>>) {
    let mut t: crate::spinlock::SpinLockGuard<'_, Thread> = thread.lock();
    if t.get_status() == &ThreadStatus::Blocked {
        t.set_status(ThreadStatus::Ready);
        enqueue(thread.clone());
    }
}

pub fn terminate() -> ! {
    if let Some(thread) = current_thread() {
        thread.lock().set_status(ThreadStatus::Terminated);
        yld();
    }
    halt_loop()
}

pub fn idle0() -> &'static Arc<SpinLock<Thread>> {
    static mut IDLE: OnceCell<Arc<SpinLock<Thread>>> = OnceCell::new();
    unsafe {
        #[cfg(feature = "kernel")]
        {
            IDLE.get_or_init(|| {
                let t = Arc::new(SpinLock::new(Thread::new_with_tid(
                    crate::sched::get_proc_by_pid(0).unwrap(),
                    halt_loop as usize,
                    "idle",
                    false,
                    alloc::vec![],
                    99,
                )));
                t.lock().set_status(ThreadStatus::Ready);
                t
            })
        }
        #[cfg(not(feature = "kernel"))]
        {
            IDLE.get().unwrap()
        }
    }
}

pub fn current_thread() -> &'static mut Option<Arc<SpinLock<Thread>>> {
    &mut get_scheduler().current
}
