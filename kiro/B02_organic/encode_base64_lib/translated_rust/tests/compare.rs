use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CStr, CString};

fn c_lib_path() -> String {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    format!("{}/c_src/build/libdriver.so", manifest)
}

/// Call C encode_base64 via libloading
unsafe fn call_c_encode_base64(lib: &Library, size: c_int, src: *const c_char) -> *mut c_char {
    let func: Symbol<unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char> =
        lib.get(b"encode_base64").unwrap();
    func(size, src)
}

/// Helper: compare C vs Rust encode_base64 for given input bytes
fn compare_encode_base64(input: &[u8], pass_size: bool) {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("Failed to load C library") };

    let (size, src): (c_int, *const c_char) = if pass_size {
        (input.len() as c_int, input.as_ptr() as *const c_char)
    } else {
        // size=0 means C uses strlen, so input must be null-terminated
        let cs = CString::new(input).unwrap();
        // Leak so pointer stays valid
        let ptr = cs.into_raw() as *const c_char;
        (0, ptr)
    };

    let c_result = unsafe { call_c_encode_base64(&c_lib, size, src) };
    let rust_result = driver::encode_base64(size, src);

    if c_result.is_null() && rust_result.is_null() {
        // Both null — match
        if !pass_size && size == 0 {
            // reclaim leaked CString
            unsafe { drop(CString::from_raw(src as *mut c_char)); }
        }
        return;
    }

    assert!(!c_result.is_null(), "C returned null but Rust didn't");
    assert!(!rust_result.is_null(), "Rust returned null but C didn't");

    let c_str = unsafe { CStr::from_ptr(c_result) };
    let rust_str = unsafe { CStr::from_ptr(rust_result) };

    assert_eq!(
        c_str.to_bytes(),
        rust_str.to_bytes(),
        "Mismatch for input {:?} (pass_size={}): C={:?} Rust={:?}",
        input,
        pass_size,
        c_str,
        rust_str
    );

    unsafe {
        libc::free(c_result as *mut _);
        libc::free(rust_result as *mut _);
    }

    if !pass_size && size == 0 {
        unsafe { drop(CString::from_raw(src as *mut c_char)); }
    }
}

// Reference the Rust function directly (lib name is "driver")
extern crate driver;

#[test]
fn test_null_input() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let c_result = unsafe { call_c_encode_base64(&c_lib, 0, std::ptr::null()) };
    let rust_result = driver::encode_base64(0, std::ptr::null());
    assert!(c_result.is_null());
    assert!(rust_result.is_null());
}

#[test]
fn test_empty_string() {
    // Use size=0 with strlen path (null-terminated empty string)
    compare_encode_base64(b"", false);
}

#[test]
fn test_single_byte() {
    for b in 0..=255u8 {
        compare_encode_base64(&[b], true);
    }
}

#[test]
fn test_two_bytes() {
    compare_encode_base64(b"AB", true);
    compare_encode_base64(b"\x00\xff", true);
    compare_encode_base64(b"\xff\x00", true);
}

#[test]
fn test_three_bytes() {
    compare_encode_base64(b"ABC", true);
    compare_encode_base64(b"\x00\x00\x00", true);
    compare_encode_base64(b"\xff\xff\xff", true);
}

#[test]
fn test_known_vectors() {
    // Standard base64 test vectors
    compare_encode_base64(b"Hello", true);
    compare_encode_base64(b"Hello, World!", true);
    compare_encode_base64(b"Man", true);
    compare_encode_base64(b"Ma", true);
    compare_encode_base64(b"M", true);
    compare_encode_base64(b"foobar", true);
    compare_encode_base64(b"fooba", true);
    compare_encode_base64(b"foob", true);
    compare_encode_base64(b"foo", true);
    compare_encode_base64(b"fo", true);
    compare_encode_base64(b"f", true);
}

#[test]
fn test_size_zero_uses_strlen() {
    // size=0 triggers strlen in C code
    compare_encode_base64(b"Hello", false);
    compare_encode_base64(b"test", false);
}

#[test]
fn test_binary_data() {
    let data: Vec<u8> = (0..=255).collect();
    compare_encode_base64(&data, true);
}

#[test]
fn test_longer_input() {
    let data = b"The quick brown fox jumps over the lazy dog";
    compare_encode_base64(data, true);
}
