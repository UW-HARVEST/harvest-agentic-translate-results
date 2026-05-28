// Integration tests that compare C and Rust .so outputs through libloading.
// Both libraries are loaded as dynamic libraries and called via FFI exactly
// as an external caller would.

use libloading::{Library, Symbol};
use std::ffi::{CString, c_char};
use std::path::PathBuf;

extern "C" {
    fn free(ptr: *mut libc::c_void);
}

type DecodeBase64Fn = unsafe extern "C" fn(*const c_char) -> *mut c_char;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    project_root().join("c_src").join("build").join("libdriver.so")
}

fn rust_lib_path() -> PathBuf {
    // Try release first then debug
    let rel = project_root()
        .join("target")
        .join("release")
        .join("libdriver.so");
    if rel.exists() {
        return rel;
    }
    project_root()
        .join("target")
        .join("debug")
        .join("libdriver.so")
}

/// Read raw bytes from a NUL-terminated C string returned by decode_base64.
/// Returns the decoded payload as a Vec<u8> excluding the trailing NUL.
unsafe fn read_cstr_bytes(p: *mut c_char) -> Vec<u8> {
    if p.is_null() {
        return Vec::new();
    }
    let mut len = 0;
    while *p.add(len) != 0 {
        len += 1;
    }
    let mut v = Vec::with_capacity(len);
    for i in 0..len {
        v.push(*p.add(i) as u8);
    }
    v
}

/// Compare both implementations on a single input.
fn compare(input: Option<&str>) {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let rust_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    let c_fn: Symbol<DecodeBase64Fn> = unsafe { c_lib.get(b"decode_base64\0") }.unwrap();
    let rust_fn: Symbol<DecodeBase64Fn> = unsafe { rust_lib.get(b"decode_base64\0") }.unwrap();

    let (c_in, r_in): (*const c_char, *const c_char) = match input {
        Some(s) => {
            // We can have NULs in some test cases - use bytes directly
            // but the C API takes NUL-terminated strings, so use CString
            let cs = CString::new(s).expect("contains NUL");
            // Leak both so that the pointers remain valid for the call
            let cs_ptr = cs.into_raw();
            (cs_ptr, cs_ptr)
        }
        None => (std::ptr::null(), std::ptr::null()),
    };

    let c_out = unsafe { c_fn(c_in) };
    let r_out = unsafe { rust_fn(r_in) };

    // Both null or both non-null
    assert_eq!(
        c_out.is_null(),
        r_out.is_null(),
        "null mismatch for input {:?}",
        input
    );

    if !c_out.is_null() {
        let cb = unsafe { read_cstr_bytes(c_out) };
        let rb = unsafe { read_cstr_bytes(r_out) };
        assert_eq!(cb, rb, "byte mismatch for input {:?}", input);
        unsafe {
            free(c_out as *mut libc::c_void);
            free(r_out as *mut libc::c_void);
        }
    }

    // Reclaim and free the input CString
    if !c_in.is_null() {
        let _ = unsafe { CString::from_raw(c_in as *mut c_char) };
    }
}

#[test]
fn null_input_returns_null() {
    compare(None);
}

#[test]
fn empty_string_returns_null() {
    compare(Some(""));
}

#[test]
fn simple_word() {
    // "Man" -> "TWFu"
    compare(Some("TWFu"));
}

#[test]
fn no_padding() {
    // "Many" -> "TWFueQ=="
    compare(Some("TWFueQ=="));
}

#[test]
fn one_padding() {
    // "Hello" -> "SGVsbG8="
    compare(Some("SGVsbG8="));
}

#[test]
fn longer_text() {
    // "Hello, World!" -> "SGVsbG8sIFdvcmxkIQ=="
    compare(Some("SGVsbG8sIFdvcmxkIQ=="));
}

#[test]
fn all_alphabet() {
    compare(Some(
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/",
    ));
}

#[test]
fn whitespace_and_newlines_are_ignored() {
    // Per POSIX, non-base64 chars must be ignored.
    compare(Some("TWFu\nTWFu"));
    compare(Some("TWFu TWFu"));
    compare(Some("  TWFu\t\nTW Fu  "));
}

#[test]
fn input_is_padding_only() {
    compare(Some("===="));
}

#[test]
fn single_padding() {
    compare(Some("="));
}

#[test]
fn slashes_and_pluses() {
    compare(Some("Pz8/Pw=="));
}

#[test]
fn weird_partial_groups() {
    // Lengths not divisible by 4 (rare but C handles them)
    compare(Some("T"));
    compare(Some("TW"));
    compare(Some("TWF"));
    compare(Some("TWFu"));
    compare(Some("TWFuT"));
    compare(Some("TWFuTW"));
}

#[test]
fn unicode_payload() {
    // "héllo" UTF-8 -> "aMOpbGxv"
    compare(Some("aMOpbGxv"));
}

#[test]
fn long_random_input() {
    // Encoded form of 1000 'A' bytes
    let raw_a = "A".repeat(1000);
    let encoded = base64_encode(raw_a.as_bytes());
    compare(Some(&encoded));
}

#[test]
fn binary_data_decoded() {
    // Encode arbitrary binary, decode and compare
    let bytes: Vec<u8> = (0u8..=255).collect();
    let encoded = base64_encode(&bytes);
    compare(Some(&encoded));
}

#[test]
fn malformed_inputs_with_random_chars() {
    // C ignores anything that isn't a base64 char.
    compare(Some("!!!T!!W!!F!!u!!"));
    compare(Some("***"));
}

/// Minimal base64 encoder for building test inputs.
fn base64_encode(data: &[u8]) -> String {
    const ALPHA: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i + 3 <= data.len() {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8) | (data[i + 2] as u32);
        out.push(ALPHA[((n >> 18) & 63) as usize] as char);
        out.push(ALPHA[((n >> 12) & 63) as usize] as char);
        out.push(ALPHA[((n >> 6) & 63) as usize] as char);
        out.push(ALPHA[(n & 63) as usize] as char);
        i += 3;
    }
    let rem = data.len() - i;
    if rem == 1 {
        let n = (data[i] as u32) << 16;
        out.push(ALPHA[((n >> 18) & 63) as usize] as char);
        out.push(ALPHA[((n >> 12) & 63) as usize] as char);
        out.push('=');
        out.push('=');
    } else if rem == 2 {
        let n = ((data[i] as u32) << 16) | ((data[i + 1] as u32) << 8);
        out.push(ALPHA[((n >> 18) & 63) as usize] as char);
        out.push(ALPHA[((n >> 12) & 63) as usize] as char);
        out.push(ALPHA[((n >> 6) & 63) as usize] as char);
        out.push('=');
    }
    out
}
