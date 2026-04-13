core::arch::global_asm!(include_str!("syscall.S"));

use core::{
    alloc::Layout,
    sync::atomic::{AtomicPtr, Ordering},
};

use alloc::{boxed::Box, string::ToString};

use crate::{
    arch::{
        drivers::time::rtc::read_rtc,
        system::{cpu::Registers, syscall::id::SyscallId},
    },
    drivers::fs::{FileDescriptor, NodeMode, Path, Permissions, VfsNode, VfsNodeMetadataExt},
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
const EISDIR: i64 = 21;
const ENOTDIR: i64 = 20;
const EINVAL: i64 = 22;
const ERANGE: i64 = 34;
const ENOSYS: i64 = 38;
const ENOTEMPTY: i64 = 39;

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

fn sys_getpid(regs: &mut Registers) {
    let current = current_process().unwrap();
    regs.rax = current.lock().get_pid();
}

fn sys_getppid(regs: &mut Registers) {
    regs.rax = 0;
}

#[repr(C)]
struct UtsName {
    sysname: [u8; 65],
    nodename: [u8; 65],
    release: [u8; 65],
    version: [u8; 65],
    machine: [u8; 65],
    domainname: [u8; 65],
}

fn fill_uts_field(field: &mut [u8; 65], value: &[u8]) {
    let len = value.len().min(64);
    field[..len].copy_from_slice(&value[..len]);
    field[len] = 0;
}

fn sys_uname(regs: &mut Registers) {
    let buf = regs.rdi;
    if !validate_user_buf(buf, core::mem::size_of::<UtsName>() as _) {
        regs.rax = -EFAULT as _;
        return;
    }

    let uts = unsafe { &mut *(buf as *mut UtsName) };
    uts.sysname = [0; 65];
    uts.nodename = [0; 65];
    uts.release = [0; 65];
    uts.version = [0; 65];
    uts.machine = [0; 65];
    uts.domainname = [0; 65];

    fill_uts_field(&mut uts.sysname, b"Chronos");
    fill_uts_field(&mut uts.nodename, b"chronos");
    fill_uts_field(&mut uts.release, b"0.1.0");
    fill_uts_field(&mut uts.version, b"#1");
    fill_uts_field(&mut uts.machine, b"x86_64");
    fill_uts_field(&mut uts.domainname, b"");

    regs.rax = 0;
}

fn sys_chdir(regs: &mut Registers) {
    let Some(path_str) = validate_user_cstr(regs.rdi) else {
        regs.rax = -EFAULT as _;
        return;
    };

    let current = current_process().unwrap();
    let mut proc = current.lock();
    let path = resolve_path(path_str, proc.get_cwd());

    let vfs = crate::drivers::fs::get_vfs();
    let Some(node) = vfs.resolve_path(path.clone()) else {
        regs.rax = -ENOENT as _;
        return;
    };

    if !node.is_dir() {
        regs.rax = -ENOTDIR as _;
        return;
    }

    drop(vfs);
    proc.set_cwd(path);
    regs.rax = 0;
}

#[repr(C)]
struct StatBuf {
    st_dev: u64,
    st_ino: u64,
    st_nlink: u64,
    st_mode: u32,
    st_uid: u32,
    st_gid: u32,
    __pad0: u32,
    st_rdev: u64,
    st_size: i64,
    st_blksize: i64,
    st_blocks: i64,
    st_atime: i64,
    st_atime_nsec: i64,
    st_mtime: i64,
    st_mtime_nsec: i64,
    st_ctime: i64,
    st_ctime_nsec: i64,
    __unused: [i64; 3],
}

fn fill_stat(stat: &mut StatBuf, node: &dyn VfsNode) {
    let meta = node.get_metadata();
    let mode_bits = meta.permissions.bits() as u32;
    let type_bits: u32 = if node.is_dir() { 0o040000 } else { 0o100000 };

    *stat = StatBuf {
        st_dev: 0,
        st_ino: 0,
        st_nlink: 1,
        st_mode: type_bits | mode_bits,
        st_uid: 0,
        st_gid: 0,
        __pad0: 0,
        st_rdev: 0,
        st_size: node.size() as i64,
        st_blksize: 4096,
        st_blocks: ((node.size() + 511) / 512) as i64,
        st_atime: meta.modified_at as i64,
        st_atime_nsec: 0,
        st_mtime: meta.modified_at as i64,
        st_mtime_nsec: 0,
        st_ctime: meta.created_at as i64,
        st_ctime_nsec: 0,
        __unused: [0; 3],
    };
}

fn sys_stat(regs: &mut Registers) {
    let Some(path_str) = validate_user_cstr(regs.rdi) else {
        regs.rax = -EFAULT as _;
        return;
    };

    let stat_buf = regs.rsi;
    if !validate_user_buf(stat_buf, core::mem::size_of::<StatBuf>() as _) {
        regs.rax = -EFAULT as _;
        return;
    }

    let current = current_process().unwrap();
    let proc = current.lock();
    let path = resolve_path(path_str, proc.get_cwd());
    drop(proc);

    let vfs = crate::drivers::fs::get_vfs();
    let Some(node) = vfs.resolve_path(path) else {
        regs.rax = -ENOENT as _;
        return;
    };

    let stat = unsafe { &mut *(stat_buf as *mut StatBuf) };
    fill_stat(stat, node);
    regs.rax = 0;
}

fn sys_fstat(regs: &mut Registers) {
    let fd = regs.rdi as i32;
    let stat_buf = regs.rsi;

    if !validate_user_buf(stat_buf, core::mem::size_of::<StatBuf>() as _) {
        regs.rax = -EFAULT as _;
        return;
    }

    let current = current_process().unwrap();
    let lock = current.lock();
    let Some(file) = lock.fds.get(&fd) else {
        regs.rax = -EBADF as _;
        return;
    };

    let stat = unsafe { &mut *(stat_buf as *mut StatBuf) };
    fill_stat(stat, file.node());
    regs.rax = 0;
}

fn sys_unlink(regs: &mut Registers) {
    let Some(path_str) = validate_user_cstr(regs.rdi) else {
        regs.rax = -EFAULT as _;
        return;
    };

    let current = current_process().unwrap();
    let path = resolve_path(path_str, current.lock().get_cwd());

    let mut vfs = crate::drivers::fs::get_vfs();

    let Some(node) = vfs.resolve_path(path.clone()) else {
        regs.rax = -ENOENT as _;
        return;
    };

    if node.is_dir() {
        regs.rax = -EISDIR as _;
        return;
    }

    let name = path.get_name().to_string();
    let Some(parent) = vfs.resolve_path_mut(path.get_parent()) else {
        regs.rax = -ENOENT as _;
        return;
    };

    if parent.remove_child(&name) {
        regs.rax = 0;
    } else {
        regs.rax = -ENOENT as _;
    }
}

fn sys_rmdir(regs: &mut Registers) {
    let Some(path_str) = validate_user_cstr(regs.rdi) else {
        regs.rax = -EFAULT as _;
        return;
    };

    let current = current_process().unwrap();
    let path = resolve_path(path_str, current.lock().get_cwd());

    let mut vfs = crate::drivers::fs::get_vfs();

    let Some(node) = vfs.resolve_path(path.clone()) else {
        regs.rax = -ENOENT as _;
        return;
    };

    if !node.is_dir() {
        regs.rax = -ENOTDIR as _;
        return;
    }

    if !node.get_children().is_empty() {
        regs.rax = -ENOTEMPTY as _;
        return;
    }

    let name = path.get_name().to_string();
    let Some(parent) = vfs.resolve_path_mut(path.get_parent()) else {
        regs.rax = -ENOENT as _;
        return;
    };

    if parent.remove_child(&name) {
        regs.rax = 0;
    } else {
        regs.rax = -ENOENT as _;
    }
}

fn sys_dup(regs: &mut Registers) {
    let old_fd = regs.rdi as i32;

    let current = current_process().unwrap();
    let mut proc = current.lock();

    let Some(file) = proc.fds.get(&old_fd) else {
        regs.rax = -EBADF as _;
        return;
    };

    let dup_fd = file.dup();
    let new_fd = proc.next_fd.fetch_add(1, Ordering::SeqCst);
    proc.fds.insert(new_fd, dup_fd);
    regs.rax = new_fd as u64;
}

fn sys_dup2(regs: &mut Registers) {
    let old_fd = regs.rdi as i32;
    let new_fd = regs.rsi as i32;

    let current = current_process().unwrap();
    let mut proc = current.lock();

    if !proc.fds.contains_key(&old_fd) {
        regs.rax = -EBADF as _;
        return;
    }

    if old_fd == new_fd {
        regs.rax = new_fd as u64;
        return;
    }

    let dup_fd = proc.fds.get(&old_fd).unwrap().dup();

    proc.fds.remove(&new_fd);
    proc.fds.insert(new_fd, dup_fd);
    regs.rax = new_fd as u64;
}

fn sys_ftruncate(regs: &mut Registers) {
    let fd = regs.rdi as i32;
    let length = regs.rsi as i64;

    if length < 0 {
        regs.rax = -EINVAL as _;
        return;
    }

    let current = current_process().unwrap();
    let mut lock = current.lock();
    let Some(file) = lock.fds.get_mut(&fd) else {
        regs.rax = -EBADF as _;
        return;
    };

    if !file.permissions.contains(Permissions::WRITE) {
        regs.rax = -EBADF as _;
        return;
    }

    if file.node().is_dir() {
        regs.rax = -EISDIR as _;
        return;
    }

    if file.node_mut().truncate(length as u64) {
        regs.rax = 0;
    } else {
        regs.rax = -EIO as _;
    }
}

#[repr(C)]
struct LinuxDirent64 {
    d_ino: u64,
    d_off: i64,
    d_reclen: u16,
    d_type: u8,
}

fn sys_getdents64(regs: &mut Registers) {
    let fd = regs.rdi as i32;
    let buf = regs.rsi;
    let count = regs.rdx;

    if !validate_user_buf(buf, count) {
        regs.rax = -EFAULT as _;
        return;
    }

    let current = current_process().unwrap();
    let mut lock = current.lock();
    let Some(file) = lock.fds.get_mut(&fd) else {
        regs.rax = -EBADF as _;
        return;
    };

    if !file.node().is_dir() {
        regs.rax = -ENOTDIR as _;
        return;
    }

    let children = file.node().get_children();
    let mut offset = file.offset as usize;
    let mut written: usize = 0;

    while offset < children.len() {
        let child = children[offset];
        let name = child.get_name().as_bytes();
        let reclen = ((19 + name.len() + 1) + 7) & !7;

        if written + reclen > count as usize {
            break;
        }

        let entry_ptr = (buf + written as u64) as *mut u8;
        let d_type: u8 = if child.is_dir() { 4 } else { 8 };

        unsafe {
            let entry = entry_ptr as *mut LinuxDirent64;
            (*entry).d_ino = (offset + 1) as u64;
            (*entry).d_off = (offset + 1) as i64;
            (*entry).d_reclen = reclen as u16;
            (*entry).d_type = d_type;

            let name_dst = entry_ptr.add(19);
            core::ptr::copy_nonoverlapping(name.as_ptr(), name_dst, name.len());
            *name_dst.add(name.len()) = 0;

            let padding_start = 19 + name.len() + 1;
            for i in padding_start..reclen {
                *entry_ptr.add(i) = 0;
            }
        }

        written += reclen;
        offset += 1;
    }

    file.offset = offset as u64;
    regs.rax = written as u64;
}

fn sys_gettid(regs: &mut Registers) {
    let Some(thread) = crate::scheduler::thread::current_thread() else {
        regs.rax = 0;
        return;
    };
    regs.rax = thread.lock().gtid;
}

fn sys_access(regs: &mut Registers) {
    let Some(path_str) = validate_user_cstr(regs.rdi) else {
        regs.rax = -EFAULT as _;
        return;
    };

    let current = current_process().unwrap();
    let proc = current.lock();
    let path = resolve_path(path_str, proc.get_cwd());
    drop(proc);

    let vfs = crate::drivers::fs::get_vfs();
    if vfs.resolve_path(path).is_some() {
        regs.rax = 0;
    } else {
        regs.rax = -ENOENT as _;
    }
}

fn sys_rename(regs: &mut Registers) {
    let Some(old_str) = validate_user_cstr(regs.rdi) else {
        regs.rax = -EFAULT as _;
        return;
    };
    let Some(new_str) = validate_user_cstr(regs.rsi) else {
        regs.rax = -EFAULT as _;
        return;
    };

    let current = current_process().unwrap();
    let proc = current.lock();
    let old_path = resolve_path(old_str, proc.get_cwd());
    let new_path = resolve_path(new_str, proc.get_cwd());
    drop(proc);

    let mut vfs = crate::drivers::fs::get_vfs();

    if vfs.resolve_path(old_path.clone()).is_none() {
        regs.rax = -ENOENT as _;
        return;
    }

    if vfs.resolve_path(new_path.get_parent()).is_none() {
        regs.rax = -ENOENT as _;
        return;
    }

    if let Some(existing) = vfs.resolve_path(new_path.clone()) {
        if existing.is_dir() && !existing.get_children().is_empty() {
            regs.rax = -ENOTEMPTY as _;
            return;
        }
    }

    let old_name = old_path.get_name().to_string();
    let old_parent = vfs.resolve_path_mut(old_path.get_parent());
    let Some(parent) = old_parent else {
        regs.rax = -ENOENT as _;
        return;
    };
    let Some(mut child) = parent.take_child(&old_name) else {
        regs.rax = -ENOENT as _;
        return;
    };

    child.set_path(new_path.clone());

    let new_name = new_path.get_name().to_string();
    let Some(new_parent) = vfs.resolve_path_mut(new_path.get_parent()) else {
        regs.rax = -ENOENT as _;
        return;
    };

    new_parent.remove_child(&new_name);

    if new_parent.add_child(child) {
        regs.rax = 0;
    } else {
        regs.rax = -EEXIST as _;
    }
}

const CLOCK_REALTIME: u64 = 0;
const CLOCK_MONOTONIC: u64 = 1;
const CLOCK_BOOTTIME: u64 = 7;

fn sys_clock_gettime(regs: &mut Registers) {
    let clock_id = regs.rdi;
    let tp = regs.rsi;

    if !validate_user_buf(tp, core::mem::size_of::<Timespec>() as _) {
        regs.rax = -EFAULT as _;
        return;
    }

    let ts = unsafe { &mut *(tp as *mut Timespec) };

    match clock_id {
        CLOCK_REALTIME => {
            let epoch = read_rtc().to_epoch().unwrap_or(0);
            ts.tv_sec = epoch as i64;
            ts.tv_nsec = 0;
        }
        CLOCK_MONOTONIC | CLOCK_BOOTTIME => {
            let ns = crate::arch::drivers::time::preferred_timer_ns();
            ts.tv_sec = (ns / 1_000_000_000) as i64;
            ts.tv_nsec = (ns % 1_000_000_000) as i64;
        }
        _ => {
            regs.rax = -EINVAL as _;
            return;
        }
    }

    regs.rax = 0;
}

pub fn init() {
    HANDLERS[SyscallId::Read as usize].store(sys_read as _, Ordering::Release);
    HANDLERS[SyscallId::Write as usize].store(sys_write as _, Ordering::Release);
    HANDLERS[SyscallId::Open as usize].store(sys_open as _, Ordering::Release);
    HANDLERS[SyscallId::Close as usize].store(sys_close as _, Ordering::Release);
    HANDLERS[SyscallId::Stat as usize].store(sys_stat as _, Ordering::Release);
    HANDLERS[SyscallId::Fstat as usize].store(sys_fstat as _, Ordering::Release);
    HANDLERS[SyscallId::Lseek as usize].store(sys_lseek as _, Ordering::Release);
    HANDLERS[SyscallId::Dup as usize].store(sys_dup as _, Ordering::Release);
    HANDLERS[SyscallId::Dup2 as usize].store(sys_dup2 as _, Ordering::Release);
    HANDLERS[SyscallId::Getpid as usize].store(sys_getpid as _, Ordering::Release);
    HANDLERS[SyscallId::Ftruncate as usize].store(sys_ftruncate as _, Ordering::Release);
    HANDLERS[SyscallId::Getdents64 as usize].store(sys_getdents64 as _, Ordering::Release);
    HANDLERS[SyscallId::Getcwd as usize].store(sys_get_cwd as _, Ordering::Release);
    HANDLERS[SyscallId::Chdir as usize].store(sys_chdir as _, Ordering::Release);
    HANDLERS[SyscallId::Mkdir as usize].store(sys_mkdir as _, Ordering::Release);
    HANDLERS[SyscallId::Rmdir as usize].store(sys_rmdir as _, Ordering::Release);
    HANDLERS[SyscallId::Unlink as usize].store(sys_unlink as _, Ordering::Release);
    HANDLERS[SyscallId::Uname as usize].store(sys_uname as _, Ordering::Release);
    HANDLERS[SyscallId::Getppid as usize].store(sys_getppid as _, Ordering::Release);
    HANDLERS[SyscallId::Access as usize].store(sys_access as _, Ordering::Release);
    HANDLERS[SyscallId::Rename as usize].store(sys_rename as _, Ordering::Release);
    HANDLERS[SyscallId::Gettid as usize].store(sys_gettid as _, Ordering::Release);
    HANDLERS[SyscallId::ClockGettime as usize].store(sys_clock_gettime as _, Ordering::Release);
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
