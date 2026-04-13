core::arch::global_asm!(include_str!("syscall.S"));

use core::{
    alloc::Layout,
    sync::atomic::{AtomicPtr, Ordering},
};

use alloc::boxed::Box;

use crate::{
    arch::{
        drivers::time::rtc::read_rtc,
        system::{cpu::Registers, syscall::id::SyscallId},
    },
    drivers::fs::{FileDescriptor, NodeMode, Path, Permissions, VfsNodeMetadataExt},
    info,
    memory::KERNEL_STACK_SIZE,
    print,
    scheduler::current_process,
    utils::asm::regs::{rdmsr, wrmsr},
};

pub mod id;

const ENOENT: i64 = 2;
const EIO: i64 = 5;
const EBADF: i64 = 9;
const EFAULT: i64 = 14;
const EEXIST: i64 = 17;
const ENOTDIR: i64 = 20;
const EINVAL: i64 = 22;
const ERANGE: i64 = 34;
const ENOSYS: i64 = 38;

const USER_ADDR_MAX: u64 = 0x0000_7FFF_FFFF_FFFF;

fn validate_user_buf(ptr: u64, len: u64) -> bool {
    if len == 0 {
        return true;
    }
    match ptr.checked_add(len.saturating_sub(1)) {
        Some(end) => ptr <= USER_ADDR_MAX && end <= USER_ADDR_MAX,
        None => false,
    }
}

fn validate_user_cstr(ptr: u64) -> Option<&'static str> {
    if ptr == 0 || ptr > USER_ADDR_MAX {
        return None;
    }
    let cstr = unsafe { core::ffi::CStr::from_ptr(ptr as *const core::ffi::c_char) };
    let len = cstr.to_bytes_with_nul().len() as _;
    if !validate_user_buf(ptr, len) {
        return None;
    }
    cstr.to_str().ok()
}

fn resolve_path(path_str: &str, cwd: &Path) -> Path {
    if path_str.starts_with('/') {
        Path::new(path_str)
    } else {
        cwd.join(path_str)
    }
}

#[repr(C)]
struct SyscallCpuData {
    _reserved: u64,
    user_rsp: u64,
    kernel_rsp: u64,
}

static HANDLERS: [AtomicPtr<()>; 333] = [const { AtomicPtr::new(sys_stub as _) }; 333];

#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(regs: &mut Registers) {
    let id = regs.rax;
    if id as usize >= HANDLERS.len() {
        regs.rax = -ENOSYS as _;
        return;
    }
    let handler_ptr = HANDLERS[id as usize].load(Ordering::Acquire);
    if !handler_ptr.is_null() {
        let handler: fn(&mut Registers) = unsafe { core::mem::transmute(handler_ptr) };
        handler(regs);
    }
}

unsafe extern "C" {
    fn syscall_entry();
}

fn sys_stub(regs: &mut Registers) {
    info!(
        "syscall {} with args {} {} {} {} {} {}",
        regs.rax, regs.rdi, regs.rsi, regs.rdx, regs.r10, regs.r8, regs.r9
    );
}

fn sys_exit(regs: &mut Registers) {
    let pid = crate::scheduler::current_process()
        .unwrap()
        .lock()
        .get_pid();
    info!("process {} exited with code {}", pid, regs.rdi);
    crate::scheduler::kill_process(pid);
}

#[repr(C)]
struct Timespec {
    tv_sec: i64,
    tv_nsec: i64,
}

fn sys_nanosleep(regs: &mut Registers) {
    let req_ptr = regs.rdi;

    if !validate_user_buf(req_ptr, core::mem::size_of::<Timespec>() as _) {
        regs.rax = -EFAULT as _;
        return;
    }

    let req = unsafe { &*(req_ptr as *const Timespec) };

    if req.tv_sec < 0 || req.tv_nsec < 0 || req.tv_nsec >= 1_000_000_000 {
        regs.rax = -EINVAL as _;
        return;
    }

    let ns = req.tv_sec as u64 * 1_000_000_000 + req.tv_nsec as u64;
    crate::scheduler::thread::sleep(ns);
    regs.rax = 0;
}

fn sys_yield(_regs: &mut Registers) {
    crate::scheduler::thread::yield_();
}

fn sys_read(regs: &mut Registers) {
    let fd = regs.rdi;
    let buf = regs.rsi;
    let count = regs.rdx;

    if !validate_user_buf(buf, count) {
        regs.rax = -EFAULT as _;
        return;
    }

    if fd == 0 {
        regs.rax = 0;
        return;
    }
    if fd == 1 || fd == 2 {
        regs.rax = -EBADF as _;
        return;
    }

    let current = current_process().unwrap();
    let mut lock = current.lock();
    let Some(file) = lock.fds.get_mut(&(fd as i32)) else {
        regs.rax = -EBADF as _;
        return;
    };

    if !file.permissions.contains(Permissions::READ) {
        regs.rax = -EBADF as _;
        return;
    }

    let slice = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, count as usize) };
    regs.rax = match file.read(slice) {
        Some(n) => n as _,
        None => -EIO as _,
    };
}

fn sys_write(regs: &mut Registers) {
    let fd = regs.rdi;
    let buf = regs.rsi;
    let count = regs.rdx;

    if !validate_user_buf(buf, count) {
        regs.rax = -EFAULT as _;
        return;
    }

    let slice = unsafe { core::slice::from_raw_parts(buf as *const u8, count as usize) };

    if fd == 1 || fd == 2 {
        if let Ok(s) = core::str::from_utf8(slice) {
            print!("{}", s);
        } else {
            print!("{:?}", slice);
        }
        regs.rax = count;
    } else if fd == 0 {
        regs.rax = -EBADF as _;
    } else {
        let current = current_process().unwrap();
        let mut lock = current.lock();
        let Some(file) = lock.fds.get_mut(&(fd as i32)) else {
            regs.rax = -EBADF as _;
            return;
        };

        if !file.permissions.contains(Permissions::WRITE) {
            regs.rax = -EBADF as _;
            return;
        }

        regs.rax = match file.write(slice) {
            Some(n) => n as _,
            None => -EIO as _,
        };
    }
}

fn sys_open(regs: &mut Registers) {
    bitflags::bitflags! {
        #[derive(Clone, Copy, PartialEq)]
        struct Flags: i32 {
            const O_RDONLY   = 0x00;
            const O_WRONLY   = 0x01;
            const O_RDWR     = 0x02;
            const PERMS_MASK = 0x03;

            const O_CREAT    = 0x40;
            const O_EXCL     = 0x80;
            const O_NOCTTY   = 0x100;
            const O_TRUNC    = 0x200;
            const O_APPEND   = 0x400;
            const O_NONBLOCK = 0x800;
            const O_DSYNC    = 0x1000;
            const O_ASYNC    = 0x2000;

            const O_DIRECTORY = 0x10000;
            const O_NOFOLLOW  = 0x20000;
            const O_CLOEXEC   = 0x80000;

            const O_SYNC    = 0x101000;
            const O_RSYNC   = 0x101000;
            const O_TMPFILE = 0x410000;
        }
    }

    let flags = Flags::from_bits_retain(regs.rsi as i32);

    let mode = if flags.contains(Flags::O_CREAT) {
        let Some(mode) = NodeMode::from_bits(regs.rdx as i32) else {
            regs.rax = -EINVAL as _;
            return;
        };
        mode
    } else {
        NodeMode::empty()
    };

    let Some(path_str) = validate_user_cstr(regs.rdi) else {
        regs.rax = -EFAULT as _;
        return;
    };

    let current = current_process().unwrap();
    let mut proc = current.lock();
    let path = resolve_path(path_str, proc.get_cwd());

    let perms = match flags & Flags::PERMS_MASK {
        Flags::O_RDONLY => Permissions::READ,
        Flags::O_WRONLY => Permissions::WRITE,
        Flags::O_RDWR => Permissions::RW,
        _ => Permissions::READ,
    };

    let mut vfs = crate::drivers::fs::get_vfs();

    let file = if vfs.resolve_path(path.clone()).is_some() {
        let file = vfs.resolve_path_mut(path.clone()).unwrap();

        if flags.contains(Flags::O_CREAT) && flags.contains(Flags::O_EXCL) {
            regs.rax = -EEXIST as _;
            return;
        }

        if flags.contains(Flags::O_DIRECTORY) && !file.is_dir() {
            regs.rax = -ENOTDIR as _;
            return;
        }

        if flags.contains(Flags::O_TRUNC)
            && (perms == Permissions::WRITE || perms == Permissions::RW)
        {
            file.truncate(0);
        }

        FileDescriptor::new(file, perms)
    } else {
        if !flags.contains(Flags::O_CREAT) {
            regs.rax = -ENOENT as _;
            return;
        }

        let Some(parent) = vfs.resolve_path_mut(path.get_parent()) else {
            regs.rax = -ENOENT as _;
            return;
        };

        let Some(created) = parent.create_file(path.get_name()) else {
            regs.rax = -EEXIST as _;
            return;
        };

        let epoch = read_rtc().to_epoch().unwrap_or_default();

        created
            .with_permissions(mode)
            .with_created_at(epoch)
            .with_modified_at(epoch);

        FileDescriptor::new(created.as_mut(), perms)
    };

    drop(vfs);

    let fd = proc.next_fd.fetch_add(1, Ordering::SeqCst);

    proc.fds
        .insert(fd, file.with_append(flags.contains(Flags::O_APPEND)));

    regs.rax = fd as _;
}

fn sys_close(regs: &mut Registers) {
    let current = current_process().unwrap();
    let mut lock = current.lock();
    let fd = regs.rdi as i32;
    if lock.fds.remove(&fd).is_some() {
        regs.rax = 0;
    } else {
        regs.rax = -EBADF as _;
    }
}

fn sys_lseek(regs: &mut Registers) {
    const SEEK_SET: u64 = 0;
    const SEEK_CUR: u64 = 1;
    const SEEK_END: u64 = 2;
    let current = current_process().unwrap();
    let mut lock = current.lock();
    let fd = regs.rdi as i32;
    let offset = regs.rsi as i64;
    let whence = regs.rdx;

    let Some(file) = lock.fds.get_mut(&fd) else {
        regs.rax = -EBADF as _;
        return;
    };

    let new_pos = match whence {
        SEEK_SET => offset,
        SEEK_CUR => file.offset as i64 + offset,
        SEEK_END => file.node().size() as i64 + offset,
        _ => {
            regs.rax = -EINVAL as _;
            return;
        }
    };

    if new_pos < 0 {
        regs.rax = -EINVAL as _;
        return;
    }

    file.offset = new_pos as _;
    regs.rax = new_pos as _;
}

fn sys_get_cwd(regs: &mut Registers) {
    let buf = regs.rdi;
    let count = regs.rsi;

    if !validate_user_buf(buf, count) {
        regs.rax = -EFAULT as _;
        return;
    }

    let current = current_process().unwrap();
    let lock = current.lock();
    let cwd = lock.get_cwd().as_str().as_bytes();

    if (count as usize) < cwd.len() + 1 {
        regs.rax = -ERANGE as _;
        return;
    }

    let slice = unsafe { core::slice::from_raw_parts_mut(buf as *mut u8, count as usize) };
    slice[..cwd.len()].copy_from_slice(cwd);
    slice[cwd.len()] = 0;
    regs.rax = regs.rdi;
}

fn sys_mkdir(regs: &mut Registers) {
    let mode = NodeMode::from_bits_truncate(regs.rsi as i32);

    let Some(path_str) = validate_user_cstr(regs.rdi) else {
        regs.rax = -EFAULT as _;
        return;
    };

    let current = current_process().unwrap();
    let path = resolve_path(path_str, current.lock().get_cwd());

    let mut vfs = crate::drivers::fs::get_vfs();
    let Some(parent) = vfs.resolve_path_mut(path.get_parent()) else {
        regs.rax = -ENOENT as _;
        return;
    };

    if let Some(dir) = parent.create_dir(path.get_name()) {
        dir.get_metadata_mut().permissions = mode;
        regs.rax = 0;
    } else {
        regs.rax = -EEXIST as _;
    }
}

pub fn init() {
    HANDLERS[SyscallId::Read as usize].store(sys_read as _, Ordering::Release);
    HANDLERS[SyscallId::Write as usize].store(sys_write as _, Ordering::Release);
    HANDLERS[SyscallId::Open as usize].store(sys_open as _, Ordering::Release);
    HANDLERS[SyscallId::Close as usize].store(sys_close as _, Ordering::Release);
    HANDLERS[SyscallId::Lseek as usize].store(sys_lseek as _, Ordering::Release);
    HANDLERS[SyscallId::Getcwd as usize].store(sys_get_cwd as _, Ordering::Release);
    HANDLERS[SyscallId::Mkdir as usize].store(sys_mkdir as _, Ordering::Release);
    HANDLERS[SyscallId::SchedYield as usize].store(sys_yield as _, Ordering::Release);
    HANDLERS[SyscallId::Nanosleep as usize].store(sys_nanosleep as _, Ordering::Release);
    HANDLERS[SyscallId::Exit as usize].store(sys_exit as _, Ordering::Release);

    // IA32_EFER syscall
    wrmsr(0xC0000080, rdmsr(0xC0000080) | (1 << 0));
    // IA32_STAR
    wrmsr(0xC0000081, (0x18_u64 << 48) | (0x08_u64 << 32));
    // IA32_LSTAR handler
    wrmsr(0xC0000082, syscall_entry as *const () as _);
    // IA32_FMASK rflags mask
    wrmsr(0xC0000084, !2);

    let kernel_stack = unsafe {
        alloc::alloc::alloc(Layout::from_size_align(KERNEL_STACK_SIZE, 0x10).unwrap()) as u64
            + KERNEL_STACK_SIZE as u64
    };
    let cpu_data = Box::into_raw(Box::new(SyscallCpuData {
        _reserved: 0,
        user_rsp: 0,
        kernel_rsp: kernel_stack,
    }));
    // IA32_KERNEL_GS_BASE
    wrmsr(0xC0000102, cpu_data as _);
}
