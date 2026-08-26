use libloading::{Library, Symbol};
use std::ffi::{c_char, CStr, CString};
use std::path::PathBuf;

struct Libs {
    _c: Library,
    _rs: Library,
    c_drop: libloading::Symbol<'static, unsafe extern "C" fn(*const c_char) -> *const c_char>,
    rs_drop: libloading::Symbol<'static, unsafe extern "C" fn(*const c_char) -> *const c_char>,
    c_filter:
        libloading::Symbol<'static, unsafe extern "C" fn(*const c_char, bool) -> *mut c_char>,
    rs_filter:
        libloading::Symbol<'static, unsafe extern "C" fn(*const c_char, bool) -> *mut c_char>,
}

fn load() -> Libs {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest.join("c_src/build/libdriver.so");
    let rs_path = manifest.join("target/debug/libdriver.so");

    // We need 'static lifetimes for the symbols, so leak the libraries
    let c_lib: &'static Library =
        Box::leak(Box::new(unsafe { Library::new(&c_path).expect("load C .so") }));
    let rs_lib: &'static Library =
        Box::leak(Box::new(unsafe { Library::new(&rs_path).expect("load Rust .so") }));

    unsafe {
        let c_drop: Symbol<'static, unsafe extern "C" fn(*const c_char) -> *const c_char> =
            c_lib.get(b"w_utf8_drop").unwrap();
        let rs_drop: Symbol<'static, unsafe extern "C" fn(*const c_char) -> *const c_char> =
            rs_lib.get(b"w_utf8_drop").unwrap();
        let c_filter: Symbol<
            'static,
            unsafe extern "C" fn(*const c_char, bool) -> *mut c_char,
        > = c_lib.get(b"w_utf8_filter").unwrap();
        let rs_filter: Symbol<
            'static,
            unsafe extern "C" fn(*const c_char, bool) -> *mut c_char,
        > = rs_lib.get(b"w_utf8_filter").unwrap();

        Libs {
            _c: unsafe { Library::new(&c_path).unwrap() }, // dummy, real refs are leaked
            _rs: unsafe { Library::new(&rs_path).unwrap() },
            c_drop,
            rs_drop,
            c_filter,
            rs_filter,
        }
    }
}

/// Compare w_utf8_drop: both should return the same offset from the start of the string.
fn cmp_drop(libs: &Libs, input: &[u8]) {
    let cstr = CString::new(input.to_vec())
        .unwrap_or_else(|_| panic!("input contains null byte"));
    let ptr = cstr.as_ptr();
    let c_res = unsafe { (libs.c_drop)(ptr) };
    let rs_res = unsafe { (libs.rs_drop)(ptr) };
    let c_off = unsafe { c_res.offset_from(ptr) };
    let rs_off = unsafe { rs_res.offset_from(ptr) };
    assert_eq!(
        c_off, rs_off,
        "w_utf8_drop offset mismatch for {:?}: C={}, Rust={}",
        input, c_off, rs_off
    );
}

/// Compare w_utf8_filter: both should return byte-identical strings.
fn cmp_filter(libs: &Libs, input: &[u8], replacement: bool) {
    let cstr = CString::new(input.to_vec())
        .unwrap_or_else(|_| panic!("input contains null byte"));
    let ptr = cstr.as_ptr();
    let c_res = unsafe { (libs.c_filter)(ptr, replacement) };
    let rs_res = unsafe { (libs.rs_filter)(ptr, replacement) };
    assert!(!c_res.is_null(), "C returned null for {:?}", input);
    assert!(!rs_res.is_null(), "Rust returned null for {:?}", input);
    let c_bytes = unsafe { CStr::from_ptr(c_res) }.to_bytes();
    let rs_bytes = unsafe { CStr::from_ptr(rs_res) }.to_bytes();
    assert_eq!(
        c_bytes, rs_bytes,
        "w_utf8_filter mismatch for input={:?} replacement={}: C={:?} Rust={:?}",
        input, replacement, c_bytes, rs_bytes
    );
    // Free malloc'd memory
    unsafe {
        libc::free(c_res as *mut _);
        libc::free(rs_res as *mut _);
    }
}

#[test]
fn test_w_utf8_drop() {
    let libs = load();
    let cases: Vec<&[u8]> = vec![
        // Empty string
        b"",
        // Pure ASCII
        b"hello world",
        // Valid 2-byte
        b"\xc2\xa9",           // ©
        // Valid 3-byte
        b"\xe2\x9c\x93",      // ✓
        // Valid 4-byte
        b"\xf0\x9f\x98\x80",  // 😀
        // Mixed valid
        b"abc\xc3\xa9\xe2\x9c\x93\xf0\x9f\x98\x80xyz",
        // Invalid continuation byte immediately
        b"\x80",
        // Invalid start byte
        b"\xfe",
        b"\xff",
        // Overlong 2-byte (C0, C1 forbidden)
        b"\xc0\x80",
        b"\xc1\xbf",
        // Valid then invalid
        b"abc\xff",
        b"abc\xc3\xa9\xff",
        // Truncated multi-byte
        b"\xc2",              // missing continuation
        b"\xe2\x9c",          // missing 3rd byte
        b"\xf0\x9f\x98",      // missing 4th byte
        // Overlong 3-byte: E0 followed by < A0
        b"\xe0\x80\x80",
        b"\xe0\x9f\x80",
        // Surrogate range: ED A0-BF
        b"\xed\xa0\x80",
        b"\xed\xbf\xbf",
        // Overlong 4-byte: F0 followed by < 90
        b"\xf0\x80\x80\x80",
        b"\xf0\x8f\xbf\xbf",
        // Above U+10FFFF: F4 followed by > 8F
        b"\xf4\x90\x80\x80",
        // F5+ start bytes
        b"\xf5\x80\x80\x80",
        b"\xf8\x80\x80\x80\x80",
        // Invalid byte in the middle
        b"ab\xffcd",
        // Multiple invalid bytes
        b"\xff\xfe\xfd",
        // Valid boundary: U+007F (1-byte max)
        b"\x7f",
        // Valid boundary: U+0080 (2-byte min)
        b"\xc2\x80",
        // Valid boundary: U+07FF (2-byte max)
        b"\xdf\xbf",
        // Valid boundary: U+0800 (3-byte min)
        b"\xe0\xa0\x80",
        // Valid boundary: U+FFFF (3-byte max, excluding surrogates)
        b"\xef\xbf\xbf",
        // Valid boundary: U+10000 (4-byte min)
        b"\xf0\x90\x80\x80",
        // Valid boundary: U+10FFFF (4-byte max)
        b"\xf4\x8f\xbf\xbf",
        // Bad continuation after valid start
        b"\xc2\x00",  // won't work (null), skip
        b"\xc2\xff",
        b"\xe2\xff\x80",
        b"\xf0\x9f\xff\x80",
    ];
    for case in &cases {
        // Skip cases with internal null bytes
        if case.contains(&0u8) { continue; }
        cmp_drop(&libs, case);
    }
}

#[test]
fn test_w_utf8_filter_no_replacement() {
    let libs = load();
    let cases: Vec<&[u8]> = vec![
        b"",
        b"hello",
        b"\xc3\xa9",
        b"\xff",
        b"abc\xffdef",
        b"\xff\xfe\xfd",
        b"abc\xc3\xa9\xff\xe2\x9c\x93",
        b"\xc0\x80",
        b"\xed\xa0\x80",
        b"\xf0\x80\x80\x80",
        b"\xf4\x90\x80\x80",
        b"\xc2",
        b"\xe2\x9c",
        b"\xf0\x9f\x98",
        b"\xc2\xff",
        b"\xe0\x9f\x80",
    ];
    for case in &cases {
        if case.contains(&0u8) { continue; }
        cmp_filter(&libs, case, false);
    }
}

#[test]
fn test_w_utf8_filter_with_replacement() {
    let libs = load();
    let cases: Vec<&[u8]> = vec![
        b"",
        b"hello",
        b"\xc3\xa9",
        b"\xff",
        b"abc\xffdef",
        b"\xff\xfe\xfd",
        b"abc\xc3\xa9\xff\xe2\x9c\x93",
        b"\xc0\x80",
        b"\xed\xa0\x80",
        b"\xf0\x80\x80\x80",
        b"\xf4\x90\x80\x80",
        b"\xc2",
        b"\xe2\x9c",
        b"\xf0\x9f\x98",
        b"\xc2\xff",
        b"\xe0\x9f\x80",
    ];
    for case in &cases {
        if case.contains(&0u8) { continue; }
        cmp_filter(&libs, case, true);
    }
}

#[test]
fn test_many_replacements() {
    // Test with many invalid bytes to trigger realloc path
    let libs = load();
    let mut input = Vec::new();
    for _ in 0..5000 {
        input.push(0xFFu8);
    }
    cmp_filter(&libs, &input, true);
    cmp_filter(&libs, &input, false);
}

#[test]
fn test_alternating_valid_invalid() {
    let libs = load();
    let mut input = Vec::new();
    for _ in 0..500 {
        input.extend_from_slice(b"a");
        input.push(0xFF);
    }
    cmp_drop(&libs, &input);
    cmp_filter(&libs, &input, false);
    cmp_filter(&libs, &input, true);
}
