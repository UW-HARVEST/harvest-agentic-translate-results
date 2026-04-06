use libloading::{Library, Symbol};
use std::ffi::{c_char, CStr, CString};

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libdriver.so");

struct CLib {
    _lib: Library,
    w_utf8_drop: Symbol<'static, unsafe extern "C" fn(*const c_char) -> *const c_char>,
    w_utf8_filter: Symbol<'static, unsafe extern "C" fn(*const c_char, bool) -> *mut c_char>,
}

impl CLib {
    fn load() -> Self {
        unsafe {
            let lib = Library::new(C_LIB).expect("Failed to load C library");
            let lib = Box::leak(Box::new(lib));
            let w_utf8_drop = lib.get(b"w_utf8_drop").unwrap();
            let w_utf8_filter = lib.get(b"w_utf8_filter").unwrap();
            CLib {
                _lib: std::ptr::read(lib as *const Library),
                w_utf8_drop,
                w_utf8_filter,
            }
        }
    }
}

// Test inputs: valid UTF-8, invalid bytes, edge cases
fn test_inputs() -> Vec<Vec<u8>> {
    vec![
        // Valid ASCII
        b"hello world".to_vec(),
        b"".to_vec(),
        b"a".to_vec(),
        // Valid multi-byte
        b"\xc3\xa9".to_vec(),                     // é (U+00E9)
        b"\xe4\xb8\xad".to_vec(),                 // 中 (U+4E2D)
        b"\xf0\x9f\x98\x80".to_vec(),             // 😀 (U+1F600)
        // Mixed valid
        b"hello \xc3\xa9 world".to_vec(),
        b"\xe4\xb8\xad\xf0\x9f\x98\x80".to_vec(),
        // Invalid: bare continuation byte
        b"\x80".to_vec(),
        b"abc\x80def".to_vec(),
        // Invalid: overlong 2-byte
        b"\xc0\x80".to_vec(),
        b"\xc1\xbf".to_vec(),
        // Invalid: overlong 3-byte (0xE0 followed by < 0xA0)
        b"\xe0\x80\x80".to_vec(),
        b"\xe0\x9f\x80".to_vec(),
        // Invalid: surrogate (0xED followed by >= 0xA0)
        b"\xed\xa0\x80".to_vec(),
        b"\xed\xbf\xbf".to_vec(),
        // Invalid: overlong 4-byte (0xF0 followed by < 0x90)
        b"\xf0\x80\x80\x80".to_vec(),
        b"\xf0\x8f\x80\x80".to_vec(),
        // Invalid: > U+10FFFF (0xF4 followed by > 0x8F)
        b"\xf4\x90\x80\x80".to_vec(),
        // Invalid: start byte >= 0xF5
        b"\xf5\x80\x80\x80".to_vec(),
        b"\xff".to_vec(),
        b"\xfe".to_vec(),
        // Truncated sequences
        b"\xc3".to_vec(),
        b"\xe4\xb8".to_vec(),
        b"\xf0\x9f\x98".to_vec(),
        // Valid 3-byte starting with 0xEF (BOM, replacement char area)
        b"\xef\xbb\xbf".to_vec(),                 // BOM U+FEFF
        b"\xef\xbf\xbd".to_vec(),                 // U+FFFD replacement char
        b"\xef\xbf\xbf".to_vec(),                 // U+FFFF
        // Mixed valid and invalid
        b"abc\xff\xfe\x80xyz".to_vec(),
        b"\xc3\xa9\xff\xe4\xb8\xad".to_vec(),
        b"\xf0\x9f\x98\x80\x80\xc3\xa9".to_vec(),
        // Multiple consecutive invalid bytes
        b"\x80\x81\x82\x83".to_vec(),
        b"\xff\xff\xff".to_vec(),
        // All valid boundary cases
        b"\x7f".to_vec(),                          // max 1-byte
        b"\xc2\x80".to_vec(),                      // min valid 2-byte
        b"\xdf\xbf".to_vec(),                      // max 2-byte
        b"\xe0\xa0\x80".to_vec(),                  // min valid 3-byte
        b"\xef\xbf\xbf".to_vec(),                  // max 3-byte
        b"\xf0\x90\x80\x80".to_vec(),              // min valid 4-byte
        b"\xf4\x8f\xbf\xbf".to_vec(),              // max valid 4-byte (U+10FFFF)
    ]
}

fn to_cstring(bytes: &[u8]) -> CString {
    // Ensure no interior NULs - the C functions expect NUL-terminated strings
    let filtered: Vec<u8> = bytes.iter().copied().filter(|&b| b != 0).collect();
    CString::new(filtered).unwrap()
}

#[test]
fn test_w_utf8_drop() {
    let c = CLib::load();
    for input in test_inputs() {
        let cs = to_cstring(&input);
        let ptr = cs.as_ptr();
        unsafe {
            let c_result = (c.w_utf8_drop)(ptr);
            let rust_result = driver::w_utf8_drop(ptr);
            let c_offset = c_result.offset_from(ptr);
            let rust_offset = rust_result.offset_from(ptr);
            assert_eq!(
                c_offset, rust_offset,
                "w_utf8_drop mismatch for input {:?}: C offset={}, Rust offset={}",
                input, c_offset, rust_offset
            );
        }
    }
}

#[test]
fn test_w_utf8_filter_no_replacement() {
    let c = CLib::load();
    for input in test_inputs() {
        let cs = to_cstring(&input);
        unsafe {
            let c_result = (c.w_utf8_filter)(cs.as_ptr(), false);
            let rust_result = driver::w_utf8_filter(cs.as_ptr(), false);
            assert!(!c_result.is_null());
            assert!(!rust_result.is_null());
            let c_str = CStr::from_ptr(c_result);
            let rust_str = CStr::from_ptr(rust_result);
            assert_eq!(
                c_str.to_bytes(),
                rust_str.to_bytes(),
                "w_utf8_filter(false) mismatch for input {:?}:\n  C:    {:?}\n  Rust: {:?}",
                input,
                c_str.to_bytes(),
                rust_str.to_bytes()
            );
            libc::free(c_result as *mut _);
            libc::free(rust_result as *mut _);
        }
    }
}

#[test]
fn test_w_utf8_filter_with_replacement() {
    let c = CLib::load();
    for input in test_inputs() {
        let cs = to_cstring(&input);
        unsafe {
            let c_result = (c.w_utf8_filter)(cs.as_ptr(), true);
            let rust_result = driver::w_utf8_filter(cs.as_ptr(), true);
            assert!(!c_result.is_null());
            assert!(!rust_result.is_null());
            let c_str = CStr::from_ptr(c_result);
            let rust_str = CStr::from_ptr(rust_result);
            assert_eq!(
                c_str.to_bytes(),
                rust_str.to_bytes(),
                "w_utf8_filter(true) mismatch for input {:?}:\n  C:    {:?}\n  Rust: {:?}",
                input,
                c_str.to_bytes(),
                rust_str.to_bytes()
            );
            libc::free(c_result as *mut _);
            libc::free(rust_result as *mut _);
        }
    }
}
