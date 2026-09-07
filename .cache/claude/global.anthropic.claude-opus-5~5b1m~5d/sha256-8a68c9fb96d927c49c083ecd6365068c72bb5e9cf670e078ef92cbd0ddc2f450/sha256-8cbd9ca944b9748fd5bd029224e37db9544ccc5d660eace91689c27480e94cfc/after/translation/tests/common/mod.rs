// Each test crate uses a subset of this shared harness.
#![allow(dead_code)]

//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects with `libloading` and driven
//! exclusively through their exported `extern "C"` symbols, exactly as an
//! external consumer would. No Rust function is called directly.

use std::ffi::{c_char, c_int, c_void, CString};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits used only by the harness (stdout capture). These are *not* part of
// the library under test.
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    /// `fflush(NULL)` flushes every open output stream, which is how we make
    /// the libc `stdout` buffer observable before restoring the real fd.
    fn fflush(stream: *mut c_void) -> c_int;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;
const STDOUT_FILENO: c_int = 1;

/// Serialises stdout redirection: fd 1 is process-global state.
fn capture_lock() -> &'static Mutex<u64> {
    static LOCK: OnceLock<Mutex<u64>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(0))
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_lib_path() -> PathBuf {
    let p = repo_root().join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_lib_path() -> PathBuf {
    // Prefer release (what `cargo build --release` produces); fall back to debug.
    let root = repo_root().join("translation/target");
    let mut found = None;
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libdriver.so");
        if p.is_file() {
            found = Some(p);
            break;
        }
    }
    let p = found.unwrap_or_else(|| {
        panic!(
            "Rust cdylib not found under {}. Build it with:\n  cd translation && cargo build --release",
            root.display()
        )
    });

    // CRITICAL: `cargo test` does NOT rebuild the `cdylib` artifact, so without
    // this check the whole suite can silently pass against a stale `.so` (a
    // mutation of src/lib.rs would go undetected and the tests would be
    // vacuous). Refuse to run if the artifact is older than the sources.
    assert_fresher_than_sources(&p);
    p
}

fn assert_fresher_than_sources(artifact: &Path) {
    let so_mtime = std::fs::metadata(artifact)
        .and_then(|m| m.modified())
        .expect("mtime of Rust .so");

    let src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    let mut stack = vec![src_dir];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_some_and(|e| e == "rs") {
                if let Ok(m) = entry.metadata().and_then(|m| m.modified()) {
                    if newest.as_ref().is_none_or(|(_, t)| m > *t) {
                        newest = Some((path, m));
                    }
                }
            }
        }
    }

    if let Some((path, src_mtime)) = newest {
        assert!(
            so_mtime >= src_mtime,
            "STALE ARTIFACT: {} is older than {}.\n`cargo test` does not rebuild the cdylib, so \
             these differential tests would be testing an out-of-date library.\nRun:  cargo build \
             --release   (or use ./run_tests.sh) before `cargo test`.",
            artifact.display(),
            path.display()
        );
    }
}

/// `void driver(int)` as exported by both shared objects.
pub type DriverFn = unsafe extern "C" fn(c_int);

struct Libs {
    c: Library,
    rust: Library,
    c_driver: DriverFn,
    rust_driver: DriverFn,
}

// The raw fn pointers are plain code addresses; the `Library` handles stay
// alive for the whole process because they live in a `OnceLock`.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_lib_path()).expect("dlopen C libdriver.so");
        let rust = Library::new(rust_lib_path()).expect("dlopen Rust libdriver.so");

        let c_sym: Symbol<DriverFn> = c.get(b"driver\0").expect("C .so exports `driver`");
        let rust_sym: Symbol<DriverFn> =
            rust.get(b"driver\0").expect("Rust .so exports `driver`");

        let c_driver = *c_sym;
        let rust_driver = *rust_sym;

        // Guard against the dynamic loader de-duplicating the two objects
        // (both files are named libdriver.so): if that happened we would be
        // comparing one implementation against itself.
        assert_ne!(
            c_driver as usize, rust_driver as usize,
            "C and Rust `driver` resolved to the SAME address - the loader \
             de-duplicated the two libdriver.so files, so the differential \
             test would be vacuous"
        );

        Libs { c, rust, c_driver, rust_driver }
    })
}

pub fn c_driver() -> DriverFn {
    libs().c_driver
}

pub fn rust_driver() -> DriverFn {
    libs().rust_driver
}

/// Both `.so` handles must outlive every call; touching them here keeps the
/// fields from being reported as dead code.
pub fn assert_libs_loaded() {
    let l = libs();
    let _ = (&l.c, &l.rust);
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// Redirecting fd 1 is process-global, so a concurrently running test would
/// have its harness output ("test ... ok") swept into our capture. Refuse to
/// produce bogus results in that case.
fn assert_single_threaded() {
    let n = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
    assert_eq!(
        n, "1",
        "these differential tests capture the process-wide stdout fd and must run \
         serially; expected RUST_TEST_THREADS=1 (set by translation/.cargo/config.toml), \
         got {n:?}. Re-run with `cargo test -- --test-threads=1`."
    );
}

/// Runs `f`, capturing everything written to file descriptor 1 (including
/// output buffered inside libc's `stdout`) and returning the raw bytes.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let mut counter = capture_lock().lock().unwrap_or_else(|e| e.into_inner());
    *counter += 1;
    let tmp = std::env::temp_dir().join(format!(
        "driver_difftest_{}_{}.out",
        std::process::id(),
        *counter
    ));
    let tmp_c = CString::new(tmp.to_str().expect("utf-8 temp path")).unwrap();

    assert_single_threaded();

    unsafe {
        // Flush anything the harness itself has pending so it is not captured:
        // first Rust's own `std::io::stdout` buffer, then every libc stream.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        fflush(std::ptr::null_mut());

        let saved = dup(STDOUT_FILENO);
        assert!(saved >= 0, "dup(stdout) failed");

        let fd = open(tmp_c.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "open({}) failed", tmp.display());

        assert!(dup2(fd, STDOUT_FILENO) >= 0, "dup2 onto stdout failed");
        close(fd);

        f();

        // Make the library's buffered output reach the temp file before we
        // put the real stdout back.
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, STDOUT_FILENO) >= 0, "restoring stdout failed");
        close(saved);
    }

    let bytes = std::fs::read(&tmp).expect("read captured stdout");
    let _ = std::fs::remove_file(&tmp);
    bytes
}

/// Captures the stdout produced by calling `driver(x)` in the C library.
pub fn c_output(x: c_int) -> Vec<u8> {
    let f = c_driver();
    capture_stdout(|| unsafe { f(x) })
}

/// Captures the stdout produced by calling `driver(x)` in the Rust library.
pub fn rust_output(x: c_int) -> Vec<u8> {
    let f = rust_driver();
    capture_stdout(|| unsafe { f(x) })
}

/// Captures the stdout of a whole *sequence* of `driver` calls, so that
/// cross-call state and stdio buffer/flush ordering are also compared.
pub fn c_output_seq(xs: &[c_int]) -> Vec<u8> {
    let f = c_driver();
    capture_stdout(|| {
        for &x in xs {
            unsafe { f(x) }
        }
    })
}

pub fn rust_output_seq(xs: &[c_int]) -> Vec<u8> {
    let f = rust_driver();
    capture_stdout(|| {
        for &x in xs {
            unsafe { f(x) }
        }
    })
}

// ---------------------------------------------------------------------------
// Assertions
// ---------------------------------------------------------------------------

fn describe(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) if s.len() <= 400 => format!("{s:?}"),
        Ok(s) => format!("{:?}... ({} bytes)", &s[..400], s.len()),
        Err(_) => format!("{:?} ({} bytes, not utf-8)", &bytes[..bytes.len().min(64)], bytes.len()),
    }
}

fn first_diff(a: &[u8], b: &[u8]) -> String {
    let at = a.iter().zip(b.iter()).position(|(x, y)| x != y);
    match at {
        Some(i) => {
            let lo = i.saturating_sub(40);
            let hi_a = (i + 40).min(a.len());
            let hi_b = (i + 40).min(b.len());
            format!(
                "first difference at byte {i}\n  C    ...{}\n  Rust ...{}",
                String::from_utf8_lossy(&a[lo..hi_a]),
                String::from_utf8_lossy(&b[lo..hi_b])
            )
        }
        None => format!("common prefix identical; lengths differ: C={} Rust={}", a.len(), b.len()),
    }
}

/// Asserts C and Rust produce byte-identical stdout for `driver(x)`.
#[track_caller]
pub fn assert_same(x: c_int) {
    let c = c_output(x);
    let r = rust_output(x);
    assert!(
        c == r,
        "stdout mismatch for driver({x}):\n  C    = {}\n  Rust = {}\n{}",
        describe(&c),
        describe(&r),
        first_diff(&c, &r)
    );
}

/// Asserts C and Rust produce byte-identical stdout for a whole call sequence.
#[track_caller]
pub fn assert_same_seq(xs: &[c_int]) {
    let c = c_output_seq(xs);
    let r = rust_output_seq(xs);
    assert!(
        c == r,
        "stdout mismatch for driver sequence {xs:?}:\n  C    = {}\n  Rust = {}\n{}",
        describe(&c),
        describe(&r),
        first_diff(&c, &r)
    );
}

/// Asserts C and Rust agree AND that both produced exactly `expected`.
#[track_caller]
pub fn assert_same_and_eq(x: c_int, expected: &str) {
    assert_same(x);
    let c = c_output(x);
    assert_eq!(
        String::from_utf8_lossy(&c),
        expected,
        "C output for driver({x}) is not the expected reference text"
    );
}

/// Asserts C and Rust agree and that both printed nothing at all.
#[track_caller]
pub fn assert_same_and_empty(x: c_int) {
    let c = c_output(x);
    let r = rust_output(x);
    assert!(
        c == r,
        "stdout mismatch for driver({x}):\n  C    = {}\n  Rust = {}",
        describe(&c),
        describe(&r)
    );
    assert!(
        c.is_empty(),
        "expected driver({x}) to print nothing, C printed {}",
        describe(&c)
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed => reproducible property-style testing)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // Avoid the zero state of xorshift64*.
        Rng(seed | 1)
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

    /// Uniform-ish `i32` over the whole domain, including `INT_MIN`/`INT_MAX`.
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Inclusive range.
    pub fn range(&mut self, lo: i64, hi: i64) -> i64 {
        assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i64
    }
}
