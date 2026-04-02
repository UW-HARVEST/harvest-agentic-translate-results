use libloading::{Library, Symbol};
use std::path::PathBuf;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2v {
    x: f32,
    y: f32,
}

fn c_lib_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("c_src")
        .join("build")
        .join("libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest.join("target").join("debug").join("libgjk_lib.so")
}

type GjkFn = unsafe extern "C" fn(
    i8, *mut c2v, *mut c2v,
    f32, f32, f32, f32,
    f32, f32, f32, f32, f32,
);

fn compare_gjk(reverse: i8, a1: f32, a2: f32, a3: f32, a4: f32,
               b1: f32, b2: f32, b3: f32, b4: f32, b5: f32) {
    let c_lib = unsafe { Library::new(c_lib_path()).expect("load C lib") };
    let c_gjk: Symbol<GjkFn> = unsafe { c_lib.get(b"gjk").expect("find C gjk") };

    let r_lib = unsafe { Library::new(rust_lib_path()).expect("load Rust lib") };
    let r_gjk: Symbol<GjkFn> = unsafe { r_lib.get(b"gjk").expect("find Rust gjk") };

    let mut c_a = c2v { x: 0.0, y: 0.0 };
    let mut c_b = c2v { x: 0.0, y: 0.0 };
    unsafe { c_gjk(reverse, &mut c_a, &mut c_b, a1, a2, a3, a4, b1, b2, b3, b4, b5) };

    let mut r_a = c2v { x: 0.0, y: 0.0 };
    let mut r_b = c2v { x: 0.0, y: 0.0 };
    unsafe { r_gjk(reverse, &mut r_a, &mut r_b, a1, a2, a3, a4, b1, b2, b3, b4, b5) };

    assert_eq!(c_a.x.to_bits(), r_a.x.to_bits(),
        "a.x mismatch: C={} Rust={} (reverse={}, aabb=[{},{},{},{}], cap=[{},{},{},{},{}])",
        c_a.x, r_a.x, reverse, a1, a2, a3, a4, b1, b2, b3, b4, b5);
    assert_eq!(c_a.y.to_bits(), r_a.y.to_bits(),
        "a.y mismatch: C={} Rust={}", c_a.y, r_a.y);
    assert_eq!(c_b.x.to_bits(), r_b.x.to_bits(),
        "b.x mismatch: C={} Rust={}", c_b.x, r_b.x);
    assert_eq!(c_b.y.to_bits(), r_b.y.to_bits(),
        "b.y mismatch: C={} Rust={}", c_b.y, r_b.y);
}

#[test]
fn test_gjk_no_overlap_forward() {
    compare_gjk(0, 0.0, 0.0, 1.0, 1.0, 5.0, 5.0, 6.0, 6.0, 0.5);
}

#[test]
fn test_gjk_no_overlap_reverse() {
    compare_gjk(1, 0.0, 0.0, 1.0, 1.0, 5.0, 5.0, 6.0, 6.0, 0.5);
}

#[test]
fn test_gjk_overlap_forward() {
    compare_gjk(0, 0.0, 0.0, 2.0, 2.0, 1.0, 1.0, 3.0, 3.0, 1.0);
}

#[test]
fn test_gjk_overlap_reverse() {
    compare_gjk(1, 0.0, 0.0, 2.0, 2.0, 1.0, 1.0, 3.0, 3.0, 1.0);
}

#[test]
fn test_gjk_touching_edge() {
    compare_gjk(0, 0.0, 0.0, 1.0, 1.0, 1.5, 0.5, 2.5, 0.5, 0.5);
}

#[test]
fn test_gjk_touching_edge_reverse() {
    compare_gjk(1, 0.0, 0.0, 1.0, 1.0, 1.5, 0.5, 2.5, 0.5, 0.5);
}

#[test]
fn test_gjk_negative_coords() {
    compare_gjk(0, -3.0, -3.0, -1.0, -1.0, -5.0, -5.0, -4.0, -4.0, 0.3);
}

#[test]
fn test_gjk_large_radius() {
    compare_gjk(0, 0.0, 0.0, 1.0, 1.0, 3.0, 0.0, 4.0, 0.0, 5.0);
}

#[test]
fn test_gjk_zero_radius() {
    compare_gjk(0, 0.0, 0.0, 1.0, 1.0, 2.0, 0.0, 3.0, 0.0, 0.0);
}

#[test]
fn test_gjk_capsule_vertical() {
    compare_gjk(0, 0.0, 0.0, 1.0, 1.0, 0.5, 3.0, 0.5, 5.0, 0.5);
}

#[test]
fn test_gjk_capsule_degenerate() {
    compare_gjk(0, 0.0, 0.0, 1.0, 1.0, 3.0, 3.0, 3.0, 3.0, 1.0);
}

#[test]
fn test_gjk_aabb_degenerate() {
    compare_gjk(0, 0.0, 0.0, 0.0, 1.0, 2.0, 0.0, 3.0, 0.0, 0.5);
}

#[test]
fn test_gjk_close_proximity() {
    compare_gjk(0, 0.0, 0.0, 1.0, 1.0, 1.1, 0.0, 2.0, 0.0, 0.05);
}

#[test]
fn test_gjk_fractional_values() {
    compare_gjk(0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9);
}

#[test]
fn test_gjk_large_values() {
    compare_gjk(0, 100.0, 200.0, 300.0, 400.0, 500.0, 600.0, 700.0, 800.0, 50.0);
}
