//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls them only through
//! their exported C symbols, so the `#[no_mangle]` export wrappers are part of
//! what is under test. Nothing in the Rust crate is ever called directly.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture and for crash-isolating child processes.
// These resolve to the *process's* libc, i.e. the very same libc whose `printf`
// both shared objects call, so buffering is shared and `fflush` is meaningful.
// ---------------------------------------------------------------------------
unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn unlink(path: *const c_char) -> c_int;
    fn lseek(fd: c_int, off: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
}

const O_RDWR: c_int = 2;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

// ---------------------------------------------------------------------------
// Function signatures of the two exported symbols.
// ---------------------------------------------------------------------------
pub type FmaArrayFn = unsafe extern "C" fn(
    out: *mut c_int,
    mul1: *const c_int,
    mul2: *const c_int,
    add: *const c_int,
    len: c_int,
);
pub type DriverFn = unsafe extern "C" fn(data: *const c_int, len: c_int);

pub struct Impl {
    pub name: &'static str,
    #[allow(unused)]
    lib: libloading::Library,
    pub fma_array: FmaArrayFn,
    pub driver: DriverFn,
}

impl Impl {
    fn load(name: &'static str, path: &PathBuf) -> Impl {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
        let fma_array: FmaArrayFn = unsafe {
            let s: libloading::Symbol<FmaArrayFn> = lib
                .get(b"fma_array\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `fma_array`: {e}"));
            *s
        };
        let driver: DriverFn = unsafe {
            let s: libloading::Symbol<DriverFn> = lib
                .get(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `driver`: {e}"));
            *s
        };
        Impl { name, lib, fma_array, driver }
    }
}

pub struct Libs {
    pub c: Impl,
    pub rs: Impl,
}

fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB_PATH") {
        return PathBuf::from(p);
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root = manifest.parent().expect("crate has a parent dir");
    root.join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB_PATH") {
        return PathBuf::from(p);
    }
    // .../target/<profile>/deps/<testbin>  ->  .../target/<profile>/libdriver.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile> dir");
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for p in ["target/release/libdriver.so", "target/debug/libdriver.so"] {
        let c = manifest.join(p);
        if c.exists() {
            return c;
        }
    }
    candidate
}

/// `(c_so, rust_so)` — the two shared objects under comparison.
pub fn lib_paths() -> (PathBuf, PathBuf) {
    (c_lib_path(), rust_lib_path())
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        assert!(cp.exists(), "C shared library not found at {}", cp.display());
        assert!(
            rp.exists(),
            "Rust shared library not found at {}",
            rp.display()
        );
        Libs {
            c: Impl::load("C", &cp),
            rs: Impl::load("Rust", &rp),
        }
    })
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed per test for reproducibility.
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
    /// Full-range random `i32`.
    pub fn i32_any(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn range_usize(&mut self, lo: usize, hi: usize) -> usize {
        debug_assert!(lo <= hi);
        lo + (self.next_u64() as usize) % (hi - lo + 1)
    }
    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.next_u64() as usize) % xs.len()]
    }
}

/// The interesting `i32` boundary values the C's `imul`/`add` can wrap on.
pub const BOUNDARY: &[i32] = &[
    i32::MIN,
    i32::MIN + 1,
    i32::MIN / 2,
    -65536,
    -65535,
    -3,
    -2,
    -1,
    0,
    1,
    2,
    3,
    65535,
    65536,
    i32::MAX / 2,
    i32::MAX - 1,
    i32::MAX,
];

// ---------------------------------------------------------------------------
// stdout capture.
//
// `driver` prints via libc `printf`, so the only faithful way to observe it is
// to redirect file descriptor 1 and compare the raw bytes. But fd 1 is
// process-global, and libtest's own progress output ("test foo ... ok") is
// written to fd 1 from the harness's main thread while worker threads run --
// so redirecting fd 1 in-process races with the harness and corrupts the
// capture.
//
// The robust answer is to do the capture in a dedicated, single-threaded
// SUBPROCESS: we re-exec this very test binary in "worker" mode, hand it a
// batch of cases via a scratch directory, and it redirects fd 1 per case with
// nothing else writing there. The worker's own libtest chatter is sent to
// /dev/null, so each output file holds exactly the bytes `driver` printed.
// ---------------------------------------------------------------------------

/// Name of the `#[test]` function that acts as the subprocess entry point.
pub const WORKER_TEST_NAME: &str = "zz_stdout_worker";
const WORKER_ENV: &str = "DIFFTEST_WORKER_DIR";

fn spawn_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

fn scratch_dir(tag: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let mut d = std::env::temp_dir();
    d.push(format!(
        "difftest-{}-{}-{}",
        std::process::id(),
        tag,
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("create scratch dir");
    d
}

fn encode_cases(cases: &[(Vec<i32>, c_int)]) -> String {
    let mut s = String::new();
    for (data, len) in cases {
        s.push_str(&len.to_string());
        s.push(' ');
        for v in data {
            s.push_str(&format!("{:08x}", *v as u32));
        }
        s.push('\n');
    }
    s
}

fn decode_cases(text: &str) -> Vec<(Vec<i32>, c_int)> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let mut it = l.splitn(2, ' ');
            let len: c_int = it.next().unwrap().parse().expect("len");
            let hex = it.next().unwrap_or("");
            let data = (0..hex.len() / 8)
                .map(|i| u32::from_str_radix(&hex[i * 8..i * 8 + 8], 16).unwrap() as i32)
                .collect();
            (data, len)
        })
        .collect()
}

/// Body of the `zz_stdout_worker` test. In a normal test run the environment
/// variable is unset and this is a no-op; in a spawned worker it performs every
/// case and terminates the process.
pub fn worker_main_if_requested() {
    let Ok(dir) = std::env::var(WORKER_ENV) else {
        return;
    };
    let dir = PathBuf::from(dir);
    let text = std::fs::read_to_string(dir.join("cases.txt")).expect("read cases.txt");
    let cases = decode_cases(&text);
    let l = libs();

    unsafe {
        // Park the inherited fd 1 (which the parent pointed at /dev/null) so we
        // can restore it between cases.
        let saved = dup(1);
        assert!(saved >= 0);
        for (i, (data, len)) in cases.iter().enumerate() {
            for (suffix, imp) in [("C", &l.c), ("R", &l.rs)] {
                let path = dir.join(format!("{i}.{suffix}"));
                let cpath = std::ffi::CString::new(path.to_str().unwrap()).unwrap();
                let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
                assert!(fd >= 0);
                fflush(std::ptr::null_mut());
                assert!(dup2(fd, 1) >= 0);

                (imp.driver)(data.as_ptr(), *len);

                fflush(std::ptr::null_mut());
                assert!(dup2(saved, 1) >= 0);
                close(fd);
            }
        }
        fflush(std::ptr::null_mut());
        _exit(0);
    }
}

/// Runs a whole batch of `driver` cases against BOTH shared objects inside one
/// isolated subprocess and returns `(c_stdout, rust_stdout)` per case.
pub fn run_driver_batch(cases: &[(Vec<i32>, c_int)]) -> Vec<(Vec<u8>, Vec<u8>)> {
    if cases.is_empty() {
        return Vec::new();
    }
    let dir = scratch_dir("drv");
    std::fs::write(dir.join("cases.txt"), encode_cases(cases)).expect("write cases.txt");

    let exe = std::env::current_exe().expect("current_exe");
    let status = {
        // Serialize spawns so a burst of parallel tests cannot exhaust fds.
        let _g = spawn_lock().lock().unwrap_or_else(|e| e.into_inner());
        std::process::Command::new(&exe)
            .args([
                "--exact",
                WORKER_TEST_NAME,
                "--test-threads=1",
                "--quiet",
            ])
            .env(WORKER_ENV, &dir)
            // libtest's own progress output must not land in the capture files.
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("spawn stdout worker")
    };
    assert!(
        status.success(),
        "stdout worker subprocess failed: {status:?} (scratch dir {})",
        dir.display()
    );

    let mut out = Vec::with_capacity(cases.len());
    for i in 0..cases.len() {
        let c = std::fs::read(dir.join(format!("{i}.C")))
            .unwrap_or_else(|e| panic!("missing C output for case {i}: {e}"));
        let r = std::fs::read(dir.join(format!("{i}.R")))
            .unwrap_or_else(|e| panic!("missing Rust output for case {i}: {e}"));
        out.push((c, r));
    }
    let _ = std::fs::remove_dir_all(&dir);
    out
}

// ---------------------------------------------------------------------------
// Crash isolation for the C code's undefined-behaviour paths (ERRORS.md rows
// 10, 11, 14, 15). Those inputs make the C die outright, so they must run in a
// separate process. As with stdout capture we re-exec this test binary rather
// than `fork()`ing a multi-threaded process.
// ---------------------------------------------------------------------------

/// Which implementation to exercise in the isolated child.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Which {
    C,
    Rust,
}

impl Which {
    pub fn tag(self) -> &'static str {
        match self {
            Which::C => "C",
            Which::Rust => "Rust",
        }
    }
}

/// What the isolated child should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrashCase {
    pub which: Which,
    /// `"driver"` or `"fma"`.
    pub func: &'static str,
    pub len: c_int,
    /// Bitmask of which pointer arguments are passed as NULL.
    /// For `fma`: bit0 = `out`, bit1 = `mul1`, bit2 = `mul2`, bit3 = `add`.
    /// For `driver`: bit0 = `data`.
    pub null_mask: u32,
    /// Number of elements allocated for the non-NULL pointers.
    pub alloc: usize,
}

/// Null-mask helpers.
pub const NULL_NONE: u32 = 0;
pub const NULL_OUT: u32 = 1 << 0;
pub const NULL_MUL1: u32 = 1 << 1;
pub const NULL_MUL2: u32 = 1 << 2;
pub const NULL_ADD: u32 = 1 << 3;
pub const NULL_ALL4: u32 = NULL_OUT | NULL_MUL1 | NULL_MUL2 | NULL_ADD;
/// For `driver`, the single `data` pointer.
pub const NULL_DATA: u32 = 1 << 0;

/// How the isolated child terminated.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Outcome {
    /// Ran to completion with this exit code (0 == survived the call).
    Exited(i32),
    /// Killed by this signal (11 == SIGSEGV, 6 == SIGABRT, 4 == SIGILL).
    Signaled(i32),
    /// Still running when the deadline expired, so it was killed. Used for the
    /// astronomically-large-`len` rows, where a surviving implementation would
    /// print billions of lines.
    TimedOut,
}

pub const SIGSEGV: i32 = 11;
pub const SIGABRT: i32 = 6;
pub const SIGBUS: i32 = 7;

const CRASH_ENV: &str = "DIFFTEST_CRASH_SPEC";
pub const CRASH_WORKER_TEST_NAME: &str = "zz_crash_worker";

/// Body of the `zz_crash_worker` test. No-op unless the env var is set.
pub fn crash_worker_main_if_requested() {
    let Ok(spec) = std::env::var(CRASH_ENV) else {
        return;
    };
    let parts: Vec<String> = spec.split('|').map(|s| s.to_string()).collect();
    assert_eq!(parts.len(), 5, "bad crash spec {spec:?}");
    let which = parts[0].clone();
    let func_owned = parts[1].clone();
    let len: c_int = parts[2].parse().expect("len");
    let null_mask: u32 = parts[3].parse().expect("null_mask");
    let alloc: usize = parts[4].parse().expect("alloc");

    // The C's `driver` puts `int out[len]` on the CALLER'S STACK. libtest runs
    // tests on worker threads whose stack is only ~2 MiB, which would make the
    // VLA threshold depend on libtest internals. Run the call on a thread with
    // an explicitly chosen stack size so the threshold is well defined and
    // reproducible.
    let h = std::thread::Builder::new()
        .stack_size(WORKER_STACK_BYTES)
        .spawn(move || crash_worker_body(&which, &func_owned, len, null_mask, alloc))
        .expect("spawn sized worker thread");
    let _ = h.join();
    unsafe {
        fflush(std::ptr::null_mut());
        _exit(0);
    }
}

/// Stack size of the thread the crash-worker performs its call on. The C's
/// `driver` VLA is `len * 4` bytes, so `len` below ~2^20 fits and `len` at or
/// above ~2^21 cannot.
pub const WORKER_STACK_BYTES: usize = 8 << 20; // 8 MiB

fn crash_worker_body(which: &str, func: &str, len: c_int, null_mask: u32, alloc: usize) {
    let l = libs();
    let imp = if which == "C" { &l.c } else { &l.rs };

    unsafe {
        // Send everything the call prints to /dev/null: these cases can print
        // billions of lines and we only care about how the process terminates.
        let devnull = std::ffi::CString::new("/dev/null").unwrap();
        let fd = open(devnull.as_ptr(), O_RDWR, 0 as c_int);
        if fd >= 0 {
            fflush(std::ptr::null_mut());
            dup2(fd, 1);
        }

        // Separate backing buffers so that a NULL for one argument does not
        // disturb the others. Filled with `vec![k; n]` rather than an iterator
        // so that setting up a huge buffer stays cheap even in a debug build
        // (these cases use `alloc` values in the tens of millions).
        let n = alloc.max(1);

        match func {
            "driver" => {
                let b: Vec<c_int> = vec![3; n];
                let p = if null_mask & NULL_DATA != 0 {
                    std::ptr::null()
                } else {
                    b.as_ptr()
                };
                (imp.driver)(p, len)
            }
            "fma" => {
                let mut b0: Vec<c_int> = vec![0; n];
                let b1: Vec<c_int> = vec![3; n];
                let b2: Vec<c_int> = vec![5; n];
                let b3: Vec<c_int> = vec![7; n];
                let out = if null_mask & NULL_OUT != 0 {
                    std::ptr::null_mut()
                } else {
                    b0.as_mut_ptr()
                };
                let m1 = if null_mask & NULL_MUL1 != 0 {
                    std::ptr::null()
                } else {
                    b1.as_ptr()
                };
                let m2 = if null_mask & NULL_MUL2 != 0 {
                    std::ptr::null()
                } else {
                    b2.as_ptr()
                };
                let ad = if null_mask & NULL_ADD != 0 {
                    std::ptr::null()
                } else {
                    b3.as_ptr()
                };
                (imp.fma_array)(out, m1, m2, ad, len)
            }
            _ => panic!("bad func {func}"),
        }
        fflush(std::ptr::null_mut());
    }
}

/// Runs one `CrashCase` in a fresh process and reports how it terminated.
pub fn run_crash_case(case: &CrashCase, timeout: std::time::Duration) -> Outcome {
    use std::os::unix::process::ExitStatusExt;

    let spec = format!(
        "{}|{}|{}|{}|{}",
        case.which.tag(),
        case.func,
        case.len,
        case.null_mask,
        case.alloc,
    );
    let exe = std::env::current_exe().expect("current_exe");
    let mut child = {
        let _g = spawn_lock().lock().unwrap_or_else(|e| e.into_inner());
        std::process::Command::new(&exe)
            .args([
                "--exact",
                CRASH_WORKER_TEST_NAME,
                "--test-threads=1",
                "--quiet",
            ])
            .env(CRASH_ENV, &spec)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("spawn crash worker")
    };

    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait().expect("try_wait") {
            Some(status) => {
                return if let Some(sig) = status.signal() {
                    Outcome::Signaled(sig)
                } else {
                    Outcome::Exited(status.code().unwrap_or(-1))
                };
            }
            None => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Outcome::TimedOut;
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }
}

/// Convenience: run the same case against both libraries.
pub fn run_crash_both(
    func: &'static str,
    len: c_int,
    null_mask: u32,
    alloc: usize,
    timeout: std::time::Duration,
) -> (Outcome, Outcome) {
    let mk = |which| CrashCase { which, func, len, null_mask, alloc };
    (
        run_crash_case(&mk(Which::C), timeout),
        run_crash_case(&mk(Which::Rust), timeout),
    )
}

/// True when the Rust `.so` under test was built with `debug_assertions`.
///
/// This matters for the null-pointer rows. Since Rust 1.78 a debug build
/// inserts `assert_unsafe_precondition` null/alignment checks into raw-pointer
/// reads and writes, so dereferencing NULL *panics* (SIGABRT across the FFI
/// boundary) instead of faulting (SIGSEGV) the way the C does. The shipped
/// artifact is the release `cdylib`; in release the two agree signal-for-signal.
pub fn rust_so_is_debug() -> bool {
    let (_, rp) = lib_paths();
    let p = rp.to_string_lossy().to_string();
    if p.contains("/release/") {
        return false;
    }
    if p.contains("/debug/") {
        return true;
    }
    cfg!(debug_assertions)
}

/// Assert that C and Rust failed the same way on an input where the C faults.
///
/// In a release build this demands the identical termination signal. In a debug
/// build it demands that both died hard, because Rust's debug-only null/
/// alignment preconditions convert a null dereference into an abort.
pub fn assert_same_fault(ctx: &str, c: Outcome, r: Outcome) {
    assert!(is_fault(c), "{ctx}: expected the C to fault, got {c:?}");
    if rust_so_is_debug() {
        assert!(
            is_fault(r),
            "{ctx}: C={c:?} but Rust={r:?} — both must fail hard"
        );
    } else {
        assert_eq!(
            r, c,
            "{ctx}: C={c:?} but Rust={r:?} — the release build must fault \
             identically"
        );
    }
}

/// True if the outcome is a hard fault (the shapes the C's UB takes).
pub fn is_fault(o: Outcome) -> bool {
    matches!(o, Outcome::Signaled(SIGSEGV) | Outcome::Signaled(SIGBUS) | Outcome::Signaled(SIGABRT))
}

// ---------------------------------------------------------------------------
// Differential helpers.
// ---------------------------------------------------------------------------

/// How the four `fma_array` pointer arguments alias one another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alias {
    /// out, mul1, mul2, add are four independent buffers.
    AllDistinct,
    /// out == mul1
    OutIsMul1,
    /// out == add
    OutIsAdd,
    /// mul1 == mul2 (squaring)
    Mul1IsMul2,
    /// out == mul1 == mul2 == add — exactly how `inner` calls it.
    AllSame,
    /// mul1 == out + 1, mul2 == add == out  (read one ahead of the write)
    Mul1AheadOfOut,
    /// out == mul1 + 1, mul2 == add == mul1 (write one ahead of the read)
    OutAheadOfMul1,
}

pub const ALL_ALIASES: &[Alias] = &[
    Alias::AllDistinct,
    Alias::OutIsMul1,
    Alias::OutIsAdd,
    Alias::Mul1IsMul2,
    Alias::AllSame,
    Alias::Mul1AheadOfOut,
    Alias::OutAheadOfMul1,
];

/// Result of one `fma_array` invocation: the final contents of every buffer the
/// call could possibly have written to.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FmaResult {
    pub buffers: Vec<Vec<i32>>,
}

/// Invoke `fma_array` from one implementation under the given aliasing scheme.
///
/// Buffers are laid out with `PAD` sentinel elements after the live region so an
/// off-by-one write past `len` shows up as a diff instead of silent corruption.
pub const PAD: usize = 16;
pub const SENTINEL: i32 = 0x5A5A_5A5Au32 as i32;

pub fn call_fma(
    imp: &Impl,
    alias: Alias,
    len: c_int,
    a: &[i32], // source values for mul1 (and for the shared buffer)
    b: &[i32], // source values for mul2
    c: &[i32], // source values for add
    out_init: &[i32],
) -> FmaResult {
    let n = if len > 0 { len as usize } else { 0 };
    // Every buffer is over-allocated: n + 1 live slots (for the +1 offset
    // schemes) plus PAD sentinels.
    let cap = n + 1 + PAD;

    let mk = |src: &[i32]| -> Vec<i32> {
        let mut v = vec![SENTINEL; cap];
        for i in 0..(n + 1).min(src.len()) {
            v[i] = src[i];
        }
        v
    };

    match alias {
        Alias::AllDistinct => {
            let mut o = mk(out_init);
            let m1 = mk(a);
            let m2 = mk(b);
            let ad = mk(c);
            unsafe {
                (imp.fma_array)(o.as_mut_ptr(), m1.as_ptr(), m2.as_ptr(), ad.as_ptr(), len)
            };
            FmaResult { buffers: vec![o, m1, m2, ad] }
        }
        Alias::OutIsMul1 => {
            let mut o = mk(a);
            let m2 = mk(b);
            let ad = mk(c);
            unsafe {
                let p = o.as_mut_ptr();
                (imp.fma_array)(p, p as *const c_int, m2.as_ptr(), ad.as_ptr(), len)
            };
            FmaResult { buffers: vec![o, m2, ad] }
        }
        Alias::OutIsAdd => {
            let mut o = mk(c);
            let m1 = mk(a);
            let m2 = mk(b);
            unsafe {
                let p = o.as_mut_ptr();
                (imp.fma_array)(p, m1.as_ptr(), m2.as_ptr(), p as *const c_int, len)
            };
            FmaResult { buffers: vec![o, m1, m2] }
        }
        Alias::Mul1IsMul2 => {
            let mut o = mk(out_init);
            let m = mk(a);
            let ad = mk(c);
            unsafe {
                (imp.fma_array)(o.as_mut_ptr(), m.as_ptr(), m.as_ptr(), ad.as_ptr(), len)
            };
            FmaResult { buffers: vec![o, m, ad] }
        }
        Alias::AllSame => {
            let mut o = mk(a);
            unsafe {
                let p = o.as_mut_ptr();
                (imp.fma_array)(
                    p,
                    p as *const c_int,
                    p as *const c_int,
                    p as *const c_int,
                    len,
                )
            };
            FmaResult { buffers: vec![o] }
        }
        Alias::Mul1AheadOfOut => {
            // mul1 = out + 1, mul2 = add = out
            let mut o = mk(a);
            unsafe {
                let p = o.as_mut_ptr();
                (imp.fma_array)(
                    p,
                    p.add(1) as *const c_int,
                    p as *const c_int,
                    p as *const c_int,
                    len,
                )
            };
            FmaResult { buffers: vec![o] }
        }
        Alias::OutAheadOfMul1 => {
            // out = base + 1, mul1 = mul2 = add = base
            let mut o = mk(a);
            unsafe {
                let p = o.as_mut_ptr();
                (imp.fma_array)(
                    p.add(1),
                    p as *const c_int,
                    p as *const c_int,
                    p as *const c_int,
                    len,
                )
            };
            FmaResult { buffers: vec![o] }
        }
    }
}

/// Run one `fma_array` configuration against BOTH shared objects and assert the
/// resulting buffers are bit-identical.
pub fn diff_fma(
    ctx: &str,
    alias: Alias,
    len: c_int,
    a: &[i32],
    b: &[i32],
    c: &[i32],
    out_init: &[i32],
) {
    let l = libs();
    let rc = call_fma(&l.c, alias, len, a, b, c, out_init);
    let rr = call_fma(&l.rs, alias, len, a, b, c, out_init);
    if rc != rr {
        for (bi, (bc, br)) in rc.buffers.iter().zip(rr.buffers.iter()).enumerate() {
            if bc != br {
                let first = bc
                    .iter()
                    .zip(br.iter())
                    .position(|(x, y)| x != y)
                    .unwrap_or(0);
                panic!(
                    "fma_array divergence [{ctx}] alias={alias:?} len={len}\n\
                     buffer #{bi} first differs at index {first}: C={} Rust={}\n\
                     inputs a={:?} b={:?} c={:?} out_init={:?}\n\
                     C   buf={:?}\nRust buf={:?}",
                    bc[first],
                    br[first],
                    &a[..a.len().min(16)],
                    &b[..b.len().min(16)],
                    &c[..c.len().min(16)],
                    &out_init[..out_init.len().min(16)],
                    &bc[..bc.len().min(24)],
                    &br[..br.len().min(24)],
                );
            }
        }
        panic!("fma_array divergence [{ctx}] alias={alias:?} len={len} (buffer count differs)");
    }
}

/// Run a batch of `driver` configurations against BOTH shared objects and
/// assert their stdout bytes are identical, case by case. Returns the (agreed)
/// stdout of each case.
pub fn diff_driver_batch(ctx: &str, cases: &[(Vec<i32>, c_int)]) -> Vec<Vec<u8>> {
    let results = run_driver_batch(cases);
    let mut agreed = Vec::with_capacity(results.len());
    for (i, ((data, len), (oc, or))) in cases.iter().zip(results.iter()).enumerate() {
        if oc != or {
            let first = oc
                .iter()
                .zip(or.iter())
                .position(|(x, y)| x != y)
                .unwrap_or(oc.len().min(or.len()));
            panic!(
                "driver stdout divergence [{ctx}] case #{i} len={len}\n\
                 first differing byte offset {first} (C len {} vs Rust len {})\n\
                 C   = {:?}\n Rust= {:?}\n data(first16)={:?}",
                oc.len(),
                or.len(),
                String::from_utf8_lossy(&oc[first.saturating_sub(40)..oc.len().min(first + 80)]),
                String::from_utf8_lossy(&or[first.saturating_sub(40)..or.len().min(first + 80)]),
                &data[..data.len().min(16)],
            );
        }
        agreed.push(oc.clone());
    }
    agreed
}

/// Run one `driver` configuration against BOTH shared objects and assert their
/// stdout bytes are identical.
pub fn diff_driver(ctx: &str, data: &[i32], len: c_int) -> Vec<u8> {
    diff_driver_batch(ctx, &[(data.to_vec(), len)])
        .into_iter()
        .next()
        .unwrap()
}

// ---------------------------------------------------------------------------
// Value generators matching the CONFIGS.md value classes.
// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueClass {
    Zeros,
    SmallPos,
    SmallNeg,
    MixedSmall,
    Boundary,
    FullRandom,
    /// Operands large enough that every product overflows.
    OverflowMul,
    /// Product pinned near INT_MAX so that the following `+ add` overflows.
    OverflowAdd,
}

pub const ALL_VALUE_CLASSES: &[ValueClass] = &[
    ValueClass::Zeros,
    ValueClass::SmallPos,
    ValueClass::SmallNeg,
    ValueClass::MixedSmall,
    ValueClass::Boundary,
    ValueClass::FullRandom,
    ValueClass::OverflowMul,
    ValueClass::OverflowAdd,
];

pub fn gen_vals(rng: &mut Rng, class: ValueClass, n: usize) -> Vec<i32> {
    (0..n)
        .map(|_| match class {
            ValueClass::Zeros => 0,
            ValueClass::SmallPos => rng.range_i32(0, 100),
            ValueClass::SmallNeg => rng.range_i32(-100, 0),
            ValueClass::MixedSmall => rng.range_i32(-100, 100),
            ValueClass::Boundary => rng.pick(BOUNDARY),
            ValueClass::FullRandom => rng.i32_any(),
            ValueClass::OverflowMul => {
                let v = rng.range_i32(1 << 16, i32::MAX);
                if rng.next_u64() & 1 == 0 { v } else { -v }
            }
            ValueClass::OverflowAdd => {
                if rng.next_u64() & 1 == 0 {
                    i32::MAX
                } else {
                    rng.range_i32(i32::MAX - 4, i32::MAX)
                }
            }
        })
        .collect()
}

/// Reference rendering of what `driver` should print, computed from the
/// wrapping FMA. Used only as a cross-check that BOTH implementations agree
/// with the documented C semantics (never as a substitute for the C output).
pub fn expected_driver_stdout(data: &[i32], len: c_int) -> Vec<u8> {
    let n = if len > 0 { len as usize } else { 0 };
    let mut s = String::new();
    for &v in &data[..n] {
        let r = v.wrapping_mul(v).wrapping_add(v);
        s.push_str(&r.to_string());
        s.push('\n');
    }
    s.into_bytes()
}
