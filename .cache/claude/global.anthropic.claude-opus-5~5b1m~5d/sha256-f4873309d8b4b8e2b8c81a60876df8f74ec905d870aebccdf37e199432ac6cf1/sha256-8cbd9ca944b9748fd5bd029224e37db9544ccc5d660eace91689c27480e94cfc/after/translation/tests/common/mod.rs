//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both shared objects are loaded with `libloading` and every call goes through
//! the `.so`'s exported symbols — the Rust crate is never linked or called
//! directly, so the `#[no_mangle] extern "C"` wrappers are under test too.
//!
//! Both libraries write to `stdout` via libc `printf`, so output is captured by
//! temporarily redirecting file descriptor 1 to a pipe-backed temp file and
//! flushing all `stdio` streams afterwards.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

// ---------------------------------------------------------------------------
// libc bits used only by the harness (not part of the code under test)
// ---------------------------------------------------------------------------
extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream, which covers the
    /// `stdout` used by both the C `.so` and the Rust `.so` (they share the
    /// process's single libc).
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Library loading
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

fn c_so_path() -> PathBuf {
    let p = repo_root().join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {p:?}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build && \\\n    cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
    );
    p
}

fn rust_so_path() -> PathBuf {
    // `DRIVER_RUST_SO` lets the runner pin an exact artifact so that BOTH the
    // debug and the release (`panic = "abort"`, opt-level 3) `.so` can be
    // verified; optimisation level can change float codegen, so both matter.
    if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
        let p = PathBuf::from(p);
        assert!(p.exists(), "DRIVER_RUST_SO points at a missing file: {p:?}");
        return p;
    }
    let base = repo_root().join("translation/target");
    // Prefer the release artifact (the shipped one, built with panic=abort),
    // fall back to debug so `cargo test` works either way.
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust shared library not found under {base:?}.\nBuild it with:\n  cd translation && cargo build --release"
    );
}

/// `cargo test` builds the test binaries but does NOT rebuild a `cdylib`, so a
/// stale `.so` would silently produce false passes (a mutated `src/lib.rs`
/// would still "pass"). Refuse to run unless each `.so` is newer than its
/// sources.
fn assert_fresh(so: &PathBuf, sources: &[PathBuf], rebuild_hint: &str) {
    let so_time = std::fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("cannot stat {so:?}: {e}"));
    for src in sources {
        let Ok(src_time) = std::fs::metadata(src).and_then(|m| m.modified()) else {
            continue;
        };
        assert!(
            so_time >= src_time,
            "STALE ARTIFACT: {so:?} is older than {src:?}.\n\
             `cargo test` does not rebuild a cdylib, so the tests would be \
             comparing an out-of-date library and could pass vacuously.\n\
             Rebuild first:\n  {rebuild_hint}"
        );
    }
}

/// The two loaded libraries plus the capture lock.
pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        let root = repo_root();
        let cp = c_so_path();
        let rp = rust_so_path();
        assert_fresh(
            &cp,
            &[
                root.join("c_src/src/driver.c"),
                root.join("c_src/include/driver.h"),
            ],
            "cd c_src/build && cmake --build .",
        );
        assert_fresh(
            &rp,
            &[root.join("translation/src/lib.rs")],
            "cd translation && cargo build --release && cargo build",
        );
        let c = unsafe { Library::new(&cp) }.expect("failed to dlopen the C libdriver.so");
        let rust = unsafe { Library::new(&rp) }.expect("failed to dlopen the Rust libdriver.so");
        Libs { c, rust }
    })
}

/// fd-1 redirection is process-global, so captures must be serialized.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

// ---------------------------------------------------------------------------
// Typed symbol accessors — every call crosses the FFI boundary.
// ---------------------------------------------------------------------------

pub type FnPrintLine = unsafe extern "C" fn(*const c_char);
pub type FnPrintIntLine = unsafe extern "C" fn(c_int);
pub type FnFloat1 = unsafe extern "C" fn(f32);
pub type FnFloat2 = unsafe extern "C" fn(f32, f32);

/// Which of the two implementations to invoke.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

impl Impl {
    fn lib(self) -> &'static Library {
        match self {
            Impl::C => &libs().c,
            Impl::Rust => &libs().rust,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Impl::C => "C",
            Impl::Rust => "Rust",
        }
    }
}

pub const BOTH: [Impl; 2] = [Impl::C, Impl::Rust];

fn sym<T>(imp: Impl, name: &str) -> Symbol<'static, T> {
    unsafe { imp.lib().get::<T>(name.as_bytes()) }
        .unwrap_or_else(|e| panic!("symbol `{name}` missing from the {} .so: {e}", imp.name()))
}

/// A bundle of all five exported entry points for one implementation.
pub struct Api {
    pub print_line: Symbol<'static, FnPrintLine>,
    pub print_int_line: Symbol<'static, FnPrintIntLine>,
    pub bad: Symbol<'static, FnFloat1>,
    pub good: Symbol<'static, FnFloat1>,
    pub driver: Symbol<'static, FnFloat2>,
}

pub fn api(imp: Impl) -> Api {
    Api {
        print_line: sym::<FnPrintLine>(imp, "printLine"),
        print_int_line: sym::<FnPrintIntLine>(imp, "printIntLine"),
        bad: sym::<FnFloat1>(imp, "bad"),
        good: sym::<FnFloat1>(imp, "good"),
        driver: sym::<FnFloat2>(imp, "driver"),
    }
}

// ---------------------------------------------------------------------------
// stdout capture
// ---------------------------------------------------------------------------

/// libtest writes its own progress text (`test foo ... ok`) to fd 1. With more
/// than one test thread those writes can land inside our capture window, so the
/// suite must run single-threaded. `translation/.cargo/config.toml` sets
/// `RUST_TEST_THREADS=1`; this check fails loudly if it was overridden.
fn assert_single_threaded() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let v = std::env::var("RUST_TEST_THREADS").unwrap_or_default();
        assert_eq!(
            v, "1",
            "stdout capture requires RUST_TEST_THREADS=1 (got {v:?}); \
             run via `cargo test` so translation/.cargo/config.toml applies, \
             or pass `-- --test-threads=1`"
        );
    });
}

/// Runs `body`, capturing everything the process writes to fd 1 while it runs.
pub fn capture<F: FnOnce()>(body: F) -> Vec<u8> {
    assert_single_threaded();
    let _guard = capture_lock();

    // Flush anything already pending so it does not land in our capture:
    // both libc's stdio buffers and Rust's own `std::io::stdout` buffer
    // (libtest's "test foo ... " prefix lives in the latter).
    {
        use std::io::Write;
        let _ = std::io::stdout().flush();
    }
    unsafe { fflush(std::ptr::null_mut()) };

    let mut tmp = tempfile();
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(tmp.as_raw_fd(), 1) } >= 0, "dup2 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(body));

    // Flush the library's stdio buffer *before* restoring fd 1.
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };

    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }

    let mut out = Vec::new();
    tmp.seek(SeekFrom::Start(0)).expect("seek temp file");
    tmp.read_to_end(&mut out).expect("read temp file");
    out
}

fn tempfile() -> std::fs::File {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let dir = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = dir.join(format!(
        "driver-difftest-{}-{}.out",
        std::process::id(),
        n
    ));
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap_or_else(|e| panic!("cannot create temp file {path:?}: {e}"));
    // Unlink immediately; the open fd keeps it alive.
    let _ = std::fs::remove_file(&path);
    f
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// Runs the same closure against the C and the Rust `.so`, capturing stdout for
/// each, and asserts the two byte streams are identical.
pub fn diff<F: Fn(&Api)>(what: &str, body: F) {
    let c_api = api(Impl::C);
    let rust_api = api(Impl::Rust);

    let c_out = capture(|| body(&c_api));
    let rust_out = capture(|| body(&rust_api));

    if c_out != rust_out {
        panic!(
            "DIVERGENCE for {what}\n  C    ({:>4} bytes): {}\n  Rust ({:>4} bytes): {}\n  first diff at byte {}",
            c_out.len(),
            render(&c_out),
            rust_out.len(),
            render(&rust_out),
            first_diff(&c_out, &rust_out)
                .map(|i| i.to_string())
                .unwrap_or_else(|| "length only".into()),
        );
    }
}

fn first_diff(a: &[u8], b: &[u8]) -> Option<usize> {
    a.iter().zip(b.iter()).position(|(x, y)| x != y)
}

/// Escaped, length-capped rendering for assertion messages.
pub fn render(bytes: &[u8]) -> String {
    const CAP: usize = 220;
    let mut s = String::new();
    for &b in bytes.iter().take(CAP) {
        match b {
            b'\n' => s.push_str("\\n"),
            b'\r' => s.push_str("\\r"),
            b'\t' => s.push_str("\\t"),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => s.push_str(&format!("\\x{b:02x}")),
        }
    }
    if bytes.len() > CAP {
        s.push_str(&format!("... (+{} bytes)", bytes.len() - CAP));
    }
    format!("\"{s}\"")
}

// ---------------------------------------------------------------------------
// Helpers for building arguments
// ---------------------------------------------------------------------------

/// NUL-terminates `bytes` so it can be handed to `printLine` as a `const char*`.
pub fn cstr(bytes: &[u8]) -> Vec<u8> {
    assert!(
        !bytes.contains(&0),
        "test string must not contain interior NUL"
    );
    let mut v = bytes.to_vec();
    v.push(0);
    v
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seeds keep runs reproducible.
// ---------------------------------------------------------------------------

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
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Uniform in `[lo, hi)`.
    pub fn below(&mut self, hi: usize) -> usize {
        assert!(hi > 0);
        (self.next_u64() % hi as u64) as usize
    }

    pub fn range(&mut self, lo: usize, hi: usize) -> usize {
        assert!(hi > lo);
        lo + self.below(hi - lo)
    }

    /// A uniformly random `f32` bit pattern (includes subnormals, ±inf, NaNs).
    pub fn any_f32(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }

    /// A "normal-looking" finite float whose magnitude spans many decades.
    pub fn finite_f32(&mut self, min_abs: f32, max_abs: f32) -> f32 {
        let lmin = min_abs.ln();
        let lmax = max_abs.ln();
        let t = (self.next_u32() as f64) / (u32::MAX as f64);
        let mag = ((lmin as f64) + t * ((lmax - lmin) as f64)).exp() as f32;
        if self.next_u64() & 1 == 0 {
            mag
        } else {
            -mag
        }
    }
}
