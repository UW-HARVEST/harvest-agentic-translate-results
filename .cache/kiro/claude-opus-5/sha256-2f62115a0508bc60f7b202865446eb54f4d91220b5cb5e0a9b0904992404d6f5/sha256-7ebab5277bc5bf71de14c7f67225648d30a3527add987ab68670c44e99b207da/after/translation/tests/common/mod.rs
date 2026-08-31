//! Shared harness: loads the C and Rust shared libraries via `libloading`
//! and captures the bytes each one writes to stdout.

use std::ffi::{CString, c_char, c_int, c_void};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;

use libloading::{Library, Symbol};

unsafe extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes every open stdio stream in the process, which
    /// covers the streams used by both loaded libraries (they share libc).
    fn fflush(stream: *mut c_void) -> c_int;
}

pub type DriverFn = unsafe extern "C" fn(*const c_char, *const c_char);

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR == <root>/translation
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("translation/ has a parent")
        .to_path_buf()
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    // Prefer the .so built with the same profile as this test binary, so a
    // debug-profile test run exercises the debug cdylib.
    let (first, second) = if cfg!(debug_assertions) {
        ("debug", "release")
    } else {
        ("release", "debug")
    };
    for dir in [first, second] {
        let c = base.join(dir).join("libdriver.so");
        if c.exists() {
            return c;
        }
    }
    panic!("no Rust libdriver.so found; run `cargo build --release` first");
}

pub struct Libs {
    c: Library,
    rust: Library,
}

impl Libs {
    pub fn load() -> Self {
        let cp = c_lib_path();
        let rp = rust_lib_path();
        assert!(cp.exists(), "missing C library at {cp:?}; build c_src first");
        // SAFETY: both paths point at plain shared objects whose initialisers
        // are benign; loading them is exactly what an external caller does.
        unsafe {
            Libs {
                c: Library::new(&cp).unwrap_or_else(|e| panic!("dlopen {cp:?}: {e}")),
                rust: Library::new(&rp).unwrap_or_else(|e| panic!("dlopen {rp:?}: {e}")),
            }
        }
    }

    fn sym(lib: &Library, name: &str) -> DriverFn {
        // SAFETY: `driver` has the signature `void(const char*, const char*)`
        // in both libraries, matching `DriverFn`.
        let s: Symbol<DriverFn> = unsafe {
            lib.get(name.as_bytes())
                .unwrap_or_else(|e| panic!("dlsym {name}: {e}"))
        };
        *s
    }

    pub fn c_driver(&self) -> DriverFn {
        Self::sym(&self.c, "driver")
    }

    pub fn rust_driver(&self) -> DriverFn {
        Self::sym(&self.rust, "driver")
    }
}

/// Redirecting fd 1 is process-global, so captures must not overlap.
static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Runs `f` with file descriptor 1 redirected to a temporary file and returns
/// every byte written to it (stdio buffers are flushed before restoring).
pub fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = CAPTURE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut tmp = std::env::temp_dir();
    tmp.push(format!(
        "driver-capture-{}-{:?}.txt",
        std::process::id(),
        std::thread::current().id()
    ));
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(&tmp)
        .expect("create capture file");
    let fd = {
        use std::os::unix::io::AsRawFd;
        file.as_raw_fd()
    };

    // SAFETY: plain POSIX fd juggling on descriptors we own.
    let mut file = unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(saved);
        file
    };

    let mut out = Vec::new();
    file.seek(SeekFrom::Start(0)).expect("seek");
    file.read_to_end(&mut out).expect("read capture");
    drop(file);
    let _ = std::fs::remove_file(&tmp);
    out
}

/// Calls `driver` in both libraries with the same inputs and asserts the bytes
/// they print are identical.
pub fn compare(libs: &Libs, s1: &[u8], s2: &[u8], label: &str) {
    let a = CString::new(s1).expect("s1 contains an interior NUL");
    let b = CString::new(s2).expect("s2 contains an interior NUL");

    let c_fn = libs.c_driver();
    let rust_fn = libs.rust_driver();

    // SAFETY: both CStrings are NUL-terminated and outlive the calls.
    let c_out = capture_stdout(|| unsafe { c_fn(a.as_ptr(), b.as_ptr()) });
    let rust_out = capture_stdout(|| unsafe { rust_fn(a.as_ptr(), b.as_ptr()) });

    assert_eq!(
        c_out,
        rust_out,
        "output mismatch for {label}\n  s1 = {:?}\n  s2 = {:?}\n  C    = {:?}\n  Rust = {:?}",
        String::from_utf8_lossy(s1),
        String::from_utf8_lossy(s2),
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out),
    );
}
