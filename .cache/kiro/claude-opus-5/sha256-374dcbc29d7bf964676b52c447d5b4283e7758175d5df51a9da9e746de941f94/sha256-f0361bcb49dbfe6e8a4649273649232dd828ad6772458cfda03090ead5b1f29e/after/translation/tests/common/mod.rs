//! Shared differential-testing harness.
//!
//! Both the C `.so` (`c_src/build/libStaticLoop.so`) and the Rust `.so`
//! (`translation/target/{release,debug}/libStaticLoop.so`) are loaded with
//! `libloading` and driven *only* through their exported symbols — the Rust
//! implementation is never called directly, so the `#[no_mangle] extern "C"`
//! wrappers are part of what is under test.
//!
//! ## Why every call is mirrored
//!
//! `static_sum` keeps a function-local `static int sum` (one instance per loaded
//! shared object). The C library and the Rust library therefore own *separate*
//! accumulators, and their outputs are only comparable if both see the exact
//! same call history. `Harness` enforces that: every public method applies the
//! operation to C and to Rust, under one global lock, so the two accumulators
//! stay in lock-step regardless of test ordering or parallelism.

#![allow(dead_code)]

use std::ffi::c_int;
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

type SumFn = unsafe extern "C" fn(c_int) -> c_int;
type DriverFn = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open C output streams, which is what makes
    /// the `printf` output of a freshly-redirected fd 1 observable.
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

/// One loaded implementation.
struct Impl {
    // Kept alive for the lifetime of the process; dropping it would `dlclose`
    // and reset the accumulator.
    _lib: libloading::Library,
    sum: SumFn,
    driver: DriverFn,
    name: &'static str,
}

impl Impl {
    fn load(path: &PathBuf, name: &'static str) -> Impl {
        let lib = unsafe { libloading::Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({name}): {e}", path.display()));
        let sum: SumFn = unsafe {
            *lib.get::<SumFn>(b"static_sum\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `static_sum`: {e}"))
        };
        let driver: DriverFn = unsafe {
            *lib.get::<DriverFn>(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol `driver`: {e}"))
        };
        Impl { _lib: lib, sum, driver, name }
    }
}

pub struct Harness {
    c: Impl,
    rust: Impl,
}

static HARNESS: OnceLock<Mutex<Harness>> = OnceLock::new();

/// Locks the process-wide harness. All operations must go through it so the two
/// accumulators observe identical call histories.
pub fn harness() -> MutexGuard<'static, Harness> {
    HARNESS
        .get_or_init(|| Mutex::new(Harness::new()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_LIB_PATH") {
        return PathBuf::from(p);
    }
    manifest_dir().join("../c_src/build/libStaticLoop.so")
}

fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_LIB_PATH") {
        return PathBuf::from(p);
    }
    let release = manifest_dir().join("target/release/libStaticLoop.so");
    if release.exists() {
        return release;
    }
    manifest_dir().join("target/debug/libStaticLoop.so")
}

impl Harness {
    fn new() -> Harness {
        Harness {
            c: Impl::load(&c_lib_path(), "C"),
            rust: Impl::load(&rust_lib_path(), "Rust"),
        }
    }

    /// Calls `static_sum(update)` in both libraries and asserts the returned
    /// values are identical. Returns the (shared) value.
    #[track_caller]
    pub fn static_sum(&mut self, update: i32) -> i32 {
        let c = unsafe { (self.c.sum)(update as c_int) } as i32;
        let r = unsafe { (self.rust.sum)(update as c_int) } as i32;
        assert_eq!(
            c, r,
            "static_sum({update}) diverged: C returned {c}, Rust returned {r}"
        );
        c
    }

    /// Calls `driver(stride)` in both libraries with fd 1 redirected to a
    /// temporary file, and asserts the produced stdout bytes are identical.
    /// Returns the (shared) bytes.
    #[track_caller]
    pub fn driver(&mut self, stride: i32) -> Vec<u8> {
        let c = capture_stdout(|| unsafe { (self.c.driver)(stride as c_int) });
        let r = capture_stdout(|| unsafe { (self.rust.driver)(stride as c_int) });
        assert_eq!(
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r),
            "driver({stride}) stdout diverged"
        );
        assert_eq!(c, r, "driver({stride}) stdout bytes diverged");
        c
    }

    /// Reads the accumulator without changing it (`update = 0` is the identity).
    #[track_caller]
    pub fn peek(&mut self) -> i32 {
        self.static_sum(0)
    }

    /// Direct, unmirrored access — only used by the "very first call" test.
    #[track_caller]
    pub fn raw_first_call(&mut self, update: i32) -> (i32, i32) {
        let c = unsafe { (self.c.sum)(update as c_int) } as i32;
        let r = unsafe { (self.rust.sum)(update as c_int) } as i32;
        (c, r)
    }
}

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Runs `f` with file descriptor 1 pointed at a temporary file and returns
/// everything written to it (including output produced by libc `printf` inside
/// a dynamically loaded library).
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let n = TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "staticloop-diff-{}-{}-{}.out",
        std::process::id(),
        n,
        std::thread::current().id().discriminator()
    ));

    let file = fs::File::create(&path).expect("create temp capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

    f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "restoring fd 1 failed");
    unsafe { close(saved) };
    drop(file);

    let bytes = fs::read(&path).expect("read temp capture file");
    let _ = fs::remove_file(&path);
    bytes
}

/// Reference model of the C implementation, used as a third opinion so that a
/// shared misunderstanding between the two `.so`s cannot pass silently.
pub struct Model {
    pub sum: i32,
}

impl Model {
    pub fn new(sum: i32) -> Model {
        Model { sum }
    }
    pub fn static_sum(&mut self, update: i32) -> i32 {
        self.sum = self.sum.wrapping_add(update);
        self.sum
    }
    pub fn driver(&mut self, stride: i32) -> Vec<u8> {
        let mut out = Vec::new();
        for i in 0..10i32 {
            let v = self.static_sum(i.wrapping_mul(stride));
            out.extend_from_slice(format!("{v}\n").as_bytes());
        }
        out
    }
}

/// Deterministic xorshift64* PRNG — fixed seed per test for reproducibility.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// Helper so the temp-file name can include a per-thread discriminator
/// without depending on unstable `ThreadId::as_u64`.
trait ThreadIdExt {
    fn discriminator(&self) -> u64;
}

impl ThreadIdExt for std::thread::ThreadId {
    fn discriminator(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut h);
        h.finish()
    }
}
