use libloading::{Library, Symbol};
use std::fs;
use std::os::unix::io::IntoRawFd;

extern "C" {
    fn dup(fd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
}

fn capture_main(lib: &Library, tag: &str) -> (i32, Vec<u8>) {
    let tmp = format!("/tmp/driver_test_{}", tag);
    let f = fs::File::create(&tmp).unwrap();
    let tmp_fd = f.into_raw_fd();

    let old_stdout = unsafe { dup(1) };
    unsafe { dup2(tmp_fd, 1) };
    unsafe { close(tmp_fd) };

    let ret: i32 = unsafe {
        let func: Symbol<unsafe extern "C" fn() -> i32> = lib.get(b"main").unwrap();
        func()
    };

    unsafe { fflush(std::ptr::null_mut()) };
    unsafe { dup2(old_stdout, 1) };
    unsafe { close(old_stdout) };

    let buf = fs::read(&tmp).unwrap();
    let _ = fs::remove_file(&tmp);
    (ret, buf)
}

#[test]
fn test_main_output_matches() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let c_path = format!("{}/c_src/build/libdriver.so", manifest);
    let rust_path = format!("{}/target/debug/libdriver.so", manifest);

    let c_lib = unsafe { Library::new(&c_path) }.expect("Failed to load C .so");
    let rust_lib = unsafe { Library::new(&rust_path) }.expect("Failed to load Rust .so");

    let (c_ret, c_out) = capture_main(&c_lib, "c");
    let (r_ret, r_out) = capture_main(&rust_lib, "rust");

    assert_eq!(c_ret, r_ret, "Return values differ: C={}, Rust={}", c_ret, r_ret);
    assert_eq!(c_out, r_out,
        "Stdout differs:\n  C:    {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&r_out),
    );
}

#[test]
fn test_main_return_value() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let c_path = format!("{}/c_src/build/libdriver.so", manifest);
    let c_lib = unsafe { Library::new(&c_path) }.expect("Failed to load C .so");
    let (ret, _) = capture_main(&c_lib, "c_ret");
    assert_eq!(ret, 0, "C main() should return 0");
}
