use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int, CStr};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libbuffapp_lib.so")
}

fn rust_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join("libbuffapp_lib.so");
    if !p.exists() {
        p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target")
            .join("release")
            .join("libbuffapp_lib.so");
    }
    p
}

// ---- lowest level: get_operation_name ----
#[test]
fn test_get_operation_name() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(c_int) -> *const c_char> =
            c_lib.get(b"get_operation_name").unwrap();
        let rust_fn: Symbol<unsafe extern "C" fn(c_int) -> *const c_char> =
            rust_lib.get(b"get_operation_name").unwrap();

        let cases: &[c_int] = &[0, 1, 2, 3, 4, -1, 100, i32::MIN, i32::MAX];
        for &op in cases {
            let c_result = CStr::from_ptr(c_fn(op)).to_str().unwrap();
            let rust_result = CStr::from_ptr(rust_fn(op)).to_str().unwrap();
            assert_eq!(
                c_result, rust_result,
                "get_operation_name({op}): C={c_result:?} Rust={rust_result:?}"
            );
        }
    }
}

// ---- perform_operation ----
#[test]
fn test_perform_operation() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, *const c_char) -> c_int> =
            c_lib.get(b"perform_operation").unwrap();
        let rust_fn: Symbol<unsafe extern "C" fn(c_int, c_int, *const c_char) -> c_int> =
            rust_lib.get(b"perform_operation").unwrap();

        let ops: &[&CStr] = &[c"add", c"subtract", c"multiply", c"divide", c"unknown"];
        let vals: &[(c_int, c_int)] = &[
            (0, 0), (1, 1), (-1, 1), (10, 3), (10, 0), (i32::MAX, 1),
            (i32::MIN, 1), (100, -7), (7, 3),
        ];

        for &op in ops {
            for &(a, b) in vals {
                let c_result = c_fn(a, b, op.as_ptr());
                let rust_result = rust_fn(a, b, op.as_ptr());
                assert_eq!(
                    c_result, rust_result,
                    "perform_operation({a}, {b}, {:?}): C={c_result} Rust={rust_result}",
                    op
                );
            }
        }
    }
}

// ---- top-level: buffapp ----
#[test]
fn test_buffapp() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).expect("load C lib");
        let rust_lib = Library::new(rust_lib_path()).expect("load Rust lib");

        let c_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            c_lib.get(b"buffapp").unwrap();
        let rust_fn: Symbol<unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int> =
            rust_lib.get(b"buffapp").unwrap();

        let cases: &[(c_int, c_int, c_int, c_int)] = &[
            (0, 0, 0, 0),
            (1, 2, 3, 4),
            (0, 1, 2, 3),
            (4, 5, 6, 7),
            (10, 20, 30, 40),
            (-1, -2, -3, -4),
            (1, 0, 1, 0),
            (3, 3, 3, 3),
            (100, 200, 300, 400),
            (7, 11, 13, 17),
            (0, 0, 1, 1),
            (2, 3, 0, 1),
        ];

        for &(a, b, c, d) in cases {
            let c_result = c_fn(a, b, c, d);
            let rust_result = rust_fn(a, b, c, d);
            assert_eq!(
                c_result, rust_result,
                "buffapp({a}, {b}, {c}, {d}): C={c_result} Rust={rust_result}"
            );
        }
    }
}
