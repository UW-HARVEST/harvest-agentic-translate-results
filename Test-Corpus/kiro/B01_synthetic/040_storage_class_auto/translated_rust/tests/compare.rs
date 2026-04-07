use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

extern "C" {
    static stdout: *mut libc::FILE;
}

fn capture_stdout<F: FnOnce()>(f: F) -> String {
    unsafe {
        // flush both C and Rust stdout before redirect
        libc::fflush(stdout);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        let mut pipe_fds = [0i32; 2];
        assert_eq!(libc::pipe(pipe_fds.as_mut_ptr()), 0);
        let saved = libc::dup(1);
        libc::dup2(pipe_fds[1], 1);
        libc::close(pipe_fds[1]);

        f();

        // flush after call
        libc::fflush(stdout);
        let _ = std::io::Write::flush(&mut std::io::stdout());

        libc::dup2(saved, 1);
        libc::close(saved);

        let mut reader = std::fs::File::from_raw_fd(pipe_fds[0]);
        libc::fcntl(pipe_fds[0], libc::F_SETFL, libc::O_NONBLOCK);
        let mut buf = String::new();
        let _ = reader.read_to_string(&mut buf);
        buf
    }
}

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver_c.so");
    unsafe { Library::new(path).expect("failed to load C .so") }
}

fn rust_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libtranslated_rust.so");
    unsafe { Library::new(path).expect("failed to load Rust .so") }
}

#[test]
fn test_driver_outputs_match() {
    let c = c_lib();
    let r = rust_lib();

    let test_inputs: &[c_int] = &[0, 1, -1, 100, -100, i32::MAX, i32::MIN, 42, 999999];

    for &x in test_inputs {
        let c_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> = unsafe { c.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(x) })
        };
        let r_out = {
            let f: Symbol<unsafe extern "C" fn(c_int)> = unsafe { r.get(b"driver").unwrap() };
            capture_stdout(|| unsafe { f(x) })
        };
        assert_eq!(c_out, r_out, "mismatch for driver({}): C={:?} Rust={:?}", x, c_out, r_out);
    }
}
