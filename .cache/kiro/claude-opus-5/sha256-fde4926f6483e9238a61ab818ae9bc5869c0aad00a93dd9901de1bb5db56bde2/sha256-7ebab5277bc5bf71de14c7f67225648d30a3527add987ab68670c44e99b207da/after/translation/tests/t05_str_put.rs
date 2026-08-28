//! Top of the call hierarchy: the only symbol declared in `include/lib.h`.
//!
//! `str_put` writes to stdout with `printf`, so the comparison captures fd 1
//! around each call. Both libraries share the process' libc stdout, hence the
//! redirection works for either side.

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

extern "C" {
    fn dup(fd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    fn open(path: *const c_char, flags: c_int, mode: c_int) -> c_int;
    fn fflush(stream: *mut c_void) -> c_int;
}

const O_WRONLY: c_int = 1;
const O_CREAT: c_int = 64;
const O_TRUNC: c_int = 512;

/// Runs `f` with fd 1 redirected into a temporary file and returns the bytes
/// that were written.
fn capture_stdout(tag: &str, f: impl FnOnce()) -> Vec<u8> {
    let path = std::env::temp_dir().join(format!("str_put_capture_{}_{}", std::process::id(), tag));
    let cpath = cbuf(path.to_str().unwrap());
    unsafe {
        fflush(std::ptr::null_mut());
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        let fd = open(cpath.as_ptr(), O_WRONLY | O_CREAT | O_TRUNC, 0o644);
        assert!(fd >= 0, "open({}) failed", path.display());
        assert!(dup2(fd, 1) >= 0, "dup2 failed");

        f();

        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "dup2 restore failed");
        close(fd);
        close(saved);
    }
    let data = std::fs::read(&path).expect("read capture file");
    let _ = std::fs::remove_file(&path);
    data
}

#[test]
fn str_put_output_matches() {
    let p = load_pair();

    // Keep the two libraries' hash-seed streams aligned.
    unsafe {
        (p.c.rand_seed)(0x3141_5926);
        (p.r.rand_seed)(0x3141_5926);
    }

    let cases: Vec<c_int> = vec![
        0, 1, 2, 3, 4, 5, 7, 8, 9, 16, 17, 31, 32, 33, 63, 64, 65, 100, 127, 128, 255, 256, 511,
        512, 513, 1000, 1024, 2000, -1, -5, -1000, i32::MIN, i32::MAX.wrapping_neg(),
    ];

    for num in cases {
        let c_out = capture_stdout("c", || unsafe { (p.c.str_put)(num) });
        let r_out = capture_stdout("r", || unsafe { (p.r.str_put)(num) });
        assert_eq!(
            c_out,
            r_out,
            "str_put({num}) stdout differs\nC   = {:?}\nRust= {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }

    // Repeated invocations mutate the shared hash seed and the `strkey` static
    // buffer; the outputs must still agree call for call.
    unsafe {
        (p.c.rand_seed)(7);
        (p.r.rand_seed)(7);
    }
    for round in 0..25 {
        let num = (round * 37) % 300;
        let c_out = capture_stdout("c2", || unsafe { (p.c.str_put)(num) });
        let r_out = capture_stdout("r2", || unsafe { (p.r.str_put)(num) });
        assert_eq!(
            c_out,
            r_out,
            "str_put({num}) round {round} stdout differs\nC   = {:?}\nRust= {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
