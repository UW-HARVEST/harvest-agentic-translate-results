use libloading::{Library, Symbol};
use std::ffi::{c_char, CStr};

type CreateLinePointersFn =
    unsafe extern "C" fn(*mut c_char, usize, usize) -> *const *const c_char;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libdriver.so"
    );
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

/// Helper: call both C (via libloading) and Rust, compare returned pointer offsets
unsafe fn compare(buf: &mut [u8], num_lines: usize) {
    let buf_size = buf.len();
    let buf_ptr = buf.as_mut_ptr() as *mut c_char;

    // Call C version
    let lib = c_lib();
    let c_fn: Symbol<CreateLinePointersFn> =
        unsafe { lib.get(b"UTIL_createLinePointers\0").unwrap() };
    let c_result = unsafe { c_fn(buf_ptr, num_lines, buf_size) };

    // Call Rust version
    let rust_result =
        unsafe { driver::UTIL_createLinePointers(buf_ptr, num_lines, buf_size) };

    // Both null or both non-null
    assert_eq!(
        c_result.is_null(),
        rust_result.is_null(),
        "Null mismatch: C null={}, Rust null={}",
        c_result.is_null(),
        rust_result.is_null()
    );

    if c_result.is_null() {
        return;
    }

    // Compare each pointer as offset from buffer start
    for i in 0..num_lines {
        let c_ptr = unsafe { *c_result.add(i) };
        let r_ptr = unsafe { *rust_result.add(i) };
        let c_off = c_ptr as usize - buf_ptr as usize;
        let r_off = r_ptr as usize - buf_ptr as usize;
        assert_eq!(
            c_off, r_off,
            "Line {} pointer offset mismatch: C={}, Rust={}",
            i, c_off, r_off
        );
        // Also compare the string content
        let c_str = unsafe { CStr::from_ptr(c_ptr) };
        let r_str = unsafe { CStr::from_ptr(r_ptr) };
        assert_eq!(
            c_str, r_str,
            "Line {} string mismatch: C={:?}, Rust={:?}",
            i, c_str, r_str
        );
    }

    // Free both results
    unsafe {
        libc_free(c_result as *mut _);
        libc_free(rust_result as *mut _);
    }
}

unsafe fn libc_free(ptr: *mut std::ffi::c_void) {
    extern "C" {
        fn free(ptr: *mut std::ffi::c_void);
    }
    unsafe { free(ptr) }
}

#[test]
fn test_basic_two_lines() {
    let mut buf = b"hello\0world\0".to_vec();
    unsafe { compare(&mut buf, 2) };
}

#[test]
fn test_single_line() {
    let mut buf = b"only\0".to_vec();
    unsafe { compare(&mut buf, 1) };
}

#[test]
fn test_empty_strings() {
    let mut buf = b"\0\0\0".to_vec();
    unsafe { compare(&mut buf, 3) };
}

#[test]
fn test_too_many_lines_returns_null() {
    let mut buf = b"a\0b\0".to_vec();
    unsafe { compare(&mut buf, 5) };
}

#[test]
fn test_zero_lines() {
    let mut buf = b"anything\0".to_vec();
    let buf_ptr = buf.as_mut_ptr() as *mut c_char;
    let lib = c_lib();
    let c_fn: Symbol<CreateLinePointersFn> =
        unsafe { lib.get(b"UTIL_createLinePointers\0").unwrap() };
    let c_result = unsafe { c_fn(buf_ptr, 0, buf.len()) };
    let rust_result =
        unsafe { driver::UTIL_createLinePointers(buf_ptr, 0, buf.len()) };
    assert_eq!(
        c_result.is_null(),
        rust_result.is_null(),
        "Zero lines null mismatch: C null={}, Rust null={}",
        c_result.is_null(),
        rust_result.is_null()
    );
}

#[test]
fn test_no_null_terminator() {
    let mut buf = b"abcdef".to_vec();
    unsafe { compare(&mut buf, 1) };
}

#[test]
fn test_many_lines() {
    let mut buf: Vec<u8> = Vec::new();
    for i in 0..10u8 {
        buf.push(b'A' + i);
        buf.push(0);
    }
    unsafe { compare(&mut buf, 10) };
}
