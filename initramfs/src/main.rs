#![no_std]
#![no_main]
#![allow(clippy::macro_metavars_in_unsafe)]

use core::{ffi::c_char, fmt::Write};

use crate::syscalls::{
    LinuxDirent64, StatBuf, Timespec, UtsName, sys_access, sys_chdir, sys_clock_gettime, sys_close,
    sys_dup, sys_dup2, sys_execve, sys_exit, sys_fork, sys_fstat, sys_ftruncate, sys_get_cwd,
    sys_getdents64, sys_getpid, sys_getppid, sys_gettid, sys_lseek, sys_mkdir, sys_nanosleep,
    sys_open, sys_read, sys_rename, sys_rmdir, sys_stat, sys_uname, sys_unlink, sys_waitpid,
    sys_write, sys_yield,
};

pub mod syscalls;

pub const O_RDONLY: i32 = 0x00;
pub const O_WRONLY: i32 = 0x01;
pub const O_RDWR: i32 = 0x02;

pub const POSIX_FADV_DONTNEED: i32 = 4;
pub const POSIX_FADV_NOREUSE: i32 = 5;

pub const VEOF: usize = 4;
pub const RTLD_DEEPBIND: i32 = 0x8;
pub const RTLD_GLOBAL: i32 = 0x100;
pub const RTLD_NOLOAD: i32 = 0x4;

pub const O_APPEND: i32 = 1024;
pub const O_CREAT: i32 = 64;
pub const O_EXCL: i32 = 128;
pub const O_NOCTTY: i32 = 256;
pub const O_NONBLOCK: i32 = 2048;
pub const O_SYNC: i32 = 1052672;
pub const O_RSYNC: i32 = 1052672;
pub const O_DSYNC: i32 = 4096;
pub const O_FSYNC: i32 = 0x101000;
pub const O_TRUNC: i32 = 0x200;
pub const SEEK_SET: i32 = 0;
pub const SEEK_CUR: i32 = 1;
pub const SEEK_END: i32 = 2;
pub const O_NOATIME: i32 = 0o1000000;
pub const O_PATH: i32 = 0o10000000;
pub const O_DIRECTORY: i32 = 0x10000;
pub const O_TMPFILE: i32 = 0o20000000 | O_DIRECTORY;

pub const MADV_SOFT_OFFLINE: i32 = 101;
pub const MAP_GROWSDOWN: i32 = 0x0100;

pub const EDEADLK: i32 = 35;
pub const ENAMETOOLONG: i32 = 36;
pub const ENOLCK: i32 = 37;
pub const ENOSYS: i32 = 38;
pub const ENOTEMPTY: i32 = 39;
pub const ELOOP: i32 = 40;
pub const ENOMSG: i32 = 42;
pub const EIDRM: i32 = 43;
pub const ECHRNG: i32 = 44;
pub const EL2NSYNC: i32 = 45;
pub const EL3HLT: i32 = 46;
pub const EL3RST: i32 = 47;
pub const ELNRNG: i32 = 48;
pub const EUNATCH: i32 = 49;
pub const ENOCSI: i32 = 50;
pub const EL2HLT: i32 = 51;
pub const EBADE: i32 = 52;
pub const EBADR: i32 = 53;
pub const EXFULL: i32 = 54;
pub const ENOANO: i32 = 55;
pub const EBADRQC: i32 = 56;
pub const EBADSLT: i32 = 57;
pub const EMULTIHOP: i32 = 72;
pub const EOVERFLOW: i32 = 75;
pub const ENOTUNIQ: i32 = 76;
pub const EBADFD: i32 = 77;
pub const EBADMSG: i32 = 74;
pub const EREMCHG: i32 = 78;
pub const ELIBACC: i32 = 79;
pub const ELIBBAD: i32 = 80;
pub const ELIBSCN: i32 = 81;
pub const ELIBMAX: i32 = 82;
pub const ELIBEXEC: i32 = 83;
pub const EILSEQ: i32 = 84;
pub const ERESTART: i32 = 85;
pub const ESTRPIPE: i32 = 86;
pub const EUSERS: i32 = 87;
pub const ENOTSOCK: i32 = 88;
pub const EDESTADDRREQ: i32 = 89;
pub const EMSGSIZE: i32 = 90;
pub const EPROTOTYPE: i32 = 91;
pub const ENOPROTOOPT: i32 = 92;
pub const EPROTONOSUPPORT: i32 = 93;
pub const ESOCKTNOSUPPORT: i32 = 94;
pub const EOPNOTSUPP: i32 = 95;
pub const EPFNOSUPPORT: i32 = 96;
pub const EAFNOSUPPORT: i32 = 97;
pub const EADDRINUSE: i32 = 98;
pub const EADDRNOTAVAIL: i32 = 99;
pub const ENETDOWN: i32 = 100;
pub const ENETUNREACH: i32 = 101;
pub const ENETRESET: i32 = 102;
pub const ECONNABORTED: i32 = 103;
pub const ECONNRESET: i32 = 104;
pub const ENOBUFS: i32 = 105;
pub const EISCONN: i32 = 106;
pub const ENOTCONN: i32 = 107;
pub const ESHUTDOWN: i32 = 108;
pub const ETOOMANYREFS: i32 = 109;
pub const ETIMEDOUT: i32 = 110;
pub const ECONNREFUSED: i32 = 111;
pub const EHOSTDOWN: i32 = 112;
pub const EHOSTUNREACH: i32 = 113;
pub const EALREADY: i32 = 114;
pub const EINPROGRESS: i32 = 115;
pub const ESTALE: i32 = 116;
pub const EDQUOT: i32 = 122;
pub const ENOMEDIUM: i32 = 123;
pub const EMEDIUMTYPE: i32 = 124;
pub const ECANCELED: i32 = 125;
pub const ENOKEY: i32 = 126;
pub const EKEYEXPIRED: i32 = 127;
pub const EKEYREVOKED: i32 = 128;
pub const EKEYREJECTED: i32 = 129;
pub const EOWNERDEAD: i32 = 130;
pub const ENOTRECOVERABLE: i32 = 131;
pub const EHWPOISON: i32 = 133;
pub const ERFKILL: i32 = 132;

static mut PASSED: u32 = 0;
static mut FAILED: u32 = 0;

fn check(name: &str, ok: bool, detail: &str) {
    if ok {
        println!("  [PASS] {}", name);
        unsafe { PASSED += 1 };
    } else {
        println!("  [FAIL] {} -- {}", name, detail);
        unsafe { FAILED += 1 };
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start(_argc: i32, _argv: *const *const c_char) -> ! {
    println!("=== syscall tests ===\n");

    test_getcwd();
    test_write();
    test_mkdir();
    test_open_close();
    test_read();
    test_write_file();
    test_lseek();
    test_close_errors();
    test_permissions();
    test_relative_paths();
    test_nanosleep();
    test_yield();
    test_getpid();
    test_uname();
    test_chdir();
    test_stat();
    test_fstat();
    test_unlink();
    test_rmdir();
    test_dup();
    test_dup2();
    test_ftruncate();
    test_getdents64();
    test_gettid();
    test_access();
    test_rename();
    test_clock_gettime();
    test_fork();
    test_fork_wait();
    test_execve();

    let (passed, failed) = unsafe { (PASSED, FAILED) };
    println!("\n=== results: {} passed, {} failed ===", passed, failed);

    sys_exit(if failed > 0 { 1 } else { 0 });
}

fn test_getcwd() {
    println!("[getcwd]");
    let buf = [0u8; 256];
    let ptr = sys_get_cwd(buf.as_ptr() as _, buf.len());
    let ok = (ptr as isize) > 0;
    check("returns valid pointer", ok, "got negative/null return");

    if ok {
        let cwd = unsafe { core::ffi::CStr::from_ptr(ptr as *const core::ffi::c_char) };
        let s = cwd.to_str().unwrap_or("");
        check("cwd is absolute path", s.starts_with('/'), s);
        check("cwd is valid utf-8", cwd.to_str().is_ok(), "");
    }

    let tiny = [0u8; 1];
    let ptr2 = sys_get_cwd(tiny.as_ptr() as _, tiny.len());
    check(
        "ERANGE on tiny buffer",
        (ptr2 as i64) == -34,
        fmt_i64(ptr2 as i64),
    );
}

fn test_write() {
    println!("[write]");
    let msg = b"write test ok\n";
    let n = sys_write(1, msg.as_ptr(), msg.len());
    check("write to stdout", n == msg.len() as isize, fmt_isize(n));

    let n2 = sys_write(2, b"".as_ptr(), 0);
    check("write 0 bytes to stderr", n2 == 0, fmt_isize(n2));

    let n3 = sys_write(0, b"x".as_ptr(), 1);
    check("EBADF writing to stdin", n3 == -9, fmt_isize(n3));
}

fn test_mkdir() {
    println!("[mkdir]");
    let r = sys_mkdir(c"/tmp".as_ptr() as _, 0o755);
    check("mkdir /tmp", r == 0, fmt_i32(r));

    let r2 = sys_mkdir(c"/tmp".as_ptr() as _, 0o755);
    check("EEXIST on duplicate mkdir", r2 == -17, fmt_i32(r2));

    let r3 = sys_mkdir(c"/no/such/path".as_ptr() as _, 0o755);
    check("ENOENT on bad parent", r3 == -2, fmt_i32(r3));
}

fn test_open_close() {
    println!("[open/close]");

    let fd = sys_open(c"/src/main.rs".as_ptr() as _, O_RDONLY, 0);
    check("open existing file", fd >= 3, fmt_i32(fd));
    if fd >= 0 {
        let r = sys_close(fd);
        check("close valid fd", r == 0, fmt_i32(r));
    }

    let fd2 = sys_open(c"/nonexistent".as_ptr() as _, O_RDONLY, 0);
    check("ENOENT on missing file", fd2 == -2, fmt_i32(fd2));

    let fd3 = sys_open(c"/tmp/hello.txt".as_ptr() as _, O_WRONLY | O_CREAT, 0o644);
    check("create new file", fd3 >= 3, fmt_i32(fd3));
    if fd3 >= 0 {
        sys_close(fd3);
    }

    let fd4 = sys_open(
        c"/tmp/hello.txt".as_ptr() as _,
        O_WRONLY | O_CREAT | O_EXCL,
        0o644,
    );
    check("EEXIST with O_EXCL", fd4 == -17, fmt_i32(fd4));
}

fn test_read() {
    println!("[read]");

    let fd = sys_open(c"/src/main.rs".as_ptr() as _, O_RDONLY, 0);
    if fd < 0 {
        check("open for read", false, fmt_i32(fd));
        return;
    }

    let mut buf = [0u8; 64];
    let n = sys_read(fd, buf.as_mut_ptr(), buf.len());
    check("read returns bytes", n > 0, fmt_isize(n));
    check(
        "read content starts with #!",
        n > 0 && buf[0] == b'#' && buf[1] == b'!',
        "unexpected content",
    );

    let n2 = sys_read(fd, buf.as_mut_ptr(), buf.len());
    check("sequential read advances", n2 >= 0, fmt_isize(n2));

    sys_close(fd);

    let n3 = sys_read(fd, buf.as_mut_ptr(), buf.len());
    check("EBADF on closed fd", n3 == -9, fmt_isize(n3));

    let n4 = sys_read(1, buf.as_mut_ptr(), 1);
    check("EBADF reading stdout", n4 == -9, fmt_isize(n4));
}

fn test_write_file() {
    println!("[write to file]");

    let fd = sys_open(
        c"/tmp/write_test.txt".as_ptr() as _,
        O_RDWR | O_CREAT | O_TRUNC,
        0o644,
    );
    if fd < 0 {
        check("create file for writing", false, fmt_i32(fd));
        return;
    }

    let data = b"hello chronos";
    let n = sys_write(fd, data.as_ptr(), data.len());
    check(
        "write returns correct count",
        n == data.len() as isize,
        fmt_isize(n),
    );
    sys_close(fd);

    let fd2 = sys_open(c"/tmp/write_test.txt".as_ptr() as _, O_RDONLY, 0);
    if fd2 < 0 {
        check("reopen for verify", false, fmt_i32(fd2));
        return;
    }
    let mut buf = [0u8; 64];
    let n2 = sys_read(fd2, buf.as_mut_ptr(), buf.len());
    check(
        "readback matches write",
        n2 == data.len() as isize && &buf[..n2 as usize] == data,
        fmt_isize(n2),
    );
    sys_close(fd2);
}

fn test_lseek() {
    println!("[lseek]");

    let fd = sys_open(c"/tmp/write_test.txt".as_ptr() as _, O_RDONLY, 0);
    if fd < 0 {
        check("open for lseek", false, fmt_i32(fd));
        return;
    }

    let pos = sys_lseek(fd, 0, SEEK_SET);
    check("SEEK_SET 0", pos == 0, fmt_i64(pos));

    let pos = sys_lseek(fd, 6, SEEK_SET);
    check("SEEK_SET 6", pos == 6, fmt_i64(pos));

    let mut buf = [0u8; 7];
    let n = sys_read(fd, buf.as_mut_ptr(), buf.len());
    check(
        "read after seek gives 'chronos'",
        n == 7 && &buf == b"chronos",
        core::str::from_utf8(&buf[..n.max(0) as usize]).unwrap_or("?"),
    );

    let _ = sys_lseek(fd, 0, SEEK_SET);
    let _ = sys_read(fd, buf.as_mut_ptr(), 3);
    let pos = sys_lseek(fd, 2, SEEK_CUR);
    check("SEEK_CUR +2 from 3", pos == 5, fmt_i64(pos));

    let pos = sys_lseek(fd, 0, SEEK_END);
    check("SEEK_END 0", pos == 13, fmt_i64(pos));

    let pos = sys_lseek(fd, -5, SEEK_END);
    check("SEEK_END -5", pos == 8, fmt_i64(pos));

    let pos = sys_lseek(fd, -100, SEEK_SET);
    check("EINVAL on negative pos", pos == -22, fmt_i64(pos));

    let pos = sys_lseek(fd, 0, 99);
    check("EINVAL on bad whence", pos == -22, fmt_i64(pos));

    sys_close(fd);

    let pos = sys_lseek(fd, 0, SEEK_SET);
    check("EBADF on closed fd", pos == -9, fmt_i64(pos));
}

fn test_close_errors() {
    println!("[close errors]");

    let r = sys_close(999);
    check("EBADF on invalid fd", r == -9, fmt_i32(r));

    let r2 = sys_close(-1);
    check("EBADF on negative fd", r2 == -9, fmt_i32(r2));
}

fn test_permissions() {
    println!("[permissions]");

    let fd = sys_open(c"/src/main.rs".as_ptr() as _, O_RDONLY, 0);
    if fd < 0 {
        check("open rdonly for perm test", false, fmt_i32(fd));
        return;
    }
    let n = sys_write(fd, b"x".as_ptr(), 1);
    check("EBADF writing to O_RDONLY fd", n == -9, fmt_isize(n));
    sys_close(fd);

    let fd2 = sys_open(
        c"/tmp/perm_test.txt".as_ptr() as _,
        O_WRONLY | O_CREAT,
        0o644,
    );
    if fd2 < 0 {
        check("open wronly for perm test", false, fmt_i32(fd2));
        return;
    }
    let mut buf = [0u8; 8];
    let n2 = sys_read(fd2, buf.as_mut_ptr(), buf.len());
    check("EBADF reading from O_WRONLY fd", n2 == -9, fmt_isize(n2));

    let n3 = sys_write(fd2, b"test".as_ptr(), 4);
    check("write to O_WRONLY fd works", n3 == 4, fmt_isize(n3));
    sys_close(fd2);

    let fd3 = sys_open(c"/tmp/perm_test.txt".as_ptr() as _, O_RDWR, 0);
    if fd3 < 0 {
        check("open rdwr for perm test", false, fmt_i32(fd3));
        return;
    }
    let n4 = sys_read(fd3, buf.as_mut_ptr(), buf.len());
    check("read from O_RDWR fd works", n4 > 0, fmt_isize(n4));
    let n5 = sys_write(fd3, b"!".as_ptr(), 1);
    check("write to O_RDWR fd works", n5 == 1, fmt_isize(n5));
    sys_close(fd3);
}

fn test_relative_paths() {
    println!("[relative paths]");

    let fd = sys_open(c"src/main.rs".as_ptr() as _, O_RDONLY, 0);
    check("open relative path from /", fd >= 3, fmt_i32(fd));
    if fd >= 0 {
        sys_close(fd);
    }

    let r = sys_mkdir(c"tmp/reltest".as_ptr() as _, 0o755);
    check("mkdir relative path", r == 0, fmt_i32(r));

    let fd2 = sys_open(
        c"tmp/reltest/file.txt".as_ptr() as _,
        O_WRONLY | O_CREAT,
        0o644,
    );
    check("create file via relative path", fd2 >= 3, fmt_i32(fd2));
    if fd2 >= 0 {
        sys_close(fd2);
    }
}

fn test_nanosleep() {
    println!("[nanosleep]");

    let req = Timespec {
        tv_sec: 0,
        tv_nsec: 100_000_000,
    };
    let r = sys_nanosleep(&req);
    check("nanosleep 100ms", r == 0, fmt_i32(r));

    let bad1 = Timespec {
        tv_sec: -1,
        tv_nsec: 0,
    };
    let r2 = sys_nanosleep(&bad1);
    check("EINVAL on negative tv_sec", r2 == -22, fmt_i32(r2));

    let bad2 = Timespec {
        tv_sec: 0,
        tv_nsec: 1_000_000_000,
    };
    let r3 = sys_nanosleep(&bad2);
    check("EINVAL on tv_nsec >= 1e9", r3 == -22, fmt_i32(r3));

    let bad3 = Timespec {
        tv_sec: 0,
        tv_nsec: -1,
    };
    let r4 = sys_nanosleep(&bad3);
    check("EINVAL on negative tv_nsec", r4 == -22, fmt_i32(r4));
}

fn test_yield() {
    println!("[sched_yield]");
    sys_yield();
    check("yield returns", true, "");
}

fn test_getpid() {
    println!("[getpid/getppid]");
    let pid = sys_getpid();
    check(
        "getpid returns non-negative",
        pid < 0x8000_0000_0000_0000,
        "",
    );
    let ppid = sys_getppid();
    check("getppid returns 0 (no parent)", ppid == 0, "");
    let pid2 = sys_getpid();
    check("getpid is stable", pid == pid2, "");
}

fn test_uname() {
    println!("[uname]");
    let mut buf = core::mem::MaybeUninit::<UtsName>::uninit();
    let r = sys_uname(buf.as_mut_ptr());
    check("uname returns 0", r == 0, fmt_i32(r));

    if r == 0 {
        let uts = unsafe { buf.assume_init_ref() };
        let sysname = unsafe { core::ffi::CStr::from_ptr(uts.sysname.as_ptr() as _) };
        check("sysname is Chronos", sysname.to_str() == Ok("Chronos"), "");
        let machine = unsafe { core::ffi::CStr::from_ptr(uts.machine.as_ptr() as _) };
        check("machine is x86_64", machine.to_str() == Ok("x86_64"), "");
    }
}

fn test_chdir() {
    println!("[chdir]");
    sys_mkdir(c"/tmp/chdir_test".as_ptr() as _, 0o755);

    let r = sys_chdir(c"/tmp/chdir_test".as_ptr() as _);
    check("chdir to /tmp/chdir_test", r == 0, fmt_i32(r));

    let buf = [0u8; 256];
    let ptr = sys_get_cwd(buf.as_ptr() as _, buf.len());
    if (ptr as isize) > 0 {
        let cwd = unsafe { core::ffi::CStr::from_ptr(ptr as *const core::ffi::c_char) };
        check(
            "cwd is /tmp/chdir_test",
            cwd.to_str() == Ok("/tmp/chdir_test"),
            cwd.to_str().unwrap_or("?"),
        );
    }

    let r2 = sys_chdir(c"/nonexistent".as_ptr() as _);
    check("ENOENT on bad chdir", r2 == -2, fmt_i32(r2));

    sys_open(
        c"/tmp/chdir_test/f".as_ptr() as _,
        O_WRONLY | O_CREAT,
        0o644,
    );
    let r3 = sys_chdir(c"/tmp/chdir_test/f".as_ptr() as _);
    check("ENOTDIR chdir to file", r3 == -20, fmt_i32(r3));

    sys_chdir(c"/".as_ptr() as _);
}

fn test_stat() {
    println!("[stat]");
    let mut st = core::mem::MaybeUninit::<StatBuf>::uninit();

    let r = sys_stat(c"/".as_ptr() as _, st.as_mut_ptr());
    check("stat / returns 0", r == 0, fmt_i32(r));
    if r == 0 {
        let s = unsafe { st.assume_init_ref() };
        check(
            "/ is a directory (mode)",
            s.st_mode & 0o170000 == 0o040000,
            "",
        );
    }

    let r2 = sys_stat(c"/src/main.rs".as_ptr() as _, st.as_mut_ptr());
    check("stat /src/main.rs returns 0", r2 == 0, fmt_i32(r2));
    if r2 == 0 {
        let s = unsafe { st.assume_init_ref() };
        check(
            "main.rs is a file (mode)",
            s.st_mode & 0o170000 == 0o100000,
            "",
        );
        check("main.rs has size > 0", s.st_size > 0, "");
    }

    let r3 = sys_stat(c"/nonexistent".as_ptr() as _, st.as_mut_ptr());
    check("ENOENT on missing path", r3 == -2, fmt_i32(r3));
}

fn test_fstat() {
    println!("[fstat]");
    let fd = sys_open(c"/src/main.rs".as_ptr() as _, O_RDONLY, 0);
    if fd < 0 {
        check("open for fstat", false, fmt_i32(fd));
        return;
    }

    let mut st = core::mem::MaybeUninit::<StatBuf>::uninit();
    let r = sys_fstat(fd, st.as_mut_ptr());
    check("fstat returns 0", r == 0, fmt_i32(r));
    if r == 0 {
        let s = unsafe { st.assume_init_ref() };
        check(
            "fstat shows file type",
            s.st_mode & 0o170000 == 0o100000,
            "",
        );
        check("fstat shows size > 0", s.st_size > 0, "");
    }
    sys_close(fd);

    let r2 = sys_fstat(999, st.as_mut_ptr());
    check("EBADF on bad fd", r2 == -9, fmt_i32(r2));
}

fn test_unlink() {
    println!("[unlink]");
    let fd = sys_open(
        c"/tmp/unlink_me.txt".as_ptr() as _,
        O_WRONLY | O_CREAT,
        0o644,
    );
    if fd >= 0 {
        sys_write(fd, b"bye".as_ptr(), 3);
        sys_close(fd);
    }

    let r = sys_unlink(c"/tmp/unlink_me.txt".as_ptr() as _);
    check("unlink file", r == 0, fmt_i32(r));

    let fd2 = sys_open(c"/tmp/unlink_me.txt".as_ptr() as _, O_RDONLY, 0);
    check("file is gone after unlink", fd2 == -2, fmt_i32(fd2));

    let r2 = sys_unlink(c"/tmp/unlink_me.txt".as_ptr() as _);
    check("ENOENT on already unlinked", r2 == -2, fmt_i32(r2));

    sys_mkdir(c"/tmp/unlink_dir".as_ptr() as _, 0o755);
    let r3 = sys_unlink(c"/tmp/unlink_dir".as_ptr() as _);
    check("EISDIR unlinking dir", r3 == -21, fmt_i32(r3));
    sys_rmdir(c"/tmp/unlink_dir".as_ptr() as _); // cleanup
}

fn test_rmdir() {
    println!("[rmdir]");
    sys_mkdir(c"/tmp/rmdir_test".as_ptr() as _, 0o755);
    let r = sys_rmdir(c"/tmp/rmdir_test".as_ptr() as _);
    check("rmdir empty dir", r == 0, fmt_i32(r));

    let r2 = sys_rmdir(c"/tmp/rmdir_test".as_ptr() as _);
    check("ENOENT on removed dir", r2 == -2, fmt_i32(r2));

    sys_mkdir(c"/tmp/rmdir_full".as_ptr() as _, 0o755);
    let fd = sys_open(
        c"/tmp/rmdir_full/file.txt".as_ptr() as _,
        O_WRONLY | O_CREAT,
        0o644,
    );
    if fd >= 0 {
        sys_close(fd);
    }
    let r3 = sys_rmdir(c"/tmp/rmdir_full".as_ptr() as _);
    check("ENOTEMPTY on non-empty", r3 == -39, fmt_i32(r3));

    sys_unlink(c"/tmp/rmdir_full/file.txt".as_ptr() as _);
    sys_rmdir(c"/tmp/rmdir_full".as_ptr() as _);
}

fn test_dup() {
    println!("[dup]");
    let fd = sys_open(c"/src/main.rs".as_ptr() as _, O_RDONLY, 0);
    if fd < 0 {
        check("open for dup", false, fmt_i32(fd));
        return;
    }

    let fd2 = sys_dup(fd);
    check("dup returns new fd", fd2 >= 3 && fd2 != fd, fmt_i32(fd2));

    let mut buf1 = [0u8; 8];
    let mut buf2 = [0u8; 8];
    let n1 = sys_read(fd, buf1.as_mut_ptr(), buf1.len());
    let n2 = sys_read(fd2, buf2.as_mut_ptr(), buf2.len());
    check("read from original fd", n1 > 0, fmt_isize(n1));
    check("read from dup'd fd", n2 > 0, fmt_isize(n2));

    sys_close(fd);
    sys_close(fd2);

    let r = sys_dup(999);
    check("EBADF dup bad fd", r == -9, fmt_i32(r));
}

fn test_dup2() {
    println!("[dup2]");
    let fd = sys_open(c"/src/main.rs".as_ptr() as _, O_RDONLY, 0);
    if fd < 0 {
        check("open for dup2", false, fmt_i32(fd));
        return;
    }

    let target = 100;
    let r = sys_dup2(fd, target);
    check("dup2 to fd 100", r == target, fmt_i32(r));

    let mut buf = [0u8; 8];
    let n = sys_read(target, buf.as_mut_ptr(), buf.len());
    check("read from dup2'd fd", n > 0, fmt_isize(n));

    let r2 = sys_dup2(fd, fd);
    check("dup2 same fd returns fd", r2 == fd, fmt_i32(r2));

    sys_close(fd);
    sys_close(target);

    let r3 = sys_dup2(999, 101);
    check("EBADF dup2 bad fd", r3 == -9, fmt_i32(r3));
}

fn test_ftruncate() {
    println!("[ftruncate]");
    let fd = sys_open(
        c"/tmp/trunc_test.txt".as_ptr() as _,
        O_RDWR | O_CREAT | O_TRUNC,
        0o644,
    );
    if fd < 0 {
        check("open for ftruncate", false, fmt_i32(fd));
        return;
    }

    sys_write(fd, b"hello world".as_ptr(), 11);

    let r = sys_ftruncate(fd, 5);
    check("ftruncate to 5", r == 0, fmt_i32(r));

    let mut st = core::mem::MaybeUninit::<StatBuf>::uninit();
    sys_fstat(fd, st.as_mut_ptr());
    let s = unsafe { st.assume_init_ref() };
    check("size is 5 after truncate", s.st_size == 5, "");

    let r2 = sys_ftruncate(fd, 20);
    check("ftruncate extend to 20", r2 == 0, fmt_i32(r2));
    sys_fstat(fd, st.as_mut_ptr());
    let s = unsafe { st.assume_init_ref() };
    check("size is 20 after extend", s.st_size == 20, "");

    sys_close(fd);

    let r3 = sys_ftruncate(999, 0);
    check("EBADF on bad fd", r3 == -9, fmt_i32(r3));
}

fn test_getdents64() {
    println!("[getdents64]");
    sys_mkdir(c"/tmp/dents_test".as_ptr() as _, 0o755);
    let fd1 = sys_open(
        c"/tmp/dents_test/aaa".as_ptr() as _,
        O_WRONLY | O_CREAT,
        0o644,
    );
    if fd1 >= 0 {
        sys_close(fd1);
    }
    let fd2 = sys_open(
        c"/tmp/dents_test/bbb".as_ptr() as _,
        O_WRONLY | O_CREAT,
        0o644,
    );
    if fd2 >= 0 {
        sys_close(fd2);
    }
    sys_mkdir(c"/tmp/dents_test/ccc".as_ptr() as _, 0o755);

    let dir_fd = sys_open(c"/tmp/dents_test".as_ptr() as _, O_RDONLY | O_DIRECTORY, 0);
    if dir_fd < 0 {
        check("open dir for getdents64", false, fmt_i32(dir_fd));
        return;
    }

    let mut buf = [0u8; 1024];
    let n = sys_getdents64(dir_fd, buf.as_mut_ptr(), buf.len());
    check("getdents64 returns bytes > 0", n > 0, fmt_isize(n));

    let mut count = 0u32;
    let mut offset = 0usize;
    while offset < n as usize {
        let entry = unsafe { &*(buf.as_ptr().add(offset) as *const LinuxDirent64) };
        count += 1;
        offset += entry.d_reclen as usize;
    }
    check("found 3 entries", count == 3, "");

    let n2 = sys_getdents64(dir_fd, buf.as_mut_ptr(), buf.len());
    check("second call returns 0", n2 == 0, fmt_isize(n2));

    sys_close(dir_fd);

    sys_unlink(c"/tmp/dents_test/aaa".as_ptr() as _);
    sys_unlink(c"/tmp/dents_test/bbb".as_ptr() as _);
    sys_rmdir(c"/tmp/dents_test/ccc".as_ptr() as _);
    sys_rmdir(c"/tmp/dents_test".as_ptr() as _);
}

fn test_gettid() {
    println!("[gettid]");
    let tid = sys_gettid();
    check("gettid returns > 0", tid > 0, "");
    let tid2 = sys_gettid();
    check("gettid is stable", tid == tid2, "");
}

fn test_access() {
    println!("[access]");
    let r = sys_access(c"/src/main.rs".as_ptr() as _, 0);
    check("access existing file", r == 0, fmt_i32(r));

    let r2 = sys_access(c"/".as_ptr() as _, 0);
    check("access root dir", r2 == 0, fmt_i32(r2));

    let r3 = sys_access(c"/nonexistent".as_ptr() as _, 0);
    check("ENOENT on missing", r3 == -2, fmt_i32(r3));
}

fn test_rename() {
    println!("[rename]");
    let fd = sys_open(
        c"/tmp/rename_src.txt".as_ptr() as _,
        O_WRONLY | O_CREAT,
        0o644,
    );
    if fd >= 0 {
        sys_write(fd, b"rename me".as_ptr(), 9);
        sys_close(fd);
    }

    let r = sys_rename(
        c"/tmp/rename_src.txt".as_ptr() as _,
        c"/tmp/rename_dst.txt".as_ptr() as _,
    );
    check("rename file", r == 0, fmt_i32(r));

    let r2 = sys_access(c"/tmp/rename_src.txt".as_ptr() as _, 0);
    check("old name gone", r2 == -2, fmt_i32(r2));

    let fd2 = sys_open(c"/tmp/rename_dst.txt".as_ptr() as _, O_RDONLY, 0);
    check("new name exists", fd2 >= 3, fmt_i32(fd2));
    if fd2 >= 0 {
        let mut buf = [0u8; 16];
        let n = sys_read(fd2, buf.as_mut_ptr(), buf.len());
        check("content preserved", n == 9 && &buf[..9] == b"rename me", "");
        sys_close(fd2);
    }

    let r3 = sys_rename(
        c"/tmp/nonexistent".as_ptr() as _,
        c"/tmp/whatever".as_ptr() as _,
    );
    check("ENOENT on missing source", r3 == -2, fmt_i32(r3));

    sys_unlink(c"/tmp/rename_dst.txt".as_ptr() as _);
}

fn test_clock_gettime() {
    println!("[clock_gettime]");
    let mut ts = Timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };

    let r = sys_clock_gettime(0, &mut ts);
    check("CLOCK_REALTIME returns 0", r == 0, fmt_i32(r));
    check("realtime sec > 0", ts.tv_sec > 0, "");

    let r2 = sys_clock_gettime(1, &mut ts);
    check("CLOCK_MONOTONIC returns 0", r2 == 0, fmt_i32(r2));
    check("monotonic sec >= 0", ts.tv_sec >= 0, "");

    let ns_before = ts.tv_sec * 1_000_000_000 + ts.tv_nsec;
    sys_clock_gettime(1, &mut ts);
    let ns_after = ts.tv_sec * 1_000_000_000 + ts.tv_nsec;
    check("monotonic advances", ns_after >= ns_before, "");

    let r3 = sys_clock_gettime(99, &mut ts);
    check("EINVAL on bad clock_id", r3 == -22, fmt_i32(r3));
}

fn test_fork() {
    println!("[fork]");
    let pid = sys_fork();
    if pid == 0 {
        let my_pid = sys_getpid();
        check("child gets new pid", my_pid > 0, "");
        let ppid = sys_getppid();
        check("child ppid != 0", ppid > 0, "");
        sys_exit(0);
    } else {
        check("parent gets child pid", pid > 0, fmt_i64(pid));
        let mut status: i32 = -1;
        loop {
            let waited = sys_waitpid(pid, &mut status, 0);
            if waited == pid {
                check("waitpid returns child pid", true, "");
                break;
            }
            sys_yield();
        }
    }
}

fn test_fork_wait() {
    println!("[fork+wait]");
    let pid = sys_fork();
    if pid == 0 {
        sys_exit(42);
    } else {
        check("fork returned child pid", pid > 0, fmt_i64(pid));
        let mut status: i32 = -1;
        loop {
            let r = sys_waitpid(pid, &mut status, 0);
            if r == pid {
                break;
            }
            sys_yield();
        }
        let exit_code = (status >> 8) & 0xff;
        check("child exited with 42", exit_code == 42, "");
    }
}

fn test_execve() {
    println!("[execve]");
    let r = sys_execve(c"/nonexistent".as_ptr(), 0, 0);
    check("ENOENT on bad path", r == -2, fmt_i64(r));
}

fn fmt_i32(v: i32) -> &'static str {
    match v {
        -2 => "ENOENT (-2)",
        -5 => "EIO (-5)",
        -9 => "EBADF (-9)",
        -14 => "EFAULT (-14)",
        -17 => "EEXIST (-17)",
        -20 => "ENOTDIR (-20)",
        -21 => "EISDIR (-21)",
        -22 => "EINVAL (-22)",
        -34 => "ERANGE (-34)",
        -38 => "ENOSYS (-38)",
        -39 => "ENOTEMPTY (-39)",
        _ => "unexpected value",
    }
}

fn fmt_isize(v: isize) -> &'static str {
    fmt_i32(v as i32)
}

fn fmt_i64(v: i64) -> &'static str {
    fmt_i32(v as i32)
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    println!("{}", info);
    sys_exit(1);
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
