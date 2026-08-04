use libloading::{Library, Symbol};
use std::ffi::{c_char, CString};

const C_SO_PATH: &str = "c_src/build/libdriver.so";
const RUST_SO_PATH: &str = "target/debug/libdriver.so";

type CustomStrdupFn = unsafe extern "C" fn(*const c_char) -> *mut c_char;
type FreeFn = unsafe extern "C" fn(*mut std::ffi::c_void);

unsafe fn load_libs() -> (Library, Library) {
    let c_lib = Library::new(C_SO_PATH).expect("failed to load C lib");
    let rust_lib = Library::new(RUST_SO_PATH).expect("failed to load Rust lib");
    (c_lib, rust_lib)
}

unsafe fn libc_lib() -> Library {
    Library::new("libc.so.6").expect("failed to load libc")
}

fn cstr_to_bytes(p: *const c_char) -> Option<Vec<u8>> {
    if p.is_null() {
        return None;
    }
    unsafe {
        let mut bytes = Vec::new();
        let mut i = 0isize;
        loop {
            let b = *p.offset(i) as u8;
            bytes.push(b);
            if b == 0 {
                break;
            }
            i += 1;
        }
        Some(bytes)
    }
}

#[test]
fn test_custom_strdup_basic() {
    unsafe {
        let (c_lib, rust_lib) = load_libs();
        let libc = libc_lib();

        let c_fn: Symbol<CustomStrdupFn> = c_lib.get(b"custom_strdup").unwrap();
        let r_fn: Symbol<CustomStrdupFn> = rust_lib.get(b"custom_strdup").unwrap();
        let free: Symbol<FreeFn> = libc.get(b"free").unwrap();

        let test_strings: Vec<&[u8]> = vec![
            b"",
            b"a",
            b"hello",
            b"hello world",
            b"this is a longer test string with various characters!@#$%^&*()",
            b"\x01\x02\x03 some bytes",
        ];

        for s in test_strings {
            let cstr = CString::new(s).unwrap();
            let p = cstr.as_ptr();

            let c_result = c_fn(p);
            let r_result = r_fn(p);

            let c_bytes = cstr_to_bytes(c_result);
            let r_bytes = cstr_to_bytes(r_result);

            assert_eq!(
                c_bytes, r_bytes,
                "Mismatch for input {:?}",
                std::str::from_utf8(s).unwrap_or("(non-utf8)")
            );

            assert!(!c_result.is_null());
            assert!(!r_result.is_null());

            // The pointers must be different (different malloc allocations)
            assert_ne!(c_result, r_result);

            free(c_result as *mut std::ffi::c_void);
            free(r_result as *mut std::ffi::c_void);
        }
    }
}

#[test]
fn test_custom_strdup_null() {
    unsafe {
        let (c_lib, rust_lib) = load_libs();

        let c_fn: Symbol<CustomStrdupFn> = c_lib.get(b"custom_strdup").unwrap();
        let r_fn: Symbol<CustomStrdupFn> = rust_lib.get(b"custom_strdup").unwrap();

        let c_result = c_fn(std::ptr::null());
        let r_result = r_fn(std::ptr::null());

        assert!(c_result.is_null());
        assert!(r_result.is_null());
    }
}

#[test]
fn test_custom_strdup_long_string() {
    unsafe {
        let (c_lib, rust_lib) = load_libs();
        let libc = libc_lib();

        let c_fn: Symbol<CustomStrdupFn> = c_lib.get(b"custom_strdup").unwrap();
        let r_fn: Symbol<CustomStrdupFn> = rust_lib.get(b"custom_strdup").unwrap();
        let free: Symbol<FreeFn> = libc.get(b"free").unwrap();

        let s = "x".repeat(10000);
        let cstr = CString::new(s.as_bytes()).unwrap();
        let p = cstr.as_ptr();

        let c_result = c_fn(p);
        let r_result = r_fn(p);

        let c_bytes = cstr_to_bytes(c_result);
        let r_bytes = cstr_to_bytes(r_result);

        assert_eq!(c_bytes, r_bytes);
        assert_eq!(c_bytes.as_ref().unwrap().len(), s.len() + 1);

        free(c_result as *mut std::ffi::c_void);
        free(r_result as *mut std::ffi::c_void);
    }
}
