//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects with `libloading` and driven
//! only through their exported `driver` symbol, exactly as an external C caller
//! would. The Rust functions are never called directly, so the
//! `#[no_mangle] extern "C"` export wrapper is under test too.
//!
//! `driver` communicates solely by writing to `stdout`, so the harness captures
//! file descriptor 1 around each invocation. Both `.so`s resolve `printf` /
//! `putchar` to the process-wide `libc.so.6`, hence they share one glibc
//! `stdout` `FILE`; redirecting fd 1 therefore captures both identically.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

pub type DriverFn = unsafe extern "C" fn(c_int, c_int);

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    /// `fflush(NULL)` drains every open output stream, which is what we need in
    /// order to see the C library's buffered bytes.
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

/// fd 1 is process-global state, so every capture must be serialised.
fn fd_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libdriver.so")
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // Prefer the release cdylib, fall back to the debug one.
    let release = manifest_dir().join("target/release/libdriver.so");
    if release.exists() {
        return release;
    }
    manifest_dir().join("target/debug/libdriver.so")
}

struct Libs {
    c: Library,
    rust: Library,
}

// `libloading::Library` is `Send + Sync`; the loaded `driver` has no internal
// state of its own (all state lives in the shared glibc `stdout`).
unsafe impl Send for Libs {}
unsafe impl Sync for Libs {}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let cp = c_so_path();
        let rp = rust_so_path();
        assert!(cp.exists(), "C shared object not found at {}", cp.display());
        assert!(
            rp.exists(),
            "Rust shared object not found at {} (run `cargo build --release`)",
            rp.display()
        );
        unsafe {
            Libs {
                c: Library::new(&cp)
                    .unwrap_or_else(|e| panic!("loading {}: {e}", cp.display())),
                rust: Library::new(&rp)
                    .unwrap_or_else(|e| panic!("loading {}: {e}", rp.display())),
            }
        }
    })
}

/// The C library's exported `driver`.
pub fn c_driver() -> DriverFn {
    static F: OnceLock<usize> = OnceLock::new();
    let addr = *F.get_or_init(|| unsafe {
        let s: Symbol<DriverFn> = libs()
            .c
            .get(b"driver\0")
            .expect("symbol `driver` missing from the C .so");
        *s.into_raw() as usize
    });
    unsafe { std::mem::transmute::<usize, DriverFn>(addr) }
}

/// The Rust library's exported `driver` (the `#[no_mangle]` wrapper).
pub fn rust_driver() -> DriverFn {
    static F: OnceLock<usize> = OnceLock::new();
    let addr = *F.get_or_init(|| unsafe {
        let s: Symbol<DriverFn> = libs()
            .rust
            .get(b"driver\0")
            .expect("symbol `driver` missing from the Rust .so");
        *s.into_raw() as usize
    });
    unsafe { std::mem::transmute::<usize, DriverFn>(addr) }
}

/// Run `f` with fd 1 redirected into a fresh regular file; return the bytes
/// written.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = fd_lock();
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "driver-diff-{}-{}.out",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));

    unsafe { fflush(std::ptr::null_mut()) };
    // libtest writes its own progress through Rust's `stdout`, which is
    // line-buffered and therefore holds a partial line ("test foo ... ") while
    // the test runs. Drain it too, or it lands in our capture file.
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let file = std::fs::File::create(&path).expect("create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Same as [`capture`], but fd 1 is a *pipe* rather than a regular file, so
/// glibc selects a different buffering mode. Only for small outputs (a pipe
/// holds ~64 KiB before blocking).
pub fn capture_via_pipe<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = fd_lock();
    let mut fds = [0 as c_int; 2];
    assert!(unsafe { pipe(fds.as_mut_ptr()) } == 0, "pipe() failed");
    let (rd, wr) = (fds[0], fds[1]);

    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(wr, 1) } >= 0, "dup2 failed");

    f();

    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe {
        close(saved);
        close(wr);
    }

    let mut buf = Vec::new();
    let mut reader = unsafe { <std::fs::File as std::os::unix::io::FromRawFd>::from_raw_fd(rd) };
    reader.read_to_end(&mut buf).expect("read from pipe");
    buf
}

fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:x?}"),
    }
}

/// Compare one `(x, y)` invocation between the two libraries.
pub fn compare_one(row: &str, x: i32, y: i32) {
    let c = capture(|| unsafe { c_driver()(x, y) });
    let r = capture(|| unsafe { rust_driver()(x, y) });
    assert_eq!(
        c,
        r,
        "{row}: divergence at x={x} (0x{x:08x}), y={y} (0x{y:08x}): C={} Rust={}",
        show(&c),
        show(&r)
    );
}

/// Compare a whole batch of invocations replayed back to back into one output
/// stream (this also checks call-to-call concatenation and flush behaviour).
/// On divergence, bisects down to the offending pair.
pub fn compare_batch(row: &str, pairs: &[(i32, i32)]) {
    assert!(!pairs.is_empty(), "{row}: empty pair list");
    let c = capture(|| {
        let f = c_driver();
        for &(x, y) in pairs {
            unsafe { f(x, y) };
        }
    });
    let r = capture(|| {
        let f = rust_driver();
        for &(x, y) in pairs {
            unsafe { f(x, y) };
        }
    });
    if c == r {
        return;
    }
    // Narrow the failure down to a single input before reporting.
    for &(x, y) in pairs {
        compare_one(row, x, y);
    }
    let at = c
        .iter()
        .zip(r.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| c.len().min(r.len()));
    panic!(
        "{row}: batched streams differ at byte {at} (C len {}, Rust len {}) \
         although every individual call matched -- flush/ordering divergence.\n\
         C   tail: {}\n Rust tail: {}",
        c.len(),
        r.len(),
        show(&c[at.saturating_sub(16)..(at + 16).min(c.len())]),
        show(&r[at.saturating_sub(16)..(at + 16).min(r.len())]),
    );
}

/// Fixed-seed xorshift64* PRNG: reproducible across runs and platforms.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E3779B97F4A7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Non-negative `i32` in `[0, i32::MAX]`.
    pub fn next_nonneg(&mut self) -> i32 {
        self.next_i32() & i32::MAX
    }
    /// Negative `i32` in `[i32::MIN, -1]`.
    pub fn next_neg(&mut self) -> i32 {
        self.next_i32() | i32::MIN
    }
}

/// The nine most interesting extreme values of `int`.
pub const EXTREMES: [i32; 9] = [
    i32::MIN,
    i32::MIN + 1,
    -2,
    -1,
    0,
    1,
    2,
    i32::MAX - 1,
    i32::MAX,
];
