// Shared differential-test harness.
//
// Both the C `libdriver.so` and the Rust `libdriver.so` are loaded at runtime
// with `libloading`; the Rust function is NEVER called directly, always through
// its `#[no_mangle]` export, exactly as an external C consumer would.
//
// `driver`'s entire observable behaviour is what it writes to the process's C
// `stdout`, so the harness captures fd 1 into a temporary file around each
// batch of calls and compares the two byte streams.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

/// fd 1 is process-global, so only one capture may be in flight at a time even
/// though `cargo test` runs test functions on several threads.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `void driver(double)` — the one and only exported entry point.
pub type DriverFn = unsafe extern "C" fn(f64);

// ---------------------------------------------------------------------------
// Locating the two shared libraries
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`, produced by the CMake build.
fn c_so_path() -> PathBuf {
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// The Rust `cdylib` for the profile the test binary itself was built under.
///
/// `current_exe()` is `<target>/<profile>/deps/<test>-<hash>`, so the sibling
/// cdylib is two directories up.
fn rust_so_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>")
        .to_path_buf();

    let mut candidates = vec![profile_dir.join("libdriver.so")];
    // Fall back to the other profile if the matching one was not built.
    candidates.push(manifest_dir().join("target/release/libdriver.so"));
    candidates.push(manifest_dir().join("target/debug/libdriver.so"));

    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib not found; looked in {:?}\nBuild it with: cargo build (and/or cargo build --release)",
        candidates
    );
}

// Keep the libraries loaded for the whole process lifetime: unloading a cdylib
// that has Rust std statically linked in is best avoided, and reloading per
// test would be pure overhead.
static C_LIB: OnceLock<Library> = OnceLock::new();
static RUST_LIB: OnceLock<Library> = OnceLock::new();

fn c_lib() -> &'static Library {
    C_LIB.get_or_init(|| unsafe { Library::new(c_so_path()).expect("dlopen C libdriver.so") })
}

fn rust_lib() -> &'static Library {
    RUST_LIB.get_or_init(|| unsafe { Library::new(rust_so_path()).expect("dlopen Rust libdriver.so") })
}

/// The C implementation's `driver`, resolved through `dlsym`.
pub fn c_driver() -> DriverFn {
    let sym: Symbol<DriverFn> = unsafe {
        c_lib()
            .get(b"driver\0")
            .expect("symbol `driver` missing from the C .so")
    };
    *sym
}

/// The Rust implementation's `driver`, resolved through `dlsym` — this is what
/// exercises the `#[no_mangle] extern "C"` export wrapper.
pub fn rust_driver() -> DriverFn {
    let sym: Symbol<DriverFn> = unsafe {
        rust_lib()
            .get(b"driver\0")
            .expect("symbol `driver` missing from the Rust .so")
    };
    *sym
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Run `body`, capturing everything the process writes to fd 1 while it runs.
///
/// A temporary *file* is used rather than a pipe so that arbitrarily large
/// output (row 10 alone emits ~320 bytes per call, and batches run into the
/// megabytes) can never deadlock on a full pipe buffer.
pub fn capture_stdout<F: FnMut()>(mut body: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // Flush anything already pending on the C and Rust sides so it is not
    // mis-attributed to the callee.
    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let path = std::env::temp_dir().join(format!(
        "driver-diff-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let mut tmp = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create capture file");

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

    body();

    // Flush the library's stdio buffer *while fd 1 is still redirected*.
    unsafe { fflush(std::ptr::null_mut()) };

    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };

    let mut out = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek capture file");
    tmp.read_to_end(&mut out).expect("read capture file");
    drop(tmp);
    let _ = std::fs::remove_file(&path);
    out
}

/// Capture the output of one `driver(f)` call.
pub fn run_one(f: DriverFn, x: f64) -> Vec<u8> {
    capture_stdout(|| unsafe { f(x) })
}

/// Capture the output of `driver(x)` for every `x`, in order, in one redirect.
pub fn run_batch(f: DriverFn, xs: &[f64]) -> Vec<u8> {
    capture_stdout(|| {
        for &x in xs {
            unsafe { f(x) }
        }
    })
}

/// Emit a line through the *test's own* libc `printf`, to prove that the
/// library's writes interleave with a caller's writes identically.
pub fn caller_printf_marker(i: usize) {
    let s = format!("caller-marker-{i}\0");
    unsafe { printf(b"%s\n\0".as_ptr() as *const c_char, s.as_ptr() as *const c_char) };
}

// ---------------------------------------------------------------------------
// The differential assertion
// ---------------------------------------------------------------------------

fn describe(x: f64) -> String {
    format!("{:?} (bits 0x{:016x})", x, x.to_bits())
}

/// Assert C and Rust produce byte-identical stdout for every input in `xs`.
///
/// The batch is compared as a single blob first (fast path); on any difference
/// the inputs are re-run one at a time so the failure message names the exact
/// diverging value.
pub fn assert_same(row: &str, xs: &[f64]) {
    let cf = c_driver();
    let rf = rust_driver();

    let c_out = run_batch(cf, xs);
    let r_out = run_batch(rf, xs);

    if c_out == r_out {
        return;
    }

    // Bisect to the first offending input.
    for &x in xs {
        let c1 = run_one(cf, x);
        let r1 = run_one(rf, x);
        if c1 != r1 {
            panic!(
                "[{row}] DIVERGENCE on input {}\n  C   : {:?}\n  Rust: {:?}\n  C   bytes: {:?}\n  Rust bytes: {:?}",
                describe(x),
                String::from_utf8_lossy(&c1),
                String::from_utf8_lossy(&r1),
                c1,
                r1,
            );
        }
    }

    panic!(
        "[{row}] batch outputs differ ({} vs {} bytes) but every individual input matched — \
         stateful/ordering divergence.\n  C   : {:?}\n  Rust: {:?}",
        c_out.len(),
        r_out.len(),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
}

/// Same as [`assert_same`] but takes raw bit patterns, so inputs unreachable
/// from a decimal literal (exotic NaN payloads, etc.) can be tested.
pub fn assert_same_bits(row: &str, bits: &[u64]) {
    let xs: Vec<f64> = bits.iter().copied().map(f64::from_bits).collect();
    assert_same(row, &xs);
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed, reproducible
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

    /// Uniform in `[0, n)`.
    pub fn below(&mut self, n: u64) -> u64 {
        assert!(n > 0);
        self.next_u64() % n
    }

    /// Uniform in `[lo, hi]`.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi - lo + 1)
    }

    pub fn sign(&mut self) -> u64 {
        (self.next_u64() & 1) << 63
    }

    /// Uniform in `[0, 1)`, built from 53 random mantissa bits.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
}

/// Assemble a `double` from its IEEE-754 fields.
pub fn compose(sign: u64, exponent: u64, mantissa: u64) -> f64 {
    debug_assert!(exponent < 2048);
    debug_assert!(mantissa < (1u64 << 52));
    f64::from_bits((sign & (1 << 63)) | (exponent << 52) | mantissa)
}

/// How many randomized inputs each property-style row uses.
pub const N: usize = 2000;
