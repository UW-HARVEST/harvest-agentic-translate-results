use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_char;

fn c_lib() -> Library {
    unsafe {
        Library::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("c_src/build/libtranslated_rust.so"),
        )
        .expect("Failed to load C .so")
    }
}

fn rust_lib() -> Library {
    unsafe {
        Library::new(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("target/debug/libdoubleneg_lib.so"),
        )
        .expect("Failed to load Rust .so")
    }
}

#[test]
fn test_process_negation() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c.get(b"process_negation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            r.get(b"process_negation").unwrap();
        for v in [0, 1, -1, 42, -100, i32::MAX, i32::MIN] {
            assert_eq!(c_fn(v), r_fn(v), "process_negation({v})");
        }
    }
}

#[test]
fn test_convert_double_to_int() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> =
            c.get(b"convert_double_to_int").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> =
            r.get(b"convert_double_to_int").unwrap();
        let vals = [
            0.0, 1.0, -1.0, 3.14, -3.14, 1e9, -1e9,
            2147483647.0, -2147483648.0,
            // UB territory — must match C's compiled behavior
            1e18, -1e18, f64::INFINITY, f64::NEG_INFINITY, f64::NAN,
            -(2.0_f64.powi(40)),
        ];
        for v in vals {
            assert_eq!(c_fn(v), r_fn(v), "convert_double_to_int({v})");
        }
    }
}

#[test]
fn test_create_numeric_buffer() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(*mut c_char, c_int, c_int)> =
            c.get(b"create_numeric_buffer").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut c_char, c_int, c_int)> =
            r.get(b"create_numeric_buffer").unwrap();
        for seed in [0, 1, -1, 42, 255, -128, 1000, i32::MAX, i32::MIN] {
            let mut c_buf = [0i8; 256];
            let mut r_buf = [0i8; 256];
            c_fn(c_buf.as_mut_ptr(), 256, seed);
            r_fn(r_buf.as_mut_ptr(), 256, seed);
            assert_eq!(c_buf, r_buf, "create_numeric_buffer(256, {seed})");
        }
    }
}

#[test]
fn test_find_value_in_buffer() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_create: Symbol<unsafe extern "C" fn(*mut c_char, c_int, c_int)> =
            c.get(b"create_numeric_buffer").unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(*const c_char, libc::size_t, c_int) -> c_int> =
            c.get(b"find_value_in_buffer").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*const c_char, libc::size_t, c_int) -> c_int> =
            r.get(b"find_value_in_buffer").unwrap();

        // Use C to create the buffer (ground truth), then search with both
        let mut buf = [0i8; 256];
        c_create(buf.as_mut_ptr(), 256, 5);
        for search in [0, 1, 42, 100, 127, -128, 255, -1, 200] {
            let cv = c_fn(buf.as_ptr(), 256, search);
            let rv = r_fn(buf.as_ptr(), 256, search);
            assert_eq!(cv, rv, "find_value_in_buffer(search={search})");
        }
    }
}

#[test]
fn test_calculate_with_doubles() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> f64> =
            c.get(b"calculate_with_doubles").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> f64> =
            r.get(b"calculate_with_doubles").unwrap();
        let cases = [
            (10, 3, 2),
            (0, 1, 0),
            (1, 0, 5),   // b==0 path
            (-7, 2, 3),
            (100, 7, 9),
            (1, 1, 0),
            (i32::MAX, 1, 1),
            (1, i32::MAX, 5),
        ];
        for (a, b, cc) in cases {
            let cv = c_fn(a, b, cc);
            let rv = r_fn(a, b, cc);
            assert_eq!(cv.to_bits(), rv.to_bits(), "calculate_with_doubles({a},{b},{cc}): C={cv} Rust={rv}");
        }
    }
}

#[test]
fn test_doubleneg() {
    let c = c_lib();
    let r = rust_lib();
    unsafe {
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c.get(b"doubleneg").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            r.get(b"doubleneg").unwrap();
        let cases = [
            (1, 2, 3, 4),
            (0, 0, 0, 0),
            (10, 3, 2, 1),
            (-5, 7, -3, 100),
            (42, 1, 0, 255),
            (100, 50, 25, 12),
        ];
        for (a, b, cc, d) in cases {
            let cv = c_fn(a, b, cc, d);
            let rv = r_fn(a, b, cc, d);
            assert_eq!(cv, rv, "doubleneg({a},{b},{cc},{d})");
        }
    }
}
