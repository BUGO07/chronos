#![no_std]
#![no_main]
#![allow(clippy::macro_metavars_in_unsafe)]

use core::fmt::Write;

use crate::syscalls::{
    Timespec, sys_close, sys_exit, sys_get_cwd, sys_lseek, sys_mkdir, sys_nanosleep, sys_open,
    sys_read, sys_write, sys_yield,
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
pub extern "C" fn _start() -> ! {
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

fn fmt_i32(v: i32) -> &'static str {
    match v {
        -2 => "ENOENT (-2)",
        -5 => "EIO (-5)",
        -9 => "EBADF (-9)",
        -17 => "EEXIST (-17)",
        -22 => "EINVAL (-22)",
        -34 => "ERANGE (-34)",
        -38 => "ENOSYS (-38)",
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
