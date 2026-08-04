use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // cargo test builds in deps dir, but the cdylib is in the target dir
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target/debug/libarrayfunc_lib.so");
    p
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Result_ {
    value: c_int,
    scaled: f64,
    rank: c_int,
}

#[repr(C)]
#[derive(Debug)]
struct ResultArray {
    data: [Result_; 10],
    count: c_int,
}

impl ResultArray {
    fn zeroed() -> Self {
        Self {
            data: [Result_ { value: 0, scaled: 0.0, rank: 0 }; 10],
            count: 0,
        }
    }
}

fn assert_result_arrays_eq(c: &ResultArray, r: &ResultArray, ctx: &str) {
    assert_eq!(c.count, r.count, "{ctx}: count mismatch");
    for i in 0..c.count as usize {
        assert_eq!(c.data[i].value, r.data[i].value, "{ctx}: data[{i}].value mismatch");
        assert_eq!(c.data[i].scaled.to_bits(), r.data[i].scaled.to_bits(), "{ctx}: data[{i}].scaled mismatch");
        assert_eq!(c.data[i].rank, r.data[i].rank, "{ctx}: data[{i}].rank mismatch");
    }
}

// ---- Lowest-level functions ----

#[test]
fn test_add_operation() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = c_lib.get(b"add_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = r_lib.get(b"add_operation").unwrap();
        for &(a, b) in &[(1, 2), (0, 0), (-5, 3), (i32::MAX, 1), (i32::MIN, -1)] {
            assert_eq!(c_fn(a, b, 0, 0), r_fn(a, b, 0, 0), "add_operation({a}, {b})");
        }
    }
}

#[test]
fn test_multiply_operation() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = c_lib.get(b"multiply_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = r_lib.get(b"multiply_operation").unwrap();
        for &(a, b) in &[(3, 4), (0, 5), (-2, 3), (i32::MAX, 2)] {
            assert_eq!(c_fn(a, b, 0, 0), r_fn(a, b, 0, 0), "multiply_operation({a}, {b})");
        }
    }
}

#[test]
fn test_subtract_operation() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = c_lib.get(b"subtract_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = r_lib.get(b"subtract_operation").unwrap();
        for &(a, b) in &[(10, 3), (0, 0), (-5, -3), (i32::MIN, 1)] {
            assert_eq!(c_fn(a, b, 0, 0), r_fn(a, b, 0, 0), "subtract_operation({a}, {b})");
        }
    }
}

#[test]
fn test_modulo_operation() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = c_lib.get(b"modulo_operation").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = r_lib.get(b"modulo_operation").unwrap();
        for &(a, b) in &[(10, 3), (7, 0), (-10, 3), (10, -3)] {
            assert_eq!(c_fn(a, b, 0, 0), r_fn(a, b, 0, 0), "modulo_operation({a}, {b})");
        }
    }
}

#[test]
fn test_safe_double_to_int() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> = c_lib.get(b"safe_double_to_int").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(f64) -> c_int> = r_lib.get(b"safe_double_to_int").unwrap();
        for &d in &[0.0, 1.5, -1.5, 3e9, -3e9, f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 2147483647.0, -2147483648.0, 2147483646.5] {
            assert_eq!(c_fn(d), r_fn(d), "safe_double_to_int({d})");
        }
    }
}

#[test]
fn test_compute_scaled_value() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, f64) -> c_int> = c_lib.get(b"compute_scaled_value").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, f64) -> c_int> = r_lib.get(b"compute_scaled_value").unwrap();
        for &(base, scale) in &[(10, 2.5), (0, 1.0), (-5, 3.0), (i32::MAX, 2.0), (100, 0.0)] {
            assert_eq!(c_fn(base, scale), r_fn(base, scale), "compute_scaled_value({base}, {scale})");
        }
    }
}

// ---- Struct-level functions ----

#[test]
fn test_init_result_array() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(*mut ResultArray, *const c_int, c_int)> = c_lib.get(b"init_result_array").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut ResultArray, *const c_int, c_int)> = r_lib.get(b"init_result_array").unwrap();

        let values: [c_int; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
        let mut c_arr = ResultArray::zeroed();
        let mut r_arr = ResultArray::zeroed();
        c_fn(&mut c_arr, values.as_ptr(), 8);
        r_fn(&mut r_arr, values.as_ptr(), 8);
        assert_result_arrays_eq(&c_arr, &r_arr, "init_result_array(8)");

        // Test clamping to 10
        let values12: [c_int; 12] = [1,2,3,4,5,6,7,8,9,10,11,12];
        let mut c_arr2 = ResultArray::zeroed();
        let mut r_arr2 = ResultArray::zeroed();
        c_fn(&mut c_arr2, values12.as_ptr(), 12);
        r_fn(&mut r_arr2, values12.as_ptr(), 12);
        assert_result_arrays_eq(&c_arr2, &r_arr2, "init_result_array(12)");
    }
}

#[test]
fn test_compare_results_in_array() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_init: Symbol<unsafe extern "C" fn(*mut ResultArray, *const c_int, c_int)> = c_lib.get(b"init_result_array").unwrap();
        let r_init: Symbol<unsafe extern "C" fn(*mut ResultArray, *const c_int, c_int)> = r_lib.get(b"init_result_array").unwrap();
        let c_cmp: Symbol<unsafe extern "C" fn(*const ResultArray, c_int, c_int) -> c_int> = c_lib.get(b"compare_results_in_array").unwrap();
        let r_cmp: Symbol<unsafe extern "C" fn(*const ResultArray, c_int, c_int) -> c_int> = r_lib.get(b"compare_results_in_array").unwrap();

        let values: [c_int; 5] = [10, 20, 30, 40, 50];
        let mut c_arr = ResultArray::zeroed();
        let mut r_arr = ResultArray::zeroed();
        c_init(&mut c_arr, values.as_ptr(), 5);
        r_init(&mut r_arr, values.as_ptr(), 5);

        for i in 0..5 {
            for j in 0..5 {
                assert_eq!(c_cmp(&c_arr, i, j), r_cmp(&r_arr, i, j), "compare({i},{j})");
            }
        }
        // Out of bounds
        assert_eq!(c_cmp(&c_arr, 5, 0), r_cmp(&r_arr, 5, 0), "compare(5,0)");
        assert_eq!(c_cmp(&c_arr, 0, 5), r_cmp(&r_arr, 0, 5), "compare(0,5)");
    }
}

#[test]
fn test_process_with_foreach() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_init: Symbol<unsafe extern "C" fn(*mut ResultArray, *const c_int, c_int)> = c_lib.get(b"init_result_array").unwrap();
        let r_init: Symbol<unsafe extern "C" fn(*mut ResultArray, *const c_int, c_int)> = r_lib.get(b"init_result_array").unwrap();
        let c_proc: Symbol<unsafe extern "C" fn(*mut ResultArray, unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int) -> c_int> = c_lib.get(b"process_with_foreach").unwrap();
        let r_proc: Symbol<unsafe extern "C" fn(*mut ResultArray, unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int) -> c_int> = r_lib.get(b"process_with_foreach").unwrap();
        let c_add: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = c_lib.get(b"add_operation").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = r_lib.get(b"add_operation").unwrap();

        let values: [c_int; 5] = [10, 20, 30, 40, 50];
        let mut c_arr = ResultArray::zeroed();
        let mut r_arr = ResultArray::zeroed();
        c_init(&mut c_arr, values.as_ptr(), 5);
        r_init(&mut r_arr, values.as_ptr(), 5);

        let c_result = c_proc(&mut c_arr, *c_add);
        let r_result = r_proc(&mut r_arr, *r_add);
        assert_eq!(c_result, r_result, "process_with_foreach return");
        assert_result_arrays_eq(&c_arr, &r_arr, "process_with_foreach state");
    }
}

#[test]
fn test_compute_weighted_sum() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_init: Symbol<unsafe extern "C" fn(*mut ResultArray, *const c_int, c_int)> = c_lib.get(b"init_result_array").unwrap();
        let r_init: Symbol<unsafe extern "C" fn(*mut ResultArray, *const c_int, c_int)> = r_lib.get(b"init_result_array").unwrap();
        let c_ws: Symbol<unsafe extern "C" fn(*const ResultArray) -> c_int> = c_lib.get(b"compute_weighted_sum").unwrap();
        let r_ws: Symbol<unsafe extern "C" fn(*const ResultArray) -> c_int> = r_lib.get(b"compute_weighted_sum").unwrap();

        let values: [c_int; 8] = [10, 20, 30, 40, 50, 60, 70, 80];
        let mut c_arr = ResultArray::zeroed();
        let mut r_arr = ResultArray::zeroed();
        c_init(&mut c_arr, values.as_ptr(), 8);
        r_init(&mut r_arr, values.as_ptr(), 8);

        assert_eq!(c_ws(&c_arr), r_ws(&r_arr), "compute_weighted_sum");
    }
}

// ---- Top-level function ----

#[test]
fn test_arrayfunc() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = c_lib.get(b"arrayfunc").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> = r_lib.get(b"arrayfunc").unwrap();

        let test_cases = [
            (1, 2, 3, 4),
            (0, 0, 0, 0),
            (10, 20, 30, 40),
            (-1, -2, -3, -4),
            (100, 200, 300, 400),
            (i32::MAX, 0, 0, 0),
            (0, i32::MIN, 0, 0),
            (1, 1, 1, 1),
            (7, 13, 42, 99),
            (-100, 50, -25, 75),
        ];

        for (a, b, c, d) in test_cases {
            assert_eq!(c_fn(a, b, c, d), r_fn(a, b, c, d), "arrayfunc({a}, {b}, {c}, {d})");
        }
    }
}
