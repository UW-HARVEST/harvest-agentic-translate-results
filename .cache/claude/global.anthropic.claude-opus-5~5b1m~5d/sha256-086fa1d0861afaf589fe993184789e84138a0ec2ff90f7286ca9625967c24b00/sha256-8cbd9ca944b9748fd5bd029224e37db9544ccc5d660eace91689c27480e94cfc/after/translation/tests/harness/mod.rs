//! Shared differential-test harness.
//!
//! Both the C `libdriver.so` and the Rust `libdriver.so` are loaded with
//! `libloading` and driven *only* through their exported symbols (`driver`,
//! `run`), exactly as an external C consumer would. Nothing in the Rust crate is
//! called directly, so the `#[no_mangle] extern "C"` wrappers are under test too.
//!
//! Two facts about the library shape the harness:
//!
//! 1. All output goes to `stdout` via `printf(3)`, so we capture fd 1 by
//!    `dup2`-ing it onto a temporary file around each call sequence.
//!
//! 2. The library keeps *file-scope mutable state* (`static house_t the_house`)
//!    which is never reset, so output depends on the entire call history. Rather
//!    than trying to obtain a pristine copy per call (re-`dlopen`ing a fresh file
//!    copy is unreliable: after `dlclose` glibc may still have the object mapped
//!    and will alias a *reused inode* back to the stale one), each library is
//!    loaded exactly **once per test process** and every operation sequence is
//!    replayed against **both** libraries while a single global lock is held.
//!    That keeps the two instances in perfect lockstep: they observe the same
//!    ops in the same global order, so their hidden state must stay identical.
//!    A divergence in the accumulated state therefore shows up as a byte
//!    difference — which is exactly what we want to detect.
//!
//! Assertions about absolute values (e.g. "bedrooms starts at 5") must not
//! depend on the accumulated state; use `first_last_bedrooms` /
//! `bedrooms_delta`, or put the test in its own test file so it gets a pristine
//! process (see `tests/phase_b_initial_state.rs`).

#![allow(dead_code)]

use std::ffi::{c_char, c_int, CString};
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../c_src/build/libdriver.so");

/// fd 1 and the libraries' hidden state are process-wide, so only one sequence
/// may be in flight at a time even though `cargo test` uses many threads.
fn big_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    match L.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// The Rust `cdylib` under test, located next to the test binary.
pub fn rust_lib() -> PathBuf {
    let mut p = std::env::current_exe().expect("current_exe");
    p.pop(); // deps/
    if p.file_name().map(|f| f == "deps").unwrap_or(false) {
        p.pop();
    }
    let candidate = p.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    if let Some(target) = p.parent() {
        for prof in ["debug", "release"] {
            let alt = target.join(prof).join("libdriver.so");
            if alt.exists() {
                return alt;
            }
        }
    }
    panic!("Rust cdylib not found at {candidate:?}; run `cargo build` before `cargo test`");
}

pub fn c_lib() -> PathBuf {
    let p = PathBuf::from(C_LIB);
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with cmake first"
    );
    p
}

/// A single call into the library.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Op {
    /// `driver(in)` with these bytes, NUL appended by the harness.
    Driver(Vec<u8>),
    /// `driver(in)` with a raw byte buffer used verbatim (must contain a NUL).
    DriverRaw(Vec<u8>),
    /// `run(extra_bedrooms)`
    Run(c_int),
    /// Set `errno` to this value immediately before the next op.
    SetErrno(c_int),
}

pub fn drv(s: &str) -> Op {
    Op::Driver(s.as_bytes().to_vec())
}

pub fn run_op(v: c_int) -> Op {
    Op::Run(v)
}

extern "C" {
    fn __errno_location() -> *mut c_int;
}

type DriverFn = unsafe extern "C" fn(*const c_char);
type RunFn = unsafe extern "C" fn(c_int);

struct Lib {
    _lib: libloading::Library,
    driver: DriverFn,
    run: RunFn,
}

// SAFETY: the raw fn pointers are only ever invoked while `big_lock()` is held.
unsafe impl Send for Lib {}
unsafe impl Sync for Lib {}

fn load(path: &std::path::Path) -> Lib {
    unsafe {
        let lib = libloading::Library::new(path).unwrap_or_else(|e| panic!("dlopen {path:?}: {e}"));
        let driver: libloading::Symbol<DriverFn> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("symbol `driver` missing from {path:?}: {e}"));
        let run: libloading::Symbol<RunFn> = lib
            .get(b"run\0")
            .unwrap_or_else(|e| panic!("symbol `run` missing from {path:?}: {e}"));
        let (d, r) = (*driver, *run);
        Lib {
            _lib: lib,
            driver: d,
            run: r,
        }
    }
}

fn libs() -> &'static (Lib, Lib) {
    static LIBS: OnceLock<(Lib, Lib)> = OnceLock::new();
    LIBS.get_or_init(|| (load(&c_lib()), load(&rust_lib())))
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn tmp_dir() -> PathBuf {
    std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// Capture everything written to fd 1 (stdout) by `f`. Caller holds `big_lock`.
///
/// Two writers must be kept out of the capture file while fd 1 is redirected:
///   * libc's own stdio buffer -> `fflush(NULL)` before redirecting;
///   * libtest's progress output (`test foo ... ok`), which the *main* thread
///     writes through `std::io::stdout()` while our test thread is running ->
///     we hold the `StdoutLock` for the whole capture so those writes block.
/// Without the second guard, `cargo test` with its default multi-threaded
/// runner intermittently mixes `"test cfg_... ... "` into the captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Write;
    let mut rust_stdout = std::io::stdout().lock();
    let _ = rust_stdout.flush();
    unsafe {
        // Flush whatever the C stdio layer buffered, so it does not land in our
        // capture file.
        libc::fflush(std::ptr::null_mut());

        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let path = tmp_dir().join(format!("difftest_out_{}_{}.txt", std::process::id(), n));
        let cpath = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();

        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = libc::open(
            cpath.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600 as libc::c_int,
        );
        assert!(fd >= 0, "open({path:?}) failed");
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");

        f();

        // fd 1 is a file now, so printf output is block buffered: flush before
        // restoring.
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);
        libc::close(fd);

        let mut buf = Vec::new();
        std::fs::File::open(&path)
            .expect("reopen capture file")
            .read_to_end(&mut buf)
            .expect("read capture file");
        let _ = std::fs::remove_file(&path);
        drop(rust_stdout);
        // Guard against any foreign writer having slipped in anyway: every line
        // must be either a house report or the error sentinel.
        for line in String::from_utf8_lossy(&buf).lines() {
            assert!(
                line == "An error occurred" || parse_line(line).is_some(),
                "captured stdout was contaminated by a foreign writer: {line:?}"
            );
        }
        buf
    }
}

/// Replay `ops` against one already-loaded library. Caller holds `big_lock`.
fn exec_locked(lib: &Lib, ops: &[Op]) -> Vec<u8> {
    capture_stdout(|| unsafe {
        let mut pending_errno: Option<c_int> = None;
        for op in ops {
            if let Op::SetErrno(v) = op {
                pending_errno = Some(*v);
                continue;
            }
            if let Some(v) = pending_errno.take() {
                *__errno_location() = v;
            }
            match op {
                Op::Driver(bytes) => {
                    let mut b = bytes.clone();
                    b.push(0);
                    (lib.driver)(b.as_ptr() as *const c_char);
                }
                Op::DriverRaw(bytes) => (lib.driver)(bytes.as_ptr() as *const c_char),
                Op::Run(v) => (lib.run)(*v),
                Op::SetErrno(_) => unreachable!(),
            }
        }
    })
}

/// Run `ops` against both libraries and assert byte-identical stdout.
///
/// The C sequence and the Rust sequence execute under a single lock acquisition,
/// so both libraries always see the same ops in the same order and their hidden
/// `the_house` state stays in lockstep.
static COMPARISONS: AtomicU64 = AtomicU64::new(0);
static OPS_EXECUTED: AtomicU64 = AtomicU64::new(0);

/// Number of C-vs-Rust byte comparisons performed so far in this process.
pub fn comparison_count() -> u64 {
    COMPARISONS.load(Ordering::SeqCst)
}

/// Number of library calls issued so far (per library).
pub fn ops_executed() -> u64 {
    OPS_EXECUTED.load(Ordering::SeqCst)
}

#[track_caller]
pub fn assert_same(label: &str, ops: &[Op]) -> Vec<u8> {
    let _g = big_lock();
    COMPARISONS.fetch_add(1, Ordering::SeqCst);
    OPS_EXECUTED.fetch_add(
        ops.iter().filter(|o| !matches!(o, Op::SetErrno(_))).count() as u64,
        Ordering::SeqCst,
    );
    let (cl, rl) = libs();
    let c = exec_locked(cl, ops);
    let r = exec_locked(rl, ops);
    if c != r {
        panic!(
            "DIVERGENCE [{label}]\n  ops:  {ops:?}\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r)
        );
    }
    assert!(!c.is_empty(), "[{label}] no output at all from either library");
    c
}

#[track_caller]
pub fn assert_same_one(label: &str, op: Op) -> Vec<u8> {
    assert_same(label, std::slice::from_ref(&op))
}

// ---------------------------------------------------------------------------
// Helpers for state-independent semantic assertions.
// ---------------------------------------------------------------------------

/// Parse `"The house has %d floors, %d bedrooms, and %.1f bathrooms"`.
pub fn parse_line(line: &str) -> Option<(i64, i64, String)> {
    let rest = line.strip_prefix("The house has ")?;
    let (floors, rest) = rest.split_once(" floors, ")?;
    let (bedrooms, rest) = rest.split_once(" bedrooms, and ")?;
    let bath = rest.strip_suffix(" bathrooms")?;
    Some((
        floors.parse().ok()?,
        bedrooms.parse().ok()?,
        bath.to_string(),
    ))
}

/// `bedrooms` on the first and last report line of `out`.
pub fn first_last_bedrooms(out: &[u8]) -> (i64, i64) {
    let text = String::from_utf8_lossy(out);
    let reports: Vec<(i64, i64, String)> = text.lines().filter_map(parse_line).collect();
    assert!(!reports.is_empty(), "no report lines in {text:?}");
    (reports[0].1, reports[reports.len() - 1].1)
}

/// How much `bedrooms` moved over the captured sequence, as wrapping i32 math.
pub fn bedrooms_delta(out: &[u8]) -> i32 {
    let (first, last) = first_last_bedrooms(out);
    (last as i32).wrapping_sub(first as i32)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), so every "randomized" row is reproducible.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }
}
