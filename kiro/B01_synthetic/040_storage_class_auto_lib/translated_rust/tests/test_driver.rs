use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;

fn c_lib_path() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/c_src/build/libdriver.so", manifest)
}

fn capture_c_driver(x: c_int) -> String {
    use std::os::unix::io::FromRawFd;
    unsafe {
        let mut fds: [c_int; 2] = [0; 2];
        libc::pipe(fds.as_mut_ptr());
        let old = libc::dup(1);
        libc::dup2(fds[1], 1);

        let lib = Library::new(c_lib_path()).unwrap();
        let func: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"driver").unwrap();
        func(x);
        libc::fflush(std::ptr::null_mut());

        libc::dup2(old, 1);
        libc::close(fds[1]);
        libc::close(old);

        let mut f = std::fs::File::from_raw_fd(fds[0]);
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        s
    }
}

fn capture_rust_driver(x: c_int) -> String {
    use std::os::unix::io::FromRawFd;
    unsafe {
        let mut fds: [c_int; 2] = [0; 2];
        libc::pipe(fds.as_mut_ptr());
        let old = libc::dup(1);
        libc::dup2(fds[1], 1);

        driver::driver(x);
        {
            use std::io::Write;
            std::io::stdout().flush().unwrap();
        }

        libc::dup2(old, 1);
        libc::close(fds[1]);
        libc::close(old);

        let mut f = std::fs::File::from_raw_fd(fds[0]);
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        s
    }
}

#[test]
fn test_driver_matches() {
    for x in [0i32, 1, -1, 100, -100, 1000000] {
        let c_out = capture_c_driver(x);
        let r_out = capture_rust_driver(x);
        assert_eq!(c_out, r_out, "Mismatch for x={}: C={:?} Rust={:?}", x, c_out, r_out);
    }
}
