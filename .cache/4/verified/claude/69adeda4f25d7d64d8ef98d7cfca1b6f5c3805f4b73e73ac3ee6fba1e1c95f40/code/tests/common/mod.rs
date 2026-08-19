//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called only through their exported `my_pow` symbol — the Rust crate is never
//! called directly, so the `#[no_mangle] extern "C"` wrapper is under test too.
//!
//! `my_pow`'s behaviour is only fully observable from a *triple*:
//!   1. the returned `f64` **bit pattern** (`-1.0` is both an error sentinel and
//!      a legal result; `-0.0`/`+0.0` and NaN payloads must not be conflated),
//!   2. the exact **bytes written to stderr** by `fprintf("%.2f")`,
//!   3. the **residual `errno`** left in the caller's TLS slot.
//! Every comparison in this suite checks all three.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::fs::File;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

pub type PowFn = unsafe extern "C" fn(f64, f64) -> f64;

pub const EDOM: c_int = 33;
pub const ERANGE: c_int = 34;
pub const EINVAL: c_int = 22;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    fn atexit(f: extern "C" fn()) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn __errno_location() -> *mut c_int;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

pub struct Impls {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: PowFn,
    pub rust: PowFn,
}

// `Library` is Send+Sync; bare `extern "C" fn` pointers are Send+Sync.
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The Rust `cdylib` sits next to the test executable's parent dir:
/// `target/<profile>/deps/<test-bin>` -> `target/<profile>/libpow.so`.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let candidates = [
        deps.parent().map(|p| p.join("libpow.so")),
        Some(deps.join("libpow.so")),
    ];
    for c in candidates.into_iter().flatten() {
        if c.exists() {
            return c;
        }
    }
    panic!(
        "could not locate the Rust libpow.so next to {}; run `cargo build` first",
        exe.display()
    );
}

fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("c_src/build/libpow.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

static IMPLS: OnceLock<Impls> = OnceLock::new();

pub fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| unsafe {
        atexit(report_comparisons);
        let c_path = c_so_path();
        let rust_path = rust_so_path();

        let c_lib = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust_lib = Library::new(&rust_path)
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));

        let c_sym: Symbol<PowFn> = c_lib
            .get(b"my_pow\0")
            .unwrap_or_else(|e| panic!("C .so does not export my_pow: {e}"));
        let rust_sym: Symbol<PowFn> = rust_lib
            .get(b"my_pow\0")
            .unwrap_or_else(|e| panic!("Rust .so does not export my_pow: {e}"));

        let c = *c_sym;
        let rust = *rust_sym;

        Impls {
            _c_lib: c_lib,
            _rust_lib: rust_lib,
            c,
            rust,
        }
    })
}

// ---------------------------------------------------------------------------
// Comparison counter (reported at process exit, so the number of differential
// comparisons actually executed is observable rather than estimated)
// ---------------------------------------------------------------------------

static COMPARISONS: AtomicU64 = AtomicU64::new(0);

/// Record a comparison performed by a test that does its own assertion
/// (e.g. the concurrency test, which cannot use the fd-capturing path).
pub fn note_comparison() {
    COMPARISONS.fetch_add(1, Ordering::Relaxed);
}

extern "C" fn report_comparisons() {
    let n = COMPARISONS.load(Ordering::Relaxed);
    let msg = format!("[harness] differential comparisons in this binary: {n}\n");
    // Raw write to fd 1: this runs after libtest's teardown, when the normal
    // print machinery may no longer be usable.
    unsafe {
        write(1, msg.as_ptr() as *const c_void, msg.len());
    }
}

// ---------------------------------------------------------------------------
// Observable outcome of one call
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub struct Outcome {
    pub bits: u64,
    pub errno: c_int,
    pub stderr: Vec<u8>,
}

impl std::fmt::Debug for Outcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Outcome {{ ret = {:?} (bits 0x{:016X}), errno = {}, stderr = {:?} }}",
            f64::from_bits(self.bits),
            self.bits,
            self.errno,
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

/// fd-2 redirection and the `errno` TLS slot are process-global, so every
/// capture pair must be serialized.
static LOCK: Mutex<()> = Mutex::new(());

fn lock() -> std::sync::MutexGuard<'static, ()> {
    // Keep running after a failed assertion in another test.
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn capture_path() -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("pow_difftest_stderr_{}.txt", std::process::id()));
    p
}

/// How stderr should behave during the call.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum StderrMode {
    /// fd 2 -> temp file, contents returned.
    Capture,
    /// fd 2 -> `/dev/full`, so every `write` fails with `ENOSPC`.
    Full,
    /// fd 2 closed, so every `write` fails with `EBADF`.
    Closed,
}

/// Call one implementation with fd 2 redirected, returning the full observable
/// triple. `preset_errno` is written to the caller's `errno` immediately before
/// the call, to prove `my_pow`'s leading `errno = 0` discards caller state.
///
/// PRIVATE and lock-free: fd 2, the `errno` TLS slot and the capture file are all
/// process-global, so callers MUST already hold `LOCK`. Use the public
/// `call_once` (which locks) or one of the `diff_*` helpers instead.
fn call_once_inner(
    f: PowFn,
    base: f64,
    exponent: f64,
    preset_errno: c_int,
    mode: StderrMode,
) -> Outcome {
    unsafe {
        // Flush anything the harness itself buffered before stealing fd 2.
        fflush(std::ptr::null_mut());

        let path = capture_path();
        // Held open for the duration so the fd stays valid.
        let redirect: Option<File> = match mode {
            StderrMode::Capture => Some(File::create(&path).expect("create capture file")),
            StderrMode::Full => Some(
                File::options()
                    .write(true)
                    .open("/dev/full")
                    .expect("open /dev/full"),
            ),
            StderrMode::Closed => None,
        };

        let saved = dup(2);
        assert!(saved >= 0, "dup(2) failed");

        match &redirect {
            Some(file) => {
                assert!(dup2(file.as_raw_fd(), 2) >= 0, "dup2 onto fd 2 failed");
            }
            None => {
                // No libc call between here and the call under test may open a
                // file descriptor, or it could be handed fd 2.
                close(2);
            }
        }

        *__errno_location() = preset_errno;
        let ret = f(base, exponent);
        // Read errno before any other libc call can clobber it.
        let errno = *__errno_location();

        if mode != StderrMode::Closed {
            fflush(std::ptr::null_mut());
        }
        assert!(dup2(saved, 2) >= 0, "restoring fd 2 failed");
        close(saved);
        drop(redirect);

        let stderr = match mode {
            StderrMode::Capture => std::fs::read(&path).expect("read capture file"),
            _ => Vec::new(),
        };

        Outcome {
            bits: ret.to_bits(),
            errno,
            stderr,
        }
    }
}

/// Public single-call entry point: acquires the global capture lock, so it is
/// safe to use from tests that libtest runs in parallel threads.
pub fn call_once(
    f: PowFn,
    base: f64,
    exponent: f64,
    preset_errno: c_int,
    mode: StderrMode,
) -> Outcome {
    let _g = lock();
    call_once_inner(f, base, exponent, preset_errno, mode)
}

/// Call with NO fd redirection and NO locking: returns just (return bits, errno).
///
/// Used by the concurrency test, where fd 2 is redirected once up front and many
/// threads then call in parallel. `errno` is thread-local, so a translation that
/// cached or globalised it instead of using `__errno_location()` per call would
/// produce cross-thread interference that this exposes.
pub fn call_raw(f: PowFn, base: f64, exponent: f64) -> (u64, c_int) {
    unsafe {
        *__errno_location() = 0;
        let ret = f(base, exponent);
        let errno = *__errno_location();
        (ret.to_bits(), errno)
    }
}

/// Point fd 2 at /dev/null for the rest of the process, so error messages from
/// the concurrency test do not spam the terminal. Returns the saved fd.
pub fn silence_stderr_forever() {
    unsafe {
        fflush(std::ptr::null_mut());
        let devnull = File::options()
            .write(true)
            .open("/dev/null")
            .expect("open /dev/null");
        assert!(dup2(devnull.as_raw_fd(), 2) >= 0, "dup2 /dev/null -> fd 2");
    }
}

// ---------------------------------------------------------------------------
// Differential assertions
// ---------------------------------------------------------------------------

fn describe(v: f64) -> String {
    format!("{:?} (0x{:016X})", v, v.to_bits())
}

fn assert_same(base: f64, exponent: f64, ctx: &str, c: &Outcome, r: &Outcome) {
    COMPARISONS.fetch_add(1, Ordering::Relaxed);
    if c == r {
        return;
    }
    panic!(
        "C/Rust divergence [{ctx}]\n  base     = {}\n  exponent = {}\n\
         \n  C    : {:?}\n  Rust : {:?}\n\
         \n  ret bits  : C 0x{:016X} vs Rust 0x{:016X}{}\n\
         errno     : C {} vs Rust {}{}\n  stderr    : C {:?}\n              Rust {:?}{}\n",
        describe(base),
        describe(exponent),
        c,
        r,
        c.bits,
        r.bits,
        if c.bits == r.bits { " (same)" } else { "  <== MISMATCH" },
        c.errno,
        r.errno,
        if c.errno == r.errno {
            " (same)"
        } else {
            "  <== MISMATCH"
        },
        String::from_utf8_lossy(&c.stderr),
        String::from_utf8_lossy(&r.stderr),
        if c.stderr == r.stderr {
            " (same)"
        } else {
            "  <== MISMATCH"
        },
    );
}

/// The workhorse: run both implementations on one input pair and require the
/// full observable triple to be identical.
pub fn diff(base: f64, exponent: f64) {
    diff_ctx(base, exponent, "");
}

pub fn diff_ctx(base: f64, exponent: f64, ctx: &str) {
    let im = impls();
    let _g = lock();
    let c = call_once_inner(im.c, base, exponent, 0, StderrMode::Capture);
    let r = call_once_inner(im.rust, base, exponent, 0, StderrMode::Capture);
    assert_same(base, exponent, ctx, &c, &r);
}

/// As `diff`, but seeds the caller's `errno` first (ERRORS.md E16–E18).
pub fn diff_preset_errno(base: f64, exponent: f64, preset: c_int, ctx: &str) {
    let im = impls();
    let _g = lock();
    let c = call_once_inner(im.c, base, exponent, preset, StderrMode::Capture);
    let r = call_once_inner(im.rust, base, exponent, preset, StderrMode::Capture);
    assert_same(base, exponent, ctx, &c, &r);
}

/// As `diff`, but with stderr made unwritable (ERRORS.md E21).
pub fn diff_stderr_mode(base: f64, exponent: f64, mode: StderrMode, ctx: &str) {
    let im = impls();
    let _g = lock();
    let c = call_once_inner(im.c, base, exponent, 0, mode);
    let r = call_once_inner(im.rust, base, exponent, 0, mode);
    assert_same(base, exponent, ctx, &c, &r);
}

/// Rust first, then C — proves the comparison is not order-dependent even though
/// both `.so`s share one `errno` TLS slot and one `stderr` `FILE*` (C42).
pub fn diff_reversed(base: f64, exponent: f64, ctx: &str) {
    let im = impls();
    let _g = lock();
    let r = call_once_inner(im.rust, base, exponent, 0, StderrMode::Capture);
    let c = call_once_inner(im.c, base, exponent, 0, StderrMode::Capture);
    assert_same(base, exponent, ctx, &c, &r);
}

/// Return the C implementation's outcome, for tests that additionally want to
/// assert *which* branch the C actually took (so a row cannot silently pass by
/// both sides doing nothing).
pub fn c_outcome(base: f64, exponent: f64) -> Outcome {
    let im = impls();
    let _g = lock();
    call_once_inner(im.c, base, exponent, 0, StderrMode::Capture)
}

/// Assert the C took the EDOM branch, and that Rust agrees.
pub fn diff_expect_domain_error(base: f64, exponent: f64, ctx: &str) {
    diff_ctx(base, exponent, ctx);
    let o = c_outcome(base, exponent);
    assert_eq!(
        o.bits,
        (-1.0f64).to_bits(),
        "[{ctx}] expected the C EDOM branch (ret -1.0) for base={}, exp={}, got {:?}",
        describe(base),
        describe(exponent),
        o
    );
    assert_eq!(o.errno, EDOM, "[{ctx}] expected residual errno == EDOM: {o:?}");
    let s = String::from_utf8_lossy(&o.stderr);
    assert!(
        s.starts_with("Domain error: pow("),
        "[{ctx}] expected the domain-error message, got {s:?}"
    );
}

/// Assert the C took the ERANGE branch, and that Rust agrees.
pub fn diff_expect_range_error(base: f64, exponent: f64, ctx: &str) {
    diff_ctx(base, exponent, ctx);
    let o = c_outcome(base, exponent);
    assert_eq!(
        o.bits,
        (-1.0f64).to_bits(),
        "[{ctx}] expected the C ERANGE branch (ret -1.0) for base={}, exp={}, got {:?}",
        describe(base),
        describe(exponent),
        o
    );
    assert_eq!(
        o.errno, ERANGE,
        "[{ctx}] expected residual errno == ERANGE: {o:?}"
    );
    let s = String::from_utf8_lossy(&o.stderr);
    assert!(
        s.starts_with("Range error: pow("),
        "[{ctx}] expected the range-error message, got {s:?}"
    );
}

/// Assert the C took NEITHER error branch (clean pass-through), and Rust agrees.
pub fn diff_expect_clean(base: f64, exponent: f64, ctx: &str) {
    diff_ctx(base, exponent, ctx);
    let o = c_outcome(base, exponent);
    assert!(
        o.stderr.is_empty(),
        "[{ctx}] expected no stderr for base={}, exp={}, got {:?}",
        describe(base),
        describe(exponent),
        String::from_utf8_lossy(&o.stderr)
    );
    assert_ne!(
        o.errno, EDOM,
        "[{ctx}] expected no EDOM for base={}, exp={}",
        describe(base),
        describe(exponent)
    );
    assert_ne!(
        o.errno, ERANGE,
        "[{ctx}] expected no ERANGE for base={}, exp={}",
        describe(base),
        describe(exponent)
    );
}

/// Assert the C returned exactly `expected` bits with no stderr, and Rust agrees.
pub fn diff_expect_bits(base: f64, exponent: f64, expected: f64, ctx: &str) {
    diff_ctx(base, exponent, ctx);
    let o = c_outcome(base, exponent);
    assert_eq!(
        o.bits,
        expected.to_bits(),
        "[{ctx}] C returned {} for base={}, exp={}, expected {}",
        describe(f64::from_bits(o.bits)),
        describe(base),
        describe(exponent),
        describe(expected)
    );
}

// ---------------------------------------------------------------------------
// Boundary search helpers (used by both Phase B and Phase C)
// ---------------------------------------------------------------------------

pub fn is_range_err(base: f64, e: f64) -> bool {
    c_outcome(base, e).errno == ERANGE
}

/// Bisect on the bit representation until `lo` (clean) and `hi` (ERANGE) are
/// adjacent doubles, so the returned pair straddles the boundary by one ULP.
/// For positive doubles the bit order equals the numeric order.
///
/// `hi` is discovered by doubling from `lo`, because the overflow threshold
/// depends steeply on the base (for a base just above 1 it is around 7e9).
/// Returns `None` if no ERANGE exponent exists for this base.
pub fn try_bisect_range_boundary(base: f64, lo: f64, negate: bool) -> Option<(f64, f64)> {
    let sign = |m: f64| if negate { -m } else { m };
    if !(lo > 0.0) || is_range_err(base, sign(lo)) {
        return None;
    }
    let mut hi = lo * 2.0;
    while !is_range_err(base, sign(hi)) {
        hi *= 2.0;
        if !hi.is_finite() {
            return None;
        }
    }
    let mut lob = lo.to_bits();
    let mut hib = hi.to_bits();
    while hib - lob > 1 {
        let mid = lob + (hib - lob) / 2;
        if is_range_err(base, sign(f64::from_bits(mid))) {
            hib = mid;
        } else {
            lob = mid;
        }
    }
    Some((sign(f64::from_bits(lob)), sign(f64::from_bits(hib))))
}

pub fn bisect_range_boundary(base: f64, lo: f64, negate: bool) -> (f64, f64) {
    try_bisect_range_boundary(base, lo, negate)
        .unwrap_or_else(|| panic!("no ERANGE boundary found for base={base} negate={negate}"))
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds, reproducible runs
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }

    /// Uniform integer in [0, n).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    /// Uniform integer in [lo, hi] inclusive.
    pub fn int_range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// `10^u` with `u` uniform in [lo_exp10, hi_exp10) — log-uniform magnitude.
    pub fn log_uniform(&mut self, lo_exp10: f64, hi_exp10: f64) -> f64 {
        let e = self.range(lo_exp10, hi_exp10);
        let v = 10f64.powf(e);
        if self.bool() { v } else { -v }
    }

    /// A finite, non-zero double with a random sign and log-uniform magnitude.
    pub fn finite(&mut self) -> f64 {
        self.log_uniform(-300.0, 300.0)
    }

    /// A fully random bit pattern reinterpreted as `f64` (any IEEE class,
    /// including non-canonical NaNs).
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }

    /// A random NaN: random payload, random sign, random signaling/quiet bit.
    pub fn any_nan(&mut self) -> f64 {
        let sign = (self.next_u64() & 1) << 63;
        // Non-zero 51-bit payload, plus an explicit quiet/signaling bit.
        let payload = (self.next_u64() & ((1u64 << 51) - 1)) | 1;
        let quiet = (self.next_u64() & 1) << 51;
        f64::from_bits(sign | 0x7FF0_0000_0000_0000 | quiet | payload)
    }

    /// A random subnormal, random sign.
    pub fn subnormal(&mut self) -> f64 {
        let mant = (self.next_u64() & ((1u64 << 52) - 1)) | 1;
        let sign = (self.next_u64() & 1) << 63;
        f64::from_bits(sign | mant)
    }
}

// ---------------------------------------------------------------------------
// Shared interesting-value catalogues
// ---------------------------------------------------------------------------

pub const QNAN: f64 = f64::NAN;
pub const INF: f64 = f64::INFINITY;

/// Signaling NaN: exponent all-ones, quiet bit clear, non-zero payload.
pub const SNAN: f64 = f64::from_bits(0x7FF0_0000_0000_0001);
/// Negative quiet NaN.
pub const NEG_QNAN: f64 = f64::from_bits(0xFFF8_0000_0000_0000);
/// Negative signaling NaN.
pub const NEG_SNAN: f64 = f64::from_bits(0xFFF0_0000_0000_0001);

/// True for a *signaling* NaN: max exponent, non-zero payload, quiet bit clear.
///
/// This matters because glibc's `pow` does NOT apply the C99 "`pow(x, 0)` is
/// always 1" / "`pow(1, y)` is always 1" rules to a signaling NaN — it returns
/// the quieted NaN instead. Both implementations agree; the tests must expect it.
pub fn is_snan(x: f64) -> bool {
    let b = x.to_bits();
    (b & 0x7FF0_0000_0000_0000) == 0x7FF0_0000_0000_0000
        && (b & 0x000F_FFFF_FFFF_FFFF) != 0
        && (b & 0x0008_0000_0000_0000) == 0
}

/// One representative of every base class the code distinguishes.
pub fn base_classes() -> Vec<(&'static str, f64)> {
    vec![
        ("+0.0", 0.0),
        ("-0.0", -0.0),
        ("+min_subnormal", 5e-324),
        ("-min_subnormal", -5e-324),
        ("+max_subnormal", f64::from_bits(0x000F_FFFF_FFFF_FFFF)),
        ("+DBL_MIN", f64::MIN_POSITIVE),
        ("-DBL_MIN", -f64::MIN_POSITIVE),
        ("0.5", 0.5),
        ("-0.5", -0.5),
        ("just_below_1", f64::from_bits(0x3FEF_FFFF_FFFF_FFFF)),
        ("1.0", 1.0),
        ("-1.0", -1.0),
        ("just_above_1", f64::from_bits(0x3FF0_0000_0000_0001)),
        ("2.0", 2.0),
        ("-2.0", -2.0),
        ("10.0", 10.0),
        ("+DBL_MAX", f64::MAX),
        ("-DBL_MAX", f64::MIN),
        ("+INF", INF),
        ("-INF", -INF),
        ("qNaN", QNAN),
        ("sNaN", SNAN),
        ("-qNaN", NEG_QNAN),
        ("-sNaN", NEG_SNAN),
    ]
}

/// One representative of every exponent class the code distinguishes.
pub fn exponent_classes() -> Vec<(&'static str, f64)> {
    vec![
        ("+0.0", 0.0),
        ("-0.0", -0.0),
        ("+min_subnormal", 5e-324),
        ("-min_subnormal", -5e-324),
        ("0.5", 0.5),
        ("-0.5", -0.5),
        ("one_third", 1.0 / 3.0),
        ("1.0", 1.0),
        ("-1.0", -1.0),
        ("1.5", 1.5),
        ("-1.5", -1.5),
        ("2.0", 2.0),
        ("-2.0", -2.0),
        ("2.5", 2.5),
        ("3.0", 3.0),
        ("-3.0", -3.0),
        ("4.0", 4.0),
        ("-4.0", -4.0),
        ("just_off_3", f64::from_bits(3.0f64.to_bits() + 1)),
        ("1e18", 1e18),
        ("-1e18", -1e18),
        ("2^53", 9007199254740992.0),
        ("2^53+1", 9007199254740993.0),
        ("+DBL_MAX", f64::MAX),
        ("-DBL_MAX", f64::MIN),
        ("+INF", INF),
        ("-INF", -INF),
        ("qNaN", QNAN),
        ("sNaN", SNAN),
        ("-qNaN", NEG_QNAN),
    ]
}
