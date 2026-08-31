//! Shared support code for the C-vs-Rust differential tests.
//!
//! Both implementations are loaded as shared objects through `libloading` and
//! invoked purely through their exported C symbols, so the `#[no_mangle]`
//! wrappers are part of what is under test.

#![allow(dead_code)]

use std::ffi::{c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

/// `void (*)(int *out, const int *mul1, const int *mul2, const int *add, int len)`
pub type FmaArrayFn =
    unsafe extern "C" fn(*mut c_int, *const c_int, *const c_int, *const c_int, c_int);

/// `void (*)(const int *data, int len)`
pub type DriverFn = unsafe extern "C" fn(*const c_int, c_int);

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared object, building it with CMake if it is absent.
fn c_library_path() -> PathBuf {
    if let Some(p) = std::env::var_os("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let build = manifest_dir().join("../c_src/build");
    let so = build.join("libdriver.so");
    if !so.exists() {
        std::fs::create_dir_all(&build).expect("failed to create c_src/build");
        let configure = std::process::Command::new("cmake")
            .arg("..")
            .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
            .current_dir(&build)
            .status()
            .expect("failed to run cmake");
        assert!(configure.success(), "cmake configure failed");
        let compile = std::process::Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build)
            .status()
            .expect("failed to run cmake --build");
        assert!(compile.success(), "cmake build failed");
    }
    so
}

/// Path to the Rust cdylib.
///
/// `cargo test` does *not* refresh the `cdylib` artifact under
/// `target/<profile>/`, so trusting whatever is left there would silently test a
/// stale library. Unless an explicit path is supplied, build the `cdylib` here
/// into a separate target directory -- which also keeps it clear of the
/// enclosing `cargo test` build lock -- and load that.
///
/// `VERIFY_FEATURES` selects the feature combination for that build so the
/// loaded `.so` matches the configuration under test.
fn rust_library_path() -> PathBuf {
    if let Some(p) = std::env::var_os("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }

    let target_dir = manifest_dir().join("target/verify-so");
    let cargo = option_env!("CARGO").unwrap_or("cargo");
    let features = std::env::var("VERIFY_FEATURES").unwrap_or_default();

    let mut cmd = std::process::Command::new(cargo);
    cmd.args(["build", "--release", "--no-default-features"]);
    if !features.trim().is_empty() {
        cmd.args(["--features", features.trim()]);
    }
    cmd.arg("--target-dir")
        .arg(&target_dir)
        .current_dir(manifest_dir());

    let status = cmd
        .status()
        .unwrap_or_else(|e| panic!("failed to run `{cargo} build`: {e}"));
    assert!(
        status.success(),
        "building the Rust cdylib failed (features: {features:?})"
    );

    target_dir.join("release/libdriver.so")
}

fn load(path: &Path) -> Library {
    assert!(
        path.exists(),
        "shared object {} is missing",
        path.display()
    );
    // SAFETY: loading a plain C shared object; it has no initialisers that can
    // violate Rust's invariants.
    unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()))
}

/// Resolved (and, if necessary, freshly built) path of the C shared object.
pub fn c_so() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(c_library_path)
}

/// Resolved (and, if necessary, freshly built) path of the Rust shared object.
pub fn rust_so() -> &'static Path {
    static P: OnceLock<PathBuf> = OnceLock::new();
    P.get_or_init(rust_library_path)
}

/// The two implementations under comparison, kept loaded for the whole test
/// binary. `dlopen` keeps the C and Rust copies of a same-named symbol separate
/// because `libloading` does not request `RTLD_GLOBAL`.
pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    pub fn get() -> &'static Libs {
        static LIBS: OnceLock<Libs> = OnceLock::new();
        LIBS.get_or_init(|| Libs {
            c: load(c_so()),
            rust: load(rust_so()),
        })
    }

    pub fn sym<T>(&self, which: Impl, name: &str) -> Symbol<'_, T> {
        let lib = match which {
            Impl::C => &self.c,
            Impl::Rust => &self.rust,
        };
        // SAFETY: the caller states the correct ABI type for `name`.
        unsafe { lib.get(name.as_bytes()) }
            .unwrap_or_else(|e| panic!("{which:?} library does not export `{name}`: {e}"))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Impl {
    C,
    Rust,
}

pub const IMPLS: [Impl; 2] = [Impl::C, Impl::Rust];

pub fn fma_array(which: Impl) -> Symbol<'static, FmaArrayFn> {
    Libs::get().sym(which, "fma_array")
}

pub fn driver(which: Impl) -> Symbol<'static, DriverFn> {
    Libs::get().sym(which, "driver")
}

/// stdout redirection is process-wide, so captures must not overlap.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

/// Runs `f` with file descriptor 1 pointed at a temporary file and returns the
/// bytes it wrote.
///
/// Both libraries print through the process's single `libc` `stdout`, so
/// flushing before and after the call is enough to attribute output precisely.
///
/// Note: fd 1 is process-wide, so any *other* thread writing to stdout during
/// the capture (including libtest's own progress output) would land in the
/// capture file. Test binaries that use this must therefore expose exactly one
/// `#[test]` entry point.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock();

    let path = std::env::temp_dir().join(format!(
        "driver-capture-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .expect("failed to create capture file");

    // SAFETY: raw fd juggling around a single call, restored before returning.
    let saved = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 onto stdout failed");
        saved
    };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    // SAFETY: restore the original stdout regardless of how `f` finished.
    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, 1);
        close(saved);
    }

    let bytes = std::fs::read(&path).expect("failed to read capture file");
    let _ = std::fs::remove_file(&path);

    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
    bytes
}

/// Deterministic xorshift PRNG so failures are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u64() as u32 as i32
    }

    /// Small magnitude values, so products stay in range most of the time.
    pub fn next_small(&mut self) -> i32 {
        (self.next_u64() % 201) as i32 - 100
    }

    pub fn range(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Values that tend to expose edge cases in multiply/add.
pub const EDGE_VALUES: [i32; 14] = [
    0,
    1,
    -1,
    2,
    -2,
    3,
    i32::MAX,
    i32::MIN,
    i32::MAX - 1,
    i32::MIN + 1,
    65536,
    -65536,
    46341, // 46341^2 overflows i32
    -46341,
];

pub fn show(v: &[i32]) -> String {
    format!("{v:?}")
}
