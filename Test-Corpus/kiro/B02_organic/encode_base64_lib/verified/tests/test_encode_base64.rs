use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CStr, CString};

type EncodeFn = unsafe extern "C" fn(c_int, *const c_char) -> *mut c_char;

fn load_libs() -> (Library, Library) {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let c_path = format!("{}/c_src/build/libdriver.so", manifest);
    let rust_path = format!("{}/target/debug/libdriver.so", manifest);
    unsafe {
        (
            Library::new(&c_path).expect("load C .so"),
            Library::new(&rust_path).expect("load Rust .so"),
        )
    }
}

fn call_encode(lib: &Library, size: c_int, src: *const c_char) -> Option<Vec<u8>> {
    unsafe {
        let f: Symbol<EncodeFn> = lib.get(b"encode_base64").unwrap();
        let ptr = f(size, src);
        if ptr.is_null() {
            return None;
        }
        let s = CStr::from_ptr(ptr).to_bytes().to_vec();
        libc::free(ptr as *mut _);
        Some(s)
    }
}

#[test]
fn null_input() {
    let (c_lib, rs_lib) = load_libs();
    let c_out = call_encode(&c_lib, 0, std::ptr::null());
    let r_out = call_encode(&rs_lib, 0, std::ptr::null());
    assert_eq!(c_out, r_out, "null input");
}

#[test]
fn empty_string() {
    let (c_lib, rs_lib) = load_libs();
    let s = CString::new("").unwrap();
    let c_out = call_encode(&c_lib, 0, s.as_ptr());
    let r_out = call_encode(&rs_lib, 0, s.as_ptr());
    assert_eq!(c_out, r_out, "empty string");
}

#[test]
fn known_vectors() {
    let (c_lib, rs_lib) = load_libs();
    // Standard base64 test vectors
    let cases: &[&[u8]] = &[
        b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar",
        b"Hello, World!", b"\x00\x01\x02\x03",
        b"\xff\xfe\xfd",
    ];
    for input in cases {
        let size = input.len() as c_int;
        let c_out = call_encode(&c_lib, size, input.as_ptr() as *const c_char);
        let r_out = call_encode(&rs_lib, size, input.as_ptr() as *const c_char);
        assert_eq!(c_out, r_out, "mismatch for input {:?}", input);
    }
}

#[test]
fn size_zero_uses_strlen() {
    let (c_lib, rs_lib) = load_libs();
    let s = CString::new("test123").unwrap();
    let c_out = call_encode(&c_lib, 0, s.as_ptr());
    let r_out = call_encode(&rs_lib, 0, s.as_ptr());
    assert_eq!(c_out, r_out, "size=0 strlen path");
}

#[test]
fn explicit_size() {
    let (c_lib, rs_lib) = load_libs();
    // Pass explicit size shorter than the string
    let s = CString::new("Hello, World!").unwrap();
    for sz in 1..=13 {
        let c_out = call_encode(&c_lib, sz, s.as_ptr());
        let r_out = call_encode(&rs_lib, sz, s.as_ptr());
        assert_eq!(c_out, r_out, "explicit size={}", sz);
    }
}

#[test]
fn all_byte_values() {
    let (c_lib, rs_lib) = load_libs();
    let input: Vec<u8> = (0..=255).collect();
    let size = input.len() as c_int;
    let c_out = call_encode(&c_lib, size, input.as_ptr() as *const c_char);
    let r_out = call_encode(&rs_lib, size, input.as_ptr() as *const c_char);
    assert_eq!(c_out, r_out, "all 256 byte values");
}

#[test]
fn single_bytes() {
    let (c_lib, rs_lib) = load_libs();
    for b in 0u8..=255 {
        let input = [b];
        let c_out = call_encode(&c_lib, 1, input.as_ptr() as *const c_char);
        let r_out = call_encode(&rs_lib, 1, input.as_ptr() as *const c_char);
        assert_eq!(c_out, r_out, "single byte 0x{:02x}", b);
    }
}
