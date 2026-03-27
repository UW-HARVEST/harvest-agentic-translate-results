use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    unsafe {
        libc::fflush(std::ptr::null_mut()); // flush all streams

        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let old_stdout = libc::dup(1);
        assert!(old_stdout >= 0);
        libc::dup2(fds[1], 1);
        libc::close(fds[1]);

        f();

        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        let mut pipe_read = std::fs::File::from_raw_fd(fds[0]);
        let mut buf = Vec::new();
        pipe_read.read_to_end(&mut buf).unwrap();
        buf
    }
}

#[test]
fn test_driver_good_path() {
    let c_lib_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so");

    // Call C driver(1) - the "good" path that prints "5\n"
    let c_output = unsafe {
        let lib = Library::new(&c_lib_path).expect("Failed to load C libdriver.so");
        let func: Symbol<unsafe extern "C" fn(c_int)> =
            lib.get(b"driver").expect("Failed to find driver symbol");
        capture_stdout(|| func(1))
    };

    // Call Rust driver(1)
    let rust_output = capture_stdout(|| driver::driver(1));

    assert_eq!(
        c_output, rust_output,
        "Mismatch!\nC output:    {:?}\nRust output: {:?}",
        String::from_utf8_lossy(&c_output),
        String::from_utf8_lossy(&rust_output)
    );
}
