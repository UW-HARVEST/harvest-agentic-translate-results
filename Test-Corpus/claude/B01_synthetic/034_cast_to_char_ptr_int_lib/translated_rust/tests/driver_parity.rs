// Parity tests: compare Rust .so output to C .so output for `driver`.
//
// All tests are merged into one #[test] function because each call to
// `driver` writes to fd 1, and we capture stdout by redirecting fd 1 to a
// temp file. If multiple tests ran in parallel, the libtest framework's own
// "test X ... ok" output (also on fd 1) could leak into our captures.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

type DriverFn = unsafe extern "C" fn(c_int);

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    manifest_dir().join("c_src").join("build").join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = manifest_dir();
    p.push("target");
    p.push("debug");
    p.push("libdriver.so");
    p
}

/// Capture writes to stdout file descriptor 1 produced while running `f`.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    let tmp_path = std::env::temp_dir().join(format!(
        "driver_parity_{}_{:?}.out",
        std::process::id(),
        std::thread::current().id()
    ));
    let tmp = File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp_path)
        .expect("open tmp file");

    let saved_fd = unsafe { libc::dup(1) };
    assert!(saved_fd >= 0, "dup failed");

    let new_fd = tmp.as_raw_fd();
    let r = unsafe { libc::dup2(new_fd, 1) };
    assert!(r >= 0, "dup2 failed");

    f();

    let _ = std::io::stdout().flush();
    unsafe { libc::fflush(std::ptr::null_mut()) };

    unsafe {
        libc::dup2(saved_fd, 1);
        libc::close(saved_fd);
    }

    let mut tmp = tmp;
    tmp.seek(SeekFrom::Start(0)).expect("seek");
    let mut buf = Vec::new();
    tmp.read_to_end(&mut buf).expect("read");

    drop(tmp);
    let _ = std::fs::remove_file(&tmp_path);

    buf
}

fn run_driver(lib: &Library, x: c_int) -> Vec<u8> {
    let driver: Symbol<DriverFn> = unsafe { lib.get(b"driver") }.expect("driver symbol");
    capture_stdout(|| unsafe { driver(x) })
}

#[test]
fn driver_parity_all() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C .so");
    let rust_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust .so");

    // Verify both expose the `driver` symbol.
    let _: Symbol<DriverFn> = unsafe { c_lib.get(b"driver") }.expect("C exports driver");
    let _: Symbol<DriverFn> = unsafe { rust_lib.get(b"driver") }.expect("Rust exports driver");

    let mut cases: Vec<c_int> = vec![
        0,
        1,
        -1,
        2,
        -2,
        42,
        -42,
        0x1234,
        i32::MAX,
        i32::MIN,
        0x7fffffff,
        -0x7fffffff_i32 - 1, // INT_MIN
        0x12345678,
        -0x12345678,
        0x000000ff,
        0x0000ff00,
        0x00ff0000,
        -1_i32 << 24,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    for x in -200..200 {
        cases.push(x as c_int);
    }
    // A few large strides too
    for k in 0..32 {
        cases.push(1_i32 << k.min(30));
        cases.push(-(1_i32 << k.min(30)));
    }

    for x in cases {
        let c_out = run_driver(&c_lib, x);
        let rust_out = run_driver(&rust_lib, x);
        assert_eq!(
            c_out, rust_out,
            "Mismatch for x={}: C={:?} Rust={:?}",
            x,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out)
        );

        // Sanity: output should be 8 lowercase hex digits + newline (for 4-byte int).
        assert_eq!(c_out.len(), 9, "unexpected length for x={}: {:?}", x, c_out);
        assert_eq!(c_out[8], b'\n');
    }
}
