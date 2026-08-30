//! Shared harness: loads the C and Rust shared objects through `libloading`
//! and calls both only through their exported `extern "C"` symbols.

use std::ffi::{c_char, c_int, c_void};
use std::path::PathBuf;
use std::sync::Mutex;

use libloading::{Library, Symbol};

pub type MyPowFn = unsafe extern "C" fn(f64, f64) -> f64;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
    fn __errno_location() -> *mut c_int;
    static stderr: *mut c_void;
}

const O_RDWR: c_int = 0o2;
const O_CREAT: c_int = 0o100;
const O_TRUNC: c_int = 0o1000;

/// Directory holding the build profile artifacts that `cargo test` produced,
/// derived from the test executable's own location (`target/<profile>/deps/..`).
fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    exe.parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>")
        .to_path_buf()
}

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// `translation/target/<profile>/libpow.so` — the Rust cdylib under test.
pub fn rust_so_path() -> PathBuf {
    let p = profile_dir().join("libpow.so");
    assert!(
        p.is_file(),
        "Rust cdylib not found at {}; run `cargo build` for this profile first",
        p.display()
    );
    p
}

/// `c_src/build/libpow.so` — the reference implementation.
///
/// `POW_C_SO` overrides the location, which is how the suite is pointed at C
/// builds made with different optimisation settings.
pub fn c_so_path() -> PathBuf {
    let p = match std::env::var_os("POW_C_SO") {
        Some(v) => PathBuf::from(v),
        None => crate_root()
            .parent()
            .expect("workspace dir")
            .join("c_src/build/libpow.so"),
    };
    assert!(
        p.is_file(),
        "C shared library not found at {}; build it with cmake first",
        p.display()
    );
    p
}

/// Both implementations, each reachable only via `dlsym`.
pub struct Pair {
    _c_lib: Library,
    _rust_lib: Library,
    pub c: MyPowFn,
    pub rust: MyPowFn,
}

impl Pair {
    pub fn load() -> Self {
        unsafe {
            let c_lib = Library::new(c_so_path()).expect("dlopen C libpow.so");
            let rust_lib = Library::new(rust_so_path()).expect("dlopen Rust libpow.so");

            let c_sym: Symbol<MyPowFn> = c_lib.get(b"my_pow\0").expect("dlsym C my_pow");
            let rust_sym: Symbol<MyPowFn> = rust_lib.get(b"my_pow\0").expect("dlsym Rust my_pow");

            let c = *c_sym;
            let rust = *rust_sym;

            Pair {
                _c_lib: c_lib,
                _rust_lib: rust_lib,
                c,
                rust,
            }
        }
    }
}

/// Outcome of one call: raw result bits, whatever the callee left in `errno`,
/// and the exact bytes the callee wrote to file descriptor 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Observation {
    pub bits: u64,
    pub errno_after: c_int,
    pub stderr: Vec<u8>,
}

/// Redirects fd 2 to a temp file, runs `f`, and returns what it printed.
///
/// Both shared objects resolve `stderr` to the one and only libc `FILE` in the
/// process, so flushing here is enough to see their output.
///
/// fd 2 is process-global, so a lock serialises captures across the parallel
/// test threads inside this binary.
fn capture_fd2<R>(tag: &str, f: impl FnOnce() -> R) -> (R, Vec<u8>) {
    static FD2_LOCK: Mutex<()> = Mutex::new(());
    let _guard = FD2_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let path = std::env::temp_dir().join(format!(
        "pow_stderr_{}_{}_{:?}.txt",
        std::process::id(),
        tag,
        std::thread::current().id()
    ));
    let c_path = std::ffi::CString::new(path.to_str().unwrap()).unwrap();

    unsafe {
        fflush(stderr);
        let saved = dup(2);
        assert!(saved >= 0, "dup(2) failed");
        let tmp_fd = open(c_path.as_ptr(), O_RDWR | O_CREAT | O_TRUNC, 0o600 as c_int);
        assert!(tmp_fd >= 0, "open temp file failed");
        assert!(dup2(tmp_fd, 2) >= 0, "dup2 onto fd 2 failed");
        close(tmp_fd);

        let out = f();

        fflush(stderr);
        assert!(dup2(saved, 2) >= 0, "restore fd 2 failed");
        close(saved);

        let bytes = std::fs::read(&path).unwrap_or_default();
        let _ = std::fs::remove_file(&path);
        (out, bytes)
    }
}

/// Calls one implementation with `errno` pre-seeded to `errno_seed` so that the
/// `errno = 0` reset inside the callee is actually exercised.
pub fn observe(tag: &str, f: MyPowFn, base: f64, exponent: f64, errno_seed: c_int) -> Observation {
    let (bits_and_errno, stderr_bytes) = capture_fd2(tag, || unsafe {
        *__errno_location() = errno_seed;
        let r = f(base, exponent);
        (r.to_bits(), *__errno_location())
    });

    Observation {
        bits: bits_and_errno.0,
        errno_after: bits_and_errno.1,
        stderr: stderr_bytes,
    }
}

/// Every input pair the suite checks, ordered lowest-risk first.
pub fn inputs() -> Vec<(f64, f64)> {
    let mut v: Vec<(f64, f64)> = Vec::new();

    // Plain, well-defined cases.
    let plain = [
        (2.0, 10.0),
        (2.0, 0.5),
        (3.0, 3.0),
        (10.0, 3.0),
        (1.5, 2.5),
        (0.5, 8.0),
        (7.0, -2.0),
        (0.1, 3.0),
        (123.456, 1.5),
        (1e100, 0.5),
        (1e-100, 0.5),
        (2.0, -1074.0),
        (2.0, 1023.0),
        (2.0, 1024.0),
        (std::f64::consts::E, 1.0),
        (std::f64::consts::PI, std::f64::consts::E),
    ];
    v.extend_from_slice(&plain);

    // Identities and edge exponents that C99 pins down.
    let identities = [
        (0.0, 0.0),
        (-0.0, 0.0),
        (1.0, f64::NAN),
        (f64::NAN, 0.0),
        (0.0, 1.0),
        (-0.0, 3.0),
        (-0.0, 2.0),
        (1.0, 1e308),
        (-1.0, f64::INFINITY),
        (-1.0, f64::NEG_INFINITY),
    ];
    v.extend_from_slice(&identities);

    // Domain errors: negative base with a non-integral exponent.
    let domain = [
        (-8.0, 1.0 / 3.0),
        (-2.0, 0.5),
        (-1.0, 0.5),
        (-1.5, 1.5),
        (-1e300, 0.25),
        (-0.5, -0.5),
    ];
    v.extend_from_slice(&domain);

    // Negative base with integral exponents (well defined, sign matters).
    let neg_int = [
        (-2.0, 3.0),
        (-2.0, 4.0),
        (-2.0, -3.0),
        (-3.0, 0.0),
        (-2.0, 1023.0),
    ];
    v.extend_from_slice(&neg_int);

    // Overflow / underflow / pole -> ERANGE.
    let range = [
        (1e300, 2.0),
        (1e300, -2.0),
        (1e-300, 2.0),
        (1e-300, -2.0),
        (2.0, 5000.0),
        (2.0, -5000.0),
        (0.0, -1.0),
        (-0.0, -1.0),
        (0.0, -2.0),
        (-0.0, -2.0),
        (0.0, -0.5),
        (f64::MAX, 1.0000001),
        (f64::MIN_POSITIVE, 2.0),
        (5e-324, 2.0),
        (1.0000001, 1e9),
        (0.9999999, 1e9),
    ];
    v.extend_from_slice(&range);

    // Infinities and NaNs in every slot.
    let specials = [
        (f64::INFINITY, 2.0),
        (f64::INFINITY, -2.0),
        (f64::NEG_INFINITY, 2.0),
        (f64::NEG_INFINITY, 3.0),
        (f64::NEG_INFINITY, -3.0),
        (2.0, f64::INFINITY),
        (2.0, f64::NEG_INFINITY),
        (0.5, f64::INFINITY),
        (0.5, f64::NEG_INFINITY),
        (f64::NAN, f64::NAN),
        (f64::NAN, 1.0),
        (2.0, f64::NAN),
        (f64::INFINITY, f64::NAN),
        (f64::NAN, f64::INFINITY),
        (-f64::NAN, 2.0),
        (0.0, f64::NAN),
    ];
    v.extend_from_slice(&specials);

    // Values whose %.2f rendering is interesting on the error paths.
    let formatting = [
        (-1.005, 0.5),
        (-1.004999, 0.5),
        (-2.675, 0.5),
        (-0.001, 0.5),
        (-1e300, 0.5),
        (-1e-300, 0.5),
        (-123456789.987654, 0.5),
    ];
    v.extend_from_slice(&formatting);

    v
}
