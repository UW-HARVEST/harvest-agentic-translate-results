use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libhatch_lib.so")
}

fn rust_lib_path() -> PathBuf {
    // Find the built Rust cdylib
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Try debug first
    let debug = p.join("debug/libhatch_lib.so");
    if debug.exists() {
        return debug;
    }
    p.join("release/libhatch_lib.so")
}

#[test]
fn test_add_three() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"add_three").unwrap();

        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            rust_lib.get(b"add_three").unwrap();

        let cases = [(1, 2, 3), (0, 0, 0), (-1, -2, -3), (i32::MAX, 0, 1), (100, 200, 300)];
        for (a, b, c) in cases {
            let c_res = c_fn(a, b, c);
            let r_res = r_fn(a, b, c);
            assert_eq!(c_res, r_res, "add_three({a},{b},{c}): C={c_res} Rust={r_res}");
        }
    }
}

#[test]
fn test_multiply_add() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"multiply_add").unwrap();

        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int) -> c_int> =
            rust_lib.get(b"multiply_add").unwrap();

        let cases = [(2, 3, 4), (0, 5, 10), (-1, 7, 3), (100, 200, 300)];
        for (a, b, c) in cases {
            let c_res = c_fn(a, b, c);
            let r_res = r_fn(a, b, c);
            assert_eq!(c_res, r_res, "multiply_add({a},{b},{c}): C={c_res} Rust={r_res}");
        }
    }
}

#[test]
fn test_compute_with_dynamic_memory() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
            c_lib.get(b"compute_with_dynamic_memory").unwrap();

        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
            rust_lib.get(b"compute_with_dynamic_memory").unwrap();

        let cases = [(10, 5), (0, 3), (1, 8), (100, 10)];
        for (base, count) in cases {
            let c_res = c_fn(base, count);
            let r_res = r_fn(base, count);
            assert_eq!(c_res, r_res, "compute_with_dynamic_memory({base},{count}): C={c_res} Rust={r_res}");
        }
    }
}

#[test]
fn test_shift_array_data() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(*mut c_int, c_int, c_int)> =
            c_lib.get(b"shift_array_data").unwrap();

        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let r_fn: Symbol<unsafe extern "C" fn(*mut c_int, c_int, c_int)> =
            rust_lib.get(b"shift_array_data").unwrap();

        for shift in [0, 1, 3, 5, 9] {
            let mut c_arr = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10i32];
            let mut r_arr = c_arr;
            c_fn(c_arr.as_mut_ptr(), 10, shift);
            r_fn(r_arr.as_mut_ptr(), 10, shift);
            assert_eq!(c_arr, r_arr, "shift_array_data shift={shift}");
        }
    }
}

#[test]
fn test_get_time_based_value() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            c_lib.get(b"get_time_based_value").unwrap();

        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let r_fn: Symbol<unsafe extern "C" fn(c_int) -> c_int> =
            rust_lib.get(b"get_time_based_value").unwrap();

        // Call back-to-back to minimize time difference
        for seed in [0, 1, 5, 10, 100] {
            let c_res = c_fn(seed);
            let r_res = r_fn(seed);
            assert_eq!(c_res, r_res, "get_time_based_value({seed}): C={c_res} Rust={r_res}");
        }
    }
}

#[test]
fn test_hatch() {
    // Each library gets fresh global state on load.
    // Call hatch once on each with same params and compare.
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"hatch").unwrap();

        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");
        let r_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            rust_lib.get(b"hatch").unwrap();

        let cases = [
            (1, 2, 3, 4),
            (10, 20, 30, 40),
            (0, 0, 0, 0),
            (5, 5, 5, 5),
        ];
        for (a, b, c, d) in cases {
            // Reload libs to reset global state for each test case
            drop(r_fn);
            drop(c_fn);
            drop(rust_lib);
            drop(c_lib);

            let c_lib = Library::new(c_lib_path()).expect("load C lib");
            let c_hatch: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
                c_lib.get(b"hatch").unwrap();

            let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");
            let r_hatch: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
                rust_lib.get(b"hatch").unwrap();

            let c_res = c_hatch(a, b, c, d);
            let r_res = r_hatch(a, b, c, d);
            assert_eq!(c_res, r_res, "hatch({a},{b},{c},{d}): C={c_res} Rust={r_res}");

            // break to avoid reuse after drop
            break;
        }

        // Test remaining cases with fresh loads each time
        for &(a, b, c, d) in &[(10, 20, 30, 40), (0, 0, 0, 0), (5, 5, 5, 5)] {
            let c_lib2 = Library::new(c_lib_path()).expect("load C lib");
            let c_hatch2: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
                c_lib2.get(b"hatch").unwrap();

            let rust_lib2 = Library::new(rust_lib_path()).expect("load Rust lib");
            let r_hatch2: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
                rust_lib2.get(b"hatch").unwrap();

            let c_res = c_hatch2(a, b, c, d);
            let r_res = r_hatch2(a, b, c, d);
            assert_eq!(c_res, r_res, "hatch({a},{b},{c},{d}): C={c_res} Rust={r_res}");
        }
    }
}
