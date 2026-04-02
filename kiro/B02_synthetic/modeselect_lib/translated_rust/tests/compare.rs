use libloading::{Library, Symbol};
use std::ffi::CString;

const C_LIB_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libmodeselect_lib.so");

fn c_lib() -> Library {
    unsafe { Library::new(C_LIB_PATH).expect("Failed to load C library") }
}

#[test]
fn test_classify_mode() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const i8) -> i32> =
        unsafe { lib.get(b"classify_mode").unwrap() };

    for input in &["standard", "enhanced", "turbo", "extreme", "unknown", ""] {
        let cs = CString::new(*input).unwrap();
        let c_result = unsafe { c_fn(cs.as_ptr()) };
        let rust_result = modeselect_lib::classify_mode(cs.as_ptr());
        assert_eq!(c_result, rust_result, "classify_mode(\"{input}\")");
    }
}

#[test]
fn test_apply_multiplier() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32, i32) -> i32> =
        unsafe { lib.get(b"apply_multiplier").unwrap() };

    for base in &[0, 0xA0, 0xFF, -1, i32::MAX] {
        for level in &[0, 1, 2, 3, 4, 5, -1, 99] {
            let c_r = unsafe { c_fn(*base, *level) };
            let rust_r = modeselect_lib::apply_multiplier(*base, *level);
            assert_eq!(c_r, rust_r, "apply_multiplier({base:#x}, {level})");
        }
    }
}

#[test]
fn test_convert_time_factor() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f64) -> i32> =
        unsafe { lib.get(b"convert_time_factor").unwrap() };

    for val in &[0.0, 1e-12, 1e-6, 0.5, 1.0, -1.0, 42.0, 1e8, -1e8, 1e20, -1e20] {
        let c_r = unsafe { c_fn(*val) };
        let rust_r = modeselect_lib::convert_time_factor(*val);
        assert_eq!(c_r, rust_r, "convert_time_factor({val})");
    }
}

#[test]
fn test_convert_negative_overflow() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f64) -> i32> =
        unsafe { lib.get(b"convert_negative_overflow").unwrap() };

    for val in &[0.0, 1.0, -1.0, 0.5, 1e-6, 1e8, -1e8, 1e20, -1e20, 42.0] {
        let c_r = unsafe { c_fn(*val) };
        let rust_r = modeselect_lib::convert_negative_overflow(*val);
        assert_eq!(c_r, rust_r, "convert_negative_overflow({val})");
    }
}

#[test]
fn test_hash_time_value() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i64) -> i32> =
        unsafe { lib.get(b"hash_time_value").unwrap() };

    for val in &[0i64, 1, -1, 100, 1000000, 0x7FFFFFFF, -100, i64::MAX, i64::MIN] {
        let c_r = unsafe { c_fn(*val) };
        let rust_r = modeselect_lib::hash_time_value(*val);
        assert_eq!(c_r, rust_r, "hash_time_value({val})");
    }
}

#[test]
fn test_get_modified_time() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32, i32) -> i64> =
        unsafe { lib.get(b"get_modified_time").unwrap() };

    for (days, hours) in &[(0, 0), (1, 1), (10, 5), (0, 23), (365, 12)] {
        // Call back-to-back to minimize time() drift
        let c_r = unsafe { c_fn(*days, *hours) };
        let rust_r = modeselect_lib::get_modified_time(*days, *hours);
        assert_eq!(c_r, rust_r, "get_modified_time({days}, {hours})");
    }
}

#[test]
fn test_modeselect() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32, i32, i32, i32) -> i32> =
        unsafe { lib.get(b"modeselect").unwrap() };

    let test_cases = vec![
        (0, 0, 0, 0),
        (1, 1, 1, 1),
        (2, 2, 2, 2),
        (3, 3, 3, 3),
        (0, 10, 4, 12),
        (1, 5, 3, 7),
        (2, 0, 2, 23),
        (3, 100, 0, 0),
    ];

    for (a, b, c, d) in &test_cases {
        let c_result = unsafe { c_fn(*a, *b, *c, *d) };
        let rust_result = modeselect_lib::modeselect(*a, *b, *c, *d);
        assert_eq!(
            c_result, rust_result,
            "modeselect({a}, {b}, {c}, {d}): C={c_result:#x} Rust={rust_result:#x}"
        );
    }
}
