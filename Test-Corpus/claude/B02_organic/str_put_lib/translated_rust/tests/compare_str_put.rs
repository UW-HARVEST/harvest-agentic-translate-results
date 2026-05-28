//! Integration test: compare C and Rust .so outputs of `str_put` byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::fs;
use std::io::Read;
use std::os::unix::io::{AsRawFd, RawFd};

const C_LIB: &str = "c_src/build/libtranslated_rust.so";
const RUST_LIB: &str = "target/release/libstr_put_lib.so";

/// Redirect libc stdout to a temp file, run `f`, then restore stdout and
/// return the captured bytes.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        // Flush any pending stdout from prior printf calls.
        libc::fflush(std::ptr::null_mut());

        let saved_fd: RawFd = libc::dup(1);
        assert!(saved_fd >= 0, "dup(1) failed");

        // Create a temp file for capture.
        let tmp_path = std::env::temp_dir().join(format!(
            "str_put_capture_{}_{}.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let tmp = fs::File::create(&tmp_path).expect("create tmp");
        let tmp_fd = tmp.as_raw_fd();

        // Redirect stdout (fd 1) to the temp file.
        let r = libc::dup2(tmp_fd, 1);
        assert!(r >= 0, "dup2 failed");
        drop(tmp);

        f();

        // Flush libc stdout buffer before restoring.
        libc::fflush(std::ptr::null_mut());

        // Restore stdout.
        libc::dup2(saved_fd, 1);
        libc::close(saved_fd);

        let mut out = Vec::new();
        let mut file = fs::File::open(&tmp_path).expect("open tmp for read");
        file.read_to_end(&mut out).expect("read tmp");
        let _ = fs::remove_file(&tmp_path);
        out
    }
}

fn run_str_put(lib_path: &str, num: c_int) -> Vec<u8> {
    unsafe {
        let lib = Library::new(lib_path).expect("load lib");
        let func: Symbol<unsafe extern "C" fn(c_int)> =
            lib.get(b"str_put").expect("find str_put");
        capture_stdout(|| {
            func(num);
        })
    }
}

#[test]
fn str_put_matches_for_various_nums() {
    // Simple smoke values plus a few interesting ones.
    // Avoid extremely large/negative values: C's str_put loops up to `num`
    // allocating strings, so very large values would OOM.
    let nums: &[c_int] = &[0, 1, 2, 5, 10, 42, 100, 999, -1, -42];
    for &n in nums {
        let c_out = run_str_put(C_LIB, n);
        let r_out = run_str_put(RUST_LIB, n);
        assert_eq!(
            c_out, r_out,
            "mismatch for num={}: C={:?} Rust={:?}",
            n,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
