//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls every entry point
//! only through its exported C symbol — the Rust crate is never linked or
//! called directly, so the `#[no_mangle] extern "C"` wrappers are under test
//! too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------- C types ---

/// `typedef struct { char description[256]; int priority; } Task;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Task {
    pub description: [c_char; 256],
    pub priority: c_int,
}

/// `typedef struct { Task *tasks; int max_tasks; int task_count; } TaskManager;`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct TaskManager {
    pub tasks: *mut Task,
    pub max_tasks: c_int,
    pub task_count: c_int,
}

const _: () = assert!(std::mem::size_of::<Task>() == 260);
const _: () = assert!(std::mem::size_of::<TaskManager>() == 16);

// ------------------------------------------------------------ libc we need --

extern "C" {
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn unsetenv(name: *const c_char) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open output stream in the process — both
    /// libraries share the same glibc, so this flushes both log files and stdout.
    fn fflush(stream: *mut c_void) -> c_int;
    fn chdir(path: *const c_char) -> c_int;
    fn getcwd(buf: *mut c_char, size: usize) -> *mut c_char;
}

pub fn cd(path: &std::path::Path) {
    let p = CString::new(path.to_str().unwrap()).unwrap();
    assert_eq!(unsafe { chdir(p.as_ptr()) }, 0, "chdir to {path:?} failed");
}

pub fn cwd() -> PathBuf {
    let mut buf = vec![0u8; 4096];
    let p = unsafe { getcwd(buf.as_mut_ptr() as *mut c_char, buf.len()) };
    assert!(!p.is_null(), "getcwd failed");
    let s = unsafe { CStr::from_ptr(p) }.to_str().unwrap().to_string();
    PathBuf::from(s)
}

pub fn set_env(name: &str, value: &str) {
    let n = CString::new(name).unwrap();
    let v = CString::new(value).unwrap();
    unsafe {
        setenv(n.as_ptr(), v.as_ptr(), 1);
    }
}

pub fn unset_env(name: &str) {
    let n = CString::new(name).unwrap();
    unsafe {
        unsetenv(n.as_ptr());
    }
}

pub fn flush_all() {
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

// ------------------------------------------------------------- the two libs --

/// One dynamically loaded implementation of the library.
pub struct Lib {
    pub name: &'static str,
    lib: Library,
}

pub type FnInitializeLogger = unsafe extern "C" fn() -> c_int;
pub type FnLog = unsafe extern "C" fn(*const c_char);
pub type FnFinalizeLogger = unsafe extern "C" fn();
pub type FnCreateTaskManager = unsafe extern "C" fn() -> *mut TaskManager;
pub type FnAddTask = unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int);
pub type FnPrintTasks = unsafe extern "C" fn(*const TaskManager);
pub type FnDestroyTaskManager = unsafe extern "C" fn(*mut TaskManager);
pub type FnDriver = unsafe extern "C" fn(*const c_char) -> c_int;

macro_rules! getter {
    ($name:ident, $ty:ty, $sym:literal) => {
        pub fn $name(&self) -> $ty {
            unsafe {
                let s: Symbol<$ty> = self
                    .lib
                    .get($sym)
                    .unwrap_or_else(|e| panic!("{}: missing symbol {:?}: {e}", self.name, $sym));
                *s
            }
        }
    };
}

impl Lib {
    getter!(initialize_logger, FnInitializeLogger, b"initialize_logger\0");
    getter!(log_info, FnLog, b"log_info\0");
    getter!(log_warning, FnLog, b"log_warning\0");
    getter!(log_error, FnLog, b"log_error\0");
    getter!(finalize_logger, FnFinalizeLogger, b"finalize_logger\0");
    getter!(
        create_task_manager,
        FnCreateTaskManager,
        b"create_task_manager\0"
    );
    getter!(add_task, FnAddTask, b"add_task\0");
    getter!(print_tasks, FnPrintTasks, b"print_tasks\0");
    getter!(
        destroy_task_manager,
        FnDestroyTaskManager,
        b"destroy_task_manager\0"
    );
    getter!(driver, FnDriver, b"driver\0");
}

pub struct Pair {
    pub c: Lib,
    pub rust: Lib,
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .unwrap()
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB") {
        return PathBuf::from(p);
    }
    let md = manifest_dir();
    let rel = md.join("target/release/libdriver.so");
    if rel.exists() {
        return rel;
    }
    md.join("target/debug/libdriver.so")
}

static LIBS: OnceLock<Pair> = OnceLock::new();
static LOCK: Mutex<()> = Mutex::new(());

/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` library, so a
/// stale `.so` would silently make every differential test vacuous. Refuse to
/// run if the `.so` is older than any Rust source or than the C `.so`'s inputs.
fn assert_fresh(so: &Path, sources: &[PathBuf], what: &str) {
    let so_time = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {so:?}: {e}"));
    for src in sources {
        let Ok(md) = std::fs::metadata(src) else {
            continue;
        };
        let t = md.modified().expect("mtime");
        if t > so_time {
            panic!(
                "STALE {what}: {so:?} is older than {src:?}.\n\
                 Rebuild first:  cargo build --release  (and cmake --build c_src/build)"
            );
        }
    }
}

fn rust_sources() -> Vec<PathBuf> {
    let dir = manifest_dir().join("src");
    std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "rs").unwrap_or(false))
        .collect()
}

fn c_sources() -> Vec<PathBuf> {
    let root = manifest_dir().parent().unwrap().join("c_src");
    let mut v = Vec::new();
    for sub in ["src", "include"] {
        v.extend(
            std::fs::read_dir(root.join(sub))
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path()),
        );
    }
    v
}

/// The whole library is built on process-global state (`static FILE *log_file`,
/// `getenv`, fd 1). Every test therefore serialises on this guard.
pub fn libs() -> (&'static Pair, MutexGuard<'static, ()>) {
    let guard = match LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };
    let pair = LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(cp.exists(), "C .so not found at {cp:?} — build c_src first");
        assert!(rp.exists(), "Rust .so not found at {rp:?} — cargo build first");
        assert_fresh(&rp, &rust_sources(), "Rust .so");
        assert_fresh(&cp, &c_sources(), "C .so");
        unsafe {
            Pair {
                c: Lib {
                    name: "C",
                    lib: Library::new(&cp).expect("dlopen C .so"),
                },
                rust: Lib {
                    name: "Rust",
                    lib: Library::new(&rp).expect("dlopen Rust .so"),
                },
            }
        }
    });
    (pair, guard)
}

// ------------------------------------------------------- stdout redirection --

/// Runs `f` with fd 1 **and** fd 2 redirected into temp files and returns the raw
/// bytes written to each. Works for both libraries because both use the
/// process's single glibc `printf`/`fprintf`.
///
/// stderr must be captured too: the C writes to it in `initialize_logger`
/// (`Failed to open log file: %s`) and in `driver` (`Error: Failed to allocate
/// memory for task.`), and those bytes are part of the observable behaviour.
pub fn capture_out_err<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>, Vec<u8>) {
    use std::os::unix::io::AsRawFd;
    let out_path = tmp_path("stdout");
    let err_path = tmp_path("stderr");
    let out_file = std::fs::File::create(&out_path).expect("create stdout capture file");
    let err_file = std::fs::File::create(&err_path).expect("create stderr capture file");
    flush_all();
    let saved_out = unsafe { dup(1) };
    let saved_err = unsafe { dup(2) };
    assert!(saved_out >= 0 && saved_err >= 0, "dup failed");
    unsafe {
        assert!(dup2(out_file.as_raw_fd(), 1) >= 0, "dup2(1) failed");
        assert!(dup2(err_file.as_raw_fd(), 2) >= 0, "dup2(2) failed");
    }
    let r = f();
    flush_all();
    unsafe {
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);
    }
    drop(out_file);
    drop(err_file);
    let out = std::fs::read(&out_path).expect("read stdout capture");
    let err = std::fs::read(&err_path).expect("read stderr capture");
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&err_path);
    (r, out, err)
}

/// Convenience wrapper for call sites that only need stdout, but which still
/// swallow stderr so it does not pollute the test log.
pub fn capture_stdout<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let (r, out, _err) = capture_out_err(f);
    (r, out)
}

// ------------------------------------------------------------- temp helpers --

static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn tmp_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("cdiff-{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

pub fn tmp_path(tag: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    tmp_dir().join(format!("{tag}-{n}"))
}

/// A pair of fresh log-file paths, one per implementation.
pub struct LogPaths {
    pub c: PathBuf,
    pub rust: PathBuf,
}

pub fn fresh_logs(tag: &str) -> LogPaths {
    let c = tmp_path(&format!("{tag}-c.log"));
    let rust = tmp_path(&format!("{tag}-rust.log"));
    let _ = std::fs::remove_file(&c);
    let _ = std::fs::remove_file(&rust);
    LogPaths { c, rust }
}

pub fn read_log(p: &PathBuf) -> Vec<u8> {
    std::fs::read(p).unwrap_or_default()
}

// ---------------------------------------------------------------- reporting --

pub fn show(b: &[u8]) -> String {
    String::from_utf8_lossy(b).escape_debug().to_string()
}

pub fn assert_bytes_eq(what: &str, ctx: &str, c: &[u8], r: &[u8]) {
    if c != r {
        panic!(
            "{what} differs [{ctx}]\n  C   ({} bytes): {}\n  Rust({} bytes): {}",
            c.len(),
            show(c),
            r.len(),
            show(r)
        );
    }
}

/// Byte-compares one `Task` slot in full (all 260 bytes are deterministic:
/// `strncpy` zero-pads to 255 and `[255]` is forced to `\0`).
pub fn task_bytes(t: &Task) -> Vec<u8> {
    let mut v = Vec::with_capacity(260);
    v.extend(t.description.iter().map(|&c| c as u8));
    v.extend(t.priority.to_ne_bytes());
    v
}

pub unsafe fn snapshot(m: *const TaskManager) -> (c_int, c_int, Vec<Vec<u8>>) {
    let max = (*m).max_tasks;
    let count = (*m).task_count;
    let mut tasks = Vec::new();
    for i in 0..count.max(0) {
        tasks.push(task_bytes(&*(*m).tasks.offset(i as isize)));
    }
    (max, count, tasks)
}

pub fn assert_manager_eq(ctx: &str, c: *const TaskManager, r: *const TaskManager) {
    unsafe {
        assert_eq!(c.is_null(), r.is_null(), "NULL-ness differs [{ctx}]");
        if c.is_null() {
            return;
        }
        let (cm, cc, ct) = snapshot(c);
        let (rm, rc, rt) = snapshot(r);
        assert_eq!(cm, rm, "max_tasks differs [{ctx}]");
        assert_eq!(cc, rc, "task_count differs [{ctx}]");
        assert_eq!(ct.len(), rt.len(), "task slot count differs [{ctx}]");
        for (i, (a, b)) in ct.iter().zip(rt.iter()).enumerate() {
            assert_bytes_eq(&format!("task[{i}] bytes"), ctx, a, b);
        }
    }
}

// ------------------------------------------------------------------- random --

/// xorshift64* — deterministic, dependency-free.
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
    pub fn range(&mut self, lo: u64, hi_inclusive: u64) -> u64 {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Printable-ASCII string of the requested length.
    pub fn ascii(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| (0x20 + self.below(0x5f) as u8) as u8)
            .collect()
    }
    /// Arbitrary non-NUL bytes (0x01..=0xFF), also excludes `\n` so the caller
    /// controls line structure.
    pub fn bytes_no_nl(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| loop {
                let b = self.range(1, 255) as u8;
                if b != b'\n' {
                    return b;
                }
            })
            .collect()
    }
    /// Printable-ASCII string of a randomized length in `lo..=hi`.
    pub fn ascii_between(&mut self, lo: u64, hi: u64) -> Vec<u8> {
        let n = self.range(lo, hi) as usize;
        self.ascii(n)
    }
    /// Arbitrary non-NUL, non-`\n` bytes of a randomized length in `lo..=hi`.
    pub fn bytes_no_nl_between(&mut self, lo: u64, hi: u64) -> Vec<u8> {
        let n = self.range(lo, hi) as usize;
        self.bytes_no_nl(n)
    }
}
pub fn cstr(bytes: &[u8]) -> CString {
    CString::new(bytes.to_vec()).expect("no interior NUL")
}

pub fn cstr_of(s: &str) -> CString {
    CString::new(s).unwrap()
}

pub fn from_cstr(p: *const c_char) -> Vec<u8> {
    unsafe { CStr::from_ptr(p).to_bytes().to_vec() }
}
