use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libdriver.so"
    );
    unsafe { Library::new(path).expect("Failed to load C libdriver.so") }
}

#[test]
fn test_normal_string() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_char> =
        unsafe { lib.get(b"custom_strdup").unwrap() };

    let input = CString::new("hello world").unwrap();

    let c_result = unsafe { (*c_fn)(input.as_ptr()) };
    let rust_result = unsafe { driver::custom_strdup(input.as_ptr()) };

    assert!(!c_result.is_null());
    assert!(!rust_result.is_null());

    let c_str = unsafe { CStr::from_ptr(c_result) };
    let rust_str = unsafe { CStr::from_ptr(rust_result) };
    assert_eq!(c_str, rust_str, "Mismatch for \"hello world\"");

    unsafe {
        libc::free(c_result as *mut _);
        libc::free(rust_result as *mut _);
    }
}

#[test]
fn test_empty_string() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_char> =
        unsafe { lib.get(b"custom_strdup").unwrap() };

    let input = CString::new("").unwrap();

    let c_result = unsafe { (*c_fn)(input.as_ptr()) };
    let rust_result = unsafe { driver::custom_strdup(input.as_ptr()) };

    assert!(!c_result.is_null());
    assert!(!rust_result.is_null());

    let c_str = unsafe { CStr::from_ptr(c_result) };
    let rust_str = unsafe { CStr::from_ptr(rust_result) };
    assert_eq!(c_str, rust_str, "Mismatch for empty string");

    unsafe {
        libc::free(c_result as *mut _);
        libc::free(rust_result as *mut _);
    }
}

#[test]
fn test_null_input() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_char> =
        unsafe { lib.get(b"custom_strdup").unwrap() };

    let c_result = unsafe { (*c_fn)(ptr::null()) };
    let rust_result = unsafe { driver::custom_strdup(ptr::null()) };

    assert!(c_result.is_null(), "C should return NULL for NULL input");
    assert!(rust_result.is_null(), "Rust should return NULL for NULL input");
}

#[test]
fn test_embedded_bytes() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_char> =
        unsafe { lib.get(b"custom_strdup").unwrap() };

    // String with special characters (build from bytes to include high values)
    let bytes: Vec<u8> = vec![b'a', b'b', b'c', 0x01, 0x7f, 0xfe];
    let input = CString::new(bytes).unwrap();

    let c_result = unsafe { (*c_fn)(input.as_ptr()) };
    let rust_result = unsafe { driver::custom_strdup(input.as_ptr()) };

    assert!(!c_result.is_null());
    assert!(!rust_result.is_null());

    let c_str = unsafe { CStr::from_ptr(c_result) };
    let rust_str = unsafe { CStr::from_ptr(rust_result) };
    assert_eq!(c_str, rust_str, "Mismatch for special chars string");

    unsafe {
        libc::free(c_result as *mut _);
        libc::free(rust_result as *mut _);
    }
}

#[test]
fn test_long_string() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_char> =
        unsafe { lib.get(b"custom_strdup").unwrap() };

    let long = "A".repeat(10000);
    let input = CString::new(long).unwrap();

    let c_result = unsafe { (*c_fn)(input.as_ptr()) };
    let rust_result = unsafe { driver::custom_strdup(input.as_ptr()) };

    assert!(!c_result.is_null());
    assert!(!rust_result.is_null());

    // Compare byte-for-byte including null terminator
    let c_bytes = unsafe { std::slice::from_raw_parts(c_result as *const u8, 10001) };
    let rust_bytes = unsafe { std::slice::from_raw_parts(rust_result as *const u8, 10001) };
    assert_eq!(c_bytes, rust_bytes, "Byte mismatch for long string");

    unsafe {
        libc::free(c_result as *mut _);
        libc::free(rust_result as *mut _);
    }
}
