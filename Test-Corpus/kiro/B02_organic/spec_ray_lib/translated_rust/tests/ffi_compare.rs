use libloading::Library;
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2Raycast {
    t: f32,
    n: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Ray {
    p: c2v,
    d: c2v,
    t: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct c2m {
    x: c2v,
    y: c2v,
}

fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn cast_bytes(rc: &c2Raycast) -> [u8; 12] {
    unsafe { std::mem::transmute_copy(rc) }
}

fn c_lib() -> Library {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    unsafe { Library::new(format!("{}/c_src/build/libtranslated_rust.so", manifest)).unwrap() }
}

fn rust_lib() -> Library {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    unsafe { Library::new(format!("{}/target/debug/libspec_ray_lib.so", manifest)).unwrap() }
}

macro_rules! load {
    ($lib:expr, $name:expr, $ty:ty) => {
        unsafe { $lib.get::<$ty>($name.as_bytes()).unwrap() }
    };
}

// ---- Level 0: c2V, c2Dot, c2Add, c2Sub, c2Mulvs ----

#[test]
fn test_c2v() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(f32, f32) -> c2v;
    let cf = load!(c, "c2V", F);
    let rf = load!(r, "c2V", F);
    for (x, y) in [(0.0, 0.0), (1.0, -2.5), (-3.14, 100.0), (f32::MAX, f32::MIN)] {
        let cv = unsafe { cf(x, y) };
        let rv = unsafe { rf(x, y) };
        assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "c2V x mismatch for ({x},{y})");
        assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "c2V y mismatch for ({x},{y})");
    }
}

#[test]
fn test_c2dot() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2v, c2v) -> f32;
    let cf = load!(c, "c2Dot", F);
    let rf = load!(r, "c2Dot", F);
    let cases = [
        (v(1.0, 0.0), v(0.0, 1.0)),
        (v(3.0, 4.0), v(3.0, 4.0)),
        (v(-1.0, 2.0), v(5.0, -3.0)),
        (v(0.0, 0.0), v(0.0, 0.0)),
    ];
    for (a, b) in cases {
        let cv = unsafe { cf(a, b) };
        let rv = unsafe { rf(a, b) };
        assert_eq!(cv.to_bits(), rv.to_bits(), "c2Dot mismatch");
    }
}

#[test]
fn test_c2len() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2v) -> f32;
    let cf = load!(c, "c2Len", F);
    let rf = load!(r, "c2Len", F);
    for a in [v(3.0, 4.0), v(0.0, 0.0), v(-1.0, 1.0), v(100.0, -200.0)] {
        let cv = unsafe { cf(a) };
        let rv = unsafe { rf(a) };
        assert_eq!(cv.to_bits(), rv.to_bits(), "c2Len mismatch for ({},{})", a.x, a.y);
    }
}

#[test]
fn test_c2add_sub_mulvs_div() {
    let c = c_lib();
    let r = rust_lib();
    type Fvv = unsafe extern "C" fn(c2v, c2v) -> c2v;
    type Fvs = unsafe extern "C" fn(c2v, f32) -> c2v;
    let pairs: Vec<(c2v, c2v)> = vec![
        (v(1.0, 2.0), v(3.0, 4.0)),
        (v(-1.0, 0.0), v(0.0, -1.0)),
    ];
    for name in ["c2Add", "c2Sub"] {
        let cf = load!(c, name, Fvv);
        let rf = load!(r, name, Fvv);
        for &(a, b) in &pairs {
            let cv = unsafe { cf(a, b) };
            let rv = unsafe { rf(a, b) };
            assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "{name} x mismatch");
            assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "{name} y mismatch");
        }
    }
    for name in ["c2Mulvs", "c2Div"] {
        let cf = load!(c, name, Fvs);
        let rf = load!(r, name, Fvs);
        for &(a, _) in &pairs {
            for s in [0.5f32, -2.0, 1.0, 0.001] {
                let cv = unsafe { cf(a, s) };
                let rv = unsafe { rf(a, s) };
                assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "{name} x mismatch");
                assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "{name} y mismatch");
            }
        }
    }
}

#[test]
fn test_c2norm_skew_absv_minv_maxv_ccw90() {
    let c = c_lib();
    let r = rust_lib();
    type Fv = unsafe extern "C" fn(c2v) -> c2v;
    type Fvv = unsafe extern "C" fn(c2v, c2v) -> c2v;
    let vecs = [v(3.0, 4.0), v(-1.0, 2.0), v(0.5, -0.5)];
    for name in ["c2Norm", "c2Skew", "c2Absv", "c2CCW90"] {
        let cf = load!(c, name, Fv);
        let rf = load!(r, name, Fv);
        for a in vecs {
            let cv = unsafe { cf(a) };
            let rv = unsafe { rf(a) };
            assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "{name} x mismatch");
            assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "{name} y mismatch");
        }
    }
    let pairs = [(v(1.0, 5.0), v(3.0, 2.0)), (v(-1.0, -1.0), v(0.0, 0.0))];
    for name in ["c2Minv", "c2Maxv"] {
        let cf = load!(c, name, Fvv);
        let rf = load!(r, name, Fvv);
        for (a, b) in pairs {
            let cv = unsafe { cf(a, b) };
            let rv = unsafe { rf(a, b) };
            assert_eq!(cv.x.to_bits(), rv.x.to_bits(), "{name} x mismatch");
            assert_eq!(cv.y.to_bits(), rv.y.to_bits(), "{name} y mismatch");
        }
    }
}

#[test]
fn test_c2mulmvt() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2m, c2v) -> c2v;
    let cf = load!(c, "c2MulmvT", F);
    let rf = load!(r, "c2MulmvT", F);
    let m = c2m { x: v(1.0, 2.0), y: v(3.0, 4.0) };
    let b = v(5.0, 6.0);
    let cv = unsafe { cf(m, b) };
    let rv = unsafe { rf(m, b) };
    assert_eq!(cv.x.to_bits(), rv.x.to_bits());
    assert_eq!(cv.y.to_bits(), rv.y.to_bits());
}

// ---- Level 1: c2AABBtoAABB, c2AABBtoPoint, c2CircleToPoint ----

#[test]
fn test_c2aabb_to_aabb() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
    let cf = load!(c, "c2AABBtoAABB", F);
    let rf = load!(r, "c2AABBtoAABB", F);
    let cases = [
        (c2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) }, c2AABB { min: v(1.0, 1.0), max: v(3.0, 3.0) }),
        (c2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }, c2AABB { min: v(5.0, 5.0), max: v(6.0, 6.0) }),
        (c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) }, c2AABB { min: v(0.0, 0.0), max: v(0.5, 0.5) }),
    ];
    for (a, b) in cases {
        assert_eq!(unsafe { cf(a, b) }, unsafe { rf(a, b) }, "c2AABBtoAABB mismatch");
    }
}

#[test]
fn test_c2aabb_to_point() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2AABB, c2v) -> c_int;
    let cf = load!(c, "c2AABBtoPoint", F);
    let rf = load!(r, "c2AABBtoPoint", F);
    let aabb = c2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) };
    for p in [v(1.0, 1.0), v(-1.0, 1.0), v(3.0, 1.0), v(0.0, 0.0), v(2.0, 2.0)] {
        assert_eq!(unsafe { cf(aabb, p) }, unsafe { rf(aabb, p) }, "c2AABBtoPoint mismatch at ({},{})", p.x, p.y);
    }
}

#[test]
fn test_c2circle_to_point() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2Circle, c2v) -> c_int;
    let cf = load!(c, "c2CircleToPoint", F);
    let rf = load!(r, "c2CircleToPoint", F);
    let circle = c2Circle { p: v(0.0, 0.0), r: 1.0 };
    for p in [v(0.0, 0.0), v(0.5, 0.5), v(1.0, 0.0), v(2.0, 0.0)] {
        assert_eq!(unsafe { cf(circle, p) }, unsafe { rf(circle, p) }, "c2CircleToPoint mismatch");
    }
}

// ---- Level 2: c2RaytoCircle, c2RaytoAABB, c2RaytoCapsule ----

#[test]
fn test_c2ray_to_circle() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2Ray, c2Circle, *mut c2Raycast) -> c_int;
    let cf = load!(c, "c2RaytoCircle", F);
    let rf = load!(r, "c2RaytoCircle", F);
    let cases = [
        // hit
        (c2Ray { p: v(-5.0, 0.0), d: v(1.0, 0.0), t: 100.0 }, c2Circle { p: v(0.0, 0.0), r: 1.0 }),
        // miss
        (c2Ray { p: v(-5.0, 5.0), d: v(1.0, 0.0), t: 100.0 }, c2Circle { p: v(0.0, 0.0), r: 1.0 }),
        // behind
        (c2Ray { p: v(5.0, 0.0), d: v(1.0, 0.0), t: 100.0 }, c2Circle { p: v(0.0, 0.0), r: 1.0 }),
        // diagonal hit
        (c2Ray { p: v(-5.0, -5.0), d: v(0.70710678, 0.70710678), t: 100.0 }, c2Circle { p: v(0.0, 0.0), r: 2.0 }),
    ];
    for (ray, circle) in cases {
        let mut c_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
        let mut r_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
        let c_hit = unsafe { cf(ray, circle, &mut c_out) };
        let r_hit = unsafe { rf(ray, circle, &mut r_out) };
        assert_eq!(c_hit, r_hit, "c2RaytoCircle hit mismatch");
        if c_hit != 0 {
            assert_eq!(cast_bytes(&c_out), cast_bytes(&r_out), "c2RaytoCircle output mismatch");
        }
    }
}

#[test]
fn test_c2ray_to_aabb() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2Ray, c2AABB, *mut c2Raycast) -> c_int;
    let cf = load!(c, "c2RaytoAABB", F);
    let rf = load!(r, "c2RaytoAABB", F);
    let aabb = c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
    let rays = [
        c2Ray { p: v(-5.0, 0.0), d: v(1.0, 0.0), t: 100.0 },
        c2Ray { p: v(0.0, -5.0), d: v(0.0, 1.0), t: 100.0 },
        c2Ray { p: v(-5.0, 5.0), d: v(1.0, 0.0), t: 100.0 },  // miss
        c2Ray { p: v(5.0, 0.0), d: v(1.0, 0.0), t: 100.0 },    // behind
        c2Ray { p: v(-5.0, -5.0), d: v(0.70710678, 0.70710678), t: 100.0 }, // diagonal
    ];
    for ray in rays {
        let mut c_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
        let mut r_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
        let c_hit = unsafe { cf(ray, aabb, &mut c_out) };
        let r_hit = unsafe { rf(ray, aabb, &mut r_out) };
        assert_eq!(c_hit, r_hit, "c2RaytoAABB hit mismatch for ray p=({},{})", ray.p.x, ray.p.y);
        if c_hit != 0 {
            assert_eq!(cast_bytes(&c_out), cast_bytes(&r_out), "c2RaytoAABB output mismatch");
        }
    }
}

#[test]
fn test_c2ray_to_capsule() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2Ray, c2Capsule, *mut c2Raycast) -> c_int;
    let cf = load!(c, "c2RaytoCapsule", F);
    let rf = load!(r, "c2RaytoCapsule", F);
    let capsule = c2Capsule { a: v(0.0, -2.0), b: v(0.0, 2.0), r: 1.0 };
    let rays = [
        c2Ray { p: v(-5.0, 0.0), d: v(1.0, 0.0), t: 100.0 },   // side hit
        c2Ray { p: v(-5.0, -3.0), d: v(1.0, 0.0), t: 100.0 },  // endcap a
        c2Ray { p: v(-5.0, 3.0), d: v(1.0, 0.0), t: 100.0 },   // endcap b
        c2Ray { p: v(-5.0, 10.0), d: v(1.0, 0.0), t: 100.0 },  // miss
        c2Ray { p: v(0.0, 0.0), d: v(1.0, 0.0), t: 100.0 },    // inside
    ];
    for ray in rays {
        let mut c_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
        let mut r_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
        let c_hit = unsafe { cf(ray, capsule, &mut c_out) };
        let r_hit = unsafe { rf(ray, capsule, &mut r_out) };
        assert_eq!(c_hit, r_hit, "c2RaytoCapsule hit mismatch for ray p=({},{})", ray.p.x, ray.p.y);
        if c_hit != 0 {
            assert_eq!(cast_bytes(&c_out), cast_bytes(&r_out),
                "c2RaytoCapsule output mismatch for ray p=({},{}): C={{t={},n=({},{})}}, Rust={{t={},n=({},{})}}",
                ray.p.x, ray.p.y, c_out.t, c_out.n.x, c_out.n.y, r_out.t, r_out.n.x, r_out.n.y);
        }
    }
}

// ---- Level 3: c2CastRay ----

#[test]
fn test_c2cast_ray() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(c2Ray, *const u8, c_int, *mut c2Raycast) -> c_int;
    let cf = load!(c, "c2CastRay", F);
    let rf = load!(r, "c2CastRay", F);
    let ray = c2Ray { p: v(-5.0, 0.0), d: v(1.0, 0.0), t: 100.0 };

    // Circle (type 0)
    let circle = c2Circle { p: v(0.0, 0.0), r: 1.0 };
    let mut c_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
    let mut r_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
    let c_hit = unsafe { cf(ray, &circle as *const _ as *const u8, 0, &mut c_out) };
    let r_hit = unsafe { rf(ray, &circle as *const _ as *const u8, 0, &mut r_out) };
    assert_eq!(c_hit, r_hit, "c2CastRay circle hit mismatch");
    if c_hit != 0 { assert_eq!(cast_bytes(&c_out), cast_bytes(&r_out)); }

    // AABB (type 1)
    let aabb = c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
    let mut c_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
    let mut r_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
    let c_hit = unsafe { cf(ray, &aabb as *const _ as *const u8, 1, &mut c_out) };
    let r_hit = unsafe { rf(ray, &aabb as *const _ as *const u8, 1, &mut r_out) };
    assert_eq!(c_hit, r_hit, "c2CastRay AABB hit mismatch");
    if c_hit != 0 { assert_eq!(cast_bytes(&c_out), cast_bytes(&r_out)); }

    // Capsule (type 2)
    let capsule = c2Capsule { a: v(0.0, -2.0), b: v(0.0, 2.0), r: 1.0 };
    let mut c_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
    let mut r_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
    let c_hit = unsafe { cf(ray, &capsule as *const _ as *const u8, 2, &mut c_out) };
    let r_hit = unsafe { rf(ray, &capsule as *const _ as *const u8, 2, &mut r_out) };
    assert_eq!(c_hit, r_hit, "c2CastRay capsule hit mismatch");
    if c_hit != 0 { assert_eq!(cast_bytes(&c_out), cast_bytes(&r_out)); }
}

// ---- Level 4: spec_ray (the public API) ----

#[test]
fn test_spec_ray() {
    let c = c_lib();
    let r = rust_lib();
    type F = unsafe extern "C" fn(*mut c2Raycast, f32, f32, f32, f32, f32, f32, f32) -> c_int;
    let cf = load!(c, "spec_ray", F);
    let rf = load!(r, "spec_ray", F);
    let cases: Vec<(f32, f32, f32, f32, f32, f32, f32)> = vec![
        // (mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y)
        (5.0, 0.0, 3.0, 0.0, 1.0, 0.0, 0.0),       // hit
        (5.0, 10.0, 3.0, 0.0, 1.0, 0.0, 0.0),       // miss
        (10.0, 0.0, 5.0, 0.0, 2.0, 0.0, 0.0),       // hit larger circle
        (1.0, 1.0, 0.0, 0.0, 0.5, -5.0, -5.0),      // diagonal
        (0.0, 0.0, 0.0, 0.0, 1.0, -2.0, 0.0),       // toward origin
        (3.0, 4.0, 2.0, 3.0, 1.5, 0.0, 0.0),        // arbitrary
        (-5.0, -5.0, -3.0, -3.0, 2.0, 0.0, 0.0),    // negative coords
    ];
    for (mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y) in cases {
        let mut c_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
        let mut r_out = c2Raycast { t: 0.0, n: v(0.0, 0.0) };
        let c_hit = unsafe { cf(&mut c_out, mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y) };
        let r_hit = unsafe { rf(&mut r_out, mp_x, mp_y, c_p_x, c_p_y, c_r, r_p_x, r_p_y) };
        assert_eq!(c_hit, r_hit, "spec_ray hit mismatch for mp=({mp_x},{mp_y})");
        if c_hit != 0 {
            assert_eq!(cast_bytes(&c_out), cast_bytes(&r_out),
                "spec_ray output mismatch for mp=({mp_x},{mp_y}): C={{t={},n=({},{})}}, Rust={{t={},n=({},{})}}",
                c_out.t, c_out.n.x, c_out.n.y, r_out.t, r_out.n.x, r_out.n.y);
        }
    }
}
