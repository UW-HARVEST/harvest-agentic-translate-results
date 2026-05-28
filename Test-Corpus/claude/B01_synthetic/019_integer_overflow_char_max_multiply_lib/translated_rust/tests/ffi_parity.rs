//! Integration tests that compare the C shared library's output to the Rust
//! shared library's output through the FFI boundary.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

fn workspace_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_dir().join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // We build the Rust .so in release mode for testing.
    workspace_dir().join("target/release/libdriver.so")
}

fn ensure_built() {
    // Build the rust lib in release if not already
    let rust_so = rust_lib_path();
    if !rust_so.exists() {
        let status = std::process::Command::new("cargo")
            .args(["build", "--release"])
            .current_dir(workspace_dir())
            .status()
            .expect("cargo build");
        assert!(status.success(), "cargo build failed");
    }
    // Build the C lib if not already
    let c_so = c_lib_path();
    if !c_so.exists() {
        let build_dir = workspace_dir().join("c_src/build");
        std::fs::create_dir_all(&build_dir).unwrap();
        let s = std::process::Command::new("cmake")
            .args([".."])
            .args(["-DCMAKE_POSITION_INDEPENDENT_CODE=ON"])
            .current_dir(&build_dir)
            .status()
            .expect("cmake config");
        assert!(s.success());
        let s = std::process::Command::new("cmake")
            .args(["--build", "."])
            .current_dir(&build_dir)
            .status()
            .expect("cmake build");
        assert!(s.success());
    }
}

/// Capture everything written to stdout (fd 1) by the closure, including
/// writes done from C via printf. Returns the bytes captured.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Flush rust stdout before redirect
    std::io::stdout().flush().ok();

    // Save original fd 1
    let stdout_fd = std::io::stdout().as_raw_fd();
    let saved = unsafe { libc::dup(stdout_fd) };
    assert!(saved >= 0, "dup failed");

    // Create a temp file to redirect to
    let mut tmp = tempfile_like();

    // Redirect fd 1 to tmp file
    let tmp_fd = tmp.as_raw_fd();
    let r = unsafe { libc::dup2(tmp_fd, stdout_fd) };
    assert!(r >= 0, "dup2 failed");

    // Run closure
    f();

    // Flush both Rust's stdout and C's stdio (libc fflush(stdout))
    std::io::stdout().flush().ok();
    unsafe {
        // fflush(NULL) flushes all open streams - but to be safe, reach for stdout
        let stdout_stream = libc_stdout();
        if !stdout_stream.is_null() {
            libc::fflush(stdout_stream);
        }
    }

    // Restore original fd 1
    let r = unsafe { libc::dup2(saved, stdout_fd) };
    assert!(r >= 0, "dup2 restore failed");
    unsafe { libc::close(saved) };

    // Read tmp contents
    tmp.seek(SeekFrom::Start(0)).unwrap();
    let mut buf = Vec::new();
    tmp.read_to_end(&mut buf).unwrap();
    buf
}

// Returns the libc FILE* for stdout. We use the externally-defined
// `stdout` on glibc, which is a `FILE *`.
extern "C" {
    static stdout: *mut libc::FILE;
}
fn libc_stdout() -> *mut libc::FILE {
    unsafe { stdout }
}

fn tempfile_like() -> std::fs::File {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path =
        std::env::temp_dir().join(format!("ffi_parity_{}_{}.txt", std::process::id(), nanos));
    std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&path)
        .unwrap()
}

// ---------- Test fixtures ----------

struct Libs {
    c: Library,
    r: Library,
}

fn load_libs() -> Libs {
    ensure_built();
    unsafe {
        Libs {
            c: Library::new(c_lib_path()).expect("load c lib"),
            r: Library::new(rust_lib_path()).expect("load rust lib"),
        }
    }
}

// ---------- Tests ----------

#[test]
fn print_line_null() {
    let libs = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(*const c_char)> =
            libs.c.get(b"printLine").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(*const c_char)> =
            libs.r.get(b"printLine").unwrap();

        let c_out = capture_stdout(|| f_c(std::ptr::null()));
        let r_out = capture_stdout(|| f_r(std::ptr::null()));
        assert_eq!(c_out, r_out, "C={:?} R={:?}", c_out, r_out);
        assert_eq!(c_out, b"");
    }
}

#[test]
fn print_line_basic() {
    let libs = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(*const c_char)> =
            libs.c.get(b"printLine").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(*const c_char)> =
            libs.r.get(b"printLine").unwrap();

        for s in ["", "hello", "data value is too large to perform arithmetic safely."] {
            let cstr = CString::new(s).unwrap();
            let c_out = capture_stdout(|| f_c(cstr.as_ptr()));
            let r_out = capture_stdout(|| f_r(cstr.as_ptr()));
            assert_eq!(
                c_out, r_out,
                "mismatch for {:?}: C={:?} R={:?}",
                s, c_out, r_out
            );
        }
    }
}

#[test]
fn print_hex_char_line_all_values() {
    let libs = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(c_char)> = libs.c.get(b"printHexCharLine").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(c_char)> = libs.r.get(b"printHexCharLine").unwrap();

        // Test full range of i8 values, since char on this platform is signed.
        for v in i8::MIN..=i8::MAX {
            let arg = v as c_char;
            let c_out = capture_stdout(|| f_c(arg));
            let r_out = capture_stdout(|| f_r(arg));
            assert_eq!(c_out, r_out, "mismatch for v={}: C={:?} R={:?}", v, c_out, r_out);
        }
    }
}

#[test]
fn bad_func() {
    let libs = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn()> = libs.c.get(b"bad").unwrap();
        let f_r: Symbol<unsafe extern "C" fn()> = libs.r.get(b"bad").unwrap();

        let c_out = capture_stdout(|| f_c());
        let r_out = capture_stdout(|| f_r());
        assert_eq!(c_out, r_out, "C={:?} R={:?}", c_out, r_out);
    }
}

#[test]
fn good_func() {
    let libs = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn()> = libs.c.get(b"good").unwrap();
        let f_r: Symbol<unsafe extern "C" fn()> = libs.r.get(b"good").unwrap();

        let c_out = capture_stdout(|| f_c());
        let r_out = capture_stdout(|| f_r());
        assert_eq!(c_out, r_out, "C={:?} R={:?}", c_out, r_out);
    }
}

#[test]
fn driver_func() {
    let libs = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(c_int)> = libs.c.get(b"driver").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(c_int)> = libs.r.get(b"driver").unwrap();

        for v in [0, 1, -1, 2, 100] {
            let c_out = capture_stdout(|| f_c(v));
            let r_out = capture_stdout(|| f_r(v));
            assert_eq!(c_out, r_out, "mismatch for v={}: C={:?} R={:?}", v, c_out, r_out);
        }
    }
}
