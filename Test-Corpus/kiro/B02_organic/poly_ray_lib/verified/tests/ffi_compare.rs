#![allow(non_snake_case, non_camel_case_types)]
use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2v { x: f32, y: f32 }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2Raycast { t: f32, n: c2v }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2r { c: f32, s: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2x { p: c2v, r: c2r }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Circle { p: c2v, r: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2AABB { min: c2v, max: c2v }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Capsule { a: c2v, b: c2v, r: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Poly {
    count: c_int,
    verts: [c2v; 8],
    norms: [c2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Ray { p: c2v, d: c2v, t: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2m { x: c2v, y: c2v }

fn v(x: f32, y: f32) -> c2v { c2v { x, y } }

fn load_libs() -> (Library, Library) {
    let c_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("c_src/build/libtranslated_rust.so");
    let rust_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/debug/libpoly_ray_lib.so");
    unsafe {
        (Library::new(&c_path).expect("load C .so"),
         Library::new(&rust_path).expect("load Rust .so"))
    }
}

fn assert_v_eq(label: &str, a: c2v, b: c2v) {
    assert!(a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits(),
        "{label}: C=({},{}) Rust=({},{})", a.x, a.y, b.x, b.y);
}

fn assert_f_eq(label: &str, a: f32, b: f32) {
    assert!(a.to_bits() == b.to_bits(), "{label}: C={a} Rust={b}");
}

fn assert_cast_eq(label: &str, a: c2Raycast, b: c2Raycast) {
    assert_f_eq(&format!("{label}.t"), a.t, b.t);
    assert_v_eq(&format!("{label}.n"), a.n, b.n);
}

// ===== Level 0: Leaf functions =====

#[test]
fn test_c2V() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(f32, f32) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2V").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2V").unwrap() };
    for (x, y) in [(0.0, 0.0), (1.5, -2.3), (f32::MAX, f32::MIN)] {
        let c = unsafe { c_fn(x, y) };
        let r = unsafe { r_fn(x, y) };
        assert_v_eq("c2V", c, r);
    }
}

#[test]
fn test_c2Dot() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2v, c2v) -> f32;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Dot").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Dot").unwrap() };
    let pairs = [(v(1.0, 2.0), v(3.0, 4.0)), (v(-1.0, 0.0), v(0.0, 1.0)), (v(0.0, 0.0), v(5.0, 5.0))];
    for (a, b) in pairs {
        assert_f_eq("c2Dot", unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) });
    }
}

#[test]
fn test_c2Len() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2v) -> f32;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Len").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Len").unwrap() };
    for a in [v(3.0, 4.0), v(0.0, 0.0), v(-1.0, 1.0)] {
        assert_f_eq("c2Len", unsafe { c_fn(a) }, unsafe { r_fn(a) });
    }
}

#[test]
fn test_c2Add() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2v, c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Add").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Add").unwrap() };
    let (a, b) = (v(1.0, 2.0), v(3.0, -4.0));
    assert_v_eq("c2Add", unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) });
}

#[test]
fn test_c2Sub() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2v, c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Sub").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Sub").unwrap() };
    let (a, b) = (v(5.0, 3.0), v(2.0, 7.0));
    assert_v_eq("c2Sub", unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) });
}

#[test]
fn test_c2Mulvs() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2v, f32) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Mulvs").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Mulvs").unwrap() };
    let a = v(2.0, -3.0);
    assert_v_eq("c2Mulvs", unsafe { c_fn(a, 2.5) }, unsafe { r_fn(a, 2.5) });
}

#[test]
fn test_c2Div() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2v, f32) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Div").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Div").unwrap() };
    let a = v(6.0, -9.0);
    assert_v_eq("c2Div", unsafe { c_fn(a, 3.0) }, unsafe { r_fn(a, 3.0) });
}

#[test]
fn test_c2Norm() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Norm").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Norm").unwrap() };
    for a in [v(3.0, 4.0), v(-1.0, 0.0), v(0.5, 0.5)] {
        assert_v_eq("c2Norm", unsafe { c_fn(a) }, unsafe { r_fn(a) });
    }
}

#[test]
fn test_c2Minv() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2v, c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Minv").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Minv").unwrap() };
    let (a, b) = (v(1.0, 5.0), v(3.0, 2.0));
    assert_v_eq("c2Minv", unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) });
}

#[test]
fn test_c2Maxv() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2v, c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Maxv").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Maxv").unwrap() };
    let (a, b) = (v(1.0, 5.0), v(3.0, 2.0));
    assert_v_eq("c2Maxv", unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) });
}

#[test]
fn test_c2Skew() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Skew").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Skew").unwrap() };
    for a in [v(1.0, 2.0), v(-3.0, 0.0)] {
        assert_v_eq("c2Skew", unsafe { c_fn(a) }, unsafe { r_fn(a) });
    }
}

#[test]
fn test_c2Absv() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Absv").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Absv").unwrap() };
    for a in [v(-1.0, 2.0), v(0.0, -5.0), v(-3.0, -4.0)] {
        assert_v_eq("c2Absv", unsafe { c_fn(a) }, unsafe { r_fn(a) });
    }
}

// ===== Level 1: Functions using leaf functions =====

#[test]
fn test_c2CCW90() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2CCW90").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2CCW90").unwrap() };
    for a in [v(1.0, 0.0), v(0.0, 1.0), v(3.0, -4.0)] {
        assert_v_eq("c2CCW90", unsafe { c_fn(a) }, unsafe { r_fn(a) });
    }
}

#[test]
fn test_c2MulmvT() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2m, c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2MulmvT").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2MulmvT").unwrap() };
    let m = c2m { x: v(1.0, 0.0), y: v(0.0, 1.0) };
    let b = v(3.0, 4.0);
    assert_v_eq("c2MulmvT", unsafe { c_fn(m, b) }, unsafe { r_fn(m, b) });
}

#[test]
fn test_c2AABBtoAABB() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2AABBtoAABB").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2AABBtoAABB").unwrap() };
    let cases = [
        (c2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) }, c2AABB { min: v(1.0, 1.0), max: v(3.0, 3.0) }),
        (c2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }, c2AABB { min: v(5.0, 5.0), max: v(6.0, 6.0) }),
    ];
    for (a, b) in cases {
        assert_eq!(unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) }, "c2AABBtoAABB");
    }
}

#[test]
fn test_c2AABBtoPoint() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2AABB, c2v) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2AABBtoPoint").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2AABBtoPoint").unwrap() };
    let aabb = c2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) };
    for p in [v(1.0, 1.0), v(3.0, 3.0), v(-1.0, 1.0)] {
        assert_eq!(unsafe { c_fn(aabb, p) }, unsafe { r_fn(aabb, p) }, "c2AABBtoPoint");
    }
}

#[test]
fn test_c2CircleToPoint() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2Circle, c2v) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2CircleToPoint").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2CircleToPoint").unwrap() };
    let circ = c2Circle { p: v(0.0, 0.0), r: 5.0 };
    for p in [v(1.0, 1.0), v(10.0, 10.0), v(3.0, 4.0)] {
        assert_eq!(unsafe { c_fn(circ, p) }, unsafe { r_fn(circ, p) }, "c2CircleToPoint");
    }
}

#[test]
fn test_c2RotIdentity() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn() -> c2r;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2RotIdentity").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2RotIdentity").unwrap() };
    let c = unsafe { c_fn() };
    let r = unsafe { r_fn() };
    assert_f_eq("c2RotIdentity.c", c.c, r.c);
    assert_f_eq("c2RotIdentity.s", c.s, r.s);
}

#[test]
fn test_c2xIdentity() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn() -> c2x;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2xIdentity").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2xIdentity").unwrap() };
    let c = unsafe { c_fn() };
    let r = unsafe { r_fn() };
    assert_v_eq("c2xIdentity.p", c.p, r.p);
    assert_f_eq("c2xIdentity.r.c", c.r.c, r.r.c);
    assert_f_eq("c2xIdentity.r.s", c.r.s, r.r.s);
}

#[test]
fn test_c2Mulrv() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2r, c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2Mulrv").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2Mulrv").unwrap() };
    let rot = c2r { c: 0.6, s: 0.8 };
    let b = v(1.0, 0.0);
    assert_v_eq("c2Mulrv", unsafe { c_fn(rot, b) }, unsafe { r_fn(rot, b) });
}

#[test]
fn test_c2MulrvT() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2r, c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2MulrvT").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2MulrvT").unwrap() };
    let rot = c2r { c: 0.6, s: 0.8 };
    let b = v(1.0, 0.0);
    assert_v_eq("c2MulrvT", unsafe { c_fn(rot, b) }, unsafe { r_fn(rot, b) });
}

#[test]
fn test_c2MulxvT() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2x, c2v) -> c2v;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2MulxvT").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2MulxvT").unwrap() };
    let xf = c2x { p: v(1.0, 2.0), r: c2r { c: 1.0, s: 0.0 } };
    let b = v(3.0, 4.0);
    assert_v_eq("c2MulxvT", unsafe { c_fn(xf, b) }, unsafe { r_fn(xf, b) });
}

// ===== Level 2: Ray-to-shape functions =====

#[test]
fn test_c2RaytoCircle() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2RaytoCircle").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2RaytoCircle").unwrap() };
    let cases = [
        // hit
        (c2Ray { p: v(-5.0, 0.0), d: v(1.0, 0.0), t: 10.0 }, c2Circle { p: v(0.0, 0.0), r: 1.0 }),
        // miss
        (c2Ray { p: v(-5.0, 5.0), d: v(1.0, 0.0), t: 10.0 }, c2Circle { p: v(0.0, 0.0), r: 1.0 }),
        // behind
        (c2Ray { p: v(5.0, 0.0), d: v(1.0, 0.0), t: 10.0 }, c2Circle { p: v(0.0, 0.0), r: 1.0 }),
    ];
    for (ray, circ) in cases {
        let (mut c_out, mut r_out) = unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
        let c_hit = unsafe { c_fn(ray, circ, &mut c_out) };
        let r_hit = unsafe { r_fn(ray, circ, &mut r_out) };
        assert_eq!(c_hit, r_hit, "c2RaytoCircle hit");
        if c_hit != 0 { assert_cast_eq("c2RaytoCircle", c_out, r_out); }
    }
}

#[test]
fn test_c2RaytoAABB() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2RaytoAABB").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2RaytoAABB").unwrap() };
    let cases = [
        // hit from left
        (c2Ray { p: v(-5.0, 0.5), d: v(1.0, 0.0), t: 10.0 }, c2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }),
        // hit from top
        (c2Ray { p: v(0.5, 5.0), d: v(0.0, -1.0), t: 10.0 }, c2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }),
        // miss
        (c2Ray { p: v(-5.0, 5.0), d: v(1.0, 0.0), t: 10.0 }, c2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }),
    ];
    for (ray, aabb) in cases {
        let (mut c_out, mut r_out) = unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
        let c_hit = unsafe { c_fn(ray, aabb, &mut c_out) };
        let r_hit = unsafe { r_fn(ray, aabb, &mut r_out) };
        assert_eq!(c_hit, r_hit, "c2RaytoAABB hit");
        if c_hit != 0 { assert_cast_eq("c2RaytoAABB", c_out, r_out); }
    }
}

#[test]
fn test_c2RaytoCapsule() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2RaytoCapsule").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2RaytoCapsule").unwrap() };
    let cases = [
        // hit side
        (c2Ray { p: v(-5.0, 0.5), d: v(1.0, 0.0), t: 10.0 }, c2Capsule { a: v(0.0, 0.0), b: v(0.0, 2.0), r: 1.0 }),
        // hit endcap
        (c2Ray { p: v(0.0, -5.0), d: v(0.0, 1.0), t: 10.0 }, c2Capsule { a: v(0.0, 0.0), b: v(0.0, 2.0), r: 1.0 }),
        // miss
        (c2Ray { p: v(-5.0, 10.0), d: v(1.0, 0.0), t: 3.0 }, c2Capsule { a: v(0.0, 0.0), b: v(0.0, 2.0), r: 1.0 }),
        // ray starts inside
        (c2Ray { p: v(0.0, 1.0), d: v(1.0, 0.0), t: 10.0 }, c2Capsule { a: v(0.0, 0.0), b: v(0.0, 2.0), r: 1.0 }),
    ];
    for (ray, cap) in cases {
        let (mut c_out, mut r_out) = unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
        let c_hit = unsafe { c_fn(ray, cap, &mut c_out) };
        let r_hit = unsafe { r_fn(ray, cap, &mut r_out) };
        assert_eq!(c_hit, r_hit, "c2RaytoCapsule hit");
        if c_hit != 0 { assert_cast_eq("c2RaytoCapsule", c_out, r_out); }
    }
}

#[test]
fn test_c2RaytoPoly() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2Ray, *const c2Poly, *const c2x, *mut c2Raycast) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2RaytoPoly").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2RaytoPoly").unwrap() };
    let mut poly: c2Poly = unsafe { std::mem::zeroed() };
    poly.count = 4;
    poly.verts[0] = v(0.875, -11.5);
    poly.verts[1] = v(0.875, 11.5);
    poly.verts[2] = v(-0.875, 11.5);
    poly.verts[3] = v(-0.875, -11.5);
    poly.norms[0] = v(1.0, 0.0);
    poly.norms[1] = v(0.0, 1.0);
    poly.norms[2] = v(-1.0, 0.0);
    poly.norms[3] = v(0.0, -1.0);
    let rays = [
        c2Ray { p: v(-3.869416, 13.0693407), d: v(1.0, 0.0), t: 4.0 },
        c2Ray { p: v(-3.869416, 13.0693407), d: v(0.0, -1.0), t: 4.0 },
        c2Ray { p: v(-5.0, 0.0), d: v(1.0, 0.0), t: 10.0 },
        c2Ray { p: v(0.0, 20.0), d: v(0.0, -1.0), t: 50.0 },
    ];
    for ray in rays {
        let (mut c_out, mut r_out) = unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
        let c_hit = unsafe { c_fn(ray, &poly, std::ptr::null(), &mut c_out) };
        let r_hit = unsafe { r_fn(ray, &poly, std::ptr::null(), &mut r_out) };
        assert_eq!(c_hit, r_hit, "c2RaytoPoly hit");
        if c_hit != 0 { assert_cast_eq("c2RaytoPoly", c_out, r_out); }
    }
    // with transform
    let xf = c2x { p: v(5.0, 5.0), r: c2r { c: 0.0, s: 1.0 } };
    let ray = c2Ray { p: v(-5.0, 5.0), d: v(1.0, 0.0), t: 20.0 };
    let (mut c_out, mut r_out) = unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
    let c_hit = unsafe { c_fn(ray, &poly, &xf, &mut c_out) };
    let r_hit = unsafe { r_fn(ray, &poly, &xf, &mut r_out) };
    assert_eq!(c_hit, r_hit, "c2RaytoPoly with xf hit");
    if c_hit != 0 { assert_cast_eq("c2RaytoPoly with xf", c_out, r_out); }
}

// ===== Level 3: Top-level functions =====

#[test]
fn test_c2CastRay() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(c2Ray, *const u8, *const c2x, c_int, *mut c2Raycast) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"c2CastRay").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"c2CastRay").unwrap() };
    // Circle (type 0)
    let circ = c2Circle { p: v(0.0, 0.0), r: 1.0 };
    let ray = c2Ray { p: v(-5.0, 0.0), d: v(1.0, 0.0), t: 10.0 };
    let (mut c_out, mut r_out) = unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
    let c_hit = unsafe { c_fn(ray, &circ as *const _ as *const u8, std::ptr::null(), 0, &mut c_out) };
    let r_hit = unsafe { r_fn(ray, &circ as *const _ as *const u8, std::ptr::null(), 0, &mut r_out) };
    assert_eq!(c_hit, r_hit, "c2CastRay circle");
    if c_hit != 0 { assert_cast_eq("c2CastRay circle", c_out, r_out); }
    // AABB (type 1)
    let aabb = c2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) };
    let ray = c2Ray { p: v(-5.0, 0.5), d: v(1.0, 0.0), t: 10.0 };
    let (mut c_out, mut r_out) = unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
    let c_hit = unsafe { c_fn(ray, &aabb as *const _ as *const u8, std::ptr::null(), 1, &mut c_out) };
    let r_hit = unsafe { r_fn(ray, &aabb as *const _ as *const u8, std::ptr::null(), 1, &mut r_out) };
    assert_eq!(c_hit, r_hit, "c2CastRay aabb");
    if c_hit != 0 { assert_cast_eq("c2CastRay aabb", c_out, r_out); }
    // Capsule (type 2)
    let cap = c2Capsule { a: v(0.0, 0.0), b: v(0.0, 2.0), r: 1.0 };
    let ray = c2Ray { p: v(-5.0, 1.0), d: v(1.0, 0.0), t: 10.0 };
    let (mut c_out, mut r_out) = unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
    let c_hit = unsafe { c_fn(ray, &cap as *const _ as *const u8, std::ptr::null(), 2, &mut c_out) };
    let r_hit = unsafe { r_fn(ray, &cap as *const _ as *const u8, std::ptr::null(), 2, &mut r_out) };
    assert_eq!(c_hit, r_hit, "c2CastRay capsule");
    if c_hit != 0 { assert_cast_eq("c2CastRay capsule", c_out, r_out); }
    // Poly (type 3)
    let mut poly: c2Poly = unsafe { std::mem::zeroed() };
    poly.count = 4;
    poly.verts[0] = v(-1.0, -1.0); poly.verts[1] = v(1.0, -1.0);
    poly.verts[2] = v(1.0, 1.0); poly.verts[3] = v(-1.0, 1.0);
    poly.norms[0] = v(0.0, -1.0); poly.norms[1] = v(1.0, 0.0);
    poly.norms[2] = v(0.0, 1.0); poly.norms[3] = v(-1.0, 0.0);
    let ray = c2Ray { p: v(-5.0, 0.0), d: v(1.0, 0.0), t: 10.0 };
    let (mut c_out, mut r_out) = unsafe { (std::mem::zeroed(), std::mem::zeroed()) };
    let c_hit = unsafe { c_fn(ray, &poly as *const _ as *const u8, std::ptr::null(), 3, &mut c_out) };
    let r_hit = unsafe { r_fn(ray, &poly as *const _ as *const u8, std::ptr::null(), 3, &mut r_out) };
    assert_eq!(c_hit, r_hit, "c2CastRay poly");
    if c_hit != 0 { assert_cast_eq("c2CastRay poly", c_out, r_out); }
}

#[test]
fn test_poly_ray() {
    let (c_lib, r_lib) = load_libs();
    type Fn = unsafe extern "C" fn(*mut c2Raycast, *mut c2Raycast) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { c_lib.get(b"poly_ray").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { r_lib.get(b"poly_ray").unwrap() };
    let (mut c1, mut c2, mut r1, mut r2) = unsafe {
        (std::mem::zeroed(), std::mem::zeroed(), std::mem::zeroed(), std::mem::zeroed())
    };
    let c_hit = unsafe { c_fn(&mut c1, &mut c2) };
    let r_hit = unsafe { r_fn(&mut r1, &mut r2) };
    assert_eq!(c_hit, r_hit, "poly_ray return");
    assert_cast_eq("poly_ray cast1", c1, r1);
    assert_cast_eq("poly_ray cast2", c2, r2);
}
