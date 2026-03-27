use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    let mut fds = [0i32; 2];
    unsafe { libc::pipe(fds.as_mut_ptr()); }
    let (read_fd, write_fd) = (fds[0], fds[1]);
    let orig_stdout = unsafe { libc::dup(1) };
    unsafe {
        libc::dup2(write_fd, 1);
        libc::close(write_fd);
    }

    f();

    unsafe { libc::fflush(std::ptr::null_mut()); }
    use std::io::Write;
    let _ = std::io::stdout().flush();

    unsafe {
        libc::dup2(orig_stdout, 1);
        libc::close(orig_stdout);
    }

    let mut pipe_read = unsafe { std::fs::File::from_raw_fd(read_fd) };
    let mut buf = String::new();
    pipe_read.read_to_string(&mut buf).unwrap();
    buf
}

#[test]
fn test_driver_matches() {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_lib_path = manifest.join("c_src/build/libdriver.so");

    // Find the Rust .so in target/debug/deps or target/debug
    let target_dir = manifest.join("target/debug");
    let rust_lib_path = ["libdriver.so"]
        .iter()
        .map(|n| target_dir.join(n))
        .find(|p| p.exists())
        .expect("Rust .so not found - run cargo build first");

    let c_lib = unsafe { Library::new(&c_lib_path).expect("load C lib") };
    let rust_lib = unsafe { Library::new(&rust_lib_path).expect("load Rust lib") };

    let c_driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { c_lib.get(b"driver").unwrap() };
    let rust_driver: Symbol<unsafe extern "C" fn(c_int)> =
        unsafe { rust_lib.get(b"driver").unwrap() };

    for x in [0, 1, 5, -1, 100, i32::MAX, i32::MIN] {
        let c_out = capture_stdout(|| unsafe { c_driver(x) });
        let r_out = capture_stdout(|| unsafe { rust_driver(x) });
        assert_eq!(c_out, r_out, "Mismatch for x={x}:\n  C:    {c_out:?}\n  Rust: {r_out:?}");
    }
}
