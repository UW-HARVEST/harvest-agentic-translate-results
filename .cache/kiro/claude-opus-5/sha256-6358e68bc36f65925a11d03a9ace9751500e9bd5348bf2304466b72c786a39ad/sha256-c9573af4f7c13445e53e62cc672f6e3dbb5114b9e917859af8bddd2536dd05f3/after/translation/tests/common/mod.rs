//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls `my_pow` only
//! through their exported symbols — the Rust crate is never linked or called
//! directly, so the `#[no_mangle] extern "C"` wrapper is under test too.

#![allow(dead_code)]

use std::ffi::{CString, c_double, c_int, c_void};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

pub type MyPow = unsafe extern "C" fn(c_double, c_double) -> c_double;

/// One loaded implementation.
pub struct Impl {
    pub name: &'static str,
    _lib: Library,
    func: MyPow,
}

impl Impl {
    pub fn call(&self, base: f64, exponent: f64) -> f64 {
        // SAFETY: signature matches `double my_pow(double, double)`.
        unsafe { (self.func)(base, exponent) }
    }
}

fn load(name: &'static str, path: &Path) -> Impl {
    assert!(
        path.exists(),
        "{name} shared object not found at {}",
        path.display()
    );
    // SAFETY: loading a plain C shared library with no initialisers of note.
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));
    let func = {
        // SAFETY: the symbol is `double my_pow(double, double)` in both libs.
        let sym: Symbol<MyPow> = unsafe { lib.get(b"my_pow\0") }
            .unwrap_or_else(|e| panic!("{name}: my_pow not exported: {e}"));
        // SAFETY: `lib` is kept alive alongside the pointer in `Impl`.
        unsafe { *sym.into_raw() }
    };
    Impl {
        name,
        _lib: lib,
        func,
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    manifest_dir().join("../c_src/build/libpow.so")
}

/// Finds the Rust `cdylib` next to the running test binary
/// (`target/<profile>/libpow.so`).
///
/// Only the *current* profile directory is accepted: silently falling back to
/// a stale `.so` from another profile would test the wrong artifact.
fn rust_so_path() -> PathBuf {
    // Escape hatch used only by `mutation_check.sh`, which points the harness
    // at a deliberately-broken build to prove the suite can fail.
    if let Some(p) = std::env::var_os("POW_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    let deps = exe.parent().expect("deps dir");
    let profile = deps.parent().expect("profile dir");
    let candidates = [profile.join("libpow.so"), deps.join("libpow.so")];
    for c in candidates.iter() {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib libpow.so not found in the current profile dir; looked in {:?}. \
         Run `cargo build` for this profile first.",
        candidates
    );
}

pub struct Pair {
    pub c: Impl,
    pub rust: Impl,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

/// Both implementations, loaded once per test binary.
pub fn pair() -> &'static Pair {
    static PAIR: OnceLock<Pair> = OnceLock::new();
    PAIR.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        Pair {
            c: load("C", &c_path),
            rust: load("Rust", &rust_path),
            c_path,
            rust_path,
        }
    })
}

/// `stderr` redirection is process-global, so capturing tests must not
/// overlap.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` with file descriptor 2 redirected to a temporary file and returns
/// `f`'s value together with everything written to `stderr`.
pub fn capture_stderr<T>(f: impl FnOnce() -> T) -> (T, Vec<u8>) {
    let _guard = capture_lock();

    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("pow_diff_{}_{}.err", std::process::id(), n));
    let cpath = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();

    // SAFETY: plain POSIX fd juggling; every raw fd obtained is closed below.
    let (saved, tmp_fd) = unsafe {
        libc::fflush(std::ptr::null_mut());
        let saved: c_int = libc::dup(2);
        assert!(saved >= 0, "dup(2) failed");
        let tmp_fd: c_int = libc::open(
            cpath.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600 as libc::c_uint,
        );
        assert!(tmp_fd >= 0, "open({}) failed", path.display());
        assert!(libc::dup2(tmp_fd, 2) >= 0, "dup2 failed");
        (saved, tmp_fd)
    };

    let value = f();

    // SAFETY: as above; restores fd 2 and closes the duplicates.
    let bytes = unsafe {
        libc::fflush(std::ptr::null_mut());
        libc::lseek(tmp_fd, 0, libc::SEEK_SET);
        let mut out = Vec::new();
        let mut buf = [0u8; 8192];
        loop {
            let got = libc::read(tmp_fd, buf.as_mut_ptr() as *mut c_void, buf.len());
            if got <= 0 {
                break;
            }
            out.extend_from_slice(&buf[..got as usize]);
        }
        libc::dup2(saved, 2);
        libc::close(saved);
        libc::close(tmp_fd);
        out
    };
    let _ = std::fs::remove_file(&path);

    (value, bytes)
}

/// Reads the calling thread's `errno`.
pub fn errno() -> c_int {
    // SAFETY: `__errno_location` always returns a valid pointer.
    unsafe { *libc::__errno_location() }
}

/// Writes the calling thread's `errno`.
pub fn set_errno(v: c_int) {
    // SAFETY: as above.
    unsafe { *libc::__errno_location() = v }
}

// ---------------------------------------------------------------------------
// Comparison helpers
// ---------------------------------------------------------------------------

fn show(x: f64) -> String {
    format!("{x:?} (bits {:#018x})", x.to_bits())
}

/// Result of driving one implementation over a list of inputs.
pub struct Run {
    pub bits: Vec<u64>,
    pub errnos: Vec<c_int>,
    pub stderr: Vec<u8>,
}

/// Calls `imp` once per input inside a single `stderr` capture, recording the
/// returned bit pattern and the resulting `errno` for each call.
pub fn run(imp: &Impl, inputs: &[(f64, f64)]) -> Run {
    let ((bits, errnos), stderr) = capture_stderr(|| {
        let mut bits = Vec::with_capacity(inputs.len());
        let mut errnos = Vec::with_capacity(inputs.len());
        for &(b, e) in inputs {
            let r = imp.call(b, e);
            errnos.push(errno());
            bits.push(r.to_bits());
        }
        (bits, errnos)
    });
    Run {
        bits,
        errnos,
        stderr,
    }
}

/// Drives BOTH implementations over `inputs` and asserts byte-identical
/// behaviour: same returned bit patterns, same `errno` left behind, same
/// `stderr` bytes.
pub fn assert_same(label: &str, inputs: &[(f64, f64)]) {
    let p = pair();
    let c = run(&p.c, inputs);
    let r = run(&p.rust, inputs);

    assert_eq!(c.bits.len(), inputs.len());
    assert_eq!(r.bits.len(), inputs.len());

    for (i, &(b, e)) in inputs.iter().enumerate() {
        assert_eq!(
            c.bits[i], r.bits[i],
            "[{label}] return value mismatch at input #{i}: my_pow({}, {})\n  C    = {}\n  Rust = {}",
            show(b),
            show(e),
            show(f64::from_bits(c.bits[i])),
            show(f64::from_bits(r.bits[i])),
        );
        assert_eq!(
            c.errnos[i], r.errnos[i],
            "[{label}] errno mismatch at input #{i}: my_pow({}, {}) -> C errno {} vs Rust errno {}",
            show(b),
            show(e),
            c.errnos[i],
            r.errnos[i],
        );
    }

    assert_stderr_eq(label, &c.stderr, &r.stderr);
}

/// Compares two captured `stderr` streams, reporting the first differing line.
pub fn assert_stderr_eq(label: &str, c: &[u8], r: &[u8]) {
    if c == r {
        return;
    }
    let cl: Vec<&[u8]> = c.split(|&b| b == b'\n').collect();
    let rl: Vec<&[u8]> = r.split(|&b| b == b'\n').collect();
    for i in 0..cl.len().max(rl.len()) {
        let a = cl.get(i).copied().unwrap_or(b"<missing>");
        let b = rl.get(i).copied().unwrap_or(b"<missing>");
        if a != b {
            panic!(
                "[{label}] stderr mismatch on line {i} \
                 (C emitted {} lines, Rust {} lines)\n  C    = {:?}\n  Rust = {:?}",
                cl.len(),
                rl.len(),
                String::from_utf8_lossy(a),
                String::from_utf8_lossy(b),
            );
        }
    }
    panic!("[{label}] stderr mismatch (lengths {} vs {})", c.len(), r.len());
}

/// Single-input convenience wrapper that also returns the C-side observations,
/// so error-path tests can additionally assert the concrete sentinel/message.
pub struct One {
    pub value: f64,
    pub errno: c_int,
    pub stderr: Vec<u8>,
}

pub fn assert_same_one(label: &str, base: f64, exponent: f64) -> One {
    let inputs = [(base, exponent)];
    assert_same(label, &inputs);
    let p = pair();
    let c = run(&p.c, &inputs);
    One {
        value: f64::from_bits(c.bits[0]),
        errno: c.errnos[0],
        stderr: c.stderr,
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed, no external dependency)
// ---------------------------------------------------------------------------

/// xoshiro256** — deterministic and reproducible across runs and platforms.
pub struct Rng {
    s: [u64; 4],
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        // SplitMix64 expansion of the seed.
        let mut z = seed;
        let mut next = || {
            z = z.wrapping_add(0x9E3779B97F4A7C15);
            let mut x = z;
            x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
            x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
            x ^ (x >> 31)
        };
        Rng {
            s: [next(), next(), next(), next()],
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let result = self.s[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
        let t = self.s[1] << 17;
        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];
        self.s[2] ^= t;
        self.s[3] = self.s[3].rotate_left(45);
        result
    }

    /// Uniform in `[0, 1)`.
    pub fn next_unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.next_unit()
    }

    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }

    /// A completely arbitrary `f64` — any FP class, any NaN payload.
    pub fn any_f64(&mut self) -> f64 {
        f64::from_bits(self.next_u64())
    }
}

/// Values every FP class / special case is represented by.
pub fn special_values() -> Vec<f64> {
    vec![
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
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::EPSILON,
        5e-324,          // smallest subnormal
        -5e-324,
        1e300,
        -1e300,
        1e-300,
        -1e-300,
        9007199254740992.0,  // 2^53
        -9007199254740992.0,
        1024.0,
        -1024.0,
        1023.0,
        -1023.0,
    ]
}
