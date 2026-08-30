//! Shared harness for the differential tests.
//!
//! Both libraries are loaded as shared objects with `libloading` and called only
//! through their exported `driver` symbol — never by linking the Rust crate
//! directly — so the `#[no_mangle] extern "C"` export wrapper is under test too.
//!
//! `driver`'s only observable is what it writes to `stdout` via libc `printf`, so
//! the harness captures file descriptor 1 into a temporary file. Because fd 1 is
//! process-global, every capture is serialised behind a global mutex.

#![allow(dead_code)]

use std::ffi::{c_int, c_long, c_void};
use std::fs::File;
use std::os::unix::fs::FileExt;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream, which is how we force
    /// libc's buffered `stdout` into the capture file at a known point.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// The `void driver(int, int)` ABI, as an external caller sees it.
pub type DriverFn = unsafe extern "C" fn(c_int, c_int);

/// The same symbol viewed with 64-bit parameters, used to prove the callee
/// ignores the undefined upper halves of the SysV argument registers.
pub type DriverFnWide = unsafe extern "C" fn(c_long, c_long);

// ---------------------------------------------------------------------------
// Library locations
// ---------------------------------------------------------------------------

pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crate dir has a parent")
        .join("c_src/build/libdriver.so")
}

pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // The integration-test executable lives in `target/<profile>/deps/`, so the
    // cdylib built alongside it is one directory up.
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|p| p.parent())
        .expect("target/<profile>");
    let candidate = profile_dir.join("libdriver.so");
    if candidate.exists() {
        return candidate;
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/release/libdriver.so")
}

fn c_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| {
        let p = c_so_path();
        assert!(
            p.exists(),
            "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && \
             cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
            p.display()
        );
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", p.display()))
    })
}

fn rust_lib() -> &'static Library {
    static L: OnceLock<Library> = OnceLock::new();
    L.get_or_init(|| {
        let p = rust_so_path();
        assert!(
            p.exists(),
            "Rust shared library not found at {}. Build it with `cargo build`.",
            p.display()
        );
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {}: {e}", p.display()))
    })
}

/// `driver` as exported by the C `.so`.
pub fn c_driver() -> DriverFn {
    let s: Symbol<DriverFn> =
        unsafe { c_lib().get(b"driver\0") }.expect("C .so must export `driver`");
    *s
}

/// `driver` as exported by the Rust `.so`.
pub fn rust_driver() -> DriverFn {
    let s: Symbol<DriverFn> =
        unsafe { rust_lib().get(b"driver\0") }.expect("Rust .so must export `driver`");
    *s
}

pub fn c_driver_wide() -> DriverFnWide {
    let s: Symbol<DriverFnWide> = unsafe { c_lib().get(b"driver\0") }.expect("C `driver`");
    *s
}

pub fn rust_driver_wide() -> DriverFnWide {
    let s: Symbol<DriverFnWide> = unsafe { rust_lib().get(b"driver\0") }.expect("Rust `driver`");
    *s
}

/// Returns `true` if both libraries export `driver`, without asserting.
pub fn both_export_driver() -> bool {
    let c: Result<Symbol<DriverFn>, _> = unsafe { c_lib().get(b"driver\0") };
    let r: Result<Symbol<DriverFn>, _> = unsafe { rust_lib().get(b"driver\0") };
    c.is_ok() && r.is_ok()
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn capture_lock() -> &'static Mutex<()> {
    static M: OnceLock<Mutex<()>> = OnceLock::new();
    M.get_or_init(|| Mutex::new(()))
}

/// Redirects fd 1 into a temporary file for as long as it is alive.
///
/// Held across a whole test so that thousands of comparisons cost only an
/// `fflush` each rather than a fresh file per input.
pub struct Capture {
    file: File,
    path: PathBuf,
    saved_fd: c_int,
    _guard: MutexGuard<'static, ()>,
}

impl Capture {
    pub fn new(tag: &str) -> Self {
        let guard = capture_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        // Don't let anything already sitting in a libc buffer land in our file.
        unsafe { fflush(std::ptr::null_mut()) };

        let dir = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        let path = PathBuf::from(dir).join(format!(
            "driver-capture-{}-{}-{}.txt",
            tag,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        // Must be readable as well as writable: we `read_at` it to recover the
        // captured bytes, and `File::create` alone would give us an O_WRONLY fd.
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .expect("create capture file");

        let saved_fd = unsafe { dup(1) };
        assert!(saved_fd >= 0, "dup(1) failed");
        assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 onto fd 1 failed");

        Capture {
            file,
            path,
            saved_fd,
            _guard: guard,
        }
    }

    /// Flushes libc's buffers and returns the current end-of-capture offset.
    pub fn mark(&self) -> u64 {
        unsafe { fflush(std::ptr::null_mut()) };
        self.file.metadata().map(|m| m.len()).unwrap_or(0)
    }

    /// Bytes written between two marks.
    pub fn slice(&self, from: u64, to: u64) -> Vec<u8> {
        assert!(to >= from, "capture offsets went backwards");
        let mut buf = vec![0u8; (to - from) as usize];
        if !buf.is_empty() {
            self.file
                .read_exact_at(&mut buf, from)
                .expect("read capture file");
        }
        buf
    }
}

impl Drop for Capture {
    fn drop(&mut self) {
        unsafe {
            fflush(std::ptr::null_mut());
            dup2(self.saved_fd, 1);
            close(self.saved_fd);
        }
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Calls C then Rust with `(x, y)` and returns `(c_bytes, rust_bytes)`.
pub fn run_pair(cap: &Capture, x: i32, y: i32) -> (Vec<u8>, Vec<u8>) {
    let c = c_driver();
    let r = rust_driver();
    let a = cap.mark();
    unsafe { c(x, y) };
    let b = cap.mark();
    unsafe { r(x, y) };
    let d = cap.mark();
    (cap.slice(a, b), cap.slice(b, d))
}

/// Asserts C and Rust produce byte-identical output for every pair.
///
/// Failures are accumulated and reported together, after fd 1 is restored, so
/// the panic message is actually visible.
pub fn assert_pairs_match(tag: &str, pairs: &[(i32, i32)]) {
    let mut failures: Vec<String> = Vec::new();
    {
        let cap = Capture::new(tag);
        for &(x, y) in pairs {
            let (cb, rb) = run_pair(&cap, x, y);
            if cb != rb {
                failures.push(format!(
                    "  driver({x}, {y}):\n    C    = {:?}\n    Rust = {:?}",
                    String::from_utf8_lossy(&cb),
                    String::from_utf8_lossy(&rb)
                ));
                if failures.len() >= 20 {
                    break;
                }
            }
        }
    } // fd 1 restored here

    assert!(
        failures.is_empty(),
        "[{tag}] {} of {} input(s) diverged between C and Rust:\n{}",
        failures.len(),
        pairs.len(),
        failures.join("\n")
    );
}

/// Sanity check that the captured text is actually the expected rendering.
///
/// Guards against a "both produced nothing, so both match" false pass.
pub fn expected_line(x: i32, y: i32) -> String {
    let quot = (x as i64 / y as i64) as i32;
    let rem = (x as i64 % y as i64) as i32;
    format!("quotient: {quot}, remainder: {rem}\n")
}

pub fn assert_pairs_match_and_nonempty(tag: &str, pairs: &[(i32, i32)]) {
    assert_pairs_match(tag, pairs);

    let mut failures: Vec<String> = Vec::new();
    {
        let cap = Capture::new(tag);
        for &(x, y) in pairs.iter().take(256) {
            let (cb, _) = run_pair(&cap, x, y);
            let want = expected_line(x, y);
            if cb != want.as_bytes() {
                failures.push(format!(
                    "  driver({x}, {y}): C produced {:?}, model expected {:?}",
                    String::from_utf8_lossy(&cb),
                    want
                ));
                if failures.len() >= 10 {
                    break;
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "[{tag}] the C library disagreed with the reference model \
         (harness/capture bug, not a translation bug):\n{}",
        failures.join("\n")
    );
}

// ---------------------------------------------------------------------------
// Deterministic randomness (SplitMix64) — fixed seed, reproducible
// ---------------------------------------------------------------------------

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

    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }

    /// Non-zero `i32`, uniform over the whole range.
    pub fn next_i32_nonzero(&mut self) -> i32 {
        loop {
            let v = self.next_i32();
            if v != 0 {
                return v;
            }
        }
    }

    /// Positive `i32` in `[1, INT_MAX]`.
    pub fn next_positive(&mut self) -> i32 {
        (self.next_u64() % (i32::MAX as u64) + 1) as i32
    }

    /// Negative `i32` in `[INT_MIN, -1]`.
    pub fn next_negative(&mut self) -> i32 {
        let mag = (self.next_u64() % (i32::MAX as u64)) as i64;
        (-mag - 1) as i32
    }
}

/// The seed every randomized row uses, so runs are reproducible.
pub const SEED: u64 = 0x5EED_1234_ABCD_0001;
