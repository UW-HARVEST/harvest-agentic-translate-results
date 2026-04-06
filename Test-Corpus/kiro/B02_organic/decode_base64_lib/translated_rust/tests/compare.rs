use libloading::{Library, Symbol};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

extern "C" {
    fn free(ptr: *mut std::ffi::c_void);
}

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");
    unsafe { Library::new(path).expect("Failed to load C libdriver.so") }
}

fn rust_lib() -> Library {
    let path = format!(
        "{}/target/debug/libdriver.so",
        env!("CARGO_MANIFEST_DIR")
    );
    unsafe { Library::new(&path).expect("Failed to load Rust libdriver.so") }
}

fn call_decode(lib: &Library, input: &[u8]) -> Option<Vec<u8>> {
    unsafe {
        let func: Symbol<unsafe extern "C" fn(*const c_char) -> *mut c_char> =
            lib.get(b"decode_base64").unwrap();
        let c_input = CString::new(input).unwrap();
        let ptr = func(c_input.as_ptr());
        if ptr.is_null() {
            return None;
        }
        let result = CStr::from_ptr(ptr).to_bytes().to_vec();
        free(ptr as *mut _);
        Some(result)
    }
}

fn assert_match(c_lib: &Library, r_lib: &Library, input: &[u8]) {
    let c_out = call_decode(c_lib, input);
    let r_out = call_decode(r_lib, input);
    assert_eq!(
        c_out, r_out,
        "Mismatch for input {:?}: C={:?} Rust={:?}",
        String::from_utf8_lossy(input), c_out, r_out
    );
}

#[test]
fn test_empty() {
    let c = c_lib();
    let r = rust_lib();
    assert_match(&c, &r, b"");
}

#[test]
fn test_standard_vectors() {
    let c = c_lib();
    let r = rust_lib();
    for input in &[b"Zg==" as &[u8], b"Zm8=", b"Zm9v", b"Zm9vYg==", b"Zm9vYmE=", b"Zm9vYmFy"] {
        assert_match(&c, &r, input);
    }
}

#[test]
fn test_no_padding() {
    let c = c_lib();
    let r = rust_lib();
    for input in &[b"YQ" as &[u8], b"YWI", b"YWJJ"] {
        assert_match(&c, &r, input);
    }
}

#[test]
fn test_non_base64_chars() {
    let c = c_lib();
    let r = rust_lib();
    for input in &[b"Zm 9v\nYm Fy" as &[u8], b"!!!Zm9v!!!"] {
        assert_match(&c, &r, input);
    }
}

#[test]
fn test_plus_slash() {
    let c = c_lib();
    let r = rust_lib();
    for input in &[b"+/+/" as &[u8], b"//8=", b"+w=="] {
        assert_match(&c, &r, input);
    }
}

#[test]
fn test_long_input() {
    let c = c_lib();
    let r = rust_lib();
    assert_match(&c, &r, b"SGVsbG8sIFdvcmxkIQ==");
}

#[test]
fn test_all_bytes() {
    let c = c_lib();
    let r = rust_lib();
    assert_match(&c, &r, b"AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaGxwdHh8gISIjJCUmJygpKissLS4v");
}
