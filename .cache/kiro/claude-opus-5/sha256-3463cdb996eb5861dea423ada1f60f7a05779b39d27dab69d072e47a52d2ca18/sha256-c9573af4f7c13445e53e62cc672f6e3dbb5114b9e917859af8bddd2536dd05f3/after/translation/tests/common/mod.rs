//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! called only through their exported `extern "C"` symbols — the Rust crate is
//! never linked directly, so the `#[no_mangle]` wrappers are under test too.

#![allow(dead_code)]
#![allow(non_camel_case_types)]

use std::ffi::{CString, c_char, c_int, c_long, c_uint, c_ulong, c_void};
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs (NOT the library under test)
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

unsafe extern "C" {
    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fdopen(fd: c_int, mode: *const c_char) -> *mut FILE;
    pub fn fclose(fp: *mut FILE) -> c_int;
    pub fn fseek(fp: *mut FILE, off: c_long, whence: c_int) -> c_int;
    pub fn ftell(fp: *mut FILE) -> c_long;
    pub fn feof(fp: *mut FILE) -> c_int;
    pub fn ferror(fp: *mut FILE) -> c_int;
    pub fn fflush(fp: *mut FILE) -> c_int;
    pub fn free(p: *mut c_void);
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strdup(s: *const c_char) -> *mut c_char;
    pub fn dup(fd: c_int) -> c_int;
    pub fn dup2(old: c_int, new: c_int) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn pipe(fds: *mut c_int) -> c_int;
    pub fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    pub static mut stderr: *mut FILE;
}

pub const SEEK_SET: c_int = 0;
pub const SEEK_CUR: c_int = 1;
pub const SEEK_END: c_int = 2;

// ---------------------------------------------------------------------------
// errno normalisation
// ---------------------------------------------------------------------------
//
// `read-alert.c` calls `perror()` on two of its error paths, and `perror`
// appends `": " + strerror(errno)`. Neither implementation ever sets `errno`
// itself on those paths, so the message depends purely on the *ambient* errno
// the caller happened to leave behind. The harness therefore pins errno to a
// known value immediately before every library call, so the two
// implementations are compared under identical ambient state — and the pinned
// value itself becomes a test axis (see `err_alert.rs::e20/e21`).

unsafe extern "C" {
    fn __errno_location() -> *mut c_int;
}

pub fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v };
}

pub fn get_errno() -> c_int {
    unsafe { *__errno_location() }
}

static PRESET_ERRNO: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

pub fn preset_errno() -> c_int {
    PRESET_ERRNO.load(std::sync::atomic::Ordering::Relaxed)
}

/// Pin the ambient `errno` used for subsequent library calls. Caller must hold
/// `world()` because this is process-global.
pub fn set_preset_errno(v: c_int) {
    PRESET_ERRNO.store(v, std::sync::atomic::Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// ABI structs (must match c_src exactly)
// ---------------------------------------------------------------------------

pub const MAX_FQUEUE: usize = 256;

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
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub __pad0: u32,
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
    pub last_change: i64,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut FILE,
    pub f_status: stat,
}

impl file_queue {
    /// `memset(&fq, 0, sizeof(file_queue))` — exactly what `driver.c` does.
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

// ---------------------------------------------------------------------------
// Comparable snapshots
// ---------------------------------------------------------------------------

/// Owned, comparable copy of an `alert_data` (or `None` for a NULL return).
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AlertSnap {
    pub rule: c_uint,
    pub level: c_uint,
    pub alertid: Option<Vec<u8>>,
    pub date: Option<Vec<u8>>,
    pub location: Option<Vec<u8>>,
    pub comment: Option<Vec<u8>>,
    pub group: Option<Vec<u8>>,
    pub srcip: Option<Vec<u8>>,
    pub srcport: c_int,
    pub dstip: Option<Vec<u8>>,
    pub dstport: c_int,
    pub user: Option<Vec<u8>>,
    pub filename: Option<Vec<u8>>,
}

unsafe fn cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        unsafe { Some(std::slice::from_raw_parts(p as *const u8, strlen(p)).to_vec()) }
    }
}

pub unsafe fn snap_alert(p: *const alert_data) -> Option<AlertSnap> {
    if p.is_null() {
        return None;
    }
    unsafe {
        let a = &*p;
        Some(AlertSnap {
            rule: a.rule,
            level: a.level,
            alertid: cstr(a.alertid),
            date: cstr(a.date),
            location: cstr(a.location),
            comment: cstr(a.comment),
            group: cstr(a.group),
            srcip: cstr(a.srcip),
            srcport: a.srcport,
            dstip: cstr(a.dstip),
            dstport: a.dstport,
            user: cstr(a.user),
            filename: cstr(a.filename),
        })
    }
}

/// Comparable copy of the observable `file_queue` state after a call.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct QueueSnap {
    pub last_change: i64,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [u8; 4],
    pub file_name: Vec<u8>,
    /// raw `file_name` bytes, all 257 of them (catches stray writes)
    pub file_name_raw: Vec<u8>,
    pub fp_is_null: bool,
    /// `ftell(fp)`, or -1 when `fp` is NULL. Catches "seek to end" vs "leave at
    /// offset 0", which is the whole point of the `CRALERT_READ_ALL` flag.
    pub fp_pos: c_long,
    pub st_size: c_long,
    pub st_mtime: c_long,
}

pub unsafe fn snap_queue(q: &file_queue) -> QueueSnap {
    let raw: Vec<u8> = q.file_name.iter().map(|&c| c as u8).collect();
    let nul = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
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
        file_name: raw[..nul].to_vec(),
        file_name_raw: raw,
        fp_is_null: q.fp.is_null(),
        fp_pos: if q.fp.is_null() {
            -1
        } else {
            unsafe { ftell(q.fp) }
        },
        st_size: q.f_status.st_size,
        st_mtime: q.f_status.st_mtim.tv_sec,
    }
}

// ---------------------------------------------------------------------------
// Loading both shared objects
// ---------------------------------------------------------------------------

pub type FnDriver = unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut alert_data;
pub type FnInitFileQueue = unsafe extern "C" fn(*mut file_queue, *const tm, c_int) -> c_int;
pub type FnReadFileMon = unsafe extern "C" fn(*mut file_queue, *const tm, c_uint) -> *mut alert_data;
pub type FnGetAlertData = unsafe extern "C" fn(c_int, *mut FILE) -> *mut alert_data;
pub type FnFreeAlertData = unsafe extern "C" fn(*mut alert_data);
pub type FnMerror = unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char);
pub type FnOsCalloc = unsafe extern "C" fn(usize, usize) -> *mut c_void;
pub type FnOsRealloc = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnOsStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;

pub struct Lib {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: &'static libloading::Library,
    pub driver: FnDriver,
    pub init_file_queue: FnInitFileQueue,
    pub read_file_mon: FnReadFileMon,
    pub get_alert_data: FnGetAlertData,
    pub free_alert_data: FnFreeAlertData,
    pub merror: FnMerror,
    pub os_calloc: FnOsCalloc,
    pub os_realloc: FnOsRealloc,
    pub os_strdup: FnOsStrdup,
}

impl Lib {
    fn open(name: &'static str, path: PathBuf) -> Lib {
        let lib: &'static libloading::Library = Box::leak(Box::new(unsafe {
            libloading::Library::new(&path)
                .unwrap_or_else(|e| panic!("dlopen {} ({:?}) failed: {e}", name, path))
        }));
        unsafe fn sym<T: Copy>(lib: &libloading::Library, n: &[u8], who: &str) -> T {
            unsafe {
                *lib.get::<T>(n)
                    .unwrap_or_else(|e| panic!("{who}: missing symbol {:?}: {e}", n))
            }
        }
        unsafe {
            Lib {
                name,
                driver: sym(lib, b"driver\0", name),
                init_file_queue: sym(lib, b"Init_FileQueue\0", name),
                read_file_mon: sym(lib, b"Read_FileMon\0", name),
                get_alert_data: sym(lib, b"GetAlertData\0", name),
                free_alert_data: sym(lib, b"FreeAlertData\0", name),
                merror: sym(lib, b"merror\0", name),
                os_calloc: sym(lib, b"os_calloc\0", name),
                os_realloc: sym(lib, b"os_realloc\0", name),
                os_strdup: sym(lib, b"os_strdup\0", name),
                path,
                _lib: lib,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libdriver.so")
}

/// Path to the Rust `.so` under test.
///
/// `cargo test` does **not** build `crate-type = ["cdylib"]` artifacts, so a
/// naive harness happily loads a stale `libdriver.so` from an earlier build and
/// reports green on code that no longer exists. We therefore build it here and
/// then assert the artifact really is newer than every Rust source file.
///
/// `DRIVER_RUST_SO` overrides both steps so the suite can be re-run against a
/// specific profile's artifact (see `scripts/run_all.sh`).
pub fn rust_so_path() -> PathBuf {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
            let p = PathBuf::from(p);
            assert!(p.exists(), "DRIVER_RUST_SO={p:?} does not exist");
            assert_fresh(&p);
            return p;
        }
        // Capture cargo's output rather than inheriting it: this function also
        // runs inside `exit_helper`, whose stderr is compared byte for byte.
        let out = Command::new(env!("CARGO"))
            .args(["build"])
            .current_dir(manifest_dir())
            .output()
            .expect("spawn cargo build");
        assert!(
            out.status.success(),
            "cargo build failed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let p = manifest_dir().join("target/debug/libdriver.so");
        assert!(p.exists(), "cargo build did not produce {p:?}");
        assert_fresh(&p);
        p
    })
    .clone()
}

/// Fail loudly if the `.so` predates any Rust source file.
fn assert_fresh(so: &std::path::Path) {
    let so_t = std::fs::metadata(so).unwrap().modified().unwrap();
    let src = manifest_dir().join("src");
    for e in std::fs::read_dir(&src).unwrap().flatten() {
        let p = e.path();
        if p.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        let t = std::fs::metadata(&p).unwrap().modified().unwrap();
        assert!(
            t <= so_t,
            "STALE ARTIFACT: {p:?} is newer than {so:?}. \
             `cargo test` does not rebuild cdylibs — rebuild before testing."
        );
    }
}

static LIBS: OnceLock<(Lib, Lib)> = OnceLock::new();

/// `(c, rust)`
pub fn libs() -> &'static (Lib, Lib) {
    LIBS.get_or_init(|| {
        (
            Lib::open("C", c_so_path().canonicalize().expect("C .so not built")),
            Lib::open("RUST", rust_so_path()),
        )
    })
}

// ---------------------------------------------------------------------------
// Process-wide resources: cwd (for "alerts.log") and fd 2 (for stderr capture)
// ---------------------------------------------------------------------------

static WORLD: Mutex<()> = Mutex::new(());

/// Serializes everything that touches process-global state: the current
/// directory (the library hardcodes the relative name `alerts.log`) and fd 2.
pub fn world() -> MutexGuard<'static, ()> {
    match WORLD.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

static CWD: OnceLock<PathBuf> = OnceLock::new();

/// chdir into a private scratch directory (idempotent) and return it.
pub fn scratch_dir() -> &'static PathBuf {
    CWD.get_or_init(|| {
        let d = std::env::temp_dir().join(format!(
            "driver-diff-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        std::env::set_current_dir(&d).unwrap();
        d
    })
}

/// Fixed mtime stamped onto every file the harness creates, so that the two
/// implementations always observe an identical `struct stat` even when the
/// 5 s `file_sleep()` separates their runs.
pub const PINNED_MTIME: c_long = 1_500_000_000;

#[repr(C)]
pub struct timeval {
    pub tv_sec: c_long,
    pub tv_usec: c_long,
}

unsafe extern "C" {
    pub fn utimes(path: *const c_char, times: *const timeval) -> c_int;
}

/// `utimes(path, {PINNED_MTIME, PINNED_MTIME})`
pub fn pin_mtime(path: &std::path::Path) {
    let c = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let times = [
        timeval {
            tv_sec: PINNED_MTIME,
            tv_usec: 0,
        },
        timeval {
            tv_sec: PINNED_MTIME,
            tv_usec: 0,
        },
    ];
    let rc = unsafe { utimes(c.as_ptr(), times.as_ptr()) };
    assert_eq!(rc, 0, "utimes({path:?}) failed");
}

/// Write `alerts.log` in the scratch dir. Caller must hold `world()`.
pub fn write_alerts_log(content: &[u8]) {
    let p = scratch_dir().join("alerts.log");
    let mut f = std::fs::File::create(&p).unwrap();
    f.write_all(content).unwrap();
    f.sync_all().unwrap();
    drop(f);
    pin_mtime(&p);
}

pub fn remove_alerts_log() {
    let p = scratch_dir().join("alerts.log");
    let _ = std::fs::remove_file(p);
}

/// Write `content` to a uniquely named file in the scratch dir, return the path.
pub fn temp_file(tag: &str, content: &[u8]) -> PathBuf {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = scratch_dir().join(format!("{tag}-{n}.txt"));
    std::fs::write(&p, content).unwrap();
    pin_mtime(&p);
    p
}

/// `fopen(path, "r")`, panicking on failure.
pub fn open_r(path: &std::path::Path) -> *mut FILE {
    let c = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    let fp = unsafe { fopen(c.as_ptr(), b"r\0".as_ptr() as *const c_char) };
    assert!(!fp.is_null(), "fopen({path:?}) failed");
    fp
}

/// A `FILE*` over a pipe: readable, **not** seekable. Used to force `fseek`
/// failures the same way in both libraries.
pub fn unseekable_stream(content: &[u8]) -> *mut FILE {
    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0);
    assert!(
        content.len() < 60 * 1024,
        "keep pipe payloads under the pipe buffer to avoid blocking"
    );
    unsafe {
        let n = write(fds[1], content.as_ptr() as *const c_void, content.len());
        assert_eq!(n as usize, content.len());
        close(fds[1]);
        let fp = fdopen(fds[0], b"r\0".as_ptr() as *const c_char);
        assert!(!fp.is_null());
        fp
    }
}

/// Run `f` with fd 2 redirected to a temp file; return the captured bytes.
/// Caller must hold `world()`.
pub fn capture_stderr<R>(f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    let path = scratch_dir().join(format!(
        "stderr-{}.bin",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let file = std::fs::File::create(&path).unwrap();
    let tmpfd = {
        use std::os::fd::AsRawFd;
        file.as_raw_fd()
    };
    unsafe {
        fflush(stderr);
        let saved = dup(2);
        assert!(saved >= 0);
        assert!(dup2(tmpfd, 2) >= 0);
        let r = f();
        fflush(stderr);
        assert!(dup2(saved, 2) >= 0);
        close(saved);
        drop(file);
        let bytes = std::fs::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        (r, bytes)
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_DEAD_BEEF;

pub struct Rng(u64);

impl Rng {
    pub fn new(extra: u64) -> Rng {
        Rng(SEED ^ extra.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
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
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
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
    /// Printable-ish token with no NUL and no newline.
    pub fn token(&mut self, max: usize) -> Vec<u8> {
        const A: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOP0123456789.:-_/'\" \t*";
        let n = 1 + self.below(max);
        (0..n).map(|_| *self.pick(A)).collect()
    }
}

// ---------------------------------------------------------------------------
// The core differential primitives
// ---------------------------------------------------------------------------

/// Result of one `GetAlertData` call, everything observable about it.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GadOutcome {
    pub alert: Option<AlertSnap>,
    pub ftell: c_long,
    pub feof: c_int,
    pub ferror: c_int,
}

/// Call `GetAlertData(flag, fp)` on `lib` against a fresh stream over
/// `content`, starting at `start_off`, and snapshot everything observable.
pub fn gad_on_file(lib: &Lib, flag: c_int, content: &[u8], start_off: c_long) -> GadOutcome {
    let path = temp_file("gad", content);
    let fp = open_r(&path);
    if start_off != 0 {
        unsafe { fseek(fp, start_off, SEEK_SET) };
    }
    let out = unsafe {
        set_errno(preset_errno());
        let a = (lib.get_alert_data)(flag, fp);
        let snap = snap_alert(a);
        let o = GadOutcome {
            alert: snap,
            ftell: ftell(fp),
            feof: feof(fp),
            ferror: ferror(fp),
        };
        if !a.is_null() {
            (lib.free_alert_data)(a);
        }
        fclose(fp);
        o
    };
    let _ = std::fs::remove_file(&path);
    out
}

/// Differentially compare one `GetAlertData` configuration.
pub fn assert_gad_eq(flag: c_int, content: &[u8], start_off: c_long, what: &str) {
    let (c, r) = libs();
    let a = gad_on_file(c, flag, content, start_off);
    let b = gad_on_file(r, flag, content, start_off);
    if a != b {
        panic!(
            "DIVERGENCE [{what}] flag={flag:#x} start={start_off}\n\
             --- input ({} bytes) ---\n{}\n--- C   -> {:#?}\n--- RUST -> {:#?}",
            content.len(),
            String::from_utf8_lossy(content),
            a,
            b
        );
    }
}

/// Call `GetAlertData` repeatedly on one stream until it returns NULL (max
/// `cap` calls), collecting every outcome. Exercises the `fseek`-back path.
pub fn gad_drain(lib: &Lib, flag: c_int, content: &[u8], cap: usize) -> Vec<GadOutcome> {
    let path = temp_file("drain", content);
    let fp = open_r(&path);
    let mut out = Vec::new();
    unsafe {
        for _ in 0..cap {
            set_errno(preset_errno());
            let a = (lib.get_alert_data)(flag, fp);
            let snap = snap_alert(a);
            let done = a.is_null();
            out.push(GadOutcome {
                alert: snap,
                ftell: ftell(fp),
                feof: feof(fp),
                ferror: ferror(fp),
            });
            if !a.is_null() {
                (lib.free_alert_data)(a);
            }
            if done {
                break;
            }
        }
        fclose(fp);
    }
    let _ = std::fs::remove_file(&path);
    out
}

pub fn assert_drain_eq(flag: c_int, content: &[u8], cap: usize, what: &str) {
    let (c, r) = libs();
    let a = gad_drain(c, flag, content, cap);
    let b = gad_drain(r, flag, content, cap);
    if a != b {
        panic!(
            "DIVERGENCE [{what}] (drain) flag={flag:#x}\n\
             --- input ---\n{}\n--- C   -> {:#?}\n--- RUST -> {:#?}",
            String::from_utf8_lossy(content),
            a,
            b
        );
    }
}

// ---------------------------------------------------------------------------
// Alert-file builders
// ---------------------------------------------------------------------------

/// A well-formed single alert, as produced by Wazuh's `alerts.log`.
pub fn alert_block(id: &str, group: &str, date_loc: &str, body: &[&str]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(format!("** Alert {id}: mail - {group}\n").as_bytes());
    v.extend_from_slice(date_loc.as_bytes());
    v.push(b'\n');
    for line in body {
        v.extend_from_slice(line.as_bytes());
        v.push(b'\n');
    }
    v
}

pub const MINIMAL: &str = concat!(
    "** Alert 1461102540.1234: mail - syslog,errors,\n",
    "2016 Apr 19 20:29:00 myhost->/var/log/messages\n",
    "Rule: 1002 (level 7) -> 'Unknown problem somewhere in the system.'\n",
    "Apr 19 20:28:59 myhost kernel: something bad\n",
);

// ---------------------------------------------------------------------------
// Streams with hostile properties (for the ERRORS.md rows)
// ---------------------------------------------------------------------------

unsafe extern "C" {
    pub fn fileno(fp: *mut FILE) -> c_int;
    pub fn fgetc(fp: *mut FILE) -> c_int;
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;
}

pub const O_WRONLY: c_int = 1;

unsafe extern "C" {
    #[link_name = "fgets"]
    pub fn fgets_raw(buf: *mut c_char, n: c_int, fp: *mut FILE) -> *mut c_char;
}

/// A readable stream whose first `fgets` calls are served from the stdio buffer
/// but whose next `read(2)` fails: the file is opened, one byte is consumed to
/// force the buffer fill, then the underlying descriptor is replaced by a
/// write-only one. `fgets` therefore eventually returns NULL with `ferror` set
/// and `feof` clear — the only way to reach `read-alert.c`'s
/// "fell out of the loop but `!feof`" path.
///
/// `content` must be smaller than one stdio buffer so a single `read` slurps it.
pub fn error_stream(content: &[u8]) -> *mut FILE {
    assert!(content.len() < 4000);
    let path = temp_file("errstream", content);
    let fp = open_r(&path);
    unsafe {
        // Force the buffer fill (reads up to BUFSIZ from the real file).
        let first = fgetc(fp);
        assert!(first >= 0);
        // Swap in a write-only descriptor: subsequent read(2) => EBADF.
        let devnull = open(b"/dev/null\0".as_ptr() as *const c_char, O_WRONLY);
        assert!(devnull >= 0);
        assert!(dup2(devnull, fileno(fp)) >= 0);
        close(devnull);
    }
    let _ = std::fs::remove_file(&path);
    fp
}

/// Generic single-shot `GetAlertData` over a caller-provided stream factory.
pub fn gad_on_stream(lib: &Lib, flag: c_int, mk: &dyn Fn() -> *mut FILE) -> GadOutcome {
    let fp = mk();
    unsafe {
        set_errno(preset_errno());
        let a = (lib.get_alert_data)(flag, fp);
        let snap = snap_alert(a);
        let o = GadOutcome {
            alert: snap,
            ftell: ftell(fp),
            feof: feof(fp),
            ferror: ferror(fp),
        };
        if !a.is_null() {
            (lib.free_alert_data)(a);
        }
        fclose(fp);
        o
    }
}

pub fn assert_stream_eq(flag: c_int, mk: &dyn Fn() -> *mut FILE, what: &str) -> GadOutcome {
    let (c, r) = libs();
    let a = gad_on_stream(c, flag, mk);
    let b = gad_on_stream(r, flag, mk);
    assert_eq!(a, b, "DIVERGENCE [{what}] flag={flag:#x}");
    a
}

// ---------------------------------------------------------------------------
// Out-of-process execution (for paths that exit() or crash the process)
// ---------------------------------------------------------------------------

use std::process::Command;

/// Locate the `exit_helper` binary cargo built alongside this test binary.
///
/// The helper is a separate `[[test]]` target, so `cargo test --test <other>`
/// does not rebuild it. A stale helper would silently answer "unknown case"
/// for both implementations and turn a real gap into a false pass, so we ask
/// cargo to (re)build it exactly once per test process.
pub fn helper_bin() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let out = Command::new(env!("CARGO"))
                .args(["test", "--no-run", "--test", "exit_helper"])
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .output()
                .expect("spawn cargo to build exit_helper");
            assert!(
                out.status.success(),
                "cargo failed to build exit_helper:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
            find_helper().expect("exit_helper binary not found after building it")
        })
        .clone()
}

fn find_helper() -> Option<PathBuf> {
    let me = std::env::current_exe().expect("current_exe");
    let dir = me.parent().expect("deps dir").to_path_buf();
    let mut cands: Vec<PathBuf> = Vec::new();
    for d in [dir.clone(), dir.parent().unwrap().to_path_buf()] {
        if let Ok(rd) = std::fs::read_dir(&d) {
            for e in rd.flatten() {
                let p = e.path();
                let name = match p.file_name().and_then(|s| s.to_str()) {
                    Some(n) => n,
                    None => continue,
                };
                if !name.starts_with("exit_helper") || p.extension().is_some() {
                    continue;
                }
                if p.is_file() {
                    cands.push(p);
                }
            }
        }
    }
    cands.sort_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    cands.pop()
}

#[derive(Debug, PartialEq, Eq)]
pub struct Run {
    pub code: Option<i32>,
    pub signal: Option<i32>,
    pub stderr: Vec<u8>,
    pub stdout: Vec<u8>,
}

pub fn run_case(which: &str, case: &str) -> Run {
    use std::os::unix::process::ExitStatusExt;
    let out = Command::new(helper_bin())
        .arg(which)
        .arg(case)
        // Resolve the artifact in the PARENT so the child does zero cargo work:
        // any cargo chatter would land in the child's stderr, which we compare
        // byte for byte.
        .env("DRIVER_RUST_SO", rust_so_path())
        .output()
        .expect("spawn exit_helper");
    let r = Run {
        code: out.status.code(),
        signal: out.status.signal(),
        stderr: out.stderr,
        stdout: out.stdout,
    };
    // 97/98 are the helper's own "I don't know that" exits: a stale helper
    // binary would otherwise make every case pass trivially.
    assert!(
        r.code != Some(97) && r.code != Some(98),
        "exit_helper rejected {which}/{case} (stale binary?): {r:?}"
    );
    r
}

/// Run `case` under both implementations and require identical exit status,
/// terminating signal, stdout and stderr.
pub fn assert_helper_same(case: &str) -> Run {
    let c = run_case("c", case);
    let r = run_case("rust", case);
    assert_eq!(
        (c.code, c.signal),
        (r.code, r.signal),
        "[{case}] termination differs: C=code {:?}/sig {:?}  RUST=code {:?}/sig {:?}\n\
         C stderr: {}\nRUST stderr: {}",
        c.code,
        c.signal,
        r.code,
        r.signal,
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        c.stderr,
        r.stderr,
        "[{case}] stderr differs:\nC   = {:?}\nRUST= {:?}",
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
    );
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{case}] stdout differs:\nC   = {:?}\nRUST= {:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout),
    );
    c
}

// ---------------------------------------------------------------------------
// FIFO helper: an `alerts.log` that `fopen` succeeds on but `fseek` fails on
// ---------------------------------------------------------------------------

unsafe extern "C" {
    pub fn mkfifo(path: *const c_char, mode: u32) -> c_int;
    pub fn unlink(path: *const c_char) -> c_int;
}

pub const O_NONBLOCK: c_int = 0o4000;

/// Replace `alerts.log` with a FIFO and hold a writer open while `f` runs, so
/// the library's `fopen("alerts.log","r")` succeeds but `fseek` fails (ESPIPE).
/// Caller must hold `world()`.
pub fn with_fifo_alerts_log<R>(f: impl FnOnce() -> R) -> R {
    let dir = scratch_dir();
    let path = dir.join("alerts.log");
    let _ = std::fs::remove_file(&path);
    let cpath = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { mkfifo(cpath.as_ptr(), 0o600) }, 0, "mkfifo failed");

    // Open a writer without blocking forever: O_WRONLY|O_NONBLOCK returns ENXIO
    // until a reader shows up, so retry from a background thread.
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop2 = stop.clone();
    let cp2 = cpath.clone();
    let writer = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        let mut fds = Vec::new();
        while !stop2.load(std::sync::atomic::Ordering::Relaxed)
            && std::time::Instant::now() < deadline
        {
            let fd = unsafe { open(cp2.as_ptr(), O_WRONLY | O_NONBLOCK) };
            if fd >= 0 {
                fds.push(fd);
            } else {
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
        for fd in fds {
            unsafe { close(fd) };
        }
    });

    let r = f();
    stop.store(true, std::sync::atomic::Ordering::Relaxed);
    let _ = writer.join();
    unsafe { unlink(cpath.as_ptr()) };
    r
}
