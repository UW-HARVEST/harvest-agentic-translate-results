use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Raycast {
    t: f32,
    n: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Ray {
    p: C2v,
    d: C2v,
    t: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2m {
    x: C2v,
    y: C2v,
}

fn v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

fn c_lib() -> Library {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libspec_ray_lib.so");
    unsafe { Library::new(path).expect("Failed to load C library") }
}

fn rust_lib() -> Library {
    // cargo builds cdylib at target/debug/libspec_ray_lib.so
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libspec_ray_lib.so");
    unsafe { Library::new(path).expect("Failed to load Rust library") }
}

fn assert_v_eq(label: &str, c: C2v, r: C2v) {
    assert!(
        c.x.to_bits() == r.x.to_bits() && c.y.to_bits() == r.y.to_bits(),
        "{label}: C=({}, {}) [bits: {:08x},{:08x}] Rust=({}, {}) [bits: {:08x},{:08x}]",
        c.x, c.y, c.x.to_bits(), c.y.to_bits(),
        r.x, r.y, r.x.to_bits(), r.y.to_bits()
    );
}

fn assert_f32_eq(label: &str, c: f32, r: f32) {
    assert!(
        c.to_bits() == r.to_bits(),
        "{label}: C={c} [bits: {:08x}] Rust={r} [bits: {:08x}]",
        c.to_bits(), r.to_bits()
    );
}

fn assert_raycast_eq(label: &str, c: C2Raycast, r: C2Raycast) {
    assert_f32_eq(&format!("{label} t"), c.t, r.t);
    assert_v_eq(&format!("{label} n"), c.n, r.n);
}

// ============ spec_ray: C vs Rust ============
#[test]
fn test_spec_ray_c_vs_rust() {
    let c = c_lib();
    let r = rust_lib();
    type SpecRayFn = unsafe extern "C" fn(*mut C2Raycast, f32, f32, f32, f32, f32, f32, f32) -> c_int;
    let c_fn: Symbol<SpecRayFn> = unsafe { c.get(b"spec_ray").unwrap() };
    let r_fn: Symbol<SpecRayFn> = unsafe { r.get(b"spec_ray").unwrap() };

    let cases: &[(f32, f32, f32, f32, f32, f32, f32)] = &[
        (10.0, 0.0, 5.0, 0.0, 1.0, 0.0, 0.0),
        (10.0, 10.0, 5.0, 0.0, 1.0, 0.0, 0.0),
        (5.0, 5.0, 0.0, 0.0, 2.0, -5.0, -5.0),
        (1.0, 0.0, 100.0, 0.0, 0.5, 0.0, 0.0),
        (10.0, 0.0, 0.0, 0.0, 5.0, 0.0, 0.0),
        (-10.0, -10.0, -5.0, -5.0, 1.0, 0.0, 0.0),
        (1000.0, 0.0, 500.0, 0.0, 10.0, 0.0, 0.0),
        (1.0, 0.0, 0.5, 0.0, 0.01, 0.0, 0.0),
        (3.0, 4.0, 1.0, 2.0, 0.5, -1.0, -1.0),
        (0.0, 10.0, 0.0, 5.0, 1.0, 0.0, 0.0),
    ];

    for (i, &(mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y)) in cases.iter().enumerate() {
        let mut c_cast = C2Raycast { t: 0.0, n: v(0.0, 0.0) };
        let mut r_cast = C2Raycast { t: 0.0, n: v(0.0, 0.0) };

        let c_hit = unsafe { c_fn(&mut c_cast, mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y) };
        let r_hit = unsafe { r_fn(&mut r_cast, mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y) };

        assert_eq!(c_hit, r_hit, "case {i}: hit mismatch C={c_hit} Rust={r_hit}");
        if c_hit != 0 {
            assert_raycast_eq(&format!("case {i}"), c_cast, r_cast);
        }
    }
}

// ============ Sanity tests for C library functions ============
#[test]
fn test_c2v() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = unsafe { lib.get(b"c2V").unwrap() };
    for &(x, y) in &[(0.0f32, 0.0f32), (1.0, -2.5), (-0.0, 0.0)] {
        let res = unsafe { c_fn(x, y) };
        assert_v_eq("c2V", res, C2v { x, y });
    }
}

#[test]
fn test_c2dot() {
    let lib = c_lib();
    let c_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = unsafe { lib.get(b"c2Dot").unwrap() };
    let cases = [(v(1.0, 0.0), v(0.0, 1.0)), (v(3.0, 4.0), v(4.0, 3.0)), (v(-1.5, 2.7), v(0.3, -0.8))];
    for (a, b) in cases {
        assert_f32_eq("c2Dot", unsafe { c_fn(a, b) }, a.x * b.x + a.y * b.y);
    }
}

#[test]
fn test_c2_arithmetic() {
    let lib = c_lib();
    let c_add: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = unsafe { lib.get(b"c2Add").unwrap() };
    let c_sub: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = unsafe { lib.get(b"c2Sub").unwrap() };
    let c_mulvs: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = unsafe { lib.get(b"c2Mulvs").unwrap() };
    let a = v(3.0, -4.0);
    let b = v(1.5, 2.5);
    assert_v_eq("c2Add", unsafe { c_add(a, b) }, v(a.x + b.x, a.y + b.y));
    assert_v_eq("c2Sub", unsafe { c_sub(a, b) }, v(a.x - b.x, a.y - b.y));
    assert_v_eq("c2Mulvs", unsafe { c_mulvs(a, 2.5) }, v(a.x * 2.5, a.y * 2.5));
}

#[test]
fn test_c2_misc_vec() {
    let lib = c_lib();
    let c_skew: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { lib.get(b"c2Skew").unwrap() };
    let c_absv: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { lib.get(b"c2Absv").unwrap() };
    let c_ccw90: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { lib.get(b"c2CCW90").unwrap() };
    let a = v(-3.0, 4.0);
    assert_v_eq("c2Skew", unsafe { c_skew(a) }, v(-4.0, -3.0));
    assert_v_eq("c2Absv", unsafe { c_absv(a) }, v(3.0, 4.0));
    assert_v_eq("c2CCW90", unsafe { c_ccw90(a) }, v(4.0, 3.0));
}
