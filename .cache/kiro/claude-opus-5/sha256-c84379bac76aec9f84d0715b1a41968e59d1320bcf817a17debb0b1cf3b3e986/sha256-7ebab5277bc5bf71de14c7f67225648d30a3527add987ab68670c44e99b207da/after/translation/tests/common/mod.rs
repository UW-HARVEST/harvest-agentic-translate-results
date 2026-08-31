//! Shared plumbing for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! are only ever reached through their exported C symbols, so the `#[no_mangle]`
//! wrappers are part of what is under test.

// This module is compiled into every integration test binary, and each one uses
// only part of it.
#![allow(dead_code)]

use std::ffi::{c_char, c_int};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

pub type FmaArrayFn =
    unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);
pub type CallFmaFn = unsafe extern "C" fn(*const c_int, c_int) -> c_int;
pub type DriverFn = unsafe extern "C" fn(*const c_char);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared library produced by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().parent().unwrap().to_path_buf();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/Release/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
}

/// Path to the Rust `cdylib`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // Walk up from the test executable: target/<profile>/deps/<test> -> target/<profile>
    if let Ok(exe) = std::env::current_exe() {
        if let Some(profile_dir) = exe.parent().and_then(|p| p.parent()) {
            let c = profile_dir.join("libdriver.so");
            if c.is_file() {
                return c;
            }
        }
    }
    let target = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let c = target.join(profile).join("libdriver.so");
        if c.is_file() {
            return c;
        }
    }
    panic!("Rust cdylib not found; run `cargo build --release` in translation/ first");
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

/// Both libraries are opened once per test process. `libloading` uses
/// `RTLD_LOCAL`, so the identically named symbols in the two objects do not
/// interpose on each other.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        let c = unsafe { Library::new(&c_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", c_path.display()));
        let rust = unsafe { Library::new(&rust_path) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", rust_path.display()));
        Libs { c, rust }
    })
}

pub fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    let mut bytes = name.as_bytes().to_vec();
    bytes.push(0);
    unsafe { lib.get::<T>(&bytes) }
        .unwrap_or_else(|e| panic!("symbol `{name}` not found: {e}"))
}

pub fn c_fma_array() -> Symbol<'static, FmaArrayFn> {
    sym(&libs().c, "fma_array")
}
pub fn rust_fma_array() -> Symbol<'static, FmaArrayFn> {
    sym(&libs().rust, "fma_array")
}
pub fn c_call_fma() -> Symbol<'static, CallFmaFn> {
    sym(&libs().c, "call_fma")
}
pub fn rust_call_fma() -> Symbol<'static, CallFmaFn> {
    sym(&libs().rust, "call_fma")
}
pub fn c_driver() -> Symbol<'static, DriverFn> {
    sym(&libs().c, "driver")
}
pub fn rust_driver() -> Symbol<'static, DriverFn> {
    sym(&libs().rust, "driver")
}

/// Serializes the file-descriptor juggling done by [`capture_stdout`].
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// Runs `f` with fd 1 redirected to a temporary file and returns the raw bytes
/// that were written.
///
/// `driver` reports its result with `printf`, and both shared objects share the
/// process-wide libc `stdout`, so the C stream is flushed on both sides of the
/// call rather than relying on Rust's `io::stdout`.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver-cap-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();

    unsafe {
        // Flush whatever is already pending so it lands on the real stdout.
        // Rust's own buffered `io::stdout` must be flushed as well, otherwise
        // libtest's progress text is emitted into the redirected fd.
        use std::io::Write;
        let _ = std::io::stdout().flush();
        let _ = std::io::stderr().flush();
        libc::fflush(std::ptr::null_mut());

        let fd = libc::open(
            c_path.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_TRUNC,
            0o600 as libc::c_int,
        );
        assert!(fd >= 0, "open() for stdout capture failed");
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(libc::dup2(fd, 1) >= 0, "dup2 failed");

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(saved, 1);
        libc::close(saved);
        libc::close(fd);
    }

    let out = std::fs::read(&path).expect("reading capture file");
    let _ = std::fs::remove_file(&path);
    out
}

/// Calls `driver` in both libraries with the same NUL-terminated input and
/// asserts the printed bytes are identical.
pub fn compare_driver(input: &str) {
    compare_driver_bytes(input.as_bytes(), &format!("{input:?}"));
}

/// Byte-level variant, so non-UTF-8 inputs (including bytes >= 0x80) can be fed
/// through the `const char *` parameter.
pub fn compare_driver_bytes(input: &[u8], label: &str) {
    // `driver` takes `const char *`, so embedded NULs simply terminate early;
    // build the C string with an explicit trailing NUL to allow testing them.
    let mut bytes = input.to_vec();
    bytes.push(0);

    let cd = c_driver();
    let rd = rust_driver();

    let c_out = capture_stdout(|| unsafe { cd(bytes.as_ptr() as *const c_char) });
    let r_out = capture_stdout(|| unsafe { rd(bytes.as_ptr() as *const c_char) });

    assert_eq!(
        c_out,
        r_out,
        "driver({label}) mismatch:\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out)
    );
}

/// Deterministic xorshift64* generator, so failures are reproducible.
pub struct Rng(u64);

/// Multiplier for the randomized test budgets, overridable with
/// `DRIVER_FUZZ_SCALE` for longer soak runs.
pub fn fuzz_scale() -> usize {
    std::env::var("DRIVER_FUZZ_SCALE")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|&v| v > 0)
        .unwrap_or(1)
}

/// `n` scaled by [`fuzz_scale`].
pub fn iters(n: usize) -> usize {
    n * fuzz_scale()
}

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
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }
}
