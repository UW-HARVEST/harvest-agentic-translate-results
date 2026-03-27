use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

extern "C" {
    fn pipe(pipefd: *mut i32) -> i32;
    fn dup(oldfd: i32) -> i32;
    fn dup2(oldfd: i32, newfd: i32) -> i32;
    fn close(fd: i32) -> i32;
    fn fflush(stream: *mut std::ffi::c_void) -> i32;
}

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    let mut fds = [0i32; 2];
    unsafe { pipe(fds.as_mut_ptr()) };
    let saved = unsafe { dup(1) };
    unsafe { dup2(fds[1], 1) };
    unsafe { close(fds[1]) };

    f();

    unsafe { fflush(std::ptr::null_mut()) };
    unsafe { dup2(saved, 1) };
    unsafe { close(saved) };

    let mut result = String::new();
    unsafe { std::fs::File::from_raw_fd(fds[0]) }
        .read_to_string(&mut result)
        .unwrap();
    result
}

#[test]
fn test_driver_matches_c() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_lib = unsafe {
        Library::new(manifest.join("c_src/build/libdriver.so")).expect("load C .so")
    };

    // Find the Rust .so in target/
    let rust_so = find_rust_so(&manifest);
    let rust_lib = unsafe { Library::new(&rust_so).expect("load Rust .so") };

    for floors in [0, 1, 2, 5, -1, 100] {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { c_lib.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(floors) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> =
                unsafe { rust_lib.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(floors) })
        };
        assert_eq!(c_out, r_out, "Mismatch for floors={floors}: C={c_out:?} Rust={r_out:?}");
    }
}

fn find_rust_so(manifest: &std::path::Path) -> std::path::PathBuf {
    // Look in target/debug or target/release
    for profile in &["debug", "release"] {
        let p = manifest.join(format!("target/{}/libdriver.so", profile));
        if p.exists() {
            return p;
        }
    }
    panic!("Could not find Rust libdriver.so in target/");
}
