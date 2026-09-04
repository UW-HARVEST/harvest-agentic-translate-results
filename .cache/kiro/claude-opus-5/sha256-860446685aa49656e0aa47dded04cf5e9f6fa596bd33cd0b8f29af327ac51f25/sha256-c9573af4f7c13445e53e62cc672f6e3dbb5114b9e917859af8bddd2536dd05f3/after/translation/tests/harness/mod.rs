// Differential-test harness: loads the C `.so` and the Rust `.so` through
// `libloading` and drives BOTH through their exported symbols only. No Rust
// function is ever called directly, so the `#[no_mangle]` export wrappers are
// part of what is under test.
#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

// ---------------------------------------------------------------------------
// libc bits we need for output capture / crash parity. Declared here rather
// than pulling in the `libc` crate.
// ---------------------------------------------------------------------------
extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(fd: c_int) -> c_int;
    fn dup2(old: c_int, new: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn kill(pid: c_int, sig: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
    fn setrlimit(resource: c_int, rlim: *const RLimit) -> c_int;
}

#[repr(C)]
struct RLimit {
    cur: u64,
    max: u64,
}

const RLIMIT_CORE: c_int = 4;

/// Disable core dumps in a forked child: the NULL-pointer rows of ERRORS.md
/// fault on purpose and writing a core per case is slow and wasteful.
fn no_core_dumps() {
    unsafe {
        let z = RLimit { cur: 0, max: 0 };
        setrlimit(RLIMIT_CORE, &z);
    }
}

const O_WRONLY: c_int = 1;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;
const WNOHANG: c_int = 1;

/// `fflush(NULL)` — flush *every* open output stream. This covers both the
/// process `stdout` used by `print_tasks` and the library's private log-file
/// handle, whichever library owns it.
pub fn flush_all() {
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// ABI mirror of task_manager.h (used only to *read back* what the libraries
// produced; never to construct behaviour).
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

// ---------------------------------------------------------------------------
// The loaded API surface. Raw fn pointers copied out of the `Library`, which is
// kept alive in the same struct.
// ---------------------------------------------------------------------------
pub struct Api {
    pub name: &'static str,
    _lib: Library,
    pub initialize_logger: unsafe extern "C" fn() -> c_int,
    pub log_info: unsafe extern "C" fn(*const c_char),
    pub log_warning: unsafe extern "C" fn(*const c_char),
    pub log_error: unsafe extern "C" fn(*const c_char),
    pub finalize_logger: unsafe extern "C" fn(),
    pub create_task_manager: unsafe extern "C" fn() -> *mut TaskManager,
    pub add_task: unsafe extern "C" fn(*mut TaskManager, *const c_char, c_int),
    pub print_tasks: unsafe extern "C" fn(*const TaskManager),
    pub destroy_task_manager: unsafe extern "C" fn(*mut TaskManager),
    pub driver: unsafe extern "C" fn(*const c_char) -> c_int,
}

unsafe fn sym<T: Copy>(lib: &Library, name: &[u8]) -> T {
    let s: Symbol<T> = lib
        .get(name)
        .unwrap_or_else(|e| panic!("missing symbol {}: {e}", String::from_utf8_lossy(name)));
    *s
}

impl Api {
    fn load(name: &'static str, path: &Path) -> Api {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
            Api {
                name,
                initialize_logger: sym(&lib, b"initialize_logger\0"),
                log_info: sym(&lib, b"log_info\0"),
                log_warning: sym(&lib, b"log_warning\0"),
                log_error: sym(&lib, b"log_error\0"),
                finalize_logger: sym(&lib, b"finalize_logger\0"),
                create_task_manager: sym(&lib, b"create_task_manager\0"),
                add_task: sym(&lib, b"add_task\0"),
                print_tasks: sym(&lib, b"print_tasks\0"),
                destroy_task_manager: sym(&lib, b"destroy_task_manager\0"),
                driver: sym(&lib, b"driver\0"),
                _lib: lib,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {} — build it with:\n  cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_SO") {
        return PathBuf::from(p);
    }
    // Prefer the release cdylib (that is what an external consumer links
    // against, and it is what `panic = "abort"` applies to).
    let rel = manifest_dir().join("target/release/libdriver.so");
    if rel.exists() {
        return rel;
    }
    let dbg = manifest_dir().join("target/debug/libdriver.so");
    assert!(
        dbg.exists(),
        "Rust shared library not found; run `cargo build --release` in translation/"
    );
    dbg
}

pub struct Libs {
    pub c: Api,
    pub rust: Api,
}

static LOCK: Mutex<()> = Mutex::new(());

/// Serialises every test: the harness manipulates process-global state (fd 1/2,
/// environment variables, the current directory).
pub fn guard() -> MutexGuard<'static, ()> {
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Scratch directory
// ---------------------------------------------------------------------------
pub struct Scratch {
    pub dir: PathBuf,
}

impl Scratch {
    pub fn new(tag: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!(
            "cdiff-{}-{}-{}",
            std::process::id(),
            tag,
            next_counter()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Scratch { dir }
    }
    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn next_counter() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------------------
// stdout / stderr capture at the file-descriptor level, so that C `printf`
// buffering behaves exactly as it does for a real consumer.
// ---------------------------------------------------------------------------
pub struct Capture {
    out_path: PathBuf,
    err_path: PathBuf,
    saved_out: c_int,
    saved_err: c_int,
}

impl Capture {
    pub fn begin(out_path: PathBuf, err_path: PathBuf) -> Capture {
        flush_all();
        let _ = std::io::Write::flush(&mut std::io::stdout());
        let _ = std::io::Write::flush(&mut std::io::stderr());
        unsafe {
            let saved_out = dup(1);
            let saved_err = dup(2);
            assert!(saved_out >= 0 && saved_err >= 0, "dup failed");
            let o = open_trunc(&out_path);
            let e = open_trunc(&err_path);
            assert_eq!(dup2(o, 1), 1, "dup2 stdout failed");
            assert_eq!(dup2(e, 2), 2, "dup2 stderr failed");
            close(o);
            close(e);
            Capture {
                out_path,
                err_path,
                saved_out,
                saved_err,
            }
        }
    }

    /// Restore the real fds and return `(stdout_bytes, stderr_bytes)`.
    pub fn end(self) -> (Vec<u8>, Vec<u8>) {
        flush_all();
        unsafe {
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

/// Replace every occurrence of `needle` in `hay` with `<SUBDIR>`.
///
/// The two libraries run in sibling scratch directories so their outputs cannot
/// collide, but the C library echoes the failing path back on `stderr`
/// (`logger.c:39`). That path legitimately differs between the two runs, so it
/// is normalised before comparison — everything else stays byte-exact.
pub fn normalize(hay: Vec<u8>, needles: &[Vec<u8>]) -> Vec<u8> {
    let mut out = hay;
    for needle in needles {
        if needle.is_empty() {
            continue;
        }
        let mut res: Vec<u8> = Vec::with_capacity(out.len());
        let mut i = 0usize;
        while i < out.len() {
            if out[i..].starts_with(needle) {
                res.extend_from_slice(b"<SUBDIR>");
                i += needle.len();
            } else {
                res.push(out[i]);
                i += 1;
            }
        }
        out = res;
    }
    out
}

unsafe fn open_trunc(p: &Path) -> c_int {
    let c = CString::new(p.as_os_str().as_encoded_bytes()).unwrap();
    let fd = open(c.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644 as c_int);
    assert!(fd >= 0, "open({}) failed", p.display());
    fd
}

// ---------------------------------------------------------------------------
// Environment helpers
// ---------------------------------------------------------------------------
pub fn set_env(key: &str, val: Option<&str>) {
    match val {
        Some(v) => std::env::set_var(key, v),
        None => std::env::remove_var(key),
    }
}

// ---------------------------------------------------------------------------
// Observation record: everything a scenario reports back for comparison.
// ---------------------------------------------------------------------------
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Rec {
    Int(String, i32),
    Null(String, bool),
    Bytes(String, Vec<u8>),
    Mgr {
        tag: String,
        max_tasks: i32,
        task_count: i32,
        tasks: Vec<(Vec<u8>, i32)>,
    },
}

/// What a single scenario run produced.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Observed {
    pub recs: Vec<Rec>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub logs: Vec<(String, Option<Vec<u8>>)>,
}

/// Collector handed to a scenario body.
pub struct Sink {
    pub recs: Vec<Rec>,
}

impl Sink {
    pub fn int(&mut self, tag: &str, v: i32) {
        self.recs.push(Rec::Int(tag.to_string(), v));
    }
    pub fn is_null<T>(&mut self, tag: &str, p: *const T) {
        self.recs.push(Rec::Null(tag.to_string(), p.is_null()));
    }
    pub fn bytes(&mut self, tag: &str, b: &[u8]) {
        self.recs.push(Rec::Bytes(tag.to_string(), b.to_vec()));
    }
    /// Snapshot every observable field of a `TaskManager` (never the pointer
    /// values themselves, which legitimately differ between the two builds).
    pub fn mgr(&mut self, tag: &str, m: *const TaskManager) {
        if m.is_null() {
            self.recs.push(Rec::Null(format!("{tag}.ptr"), true));
            return;
        }
        unsafe {
            let max_tasks = (*m).max_tasks;
            let task_count = (*m).task_count;
            let mut tasks = Vec::new();
            if !(*m).tasks.is_null() && task_count > 0 {
                for i in 0..task_count {
                    let t = (*m).tasks.offset(i as isize);
                    // All 256 description bytes, including the padding, so that
                    // `strncpy`'s NUL-fill behaviour is compared too.
                    let desc =
                        std::slice::from_raw_parts((*t).description.as_ptr() as *const u8, 256)
                            .to_vec();
                    tasks.push((desc, (*t).priority));
                }
            }
            self.recs.push(Rec::Mgr {
                tag: tag.to_string(),
                max_tasks,
                task_count,
                tasks,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Scenario driver
// ---------------------------------------------------------------------------
/// Files whose contents are compared after the run (log files). Paths are
/// per-library, produced by the `logs` callback from the run's scratch dir.
pub struct Run<'a> {
    pub tag: &'a str,
    /// Environment applied before the run. `MAX_TASKS` / `LOG_FILE` values are
    /// resolved per-library because the log path must differ.
    pub env: Vec<(String, Option<String>)>,
    /// Log files (label, filename inside the scratch dir) to read back.
    pub log_files: Vec<(String, String)>,
    /// Run in the scratch dir as CWD (needed for the `$LOG_FILE`-unset rows,
    /// which create `default.log` relative to the process CWD).
    pub chdir: bool,
}

/// Execute `body` against one library, capturing everything observable.
fn run_one<F>(api: &Api, scratch: &Scratch, run: &Run, body: F) -> Observed
where
    F: FnOnce(&Api, &mut Sink),
{
    let sub = scratch.dir.join(api.name);
    std::fs::create_dir_all(&sub).unwrap();

    for (k, v) in &run.env {
        // `$LOG_FILE` is rewritten into this library's private subdirectory so
        // the two runs cannot see each other's output.
        let v = v.as_ref().map(|v| {
            if v.starts_with("@/") {
                sub.join(&v[2..]).to_string_lossy().into_owned()
            } else {
                v.clone()
            }
        });
        set_env(k, v.as_deref());
    }

    let prev_cwd = std::env::current_dir().unwrap();
    if run.chdir {
        std::env::set_current_dir(&sub).unwrap();
    }

    let cap = Capture::begin(
        scratch.dir.join(format!("{}.stdout", api.name)),
        scratch.dir.join(format!("{}.stderr", api.name)),
    );
    let mut sink = Sink { recs: Vec::new() };
    body(api, &mut sink);
    let (stdout, stderr) = cap.end();

    if run.chdir {
        std::env::set_current_dir(&prev_cwd).unwrap();
    }

    let needles = vec![
        sub.to_string_lossy().as_bytes().to_vec(),
        scratch.dir.to_string_lossy().as_bytes().to_vec(),
    ];
    let logs = run
        .log_files
        .iter()
        .map(|(label, name)| {
            (
                label.clone(),
                std::fs::read(sub.join(name))
                    .ok()
                    .map(|b| normalize(b, &needles)),
            )
        })
        .collect();
    let recs = sink
        .recs
        .into_iter()
        .map(|r| match r {
            Rec::Bytes(t, b) => Rec::Bytes(t, normalize(b, &needles)),
            other => other,
        })
        .collect();

    Observed {
        recs,
        stdout: normalize(stdout, &needles),
        stderr: normalize(stderr, &needles),
        logs,
    }
}

/// Run the same scenario against both libraries and assert byte-identical
/// observables. `body` is invoked twice — once per library — and must be
/// deterministic.
pub fn diff<F>(libs: &Libs, run: Run, body: F)
where
    F: Fn(&Api, &mut Sink),
{
    let scratch = Scratch::new(run.tag);
    let c = run_one(&libs.c, &scratch, &run, |a, s| body(a, s));
    let r = run_one(&libs.rust, &scratch, &run, |a, s| body(a, s));

    if c != r {
        panic!("{}", render_diff(run.tag, &c, &r));
    }
}

fn render_diff(tag: &str, c: &Observed, r: &Observed) -> String {
    let mut s = format!("DIVERGENCE in scenario `{tag}`\n");
    if c.recs != r.recs {
        s.push_str("-- return values / struct state differ --\n");
        let n = c.recs.len().max(r.recs.len());
        for i in 0..n {
            let a = c.recs.get(i);
            let b = r.recs.get(i);
            if a != b {
                s.push_str(&format!("  [{i}] C   = {a:?}\n       Rust= {b:?}\n"));
            }
        }
    }
    if c.stdout != r.stdout {
        s.push_str(&format!(
            "-- stdout differs --\n  C   = {:?}\n  Rust= {:?}\n",
            String::from_utf8_lossy(&c.stdout),
            String::from_utf8_lossy(&r.stdout)
        ));
    }
    if c.stderr != r.stderr {
        s.push_str(&format!(
            "-- stderr differs --\n  C   = {:?}\n  Rust= {:?}\n",
            String::from_utf8_lossy(&c.stderr),
            String::from_utf8_lossy(&r.stderr)
        ));
    }
    if c.logs != r.logs {
        s.push_str("-- log file contents differ --\n");
        for i in 0..c.logs.len().max(r.logs.len()) {
            let a = c.logs.get(i);
            let b = r.logs.get(i);
            if a != b {
                s.push_str(&format!(
                    "  {:?}\n    C   = {:?}\n    Rust= {:?}\n",
                    a.map(|x| &x.0),
                    a.map(|x| x.1.as_ref().map(|v| String::from_utf8_lossy(v).into_owned())),
                    b.map(|x| x.1.as_ref().map(|v| String::from_utf8_lossy(v).into_owned())),
                ));
            }
        }
    }
    s
}

/// Convenience builder: `$LOG_FILE` pointed at a private `log.txt`, `$MAX_TASKS`
/// as given, and that log file read back afterwards.
pub fn run_with<'a>(tag: &'a str, max_tasks: Option<&'a str>) -> Run<'a> {
    Run {
        tag,
        env: vec![
            ("LOG_FILE".into(), Some("@/log.txt".into())),
            ("MAX_TASKS".into(), max_tasks.map(|s| s.to_string())),
        ],
        log_files: vec![("log.txt".into(), "log.txt".into())],
        chdir: false,
    }
}

/// Same, but with no `$LOG_FILE` at all — the library must fall back to
/// `default.log` in the CWD, so the run happens inside the scratch dir.
pub fn run_default_log<'a>(tag: &'a str, max_tasks: Option<&'a str>) -> Run<'a> {
    Run {
        tag,
        env: vec![
            ("LOG_FILE".into(), None),
            ("MAX_TASKS".into(), max_tasks.map(|s| s.to_string())),
        ],
        log_files: vec![("default.log".into(), "default.log".into())],
        chdir: true,
    }
}

// ---------------------------------------------------------------------------
// Crash parity via fork(): for the NULL-pointer rows of ERRORS.md, where the C
// behaviour is a fault. The child performs the call; the parent compares the
// raw wait status.
// ---------------------------------------------------------------------------
#[derive(Debug, PartialEq, Eq)]
pub enum Exit {
    Code(i32),
    Signal(i32),
    Timeout,
}

pub fn fork_call<F: FnOnce()>(f: F) -> Exit {
    flush_all();
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let _ = std::io::Write::flush(&mut std::io::stderr());
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            no_core_dumps();
            f();
            flush_all();
            _exit(0);
        }
        let mut status: c_int = 0;
        // Poll instead of blocking so a hung child cannot hang the suite.
        for _ in 0..2000 {
            let r = waitpid(pid, &mut status, WNOHANG);
            if r == pid {
                return decode(status);
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        kill(pid, 9);
        let _ = waitpid(pid, &mut status, 0);
        Exit::Timeout
    }
}

fn decode(status: c_int) -> Exit {
    if status & 0x7f == 0x7f {
        // stopped — treat as timeout-ish; should not happen here
        Exit::Timeout
    } else if status & 0x7f != 0 {
        Exit::Signal(status & 0x7f)
    } else {
        Exit::Code((status >> 8) & 0xff)
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed per test, reproducible)
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn range(&mut self, lo: usize, hi_incl: usize) -> usize {
        lo + self.below(hi_incl - lo + 1)
    }
    /// A NUL-free byte string of the given length (NUL would terminate it and
    /// make the test vacuous).
    pub fn cbytes(&mut self, len: usize) -> Vec<u8> {
        (0..len).map(|_| 1 + (self.next_u32() % 255) as u8).collect()
    }
    /// Printable ASCII of a random length in `lo..=hi`.
    pub fn printable_range(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.printable(n)
    }
    /// Arbitrary NUL-free bytes of a random length in `lo..=hi`.
    pub fn cbytes_range(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        let n = self.range(lo, hi);
        self.cbytes(n)
    }
    /// Printable ASCII, NUL- and newline-free.
    pub fn printable(&mut self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|_| {
                let c = 0x20 + (self.next_u32() % 95) as u8;
                if c == b'\n' {
                    b'.'
                } else {
                    c
                }
            })
            .collect()
    }
}

/// Build a `CString` from bytes, replacing interior NULs (which C could never
/// see anyway) so no input is silently dropped.
pub fn cstr(bytes: &[u8]) -> CString {
    let cleaned: Vec<u8> = bytes.iter().map(|&b| if b == 0 { b'?' } else { b }).collect();
    CString::new(cleaned).unwrap()
}

pub fn as_bytes(s: &CStr) -> &[u8] {
    s.to_bytes()
}

// ---------------------------------------------------------------------------
// Pristine library instances.
//
// Both libraries keep process-wide mutable state (`static FILE *log_file`).
// `dlopen`ing the same path twice returns the *same* mapping, so tests would
// otherwise inherit each other's logger state and test order would matter.
// Copying each `.so` to a unique path forces a fresh mapping with
// freshly-zeroed statics, which is also the only way to observe the
// "`log_file` is still NULL" rows of ERRORS.md / CONFIGS.md.
// ---------------------------------------------------------------------------
pub struct FreshLibs {
    pub libs: Libs,
    dir: PathBuf,
}

impl Drop for FreshLibs {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl std::ops::Deref for FreshLibs {
    type Target = Libs;
    fn deref(&self) -> &Libs {
        &self.libs
    }
}

pub fn fresh(tag: &str) -> FreshLibs {
    let dir = std::env::temp_dir().join(format!(
        "cdiff-so-{}-{}-{}",
        std::process::id(),
        tag,
        next_counter()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let cp = dir.join("c_libdriver.so");
    let rp = dir.join("rust_libdriver.so");
    std::fs::copy(c_so_path(), &cp).unwrap();
    std::fs::copy(rust_so_path(), &rp).unwrap();
    FreshLibs {
        libs: Libs {
            c: Api::load("C", &cp),
            rust: Api::load("Rust", &rp),
        },
        dir,
    }
}

/// Full test body wrapper: takes the global lock, loads pristine instances and
/// runs one differential scenario.
pub fn scenario<F>(tag: &'static str, run: Run<'static>, body: F)
where
    F: Fn(&Api, &mut Sink),
{
    let _g = guard();
    let libs = fresh(tag);
    diff(&libs, run, body);
}

// ---------------------------------------------------------------------------
// Differential crash tests: the body is executed in a forked child so a
// SIGSEGV (the documented C behaviour for the NULL-pointer rows of ERRORS.md)
// can be observed instead of killing the test runner.
// ---------------------------------------------------------------------------
#[derive(Debug, PartialEq, Eq)]
pub struct ObservedCrash {
    pub exit: Exit,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub logs: Vec<(String, Option<Vec<u8>>)>,
}

fn run_one_forked<F>(api: &Api, scratch: &Scratch, run: &Run, body: F) -> ObservedCrash
where
    F: FnOnce(&Api),
{
    let sub = scratch.dir.join(api.name);
    std::fs::create_dir_all(&sub).unwrap();
    for (k, v) in &run.env {
        let v = v.as_ref().map(|v| {
            if v.starts_with("@/") {
                sub.join(&v[2..]).to_string_lossy().into_owned()
            } else {
                v.clone()
            }
        });
        set_env(k, v.as_deref());
    }
    let prev_cwd = std::env::current_dir().unwrap();
    if run.chdir {
        std::env::set_current_dir(&sub).unwrap();
    }

    let cap = Capture::begin(
        scratch.dir.join(format!("{}.stdout", api.name)),
        scratch.dir.join(format!("{}.stderr", api.name)),
    );
    let exit = fork_call(|| body(api));
    let (stdout, stderr) = cap.end();

    if run.chdir {
        std::env::set_current_dir(&prev_cwd).unwrap();
    }
    let needles = vec![
        sub.to_string_lossy().as_bytes().to_vec(),
        scratch.dir.to_string_lossy().as_bytes().to_vec(),
    ];
    let logs = run
        .log_files
        .iter()
        .map(|(label, name)| {
            (
                label.clone(),
                std::fs::read(sub.join(name))
                    .ok()
                    .map(|b| normalize(b, &needles)),
            )
        })
        .collect();
    ObservedCrash {
        exit,
        stdout: normalize(stdout, &needles),
        stderr: normalize(stderr, &needles),
        logs,
    }
}

/// Run `body` against both libraries inside a forked child and require the
/// termination status (exit code *or* signal number) plus all captured output to
/// be identical.
pub fn diff_crash<F>(libs: &Libs, run: Run, expected: Exit, body: F)
where
    F: Fn(&Api),
{
    let scratch = Scratch::new(run.tag);
    let c = run_one_forked(&libs.c, &scratch, &run, |a| body(a));
    let r = run_one_forked(&libs.rust, &scratch, &run, |a| body(a));
    // Guard against a vacuous pass: the C side must really have terminated the
    // way ERRORS.md says it does.
    assert_eq!(
        c.exit, expected,
        "scenario `{}`: C child terminated as {:?}, expected {:?}",
        run.tag, c.exit, expected
    );
    assert_eq!(
        c, r,
        "DIVERGENCE in crash scenario `{}`\n  C   = {:?}\n  Rust= {:?}",
        run.tag, c, r
    );
}

pub fn crash_scenario<F>(tag: &'static str, run: Run<'static>, expected: Exit, body: F)
where
    F: Fn(&Api),
{
    let _g = guard();
    let libs = fresh(tag);
    diff_crash(&libs, run, expected, body);
}

pub const SIGSEGV: i32 = 11;
pub const SIGBUS: i32 = 7;
