use libloading::{Library, Symbol};
use std::os::raw::c_int;

fn c_lib() -> Library {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/c_src/build/libmemchra2_lib.so"
    );
    unsafe { Library::new(path).expect("failed to load C .so") }
}

fn call_c_memchra2(lib: &Library, a: c_int, b: c_int, c: c_int, d: c_int) -> c_int {
    unsafe {
        let f: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            lib.get(b"memchra2").unwrap();
        f(a, b, c, d)
    }
}

#[test]
fn test_memchra2_matches_c() {
    let lib = c_lib();

    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (-1, -2, -3, -4),
        (100, 200, 50, 75),
        (0, 1, 0, 0),
        (1065353216, 0, 0, 0), // 1.0f as int bits
        (1, 1, 1, 1),
        (255, 255, 255, 255),
        (i32::MAX, 0, 0, 0),
        (i32::MIN, 0, 0, 0),
        (42, 99, 7, 13),
        (1000, 2000, 3000, 4000),
    ];

    for &(a, b, c, d) in cases {
        let c_result = call_c_memchra2(&lib, a, b, c, d);
        let rust_result = memchra2_lib::memchra2(a, b, c, d);
        assert_eq!(
            c_result, rust_result,
            "mismatch for ({}, {}, {}, {}): C={}, Rust={}",
            a, b, c, d, c_result, rust_result
        );
    }
}
