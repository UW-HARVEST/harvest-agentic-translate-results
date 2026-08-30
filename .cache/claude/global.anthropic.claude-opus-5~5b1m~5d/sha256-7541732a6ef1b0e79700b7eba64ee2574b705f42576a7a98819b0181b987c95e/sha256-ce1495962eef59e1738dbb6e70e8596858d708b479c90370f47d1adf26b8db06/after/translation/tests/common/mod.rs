//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both libraries are loaded as shared objects with `libloading` and driven
//! purely through their exported `driver` symbol, exactly as an external C
//! consumer would. The Rust functions are never called directly, so the
//! `#[no_mangle] extern "C"` wrapper is part of what is under test.
//!
//! `driver`'s only observable effect is what it writes to `stdout` via libc
//! `printf`, so the harness captures fd 1 around each call. Capture is
//! process-global, hence serialised behind a mutex.

#![allow(dead_code)]

use std::ffi::{c_int, c_void};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open libc output stream, which is how we
    /// force both libraries' buffered `printf` output out before we look at it.
    fn fflush(stream: *mut c_void) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
}

pub type DriverFn = unsafe extern "C" fn(c_int);

/// Which implementation to invoke.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

impl Impl {
    pub fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
}

/// A loaded `.so` plus its resolved `driver` entry point.
pub struct Lib {
    // Field order matters: `driver` must be dropped before `_lib`.
    pub driver: DriverFn,
    _lib: libloading::Library,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent directory")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let p = workspace_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    // Pick the freshest of the two profile artifacts. `cargo test` does NOT
    // rebuild a `cdylib`-only lib target, so whichever exists may be stale --
    // hence the mtime guard below, which is essential: without it the whole
    // suite can silently validate an outdated `.so` and pass vacuously.
    let mut newest: Option<(PathBuf, std::time::SystemTime)> = None;
    for profile in ["release", "debug"] {
        let p = root.join("target").join(profile).join("libdriver.so");
        if let Ok(m) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            if newest.as_ref().map_or(true, |(_, best)| m > *best) {
                newest = Some((p, m));
            }
        }
    }
    let (path, so_mtime) = newest.unwrap_or_else(|| {
        panic!(
            "Rust shared library not found under {}/target/{{release,debug}}/libdriver.so.\n\
             Build it with:\n  cd translation && cargo build --release",
            root.display()
        )
    });

    let src = root.join("src/lib.rs");
    let src_mtime = std::fs::metadata(&src)
        .and_then(|m| m.modified())
        .expect("stat src/lib.rs");
    assert!(
        so_mtime >= src_mtime,
        "STALE ARTIFACT: {} is older than {}.\n\
         `cargo test` does not rebuild a cdylib-only lib target, so this run \
         would have compared against outdated code.\n\
         Run `cargo build --release` (or ./run_all.sh) first.",
        path.display(),
        src.display()
    );

    path
}

fn load(path: &Path) -> Lib {
    unsafe {
        let lib = libloading::Library::new(path)
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let sym: libloading::Symbol<DriverFn> = lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("symbol `driver` missing from {}: {e}", path.display()));
        let driver = *sym;
        Lib { driver, _lib: lib }
    }
}

struct Libs {
    c: Lib,
    rust: Lib,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| Libs {
        c: load(&c_so_path()),
        rust: load(&rust_so_path()),
    })
}

pub fn driver_of(which: Impl) -> DriverFn {
    match which {
        Impl::C => libs().c.driver,
        Impl::Rust => libs().rust.driver,
    }
}

/// Assert both `.so`s keep `print_hex` internal, i.e. no external caller can
/// reach the pointer/length API. (ERRORS.md note on null pointers / lengths.)
pub fn print_hex_is_hidden() -> bool {
    unsafe {
        let missing = |p: &Path| {
            let lib = libloading::Library::new(p).unwrap();
            lib.get::<DriverFn>(b"print_hex\0").is_err()
        };
        missing(&c_so_path()) && missing(&rust_so_path())
    }
}

/// stdout redirection is a process-wide mutation, so only one capture at a time.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// RAII redirection of fd 1. Restoring in `Drop` matters: an assertion failure
/// inside a capture window unwinds, and without this fd 1 would stay pointed at
/// the temp file, silently swallowing all subsequent output including the
/// failure report itself.
struct Redirect {
    saved: c_int,
}

impl Redirect {
    fn to(target_fd: c_int) -> Self {
        unsafe {
            // Flush first so previously buffered bytes go to the REAL stdout
            // rather than being attributed to this capture.
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(target_fd, 1) >= 0, "dup2 onto stdout failed");
            Redirect { saved }
        }
    }
}

impl Drop for Redirect {
    fn drop(&mut self) {
        unsafe {
            // Flush the library's buffered output INTO the capture, then restore.
            fflush(std::ptr::null_mut());
            dup2(self.saved, 1);
            close(self.saved);
        }
    }
}

/// Run `body` with fd 1 pointed at a fresh temporary file (fully buffered
/// stdout, config C15) and return every byte written.
pub fn capture_stdout_via_file<F: FnOnce()>(body: F) -> Vec<u8> {
    let _guard = capture_lock();

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver_diff_{}_{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create temp capture file");

    {
        let _redirect = Redirect::to(file.as_raw_fd());
        body();
    } // fd 1 restored here, even if `body` panicked.
    drop(file);

    let out = std::fs::read(&path).expect("read back captured stdout");
    let _ = std::fs::remove_file(&path);
    out
}

/// Same, but fd 1 is a **pipe** rather than a regular file (config C16).
/// The read end is drained on a helper thread so a large volume of output
/// cannot deadlock on the pipe buffer.
pub fn capture_stdout_via_pipe<F: FnOnce()>(body: F) -> Vec<u8> {
    let _guard = capture_lock();

    let mut fds = [0 as c_int; 2];
    unsafe {
        assert!(pipe(fds.as_mut_ptr()) == 0, "pipe() failed");
    }
    let (read_fd, write_fd) = (fds[0], fds[1]);

    let reader = std::thread::spawn(move || {
        use std::os::unix::io::FromRawFd;
        let mut f = unsafe { std::fs::File::from_raw_fd(read_fd) };
        let mut buf = Vec::new();
        let _ = f.read_to_end(&mut buf);
        buf
    });

    {
        let _redirect = Redirect::to(write_fd);
        body();
    } // fd 1 restored here, even if `body` panicked.
    unsafe {
        // Close the write end so the reader thread sees EOF.
        close(write_fd);
    }

    reader.join().expect("pipe reader thread panicked")
}

/// Output of one `driver(x)` call on one implementation, captured via file.
pub fn run_one(which: Impl, x: c_int) -> Vec<u8> {
    let f = driver_of(which);
    capture_stdout_via_file(|| unsafe { f(x) })
}

/// Output of `driver(x)` for every `x` in `xs`, in order, on one loaded handle.
pub fn run_many(which: Impl, xs: &[c_int]) -> Vec<u8> {
    let f = driver_of(which);
    capture_stdout_via_file(|| {
        for &x in xs {
            unsafe { f(x) }
        }
    })
}

fn show(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).escape_debug().to_string()
}

/// Core differential assertion: C and Rust must agree byte-for-byte on `x`.
#[track_caller]
pub fn assert_same(label: &str, x: c_int) {
    let c = run_one(Impl::C, x);
    let r = run_one(Impl::Rust, x);
    assert_eq!(
        c,
        r,
        "[{label}] driver({x}) (0x{:08x}) diverged:\n   C output = \"{}\"\nRust output = \"{}\"",
        x as u32,
        show(&c),
        show(&r)
    );
    assert!(
        !c.is_empty(),
        "[{label}] driver({x}) produced no output at all — capture harness is broken"
    );
}

#[track_caller]
pub fn assert_same_all(label: &str, xs: &[c_int]) {
    // Per-value comparison first: pinpoints the offending input.
    for &x in xs {
        assert_same(label, x);
    }
    // Then the whole batch through a single loaded handle, which also catches
    // any cross-call state drift.
    let c = run_many(Impl::C, xs);
    let r = run_many(Impl::Rust, xs);
    assert_eq!(
        c,
        r,
        "[{label}] batch of {} values diverged:\n   C = \"{}\"\nRust = \"{}\"",
        xs.len(),
        show(&c),
        show(&r)
    );
    assert_eq!(
        c.len(),
        9 * xs.len(),
        "[{label}] expected 9 bytes per call (8 hex digits + newline), got {} for {} calls",
        c.len(),
        xs.len()
    );
}

/// Deterministic splitmix64 PRNG — fixed seed, reproducible across runs.
pub struct Rng(u64);

pub const SEED: u64 = 0x5EED_1234_5678_9ABC;

impl Rng {
    pub fn new() -> Self {
        Rng(SEED)
    }
    pub fn seeded(s: u64) -> Self {
        Rng(s)
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
    /// Uniform `int` over the whole 32-bit range (all bit patterns).
    pub fn next_i32(&mut self) -> c_int {
        self.next_u32() as c_int
    }
    /// Uniform in `[lo, hi]` inclusive, computed in i64 to avoid overflow.
    pub fn range_i32(&mut self, lo: i64, hi: i64) -> c_int {
        debug_assert!(lo <= hi);
        let span = (hi - lo + 1) as u64;
        (lo + (self.next_u64() % span) as i64) as c_int
    }
}
