//! Shared harness for the C-vs-Rust differential tests.
//!
//! BOTH implementations are loaded as shared objects through `libloading` and
//! invoked purely through their exported `driver` symbol — the Rust crate is
//! never linked directly, so the `#[no_mangle] extern "C"` wrapper is part of
//! what is under test.
//!
//! `driver` returns `void`; its entire observable contract is the byte stream
//! it writes to stdout with `printf`. So the harness redirects fd 1 to a
//! temporary file around each call, flushes libc's stream, restores fd 1 and
//! reads the bytes back.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open output stream, which is what we need:
    /// the `printf` calls happen inside the freshly `dlopen`ed objects but they
    /// share this process's libc `stdout`.
    fn fflush(stream: *mut c_void) -> c_int;
}

pub type DriverFn = unsafe extern "C" fn(c_int, c_int);

/// fd 1 is process-global, so only one capture may be in flight at a time.
fn capture_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

pub fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.push("c_src");
    p.push("build");
    p.push("libdriver.so");
    assert!(
        p.exists(),
        "C shared library not built at {p:?}; run:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

/// Guard against the trap that `cargo test` used to fall into here: an
/// integration test that only `dlopen`s the cdylib creates no build dependency
/// on it, so a stale `libdriver.so` would be tested instead of the current
/// source. (`crate-type = ["cdylib", "rlib"]` fixes the dependency; this is the
/// belt-and-braces check.)
fn assert_fresh(so: &std::path::Path) {
    let src = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    let m = |p: &std::path::Path| {
        std::fs::metadata(p)
            .and_then(|md| md.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    };
    assert!(
        m(so) >= m(&src),
        "{so:?} is OLDER than {src:?} — the tests would run against a stale \
         shared object. Run `cargo build --release` (or keep `rlib` in \
         `crate-type` so `cargo test` rebuilds it)."
    );
}

pub fn rust_so_path() -> PathBuf {
    // <target>/<profile>/deps/<test-bin> -> <target>/<profile>/libdriver.so
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent()
        .and_then(|d| d.parent())
        .expect("test binary should live in target/<profile>/deps")
        .to_path_buf();

    // IMPORTANT: `cargo test` rebuilds `target/<profile>/deps/libdriver.so` but
    // does NOT refresh the `target/<profile>/libdriver.so` hardlink (only
    // `cargo build` does). Loading the latter silently tests a stale object, so
    // pick the NEWEST of the two candidates and then verify it is up to date.
    let mut candidates: Vec<PathBuf> = vec![
        profile_dir.join("deps").join("libdriver.so"),
        profile_dir.join("libdriver.so"),
    ];
    candidates.retain(|p| p.exists());
    assert!(
        !candidates.is_empty(),
        "Rust cdylib libdriver.so not found under {profile_dir:?}; run `cargo build --release`"
    );
    candidates.sort_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|md| md.modified())
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
    });
    let newest = candidates.pop().unwrap();
    assert_fresh(&newest);
    newest
}

/// The two libraries, loaded once. `libloading` uses `RTLD_LOCAL`, so the two
/// identically-named `driver` symbols do not collide.
pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| unsafe {
        Libs {
            c: Library::new(c_so_path()).expect("dlopen C libdriver.so"),
            rust: Library::new(rust_so_path()).expect("dlopen Rust libdriver.so"),
        }
    })
}

pub fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0").expect("C `driver` symbol") }
}

pub fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0").expect("Rust `driver` symbol") }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

fn temp_capture_path() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "driver_diff_{}_{}_{}.out",
        std::process::id(),
        n,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

/// Run `f` with fd 1 pointed at a fresh temporary file; return everything it
/// wrote.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock().lock().unwrap_or_else(|e| e.into_inner());

    let path = temp_capture_path();
    let file = std::fs::File::create(&path).expect("create capture file");

    let bytes = unsafe {
        // Flush anything already pending so it is not misattributed. Rust's
        // `Stdout` has its own buffer (libtest's progress banner lives there),
        // so flush that as well as every libc stream.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restore fd 1 failed");
        close(saved);

        let mut buf = Vec::new();
        std::fs::File::open(&path)
            .expect("reopen capture file")
            .read_to_end(&mut buf)
            .expect("read capture file");
        buf
    };

    drop(file);
    let _ = std::fs::remove_file(&path);
    bytes
}

pub fn run_c(x: i32, y: i32) -> Vec<u8> {
    let f = c_driver();
    capture_stdout(|| unsafe { f(x, y) })
}

pub fn run_rust(x: i32, y: i32) -> Vec<u8> {
    let f = rust_driver();
    capture_stdout(|| unsafe { f(x, y) })
}

fn show(bytes: &[u8]) -> String {
    let s = String::from_utf8_lossy(bytes);
    if s.len() <= 600 {
        format!("{:?} ({} bytes)", s, bytes.len())
    } else {
        format!(
            "{:?}...{:?} ({} bytes)",
            &s[..300],
            &s[s.len() - 300..],
            bytes.len()
        )
    }
}

/// Core differential assertion: C and Rust must emit byte-identical stdout.
#[track_caller]
pub fn assert_same(x: i32, y: i32) -> Vec<u8> {
    let c = run_c(x, y);
    let r = run_rust(x, y);
    if c != r {
        // Report the first differing byte to make divergences easy to localise.
        let first = c
            .iter()
            .zip(r.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| c.len().min(r.len()));
        panic!(
            "driver({x}, {y}) diverged at byte {first}\n  C   : {}\n  Rust: {}",
            show(&c),
            show(&r)
        );
    }
    c
}

#[track_caller]
pub fn assert_same_all<I: IntoIterator<Item = (i32, i32)>>(cases: I) {
    for (x, y) in cases {
        assert_same(x, y);
    }
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (fixed seed, reproducible)
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub const DEFAULT_SEED: u64 = 0x2545_F491_4F6C_DD1D;

    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { Self::DEFAULT_SEED } else { seed })
    }

    /// xorshift64*
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// Uniform in `[lo, hi]` inclusive.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}
