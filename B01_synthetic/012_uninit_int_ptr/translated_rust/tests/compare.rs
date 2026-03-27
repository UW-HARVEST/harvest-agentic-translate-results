use libloading::{Library, Symbol};
use std::io::Read;
use std::os::unix::io::FromRawFd;
use std::sync::Mutex;

static STDOUT_LOCK: Mutex<()> = Mutex::new(());

/// Capture stdout (fd 1) output by redirecting to a temp file.
fn capture_stdout<F: FnOnce()>(f: F) -> Vec<u8> {
    let _guard = STDOUT_LOCK.lock().unwrap();
    unsafe {
        libc::fflush(std::ptr::null_mut());

        let name = b"/tmp/capture_XXXXXX\0";
        let mut buf = name.to_vec();
        let fd = libc::mkstemp(buf.as_mut_ptr() as *mut _);
        assert!(fd >= 0);
        libc::unlink(buf.as_ptr() as *const _);

        let old_stdout = libc::dup(1);
        assert!(old_stdout >= 0);
        libc::dup2(fd, 1);

        f();

        libc::fflush(std::ptr::null_mut());

        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);

        libc::lseek(fd, 0, libc::SEEK_SET);
        let mut file = std::fs::File::from_raw_fd(fd);
        let mut out = Vec::new();
        file.read_to_end(&mut out).unwrap();
        out
    }
}

fn c_lib_path() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

#[test]
fn test_print_int_ptr_line() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn(*const i32)> =
        unsafe { c_lib.get(b"printIntPtrLine").unwrap() };

    for val in &[0i32, 1, -1, 42, i32::MAX, i32::MIN] {
        let c_out = capture_stdout(|| unsafe { c_fn(val as *const i32) });
        let rust_out = capture_stdout(|| driver::printIntPtrLine(val as *const i32));
        assert_eq!(
            c_out, rust_out,
            "printIntPtrLine mismatch for val={val}: C={:?} Rust={:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&rust_out)
        );
    }
}

#[test]
fn test_good() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_fn: Symbol<unsafe extern "C" fn()> =
        unsafe { c_lib.get(b"good").unwrap() };

    let c_out = capture_stdout(|| unsafe { c_fn() });
    let rust_out = capture_stdout(|| driver::good());
    assert_eq!(
        c_out, rust_out,
        "good() mismatch: C={:?} Rust={:?}",
        String::from_utf8_lossy(&c_out),
        String::from_utf8_lossy(&rust_out)
    );
}
