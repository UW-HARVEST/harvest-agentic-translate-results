//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared objects are loaded with `libloading` (i.e. `dlopen`) and every
//! call goes through an exported symbol looked up by name — the Rust functions
//! are never called directly, so the `#[no_mangle]` / `extern "C"` wrappers are
//! under test too.
//!
//! The library's *only* observable channel is `stdout` (every exported function
//! returns `void` and there is no `errno` / return-code / global state), so the
//! harness captures file descriptor 1 around each call and compares raw bytes.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_float, c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits used for capturing fd 1. Declared inline so the test target needs
// no dependency beyond `libloading`.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams, which is exactly what
    /// is needed: it drains whichever `FILE*` the `.so`s share.
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects.
// ---------------------------------------------------------------------------

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    // `DRIVER_C_SO` lets the runner point at an alternative build of the C
    // reference (e.g. an -O3 one) without touching c_src/.
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_C_SO={} is not a file", p.display());
        return p;
    }
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    // `DRIVER_RUST_SO` selects a specific profile's cdylib (used to verify the
    // debug build as well as the release build).
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.is_file(), "DRIVER_RUST_SO={} is not a file", p.display());
        assert_not_stale(&p);
        return p;
    }
    // Prefer the release cdylib; fall back to debug so `cargo test` (which
    // builds the debug profile) still finds something if release is absent.
    let root = workspace_root().join("translation/target");
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libdriver.so");
        if p.is_file() {
            assert_not_stale(&p);
            return p;
        }
    }
    panic!(
        "Rust shared library not found under {}.\nBuild it with:  cd translation && cargo build --release",
        root.display()
    );
}

/// `cargo test` builds the *test* binary but does **not** re-link the `cdylib`
/// artifact, so it is entirely possible to run the whole suite against a stale
/// `libdriver.so` and see 56 green results that prove nothing. Refuse to run in
/// that situation.
fn assert_not_stale(so: &PathBuf) {
    let root = workspace_root().join("translation");
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat the Rust .so");
    for src in ["src/lib.rs", "Cargo.toml"] {
        let p = root.join(src);
        let src_mtime = std::fs::metadata(&p)
            .and_then(|m| m.modified())
            .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()));
        assert!(
            src_mtime <= so_mtime,
            "STALE ARTIFACT: {} is newer than {}.\n\
             `cargo test` does not re-link the cdylib, so the suite would be \
             testing an out-of-date library.\nRun:  cd translation && \
             cargo build --release && cargo test --release",
            p.display(),
            so.display()
        );
    }
}

/// Public accessors so tests (e.g. the symbol-parity check) resolve the very
/// same files the differential cases loaded, env overrides included.
pub fn c_so() -> PathBuf {
    c_so_path()
}
pub fn rust_so() -> PathBuf {
    rust_so_path()
}

/// The five exported symbols, resolved once per library.
pub struct Driver {
    _lib: Library,
    pub print_line: unsafe extern "C" fn(*const c_char),
    pub print_int_line: unsafe extern "C" fn(c_int),
    pub bad: unsafe extern "C" fn(c_float),
    pub good: unsafe extern "C" fn(c_float),
    pub driver: unsafe extern "C" fn(c_float, c_float),
}

impl Driver {
    fn load(path: &PathBuf) -> Driver {
        unsafe {
            let lib = Library::new(path)
                .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()));

            macro_rules! sym {
                ($name:literal, $ty:ty) => {{
                    let s: Symbol<$ty> = lib.get($name).unwrap_or_else(|e| {
                        panic!("{} does not export `{}`: {e}", path.display(), 
                               std::str::from_utf8($name).unwrap())
                    });
                    *s.into_raw()
                }};
            }

            let print_line = sym!(b"printLine\0", unsafe extern "C" fn(*const c_char));
            let print_int_line = sym!(b"printIntLine\0", unsafe extern "C" fn(c_int));
            let bad = sym!(b"bad\0", unsafe extern "C" fn(c_float));
            let good = sym!(b"good\0", unsafe extern "C" fn(c_float));
            let driver = sym!(b"driver\0", unsafe extern "C" fn(c_float, c_float));

            Driver {
                _lib: lib,
                print_line,
                print_int_line,
                bad,
                good,
                driver,
            }
        }
    }
}

/// Both libraries plus the capture lock.
pub struct Pair {
    pub c: Driver,
    pub rust: Driver,
}

static PAIR: OnceLock<Pair> = OnceLock::new();
/// fd-1 redirection is process-global, so captures must be serialised.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

pub fn libs() -> &'static Pair {
    PAIR.get_or_init(|| Pair {
        c: Driver::load(&c_so_path()),
        rust: Driver::load(&rust_so_path()),
    })
}

fn capture_guard() -> MutexGuard<'static, ()> {
    CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Run `f` with fd 1 redirected to a temporary file and return the bytes it
/// wrote. `fflush(NULL)` is issued before and after so nothing leaks across
/// the boundary from either `.so`'s stdio buffer.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        let mut file = tempfile();
        // Drain BOTH buffering layers before stealing fd 1: libc's FILE* (used
        // by the two .so's) and Rust's own `Stdout` LineWriter (used by this
        // test binary). Without the latter, a partially buffered Rust line
        // would be flushed into the capture file and corrupt the comparison.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 -> 1 failed");

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);

        if let Err(p) = result {
            std::panic::resume_unwind(p);
        }

        file.seek(SeekFrom::Start(0)).expect("seek");
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).expect("read capture");
        buf
    }
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "driver-diff-{}-{}.out",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create temp capture file");
    // Unlink immediately; the fd keeps it alive and nothing is left behind.
    let _ = std::fs::remove_file(&path);
    file
}

// ---------------------------------------------------------------------------
// The differential primitive
// ---------------------------------------------------------------------------

/// Call the same operation on the C `.so` then the Rust `.so`, capturing each
/// separately, and assert the emitted bytes are identical.
///
/// `op` receives one `&Driver` and performs an arbitrary sequence of calls, so
/// multi-call / ordering scenarios go through the same path as single calls.
pub fn assert_same<F>(label: &str, op: F)
where
    F: Fn(&Driver),
{
    let libs = libs();
    let _g = capture_guard();
    let from_c = capture(|| op(&libs.c));
    let from_rust = capture(|| op(&libs.rust));
    if from_c != from_rust {
        panic!(
            "DIVERGENCE [{label}]\n  C    ({:>4} bytes): {}\n  Rust ({:>4} bytes): {}",
            from_c.len(),
            show(&from_c),
            from_rust.len(),
            show(&from_rust),
        );
    }
}

/// Public wrapper around [`capture`] that takes the capture lock itself, for
/// tests that want to inspect one side's bytes directly (e.g. to pin the exact
/// error message the C library emits).
pub fn capture_bytes<F: FnOnce()>(f: F) -> Vec<u8> {
    let _g = capture_guard();
    capture(f)
}

fn show(bytes: &[u8]) -> String {
    let head: Vec<u8> = bytes.iter().copied().take(300).collect();
    let mut s = String::new();
    for b in &head {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            0x20..=0x7e => s.push(*b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > head.len() {
        s.push_str(&format!("... (+{} bytes)", bytes.len() - head.len()));
    }
    format!("\"{s}\"")
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed, reproducible across runs.
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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
    /// Uniform over `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// An arbitrary `f32` **bit pattern** — includes NaNs, infinities and
    /// subnormals, all of which are legal inputs across the FFI boundary.
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
    /// Log-uniform magnitude in `[lo, hi]` with a random sign.
    pub fn log_uniform_f32(&mut self, lo: f64, hi: f64) -> f32 {
        let t = self.unit();
        let mag = (lo.ln() + t * (hi.ln() - lo.ln())).exp();
        let v = if self.next_u64() & 1 == 0 { mag } else { -mag };
        v as f32
    }
}

/// Seed used by every property-style test, so failures reproduce exactly.
pub const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------------------
// Fixed input sets reused by several rows.
// ---------------------------------------------------------------------------

/// The degenerate / boundary `f32` values the C code distinguishes.
pub fn special_f32() -> Vec<f32> {
    vec![
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FA0_0000), // signalling NaN
        f32::from_bits(0xFFA0_0000), // negative signalling NaN
        f32::from_bits(0x7FC0_0001), // quiet NaN, payload 1
        f32::from_bits(0x0000_0001), // smallest positive subnormal
        f32::from_bits(0x8000_0001), // smallest negative subnormal
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
        1.0,
        -1.0,
        2.0,
        -2.0,
        100.0,
        -100.0,
        1e-6,
        -1e-6,
        1e-7,
        -1e-7,
        5e-7,
        1e-5,
        0.000001_f64 as f32,
        // the cvttsd2si cliff: 100.0/data == 2^31 exactly
        (100.0f64 / 2147483648.0f64) as f32,
        (-100.0f64 / 2147483648.0f64) as f32,
        4.656_612_9e-8,
        -4.656_612_9e-8,
        // just above/below the goodB2G guard
        1.000_000_1e-6,
        -1.000_000_1e-6,
        9.999_999e-7,
    ]
}

/// Bytes/strings that stress `printf("%s\n", ...)`.
pub fn special_strings() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"a".to_vec(),
        b"Calling good()...".to_vec(),
        b"This would result in a divide by zero".to_vec(),
        b"%s".to_vec(),
        b"%d %d %d".to_vec(),
        b"%n".to_vec(),
        b"%%".to_vec(),
        b"100%".to_vec(),
        b"tab\there".to_vec(),
        b"cr\rhere".to_vec(),
        b"vt\x0bhere".to_vec(),
        b"embedded\nnewline".to_vec(),
        vec![0x80, 0xff, 0xfe, 0xc3, 0x28],
        (0x01u8..=0xffu8).collect(),
    ];
    // stdio buffer crossings
    for n in [1usize, 2, 79, 80, 4095, 4096, 4097, 65536] {
        v.push(vec![b'x'; n]);
    }
    v
}
