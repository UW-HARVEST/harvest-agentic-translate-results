// Compare Rust .so output against the C .so output via libloading.
//
// We capture stdout using a pipe + dup2 around each call so we can compare the
// bytes printed by printf() (in C) and printf() (in Rust via libc).

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::{FromRawFd, IntoRawFd, RawFd};
use std::sync::Mutex;

// Serialize stdout-capture across tests in the same process. Tests within
// libtest run in parallel by default; sharing fd 1 between threads via dup2
// races and produces wrong results, so we protect captures with a mutex.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn c_lib_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libdriver.so", manifest_dir)
}

fn rust_lib_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    // cargo builds tests with the same profile; the cdylib is at target/<profile>/libdriver.so
    // We try release first, then debug.
    let release = format!("{}/target/release/libdriver.so", manifest_dir);
    let debug = format!("{}/target/debug/libdriver.so", manifest_dir);
    if std::path::Path::new(&release).exists() {
        release
    } else {
        debug
    }
}

/// Capture everything written to stdout (file descriptor 1) by the closure
/// `f` and return it as a Vec<u8>.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    // Flush stdout first.
    use std::io::Write;
    let _ = std::io::stdout().flush();
    unsafe {
        // Also flush libc's stdout.
        libc::fflush(std::ptr::null_mut());
    }

    // Create pipe.
    let mut fds: [RawFd; 2] = [0, 0];
    let r = unsafe { libc::pipe(fds.as_mut_ptr()) };
    assert_eq!(r, 0, "pipe failed");
    let read_fd = fds[0];
    let write_fd = fds[1];

    // Save original stdout.
    let saved = unsafe { libc::dup(1) };
    assert!(saved >= 0, "dup failed");

    // Replace stdout with the write end of the pipe.
    let r = unsafe { libc::dup2(write_fd, 1) };
    assert!(r >= 0, "dup2 failed");
    unsafe { libc::close(write_fd) };

    // Run the closure.
    f();

    // Flush again to make sure all output is in the pipe.
    let _ = std::io::stdout().flush();
    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    // Restore stdout.
    let r = unsafe { libc::dup2(saved, 1) };
    assert!(r >= 0, "restore dup2 failed");
    unsafe { libc::close(saved) };

    // Read from read end.
    let mut output = Vec::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(read_fd) };
    let _ = file.read_to_end(&mut output);
    // file is dropped which closes read_fd
    let _ = file.into_raw_fd();
    unsafe { libc::close(read_fd) };
    output
}

#[test]
fn test_print_int_ptr_line_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_fn: Symbol<unsafe extern "C" fn(*const c_int)> =
        unsafe { c_lib.get(b"printIntPtrLine").expect("C printIntPtrLine") };
    let rust_fn: Symbol<unsafe extern "C" fn(*const c_int)> =
        unsafe { rust_lib.get(b"printIntPtrLine").expect("Rust printIntPtrLine") };

    let test_values: &[c_int] = &[0, 1, -1, 5, 42, -42, i32::MAX, i32::MIN, 1234567];

    for &v in test_values {
        let ptr = &v as *const c_int;

        let c_out = capture_stdout(|| unsafe { c_fn(ptr) });
        let rust_out = capture_stdout(|| unsafe { rust_fn(ptr) });

        assert_eq!(
            c_out, rust_out,
            "printIntPtrLine output mismatch for value {}",
            v
        );
    }
}

#[test]
fn test_good_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c_lib.get(b"good").expect("C good") };
    let rust_fn: Symbol<unsafe extern "C" fn()> =
        unsafe { rust_lib.get(b"good").expect("Rust good") };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let rust_out = capture_stdout(|| unsafe { rust_fn() });

    assert_eq!(c_out, rust_out, "good() output mismatch");
    // good() should print "5\n"
    assert_eq!(c_out, b"5\n");
}

#[test]
fn test_driver_use_good_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    let c_fn: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c_lib.get(b"driver").expect("C driver") };
    let rust_fn: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { rust_lib.get(b"driver").expect("Rust driver") };

    // Only test useGood=1; useGood=0 calls bad() which dereferences an
    // uninitialized pointer -- undefined behavior that may segfault and
    // certainly will not produce deterministic output.
    for &use_good in &[1, 2, 100] {
        let c_out = capture_stdout(|| unsafe { c_fn(use_good) });
        let rust_out = capture_stdout(|| unsafe { rust_fn(use_good) });
        assert_eq!(
            c_out, rust_out,
            "driver({}) output mismatch",
            use_good
        );
    }
}

#[test]
fn test_symbols_exported() {
    // Verify that every symbol exported by the C .so is also exported by the
    // Rust .so.
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    for sym in &["bad", "good", "driver", "printIntPtrLine"] {
        let _: Symbol<unsafe extern "C" fn()> = unsafe {
            c_lib
                .get(sym.as_bytes())
                .unwrap_or_else(|_| panic!("C lib missing {}", sym))
        };
        let _: Symbol<unsafe extern "C" fn()> = unsafe {
            rust_lib
                .get(sym.as_bytes())
                .unwrap_or_else(|_| panic!("Rust lib missing {}", sym))
        };
    }
}
