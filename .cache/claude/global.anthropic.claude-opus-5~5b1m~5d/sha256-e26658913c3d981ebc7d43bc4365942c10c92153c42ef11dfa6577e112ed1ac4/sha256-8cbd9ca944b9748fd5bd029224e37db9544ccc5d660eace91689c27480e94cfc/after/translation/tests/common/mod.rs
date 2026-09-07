//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared objects are loaded with `libloading` and driven **only** through
//! their exported C symbols — the Rust crate is never called directly, so the
//! `#[no_mangle] extern "C"` wrappers are part of what is under test.

#![allow(dead_code)]

use std::ffi::{c_char, CString};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

pub type NullaryFn = unsafe extern "C" fn();
pub type PrintLineFn = unsafe extern "C" fn(*const c_char);

/// fd 1 is process-wide, so stdout capture must be serialized across the
/// (multi-threaded) test harness.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The C reference `.so`, built by `c_src/CMakeLists.txt`.
pub fn c_lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let path = manifest_dir().join("../c_src/build/libdriver.so");
        assert!(
            path.exists(),
            "C shared library missing at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            path.display()
        );
        unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    })
}

/// The Rust `cdylib` under test. Loaded through `dlopen` exactly as an external
/// consumer would, never linked in directly.
pub fn rust_lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let candidates: Vec<PathBuf> = match std::env::var_os("DRIVER_RUST_SO") {
            Some(p) => vec![PathBuf::from(p)],
            None => vec![
                manifest_dir().join("target/release/libdriver.so"),
                manifest_dir().join("target/debug/libdriver.so"),
            ],
        };
        let path = candidates
            .iter()
            .find(|p| p.exists())
            .unwrap_or_else(|| {
                panic!(
                    "Rust shared library missing; tried {:?}.\nBuild it with: cargo build --release",
                    candidates
                )
            })
            .clone();
        unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", path.display()))
    })
}

/// dlsym a nullary `void(void)` export.
pub fn nullary<'a>(lib: &'a Library, name: &str) -> Symbol<'a, NullaryFn> {
    let mut sym = name.as_bytes().to_vec();
    sym.push(0);
    unsafe { lib.get::<NullaryFn>(&sym) }
        .unwrap_or_else(|e| panic!("symbol `{name}` not exported: {e}"))
}

/// dlsym `void printLine(const char *)`.
pub fn print_line<'a>(lib: &'a Library) -> Symbol<'a, PrintLineFn> {
    unsafe { lib.get::<PrintLineFn>(b"printLine\0") }
        .unwrap_or_else(|e| panic!("symbol `printLine` not exported: {e}"))
}

/// Run `f` with fd 1 redirected to a temp file and return the exact bytes
/// written. `fflush(NULL)` on both sides pins down the libc `stdout` buffer that
/// the C `.so` and the Rust `.so` share (both import the same `puts@GLIBC`).
pub fn capture(f: impl FnOnce()) -> Vec<u8> {
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let _guard = capture_lock();

    // fd 1 is process-wide, so libtest's own progress text ("test foo ... ok")
    // would otherwise land inside our capture file. Two things prevent that:
    //
    //  1. libtest writes progress to the shared `io::Stdout` LineWriter, and
    //     "test foo ... " has no trailing newline, so it sits in that buffer.
    //     Flushing Rust's stdout/stderr here pushes it out to the *real* fd 1
    //     before we redirect.
    //  2. A concurrently-running test could still write progress while fd 1 is
    //     redirected, so the suite must run single-threaded.
    assert_eq!(
        std::env::var("RUST_TEST_THREADS").ok().as_deref(),
        Some("1"),
        "stdout capture requires single-threaded tests; run with \
         `cargo test -- --test-threads=1` (translation/.cargo/config.toml sets \
         RUST_TEST_THREADS=1 automatically)"
    );
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
    }

    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver_capture_{}_{}.out",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));

    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup(1) failed");

    {
        let file = std::fs::File::create(&path).expect("create capture file");
        assert!(
            unsafe { libc::dup2(file.as_raw_fd(), 1) } >= 0,
            "dup2 onto stdout failed"
        );
    }

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    unsafe {
        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);
    }

    let out = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);

    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
    out
}

/// Assert two captures are byte-identical, with a readable diff on failure.
#[track_caller]
pub fn assert_same(case: &str, c_out: &[u8], rust_out: &[u8]) {
    if c_out != rust_out {
        panic!(
            "DIVERGENCE in {case}\n  C    ({} bytes): {:?}\n  Rust ({} bytes): {:?}\n  first differing byte at index {:?}",
            c_out.len(),
            String::from_utf8_lossy(c_out),
            rust_out.len(),
            String::from_utf8_lossy(rust_out),
            c_out.iter().zip(rust_out.iter()).position(|(a, b)| a != b),
        );
    }
}

/// Call `printLine` in both libraries with the same NUL-terminated payload and
/// compare stdout byte-for-byte.
#[track_caller]
pub fn diff_print_line(case: &str, payload: &[u8]) {
    let cs = CString::new(payload).expect("payload must not contain an interior NUL");
    let ptr = cs.as_ptr();

    let c_fn = print_line(c_lib());
    let r_fn = print_line(rust_lib());

    let c_out = capture(|| unsafe { c_fn(ptr) });
    let r_out = capture(|| unsafe { r_fn(ptr) });
    assert_same(case, &c_out, &r_out);

    // The C emits the payload verbatim plus a single '\n'; pin that down too so
    // a bug that is symmetric across both libraries cannot hide.
    let mut expected = payload.to_vec();
    expected.push(b'\n');
    assert_same(&format!("{case} (vs. expected literal)"), &expected, &c_out);
}

/// Call a nullary export in both libraries and compare stdout.
#[track_caller]
pub fn diff_nullary(name: &str) -> Vec<u8> {
    let c_fn = nullary(c_lib(), name);
    let r_fn = nullary(rust_lib(), name);
    let c_out = capture(|| unsafe { c_fn() });
    let r_out = capture(|| unsafe { r_fn() });
    assert_same(name, &c_out, &r_out);
    c_out
}

/// Deterministic xorshift64* PRNG — fixed seed keeps every row reproducible.
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
    /// Uniform-ish in `[lo, hi]`.
    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(lo <= hi);
        lo + (self.next_u64() as usize) % (hi - lo + 1)
    }
    /// A byte in `0x01..=0xFF` (never a NUL — that would terminate the string).
    pub fn nonzero_byte(&mut self) -> u8 {
        1 + (self.next_u64() % 255) as u8
    }
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.range(0, items.len() - 1)]
    }
}

/// The fixed seed used by every randomized row.
pub const SEED: u64 = 0x2026_09_05;
