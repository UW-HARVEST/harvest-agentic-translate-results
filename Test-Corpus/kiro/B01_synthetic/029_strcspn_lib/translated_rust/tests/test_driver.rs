use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;

/// Capture stdout from a closure that calls printf-based functions.
/// Uses pipe + dup2 to redirect file descriptor 1.
fn capture_stdout(f: impl FnOnce()) -> String {
    use std::io::Read;
    unsafe {
        // flush any pending stdout
        libc::fflush(libc::fdopen(1, b"w\0".as_ptr() as *const c_char));

        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let old_stdout = libc::dup(1);
        assert!(old_stdout >= 0);
        libc::dup2(fds[1], 1);
        libc::close(fds[1]);

        f();

        libc::fflush(libc::fdopen(1, b"w\0".as_ptr() as *const c_char));
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        let mut buf = String::new();
        let mut file = std::fs::File::from_raw_fd(fds[0]);
        file.read_to_string(&mut buf).unwrap();
        buf
    }
}

use std::os::unix::io::FromRawFd;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libdriver.so"
    );
    unsafe { Library::new(path).expect("Failed to load C libdriver.so") }
}

type DriverFn = unsafe extern "C" fn(*const c_char, *const c_char);

#[test]
fn test_driver_outputs_match() {
    let lib = c_lib();
    let c_driver: Symbol<DriverFn> = unsafe { lib.get(b"driver").unwrap() };

    let cases: Vec<(&str, &str)> = vec![
        ("hello", "aeiou"),
        ("", "abc"),
        ("abc", ""),
        ("abcdef", "dc"),
        ("xxxyz", "y"),
        ("test", "test"),
        ("abcdef", "zzzz"),
        ("", ""),
    ];

    for (s1, s2) in &cases {
        let cs1 = CString::new(*s1).unwrap();
        let cs2 = CString::new(*s2).unwrap();

        let c_out = capture_stdout(|| unsafe {
            c_driver(cs1.as_ptr(), cs2.as_ptr());
        });

        let rust_out = capture_stdout(|| {
            driver::driver(cs1.as_ptr(), cs2.as_ptr());
        });

        assert_eq!(
            c_out, rust_out,
            "Mismatch for driver({:?}, {:?}): C={:?} Rust={:?}",
            s1, s2, c_out, rust_out
        );
    }
}
