// Integration tests that load BOTH the C .so and the Rust .so via libloading
// and compare their outputs through the FFI boundary.

use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};
use std::fs;
use std::io::Read;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_so_path() -> PathBuf {
    manifest_dir().join("c_src").join("build").join("libdriver.so")
}

fn rust_so_path() -> PathBuf {
    // Standard cargo target dir for cdylib
    let mut p = manifest_dir().join("target").join("debug").join("libdriver.so");
    if !p.exists() {
        // Fallback to release
        p = manifest_dir().join("target").join("release").join("libdriver.so");
    }
    p
}

/// Capture stdout (and stderr) of a closure that calls into a shared library.
/// We redirect file-descriptor 1 (and 2) to a temp file, then read that file
/// after the closure returns. This works for libc `printf` which writes to
/// the standard C `stdout` (FILE*) -- the underlying fd is 1.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    // Make sure C-level stdout buffer is flushed before redirection.
    extern "C" {
        fn fflush(stream: *mut libc_file) -> i32;
    }
    #[allow(non_camel_case_types)]
    type libc_file = std::ffi::c_void;

    // Use a temp file as the redirection target
    let tmp_path = std::env::temp_dir().join(format!(
        "driver_capture_{}_{}.out",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    let tmp_file = fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .read(true)
        .write(true)
        .open(&tmp_path)
        .expect("failed to open tmp file");

    let tmp_fd = tmp_file.as_raw_fd();

    unsafe {
        // Flush C stdout first
        let stdout_ptr: *mut libc_file = std::ptr::null_mut();
        let _ = fflush(stdout_ptr); // fflush(NULL) flushes all open output streams

        // Save current stdout fd
        let saved_stdout = libc::dup(1);
        assert!(saved_stdout >= 0, "dup failed");

        // Redirect fd 1 to the tmp file
        let r = libc::dup2(tmp_fd, 1);
        assert!(r >= 0, "dup2 failed");

        // Run the closure
        f();

        // Flush again
        let _ = fflush(stdout_ptr);

        // Restore stdout
        let r = libc::dup2(saved_stdout, 1);
        assert!(r >= 0, "dup2 restore failed");
        libc::close(saved_stdout);
    }

    // Read what was written
    let mut buf = Vec::new();
    let mut f = fs::File::open(&tmp_path).expect("failed to reopen tmp file");
    f.read_to_end(&mut buf).expect("failed to read tmp file");
    let _ = fs::remove_file(&tmp_path);
    buf
}

mod libc {
    extern "C" {
        pub fn dup(oldfd: i32) -> i32;
        pub fn dup2(oldfd: i32, newfd: i32) -> i32;
        pub fn close(fd: i32) -> i32;
    }
}

fn load_lib(path: &PathBuf) -> Library {
    unsafe { Library::new(path).unwrap_or_else(|e| panic!("failed to load {:?}: {}", path, e)) }
}

#[test]
fn test_print_line_with_string() {
    let c_lib = load_lib(&c_so_path());
    let r_lib = load_lib(&rust_so_path());

    let s = b"hello world\0";
    let ptr = s.as_ptr() as *const c_char;

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").unwrap();
        f(ptr);
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            r_lib.get(b"printLine").unwrap();
        f(ptr);
    });

    assert_eq!(c_out, r_out, "printLine output mismatch");
    assert_eq!(c_out, b"hello world\n");
}

#[test]
fn test_print_line_with_empty_string() {
    let c_lib = load_lib(&c_so_path());
    let r_lib = load_lib(&rust_so_path());

    let s = b"\0";
    let ptr = s.as_ptr() as *const c_char;

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").unwrap();
        f(ptr);
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            r_lib.get(b"printLine").unwrap();
        f(ptr);
    });

    assert_eq!(c_out, r_out, "printLine empty string output mismatch");
    assert_eq!(c_out, b"\n");
}

#[test]
fn test_print_line_with_null() {
    let c_lib = load_lib(&c_so_path());
    let r_lib = load_lib(&rust_so_path());

    let ptr: *const c_char = std::ptr::null();

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            c_lib.get(b"printLine").unwrap();
        f(ptr);
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(*const c_char)> =
            r_lib.get(b"printLine").unwrap();
        f(ptr);
    });

    assert_eq!(c_out, r_out, "printLine NULL output mismatch");
    assert_eq!(c_out, b"");
}

#[test]
fn test_good() {
    let c_lib = load_lib(&c_so_path());
    let r_lib = load_lib(&rust_so_path());

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = c_lib.get(b"good").unwrap();
        f();
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn()> = r_lib.get(b"good").unwrap();
        f();
    });

    assert_eq!(c_out, r_out, "good output mismatch");
    assert_eq!(c_out, b"string\n");
}

#[test]
fn test_driver_use_good() {
    let c_lib = load_lib(&c_so_path());
    let r_lib = load_lib(&rust_so_path());

    let c_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> = c_lib.get(b"driver").unwrap();
        f(1);
    });
    let r_out = capture_stdout(|| unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int)> = r_lib.get(b"driver").unwrap();
        f(1);
    });

    assert_eq!(c_out, r_out, "driver(1) output mismatch");
    assert_eq!(c_out, b"string\n");
}

#[test]
fn test_driver_various_truthy_values() {
    let c_lib = load_lib(&c_so_path());
    let r_lib = load_lib(&rust_so_path());

    for &v in &[1i32, 2, -1, 100, i32::MAX, i32::MIN] {
        let c_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = c_lib.get(b"driver").unwrap();
            f(v);
        });
        let r_out = capture_stdout(|| unsafe {
            let f: Symbol<unsafe extern "C" fn(c_int)> = r_lib.get(b"driver").unwrap();
            f(v);
        });

        assert_eq!(c_out, r_out, "driver({}) output mismatch", v);
        assert_eq!(c_out, b"string\n");
    }
}

// NOTE: We deliberately do NOT exercise `bad()` or `driver(0)` because the C
// implementation of `bad()` reads from an uninitialized pointer
// (CWE-457: Use of Uninitialized Variable). The behavior is undefined --
// it may print arbitrary bytes or segfault. The Rust translation cannot
// produce byte-identical output for undefined-behavior inputs, and the
// documentation only requires byte-identical results for well-defined
// inputs. The exported symbol is still verified to exist via nm.
