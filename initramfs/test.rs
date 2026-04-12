#![no_std]
#![no_main]

use core::{arch::asm, fmt::Write};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    println!("Hello from the test process!");
    sleep_ms(1000);
    println!("Goodbye from the test process!");
    panic!("This is a test panic!");
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    sys_exit(1);
}

#[inline(always)]
fn sys_exit(code: u64) -> ! {
    syscall!(SyscallId::Exit, code);
    sys_yield();
    unsafe { core::hint::unreachable_unchecked() }
}

#[inline(always)]
fn sys_sleep(ns: u64) {
    syscall!(SyscallId::Nanosleep, ns);
}

#[inline(always)]
fn sleep_us(us: u64) {
    sys_sleep(us * 1_000);
}

#[inline(always)]
fn sleep_ms(ms: u64) {
    sys_sleep(ms * 1_000_000);
}

#[inline(always)]
fn sys_yield() {
    syscall!(SyscallId::SchedYield);
}

#[inline(always)]
fn sys_write(fd: i32, buf: *const u8, count: usize) -> isize {
    syscall!(SyscallId::Write, fd, buf, count) as isize
}

pub struct Writer;

impl Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        sys_write(1, s.as_ptr(), s.len());
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print(args: ::core::fmt::Arguments) {
    write!(Writer, "{args}").ok();
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        $crate::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($fmt:expr) => ($crate::print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::print!(
        concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! syscall {
    ($id:expr) => {{
        let ret: u64;
        unsafe {
            asm!(
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
            asm!(
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
            asm!(
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
            asm!(
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
            asm!(
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
            asm!(
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
            asm!(
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
