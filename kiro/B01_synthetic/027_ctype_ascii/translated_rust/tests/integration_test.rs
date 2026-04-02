use libloading::{Library, Symbol};

/// Capture stdout from a function call by redirecting fd 1 to a temp file.
fn capture_stdout(f: impl FnOnce()) -> String {
    unsafe {
        let tmpfile = libc::tmpfile();
        let tmp_fd = libc::fileno(tmpfile);
        let old_stdout = libc::dup(1);
        libc::dup2(tmp_fd, 1);
        f();
        libc::fflush(std::ptr::null_mut());
        libc::dup2(old_stdout, 1);
        libc::close(old_stdout);
        libc::lseek(tmp_fd, 0, libc::SEEK_SET);
        let mut buf = vec![0u8; 16384];
        let n = libc::read(tmp_fd, buf.as_mut_ptr() as *mut _, buf.len());
        libc::fclose(tmpfile);
        if n > 0 {
            buf.truncate(n as usize);
            String::from_utf8_lossy(&buf).into_owned()
        } else {
            String::new()
        }
    }
}

fn c_lib_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest).join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::path::PathBuf::from(manifest).join("target/debug/libdriver.so")
}

#[test]
fn test_driver_all_ascii() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };
    let c_driver: Symbol<unsafe extern "C" fn(libc::c_char)> =
        unsafe { c_lib.get(b"driver").expect("C driver symbol not found") };

    let r_lib = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust .so") };
    let r_driver: Symbol<unsafe extern "C" fn(libc::c_int)> =
        unsafe { r_lib.get(b"driver").expect("Rust driver symbol not found") };

    for i in 0..=127u8 {
        let c_out = capture_stdout(|| unsafe { c_driver(i as libc::c_char) });
        let r_out = capture_stdout(|| unsafe { r_driver(i as libc::c_int) });
        assert_eq!(
            c_out, r_out,
            "Mismatch for byte {}: C={:?} Rust={:?}",
            i, c_out, r_out
        );
    }
}

#[test]
fn test_driver_eof() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") };
    let c_driver: Symbol<unsafe extern "C" fn(libc::c_char)> =
        unsafe { c_lib.get(b"driver").expect("C driver symbol not found") };

    let r_lib = unsafe { Library::new(rust_lib_path()).expect("Failed to load Rust .so") };
    let r_driver: Symbol<unsafe extern "C" fn(libc::c_int)> =
        unsafe { r_lib.get(b"driver").expect("Rust driver symbol not found") };

    let c_out = capture_stdout(|| unsafe { c_driver(-1i8) });
    let r_out = capture_stdout(|| unsafe { r_driver(-1 as libc::c_int) });
    assert_eq!(c_out, r_out, "Mismatch for EOF(-1): C={:?} Rust={:?}", c_out, r_out);
}
