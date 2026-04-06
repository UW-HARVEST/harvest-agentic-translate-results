use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    let mut fds = [0i32; 2];
    unsafe { libc::pipe(fds.as_mut_ptr()); }
    let old_stdout = unsafe { libc::dup(1) };
    unsafe { libc::dup2(fds[1], 1); }
    f();
    unsafe {
        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::close(fds[1]);
    }
    let mut buf = String::new();
    let mut file = unsafe { std::fs::File::from_raw_fd(fds[0]) };
    file.read_to_string(&mut buf).ok();
    buf
}

fn find_rust_lib() -> std::path::PathBuf {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Look in target/debug for the cdylib
    let target_dir = manifest.join("target/debug");
    for name in &["libdriver.so", "libdriver.dylib"] {
        let p = target_dir.join(name);
        if p.exists() { return p; }
    }
    panic!("Could not find Rust .so in {:?}", target_dir);
}

#[test]
fn test_driver() {
    let c_lib_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");
    let c_lib = unsafe { Library::new(&c_lib_path).expect("Failed to load C lib") };
    let c_driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c_lib.get(b"driver").expect("Failed to find C driver") };

    let rust_lib_path = find_rust_lib();
    let rust_lib = unsafe { Library::new(&rust_lib_path).expect("Failed to load Rust lib") };
    let rust_driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { rust_lib.get(b"driver").expect("Failed to find Rust driver") };

    for &x in &[0i32, 1, -1, 0x12345678, i32::MAX, i32::MIN, 42, 255, 256] {
        let c_out = capture_stdout(|| unsafe { c_driver(x) });
        let rust_out = capture_stdout(|| unsafe { rust_driver(x) });
        assert_eq!(c_out, rust_out, "Mismatch for driver({}): C={:?} Rust={:?}", x, c_out, rust_out);
    }
}
