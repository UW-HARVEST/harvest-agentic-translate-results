use libloading::{Library, Symbol};
use std::ffi::c_int;

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
struct C2Ray {
    p: C2v,
    d: C2v,
    t: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Circle {
    p: C2v,
    r: f32,
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
struct C2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Poly {
    count: c_int,
    verts: [C2v; 8],
    norms: [C2v; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2m {
    x: C2v,
    y: C2v,
}

fn c_lib_path() -> String {
    let manifest = env!("CARGO_MANIFEST_DIR");
    format!("{}/c_src/build/libtranslated_rust.so", manifest)
}

fn rust_lib_path() -> String {
    // Find the built Rust cdylib
    let manifest = env!("CARGO_MANIFEST_DIR");
    let target_dir = format!("{}/target/debug", manifest);
    format!("{}/libpoly_ray_lib.so", target_dir)
}

fn assert_f32_eq(label: &str, c: f32, r: f32) {
    assert!(
        c.to_bits() == r.to_bits(),
        "{}: C={} (bits {:08x}) != Rust={} (bits {:08x})",
        label, c, c.to_bits(), r, r.to_bits()
    );
}

fn assert_c2v_eq(label: &str, c: C2v, r: C2v) {
    assert_f32_eq(&format!("{}.x", label), c.x, r.x);
    assert_f32_eq(&format!("{}.y", label), c.y, r.y);
}

fn assert_raycast_eq(label: &str, c: C2Raycast, r: C2Raycast) {
    assert_f32_eq(&format!("{}.t", label), c.t, r.t);
    assert_c2v_eq(&format!("{}.n", label), c.n, r.n);
}

// ============ LOWEST LEVEL FUNCTIONS ============

#[test]
fn test_c2v() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = c_lib.get(b"c2V").unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = r_lib.get(b"c2V").unwrap();

        for (x, y) in [(0.0f32, 0.0), (1.5, -2.3), (-0.875, 11.5), (f32::MAX, f32::MIN)] {
            let c = c_fn(x, y);
            let r = r_fn(x, y);
            assert_c2v_eq("c2V", c, r);
        }
    }
}

#[test]
fn test_c2dot() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = c_lib.get(b"c2Dot").unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = r_lib.get(b"c2Dot").unwrap();

        let a = C2v { x: 3.0, y: 4.0 };
        let b = C2v { x: -1.0, y: 2.0 };
        assert_f32_eq("c2Dot", c_fn(a, b), r_fn(a, b));
    }
}

#[test]
fn test_c2len() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2v) -> f32> = c_lib.get(b"c2Len").unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2v) -> f32> = r_lib.get(b"c2Len").unwrap();

        let a = C2v { x: 3.0, y: 4.0 };
        assert_f32_eq("c2Len", c_fn(a), r_fn(a));
    }
}

#[test]
fn test_c2add_sub_mulvs_div() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let a = C2v { x: 1.5, y: -2.5 };
        let b = C2v { x: 0.5, y: 3.0 };

        // c2Add
        let c_add: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c_lib.get(b"c2Add").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r_lib.get(b"c2Add").unwrap();
        assert_c2v_eq("c2Add", c_add(a, b), r_add(a, b));

        // c2Sub
        let c_sub: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c_lib.get(b"c2Sub").unwrap();
        let r_sub: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r_lib.get(b"c2Sub").unwrap();
        assert_c2v_eq("c2Sub", c_sub(a, b), r_sub(a, b));

        // c2Mulvs
        let c_mul: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = c_lib.get(b"c2Mulvs").unwrap();
        let r_mul: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = r_lib.get(b"c2Mulvs").unwrap();
        assert_c2v_eq("c2Mulvs", c_mul(a, 2.5), r_mul(a, 2.5));

        // c2Div
        let c_div: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = c_lib.get(b"c2Div").unwrap();
        let r_div: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = r_lib.get(b"c2Div").unwrap();
        assert_c2v_eq("c2Div", c_div(a, 2.0), r_div(a, 2.0));
    }
}

#[test]
fn test_c2norm_minv_maxv_skew_absv() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let a = C2v { x: 3.0, y: -4.0 };
        let b = C2v { x: -1.0, y: 2.0 };

        macro_rules! test_v2v {
            ($name:expr, $sym:expr, $($arg:expr),+) => {{
                let c_fn: Symbol<unsafe extern "C" fn($(test_v2v!(@ty $arg)),+) -> C2v> = c_lib.get($sym).unwrap();
                let r_fn: Symbol<unsafe extern "C" fn($(test_v2v!(@ty $arg)),+) -> C2v> = r_lib.get($sym).unwrap();
                assert_c2v_eq($name, c_fn($($arg),+), r_fn($($arg),+));
            }};
            (@ty $e:expr) => { C2v };
        }

        test_v2v!("c2Norm", b"c2Norm", a);
        test_v2v!("c2Minv", b"c2Minv", a, b);
        test_v2v!("c2Maxv", b"c2Maxv", a, b);
        test_v2v!("c2Skew", b"c2Skew", a);
        test_v2v!("c2Absv", b"c2Absv", a);
    }
}

#[test]
fn test_c2ccw90() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2v) -> C2v> = c_lib.get(b"c2CCW90").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2v) -> C2v> = r_lib.get(b"c2CCW90").unwrap();
        let a = C2v { x: 1.0, y: 2.0 };
        assert_c2v_eq("c2CCW90", c_fn(a), r_fn(a));
    }
}

#[test]
fn test_c2mulmvt() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2m, C2v) -> C2v> = c_lib.get(b"c2MulmvT").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2m, C2v) -> C2v> = r_lib.get(b"c2MulmvT").unwrap();
        let m = C2m { x: C2v { x: 1.0, y: 0.0 }, y: C2v { x: 0.0, y: 1.0 } };
        let v = C2v { x: 3.0, y: 4.0 };
        assert_c2v_eq("c2MulmvT", c_fn(m, v), r_fn(m, v));
    }
}

#[test]
fn test_c2rot_identity_and_mulrv() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_rot: Symbol<unsafe extern "C" fn() -> C2r> = c_lib.get(b"c2RotIdentity").unwrap();
        let r_rot: Symbol<unsafe extern "C" fn() -> C2r> = r_lib.get(b"c2RotIdentity").unwrap();
        let cr = c_rot();
        let rr = r_rot();
        assert_f32_eq("c2RotIdentity.c", cr.c, rr.c);
        assert_f32_eq("c2RotIdentity.s", cr.s, rr.s);

        let c_mulrv: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = c_lib.get(b"c2Mulrv").unwrap();
        let r_mulrv: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = r_lib.get(b"c2Mulrv").unwrap();
        let v = C2v { x: 1.0, y: 2.0 };
        assert_c2v_eq("c2Mulrv", c_mulrv(cr, v), r_mulrv(rr, v));

        let c_mulrvt: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = c_lib.get(b"c2MulrvT").unwrap();
        let r_mulrvt: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = r_lib.get(b"c2MulrvT").unwrap();
        assert_c2v_eq("c2MulrvT", c_mulrvt(cr, v), r_mulrvt(rr, v));
    }
}

#[test]
fn test_c2xidentity_and_mulxvt() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();

        let c_xi: Symbol<unsafe extern "C" fn() -> C2x> = c_lib.get(b"c2xIdentity").unwrap();
        let r_xi: Symbol<unsafe extern "C" fn() -> C2x> = r_lib.get(b"c2xIdentity").unwrap();
        let cx = c_xi();
        let rx = r_xi();
        assert_c2v_eq("c2xIdentity.p", cx.p, rx.p);
        assert_f32_eq("c2xIdentity.r.c", cx.r.c, rx.r.c);
        assert_f32_eq("c2xIdentity.r.s", cx.r.s, rx.r.s);

        let c_mulxvt: Symbol<unsafe extern "C" fn(C2x, C2v) -> C2v> = c_lib.get(b"c2MulxvT").unwrap();
        let r_mulxvt: Symbol<unsafe extern "C" fn(C2x, C2v) -> C2v> = r_lib.get(b"c2MulxvT").unwrap();
        let v = C2v { x: 5.0, y: -3.0 };
        assert_c2v_eq("c2MulxvT", c_mulxvt(cx, v), r_mulxvt(rx, v));
    }
}

// ============ MID-LEVEL FUNCTIONS ============

#[test]
fn test_c2aabb_to_aabb() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> = c_lib.get(b"c2AABBtoAABB").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> = r_lib.get(b"c2AABBtoAABB").unwrap();

        let a = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 2.0, y: 2.0 } };
        let b = C2AABB { min: C2v { x: 1.0, y: 1.0 }, max: C2v { x: 3.0, y: 3.0 } };
        let c = C2AABB { min: C2v { x: 5.0, y: 5.0 }, max: C2v { x: 6.0, y: 6.0 } };
        assert_eq!(c_fn(a, b), r_fn(a, b), "overlapping");
        assert_eq!(c_fn(a, c), r_fn(a, c), "non-overlapping");
    }
}

#[test]
fn test_c2aabb_to_point() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2AABB, C2v) -> c_int> = c_lib.get(b"c2AABBtoPoint").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2AABB, C2v) -> c_int> = r_lib.get(b"c2AABBtoPoint").unwrap();

        let a = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 2.0, y: 2.0 } };
        let inside = C2v { x: 1.0, y: 1.0 };
        let outside = C2v { x: 3.0, y: 3.0 };
        assert_eq!(c_fn(a, inside), r_fn(a, inside), "inside");
        assert_eq!(c_fn(a, outside), r_fn(a, outside), "outside");
    }
}

#[test]
fn test_c2circle_to_point() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2Circle, C2v) -> c_int> = c_lib.get(b"c2CircleToPoint").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2Circle, C2v) -> c_int> = r_lib.get(b"c2CircleToPoint").unwrap();

        let circ = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 5.0 };
        let inside = C2v { x: 1.0, y: 1.0 };
        let outside = C2v { x: 10.0, y: 10.0 };
        assert_eq!(c_fn(circ, inside), r_fn(circ, inside), "inside");
        assert_eq!(c_fn(circ, outside), r_fn(circ, outside), "outside");
    }
}

// ============ RAY CAST FUNCTIONS ============

#[test]
fn test_c2ray_to_circle() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int> = c_lib.get(b"c2RaytoCircle").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int> = r_lib.get(b"c2RaytoCircle").unwrap();

        let ray = C2Ray { p: C2v { x: -5.0, y: 0.0 }, d: C2v { x: 1.0, y: 0.0 }, t: 10.0 };
        let circ = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 };
        let mut c_out = std::mem::zeroed::<C2Raycast>();
        let mut r_out = std::mem::zeroed::<C2Raycast>();
        let c_hit = c_fn(ray, circ, &mut c_out);
        let r_hit = r_fn(ray, circ, &mut r_out);
        assert_eq!(c_hit, r_hit, "c2RaytoCircle hit");
        if c_hit != 0 {
            assert_raycast_eq("c2RaytoCircle", c_out, r_out);
        }
    }
}

#[test]
fn test_c2ray_to_aabb() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> c_int> = c_lib.get(b"c2RaytoAABB").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> c_int> = r_lib.get(b"c2RaytoAABB").unwrap();

        let ray = C2Ray { p: C2v { x: -5.0, y: 0.5 }, d: C2v { x: 1.0, y: 0.0 }, t: 10.0 };
        let aabb = C2AABB { min: C2v { x: -1.0, y: -1.0 }, max: C2v { x: 1.0, y: 1.0 } };
        let mut c_out = std::mem::zeroed::<C2Raycast>();
        let mut r_out = std::mem::zeroed::<C2Raycast>();
        let c_hit = c_fn(ray, aabb, &mut c_out);
        let r_hit = r_fn(ray, aabb, &mut r_out);
        assert_eq!(c_hit, r_hit, "c2RaytoAABB hit");
        if c_hit != 0 {
            assert_raycast_eq("c2RaytoAABB", c_out, r_out);
        }
    }
}

#[test]
fn test_c2ray_to_capsule() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int> = c_lib.get(b"c2RaytoCapsule").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int> = r_lib.get(b"c2RaytoCapsule").unwrap();

        let ray = C2Ray { p: C2v { x: -5.0, y: 0.0 }, d: C2v { x: 1.0, y: 0.0 }, t: 10.0 };
        let cap = C2Capsule { a: C2v { x: 0.0, y: -2.0 }, b: C2v { x: 0.0, y: 2.0 }, r: 1.0 };
        let mut c_out = std::mem::zeroed::<C2Raycast>();
        let mut r_out = std::mem::zeroed::<C2Raycast>();
        let c_hit = c_fn(ray, cap, &mut c_out);
        let r_hit = r_fn(ray, cap, &mut r_out);
        assert_eq!(c_hit, r_hit, "c2RaytoCapsule hit");
        if c_hit != 0 {
            assert_raycast_eq("c2RaytoCapsule", c_out, r_out);
        }
    }
}

#[test]
fn test_c2ray_to_poly() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(C2Ray, *const C2Poly, *const C2x, *mut C2Raycast) -> c_int> = c_lib.get(b"c2RaytoPoly").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(C2Ray, *const C2Poly, *const C2x, *mut C2Raycast) -> c_int> = r_lib.get(b"c2RaytoPoly").unwrap();

        let zero = C2v { x: 0.0, y: 0.0 };
        let mut p = C2Poly { count: 4, verts: [zero; 8], norms: [zero; 8] };
        p.verts[0] = C2v { x: 0.875, y: -11.5 };
        p.verts[1] = C2v { x: 0.875, y: 11.5 };
        p.verts[2] = C2v { x: -0.875, y: 11.5 };
        p.verts[3] = C2v { x: -0.875, y: -11.5 };
        p.norms[0] = C2v { x: 1.0, y: 0.0 };
        p.norms[1] = C2v { x: 0.0, y: 1.0 };
        p.norms[2] = C2v { x: -1.0, y: 0.0 };
        p.norms[3] = C2v { x: 0.0, y: -1.0 };

        let ray = C2Ray { p: C2v { x: -3.869416, y: 13.0693407 }, d: C2v { x: 1.0, y: 0.0 }, t: 4.0 };
        let mut c_out = std::mem::zeroed::<C2Raycast>();
        let mut r_out = std::mem::zeroed::<C2Raycast>();
        let c_hit = c_fn(ray, &p, std::ptr::null(), &mut c_out);
        let r_hit = r_fn(ray, &p, std::ptr::null(), &mut r_out);
        assert_eq!(c_hit, r_hit, "c2RaytoPoly hit");
        if c_hit != 0 {
            assert_raycast_eq("c2RaytoPoly", c_out, r_out);
        }
    }
}

// ============ TOP-LEVEL: poly_ray ============

#[test]
fn test_poly_ray() {
    unsafe {
        let c_lib = Library::new(c_lib_path()).unwrap();
        let r_lib = Library::new(rust_lib_path()).unwrap();
        let c_fn: Symbol<unsafe extern "C" fn(*mut C2Raycast, *mut C2Raycast) -> c_int> = c_lib.get(b"poly_ray").unwrap();
        let r_fn: Symbol<unsafe extern "C" fn(*mut C2Raycast, *mut C2Raycast) -> c_int> = r_lib.get(b"poly_ray").unwrap();

        let mut c_cast1 = std::mem::zeroed::<C2Raycast>();
        let mut c_cast2 = std::mem::zeroed::<C2Raycast>();
        let mut r_cast1 = std::mem::zeroed::<C2Raycast>();
        let mut r_cast2 = std::mem::zeroed::<C2Raycast>();

        let c_ret = c_fn(&mut c_cast1, &mut c_cast2);
        let r_ret = r_fn(&mut r_cast1, &mut r_cast2);

        assert_eq!(c_ret, r_ret, "poly_ray return value");
        assert_raycast_eq("poly_ray cast1", c_cast1, r_cast1);
        assert_raycast_eq("poly_ray cast2", c_cast2, r_cast2);
    }
}
