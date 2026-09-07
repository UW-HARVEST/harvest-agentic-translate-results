//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and never calls any Rust
//! function directly, so the `#[no_mangle]` export wrappers are under test too.
//!
//! * C    : `c_src/build/libdriver.so`
//! * Rust : `translation/target/<profile>/libdriver.so`
//!
//! Every observable channel is compared: the return values / struct fields the
//! test reports (`obs` string), the bytes written to `stdout`, the bytes written
//! to `stderr`, and the bytes of the log file.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{CString, c_char, c_int, c_void};
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// ABI mirrors of task_manager.h
// ---------------------------------------------------------------------------

pub const DESC_LEN: usize = 256;
pub const TASK_SIZE: usize = 260;

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

// ---------------------------------------------------------------------------
// libc bits the harness itself needs
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn unsetenv(name: *const c_char) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// Exported-symbol signatures
// ---------------------------------------------------------------------------

pub type FnInitializeLogger = unsafe extern "C" fn() -> c_int;
pub type FnLog = unsafe extern "C" fn(*const c_char);
pub type FnFinalizeLogger = unsafe extern "C" fn();
pub type FnCreateTaskManager = unsafe extern "C" fn() -> *mut TaskManager;
pub type FnAddTask = unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int);
pub type FnPrintTasks = unsafe extern "C" fn(*const TaskManager);
pub type FnDestroyTaskManager = unsafe extern "C" fn(*mut TaskManager);
pub type FnDriver = unsafe extern "C" fn(*const c_char) -> c_int;

/// All ten exported symbols of one implementation, resolved via `dlsym`.
pub struct Api {
    pub name: &'static str,
    pub path: PathBuf,
    _lib: Library,
    pub initialize_logger: FnInitializeLogger,
    pub log_info: FnLog,
    pub log_warning: FnLog,
    pub log_error: FnLog,
    pub finalize_logger: FnFinalizeLogger,
    pub create_task_manager: FnCreateTaskManager,
    pub add_task: FnAddTask,
    pub print_tasks: FnPrintTasks,
    pub destroy_task_manager: FnDestroyTaskManager,
    pub driver: FnDriver,
}

impl Api {
    fn open(name: &'static str, path: PathBuf) -> Api {
        assert!(path.is_file(), "shared object not found: {}", path.display());
        // RTLD_LOCAL (libloading's default) keeps the two copies' symbol
        // namespaces separate, so `dlsym` on each handle resolves to that
        // library's own implementation even though the names collide.
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
        macro_rules! sym {
            ($t:ty, $n:literal) => {
                *unsafe { lib.get::<$t>($n) }
                    .unwrap_or_else(|e| panic!("{name}: missing symbol {:?}: {e}", $n))
            };
        }
        let api = Api {
            initialize_logger: sym!(FnInitializeLogger, b"initialize_logger\0"),
            log_info: sym!(FnLog, b"log_info\0"),
            log_warning: sym!(FnLog, b"log_warning\0"),
            log_error: sym!(FnLog, b"log_error\0"),
            finalize_logger: sym!(FnFinalizeLogger, b"finalize_logger\0"),
            create_task_manager: sym!(FnCreateTaskManager, b"create_task_manager\0"),
            add_task: sym!(FnAddTask, b"add_task\0"),
            print_tasks: sym!(FnPrintTasks, b"print_tasks\0"),
            destroy_task_manager: sym!(FnDestroyTaskManager, b"destroy_task_manager\0"),
            driver: sym!(FnDriver, b"driver\0"),
            name,
            path,
            _lib: lib,
        };
        api
    }
}

struct Both {
    c: Api,
    rust: Api,
}
// The two handles are only ever touched while the global harness mutex is held.
unsafe impl Send for Both {}
unsafe impl Sync for Both {}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <workdir>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("manifest dir has a parent")
        .to_path_buf()
}

fn both() -> &'static Both {
    static BOTH: OnceLock<Both> = OnceLock::new();
    BOTH.get_or_init(|| {
        let root = workspace_root();
        let c_path = std::env::var_os("DRIVER_C_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| root.join("c_src/build/libdriver.so"));
        let profile = if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        };
        let rust_path = std::env::var_os("DRIVER_RUST_SO")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                // `cargo test` does not build the `cdylib` artifact, so prefer
                // the one matching this test binary's profile and fall back to
                // the other. `run_tests.sh` builds both up-front.
                let primary = root.join(format!("translation/target/{profile}/libdriver.so"));
                if primary.is_file() {
                    return primary;
                }
                let other = if profile == "debug" { "release" } else { "debug" };
                let fallback = root.join(format!("translation/target/{other}/libdriver.so"));
                if fallback.is_file() { fallback } else { primary }
            });
        Both {
            c: Api::open("C", c_path),
            rust: Api::open("Rust", rust_path),
        }
    })
}

pub fn c_api() -> &'static Api {
    &both().c
}
pub fn rust_api() -> &'static Api {
    &both().rust
}

// ---------------------------------------------------------------------------
// Global serialisation: the harness mutates process-global state
// (environment, CWD, fd 1 and fd 2), so only one scenario may run at a time.
// ---------------------------------------------------------------------------

pub fn lock() -> MutexGuard<'static, ()> {
    static M: Mutex<()> = Mutex::new(());
    M.lock().unwrap_or_else(|e| e.into_inner())
}

fn unique() -> u64 {
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

fn sandbox_root() -> PathBuf {
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let p = std::env::temp_dir().join(format!("driver-difftest-{}", std::process::id()));
        std::fs::create_dir_all(&p).expect("create sandbox root");
        p
    })
    .clone()
}

fn fresh_dir(tag: &str) -> PathBuf {
    let p = sandbox_root().join(format!("{tag}-{}", unique()));
    std::fs::create_dir_all(&p).expect("create sandbox dir");
    p
}

fn put_env(key: &str, value: Option<&str>) {
    let k = CString::new(key).unwrap();
    unsafe {
        match value {
            Some(v) => {
                let v = CString::new(v).unwrap();
                setenv(k.as_ptr(), v.as_ptr(), 1);
            }
            None => {
                unsetenv(k.as_ptr());
            }
        }
    }
}

// ---------------------------------------------------------------------------
// fd capture
// ---------------------------------------------------------------------------

struct Capture {
    saved_out: c_int,
    saved_err: c_int,
    out_path: PathBuf,
    err_path: PathBuf,
    _out: std::fs::File,
    _err: std::fs::File,
}

impl Capture {
    fn start(dir: &Path) -> Capture {
        let out_path = dir.join(".captured-stdout");
        let err_path = dir.join(".captured-stderr");
        let out = std::fs::File::create(&out_path).expect("create stdout capture");
        let err = std::fs::File::create(&err_path).expect("create stderr capture");
        unsafe {
            fflush(std::ptr::null_mut());
        }
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let saved_out = unsafe { dup(1) };
        let saved_err = unsafe { dup(2) };
        assert!(saved_out >= 0 && saved_err >= 0, "dup failed");
        unsafe {
            dup2(out.as_raw_fd(), 1);
            dup2(err.as_raw_fd(), 2);
        }
        Capture {
            saved_out,
            saved_err,
            out_path,
            err_path,
            _out: out,
            _err: err,
        }
    }

    /// Flushes every C stream (so both the captured fds *and* the log file
    /// reflect everything written so far) and restores fd 1 / fd 2.
    fn finish(self) -> (Vec<u8>, Vec<u8>) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved_out, 1);
            dup2(self.saved_err, 2);
            close(self.saved_out);
            close(self.saved_err);
        }
        let out = std::fs::read(&self.out_path).unwrap_or_default();
        let err = std::fs::read(&self.err_path).unwrap_or_default();
        (out, err)
    }
}

// ---------------------------------------------------------------------------
// Scenario runner
// ---------------------------------------------------------------------------

/// How `LOG_FILE` should be configured for a scenario.
#[derive(Clone, Debug)]
pub enum LogCfg {
    /// `LOG_FILE` unset — the C falls back to `"default.log"` relative to CWD.
    Unset,
    /// `LOG_FILE` = `<per-side sandbox dir>/<name>`; a fresh, writable location.
    InDir(String),
    /// `LOG_FILE` = this exact string (used for the unopenable-path error rows).
    Exact(String),
    /// `LOG_FILE` = `<per-side sandbox dir>/<name>`, pre-seeded with these bytes.
    Seeded(String, Vec<u8>),
}

/// Everything one side of the comparison produced.
#[derive(Debug)]
pub struct Observed {
    pub obs: String,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub log: Vec<u8>,
}

pub struct Ctx {
    pub dir: PathBuf,
    /// Absolute path the logger was told (or is expected) to use.
    pub log_path: PathBuf,
}

/// Runs `body` against one implementation inside a pristine sandbox.
///
/// Guarantees, in order:
/// 1. a fresh directory becomes the process CWD;
/// 2. the implementation's `static FILE *log_file` is forced back to `NULL`
///    (by pointing `LOG_FILE` at an unopenable path and calling
///    `initialize_logger`, whose `fopen` failure assigns `NULL`) — this makes
///    every scenario independent of test execution order;
/// 3. `LOG_FILE` / `MAX_TASKS` are set as requested;
/// 4. fd 1 and fd 2 are redirected to files;
/// 5. `body` runs;
/// 6. all C streams are flushed, fds restored, CWD restored.
pub fn run_side<F>(api: &Api, log: &LogCfg, max_tasks: Option<&str>, body: F) -> Observed
where
    F: FnOnce(&Api, &Ctx) -> String,
{
    let dir = fresh_dir(api.name);
    let original_cwd = std::env::current_dir().expect("cwd");

    // (2) force log_file back to NULL, with the resulting stderr noise discarded.
    {
        let reset = dir.join("no_such_subdir").join("reset.log");
        put_env("LOG_FILE", Some(reset.to_str().unwrap()));
        let cap = Capture::start(&dir);
        let rc = unsafe { (api.initialize_logger)() };
        let _ = cap.finish();
        assert_eq!(
            rc, -1,
            "{}: harness precondition failed — initialize_logger on unopenable \
             path {} should return -1 so that log_file becomes NULL",
            api.name,
            reset.display()
        );
    }

    // (1) CWD
    std::env::set_current_dir(&dir).expect("chdir into sandbox");

    // (3) environment
    let log_path = match log {
        LogCfg::Unset => {
            put_env("LOG_FILE", None);
            dir.join("default.log")
        }
        LogCfg::InDir(name) => {
            let p = dir.join(name);
            put_env("LOG_FILE", Some(p.to_str().unwrap()));
            p
        }
        LogCfg::Seeded(name, bytes) => {
            let p = dir.join(name);
            std::fs::write(&p, bytes).expect("seed log file");
            put_env("LOG_FILE", Some(p.to_str().unwrap()));
            p
        }
        LogCfg::Exact(s) => {
            put_env("LOG_FILE", Some(s));
            PathBuf::from(s)
        }
    };
    put_env("MAX_TASKS", max_tasks);

    let ctx = Ctx {
        dir: dir.clone(),
        log_path: log_path.clone(),
    };

    // (4)(5)
    let cap = Capture::start(&dir);
    let obs = body(api, &ctx);
    let (stdout, stderr) = cap.finish();

    // (6)
    std::env::set_current_dir(&original_cwd).expect("restore cwd");
    put_env("LOG_FILE", None);
    put_env("MAX_TASKS", None);

    let log_bytes = std::fs::read(&log_path).unwrap_or_default();

    Observed {
        obs,
        stdout,
        stderr,
        log: log_bytes,
    }
}

/// Runs the same scenario against both implementations and asserts every
/// observable channel is byte-identical.
pub fn diff<F>(case: &str, log: LogCfg, max_tasks: Option<&str>, body: F)
where
    F: Fn(&Api, &Ctx) -> String,
{
    let c = run_side(c_api(), &log, max_tasks, &body);
    let r = run_side(rust_api(), &log, max_tasks, &body);
    compare(case, &c, &r);
}

pub fn compare(case: &str, c: &Observed, r: &Observed) {
    if c.obs != r.obs {
        panic!(
            "[{case}] observation mismatch\n  C    = {}\n  Rust = {}",
            c.obs, r.obs
        );
    }
    if c.stdout != r.stdout {
        panic!(
            "[{case}] stdout mismatch\n  C    ({} bytes) = {}\n  Rust ({} bytes) = {}",
            c.stdout.len(),
            show(&c.stdout),
            r.stdout.len(),
            show(&r.stdout)
        );
    }
    if c.stderr != r.stderr {
        panic!(
            "[{case}] stderr mismatch\n  C    ({} bytes) = {}\n  Rust ({} bytes) = {}",
            c.stderr.len(),
            show(&c.stderr),
            r.stderr.len(),
            show(&r.stderr)
        );
    }
    if c.log != r.log {
        panic!(
            "[{case}] log-file mismatch\n  C    ({} bytes) = {}\n  Rust ({} bytes) = {}",
            c.log.len(),
            show(&c.log),
            r.log.len(),
            show(&r.log)
        );
    }
}

/// Same as [`diff`], but the log path is embedded in the output, so it is
/// normalised out of `stderr` before comparing (the two sides necessarily use
/// different sandbox directories).
pub fn diff_normalising_paths<F>(case: &str, log: LogCfg, max_tasks: Option<&str>, body: F)
where
    F: Fn(&Api, &Ctx) -> String,
{
    let c_raw = run_side(c_api(), &log, max_tasks, &body);
    let r_raw = run_side(rust_api(), &log, max_tasks, &body);
    let c = Observed {
        stderr: normalise(&c_raw.stderr, "C"),
        ..c_raw
    };
    let r = Observed {
        stderr: normalise(&r_raw.stderr, "Rust"),
        ..r_raw
    };
    compare(case, &c, &r);
}

/// Replaces the per-side sandbox directory prefix with a stable placeholder.
fn normalise(bytes: &[u8], side: &str) -> Vec<u8> {
    let root = sandbox_root();
    let needle = format!("{}/{}-", root.display(), side).into_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i..].starts_with(&needle) {
            out.extend_from_slice(b"<SANDBOX>/");
            i += needle.len();
            // skip the numeric suffix of the directory name
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Reporting helpers used to build the `obs` string
// ---------------------------------------------------------------------------

pub fn show(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            b'"' => s.push_str("\\\""),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    s.push('"');
    s
}

pub fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// Reports the manager header plus a full byte-level dump of every *initialised*
/// `Task` (indices `0 .. task_count`). Slots past `task_count` are untouched
/// `malloc` memory on both sides and are deliberately not compared.
pub fn dump_manager(m: *const TaskManager) -> String {
    if m.is_null() {
        return "manager=NULL".to_string();
    }
    unsafe {
        let max = (*m).max_tasks;
        let count = (*m).task_count;
        let mut s = format!(
            "manager{{max_tasks={max},task_count={count},tasks_null={}}}",
            (*m).tasks.is_null()
        );
        if !(*m).tasks.is_null() && count > 0 {
            for i in 0..count {
                let t = (*m).tasks.offset(i as isize) as *const u8;
                let raw = std::slice::from_raw_parts(t, TASK_SIZE);
                s.push_str(&format!("\n  task[{i}]={}", hex(raw)));
            }
        }
        s
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seeds, reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    /// Random printable-ASCII string of the given length (never contains NUL
    /// or `\n`, so it survives `CString` and the `driver` tokeniser).
    pub fn ascii(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let c = 0x20u8 + (self.below(0x5f) as u8);
                c
            })
            .collect()
    }
    /// Random non-NUL bytes, including high-bit and control bytes, but never
    /// `\n` (kept out so the same generator serves the `driver` line tests).
    pub fn bytes_no_nl(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| loop {
                let b = (self.below(255) + 1) as u8;
                if b != b'\n' {
                    return b;
                }
            })
            .collect()
    }
    /// `ascii` with a length drawn uniformly from `0..n`.
    pub fn ascii_below(&mut self, n: usize) -> Vec<u8> {
        let len = self.below(n);
        self.ascii(len)
    }
    /// `ascii` with a length drawn uniformly from `1..=n`.
    pub fn ascii_1_to(&mut self, n: usize) -> Vec<u8> {
        let len = 1 + self.below(n);
        self.ascii(len)
    }
    /// `bytes_no_nl` with a length drawn uniformly from `0..n`.
    pub fn bytes_below(&mut self, n: usize) -> Vec<u8> {
        let len = self.below(n);
        self.bytes_no_nl(len)
    }
}

/// Builds a `CString`, panicking on interior NULs (tests never generate them).
pub fn cs(bytes: &[u8]) -> CString {
    CString::new(bytes.to_vec()).expect("no interior NUL")
}

// ---------------------------------------------------------------------------
// fork-based crash comparison
//
// Some C paths are genuinely undefined behaviour (NULL `TaskManager*`, NULL
// `tasks`, double `finalize_logger`). Calling them in-process would abort the
// harness, so they are run in a forked child and the two implementations'
// *termination status* is compared instead — a real differential assertion
// rather than "both failed somehow".
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// Runs `f` in a forked child (child stdout/stderr sent to `/dev/null`) and
/// returns a stable description of how the child terminated.
pub fn fork_status<F: FnOnce()>(f: F) -> String {
    unsafe {
        fflush(std::ptr::null_mut());
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            if let Ok(devnull) = std::fs::OpenOptions::new().write(true).open("/dev/null") {
                dup2(devnull.as_raw_fd(), 1);
                dup2(devnull.as_raw_fd(), 2);
            }
            f();
            _exit(0);
        }
        let mut status: c_int = 0;
        let r = waitpid(pid, &mut status, 0);
        assert_eq!(r, pid, "waitpid failed");
        let low = status & 0x7f;
        if low == 0 {
            format!("exited({})", (status >> 8) & 0xff)
        } else if low == 0x7f {
            "stopped".to_string()
        } else {
            format!("signal({low})")
        }
    }
}

/// Reads the raw bytes of a shared object (used to prove that an unreachable
/// error path's string literal really is compiled into both libraries).
pub fn so_bytes(api: &Api) -> Vec<u8> {
    std::fs::read(&api.path).expect("read shared object")
}

pub fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

/// True when the loaded Rust `.so` was compiled with `-C debug-assertions`
/// (cargo's `dev`/`test` profiles). Detected mechanically: such a build embeds
/// the raw-pointer precondition panic message.
///
/// This matters for exactly one class of row — the genuinely-undefined
/// NULL-dereference paths. With debug assertions on, Rust turns `*ptr` on a null
/// raw pointer into a panic (`SIGABRT`) instead of the hardware fault the C
/// takes (`SIGSEGV`). The release cdylib — the artifact this crate actually
/// ships (`crate-type = ["cdylib"]`, `[profile.release] panic = "abort"`) —
/// faults identically to the C, which the same tests assert.
///
/// Debug assertions are deliberately left ENABLED so that Phase B/C also prove
/// the translation never relies on a panicking arithmetic operation where the C
/// wraps (the translation uses `wrapping_*` throughout).
pub fn rust_has_debug_assertions() -> bool {
    static V: OnceLock<bool> = OnceLock::new();
    *V.get_or_init(|| {
        let b = so_bytes(rust_api());
        contains(&b, b"null pointer dereference occurred")
            || contains(&b, b"misaligned pointer dereference")
    })
}

/// Compares a child-termination status for an undefined-behaviour row.
///
/// Requires an exact match, except that a debug-assertions Rust build is allowed
/// to report `signal(6)` (panic → abort) where the C reports `signal(11)`
/// (SIGSEGV). Any other combination — including either side exiting normally —
/// is a failure.
pub fn assert_same_ub_status(case: &str, c: &str, rust: &str) {
    if c == rust {
        return;
    }
    if rust_has_debug_assertions() && c == "signal(11)" && rust == "signal(6)" {
        eprintln!(
            "[{case}] NOTE: C={c}, Rust={rust} — accepted only because the Rust \
             .so has debug assertions enabled (null-deref precondition panic). \
             The release cdylib is asserted to match C exactly."
        );
        return;
    }
    panic!("[{case}] UB termination status mismatch: C={c}, Rust={rust}");
}

/// Runs `body` (which must return a `fork_status` string) against both
/// implementations and compares with [`assert_same_ub_status`].
pub fn diff_ub<F>(case: &str, log: LogCfg, max_tasks: Option<&str>, body: F)
where
    F: Fn(&Api, &Ctx) -> String,
{
    let c = run_side(c_api(), &log, max_tasks, &body);
    let r = run_side(rust_api(), &log, max_tasks, &body);
    assert_same_ub_status(case, &c.obs, &r.obs);
    assert!(
        c.obs.starts_with("signal(") || c.obs.starts_with("exited("),
        "[{case}] C produced an unexpected status: {}",
        c.obs
    );
}
