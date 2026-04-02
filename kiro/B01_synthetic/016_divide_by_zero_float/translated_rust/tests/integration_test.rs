use libloading::{Library, Symbol};
use std::ffi::CString;
use std::io::Read;
use std::os::raw::{c_char, c_int};
use std::os::unix::io::FromRawFd;
use std::process::Command;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
const RS_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libdriver.so");

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    let mut fds = [0i32; 2];
    unsafe { libc::pipe(fds.as_mut_ptr()) };
    let old = unsafe { libc::dup(1) };
    unsafe { libc::dup2(fds[1], 1) };
    unsafe { libc::close(fds[1]) };
    f();
    unsafe {
        libc::fflush(std::ptr::null_mut());
        libc::dup2(old, 1);
        libc::close(old);
    }
    let mut buf = Vec::new();
    unsafe { std::fs::File::from_raw_fd(fds[0]) }
        .read_to_end(&mut buf)
        .unwrap();
    String::from_utf8(buf).unwrap()
}

fn build_rust_lib() {
    let status = Command::new("cargo")
        .args(["build", "--lib"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("cargo build --lib");
    assert!(status.success(), "Rust lib build failed");
}

#[test]
fn test_print_int_line() {
    build_rust_lib();
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rs_lib = unsafe { Library::new(RS_LIB).expect("load Rust lib") };
    let c_fn: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c_lib.get(b"printIntLine").unwrap() };
    let rs_fn: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { rs_lib.get(b"printIntLine").unwrap() };

    for &val in &[0, 1, -1, 42, 50, 100, i32::MAX, i32::MIN] {
        let c_out = capture_stdout(|| unsafe { c_fn(val) });
        let rs_out = capture_stdout(|| unsafe { rs_fn(val) });
        assert_eq!(c_out, rs_out, "printIntLine({}) mismatch", val);
    }
}

#[test]
fn test_print_line() {
    build_rust_lib();
    let c_lib = unsafe { Library::new(C_LIB).expect("load C lib") };
    let rs_lib = unsafe { Library::new(RS_LIB).expect("load Rust lib") };
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { c_lib.get(b"printLine").unwrap() };
    let rs_fn: Symbol<unsafe extern "C" fn(*const c_char)> =
        unsafe { rs_lib.get(b"printLine").unwrap() };

    for s in &[
        "hello",
        "Calling good()...",
        "Finished good()",
        "",
        "fgets() failed.",
        "This would result in a divide by zero",
    ] {
        let cs = CString::new(*s).unwrap();
        let c_out = capture_stdout(|| unsafe { c_fn(cs.as_ptr()) });
        let rs_out = capture_stdout(|| unsafe { rs_fn(cs.as_ptr()) });
        assert_eq!(c_out, rs_out, "printLine({:?}) mismatch", s);
    }
    // NULL
    let c_out = capture_stdout(|| unsafe { c_fn(std::ptr::null()) });
    let rs_out = capture_stdout(|| unsafe { rs_fn(std::ptr::null()) });
    assert_eq!(c_out, rs_out, "printLine(NULL) mismatch");
}

#[test]
fn test_binary_output() {
    let dir = env!("CARGO_MANIFEST_DIR");
    let c_bin = format!("{}/c_src/build/driver", dir);

    let status = Command::new("cargo")
        .args(["build", "--bin", "driver", "--features", "_bin"])
        .current_dir(dir)
        .status()
        .expect("cargo build");
    assert!(status.success());

    let rs_bin = format!("{}/target/debug/driver", dir);

    for input in &[
        "5.0\n5.0\n",
        "2.0\n2.0\n",
        "0.0\n0.0\n",
        "100.0\n100.0\n",
        "-3.5\n-3.5\n",
    ] {
        let run = |bin: &str| -> String {
            use std::io::Write;
            let mut child = Command::new(bin)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.as_mut().unwrap().write_all(input.as_bytes()).unwrap();
            drop(child.stdin.take());
            let out = child.wait_with_output().unwrap();
            String::from_utf8_lossy(&out.stdout).into_owned()
        };
        let c_out = run(&c_bin);
        let rs_out = run(&rs_bin);
        assert_eq!(
            c_out, rs_out,
            "Binary mismatch for input {:?}\nC:\n{}\nRust:\n{}",
            input, c_out, rs_out
        );
    }
}
