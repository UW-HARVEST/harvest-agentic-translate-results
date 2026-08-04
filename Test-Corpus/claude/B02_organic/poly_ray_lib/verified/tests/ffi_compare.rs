use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2v {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2Raycast {
    pub t: f32,
    pub n: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2r {
    pub c: f32,
    pub s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2x {
    pub p: C2v,
    pub r: C2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2Circle {
    pub p: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2AABB {
    pub min: C2v,
    pub max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2Capsule {
    pub a: C2v,
    pub b: C2v,
    pub r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct C2Poly {
    pub count: c_int,
    pub verts: [C2v; 8],
    pub norms: [C2v; 8],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2Ray {
    pub p: C2v,
    pub d: C2v,
    pub t: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct C2m {
    pub x: C2v,
    pub y: C2v,
}

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;
const C2_TYPE_POLY: c_int = 3;

fn c_lib_path() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("c_src");
    p.push("build");
    p.push("libtranslated_rust.so");
    p
}

fn rust_lib_path() -> PathBuf {
    // Tests build crate as cdylib, library is in target/<profile>/
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.push("target");
    // Try debug first then release.
    let debug = p.join("debug").join("libpoly_ray_lib.so");
    if debug.exists() {
        return debug;
    }
    p.push("release");
    p.push("libpoly_ray_lib.so");
    p
}

fn libs() -> (Library, Library) {
    // The C library uses sqrtf but isn't linked with -lm. Pull libm into the
    // process with RTLD_GLOBAL so the C lib can resolve `sqrtf`.
    unsafe {
        // Try several common libm locations.
        let candidates: &[&[u8]] = &[
            b"libm.so.6\0",
            b"libm.so\0",
            b"/lib/x86_64-linux-gnu/libm.so.6\0",
            b"/usr/lib/x86_64-linux-gnu/libm.so.6\0",
        ];
        for name in candidates {
            // RTLD_NOW | RTLD_GLOBAL = 2 | 0x100 = 258 on glibc/Linux.
            let h = libc_dlopen(name.as_ptr() as *const _, 0x102);
            if !h.is_null() {
                break;
            }
        }
    }
    let c_lib = unsafe { Library::new(c_lib_path()).expect("failed to load C lib") };
    let r_lib = unsafe { Library::new(rust_lib_path()).expect("failed to load Rust lib") };
    (c_lib, r_lib)
}

extern "C" {
    fn dlopen(filename: *const i8, flag: c_int) -> *mut std::ffi::c_void;
}

unsafe fn libc_dlopen(name: *const i8, flag: c_int) -> *mut std::ffi::c_void {
    dlopen(name, flag)
}

fn bits(x: f32) -> u32 {
    x.to_bits()
}

fn vec_eq(a: C2v, b: C2v) -> bool {
    bits(a.x) == bits(b.x) && bits(a.y) == bits(b.y)
}

fn raycast_eq(a: C2Raycast, b: C2Raycast) -> bool {
    bits(a.t) == bits(b.t) && vec_eq(a.n, b.n)
}

#[test]
fn test_c2v() {
    let (c, r) = libs();
    let c_fn: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> =
        unsafe { c.get(b"c2V").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> =
        unsafe { r.get(b"c2V").unwrap() };
    for &(x, y) in &[(0.0, 0.0), (1.0, -1.0), (-3.5, 7.25), (1e10, -1e-10)] {
        let cv = unsafe { c_fn(x, y) };
        let rv = unsafe { r_fn(x, y) };
        assert!(vec_eq(cv, rv), "c2V({},{}) C={:?} R={:?}", x, y, cv, rv);
    }
}

#[test]
fn test_c2dot() {
    let (c, r) = libs();
    let c_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> =
        unsafe { c.get(b"c2Dot").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> =
        unsafe { r.get(b"c2Dot").unwrap() };
    let cases = [
        (C2v { x: 0.0, y: 0.0 }, C2v { x: 0.0, y: 0.0 }),
        (C2v { x: 1.0, y: 0.0 }, C2v { x: 0.0, y: 1.0 }),
        (C2v { x: 3.0, y: 4.0 }, C2v { x: 5.0, y: 6.0 }),
        (C2v { x: -3.0, y: 4.5 }, C2v { x: 1.5, y: -2.5 }),
    ];
    for (a, b) in cases {
        let cv = unsafe { c_fn(a, b) };
        let rv = unsafe { r_fn(a, b) };
        assert_eq!(bits(cv), bits(rv), "c2Dot mismatch");
    }
}

#[test]
fn test_c2len() {
    let (c, r) = libs();
    let c_fn: Symbol<unsafe extern "C" fn(C2v) -> f32> =
        unsafe { c.get(b"c2Len").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(C2v) -> f32> =
        unsafe { r.get(b"c2Len").unwrap() };
    for v in [
        C2v { x: 0.0, y: 0.0 },
        C2v { x: 3.0, y: 4.0 },
        C2v { x: -3.0, y: -4.0 },
        C2v { x: 1e10, y: 1e10 },
    ] {
        let cv = unsafe { c_fn(v) };
        let rv = unsafe { r_fn(v) };
        assert_eq!(bits(cv), bits(rv));
    }
}

#[test]
fn test_c2_arith() {
    let (c, r) = libs();
    let c_add: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> =
        unsafe { c.get(b"c2Add").unwrap() };
    let r_add: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> =
        unsafe { r.get(b"c2Add").unwrap() };
    let c_sub: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> =
        unsafe { c.get(b"c2Sub").unwrap() };
    let r_sub: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> =
        unsafe { r.get(b"c2Sub").unwrap() };
    let c_mulvs: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> =
        unsafe { c.get(b"c2Mulvs").unwrap() };
    let r_mulvs: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> =
        unsafe { r.get(b"c2Mulvs").unwrap() };
    let c_div: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> =
        unsafe { c.get(b"c2Div").unwrap() };
    let r_div: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> =
        unsafe { r.get(b"c2Div").unwrap() };
    let pairs = [
        (C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: 4.0 }),
        (C2v { x: -1.5, y: 2.25 }, C2v { x: 0.5, y: -7.0 }),
    ];
    for (a, b) in pairs {
        assert!(vec_eq(unsafe { c_add(a, b) }, unsafe { r_add(a, b) }));
        assert!(vec_eq(unsafe { c_sub(a, b) }, unsafe { r_sub(a, b) }));
        assert!(vec_eq(unsafe { c_mulvs(a, 2.5) }, unsafe { r_mulvs(a, 2.5) }));
        assert!(vec_eq(unsafe { c_div(a, 4.0) }, unsafe { r_div(a, 4.0) }));
    }
}

#[test]
fn test_c2_norm_minv_maxv_skew_absv_ccw90() {
    let (c, r) = libs();
    let c_norm: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { c.get(b"c2Norm").unwrap() };
    let r_norm: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { r.get(b"c2Norm").unwrap() };
    let c_minv: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = unsafe { c.get(b"c2Minv").unwrap() };
    let r_minv: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = unsafe { r.get(b"c2Minv").unwrap() };
    let c_maxv: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = unsafe { c.get(b"c2Maxv").unwrap() };
    let r_maxv: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = unsafe { r.get(b"c2Maxv").unwrap() };
    let c_skew: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { c.get(b"c2Skew").unwrap() };
    let r_skew: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { r.get(b"c2Skew").unwrap() };
    let c_absv: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { c.get(b"c2Absv").unwrap() };
    let r_absv: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { r.get(b"c2Absv").unwrap() };
    let c_ccw: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { c.get(b"c2CCW90").unwrap() };
    let r_ccw: Symbol<unsafe extern "C" fn(C2v) -> C2v> = unsafe { r.get(b"c2CCW90").unwrap() };
    let pairs = [
        (C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: 4.0 }),
        (C2v { x: -1.5, y: 2.25 }, C2v { x: 0.5, y: -7.0 }),
        (C2v { x: 0.0, y: -3.7 }, C2v { x: 4.4, y: 0.0 }),
    ];
    for (a, b) in pairs {
        assert!(vec_eq(unsafe { c_norm(a) }, unsafe { r_norm(a) }));
        assert!(vec_eq(unsafe { c_minv(a, b) }, unsafe { r_minv(a, b) }));
        assert!(vec_eq(unsafe { c_maxv(a, b) }, unsafe { r_maxv(a, b) }));
        assert!(vec_eq(unsafe { c_skew(a) }, unsafe { r_skew(a) }));
        assert!(vec_eq(unsafe { c_absv(a) }, unsafe { r_absv(a) }));
        assert!(vec_eq(unsafe { c_ccw(a) }, unsafe { r_ccw(a) }));
    }
}

#[test]
fn test_aabb_to_aabb_and_point() {
    let (c, r) = libs();
    let c_aa: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> =
        unsafe { c.get(b"c2AABBtoAABB").unwrap() };
    let r_aa: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> =
        unsafe { r.get(b"c2AABBtoAABB").unwrap() };
    let c_ap: Symbol<unsafe extern "C" fn(C2AABB, C2v) -> c_int> =
        unsafe { c.get(b"c2AABBtoPoint").unwrap() };
    let r_ap: Symbol<unsafe extern "C" fn(C2AABB, C2v) -> c_int> =
        unsafe { r.get(b"c2AABBtoPoint").unwrap() };
    let cases_aa = [
        (
            C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
            C2AABB { min: C2v { x: 0.5, y: 0.5 }, max: C2v { x: 2.0, y: 2.0 } },
        ),
        (
            C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
            C2AABB { min: C2v { x: 1.5, y: 0.0 }, max: C2v { x: 2.0, y: 1.0 } },
        ),
    ];
    for (a, b) in cases_aa {
        assert_eq!(unsafe { c_aa(a, b) }, unsafe { r_aa(a, b) });
    }
    let cases_ap = [
        (
            C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
            C2v { x: 0.5, y: 0.5 },
        ),
        (
            C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
            C2v { x: -0.5, y: 0.5 },
        ),
    ];
    for (a, b) in cases_ap {
        assert_eq!(unsafe { c_ap(a, b) }, unsafe { r_ap(a, b) });
    }
}

#[test]
fn test_circle_to_point() {
    let (c, r) = libs();
    let c_fn: Symbol<unsafe extern "C" fn(C2Circle, C2v) -> c_int> =
        unsafe { c.get(b"c2CircleToPoint").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(C2Circle, C2v) -> c_int> =
        unsafe { r.get(b"c2CircleToPoint").unwrap() };
    for (circle, point) in [
        (
            C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 },
            C2v { x: 0.5, y: 0.0 },
        ),
        (
            C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 },
            C2v { x: 2.0, y: 0.0 },
        ),
        (
            C2Circle { p: C2v { x: 5.0, y: 5.0 }, r: 3.0 },
            C2v { x: 4.0, y: 5.0 },
        ),
    ] {
        assert_eq!(unsafe { c_fn(circle, point) }, unsafe { r_fn(circle, point) });
    }
}

#[test]
fn test_rot_and_xform_identity() {
    let (c, r) = libs();
    let c_fn: Symbol<unsafe extern "C" fn() -> C2r> = unsafe { c.get(b"c2RotIdentity").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn() -> C2r> = unsafe { r.get(b"c2RotIdentity").unwrap() };
    let cv = unsafe { c_fn() };
    let rv = unsafe { r_fn() };
    assert_eq!(bits(cv.c), bits(rv.c));
    assert_eq!(bits(cv.s), bits(rv.s));

    let c_fn: Symbol<unsafe extern "C" fn() -> C2x> = unsafe { c.get(b"c2xIdentity").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn() -> C2x> = unsafe { r.get(b"c2xIdentity").unwrap() };
    let cv = unsafe { c_fn() };
    let rv = unsafe { r_fn() };
    assert!(vec_eq(cv.p, rv.p));
    assert_eq!(bits(cv.r.c), bits(rv.r.c));
    assert_eq!(bits(cv.r.s), bits(rv.r.s));
}

#[test]
fn test_mulrv_mulrvT_mulxvT_mulmvT_mulrv() {
    let (c, r) = libs();
    let c_mulrv: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> =
        unsafe { c.get(b"c2Mulrv").unwrap() };
    let r_mulrv: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> =
        unsafe { r.get(b"c2Mulrv").unwrap() };
    let c_mulrvt: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> =
        unsafe { c.get(b"c2MulrvT").unwrap() };
    let r_mulrvt: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> =
        unsafe { r.get(b"c2MulrvT").unwrap() };
    let c_mulxvt: Symbol<unsafe extern "C" fn(C2x, C2v) -> C2v> =
        unsafe { c.get(b"c2MulxvT").unwrap() };
    let r_mulxvt: Symbol<unsafe extern "C" fn(C2x, C2v) -> C2v> =
        unsafe { r.get(b"c2MulxvT").unwrap() };
    let c_mulmvt: Symbol<unsafe extern "C" fn(C2m, C2v) -> C2v> =
        unsafe { c.get(b"c2MulmvT").unwrap() };
    let r_mulmvt: Symbol<unsafe extern "C" fn(C2m, C2v) -> C2v> =
        unsafe { r.get(b"c2MulmvT").unwrap() };
    let r = C2r { c: 0.6, s: 0.8 };
    let xform = C2x {
        p: C2v { x: 1.0, y: 2.0 },
        r,
    };
    let m = C2m {
        x: C2v { x: 0.6, y: -0.8 },
        y: C2v { x: 0.8, y: 0.6 },
    };
    for v in [
        C2v { x: 1.0, y: 0.0 },
        C2v { x: 0.0, y: 1.0 },
        C2v { x: 1.5, y: -2.5 },
    ] {
        assert!(vec_eq(unsafe { c_mulrv(r, v) }, unsafe { r_mulrv(r, v) }));
        assert!(vec_eq(unsafe { c_mulrvt(r, v) }, unsafe { r_mulrvt(r, v) }));
        assert!(vec_eq(
            unsafe { c_mulxvt(xform, v) },
            unsafe { r_mulxvt(xform, v) }
        ));
        assert!(vec_eq(
            unsafe { c_mulmvt(m, v) },
            unsafe { r_mulmvt(m, v) }
        ));
    }
}

#[test]
fn test_ray_to_circle() {
    let (c, r) = libs();
    let c_fn: Symbol<unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int> =
        unsafe { c.get(b"c2RaytoCircle").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(C2Ray, C2Circle, *mut C2Raycast) -> c_int> =
        unsafe { r.get(b"c2RaytoCircle").unwrap() };
    let cases = [
        (
            C2Ray { p: C2v { x: -3.0, y: 0.0 }, d: C2v { x: 1.0, y: 0.0 }, t: 5.0 },
            C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 },
        ),
        (
            C2Ray { p: C2v { x: -3.0, y: 5.0 }, d: C2v { x: 1.0, y: 0.0 }, t: 5.0 },
            C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 },
        ),
        (
            C2Ray { p: C2v { x: -3.0, y: 0.0 }, d: C2v { x: 1.0, y: 0.0 }, t: 1.0 },
            C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 },
        ),
    ];
    for (ray, circle) in cases {
        let mut co = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
        let mut ro = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
        let cr = unsafe { c_fn(ray, circle, &mut co) };
        let rr = unsafe { r_fn(ray, circle, &mut ro) };
        assert_eq!(cr, rr);
        if cr != 0 {
            assert!(raycast_eq(co, ro));
        }
    }
}

#[test]
fn test_ray_to_aabb() {
    let (c, r) = libs();
    let c_fn: Symbol<unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> c_int> =
        unsafe { c.get(b"c2RaytoAABB").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(C2Ray, C2AABB, *mut C2Raycast) -> c_int> =
        unsafe { r.get(b"c2RaytoAABB").unwrap() };
    let cases = [
        (
            C2Ray { p: C2v { x: -3.0, y: 0.5 }, d: C2v { x: 1.0, y: 0.0 }, t: 10.0 },
            C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
        ),
        (
            C2Ray { p: C2v { x: -3.0, y: 5.0 }, d: C2v { x: 1.0, y: 0.0 }, t: 10.0 },
            C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
        ),
        (
            C2Ray { p: C2v { x: 0.5, y: -3.0 }, d: C2v { x: 0.0, y: 1.0 }, t: 10.0 },
            C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } },
        ),
    ];
    for (ray, aabb) in cases {
        let mut co = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
        let mut ro = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
        let cr = unsafe { c_fn(ray, aabb, &mut co) };
        let rr = unsafe { r_fn(ray, aabb, &mut ro) };
        assert_eq!(cr, rr);
        if cr != 0 {
            assert!(raycast_eq(co, ro), "C={:?} R={:?}", co, ro);
        }
    }
}

#[test]
fn test_ray_to_capsule() {
    let (c, r) = libs();
    let c_fn: Symbol<unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int> =
        unsafe { c.get(b"c2RaytoCapsule").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(C2Ray, C2Capsule, *mut C2Raycast) -> c_int> =
        unsafe { r.get(b"c2RaytoCapsule").unwrap() };
    let cases = [
        (
            C2Ray { p: C2v { x: -3.0, y: 0.5 }, d: C2v { x: 1.0, y: 0.0 }, t: 10.0 },
            C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 0.0, y: 2.0 }, r: 0.5 },
        ),
        (
            C2Ray { p: C2v { x: -3.0, y: 10.0 }, d: C2v { x: 1.0, y: 0.0 }, t: 10.0 },
            C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 0.0, y: 2.0 }, r: 0.5 },
        ),
    ];
    for (ray, capsule) in cases {
        let mut co = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
        let mut ro = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
        let cr = unsafe { c_fn(ray, capsule, &mut co) };
        let rr = unsafe { r_fn(ray, capsule, &mut ro) };
        assert_eq!(cr, rr);
        if cr != 0 {
            assert!(raycast_eq(co, ro), "C={:?} R={:?}", co, ro);
        }
    }
}

#[test]
fn test_ray_to_poly() {
    let (c, r) = libs();
    let c_fn: Symbol<
        unsafe extern "C" fn(C2Ray, *const C2Poly, *const C2x, *mut C2Raycast) -> c_int,
    > = unsafe { c.get(b"c2RaytoPoly").unwrap() };
    let r_fn: Symbol<
        unsafe extern "C" fn(C2Ray, *const C2Poly, *const C2x, *mut C2Raycast) -> c_int,
    > = unsafe { r.get(b"c2RaytoPoly").unwrap() };
    let mut p = C2Poly {
        count: 4,
        verts: [C2v { x: 0.0, y: 0.0 }; 8],
        norms: [C2v { x: 0.0, y: 0.0 }; 8],
    };
    p.verts[0] = C2v { x: 0.875, y: -11.5 };
    p.verts[1] = C2v { x: 0.875, y: 11.5 };
    p.verts[2] = C2v { x: -0.875, y: 11.5 };
    p.verts[3] = C2v { x: -0.875, y: -11.5 };
    p.norms[0] = C2v { x: 1.0, y: 0.0 };
    p.norms[1] = C2v { x: 0.0, y: 1.0 };
    p.norms[2] = C2v { x: -1.0, y: 0.0 };
    p.norms[3] = C2v { x: 0.0, y: -1.0 };

    let rays = [
        C2Ray { p: C2v { x: -3.869416, y: 13.0693407 }, d: C2v { x: 1.0, y: 0.0 }, t: 4.0 },
        C2Ray { p: C2v { x: -3.869416, y: 13.0693407 }, d: C2v { x: 0.0, y: -1.0 }, t: 4.0 },
        C2Ray { p: C2v { x: -2.0, y: 0.0 }, d: C2v { x: 1.0, y: 0.0 }, t: 5.0 },
    ];
    for ray in rays {
        let mut co = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
        let mut ro = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
        let cr = unsafe { c_fn(ray, &p, std::ptr::null(), &mut co) };
        let rr = unsafe { r_fn(ray, &p, std::ptr::null(), &mut ro) };
        assert_eq!(cr, rr);
        if cr != 0 {
            assert!(raycast_eq(co, ro), "C={:?} R={:?}", co, ro);
        }
    }
}

#[test]
fn test_cast_ray() {
    let (c, r) = libs();
    let c_fn: Symbol<
        unsafe extern "C" fn(
            C2Ray,
            *const std::ffi::c_void,
            *const C2x,
            c_int,
            *mut C2Raycast,
        ) -> c_int,
    > = unsafe { c.get(b"c2CastRay").unwrap() };
    let r_fn: Symbol<
        unsafe extern "C" fn(
            C2Ray,
            *const std::ffi::c_void,
            *const C2x,
            c_int,
            *mut C2Raycast,
        ) -> c_int,
    > = unsafe { r.get(b"c2CastRay").unwrap() };

    // circle
    let ray = C2Ray { p: C2v { x: -3.0, y: 0.0 }, d: C2v { x: 1.0, y: 0.0 }, t: 5.0 };
    let circle = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 };
    let mut co = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
    let mut ro = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
    let cr = unsafe {
        c_fn(
            ray,
            &circle as *const _ as *const std::ffi::c_void,
            std::ptr::null(),
            C2_TYPE_CIRCLE,
            &mut co,
        )
    };
    let rr = unsafe {
        r_fn(
            ray,
            &circle as *const _ as *const std::ffi::c_void,
            std::ptr::null(),
            C2_TYPE_CIRCLE,
            &mut ro,
        )
    };
    assert_eq!(cr, rr);
    if cr != 0 {
        assert!(raycast_eq(co, ro));
    }

    // aabb
    let aabb = C2AABB { min: C2v { x: 0.0, y: 0.0 }, max: C2v { x: 1.0, y: 1.0 } };
    let ray = C2Ray { p: C2v { x: -3.0, y: 0.5 }, d: C2v { x: 1.0, y: 0.0 }, t: 10.0 };
    let mut co = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
    let mut ro = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
    let cr = unsafe {
        c_fn(
            ray,
            &aabb as *const _ as *const std::ffi::c_void,
            std::ptr::null(),
            C2_TYPE_AABB,
            &mut co,
        )
    };
    let rr = unsafe {
        r_fn(
            ray,
            &aabb as *const _ as *const std::ffi::c_void,
            std::ptr::null(),
            C2_TYPE_AABB,
            &mut ro,
        )
    };
    assert_eq!(cr, rr);
    if cr != 0 {
        assert!(raycast_eq(co, ro));
    }

    // capsule
    let capsule = C2Capsule {
        a: C2v { x: 0.0, y: 0.0 },
        b: C2v { x: 0.0, y: 2.0 },
        r: 0.5,
    };
    let ray = C2Ray { p: C2v { x: -3.0, y: 0.5 }, d: C2v { x: 1.0, y: 0.0 }, t: 10.0 };
    let mut co = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
    let mut ro = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
    let cr = unsafe {
        c_fn(
            ray,
            &capsule as *const _ as *const std::ffi::c_void,
            std::ptr::null(),
            C2_TYPE_CAPSULE,
            &mut co,
        )
    };
    let rr = unsafe {
        r_fn(
            ray,
            &capsule as *const _ as *const std::ffi::c_void,
            std::ptr::null(),
            C2_TYPE_CAPSULE,
            &mut ro,
        )
    };
    assert_eq!(cr, rr);
    if cr != 0 {
        assert!(raycast_eq(co, ro));
    }
}

#[test]
fn test_poly_ray() {
    let (c, r) = libs();
    let c_fn: Symbol<unsafe extern "C" fn(*mut C2Raycast, *mut C2Raycast) -> c_int> =
        unsafe { c.get(b"poly_ray").unwrap() };
    let r_fn: Symbol<unsafe extern "C" fn(*mut C2Raycast, *mut C2Raycast) -> c_int> =
        unsafe { r.get(b"poly_ray").unwrap() };

    let mut c1 = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
    let mut c2 = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
    let mut r1 = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };
    let mut r2 = C2Raycast { t: 0.0, n: C2v { x: 0.0, y: 0.0 } };

    let cret = unsafe { c_fn(&mut c1, &mut c2) };
    let rret = unsafe { r_fn(&mut r1, &mut r2) };
    assert_eq!(cret, rret, "poly_ray return mismatch");
    // We compare only outputs that were updated by the call. The C/Rust functions
    // both write the output unconditionally for AABB-style hits but leave
    // unchanged on miss — but since both paths run identically, the full memory
    // contents should match.
    assert!(raycast_eq(c1, r1), "cast1 mismatch C={:?} R={:?}", c1, r1);
    assert!(raycast_eq(c2, r2), "cast2 mismatch C={:?} R={:?}", c2, r2);
}
