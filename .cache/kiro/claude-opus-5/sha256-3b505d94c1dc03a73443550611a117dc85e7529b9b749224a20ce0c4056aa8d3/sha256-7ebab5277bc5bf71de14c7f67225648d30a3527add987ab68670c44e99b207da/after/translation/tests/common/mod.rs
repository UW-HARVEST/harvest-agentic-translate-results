//! Shared harness for differential testing of the C `libdriver.so` against the
//! Rust `libdriver.so`.
//!
//! Both libraries are loaded with `libloading` (i.e. `dlopen` with `RTLD_LOCAL`)
//! so every call crosses the real FFI boundary and exercises the `#[no_mangle]`
//! export wrappers. Nothing in the Rust crate is called directly.

#![allow(dead_code)]

use std::ffi::{c_char, c_float, c_int, c_void};
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

static CAPTURE_COUNTER: AtomicU64 = AtomicU64::new(0);

use libloading::{Library, Symbol};

extern "C" {
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

/// Serializes the process wide stdout redirection used by [`capture`].
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("C_DRIVER_SO") {
        return PathBuf::from(path);
    }
    manifest_dir().join("../c_src/build/libdriver.so")
}

/// Locates the Rust `cdylib`, building it if necessary.
///
/// The crate only declares `crate-type = ["cdylib"]`, and the integration tests
/// deliberately do not depend on the library crate, so `cargo test` never emits
/// the shared object on its own. It is therefore built here with a dedicated
/// `CARGO_TARGET_DIR` (avoiding any lock contention with the outer `cargo test`)
/// and with the same feature selection as this test binary.
fn rust_library_path() -> PathBuf {
    if let Some(path) = std::env::var_os("RUST_DRIVER_SO") {
        return PathBuf::from(path);
    }

    // No filesystem search: picking up a stale artifact left behind by an
    // earlier build would silently test the wrong code.
    build_rust_library()
}

/// The features this test binary was compiled with, so the shared object under
/// test matches the configuration being exercised.
fn active_features() -> Vec<&'static str> {
    // The crate currently declares no `[features]`; entries are added here as
    // features appear so that `--features` propagates to the cdylib build.
    Vec::new()
}

fn build_rust_library() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| {
            let manifest = manifest_dir().join("Cargo.toml");
            let features = active_features();
            let target_dir = manifest_dir()
                .join("target")
                .join(format!("ffi-cdylib-{}", features.join("+")));

            let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
            let mut command = std::process::Command::new(cargo);
            command
                .arg("build")
                .arg("--lib")
                .arg("--manifest-path")
                .arg(&manifest)
                .arg("--no-default-features")
                .env("CARGO_TARGET_DIR", &target_dir);
            if !features.is_empty() {
                command.arg("--features").arg(features.join(","));
            }

            let output = command.output().expect("failed to run cargo build --lib");
            assert!(
                output.status.success(),
                "cargo build --lib failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            );

            let built = target_dir.join("debug").join("libdriver.so");
            assert!(
                built.is_file(),
                "cargo build --lib did not produce {}",
                built.display()
            );
            built
        })
        .clone()
}

fn load(path: &Path) -> &'static Library {
    let library = unsafe { Library::new(path) }
        .unwrap_or_else(|err| panic!("failed to dlopen {}: {err}", path.display()));
    Box::leak(Box::new(library))
}

pub fn c_lib() -> &'static Library {
    static LIB: OnceLock<&'static Library> = OnceLock::new();
    LIB.get_or_init(|| load(&c_library_path()))
}

pub fn rust_lib() -> &'static Library {
    static LIB: OnceLock<&'static Library> = OnceLock::new();
    LIB.get_or_init(|| load(&rust_library_path()))
}

/// Resolves `name` in the given library. Panics if the symbol is missing, which
/// is exactly what should happen when the Rust side forgets an export.
pub fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    let mut bytes = Vec::with_capacity(name.len() + 1);
    bytes.extend_from_slice(name.as_bytes());
    bytes.push(0);
    unsafe { lib.get::<T>(&bytes) }
        .unwrap_or_else(|err| panic!("missing exported symbol `{name}`: {err}"))
}

/// Runs `f` with file descriptor 1 redirected into a temporary file and returns
/// the raw bytes that were written.
///
/// `fflush(NULL)` is used on both sides of the redirection because the C library
/// and the Rust `cdylib` share the process' libc `stdout` buffer.
pub fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock();

    let path = std::env::temp_dir().join(format!(
        "driver_diff_{}_{}.out",
        std::process::id(),
        CAPTURE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    let bytes = {
        let file = std::fs::File::create(&path).expect("create capture file");
        let saved = unsafe {
            // Drain both buffering layers that sit in front of fd 1: libc's
            // `FILE` buffers (used by the libraries under test) and Rust's
            // `LineWriter` (used by the test harness' progress output).
            let _ = std::io::stdout().flush();
            fflush(std::ptr::null_mut());
            let saved = dup(1);
            assert!(saved >= 0, "dup(1) failed");
            assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");
            saved
        };

        // The redirection must be undone even if `f` panics, otherwise the whole
        // test binary loses its stdout.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

        unsafe {
            // Only libc's buffers are flushed here. Flushing Rust's `Stdout`
            // while fd 1 is still redirected would push unrelated harness output
            // into the capture file.
            fflush(std::ptr::null_mut());
            assert!(dup2(saved, 1) >= 0, "restoring stdout failed");
            close(saved);
        }
        drop(file);

        let bytes = std::fs::read(&path).expect("read capture file");
        let _ = std::fs::remove_file(&path);

        if let Err(payload) = result {
            std::panic::resume_unwind(payload);
        }
        bytes
    };

    bytes
}

/// Renders bytes for assertion messages without losing non UTF-8 content.
pub fn show(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => format!("{text:?}"),
        Err(_) => format!("{bytes:?}"),
    }
}

pub type PrintLineFn = unsafe extern "C" fn(*const c_char);
pub type PrintIntLineFn = unsafe extern "C" fn(c_int);
pub type FloatFn = unsafe extern "C" fn(c_float);
pub type DriverFn = unsafe extern "C" fn(c_float, c_float);

pub struct Api {
    pub print_line: Symbol<'static, PrintLineFn>,
    pub print_int_line: Symbol<'static, PrintIntLineFn>,
    pub bad: Symbol<'static, FloatFn>,
    pub good: Symbol<'static, FloatFn>,
    pub driver: Symbol<'static, DriverFn>,
}

impl Api {
    fn new(lib: &'static Library) -> Self {
        Api {
            print_line: sym(lib, "printLine"),
            print_int_line: sym(lib, "printIntLine"),
            bad: sym(lib, "bad"),
            good: sym(lib, "good"),
            driver: sym(lib, "driver"),
        }
    }
}

pub fn c_api() -> &'static Api {
    static API: OnceLock<Api> = OnceLock::new();
    API.get_or_init(|| Api::new(c_lib()))
}

pub fn rust_api() -> &'static Api {
    static API: OnceLock<Api> = OnceLock::new();
    API.get_or_init(|| Api::new(rust_lib()))
}

/// Deterministic 64 bit LCG, used so failures are reproducible.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u32(&mut self) -> u32 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 32) as u32
    }

    pub fn next_i32(&mut self) -> i32 {
        self.next_u32() as i32
    }

    /// Any `f32` bit pattern, including infinities, NaNs and subnormals.
    pub fn next_f32_bits(&mut self) -> f32 {
        f32::from_bits(self.next_u32())
    }
}

/// Minimal sequential test runner used instead of libtest.
///
/// The integration tests are declared with `harness = false` because libtest
/// writes its progress to file descriptor 1 from a separate thread, which would
/// otherwise be captured alongside the output of the libraries under test. This
/// runner is single threaded and reports on stderr, so fd 1 belongs exclusively
/// to [`capture`].
pub fn run_suite(suite: &str, tests: &[(&str, fn())]) -> ! {
    eprintln!("running {} tests in {suite}", tests.len());

    let mut failed = Vec::new();
    for (name, test) in tests {
        eprint!("test {name} ... ");
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(*test)) {
            Ok(()) => eprintln!("ok"),
            Err(_) => {
                eprintln!("FAILED");
                failed.push(*name);
            }
        }
    }

    if failed.is_empty() {
        eprintln!(
            "test result: ok. {} passed; 0 failed (suite {suite})",
            tests.len()
        );
        std::process::exit(0);
    }

    eprintln!("failures:");
    for name in &failed {
        eprintln!("    {name}");
    }
    eprintln!(
        "test result: FAILED. {} passed; {} failed (suite {suite})",
        tests.len() - failed.len(),
        failed.len()
    );
    std::process::exit(1);
}
