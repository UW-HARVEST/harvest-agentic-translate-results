use libloading::{Library, Symbol};
use std::ffi::c_double;

/// Capture stdout from a closure that writes via C printf/stdout.
fn capture_stdout(f: impl FnOnce()) -> String {
    unsafe {
        libc::fflush(libc::fdopen(1, b"w\0".as_ptr() as *const libc::c_char));

        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);

        let saved_stdout = libc::dup(1);
        assert!(saved_stdout >= 0);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        f();

        libc::fflush(libc::fdopen(1, b"w\0".as_ptr() as *const libc::c_char));
        libc::dup2(saved_stdout, 1);
        libc::close(saved_stdout);

        let mut buf = vec![0u8; 4096];
        let n = libc::read(pipe_fds[0], buf.as_mut_ptr() as *mut libc::c_void, buf.len());
        libc::close(pipe_fds[0]);

        buf.truncate(if n > 0 { n as usize } else { 0 });
        String::from_utf8_lossy(&buf).into_owned()
    }
}

fn c_lib() -> Library {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    unsafe { Library::new(&path).expect("Failed to load C libdriver.so") }
}

#[test]
fn test_driver_outputs_match() {
    let lib = c_lib();
    let c_driver: Symbol<unsafe extern "C" fn(c_double)> =
        unsafe { lib.get(b"driver").unwrap() };

    let test_values: &[f64] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        3.14,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        1e-300,
        1e300,
        0.1,
        2.5,
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::MIN,
    ];

    for &val in test_values {
        let c_out = capture_stdout(|| unsafe { c_driver(val) });
        let rust_out = capture_stdout(|| driver::driver(val));

        assert_eq!(
            c_out.as_bytes(),
            rust_out.as_bytes(),
            "Mismatch for input {val}: C={c_out:?} Rust={rust_out:?}"
        );
    }
}
