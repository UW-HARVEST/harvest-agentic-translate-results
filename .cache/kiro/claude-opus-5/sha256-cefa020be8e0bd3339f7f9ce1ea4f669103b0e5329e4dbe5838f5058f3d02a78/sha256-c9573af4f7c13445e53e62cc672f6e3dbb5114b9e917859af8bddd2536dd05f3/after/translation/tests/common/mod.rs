//! Shared differential-testing harness.
//!
//! Both libraries are loaded as shared objects via `libloading` and called only
//! through their exported `driver` symbol — the Rust implementation is never
//! called directly as a Rust function, so the `#[no_mangle] extern "C"` export
//! wrapper is part of what is under test.
//!
//! `driver` returns `void` and its entire observable behaviour is the bytes it
//! writes to `stdout`, so the harness captures `stdout` at the *file descriptor*
//! level (`dup`/`dup2`). That captures whatever the C library's `printf` emits
//! from inside either `.so`, including the trailing newline and the exact digit
//! casing, which a higher-level capture would miss.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams, which is exactly what
    /// is needed: it forces out whatever either `.so` buffered in the process's
    /// shared `stdout` FILE without needing to name the `stdout` object.
    fn fflush(stream: *mut c_void) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
}

const STDOUT_FD: c_int = 1;

/// The C ABI signature of the symbol under test: `void driver(float)`.
pub type DriverFn = unsafe extern "C" fn(f32);

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    manifest_dir()
        .parent()
        .expect("crate has a parent directory")
        .join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    // The `.so` produced by `cargo build --release`, i.e. the real artifact an
    // external consumer would link or `dlopen`.
    let p = manifest_dir().join("target/release/libdriver.so");
    if p.exists() {
        return p;
    }
    manifest_dir().join("target/debug/libdriver.so")
}

/// Both libraries, kept alive for the whole process.
///
/// They are loaded *simultaneously* and deliberately share the one process-wide
/// `stdout` FILE, so row C16 can interleave calls between them on that stream.
struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not built: {}\nbuild it with:\n  cd c_src && mkdir -p build && cd build \\\n    && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            c_path.display()
        );
        assert!(
            rust_path.exists(),
            "Rust shared library not built: {}\nbuild it with:\n  cd translation && cargo build --release",
            rust_path.display()
        );
        unsafe {
            Libs {
                c: Library::new(&c_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", c_path.display())),
                rust: Library::new(&rust_path)
                    .unwrap_or_else(|e| panic!("dlopen {}: {e}", rust_path.display())),
            }
        }
    })
}

/// `driver` as exported by the **C** `.so`.
pub fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("C .so exports `driver`") }
}

/// `driver` as exported by the **Rust** `.so`.
pub fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe {
        libs()
            .rust
            .get(b"driver\0")
            .expect("Rust .so exports `driver`")
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// `dup2` on fd 1 mutates process-global state, so captures must not overlap.
/// The Rust test harness runs test functions on parallel threads, hence the lock.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Runs `f` with fd 1 redirected to a temporary file and returns every byte
/// written to it.
///
/// `printf("%02x", ...)` can only ever emit bytes from `[0-9a-f]`, and the only
/// other byte either library writes is the terminating `\n`. So any *other* byte
/// showing up in a capture is not library output — it is the test runner's own
/// progress text racing into the window. That is detected and the capture retried
/// rather than silently compared, and if it persists the raw bytes are reported
/// instead of being filtered away, so a genuine bug (say, uppercase hex digits)
/// still fails loudly.
pub fn capture_stdout<F: Fn()>(f: F) -> Vec<u8> {
    const RETRIES: usize = 5;
    let mut last = Vec::new();
    for _ in 0..RETRIES {
        last = capture_stdout_once(&f);
        if last
            .iter()
            .all(|&b| b == b'\n' || b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return last;
        }
    }
    panic!(
        "captured output contains bytes that neither library can emit, after {RETRIES} attempts \
         (both only ever write lowercase hex digits and '\\n'): {:?}",
        String::from_utf8_lossy(&last)
    );
}

fn capture_stdout_once<F: Fn()>(f: &F) -> Vec<u8> {
    let _guard = capture_lock();

    // Push out anything either `.so` buffered *before* the redirect, so it lands
    // on the real stdout rather than in our capture file.
    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "driver-difftest-{}-{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let tmp_fd = file.as_raw_fd();

    let saved = unsafe { dup(STDOUT_FD) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp_fd, STDOUT_FD) } >= 0, "dup2 onto 1 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    // Force the library's buffered bytes into the capture file before restoring.
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(
        unsafe { dup2(saved, STDOUT_FD) } >= 0,
        "dup2 restore of 1 failed"
    );
    unsafe { close(saved) };
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);

    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
    bytes
}

/// Runs `f` with fd 1 pointed at `device` (e.g. `/dev/full`, to make `printf`'s
/// underlying `write` fail). Returns the value `fflush` reported, so the test can
/// confirm the write really did fail while `driver` still returned normally.
pub fn with_stdout_on_device<F: Fn()>(device: &str, f: F) -> c_int {
    let _guard = capture_lock();

    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let cpath = std::ffi::CString::new(device).unwrap();
    // O_WRONLY = 1 on Linux.
    let dev_fd = unsafe { open(cpath.as_ptr(), 1) };
    assert!(dev_fd >= 0, "open({device}) failed");

    let saved = unsafe { dup(STDOUT_FD) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(dev_fd, STDOUT_FD) } >= 0, "dup2 onto 1 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    let flush_rc = unsafe { fflush(std::ptr::null_mut()) };
    // The failed stream keeps its error flag set, which would poison later
    // captures, so clear it by flushing again after restoring a good fd.
    assert!(
        unsafe { dup2(saved, STDOUT_FD) } >= 0,
        "dup2 restore of 1 failed"
    );
    unsafe { close(saved) };
    unsafe { close(dev_fd) };
    clear_stdout_error();

    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
    flush_rc
}

/// `clearerr(stdout)` — resets the sticky error flag a failed write leaves behind.
fn clear_stdout_error() {
    unsafe extern "C" {
        fn clearerr(stream: *mut c_void);
        static mut stdout: *mut c_void;
    }
    unsafe {
        let s = stdout;
        if !s.is_null() {
            clearerr(s);
        }
        fflush(std::ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// Calls `driver` once per element of `bits` (each element a raw `f32` bit
/// pattern, so NaN payloads survive untouched) and returns all captured bytes.
fn run_batch(driver: &Symbol<'static, DriverFn>, bits: &[u32]) -> Vec<u8> {
    capture_stdout(|| {
        for &b in bits {
            unsafe { driver(f32::from_bits(b)) };
        }
    })
}

/// The core assertion: run the same sequence of inputs through the C `.so` and
/// the Rust `.so` and require the captured `stdout` bytes to be identical.
///
/// On divergence the mismatching line is located and reported alongside the
/// input bit pattern that produced it, since one call emits exactly one line.
pub fn assert_same(label: &str, bits: &[u32]) {
    let c_out = run_batch(&c_driver(), bits);
    let rust_out = run_batch(&rust_driver(), bits);

    if c_out == rust_out {
        // Line framing must also line up with the inputs: exactly one line per
        // call, which is what makes the per-line localisation below valid.
        let lines = c_out.iter().filter(|&&b| b == b'\n').count();
        assert_eq!(
            lines,
            bits.len(),
            "[{label}] expected one output line per call, got {lines} lines for {} calls",
            bits.len()
        );
        return;
    }

    let c_lines: Vec<&[u8]> = c_out.split(|&b| b == b'\n').collect();
    let r_lines: Vec<&[u8]> = rust_out.split(|&b| b == b'\n').collect();
    for (i, (cl, rl)) in c_lines.iter().zip(r_lines.iter()).enumerate() {
        if cl != rl {
            let input = bits.get(i).copied().unwrap_or(0);
            panic!(
                "[{label}] divergence at call #{i}\n  \
                 input bits : 0x{input:08x} (as f32: {})\n  \
                 C   output : {:?}\n  \
                 Rust output: {:?}",
                f32::from_bits(input),
                String::from_utf8_lossy(cl),
                String::from_utf8_lossy(rl),
            );
        }
    }
    panic!(
        "[{label}] divergence: C produced {} bytes / {} lines, Rust produced {} bytes / {} lines",
        c_out.len(),
        c_lines.len(),
        rust_out.len(),
        r_lines.len()
    );
}

/// Same as [`assert_same`] but takes `f32` values directly.
pub fn assert_same_floats(label: &str, values: &[f32]) {
    let bits: Vec<u32> = values.iter().map(|v| v.to_bits()).collect();
    assert_same(label, &bits);
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed, so every run is reproducible)
// ---------------------------------------------------------------------------

/// SplitMix64 — small, seedable, and good enough to spray bit patterns.
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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u32) -> u32 {
        self.next_u32() % n
    }
}
