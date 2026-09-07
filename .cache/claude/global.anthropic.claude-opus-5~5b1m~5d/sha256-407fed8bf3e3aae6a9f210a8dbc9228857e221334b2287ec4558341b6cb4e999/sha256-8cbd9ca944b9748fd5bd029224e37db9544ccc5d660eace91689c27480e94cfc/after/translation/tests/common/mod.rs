// Differential-test harness: loads BOTH the C `.so` and the Rust `.so` through
// `libloading` and calls `my_pow` across the FFI boundary in each, capturing
//   (1) the returned f64's raw bit pattern,
//   (2) the exact bytes the call wrote to fd 2 (stderr),
//   (3) the caller-visible `errno` after the call returns.
//
// Nothing in this file calls the Rust implementation directly — it is always
// reached via `dlsym` on the built shared object, exactly as an external C
// consumer would, so the `#[no_mangle]` export wrapper is under test too.

#![allow(dead_code)]

use std::ffi::{c_int, c_void};
use std::os::unix::fs::FileExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

use libloading::{Library, Symbol};

pub type MyPowFn = unsafe extern "C" fn(f64, f64) -> f64;

extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

/// `<workspace>/c_src/build/libpow.so`
pub fn c_so_path() -> PathBuf {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let p = manifest
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src/build/libpow.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// The Rust `cdylib` built for the same profile as this test binary
/// (`target/<profile>/libpow.so`).
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_POW_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "RUST_POW_SO={} does not exist", p.display());
        return p;
    }
    // current_exe() == target/<profile>/deps/<testname>-<hash>
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent() // deps/
        .and_then(|p| p.parent()) // <profile>/
        .expect("test exe is under target/<profile>/deps/");
    let candidate = profile_dir.join("libpow.so");
    if candidate.exists() {
        return candidate;
    }
    // Fall back to the sibling profile directory.
    let target_dir = profile_dir.parent().expect("target dir");
    for prof in ["release", "debug"] {
        let c = target_dir.join(prof).join("libpow.so");
        if c.exists() {
            return c;
        }
    }
    panic!(
        "Rust shared library libpow.so not found under {}. Build it with `cargo build --release`.",
        target_dir.display()
    );
}

/// Owns both loaded libraries and the resolved `my_pow` symbols.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: MyPowFn,
    pub rust: MyPowFn,
}

impl Pair {
    pub fn load() -> Pair {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("dlopen C libpow.so");
            let rust_lib = Library::new(rust_so_path()).expect("dlopen Rust libpow.so");
            let c_sym: Symbol<MyPowFn> =
                c_lib.get(b"my_pow\0").expect("dlsym my_pow in C libpow.so");
            let rust_sym: Symbol<MyPowFn> = rust_lib
                .get(b"my_pow\0")
                .expect("dlsym my_pow in Rust libpow.so");
            let c = *c_sym;
            let rust = *rust_sym;
            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// stderr capture (fd-level, so it catches writes made inside either .so)
// ---------------------------------------------------------------------------

static CAP_SEQ: AtomicU32 = AtomicU32::new(0);

/// fd 2 is a PROCESS-wide resource, but `cargo test` runs test functions on
/// parallel threads. Redirecting fd 2 in one thread would capture the
/// diagnostics emitted by every other thread as well. This mutex serializes the
/// whole lifetime of a `StderrCapture`, so exactly one test at a time owns fd 2.
///
/// Consequence: all *harness* progress messages must go to **stdout**
/// (`println!`), never stderr, so they can never land inside a capture file.
static STDERR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct StderrCapture {
    saved_fd: c_int,
    path: PathBuf,
    reader: std::fs::File,
    pos: u64,
    /// Held for the whole capture; released only after fd 2 has been restored
    /// (struct fields drop after the `Drop::drop` body runs).
    _guard: std::sync::MutexGuard<'static, ()>,
}

impl StderrCapture {
    pub fn new() -> StderrCapture {
        let guard = STDERR_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        let n = CAP_SEQ.fetch_add(1, Ordering::SeqCst);
        let path = PathBuf::from(dir).join(format!(
            "pow_diff_stderr_{}_{}_{}.txt",
            std::process::id(),
            n,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        // Create/truncate the sink.
        let sink = std::fs::File::create(&path).expect("create stderr sink");
        let reader = std::fs::File::open(&path).expect("open stderr sink for reading");
        unsafe {
            // Make sure nothing is left pending on the real stderr.
            fflush(std::ptr::null_mut());
            let saved_fd = libc::dup(2);
            assert!(saved_fd >= 0, "dup(2) failed");
            let rc = libc::dup2(std::os::unix::io::AsRawFd::as_raw_fd(&sink), 2);
            assert!(rc >= 0, "dup2 -> 2 failed");
            drop(sink);
            StderrCapture {
                saved_fd,
                path,
                reader,
                pos: 0,
                _guard: guard,
            }
        }
    }

    /// Bytes written to stderr since the previous `take()`.
    pub fn take(&mut self) -> Vec<u8> {
        unsafe {
            fflush(std::ptr::null_mut());
        }
        let len = std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(self.pos);
        if len <= self.pos {
            self.pos = len;
            return Vec::new();
        }
        let mut buf = vec![0u8; (len - self.pos) as usize];
        self.reader
            .read_exact_at(&mut buf, self.pos)
            .expect("read stderr sink");
        self.pos = len;
        buf
    }
}

impl Drop for StderrCapture {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            libc::dup2(self.saved_fd, 2);
            libc::close(self.saved_fd);
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

// ---------------------------------------------------------------------------
// One observation
// ---------------------------------------------------------------------------

#[derive(Clone, PartialEq, Eq)]
pub struct Obs {
    /// Raw IEEE-754 bits of the returned double (distinguishes ±0.0 and NaN payloads).
    pub bits: u64,
    /// `errno` as seen by the caller immediately after the call returned.
    pub errno: c_int,
    /// Bytes the call wrote to fd 2.
    pub stderr: Vec<u8>,
}

impl std::fmt::Debug for Obs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Obs {{ bits: {:#018x} ({:?}), errno: {}, stderr: {:?} }}",
            self.bits,
            f64::from_bits(self.bits),
            self.errno,
            String::from_utf8_lossy(&self.stderr)
        )
    }
}

pub unsafe fn errno_get() -> c_int {
    *libc::__errno_location()
}

pub unsafe fn errno_set(v: c_int) {
    *libc::__errno_location() = v;
}

/// Invoke one `my_pow` implementation and record all three observable outputs.
///
/// `preset_errno` is written into the caller's `errno` slot before the call, so
/// the "does the callee reset `errno` first?" behaviour is observable.
pub fn observe(cap: &mut StderrCapture, f: MyPowFn, base: f64, exp: f64, preset_errno: c_int) -> Obs {
    // Discard anything buffered from earlier activity.
    let _ = cap.take();
    unsafe {
        errno_set(preset_errno);
        let r = f(base, exp);
        let e = errno_get();
        let out = cap.take();
        Obs {
            bits: r.to_bits(),
            errno: e,
            stderr: out,
        }
    }
}

/// A single divergence record, rendered lazily so nothing prints while stderr
/// is still redirected.
pub struct Divergence {
    pub base: f64,
    pub exp: f64,
    pub preset_errno: c_int,
    pub c: Obs,
    pub rust: Obs,
}

impl std::fmt::Display for Divergence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "my_pow(base={:?} [{:#018x}], exp={:?} [{:#018x}], preset_errno={})\n     C: {:?}\n  RUST: {:?}",
            self.base,
            self.base.to_bits(),
            self.exp,
            self.exp.to_bits(),
            self.preset_errno,
            self.c,
            self.rust
        )
    }
}

/// Core differential step: call C then Rust with identical inputs and compare
/// return bits, `errno`, and stderr bytes. Returns `Some(Divergence)` on
/// mismatch. Never panics and never prints (safe to call while fd 2 is
/// redirected).
pub fn diff_once(
    cap: &mut StderrCapture,
    p: &Pair,
    base: f64,
    exp: f64,
    preset_errno: c_int,
) -> Option<Divergence> {
    let c = observe(cap, p.c, base, exp, preset_errno);
    let rust = observe(cap, p.rust, base, exp, preset_errno);
    if c == rust {
        None
    } else {
        Some(Divergence {
            base,
            exp,
            preset_errno,
            c,
            rust,
        })
    }
}

/// Runs `cases` through `diff_once`, restores stderr, then reports.
pub fn assert_all_match<I: IntoIterator<Item = (f64, f64)>>(label: &str, cases: I) {
    assert_all_match_with_errno(label, cases.into_iter().map(|(b, e)| (b, e, 0)));
}

pub fn assert_all_match_with_errno<I: IntoIterator<Item = (f64, f64, c_int)>>(
    label: &str,
    cases: I,
) {
    let p = Pair::load();
    let mut cap = StderrCapture::new();
    let mut diffs: Vec<Divergence> = Vec::new();
    let mut n: u64 = 0;
    for (base, exp, pe) in cases {
        n += 1;
        if let Some(d) = diff_once(&mut cap, &p, base, exp, pe) {
            if diffs.len() < 25 {
                diffs.push(d);
            }
        }
    }
    drop(cap); // restore fd 2 BEFORE any printing/panicking
    if !diffs.is_empty() {
        let mut msg = format!(
            "[{}] {} of {} cases diverged between C and Rust (first {} shown):\n",
            label,
            diffs.len(),
            n,
            diffs.len()
        );
        for d in &diffs {
            msg.push_str(&format!("  - {}\n", d));
        }
        panic!("{}", msg);
    }
    assert!(n > 0, "[{}] generated no cases", label);
    println!("[{}] {} cases matched byte-for-byte", label, n);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*), fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const SEED: u64 = 0x2545_F491_4F6C_DD1D;

    pub fn new() -> Rng {
        Rng(Self::SEED)
    }

    pub fn with_seed(s: u64) -> Rng {
        Rng(if s == 0 { Self::SEED } else { s })
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
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    /// Uniform in [lo, hi).
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }

    pub fn bool(&mut self) -> bool {
        self.next_u64() & 1 == 1
    }

    /// An arbitrary IEEE-754 double from a fully random bit pattern
    /// (may be NaN of any payload, ±inf, ±0, subnormal, …).
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }

    pub fn pick<T: Copy>(&mut self, xs: &[T]) -> T {
        xs[(self.below(xs.len() as u64)) as usize]
    }

    /// Random even integer in [-40, 40].
    pub fn even_int(&mut self) -> f64 {
        let k = (self.below(21) as i64) - 10;
        (k * 2) as f64
    }

    /// Random odd integer in [-41, 41].
    pub fn odd_int(&mut self) -> f64 {
        let k = (self.below(21) as i64) - 10;
        (k * 2 + 1) as f64
    }
}

// ---------------------------------------------------------------------------
// Curated special-value pool
// ---------------------------------------------------------------------------

pub const QUIET_NAN_A: f64 = f64::NAN;

pub fn special_doubles() -> Vec<f64> {
    vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        0.5,
        -0.5,
        1.5,
        -1.5,
        3.0,
        -3.0,
        4.0,
        -4.0,
        10.0,
        -10.0,
        0.1,
        -0.1,
        f64::consts_pi(),
        -f64::consts_pi(),
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        // distinct quiet-NaN payloads
        f64::from_bits(0x7ff8_0000_0000_0001),
        f64::from_bits(0xfff8_dead_beef_cafe),
        // signalling NaN bit patterns
        f64::from_bits(0x7ff0_0000_0000_0001),
        f64::from_bits(0xfff0_0000_dead_0001),
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        // subnormals
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
        f64::from_bits(0x000f_ffff_ffff_ffff),
        f64::from_bits(0x800f_ffff_ffff_ffff),
        f64::EPSILON,
        -f64::EPSILON,
        1e300,
        -1e300,
        1e-300,
        -1e-300,
        1022.0,
        -1022.0,
        1023.0,
        -1023.0,
        1024.0,
        -1024.0,
        -1040.0,
        100000.0,
        -100000.0,
        9007199254740992.0,  // 2^53
        9007199254740993.0,  // 2^53 + 1 (rounds to 2^53)
        1e17,
        -1e17,
        1.0000001,
        0.9999999,
    ]
}

trait Pi {
    fn consts_pi() -> f64;
}
impl Pi for f64 {
    fn consts_pi() -> f64 {
        std::f64::consts::PI
    }
}
