use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/override/libinreftree_lib.so")
}

fn load_c_lib() -> Library {
    unsafe { Library::new(c_lib_path()).expect("Failed to load C .so") }
}

// --- Low-level: stateless arithmetic ops ---

#[test]
fn test_add_op() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32, i32, i32, i32) -> i32> =
        unsafe { lib.get(b"add_op").unwrap() };
    for &(a, b) in &[(1, 2), (0, 0), (-5, 3), (i32::MAX, 1), (i32::MIN, -1)] {
        let c_res = unsafe { c_fn(a, b, 0, 0) };
        assert_eq!(c_res, a.wrapping_add(b), "add_op({a}, {b})");
    }
}

#[test]
fn test_multiply_op() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32, i32, i32, i32) -> i32> =
        unsafe { lib.get(b"multiply_op").unwrap() };
    for &(a, b) in &[(2, 3), (0, 5), (-2, 4), (100000, 100000)] {
        let c_res = unsafe { c_fn(a, b, 0, 0) };
        assert_eq!(c_res, a.wrapping_mul(b), "multiply_op({a}, {b})");
    }
}

#[test]
fn test_subtract_op() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32, i32, i32, i32) -> i32> =
        unsafe { lib.get(b"subtract_op").unwrap() };
    for &(a, b) in &[(5, 3), (0, 0), (-1, -1), (i32::MIN, 1)] {
        let c_res = unsafe { c_fn(a, b, 0, 0) };
        assert_eq!(c_res, a.wrapping_sub(b), "subtract_op({a}, {b})");
    }
}

#[test]
fn test_divide_op() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32, i32, i32, i32) -> i32> =
        unsafe { lib.get(b"divide_op").unwrap() };
    for &(a, b, expected) in &[(10, 3, 3), (7, 2, 3), (0, 5, 0), (5, 0, 0), (-7, 2, -3)] {
        let c_res = unsafe { c_fn(a, b, 0, 0) };
        assert_eq!(c_res, expected, "divide_op({a}, {b})");
    }
}

#[test]
fn test_modulo_op() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32, i32, i32, i32) -> i32> =
        unsafe { lib.get(b"modulo_op").unwrap() };
    for &(a, b, expected) in &[(10, 3, 1), (7, 2, 1), (0, 5, 0), (5, 0, 0), (-7, 2, -1)] {
        let c_res = unsafe { c_fn(a, b, 0, 0) };
        assert_eq!(c_res, expected, "modulo_op({a}, {b})");
    }
}

// --- Top-level: inreftree (resets global state each call) ---

#[test]
fn test_inreftree_matches_c() {
    let lib = load_c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(i32, i32, i32, i32) -> i32> =
        unsafe { lib.get(b"inreftree").unwrap() };

    let test_cases: &[(i32, i32, i32, i32)] = &[
        (1, 2, 3, 4),
        (0, 0, 0, 0),
        (10, 20, 30, 40),
        (-1, -2, -3, -4),
        (100, 200, 300, 400),
        (1, 0, 0, 0),
        (0, 1, 0, 0),
        (5, 5, 5, 5),
        (i32::MAX, 0, 0, 0),
        (0, 0, 0, i32::MIN),
        (7, 3, 11, 2),
        (-10, 5, -3, 8),
    ];

    for &(a, b, c, d) in test_cases {
        let c_res = unsafe { c_fn(a, b, c, d) };
        let rust_res = inreftree_lib::inreftree(a, b, c, d);
        assert_eq!(
            c_res, rust_res,
            "inreftree({a}, {b}, {c}, {d}): C={c_res}, Rust={rust_res}"
        );
    }
}
