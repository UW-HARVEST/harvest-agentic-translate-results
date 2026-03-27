use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_src/build/libStaticLoop.so")
}

fn rust_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libStaticLoop.so")
}

/// Copy a .so to a unique temp file so dlopen gives us a fresh instance with reset statics.
fn fresh_lib(src: &PathBuf) -> (tempfile::NamedTempFile, Library) {
    let tmp = tempfile::Builder::new().suffix(".so").tempfile().unwrap();
    std::fs::copy(src, tmp.path()).unwrap();
    let lib = unsafe { Library::new(tmp.path()).expect("load fresh .so") };
    (tmp, lib) // keep tmp alive so the file isn't deleted
}

#[test]
fn test_static_sum_sequence() {
    let updates = [5, 3, -2, 10, 0, 7, -1, 100, -50, 1];

    let (_tc, c_lib) = fresh_lib(&c_lib_path());
    let (_tr, rust_lib) = fresh_lib(&rust_lib_path());

    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"static_sum").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            rust_lib.get(b"static_sum").unwrap();

        for (i, &u) in updates.iter().enumerate() {
            let c_val = c_fn(u);
            let r_val = r_fn(u);
            assert_eq!(c_val, r_val,
                "static_sum mismatch at call {i}: input={u}, C={c_val}, Rust={r_val}");
        }
    }
}

#[test]
fn test_driver_output() {
    for &stride in &[1, 2, 3, 0, -1, 5] {
        let c_out = capture_driver(&c_lib_path(), stride);
        let r_out = capture_driver(&rust_lib_path(), stride);
        assert_eq!(c_out, r_out,
            "driver({stride}) mismatch:\nC:\n{c_out}\nRust:\n{r_out}");
    }
}

fn capture_driver(lib_path: &PathBuf, stride: c_int) -> String {
    use std::io::Read;
    use std::os::unix::io::FromRawFd;

    let (_tmp, lib) = fresh_lib(lib_path);
    unsafe {
        let driver_fn: Symbol<unsafe extern "C" fn(c_int)> = lib.get(b"driver").unwrap();

        let mut fds = [0i32; 2];
        assert_eq!(libc::pipe(fds.as_mut_ptr()), 0);
        let (read_fd, write_fd) = (fds[0], fds[1]);

        let orig = libc::dup(1);
        libc::dup2(write_fd, 1);
        libc::close(write_fd);

        driver_fn(stride);
        libc::fflush(std::ptr::null_mut());

        libc::dup2(orig, 1);
        libc::close(orig);

        let mut f = std::fs::File::from_raw_fd(read_fd);
        let mut buf = String::new();
        f.read_to_string(&mut buf).unwrap();
        buf
    }
}
