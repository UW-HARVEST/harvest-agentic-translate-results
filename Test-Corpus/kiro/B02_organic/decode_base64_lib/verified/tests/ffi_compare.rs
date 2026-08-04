use libloading::{Library, Symbol};
use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::PathBuf;

extern "C" {
    fn free(ptr: *mut std::ffi::c_void);
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // The cdylib is built in the deps dir or the debug dir
    let debug = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/libdriver.so");
    debug
}

type DecodeFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

fn call_decode(lib: &Library, input: &[u8]) -> Option<Vec<u8>> {
    unsafe {
        let func: Symbol<DecodeFn> = lib.get(b"decode_base64").unwrap();
        let result = func(input.as_ptr() as *const c_char);
        if result.is_null() {
            return None;
        }
        let bytes = CStr::from_ptr(result).to_bytes().to_vec();
        free(result as *mut std::ffi::c_void);
        Some(bytes)
    }
}

fn compare(c_lib: &Library, rs_lib: &Library, input: &[u8], label: &str) {
    let c_out = call_decode(c_lib, input);
    let rs_out = call_decode(rs_lib, input);
    assert_eq!(c_out, rs_out, "Mismatch for test case: {label}");
}

#[test]
fn test_decode_base64() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let rs_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    // NULL input — we can't pass null through CString, test empty string instead
    compare(&c_lib, &rs_lib, b"\0", "empty string");

    // Standard base64 test vectors
    let cases: &[(&[u8], &str)] = &[
        (b"aGVsbG8=\0", "hello"),
        (b"d29ybGQ=\0", "world"),
        (b"SGVsbG8gV29ybGQ=\0", "Hello World"),
        (b"YQ==\0", "a"),
        (b"YWI=\0", "ab"),
        (b"YWJj\0", "abc"),
        (b"YWJjZA==\0", "abcd"),
        (b"\0", "empty"),
        // No padding
        (b"YQ\0", "a no pad"),
        (b"YWI\0", "ab no pad"),
        // All chars in alphabet
        (b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/\0", "full alphabet"),
        // With invalid chars interspersed (should be ignored)
        (b"aGV sb G8=\0", "with spaces"),
        (b"aGVs\nbG8=\0", "with newline"),
        // Padding only
        (b"====\0", "all padding"),
        (b"=\0", "single equals"),
        // Single char
        (b"A\0", "single A"),
        (b"/\0", "single slash"),
        (b"+\0", "single plus"),
        // Longer input
        (b"VGhlIHF1aWNrIGJyb3duIGZveCBqdW1wcyBvdmVyIHRoZSBsYXp5IGRvZw==\0",
         "the quick brown fox"),
    ];

    for (input, label) in cases {
        compare(&c_lib, &rs_lib, input, label);
    }

    // Test NULL pointer directly
    unsafe {
        let c_func: Symbol<DecodeFn> = c_lib.get(b"decode_base64").unwrap();
        let rs_func: Symbol<DecodeFn> = rs_lib.get(b"decode_base64").unwrap();
        let c_res = c_func(std::ptr::null());
        let rs_res = rs_func(std::ptr::null());
        assert!(c_res.is_null(), "C should return NULL for null input");
        assert!(rs_res.is_null(), "Rust should return NULL for null input");
    }
}
