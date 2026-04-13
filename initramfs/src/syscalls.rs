macro_rules! syscall {
    ($id:expr) => {{
        let ret: u64;
        unsafe {
            ::core::arch::asm!(
                "syscall",
                in("rax") $id as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
    ($id:expr, $a0:expr) => {{
        let ret: u64;
        unsafe {
            ::core::arch::asm!(
                "syscall",
                in("rax") $id as u64,
                in("rdi") $a0 as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
    ($id:expr, $a0:expr, $a1:expr) => {{
        let ret: u64;
        unsafe {
            ::core::arch::asm!(
                "syscall",
                in("rax") $id as u64,
                in("rdi") $a0 as u64,
                in("rsi") $a1 as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
    ($id:expr, $a0:expr, $a1:expr, $a2:expr) => {{
        let ret: u64;
        unsafe {
            ::core::arch::asm!(
                "syscall",
                in("rax") $id as u64,
                in("rdi") $a0 as u64,
                in("rsi") $a1 as u64,
                in("rdx") $a2 as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
    ($id:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr) => {{
        let ret: u64;
        unsafe {
            ::core::arch::asm!(
                "syscall",
                in("rax") $id as u64,
                in("rdi") $a0 as u64,
                in("rsi") $a1 as u64,
                in("rdx") $a2 as u64,
                in("r10") $a3 as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
    ($id:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr) => {{
        let ret: u64;
        unsafe {
            ::core::arch::asm!(
                "syscall",
                in("rax") $id as u64,
                in("rdi") $a0 as u64,
                in("rsi") $a1 as u64,
                in("rdx") $a2 as u64,
                in("r10") $a3 as u64,
                in("r8")  $a4 as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
    ($id:expr, $a0:expr, $a1:expr, $a2:expr, $a3:expr, $a4:expr, $a5:expr) => {{
        let ret: u64;
        unsafe {
            ::core::arch::asm!(
                "syscall",
                in("rax") $id as u64,
                in("rdi") $a0 as u64,
                in("rsi") $a1 as u64,
                in("rdx") $a2 as u64,
                in("r10") $a3 as u64,
                in("r8")  $a4 as u64,
                in("r9")  $a5 as u64,
                lateout("rax") ret,
            );
        }
        ret
    }};
}

#[inline(always)]
pub fn sys_open(path: *const core::ffi::c_char, flags: i32, mode: i32) -> i32 {
    syscall!(SyscallId::Open, path, flags, mode) as i32
}

#[inline(always)]
pub fn sys_close(fd: i32) -> i32 {
    syscall!(SyscallId::Close, fd) as i32
}

#[inline(always)]
pub fn sys_read(fd: i32, buf: *mut u8, count: usize) -> isize {
    syscall!(SyscallId::Read, fd, buf, count) as isize
}

#[inline(always)]
pub fn sys_write(fd: i32, buf: *const u8, count: usize) -> isize {
    syscall!(SyscallId::Write, fd, buf, count) as isize
}

#[inline(always)]
pub fn sys_mkdir(path: *const core::ffi::c_char, mode: u32) -> i32 {
    syscall!(SyscallId::Mkdir, path, mode) as i32
}

#[inline(always)]
pub fn sys_lseek(fd: i32, offset: i64, whence: i32) -> i64 {
    syscall!(SyscallId::Lseek, fd, offset, whence) as i64
}

#[inline(always)]
pub fn sys_get_cwd(buf: *mut u8, count: usize) -> *mut u8 {
    syscall!(SyscallId::Getcwd, buf, count) as *mut u8
}

#[inline(always)]
pub fn sys_exit(code: u64) -> ! {
    syscall!(SyscallId::Exit, code);
    sys_yield();
    unsafe { core::hint::unreachable_unchecked() }
}

#[repr(C)]
pub struct Timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

#[inline(always)]
pub fn sys_nanosleep(req: *const Timespec) -> i32 {
    syscall!(SyscallId::Nanosleep, req) as i32
}

#[inline(always)]
pub fn sleep_ns(ns: u64) {
    let req = Timespec {
        tv_sec: (ns / 1_000_000_000) as i64,
        tv_nsec: (ns % 1_000_000_000) as i64,
    };
    sys_nanosleep(&raw const req);
}

#[inline(always)]
pub fn sleep_us(us: u64) {
    sleep_ns(us * 1_000);
}

#[inline(always)]
pub fn sleep_ms(ms: u64) {
    sleep_ns(ms * 1_000_000);
}

#[inline(always)]
pub fn sys_yield() {
    syscall!(SyscallId::SchedYield);
}

#[inline(always)]
pub fn sys_getpid() -> u64 {
    syscall!(SyscallId::Getpid)
}

#[inline(always)]
pub fn sys_getppid() -> u64 {
    syscall!(SyscallId::Getppid)
}

#[repr(C)]
pub struct UtsName {
    pub sysname: [u8; 65],
    pub nodename: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domainname: [u8; 65],
}

#[inline(always)]
pub fn sys_uname(buf: *mut UtsName) -> i32 {
    syscall!(SyscallId::Uname, buf) as i32
}

#[inline(always)]
pub fn sys_chdir(path: *const core::ffi::c_char) -> i32 {
    syscall!(SyscallId::Chdir, path) as i32
}

#[repr(C)]
pub struct StatBuf {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atime: i64,
    pub st_atime_nsec: i64,
    pub st_mtime: i64,
    pub st_mtime_nsec: i64,
    pub st_ctime: i64,
    pub st_ctime_nsec: i64,
    pub __unused: [i64; 3],
}

#[inline(always)]
pub fn sys_stat(path: *const core::ffi::c_char, buf: *mut StatBuf) -> i32 {
    syscall!(SyscallId::Stat, path, buf) as i32
}

#[inline(always)]
pub fn sys_fstat(fd: i32, buf: *mut StatBuf) -> i32 {
    syscall!(SyscallId::Fstat, fd, buf) as i32
}

#[inline(always)]
pub fn sys_unlink(path: *const core::ffi::c_char) -> i32 {
    syscall!(SyscallId::Unlink, path) as i32
}

#[inline(always)]
pub fn sys_rmdir(path: *const core::ffi::c_char) -> i32 {
    syscall!(SyscallId::Rmdir, path) as i32
}

#[inline(always)]
pub fn sys_dup(fd: i32) -> i32 {
    syscall!(SyscallId::Dup, fd) as i32
}

#[inline(always)]
pub fn sys_dup2(old_fd: i32, new_fd: i32) -> i32 {
    syscall!(SyscallId::Dup2, old_fd, new_fd) as i32
}

#[inline(always)]
pub fn sys_ftruncate(fd: i32, length: i64) -> i32 {
    syscall!(SyscallId::Ftruncate, fd, length) as i32
}

#[repr(C)]
pub struct LinuxDirent64 {
    pub d_ino: u64,
    pub d_off: i64,
    pub d_reclen: u16,
    pub d_type: u8,
}

#[inline(always)]
pub fn sys_getdents64(fd: i32, buf: *mut u8, count: usize) -> isize {
    syscall!(SyscallId::Getdents64, fd, buf, count) as isize
}

#[inline(always)]
pub fn sys_gettid() -> u64 {
    syscall!(SyscallId::Gettid)
}

#[inline(always)]
pub fn sys_access(path: *const core::ffi::c_char, _mode: i32) -> i32 {
    syscall!(SyscallId::Access, path, _mode) as i32
}

#[inline(always)]
pub fn sys_rename(oldpath: *const core::ffi::c_char, newpath: *const core::ffi::c_char) -> i32 {
    syscall!(SyscallId::Rename, oldpath, newpath) as i32
}

#[inline(always)]
pub fn sys_clock_gettime(clock_id: u64, tp: *mut Timespec) -> i32 {
    syscall!(SyscallId::ClockGettime, clock_id, tp) as i32
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
