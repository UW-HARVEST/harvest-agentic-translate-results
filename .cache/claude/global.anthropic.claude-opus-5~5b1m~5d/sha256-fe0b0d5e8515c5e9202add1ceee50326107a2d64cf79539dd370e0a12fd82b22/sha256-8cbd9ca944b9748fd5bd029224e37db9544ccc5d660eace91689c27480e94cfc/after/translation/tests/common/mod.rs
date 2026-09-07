//! Shared differential-test harness.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! called only through their exported C symbols -- the Rust functions are never
//! called directly, so the `#[no_mangle]` wrappers and the C ABI are under test
//! too.

#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int, c_long, c_uint, c_ulong, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

/* ------------------------------------------------------------------ */
/* libc bits the tests themselves need                                 */
/* ------------------------------------------------------------------ */

#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

extern "C" {
    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fdopen(fd: c_int, mode: *const c_char) -> *mut FILE;
    pub fn fmemopen(buf: *mut c_void, size: usize, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;
    pub fn fseek(stream: *mut FILE, off: c_long, whence: c_int) -> c_int;
    pub fn ftell(stream: *mut FILE) -> c_long;
    pub fn fflush(stream: *mut FILE) -> c_int;
    pub fn free(p: *mut c_void);
    pub fn strlen(s: *const c_char) -> usize;
    pub fn pipe(fds: *mut c_int) -> c_int;
    pub fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    pub fn close(fd: c_int) -> c_int;
    pub fn dup(fd: c_int) -> c_int;
    pub fn dup2(old: c_int, new: c_int) -> c_int;
    pub fn fork() -> c_int;
    pub fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    pub fn _exit(code: c_int) -> !;
    pub fn chdir(path: *const c_char) -> c_int;
}

pub const SEEK_SET: c_int = 0;
pub const SEEK_CUR: c_int = 1;
pub const SEEK_END: c_int = 2;

/* ------------------------------------------------------------------ */
/* mirrored C types                                                    */
/* ------------------------------------------------------------------ */

pub const MAX_FQUEUE: usize = 256;

pub const CRALERT_MAIL_SET: c_int = 0x001;
pub const CRALERT_EXEC_SET: c_int = 0x002;
pub const CRALERT_READ_ALL: c_int = 0x004;
pub const CRALERT_READ_FAILED: c_int = 0x008;
pub const CRALERT_FP_SET: c_int = 0x010;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct timespec {
    pub tv_sec: c_long,
    pub tv_nsec: c_long,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct stat {
    pub st_dev: c_ulong,
    pub st_ino: c_ulong,
    pub st_nlink: c_ulong,
    pub st_mode: c_uint,
    pub st_uid: c_uint,
    pub st_gid: c_uint,
    pub __pad0: c_uint,
    pub st_rdev: c_ulong,
    pub st_size: c_long,
    pub st_blksize: c_long,
    pub st_blocks: c_long,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __glibc_reserved: [c_long; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
    pub tm_gmtoff: c_long,
    pub tm_zone: *const c_char,
}

impl tm {
    pub fn new(mday: c_int, mon: c_int, year: c_int) -> tm {
        let mut t: tm = unsafe { std::mem::zeroed() };
        t.tm_mday = mday;
        t.tm_mon = mon;
        t.tm_year = year;
        t
    }
}

#[repr(C)]
pub struct file_queue {
    pub last_change: c_long,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut FILE,
    pub f_status: stat,
}

impl file_queue {
    pub fn zeroed() -> file_queue {
        unsafe { std::mem::zeroed() }
    }
}

#[repr(C)]
pub struct alert_data {
    pub rule: c_uint,
    pub level: c_uint,
    pub alertid: *mut c_char,
    pub date: *mut c_char,
    pub location: *mut c_char,
    pub comment: *mut c_char,
    pub group: *mut c_char,
    pub srcip: *mut c_char,
    pub srcport: c_int,
    pub dstip: *mut c_char,
    pub dstport: c_int,
    pub user: *mut c_char,
    pub filename: *mut c_char,
}

/* ------------------------------------------------------------------ */
/* value snapshot used for byte-for-byte comparison                    */
/* ------------------------------------------------------------------ */

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AlertSnap {
    pub rule: c_uint,
    pub level: c_uint,
    pub srcport: c_int,
    pub dstport: c_int,
    pub alertid: Option<Vec<u8>>,
    pub date: Option<Vec<u8>>,
    pub location: Option<Vec<u8>>,
    pub comment: Option<Vec<u8>>,
    pub group: Option<Vec<u8>>,
    pub srcip: Option<Vec<u8>>,
    pub dstip: Option<Vec<u8>>,
    pub user: Option<Vec<u8>>,
    pub filename: Option<Vec<u8>>,
}

unsafe fn cstr_opt(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(std::slice::from_raw_parts(p as *const u8, strlen(p)).to_vec())
    }
}

/// `None` == the call returned `NULL`.
pub unsafe fn snap(p: *const alert_data) -> Option<AlertSnap> {
    if p.is_null() {
        return None;
    }
    let a = &*p;
    Some(AlertSnap {
        rule: a.rule,
        level: a.level,
        srcport: a.srcport,
        dstport: a.dstport,
        alertid: cstr_opt(a.alertid),
        date: cstr_opt(a.date),
        location: cstr_opt(a.location),
        comment: cstr_opt(a.comment),
        group: cstr_opt(a.group),
        srcip: cstr_opt(a.srcip),
        dstip: cstr_opt(a.dstip),
        user: cstr_opt(a.user),
        filename: cstr_opt(a.filename),
    })
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct QueueSnap {
    pub last_change: c_long,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [u8; 4],
    pub file_name: Vec<u8>,
    pub fp_null: bool,
    pub st_size: c_long,
    pub st_ino: c_ulong,
    pub st_mtime: c_long,
    pub offset: c_long,
}

/// Snapshot of a `file_queue`. `include_mon` is false for out-of-range
/// `tm_mon`, where the C reads past the end of the 12-entry `s_month` table
/// (undefined behaviour with no reproducible value).
pub unsafe fn qsnap(q: *const file_queue, include_mon: bool) -> QueueSnap {
    let q = &*q;
    let fn_bytes = {
        let s = q.file_name.as_ptr();
        std::slice::from_raw_parts(s as *const u8, strlen(s)).to_vec()
    };
    QueueSnap {
        last_change: q.last_change,
        year: q.year,
        day: q.day,
        flags: q.flags,
        mon: if include_mon {
            [
                q.mon[0] as u8,
                q.mon[1] as u8,
                q.mon[2] as u8,
                q.mon[3] as u8,
            ]
        } else {
            [0; 4]
        },
        file_name: fn_bytes,
        fp_null: q.fp.is_null(),
        st_size: q.f_status.st_size,
        st_ino: q.f_status.st_ino,
        st_mtime: q.f_status.st_mtim.tv_sec,
        offset: if q.fp.is_null() { -1 } else { ftell(q.fp) },
    }
}

/* ------------------------------------------------------------------ */
/* the two libraries                                                   */
/* ------------------------------------------------------------------ */

pub type FnDriver = unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut alert_data;
pub type FnInitFileQueue = unsafe extern "C" fn(*mut file_queue, *const tm, c_int) -> c_int;
pub type FnReadFileMon = unsafe extern "C" fn(*mut file_queue, *const tm, c_uint) -> *mut alert_data;
pub type FnGetAlertData = unsafe extern "C" fn(c_int, *mut FILE) -> *mut alert_data;
pub type FnFreeAlertData = unsafe extern "C" fn(*mut alert_data);
pub type FnMerror = unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char);
pub type FnOsCalloc = unsafe extern "C" fn(usize, usize) -> *mut c_void;
pub type FnOsRealloc = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnOsStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;

pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    pub driver: FnDriver,
    pub Init_FileQueue: FnInitFileQueue,
    pub Read_FileMon: FnReadFileMon,
    pub GetAlertData: FnGetAlertData,
    pub FreeAlertData: FnFreeAlertData,
    pub merror: FnMerror,
    pub os_calloc: FnOsCalloc,
    pub os_realloc: FnOsRealloc,
    pub os_strdup: FnOsStrdup,
}

impl Impl {
    unsafe fn load(name: &'static str, path: &Path) -> Impl {
        let lib = Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        macro_rules! g {
            ($t:ty, $s:literal) => {{
                let s: Symbol<$t> = lib
                    .get($s)
                    .unwrap_or_else(|e| {
                        panic!("{} missing symbol {:?}: {e}", name, &$s[..$s.len() - 1])
                    });
                *s
            }};
        }
        Impl {
            name,
            driver: g!(FnDriver, b"driver\0"),
            Init_FileQueue: g!(FnInitFileQueue, b"Init_FileQueue\0"),
            Read_FileMon: g!(FnReadFileMon, b"Read_FileMon\0"),
            GetAlertData: g!(FnGetAlertData, b"GetAlertData\0"),
            FreeAlertData: g!(FnFreeAlertData, b"FreeAlertData\0"),
            merror: g!(FnMerror, b"merror\0"),
            os_calloc: g!(FnOsCalloc, b"os_calloc\0"),
            os_realloc: g!(FnOsRealloc, b"os_realloc\0"),
            os_strdup: g!(FnOsStrdup, b"os_strdup\0"),
            _lib: lib,
        }
    }
}

pub struct Both {
    pub c: Impl,
    pub rs: Impl,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so")
}

pub fn rs_so_path() -> PathBuf {
    let rel = manifest_dir().join("target/release/libdriver.so");
    if rel.exists() {
        return rel;
    }
    let dbg = manifest_dir().join("target/debug/libdriver.so");
    if dbg.exists() {
        return dbg;
    }
    panic!("no Rust libdriver.so found; run `cargo build --release` first");
}

static BOTH: OnceLock<Both> = OnceLock::new();

pub fn both() -> &'static Both {
    BOTH.get_or_init(|| unsafe {
        Both {
            c: Impl::load("C", &c_so_path()),
            rs: Impl::load("Rust", &rs_so_path()),
        }
    })
}

/* ------------------------------------------------------------------ */
/* working directory (CWD is process global -> serialise)              */
/* ------------------------------------------------------------------ */

static CWD_LOCK: Mutex<()> = Mutex::new(());

pub fn tmp_root() -> PathBuf {
    let p = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("difftest");
    std::fs::create_dir_all(&p).unwrap();
    p
}

pub struct WorkDir {
    _guard: MutexGuard<'static, ()>,
    pub path: PathBuf,
}

/// Serialised chdir into a scratch directory. Every test that relies on the
/// relative `alerts.log` / `<stdin>` file names must run inside one.
pub fn workdir(name: &str) -> WorkDir {
    let guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let path = tmp_root().join(name);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    let c = CString::new(path.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { chdir(c.as_ptr()) }, 0, "chdir failed");
    WorkDir {
        _guard: guard,
        path,
    }
}

impl WorkDir {
    pub fn write(&self, name: &str, bytes: &[u8]) {
        std::fs::write(self.path.join(name), bytes).unwrap();
    }
    pub fn remove(&self, name: &str) {
        let _ = std::fs::remove_file(self.path.join(name));
    }
}

/* ------------------------------------------------------------------ */
/* misc helpers                                                        */
/* ------------------------------------------------------------------ */

/// Deterministic xorshift64* PRNG (fixed seed => reproducible runs).
pub struct Rng(pub u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
    /// Printable-ish token, never containing `\n` or `\0`.
    pub fn token(&mut self, max: usize) -> Vec<u8> {
        const CH: &[u8] = b"abcXYZ019._-:/ '\"\\@#";
        let n = self.below(max) + 1;
        (0..n).map(|_| *self.pick(CH)).collect()
    }
}

/// An unseekable stream: the read end of a pipe, pre-filled with `payload`.
pub unsafe fn pipe_stream(payload: &[u8]) -> *mut FILE {
    let mut fds = [0 as c_int; 2];
    assert_eq!(pipe(fds.as_mut_ptr()), 0);
    write(fds[1], payload.as_ptr() as *const c_void, payload.len());
    close(fds[1]);
    let fp = fdopen(fds[0], b"r\0".as_ptr() as *const c_char);
    assert!(!fp.is_null(), "fdopen failed");
    fp
}

/// Write a NUL-terminated temp file and `fopen` it; returns the stream.
pub unsafe fn open_bytes(dir: &Path, name: &str, bytes: &[u8]) -> *mut FILE {
    let p = dir.join(name);
    std::fs::write(&p, bytes).unwrap();
    let cp = CString::new(p.to_str().unwrap()).unwrap();
    let fp = fopen(cp.as_ptr(), b"r\0".as_ptr() as *const c_char);
    assert!(!fp.is_null(), "fopen {} failed", p.display());
    fp
}

/// Call `GetAlertData` repeatedly on a fresh stream over `bytes` until it
/// returns NULL (max `limit` iterations), snapshotting each result.
pub unsafe fn drain(imp: &Impl, dir: &Path, tag: &str, bytes: &[u8], flag: c_int, limit: usize)
    -> Vec<Option<AlertSnap>>
{
    let fp = open_bytes(dir, &format!("drain_{}_{}.log", imp.name, tag), bytes);
    let mut out = Vec::new();
    for _ in 0..limit {
        let a = (imp.GetAlertData)(flag, fp);
        let s = snap(a);
        let done = s.is_none();
        if !a.is_null() {
            (imp.FreeAlertData)(a);
        }
        out.push(s);
        if done {
            break;
        }
    }
    fclose(fp);
    out
}

/// fd 2 is process-global, so redirecting it must be serialised. The same lock
/// also guards `fork()` (see below).
static STDERR_LOCK: Mutex<()> = Mutex::new(());

/// Run `f` in a forked child with stderr redirected to `err_path`;
/// return the raw `waitpid` status.
pub unsafe fn fork_status<F: FnOnce()>(err_path: &Path, f: F) -> c_int {
    // serialise: forking a multi-threaded process and then calling malloc in the
    // child is only safe if no other thread is mid-allocation.
    let _lock = STDERR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let ep = CString::new(err_path.to_str().unwrap()).unwrap();
    let pid = fork();
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // child
        let fp = fopen(ep.as_ptr(), b"w\0".as_ptr() as *const c_char);
        if !fp.is_null() {
            dup2(
                {
                    extern "C" {
                        fn fileno(f: *mut FILE) -> c_int;
                    }
                    fileno(fp)
                },
                2,
            );
        }
        f();
        // the callee was supposed to exit(); mark "returned normally"
        fflush(std::ptr::null_mut());
        _exit(42);
    }
    let mut st: c_int = 0;
    waitpid(pid, &mut st, 0);
    st
}

pub fn wait_exited(st: c_int) -> Option<c_int> {
    if st & 0x7f == 0 {
        Some((st >> 8) & 0xff)
    } else {
        None
    }
}

pub fn wait_signal(st: c_int) -> Option<c_int> {
    let low = st & 0x7f;
    if low != 0 && low != 0x7f {
        Some(low)
    } else {
        None
    }
}

/// Capture everything written to fd 2 while running `f`.
pub unsafe fn capture_stderr<F: FnOnce()>(path: &Path, f: F) -> Vec<u8> {
    let _lock = STDERR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _ = std::fs::remove_file(path);
    let cp = CString::new(path.to_str().unwrap()).unwrap();
    let fp = fopen(cp.as_ptr(), b"w\0".as_ptr() as *const c_char);
    assert!(!fp.is_null());
    extern "C" {
        fn fileno(f: *mut FILE) -> c_int;
    }
    let saved = dup(2);
    dup2(fileno(fp), 2);
    f();
    fflush(std::ptr::null_mut());
    dup2(saved, 2);
    close(saved);
    fclose(fp);
    std::fs::read(path).unwrap_or_default()
}

/* ------------------------------------------------------------------ */
/* alert-file builders                                                 */
/* ------------------------------------------------------------------ */

/// A canonical, well-formed single alert.
pub fn simple_alert() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(b"** Alert 1500000000.1234: mail  - syslog,errors,\n");
    v.extend_from_slice(b"2006 Apr 13 16:15:17 myhost->/var/log/auth.log\n");
    v.extend_from_slice(b"Rule: 5715 (level 5) -> 'sshd authentication success.'\n");
    v.extend_from_slice(b"Src IP: 192.168.1.1\n");
    v.extend_from_slice(b"Src Port: 4321\n");
    v.extend_from_slice(b"Dst IP: 10.0.0.5\n");
    v.extend_from_slice(b"Dst Port: 22\n");
    v.extend_from_slice(b"User: root\n");
    v.extend_from_slice(b"Apr 13 16:15:16 host sshd[123]: Accepted password\n");
    v.extend_from_slice(b"\n");
    v
}

pub struct AlertSpec {
    pub id: Vec<u8>,
    pub tag: Vec<u8>,
    pub group: Vec<u8>,
    pub date: Vec<u8>,
    pub location: Vec<u8>,
    pub rule: Option<Vec<u8>>,
    pub srcip: Option<Vec<u8>>,
    pub srcport: Option<Vec<u8>>,
    pub dstip: Option<Vec<u8>>,
    pub dstport: Option<Vec<u8>>,
    pub user: Option<Vec<u8>>,
    pub logs: Vec<Vec<u8>>,
    /// Order of the field lines; indices into `[rule, srcip, srcport, dstip, dstport, user]`.
    pub order: Vec<usize>,
}

impl AlertSpec {
    pub fn render(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(b"** Alert ");
        out.extend_from_slice(&self.id);
        out.extend_from_slice(b": ");
        out.extend_from_slice(&self.tag);
        out.extend_from_slice(b" - ");
        out.extend_from_slice(&self.group);
        out.push(b'\n');
        out.extend_from_slice(&self.date);
        out.push(b' ');
        out.extend_from_slice(&self.location);
        out.push(b'\n');
        for &i in &self.order {
            let (prefix, val) = match i {
                0 => (&b"Rule: "[..], &self.rule),
                1 => (&b"Src IP: "[..], &self.srcip),
                2 => (&b"Src Port: "[..], &self.srcport),
                3 => (&b"Dst IP: "[..], &self.dstip),
                4 => (&b"Dst Port: "[..], &self.dstport),
                _ => (&b"User: "[..], &self.user),
            };
            if let Some(v) = val {
                out.extend_from_slice(prefix);
                out.extend_from_slice(v);
                out.push(b'\n');
            }
        }
        for l in &self.logs {
            out.extend_from_slice(l);
            out.push(b'\n');
        }
    }
}
