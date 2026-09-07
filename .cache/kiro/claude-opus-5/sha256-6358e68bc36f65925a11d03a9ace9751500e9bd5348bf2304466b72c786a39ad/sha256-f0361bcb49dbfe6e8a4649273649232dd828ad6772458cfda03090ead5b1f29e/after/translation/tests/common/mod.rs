//! Shared differential-testing harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `my_pow` only via
//! the dynamic-symbol table, exactly as an external C consumer would. The Rust
//! implementation is never called directly, so the `#[no_mangle] extern "C"`
//! wrapper is part of what is under test.
//!
//! Three observables are compared for every input pair:
//!   1. the returned `double`, compared **bit-for-bit** (`f64::to_bits`), so
//!      `+0.0` vs `-0.0` and differing NaN payloads are caught;
//!   2. the exact bytes written to `stderr` by the call;
//!   3. the value of `errno` left behind after the call.

#![allow(dead_code)]

use std::ffi::{c_int, c_void};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

pub type PowFn = unsafe extern "C" fn(f64, f64) -> f64;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn __errno_location() -> *mut c_int;
}

fn errno() -> c_int {
    unsafe { *__errno_location() }
}

fn set_errno(v: c_int) {
    unsafe { *__errno_location() = v }
}

/// glibc `EDOM` / `ERANGE`, as hard-coded by the translation.
pub const EDOM: c_int = 33;
pub const ERANGE: c_int = 34;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locates the C `.so` produced by `cmake --build`.
fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("POW_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libpow.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}; build it with\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Locates the Rust `cdylib`. Prefers the profile the test itself was built
/// with so `cargo test` and `cargo test --release` both work.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("POW_RUST_SO") {
        return PathBuf::from(p);
    }
    let target = manifest_dir().join("target");
    let preferred = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    for profile in preferred {
        let p = target.join(profile).join("libpow.so");
        if p.exists() {
            return p;
        }
    }
    panic!("Rust cdylib libpow.so not found under {target:?}; run `cargo build`");
}

fn load(path: &PathBuf) -> PowFn {
    // Leaked so the library stays mapped for the whole test binary's life.
    let lib: &'static Library = Box::leak(Box::new(unsafe {
        Library::new(path).unwrap_or_else(|e| panic!("dlopen({path:?}) failed: {e}"))
    }));
    let sym: Symbol<'static, PowFn> = unsafe {
        lib.get(b"my_pow\0")
            .unwrap_or_else(|e| panic!("dlsym(my_pow) in {path:?} failed: {e}"))
    };
    *sym
}

pub fn c_pow() -> PowFn {
    static F: OnceLock<PowFn> = OnceLock::new();
    *F.get_or_init(|| load(&c_so_path()))
}

pub fn rust_pow() -> PowFn {
    static F: OnceLock<PowFn> = OnceLock::new();
    *F.get_or_init(|| load(&rust_so_path()))
}

/// fd 2 is process-global, so captures must not overlap.
fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Acquires exclusive ownership of fd 2 for the lifetime of the returned guard.
///
/// Tests run on several threads but fd 2 is shared by the whole process, so
/// *every* redirection of it — this harness's per-call capture and the bulk
/// sweeps' `/dev/null` silencing alike — must go through this one lock or one
/// test's diagnostics land in another test's capture file.
pub fn lock_stderr() -> std::sync::MutexGuard<'static, ()> {
    capture_lock().lock().unwrap_or_else(|e| e.into_inner())
}

static SEQ: AtomicU64 = AtomicU64::new(0);

/// Runs `f` with fd 2 redirected to a temporary file and returns
/// `(value, errno_after, stderr_bytes)`.
pub fn call_capturing(f: PowFn, base: f64, exponent: f64, errno_in: c_int) -> (f64, c_int, Vec<u8>) {
    let _guard = lock_stderr();

    let id = SEQ.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("pow_diff_{}_{}.err", std::process::id(), id));
    let file = File::create(&tmp).expect("create temp stderr file");

    let (value, errno_out) = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(2);
        assert!(saved >= 0, "dup(2) failed");
        assert!(dup2(file.as_raw_fd(), 2) >= 0, "dup2 onto fd 2 failed");

        set_errno(errno_in);
        let v = f(base, exponent);
        let e = errno();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 2) >= 0, "restoring fd 2 failed");
        close(saved);
        (v, e)
    };

    drop(file);
    let bytes = std::fs::read(&tmp).expect("read captured stderr");
    let _ = std::fs::remove_file(&tmp);
    (value, errno_out, bytes)
}

/// Core differential assertion: C and Rust must agree on all three
/// observables. `errno_in` is the value `errno` holds on entry.
#[track_caller]
pub fn diff_with_errno(base: f64, exponent: f64, errno_in: c_int, ctx: &str) {
    let (cv, ce, cerr) = call_capturing(c_pow(), base, exponent, errno_in);
    let (rv, re, rerr) = call_capturing(rust_pow(), base, exponent, errno_in);

    assert_eq!(
        cv.to_bits(),
        rv.to_bits(),
        "[{ctx}] return value mismatch for my_pow({base:?} /*{:#018x}*/, {exponent:?} /*{:#018x}*/) \
         with errno_in={errno_in}: C={cv:?} ({:#018x}) vs Rust={rv:?} ({:#018x})",
        base.to_bits(),
        exponent.to_bits(),
        cv.to_bits(),
        rv.to_bits(),
    );

    assert_eq!(
        String::from_utf8_lossy(&cerr),
        String::from_utf8_lossy(&rerr),
        "[{ctx}] stderr mismatch for my_pow({base:?}, {exponent:?}) with errno_in={errno_in}"
    );
    assert_eq!(
        cerr, rerr,
        "[{ctx}] stderr bytes mismatch for my_pow({base:?}, {exponent:?})"
    );

    assert_eq!(
        ce, re,
        "[{ctx}] errno-after mismatch for my_pow({base:?}, {exponent:?}) \
         with errno_in={errno_in}: C={ce} vs Rust={re}"
    );
}

/// The common case: `errno` starts at a neutral non-zero sentinel so that the
/// `errno = 0` at the top of the C function is actually exercised.
#[track_caller]
pub fn diff(base: f64, exponent: f64, ctx: &str) {
    diff_with_errno(base, exponent, 0, ctx);
    // 4 == EINTR: a value that is neither EDOM nor ERANGE.
    diff_with_errno(base, exponent, 4, ctx);
}

/// Deterministic xorshift64* PRNG — fixed seed, reproducible runs.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
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
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }
    /// Uniform integer in `[lo, hi]`.
    pub fn int(&mut self, lo: i64, hi: i64) -> i64 {
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
    /// A raw 64-bit pattern reinterpreted as `f64` (any float class).
    pub fn raw_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
}

/// Pool of interesting `double` values, used for exhaustive cross products.
pub const SPECIALS: &[f64] = &[
    0.0,
    -0.0,
    1.0,
    -1.0,
    0.5,
    -0.5,
    2.0,
    -2.0,
    3.0,
    -3.0,
    f64::INFINITY,
    f64::NEG_INFINITY,
    f64::NAN,
    -f64::NAN,
    f64::MAX,
    f64::MIN,
    f64::MIN_POSITIVE,
    -f64::MIN_POSITIVE,
    5e-324,
    -5e-324,
    9007199254740992.0,  // 2^53
    9007199254740993.0,  // 2^53 + 1 (rounds to 2^53)
    1e300,
    1e-300,
];
