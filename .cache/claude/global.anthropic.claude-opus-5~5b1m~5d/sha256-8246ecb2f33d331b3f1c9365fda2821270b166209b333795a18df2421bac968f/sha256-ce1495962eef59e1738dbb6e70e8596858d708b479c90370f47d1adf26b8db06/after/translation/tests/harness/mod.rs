//! Differential-test harness.
//!
//! Loads BOTH the C `.so` and the Rust `.so` with `libloading` and calls
//! `driver` through the FFI boundary in both. The Rust implementation is NEVER
//! called directly as a Rust function — always through the dynamic symbol, so
//! the `#[unsafe(no_mangle)] extern "C"` export wrapper is under test too.
//!
//! `driver` returns `void`; its ONLY observable is the bytes it writes to
//! stdout via `printf`. So the harness redirects file descriptor 1 to a
//! temporary file around each call, then reads the bytes back.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, c_void, CString};
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

// ---------------------------------------------------------------------------
// libc pieces needed for fd redirection
// ---------------------------------------------------------------------------

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn lseek(fd: c_int, offset: i64, whence: c_int) -> i64;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn unlink(path: *const c_char) -> c_int;
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(status: c_int) -> !;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;
const SEEK_SET: c_int = 0;
const STDOUT_FILENO: c_int = 1;

/// Serializes the fork+flush sequence. Not strictly required for correctness of
/// the capture (each child has a private fd table), but it keeps the parent's
/// stdio buffers from being flushed concurrently with a fork.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static COUNTER: AtomicU64 = AtomicU64::new(0);

fn scratch_dir() -> PathBuf {
    std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

/// Run `f` with stdout redirected to a temp file; return the captured bytes.
///
/// The redirection happens in a FORKED CHILD, not in this process. That matters:
/// libtest runs test functions on worker threads while its *main* thread writes
/// progress lines ("test foo ... ok") straight to fd 1. If we redirected this
/// process's fd 1, those progress bytes would be interleaved into the capture
/// file and corrupt the comparison. A forked child gets a private copy of the
/// file-descriptor table, so redirecting fd 1 there is invisible to libtest and
/// the capture contains *only* what the library under test printed.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let n = COUNTER.fetch_add(1, Ordering::SeqCst);
    let path = scratch_dir().join(format!("driver_cap_{}_{}.txt", std::process::id(), n));
    let cpath = CString::new(path.to_str().expect("utf-8 path")).unwrap();

    let out = unsafe {
        let fd = open(cpath.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(fd >= 0, "could not create capture file {}", path.display());

        // Drain both the Rust-side and the C-side stdout buffers BEFORE forking,
        // otherwise the child would inherit a copy of the pending bytes and
        // write them into the capture file (and the parent would write them
        // again later).
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());

        let pid = fork();
        assert!(pid >= 0, "fork() failed");

        if pid == 0 {
            // ---- child ----
            // Point stdout at the capture file, run the calls, flush, leave.
            // `_exit` (not `exit`) so no atexit handler runs twice.
            if dup2(fd, STDOUT_FILENO) < 0 {
                _exit(101);
            }
            let ok = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).is_ok();
            fflush(std::ptr::null_mut());
            _exit(if ok { 0 } else { 102 });
        }

        // ---- parent ----
        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed for child {pid}");
        let exited_normally = (status & 0x7f) == 0;
        let code = (status >> 8) & 0xff;
        assert!(
            exited_normally && code == 0,
            "capture child exited abnormally (raw status {status:#x}, code {code}); \
             the library under test may have crashed"
        );

        lseek(fd, 0, SEEK_SET);
        let mut buf = Vec::new();
        let mut chunk = [0u8; 1 << 16];
        loop {
            let got = read(fd, chunk.as_mut_ptr() as *mut c_void, chunk.len());
            if got <= 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..got as usize]);
        }
        close(fd);
        unlink(cpath.as_ptr());
        buf
    };

    drop(guard);
    out
}

// ---------------------------------------------------------------------------
// Locating and loading the two shared objects
// ---------------------------------------------------------------------------

pub type DriverFn = unsafe extern "C" fn(c_int);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}.\nBuild it with:\n  cd c_src && mkdir -p build && cd build \
         && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

pub fn rust_so_path() -> PathBuf {
    if let Some(p) = std::env::var_os("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    // Prefer whichever profile dir actually has a freshly built cdylib.
    let base = manifest_dir().join("target");
    let mut best: Option<(PathBuf, std::time::SystemTime)> = None;
    for profile in ["release", "debug"] {
        let p = base.join(profile).join("libdriver.so");
        if let Ok(md) = std::fs::metadata(&p) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(_, bt)| t > *bt).unwrap_or(true) {
                best = Some((p, t));
            }
        }
    }
    best.map(|(p, _)| p).unwrap_or_else(|| {
        panic!(
            "Rust cdylib not found under {}.\nBuild it with:  cd translation && cargo build --release",
            base.display()
        )
    })
}

struct Impls {
    c: DriverFn,
    rust: DriverFn,
    // Keep the handles alive for the whole process; the fn pointers above
    // point into these mappings.
    _c_lib: Library,
    _rust_lib: Library,
}

// The two resolved `driver` symbols are plain code pointers into libraries we
// never unload, and `driver` itself touches no shared mutable state.
unsafe impl Send for Impls {}
unsafe impl Sync for Impls {}

static IMPLS: OnceLock<Impls> = OnceLock::new();

fn impls() -> &'static Impls {
    IMPLS.get_or_init(|| unsafe {
        let c_path = c_so_path();
        let r_path = rust_so_path();

        let c_lib = Library::new(&c_path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", c_path.display()));
        let rust_lib = Library::new(&r_path)
            .unwrap_or_else(|e| panic!("dlopen({}) failed: {e}", r_path.display()));

        let c_sym: Symbol<DriverFn> = c_lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("symbol `driver` missing from C .so: {e}"));
        let r_sym: Symbol<DriverFn> = rust_lib
            .get(b"driver\0")
            .unwrap_or_else(|e| panic!("symbol `driver` missing from Rust .so: {e}"));

        let c = *c_sym;
        let rust = *r_sym;

        Impls {
            c,
            rust,
            _c_lib: c_lib,
            _rust_lib: rust_lib,
        }
    })
}

/// The C `driver`, reached only through `dlsym`.
pub fn c_driver() -> DriverFn {
    impls().c
}

/// The Rust `driver`, reached only through `dlsym` on the cdylib.
pub fn rust_driver() -> DriverFn {
    impls().rust
}

// ---------------------------------------------------------------------------
// Differential comparison
// ---------------------------------------------------------------------------

/// What `2*x + 300` must print, per the wrapping 32-bit arithmetic the C
/// compiler actually emits (`lea (%rax,%rax,1),%ebx` then `add $0x12c,%ebx`).
pub fn expected_line(x: i32) -> String {
    format!("{}\n", x.wrapping_mul(2).wrapping_add(300))
}

fn run_all(f: DriverFn, xs: &[i32]) -> Vec<u8> {
    capture(|| unsafe {
        for &x in xs {
            f(x);
        }
    })
}

/// Core differential assertion for row-based tests.
///
/// Calls both `.so`s over `xs`, compares the produced stdout byte-for-byte,
/// and on mismatch re-runs input-by-input to name the exact offending value.
/// Also cross-checks the C output against the independently computed expected
/// text, which keeps a silently-broken capture harness from reporting a
/// vacuous "match".
pub fn assert_same(row: &str, xs: &[i32]) {
    assert!(!xs.is_empty(), "{row}: refusing to assert on an empty input set");

    let c_out = run_all(c_driver(), xs);
    let r_out = run_all(rust_driver(), xs);

    if c_out != r_out {
        // Narrow it down to the first divergent input.
        for &x in xs {
            let c1 = run_all(c_driver(), &[x]);
            let r1 = run_all(rust_driver(), &[x]);
            if c1 != r1 {
                panic!(
                    "{row}: DIVERGENCE at x = {x} (0x{x:08X})\n  C    stdout: {:?}\n  Rust stdout: {:?}",
                    String::from_utf8_lossy(&c1),
                    String::from_utf8_lossy(&r1),
                );
            }
        }
        panic!(
            "{row}: batch outputs differ but no single input diverged \
             (ordering/statefulness bug?)\n  C   len {} \n  Rust len {}",
            c_out.len(),
            r_out.len()
        );
    }

    // Harness sanity: the C output must be exactly the concatenation of the
    // expected lines. If this fails, the capture is lying, not the Rust.
    let expected: String = xs.iter().copied().map(expected_line).collect();
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        expected,
        "{row}: captured C output does not match independently computed text \
         (capture harness problem)"
    );
    assert!(
        !c_out.is_empty(),
        "{row}: captured nothing at all — harness is not observing printf"
    );
}

/// Capture with no calls at all, for the harness self-check row.
pub fn capture_nothing() -> Vec<u8> {
    capture(|| {})
}

/// Interleave C and Rust in a SINGLE captured stream and require that the two
/// lines of each consecutive pair are identical. Detects ordering / buffering /
/// statefulness differences that per-implementation batching cannot see.
pub fn assert_interleaved(row: &str, xs: &[i32]) {
    let c = c_driver();
    let r = rust_driver();
    let out = capture(|| unsafe {
        for &x in xs {
            c(x);
            r(x);
        }
    });
    let text = String::from_utf8_lossy(&out).to_string();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        xs.len() * 2,
        "{row}: expected {} interleaved lines, captured {}",
        xs.len() * 2,
        lines.len()
    );
    for (i, &x) in xs.iter().enumerate() {
        let (cl, rl) = (lines[2 * i], lines[2 * i + 1]);
        assert_eq!(
            cl, rl,
            "{row}: interleaved divergence at x = {x} (0x{x:08X}): C {cl:?} vs Rust {rl:?}"
        );
        assert_eq!(
            format!("{cl}\n"),
            expected_line(x),
            "{row}: unexpected C text at x = {x}"
        );
    }
}

/// Load each `.so` freshly via `dlopen`, call once, drop the handle. Proves
/// there is no load-time / first-call initialisation difference between the two.
pub fn assert_same_fresh_handles(row: &str, x: i32) {
    let c_path = c_so_path();
    let r_path = rust_so_path();

    let c_out = unsafe {
        let lib = Library::new(&c_path).expect("dlopen C");
        let f: Symbol<DriverFn> = lib.get(b"driver\0").expect("dlsym C driver");
        let f = *f;
        capture(|| f(x))
    };
    let r_out = unsafe {
        let lib = Library::new(&r_path).expect("dlopen Rust");
        let f: Symbol<DriverFn> = lib.get(b"driver\0").expect("dlsym Rust driver");
        let f = *f;
        capture(|| f(x))
    };

    assert_eq!(
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
        "{row}: fresh-handle divergence at x = {x} (0x{x:08X})"
    );
    assert_eq!(
        String::from_utf8_lossy(&c_out),
        expected_line(x),
        "{row}: unexpected C text at x = {x}"
    );
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (splitmix64) — fixed seed, no external dependency
// ---------------------------------------------------------------------------

pub struct Rng(u64);

impl Rng {
    /// Fixed seed so every run is reproducible.
    pub fn new() -> Self {
        Rng(0x243F_6A88_85A3_08D3)
    }
    pub fn with_seed(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// Uniform over the FULL i32 range, including both extremes.
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `[lo, hi]` inclusive, computed in i64 to avoid overflow.
    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        debug_assert!(lo <= hi);
        let span = (hi as i64 - lo as i64 + 1) as u64;
        (lo as i64 + (self.next_u64() % span) as i64) as i32
    }
}

/// Every `x` in `[lo, hi]` inclusive.
pub fn inclusive(lo: i32, hi: i32) -> Vec<i32> {
    (lo as i64..=hi as i64).map(|v| v as i32).collect()
}

/// `center` and its neighbours within `±radius`, saturating at the i32 ends.
pub fn around(center: i32, radius: i32) -> Vec<i32> {
    let lo = (center as i64 - radius as i64).max(i32::MIN as i64);
    let hi = (center as i64 + radius as i64).min(i32::MAX as i64);
    (lo..=hi).map(|v| v as i32).collect()
}
