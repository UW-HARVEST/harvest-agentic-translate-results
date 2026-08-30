//! Shared differential-test harness.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and driven
//! purely through their exported `my_pow` symbol — the Rust implementation is
//! never called directly, so the `#[no_mangle] extern "C"` wrapper is under
//! test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type MyPowFn = unsafe extern "C" fn(f64, f64) -> f64;

extern "C" {
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut c_void) -> i32;
    fn __errno_location() -> *mut i32;
}

/// `errno` values on Linux, matching `<errno.h>`.
pub const EDOM: i32 = 33;
pub const ERANGE: i32 = 34;

/// Serializes ALL manipulation of the process-global fd 2.
///
/// `my_pow` reports errors by writing to `stderr`, so both silencing it and
/// capturing it mean `dup2`-ing fd 2 — a process-wide side effect. libtest runs
/// the tests in a binary on parallel threads, so without full mutual exclusion
/// the redirects interleave and one test's diagnostics land in another test's
/// capture buffer (and a restoring guard can hand fd 2 back to the wrong
/// target). A single mutex, held for the whole duration of every redirect, is
/// the only sound arrangement here.
///
/// Invariant: never acquire this twice in the same scope — the guards below are
/// not reentrant.
static STDERR_MUTEX: Mutex<()> = Mutex::new(());

pub struct Libs {
    _c_lib: Library,
    _r_lib: Library,
    pub c_pow: MyPowFn,
    pub r_pow: MyPowFn,
    pub c_path: PathBuf,
    pub r_path: PathBuf,
}

// The raw fn pointers are plain code addresses in libraries we keep alive for
// the whole process; sharing them across test threads is sound.
unsafe impl Sync for Libs {}
unsafe impl Send for Libs {}

fn c_so_path() -> PathBuf {
    // <crate>/../c_src/build/libpow.so
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.push("c_src");
    p.push("build");
    p.push("libpow.so");
    p
}

fn rust_so_path() -> PathBuf {
    // The test executable lives in target/<profile>/deps/, so the cdylib built
    // alongside it is at target/<profile>/libpow.so. Deriving it this way makes
    // the tests work for both `cargo test` and `cargo test --release`.
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir = exe.parent().expect("deps dir").to_path_buf();
    if dir.file_name().map(|n| n == "deps").unwrap_or(false) {
        dir.pop();
    }
    dir.join("libpow.so")
}

/// Newest mtime among the files that determine the contents of a `.so`.
fn newest_mtime(paths: &[PathBuf]) -> Option<(std::time::SystemTime, PathBuf)> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for p in paths {
        let Ok(md) = std::fs::metadata(p) else { continue };
        let Ok(t) = md.modified() else { continue };
        if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
            best = Some((t, p.clone()));
        }
    }
    best
}

/// Guard against testing a STALE artifact.
///
/// `cargo test` does **not** rebuild a `crate-type = ["cdylib"]` target: the
/// integration tests here never link the crate (they `dlopen` it), so nothing in
/// the test graph depends on the `.so` and cargo happily leaves an old one in
/// place. Without this check the whole suite silently validates a previous
/// build — every mutation to `src/lib.rs` would "pass". Always build first:
///
/// ```sh
/// cargo build --release && cargo test --release
/// ```
fn assert_fresh(so: &std::path::Path, sources: &[PathBuf], what: &str, how: &str) {
    let Ok(so_time) = std::fs::metadata(so).and_then(|m| m.modified()) else {
        return;
    };
    if let Some((src_time, src)) = newest_mtime(sources) {
        assert!(
            so_time >= src_time,
            "STALE {what}: {so:?} is older than its source {src:?}.\n\
             The tests would be validating a previous build and would pass \
             vacuously.\nRebuild first with:\n  {how}"
        );
    }
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let r_path = rust_so_path();
        assert!(
            c_path.is_file(),
            "C shared library not found at {c_path:?}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        assert!(
            r_path.is_file(),
            "Rust shared library not found at {r_path:?}. Build it with \
             `cargo build` / `cargo build --release`."
        );

        // Refuse to run against a stale build (see `assert_fresh`).
        let crate_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut rust_srcs = vec![crate_root.join("Cargo.toml")];
        if let Ok(rd) = std::fs::read_dir(crate_root.join("src")) {
            rust_srcs.extend(
                rd.filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.extension().map(|x| x == "rs").unwrap_or(false)),
            );
        }
        assert_fresh(
            &r_path,
            &rust_srcs,
            "Rust .so",
            "cargo build --release && cargo test --release   (or ./run_tests.sh)",
        );

        let c_root = {
            let mut p = crate_root.clone();
            p.pop();
            p.push("c_src");
            p
        };
        assert_fresh(
            &c_path,
            &[c_root.join("src/pow.c"), c_root.join("include/pow.h")],
            "C .so",
            "cd c_src/build && cmake --build .",
        );

        // Default flags are RTLD_LAZY | RTLD_LOCAL, so the two `my_pow`
        // definitions do not interpose on each other and each `get` resolves
        // within its own handle.
        let c_lib = unsafe { Library::new(&c_path) }.expect("dlopen C libpow.so");
        let r_lib = unsafe { Library::new(&r_path) }.expect("dlopen Rust libpow.so");

        let c_pow = unsafe {
            let s: Symbol<MyPowFn> = c_lib.get(b"my_pow\0").expect("C my_pow symbol");
            *s
        };
        let r_pow = unsafe {
            let s: Symbol<MyPowFn> = r_lib.get(b"my_pow\0").expect("Rust my_pow symbol");
            *s
        };

        Libs {
            _c_lib: c_lib,
            _r_lib: r_lib,
            c_pow,
            r_pow,
            c_path,
            r_path,
        }
    })
}

// ---------------------------------------------------------------------------
// fd-2 redirection
// ---------------------------------------------------------------------------

/// Holds `STDERR_MUTEX` and restores fd 2 when dropped. Because the guard owns
/// the mutex, fd 2 can only ever be redirected by one thread at a time.
pub struct FdGuard {
    saved: i32,
    _lock: MutexGuard<'static, ()>,
}

impl Drop for FdGuard {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved, 2);
            close(self.saved);
        }
    }
}

/// Point fd 2 at `path`, holding `STDERR_MUTEX`, until the guard is dropped.
fn redirect_stderr_to(path: &std::path::Path) -> FdGuard {
    let lock = STDERR_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)
        .expect("open redirect target");
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(2);
        assert!(saved >= 0, "dup(2) failed");
        assert!(dup2(file.as_raw_fd(), 2) >= 0, "dup2 failed");
        FdGuard { saved, _lock: lock }
    }
}

/// Silence the library diagnostics for the duration of the guard. `my_pow`
/// writes multi-hundred-digit `%.2f` expansions to stderr on every error path,
/// which would otherwise bury the test output.
///
/// IMPORTANT: never panic while this guard is alive — the panic message would go
/// to `/dev/null`. Collect results, drop the guard, then assert.
pub fn quiet() -> FdGuard {
    redirect_stderr_to(std::path::Path::new("/dev/null"))
}

static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Run `f` with fd 2 redirected to a private temp file and return everything
/// written. Holds `STDERR_MUTEX` for the whole capture, so no other test can
/// contribute bytes.
pub fn capture_stderr<F: FnOnce()>(f: F) -> Vec<u8> {
    let n = CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "pow_diff_stderr_{}_{}.txt",
        std::process::id(),
        n
    ));
    {
        let _g = redirect_stderr_to(&path);
        f();
        unsafe { fflush(std::ptr::null_mut()) };
    }
    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// errno helpers
// ---------------------------------------------------------------------------

pub fn errno_get() -> i32 {
    unsafe { *__errno_location() }
}

pub fn errno_set(v: i32) {
    unsafe { *__errno_location() = v }
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

pub struct Mismatch {
    pub base: f64,
    pub exponent: f64,
    pub c: f64,
    pub r: f64,
}

impl std::fmt::Display for Mismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "my_pow(base={:.17e} [{:#018x}], exponent={:.17e} [{:#018x}]) \
             -> C={:.17e} [{:#018x}]  RUST={:.17e} [{:#018x}]",
            self.base,
            self.base.to_bits(),
            self.exponent,
            self.exponent.to_bits(),
            self.c,
            self.c.to_bits(),
            self.r,
            self.r.to_bits()
        )
    }
}

/// Call both `.so`s for every pair and collect bit-level divergences.
pub fn diff_pairs(pairs: &[(f64, f64)]) -> Vec<Mismatch> {
    let l = libs();
    let mut out = Vec::new();
    {
        // `quiet()` also takes STDERR_MUTEX for the whole batch.
        let _q = quiet();
        for &(base, exponent) in pairs {
            // The C body starts with `errno = 0`, so a stale errno must not
            // affect either implementation. Seed a hostile value to prove it.
            errno_set(ERANGE);
            let c = unsafe { (l.c_pow)(base, exponent) };
            errno_set(EDOM);
            let r = unsafe { (l.r_pow)(base, exponent) };
            if c.to_bits() != r.to_bits() && out.len() < 25 {
                out.push(Mismatch {
                    base,
                    exponent,
                    c,
                    r,
                });
            }
        }
    }
    out
}

/// Assert the C and Rust `.so` agree bit-for-bit on every pair.
#[track_caller]
pub fn check_pairs(ctx: &str, pairs: &[(f64, f64)]) {
    let bad = diff_pairs(pairs);
    if !bad.is_empty() {
        let mut msg = format!(
            "{ctx}: {} of {} input pair(s) diverged between C and Rust:\n",
            bad.len(),
            pairs.len()
        );
        for m in &bad {
            msg.push_str("  ");
            msg.push_str(&m.to_string());
            msg.push('\n');
        }
        panic!("{msg}");
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed for reproducibility
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

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }

    /// Uniform integer in [lo, hi] inclusive.
    pub fn int_range(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// An arbitrary `f64` drawn from the whole 2^64 bit space (covers NaNs,
    /// infinities, subnormals and every magnitude).
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }

    /// Log-uniform magnitude in [10^lo_exp, 10^hi_exp), random sign if `signed`.
    pub fn log_uniform(&mut self, lo_exp: f64, hi_exp: f64, signed: bool) -> f64 {
        let e = self.range(lo_exp, hi_exp);
        let m = 10f64.powf(e);
        if signed && self.bool() {
            -m
        } else {
            m
        }
    }
}

/// Base seed; every row derives a distinct stream from it.
pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------------------
// Interesting-value corpus
// ---------------------------------------------------------------------------

/// Every special / boundary `double` the libm `pow` paths distinguish.
pub const SPECIALS: &[f64] = &[
    f64::NAN,
    -f64::NAN,
    f64::INFINITY,
    f64::NEG_INFINITY,
    0.0,
    -0.0,
    1.0,
    -1.0,
    2.0,
    -2.0,
    3.0,
    -3.0,
    0.5,
    -0.5,
    1.5,
    -1.5,
    2.5,
    -2.5,
    0.1,
    -0.1,
    10.0,
    -10.0,
    f64::MAX,
    f64::MIN,
    f64::MIN_POSITIVE,
    -f64::MIN_POSITIVE,
    5e-324,  // smallest positive subnormal
    -5e-324,
    f64::EPSILON,
    -f64::EPSILON,
    1e300,
    -1e300,
    1e-300,
    -1e-300,
    1024.0,
    -1024.0,
    1023.0,
    -1074.0,
    10000.0,
    -10000.0,
    4503599627370496.0,  // 2^52
    9007199254740992.0,  // 2^53
    9007199254740993.0,  // 2^53 + 1 (not representable -> rounds)
];

/// NaN bit patterns, including signalling NaNs, to check payload propagation.
pub const NAN_BITS: &[u64] = &[
    0x7FF8_0000_0000_0000, // canonical quiet NaN
    0xFFF8_0000_0000_0000, // negative quiet NaN
    0x7FF8_0000_0000_0001, // quiet NaN, payload 1
    0x7FFF_FFFF_FFFF_FFFF, // quiet NaN, all payload bits set
    0x7FF0_0000_0000_0001, // signalling NaN, payload 1
    0xFFF0_0000_0000_0001, // negative signalling NaN
    0x7FF4_0000_0000_0000, // signalling NaN
    0xFFF7_FFFF_FFFF_FFFF, // negative signalling NaN, max payload
];
