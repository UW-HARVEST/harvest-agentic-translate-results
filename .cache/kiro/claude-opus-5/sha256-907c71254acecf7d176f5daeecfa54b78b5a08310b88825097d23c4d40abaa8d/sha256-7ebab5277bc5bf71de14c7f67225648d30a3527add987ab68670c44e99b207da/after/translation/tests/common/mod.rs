//! Shared harness for differential testing of the C `libdriver.so` against the
//! Rust `libdriver.so`.
//!
//! Both libraries are loaded through `libloading` and only their exported
//! symbols are ever called, so the `#[no_mangle]` wrappers are part of what is
//! under test.
//!
//! Two things make this trickier than a plain function comparison:
//!
//! * `driver.c` keeps its state in a file-static `the_house`, which mutates on
//!   every call. Each `.so` is therefore `dlopen`ed exactly once per test
//!   process and every C call is immediately paired with the identical Rust
//!   call, so the two copies of the state stay in lockstep no matter how the
//!   test harness schedules its threads.
//! * The output only exists as `printf` writes to fd 1 from inside a `dlopen`ed
//!   library, which Rust's own stdout capture cannot see, so fd 1 is
//!   temporarily redirected to a file around each call.
//!
//! Both concerns are handled by taking a single process-wide lock for the
//! duration of each (C call, Rust call) pair.

#![allow(dead_code)]

use std::ffi::{CString, c_char, c_int, c_void};
use std::fs;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Directory holding the crate (`translation/`).
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared library produced by `c_src/build`.
fn c_lib_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("translation has a parent dir")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}; build it with cmake first",
        p.display()
    );
    p
}

/// Path to the Rust `cdylib` for the profile the test itself was built with.
///
/// The test executable lives in `target/<profile>/deps/`, so the sibling
/// `../libdriver.so` is the matching artifact. `cargo test` does not emit the
/// `cdylib` on its own (it only needs an rlib to link the harness), so build it
/// on demand the first time a test asks for it.
fn rust_lib_path() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    let deps = exe.parent().expect("deps dir");
    let profile_dir = deps.parent().expect("profile dir");
    let p = profile_dir.join("libdriver.so");
    if !p.exists() {
        ensure_cdylib_built(profile_dir);
    }
    assert!(
        p.exists(),
        "Rust shared library not found at {}; run `cargo build` in translation/ first",
        p.display()
    );
    p
}

/// Invoke `cargo build` once per test process so the `cdylib` exists.
fn ensure_cdylib_built(profile_dir: &Path) {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let profile = profile_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("debug");
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
        let mut cmd = std::process::Command::new(cargo);
        cmd.current_dir(manifest_dir()).arg("build");
        if profile != "debug" {
            cmd.arg("--profile").arg(profile);
        }
        // Mirror the feature selection the test binary was compiled with.
        cmd.arg("--no-default-features");
        let features = active_features().join(",");
        if !features.is_empty() {
            cmd.arg("--features").arg(features);
        }
        match cmd.status() {
            Ok(s) if s.success() => {}
            other => panic!("`cargo build` for the cdylib failed: {other:?}"),
        }
    });
}

/// The crate features this test binary was compiled with. The crate declares
/// no `[features]`, so this is empty; keeping it feature-driven means the
/// on-demand build never diverges from the test binary.
fn active_features() -> Vec<&'static str> {
    Vec::new()
}

/// The two entry points declared by `driver.h` / defined non-`static` in
/// `driver.c`.
struct DriverLib {
    _lib: Library,
    driver: unsafe extern "C" fn(*const c_char),
    run: unsafe extern "C" fn(c_int),
    path: PathBuf,
}

impl DriverLib {
    fn open(path: PathBuf, label: &str) -> DriverLib {
        let lib = unsafe { Library::new(&path) }
            .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
        let driver: Symbol<unsafe extern "C" fn(*const c_char)> =
            unsafe { lib.get(b"driver\0") }
                .unwrap_or_else(|e| panic!("{label}: missing exported symbol `driver`: {e}"));
        let run: Symbol<unsafe extern "C" fn(c_int)> = unsafe { lib.get(b"run\0") }
            .unwrap_or_else(|e| panic!("{label}: missing exported symbol `run`: {e}"));
        let driver = *driver;
        let run = *run;
        DriverLib {
            _lib: lib,
            driver,
            run,
            path,
        }
    }
}

/// The C and Rust libraries, loaded side by side.
pub struct Pair {
    c: DriverLib,
    rust: DriverLib,
}

fn pair_lock() -> MutexGuard<'static, Pair> {
    static PAIR: OnceLock<Mutex<Pair>> = OnceLock::new();
    let m = PAIR.get_or_init(|| {
        Mutex::new(Pair {
            c: DriverLib::open(c_lib_path(), "C"),
            rust: DriverLib::open(rust_lib_path(), "Rust"),
        })
    });
    // A panicking test leaves accumulated state behind either way; recovering
    // from poisoning keeps the remaining assertions running.
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Filesystem path of the C shared library.
pub fn c_so() -> PathBuf {
    pair_lock().c.path.clone()
}

/// Filesystem path of the Rust shared library.
pub fn rust_so() -> PathBuf {
    pair_lock().rust.path.clone()
}

/// `void run(int extra_bedrooms)` on both libraries; returns `(c_out, rust_out)`.
pub fn call_run(extra_bedrooms: c_int) -> (Vec<u8>, Vec<u8>) {
    let p = pair_lock();
    let c_out = capture_stdout(|| unsafe { (p.c.run)(extra_bedrooms) });
    let rust_out = capture_stdout(|| unsafe { (p.rust.run)(extra_bedrooms) });
    (c_out, rust_out)
}

/// `void driver(const char *in)` on both libraries, with `input` passed as a
/// NUL-terminated C string.
pub fn call_driver(input: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let s = CString::new(input).expect("use call_driver_raw for embedded NULs");
    let p = pair_lock();
    let c_out = capture_stdout(|| unsafe { (p.c.driver)(s.as_ptr()) });
    let rust_out = capture_stdout(|| unsafe { (p.rust.driver)(s.as_ptr()) });
    (c_out, rust_out)
}

/// `driver` on a buffer used verbatim; it must contain a NUL terminator.
pub fn call_driver_raw(raw: &[u8]) -> (Vec<u8>, Vec<u8>) {
    assert!(raw.contains(&0), "raw input must contain a NUL terminator");
    let ptr = raw.as_ptr() as *const c_char;
    let p = pair_lock();
    let c_out = capture_stdout(|| unsafe { (p.c.driver)(ptr) });
    let rust_out = capture_stdout(|| unsafe { (p.rust.driver)(ptr) });
    (c_out, rust_out)
}

/// Call both libraries and assert their stdout bytes are identical.
pub fn check_run(extra_bedrooms: c_int, case: &str) {
    let (c_out, rust_out) = call_run(extra_bedrooms);
    assert!(!c_out.is_empty(), "C produced no output for {case}");
    assert_same(case, &c_out, &rust_out);
}

/// Call both libraries and assert their stdout bytes are identical.
pub fn check_driver(input: &[u8], case: &str) {
    let (c_out, rust_out) = call_driver(input);
    assert!(!c_out.is_empty(), "C produced no output for {case}");
    assert_same(case, &c_out, &rust_out);
}

static CAPTURE_SEQ: AtomicUsize = AtomicUsize::new(0);

/// Redirect fd 1 to a temporary file for the duration of `f` and return the
/// bytes written.
///
/// Callers must hold the pair lock, since fd 1 is process-wide.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let seq = CAPTURE_SEQ.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver-capture-{}-{}.out",
        std::process::id(),
        seq
    ));

    // Flush anything already buffered so it is not misattributed to this call:
    // C stdio streams, and Rust's own stdout buffer.
    let _ = std::io::Write::flush(&mut std::io::stdout());
    unsafe { fflush(std::ptr::null_mut()) };

    let file = fs::File::create(&path).expect("create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    // Flush the library's stdio buffers *before* restoring fd 1.
    unsafe { fflush(std::ptr::null_mut()) };
    assert!(unsafe { dup2(saved, 1) } >= 0, "dup2 restore failed");
    unsafe { close(saved) };
    drop(file);

    let mut bytes = Vec::new();
    fs::File::open(&path)
        .expect("open capture file")
        .read_to_end(&mut bytes)
        .expect("read capture file");
    let _ = fs::remove_file(&path);
    bytes
}

/// Assert that two captures are byte-identical, with a readable diff.
pub fn assert_same(case: &str, c_out: &[u8], rust_out: &[u8]) {
    if c_out != rust_out {
        panic!(
            "output mismatch for {case}\n  C    ({} bytes): {:?}\n  Rust ({} bytes): {:?}",
            c_out.len(),
            String::from_utf8_lossy(c_out),
            rust_out.len(),
            String::from_utf8_lossy(rust_out),
        );
    }
}
