use libloading::{Library, Symbol};
use std::path::PathBuf;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    // Find the built Rust cdylib
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Try debug first
    let debug = p.join("debug").join("libcapsule_lib.so");
    if debug.exists() {
        return debug;
    }
    p.join("release").join("libcapsule_lib.so")
}

/// Test the public API: capsule(min_x, min_y, max_x, max_y, r) -> int
#[test]
fn test_capsule_public_api() {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };

    type CapsuleFn = unsafe extern "C" fn(f32, f32, f32, f32, f32) -> i32;
    let c_fn: Symbol<CapsuleFn> = unsafe { c_lib.get(b"capsule").unwrap() };
    let r_fn: Symbol<CapsuleFn> = unsafe { r_lib.get(b"capsule").unwrap() };

    // Test cases: (min_x, min_y, max_x, max_y, r)
    let cases: &[(f32, f32, f32, f32, f32)] = &[
        (0.0, 0.0, 10.0, 10.0, 5.0),
        (-100.0, -100.0, 100.0, 100.0, 50.0),
        (-50.0, -50.0, -30.0, -30.0, 15.0),
        (0.0, 0.0, 0.0, 0.0, 0.0),
        (-70.0, 0.0, -60.0, 10.0, 25.0),
        (-40.0, 40.0, -20.0, 100.0, 10.0),
        (1.0, 2.0, 3.0, 4.0, 0.5),
        (-1000.0, -1000.0, 1000.0, 1000.0, 100.0),
        (-30.0, -30.0, -20.0, -20.0, 5.0),
        (-45.0, 50.0, -15.0, 90.0, 12.0),
    ];

    for &(min_x, min_y, max_x, max_y, r) in cases {
        let c_result = unsafe { c_fn(min_x, min_y, max_x, max_y, r) };
        let r_result = unsafe { r_fn(min_x, min_y, max_x, max_y, r) };
        assert_eq!(
            c_result, r_result,
            "capsule({min_x}, {min_y}, {max_x}, {max_y}, {r}): C={c_result}, Rust={r_result}"
        );
    }
}
