//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH libraries are loaded as shared objects through `libloading` and called
//! only through their exported `extern "C"` symbols. The Rust crate is never
//! called directly, so the `#[no_mangle]` export wrappers are under test too.
//!
//! `driver` communicates exclusively by writing to the process's libc `stdout`,
//! so the comparison is done by redirecting file descriptor 1 to a temporary
//! file around each call and diffing the raw bytes.

#![allow(dead_code)]

use std::ffi::{c_int, c_void};
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits needed for fd-level stdout capture
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream, which is how we make
    /// sure both libraries' buffered `printf` output has reached fd 1 before we
    /// read it back.
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FD: c_int = 1;

/// fd 1 is process-global, so captures must never run concurrently.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Runs `f` with fd 1 redirected to a scratch file and returns everything that
/// was written to it.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock();

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver-diff-{}-{:?}-{}.out",
        std::process::id(),
        std::thread::current().id(),
        COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));

    let file = std::fs::File::create(&path).expect("create scratch file");
    let tmp_fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    unsafe {
        // Flush anything already pending so it does not land in our capture.
        fflush(std::ptr::null_mut());
        let saved = dup(STDOUT_FD);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(tmp_fd, STDOUT_FD) >= 0, "dup2 onto 1 failed");

        f();

        // Flush the library's buffered printf output into the scratch file.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, STDOUT_FD) >= 0, "dup2 restore failed");
        close(saved);
    }
    drop(file);

    let mut bytes = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen scratch file")
        .read_to_end(&mut bytes)
        .expect("read scratch file");
    let _ = std::fs::remove_file(&path);
    bytes
}

static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let target = manifest_dir().join("target");
    // Prefer the release artifact, fall back to debug.
    for profile in ["release", "debug"] {
        let p = target.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust shared library not found under {target:?}. Build it with:\n  \
         cd translation && cargo build --release"
    );
}

/// The two loaded libraries, kept alive for the whole test binary.
pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("dlopen C libdriver.so");
        let rust = Library::new(rust_so_path()).expect("dlopen Rust libdriver.so");
        Libs { c, rust }
    })
}

/// `void driver(int)` as exported by each `.so`.
pub type DriverFn = unsafe extern "C" fn(c_int);

/// The same symbol viewed through a deliberately *wider* parameter type, used to
/// push out-of-range bit patterns across the FFI boundary (a C `int` parameter,
/// like a C enum, accepts whatever the ABI register holds).
pub type DriverFnWide = unsafe extern "C" fn(i64);

pub fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("dlsym driver in C .so") }
}

pub fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0").expect("dlsym driver in Rust .so") }
}

pub fn c_driver_wide() -> Symbol<'static, DriverFnWide> {
    unsafe { libs().c.get(b"driver\0").expect("dlsym driver in C .so") }
}

pub fn rust_driver_wide() -> Symbol<'static, DriverFnWide> {
    unsafe { libs().rust.get(b"driver\0").expect("dlsym driver in Rust .so") }
}

// ---------------------------------------------------------------------------
// Differential comparison helpers
// ---------------------------------------------------------------------------

fn render(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => format!("{s:?}"),
        Err(_) => format!("{bytes:02x?}"),
    }
}

/// Calls `driver(x)` in the C `.so` and in the Rust `.so`, each with its own
/// stdout capture, and asserts the captured bytes are identical.
///
/// Returns the (shared) output so callers can make extra structural assertions.
pub fn assert_same(row: &str, x: c_int) -> Vec<u8> {
    let cf = c_driver();
    let rf = rust_driver();
    let c_out = capture_stdout(|| unsafe { cf(x) });
    let r_out = capture_stdout(|| unsafe { rf(x) });
    assert_eq!(
        c_out,
        r_out,
        "[{row}] driver({x}) (0x{:08x}) diverged:\n  C    = {}\n  Rust = {}",
        x as u32,
        render(&c_out),
        render(&r_out)
    );
    c_out
}

/// Same as [`assert_same`] but over a whole batch of inputs in ONE capture, so
/// the full concatenated stdout stream (record framing included) is compared.
pub fn assert_same_batch(row: &str, xs: &[c_int]) -> Vec<u8> {
    let cf = c_driver();
    let rf = rust_driver();
    let c_out = capture_stdout(|| {
        for &x in xs {
            unsafe { cf(x) }
        }
    });
    let r_out = capture_stdout(|| {
        for &x in xs {
            unsafe { rf(x) }
        }
    });
    assert_eq!(
        c_out,
        r_out,
        "[{row}] batch of {} inputs diverged (first inputs: {:?})\n  C    = {}\n  Rust = {}",
        xs.len(),
        &xs[..xs.len().min(8)],
        render(&c_out),
        render(&r_out)
    );
    c_out
}

/// Pushes an out-of-range / oversized bit pattern through the `int` parameter.
pub fn assert_same_wide(row: &str, v: i64) -> Vec<u8> {
    let cf = c_driver_wide();
    let rf = rust_driver_wide();
    let c_out = capture_stdout(|| unsafe { cf(v) });
    let r_out = capture_stdout(|| unsafe { rf(v) });
    assert_eq!(
        c_out,
        r_out,
        "[{row}] driver(<wide {v} / 0x{:016x}>) diverged:\n  C    = {}\n  Rust = {}",
        v as u64,
        render(&c_out),
        render(&r_out)
    );
    c_out
}

// ---------------------------------------------------------------------------
// Deterministic RNG (splitmix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_EF01;

pub struct Rng(u64);

impl Rng {
    pub fn new(stream: u64) -> Self {
        Rng(SEED ^ stream.wrapping_mul(0x9E37_79B9_7F4A_7C15))
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
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}

/// Builds an `i32` from four explicit little-endian-order bytes
/// (`b[0]` is the byte at the lowest address, i.e. the one printed first).
pub fn from_le_bytes(b: [u8; 4]) -> c_int {
    i32::from_le_bytes(b)
}
