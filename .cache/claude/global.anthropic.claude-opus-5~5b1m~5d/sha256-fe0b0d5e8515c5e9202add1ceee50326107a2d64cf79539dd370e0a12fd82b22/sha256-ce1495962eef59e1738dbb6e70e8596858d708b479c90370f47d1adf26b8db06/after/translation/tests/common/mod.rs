//! Shared differential-testing harness.
//!
//! Both the C `libdriver.so` and the Rust `libdriver.so` are loaded with
//! `libloading` and driven exclusively through their exported C symbols, so the
//! `#[no_mangle] extern "C"` wrappers are part of what is under test.

#![allow(dead_code, non_snake_case, non_camel_case_types, non_upper_case_globals)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void};
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

/* ------------------------------------------------------------------ */
/* libc                                                               */
/* ------------------------------------------------------------------ */

#[repr(C)]
pub struct FILE {
    _o: [u8; 0],
}

pub const SEEK_SET: c_int = 0;
pub const SEEK_CUR: c_int = 1;
pub const SEEK_END: c_int = 2;

extern "C" {
    pub fn fopen(p: *const c_char, m: *const c_char) -> *mut FILE;
    pub fn fdopen(fd: c_int, m: *const c_char) -> *mut FILE;
    pub fn fclose(f: *mut FILE) -> c_int;
    pub fn fseek(f: *mut FILE, off: c_long, whence: c_int) -> c_int;
    pub fn ftell(f: *mut FILE) -> c_long;
    pub fn feof(f: *mut FILE) -> c_int;
    pub fn ferror(f: *mut FILE) -> c_int;
    pub fn fileno(f: *mut FILE) -> c_int;
    pub fn fflush(f: *mut FILE) -> c_int;
    pub fn free(p: *mut c_void);
    pub fn malloc(n: usize) -> *mut c_void;
    pub fn calloc(n: usize, s: usize) -> *mut c_void;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strerror(e: c_int) -> *mut c_char;
    pub fn __errno_location() -> *mut c_int;
    pub fn dup(fd: c_int) -> c_int;
    pub fn dup2(a: c_int, b: c_int) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn pipe(fds: *mut c_int) -> c_int;
    pub fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
}

pub unsafe fn set_errno(v: c_int) {
    *__errno_location() = v;
}
pub unsafe fn get_errno() -> c_int {
    *__errno_location()
}

/* ------------------------------------------------------------------ */
/* C types                                                            */
/* ------------------------------------------------------------------ */

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct timespec {
    pub tv_sec: c_long,
    pub tv_nsec: c_long,
}

/// glibc x86-64 `struct stat` (144 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct stat_t {
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
#[derive(Clone, Copy, Debug)]
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
    pub fn zeroed() -> tm {
        unsafe { std::mem::zeroed() }
    }
    pub fn new(mday: c_int, mon: c_int, year: c_int) -> tm {
        let mut t = tm::zeroed();
        t.tm_mday = mday;
        t.tm_mon = mon;
        t.tm_year = year;
        t
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
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

pub const MAX_FQUEUE: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct file_queue {
    pub last_change: c_long,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut FILE,
    pub f_status: stat_t,
}

impl file_queue {
    pub fn zeroed() -> file_queue {
        unsafe { std::mem::zeroed() }
    }
}

pub const CRALERT_MAIL_SET: c_int = 0x001;
pub const CRALERT_EXEC_SET: c_int = 0x002;
pub const CRALERT_READ_ALL: c_int = 0x004;
pub const CRALERT_READ_FAILED: c_int = 0x008;
pub const CRALERT_FP_SET: c_int = 0x010;

/* ------------------------------------------------------------------ */
/* Snapshots (comparable, owned)                                      */
/* ------------------------------------------------------------------ */

#[derive(Clone, Debug, PartialEq, Eq)]
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
        let n = strlen(p);
        Some(std::slice::from_raw_parts(p as *const u8, n).to_vec())
    }
}

pub unsafe fn snap_alert(p: *const alert_data) -> Option<AlertSnap> {
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

/// Everything about a `file_queue` that is deterministic and observable.
///
/// `st_atim` and `st_ctim` are excluded: opening/reading a file can update the
/// access time, and any test that has to recreate / re-permission the queue file
/// between the C run and the Rust run necessarily changes the inode change time.
/// Everything the library actually consumes (`st_mtime` -> `last_change`) plus
/// the remaining identifying fields are compared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueSnap {
    pub last_change: c_long,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [u8; 4],
    pub file_name: Vec<u8>,
    /// full 257-byte raw buffer (catches anything written past the NUL)
    pub file_name_raw: Vec<u8>,
    pub fp_null: bool,
    pub st_dev: c_ulong,
    pub st_ino: c_ulong,
    pub st_nlink: c_ulong,
    pub st_mode: c_uint,
    pub st_uid: c_uint,
    pub st_gid: c_uint,
    pub st_rdev: c_ulong,
    pub st_size: c_long,
    pub st_blksize: c_long,
    pub st_blocks: c_long,
    pub st_mtim: timespec,
}

pub unsafe fn snap_queue(q: *const file_queue) -> QueueSnap {
    let q = &*q;
    let raw: Vec<u8> = q.file_name.iter().map(|&c| c as u8).collect();
    let name_len = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
    QueueSnap {
        last_change: q.last_change,
        year: q.year,
        day: q.day,
        flags: q.flags,
        mon: [
            q.mon[0] as u8,
            q.mon[1] as u8,
            q.mon[2] as u8,
            q.mon[3] as u8,
        ],
        file_name: raw[..name_len].to_vec(),
        file_name_raw: raw,
        fp_null: q.fp.is_null(),
        st_dev: q.f_status.st_dev,
        st_ino: q.f_status.st_ino,
        st_nlink: q.f_status.st_nlink,
        st_mode: q.f_status.st_mode,
        st_uid: q.f_status.st_uid,
        st_gid: q.f_status.st_gid,
        st_rdev: q.f_status.st_rdev,
        st_size: q.f_status.st_size,
        st_blksize: q.f_status.st_blksize,
        st_blocks: q.f_status.st_blocks,
        st_mtim: q.f_status.st_mtim,
    }
}

/// Position / flags of a stream, `None` when the pointer is NULL.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamSnap {
    pub pos: c_long,
    pub eof: bool,
    pub err: bool,
}

pub unsafe fn snap_stream(f: *mut FILE) -> Option<StreamSnap> {
    if f.is_null() {
        return None;
    }
    Some(StreamSnap {
        pos: ftell(f),
        eof: feof(f) != 0,
        err: ferror(f) != 0,
    })
}

/* ------------------------------------------------------------------ */
/* The two loaded libraries                                           */
/* ------------------------------------------------------------------ */

pub struct Api {
    pub name: &'static str,
    pub os_calloc: unsafe extern "C" fn(usize, usize) -> *mut c_void,
    pub os_realloc: unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void,
    pub os_strdup: unsafe extern "C" fn(*const c_char) -> *mut c_char,
    pub merror: unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char),
    pub FreeAlertData: unsafe extern "C" fn(*mut alert_data),
    pub GetAlertData: unsafe extern "C" fn(c_int, *mut FILE) -> *mut alert_data,
    pub Init_FileQueue: unsafe extern "C" fn(*mut file_queue, *const tm, c_int) -> c_int,
    pub Read_FileMon: unsafe extern "C" fn(*mut file_queue, *const tm, c_uint) -> *mut alert_data,
    pub driver: unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut alert_data,
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    // Locate the cdylib that belongs to the profile the tests were built with.
    let dir = if cfg!(debug_assertions) {
        "target/debug"
    } else {
        "target/release"
    };
    let p = manifest_dir().join(dir).join("libdriver.so");
    if p.exists() {
        return p;
    }
    for d in ["target/debug", "target/release"] {
        let p = manifest_dir().join(d).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!("no Rust libdriver.so found; run `cargo build` first");
}

unsafe fn load(name: &'static str, path: &PathBuf) -> Api {
    let lib: &'static Library = Box::leak(Box::new(
        Library::new(path).unwrap_or_else(|e| panic!("dlopen {:?}: {e}", path)),
    ));
    macro_rules! sym {
        ($t:ty, $s:expr) => {
            *lib.get::<$t>($s)
                .unwrap_or_else(|e| panic!("{} missing {:?}: {e}", name, $s))
        };
    }
    Api {
        name,
        os_calloc: sym!(unsafe extern "C" fn(usize, usize) -> *mut c_void, b"os_calloc\0"),
        os_realloc: sym!(
            unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void,
            b"os_realloc\0"
        ),
        os_strdup: sym!(
            unsafe extern "C" fn(*const c_char) -> *mut c_char,
            b"os_strdup\0"
        ),
        merror: sym!(
            unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char),
            b"merror\0"
        ),
        FreeAlertData: sym!(unsafe extern "C" fn(*mut alert_data), b"FreeAlertData\0"),
        GetAlertData: sym!(
            unsafe extern "C" fn(c_int, *mut FILE) -> *mut alert_data,
            b"GetAlertData\0"
        ),
        Init_FileQueue: sym!(
            unsafe extern "C" fn(*mut file_queue, *const tm, c_int) -> c_int,
            b"Init_FileQueue\0"
        ),
        Read_FileMon: sym!(
            unsafe extern "C" fn(*mut file_queue, *const tm, c_uint) -> *mut alert_data,
            b"Read_FileMon\0"
        ),
        driver: sym!(
            unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut alert_data,
            b"driver\0"
        ),
    }
}

/// `cargo test --test <x>` does not necessarily relink the `cdylib` (the test
/// binaries do not depend on it), so guard against silently testing a stale
/// `.so`.
fn assert_so_fresh(path: &PathBuf, srcs: &PathBuf) {
    let so = match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };
    let mut newest = None;
    if let Ok(rd) = std::fs::read_dir(srcs) {
        for e in rd.flatten() {
            if let Ok(t) = e.metadata().and_then(|m| m.modified()) {
                if newest.map(|n| t > n).unwrap_or(true) {
                    newest = Some(t);
                }
            }
        }
    }
    if let Some(n) = newest {
        assert!(
            so >= n,
            "{:?} is older than the sources in {:?} -- run `cargo build` (or \
             scripts/run_tests.sh) before `cargo test`",
            path,
            srcs
        );
    }
}

static APIS: OnceLock<(Api, Api)> = OnceLock::new();

/// `(C, Rust)`
pub fn apis() -> &'static (Api, Api) {
    APIS.get_or_init(|| unsafe {
        assert_so_fresh(&rust_so_path(), &manifest_dir().join("src"));
        assert_so_fresh(
            &c_so_path(),
            &manifest_dir().parent().unwrap().join("c_src/src"),
        );
        let c = load("C", &c_so_path());
        let r = load("RUST", &rust_so_path());
        // Warm both libraries up so that lazy PLT resolution / Rust std
        // initialisation cannot perturb `errno` inside a measured call.
        for a in [&c, &r] {
            let p = (a.os_calloc)(1, 1);
            free(p);
        }
        (c, r)
    })
}

pub fn cc() -> &'static Api {
    &apis().0
}
pub fn rs() -> &'static Api {
    &apis().1
}

/* ------------------------------------------------------------------ */
/* Working directory / queue files                                    */
/* ------------------------------------------------------------------ */

static CWD_LOCK: OnceLock<RwLock<()>> = OnceLock::new();

/// chdir into a private scratch directory, once per test process.
fn cwd_lock() -> &'static RwLock<()> {
    CWD_LOCK.get_or_init(|| {
        // A worker sub-process inherits the parent's scratch cwd and must keep
        // using it (the parent holds the lock for the child's whole lifetime).
        if worker_action().is_none() {
            let dir = manifest_dir()
                .join("target")
                .join(format!("testcwd-{}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            std::env::set_current_dir(&dir).unwrap();
        }
        RwLock::new(())
    })
}

/// Makes sure the process has moved into its private scratch directory.
pub fn ensure_cwd() {
    cwd_lock();
}

/// EXCLUSIVE guard. Required by anything that touches the fixed queue file
/// names (`alerts.log`, `<stdin>`) or that redirects fd 2 with
/// [`capture_stderr`] — no other test may be writing to stderr meanwhile.
pub fn guard() -> RwLockWriteGuard<'static, ()> {
    match cwd_lock().write() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// SHARED guard. Required by anything that only uses private scratch files but
/// may emit to stderr (`perror` inside `GetAlertData`), so that it cannot run
/// while another test is capturing fd 2.
pub fn shared() -> RwLockReadGuard<'static, ()> {
    match cwd_lock().read() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

pub const ALERTS_DAILY: &str = "alerts.log";
pub const STDIN_NAME: &str = "<stdin>";

pub fn write_file(name: &str, bytes: &[u8]) {
    std::fs::write(name, bytes).unwrap();
}

pub fn remove_file(name: &str) {
    let _ = std::fs::remove_file(name);
}

/// Writes `content` into a fresh scratch file and returns its (cwd-relative)
/// name. Each caller gets a distinct name so parallel tests cannot collide.
pub fn scratch(tag: &str, content: &[u8]) -> String {
    ensure_cwd();
    let name = format!("scratch-{}-{}.txt", tag, next_id());
    std::fs::write(&name, content).unwrap();
    name
}

fn next_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

pub fn cpath(s: &str) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
    v.push(0);
    v
}

pub fn cbytes(s: &[u8]) -> Vec<c_char> {
    let mut v: Vec<c_char> = s.iter().map(|&b| b as c_char).collect();
    v.push(0);
    v
}

pub unsafe fn open_ro(name: &str) -> *mut FILE {
    let p = cpath(name);
    let m = cpath("r");
    let f = fopen(p.as_ptr(), m.as_ptr());
    assert!(!f.is_null(), "fopen({name}) failed");
    f
}

/* ------------------------------------------------------------------ */
/* stderr capture                                                     */
/* ------------------------------------------------------------------ */

/// Redirects fd 2 to a temporary file, runs `f`, restores fd 2 and returns
/// everything that was written. Caller must already hold [`guard`].
pub unsafe fn capture_stderr<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let path = format!("stderr-{}.log", next_id());
    let file = std::fs::File::create(&path).unwrap();
    use std::os::fd::AsRawFd;
    let saved = dup(2);
    assert!(saved >= 0);
    fflush(std::ptr::null_mut());
    dup2(file.as_raw_fd(), 2);
    let r = f();
    fflush(std::ptr::null_mut());
    dup2(saved, 2);
    close(saved);
    drop(file);
    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    (r, bytes)
}

/* ------------------------------------------------------------------ */
/* deterministic PRNG                                                 */
/* ------------------------------------------------------------------ */

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
    pub fn u32(&mut self) -> u32 {
        self.next_u64() as u32
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as i32
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
    /// Random printable-ish token without `\n`, `\0`.
    pub fn token(&mut self, max: usize) -> Vec<u8> {
        let n = self.below(max as u64 + 1) as usize;
        (0..n)
            .map(|_| {
                let c = 0x20u8 + (self.below(0x5f) as u8);
                if c == b'\n' || c == 0 {
                    b'x'
                } else {
                    c
                }
            })
            .collect()
    }
    /// Random raw bytes (may contain anything except NUL, which `fgets` based
    /// parsing cannot represent in a C string anyway).
    pub fn raw_line(&mut self, max: usize) -> Vec<u8> {
        let n = self.below(max as u64 + 1) as usize;
        (0..n)
            .map(|_| {
                let c = self.below(256) as u8;
                if c == 0 || c == b'\n' {
                    b'?'
                } else {
                    c
                }
            })
            .collect()
    }
}

/* ------------------------------------------------------------------ */
/* Differential drivers                                               */
/* ------------------------------------------------------------------ */

/// Runs `GetAlertData` repeatedly on a freshly opened copy of `content` until
/// it returns NULL (or `max` records were produced), recording every returned
/// record and the stream position after each call.
pub unsafe fn drain_get_alert_data(
    api: &Api,
    tag: &str,
    content: &[u8],
    flag: c_int,
    max: usize,
    start_pos: Option<c_long>,
) -> Vec<(Option<AlertSnap>, Option<StreamSnap>)> {
    let name = scratch(tag, content);
    let fp = open_ro(&name);
    if let Some(p) = start_pos {
        fseek(fp, p, SEEK_SET);
    }
    let mut out = Vec::new();
    for _ in 0..max {
        set_errno(0);
        let a = (api.GetAlertData)(flag, fp);
        let snap = snap_alert(a);
        let stream = snap_stream(fp);
        if !a.is_null() {
            (api.FreeAlertData)(a);
        }
        let done = snap.is_none();
        out.push((snap, stream));
        if done {
            break;
        }
    }
    fclose(fp);
    let _ = std::fs::remove_file(&name);
    out
}

/// Full differential assertion for `GetAlertData` on one input.
pub unsafe fn diff_get_alert_data(content: &[u8], flag: c_int, label: &str) {
    let _s = shared();
    let c = drain_get_alert_data(cc(), "c", content, flag, 24, None);
    let r = drain_get_alert_data(rs(), "r", content, flag, 24, None);
    assert_eq!(
        c.len(),
        r.len(),
        "{label}: record count differs\ninput={:?}\nC={:#?}\nRUST={:#?}",
        String::from_utf8_lossy(content),
        c,
        r
    );
    for (i, (a, b)) in c.iter().zip(r.iter()).enumerate() {
        assert_eq!(
            a.0,
            b.0,
            "{label}: record #{i} differs (flag={flag:#x})\ninput={:?}",
            String::from_utf8_lossy(content)
        );
        assert_eq!(
            a.1,
            b.1,
            "{label}: stream state after record #{i} differs (flag={flag:#x})\ninput={:?}",
            String::from_utf8_lossy(content)
        );
    }
}

/// Differential `driver()` call. Caller must hold [`guard`] and have set up
/// the queue files.
pub unsafe fn diff_driver(day: c_int, month: c_int, year: c_int, timeout: c_uint, flags: c_int, label: &str) {
    let (cres, cerr) = capture_stderr(|| {
        set_errno(0);
        let p = (cc().driver)(day, month, year, timeout, flags);
        let s = snap_alert(p);
        if !p.is_null() {
            (cc().FreeAlertData)(p);
        }
        s
    });
    let (rres, rerr) = capture_stderr(|| {
        set_errno(0);
        let p = (rs().driver)(day, month, year, timeout, flags);
        let s = snap_alert(p);
        if !p.is_null() {
            (rs().FreeAlertData)(p);
        }
        s
    });
    assert_eq!(
        cres, rres,
        "{label}: driver({day},{month},{year},{timeout},{flags:#x}) result differs"
    );
    assert_eq!(
        String::from_utf8_lossy(&cerr),
        String::from_utf8_lossy(&rerr),
        "{label}: driver({day},{month},{year},{timeout},{flags:#x}) stderr differs"
    );
}

/* ------------------------------------------------------------------ */
/* Alert-text generators                                              */
/* ------------------------------------------------------------------ */

pub fn header(id: &str, tag: &str, groups: &str) -> String {
    format!("** Alert {}: {} - {}\n", id, tag, groups)
}

/// A complete, well-formed alert with every optional field.
pub fn full_alert(id: &str, mail: bool, groups: &str) -> String {
    let mut s = String::new();
    s.push_str(&header(id, if mail { "mail" } else { "no-mail" }, groups));
    s.push_str("2006 Apr 13 16:15:17 myhost->/var/log/auth.log\n");
    s.push_str("Rule: 5715 (level 4) -> 'SSHD authentication success.'\n");
    s.push_str("Src IP: 192.168.0.1\n");
    s.push_str("Src Port: 4321\n");
    s.push_str("Dst IP: 10.0.0.7\n");
    s.push_str("Dst Port: 22\n");
    s.push_str("User: root\n");
    s.push_str("Accepted password for root from 192.168.0.1 port 4321 ssh2\n");
    s
}

/// The randomized alert generator used by the property-style sweeps.
pub fn random_stream(rng: &mut Rng) -> Vec<u8> {
    let n_alerts = rng.below(5);
    let mut out: Vec<u8> = Vec::new();

    if rng.below(4) == 0 {
        // Junk before the first header.
        for _ in 0..rng.below(3) {
            out.extend_from_slice(&rng.token(30));
            out.push(b'\n');
        }
    }

    for i in 0..n_alerts {
        // ---- header ----
        match rng.below(10) {
            0 => out.extend_from_slice(b"** Alert\n"),
            1 => out.extend_from_slice(b"** Alert 1234567890.123 no colon here\n"),
            2 => out.extend_from_slice(b"** Alert 1234567890.123:nospaceafter\n"),
            _ => {
                let id = format!("{}.{}", 1500000000u64 + rng.below(9_000_000), rng.below(1000));
                let tag = match rng.below(4) {
                    0 => "mail".to_string(),
                    1 => "mail active-response".to_string(),
                    2 => "".to_string(),
                    _ => String::from_utf8_lossy(&rng.token(6)).to_string(),
                };
                let groups = match rng.below(5) {
                    0 => "syscheck,".to_string(),
                    1 => "ossec,syscheck,pci_dss_11.5,".to_string(),
                    2 => "authentication_success,pci_dss_10.2.5,".to_string(),
                    3 => String::from_utf8_lossy(&rng.token(20)).to_string(),
                    _ => "".to_string(),
                };
                if rng.below(6) == 0 {
                    // header without the '-' group separator
                    out.extend_from_slice(format!("** Alert {}: {}\n", id, tag).as_bytes());
                } else {
                    let spaces = " ".repeat(rng.below(4) as usize);
                    out.extend_from_slice(
                        format!("** Alert {}: {} -{}{}\n", id, tag, spaces, groups).as_bytes(),
                    );
                }
            }
        }

        // ---- date / location line ----
        match rng.below(12) {
            0 => out.extend_from_slice(b"no colon and no space\n"),
            1 => out.extend_from_slice(b"colon:butnospace\n"),
            _ => {
                let loc = match rng.below(3) {
                    0 => "(agent) 10.0.0.1->syscheck".to_string(),
                    1 => "/var/log/messages".to_string(),
                    _ => String::from_utf8_lossy(&rng.token(24)).to_string(),
                };
                out.extend_from_slice(
                    format!(
                        "20{:02} {} {:02} {:02}:{:02}:{:02} {}\n",
                        rng.below(30),
                        ["Jan", "Feb", "Mar", "Dec"][rng.below(4) as usize],
                        1 + rng.below(28),
                        rng.below(24),
                        rng.below(60),
                        rng.below(60),
                        loc
                    )
                    .as_bytes(),
                );
            }
        }

        // ---- body ----
        let nbody = rng.below(7);
        for _ in 0..nbody {
            match rng.below(11) {
                0 => out.extend_from_slice(
                    format!(
                        "Rule: {} (level {}) -> '{}'\n",
                        rng.below(100000),
                        rng.below(20),
                        String::from_utf8_lossy(&rng.token(20)).replace('\'', "q")
                    )
                    .as_bytes(),
                ),
                1 => out.extend_from_slice(b"Rule: 1 (level\n"),
                2 => out.extend_from_slice(b"Rule: 12 (level 7) -> no quotes here\n"),
                3 => out.extend_from_slice(b"Rule: 12 (level 7) -> 'unterminated\n"),
                4 => out.extend_from_slice(
                    format!("Src IP: {}\n", String::from_utf8_lossy(&rng.token(18))).as_bytes(),
                ),
                5 => out.extend_from_slice(
                    format!("Src Port: {}\n", String::from_utf8_lossy(&rng.token(12))).as_bytes(),
                ),
                6 => out.extend_from_slice(
                    format!("Dst IP: {}\n", String::from_utf8_lossy(&rng.token(18))).as_bytes(),
                ),
                7 => out.extend_from_slice(
                    format!("Dst Port: {}\n", String::from_utf8_lossy(&rng.token(12))).as_bytes(),
                ),
                8 => out.extend_from_slice(
                    format!("User: {}\n", String::from_utf8_lossy(&rng.token(14))).as_bytes(),
                ),
                9 => out.extend_from_slice(
                    format!(
                        "Integrity checksum changed for: '{}'\n",
                        String::from_utf8_lossy(&rng.token(20))
                    )
                    .as_bytes(),
                ),
                _ => {
                    out.extend_from_slice(&rng.token(40));
                    out.push(b'\n');
                }
            }
        }
        let _ = i;
    }

    // Sometimes drop the final newline.
    if rng.below(4) == 0 {
        while out.last() == Some(&b'\n') {
            out.pop();
        }
    }
    out
}

/* ------------------------------------------------------------------ */
/* Sub-process worker (for exit()/SIGSEGV paths)                      */
/* ------------------------------------------------------------------ */

pub const WORKER_ENV: &str = "DRIVER_DIFF_WORKER";
pub const WORKER_TEST: &str = "zz_subprocess_worker";

#[derive(Debug)]
pub struct ChildOutcome {
    pub status: Option<i32>,
    pub signal: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Re-executes this very test binary, running only the `zz_subprocess_worker`
/// test, with `DRIVER_DIFF_WORKER=<action>` set.
pub fn run_worker(action: &str) -> ChildOutcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().unwrap();
    let out = std::process::Command::new(exe)
        .arg("--exact")
        .arg(WORKER_TEST)
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(WORKER_ENV, action)
        .env("RUST_BACKTRACE", "0")
        .current_dir(std::env::current_dir().unwrap())
        .output()
        .expect("spawn worker");
    ChildOutcome {
        status: out.status.code(),
        signal: out.status.signal(),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    }
}

/// Runs the same worker action against the C and the Rust `.so` and asserts the
/// process-level outcome (exit status / fatal signal / emitted text) matches.
pub fn diff_worker(action: &str) {
    let c = run_worker(&format!("c:{action}"));
    let r = run_worker(&format!("rust:{action}"));
    assert_eq!(
        (c.status, c.signal),
        (r.status, r.signal),
        "worker {action}: exit differs\nC={:#?}\nRUST={:#?}",
        c,
        r
    );
    assert_eq!(
        worker_payload(&c.stdout),
        worker_payload(&r.stdout),
        "worker {action}: stdout payload differs\nC={:#?}\nRUST={:#?}",
        c,
        r
    );
    assert_eq!(
        worker_payload(&c.stderr),
        worker_payload(&r.stderr),
        "worker {action}: stderr payload differs\nC={:#?}\nRUST={:#?}",
        c,
        r
    );
}

/// True when the Rust `.so` under test was built with `debug-assertions` off
/// (i.e. the release cdylib). Rust's debug builds insert a UB-check that turns a
/// null-pointer dereference into `SIGABRT`, whereas the C (and the release Rust
/// build) raise `SIGSEGV`; the strict signal comparison therefore only applies
/// to the release library.
pub fn rust_so_is_release() -> bool {
    rust_so_path().to_string_lossy().contains("/release/")
}

/// For UB paths (`__attribute__((nonnull))` violated): both libraries must die
/// abnormally, and — for the release Rust build — with the very same signal.
pub fn diff_worker_fatal(action: &str) {
    let c = run_worker(&format!("c:{action}"));
    let r = run_worker(&format!("rust:{action}"));
    assert!(
        c.signal.is_some() || c.status.map(|s| s != 0).unwrap_or(true),
        "worker {action}: C did not die: {c:#?}"
    );
    assert!(
        r.signal.is_some() || r.status.map(|s| s != 0).unwrap_or(true),
        "worker {action}: RUST did not die: {r:#?}"
    );
    if rust_so_is_release() {
        assert_eq!(
            (c.status, c.signal),
            (r.status, r.signal),
            "worker {action}: fatal outcome differs\nC={c:#?}\nRUST={r:#?}"
        );
    }
}

/// The worker brackets everything it wants compared between `@@BEGIN@@` and
/// `@@END@@` so that libtest's own chatter is ignored.
pub fn worker_payload(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("@@BEGIN@@") {
        rest = &rest[i + 9..];
        match rest.find("@@END@@") {
            Some(j) => {
                out.push_str(&rest[..j]);
                rest = &rest[j + 7..];
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }
    out
}

pub fn emit(s: &str) {
    print!("@@BEGIN@@{s}@@END@@");
    use std::io::Write;
    std::io::stdout().flush().unwrap();
}

/// Returns the requested worker action, if this process is a worker.
pub fn worker_action() -> Option<String> {
    std::env::var(WORKER_ENV).ok()
}

/// Resolves `"c:<rest>"` / `"rust:<rest>"` to the matching [`Api`].
pub fn worker_api<'a>(action: &'a str) -> (&'static Api, &'a str) {
    match action.split_once(':') {
        Some(("c", rest)) => (cc(), rest),
        Some(("rust", rest)) => (rs(), rest),
        other => panic!("bad worker action {other:?}"),
    }
}
