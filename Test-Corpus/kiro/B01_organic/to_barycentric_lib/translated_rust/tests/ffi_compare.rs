use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct LmVec2 {
    x: f32,
    y: f32,
}

type ToBarycentricFn = unsafe extern "C" fn(LmVec2, LmVec2, LmVec2, LmVec2) -> LmVec2;

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug");
    dir.join("libto_barycentric_lib.so")
}

fn v(x: f32, y: f32) -> LmVec2 {
    LmVec2 { x, y }
}

#[test]
fn test_to_barycentric() {
    let c_lib = unsafe { Library::new(c_lib_path()) }.expect("load C lib");
    let r_lib = unsafe { Library::new(rust_lib_path()) }.expect("load Rust lib");

    let c_fn: Symbol<ToBarycentricFn> =
        unsafe { c_lib.get(b"to_barycentric") }.expect("C symbol");
    let r_fn: Symbol<ToBarycentricFn> =
        unsafe { r_lib.get(b"to_barycentric") }.expect("Rust symbol");

    let cases: Vec<[LmVec2; 4]> = vec![
        // basic triangle
        [v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(0.25, 0.25)],
        // point at vertex p1
        [v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(0.0, 0.0)],
        // point at vertex p2
        [v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(1.0, 0.0)],
        // point at vertex p3
        [v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(0.0, 1.0)],
        // outside triangle
        [v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(2.0, 2.0)],
        // negative coords
        [v(-1.0, -1.0), v(1.0, -1.0), v(0.0, 1.0), v(0.0, 0.0)],
        // degenerate (collinear) — produces inf/nan, must still match bitwise
        [v(0.0, 0.0), v(1.0, 0.0), v(2.0, 0.0), v(0.5, 0.0)],
        // large values
        [v(1e6, 1e6), v(1e6 + 1.0, 1e6), v(1e6, 1e6 + 1.0), v(1e6 + 0.5, 1e6 + 0.5)],
        // tiny values
        [v(1e-7, 1e-7), v(2e-7, 1e-7), v(1e-7, 2e-7), v(1.5e-7, 1.5e-7)],
    ];

    for (i, c) in cases.iter().enumerate() {
        let c_res = unsafe { c_fn(c[0], c[1], c[2], c[3]) };
        let r_res = unsafe { r_fn(c[0], c[1], c[2], c[3]) };
        assert_eq!(
            c_res.x.to_bits(), r_res.x.to_bits(),
            "case {i}: x mismatch: C={} Rust={}", c_res.x, r_res.x
        );
        assert_eq!(
            c_res.y.to_bits(), r_res.y.to_bits(),
            "case {i}: y mismatch: C={} Rust={}", c_res.y, r_res.y
        );
    }
}
