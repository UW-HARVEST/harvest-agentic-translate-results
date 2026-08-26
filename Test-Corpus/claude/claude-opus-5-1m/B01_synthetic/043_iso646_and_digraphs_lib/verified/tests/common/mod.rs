//! Shared differential-test harness.
//!
//! Both implementations are loaded as **shared libraries** through `libloading`
//! and called only through their exported `extern "C"` symbols:
//!
//!   * the C ground truth  -> `c_src/build/libdriver.so`
//!   * the Rust port       -> `target/<dir>/libdriver.so`  (crate-type = cdylib)
//!
//! Rust functions are never called directly, so the `#[no_mangle]` export
//! wrappers are part of what is under test.

#![allow(dead_code)]

use std::ffi::{c_char, c_int, c_void, OsStr};
use std::fs;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

// ---------------------------------------------------------------------------
// libc bits we need for fd redirection / stream configuration.
// ---------------------------------------------------------------------------

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn printf(fmt: *const c_char, ...) -> c_int;
    fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
    fn pipe(fds: *mut c_int) -> c_int;
    fn write(fd: c_int, buf: *const c_void, n: usize) -> isize;
    static stdout: *mut c_void;
}

pub const IOFBF: c_int = 0; // fully buffered
pub const IOLBF: c_int = 1; // line buffered
pub const IONBF: c_int = 2; // unbuffered

/// Flush *every* open output stream (POSIX semantics of `fflush(NULL)`), which
/// covers the `stdout` `FILE` shared by the test binary, the C `.so` and the
/// Rust `.so`.
pub fn flush_all() {
    unsafe { fflush(std::ptr::null_mut()) };
}

/// Write bytes to `stdout` through libc `printf`, i.e. through the very same
/// `FILE` object the libraries use.
pub fn caller_printf(s: &str) {
    let c = std::ffi::CString::new(s).unwrap();
    unsafe { printf(c"%s".as_ptr(), c.as_ptr()) };
}

/// Reconfigure the shared `stdout` buffering mode.
pub fn set_stdout_buffering(mode: c_int, size: usize) {
    flush_all();
    unsafe { setvbuf(stdout, std::ptr::null_mut(), mode, size) };
}

// ---------------------------------------------------------------------------
// Locating / building the two shared objects.
// ---------------------------------------------------------------------------

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Build (if needed) and return the path to the **C** shared library.
pub fn c_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        if let Some(p) = std::env::var_os("DRIVER_C_SO") {
            let p = PathBuf::from(p);
            assert!(p.exists(), "DRIVER_C_SO={} does not exist", p.display());
            return p;
        }
        let root = manifest_dir();
        let build = root.join("c_src/build");
        let so = build.join("libdriver.so");
        if !so.exists() {
            fs::create_dir_all(&build).expect("mkdir c_src/build");
            let st = Command::new("cmake")
                .current_dir(&build)
                .args(["..", "-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
                .status()
                .expect("run cmake configure");
            assert!(st.success(), "cmake configure failed");
            let st = Command::new("cmake")
                .current_dir(&build)
                .args(["--build", "."])
                .status()
                .expect("run cmake build");
            assert!(st.success(), "cmake build failed");
        }
        assert!(so.exists(), "C shared library not found at {}", so.display());
        so
    })
    .as_path()
}

/// Build (if needed) and return the path to the **Rust** `cdylib`.
///
/// `cargo test` does not produce the `cdylib` artifact by itself, so the
/// harness builds it explicitly. A dedicated `--target-dir` is used so this
/// nested `cargo` invocation can never contend with the lock held by the outer
/// `cargo test`.
pub fn rust_so_path() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(|| {
        // An explicit override lets the same suite be pointed at a differently
        // built artifact (e.g. the `release` profile, which sets
        // `panic = "abort"`), without rebuilding here.
        if let Some(p) = std::env::var_os("DRIVER_RUST_SO") {
            let p = PathBuf::from(p);
            assert!(p.exists(), "DRIVER_RUST_SO={} does not exist", p.display());
            return p;
        }
        let root = manifest_dir();
        let target_dir = root.join("target/ffi-so");

        let mut cmd = Command::new(option_env!("CARGO").unwrap_or("cargo"));
        cmd.current_dir(&root)
            .arg("build")
            .arg("--offline")
            .arg("--lib")
            .arg("--target-dir")
            .arg(&target_dir);
        // Mirror the feature selection of this test binary onto the cdylib so
        // every feature combination is exercised through the .so as well.
        // (The crate currently declares no [features]; the loop below keeps
        // working if any are added.)
        let feats = enabled_features();
        cmd.arg("--no-default-features");
        if !feats.is_empty() {
            cmd.arg("--features").arg(feats.join(","));
        }
        let st = cmd.status().expect("run cargo build for cdylib");
        assert!(st.success(), "cargo build of the cdylib failed");

        let so = target_dir.join("debug/libdriver.so");
        assert!(
            so.exists(),
            "Rust shared library not found at {}",
            so.display()
        );
        so
    })
    .as_path()
}

/// The features enabled for *this* test binary. The crate declares no
/// `[features]`, so this is always empty; it exists so the harness stays
/// correct if features are added later.
pub fn enabled_features() -> Vec<&'static str> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// The loaded libraries.
// ---------------------------------------------------------------------------

pub type DriverFn = unsafe extern "C" fn(c_int, c_int);
/// Deliberately *mis*-declared prototype used to probe C-ABI argument
/// truncation (the analogue of feeding an out-of-range enum value across FFI).
pub type DriverFn64 = unsafe extern "C" fn(i64, i64);

pub struct Lib {
    _lib: libloading::Library,
    pub driver: DriverFn,
    pub driver_i64: DriverFn64,
    pub path: PathBuf,
}

impl Lib {
    fn open(path: &Path) -> Lib {
        // RTLD_LOCAL (libloading's default) keeps the two libraries' identically
        // named `driver` symbols from interposing on each other.
        let lib = unsafe { libloading::Library::new(OsStr::new(path)) }
            .unwrap_or_else(|e| panic!("dlopen {} failed: {e}", path.display()));
        let driver: DriverFn = unsafe {
            *lib.get::<DriverFn>(b"driver\0")
                .unwrap_or_else(|e| panic!("dlsym driver in {}: {e}", path.display()))
        };
        let driver_i64: DriverFn64 = unsafe { std::mem::transmute(driver) };
        Lib {
            _lib: lib,
            driver,
            driver_i64,
            path: path.to_path_buf(),
        }
    }
}

pub fn c_lib() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| Lib::open(c_so_path()))
}

pub fn rust_lib() -> &'static Lib {
    static L: OnceLock<Lib> = OnceLock::new();
    L.get_or_init(|| Lib::open(rust_so_path()))
}

// ---------------------------------------------------------------------------
// stdout capture (fd level, so it catches libc `printf`/`puts` from any .so).
// ---------------------------------------------------------------------------

static SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_path(tag: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    std::env::temp_dir().join(format!(
        "driver_difftest_{}_{}_{}.out",
        std::process::id(),
        tag,
        n
    ))
}

/// Run `f` with fd 1 redirected to a temporary file and return everything that
/// was written to `stdout` (by any library in the process) while it ran.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    let path = temp_path(tag);
    let file = fs::File::create(&path).expect("create capture file");
    let fd = file.as_raw_fd();

    // Push out anything already sitting in the shared stdout buffer so it does
    // not leak into the capture.
    flush_all();

    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(fd, 1) } >= 0, "dup2 onto fd 1 failed");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    // Drain the library's buffered bytes into the capture file before restoring.
    flush_all();
    assert!(unsafe { dup2(saved, 1) } >= 0, "restore fd 1 failed");
    unsafe { close(saved) };
    drop(file);

    let mut buf = Vec::new();
    fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut buf)
        .expect("read capture file");
    let _ = fs::remove_file(&path);

    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }
    buf
}

/// Like [`capture_stdout`] but the redirect target is a **pipe** instead of a
/// regular file (glibc picks its default buffering from the fd kind).
pub fn capture_stdout_pipe<F: FnOnce()>(f: F) -> Vec<u8> {
    let mut fds = [0 as c_int; 2];
    assert_eq!(unsafe { pipe(fds.as_mut_ptr()) }, 0, "pipe() failed");
    let (rd, wr) = (fds[0], fds[1]);

    flush_all();
    let saved = unsafe { dup(1) };
    assert!(saved >= 0);
    assert!(unsafe { dup2(wr, 1) } >= 0);

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    flush_all();
    assert!(unsafe { dup2(saved, 1) } >= 0);
    unsafe {
        close(saved);
        close(wr);
    }

    // Read until EOF. The pipe write end is closed above, so this terminates.
    let mut out = Vec::new();
    {
        use std::os::unix::io::FromRawFd;
        let mut f = unsafe { fs::File::from_raw_fd(rd) };
        f.read_to_end(&mut out).expect("read pipe");
    }

    if let Err(p) = result {
        std::panic::resume_unwind(p);
    }
    out
}

// ---------------------------------------------------------------------------
// Deterministic PRNG (SplitMix64) — fixed seed for reproducibility.
// ---------------------------------------------------------------------------

pub const SEED: u64 = 0x5EED_1234_ABCD_F00D;

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
    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }
    /// Uniform in `1 ..= i32::MAX`
    pub fn next_pos(&mut self) -> i32 {
        (self.next_u64() % (i32::MAX as u64)) as i32 + 1
    }
    /// Uniform in `i32::MIN ..= -1`
    pub fn next_neg(&mut self) -> i32 {
        let m = (self.next_u64() % (i32::MAX as u64 + 1)) as i32; // 0 ..= i32::MAX
        -m - 1 // -1 ..= i32::MIN
    }
}

// ---------------------------------------------------------------------------
// Differential assertions.
// ---------------------------------------------------------------------------

/// Call `driver` in **both** libraries for every `(x, y)` in `inputs` — each
/// library's whole run captured as one stream — and require the two byte
/// streams to be identical. On divergence, re-run the inputs one at a time to
/// pin down the first offending pair.
#[track_caller]
pub fn assert_same_batch(row: &str, inputs: &[(i32, i32)]) {
    let c = c_lib();
    let r = rust_lib();

    let c_out = capture_stdout("c", || {
        for &(x, y) in inputs {
            unsafe { (c.driver)(x, y) };
        }
    });
    let r_out = capture_stdout("rust", || {
        for &(x, y) in inputs {
            unsafe { (r.driver)(x, y) };
        }
    });

    if c_out == r_out {
        return;
    }

    // Localise the first divergent input.
    for &(x, y) in inputs {
        let cs = capture_stdout("c1", || unsafe { (c.driver)(x, y) });
        let rs = capture_stdout("r1", || unsafe { (r.driver)(x, y) });
        if cs != rs {
            panic!(
                "[{row}] divergence at driver({x}, {y}):\n  C    = {:?}\n  Rust = {:?}",
                String::from_utf8_lossy(&cs),
                String::from_utf8_lossy(&rs)
            );
        }
    }
    panic!(
        "[{row}] batched streams differ but no single input diverges \
         (length C={} Rust={}); C={:?} Rust={:?}",
        c_out.len(),
        r_out.len(),
        String::from_utf8_lossy(&c_out[..c_out.len().min(200)]),
        String::from_utf8_lossy(&r_out[..r_out.len().min(200)])
    );
}

/// Per-call comparison: one fresh capture per invocation.
#[track_caller]
pub fn assert_same_each(row: &str, inputs: &[(i32, i32)]) {
    let c = c_lib();
    let r = rust_lib();
    for &(x, y) in inputs {
        let cs = capture_stdout("c1", || unsafe { (c.driver)(x, y) });
        let rs = capture_stdout("r1", || unsafe { (r.driver)(x, y) });
        assert_eq!(
            cs,
            rs,
            "[{row}] driver({x}, {y}): C={:?} Rust={:?}",
            String::from_utf8_lossy(&cs),
            String::from_utf8_lossy(&rs)
        );
    }
}

/// Same as [`assert_same_batch`] but the arguments are pushed through the
/// deliberately mis-declared 64-bit prototype.
#[track_caller]
pub fn assert_same_batch_i64(row: &str, inputs: &[(i64, i64)]) {
    let c = c_lib();
    let r = rust_lib();

    let c_out = capture_stdout("c", || {
        for &(x, y) in inputs {
            unsafe { (c.driver_i64)(x, y) };
        }
    });
    let r_out = capture_stdout("rust", || {
        for &(x, y) in inputs {
            unsafe { (r.driver_i64)(x, y) };
        }
    });
    assert_eq!(
        c_out,
        r_out,
        "[{row}] 64-bit-prototype streams differ; C={:?} Rust={:?}",
        String::from_utf8_lossy(&c_out[..c_out.len().min(300)]),
        String::from_utf8_lossy(&r_out[..r_out.len().min(300)])
    );
}

/// Raw `write(2)` to whatever fd 1 currently is — used by the failure-injection
/// tests to leave a marker that proves the code kept running.
pub fn raw_write_fd1(bytes: &[u8]) {
    unsafe { write(1, bytes.as_ptr() as *const c_void, bytes.len()) };
}
