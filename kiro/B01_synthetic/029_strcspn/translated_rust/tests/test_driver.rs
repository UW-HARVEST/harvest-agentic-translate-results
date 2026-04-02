use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::c_char;

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
    unsafe { Library::new(path).expect("Failed to load C libdriver.so") }
}

/// Test strcspn logic: C's driver prints strcspn result, so we parse it.
/// We call C driver redirecting stdout to a file, and compare with Rust strcspn directly.
#[test]
fn test_strcspn_matches() {
    let lib = c_lib();
    let c_driver: Symbol<unsafe extern "C" fn(*const c_char, *const c_char)> =
        unsafe { lib.get(b"driver").unwrap() };

    let cases: &[(&str, &str)] = &[
        ("hello", "world"),
        ("abcdef", "dc"),
        ("abcdef", "xyz"),
        ("", "abc"),
        ("abc", ""),
        ("", ""),
        ("aaaaaa", "a"),
        ("abcdef", "f"),
        ("abcdef", "a"),
        ("hello world", " "),
        ("test123", "0123456789"),
        ("abcabc", "c"),
        ("\t\n test", " "),
    ];

    for &(s1, s2) in cases {
        let cs1 = CString::new(s1).unwrap();
        let cs2 = CString::new(s2).unwrap();

        // Get C result by capturing printf output via pipe on fd level
        let c_result = capture_c_driver_output(&c_driver, cs1.as_ptr(), cs2.as_ptr());

        // Get Rust result
        let rust_result = driver::strcspn(s1.as_bytes(), s2.as_bytes());

        let c_val: usize = c_result.trim().parse().unwrap_or_else(|_| {
            panic!("Failed to parse C output {:?} for ({:?}, {:?})", c_result, s1, s2)
        });

        assert_eq!(
            c_val, rust_result,
            "strcspn({:?}, {:?}): C={} Rust={}",
            s1, s2, c_val, rust_result
        );
    }
}

fn capture_c_driver_output(
    c_driver: &libloading::Symbol<unsafe extern "C" fn(*const c_char, *const c_char)>,
    s1: *const c_char,
    s2: *const c_char,
) -> String {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        libc::fflush(std::ptr::null_mut());
    }

    let mut pipe_fds = [0i32; 2];
    unsafe { libc::pipe(pipe_fds.as_mut_ptr()) };
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(pipe_fds[1], 1) };

    unsafe { c_driver(s1, s2) };

    unsafe {
        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(pipe_fds[1]);
    }

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(pipe_fds[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}
