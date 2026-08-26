// Shared differential-testing harness.
//
// Both implementations are loaded as *shared objects* with `libloading` and are
// only ever reached through their exported `driver` symbol, exactly as an
// external C consumer would reach them.  The Rust crate is never linked into
// the test binary, so the `#[no_mangle] extern "C"` export wrapper is part of
// what is under test.
//
// The library's only observable effect is the byte stream it writes onto the
// process-wide C `stdout` (`printf`), so the harness captures file descriptor 1
// around each call and compares the captured bytes byte-for-byte.

#![allow(dead_code)]

use std::ffi::{c_int, c_void};
use std::fs::File;
use std::io::{Read, Write};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits we need.  Declared directly instead of pulling in the `libc` crate
// so the harness has exactly one dev-dependency (`libloading`).
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream, which is what makes
    /// the C library's buffered `printf` output land in our capture target.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// The signature of the one and only exported symbol: `void driver(int x)`.
pub type DriverFn = unsafe extern "C" fn(c_int);

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path of the C shared object built from `c_src/` by CMake.
fn c_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let candidates = [
        manifest_dir().join("c_src/build/libdriver.so"),
        manifest_dir().join("c_build/libdriver.so"),
    ];
    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found; build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .\n\
         (looked in {candidates:?})"
    );
}

/// Path of the Rust `cdylib`.  Derived from the location of the running test
/// binary (`target/<profile>/deps/<test>` -> `target/<profile>/libdriver.so`)
/// so that it always matches the profile / feature set cargo just built.
fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let mut dir: Option<&Path> = exe.parent();
    let mut tried = Vec::new();
    while let Some(d) = dir {
        let cand = d.join("libdriver.so");
        if cand.is_file() {
            return cand;
        }
        tried.push(cand);
        dir = d.parent();
        if tried.len() > 4 {
            break;
        }
    }
    panic!("Rust cdylib not found (run `cargo build` first); looked in {tried:?}");
}

fn c_lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let p = c_so_path();
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {p:?}: {e}"))
    })
}

fn rust_lib() -> &'static Library {
    static LIB: OnceLock<Library> = OnceLock::new();
    LIB.get_or_init(|| {
        let p = rust_so_path();
        unsafe { Library::new(&p) }.unwrap_or_else(|e| panic!("dlopen {p:?}: {e}"))
    })
}

/// `driver` as exported by the **C** shared object (resolved via `dlsym`).
pub fn c_driver() -> DriverFn {
    // The `Library` lives in a `'static` `OnceLock`, so the resolved function
    // pointer stays valid for the whole process lifetime.
    let s: Symbol<'static, DriverFn> =
        unsafe { c_lib().get(b"driver\0") }.expect("C .so does not export `driver`");
    *s
}

/// `driver` as exported by the **Rust** shared object (resolved via `dlsym`).
/// Nothing in these tests calls the Rust code directly: it is always reached
/// through this exported symbol, so the `#[no_mangle]` wrapper is under test.
pub fn rust_driver() -> DriverFn {
    let s: Symbol<'static, DriverFn> =
        unsafe { rust_lib().get(b"driver\0") }.expect("Rust .so does not export `driver`");
    *s
}

/// Sanity check that both `.so`s really are loadable and export the symbol.
pub fn both_libraries_loaded() -> (DriverFn, DriverFn) {
    (c_driver(), rust_driver())
}

pub fn so_paths() -> (PathBuf, PathBuf) {
    (c_so_path(), rust_so_path())
}

// ---------------------------------------------------------------------------
// fd-1 capture.  Redirecting fd 1 is a process-global operation, so it is
// serialised behind a mutex (tests inside one binary run on many threads).
// ---------------------------------------------------------------------------

fn capture_lock() -> MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    match L.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Exclusive right to redirect fd 1.
///
/// Two independent writers have to be kept out of the captured byte range:
///   * other *test threads* doing their own capture -> the process-wide mutex;
///   * the *libtest harness itself*, which writes its `test foo ... ok`
///     progress lines to `std::io::stdout()` from another thread -> we hold the
///     `StdoutLock` for the whole capture, which blocks those writes (the lock
///     is re-entrant, so this thread may still flush through it).
///
/// Dropped only after fd 1 has been restored, so the harness's buffered
/// progress text goes to the real stdout, never into the capture.
struct CaptureGuard {
    _stdout: std::io::StdoutLock<'static>,
    _global: MutexGuard<'static, ()>,
}

fn capture_guard() -> CaptureGuard {
    let global = capture_lock();
    let mut stdout = std::io::stdout().lock();
    // Push out anything libtest has half-written ("test foo ... ") *before*
    // fd 1 is redirected.
    let _ = stdout.flush();
    CaptureGuard {
        _stdout: stdout,
        _global: global,
    }
}

/// Restores fd 1 (and flushes the C streams) even if the body panics.
struct Fd1Redirect {
    saved: c_int,
}

impl Fd1Redirect {
    unsafe fn to(target_fd: c_int) -> Self {
        // Make sure nothing pending from *before* the redirect leaks into the
        // capture, on either the Rust or the C side.
        let _ = std::io::stdout().flush();
        assert_eq!(fflush(ptr::null_mut()), 0, "pre-redirect fflush failed");
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(target_fd, 1) >= 0, "dup2 onto fd 1 failed");
        Fd1Redirect { saved }
    }
}

impl Drop for Fd1Redirect {
    fn drop(&mut self) {
        unsafe {
            // Flush the captured bytes *before* fd 1 points elsewhere again.
            fflush(ptr::null_mut());
            dup2(self.saved, 1);
            close(self.saved);
        }
    }
}

fn tmp_path(tag: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join(format!(
        "driver-diff-{}-{}-{}-{}.out",
        std::process::id(),
        tag,
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0)
    ))
}

/// Runs `body` with fd 1 redirected into a **regular file** (glibc:
/// fully buffered) and returns every byte written.
pub fn capture_file<F: FnOnce()>(body: F) -> Vec<u8> {
    let _guard = capture_guard();
    let path = tmp_path("file");
    {
        let f = File::create(&path).expect("create capture file");
        let _r = unsafe { Fd1Redirect::to(f.as_raw_fd()) };
        body();
        // `_r`'s Drop flushes the C streams and restores fd 1 here.
    }
    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Runs `body` with fd 1 redirected into a **pipe** (a different glibc
/// buffering decision than a regular file) and returns every byte written.
/// A reader thread drains the pipe so arbitrarily large output cannot deadlock.
pub fn capture_pipe<F: FnOnce()>(body: F) -> Vec<u8> {
    let _guard = capture_guard();
    let mut fds: [c_int; 2] = [-1, -1];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
    let (rd, wr) = (fds[0], fds[1]);

    let reader = std::thread::spawn(move || {
        let mut f = unsafe { File::from_raw_fd(rd) };
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).expect("drain pipe");
        buf
    });

    {
        let _r = unsafe { Fd1Redirect::to(wr) };
        body();
    }
    // Close the write end so the reader sees EOF.
    unsafe { close(wr) };
    reader.join().expect("reader thread")
}

// ---------------------------------------------------------------------------
// Differential helpers
// ---------------------------------------------------------------------------

fn diff_report(x_desc: &str, c: &[u8], r: &[u8]) -> String {
    let first = c
        .iter()
        .zip(r.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| c.len().min(r.len()));
    let ctx = |b: &[u8]| {
        let lo = first.saturating_sub(48);
        let hi = (first + 48).min(b.len());
        String::from_utf8_lossy(&b[lo..hi]).escape_debug().to_string()
    };
    format!(
        "output mismatch for {x_desc}\n  C   len={} \n  RS  len={} \n  first differing byte index = {first}\n  C   ...{}...\n  RS  ...{}...",
        c.len(),
        r.len(),
        ctx(c),
        ctx(r)
    )
}

/// Core Phase-B/C assertion: call `driver(x)` in the C `.so` and in the Rust
/// `.so`, capture fd 1 for each, require the byte streams to be identical.
pub fn assert_same(x: c_int) {
    let (cf, rf) = both_libraries_loaded();
    let c_out = capture_file(|| unsafe { cf(x) });
    let r_out = capture_file(|| unsafe { rf(x) });
    assert!(
        c_out == r_out,
        "{}",
        diff_report(&format!("driver({x})"), &c_out, &r_out)
    );
}

/// Same, but with fd 1 being a pipe rather than a regular file.
pub fn assert_same_pipe(x: c_int) {
    let (cf, rf) = both_libraries_loaded();
    let c_out = capture_pipe(|| unsafe { cf(x) });
    let r_out = capture_pipe(|| unsafe { rf(x) });
    assert!(
        c_out == r_out,
        "{}",
        diff_report(&format!("driver({x}) [pipe]"), &c_out, &r_out)
    );
}

/// Runs a whole *sequence* of calls as one capture against each library, so
/// that any hidden per-call state or stream-state difference shows up.
pub fn assert_same_sequence(xs: &[c_int]) {
    let (cf, rf) = both_libraries_loaded();
    let c_out = capture_file(|| {
        for &x in xs {
            unsafe { cf(x) };
        }
    });
    let r_out = capture_file(|| {
        for &x in xs {
            unsafe { rf(x) };
        }
    });
    assert!(
        c_out == r_out,
        "{}",
        diff_report(&format!("sequence {xs:?}"), &c_out, &r_out)
    );
}

/// Captures the C output for `x` and returns it (used by rejection tests that
/// assert an exact expected byte string as well as equality).
pub fn c_output(x: c_int) -> Vec<u8> {
    let f = c_driver();
    capture_file(|| unsafe { f(x) })
}

pub fn rust_output(x: c_int) -> Vec<u8> {
    let f = rust_driver();
    capture_file(|| unsafe { f(x) })
}

// ---------------------------------------------------------------------------
// Deterministic RNG (fixed seed => reproducible property-style runs).
// splitmix64.
// ---------------------------------------------------------------------------
pub struct Rng(u64);

pub const SEED: u64 = 0x243F_6A88_85A3_08D3;

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
    /// Uniform-ish in `lo..=hi`.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}
