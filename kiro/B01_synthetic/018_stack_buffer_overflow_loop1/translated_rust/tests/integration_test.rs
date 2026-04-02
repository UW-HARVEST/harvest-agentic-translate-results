use libloading::{Library, Symbol};
use std::ffi::CString;
use std::os::raw::{c_char, c_int};
use std::process::Command;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

fn rust_lib_path() -> String {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    format!("{}/target/debug/libdriver.so", manifest_dir)
}

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    use std::io::{Read, Write};
    use std::os::unix::io::FromRawFd;

    unsafe {
        std::io::stdout().flush().ok();
        libc::fflush(std::ptr::null_mut());

        let mut pipes = [0i32; 2];
        assert_eq!(libc::pipe(pipes.as_mut_ptr()), 0);

        let old_stdout = libc::dup(1);
        libc::dup2(pipes[1], 1);
        libc::close(pipes[1]);

        f();

        std::io::stdout().flush().ok();
        libc::fflush(std::ptr::null_mut());

        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        let mut reader = std::fs::File::from_raw_fd(pipes[0]);
        let mut buf = String::new();
        reader.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn build_rust_lib() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let status = Command::new("timeout")
        .args(&["120", "cargo", "build", "--lib"])
        .current_dir(manifest_dir)
        .status()
        .expect("Failed to build Rust lib");
    assert!(status.success(), "Rust lib build failed");
}

#[test]
fn test_print_int_line() {
    build_rust_lib();
    let c_lib = unsafe { Library::new(C_LIB).expect("Failed to load C library") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust library") };

    let c_fn: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c_lib.get(b"printIntLine").unwrap() };
    let rust_fn: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { rust_lib.get(b"printIntLine").unwrap() };

    for val in &[0i32, 1, -1, 42, i32::MAX, i32::MIN] {
        let c_out = capture_stdout(|| unsafe { c_fn(*val) });
        let rust_out = capture_stdout(|| unsafe { rust_fn(*val) });
        assert_eq!(c_out, rust_out, "printIntLine mismatch for {}", val);
    }
}

#[test]
fn test_print_line() {
    build_rust_lib();
    let c_lib = unsafe { Library::new(C_LIB).expect("Failed to load C library") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust library") };

    let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { c_lib.get(b"printLine").unwrap() };
    let rust_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { rust_lib.get(b"printLine").unwrap() };

    for s in &["hello", "", "test 123"] {
        let cs = CString::new(*s).unwrap();
        let c_out = capture_stdout(|| unsafe { c_fn(cs.as_ptr()) });
        let rust_out = capture_stdout(|| unsafe { rust_fn(cs.as_ptr()) });
        assert_eq!(c_out, rust_out, "printLine mismatch for {:?}", s);
    }

    let c_out = capture_stdout(|| unsafe { c_fn(std::ptr::null()) });
    let rust_out = capture_stdout(|| unsafe { rust_fn(std::ptr::null()) });
    assert_eq!(c_out, rust_out, "printLine mismatch for NULL");
}

#[test]
fn test_good() {
    build_rust_lib();
    let c_lib = unsafe { Library::new(C_LIB).expect("Failed to load C library") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust library") };

    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c_lib.get(b"good").unwrap() };
    let rust_fn: Symbol<unsafe extern "C" fn()> = unsafe { rust_lib.get(b"good").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let rust_out = capture_stdout(|| unsafe { rust_fn() });
    assert_eq!(c_out, rust_out, "good() output mismatch");
}

#[test]
fn test_bad() {
    build_rust_lib();
    let c_lib = unsafe { Library::new(C_LIB).expect("Failed to load C library") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust library") };

    let c_fn: Symbol<unsafe extern "C" fn()> = unsafe { c_lib.get(b"bad").unwrap() };
    let rust_fn: Symbol<unsafe extern "C" fn()> = unsafe { rust_lib.get(b"bad").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let rust_out = capture_stdout(|| unsafe { rust_fn() });
    assert_eq!(c_out, rust_out, "bad() output mismatch");
}

#[test]
fn test_binary_output() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let c_bin = format!("{}/c_src/build/driver", manifest_dir);

    let status = Command::new("timeout")
        .args(&["120", "cargo", "build", "--bin", "driver"])
        .current_dir(manifest_dir)
        .status()
        .expect("Failed to build Rust binary");
    assert!(status.success(), "Rust binary build failed");

    let rust_bin = format!("{}/target/debug/driver", manifest_dir);

    for input in &["1\n", "0\n"] {
        let c_result = Command::new(&c_bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
                child.wait_with_output()
            })
            .expect("Failed to run C binary");

        let rust_result = Command::new(&rust_bin)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
                child.wait_with_output()
            })
            .expect("Failed to run Rust binary");

        assert_eq!(
            c_result.stdout, rust_result.stdout,
            "Binary stdout mismatch for input {:?}.\nC: {:?}\nRust: {:?}",
            input,
            String::from_utf8_lossy(&c_result.stdout),
            String::from_utf8_lossy(&rust_result.stdout)
        );
    }
}
