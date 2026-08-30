//! Shared harness: loads both the C and the Rust shared libraries through
//! `libloading` and captures whatever they write to `stdout` (fd 1) so the two
//! implementations can be compared byte-for-byte.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open C stream, which is what the C and the
    /// Rust library both write through (`printf`).
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
}

/// fd redirection is process-global, so every capture has to be serialized.
fn capture_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    match LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared library built from `c_src/`.
pub fn c_lib_path() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("manifest dir has a parent")
        .join("c_src/build/libdriver.so");
    assert!(
        p.is_file(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Path to the Rust `cdylib`. `cargo test` builds the lib target for the
/// current profile, so look next to the test executable first and fall back to
/// the well-known profile directories.
pub fn rust_lib_path() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        // .../target/<profile>/deps/<test-bin>
        if let Some(deps) = exe.parent() {
            candidates.push(deps.join("libdriver.so"));
            if let Some(profile_dir) = deps.parent() {
                candidates.push(profile_dir.join("libdriver.so"));
            }
        }
    }
    let target = manifest_dir().join("target");
    candidates.push(target.join("debug/libdriver.so"));
    candidates.push(target.join("release/libdriver.so"));

    for c in &candidates {
        if c.is_file() {
            return c.clone();
        }
    }
    panic!(
        "Rust cdylib libdriver.so not found; looked in: {:?}",
        candidates
    );
}

pub struct Libs {
    pub c: Library,
    pub rust: Library,
}

impl Libs {
    pub fn load() -> &'static Libs {
        static LIBS: OnceLock<Libs> = OnceLock::new();
        LIBS.get_or_init(|| {
            // RTLD_LOCAL (libloading's default) keeps the two libraries'
            // identically-named symbols from interposing on each other.
            let c = unsafe { Library::new(c_lib_path()) }.expect("dlopen C library");
            let rust = unsafe { Library::new(rust_lib_path()) }.expect("dlopen Rust library");
            Libs { c, rust }
        })
    }
}

/// Redirect fd 1 into a temp file, run `f`, flush all C streams, restore fd 1
/// and return the raw bytes that were written.
fn capture<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = capture_lock();

    // Make sure nothing already buffered leaks into this capture.
    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver_capture_{}_{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));

    let file = std::fs::File::create(&path).expect("create capture file");
    let saved = unsafe { dup(1) };
    assert!(saved >= 0, "dup(1) failed");
    assert!(unsafe { dup2(file.as_raw_fd(), 1) } >= 0, "dup2 failed");

    f();

    unsafe { fflush(std::ptr::null_mut()) };
    let _ = std::io::Write::flush(&mut std::io::stdout());

    assert!(unsafe { dup2(saved, 1) } >= 0, "restore dup2 failed");
    unsafe { close(saved) };
    drop(file);

    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}

/// Call a `void fn(void)` exported by both libraries and return
/// `(c_stdout, rust_stdout)`.
pub fn run_void(name: &str) -> (Vec<u8>, Vec<u8>) {
    let libs = Libs::load();
    let cf: Symbol<unsafe extern "C" fn()> =
        unsafe { libs.c.get(name.as_bytes()) }.unwrap_or_else(|e| panic!("C {name}: {e}"));
    let rf: Symbol<unsafe extern "C" fn()> =
        unsafe { libs.rust.get(name.as_bytes()) }.unwrap_or_else(|e| panic!("Rust {name}: {e}"));

    let c_out = capture(|| unsafe { cf() });
    let r_out = capture(|| unsafe { rf() });
    (c_out, r_out)
}

/// Call a `void fn(int)` exported by both libraries.
pub fn run_int(name: &str, arg: c_int) -> (Vec<u8>, Vec<u8>) {
    let libs = Libs::load();
    let cf: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { libs.c.get(name.as_bytes()) }.unwrap_or_else(|e| panic!("C {name}: {e}"));
    let rf: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { libs.rust.get(name.as_bytes()) }.unwrap_or_else(|e| panic!("Rust {name}: {e}"));

    let c_out = capture(|| unsafe { cf(arg) });
    let r_out = capture(|| unsafe { rf(arg) });
    (c_out, r_out)
}

/// Call a `void fn(const char *)` exported by both libraries. `arg` must be
/// NUL-terminated (or null).
pub fn run_str(name: &str, arg: *const std::ffi::c_char) -> (Vec<u8>, Vec<u8>) {
    let libs = Libs::load();
    let cf: Symbol<unsafe extern "C" fn(*const std::ffi::c_char)> =
        unsafe { libs.c.get(name.as_bytes()) }.unwrap_or_else(|e| panic!("C {name}: {e}"));
    let rf: Symbol<unsafe extern "C" fn(*const std::ffi::c_char)> =
        unsafe { libs.rust.get(name.as_bytes()) }.unwrap_or_else(|e| panic!("Rust {name}: {e}"));

    let c_out = capture(|| unsafe { cf(arg) });
    let r_out = capture(|| unsafe { rf(arg) });
    (c_out, r_out)
}

pub fn assert_same(label: &str, c_out: &[u8], r_out: &[u8]) {
    assert_eq!(
        c_out,
        r_out,
        "{label}: stdout mismatch\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(c_out),
        String::from_utf8_lossy(r_out)
    );
}
