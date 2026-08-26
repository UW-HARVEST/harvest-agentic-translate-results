//! Shared differential-test harness.
//!
//! Both the C `libdriver.so` and the Rust `libdriver.so` are loaded with
//! `libloading` and driven purely through their exported symbols, so the
//! `#[no_mangle]` wrappers are part of what is under test.

#![allow(dead_code, non_camel_case_types, non_snake_case)]

use std::ffi::{c_char, c_int, c_long, c_uint, c_void, CString};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct FILE {
    _p: [u8; 0],
}

pub const SEEK_SET: c_int = 0;
pub const SEEK_CUR: c_int = 1;
pub const SEEK_END: c_int = 2;

extern "C" {
    pub static mut stderr: *mut FILE;

    pub fn free(p: *mut c_void);
    pub fn malloc(n: usize) -> *mut c_void;
    pub fn strlen(s: *const c_char) -> usize;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fdopen(fd: c_int, mode: *const c_char) -> *mut FILE;
    pub fn fmemopen(buf: *mut c_void, size: usize, mode: *const c_char) -> *mut FILE;
    pub fn fclose(f: *mut FILE) -> c_int;
    pub fn fflush(f: *mut FILE) -> c_int;
    pub fn ftell(f: *mut FILE) -> c_long;
    pub fn fseek(f: *mut FILE, off: c_long, whence: c_int) -> c_int;
    pub fn feof(f: *mut FILE) -> c_int;
    pub fn ferror(f: *mut FILE) -> c_int;
    pub fn fileno(f: *mut FILE) -> c_int;

    pub fn pipe(fds: *mut c_int) -> c_int;
    pub fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    pub fn read(fd: c_int, buf: *mut c_void, n: usize) -> isize;
    pub fn close(fd: c_int) -> c_int;
    pub fn dup(fd: c_int) -> c_int;
    pub fn dup2(old: c_int, new: c_int) -> c_int;
    pub fn open(path: *const c_char, flags: c_int, mode: c_uint) -> c_int;
    pub fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;

    pub fn fork() -> c_int;
    pub fn waitpid(pid: c_int, status: *mut c_int, opts: c_int) -> c_int;
    pub fn _exit(code: c_int) -> !;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

// ---------------------------------------------------------------------------
// C struct mirrors (validated against the C sizes by `tests/symbols.rs`)
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
pub struct timespec {
    pub tv_sec: i64,
    pub tv_nsec: i64,
}

/// glibc x86_64 `struct stat`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct stat {
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: c_uint,
    pub st_uid: c_uint,
    pub st_gid: c_uint,
    pub __pad0: c_int,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_atim: timespec,
    pub st_mtim: timespec,
    pub st_ctim: timespec,
    pub __glibc_reserved: [i64; 3],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Tm {
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

impl Default for Tm {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

pub const MAX_FQUEUE: usize = 256;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FileQueue {
    pub last_change: i64,
    pub year: c_int,
    pub day: c_int,
    pub flags: c_int,
    pub mon: [c_char; 4],
    pub file_name: [c_char; MAX_FQUEUE + 1],
    pub fp: *mut FILE,
    pub f_status: stat,
}

impl FileQueue {
    pub fn zeroed() -> Self {
        unsafe { std::mem::zeroed() }
    }
    /// Fill every byte with `b` — used to prove the re-initialisation order.
    pub fn dirty(b: u8) -> Self {
        let mut q = Self::zeroed();
        unsafe {
            std::ptr::write_bytes(&mut q as *mut FileQueue as *mut u8, b, size_of::<FileQueue>());
            q.fp = std::ptr::null_mut();
        }
        q
    }
}

#[repr(C)]
pub struct AlertData {
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
// Loaded library surface
// ---------------------------------------------------------------------------

pub type FnOsCalloc = unsafe extern "C" fn(usize, usize) -> *mut c_void;
pub type FnOsRealloc = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;
pub type FnOsStrdup = unsafe extern "C" fn(*const c_char) -> *mut c_char;
pub type FnMerror = unsafe extern "C" fn(*const c_char, *const c_char, c_int, *const c_char);
pub type FnGetAlertData = unsafe extern "C" fn(c_int, *mut FILE) -> *mut AlertData;
pub type FnFreeAlertData = unsafe extern "C" fn(*mut AlertData);
pub type FnInitFileQueue = unsafe extern "C" fn(*mut FileQueue, *const Tm, c_int) -> c_int;
pub type FnReadFileMon = unsafe extern "C" fn(*mut FileQueue, *const Tm, c_uint) -> *mut AlertData;
pub type FnDriver = unsafe extern "C" fn(c_int, c_int, c_int, c_uint, c_int) -> *mut AlertData;

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    pub os_calloc: FnOsCalloc,
    pub os_realloc: FnOsRealloc,
    pub os_strdup: FnOsStrdup,
    pub merror: FnMerror,
    pub GetAlertData: FnGetAlertData,
    pub FreeAlertData: FnFreeAlertData,
    pub Init_FileQueue: FnInitFileQueue,
    pub Read_FileMon: FnReadFileMon,
    pub driver: FnDriver,
    _lib: libloading::Library,
}

unsafe fn sym<T: Copy>(lib: &libloading::Library, n: &[u8]) -> T {
    let s: libloading::Symbol<T> = lib
        .get(n)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(n)));
    *s
}

unsafe fn load(name: &'static str, path: PathBuf) -> Api {
    let lib = libloading::Library::new(&path)
        .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
    Api {
        name,
        os_calloc: sym(&lib, b"os_calloc\0"),
        os_realloc: sym(&lib, b"os_realloc\0"),
        os_strdup: sym(&lib, b"os_strdup\0"),
        merror: sym(&lib, b"merror\0"),
        GetAlertData: sym(&lib, b"GetAlertData\0"),
        FreeAlertData: sym(&lib, b"FreeAlertData\0"),
        Init_FileQueue: sym(&lib, b"Init_FileQueue\0"),
        Read_FileMon: sym(&lib, b"Read_FileMon\0"),
        driver: sym(&lib, b"driver\0"),
        path,
        _lib: lib,
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built: {} — run:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// The Rust cdylib that cargo just built, found relative to the test executable
/// (`target/<profile>/deps/<test>-<hash>` → `libdriver.so` in `deps/` or its
/// parent), so it works for both `--debug` and `--release` runs.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO={} does not exist", p.display());
        return p;
    }
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe.parent().expect("exe dir").to_path_buf();
    // Prefer the profile the test binary itself was built into.
    let profile_dir = dir.parent().map(|p| p.to_path_buf());
    let mut cands = vec![dir.join("libdriver.so")];
    if let Some(pd) = &profile_dir {
        cands.push(pd.join("libdriver.so"));
    }
    cands.push(manifest_dir().join("target/debug/libdriver.so"));
    cands.push(manifest_dir().join("target/release/libdriver.so"));
    for cand in &cands {
        if cand.exists() {
            return cand.clone();
        }
    }
    panic!(
        "Rust libdriver.so not found (looked in {cands:?}).\n\
         `cargo test` does not build a `crate-type = [\"cdylib\"]` target — run\n\
         `cargo build` (or `./run_tests.sh`) first, or point DRIVER_RUST_SO at the .so."
    );
}

static APIS: OnceLock<(Api, Api)> = OnceLock::new();

/// `(c, rust)`
pub fn apis() -> (&'static Api, &'static Api) {
    let (c, r) = APIS.get_or_init(|| unsafe {
        (load("C", c_so_path()), load("RUST", rust_so_path()))
    });
    (c, r)
}

// ---------------------------------------------------------------------------
// Comparable snapshots
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Clone)]
pub struct AlertSnap {
    pub rule: u32,
    pub level: u32,
    pub alertid: Option<Vec<u8>>,
    pub date: Option<Vec<u8>>,
    pub location: Option<Vec<u8>>,
    pub comment: Option<Vec<u8>>,
    pub group: Option<Vec<u8>>,
    pub srcip: Option<Vec<u8>>,
    pub srcport: i32,
    pub dstip: Option<Vec<u8>>,
    pub dstport: i32,
    pub user: Option<Vec<u8>>,
    pub filename: Option<Vec<u8>>,
}

fn show(o: &Option<Vec<u8>>) -> String {
    match o {
        None => "NULL".to_string(),
        Some(v) => format!("{:?}", String::from_utf8_lossy(v)),
    }
}

impl std::fmt::Debug for AlertSnap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AlertSnap {{ rule: {}, level: {}, alertid: {}, date: {}, location: {}, \
             comment: {}, group: {}, srcip: {}, srcport: {}, dstip: {}, dstport: {}, \
             user: {}, filename: {} }}",
            self.rule,
            self.level,
            show(&self.alertid),
            show(&self.date),
            show(&self.location),
            show(&self.comment),
            show(&self.group),
            show(&self.srcip),
            self.srcport,
            show(&self.dstip),
            self.dstport,
            show(&self.user),
            show(&self.filename),
        )
    }
}

unsafe fn cstr(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        None
    } else {
        Some(std::slice::from_raw_parts(p as *const u8, strlen(p)).to_vec())
    }
}

/// Snapshot then free through the *same* library that produced the value.
pub unsafe fn take_alert(api: &Api, p: *mut AlertData) -> Option<AlertSnap> {
    if p.is_null() {
        return None;
    }
    let a = &*p;
    let snap = AlertSnap {
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
    };
    (api.FreeAlertData)(p);
    Some(snap)
}

/// Everything about a `file_queue` that the C defines. `fp` is compared only by
/// NULL-ness (the pointer value is necessarily different) plus the stream state
/// captured separately by [`StreamState`].
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct QueueSnap {
    pub last_change: i64,
    pub year: i32,
    pub day: i32,
    pub flags: i32,
    pub mon: [u8; 4],
    pub file_name: Vec<u8>,
    pub fp_null: bool,
    pub st_dev: u64,
    pub st_ino: u64,
    pub st_nlink: u64,
    pub st_mode: u32,
    pub st_uid: u32,
    pub st_gid: u32,
    pub st_rdev: u64,
    pub st_size: i64,
    pub st_blksize: i64,
    pub st_blocks: i64,
    pub st_mtim_sec: i64,
    pub st_mtim_nsec: i64,
    pub st_ctim_sec: i64,
}

pub fn snap_queue(q: &FileQueue) -> QueueSnap {
    let mon = [
        q.mon[0] as u8,
        q.mon[1] as u8,
        q.mon[2] as u8,
        q.mon[3] as u8,
    ];
    QueueSnap {
        last_change: q.last_change,
        year: q.year,
        day: q.day,
        flags: q.flags,
        mon,
        // Whole fixed-size buffer, so trailing bytes are compared too.
        file_name: q.file_name.iter().map(|&c| c as u8).collect(),
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
        st_mtim_sec: q.f_status.st_mtim.tv_sec,
        st_mtim_nsec: q.f_status.st_mtim.tv_nsec,
        st_ctim_sec: q.f_status.st_ctim.tv_sec,
    }
}

/// `mon` is written from a `static const char *s_month[12]` indexed by
/// `tm_mon`; for out-of-range `tm_mon` the C read is out of bounds (UB) so the
/// bytes are not comparable. Everything else still is.
pub fn snap_queue_ignoring_mon(q: &FileQueue) -> QueueSnap {
    let mut s = snap_queue(q);
    s.mon = [0; 4];
    s
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct StreamState {
    pub tell: i64,
    pub eof: bool,
    pub err: bool,
}

pub unsafe fn stream_state(f: *mut FILE) -> Option<StreamState> {
    if f.is_null() {
        return None;
    }
    Some(StreamState {
        tell: ftell(f) as i64,
        eof: feof(f) != 0,
        err: ferror(f) != 0,
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed per test for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn below(&mut self, n: u64) -> u64 {
        if n == 0 {
            0
        } else {
            self.next_u64() % n
        }
    }
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }
    pub fn i32_any(&mut self) -> i32 {
        self.next_u64() as i32
    }
    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 13) as u8
    }
    /// Printable-ish token that never contains NUL, `\n`, `:`, `'` or space, so
    /// callers can compose alert lines with predictable structure.
    pub fn token(&mut self, len: usize) -> Vec<u8> {
        const A: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_./-";
        (0..len).map(|_| A[self.below(A.len() as u64) as usize]).collect()
    }
    /// Any non-NUL, non-newline byte (includes high-bit bytes, quotes, colons).
    pub fn wild(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| loop {
                let b = self.byte();
                if b != 0 && b != b'\n' {
                    return b;
                }
            })
            .collect()
    }
    /// `token` with a randomized length in `lo..=hi` (avoids nested borrows).
    pub fn token_len(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = lo + self.below((hi - lo + 1) as u64) as usize;
        self.token(n)
    }
    /// Any non-NUL byte, newline included.
    pub fn wild_nl(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| loop {
                let b = self.byte();
                if b != 0 {
                    return b;
                }
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Filesystem / CWD helpers
// ---------------------------------------------------------------------------

static CWD_LOCK: Mutex<()> = Mutex::new(());
static SCRATCH_SEQ: Mutex<u64> = Mutex::new(0);

/// Serialises every test that depends on the process-wide CWD and gives it a
/// private scratch directory. Both implementations therefore always see the
/// exact same `alerts.log`.
pub struct Scratch {
    _guard: MutexGuard<'static, ()>,
    prev: PathBuf,
    pub dir: PathBuf,
}

impl Scratch {
    pub fn new(tag: &str) -> Scratch {
        let guard = CWD_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let n = {
            let mut s = SCRATCH_SEQ.lock().unwrap_or_else(|e| e.into_inner());
            *s += 1;
            *s
        };
        let prev = std::env::current_dir().expect("cwd");
        let dir = manifest_dir()
            .join("target/scratch")
            .join(format!("{tag}-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch");
        std::env::set_current_dir(&dir).expect("chdir scratch");
        Scratch {
            _guard: guard,
            prev,
            dir,
        }
    }

    pub fn write(&self, name: &str, bytes: &[u8]) {
        std::fs::write(self.dir.join(name), bytes).expect("write scratch file");
    }
    pub fn remove(&self, name: &str) {
        let _ = std::fs::remove_file(self.dir.join(name));
    }
    pub fn exists(&self, name: &str) -> bool {
        self.dir.join(name).exists()
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.prev);
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// `fopen(path, mode)` on a `&str` path.
pub unsafe fn fopen_str(path: &str, mode: &str) -> *mut FILE {
    let p = CString::new(path).unwrap();
    let m = CString::new(mode).unwrap();
    fopen(p.as_ptr(), m.as_ptr())
}

/// A read-only `FILE*` over `bytes`, backed by a real (seekable) file.
pub unsafe fn file_stream(dir: &Path, name: &str, bytes: &[u8]) -> *mut FILE {
    let p = dir.join(name);
    std::fs::write(&p, bytes).expect("write stream file");
    let cp = CString::new(p.to_str().unwrap()).unwrap();
    let m = CString::new("r").unwrap();
    let f = fopen(cp.as_ptr(), m.as_ptr());
    assert!(!f.is_null(), "fopen {} failed", p.display());
    f
}

/// A **non-seekable** read `FILE*` carrying `bytes` (write end closed).
/// `bytes.len()` must stay under the pipe buffer (64 KiB).
pub unsafe fn pipe_stream(bytes: &[u8]) -> *mut FILE {
    assert!(bytes.len() < 60 * 1024);
    let mut fds = [0 as c_int; 2];
    assert_eq!(pipe(fds.as_mut_ptr()), 0);
    if !bytes.is_empty() {
        let n = write(fds[1], bytes.as_ptr() as *const c_void, bytes.len());
        assert_eq!(n as usize, bytes.len());
    }
    close(fds[1]);
    let m = CString::new("r").unwrap();
    let f = fdopen(fds[0], m.as_ptr());
    assert!(!f.is_null());
    f
}

/// `fmemopen` over a leaked copy of `bytes` (so the buffer outlives the FILE).
/// `fileno` on such a stream is -1, which is what makes `fstat` fail.
pub unsafe fn mem_stream(bytes: &[u8]) -> *mut FILE {
    let buf = Box::leak(bytes.to_vec().into_boxed_slice());
    let m = CString::new("r").unwrap();
    let f = fmemopen(buf.as_mut_ptr() as *mut c_void, buf.len(), m.as_ptr());
    assert!(!f.is_null());
    f
}

// ---------------------------------------------------------------------------
// stderr capture
// ---------------------------------------------------------------------------

/// Run `body` with fd 2 redirected into a temporary file and return the bytes
/// written. Serialised by [`CWD_LOCK`]-independent lock so concurrent tests do
/// not steal each other's output.
static STDERR_LOCK: Mutex<()> = Mutex::new(());

pub fn capture_stderr<F: FnOnce()>(body: F) -> Vec<u8> {
    let _g = STDERR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let tmp = std::env::temp_dir().join(format!("difftest-err-{}", std::process::id()));
        let cpath = CString::new(tmp.to_str().unwrap()).unwrap();
        let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600);
        assert!(fd >= 0, "open capture file");
        fflush(stderr);
        let saved = dup(2);
        assert!(saved >= 0);
        assert!(dup2(fd, 2) >= 0);

        body();

        fflush(stderr);
        assert!(dup2(saved, 2) >= 0);
        close(saved);
        lseek(fd, 0, SEEK_SET);
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(fd);
        let _ = std::fs::remove_file(&tmp);
        out
    }
}

// ---------------------------------------------------------------------------
// Child-process helper for the `exit(EXIT_FAILURE)` paths
// ---------------------------------------------------------------------------

#[derive(PartialEq, Eq, Debug)]
pub enum ChildOutcome {
    Exited(i32),
    Signalled(i32),
}

/// Fork, run `body` in the child (it is expected to `exit()`), and report how
/// the child terminated together with whatever it wrote to fd 2.
pub fn run_in_child<F: FnOnce()>(body: F) -> (ChildOutcome, Vec<u8>) {
    let _g = STDERR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    unsafe {
        let tmp = std::env::temp_dir().join(format!(
            "difftest-child-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let cpath = CString::new(tmp.to_str().unwrap()).unwrap();
        let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600);
        assert!(fd >= 0);
        fflush(stderr);

        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            // Child: point fd 2 at the capture file, run, and make sure we never
            // return into the test harness.
            dup2(fd, 2);
            body();
            _exit(66);
        }
        let mut status: c_int = 0;
        assert_eq!(waitpid(pid, &mut status, 0), pid);

        lseek(fd, 0, SEEK_SET);
        let mut out = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            let n = read(fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if n <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..n as usize]);
        }
        close(fd);
        let _ = std::fs::remove_file(&tmp);

        let outcome = if status & 0x7f == 0 {
            ChildOutcome::Exited((status >> 8) & 0xff)
        } else {
            ChildOutcome::Signalled(status & 0x7f)
        };
        (outcome, out)
    }
}

// ---------------------------------------------------------------------------
// Alert-text builders
// ---------------------------------------------------------------------------

/// One well-formed alert as the C parser expects it.
#[derive(Clone, Debug)]
pub struct AlertText {
    pub id: Vec<u8>,
    /// text between the id's `:` and the `-` (`" mail "`, `" "`, ...)
    pub mid: Vec<u8>,
    pub group: Option<Vec<u8>>,
    pub date: Vec<u8>,
    pub location: Vec<u8>,
    pub rule: Option<(Vec<u8>, Vec<u8>, Vec<u8>)>, // (rule, level, comment)
    pub srcip: Option<Vec<u8>>,
    pub srcport: Option<Vec<u8>>,
    pub dstip: Option<Vec<u8>>,
    pub dstport: Option<Vec<u8>>,
    pub user: Option<Vec<u8>>,
    pub body: Vec<Vec<u8>>,
}

impl AlertText {
    pub fn render(&self, out: &mut Vec<u8>) {
        out.extend_from_slice(b"** Alert ");
        out.extend_from_slice(&self.id);
        out.push(b':');
        out.extend_from_slice(&self.mid);
        if let Some(g) = &self.group {
            out.push(b'-');
            out.extend_from_slice(g);
        }
        out.push(b'\n');
        out.extend_from_slice(&self.date);
        out.push(b' ');
        out.extend_from_slice(&self.location);
        out.push(b'\n');
        if let Some((r, l, c)) = &self.rule {
            out.extend_from_slice(b"Rule: ");
            out.extend_from_slice(r);
            out.extend_from_slice(b" (level) ");
            out.extend_from_slice(l);
            out.extend_from_slice(b" -> '");
            out.extend_from_slice(c);
            out.extend_from_slice(b"'\n");
        }
        for (tag, v) in [
            (&b"Src IP: "[..], &self.srcip),
            (&b"Src Port: "[..], &self.srcport),
            (&b"Dst IP: "[..], &self.dstip),
            (&b"Dst Port: "[..], &self.dstport),
            (&b"User: "[..], &self.user),
        ] {
            if let Some(v) = v {
                out.extend_from_slice(tag);
                out.extend_from_slice(v);
                out.push(b'\n');
            }
        }
        for l in &self.body {
            out.extend_from_slice(l);
            out.push(b'\n');
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        let mut v = Vec::new();
        self.render(&mut v);
        v
    }
}

/// A randomized, fully populated, well-formed alert.
pub fn rand_alert(rng: &mut Rng, mail: bool) -> AlertText {
    let id = {
        let mut v = rng.token_len(1, 6);
        // an alertid must not itself contain a ':' (that would move `strstr`)
        v.retain(|&b| b != b':');
        if v.is_empty() {
            v.push(b'7');
        }
        v
    };
    let mid: Vec<u8> = if mail {
        b" mail ".to_vec()
    } else {
        match rng.below(3) {
            0 => b" ".to_vec(),
            1 => b" noemail ".to_vec(),
            _ => b" x ".to_vec(),
        }
    };
    let group = if rng.below(4) == 0 {
        None
    } else {
        let mut g = rng.token_len(1, 12);
        if rng.below(3) == 0 {
            g.extend_from_slice(b",syscheck,");
        }
        Some(g)
    };
    let date = format!(
        "20{:02} {} {:02} {:02}:{:02}:{:02}",
        rng.below(30),
        ["Jan", "Feb", "Mar", "Jul", "Dec"][rng.below(5) as usize],
        1 + rng.below(28),
        rng.below(24),
        rng.below(60),
        rng.below(60)
    )
    .into_bytes();
    let mut location = rng.token_len(1, 20);
    location.retain(|&b| b != b':');
    let rule = if rng.below(6) == 0 {
        None
    } else {
        let mut comment = rng.token_len(1, 30);
        comment.retain(|&b| b != b'\'');
        Some((
            format!("{}", rng.below(100000)).into_bytes(),
            format!("{}", rng.below(20)).into_bytes(),
            comment,
        ))
    };
    let ip = |rng: &mut Rng| {
        format!(
            "{}.{}.{}.{}",
            rng.below(256),
            rng.below(256),
            rng.below(256),
            rng.below(256)
        )
        .into_bytes()
    };
    AlertText {
        id,
        mid,
        group,
        date,
        location,
        rule,
        srcip: if rng.below(3) == 0 { None } else { Some(ip(rng)) },
        srcport: if rng.below(3) == 0 {
            None
        } else {
            Some(format!("{}", rng.below(65536)).into_bytes())
        },
        dstip: if rng.below(3) == 0 { None } else { Some(ip(rng)) },
        dstport: if rng.below(3) == 0 {
            None
        } else {
            Some(format!("{}", rng.below(65536)).into_bytes())
        },
        user: if rng.below(3) == 0 {
            None
        } else {
            Some(rng.token_len(1, 8))
        },
        body: (0..rng.below(4))
            .map(|_| {
                let mut l = rng.token_len(1, 40);
                // keep body lines from accidentally looking like a token line
                if l.starts_with(b"Rule") || l.starts_with(b"User") {
                    l.insert(0, b'z');
                }
                l
            })
            .collect(),
    }
}

// ---------------------------------------------------------------------------
// Differential drivers
// ---------------------------------------------------------------------------

/// Call `GetAlertData(flag, fp)` repeatedly on the *same* byte stream in both
/// libraries and compare the whole sequence of results plus the stream state
/// after every call.
pub struct ReadRun {
    pub alerts: Vec<Option<AlertSnap>>,
    pub states: Vec<Option<StreamState>>,
}

pub unsafe fn read_all(api: &Api, f: *mut FILE, flag: c_int, max_calls: usize) -> ReadRun {
    let mut alerts = Vec::new();
    let mut states = Vec::new();
    for _ in 0..max_calls {
        let p = (api.GetAlertData)(flag, f);
        let a = take_alert(api, p);
        let stop = a.is_none();
        alerts.push(a);
        states.push(stream_state(f));
        if stop {
            break;
        }
    }
    ReadRun { alerts, states }
}

/// Byte-for-byte differential over a `GetAlertData` stream, for every stream
/// kind requested.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    File,
    Pipe,
    Mem,
}

/// Process-wide scratch directory that needs no `chdir` (so `GetAlertData`
/// differentials do not have to serialise on the CWD lock).
pub fn tmp_dir() -> &'static Path {
    static D: OnceLock<PathBuf> = OnceLock::new();
    D.get_or_init(|| {
        let d = manifest_dir()
            .join("target/scratch")
            .join(format!("streams-{}", std::process::id()));
        std::fs::create_dir_all(&d).expect("create stream dir");
        d
    })
    .as_path()
}

pub fn diff_get_alert_data(tag: &str, bytes: &[u8], flag: c_int, kinds: &[Kind], max_calls: usize) {
    let (c, r) = apis();
    let dir = tmp_dir();
    for &kind in kinds {
        unsafe {
            let mk = |n: &str| -> *mut FILE {
                match kind {
                    Kind::File => file_stream(dir, n, bytes),
                    Kind::Pipe => pipe_stream(bytes),
                    Kind::Mem => mem_stream(bytes),
                }
            };
            let fc = mk("c.log");
            let cr = read_all(c, fc, flag, max_calls);
            fclose(fc);

            let fr = mk("r.log");
            let rr = read_all(r, fr, flag, max_calls);
            fclose(fr);

            assert_eq!(
                cr.alerts,
                rr.alerts,
                "[{tag}] kind={kind:?} flag={flag:#x} alert mismatch\ninput = {:?}",
                String::from_utf8_lossy(bytes)
            );
            // A pipe has no meaningful ftell; compare eof/err only.
            let norm = |s: &Vec<Option<StreamState>>| -> Vec<Option<(i64, bool, bool)>> {
                s.iter()
                    .map(|o| {
                        o.as_ref().map(|s| {
                            if kind == Kind::Pipe {
                                (0, s.eof, s.err)
                            } else {
                                (s.tell, s.eof, s.err)
                            }
                        })
                    })
                    .collect()
            };
            assert_eq!(
                norm(&cr.states),
                norm(&rr.states),
                "[{tag}] kind={kind:?} flag={flag:#x} stream-state mismatch\ninput = {:?}",
                String::from_utf8_lossy(bytes)
            );
        }
    }
}
