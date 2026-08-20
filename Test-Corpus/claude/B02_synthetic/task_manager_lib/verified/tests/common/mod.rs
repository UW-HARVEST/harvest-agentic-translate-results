//! Differential-test harness.
//!
//! Both implementations are loaded **as shared objects through `libloading`**
//! and driven exclusively through their exported `extern "C"` symbols — the
//! Rust functions are never called directly, so the `#[no_mangle]` export
//! wrappers are part of what is under test.
//!
//! For every scenario we compare four observables between the two `.so`s:
//!   * the function return value(s),
//!   * every byte written to `stdout` (`printf` from `print_tasks`),
//!   * every byte written to `stderr` (`fprintf(stderr, …)` error messages),
//!   * every byte of the log file the library produced,
//!   * plus, when a `TaskManager` is reachable, a raw byte snapshot of the
//!     struct and of all its `Task` slots (this also pins the C struct layout).

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{CString, c_char, c_int};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// C types (must match c_src/include/task_manager.h exactly)
// ---------------------------------------------------------------------------

pub const DESC_LEN: usize = 256;
pub const TASK_SIZE: usize = 260;
pub const TASKMANAGER_SIZE: usize = 16;
pub const DEFAULT_MAX_TASKS: c_int = 10;
pub const EXIT_FAILURE: c_int = 1;

#[repr(C)]
pub struct Task {
    pub description: [c_char; DESC_LEN],
    pub priority: c_int,
}

#[repr(C)]
pub struct TaskManager {
    pub tasks: *mut Task,
    pub max_tasks: c_int,
    pub task_count: c_int,
}

const _: () = assert!(size_of::<Task>() == TASK_SIZE);
const _: () = assert!(size_of::<TaskManager>() == TASKMANAGER_SIZE);

// ---------------------------------------------------------------------------
// Loaded implementation (one per .so)
// ---------------------------------------------------------------------------

pub type FnInitializeLogger = unsafe extern "C" fn() -> c_int;
pub type FnLog = unsafe extern "C" fn(*const c_char);
pub type FnFinalizeLogger = unsafe extern "C" fn();
pub type FnCreate = unsafe extern "C" fn() -> *mut TaskManager;
pub type FnAddTask = unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int);
pub type FnPrint = unsafe extern "C" fn(*const TaskManager);
pub type FnDestroy = unsafe extern "C" fn(*mut TaskManager);
pub type FnDriver = unsafe extern "C" fn(*const c_char) -> c_int;

pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub initialize_logger: FnInitializeLogger,
    pub log_info: FnLog,
    pub log_warning: FnLog,
    pub log_error: FnLog,
    pub finalize_logger: FnFinalizeLogger,
    pub create_task_manager: FnCreate,
    pub add_task: FnAddTask,
    pub print_tasks: FnPrint,
    pub destroy_task_manager: FnDestroy,
    pub driver: FnDriver,
}

/// The 10 symbols the C `.so` exports; both libraries must provide all of them.
pub const EXPECTED_SYMBOLS: &[&str] = &[
    "initialize_logger",
    "log_info",
    "log_warning",
    "log_error",
    "finalize_logger",
    "create_task_manager",
    "add_task",
    "print_tasks",
    "destroy_task_manager",
    "driver",
];

impl Api {
    pub fn load(name: &'static str, path: &Path) -> Api {
        assert!(
            path.exists(),
            "shared object not found: {}\n\
             build the C side with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            path.display()
        );
        // RTLD_LOCAL (libloading's default) keeps each library's symbols out of
        // the global scope, so the C .so's internal calls to `log_info` bind to
        // its own definition and are not interposed by the Rust .so (verified
        // explicitly by `isolation` tests).
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));

        macro_rules! sym {
            ($n:literal, $t:ty) => {
                *unsafe { lib.get::<$t>(concat!($n, "\0").as_bytes()) }.unwrap_or_else(|e| {
                    panic!("{} does not export `{}`: {e}", path.display(), $n)
                })
            };
        }

        let initialize_logger = sym!("initialize_logger", FnInitializeLogger);
        let log_info = sym!("log_info", FnLog);
        let log_warning = sym!("log_warning", FnLog);
        let log_error = sym!("log_error", FnLog);
        let finalize_logger = sym!("finalize_logger", FnFinalizeLogger);
        let create_task_manager = sym!("create_task_manager", FnCreate);
        let add_task = sym!("add_task", FnAddTask);
        let print_tasks = sym!("print_tasks", FnPrint);
        let destroy_task_manager = sym!("destroy_task_manager", FnDestroy);
        let driver = sym!("driver", FnDriver);

        Api {
            name,
            path: path.to_path_buf(),
            _lib: lib,
            initialize_logger,
            log_info,
            log_warning,
            log_error,
            finalize_logger,
            create_task_manager,
            add_task,
            print_tasks,
            destroy_task_manager,
            driver,
        }
    }
}

pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir().join("c_src").join("build").join("libdriver.so")
}

/// Locate the Rust `cdylib` for the profile the tests were built with.
///
/// `cargo build` "uplifts" the artifact to `target/<profile>/libdriver.so`,
/// whereas `cargo test` only produces `target/<profile>/deps/libdriver.so`.
/// Both may exist, and the uplifted one can be **stale** (e.g. left over from a
/// build with different flags) — which would silently test the wrong binary. So
/// pick whichever candidate was modified most recently.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // .../target/<profile>/deps/<test-binary>
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");

    let candidates = [deps.join("libdriver.so"), profile.join("libdriver.so")];
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for c in candidates.iter() {
        if let Ok(m) = std::fs::metadata(c).and_then(|m| m.modified()) {
            if best.as_ref().is_none_or(|(t, _)| m > *t) {
                best = Some((m, c.clone()));
            }
        }
    }
    match best {
        Some((_, p)) => p,
        // Nothing built yet: return the cargo-test location so the panic message
        // in `Api::load` points at the right place.
        None => deps.join("libdriver.so"),
    }
}

struct Impls {
    c: Api,
    rust: Api,
}

// Api holds a dlopen handle + plain fn pointers; sharing it across the
// serialized tests is fine (all access is under `lock()`).
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

static IMPLS: OnceLock<Impls> = OnceLock::new();

fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| Impls {
        c: Api::load("C", &c_so_path()),
        rust: Api::load("RUST", &rust_so_path()),
    })
}

pub fn c_api() -> &'static Api {
    &impls().c
}
pub fn rust_api() -> &'static Api {
    &impls().rust
}
/// C first, Rust second — the C result is always the reference.
pub fn both() -> [&'static Api; 2] {
    [c_api(), rust_api()]
}

// ---------------------------------------------------------------------------
// Serialization: the library has process-global state (the `log_file` static),
// and the harness redirects fd 1 / fd 2 and mutates the environment, so tests
// must not overlap.
// ---------------------------------------------------------------------------

static LOCK: Mutex<()> = Mutex::new(());

pub fn lock() -> MutexGuard<'static, ()> {
    match LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(), // ignore poisoning from an earlier failed test
    }
}

// ---------------------------------------------------------------------------
// Scratch files
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn scratch_dir() -> PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let d = std::env::temp_dir().join(format!(
            "driver_diff_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&d).expect("create scratch dir");
        d
    })
    .clone()
}

pub fn unique_path(stem: &str) -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    scratch_dir().join(format!("{stem}_{n}"))
}

// ---------------------------------------------------------------------------
// Environment
// ---------------------------------------------------------------------------

pub fn set_env(key: &str, val: &str) {
    let k = CString::new(key).unwrap();
    let v = CString::new(val).unwrap();
    let rc = unsafe { libc::setenv(k.as_ptr(), v.as_ptr(), 1) };
    assert_eq!(rc, 0, "setenv({key}) failed");
}

pub fn unset_env(key: &str) {
    let k = CString::new(key).unwrap();
    unsafe { libc::unsetenv(k.as_ptr()) };
}

// ---------------------------------------------------------------------------
// stdout / stderr capture
// ---------------------------------------------------------------------------

#[derive(Default, Clone, PartialEq, Eq)]
pub struct Captured {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

fn flush_all() {
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    // Flush *every* libc stream: the C and Rust libraries share glibc's
    // `stdout`, and their log-file streams are buffered too.
    unsafe { libc::fflush(std::ptr::null_mut()) };
}

fn open_trunc(p: &Path) -> c_int {
    let c = CString::new(p.as_os_str().as_encoded_bytes()).unwrap();
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC, 0o600) };
    assert!(fd >= 0, "open({}) failed", p.display());
    fd
}

/// Run `f` with fd 1 and fd 2 redirected to scratch files and return whatever
/// was written to each.
pub fn capture<R>(f: impl FnOnce() -> R) -> (R, Captured) {
    flush_all();

    let out_path = unique_path("cap_out");
    let err_path = unique_path("cap_err");
    let out_fd = open_trunc(&out_path);
    let err_fd = open_trunc(&err_path);

    let (save_out, save_err) = unsafe { (libc::dup(1), libc::dup(2)) };
    assert!(save_out >= 0 && save_err >= 0, "dup failed");
    unsafe {
        assert!(libc::dup2(out_fd, 1) >= 0, "dup2 stdout");
        assert!(libc::dup2(err_fd, 2) >= 0, "dup2 stderr");
    }

    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    flush_all();
    unsafe {
        libc::dup2(save_out, 1);
        libc::dup2(save_err, 2);
        libc::close(save_out);
        libc::close(save_err);
        libc::close(out_fd);
        libc::close(err_fd);
    }

    let cap = Captured {
        stdout: std::fs::read(&out_path).unwrap_or_default(),
        stderr: std::fs::read(&err_path).unwrap_or_default(),
    };
    let _ = std::fs::remove_file(&out_path);
    let _ = std::fs::remove_file(&err_path);

    match r {
        Ok(v) => (v, cap),
        Err(p) => {
            eprintln!(
                "--- captured stdout during panic ---\n{}\n--- captured stderr ---\n{}\n---",
                String::from_utf8_lossy(&cap.stdout),
                String::from_utf8_lossy(&cap.stderr)
            );
            std::panic::resume_unwind(p)
        }
    }
}

// ---------------------------------------------------------------------------
// Observation bundle
// ---------------------------------------------------------------------------

/// Everything a scenario observed for one implementation.
#[derive(Clone, PartialEq, Eq)]
pub struct Obs {
    /// Return values / derived integers, in call order.
    pub rets: Vec<i64>,
    /// Extra raw bytes (struct snapshots, …).
    pub extra: Vec<u8>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub log: Vec<u8>,
}

impl Obs {
    pub fn new() -> Obs {
        Obs {
            rets: Vec::new(),
            extra: Vec::new(),
            stdout: Vec::new(),
            stderr: Vec::new(),
            log: Vec::new(),
        }
    }
}

/// What a scenario body may record while it runs.
pub struct Rec {
    pub rets: Vec<i64>,
    pub extra: Vec<u8>,
    /// Absolute path of the log file this run will write to.
    pub log_path: PathBuf,
}

impl Rec {
    pub fn ret(&mut self, v: impl Into<i64>) {
        self.rets.push(v.into());
    }
    pub fn ptr_is_null(&mut self, p: *const u8) {
        self.rets.push(p.is_null() as i64);
    }
    pub fn bytes(&mut self, b: &[u8]) {
        self.extra.extend_from_slice(b);
    }
    pub fn manager(&mut self, m: *const TaskManager) {
        let s = unsafe { snapshot_manager(m) };
        self.extra.extend_from_slice(&s);
    }
}

/// Raw snapshot of a `TaskManager` plus every populated `Task` slot.
///
/// The `tasks` pointer *value* is deliberately reduced to "is it NULL", since
/// heap addresses legitimately differ between the two runs.
pub unsafe fn snapshot_manager(m: *const TaskManager) -> Vec<u8> {
    let mut v = Vec::new();
    if m.is_null() {
        v.extend_from_slice(b"<null manager>");
        return v;
    }
    let max_tasks = unsafe { (*m).max_tasks };
    let task_count = unsafe { (*m).task_count };
    let tasks = unsafe { (*m).tasks };
    v.extend_from_slice(&max_tasks.to_le_bytes());
    v.extend_from_slice(&task_count.to_le_bytes());
    v.push(u8::from(!tasks.is_null()));
    // Only the slots actually written by `add_task` are initialised; anything
    // past `task_count` is malloc garbage and would differ run to run.
    if !tasks.is_null() && task_count > 0 && max_tasks > 0 {
        let n = task_count.min(max_tasks) as usize;
        let raw = unsafe { std::slice::from_raw_parts(tasks as *const u8, n * TASK_SIZE) };
        v.extend_from_slice(raw);
    }
    v
}

// ---------------------------------------------------------------------------
// The differential runner
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub enum LogSetting {
    /// `LOG_FILE` points at a fresh (nonexistent) scratch path.
    Fresh,
    /// `LOG_FILE` points at a scratch path pre-seeded with these bytes
    /// (exercises `fopen(..., "a")` append semantics).
    PreExisting(Vec<u8>),
    /// `LOG_FILE` unset: the library must use `./default.log`. The run happens
    /// with the CWD switched to a private scratch directory.
    UnsetUseCwdDefault,
    /// `LOG_FILE` set to this exact (possibly invalid) value.
    Explicit(String),
    /// Leave `LOG_FILE` alone entirely (used by scenarios that manage it).
    Untouched,
}

#[derive(Clone)]
pub struct Cfg {
    pub log: LogSetting,
    /// `None` = `MAX_TASKS` unset (library default 10).
    pub max_tasks: Option<String>,
}

impl Cfg {
    pub fn fresh() -> Cfg {
        Cfg {
            log: LogSetting::Fresh,
            max_tasks: None,
        }
    }
    pub fn max(mut self, v: &str) -> Cfg {
        self.max_tasks = Some(v.to_string());
        self
    }
    pub fn max_unset(mut self) -> Cfg {
        self.max_tasks = None;
        self
    }
    pub fn log(mut self, l: LogSetting) -> Cfg {
        self.log = l;
        self
    }
}

fn run_one(api: &Api, cfg: &Cfg, body: &dyn Fn(&Api, &mut Rec)) -> Obs {
    // --- MAX_TASKS ---
    match &cfg.max_tasks {
        Some(v) => set_env("MAX_TASKS", v),
        None => unset_env("MAX_TASKS"),
    }

    // --- LOG_FILE ---
    let mut restore_cwd: Option<PathBuf> = None;
    let log_path: PathBuf = match &cfg.log {
        LogSetting::Fresh => {
            let p = unique_path(&format!("log_{}", api.name));
            let _ = std::fs::remove_file(&p);
            set_env("LOG_FILE", p.to_str().unwrap());
            p
        }
        LogSetting::PreExisting(seed) => {
            let p = unique_path(&format!("log_{}", api.name));
            let _ = std::fs::remove_file(&p);
            std::fs::write(&p, seed).expect("seed log file");
            set_env("LOG_FILE", p.to_str().unwrap());
            p
        }
        LogSetting::UnsetUseCwdDefault => {
            let d = unique_path(&format!("cwd_{}", api.name));
            std::fs::create_dir_all(&d).expect("create cwd dir");
            let cur = std::env::current_dir().expect("cwd");
            std::env::set_current_dir(&d).expect("chdir");
            restore_cwd = Some(cur);
            unset_env("LOG_FILE");
            d.join("default.log")
        }
        LogSetting::Explicit(v) => {
            set_env("LOG_FILE", v);
            // Not necessarily a readable file (that is the point); use a path
            // that simply will not exist so the "log" observable stays empty.
            unique_path(&format!("unused_{}", api.name))
        }
        LogSetting::Untouched => unique_path(&format!("unused_{}", api.name)),
    };

    let mut rec = Rec {
        rets: Vec::new(),
        extra: Vec::new(),
        log_path: log_path.clone(),
    };

    let (_, cap) = capture(|| body(api, &mut rec));

    if let Some(cur) = restore_cwd {
        let _ = std::env::set_current_dir(cur);
    }

    Obs {
        rets: rec.rets,
        extra: rec.extra,
        stdout: cap.stdout,
        stderr: cap.stderr,
        log: std::fs::read(&log_path).unwrap_or_default(),
    }
}

/// Run `body` against the C `.so` and the Rust `.so` in the given
/// configuration and assert every observable matches byte-for-byte.
///
/// Returns the **C** observation, so callers can additionally pin down absolute
/// expectations (the C behaviour is the ground truth).
///
/// NOTE: this deliberately does **not** take [`lock`]. Every test file exposes a
/// single `#[test]`, so scenarios are already strictly serialized; taking a
/// non-reentrant mutex here would deadlock any scenario that holds the lock
/// across several `diff` calls.
pub fn diff(label: &str, cfg: &Cfg, body: impl Fn(&Api, &mut Rec)) -> Obs {
    diff_locked(label, cfg, body)
}

/// Alias of [`diff`], kept for scenarios that explicitly hold [`lock`].
pub fn diff_locked(label: &str, cfg: &Cfg, body: impl Fn(&Api, &mut Rec)) -> Obs {
    let c = run_one(c_api(), cfg, &body);
    let r = run_one(rust_api(), cfg, &body);
    compare(label, &c, &r);
    c
}

pub fn compare(label: &str, c: &Obs, r: &Obs) {
    let mut problems: Vec<String> = Vec::new();
    if c.rets != r.rets {
        problems.push(format!(
            "return values differ:\n     C: {:?}\n  RUST: {:?}",
            c.rets, r.rets
        ));
    }
    if c.stdout != r.stdout {
        problems.push(format!("stdout differs:{}", show2(&c.stdout, &r.stdout)));
    }
    if c.stderr != r.stderr {
        problems.push(format!("stderr differs:{}", show2(&c.stderr, &r.stderr)));
    }
    if c.log != r.log {
        problems.push(format!("log file differs:{}", show2(&c.log, &r.log)));
    }
    if c.extra != r.extra {
        problems.push(format!(
            "struct/extra bytes differ:{}",
            show2(&c.extra, &r.extra)
        ));
    }
    assert!(
        problems.is_empty(),
        "\n[{label}] C and Rust diverged:\n{}\n",
        problems.join("\n")
    );
}

fn show2(c: &[u8], r: &[u8]) -> String {
    let mut s = String::new();
    s.push_str(&format!(
        "\n     C ({} bytes): {}",
        c.len(),
        String::from_utf8_lossy(&c[..c.len().min(2048)]).escape_debug()
    ));
    s.push_str(&format!(
        "\n  RUST ({} bytes): {}",
        r.len(),
        String::from_utf8_lossy(&r[..r.len().min(2048)]).escape_debug()
    ));
    if let Some(i) = (0..c.len().min(r.len())).find(|&i| c[i] != r[i]) {
        s.push_str(&format!(
            "\n  first difference at byte {i}: C={:#04x} RUST={:#04x}",
            c[i], r[i]
        ));
    } else {
        s.push_str("\n  (one is a prefix of the other)");
    }
    s
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seeds keep failures reproducible.
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 { 0 } else { (self.next_u64() % n as u64) as usize }
    }
    pub fn range(&mut self, lo: usize, hi_incl: usize) -> usize {
        lo + self.below(hi_incl - lo + 1)
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }
    /// A random NUL-free byte string of the given length, drawn from a mix of
    /// printable ASCII, `printf` metacharacters, control bytes and high bytes.
    pub fn text(&mut self, len: usize) -> Vec<u8> {
        let mut v = Vec::with_capacity(len);
        for _ in 0..len {
            let b = match self.below(10) {
                0..=4 => b'a' + (self.below(26) as u8), // letters
                5 => b'0' + (self.below(10) as u8),     // digits
                6 => *b" \t.,:;-_/()[]".get(self.below(13)).unwrap(),
                7 => *b"%sdnxfp*#".get(self.below(9)).unwrap(), // printf metachars
                8 => self.range(1, 31) as u8,                   // control bytes (never NUL)
                _ => self.range(0x80, 0xFF) as u8,              // high bytes
            };
            // never a NUL (it would truncate the C string) and never '\n'
            // unless the caller asks for line separators
            v.push(if b == 0 || b == b'\n' { b'z' } else { b });
        }
        v
    }
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

/// A NUL-terminated buffer holding exactly `bytes` (which must be NUL-free).
pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    assert!(!bytes.contains(&0), "test string must not contain NUL");
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

/// `malloc` a `TaskManager` + `max_tasks` `Task` slots exactly the way
/// `create_task_manager` would, but owned by the *test*, so caller-crafted
/// shapes can be handed to `add_task` / `print_tasks` / `destroy_task_manager`.
/// The blocks come from the same libc `malloc`, so `destroy_task_manager` may
/// legitimately `free` them.
pub unsafe fn craft_manager(max_tasks: c_int, task_count: c_int, fill: u8) -> *mut TaskManager {
    let m = unsafe { libc::malloc(TASKMANAGER_SIZE) } as *mut TaskManager;
    assert!(!m.is_null());
    let slots = max_tasks.max(0) as usize;
    let tasks = if slots == 0 {
        (unsafe { libc::malloc(0) }) as *mut Task
    } else {
        let p = unsafe { libc::malloc(slots * TASK_SIZE) };
        assert!(!p.is_null());
        unsafe { libc::memset(p, fill as i32, slots * TASK_SIZE) };
        p as *mut Task
    };
    unsafe {
        (*m).tasks = tasks;
        (*m).max_tasks = max_tasks;
        (*m).task_count = task_count;
    }
    m
}

pub unsafe fn free_manager(m: *mut TaskManager) {
    if m.is_null() {
        return;
    }
    unsafe {
        libc::free((*m).tasks as *mut libc::c_void);
        libc::free(m as *mut libc::c_void);
    }
}

/// Write `desc` (truncated/padded exactly like `strncpy(dst, src, 255)` plus a
/// forced NUL at index 255) into slot `idx` — used to build deterministic
/// caller-owned task arrays.
pub unsafe fn set_slot(m: *mut TaskManager, idx: usize, desc: &[u8], priority: c_int) {
    let t = unsafe { (*m).tasks.add(idx) };
    let d = unsafe { (&raw mut (*t).description).cast::<u8>() };
    for i in 0..DESC_LEN {
        let b = if i < desc.len().min(DESC_LEN - 1) {
            desc[i]
        } else {
            0
        };
        unsafe { *d.add(i) = b };
    }
    unsafe { (*t).priority = priority };
}
