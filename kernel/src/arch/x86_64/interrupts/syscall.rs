use core::{
    alloc::Layout,
    sync::atomic::{AtomicPtr, Ordering},
};

use alloc::boxed::Box;

use crate::{
    arch::interrupts::StackFrame,
    drivers::fs::{FileDescriptor, Path},
    info,
    memory::KERNEL_STACK_SIZE,
    print,
    scheduler::current_process,
    utils::asm::regs::{rdmsr, wrmsr},
};

#[repr(C)]
struct SyscallCpuData {
    _reserved: u64,
    user_rsp: u64,
    kernel_rsp: u64,
}

core::arch::global_asm! {
    r#"
.extern syscall_handler
syscall_entry:
    swapgs

    mov gs:[8], rsp
    mov rsp, gs:[16]

    push 0x23 # ss
    push gs:[8] # rsp
    push r11 # rflags
    push 0x2B # cs
    push rcx # rip
    push 0 # error_code
    push 0 # vector

    push rax
    push rbx
    push rcx
    push rdx
    push rsi
    push rdi
    push rbp
    push r8
    push r9
    push r10
    push r11
    push r12
    push r13
    push r14
    push r15

    sti
    mov rdi, rsp
    call syscall_handler
    cli

    pop r15
    pop r14
    pop r13
    pop r12
    pop r11
    pop r10
    pop r9
    pop r8
    pop rbp
    pop rdi
    pop rsi
    pop rdx
    pop rcx
    pop rbx
    pop rax

    add rsp, 16

    pop rcx
    add rsp, 8
    pop r11
    pop rsp

    swapgs
    sysretq
.global syscall_entry
    "#
}

#[repr(u64)]
pub enum SyscallId {
    Read,
    Write,
    Open,
    Close,
    Stat,
    Fstat,
    Lstat,
    Poll,
    Lseek,
    Mmap,
    Mprotect,
    Munmap,
    Brk,
    RtSigaction,
    RtSigprocmask,
    RtSigreturn,
    Ioctl,
    Pread64,
    Pwrite64,
    Readv,
    Writev,
    Access,
    Pipe,
    Select,
    SchedYield,
    Mremap,
    Msync,
    MinCore,
    Madvise,
    Shmget,
    Shmat,
    Shmctl,
    Dup,
    Dup2,
    Pause,
    Nanosleep,
    Getitimer,
    Alarm,
    Setitimer,
    Getpid,
    Sendfile,
    Socket,
    Connect,
    Accept,
    Sendto,
    Recvfrom,
    Sendmsg,
    Recvmsg,
    Shutdown,
    Bind,
    Listen,
    Getsockname,
    Getpeername,
    Socketpair,
    Setsockopt,
    Getsockopt,
    Clone,
    Fork,
    Vfork,
    Execve,
    Exit,
    Wait4,
    Kill,
    Uname,
    Semget,
    Semop,
    Semctl,
    Shmdt,
    Msgget,
    Msgsnd,
    Msgrcv,
    Msgctl,
    Fcntl,
    Flock,
    Fsync,
    Fdatasync,
    Truncate,
    Ftruncate,
    Getdents,
    Getcwd,
    Chdir,
    Fchdir,
    Rename,
    Mkdir,
    Rmdir,
    Creat,
    Link,
    Unlink,
    Symlink,
    Readlink,
    Chmod,
    Fchmod,
    Chown,
    Fchown,
    Lchown,
    Umask,
    Gettimeofday,
    Getrlimit,
    Getrusage,
    Sysinfo,
    Times,
    Ptrace,
    Getuid,
    Syslog,
    Getgid,
    Setuid,
    Setgid,
    Geteuid,
    Getegid,
    Setpgid,
    Getppid,
    Getpgrp,
    Setsid,
    Setreuid,
    Setregid,
    Getgroups,
    Setgroups,
    Setresuid,
    Getresuid,
    Setresgid,
    Getresgid,
    Getpgid,
    Setfsuid,
    Setfsgid,
    Getsid,
    Capget,
    Capset,
    RtSigpending,
    RtSigtimedwait,
    RtSigqueueinfo,
    RtSigsuspend,
    Sigaltstack,
    Utime,
    Mknod,
    Uselib,
    Personality,
    Ustat,
    Statfs,
    Fstatfs,
    Sysfs,
    Getpriority,
    Setpriority,
    SchedSetparam,
    SchedGetparam,
    SchedSetscheduler,
    SchedGetscheduler,
    SchedGetPriorityMax,
    SchedGetPriorityMin,
    SchedRrGetInterval,
    Mlock,
    Munlock,
    Mlockall,
    Munlockall,
    Vhangup,
    ModifyLdt,
    PivotRoot,
    _Sysctl,
    Prctl,
    ArchPrctl,
    Adjtimex,
    Setrlimit,
    Chroot,
    Sync,
    Acct,
    Settimeofday,
    Mount,
    Umount2,
    Swapon,
    Swapoff,
    Reboot,
    Sethostname,
    Setdomainname,
    Iopl,
    Ioperm,
    CreateModule,
    InitModule,
    DeleteModule,
    GetKernelSyms,
    QueryModule,
    Quotactl,
    Nfsservctl,
    Getpmsg,
    Putpmsg,
    AfsSyscall,
    Tuxcall,
    Security,
    Gettid,
    Readahead,
    Setxattr,
    Lsetxattr,
    Fsetxattr,
    Getxattr,
    Lgetxattr,
    Fgetxattr,
    Listxattr,
    Llistxattr,
    Flistxattr,
    Removexattr,
    Lremovexattr,
    Fremovexattr,
    Tkill,
    Time,
    Futex,
    SchedSetaffinity,
    SchedGetaffinity,
    SetThreadArea,
    IoSetup,
    IoDestroy,
    IoGetevents,
    IoSubmit,
    IoCancel,
    GetThreadArea,
    LookupDcookie,
    EpollCreate,
    EpollCtlOld,
    EpollWaitOld,
    RemapFilePages,
    Getdents64,
    SetTidAddress,
    RestartSyscall,
    Semtimedop,
    Fadvise64,
    TimerCreate,
    TimerSettime,
    TimerGettime,
    TimerGetoverrun,
    TimerDelete,
    ClockSettime,
    ClockGettime,
    ClockGetres,
    ClockNanosleep,
    ExitGroup,
    EpollWait,
    EpollCtl,
    Tgkill,
    Utimes,
    Vserver,
    Mbind,
    SetMempolicy,
    GetMempolicy,
    MqOpen,
    MqUnlink,
    MqTimedsend,
    MqTimedreceive,
    MqNotify,
    MqGetsetattr,
    KexecLoad,
    Waitid,
    AddKey,
    RequestKey,
    Keyctl,
    IoprioSet,
    IoprioGet,
    InotifyInit,
    InotifyAddWatch,
    InotifyRmWatch,
    MigratePages,
    Openat,
    Mkdirat,
    Mknodat,
    Fchownat,
    Futimesat,
    Newfstatat,
    Unlinkat,
    Renameat,
    Linkat,
    Symlinkat,
    Readlinkat,
    Fchmodat,
    Faccessat,
    Pselect6,
    Ppoll,
    Unshare,
    SetRobustList,
    GetRobustList,
    Splice,
    Tee,
    SyncFileRange,
    Vmsplice,
    MovePages,
    Utimensat,
    EpollPwait,
    Signalfd,
    TimerfdCreate,
    Eventfd,
    Fallocate,
    TimerfdSettime,
    TimerfdGettime,
    Accept6,
    Signalfd4,
    Eventfd2,
    EpollCreate1,
    Dup3,
    Pip2,
    InotifyInit1,
    Preadv,
    Pwritev,
    RtTgsigqueueinfo,
    PerfEventOpen,
    Recvmmsg,
    FanotifyInit,
    FanotifyMark,
    Prlimit64,
    NameToHandleAt,
    OpenByHandleAt,
    ClockAdjtime,
    SyncFs,
    Sendmmsg,
    Setns,
    Getcpu,
    ProcessVmReadv,
    ProcessVmWritev,
    Kcmp,
    FinitModule,
    SchedSetattr,
    SchedGetattr,
    Renameat2,
    Seccomp,
    Getrandom,
    MemfdCreate,
    KexecFileLoad,
    Bpf,
    Execveat,
    Userfaultfd,
    Membarrier,
    Mlock2,
    CopyFileRange,
    Preadv2,
    Pwritev2,
    PkeyMProtect,
    PkeyAlloc,
    PkeyFree,
    Statx,
}

static HANDLERS: [AtomicPtr<()>; 333] = [const { AtomicPtr::new(sys_stub as _) }; 333];

#[unsafe(no_mangle)]
pub extern "C" fn syscall_handler(regs: &mut StackFrame) {
    let id = regs.rax;
    let handler_ptr = HANDLERS[id as usize].load(Ordering::Acquire);
    if !handler_ptr.is_null() {
        let handler: fn(&mut StackFrame) = unsafe { core::mem::transmute(handler_ptr) };
        handler(regs);
    }
}

unsafe extern "C" {
    fn syscall_entry();
}

fn sys_stub(regs: &mut StackFrame) {
    info!(
        "syscall {} with args {} {} {} {} {} {}",
        regs.rax, regs.rdi, regs.rsi, regs.rdx, regs.r10, regs.r8, regs.r9
    );
}

fn sys_exit(regs: &mut StackFrame) {
    let pid = crate::scheduler::current_process()
        .unwrap()
        .lock()
        .get_pid();
    info!("process {} exited with code {}", pid, regs.rdi);
    crate::scheduler::kill_process(pid);
}

fn sys_nanosleep(regs: &mut StackFrame) {
    crate::scheduler::thread::sleep(regs.rdi);
}

fn sys_yield(_regs: &mut StackFrame) {
    crate::scheduler::thread::yield_();
}

fn sys_read(regs: &mut StackFrame) {
    let fd = regs.rdi;
    let buf = regs.rsi as *mut u8;
    let count = regs.rdx;

    let slice = unsafe { core::slice::from_raw_parts_mut(buf, count as usize) };

    if fd < 3 {
        // TODO:
        regs.rax = -38i32 as _; // enosys
        return;
    }

    let current = current_process().unwrap();
    let mut lock = current.lock();
    let Some(file) = lock.fds.get_mut(&(fd as i32)) else {
        regs.rax = -9i32 as _; // bad file descriptor
        return;
    };
    regs.rax = if let Some(size) = file.read(slice) {
        size as _
    } else {
        -38i32 as _ // enosys
    };
}

fn sys_write(regs: &mut StackFrame) {
    let fd = regs.rdi;
    let buf = regs.rsi as *const u8;
    let count = regs.rdx;

    let slice = unsafe { core::slice::from_raw_parts(buf, count as usize) };

    if fd == 1 {
        if let Ok(s) = core::str::from_utf8(slice) {
            print!("{}", s);
        } else {
            print!("{:?}", slice);
        }
        regs.rax = count;
    } else {
        let current = current_process().unwrap();
        let mut lock = current.lock();
        let Some(file) = lock.fds.get_mut(&(fd as i32)) else {
            regs.rax = -9i32 as _; // bad file descriptor
            return;
        };
        file.write(slice);
    }
}

fn sys_open(regs: &mut StackFrame) {
    let current = current_process().unwrap();
    let mut lock = current.lock();
    let fd = lock.next_fd.fetch_add(1, Ordering::SeqCst);
    let Ok(path) = unsafe { core::ffi::CStr::from_ptr(regs.rdi as _) }.to_str() else {
        regs.rax = -14i32 as _; // efault
        return;
    };
    // TODO!
    let Some(file) = crate::drivers::fs::get_vfs().resolve_path_mut(Path::new(path)) else {
        regs.rax = -38i32 as _; // no such file or directory
        return;
    };
    lock.fds.insert(fd, FileDescriptor::new(file));
    regs.rax = fd as _;
}

fn sys_close(regs: &mut StackFrame) {
    let current = current_process().unwrap();
    let mut lock = current.lock();
    let fd = regs.rdi as i32;
    if lock.fds.remove(&fd).is_some() {
        regs.rax = 0;
    } else {
        regs.rax = -9i32 as _; // bad file descriptor
    }
}

fn sys_lseek(regs: &mut StackFrame) {
    const SEEK_SET: u64 = 0;
    const SEEK_CUR: u64 = 1;
    const SEEK_END: u64 = 2;
    let current = current_process().unwrap();
    let mut lock = current.lock();
    let fd = regs.rdi as i32;
    let offset = regs.rsi as i64;
    let whence = regs.rdx;

    let Some(file) = lock.fds.get_mut(&fd) else {
        regs.rax = -9i32 as _; // bad file descriptor
        return;
    };

    let new_pos = match whence {
        SEEK_SET => offset,
        SEEK_CUR => file.offset as i64 + offset,
        SEEK_END => file.node.size() as i64 + offset,
        _ => {
            regs.rax = -22i32 as _; // invalid argument
            return;
        }
    };

    if new_pos < 0 {
        regs.rax = -22i32 as _; // invalid argument
    }
}

pub fn sys_get_cwd(regs: &mut StackFrame) {
    let buf = regs.rdi as *mut u8;
    let count = regs.rsi;

    let slice = unsafe { core::slice::from_raw_parts_mut(buf, count as usize) };
    let current = current_process().unwrap();
    let lock = current.lock();
    let cwd = lock.get_cwd().as_str().as_bytes();

    if slice.len() < cwd.len() {
        regs.rax = -34i32 as _; // ERANGE
        return;
    }

    slice[..cwd.len()].copy_from_slice(cwd);
    regs.rax = cwd.len() as _;
}

pub fn sys_mkdir(regs: &mut StackFrame) {
    let path = regs.rdi as *const core::ffi::c_char;
    let Ok(path) = unsafe { core::ffi::CStr::from_ptr(path) }.to_str() else {
        regs.rax = -14i32 as _; // efault
        return;
    };

    let path = Path::new(path);

    if let Some(node) = crate::drivers::fs::get_vfs().resolve_path_mut(path.get_parent()) {
        if node.create_dir(path.get_name()).is_some() {
            regs.rax = 0;
        } else {
            regs.rax = -38i32 as _; // enosys
        }
    } else {
        regs.rax = -38i32 as _; // enosys
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
    wrmsr(0xC0000102, cpu_data as u64);
}
