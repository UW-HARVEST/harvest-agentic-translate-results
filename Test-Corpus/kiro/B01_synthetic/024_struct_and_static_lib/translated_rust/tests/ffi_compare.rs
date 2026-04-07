use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::io::Read;
use std::os::unix::io::FromRawFd;

fn capture_call(lib: &Library, func: &str, arg: c_int) -> String {
    unsafe {
        let mut fds = [0 as libc::c_int; 2];
        libc::pipe(fds.as_mut_ptr());
        let saved = libc::dup(1);
        libc::dup2(fds[1], 1);

        let f: Symbol<unsafe extern "C" fn(c_int)> = lib.get(func.as_bytes()).unwrap();
        f(arg);

        // Flush both C stdio and any remaining buffers
        libc::fflush(std::ptr::null_mut());
        // Restore stdout before reading
        libc::dup2(saved, 1);
        libc::close(saved);
        // Now close the write end so read_to_string sees EOF
        libc::close(fds[1]);

        let mut pipe_read = std::fs::File::from_raw_fd(fds[0]);
        let mut buf = String::new();
        pipe_read.read_to_string(&mut buf).unwrap();
        buf
    }
}

fn load_fresh(src: &str) -> Library {
    use std::sync::atomic::{AtomicU64, Ordering};
    static CTR: AtomicU64 = AtomicU64::new(0);
    let n = CTR.fetch_add(1, Ordering::Relaxed);
    let tmp = format!("/tmp/_ffi_test_{}_{}.so", std::process::id(), n);
    std::fs::copy(src, &tmp).unwrap();
    let lib = unsafe { Library::new(&tmp).unwrap() };
    let _ = std::fs::remove_file(&tmp);
    lib
}

fn c_so() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so")
}

fn rust_so() -> String {
    format!("{}/target/debug/libdriver.so", env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn test_run() {
    for x in [0, 1, 3, -1, 100] {
        let c = load_fresh(c_so());
        let r = load_fresh(&rust_so());
        let c_out = capture_call(&c, "run", x);
        let r_out = capture_call(&r, "run", x);
        assert_eq!(c_out, r_out, "run({x}) mismatch");
    }
}

#[test]
fn test_driver() {
    for x in [0, 1, 3, -1, 100] {
        let c = load_fresh(c_so());
        let r = load_fresh(&rust_so());
        let c_out = capture_call(&c, "driver", x);
        let r_out = capture_call(&r, "driver", x);
        assert_eq!(c_out, r_out, "driver({x}) mismatch");
    }
}
