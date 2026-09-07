//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` — the C one built by CMake and
//! the Rust `cdylib` built by Cargo — and calls them only through their exported
//! `extern "C"` symbols. Nothing in this crate's Rust API is ever called
//! directly, so the `#[no_mangle]` wrappers are part of what is under test.
//!
//! Every function in this library returns `void`; the *only* observable output is
//! what it writes to libc's `stdout`. So the harness captures file descriptor 1
//! around each call (`dup`/`dup2`/`fflush`) and compares the raw bytes.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed to capture fd 1
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// The loaded API
// ---------------------------------------------------------------------------

/// The five symbols the C `.so` exports, as raw `extern "C"` function pointers.
///
/// The pointers are copied out of the `libloading::Symbol` guards; `_lib` keeps
/// the `Library` (and hence the mapping) alive for the whole process.
pub struct Api {
    pub name: &'static str,
    pub print_line: unsafe extern "C" fn(*const c_char),
    pub print_int_line: unsafe extern "C" fn(c_int),
    pub bad: unsafe extern "C" fn(f32),
    pub good: unsafe extern "C" fn(f32),
    pub driver: unsafe extern "C" fn(f32, f32),
    _lib: Library,
}

impl Api {
    fn load(name: &'static str, path: &PathBuf) -> Api {
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
        unsafe {
            let print_line: Symbol<unsafe extern "C" fn(*const c_char)> =
                lib.get(b"printLine\0").expect("missing symbol printLine");
            let print_int_line: Symbol<unsafe extern "C" fn(c_int)> =
                lib.get(b"printIntLine\0").expect("missing symbol printIntLine");
            let bad: Symbol<unsafe extern "C" fn(f32)> =
                lib.get(b"bad\0").expect("missing symbol bad");
            let good: Symbol<unsafe extern "C" fn(f32)> =
                lib.get(b"good\0").expect("missing symbol good");
            let driver: Symbol<unsafe extern "C" fn(f32, f32)> =
                lib.get(b"driver\0").expect("missing symbol driver");
            Api {
                name,
                print_line: *print_line,
                print_int_line: *print_int_line,
                bad: *bad,
                good: *good,
                driver: *driver,
                _lib: lib,
            }
        }
    }
}

/// Directory holding the compiled test binary, i.e. `target/<profile>/deps`'s parent.
fn artifact_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test>-<hash>  ->  .../target/<profile>
    exe.parent()
        .and_then(|p| p.parent())
        .expect("test exe has no grandparent dir")
        .to_path_buf()
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the CMake-built C shared library.
pub fn c_so_path() -> PathBuf {
    let root = manifest_dir().parent().expect("crate has a parent dir").to_path_buf();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/lib/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         looked in: {candidates:?}"
    );
}

/// Path to the Cargo-built Rust `cdylib`, for the SAME profile as this test
/// binary.
///
/// `cargo test` does *not* build `crate-type = ["cdylib"]` artifacts (the test
/// harness never links them), so the `.so` must be produced by an explicit
/// `cargo build [--release]`. This function deliberately does **not** fall back
/// to the other profile: silently testing the release object from a debug run
/// would report coverage that was never exercised. Override with the
/// `DRIVER_RUST_SO` environment variable if needed.
pub fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_RUST_SO points at a missing file: {}", p.display());
        return p;
    }
    let dir = artifact_dir();
    let direct = dir.join("libdriver.so");
    if direct.is_file() {
        return direct;
    }
    let profile = dir
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("debug")
        .to_string();
    let build_flag = if profile == "release" { " --release" } else { "" };
    panic!(
        "Rust cdylib not found at {}.\n\
         `cargo test` does not build cdylib artifacts — build it first:\n  \
         cd translation && cargo build{build_flag}\n\
         (or point DRIVER_RUST_SO at the .so you want to test)",
        direct.display()
    );
}

pub fn c_api() -> &'static Api {
    static C: OnceLock<Api> = OnceLock::new();
    C.get_or_init(|| Api::load("C", &c_so_path()))
}

pub fn rust_api() -> &'static Api {
    static R: OnceLock<Api> = OnceLock::new();
    R.get_or_init(|| Api::load("Rust", &rust_so_path()))
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// `dup2` on fd 1 is process-global, and Cargo runs tests on multiple threads,
/// so captures must be serialised.
fn capture_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Runs `f` with fd 1 redirected to a temporary file and returns everything it
/// wrote. `fflush(NULL)` is issued before restoring fd 1 so that the (fully
/// buffered, because fd 1 is a file) libc stream is drained.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);

    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "driver_difftest_{}_{}.out",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    let bytes = unsafe {
        // Drain anything already pending on the real stdout.
        fflush(std::ptr::null_mut());

        let file = std::fs::File::create(&path).expect("create temp capture file");
        let tmp_fd = {
            use std::os::unix::io::AsRawFd;
            file.as_raw_fd()
        };

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp_fd, 1) >= 0, "dup2 onto fd 1 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
        drop(file);

        let mut buf = Vec::new();
        std::fs::File::open(&path)
            .expect("reopen temp capture file")
            .read_to_end(&mut buf)
            .expect("read temp capture file");
        buf
    };

    let _ = std::fs::remove_file(&path);
    bytes
}

/// Renders a byte string for assertion messages.
pub fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Runs the same closure against the C API and the Rust API and asserts the
/// captured stdout bytes are identical.
pub fn diff<F>(label: &str, f: F)
where
    F: Fn(&Api),
{
    let c_out = capture(|| f(c_api()));
    let r_out = capture(|| f(rust_api()));
    if c_out != r_out {
        panic!(
            "DIVERGENCE [{label}]\n  C    ({:3} bytes): \"{}\"\n  Rust ({:3} bytes): \"{}\"",
            c_out.len(),
            show(&c_out),
            r_out.len(),
            show(&r_out),
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (xorshift64*) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 1 } else { seed })
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
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Log-uniform magnitude in `[lo, hi]` with a random sign.
    pub fn signed_log_f32(&mut self, lo: f64, hi: f64) -> f32 {
        let l = lo.ln();
        let h = hi.ln();
        let m = (l + (h - l) * self.unit()).exp();
        let v = if self.next_u64() & 1 == 0 { m } else { -m };
        v as f32
    }
    /// An arbitrary `f32` built from random bits — includes NaNs, infinities,
    /// subnormals and zeroes, i.e. the whole domain.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

// ---------------------------------------------------------------------------
// Named boundary values, shared by several test files
// ---------------------------------------------------------------------------

/// `0.000001` is the `goodB2G` guard threshold. `1e-6f32` converted to `f64` is
/// `9.9999999747524271e-07`, which is *below* the threshold, so `1e-6f32` is
/// rejected while the next float up is accepted.
pub const GUARD_F32: f32 = 1e-6f32;

/// Smallest `f32` strictly greater than `GUARD_F32` (`nextafterf(1e-6f, 1.0f)`).
pub fn guard_next_up() -> f32 {
    f32::from_bits(GUARD_F32.to_bits() + 1)
}

/// `100.0 / x == 2^31` exactly at `x == 100.0 / 2147483648.0`; divisors at or
/// below this magnitude make the `(int)` cast overflow.
pub const CAST_LIMIT: f64 = 100.0 / 2147483648.0;

/// The float classes the C code distinguishes (see `CONFIGS.md`, axis F).
pub fn named_floats() -> Vec<(&'static str, f32)> {
    vec![
        ("+0.0", 0.0f32),
        ("-0.0", -0.0f32),
        ("subnormal_min", f32::from_bits(1)),
        ("subnormal_mid", f32::from_bits(0x0040_0000)),
        ("FLT_MIN", f32::MIN_POSITIVE),
        ("-FLT_MIN", -f32::MIN_POSITIVE),
        ("5e-07", 5e-07f32),
        ("-5e-07", -5e-07f32),
        ("guard_1e-6", GUARD_F32),
        ("-guard_1e-6", -GUARD_F32),
        ("guard_next_up", guard_next_up()),
        ("-guard_next_up", -guard_next_up()),
        ("cast_limit", CAST_LIMIT as f32),
        ("-cast_limit", -(CAST_LIMIT as f32)),
        ("cast_limit_up", f32::from_bits((CAST_LIMIT as f32).to_bits() + 1)),
        ("cast_limit_down", f32::from_bits((CAST_LIMIT as f32).to_bits() - 1)),
        ("1.0", 1.0f32),
        ("-1.0", -1.0f32),
        ("2.0", 2.0f32),
        ("3.0", 3.0f32),
        ("-3.0", -3.0f32),
        ("7.0", 7.0f32),
        ("-7.0", -7.0f32),
        ("100.0", 100.0f32),
        ("-100.0", -100.0f32),
        ("1e10", 1e10f32),
        ("-1e10", -1e10f32),
        ("FLT_MAX", f32::MAX),
        ("-FLT_MAX", f32::MIN),
        ("+inf", f32::INFINITY),
        ("-inf", f32::NEG_INFINITY),
        ("qnan", f32::NAN),
        ("-qnan", -f32::NAN),
        ("snan", f32::from_bits(0x7F80_0001)),
    ]
}
