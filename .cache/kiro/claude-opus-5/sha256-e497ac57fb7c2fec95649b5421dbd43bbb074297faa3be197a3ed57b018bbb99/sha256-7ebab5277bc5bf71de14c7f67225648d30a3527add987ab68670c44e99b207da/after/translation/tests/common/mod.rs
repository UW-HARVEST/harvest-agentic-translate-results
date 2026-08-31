//! Shared harness for the differential C-vs-Rust tests.
//!
//! Both implementations are loaded as shared objects with `libloading` and
//! invoked purely through their exported C symbols, so the `#[no_mangle]`
//! wrappers are part of what is under test.
//!
//! `driver` communicates its result by writing to `stdout` via C `printf`, so
//! comparing behaviour means capturing the process's file descriptor 1 around
//! each call. That is inherently global state, hence the `OUT_LOCK` mutex which
//! every capture must hold.

use std::ffi::{c_int, c_void};
use std::fs;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use libloading::{Library, Symbol};

/// `void driver(int)` — the single symbol exported by the C library.
pub type DriverFn = unsafe extern "C" fn(c_int);

unsafe extern "C" {
    /// `fflush(NULL)` flushes *every* open output stream, which is what we need:
    /// the buffered bytes may sit in the `stdout` FILE owned by libc and written
    /// to from inside either shared object.
    fn fflush(stream: *mut c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
}

/// Serialises stdout redirection across the (multi-threaded) test harness.
pub static OUT_LOCK: Mutex<()> = Mutex::new(());

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Fail loudly if `so` is older than any of its `sources`.
///
/// `cargo test` does *not* rebuild a `cdylib` artifact, so without this check a
/// stale `.so` from an earlier `cargo build` could be tested and pass.
fn assert_fresh(so: &Path, sources: &[PathBuf], how_to_build: &str) {
    let so_time = fs::metadata(so)
        .and_then(|m| m.modified())
        .unwrap_or_else(|e| panic!("stat {}: {e}", so.display()));
    for src in sources {
        let Ok(src_time) = fs::metadata(src).and_then(|m| m.modified()) else {
            continue;
        };
        assert!(
            so_time >= src_time,
            "{} is older than {} — it is stale. Rebuild with:\n  {how_to_build}",
            so.display(),
            src.display()
        );
    }
}

/// Locate the freshly built C shared library.
pub fn c_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().parent().expect("translation/ has a parent").to_path_buf();
    let candidate = root.join("c_src/build/libdriver.so");
    assert!(
        candidate.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        candidate.display()
    );
    assert_fresh(
        &candidate,
        &[root.join("c_src/src/driver.c"), root.join("c_src/include/driver.h")],
        "cd c_src/build && cmake --build .",
    );
    candidate
}

/// Locate the Rust `cdylib`.
///
/// `cargo test` builds the crate's lib target for the active profile, so prefer
/// the profile this test binary was compiled with and fall back to the other.
pub fn rust_so_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let target = manifest_dir().join("target");
    let (first, second) = if cfg!(debug_assertions) {
        ("debug", "release")
    } else {
        ("release", "debug")
    };
    for profile in [first, second] {
        let candidate = target.join(profile).join("libdriver.so");
        if candidate.exists() {
            assert_fresh(
                &candidate,
                &[
                    manifest_dir().join("src/lib.rs"),
                    manifest_dir().join("Cargo.toml"),
                ],
                if profile == "release" {
                    "cd translation && cargo build --release"
                } else {
                    "cd translation && cargo build"
                },
            );
            return candidate;
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found under {}. Build it with `cargo build`.",
        target.display()
    );
}

fn load(path: &Path) -> &'static Library {
    // Leaked deliberately: the loaded code must outlive every `Symbol` handed
    // out below, and the libraries stay mapped for the whole test run anyway.
    let lib = unsafe { Library::new(path) }
        .unwrap_or_else(|e| panic!("failed to dlopen {}: {e}", path.display()));
    Box::leak(Box::new(lib))
}

pub fn c_lib() -> &'static Library {
    static LIB: OnceLock<&'static Library> = OnceLock::new();
    LIB.get_or_init(|| load(&c_so_path()))
}

pub fn rust_lib() -> &'static Library {
    static LIB: OnceLock<&'static Library> = OnceLock::new();
    LIB.get_or_init(|| load(&rust_so_path()))
}

/// Resolve an exported symbol by its exact C name.
pub fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    let mut bytes = name.as_bytes().to_vec();
    bytes.push(0);
    unsafe { lib.get::<T>(&bytes) }
        .unwrap_or_else(|e| panic!("symbol `{name}` not exported: {e}"))
}

pub fn c_driver() -> Symbol<'static, DriverFn> {
    sym::<DriverFn>(c_lib(), "driver")
}

pub fn rust_driver() -> Symbol<'static, DriverFn> {
    sym::<DriverFn>(rust_lib(), "driver")
}

/// Run `f` with file descriptor 1 pointed at a temporary file and return every
/// byte it produced.
///
/// The caller must already hold [`OUT_LOCK`]. Redirecting the descriptor (rather
/// than swapping a Rust-level writer) is what lets us observe output emitted by
/// C `printf` from inside either shared object.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{}.bin",
        std::process::id(),
        TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let file = fs::File::create(&path).expect("create capture file");
    let target_fd = file.as_raw_fd();

    let saved = unsafe {
        // Drain anything already pending so it is not misattributed to `f`.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(target_fd, 1) >= 0, "dup2 onto stdout failed");
        saved
    };

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));

    unsafe {
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "failed to restore stdout");
        close(saved);
    }
    drop(file);

    let data = fs::read(&path).expect("read capture file");
    let _ = fs::remove_file(&path);

    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
    data
}

/// Call `driver(x)` in both libraries and return `(c_output, rust_output)`.
pub fn run_both(x: c_int) -> (Vec<u8>, Vec<u8>) {
    let c = c_driver();
    let r = rust_driver();
    let _guard = OUT_LOCK.lock().unwrap();
    let c_out = capture_stdout(|| unsafe { c(x) });
    let r_out = capture_stdout(|| unsafe { r(x) });
    (c_out, r_out)
}

/// Assert that `driver(x)` is byte-identical across the two implementations.
pub fn assert_driver_matches(x: c_int) {
    let (c_out, r_out) = run_both(x);
    assert_eq!(
        c_out,
        r_out,
        "driver({x}) output mismatch\n  C   : {:?} ({})\n  Rust: {:?} ({})",
        String::from_utf8_lossy(&c_out),
        hex(&c_out),
        String::from_utf8_lossy(&r_out),
        hex(&r_out),
    );
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
