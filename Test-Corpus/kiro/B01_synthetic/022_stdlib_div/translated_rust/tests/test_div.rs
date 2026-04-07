use libloading::{Library, Symbol};
use std::path::PathBuf;

type CDivFn = unsafe extern "C" fn(i32, i32, *mut i32, *mut i32);

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("c_lib/libdiv_c.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Try release first, then debug
    let release = dir.join("target/release/libdriver.so");
    if release.exists() { return release; }
    dir.join("target/debug/libdriver.so")
}

fn call_c_div(lib: &Library, x: i32, y: i32) -> (i32, i32) {
    unsafe {
        let func: Symbol<CDivFn> = lib.get(b"c_div").unwrap();
        let (mut q, mut r) = (0i32, 0i32);
        func(x, y, &mut q, &mut r);
        (q, r)
    }
}

#[test]
fn test_c_div_matches() {
    let c_lib = unsafe { Library::new(c_lib_path()).unwrap() };
    let rs_lib = unsafe { Library::new(rust_lib_path()).unwrap() };

    let cases: &[(i32, i32)] = &[
        (10, 3), (10, -3), (-10, 3), (-10, -3),
        (0, 1), (1, 1), (-1, 1), (7, 7),
        (100, 7), (-100, 7), (i32::MAX, 1),
        (i32::MAX, 2), (i32::MIN, 2), (1, i32::MAX), (1, i32::MIN),
        (99, -13), (-99, 13), (-99, -13),
    ];

    for &(x, y) in cases {
        let c_result = call_c_div(&c_lib, x, y);
        let rs_result = call_c_div(&rs_lib, x, y);
        assert_eq!(
            c_result, rs_result,
            "Mismatch for div({}, {}): C={:?} Rust={:?}", x, y, c_result, rs_result
        );
    }
}
