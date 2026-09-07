//! Differential-test harness.
//!
//! Loads BOTH shared objects (the C one built by cmake and the Rust `cdylib`)
//! through `libloading` and calls them only through their exported C symbols.
//! Nothing in the Rust crate is ever called directly, so the `#[no_mangle]`
//! export wrappers are exercised too.
//!
//! `static_sum` keeps a function-scope `static int sum`, i.e. the library owns
//! process-wide hidden state.  To be able to test a given *state
//! configuration* deterministically, `Pair::fresh()` copies each `.so` to a
//! unique temporary path before `dlopen`ing it: glibc keys loaded images by
//! (dev, inode) + name, so a copy is a brand-new image whose `sum` is
//! re-initialised to 0.

#![allow(dead_code)]

use std::ffi::OsStr;
use std::fs;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits we need for stdout capture (declared directly to avoid pulling in
// an extra dependency).
// ---------------------------------------------------------------------------
extern "C" {
    fn fflush(stream: *mut core::ffi::c_void) -> core::ffi::c_int;
    fn dup(oldfd: core::ffi::c_int) -> core::ffi::c_int;
    fn dup2(oldfd: core::ffi::c_int, newfd: core::ffi::c_int) -> core::ffi::c_int;
    fn close(fd: core::ffi::c_int) -> core::ffi::c_int;
}

// ---------------------------------------------------------------------------
// Locating the two shared objects
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn mtime(p: &Path) -> std::time::SystemTime {
    fs::metadata(p)
        .unwrap_or_else(|e| panic!("stat {}: {e}", p.display()))
        .modified()
        .unwrap_or_else(|e| panic!("mtime {}: {e}", p.display()))
}

/// Refuse to run against a shared object that is older than its sources.
///
/// This guard matters a great deal here: `cargo test` does **not** rebuild a
/// `crate-type = ["cdylib"]` artifact (integration tests do not link it), so
/// without this check the suite would happily `dlopen` a stale `.so` and report
/// green for a Rust library that no longer matches `src/`.  Verified by
/// mutation testing: with the guard removed, five deliberate bugs injected into
/// `src/lib.rs` all "passed".
fn assert_not_stale(so: &Path, sources: &[PathBuf], how_to_build: &str) {
    let so_t = mtime(so);
    for s in sources {
        if !s.exists() {
            continue;
        }
        assert!(
            mtime(s) <= so_t,
            "STALE ARTIFACT: {} is older than its source {}.\n\
             The differential tests would be testing an out-of-date library.\n\
             Rebuild with:\n  {how_to_build}",
            so.display(),
            s.display()
        );
    }
}

fn c_src_files() -> Vec<PathBuf> {
    let root = manifest_dir().parent().unwrap().join("c_src");
    vec![
        root.join("src/staticloop.c"),
        root.join("include/staticloop.h"),
        root.join("CMakeLists.txt"),
    ]
}

fn rust_src_files() -> Vec<PathBuf> {
    let root = manifest_dir();
    vec![root.join("src/lib.rs"), root.join("Cargo.toml")]
}

/// Path to the C shared library produced by `c_src/CMakeLists.txt`.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("STATICLOOP_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir()
        .parent()
        .expect("crate has a parent dir")
        .join("c_src/build/libStaticLoop.so");
    assert!(
        p.exists(),
        "C shared library not found at {}\nBuild it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    assert_not_stale(
        &p,
        &c_src_files(),
        "cd c_src/build && cmake --build .",
    );
    p
}

/// Path to the Rust `cdylib`.  Prefers the shipped release artifact, falls
/// back to the debug one.  Override with `STATICLOOP_RUST_SO`.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("STATICLOOP_RUST_SO") {
        let p = PathBuf::from(p);
        assert_not_stale(&p, &rust_src_files(), "cargo build --release");
        return p;
    }
    let base = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libStaticLoop.so");
        if p.exists() {
            assert_not_stale(
                &p,
                &rust_src_files(),
                "cargo build --release   # `cargo test` alone does NOT rebuild a cdylib",
            );
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {}; run `cargo build --release`",
        base.display()
    );
}

// ---------------------------------------------------------------------------
// A freshly-`dlopen`ed private copy of one library
// ---------------------------------------------------------------------------

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn scratch_dir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("staticloop-diff-{}", std::process::id()));
    fs::create_dir_all(&d).expect("create scratch dir");
    d
}

/// One loaded library image, with its own private `sum`.
pub struct Lib {
    /// Kept alive so the mapping (and therefore the symbols) stay valid.
    lib: Library,
    copy: PathBuf,
    pub tag: &'static str,
}

impl Lib {
    fn open(orig: &Path, tag: &'static str) -> Lib {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let copy = scratch_dir().join(format!("{tag}-{n}-libStaticLoop.so"));
        fs::copy(orig, &copy)
            .unwrap_or_else(|e| panic!("copy {} -> {}: {e}", orig.display(), copy.display()));
        // SAFETY: the object is a plain C ABI library with no initialisers
        // beyond the usual CRT ones.
        let lib = unsafe { Library::new(OsStr::new(&copy)) }
            .unwrap_or_else(|e| panic!("dlopen {}: {e}", copy.display()));
        Lib { lib, copy, tag }
    }

    fn sym<T>(&self, name: &[u8]) -> Symbol<'_, T> {
        unsafe { self.lib.get::<T>(name) }.unwrap_or_else(|e| {
            panic!(
                "symbol {:?} missing from {}: {e}",
                String::from_utf8_lossy(name),
                self.copy.display()
            )
        })
    }

    /// `int static_sum(int update)`
    pub fn static_sum(&self, update: i32) -> i32 {
        let f: Symbol<unsafe extern "C" fn(i32) -> i32> = self.sym(b"static_sum\0");
        unsafe { f(update) }
    }

    /// `void driver(int stride)`
    pub fn driver_raw(&self, stride: i32) {
        let f: Symbol<unsafe extern "C" fn(i32)> = self.sym(b"driver\0");
        unsafe { f(stride) }
    }

    /// `driver(stride)`, returning the exact bytes it wrote to stdout.
    pub fn driver(&self, stride: i32) -> Vec<u8> {
        capture_stdout(|| self.driver_raw(stride))
    }
}

impl Drop for Lib {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.copy);
    }
}

// ---------------------------------------------------------------------------
// A matched C/Rust pair, both in a known-fresh state
// ---------------------------------------------------------------------------

pub struct Pair {
    pub c: Lib,
    pub r: Lib,
}

impl Pair {
    /// Two brand-new library images whose hidden `sum` is 0.
    pub fn fresh() -> Pair {
        Pair {
            c: Lib::open(&c_so_path(), "c"),
            r: Lib::open(&rust_so_path(), "rust"),
        }
    }

    /// `static_sum(update)` on both; asserts the returned ints are identical.
    pub fn check_static_sum(&self, update: i32, ctx: &str) -> i32 {
        let cv = self.c.static_sum(update);
        let rv = self.r.static_sum(update);
        assert_eq!(
            cv, rv,
            "static_sum({update}) diverged [{ctx}]: C returned {cv}, Rust returned {rv}"
        );
        cv
    }

    /// `driver(stride)` on both; asserts the stdout bytes are identical.
    pub fn check_driver(&self, stride: i32, ctx: &str) -> Vec<u8> {
        let cv = self.c.driver(stride);
        let rv = self.r.driver(stride);
        assert_eq!(
            cv,
            rv,
            "driver({stride}) stdout diverged [{ctx}]:\n  C   = {:?}\n  Rust= {:?}",
            String::from_utf8_lossy(&cv),
            String::from_utf8_lossy(&rv)
        );
        cv
    }

    /// Drive both libraries' hidden state to the same value by replaying the
    /// same `static_sum` sequence, checking agreement at every step.
    pub fn seed(&self, updates: &[i32], ctx: &str) {
        for (i, &u) in updates.iter().enumerate() {
            self.check_static_sum(u, &format!("{ctx} seed step {i}"));
        }
    }
}

// ---------------------------------------------------------------------------
// stdout capture (fd 1 is process-global, so it must be serialised)
// ---------------------------------------------------------------------------

fn stdout_lock() -> &'static Mutex<()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
}

/// Runs `f` with file descriptor 1 redirected to a temporary file and returns
/// everything written to it.  `fflush(NULL)` is issued on both sides so the
/// C stdio buffer (shared by the C library and by the Rust library, which
/// deliberately calls the very same glibc `printf`) is fully drained.
///
/// fd 1 is process-global, so this must be the only writer while it is
/// redirected:
///   * the `STDOUT_LOCK` mutex serialises concurrent captures;
///   * Rust's own `io::stdout()` `LineWriter` is flushed first, so libtest's
///     partially-buffered progress line (`"test foo ... "`, which carries no
///     newline and would otherwise be flushed *into* our capture) is drained
///     to the real stdout beforehand;
///   * `.cargo/config.toml` sets `RUST_TEST_THREADS=1` so no other test thread
///     can have libtest write progress text while fd 1 is redirected.
/// The last point is verified defensively below: libtest progress text leaking
/// into a capture is reported as a harness error, never as a C/Rust divergence.
pub fn capture_stdout(f: impl FnOnce()) -> Vec<u8> {
    let guard = stdout_lock();
    let _held = guard.lock().unwrap_or_else(|e| e.into_inner());

    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = scratch_dir().join(format!("stdout-{n}.bin"));
    let file = fs::File::create(&path).expect("create capture file");

    let out = unsafe {
        // Drain Rust's line-buffered stdout before stealing fd 1.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        fflush(core::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

        let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        fflush(core::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
        r
    };
    drop(file);

    let mut bytes = Vec::new();
    fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut bytes)
        .expect("read capture file");
    let _ = fs::remove_file(&path);

    match out {
        Ok(()) => {
            assert!(
                !contains(&bytes, b"test ") || !contains(&bytes, b" ... "),
                "HARNESS ERROR: libtest progress text leaked into the stdout capture \
                 ({:?}). Run the suite single-threaded (RUST_TEST_THREADS=1 or \
                 `cargo test -- --test-threads=1`).",
                String::from_utf8_lossy(&bytes)
            );
            bytes
        }
        Err(p) => std::panic::resume_unwind(p),
    }
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seeds keep every run reproducible
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform over the whole `i32` domain.
    pub fn i32_any(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive.
    pub fn i32_in(&mut self, lo: i32, hi: i32) -> i32 {
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

/// Every "interesting" `i32`: 0, ±1, ±2^k, ±(2^k − 1), INT_MIN, INT_MAX and
/// their neighbours.  Used to sweep the full bit-pattern space of the domain.
pub fn boundary_i32s() -> Vec<i32> {
    let mut v = vec![0i32, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
    for k in 0..32u32 {
        let p = 1u32 << k;
        v.push(p as i32);
        v.push((p as i32).wrapping_neg());
        v.push((p as i32).wrapping_sub(1));
        v.push((p as i32).wrapping_sub(1).wrapping_neg());
    }
    v.sort_unstable();
    v.dedup();
    v
}
