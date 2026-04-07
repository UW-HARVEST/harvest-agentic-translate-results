use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::FromRawFd;

extern "C" {
    fn pipe(fds: *mut i32) -> i32;
    fn dup(fd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
    static stdout: *mut std::ffi::c_void;
}

fn capture_stdout(f: impl FnOnce()) -> String {
    unsafe { fflush(stdout); }

    let mut fds = [0i32; 2];
    unsafe { pipe(fds.as_mut_ptr()); }
    let old_stdout = unsafe { dup(1) };
    unsafe { dup2(fds[1], 1); }
    unsafe { close(fds[1]); }

    f();

    unsafe { fflush(stdout); }
    unsafe { dup2(old_stdout, 1); }
    unsafe { close(old_stdout); }

    let mut buf = String::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    reader.read_to_string(&mut buf).unwrap();
    buf
}

fn c_lib_path() -> String {
    std::env::var("CARGO_MANIFEST_DIR").unwrap() + "/c_lib/libdriver_c.so"
}

fn rust_lib_path() -> String {
    let dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let debug = format!("{}/target/debug/libdriver.so", dir);
    if std::path::Path::new(&debug).exists() { return debug; }
    format!("{}/target/release/libdriver.so", dir)
}

fn call_driver(lib: &Library, val: f64) -> String {
    capture_stdout(|| unsafe {
        let func: Symbol<unsafe extern "C" fn(f64)> = lib.get(b"driver").unwrap();
        func(val);
    })
}

#[test]
fn test_driver_outputs_match() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C .so") };
    let rust_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust .so") };

    let test_values: Vec<f64> = vec![
        0.0, -0.0, 1.0, -1.0, 0.5, 0.1, 0.3, 1.5, -2.5,
        100.0, 1e10, 1e-10, 1e100, 1e-100,
        f64::INFINITY, f64::NEG_INFINITY, f64::NAN,
        f64::MIN_POSITIVE, 5e-324, 2.2250738585072009e-308,
        f64::MAX, f64::MIN,
        std::f64::consts::PI, std::f64::consts::E,
        1.0 / 3.0, -1.0 / 3.0,
        0.15625, 3.14, -0.0001, 999999.9999,
    ];

    let mut failures = Vec::new();
    for &val in &test_values {
        let c_out = call_driver(&c_lib, val);
        let rust_out = call_driver(&rust_lib, val);
        if c_out != rust_out {
            failures.push(format!(
                "MISMATCH for {val:?} (bits={:#018x}):\n  C:    {}\n  Rust: {}",
                val.to_bits(), c_out.trim_end(), rust_out.trim_end()
            ));
        }
    }
    if !failures.is_empty() {
        panic!("Output mismatches:\n{}", failures.join("\n"));
    }
}
