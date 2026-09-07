// Shared differential-test harness.
//
// Both libraries are loaded through `libloading` and driven ONLY through their
// exported C symbols (`driver`, `run`), so the `#[no_mangle]` export wrappers
// are part of what is under test.
//
// Two problems have to be solved to make the comparison meaningful:
//
// 1. `the_house` is file-scope mutable state, so results depend on call
//    history. Every scenario therefore loads a *fresh copy of the shared
//    object at a unique filesystem path*: `dlopen` keys its cache on the
//    resolved path, so a unique path is guaranteed to produce a freshly
//    initialised data segment (`{2, 5, 2.5}`) instead of reusing an already
//    mutated mapping.
//
// 2. Both libraries write to `stdout` through the process' single libc
//    `FILE *stdout`. Output is captured by temporarily `dup2`-ing a temp file
//    over file descriptor 1 and `fflush(NULL)`-ing before restoring. That is
//    process-global, so it is serialised behind a mutex because the libtest
//    harness runs tests on multiple threads.

#![allow(dead_code)]

use std::ffi::{CStr, CString};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use core::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

pub type DriverFn = unsafe extern "C" fn(*const c_char);
pub type RunFn = unsafe extern "C" fn(c_int);

// ---------------------------------------------------------------- library paths

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C `.so` produced by the CMake build.
pub fn c_so() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with \
         `cd c_src && mkdir -p build && cd build && cmake .. \
          -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`"
    );
    p
}

/// Path to the Rust `cdylib`. Prefers the release artifact (the one an external
/// consumer would link against) and falls back to debug.
pub fn rust_so() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib not found; run `cargo build --release` first");
}

// ------------------------------------------------------------------- stdout cap

fn io_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    // A poisoned lock only means some other test panicked while holding it;
    // the fd has already been restored by the guard's drop, so recover.
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(e) => e.into_inner(),
    }
}

fn scratch_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("driver-diff-{}", std::process::id()));
    std::fs::create_dir_all(&d).expect("create scratch dir");
    d
}

fn unique(tag: &str) -> String {
    static N: AtomicU64 = AtomicU64::new(0);
    format!("{tag}-{}", N.fetch_add(1, Ordering::Relaxed))
}

/// Run `f` with file descriptor 1 pointed at a temp file and return every byte
/// written to it (by Rust *or* by libc `printf`/`puts`).
pub fn capture<R, F: FnOnce() -> R>(f: F) -> (R, Vec<u8>) {
    let _guard = io_lock();
    let path = scratch_dir().join(unique("stdout"));
    let file = std::fs::File::create(&path).expect("create capture file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");
    }

    let out = f();

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }
    drop(file);

    let mut buf = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut buf)
        .expect("read capture file");
    let _ = std::fs::remove_file(&path);
    (out, buf)
}

// ----------------------------------------------------------------- the sessions

/// A freshly loaded copy of one implementation, with pristine `the_house`.
pub struct Session {
    name: &'static str,
    _lib: libloading::Library,
    copy_path: PathBuf,
    pub driver: DriverFn,
    pub run: RunFn,
}

impl Session {
    fn open(name: &'static str, original: &Path) -> Session {
        // Unique path => guaranteed-fresh data segment.
        let copy_path = scratch_dir().join(format!("{}-libdriver.so", unique(name)));
        std::fs::copy(original, &copy_path)
            .unwrap_or_else(|e| panic!("copy {original:?} -> {copy_path:?}: {e}"));

        let lib = unsafe { libloading::Library::new(&copy_path) }
            .unwrap_or_else(|e| panic!("dlopen {copy_path:?}: {e}"));
        let driver: DriverFn = unsafe {
            *lib.get::<DriverFn>(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: missing exported symbol `driver`: {e}"))
        };
        let run: RunFn = unsafe {
            *lib.get::<RunFn>(b"run\0")
                .unwrap_or_else(|e| panic!("{name}: missing exported symbol `run`: {e}"))
        };
        Session {
            name,
            _lib: lib,
            copy_path,
            driver,
            run,
        }
    }

    pub fn fresh_c() -> Session {
        Session::open("c", &c_so())
    }

    pub fn fresh_rust() -> Session {
        Session::open("rust", &rust_so())
    }

    pub fn name(&self) -> &'static str {
        self.name
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.copy_path);
    }
}

// ------------------------------------------------------------------- operations

/// One call into the library under test.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// `run(extra_bedrooms)` — the low-level entry point.
    Run(i32),
    /// `driver(in)` — the convenience wrapper. Bytes are the C string content
    /// without the terminating NUL.
    Driver(Vec<u8>),
}

impl Op {
    pub fn driver(s: &str) -> Op {
        Op::Driver(s.as_bytes().to_vec())
    }

    pub fn describe(&self) -> String {
        match self {
            Op::Run(v) => format!("run({v})"),
            Op::Driver(bytes) => {
                let s = String::from_utf8_lossy(bytes);
                let shown: String = s.chars().take(64).collect();
                format!(
                    "driver({:?}{})",
                    shown,
                    if s.chars().count() > 64 { "…" } else { "" }
                )
            }
        }
    }

    fn invoke(&self, sess: &Session) {
        match self {
            Op::Run(v) => unsafe { (sess.run)(*v) },
            Op::Driver(bytes) => {
                let cs = CString::new(bytes.clone())
                    .expect("test input must not contain an interior NUL");
                unsafe { (sess.driver)(cs.as_ptr()) }
            }
        }
    }
}

/// Execute the same op sequence against a fresh C session and a fresh Rust
/// session, comparing the stdout bytes of EVERY individual call.
///
/// Comparing per call (rather than only the concatenation at the end) keeps the
/// accumulated `the_house` state under test while still pinpointing the first
/// divergence.
pub fn diff_sequence(row: &str, ops: &[Op]) {
    let c = Session::fresh_c();
    let r = Session::fresh_rust();

    for (i, op) in ops.iter().enumerate() {
        let ((), out_c) = capture(|| op.invoke(&c));
        let ((), out_r) = capture(|| op.invoke(&r));
        if out_c != out_r {
            panic!(
                "[{row}] divergence at op #{i} {}\n  C    ({} bytes): {:?}\n  Rust ({} bytes): {:?}\n  \
                 preceding ops: {}",
                op.describe(),
                out_c.len(),
                String::from_utf8_lossy(&out_c),
                out_r.len(),
                String::from_utf8_lossy(&out_r),
                ops[i.saturating_sub(3)..i]
                    .iter()
                    .map(Op::describe)
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
        assert!(
            !out_c.is_empty(),
            "[{row}] op #{i} {} produced no output at all — the scenario is not \
             exercising the library",
            op.describe()
        );
    }
}

/// Like [`diff_sequence`] but each op gets a pristine pair of libraries, so the
/// op is observed in isolation from the pristine `{2, 5, 2.5}` state.
pub fn diff_each_from_pristine(row: &str, ops: &[Op]) {
    for (i, op) in ops.iter().enumerate() {
        let c = Session::fresh_c();
        let r = Session::fresh_rust();
        let ((), out_c) = capture(|| op.invoke(&c));
        let ((), out_r) = capture(|| op.invoke(&r));
        assert_eq!(
            String::from_utf8_lossy(&out_c),
            String::from_utf8_lossy(&out_r),
            "[{row}] pristine-state divergence at op #{i} {}",
            op.describe()
        );
    }
}

/// Assert that a rejected `driver` call left `the_house` untouched on both
/// sides: the follow-up `run(0)` must print the pristine numbers.
pub fn diff_rejection_leaves_state_intact(row: &str, bad_inputs: &[&str]) {
    for (i, bad) in bad_inputs.iter().enumerate() {
        let c = Session::fresh_c();
        let r = Session::fresh_rust();

        let ((), err_c) = capture(|| Op::driver(bad).invoke(&c));
        let ((), err_r) = capture(|| Op::driver(bad).invoke(&r));
        assert_eq!(
            String::from_utf8_lossy(&err_c),
            String::from_utf8_lossy(&err_r),
            "[{row}] rejection output differs for input #{i} {bad:?}"
        );
        assert_eq!(
            err_c, b"An error occurred\n",
            "[{row}] input #{i} {bad:?} was expected to be rejected by the C \
             library but produced: {:?}",
            String::from_utf8_lossy(&err_c)
        );

        let ((), st_c) = capture(|| Op::Run(0).invoke(&c));
        let ((), st_r) = capture(|| Op::Run(0).invoke(&r));
        assert_eq!(
            String::from_utf8_lossy(&st_c),
            String::from_utf8_lossy(&st_r),
            "[{row}] post-rejection state differs for input #{i} {bad:?}"
        );
        // Pristine state means the first printed line is the initial house.
        let first = st_c.split(|b| *b == b'\n').next().unwrap().to_vec();
        assert_eq!(
            String::from_utf8_lossy(&first),
            "The house has 2 floors, 5 bedrooms, and 2.5 bathrooms",
            "[{row}] input #{i} {bad:?}: the C library mutated state on the \
             error path (unexpected but authoritative — adjust the expectation, \
             not the C)"
        );
    }
}

/// Assert BOTH libraries are accepted (i.e. not the error branch) for `input`.
pub fn assert_accepted_by_c(input: &str) {
    let c = Session::fresh_c();
    let ((), out) = capture(|| Op::driver(input).invoke(&c));
    assert_ne!(
        out, b"An error occurred\n",
        "expected the C library to ACCEPT {input:?}"
    );
}

// ----------------------------------------------------------------------- random

/// xorshift64* — deterministic, dependency-free, fixed seed per row.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i64(&mut self, lo: i64, hi: i64) -> i64 {
        debug_assert!(lo <= hi);
        let span = (hi as i128 - lo as i128 + 1) as u128;
        (lo as i128 + (self.next_u64() as u128 % span) as i128) as i64
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next_u64() % items.len() as u64) as usize]
    }
    pub fn chance(&mut self, one_in: u64) -> bool {
        self.next_u64() % one_in == 0
    }
}

/// Base seed shared by all rows (each row perturbs it) for reproducibility.
pub const SEED: u64 = 0x5EED_1234;

// ---------------------------------------------------------- crash-parity helper

/// Environment variable that turns this test binary into a "call `driver` with a
/// NULL pointer and die" worker, used for crash-parity checks that cannot be
/// done in-process.
pub const NULL_WORKER_ENV: &str = "DRIVER_NULL_WORKER_SO";

/// If the worker env var is set, dlopen that library, call `driver(NULL)` and
/// never return normally. Returns `false` when not in worker mode.
pub fn maybe_act_as_null_worker() -> bool {
    let Ok(path) = std::env::var(NULL_WORKER_ENV) else {
        return false;
    };
    let lib = unsafe { libloading::Library::new(&path) }.expect("worker: dlopen");
    let driver: DriverFn = unsafe { *lib.get::<DriverFn>(b"driver\0").expect("worker: driver") };
    unsafe { driver(std::ptr::null()) };
    // If it somehow survives, report that distinctly (exit code 7).
    unsafe { fflush(std::ptr::null_mut()) };
    std::process::exit(7);
}

/// Spawn this test binary as a NULL-pointer worker against `so` and report the
/// raw wait status.
pub fn null_worker_status(so: &Path, worker_test_name: &str) -> std::process::Output {
    let exe = std::env::current_exe().expect("current_exe");
    std::process::Command::new(exe)
        .arg("--exact")
        .arg(worker_test_name)
        .arg("--nocapture")
        .arg("--test-threads=1")
        .env(NULL_WORKER_ENV, so)
        .output()
        .expect("spawn null worker")
}

/// Helper to render bytes for assertion messages.
pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Convenience: the CStr for a literal used in expectations.
pub fn cstr(s: &'static str) -> &'static CStr {
    CStr::from_bytes_with_nul(s.as_bytes()).expect("literal must end in NUL")
}

// ------------------------------------------------------------ minimal harness
//
// The test binaries use `harness = false`. A custom sequential runner is
// required rather than libtest, because capturing stdout means `dup2`-ing over
// file descriptor 1 for the whole process: libtest's own progress lines
// ("test foo ... ok") are written from its main thread while other test threads
// run, so they leak into the captured bytes and produce phantom divergences.
// Running scenarios strictly sequentially, with all runner output emitted only
// while no capture is active, removes that entire failure mode.

pub type TestFn = fn();

fn panic_message_slot() -> &'static Mutex<Option<String>> {
    static SLOT: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

fn install_panic_hook() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        std::panic::set_hook(Box::new(|info| {
            let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
                (*s).to_string()
            } else if let Some(s) = info.payload().downcast_ref::<String>() {
                s.clone()
            } else {
                "<non-string panic payload>".to_string()
            };
            let loc = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_else(|| "<unknown>".to_string());
            if let Ok(mut slot) = panic_message_slot().lock() {
                *slot = Some(format!("at {loc}\n{payload}"));
            }
        }));
    });
}

/// Sequential test runner. Supports the argument forms this project uses:
/// a bare substring filter, `--exact <name>`, and the ignored-but-accepted
/// `--nocapture` / `--test-threads=N` / `--list`.
pub fn run_suite(suite: &str, tests: &[(&str, TestFn)]) -> ! {
    // Worker mode short-circuits everything (used for crash-parity checks).
    maybe_act_as_null_worker();

    install_panic_hook();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut filter: Option<String> = None;
    let mut exact = false;
    let mut list = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--exact" => {
                exact = true;
                if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                    filter = Some(args[i + 1].clone());
                    i += 1;
                }
            }
            "--list" => list = true,
            "--nocapture" | "--quiet" | "-q" | "--show-output" | "--include-ignored"
            | "--ignored" | "--force-run-in-process" => {}
            a if a.starts_with("--test-threads") || a.starts_with("--format") => {
                if a == "--test-threads" || a == "--format" {
                    i += 1;
                }
            }
            a if a.starts_with("--") => {}
            a => filter = Some(a.to_string()),
        }
        i += 1;
    }

    let selected: Vec<&(&str, TestFn)> = tests
        .iter()
        .filter(|(name, _)| match &filter {
            None => true,
            Some(f) if exact => *name == f.as_str(),
            Some(f) => name.contains(f.as_str()),
        })
        .collect();

    if list {
        for (name, _) in &selected {
            println!("{name}: test");
        }
        std::process::exit(0);
    }

    println!("\nrunning {} tests ({suite})", selected.len());
    let mut failures: Vec<(String, String)> = Vec::new();
    for (name, f) in &selected {
        let started = std::time::Instant::now();
        if let Ok(mut slot) = panic_message_slot().lock() {
            *slot = None;
        }
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(*f));
        // Make sure a panic mid-capture cannot leave fd 1 redirected.
        let elapsed = started.elapsed();
        match outcome {
            Ok(()) => println!("test {name} ... ok ({:.2?})", elapsed),
            Err(_) => {
                println!("test {name} ... FAILED ({:.2?})", elapsed);
                let msg = panic_message_slot()
                    .lock()
                    .ok()
                    .and_then(|mut s| s.take())
                    .unwrap_or_else(|| "<no panic message captured>".to_string());
                failures.push(((*name).to_string(), msg));
            }
        }
    }

    if failures.is_empty() {
        println!(
            "\ntest result: ok. {} passed; 0 failed\n",
            selected.len()
        );
        std::process::exit(0);
    } else {
        println!("\nfailures:\n");
        for (name, msg) in &failures {
            println!("---- {name} ----\n{msg}\n");
        }
        println!(
            "test result: FAILED. {} passed; {} failed\n",
            selected.len() - failures.len(),
            failures.len()
        );
        std::process::exit(101);
    }
}
