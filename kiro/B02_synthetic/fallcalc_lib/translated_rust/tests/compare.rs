use libloading::{Library, Symbol};
use std::os::raw::c_int;

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

#[test]
fn test_safe_double_to_int() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> =
        unsafe { lib.get(b"safe_double_to_int").unwrap() };

    let cases: &[f64] = &[
        0.0, 1.0, -1.0, 3.7, -3.7, 100.9, -100.9,
        f64::NAN, f64::INFINITY, f64::NEG_INFINITY,
        c_int::MAX as f64, c_int::MIN as f64,
        c_int::MAX as f64 + 1.0, c_int::MIN as f64 - 1.0,
        0.5, -0.5, 999999.999, -999999.999,
    ];
    for &d in cases {
        let c_r = unsafe { c_fn(d) };
        let r_r = fallcalc_lib::safe_double_to_int(d);
        assert_eq!(c_r, r_r, "safe_double_to_int({d})");
    }
}

#[test]
fn test_process_array_reverse() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const c_int, c_int) -> c_int> =
        unsafe { lib.get(b"process_array_reverse").unwrap() };

    let arrays: &[&[c_int]] = &[
        &[1, 2, 3, 4, 5],
        &[10, 20, 30],
        &[0],
        &[-1, -2, -3],
        &[8, 16, 24, 32, 40],
    ];
    for arr in arrays {
        let count = arr.len() as c_int;
        let end_ptr = unsafe { arr.as_ptr().add(arr.len() - 1) };
        let c_r = unsafe { c_fn(end_ptr, count) };
        let r_r = fallcalc_lib::process_array_reverse(end_ptr, count);
        assert_eq!(c_r, r_r, "process_array_reverse({arr:?})");
    }
}

#[test]
fn test_switch_fallthrough_calculator() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
        unsafe { lib.get(b"switch_fallthrough_calculator").unwrap() };

    let values = [0, 1, 5, 10, 50, 100, 127, 128, 255, 511, 512, -1, -10];
    for &val in &values {
        for op in 0..=6 {
            let c_r = unsafe { c_fn(val, op) };
            let r_r = fallcalc_lib::switch_fallthrough_calculator(val, op);
            assert_eq!(c_r, r_r, "switch_fallthrough_calculator({val}, {op})");
        }
    }
}

#[test]
fn test_allocate_and_compute() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, f64) -> c_int> =
        unsafe { lib.get(b"allocate_and_compute").unwrap() };

    let cases: &[(c_int, f64)] = &[
        (1, 1.5), (2, 1.5), (5, 1.5), (10, 1.5),
        (1, 0.0), (3, 2.0), (5, -1.0), (0, 1.5),
        (-1, 1.5), (7, 3.14),
    ];
    for &(size, mult) in cases {
        let c_r = unsafe { c_fn(size, mult) };
        let r_r = fallcalc_lib::allocate_and_compute(size, mult);
        assert_eq!(c_r, r_r, "allocate_and_compute({size}, {mult})");
    }
}

#[test]
fn test_foreach_sum() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(*const c_int, c_int) -> c_int> =
        unsafe { lib.get(b"foreach_sum").unwrap() };

    let arrays: &[&[c_int]] = &[
        &[1, 2, 3, 4, 5],
        &[10, 20, 30],
        &[0],
        &[-1, -2, -3],
        &[8, 16, 24, 32, 40],
    ];
    for arr in arrays {
        let count = arr.len() as c_int;
        let c_r = unsafe { c_fn(arr.as_ptr(), count) };
        let r_r = fallcalc_lib::foreach_sum(arr.as_ptr(), count);
        assert_eq!(c_r, r_r, "foreach_sum({arr:?})");
    }
}

#[test]
fn test_fallcalc() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
        unsafe { lib.get(b"fallcalc").unwrap() };

    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (10, 20, 0, 5),
        (10, 20, 1, 5),
        (10, 20, 2, 5),
        (10, 20, 3, 5),
        (10, 20, 4, 5),
        (5, 10, 129, 3),
        (5, 10, 127, 3),
        (100, 200, 300, 9),
        (-1, -2, -3, -4),
        (0, 0, 0, 9),
        (1, 1, 1, 1),
        (50, 100, 200, 7),
    ];
    for &(a, b, c, d) in cases {
        let c_r = unsafe { c_fn(a, b, c, d) };
        let r_r = fallcalc_lib::fallcalc(a, b, c, d);
        assert_eq!(c_r, r_r, "fallcalc({a}, {b}, {c}, {d})");
    }
}
