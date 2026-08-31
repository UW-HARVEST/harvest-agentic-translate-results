//! Shared test harness: loads the C and Rust shared objects via `libloading`
//! and captures everything they write to stdout so the two can be compared
//! byte-for-byte.
//!
//! NOTE: `dlopen` returns the *same* mapping for a library that is already
//! loaded in the process, and each library owns a mutable `the_house` global.
//! State therefore accumulates for the lifetime of the test binary. To keep
//! comparisons meaningful every test file contains exactly one `#[test]` (cargo
//! gives each file its own process) and issues an identical call sequence to
//! both implementations.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};
use std::io::Read;
use std::path::PathBuf;

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const STDOUT_FILENO: c_int = 1;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the C shared library produced by the CMake build.
pub fn c_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("C_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let p = manifest_dir().join("../c_src/build/libdriver.so");
    assert!(
        p.exists(),
        "C shared library not found at {}. Build it with:\n  cd c_src && mkdir -p build && cd build && \
         cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .",
        p.display()
    );
    p
}

/// Path to the Rust `cdylib`. Picks whichever of debug/release is newest so the
/// tests work under both `cargo test` and `cargo test --release`.
pub fn rust_lib_path() -> PathBuf {
    if let Ok(p) = std::env::var("RUST_DRIVER_SO") {
        return PathBuf::from(p);
    }
    let root = manifest_dir().join("target");
    let candidates = [
        root.join("debug/libdriver.so"),
        root.join("release/libdriver.so"),
    ];
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for c in candidates {
        if let Ok(md) = std::fs::metadata(&c) {
            let t = md.modified().unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().map(|(bt, _)| t > *bt).unwrap_or(true) {
                best = Some((t, c));
            }
        }
    }
    best.map(|(_, p)| p).unwrap_or_else(|| {
        panic!(
            "Rust cdylib not found under {}. Run `cargo build` first.",
            root.display()
        )
    })
}

/// Runs `f` with fd 1 redirected into a temporary file and returns the bytes
/// written. Flushes libc's stdout buffer on both sides of the call so nothing
/// leaks between captures.
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        fflush(std::ptr::null_mut());
    }

    let mut path = std::env::temp_dir();
    path.push(format!(
        "driver_capture_{}_{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::File::create(&path).expect("create capture file");
    let tmp_fd = {
        use std::os::fd::AsRawFd;
        file.as_raw_fd()
    };

    let saved = unsafe { dup(STDOUT_FILENO) };
    assert!(saved >= 0, "dup(stdout) failed");
    assert!(
        unsafe { dup2(tmp_fd, STDOUT_FILENO) } >= 0,
        "dup2 onto stdout failed"
    );

    f();

    unsafe {
        fflush(std::ptr::null_mut());
        dup2(saved, STDOUT_FILENO);
        close(saved);
    }
    drop(file);

    let mut out = Vec::new();
    std::fs::File::open(&path)
        .expect("reopen capture file")
        .read_to_end(&mut out)
        .expect("read capture file");
    let _ = std::fs::remove_file(&path);
    out
}

/// A loaded implementation (either the C or the Rust one).
pub struct Impl {
    pub name: &'static str,
    lib: Library,
}

impl Impl {
    pub fn load_c() -> Impl {
        Impl {
            name: "C",
            lib: unsafe { Library::new(c_lib_path()).expect("load C libdriver.so") },
        }
    }

    pub fn load_rust() -> Impl {
        Impl {
            name: "Rust",
            lib: unsafe { Library::new(rust_lib_path()).expect("load Rust libdriver.so") },
        }
    }

    fn sym(&self, name: &str) -> Symbol<'_, unsafe extern "C" fn(c_int)> {
        unsafe {
            self.lib
                .get(name.as_bytes())
                .unwrap_or_else(|e| panic!("{} lib is missing symbol `{}`: {}", self.name, name, e))
        }
    }

    /// Calls the exported `run(int)` and returns what it printed.
    pub fn run(&self, extra_bedrooms: c_int) -> Vec<u8> {
        let f = self.sym("run");
        capture_stdout(|| unsafe { f(extra_bedrooms) })
    }

    /// Calls the exported `driver(int)` and returns what it printed.
    pub fn driver(&self, x: c_int) -> Vec<u8> {
        let f = self.sym("driver");
        capture_stdout(|| unsafe { f(x) })
    }
}

/// Loads both libraries. See the module note: this is *not* a state reset.
pub fn load_pair() -> (Impl, Impl) {
    (Impl::load_c(), Impl::load_rust())
}

#[track_caller]
pub fn assert_same(label: &str, c_out: &[u8], rust_out: &[u8]) {
    if c_out != rust_out {
        panic!(
            "output mismatch for {label}\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(c_out),
            String::from_utf8_lossy(rust_out)
        );
    }
}
