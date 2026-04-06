use libloading::{Library, Symbol};
use std::os::raw::c_int;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libarrayfunc_lib.so"
    );
    unsafe { Library::new(path).expect("Failed to load C library") }
}

// --- Lowest level: arithmetic operations ---

#[test]
fn test_add_operation() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"add_operation").unwrap() };

    let cases: &[(i32, i32)] = &[
        (0, 0), (1, 2), (-1, 1), (i32::MAX, 1), (i32::MIN, -1),
        (100, -200), (i32::MAX, i32::MAX), (i32::MIN, i32::MIN),
    ];
    for &(a, b) in cases {
        let c_result = unsafe { c_fn(a, b, 0, 0) };
        let rust_result = arrayfunc_lib::add_operation(a, b, 0, 0);
        assert_eq!(c_result, rust_result, "add_operation({a}, {b})");
    }
}

#[test]
fn test_multiply_operation() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"multiply_operation").unwrap() };

    let cases: &[(i32, i32)] = &[
        (0, 0), (1, 2), (-1, 1), (100, 200), (-100, 200),
        (i32::MAX, 2), (i32::MIN, 2),
    ];
    for &(a, b) in cases {
        let c_result = unsafe { c_fn(a, b, 0, 0) };
        let rust_result = arrayfunc_lib::multiply_operation(a, b, 0, 0);
        assert_eq!(c_result, rust_result, "multiply_operation({a}, {b})");
    }
}

#[test]
fn test_subtract_operation() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"subtract_operation").unwrap() };

    let cases: &[(i32, i32)] = &[
        (0, 0), (5, 3), (3, 5), (-1, -1), (i32::MIN, 1), (i32::MAX, -1),
    ];
    for &(a, b) in cases {
        let c_result = unsafe { c_fn(a, b, 0, 0) };
        let rust_result = arrayfunc_lib::subtract_operation(a, b, 0, 0);
        assert_eq!(c_result, rust_result, "subtract_operation({a}, {b})");
    }
}

#[test]
fn test_modulo_operation() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"modulo_operation").unwrap() };

    let cases: &[(i32, i32)] = &[
        (10, 3), (10, -3), (-10, 3), (-10, -3), (0, 5), (5, 0),
        (7, 1), (i32::MAX, 2), (i32::MIN, 2),
    ];
    for &(a, b) in cases {
        let c_result = unsafe { c_fn(a, b, 0, 0) };
        let rust_result = arrayfunc_lib::modulo_operation(a, b, 0, 0);
        assert_eq!(c_result, rust_result, "modulo_operation({a}, {b})");
    }
}

// --- safe_double_to_int ---

#[test]
fn test_safe_double_to_int() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> =
        unsafe { lib.get(b"safe_double_to_int").unwrap() };

    let cases: &[f64] = &[
        0.0, 1.0, -1.0, 1.5, -1.5, 100.7, -100.7,
        i32::MAX as f64, i32::MIN as f64,
        i32::MAX as f64 + 1.0, i32::MIN as f64 - 1.0,
        f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
        2147483647.0, 2147483648.0, -2147483648.0, -2147483649.0,
        0.999, -0.999, 0.5, -0.5,
    ];
    for &d in cases {
        let c_result = unsafe { c_fn(d) };
        let rust_result = arrayfunc_lib::safe_double_to_int(d);
        assert_eq!(c_result, rust_result, "safe_double_to_int({d})");
    }
}

// --- compute_scaled_value ---

#[test]
fn test_compute_scaled_value() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, f64) -> c_int> =
        unsafe { lib.get(b"compute_scaled_value").unwrap() };

    let cases: &[(i32, f64)] = &[
        (0, 1.0), (10, 2.0), (-10, 2.0), (10, -2.0),
        (100, 0.5), (i32::MAX, 1.0), (i32::MIN, 1.0),
        (i32::MAX, 2.0), (1000, 0.001),
    ];
    for &(base, scale) in cases {
        let c_result = unsafe { c_fn(base, scale) };
        let rust_result = arrayfunc_lib::compute_scaled_value(base, scale);
        assert_eq!(c_result, rust_result, "compute_scaled_value({base}, {scale})");
    }
}

// --- Top-level arrayfunc ---

#[test]
fn test_arrayfunc() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"arrayfunc").unwrap() };

    let cases: &[(i32, i32, i32, i32)] = &[
        (1, 2, 3, 4),
        (0, 0, 0, 0),
        (10, 20, 30, 40),
        (-1, -2, -3, -4),
        (100, 200, 300, 400),
        (1, 0, 0, 0),
        (0, 1, 0, 0),
        (0, 0, 1, 0),
        (0, 0, 0, 1),
        (i32::MAX, 0, 0, 0),
        (0, i32::MAX, 0, 0),
        (0, 0, i32::MAX, 0),
        (0, 0, 0, i32::MAX),
        (i32::MIN, 0, 0, 0),
        (-100, 50, -25, 12),
        (1000, -500, 250, -125),
        (7, 13, 19, 23),
    ];
    for &(a, b, c, d) in cases {
        let c_result = unsafe { c_fn(a, b, c, d) };
        let rust_result = arrayfunc_lib::arrayfunc(a, b, c, d);
        assert_eq!(c_result, rust_result, "arrayfunc({a}, {b}, {c}, {d})");
    }
}
