//! Shared differential-test harness.
//!
//! Loads BOTH shared libraries through `libloading`:
//!
//!   * the C one  — `c_src/build/libdriver.so`
//!   * the Rust one — `translation/target/<profile>/libdriver.so`
//!
//! Nothing in this harness calls a Rust translation function directly; every
//! call goes through `dlsym` on the `.so`, exactly as an external C consumer
//! would, so the `#[no_mangle] extern "C"` wrappers are under test too.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

// ---------------------------------------------------------------------------
// libc bits the harness itself needs (fd plumbing + FILE* flushing).
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* open output streams, which is exactly what
    /// we need: both `.so`s write through the one process-wide glibc `stdout`.
    fn fflush(stream: *mut c_void) -> c_int;
}

// ---------------------------------------------------------------------------
// Locating and loading the two libraries.
// ---------------------------------------------------------------------------

pub struct Libs {
    pub c: Library,
    pub rust: Library,
    pub c_path: PathBuf,
    pub rust_path: PathBuf,
}

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ must have a parent")
        .to_path_buf()
}

/// `target/<profile>/libdriver.so` for whichever profile this test binary was
/// built with. Derived from the test executable's own location
/// (`target/<profile>/deps/<test>-<hash>`) so it works for `cargo test`,
/// `cargo test --release`, and custom target dirs alike.
fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let profile_dir = exe
        .parent() // deps/
        .and_then(|p| p.parent()) // <profile>/
        .expect("test exe layout")
        .to_path_buf();
    let direct = profile_dir.join("libdriver.so");
    if direct.exists() {
        return direct;
    }
    // Fall back to the sibling profile dir (e.g. tests built in debug while
    // only a release cdylib exists).
    for prof in ["release", "debug"] {
        let cand = profile_dir
            .parent()
            .map(|t| t.join(prof).join("libdriver.so"))
            .unwrap_or_default();
        if cand.exists() {
            return cand;
        }
    }
    direct
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    workspace_root().join("c_src/build/libdriver.so")
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| {
        let c_path = c_so_path();
        let rust_path = rust_so_path();
        assert!(
            c_path.exists(),
            "C shared library not found at {c_path:?}. Build it with:\n  \
             cd c_src && mkdir -p build && cd build && \
             cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build ."
        );
        assert!(
            rust_path.exists(),
            "Rust shared library not found at {rust_path:?}. Build it with:\n  \
             cd translation && cargo build"
        );
        // SAFETY: both paths point at plain C-ABI shared objects with no
        // constructors that could violate Rust's invariants.
        let c = unsafe { Library::new(&c_path) }.expect("dlopen C libdriver.so");
        let rust = unsafe { Library::new(&rust_path) }.expect("dlopen Rust libdriver.so");
        Libs {
            c,
            rust,
            c_path,
            rust_path,
        }
    })
}

// Signatures of the five exported symbols.
pub type FnPrintLine = unsafe extern "C" fn(*const c_char);
pub type FnPrintIntLine = unsafe extern "C" fn(c_int);
pub type FnBad = unsafe extern "C" fn(c_int);
pub type FnGood = unsafe extern "C" fn(c_int);
pub type FnDriver = unsafe extern "C" fn(c_int, c_int);

fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    unsafe { lib.get(format!("{name}\0").as_bytes()) }
        .unwrap_or_else(|e| panic!("dlsym {name} failed: {e}"))
}

/// The five entry points of one library, resolved via `dlsym`.
pub struct Api {
    pub which: &'static str,
    pub print_line: Symbol<'static, FnPrintLine>,
    pub print_int_line: Symbol<'static, FnPrintIntLine>,
    pub bad: Symbol<'static, FnBad>,
    pub good: Symbol<'static, FnGood>,
    pub driver: Symbol<'static, FnDriver>,
}

fn api_of(which: &'static str, lib: &'static Library) -> Api {
    Api {
        which,
        print_line: sym(lib, "printLine"),
        print_int_line: sym(lib, "printIntLine"),
        bad: sym(lib, "bad"),
        good: sym(lib, "good"),
        driver: sym(lib, "driver"),
    }
}

static C_API: OnceLock<Api> = OnceLock::new();
static RUST_API: OnceLock<Api> = OnceLock::new();

pub fn c_api() -> &'static Api {
    C_API.get_or_init(|| api_of("C", &libs().c))
}
pub fn rust_api() -> &'static Api {
    RUST_API.get_or_init(|| api_of("Rust", &libs().rust))
}

// ---------------------------------------------------------------------------
// stdout capture at the file-descriptor level.
// ---------------------------------------------------------------------------

/// Serializes fd-1 redirection: `cargo test` runs test fns concurrently.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

fn lock() -> MutexGuard<'static, ()> {
    match CAPTURE_LOCK.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Runs `f` with fd 1 redirected into a temporary file and returns everything
/// written to it (by *either* library, through glibc `printf`/`puts`).
///
/// A real file — not a pipe — is used so that outputs larger than the 64 KiB
/// pipe buffer (see the 64 KiB `printLine` case) cannot deadlock.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _g = lock();

    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let mut path = std::env::temp_dir();
    path.push(format!("driver_capture_{}_{}.out", std::process::id(), n));

    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("create capture file");
    let fd = file.as_raw_fd();

    unsafe {
        // Flush anything already pending on the real stdout so it does not
        // leak into the capture. Two independent buffers must be drained:
        //  * Rust's `std::io::Stdout` LineWriter, which holds libtest's
        //    partial `"test <name> ... "` progress line, and
        //  * glibc's `FILE *stdout`, used by both libraries under test.
        let _ = std::io::Write::flush(&mut std::io::stdout());
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 -> 1 failed");

        f();

        // Force both libraries' buffered output out before restoring fd 1.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }

    let mut out = Vec::new();
    {
        use std::io::Seek;
        let mut file = file;
        file.rewind().expect("rewind capture file");
        file.read_to_end(&mut out).expect("read capture file");
    }
    let _ = std::fs::remove_file(&path);
    out
}

// ---------------------------------------------------------------------------
// Differential assertion.
// ---------------------------------------------------------------------------

fn render(bytes: &[u8]) -> String {
    let shown: Vec<u8> = bytes.iter().copied().take(512).collect();
    format!(
        "{} bytes{}: {:?}",
        bytes.len(),
        if bytes.len() > 512 { " (first 512)" } else { "" },
        String::from_utf8_lossy(&shown)
    )
}

/// Core differential check: run `op` against the C API and against the Rust
/// API, capturing each one's stdout, and require the bytes to be identical.
pub fn diff<F>(label: &str, op: F)
where
    F: Fn(&Api),
{
    let c_out = capture(|| op(c_api()));
    let rust_out = capture(|| op(rust_api()));
    if c_out != rust_out {
        let first = c_out
            .iter()
            .zip(rust_out.iter())
            .position(|(a, b)| a != b)
            .unwrap_or(c_out.len().min(rust_out.len()));
        panic!(
            "DIVERGENCE [{label}] first differing byte at offset {first}\n  \
             C   : {}\n  Rust: {}",
            render(&c_out),
            render(&rust_out)
        );
    }
}

// Convenience wrappers, one per entry point.

pub fn diff_print_line(label: &str, s: Option<&[u8]>) {
    match s {
        None => diff(label, |api| unsafe { (api.print_line)(std::ptr::null()) }),
        Some(bytes) => {
            let cs = CString::new(bytes).expect("no interior NUL");
            diff(label, |api| unsafe { (api.print_line)(cs.as_ptr()) });
        }
    }
}

pub fn diff_print_int_line(label: &str, v: c_int) {
    diff(label, |api| unsafe { (api.print_int_line)(v) });
}

pub fn diff_bad(label: &str, v: c_int) {
    diff(label, |api| unsafe { (api.bad)(v) });
}

pub fn diff_good(label: &str, v: c_int) {
    diff(label, |api| unsafe { (api.good)(v) });
}

pub fn diff_driver(label: &str, g: c_int, b: c_int) {
    diff(label, |api| unsafe { (api.driver)(g, b) });
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (xorshift64*) — fixed seed => reproducible runs.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x2025_0905_D8_1EA7;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed })
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn next_i32(&mut self) -> i32 {
        (self.next_u64() >> 32) as u32 as i32
    }
    /// Uniform in `0..n` (n > 0).
    pub fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + self.below(span) as i64) as i32
    }
}

/// The C `bad()` writes `buffer[data]` with no upper bound. Indices 0..=11 stay
/// inside the frame's own slack (`-0x30..-0x4` relative to `%rbp`, verified by
/// disassembly); index 12 hits the saved `%rbp` and 14 the return address, so
/// `data >= 12` is genuine UB and is excluded from differential testing.
/// See ERRORS.md row 9.
pub const BAD_MAX_DEFINED: i32 = 11;
