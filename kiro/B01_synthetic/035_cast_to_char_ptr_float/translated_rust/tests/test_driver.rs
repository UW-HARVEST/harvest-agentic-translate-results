use std::process::Command;

fn c_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libdriver.so", manifest)
}

fn rust_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/release/libdriver.so", manifest)
}

fn call_driver_from_lib(lib_path: &str, x: f32) -> String {
    unsafe {
        let lib = libloading::Library::new(lib_path).expect("load lib");
        let func: libloading::Symbol<unsafe extern "C" fn(f32)> =
            lib.get(b"driver").expect("find driver");
        capture_stdout(|| func(x))
    }
}

extern "C" {
    fn fflush(stream: *mut libc::c_void) -> libc::c_int;
}

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    unsafe {
        fflush(std::ptr::null_mut());

        let mut pipefd = [0i32; 2];
        assert_eq!(libc::pipe(pipefd.as_mut_ptr()), 0);

        let old_stdout = libc::dup(1);
        libc::dup2(pipefd[1], 1);

        f();

        fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(pipefd[1]);

        let mut result = String::new();
        let mut reader = std::fs::File::from_raw_fd(pipefd[0]);
        reader.read_to_string(&mut result).unwrap();
        result
    }
}

#[test]
fn test_driver_values() {
    // Build release first to get the Rust .so
    let manifest = env!("CARGO_MANIFEST_DIR");
    let status = Command::new("timeout")
        .args(["600", "cargo", "build", "--release"])
        .current_dir(manifest)
        .status()
        .expect("cargo build");
    assert!(status.success());

    let c_lib = c_lib_path();
    let rust_lib = rust_lib_path();

    let test_values: &[f32] = &[
        0.0, 1.0, -1.0, 3.14,
        f32::INFINITY, f32::NEG_INFINITY, f32::NAN,
        f32::MIN, f32::MAX, f32::MIN_POSITIVE,
        42.5, -0.0,
    ];

    for &val in test_values {
        let c_out = call_driver_from_lib(&c_lib, val);
        let rust_out = call_driver_from_lib(&rust_lib, val);
        assert_eq!(
            c_out, rust_out,
            "Mismatch for driver({val}): C={c_out:?} Rust={rust_out:?}"
        );
    }
}

#[test]
fn test_binary_output() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let c_bin = format!("{}/c_src/build/driver", manifest);

    let status = Command::new("timeout")
        .args(["600", "cargo", "build", "--release"])
        .current_dir(manifest)
        .status()
        .expect("cargo build");
    assert!(status.success());

    let rust_bin = format!("{}/target/release/driver", manifest);

    let test_inputs = &["0.0", "1.0", "-1.0", "3.14", "42.5"];
    for input in test_inputs {
        let c_output = Command::new(&c_bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).ok();
                child.wait_with_output()
            })
            .expect("run C binary");

        let rust_output = Command::new(&rust_bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).ok();
                child.wait_with_output()
            })
            .expect("run Rust binary");

        assert_eq!(
            c_output.stdout, rust_output.stdout,
            "Binary output mismatch for input {input}: C={:?} Rust={:?}",
            String::from_utf8_lossy(&c_output.stdout),
            String::from_utf8_lossy(&rust_output.stdout)
        );
    }
}
