use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CString};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Find the .so in target/debug or target/release
    for sub in &["debug", "release"] {
        let candidate = p.join(sub).join("libdriver.so");
        if candidate.exists() {
            return candidate;
        }
    }
    p.join("debug").join("libdriver.so")
}

#[test]
fn test_foo_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type FooFn = unsafe extern "C" fn(*const c_char, c_char) -> c_int;
    let c_foo: Symbol<FooFn> = unsafe { c_lib.get(b"foo").expect("C foo") };
    let r_foo: Symbol<FooFn> = unsafe { rust_lib.get(b"foo").expect("Rust foo") };

    let cases: &[(&str, u8)] = &[
        ("", b'A'),
        ("A", b'A'),
        ("AAA", b'A'),
        ("hello", b'A'),
        ("AxAx", b'A'),
        ("AxAx", b'x'),
        ("xxxxx", b'x'),
        ("Hello World", b'o'),
        ("abcdef", b'z'),
    ];

    for &(s, c) in cases {
        let cs = CString::new(s).unwrap();
        let c_result = unsafe { c_foo(cs.as_ptr(), c as c_char) };
        let r_result = unsafe { r_foo(cs.as_ptr(), c as c_char) };
        assert_eq!(
            c_result, r_result,
            "foo({:?}, {:?}): C={} Rust={}",
            s, c as char, c_result, r_result
        );
    }
}

#[test]
fn test_driver_matches() {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    fn capture_driver_output(lib_path: &std::path::Path, input: &str) -> String {
        let cs = CString::new(input).unwrap();
        let lib = unsafe { Library::new(lib_path).expect("load lib") };
        type DriverFn = unsafe extern "C" fn(*const c_char);
        let driver_fn: Symbol<DriverFn> = unsafe { lib.get(b"driver").expect("driver") };

        // Create a pipe, redirect stdout to it, call driver, restore stdout, read pipe
        let mut fds = [0i32; 2];
        unsafe {
            libc::pipe(fds.as_mut_ptr());
            let saved = libc::dup(1);
            libc::dup2(fds[1], 1);
            driver_fn(cs.as_ptr());
            libc::fflush(std::ptr::null_mut());
            libc::dup2(saved, 1);
            libc::close(saved);
            libc::close(fds[1]);
            let mut f = std::fs::File::from_raw_fd(fds[0]);
            let mut buf = String::new();
            f.read_to_string(&mut buf).unwrap();
            buf
        }
    }

    let inputs = &["", "A", "AxAx", "hello world", "AAAxxxAAA"];
    for &input in inputs {
        let c_out = capture_driver_output(&c_lib_path(), input);
        let r_out = capture_driver_output(&rust_lib_path(), input);
        assert_eq!(
            c_out, r_out,
            "driver({:?}): C={:?} Rust={:?}",
            input, c_out, r_out
        );
    }
}
