//! FFI parity tests between the C shared library and the Rust shared library.
//!
//! Both libraries are loaded via libloading and the exported `driver` symbol
//! is invoked. `driver(x, y)` writes `(x | ~y)\n` to stdout, so we redirect
//! stdout to a pipe (using `dup`/`dup2`) before each call and read back the
//! captured bytes for comparison.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::path::PathBuf;
use std::sync::Mutex;

type DriverFn = unsafe extern "C" fn(c_int, c_int);

// Serialize stdout-capturing operations across the test binary so concurrent
// tests don't trample each other's redirected file descriptors.
static STDOUT_LOCK: Mutex<()> = Mutex::new(());

fn c_lib_path() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("c_src/build/libdriver_c.so")
}

fn rust_lib_path() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    // tests run with `cargo test`, which builds in target/debug by default.
    let debug = PathBuf::from(&manifest_dir).join("target/debug/libdriver.so");
    if debug.exists() {
        return debug;
    }
    PathBuf::from(&manifest_dir).join("target/release/libdriver.so")
}

/// Run `f` while redirecting fd 1 (stdout) into a pipe, then return whatever
/// was written.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Write;

    // Make sure Rust's stdout buffer is flushed before we swap the fd out
    // from underneath it.
    let _ = std::io::stdout().flush();

    unsafe {
        // Save a duplicate of the original stdout fd so we can restore it.
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup(1) failed");

        // Make a pipe.
        let mut fds = [0i32; 2];
        let pipe_rc = libc::pipe(fds.as_mut_ptr());
        assert!(pipe_rc == 0, "pipe() failed");

        // Replace stdout with the write end of the pipe.
        let dup_rc = libc::dup2(fds[1], 1);
        assert!(dup_rc >= 0, "dup2 failed");
        libc::close(fds[1]);

        // Run the function, then flush and restore.
        f();
        // Flush the C-level stdout (printf/puts buffer).
        let stdout_handle =
            libc::fdopen(libc::dup(1), b"w\0".as_ptr() as *const libc::c_char);
        if !stdout_handle.is_null() {
            libc::fflush(stdout_handle);
            libc::fclose(stdout_handle);
        }
        // Also try flushing the standard stdout stream itself.
        let _ = std::io::stdout().flush();

        // Restore the original stdout, which closes our pipe-write end as a
        // side effect of dup2.
        libc::dup2(saved, 1);
        libc::close(saved);

        // Read everything that was written into the pipe.
        let mut reader = std::fs::File::from_raw_fd(fds[0]);
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf);
        buf
    }
}

fn run_driver(lib_path: &std::path::Path, x: c_int, y: c_int) -> Vec<u8> {
    unsafe {
        let lib = Library::new(lib_path)
            .unwrap_or_else(|e| panic!("failed to load {}: {e}", lib_path.display()));
        let driver: Symbol<DriverFn> =
            lib.get(b"driver").expect("driver symbol not found");

        // The libc stdio stream needs to be told its underlying fd has changed
        // each time we install a new pipe; the simplest way is to call
        // setvbuf to force unbuffered mode, but in practice fflush before/after
        // and `fflush(stdout)` after the call is enough.
        capture_stdout(|| {
            driver(x, y);
            // flush libc stdout so any buffered printf output reaches the pipe
            extern "C" {
                static stdout: *mut libc::FILE;
            }
            libc::fflush(stdout);
        })
    }
}

#[test]
fn driver_matches_c_for_basic_inputs() {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let c_path = c_lib_path();
    let rust_path = rust_lib_path();
    assert!(c_path.exists(), "C .so not found at {}", c_path.display());
    assert!(
        rust_path.exists(),
        "Rust .so not found at {}",
        rust_path.display()
    );

    let cases: &[(i32, i32)] = &[
        (0, 0),
        (0, 1),
        (1, 0),
        (1, 1),
        (-1, -1),
        (123, 456),
        (-123, 456),
        (123, -456),
        (-123, -456),
        (i32::MAX, 0),
        (0, i32::MAX),
        (i32::MIN, 0),
        (0, i32::MIN),
        (i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX),
        (0xDEADBEEFu32 as i32, 0x12345678),
        (0x12345678, 0xDEADBEEFu32 as i32),
        (42, -42),
        (1024, 2048),
        (-2147483648, 1),
    ];

    for &(x, y) in cases {
        let c_out = run_driver(&c_path, x, y);
        let rust_out = run_driver(&rust_path, x, y);
        assert_eq!(
            c_out,
            rust_out,
            "mismatch for driver({x}, {y}): C={:?} Rust={:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out)
        );
    }
}

#[test]
fn driver_matches_c_random_sample() {
    let _guard = STDOUT_LOCK.lock().unwrap();
    let c_path = c_lib_path();
    let rust_path = rust_lib_path();
    assert!(c_path.exists());
    assert!(rust_path.exists());

    // Deterministic LCG to avoid bringing in a rand crate.
    let mut state: u64 = 0xC0FFEE_u64.wrapping_mul(0x9E3779B97F4A7C15);
    for _ in 0..200 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let x = (state >> 32) as i32;
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let y = (state >> 32) as i32;

        let c_out = run_driver(&c_path, x, y);
        let rust_out = run_driver(&rust_path, x, y);
        assert_eq!(
            c_out, rust_out,
            "mismatch for driver({x}, {y})"
        );
    }
}
