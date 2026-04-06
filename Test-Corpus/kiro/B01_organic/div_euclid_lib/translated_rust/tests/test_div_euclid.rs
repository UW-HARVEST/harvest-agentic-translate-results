use libloading::{Library, Symbol};
use std::os::raw::c_int;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libtranslated_rust.so"
    );
    unsafe { Library::new(path).expect("Failed to load C .so") }
}

fn call_c(lib: &Library, v1: c_int, v2: c_int) -> c_int {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int, c_int) -> c_int> =
            lib.get(b"div_euclid").expect("symbol not found");
        f(v1, v2)
    }
}

#[test]
fn test_div_euclid_matches_c() {
    let lib = c_lib();
    let int_min: c_int = c_int::MIN;
    let int_max: c_int = c_int::MAX;

    let cases: Vec<(c_int, c_int)> = vec![
        // basic
        (7, 3),
        (7, -3),
        (-7, 3),
        (-7, -3),
        // zero dividend
        (0, 1),
        (0, -1),
        // zero divisor
        (1, 0),
        (0, 0),
        // exact division
        (6, 3),
        (6, -3),
        (-6, 3),
        (-6, -3),
        // one
        (1, 1),
        (-1, 1),
        (1, -1),
        (-1, -1),
        // INT_MIN edge cases
        (int_min, 1),
        (int_min, -1),
        (int_min, 2),
        (int_min, -2),
        (int_min, int_max),
        (int_min, int_min),
        (int_min, 3),
        (int_min, -3),
        // INT_MAX edge cases
        (int_max, 1),
        (int_max, -1),
        (int_max, 2),
        (int_max, -2),
        (int_max, int_max),
        (int_max, int_min),
        // v1 positive, v2 = INT_MIN
        (1, int_min),
        (100, int_min),
        // v1 negative (not INT_MIN), v2 = INT_MIN
        (-1, int_min),
        (-100, int_min),
        // large values
        (1000000, 7),
        (-1000000, 7),
        (1000000, -7),
        (-1000000, -7),
    ];

    for (v1, v2) in &cases {
        let c_result = call_c(&lib, *v1, *v2);
        let rust_result = div_euclid_lib::div_euclid(*v1, *v2);
        assert_eq!(
            c_result, rust_result,
            "MISMATCH for div_euclid({}, {}): C={}, Rust={}",
            v1, v2, c_result, rust_result
        );
    }
}
