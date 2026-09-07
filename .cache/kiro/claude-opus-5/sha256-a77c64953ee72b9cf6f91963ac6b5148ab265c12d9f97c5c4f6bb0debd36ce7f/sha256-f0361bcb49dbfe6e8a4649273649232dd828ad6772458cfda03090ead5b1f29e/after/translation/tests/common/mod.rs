//! Shared differential-test harness.
//!
//! Loads BOTH shared objects through `libloading` and calls only their exported
//! `extern "C"` symbols — the Rust crate is never called directly, so the
//! `#[no_mangle]` export wrapper is part of what gets tested.
//!
//! `driver` returns `void` and communicates solely through `printf` on
//! `stdout`, so the differential observation is the exact byte sequence written
//! to file descriptor 1 by one call. We capture it in-process by temporarily
//! `dup2`-ing a scratch file over fd 1; both `.so`s share the process's libc
//! `stdout`, so the capture is symmetric for C and Rust.

#![allow(dead_code)]

use std::ffi::c_int;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open C output stream.
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

pub type DriverFn = unsafe extern "C" fn(c_int);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `c_src/build/libdriver.so`, built by CMake.
pub fn c_so_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libdriver.so")
}

/// `translation/target/<profile>/libdriver.so`, built by cargo.
///
/// The integration-test binary lives in `target/<profile>/deps/`, so the cdylib
/// is one directory up from it. Falling back to both profile directories keeps
/// the tests working under `cargo test` and `cargo test --release`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(explicit) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(explicit);
    }
    let exe = std::env::current_exe().expect("test exe path");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    for p in ["release", "debug"] {
        let alt = manifest_dir().join("target").join(p).join("libdriver.so");
        if alt.exists() {
            return alt;
        }
    }
    candidate
}

/// Fails loudly if a `.so` is older than a source it is built from.
///
/// This guard is essential here: `cargo test` compiles the lib target as a unit
/// test binary but does **not** rebuild the `cdylib` artifact, so without this
/// check the differential tests can silently pass against a stale
/// `libdriver.so` (verified: a deliberate mutation of `src/lib.rs` went
/// undetected until this guard was added). Always run
/// `cargo build --release` before `cargo test --release`.
fn assert_not_stale(so: &std::path::Path, sources: &[PathBuf]) {
    let so_mtime = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .expect("stat .so");
    for src in sources {
        if let Ok(src_mtime) = std::fs::metadata(src).and_then(|m| m.modified()) {
            assert!(
                src_mtime <= so_mtime,
                "STALE ARTIFACT: {} is newer than {}.\n\
                 `cargo test` does not rebuild the cdylib — rebuild first:\n  \
                 cd translation && cargo build --release   (or: cd c_src/build && cmake --build .)",
                src.display(),
                so.display()
            );
        }
    }
}

struct Libs {
    c: DriverFn,
    rust: DriverFn,
    // Keep the handles alive for the whole process.
    _c_lib: &'static Library,
    _rust_lib: &'static Library,
}

// The function pointers are plain code addresses in libraries that are never
// unloaded, so sharing them across threads is sound.
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

static LIBS: OnceLock<Libs> = OnceLock::new();

fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );
        assert!(
            rust_path.exists(),
            "Rust shared library not found at {}. Build it with:\n  cd translation && cargo build --release",
            rust_path.display()
        );
        let root = manifest_dir().parent().expect("workspace root").to_path_buf();
        assert_not_stale(
            &rust_path,
            &[
                manifest_dir().join("src/lib.rs"),
                manifest_dir().join("Cargo.toml"),
            ],
        );
        assert_not_stale(
            &c_path,
            &[
                root.join("c_src/src/driver.c"),
                root.join("c_src/include/driver.h"),
            ],
        );
        unsafe {
            let c_lib: &'static Library = Box::leak(Box::new(
                Library::new(&c_path).expect("dlopen C libdriver.so"),
            ));
            let rust_lib: &'static Library = Box::leak(Box::new(
                Library::new(&rust_path).expect("dlopen Rust libdriver.so"),
            ));
            let c_sym: Symbol<DriverFn> =
                c_lib.get(b"driver\0").expect("C .so exports `driver`");
            let rust_sym: Symbol<DriverFn> = rust_lib
                .get(b"driver\0")
                .expect("Rust .so exports `driver` (check #[no_mangle])");
            Libs {
                c: *c_sym,
                rust: *rust_sym,
                _c_lib: c_lib,
                _rust_lib: rust_lib,
            }
        }
    })
}

/// Serializes fd-1 redirection across the (possibly parallel) test threads.
static FD_LOCK: Mutex<()> = Mutex::new(());
static SEQ: AtomicU64 = AtomicU64::new(0);

/// Runs `f` with fd 1 redirected to a scratch file and returns the bytes written.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = FD_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("driver_capture_{}_{}.txt", std::process::id(), n));
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create scratch capture file");

    // Drain anything already buffered on the Rust and C sides so it does not
    // land in the capture file.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

    f();

    // Flush what the library wrote before restoring fd 1.
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "restoring fd 1 failed");
    unsafe { close(saved) };

    let mut out = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("rewind capture file");
    file.read_to_end(&mut out).expect("read capture file");
    drop(file);
    let _ = std::fs::remove_file(&path);
    out
}

/// Address of the C `.so`'s `driver`, for the "distinct implementations" guard.
pub fn c_fn_addr() -> usize {
    libs().c as usize
}

/// Address of the Rust `.so`'s `driver`.
pub fn rust_fn_addr() -> usize {
    libs().rust as usize
}

/// Public wrapper around the fd-1 capture, for tests that load the `.so`s
/// themselves (e.g. to call `driver` through a differently-typed ABI view).
pub fn capture_pub<F: FnOnce()>(f: F) -> Vec<u8> {
    capture(f)
}

/// Calls the C `.so`'s `driver(x)` and returns its stdout bytes.
pub fn c_driver(x: c_int) -> Vec<u8> {
    let f = libs().c;
    capture(|| unsafe { f(x) })
}

/// Calls the Rust `.so`'s `driver(x)` and returns its stdout bytes.
pub fn rust_driver(x: c_int) -> Vec<u8> {
    let f = libs().rust;
    capture(|| unsafe { f(x) })
}

/// Calls the C `.so`'s `driver` once per input inside a single capture session.
pub fn c_driver_batch(xs: &[c_int]) -> Vec<u8> {
    let f = libs().c;
    capture(|| {
        for &x in xs {
            unsafe { f(x) }
        }
    })
}

/// Calls the Rust `.so`'s `driver` once per input inside a single capture session.
pub fn rust_driver_batch(xs: &[c_int]) -> Vec<u8> {
    let f = libs().rust;
    capture(|| {
        for &x in xs {
            unsafe { f(x) }
        }
    })
}

/// Differential assertion for one input: both `.so`s must emit identical bytes.
#[track_caller]
pub fn assert_same(label: &str, x: c_int) {
    let c = c_driver(x);
    let r = rust_driver(x);
    assert_eq!(
        c,
        r,
        "[{label}] driver({x}) diverged:\n  C    = {:?}\n  Rust = {:?}",
        String::from_utf8_lossy(&c),
        String::from_utf8_lossy(&r)
    );
    // Sanity: the C library always prints exactly one newline-terminated line.
    assert!(
        c.ends_with(b"\n") && c.iter().filter(|&&b| b == b'\n').count() == 1,
        "[{label}] driver({x}) C output is not a single line: {:?}",
        String::from_utf8_lossy(&c)
    );
}

/// Differential assertion for a whole batch of inputs (also catches ordering
/// and buffering differences across consecutive calls).
#[track_caller]
pub fn assert_same_batch(label: &str, xs: &[c_int]) {
    let c = c_driver_batch(xs);
    let r = rust_driver_batch(xs);
    if c != r {
        // Narrow down to the first diverging input for a useful message.
        for &x in xs {
            let cc = c_driver(x);
            let rr = rust_driver(x);
            assert_eq!(
                cc,
                rr,
                "[{label}] batch diverged, first bad input driver({x}):\n  C    = {:?}\n  Rust = {:?}",
                String::from_utf8_lossy(&cc),
                String::from_utf8_lossy(&rr)
            );
        }
        panic!("[{label}] batch outputs differ but every single call matched");
    }
    assert_eq!(
        c.iter().filter(|&&b| b == b'\n').count(),
        xs.len(),
        "[{label}] expected one output line per call"
    );
}

/// Deterministic PRNG (SplitMix64) — fixed seed, reproducible sequences.
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

    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Uniform in `[lo, hi]` inclusive, correct across the full `i32` range.
    pub fn in_range(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64) as u64 + 1;
        let v = self.next_u64() % span;
        (lo as i64 + v as i64) as i32
    }
}

/// Minimal sequential test runner used by the `harness = false` test targets.
///
/// Running strictly one test at a time on the main thread guarantees that no
/// other thread writes to fd 1 while a capture is in progress. Progress lines
/// are flushed before each test so they cannot end up in the capture file.
pub fn run_suite(suite: &str, tests: &[(&str, fn())]) {
    // Honour a name filter argument, like libtest does.
    let filter: Option<String> = std::env::args().skip(1).find(|a| !a.starts_with('-'));

    let selected: Vec<&(&str, fn())> = tests
        .iter()
        .filter(|(name, _)| filter.as_deref().is_none_or(|f| name.contains(f)))
        .collect();

    println!("\nrunning {} tests ({suite})", selected.len());
    let _ = std::io::stdout().flush();

    let mut failures: Vec<&str> = Vec::new();
    for (name, f) in selected.iter() {
        print!("test {name} ... ");
        let _ = std::io::stdout().flush();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(*f));
        match result {
            Ok(()) => println!("ok"),
            Err(_) => {
                println!("FAILED");
                failures.push(name);
            }
        }
        let _ = std::io::stdout().flush();
    }

    let passed = selected.len() - failures.len();
    if failures.is_empty() {
        println!("\ntest result: ok. {passed} passed; 0 failed\n");
    } else {
        println!(
            "\ntest result: FAILED. {passed} passed; {} failed: {:?}\n",
            failures.len(),
            failures
        );
        let _ = std::io::stdout().flush();
        std::process::exit(1);
    }
}
