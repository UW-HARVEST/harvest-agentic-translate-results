//! Shared harness: loads the C and the Rust shared objects side by side and
//! compares their behaviour (return values *and* the bytes they write to the
//! C stdout/stderr streams) through the FFI boundary only.

#![allow(dead_code)]

use libloading::{Library, Symbol};
use std::ffi::{CString, c_char, c_int, c_uint};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::OnceLock;

unsafe extern "C" {
    fn fflush(stream: *mut std::ffi::c_void) -> c_int;
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int;
    fn unsetenv(name: *const c_char) -> c_int;
}

pub struct Libs {
    pub c: Library,
    pub rs: Library,
}

fn c_so_path() -> PathBuf {
    // translation/ -> workspace root -> c_src/build
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .to_path_buf();
    let build = root.join("c_src/build");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(&build)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}. Build the C library first.", build.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("lib") && n.ends_with(".so"))
                .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    candidates
        .pop()
        .unwrap_or_else(|| panic!("no .so found in {}", build.display()))
}

fn rust_so_path() -> PathBuf {
    // The test executable lives in target/<profile>/deps/
    let mut dir = std::env::current_exe().expect("current_exe");
    dir.pop(); // test binary name
    if dir.file_name().and_then(|n| n.to_str()) == Some("deps") {
        dir.pop();
    }
    let p = dir.join("libenvy_lib.so");
    assert!(
        p.exists(),
        "Rust cdylib not found at {} - run `cargo build` first",
        p.display()
    );

    // `cargo test` does *not* relink a cdylib-only library, so a stale .so
    // would silently make every differential test pass. Refuse to run unless
    // the artifact is at least as new as the sources.
    let so_mtime = std::fs::metadata(&p)
        .and_then(|m| m.modified())
        .expect("cdylib mtime");
    let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut newest_src = std::time::SystemTime::UNIX_EPOCH;
    let mut newest_name = String::new();
    let mut stack = vec![src_dir];
    while let Some(d) = stack.pop() {
        for entry in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                if let Ok(t) = entry.metadata().and_then(|m| m.modified()) {
                    if t > newest_src {
                        newest_src = t;
                        newest_name = path.display().to_string();
                    }
                }
            }
        }
    }
    assert!(
        so_mtime >= newest_src,
        "{} is older than {} - the differential tests would compare against a \
         stale library. Run `cargo build` (same profile) before `cargo test`.",
        p.display(),
        newest_name
    );
    p
}

static LIBS: OnceLock<Libs> = OnceLock::new();

pub fn libs() -> &'static Libs {
    LIBS.get_or_init(|| unsafe {
        let c = Library::new(c_so_path()).expect("load C .so");
        let rs = Library::new(rust_so_path()).expect("load Rust .so");
        Libs { c, rs }
    })
}

/// Global lock: the tests mutate process-wide environment variables and
/// redirect process-wide file descriptors, so they must not overlap.
pub fn guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<std::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| std::sync::Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

// --------------------------------------------------------------------------
// Environment handling
// --------------------------------------------------------------------------

pub const ENV_NAMES: [&str; 5] = [
    "PROG_VERBOSE",
    "PROG_DEBUG",
    "PROG_OPTIMIZE",
    "PROG_BASE_OFFSET",
    "PROG_MULTIPLIER",
];

/// Applies an environment description: `None` means "unset".
pub fn set_env(vars: &[(&str, Option<&str>)]) {
    for name in ENV_NAMES {
        let cname = CString::new(name).unwrap();
        unsafe {
            unsetenv(cname.as_ptr());
        }
    }
    for (name, value) in vars {
        let cname = CString::new(*name).unwrap();
        match value {
            Some(v) => {
                let cvalue = CString::new(*v).unwrap();
                unsafe {
                    setenv(cname.as_ptr(), cvalue.as_ptr(), 1);
                }
            }
            None => unsafe {
                unsetenv(cname.as_ptr());
            },
        }
    }
}

// --------------------------------------------------------------------------
// stdout / stderr capture
// --------------------------------------------------------------------------

pub struct Captured<T> {
    pub value: T,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl<T> Captured<T> {
    pub fn out_str(&self) -> String {
        String::from_utf8_lossy(&self.stdout).into_owned()
    }
    pub fn err_str(&self) -> String {
        String::from_utf8_lossy(&self.stderr).into_owned()
    }
}

/// Scratch files are created once and reused: the differential tests run tens
/// of thousands of captures, so per-call file creation would dominate runtime.
struct Scratch {
    out: std::fs::File,
    err: std::fs::File,
}

fn scratch() -> &'static std::sync::Mutex<Scratch> {
    static SCRATCH: OnceLock<std::sync::Mutex<Scratch>> = OnceLock::new();
    SCRATCH.get_or_init(|| {
        let dir = std::env::temp_dir();
        let mk = |tag: &str| {
            let path = dir.join(format!("c2rust_cap_{}_{}", tag, std::process::id()));
            let f = std::fs::File::options()
                .read(true)
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)
                .expect("temp capture file");
            // Unlink immediately; the open handle keeps it alive.
            let _ = std::fs::remove_file(&path);
            f
        };
        std::sync::Mutex::new(Scratch {
            out: mk("out"),
            err: mk("err"),
        })
    })
}

/// Runs `f` with fds 1 and 2 redirected to scratch files and returns
/// everything that was written, byte for byte.
pub fn capture<T, F: FnOnce() -> T>(f: F) -> Captured<T> {
    use std::os::fd::AsRawFd;

    let mut s = scratch().lock().unwrap_or_else(|e| e.into_inner());
    s.out.set_len(0).unwrap();
    s.err.set_len(0).unwrap();
    s.out.seek(SeekFrom::Start(0)).unwrap();
    s.err.seek(SeekFrom::Start(0)).unwrap();

    let value;
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    unsafe {
        // Flush anything already pending so it does not leak into the capture.
        fflush(std::ptr::null_mut());
        let saved_out = dup(1);
        let saved_err = dup(2);
        assert!(saved_out >= 0 && saved_err >= 0);
        dup2(s.out.as_raw_fd(), 1);
        dup2(s.err.as_raw_fd(), 2);

        value = f();

        fflush(std::ptr::null_mut());
        dup2(saved_out, 1);
        dup2(saved_err, 2);
        close(saved_out);
        close(saved_err);
    }

    s.out.seek(SeekFrom::Start(0)).unwrap();
    s.err.seek(SeekFrom::Start(0)).unwrap();
    s.out.read_to_end(&mut stdout).unwrap();
    s.err.read_to_end(&mut stderr).unwrap();

    Captured {
        value,
        stdout,
        stderr,
    }
}

// --------------------------------------------------------------------------
// Typed symbol accessors (identical signatures for both libraries)
// --------------------------------------------------------------------------

pub type FnEnvy = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
pub type FnParseEnvNumeric = unsafe extern "C" fn(*const c_char, c_int) -> c_int;
pub type FnInitConfig = unsafe extern "C" fn(*mut c_uint);
pub type FnPerformOperation = unsafe extern "C" fn(c_int, c_int, *mut c_uint) -> c_int;
pub type FnApplyBitOps = unsafe extern "C" fn(c_int, *mut c_uint) -> c_int;

pub fn sym<T>(lib: &'static Library, name: &str) -> Symbol<'static, T> {
    unsafe {
        lib.get(CString::new(name).unwrap().as_bytes_with_nul())
            .unwrap_or_else(|e| panic!("symbol {name} missing: {e}"))
    }
}

pub fn pair<T>(name: &str) -> (Symbol<'static, T>, Symbol<'static, T>) {
    let l = libs();
    (sym::<T>(&l.c, name), sym::<T>(&l.rs, name))
}

/// Asserts two captures are identical, printing a helpful diff on failure.
pub fn assert_same<T: PartialEq + std::fmt::Debug>(
    ctx: &str,
    c: &Captured<T>,
    r: &Captured<T>,
) {
    assert_eq!(
        c.value, r.value,
        "return value mismatch [{ctx}]\n  C : {:?}\n  Rs: {:?}",
        c.value, r.value
    );
    assert_eq!(
        c.stdout, r.stdout,
        "stdout mismatch [{ctx}]\n  C : {:?}\n  Rs: {:?}",
        c.out_str(),
        r.out_str()
    );
    assert_eq!(
        c.stderr, r.stderr,
        "stderr mismatch [{ctx}]\n  C : {:?}\n  Rs: {:?}",
        c.err_str(),
        r.err_str()
    );
}
