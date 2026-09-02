//! Shared differential-test harness.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! called only through their exported `driver` symbol — the Rust crate is never
//! linked or called directly, so these tests exercise the `#[no_mangle]`
//! `extern "C"` wrapper exactly as an external C consumer would.
//!
//! `driver` returns `void` and reports everything through `printf`, so the only
//! observable output is the byte stream written to file descriptor 1. Capture
//! works by `dup2`-ing a temporary file over fd 1, invoking the symbol,
//! `fflush(NULL)`-ing every libc stream (both `.so`s share the process's libc
//! stdio buffers), then restoring fd 1 and reading the bytes back.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_void;
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::AsRawFd;
use std::os::raw::c_int;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Mutex, MutexGuard, OnceLock};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Signature of the one exported symbol: `void driver(int x, int y, int z)`.
pub type DriverFn = unsafe extern "C" fn(c_int, c_int, c_int);

/// Serialises fd-1 redirection, which is process-global.
static CAPTURE_LOCK: Mutex<()> = Mutex::new(());

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Locate the C shared object produced by the CMake build.
fn c_so_path() -> PathBuf {
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

/// Build the Rust `cdylib` and return its path.
///
/// This is NOT optional: with `crate-type = ["cdylib"]`, `cargo test` compiles
/// `src/lib.rs` only as a test harness — it never produces `libdriver.so`. A
/// harness that merely *looked* for the `.so` would happily `dlopen` a stale
/// artifact from an earlier `cargo build --release` and pass no matter what the
/// current source says (verified: injected mutations went undetected that way).
///
/// An isolated `--target-dir` is used so this nested `cargo` never contends for
/// the outer `cargo test`'s build lock.
fn rust_so_path() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let manifest = manifest_dir();
            let target_dir = manifest.join("target/difftest");
            let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());

            let mut cmd = Command::new(&cargo);
            cmd.current_dir(&manifest)
                .arg("build")
                .arg("--release")
                .arg("--lib")
                .arg("--target-dir")
                .arg(&target_dir)
                .stdout(Stdio::null())
                .stderr(Stdio::piped());

            // Strip the outer cargo's per-invocation environment so the nested
            // build is not misled by it.
            for (k, _) in std::env::vars() {
                if (k.starts_with("CARGO_") && k != "CARGO_HOME") || k == "RUSTC_WRAPPER" {
                    cmd.env_remove(k);
                }
            }

            let out = cmd
                .output()
                .unwrap_or_else(|e| panic!("failed to spawn `{cargo} build`: {e}"));
            assert!(
                out.status.success(),
                "building the Rust cdylib failed:\n{}",
                String::from_utf8_lossy(&out.stderr)
            );

            let so = target_dir.join("release/libdriver.so");
            assert!(
                so.exists(),
                "cargo build succeeded but {} is missing",
                so.display()
            );

            // Freshness guard: the .so must be newer than every source file.
            let so_mtime = std::fs::metadata(&so).and_then(|m| m.modified()).unwrap();
            for entry in std::fs::read_dir(manifest.join("src")).expect("read src/") {
                let p = entry.expect("dir entry").path();
                let src_mtime = std::fs::metadata(&p).and_then(|m| m.modified()).unwrap();
                assert!(
                    so_mtime >= src_mtime,
                    "{} is older than {} — stale artifact",
                    so.display(),
                    p.display()
                );
            }
            so
        })
        .clone()
}

/// The two loaded libraries plus their resolved `driver` symbols.
pub struct Libs {
    _c_lib: Library,
    _rust_lib: Library,
    pub c_driver: DriverFn,
    pub rust_driver: DriverFn,
}

impl Libs {
    fn load() -> Self {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("failed to dlopen the C libdriver.so");
            let rust_lib =
                Library::new(rust_so_path()).expect("failed to dlopen the Rust libdriver.so");

            let c_sym: Symbol<DriverFn> = c_lib
                .get(b"driver\0")
                .expect("symbol `driver` missing from the C .so");
            let rust_sym: Symbol<DriverFn> = rust_lib
                .get(b"driver\0")
                .expect("symbol `driver` missing from the Rust .so — check #[no_mangle]");

            let c_driver = *c_sym;
            let rust_driver = *rust_sym;

            Libs {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c_driver,
                rust_driver,
            }
        }
    }
}

/// Both libraries stay loaded for the lifetime of the test binary so that each
/// one's file-scope `static int y` keeps its state across calls, mirroring how
/// a real consumer would use them.
pub fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(Libs::load)
}

/// Take the capture lock. Hold it across a whole call *sequence* when the test
/// depends on the libraries' residual `static y` state.
pub fn capture_guard() -> MutexGuard<'static, ()> {
    CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Run `f` with fd 1 redirected to a temporary file and return everything
/// written to it. Assumes the caller already holds [`capture_guard`].
///
/// Both Rust's own `Stdout` buffer and every libc `FILE*` are flushed before the
/// redirect, so no pre-existing pending bytes (notably libtest's unterminated
/// `"test <name> ... "` progress line) can be misattributed to `f`.
fn capture_locked<F: FnOnce()>(f: F) -> Vec<u8> {
    let mut file = tempfile::anonymous();

    unsafe {
        // Drain Rust's line-buffered stdout, then every libc stream, so that
        // nothing already pending is attributed to this call.
        let _ = std::io::stdout().flush();
        fflush(std::ptr::null_mut());

        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto fd 1 failed");

        f();

        // Flush every libc stream: the C .so writes through its own
        // (process-shared) stdio buffers, which are full-buffered on a file.
        fflush(std::ptr::null_mut());

        assert!(dup2(saved, 1) >= 0, "failed to restore fd 1");
        close(saved);
    }

    let mut out = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek temp file");
    file.read_to_end(&mut out).expect("read temp file");
    out
}

/// Minimal anonymous-temp-file helper (avoids pulling in an extra crate).
mod tempfile {
    use std::fs::{File, OpenOptions};
    use std::sync::atomic::{AtomicU64, Ordering};

    pub fn anonymous() -> File {
        static N: AtomicU64 = AtomicU64::new(0);
        let dir = std::env::temp_dir();
        let name = format!(
            "driver-difftest-{}-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let path = dir.join(name);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("create temp capture file");
        // Unlink immediately; the open fd keeps the inode alive.
        let _ = std::fs::remove_file(&path);
        file
    }
}

/// Call the C `driver` with `(x, y, z)` and return its stdout bytes.
pub fn c_call(x: c_int, y: c_int, z: c_int) -> Vec<u8> {
    let l = libs();
    capture_locked(|| unsafe { (l.c_driver)(x, y, z) })
}

/// Call the Rust `driver` with `(x, y, z)` and return its stdout bytes.
pub fn rust_call(x: c_int, y: c_int, z: c_int) -> Vec<u8> {
    let l = libs();
    capture_locked(|| unsafe { (l.rust_driver)(x, y, z) })
}

/// Core differential assertion: identical stdout bytes for the same arguments.
///
/// Returns the (shared) output so callers can additionally assert on content.
#[track_caller]
pub fn assert_same(x: c_int, y: c_int, z: c_int) -> Vec<u8> {
    let _g = capture_guard();
    let c_out = c_call(x, y, z);
    let rust_out = rust_call(x, y, z);
    assert_eq!(
        c_out,
        rust_out,
        "stdout mismatch for driver({x}, {y}, {z})\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out),
    );
    c_out
}

/// Differential assertion over an ordered *sequence* of calls, so that each
/// library's residual `static int y` is compared too. The C library replays the
/// whole sequence first, then the Rust library replays the identical sequence,
/// and the two transcripts must match.
#[track_caller]
pub fn assert_same_sequence(calls: &[(c_int, c_int, c_int)]) {
    let _g = capture_guard();
    let l = libs();

    let c_out = capture_locked(|| {
        for &(x, y, z) in calls {
            unsafe { (l.c_driver)(x, y, z) };
        }
    });
    let rust_out = capture_locked(|| {
        for &(x, y, z) in calls {
            unsafe { (l.rust_driver)(x, y, z) };
        }
    });

    assert_eq!(
        c_out,
        rust_out,
        "stdout mismatch over a {}-call sequence (first calls: {:?})",
        calls.len(),
        &calls[..calls.len().min(8)],
    );
}

/// Deterministic PRNG (SplitMix64) so every randomized row is reproducible.
pub struct Rng(u64);

/// Fixed seed shared by all randomized rows.
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

    /// Any `int` value, uniformly over the full 32-bit range.
    pub fn any_int(&mut self) -> c_int {
        self.next_u64() as u32 as i32
    }

    /// Any `int` value except `excluded`.
    pub fn any_int_except(&mut self, excluded: c_int) -> c_int {
        loop {
            let v = self.any_int();
            if v != excluded {
                return v;
            }
        }
    }

    /// Biased generator: mixes full-range values with small values near the
    /// magic constants 1/2/3, so guard boundaries are hit often.
    pub fn interesting_int(&mut self) -> c_int {
        match self.next_u64() % 4 {
            0 => self.any_int(),
            1 => (self.next_u64() % 9) as i32 - 4, // -4..=4
            2 => [i32::MIN, i32::MAX, 0, 1, 2, 3, -1, 4][(self.next_u64() % 8) as usize],
            _ => (self.next_u64() % 1000) as i32 - 500,
        }
    }

    /// Boundary-biased value that is never `excluded`. Preferred over
    /// [`Rng::any_int_except`] for single-axis rows: a uniform 32-bit draw
    /// essentially never lands on the guard's off-by-one neighbours, which is
    /// exactly where an off-by-one translation bug would hide.
    pub fn interesting_int_except(&mut self, excluded: c_int) -> c_int {
        loop {
            let v = self.interesting_int();
            if v != excluded {
                return v;
            }
        }
    }
}

/// The exact byte strings the C source prints, transcribed from
/// `c_src/src/driver.c`. Used as an independent oracle so a test cannot pass
/// by both implementations being wrong in the same way.
pub mod expected {
    pub const OK: &str = "Ok!\n";
    pub const ERR_X: &str = "Error: x != 1\n";
    pub const ERR_Y: &str = "Error: x == 1 but y != 2\n";
    pub const ERR_Z: &str = "Error: x == 1 and y == 2, but z != 3\n";
    pub const FAILED: &str = "Operation failed\n";

    pub fn result_line(code: i32) -> String {
        format!("Result: {code}\n")
    }

    /// Full expected transcript of one `driver(x, y, z)` call, computed
    /// independently from the C control flow.
    pub fn transcript(x: i32, y: i32, z: i32) -> String {
        if x != 1 {
            format!("{ERR_X}{FAILED}{}", result_line(1))
        } else if y != 2 {
            format!("{ERR_Y}{FAILED}{}", result_line(2))
        } else if z != 3 {
            format!("{ERR_Z}{FAILED}{}", result_line(3))
        } else {
            format!("{OK}{}", result_line(0))
        }
    }
}
