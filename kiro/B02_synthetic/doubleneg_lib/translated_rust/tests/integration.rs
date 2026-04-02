use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::os::raw::c_char;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libtranslated_rust.so"
    );
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

// --- convert_double_to_int ---
#[test]
fn test_convert_double_to_int() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> =
        unsafe { lib.get(b"convert_double_to_int").unwrap() };

    let cases: &[f64] = &[0.0, 1.5, -1.5, 42.9, -42.9, 100.0, -100.0, 2147483647.0, -2147483648.0];
    for &v in cases {
        let c_res = unsafe { c_fn(v) };
        let r_res = doubleneg_lib::convert_double_to_int(v);
        assert_eq!(c_res, r_res, "convert_double_to_int({v}): C={c_res}, Rust={r_res}");
    }
}

// --- find_value_in_buffer ---
#[test]
fn test_find_value_in_buffer() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const c_char, usize, c_int) -> c_int> =
        unsafe { lib.get(b"find_value_in_buffer").unwrap() };

    let buf: [c_char; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
    let searches: &[c_int] = &[10, 30, 80, 99, 0];
    for &s in searches {
        let c_res = unsafe { c_fn(buf.as_ptr(), 8, s) };
        let r_res = doubleneg_lib::find_value_in_buffer(buf.as_ptr(), 8, s);
        assert_eq!(c_res, r_res, "find_value_in_buffer(search={s}): C={c_res}, Rust={r_res}");
    }
}

// --- process_negation ---
#[test]
fn test_process_negation() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
        unsafe { lib.get(b"process_negation").unwrap() };

    for v in [-100, -1, 0, 1, 100, i32::MIN, i32::MAX] {
        let c_res = unsafe { c_fn(v) };
        let r_res = doubleneg_lib::process_negation(v);
        assert_eq!(c_res, r_res, "process_negation({v}): C={c_res}, Rust={r_res}");
    }
}

// --- create_numeric_buffer ---
#[test]
fn test_create_numeric_buffer() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*mut c_char, c_int, c_int)> =
        unsafe { lib.get(b"create_numeric_buffer").unwrap() };

    for seed in [0, 1, 42, 255, -1, 100] {
        let mut c_buf = [0i8; 256];
        let mut r_buf = [0i8; 256];
        unsafe { c_fn(c_buf.as_mut_ptr(), 256, seed) };
        doubleneg_lib::create_numeric_buffer(r_buf.as_mut_ptr(), 256, seed);
        assert_eq!(c_buf, r_buf, "create_numeric_buffer(seed={seed}) mismatch");
    }
}

// --- calculate_with_doubles ---
#[test]
fn test_calculate_with_doubles() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> f64> =
        unsafe { lib.get(b"calculate_with_doubles").unwrap() };

    let cases: &[(c_int, c_int, c_int)] = &[
        (10, 3, 2), (0, 1, 0), (100, 7, 5), (-10, 3, 1), (1, 1, 0),
        (42, 0, 3), (1, -1, 2), (0, 0, 0),
    ];
    for &(a, b, c) in cases {
        let c_res = unsafe { c_fn(a, b, c) };
        let r_res = doubleneg_lib::calculate_with_doubles(a, b, c);
        assert!(
            (c_res.is_nan() && r_res.is_nan()) || c_res == r_res,
            "calculate_with_doubles({a},{b},{c}): C={c_res}, Rust={r_res}"
        );
    }
}

// --- doubleneg (top-level) ---
#[test]
fn test_doubleneg() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"doubleneg").unwrap() };

    // Use inputs that avoid UB in convert_double_to_int
    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        (1, 2, 3, 4),
        (0, 0, 0, 0),
        (10, 5, 2, 7),
        (100, 3, 1, 50),
        (-1, 1, 1, 1),
    ];
    for &(a, b, c, d) in cases {
        let c_res = unsafe { c_fn(a, b, c, d) };
        let r_res = doubleneg_lib::doubleneg(a, b, c, d);
        assert_eq!(c_res, r_res, "doubleneg({a},{b},{c},{d}): C={c_res}, Rust={r_res}");
    }
}
