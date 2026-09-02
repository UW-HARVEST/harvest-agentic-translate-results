//! Shared harness for the C-vs-Rust differential test suite.
//!
//! Every test loads BOTH shared objects through `libloading` and calls them
//! only through their exported C symbols:
//!
//!   * `../c_src/build/libdriver.so`        (ground truth)
//!   * `target/release/libdriver.so`        (the translation under test)
//!
//! The library's whole observable surface is
//!
//!   1. the function return value,
//!   2. bytes written to `stdout` (`printf` in `print_tasks`),
//!   3. bytes written to `stderr` (`fprintf` in `initialize_logger`/`driver`),
//!   4. bytes written to the log file (`$LOG_FILE` or `./default.log`),
//!   5. process termination for the unchecked-NULL paths,
//!
//! so the harness captures all five and compares them byte-for-byte.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, CString};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// C structs (must be layout-identical to c_src/include/task_manager.h)
// ---------------------------------------------------------------------------

#[repr(C)]
pub struct Task {
    pub description: [c_char; 256],
    pub priority: c_int,
}

#[repr(C)]
pub struct TaskManager {
    pub tasks: *mut Task,
    pub max_tasks: c_int,
    pub task_count: c_int,
}

const _: () = assert!(std::mem::size_of::<Task>() == 260);
const _: () = assert!(std::mem::size_of::<TaskManager>() == 16);

// ---------------------------------------------------------------------------
// libc
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn unsetenv(name: *const c_char) -> c_int;
    fn chdir(path: *const c_char) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn __libc_malloc(size: usize) -> *mut c_void;
    fn __libc_free(ptr: *mut c_void);
    fn geteuid() -> u32;
}

/// True when the process runs as root, in which case some permission-based
/// `fopen` failures cannot be provoked.
pub fn is_root() -> bool {
    unsafe { geteuid() == 0 }
}

// ---------------------------------------------------------------------------
// malloc interposition
// ---------------------------------------------------------------------------
//
// `build.rs` links the test binaries with `-rdynamic`, which puts this `malloc`
// into the executable's dynamic symbol table.  The executable is searched
// before libc when resolving symbols for a dlopen'ed object, so BOTH
// `libdriver.so`s call this function.  It forwards to `__libc_malloc` unless a
// test has explicitly armed a failure for one specific allocation size, which
// is how the C code's `malloc() == NULL` branches get exercised.

static MALLOC_ARMED: AtomicBool = AtomicBool::new(false);
static MALLOC_FAIL_SIZE: AtomicUsize = AtomicUsize::new(0);
static MALLOC_FAILURES: AtomicUsize = AtomicUsize::new(0);

// Allocator tracing.  `malloc`/`free` are the only observable difference for
// changes such as "forgot the `free(manager)` on the tasks-allocation failure
// path", which are invisible in stdout/stderr/log, so the hook also counts.
static TRACE_ON: AtomicBool = AtomicBool::new(false);
static N_MALLOC: AtomicUsize = AtomicUsize::new(0);
static N_FREE: AtomicUsize = AtomicUsize::new(0);
static N_BYTES: AtomicUsize = AtomicUsize::new(0);

/// Allocator event log: `(kind, value)` where kind 0 = malloc(size),
/// 1 = free(ptr), 2 = malloc(size) forced to fail.
static mut TRACE_EV: [(u8, usize); 512] = [(0, 0); 512];
static TRACE_N: AtomicUsize = AtomicUsize::new(0);

#[inline]
fn record(kind: u8, value: usize) {
    let i = TRACE_N.fetch_add(1, Ordering::SeqCst);
    if i < 512 {
        unsafe {
            let p = std::ptr::addr_of_mut!(TRACE_EV) as *mut (u8, usize);
            p.add(i).write((kind, value));
        }
    }
}

/// The allocator events of the most recent traced region.
pub fn trace_events() -> Vec<(u8, usize)> {
    let n = TRACE_N.load(Ordering::SeqCst).min(512);
    unsafe {
        let p = std::ptr::addr_of!(TRACE_EV) as *const (u8, usize);
        (0..n).map(|i| p.add(i).read()).collect()
    }
}

#[no_mangle]
pub unsafe extern "C" fn malloc(size: usize) -> *mut c_void {
    if MALLOC_ARMED.load(Ordering::SeqCst) && size == MALLOC_FAIL_SIZE.load(Ordering::SeqCst) {
        MALLOC_ARMED.store(false, Ordering::SeqCst);
        MALLOC_FAILURES.fetch_add(1, Ordering::SeqCst);
        if TRACE_ON.load(Ordering::SeqCst) {
            // A failed malloc is still an observable request.
            N_BYTES.fetch_add(size, Ordering::SeqCst);
            record(2, size);
        }
        return std::ptr::null_mut();
    }
    let p = __libc_malloc(size);
    if TRACE_ON.load(Ordering::SeqCst) {
        N_MALLOC.fetch_add(1, Ordering::SeqCst);
        N_BYTES.fetch_add(size, Ordering::SeqCst);
        record(0, size);
    }
    p
}

#[no_mangle]
pub unsafe extern "C" fn free(ptr: *mut c_void) {
    if TRACE_ON.load(Ordering::SeqCst) && !ptr.is_null() {
        N_FREE.fetch_add(1, Ordering::SeqCst);
        record(1, ptr as usize);
    }
    __libc_free(ptr)
}

/// Totals of the interposed allocator calls made during one traced region.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct AllocStats {
    pub mallocs: usize,
    pub frees: usize,
    pub bytes: usize,
}

/// Begin counting `malloc`/`free`.  Perform NO Rust allocation until
/// `trace_stop`, or the counts will include the harness's own work.
pub fn trace_start() {
    TRACE_N.store(0, Ordering::SeqCst);
    N_MALLOC.store(0, Ordering::SeqCst);
    N_FREE.store(0, Ordering::SeqCst);
    N_BYTES.store(0, Ordering::SeqCst);
    TRACE_ON.store(true, Ordering::SeqCst);
}

pub fn trace_stop() -> AllocStats {
    TRACE_ON.store(false, Ordering::SeqCst);
    AllocStats {
        mallocs: N_MALLOC.load(Ordering::SeqCst),
        frees: N_FREE.load(Ordering::SeqCst),
        bytes: N_BYTES.load(Ordering::SeqCst),
    }
}

/// Make the next `malloc(size)` (from anywhere in the process) return NULL.
///
/// The window must be kept as small as possible: perform no Rust allocation
/// between `arm_malloc_failure` and the call under test.
pub fn arm_malloc_failure(size: usize) -> usize {
    MALLOC_FAIL_SIZE.store(size, Ordering::SeqCst);
    MALLOC_ARMED.store(true, Ordering::SeqCst);
    MALLOC_FAILURES.load(Ordering::SeqCst)
}

/// Disarm and report whether the armed failure actually fired.
pub fn disarm_malloc_failure(before: usize) -> bool {
    MALLOC_ARMED.store(false, Ordering::SeqCst);
    MALLOC_FAILURES.load(Ordering::SeqCst) > before
}

// ---------------------------------------------------------------------------
// Loaded library surface
// ---------------------------------------------------------------------------

pub type FnCreateTaskManager = unsafe extern "C" fn() -> *mut TaskManager;
pub type FnAddTask = unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int);
pub type FnPrintTasks = unsafe extern "C" fn(*const TaskManager);
pub type FnDestroyTaskManager = unsafe extern "C" fn(*mut TaskManager);
pub type FnInitializeLogger = unsafe extern "C" fn() -> c_int;
pub type FnLog = unsafe extern "C" fn(*const c_char);
pub type FnFinalizeLogger = unsafe extern "C" fn();
pub type FnDriver = unsafe extern "C" fn(*const c_char) -> c_int;

/// The ten exported symbols, resolved from one `.so`.
pub struct Api {
    pub name: &'static str,
    _lib: libloading::Library,
    pub create_task_manager: FnCreateTaskManager,
    pub add_task: FnAddTask,
    pub print_tasks: FnPrintTasks,
    pub destroy_task_manager: FnDestroyTaskManager,
    pub initialize_logger: FnInitializeLogger,
    pub log_info: FnLog,
    pub log_warning: FnLog,
    pub log_error: FnLog,
    pub finalize_logger: FnFinalizeLogger,
    pub driver: FnDriver,
}

pub fn c_so_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../c_src/build/libdriver.so"
    ))
}

pub fn rust_so_path() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/target/release/libdriver.so"
    ))
}

/// Guard against testing a stale shared object.
///
/// `cargo test` does **not** rebuild the `cdylib` artifact at
/// `target/release/libdriver.so` -- it only builds the test harnesses.  So a
/// plain `cargo test --release` after editing `src/` happily runs against the
/// *previous* `.so`, and any tool that leaves a modified `.so` behind (see
/// `mutation_check.sh`) poisons every later run.  Both failure modes produce
/// results that look real but describe a binary nobody asked about, so the
/// harness refuses to run unless each `.so` is newer than its sources.
fn assert_fresh(name: &str, so: &Path) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let (sources, hint): (Vec<PathBuf>, &str) = if name == "C" {
        (
            vec![root.join("../c_src/src"), root.join("../c_src/include")],
            "cd c_src/build && cmake --build .",
        )
    } else {
        (
            vec![root.join("src"), root.join("Cargo.toml")],
            "cd translation && cargo build --release",
        )
    };

    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("cannot stat {}: {e}", so.display()));

    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = sources;
    while let Some(p) = stack.pop() {
        let md = match std::fs::metadata(&p) {
            Ok(md) => md,
            Err(_) => continue,
        };
        if md.is_dir() {
            if let Ok(rd) = std::fs::read_dir(&p) {
                stack.extend(rd.filter_map(|e| e.ok()).map(|e| e.path()));
            }
            continue;
        }
        if let Ok(t) = md.modified() {
            if newest.as_ref().map(|(_, best)| t > *best).unwrap_or(true) {
                newest = Some((p, t));
            }
        }
    }

    if let Some((path, t)) = newest {
        assert!(
            t <= so_mtime,
            "STALE {name} shared object.\n  {} was modified after {} was built.\n  \
             `cargo test` does not rebuild the cdylib; rebuild first:\n    {hint}",
            path.display(),
            so.display()
        );
    }
}

impl Api {
    fn load(name: &'static str, path: &Path) -> Api {
        assert!(
            path.exists(),
            "{} not found.\n  build the C library with:\n    cd c_src && mkdir -p build && cd build \
             && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n  build the Rust \
             library with:\n    cd translation && cargo build --release",
            path.display()
        );
        assert_fresh(name, path);
        unsafe {
            let lib = libloading::Library::new(path).expect("dlopen failed");
            macro_rules! sym {
                ($t:ty, $n:literal) => {{
                    let s: libloading::Symbol<$t> = lib
                        .get($n)
                        .unwrap_or_else(|e| panic!("missing symbol {:?}: {e}", $n));
                    *s
                }};
            }
            Api {
                name,
                create_task_manager: sym!(FnCreateTaskManager, b"create_task_manager"),
                add_task: sym!(FnAddTask, b"add_task"),
                print_tasks: sym!(FnPrintTasks, b"print_tasks"),
                destroy_task_manager: sym!(FnDestroyTaskManager, b"destroy_task_manager"),
                initialize_logger: sym!(FnInitializeLogger, b"initialize_logger"),
                log_info: sym!(FnLog, b"log_info"),
                log_warning: sym!(FnLog, b"log_warning"),
                log_error: sym!(FnLog, b"log_error"),
                finalize_logger: sym!(FnFinalizeLogger, b"finalize_logger"),
                driver: sym!(FnDriver, b"driver"),
                _lib: lib,
            }
        }
    }
}

pub struct Pair {
    pub c: Api,
    pub rust: Api,
}

/// The two libraries, loaded once per test binary.
pub fn pair() -> &'static Pair {
    static P: OnceLock<Pair> = OnceLock::new();
    P.get_or_init(|| Pair {
        c: Api::load("C", &c_so_path()),
        rust: Api::load("Rust", &rust_so_path()),
    })
}

/// fd redirection, `chdir` and `$LOG_FILE`/`$MAX_TASKS` are process-wide, so
/// only one scenario may be in flight at a time.
pub fn serial() -> MutexGuard<'static, ()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    match M.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

// ---------------------------------------------------------------------------
// Configuration + capture
// ---------------------------------------------------------------------------

/// Where the C code should be told to put its log.
#[derive(Clone, Debug)]
pub enum LogTarget {
    /// `$LOG_FILE` unset -> the C falls back to the literal `"default.log"`,
    /// resolved against the current working directory.
    Unset,
    /// `$LOG_FILE` = this literal *relative* path (identical string on both
    /// sides, resolved against each side's private scratch directory).
    Relative(&'static str),
    /// `$LOG_FILE` = `<scratch dir>/<name>` (absolute; differs per side).
    Absolute(&'static str),
    /// `$LOG_FILE` = exactly this string; no log file is read back.
    Raw(String),
}

#[derive(Clone, Debug)]
pub struct Config {
    pub log: LogTarget,
    /// `$MAX_TASKS`; `None` means unset (the C then defaults to 10).
    pub max_tasks: Option<String>,
}

impl Config {
    /// `$LOG_FILE=log.txt`, `$MAX_TASKS` unset (C default of 10).
    pub fn new() -> Config {
        Config {
            log: LogTarget::Relative("log.txt"),
            max_tasks: None,
        }
    }
    pub fn log(mut self, l: LogTarget) -> Config {
        self.log = l;
        self
    }
    pub fn max_tasks(mut self, v: impl Into<String>) -> Config {
        self.max_tasks = Some(v.into());
        self
    }
    /// Effective `max_tasks` the C code will compute (unset -> 10).
    pub fn effective_max_tasks(&self) -> i32 {
        match &self.max_tasks {
            None => 10,
            Some(s) => c_atoi(s.as_bytes()),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::new()
    }
}

/// Everything observable about one run of one implementation.
#[derive(Debug, PartialEq, Eq)]
pub struct Outcome {
    pub ret: i64,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub log: Vec<u8>,
}

fn scratch_root() -> PathBuf {
    let p = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/target/difftest"));
    std::fs::create_dir_all(&p).unwrap();
    p
}

fn next_scratch(tag: &str, side: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::SeqCst);
    let safe: String = tag
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let d = scratch_root().join(format!("{safe}-{side}-{n}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn set_env(cfg: &Config, dir: &Path) -> Option<PathBuf> {
    unsafe {
        match &cfg.max_tasks {
            None => {
                unsetenv(c"MAX_TASKS".as_ptr());
            }
            Some(v) => {
                let v = CString::new(v.as_bytes()).unwrap();
                setenv(c"MAX_TASKS".as_ptr(), v.as_ptr(), 1);
            }
        }
        match &cfg.log {
            LogTarget::Unset => {
                unsetenv(c"LOG_FILE".as_ptr());
                Some(dir.join("default.log"))
            }
            LogTarget::Relative(rel) => {
                let v = CString::new(*rel).unwrap();
                setenv(c"LOG_FILE".as_ptr(), v.as_ptr(), 1);
                Some(dir.join(rel))
            }
            LogTarget::Absolute(name) => {
                let abs = dir.join(name);
                let v = CString::new(abs.to_str().unwrap()).unwrap();
                setenv(c"LOG_FILE".as_ptr(), v.as_ptr(), 1);
                Some(abs)
            }
            LogTarget::Raw(s) => {
                let v = CString::new(s.as_bytes()).unwrap();
                setenv(c"LOG_FILE".as_ptr(), v.as_ptr(), 1);
                None
            }
        }
    }
}

/// Run `f` against one implementation with `cfg` applied, capturing stdout,
/// stderr and the log file.
///
/// `f` is handed the `Api` and returns the value under comparison (0 for
/// scenarios whose functions are all `void`).
pub fn run_one<F>(api: &Api, cfg: &Config, tag: &str, f: F) -> Outcome
where
    F: FnOnce(&Api) -> i64,
{
    let dir = next_scratch(tag, api.name);
    let out_path = dir.join("__stdout");
    let err_path = dir.join("__stderr");
    let log_path = set_env(cfg, &dir);

    let out_file = std::fs::File::create(&out_path).unwrap();
    let err_file = std::fs::File::create(&err_path).unwrap();
    let cwd = std::env::current_dir().unwrap();
    let dir_c = CString::new(dir.to_str().unwrap()).unwrap();
    let cwd_c = CString::new(cwd.to_str().unwrap()).unwrap();

    let ret = unsafe {
        // Flush anything the harness itself buffered so it is not attributed
        // to the library under test.
        fflush(std::ptr::null_mut());
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let _ = std::io::Write::flush(&mut std::io::stderr());

        let saved_out = dup(1);
        let saved_err = dup(2);
        dup2(out_file.as_raw_fd(), 1);
        dup2(err_file.as_raw_fd(), 2);
        chdir(dir_c.as_ptr());

        let ret = f(api);

        // Flush every stdio stream, including a log `FILE *` the scenario left
        // open, so buffered bytes are observable.
        fflush(std::ptr::null_mut());
        chdir(cwd_c.as_ptr());
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);
        ret
    };
    drop(out_file);
    drop(err_file);

    Outcome {
        ret,
        stdout: std::fs::read(&out_path).unwrap_or_default(),
        stderr: std::fs::read(&err_path).unwrap_or_default(),
        log: log_path
            .and_then(|p| std::fs::read(p).ok())
            .unwrap_or_default(),
    }
}

/// Run the same scenario against the C `.so` and the Rust `.so` and assert
/// that every observable byte matches.  Returns the C outcome so a test can
/// additionally assert that the branch it targets really fired.
pub fn assert_same<F>(tag: &str, cfg: &Config, f: F) -> Outcome
where
    F: Fn(&Api) -> i64,
{
    let _g = serial();
    let p = pair();
    let c = run_one(&p.c, cfg, tag, &f);
    let r = run_one(&p.rust, cfg, tag, &f);
    compare(tag, cfg, &c, &r);
    c
}

pub fn compare(tag: &str, cfg: &Config, c: &Outcome, r: &Outcome) {
    if c == r {
        return;
    }
    let mut msg = format!("DIVERGENCE in `{tag}`\n  config: {cfg:?}\n");
    if c.ret != r.ret {
        msg += &format!("  return: C={} Rust={}\n", c.ret, r.ret);
    }
    for (what, a, b) in [
        ("stdout", &c.stdout, &r.stdout),
        ("stderr", &c.stderr, &r.stderr),
        ("log", &c.log, &r.log),
    ] {
        if a != b {
            msg += &format!(
                "  {what} differs ({} vs {} bytes)\n    C   : {}\n    Rust: {}\n",
                a.len(),
                b.len(),
                show(a),
                show(b)
            );
            if let Some(i) = a.iter().zip(b.iter()).position(|(x, y)| x != y) {
                msg += &format!("    first difference at byte {i}\n");
            }
        }
    }
    panic!("{msg}");
}

pub fn show(b: &[u8]) -> String {
    let mut s = String::new();
    for &x in b.iter().take(700) {
        match x {
            b'\n' => s.push_str("\\n"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(x as char),
            _ => s.push_str(&format!("\\x{x:02x}")),
        }
    }
    if b.len() > 700 {
        s.push_str("...<truncated>");
    }
    s
}

// ---------------------------------------------------------------------------
// Crash-equivalence (for the C code's unchecked NULL dereferences)
// ---------------------------------------------------------------------------

/// How a child process that ran a scenario terminated.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Term {
    Exited(i32),
    Signaled(i32),
}

/// Run `f` in a forked child (stdout/stderr sent to `/dev/null`) and report how
/// the child terminated.  Used for the paths where the C code dereferences a
/// NULL pointer or touches a closed `FILE *`.
pub fn term_of<F: FnOnce()>(f: F) -> Term {
    unsafe {
        fflush(std::ptr::null_mut());
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            let devnull = std::fs::File::create("/dev/null").unwrap();
            dup2(devnull.as_raw_fd(), 1);
            dup2(devnull.as_raw_fd(), 2);
            f();
            fflush(std::ptr::null_mut());
            _exit(0);
        }
        let mut status: c_int = 0;
        assert!(waitpid(pid, &mut status, 0) == pid, "waitpid failed");
        if status & 0x7f == 0x7f {
            Term::Signaled((status >> 8) & 0xff)
        } else if status & 0x7f == 0 {
            Term::Exited((status >> 8) & 0xff)
        } else {
            Term::Signaled(status & 0x7f)
        }
    }
}

/// Assert C and Rust terminate identically for a scenario that may crash.
pub fn assert_same_term<F: Fn(&Api)>(tag: &str, cfg: &Config, f: F) -> Term {
    let _g = serial();
    let p = pair();
    let dir = next_scratch(tag, "term");
    set_env(cfg, &dir);
    let cwd = std::env::current_dir().unwrap();
    let dir_c = CString::new(dir.to_str().unwrap()).unwrap();
    let cwd_c = CString::new(cwd.to_str().unwrap()).unwrap();
    unsafe { chdir(dir_c.as_ptr()) };
    let tc = term_of(|| f(&p.c));
    let tr = term_of(|| f(&p.rust));
    unsafe { chdir(cwd_c.as_ptr()) };
    assert_eq!(tc, tr, "termination differs in `{tag}` (config {cfg:?})");
    tc
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) + input generators
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9e37_79b9_7f4a_7c15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % (n as u64)) as usize
    }
    pub fn range(&mut self, lo: usize, hi_inclusive: usize) -> usize {
        lo + self.below(hi_inclusive - lo + 1)
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// An `int` drawn from a distribution that favours boundary values.
    pub fn priority(&mut self) -> i32 {
        match self.below(8) {
            0 => 0,
            1 => 1,
            2 => -1,
            3 => i32::MIN,
            4 => i32::MAX,
            5 => self.below(1000) as i32,
            6 => -(self.below(1000) as i32),
            _ => self.i32(),
        }
    }
    /// A NUL-free byte (any value a C string may contain).
    pub fn nonzero_byte(&mut self) -> u8 {
        match self.below(4) {
            // Bias towards interesting bytes: printf conversion specifiers and
            // non-UTF-8 sequences (which a String-based translation mangles).
            0 => *b"%sdnxc\\\"' \t"
                .get(self.below(10))
                .unwrap_or(&b'%'),
            1 => self.range(0x80, 0xff) as u8,
            _ => self.range(0x01, 0xff) as u8,
        }
    }
    /// A NUL-free C string body of `len` bytes.
    pub fn cstr_body(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| self.nonzero_byte()).collect()
    }
    /// Newline-separated blob for `driver`, `lines` lines of random length.
    pub fn blob(&mut self, lines: usize, max_len: usize) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();
        for i in 0..lines {
            if i > 0 {
                v.push(b'\n');
            }
            let len = self.below(max_len + 1);
            for _ in 0..len {
                let b = self.nonzero_byte();
                if b != b'\n' {
                    v.push(b);
                }
            }
        }
        v
    }
}

/// `CString`-like: a NUL-terminated owned buffer that tolerates any non-NUL byte.
pub fn cstring(body: &[u8]) -> Vec<u8> {
    assert!(!body.contains(&0), "C string body must not contain NUL");
    let mut v = body.to_vec();
    v.push(0);
    v
}

/// glibc `atoi` == `(int)strtol(s, NULL, 10)`; used to predict `max_tasks`.
pub fn c_atoi(bytes: &[u8]) -> i32 {
    let mut i = 0usize;
    while i < bytes.len() && matches!(bytes[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        neg = bytes[i] == b'-';
        i += 1;
    }
    let mut mag: u64 = 0;
    let mut ovf = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        let d = (bytes[i] - b'0') as u64;
        if !ovf {
            match mag.checked_mul(10).and_then(|v| v.checked_add(d)) {
                Some(v) => mag = v,
                None => ovf = true,
            }
        }
        i += 1;
    }
    let as_long: i64 = if neg {
        if ovf || mag >= (i64::MAX as u64) + 1 {
            i64::MIN
        } else {
            -(mag as i64)
        }
    } else if ovf || mag > i64::MAX as u64 {
        i64::MAX
    } else {
        mag as i64
    };
    as_long as i32
}

/// Byte size the C computes for the tasks array: `max_tasks * sizeof(Task)`
/// with the `int -> size_t` conversion and wrap-around the C performs.
pub fn tasks_alloc_bytes(max_tasks: i32) -> usize {
    (max_tasks as isize as usize).wrapping_mul(260)
}
