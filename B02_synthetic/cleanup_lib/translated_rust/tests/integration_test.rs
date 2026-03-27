use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_char;
use std::ptr;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");

fn load_c_lib() -> Library {
    unsafe { Library::new(C_LIB_PATH).expect("Failed to load C .so") }
}

// Test cleanup_resources with null (should not crash)
#[test]
fn test_cleanup_resources_null() {
    let c_lib = load_c_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut c_char)> =
            c_lib.get(b"cleanup_resources").unwrap();
        c_fn(ptr::null_mut());
    }
    unsafe { cleanup_lib::cleanup_resources(ptr::null_mut()) };
}

// Test cleanup return value: C vs Rust must match
#[test]
fn test_cleanup_basic() {
    let c_lib = load_c_lib();
    let cases: Vec<(c_int, c_int, c_int, c_int)> = vec![
        (1, 2, 3, 4),
        (10, 20, 30, 40),
        (10, 10, 10, 10),
        (20, 20, 20, 20),
        (30, 30, 30, 30),
        (40, 40, 40, 40),
        (0, 0, 0, 0),
        (10, 30, 20, 40),
        (100, 200, 300, 400),
        (-1, -2, -3, -4),
        (10, 40, 30, 20),
    ];
    unsafe {
        let c_cleanup: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"cleanup").unwrap();
        for (a, b, c, d) in cases {
            let c_result = c_cleanup(a, b, c, d);
            let rust_result = cleanup_lib::cleanup(a, b, c, d);
            assert_eq!(
                c_result, rust_result,
                "cleanup({a},{b},{c},{d}): C={c_result}, Rust={rust_result}"
            );
        }
    }
}
