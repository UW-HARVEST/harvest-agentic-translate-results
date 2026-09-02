//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries (the C `libdriver.so` built by CMake and the
//! Rust `libdriver.so` cdylib) through `libloading` and calls them only through
//! their exported C symbols — never by linking the Rust crate directly. This
//! exercises the `#[no_mangle]` / `extern "C"` wrappers exactly as an external
//! consumer would.
//!
//! The library's only observable output is the byte stream it writes to
//! `stdout`, so the harness redirects fd 1 to a temporary file around each
//! call, flushes every stdio stream, and compares the captured bytes.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams, which is what we need
    /// because the C `.so` and the Rust `.so` both write through the same
    /// process-wide glibc `stdout`.
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// The exported ABI surface, resolved by dlsym
// ---------------------------------------------------------------------------

pub struct Api {
    pub name: &'static str,
    _lib: libloading::Library,
    pub print_line: unsafe extern "C" fn(*const c_char),
    pub print_hex_char_line: unsafe extern "C" fn(c_char),
    /// Same symbol as `print_hex_char_line`, but called with a full `int`
    /// argument so the upper 24 bits of the argument register are non-zero.
    /// A C `char` parameter must consider only the low byte.
    pub print_hex_char_line_int: unsafe extern "C" fn(c_int),
    pub bad: unsafe extern "C" fn(),
    pub good: unsafe extern "C" fn(),
    pub driver: unsafe extern "C" fn(c_int),
}

impl Api {
    fn load(name: &'static str, path: &PathBuf) -> Api {
        unsafe {
            let lib = libloading::Library::new(path)
                .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));

            macro_rules! sym {
                ($t:ty, $s:literal) => {{
                    let s: libloading::Symbol<$t> = lib.get($s).unwrap_or_else(|e| {
                        panic!(
                            "{} missing symbol {}: {e}",
                            name,
                            String::from_utf8_lossy($s)
                        )
                    });
                    *s
                }};
            }

            let print_line = sym!(unsafe extern "C" fn(*const c_char), b"printLine\0");
            let print_hex_char_line =
                sym!(unsafe extern "C" fn(c_char), b"printHexCharLine\0");
            let print_hex_char_line_int =
                sym!(unsafe extern "C" fn(c_int), b"printHexCharLine\0");
            let bad = sym!(unsafe extern "C" fn(), b"bad\0");
            let good = sym!(unsafe extern "C" fn(), b"good\0");
            let driver = sym!(unsafe extern "C" fn(c_int), b"driver\0");

            Api {
                name,
                _lib: lib,
                print_line,
                print_hex_char_line,
                print_hex_char_line_int,
                bad,
                good,
                driver,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().parent().unwrap().to_path_buf();
    let candidates = [
        root.join("c_src/build/libdriver.so"),
        root.join("c_src/build/Release/libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!(
        "C shared library not found. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
}

fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let md = manifest_dir();
    // Prefer the release cdylib (the artifact an external consumer ships),
    // fall back to debug so `cargo test` alone still works.
    let candidates = [
        md.join("target/release/libdriver.so"),
        md.join("target/debug/libdriver.so"),
    ];
    for c in &candidates {
        if c.exists() {
            return c.clone();
        }
    }
    panic!("Rust cdylib not found; run `cargo build --release` first");
}

/// Guards against the single most dangerous failure mode of this suite:
/// silently testing a STALE `.so`.
///
/// The integration test has no link-time dependency on the `cdylib` target
/// (`crate-type = ["cdylib"]` produces no rlib), so `cargo test` does NOT
/// rebuild `libdriver.so`. Without this check, editing `src/lib.rs` and running
/// `cargo test` would compare against the previously built library and every
/// test would pass vacuously. Verified: a deliberately broken `src/lib.rs`
/// passed all 41 tests before this guard existed.
fn assert_fresh(so: &std::path::Path) {
    use std::time::SystemTime;

    let so_mtime = match std::fs::metadata(so).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return,
    };

    let mut newest: Option<(SystemTime, PathBuf)> = None;
    let mut consider = |p: PathBuf| {
        if let Ok(t) = std::fs::metadata(&p).and_then(|m| m.modified()) {
            if newest.as_ref().map_or(true, |(bt, _)| t > *bt) {
                newest = Some((t, p));
            }
        }
    };

    // Walk src/ recursively plus the manifest.
    let mut stack = vec![manifest_dir().join("src")];
    while let Some(dir) = stack.pop() {
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else {
                    consider(p);
                }
            }
        }
    }
    consider(manifest_dir().join("Cargo.toml"));

    if let Some((t, p)) = newest {
        assert!(
            t <= so_mtime,
            "STALE RUST LIBRARY: {} is newer than {}.\n\
             `cargo test` does not rebuild the cdylib (nothing links it), so the \
             tests would compare against an out-of-date .so and pass vacuously.\n\
             Run `cargo build --release` (and `cargo build`) first, or use \
             ./check_features.sh which does it for you.",
            p.display(),
            so.display()
        );
    }
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

pub fn c_api() -> &'static Api {
    C_API.get_or_init(|| Api::load("C", &c_lib_path()))
}

pub fn rust_api() -> &'static Api {
    RUST_API.get_or_init(|| {
        let p = rust_lib_path();
        assert_fresh(&p);
        Api::load("RUST", &p)
    })
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// fd 1 is process-global, so captures must never overlap.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static SEQ: AtomicU64 = AtomicU64::new(0);

fn lock() -> MutexGuard<'static, ()> {
    match CAPTURE_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

/// Runs `f` with fd 1 redirected to a fresh temp file and returns everything
/// written to it.
pub fn capture(f: &mut dyn FnMut()) -> Vec<u8> {
    use std::io::Write;
    let _g = lock();

    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        n
    ));

    // Make sure nothing of ours is still sitting in a userspace buffer.
    let _ = std::io::stdout().flush();
    unsafe { fflush(std::ptr::null_mut()) };

    let bytes = unsafe {
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");

        {
            let file = std::fs::File::create(&path).expect("create temp capture file");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");
        } // file closed here; fd 1 keeps its own reference

        f();

        // Push the library's stdio buffer out to the file before restoring.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);

        std::fs::read(&path).expect("read temp capture file")
    };

    let _ = std::fs::remove_file(&path);
    bytes
}

fn show(b: &[u8]) -> String {
    format!("{:?} ({} bytes, hex {})", String::from_utf8_lossy(b), b.len(), {
        let mut s = String::new();
        for x in b.iter().take(96) {
            s.push_str(&format!("{x:02x}"));
        }
        s
    })
}

/// Runs the same program against both libraries and asserts the stdout bytes
/// are identical.
pub fn assert_same(label: &str, run: &dyn Fn(&Api)) {
    // Resolve both libraries BEFORE fd 1 is redirected, so a dlopen/dlsym
    // failure produces a legible panic instead of one swallowed by the capture.
    let (ca, ra) = (c_api(), rust_api());
    let c = capture(&mut || run(ca));
    let r = capture(&mut || run(ra));
    assert!(
        c == r,
        "DIVERGENCE [{label}]\n     C: {}\n  RUST: {}",
        show(&c),
        show(&r)
    );
}

/// Like `assert_same`, but also checks the C output against an expected byte
/// string so the test pins down C's *actual* behaviour, not just agreement.
pub fn assert_same_and_eq(label: &str, expected: &[u8], run: &dyn Fn(&Api)) {
    let (ca, ra) = (c_api(), rust_api());
    let c = capture(&mut || run(ca));
    let r = capture(&mut || run(ra));
    assert!(
        c == expected,
        "C BEHAVIOUR CHANGED [{label}]\n  expected: {}\n     got C: {}",
        show(expected),
        show(&c)
    );
    assert!(
        c == r,
        "DIVERGENCE [{label}]\n     C: {}\n  RUST: {}",
        show(&c),
        show(&r)
    );
}

// ---------------------------------------------------------------------------
// deterministic RNG (fixed seed -> reproducible property-style tests)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9E3779B97F4A7C15)
    }
    pub fn next_u64(&mut self) -> u64 {
        // splitmix64
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }
    pub fn next_u8(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }
    /// Uniform in `0..n`.
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// Builds a NUL-terminated buffer from `bytes` (which must not contain NUL for
/// the round-trip tests to be meaningful).
pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    let mut v = bytes.to_vec();
    v.push(0);
    v
}
