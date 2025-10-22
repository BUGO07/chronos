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
    memory::{KERNEL_STACK_SIZE, USER_STACK_SIZE},
    println,
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

// Minimal ELF structures (64-bit little endian) used by the loader
#[repr(C)]
#[derive(Copy, Clone)]
struct ElfHeader {
    e_ident: [u8; 16],
    e_type: u16,
    e_machine: u16,
    e_version: u32,
    e_entry: u64,
    e_phoff: u64,
    e_shoff: u64,
    e_flags: u32,
    e_ehsize: u16,
    e_phentsize: u16,
    e_phnum: u16,
    e_shentsize: u16,
    e_shnum: u16,
    e_shstrndx: u16,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct ProgramHeader {
    p_type: u32,
    p_flags: u32,
    p_offset: u64,
    p_vaddr: u64,
    p_paddr: u64,
    p_filesz: u64,
    p_memsz: u64,
    p_align: u64,
}

#[derive(Clone, Default)]
pub struct ElfLoadResult {
    pub real_entry: u64,
    pub interp_entry: u64,
    pub phdr: u64,
    pub phnum: u16,
    pub phentsize: u16,
    pub status: i32,
}

// ELF constants we need
const PT_LOAD: u32 = 1;
const PT_PHDR: u32 = 6;
const PT_INTERP: u32 = 3;
const ET_DYN: u16 = 3;

use crate::memory::vmm::flag as PTE_FLAG;

const ENOENT: i32 = 2;

impl ElfHeader {
    fn as_ptr<'a>(buf: *const u8) -> &'a Self {
        unsafe { &*(buf as u64 as *const ElfHeader) }
    }
}

impl ProgramHeader {
    fn from_ptr<'a>(ptr: *const u8) -> &'a Self {
        unsafe { &*(ptr as *const ProgramHeader) }
    }
}

pub fn load_elf_from_buffer(proc: &mut Process, buf: &[u8]) -> ElfLoadResult {
    let mut res = ElfLoadResult::default();

    if buf.len() < core::mem::size_of::<ElfHeader>() {
        res.status = -1;
        return res;
    }

    let head = ElfHeader::as_ptr(buf.as_ptr());

    if head.e_ident[0..4] != [0x7F, b'E', b'L', b'F'] {
        res.status = ENOENT;
        return res;
    }

    res.real_entry = head.e_entry;
    res.interp_entry = head.e_entry;
    res.phnum = head.e_phnum;
    res.phentsize = head.e_phentsize;

    let mut elf_base: u64 = u64::MAX;
    let mut size: u64 = 0;
    let _phdr_vaddr: u64 = 0;

    for i in 0..head.e_phnum {
        let off = head.e_phoff as usize + (head.e_phentsize as usize) * (i as usize);
        if off + core::mem::size_of::<ProgramHeader>() > buf.len() {
            continue;
        }
        let ph = ProgramHeader::from_ptr(unsafe { buf.as_ptr().add(off) });
        if ph.p_type == PT_LOAD && ph.p_vaddr < elf_base {
            elf_base = ph.p_vaddr;
        }
    }

    for i in 0..head.e_phnum {
        let off = head.e_phoff as usize + (head.e_phentsize as usize) * (i as usize);
        if off + core::mem::size_of::<ProgramHeader>() > buf.len() {
            continue;
        }
        let ph = ProgramHeader::from_ptr(unsafe { buf.as_ptr().add(off) });
        if ph.p_type == PT_PHDR {
            res.phdr = ph.p_vaddr;
        } else if ph.p_type == PT_INTERP && (ph.p_offset as usize) < buf.len() {
            let mut end = ph.p_offset as usize;
            while end < buf.len() && buf[end] != 0 {
                end += 1;
            }
            let interp = core::str::from_utf8(&buf[ph.p_offset as usize..end]).unwrap_or("");

            if let Some(node) =
                crate::drivers::fs::get_vfs().resolve_path(crate::drivers::fs::Path::new(interp))
                && let Some(data) = node.read()
            {
                let _locked = proc.pagemap.lock();

                let inner = load_elf_from_buffer(proc, data);
                res.interp_entry = inner.real_entry;
            }
        }

        if ph.p_type == PT_LOAD {
            let end = ph.p_vaddr - elf_base + ph.p_memsz;
            if end > size {
                size = end;
            }
        }
    }

    let elf_vaddr = if head.e_type != ET_DYN {
        let mut locked = proc.pagemap.lock();

        let page_size = crate::memory::vmm::page_size::SMALL;
        let flags = PTE_FLAG::PRESENT | PTE_FLAG::WRITE | PTE_FLAG::USER;
        let mut off = 0u64;
        while off < size {
            let va = elf_base + off;

            let phys = unsafe {
                alloc::alloc::alloc_zeroed(
                    core::alloc::Layout::from_size_align(page_size as usize, page_size as usize)
                        .unwrap(),
                ) as u64
            } - crate::utils::limine::get_hhdm_offset();
            locked.map(va, phys, flags, page_size).unwrap();
            off += page_size;
        }
        elf_base
    } else {
        let mut locked = proc.pagemap.lock();
        let base = proc.next_stack_address();
        let page_size = crate::memory::vmm::page_size::SMALL;
        let flags = PTE_FLAG::PRESENT | PTE_FLAG::WRITE | PTE_FLAG::USER;
        let mut off = 0u64;
        while off < size {
            let va = base + off;
            let phys = unsafe {
                alloc::alloc::alloc_zeroed(
                    core::alloc::Layout::from_size_align(page_size as usize, page_size as usize)
                        .unwrap(),
                ) as u64
            } - crate::utils::limine::get_hhdm_offset();
            locked.map(va, phys, flags, page_size).unwrap();
            off += page_size;
        }
        res.real_entry = base + head.e_entry;
        base
    };

    let hhdm = crate::utils::limine::get_hhdm_offset();
    for i in 0..head.e_phnum {
        let off = head.e_phoff as usize + (head.e_phentsize as usize) * (i as usize);
        if off + core::mem::size_of::<ProgramHeader>() > buf.len() {
            continue;
        }
        let ph = ProgramHeader::from_ptr(unsafe { buf.as_ptr().add(off) });
        if ph.p_type == PT_LOAD {
            let dest = if head.e_type != ET_DYN {
                elf_vaddr as usize + (ph.p_vaddr - elf_base) as usize
            } else {
                elf_vaddr as usize + ph.p_vaddr as usize
            };

            unsafe {
                let dst_ptr = (dest + hhdm as usize) as *mut u8;

                for j in 0..(ph.p_memsz as usize) {
                    core::ptr::write_volatile(dst_ptr.add(j), 0);
                }

                if (ph.p_offset as usize) + (ph.p_filesz as usize) <= buf.len() {
                    core::ptr::copy_nonoverlapping(
                        buf.as_ptr().add(ph.p_offset as usize),
                        dst_ptr,
                        ph.p_filesz as usize,
                    );
                }
            }
        }
    }

    res.status = 0;
    res
}

pub fn loadelf_from_path(
    proc_arc: &'static Arc<SpinLock<Process>>,
    path: &str,
    _argv: &[&str],
    _envp: &[&str],
) -> i32 {
    let mut proc_locked = proc_arc.lock();

    let node = match crate::drivers::fs::get_vfs().resolve_path(crate::drivers::fs::Path::new(path))
    {
        Some(n) => n,
        None => return ENOENT,
    };
    let data = match node.read() {
        Some(d) => d,
        None => return ENOENT,
    };

    let elfload = load_elf_from_buffer(&mut proc_locked, data);
    if elfload.status != 0 {
        return elfload.status;
    }

    drop(proc_locked);

    spawn(proc_arc, elfload.interp_entry as usize, "main", true);

    0
}

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
        self.next_stack_addr -= USER_STACK_SIZE as u64;
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
                            Layout::from_size_align(KERNEL_STACK_SIZE, 0x8).unwrap(),
                        );
                        if t.ustack != 0 {
                            alloc::alloc::dealloc(
                                (t.ustack + get_hhdm_offset()) as *mut u8,
                                Layout::from_size_align(USER_STACK_SIZE, 0x8).unwrap(),
                            );
                        }
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
