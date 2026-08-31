//! Shared helpers for the differential C-vs-Rust tests.
//!
//! Both libraries are loaded as shared objects via `libloading` and called
//! purely through their exported C ABI symbols, so the `#[no_mangle]` export
//! wrappers are exercised exactly as an external caller would exercise them.

#![allow(dead_code)]

use std::ffi::{c_int, c_void};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open C stdio stream in the process.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Directory holding the crate (`translation/`).
pub fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The cargo target profile directory that this test binary lives in
/// (`target/debug` or `target/release`), derived from the running executable
/// so the tests work under any profile.
pub fn profile_dir() -> PathBuf {
    let exe = std::env::current_exe().expect("current_exe");
    // .../target/<profile>/deps/<test-bin>
    exe.parent()
        .and_then(Path::parent)
        .expect("profile dir")
        .to_path_buf()
}

/// Path to the Rust `cdylib` produced by this crate.
pub fn rust_so() -> PathBuf {
    let p = profile_dir().join("libdriver.so");
    assert!(
        p.exists(),
        "Rust shared library not found at {}. Run `cargo build` first.",
        p.display()
    );
    p
}

/// Path to the C shared library built from `c_src/`.
pub fn c_so() -> PathBuf {
    let p = manifest_dir()
        .parent()
        .expect("workspace root")
        .join("c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Loads both libraries. Returned in `(c, rust)` order.
pub fn load_both() -> (libloading::Library, libloading::Library) {
    unsafe {
        let c = libloading::Library::new(c_so()).expect("load C .so");
        let r = libloading::Library::new(rust_so()).expect("load Rust .so");
        (c, r)
    }
}

/// Runs `f` with file descriptor 1 redirected to a temporary file and returns
/// everything that was written, including anything sitting in C stdio buffers.
pub fn capture_stdout<F: FnOnce()>(tag: &str, f: F) -> Vec<u8> {
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{}_{}.txt",
        std::process::id(),
        tag,
        n
    ));

    let file = std::fs::File::create(&path).expect("create capture file");

    unsafe {
        // Drain anything already pending so it is not attributed to `f`.
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
    }

    drop(file);
    let bytes = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    bytes
}
