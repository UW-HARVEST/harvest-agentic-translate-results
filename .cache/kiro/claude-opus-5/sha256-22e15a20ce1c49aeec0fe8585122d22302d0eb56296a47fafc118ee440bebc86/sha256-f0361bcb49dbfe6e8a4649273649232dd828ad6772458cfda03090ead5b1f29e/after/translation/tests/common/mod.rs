//! Shared harness for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! called only through their exported symbols, exactly as an external consumer
//! would. The Rust functions are never called directly, so the
//! `#[no_mangle] extern "C"` export wrappers are under test too.
//!
//! The library's only observable effect is the bytes it writes to `stdout` via
//! libc `printf`, so the harness captures stdout at the *file-descriptor* level
//! (`dup`/`dup2`), which sees writes from both `.so`s regardless of which
//! runtime issued them.

#![allow(dead_code)]

use libloading::Library;
use std::ffi::{c_char, c_int, c_void};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *every* open output stream, including the
    /// `stdout` shared by the C `.so`, the Rust `.so`, and this test binary.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// The exported surface of one implementation.
///
/// Each symbol is looked up twice, under two different function signatures:
///
/// * `..._char` — the ABI-correct signature `void f(char)`.
/// * `..._int`  — `void f(int)`, used to present the symbol with a full-width
///   argument that has no `char` representation. C performs no check on such a
///   value, so it is a real input both implementations must handle identically
///   (rows C12/C13 in `CONFIGS.md`, E6/E12 in `ERRORS.md`).
pub struct Api {
    pub name: &'static str,
    pub print_hex_char_line: unsafe extern "C" fn(c_char),
    pub driver: unsafe extern "C" fn(c_char),
    pub print_hex_char_line_int: unsafe extern "C" fn(c_int),
    pub driver_int: unsafe extern "C" fn(c_int),
    // Keep the handle alive for the process lifetime; the fn pointers above
    // borrow from the mapped image.
    _lib: Library,
}

// SAFETY: `libloading::Library` is `Send + Sync`, and bare `extern "C"` fn
// pointers are too. Access to the (process-global) stdout the functions write
// to is serialised by `CAPTURE_LOCK`.
unsafe impl Send for Api {}
unsafe impl Sync for Api {}

impl Api {
    fn load(name: &'static str, path: &PathBuf) -> Api {
        // `Library::new` uses RTLD_LOCAL, so the two `.so`s can both export
        // `driver`/`printHexCharLine` without colliding: each `get` resolves
        // within its own handle.
        let lib = unsafe { Library::new(path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {} ({}): {e}", path.display(), name));
        unsafe {
            let print_hex_char_line = *lib
                .get::<unsafe extern "C" fn(c_char)>(b"printHexCharLine\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol printHexCharLine: {e}"));
            let driver = *lib
                .get::<unsafe extern "C" fn(c_char)>(b"driver\0")
                .unwrap_or_else(|e| panic!("{name}: missing symbol driver: {e}"));
            let print_hex_char_line_int = *lib
                .get::<unsafe extern "C" fn(c_int)>(b"printHexCharLine\0")
                .unwrap();
            let driver_int = *lib.get::<unsafe extern "C" fn(c_int)>(b"driver\0").unwrap();
            Api {
                name,
                print_hex_char_line,
                driver,
                print_hex_char_line_int,
                driver_int,
                _lib: lib,
            }
        }
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_C_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  \
         cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("DRIVER_RUST_SO") {
        return PathBuf::from(p);
    }
    // Prefer the release artifact (the one that actually ships, built with
    // `panic = "abort"`), fall back to the dev cdylib that `cargo test` builds.
    let root = manifest_dir().join("target");
    for profile in ["release", "debug"] {
        let p = root.join(profile).join("libdriver.so");
        if p.exists() {
            return p;
        }
    }
    panic!(
        "Rust cdylib not found under {}. Build it with `cargo build --release`.",
        root.display()
    );
}

/// The two implementations under comparison: `(c, rust)`.
pub fn apis() -> &'static (Api, Api) {
    static APIS: OnceLock<(Api, Api)> = OnceLock::new();
    APIS.get_or_init(|| {
        (
            Api::load("C", &c_so_path()),
            Api::load("Rust", &rust_so_path()),
        )
    })
}

pub fn c() -> &'static Api {
    &apis().0
}

pub fn rs() -> &'static Api {
    &apis().1
}

/// Serialises stdout redirection: `cargo test` runs tests on multiple threads,
/// and fd 1 is process-global.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
static CAPTURE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Runs `f` with fd 1 redirected to a temporary file and returns everything
/// written to it.
///
/// `fflush(NULL)` is issued both before the redirect (so unrelated buffered
/// output does not land in the capture) and after `f` (so `printf`'s buffered
/// bytes are in the file before it is read).
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        CAPTURE_SEQ.fetch_add(1, Ordering::Relaxed)
    ));

    let file = File::create(&path).expect("create capture file");

    let bytes = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");

        // The call under test happens here, writing through libc `stdout`.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore of stdout failed");
        close(saved);

        if let Err(payload) = result {
            drop(file);
            let _ = std::fs::remove_file(&path);
            std::panic::resume_unwind(payload);
        }

        drop(file);
        std::fs::read(&path).expect("read capture file")
    };

    let _ = std::fs::remove_file(&path);
    bytes
}

// ---------------------------------------------------------------------------
// Single-call capture helpers
// ---------------------------------------------------------------------------

pub fn cap_print_hex(api: &Api, v: c_char) -> Vec<u8> {
    capture(|| unsafe { (api.print_hex_char_line)(v) })
}

pub fn cap_driver(api: &Api, v: c_char) -> Vec<u8> {
    capture(|| unsafe { (api.driver)(v) })
}

pub fn cap_print_hex_int(api: &Api, v: c_int) -> Vec<u8> {
    capture(|| unsafe { (api.print_hex_char_line_int)(v) })
}

pub fn cap_driver_int(api: &Api, v: c_int) -> Vec<u8> {
    capture(|| unsafe { (api.driver_int)(v) })
}

/// Asserts the two byte strings are identical, rendering them readably on
/// failure (the payloads are ASCII hex lines).
#[track_caller]
pub fn assert_same(context: &str, c_out: &[u8], rust_out: &[u8]) {
    if c_out != rust_out {
        panic!(
            "DIVERGENCE ({context})\n  C    : {:?} = {:02x?}\n  Rust : {:?} = {:02x?}",
            String::from_utf8_lossy(c_out),
            c_out,
            String::from_utf8_lossy(rust_out),
            rust_out,
        );
    }
}

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — fixed seed for reproducibility
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_C0FF_EE00_1234;

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

    pub fn next_u32(&mut self) -> u32 {
        (self.next_u64() >> 32) as u32
    }

    /// Uniform-ish value in `lo..=hi` (inclusive), for small ranges.
    pub fn in_range_u8(&mut self, lo: u8, hi: u8) -> u8 {
        debug_assert!(lo <= hi);
        let span = (hi - lo) as u64 + 1;
        lo + (self.next_u64() % span) as u8
    }
}

/// Number of randomized samples per `CONFIGS.md` row.
pub const SAMPLES: usize = 512;

// ---------------------------------------------------------------------------
// Sequential row runner
// ---------------------------------------------------------------------------
//
// These test binaries use `harness = false`. That is not a stylistic choice:
// the only observable output of this library is bytes on file descriptor 1, so
// `capture` must temporarily `dup2` over fd 1, which is process-global. libtest
// prints its own progress lines ("test foo ... ok") to the same fd from the
// main thread while test threads are still running, so under the default
// parallel harness those lines land inside the capture file and corrupt the
// comparison. A `harness = false` binary that runs every row sequentially from
// `main` removes the interference entirely and keeps per-row reporting.

pub struct Runner {
    passed: Vec<&'static str>,
    failed: Vec<(&'static str, String)>,
}

impl Runner {
    pub fn new() -> Runner {
        Runner {
            passed: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// Runs one `CONFIGS.md`/`ERRORS.md` row, recording pass/fail.
    pub fn row<F: FnOnce()>(&mut self, name: &'static str, f: F) {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
        match result {
            Ok(()) => {
                println!("row {name} ... ok");
                self.passed.push(name);
            }
            Err(payload) => {
                let msg = if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = payload.downcast_ref::<&str>() {
                    (*s).to_string()
                } else {
                    "<non-string panic payload>".to_string()
                };
                println!("row {name} ... FAILED");
                self.failed.push((name, msg));
            }
        }
    }

    /// Prints the summary and exits non-zero if any row failed.
    pub fn finish(self, phase: &str) {
        println!();
        if self.failed.is_empty() {
            println!(
                "{phase}: all {} rows passed",
                self.passed.len()
            );
            return;
        }
        println!("{phase} FAILURES:");
        for (name, msg) in &self.failed {
            println!("---- {name} ----\n{msg}\n");
        }
        println!(
            "{phase}: {} passed, {} FAILED",
            self.passed.len(),
            self.failed.len()
        );
        std::process::exit(1);
    }
}

impl Default for Runner {
    fn default() -> Runner {
        Runner::new()
    }
}

/// Quietens the default panic hook so a deliberately-failing row does not
/// interleave a backtrace with the runner's own report. The payload is still
/// recovered by `catch_unwind`.
pub fn install_quiet_panic_hook() {
    std::panic::set_hook(Box::new(|_| {}));
}

