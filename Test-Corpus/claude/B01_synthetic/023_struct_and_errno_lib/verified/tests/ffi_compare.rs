// Integration test that compares the C .so's behavior against the Rust .so's
// behavior by loading both via libloading and capturing the stdout produced
// by each exported function.

use libloading::{Library, Symbol};
use std::ffi::CString;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::raw::{c_char, c_double, c_int};
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct HouseT {
    floors: c_int,
    bedrooms: c_int,
    bathrooms: c_double,
}

fn c_so_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src/build/libdriver.so");
    p
}

fn rust_so_path() -> PathBuf {
    // Try to find the compiled cdylib built by `cargo test` (which builds both
    // the dev binary and the cdylib for our crate).
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libdriver.so");
    p
}

/// Capture everything written to fd 1 (stdout) by a closure. Uses a temp file
/// so we can capture from C-level printf (which writes via libc's stdout
/// FILE *), not Rust's println!. We fflush(NULL) afterwards to make sure the
/// C library's output is committed to the underlying fd before we read it.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        // Make sure any pending output on the real stdout is flushed before
        // we redirect.
        libc::fflush(std::ptr::null_mut());

        let tmp = tempfile();
        let tmp_fd = tmp.as_raw_fd();

        // Save the original stdout fd.
        let saved = libc::dup(1);
        assert!(saved >= 0, "dup failed");

        // Redirect fd 1 -> tmp file.
        let r = libc::dup2(tmp_fd, 1);
        assert!(r >= 0, "dup2 failed");

        f();

        // Flush all libc FILE* streams; this is important because printf
        // writes through the libc stdio buffer, which might not yet have
        // hit the underlying fd.
        libc::fflush(std::ptr::null_mut());

        // Restore stdout.
        libc::dup2(saved, 1);
        libc::close(saved);

        // Now read everything that was written to the tmp file.
        let mut tmp = tmp;
        tmp.seek(SeekFrom::Start(0)).expect("seek failed");
        let mut buf = Vec::new();
        tmp.read_to_end(&mut buf).expect("read failed");
        buf
    }
}

fn tempfile() -> File {
    // Create a unique temp file using mkstemp.
    let template = CString::new("/tmp/harvest_capture_XXXXXX").unwrap();
    // mkstemp modifies the template buffer in place.
    let mut bytes = template.into_bytes_with_nul();
    let fd = unsafe { libc::mkstemp(bytes.as_mut_ptr() as *mut c_char) };
    assert!(fd >= 0, "mkstemp failed");
    // Unlink immediately so it gets cleaned up on close.
    let path = unsafe { CString::from_vec_with_nul_unchecked(bytes) };
    unsafe {
        libc::unlink(path.as_ptr());
    }
    use std::os::unix::io::FromRawFd;
    unsafe { File::from_raw_fd(fd) }
}

fn assert_libs_present() {
    assert!(
        c_so_path().exists(),
        "C library not built at {:?}; build with cmake first",
        c_so_path()
    );
    assert!(
        rust_so_path().exists(),
        "Rust library not built at {:?}; cargo build first",
        rust_so_path()
    );
}

#[test]
fn run_matches_c() {
    assert_libs_present();

    let c_lib = unsafe { Library::new(c_so_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_so_path()).expect("load Rust lib") };

    type RunFn = unsafe extern "C" fn(*mut HouseT, c_int);
    let c_run: Symbol<RunFn> = unsafe { c_lib.get(b"run").expect("c run") };
    let r_run: Symbol<RunFn> = unsafe { r_lib.get(b"run").expect("rust run") };

    // Test a variety of inputs that exercise the print-format paths.
    let cases: Vec<(HouseT, c_int)> = vec![
        (
            HouseT {
                floors: 2,
                bedrooms: 5,
                bathrooms: 2.5,
            },
            3,
        ),
        (
            HouseT {
                floors: 0,
                bedrooms: 0,
                bathrooms: 0.0,
            },
            0,
        ),
        (
            HouseT {
                floors: -1,
                bedrooms: -2,
                bathrooms: -1.5,
            },
            10,
        ),
        (
            HouseT {
                floors: 100,
                bedrooms: 7,
                bathrooms: 3.75,
            },
            -5,
        ),
        (
            HouseT {
                floors: 1,
                bedrooms: 1,
                bathrooms: 1.04,
            },
            1,
        ),
        (
            HouseT {
                floors: 1,
                bedrooms: 1,
                bathrooms: 1.05,
            },
            1,
        ),
        (
            HouseT {
                floors: 1,
                bedrooms: 1,
                bathrooms: 1.95,
            },
            1,
        ),
    ];

    for (init, extra) in cases {
        let mut h_c = init;
        let mut h_r = init;

        let c_out = capture_stdout(|| unsafe { c_run(&mut h_c as *mut HouseT, extra) });
        let r_out = capture_stdout(|| unsafe { r_run(&mut h_r as *mut HouseT, extra) });

        assert_eq!(
            c_out,
            r_out,
            "stdout mismatch for input {:?} extra={}\nC:\n{}\nRust:\n{}",
            init,
            extra,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        assert_eq!(
            h_c, h_r,
            "house struct mismatch after run for {:?} extra={}",
            init, extra
        );
    }
}

#[test]
fn driver_matches_c() {
    assert_libs_present();

    let c_lib = unsafe { Library::new(c_so_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_so_path()).expect("load Rust lib") };

    type DriverFn = unsafe extern "C" fn(*const c_char);
    let c_driver: Symbol<DriverFn> = unsafe { c_lib.get(b"driver").expect("c driver") };
    let r_driver: Symbol<DriverFn> = unsafe { r_lib.get(b"driver").expect("rust driver") };

    let inputs = vec![
        "0",
        "1",
        "-1",
        "42",
        "  5",          // leading whitespace allowed by strtol
        "5abc",         // partial parse: strtol consumes "5", endp != str
        "abc",          // invalid: endp == str
        "",             // invalid: empty input -> endp == str
        "2147483647",   // INT_MAX
        "-2147483648",  // INT_MIN
        "2147483648",   // > INT_MAX, should fail predicate
        "-2147483649",  // < INT_MIN, should fail predicate
        "9999999999999999999",  // overflow, errno=ERANGE
        "-9999999999999999999", // underflow
        "+10",
        "0x10",         // strtol with base 10 stops at 'x' -> parses "0"
        "  ",           // only whitespace -> fails
        "100",
    ];

    for input in inputs {
        let cs = CString::new(input).unwrap();

        let c_out = capture_stdout(|| unsafe { c_driver(cs.as_ptr()) });
        let r_out = capture_stdout(|| unsafe { r_driver(cs.as_ptr()) });

        assert_eq!(
            c_out,
            r_out,
            "stdout mismatch for driver input {:?}\nC:\n{}\nRust:\n{}",
            input,
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}
