use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target");
    if cfg!(debug_assertions) {
        p.push("debug");
    } else {
        p.push("release");
    }
    p.push("libdriver.so");
    p
}

#[test]
fn test_fma_array_separate_buffers() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fma: Symbol<unsafe extern "C" fn(*mut i32, *const i32, *const i32, *const i32, i32)> =
            c_lib.get(b"fma_array").expect("c fma_array");

        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let r_fma: Symbol<unsafe extern "C" fn(*mut i32, *const i32, *const i32, *const i32, i32)> =
            r_lib.get(b"fma_array").expect("r fma_array");

        let mul1 = [1i32, 2, 3, 4, 5];
        let mul2 = [10i32, 20, 30, 40, 50];
        let add = [100i32, 200, 300, 400, 500];

        let mut c_out = [0i32; 5];
        let mut r_out = [0i32; 5];

        c_fma(c_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 5);
        r_fma(r_out.as_mut_ptr(), mul1.as_ptr(), mul2.as_ptr(), add.as_ptr(), 5);

        assert_eq!(c_out, r_out, "fma_array separate buffers mismatch: C={:?} Rust={:?}", c_out, r_out);
    }
}

#[test]
fn test_fma_array_aliased_buffers() {
    // This is the critical test: all pointers are the same buffer (like driver() does)
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fma: Symbol<unsafe extern "C" fn(*mut i32, *const i32, *const i32, *const i32, i32)> =
            c_lib.get(b"fma_array").expect("c fma_array");

        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let r_fma: Symbol<unsafe extern "C" fn(*mut i32, *const i32, *const i32, *const i32, i32)> =
            r_lib.get(b"fma_array").expect("r fma_array");

        let input = [1i32, 2, 3, 4, 5, -1, 0, 100];

        let mut c_buf = input;
        let mut r_buf = input;

        c_fma(c_buf.as_mut_ptr(), c_buf.as_ptr(), c_buf.as_ptr(), c_buf.as_ptr(), input.len() as i32);
        r_fma(r_buf.as_mut_ptr(), r_buf.as_ptr(), r_buf.as_ptr(), r_buf.as_ptr(), input.len() as i32);

        assert_eq!(c_buf, r_buf, "fma_array aliased mismatch: C={:?} Rust={:?}", c_buf, r_buf);
    }
}

#[test]
fn test_driver_output() {
    // driver() calls fma_array then prints. Compare the buffer state after call.
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_driver: Symbol<unsafe extern "C" fn(*mut i32, i32)> =
            c_lib.get(b"driver").expect("c driver");

        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let r_driver: Symbol<unsafe extern "C" fn(*mut i32, i32)> =
            r_lib.get(b"driver").expect("r driver");

        let input = [3i32, 7, -2, 0, 10];

        let mut c_buf = input;
        let mut r_buf = input;

        c_driver(c_buf.as_mut_ptr(), input.len() as i32);
        r_driver(r_buf.as_mut_ptr(), input.len() as i32);

        assert_eq!(c_buf, r_buf, "driver buffer mismatch: C={:?} Rust={:?}", c_buf, r_buf);
    }
}

#[test]
fn test_fma_array_empty() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fma: Symbol<unsafe extern "C" fn(*mut i32, *const i32, *const i32, *const i32, i32)> =
            c_lib.get(b"fma_array").expect("c fma_array");

        let r_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let r_fma: Symbol<unsafe extern "C" fn(*mut i32, *const i32, *const i32, *const i32, i32)> =
            r_lib.get(b"fma_array").expect("r fma_array");

        let mut c_out = [0i32; 1];
        let mut r_out = [0i32; 1];

        c_fma(c_out.as_mut_ptr(), c_out.as_ptr(), c_out.as_ptr(), c_out.as_ptr(), 0);
        r_fma(r_out.as_mut_ptr(), r_out.as_ptr(), r_out.as_ptr(), r_out.as_ptr(), 0);

        assert_eq!(c_out, r_out);
    }
}
