// FFI integration tests. We load both the C .so and Rust .so via libloading
// and compare outputs byte-for-byte.

mod common;

use common::*;
use libloading::Library;

// ---------- Low-level math helpers ----------

#[test]
fn test_c2V() {
    let (c, r) = load_libs();
    let cf: libloading::Symbol<unsafe extern "C" fn(f32, f32) -> c2v> = get(&c, b"c2V");
    let rf: libloading::Symbol<unsafe extern "C" fn(f32, f32) -> c2v> = get(&r, b"c2V");
    let cases: &[(f32, f32)] = &[
        (0.0, 0.0),
        (1.0, 2.0),
        (-1.5, 3.5),
        (f32::INFINITY, -f32::INFINITY),
        (f32::NAN, 0.0),
        (1e30, 1e-30),
    ];
    unsafe {
        for &(a, b) in cases {
            let cv = cf(a, b);
            let rv = rf(a, b);
            assert_v_eq(cv, rv, &format!("c2V({},{})", a, b));
        }
    }
}

#[test]
fn test_c2Mulvs() {
    let (c, r) = load_libs();
    let cf: libloading::Symbol<unsafe extern "C" fn(c2v, f32) -> c2v> = get(&c, b"c2Mulvs");
    let rf: libloading::Symbol<unsafe extern "C" fn(c2v, f32) -> c2v> = get(&r, b"c2Mulvs");
    let cases: &[(c2v, f32)] = &[
        (c2v { x: 1.0, y: 2.0 }, 3.0),
        (c2v { x: 0.0, y: 0.0 }, 1.0),
        (c2v { x: -1.0, y: 0.5 }, -2.0),
    ];
    unsafe {
        for &(a, s) in cases {
            assert_v_eq(cf(a, s), rf(a, s), "c2Mulvs");
        }
    }
}

#[test]
fn test_c2Maxv_Minv_Clampv() {
    let (c, r) = load_libs();
    let pairs = [
        (c2v { x: 1.0, y: 2.0 }, c2v { x: 3.0, y: 1.0 }),
        (c2v { x: -5.0, y: 5.0 }, c2v { x: 5.0, y: -5.0 }),
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }),
    ];
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = get(&c, b"c2Maxv");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = get(&r, b"c2Maxv");
        for &(a, b) in &pairs {
            assert_v_eq(cf(a, b), rf(a, b), "c2Maxv");
        }
        let cf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = get(&c, b"c2Minv");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = get(&r, b"c2Minv");
        for &(a, b) in &pairs {
            assert_v_eq(cf(a, b), rf(a, b), "c2Minv");
        }
        let cf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v, c2v) -> c2v> = get(&c, b"c2Clampv");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v, c2v) -> c2v> = get(&r, b"c2Clampv");
        let lo = c2v { x: -1.0, y: -1.0 };
        let hi = c2v { x: 1.0, y: 1.0 };
        for &(a, _b) in &pairs {
            assert_v_eq(cf(a, lo, hi), rf(a, lo, hi), "c2Clampv");
        }
    }
}

#[test]
fn test_c2Sub_Add() {
    let (c, r) = load_libs();
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = get(&c, b"c2Sub");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = get(&r, b"c2Sub");
        let a = c2v { x: 5.0, y: 2.0 };
        let b = c2v { x: 1.0, y: 3.0 };
        assert_v_eq(cf(a, b), rf(a, b), "c2Sub");

        let cf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = get(&c, b"c2Add");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = get(&r, b"c2Add");
        assert_v_eq(cf(a, b), rf(a, b), "c2Add");
    }
}

#[test]
fn test_c2Dot_Det2_Len_Div_Norm_Neg_Skew_CCW90_Absv() {
    let (c, r) = load_libs();
    let v = c2v { x: 3.0, y: 4.0 };
    let w = c2v { x: -1.0, y: 2.0 };
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v) -> f32> = get(&c, b"c2Dot");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v) -> f32> = get(&r, b"c2Dot");
        assert_f_eq(cf(v, w), rf(v, w), "c2Dot");

        let cf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v) -> f32> = get(&c, b"c2Det2");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v) -> f32> = get(&r, b"c2Det2");
        assert_f_eq(cf(v, w), rf(v, w), "c2Det2");

        let cf: libloading::Symbol<unsafe extern "C" fn(c2v) -> f32> = get(&c, b"c2Len");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v) -> f32> = get(&r, b"c2Len");
        assert_f_eq(cf(v), rf(v), "c2Len");

        let cf: libloading::Symbol<unsafe extern "C" fn(c2v, f32) -> c2v> = get(&c, b"c2Div");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v, f32) -> c2v> = get(&r, b"c2Div");
        assert_v_eq(cf(v, 2.0), rf(v, 2.0), "c2Div");

        let cf: libloading::Symbol<unsafe extern "C" fn(c2v) -> c2v> = get(&c, b"c2Norm");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v) -> c2v> = get(&r, b"c2Norm");
        assert_v_eq(cf(v), rf(v), "c2Norm");

        let cf: libloading::Symbol<unsafe extern "C" fn(c2v) -> c2v> = get(&c, b"c2Neg");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v) -> c2v> = get(&r, b"c2Neg");
        assert_v_eq(cf(v), rf(v), "c2Neg");

        let cf: libloading::Symbol<unsafe extern "C" fn(c2v) -> c2v> = get(&c, b"c2Skew");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v) -> c2v> = get(&r, b"c2Skew");
        assert_v_eq(cf(v), rf(v), "c2Skew");

        let cf: libloading::Symbol<unsafe extern "C" fn(c2v) -> c2v> = get(&c, b"c2CCW90");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v) -> c2v> = get(&r, b"c2CCW90");
        assert_v_eq(cf(v), rf(v), "c2CCW90");

        let cf: libloading::Symbol<unsafe extern "C" fn(c2v) -> c2v> = get(&c, b"c2Absv");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v) -> c2v> = get(&r, b"c2Absv");
        let neg = c2v { x: -3.0, y: 4.0 };
        assert_v_eq(cf(neg), rf(neg), "c2Absv");
    }
}

#[test]
fn test_c2Mulrv_T_Mulxv_T() {
    let (c, r) = load_libs();
    let rot = c2r { c: 0.6, s: 0.8 };
    let v = c2v { x: 1.0, y: 2.0 };
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(c2r, c2v) -> c2v> = get(&c, b"c2Mulrv");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2r, c2v) -> c2v> = get(&r, b"c2Mulrv");
        assert_v_eq(cf(rot, v), rf(rot, v), "c2Mulrv");

        let cf: libloading::Symbol<unsafe extern "C" fn(c2r, c2v) -> c2v> = get(&c, b"c2MulrvT");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2r, c2v) -> c2v> = get(&r, b"c2MulrvT");
        assert_v_eq(cf(rot, v), rf(rot, v), "c2MulrvT");

        let x = c2x { p: c2v { x: 1.0, y: 2.0 }, r: rot };
        let cf: libloading::Symbol<unsafe extern "C" fn(c2x, c2v) -> c2v> = get(&c, b"c2Mulxv");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2x, c2v) -> c2v> = get(&r, b"c2Mulxv");
        assert_v_eq(cf(x, v), rf(x, v), "c2Mulxv");

        let cf: libloading::Symbol<unsafe extern "C" fn(c2x, c2v) -> c2v> = get(&c, b"c2MulxvT");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2x, c2v) -> c2v> = get(&r, b"c2MulxvT");
        assert_v_eq(cf(x, v), rf(x, v), "c2MulxvT");
    }
}

#[test]
fn test_c2RotIdentity_xIdentity() {
    let (c, r) = load_libs();
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn() -> c2r> = get(&c, b"c2RotIdentity");
        let rf: libloading::Symbol<unsafe extern "C" fn() -> c2r> = get(&r, b"c2RotIdentity");
        let cv = cf();
        let rv = rf();
        assert_f_eq(cv.c, rv.c, "c2RotIdentity.c");
        assert_f_eq(cv.s, rv.s, "c2RotIdentity.s");

        let cf: libloading::Symbol<unsafe extern "C" fn() -> c2x> = get(&c, b"c2xIdentity");
        let rf: libloading::Symbol<unsafe extern "C" fn() -> c2x> = get(&r, b"c2xIdentity");
        let cv = cf();
        let rv = rf();
        assert_v_eq(cv.p, rv.p, "c2xIdentity.p");
        assert_f_eq(cv.r.c, rv.r.c, "c2xIdentity.r.c");
        assert_f_eq(cv.r.s, rv.r.s, "c2xIdentity.r.s");
    }
}

#[test]
fn test_c2Dist() {
    let (c, r) = load_libs();
    let h = c2h { n: c2v { x: 1.0, y: 0.0 }, d: 0.5 };
    let p = c2v { x: 2.0, y: 3.0 };
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(c2h, c2v) -> f32> = get(&c, b"c2Dist");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2h, c2v) -> f32> = get(&r, b"c2Dist");
        assert_f_eq(cf(h, p), rf(h, p), "c2Dist");
    }
}

#[test]
fn test_c2Intersect() {
    let (c, r) = load_libs();
    let a = c2v { x: 0.0, y: 0.0 };
    let b = c2v { x: 4.0, y: 0.0 };
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v, f32, f32) -> c2v> =
            get(&c, b"c2Intersect");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2v, c2v, f32, f32) -> c2v> =
            get(&r, b"c2Intersect");
        assert_v_eq(cf(a, b, 1.0, -1.0), rf(a, b, 1.0, -1.0), "c2Intersect");
        assert_v_eq(cf(a, b, 2.0, -2.0), rf(a, b, 2.0, -2.0), "c2Intersect2");
    }
}

#[test]
fn test_c2BBVerts() {
    let (c, r) = load_libs();
    let mut bb = c2AABB {
        min: c2v { x: 1.0, y: 2.0 },
        max: c2v { x: 5.0, y: 7.0 },
    };
    let mut out_c = [c2v::default(); 4];
    let mut out_r = [c2v::default(); 4];
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(*mut c2v, *mut c2AABB)> = get(&c, b"c2BBVerts");
        let rf: libloading::Symbol<unsafe extern "C" fn(*mut c2v, *mut c2AABB)> = get(&r, b"c2BBVerts");
        cf(out_c.as_mut_ptr(), &mut bb as *mut _);
        rf(out_r.as_mut_ptr(), &mut bb as *mut _);
        for i in 0..4 {
            assert_v_eq(out_c[i], out_r[i], &format!("c2BBVerts[{}]", i));
        }
    }
}

#[test]
fn test_c2MakeProxy() {
    let (c, r) = load_libs();
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(*const std::ffi::c_void, i32, *mut c2Proxy)> =
            get(&c, b"c2MakeProxy");
        let rf: libloading::Symbol<unsafe extern "C" fn(*const std::ffi::c_void, i32, *mut c2Proxy)> =
            get(&r, b"c2MakeProxy");

        // circle
        let circle = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 0.5 };
        let mut pc = c2Proxy::default();
        let mut pr = c2Proxy::default();
        cf(&circle as *const _ as *const _, C2_TYPE_CIRCLE, &mut pc);
        rf(&circle as *const _ as *const _, C2_TYPE_CIRCLE, &mut pr);
        assert_eq!(pc.count, pr.count);
        assert_f_eq(pc.radius, pr.radius, "circle proxy radius");
        for i in 0..pc.count as usize {
            assert_v_eq(pc.verts[i], pr.verts[i], &format!("circle proxy verts[{}]", i));
        }

        // aabb
        let aabb = c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        let mut pc = c2Proxy::default();
        let mut pr = c2Proxy::default();
        cf(&aabb as *const _ as *const _, C2_TYPE_AABB, &mut pc);
        rf(&aabb as *const _ as *const _, C2_TYPE_AABB, &mut pr);
        assert_eq!(pc.count, pr.count);
        assert_f_eq(pc.radius, pr.radius, "aabb proxy radius");
        for i in 0..pc.count as usize {
            assert_v_eq(pc.verts[i], pr.verts[i], &format!("aabb proxy verts[{}]", i));
        }

        // capsule
        let capsule = c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 1.0, y: 1.0 },
            r: 0.25,
        };
        let mut pc = c2Proxy::default();
        let mut pr = c2Proxy::default();
        cf(&capsule as *const _ as *const _, C2_TYPE_CAPSULE, &mut pc);
        rf(&capsule as *const _ as *const _, C2_TYPE_CAPSULE, &mut pr);
        assert_eq!(pc.count, pr.count);
        assert_f_eq(pc.radius, pr.radius, "capsule proxy radius");
        for i in 0..pc.count as usize {
            assert_v_eq(pc.verts[i], pr.verts[i], &format!("capsule proxy verts[{}]", i));
        }
    }
}

#[test]
fn test_c2Support() {
    let (c, r) = load_libs();
    let verts = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 1.0, y: 1.0 },
        c2v { x: 0.0, y: 1.0 },
    ];
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(*const c2v, i32, c2v) -> i32> =
            get(&c, b"c2Support");
        let rf: libloading::Symbol<unsafe extern "C" fn(*const c2v, i32, c2v) -> i32> =
            get(&r, b"c2Support");
        let dirs = [
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: -1.0 },
            c2v { x: 1.0, y: 1.0 },
        ];
        for d in dirs.iter() {
            assert_eq!(
                cf(verts.as_ptr(), 4, *d),
                rf(verts.as_ptr(), 4, *d),
                "c2Support direction {:?}",
                d
            );
        }
    }
}

// ---------- Manifold-level tests ----------

#[test]
fn test_c2CircletoCircleManifold() {
    let (c, r) = load_libs();
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(c2Circle, c2Circle, *mut c2Manifold)> =
            get(&c, b"c2CircletoCircleManifold");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2Circle, c2Circle, *mut c2Manifold)> =
            get(&r, b"c2CircletoCircleManifold");

        let cases = [
            // overlapping
            (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
             c2Circle { p: c2v { x: 1.0, y: 0.0 }, r: 1.0 }),
            // not overlapping
            (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.5 },
             c2Circle { p: c2v { x: 5.0, y: 0.0 }, r: 0.5 }),
            // identical centers
            (c2Circle { p: c2v { x: 1.0, y: 1.0 }, r: 1.0 },
             c2Circle { p: c2v { x: 1.0, y: 1.0 }, r: 0.5 }),
            // touching
            (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
             c2Circle { p: c2v { x: 2.0, y: 0.0 }, r: 1.0 }),
        ];
        for (a, b) in cases.iter() {
            let mut mc = c2Manifold::default();
            let mut mr = c2Manifold::default();
            cf(*a, *b, &mut mc);
            rf(*a, *b, &mut mr);
            assert_manifold_eq(mc, mr, &format!("circ-circ {:?},{:?}", a, b));
        }
    }
}

#[test]
fn test_c2CircletoAABBManifold() {
    let (c, r) = load_libs();
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(c2Circle, c2AABB, *mut c2Manifold)> =
            get(&c, b"c2CircletoAABBManifold");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2Circle, c2AABB, *mut c2Manifold)> =
            get(&r, b"c2CircletoAABBManifold");

        let cases = [
            (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 },
             c2AABB { min: c2v { x: 0.5, y: 0.5 }, max: c2v { x: 2.0, y: 2.0 } }),
            (c2Circle { p: c2v { x: 5.0, y: 5.0 }, r: 1.0 },
             c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } }),
            // circle center inside aabb
            (c2Circle { p: c2v { x: 0.5, y: 0.5 }, r: 0.6 },
             c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } }),
            // circle center on edge
            (c2Circle { p: c2v { x: 1.0, y: 0.5 }, r: 0.4 },
             c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } }),
        ];
        for (a, b) in cases.iter() {
            let mut mc = c2Manifold::default();
            let mut mr = c2Manifold::default();
            cf(*a, *b, &mut mc);
            rf(*a, *b, &mut mr);
            assert_manifold_eq(mc, mr, &format!("circ-aabb {:?},{:?}", a, b));
        }
    }
}

#[test]
fn test_c2AABBtoAABBManifold() {
    let (c, r) = load_libs();
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(c2AABB, c2AABB, *mut c2Manifold)> =
            get(&c, b"c2AABBtoAABBManifold");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2AABB, c2AABB, *mut c2Manifold)> =
            get(&r, b"c2AABBtoAABBManifold");

        let cases = [
            (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 2.0, y: 2.0 } },
             c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: 3.0, y: 3.0 } }),
            (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
             c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: 6.0, y: 6.0 } }),
            // overlap-only on x
            (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 4.0, y: 1.0 } },
             c2AABB { min: c2v { x: 2.0, y: 0.5 }, max: c2v { x: 5.0, y: 0.7 } }),
        ];
        for (a, b) in cases.iter() {
            let mut mc = c2Manifold::default();
            let mut mr = c2Manifold::default();
            cf(*a, *b, &mut mc);
            rf(*a, *b, &mut mr);
            assert_manifold_eq(mc, mr, &format!("aabb-aabb {:?},{:?}", a, b));
        }
    }
}

#[test]
fn test_c2CircletoCapsuleManifold() {
    let (c, r) = load_libs();
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(c2Circle, c2Capsule, *mut c2Manifold)> =
            get(&c, b"c2CircletoCapsuleManifold");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2Circle, c2Capsule, *mut c2Manifold)> =
            get(&r, b"c2CircletoCapsuleManifold");

        let cases = [
            (c2Circle { p: c2v { x: 0.0, y: 0.5 }, r: 1.0 },
             c2Capsule { a: c2v { x: -2.0, y: 0.0 }, b: c2v { x: 2.0, y: 0.0 }, r: 0.25 }),
            (c2Circle { p: c2v { x: 10.0, y: 0.0 }, r: 0.5 },
             c2Capsule { a: c2v { x: -1.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.25 }),
            (c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.5 },
             c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.25 }),
        ];
        for (a, b) in cases.iter() {
            let mut mc = c2Manifold::default();
            let mut mr = c2Manifold::default();
            cf(*a, *b, &mut mc);
            rf(*a, *b, &mut mr);
            assert_manifold_eq(mc, mr, &format!("circ-cap {:?},{:?}", a, b));
        }
    }
}

#[test]
fn test_c2CapsuletoCapsuleManifold() {
    let (c, r) = load_libs();
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(c2Capsule, c2Capsule, *mut c2Manifold)> =
            get(&c, b"c2CapsuletoCapsuleManifold");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2Capsule, c2Capsule, *mut c2Manifold)> =
            get(&r, b"c2CapsuletoCapsuleManifold");

        let cases = [
            (c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 2.0, y: 0.0 }, r: 0.5 },
             c2Capsule { a: c2v { x: 1.0, y: 0.5 }, b: c2v { x: 3.0, y: 0.5 }, r: 0.5 }),
            // far apart
            (c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 0.25 },
             c2Capsule { a: c2v { x: 5.0, y: 5.0 }, b: c2v { x: 6.0, y: 6.0 }, r: 0.25 }),
            // overlapping at angle
            (c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 0.0, y: 2.0 }, r: 0.5 },
             c2Capsule { a: c2v { x: -1.0, y: 1.0 }, b: c2v { x: 1.0, y: 1.0 }, r: 0.4 }),
        ];
        for (a, b) in cases.iter() {
            let mut mc = c2Manifold::default();
            let mut mr = c2Manifold::default();
            cf(*a, *b, &mut mc);
            rf(*a, *b, &mut mr);
            assert_manifold_eq(mc, mr, &format!("cap-cap {:?},{:?}", a, b));
        }
    }
}

#[test]
fn test_c2AABBtoCapsuleManifold() {
    let (c, r) = load_libs();
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(c2AABB, c2Capsule, *mut c2Manifold)> =
            get(&c, b"c2AABBtoCapsuleManifold");
        let rf: libloading::Symbol<unsafe extern "C" fn(c2AABB, c2Capsule, *mut c2Manifold)> =
            get(&r, b"c2AABBtoCapsuleManifold");

        let cases = [
            (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 2.0, y: 2.0 } },
             c2Capsule { a: c2v { x: 1.0, y: -1.0 }, b: c2v { x: 1.0, y: 3.0 }, r: 0.4 }),
            // not overlapping
            (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
             c2Capsule { a: c2v { x: 5.0, y: 5.0 }, b: c2v { x: 6.0, y: 6.0 }, r: 0.25 }),
            // capsule horizontal through middle of aabb
            (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 4.0, y: 4.0 } },
             c2Capsule { a: c2v { x: -1.0, y: 2.0 }, b: c2v { x: 5.0, y: 2.0 }, r: 0.5 }),
        ];
        for (a, b) in cases.iter() {
            let mut mc = c2Manifold::default();
            let mut mr = c2Manifold::default();
            cf(*a, *b, &mut mc);
            rf(*a, *b, &mut mr);
            assert_manifold_eq(mc, mr, &format!("aabb-cap {:?},{:?}", a, b));
        }
    }
}

// ---------- Top-level omni_manifold ----------

fn omni_manifold_call(lib: &Library, type_a: i32, a1: f32, a2: f32, a3: f32, a4: f32, a5: f32,
                      type_b: i32, b1: f32, b2: f32, b3: f32, b4: f32, b5: f32) -> c2Manifold {
    let mut m = c2Manifold::default();
    unsafe {
        let f: libloading::Symbol<unsafe extern "C" fn(*mut c2Manifold, i32, f32, f32, f32, f32, f32,
                                                       i32, f32, f32, f32, f32, f32)> = get(lib, b"omni_manifold");
        f(&mut m, type_a, a1, a2, a3, a4, a5, type_b, b1, b2, b3, b4, b5);
    }
    m
}

#[test]
fn test_omni_manifold_all_pairs() {
    let (c, r) = load_libs();

    // Each tuple: (type, p1..p5)
    // For circle: (x, y, radius, _, _)
    // For aabb: (minx, miny, maxx, maxy, _)
    // For capsule: (ax, ay, bx, by, radius)
    let circle1 = (C2_TYPE_CIRCLE, 0.0_f32, 0.0, 1.0, 0.0, 0.0);
    let circle2 = (C2_TYPE_CIRCLE, 1.5, 0.0, 1.0, 0.0, 0.0);
    let aabb1 = (C2_TYPE_AABB, 0.0_f32, 0.0, 2.0, 2.0, 0.0);
    let aabb2 = (C2_TYPE_AABB, 1.0_f32, 1.0, 3.0, 3.0, 0.0);
    let cap1 = (C2_TYPE_CAPSULE, 0.0_f32, 0.0, 2.0, 0.0, 0.5);
    let cap2 = (C2_TYPE_CAPSULE, 1.0_f32, 1.0, 3.0, 1.0, 0.5);

    let shapes = [circle1, circle2, aabb1, aabb2, cap1, cap2];
    for a in shapes.iter() {
        for b in shapes.iter() {
            let mc = omni_manifold_call(&c, a.0, a.1, a.2, a.3, a.4, a.5,
                                        b.0, b.1, b.2, b.3, b.4, b.5);
            let mr = omni_manifold_call(&r, a.0, a.1, a.2, a.3, a.4, a.5,
                                        b.0, b.1, b.2, b.3, b.4, b.5);
            assert_manifold_eq(mc, mr, &format!("omni_manifold {:?} vs {:?}", a, b));
        }
    }
}

#[test]
fn test_omni_manifold_random_like() {
    let (c, r) = load_libs();
    // Hand-picked combos that exercise edge cases
    let test_cases: &[(i32, f32, f32, f32, f32, f32, i32, f32, f32, f32, f32, f32)] = &[
        // overlapping circles
        (C2_TYPE_CIRCLE, 0.0, 0.0, 2.0, 0.0, 0.0,
         C2_TYPE_CIRCLE, 1.0, 1.0, 1.5, 0.0, 0.0),
        // identical circles
        (C2_TYPE_CIRCLE, 5.0, 5.0, 1.0, 0.0, 0.0,
         C2_TYPE_CIRCLE, 5.0, 5.0, 1.0, 0.0, 0.0),
        // circle inside aabb
        (C2_TYPE_CIRCLE, 1.0, 1.0, 0.3, 0.0, 0.0,
         C2_TYPE_AABB, 0.0, 0.0, 2.0, 2.0, 0.0),
        // aabb inside circle (reversed)
        (C2_TYPE_AABB, 0.0, 0.0, 1.0, 1.0, 0.0,
         C2_TYPE_CIRCLE, 0.5, 0.5, 5.0, 0.0, 0.0),
        // capsule above aabb
        (C2_TYPE_AABB, 0.0, 0.0, 2.0, 2.0, 0.0,
         C2_TYPE_CAPSULE, 0.5, 1.0, 1.5, 1.0, 0.4),
        // skewed capsule into aabb
        (C2_TYPE_AABB, 0.0, 0.0, 4.0, 2.0, 0.0,
         C2_TYPE_CAPSULE, -1.0, 1.0, 3.0, 0.5, 0.3),
        // two circles touching exactly
        (C2_TYPE_CIRCLE, 0.0, 0.0, 1.0, 0.0, 0.0,
         C2_TYPE_CIRCLE, 2.0, 0.0, 1.0, 0.0, 0.0),
        // two circles barely overlapping
        (C2_TYPE_CIRCLE, 0.0, 0.0, 1.0, 0.0, 0.0,
         C2_TYPE_CIRCLE, 1.999, 0.0, 1.0, 0.0, 0.0),
        // circle near aabb corner
        (C2_TYPE_CIRCLE, -0.1, -0.1, 0.3, 0.0, 0.0,
         C2_TYPE_AABB, 0.0, 0.0, 1.0, 1.0, 0.0),
        // two perpendicular capsules
        (C2_TYPE_CAPSULE, 0.0, 0.0, 0.0, 4.0, 0.5,
         C2_TYPE_CAPSULE, -2.0, 2.0, 2.0, 2.0, 0.5),
    ];

    for tc in test_cases {
        let mc = omni_manifold_call(&c, tc.0, tc.1, tc.2, tc.3, tc.4, tc.5,
                                    tc.6, tc.7, tc.8, tc.9, tc.10, tc.11);
        let mr = omni_manifold_call(&r, tc.0, tc.1, tc.2, tc.3, tc.4, tc.5,
                                    tc.6, tc.7, tc.8, tc.9, tc.10, tc.11);
        assert_manifold_eq(mc, mr, &format!("omni_manifold case {:?}", tc));
    }
}

#[test]
fn test_omni_manifold_random_inputs() {
    let (c, r) = load_libs();

    // Pseudo-random generator (deterministic LCG)
    let mut state: u64 = 0xdeadbeefcafebabe;
    let mut next = || -> u64 {
        state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        state
    };
    let rand_f = |s: u64, lo: f32, hi: f32| -> f32 {
        let v = ((s >> 33) as u32) as f32 / (u32::MAX as f32);
        lo + v * (hi - lo)
    };
    let rand_t = |s: u64| -> i32 {
        ((s >> 33) as u32 % 3) as i32
    };

    for _ in 0..100 {
        let ta = match rand_t(next()) { 0 => C2_TYPE_CIRCLE, 1 => C2_TYPE_AABB, _ => C2_TYPE_CAPSULE };
        let tb = match rand_t(next()) { 0 => C2_TYPE_CIRCLE, 1 => C2_TYPE_AABB, _ => C2_TYPE_CAPSULE };
        let mut a = [0.0_f32; 5];
        let mut b = [0.0_f32; 5];
        // Generate non-degenerate values
        match ta {
            C2_TYPE_CIRCLE => {
                a[0] = rand_f(next(),-3.0, 3.0);
                a[1] = rand_f(next(),-3.0, 3.0);
                a[2] = rand_f(next(),0.1, 1.5); // radius
            }
            C2_TYPE_AABB => {
                let cx = rand_f(next(),-3.0, 3.0);
                let cy = rand_f(next(),-3.0, 3.0);
                let hx = rand_f(next(),0.1, 1.5);
                let hy = rand_f(next(),0.1, 1.5);
                a[0] = cx - hx; a[1] = cy - hy;
                a[2] = cx + hx; a[3] = cy + hy;
            }
            C2_TYPE_CAPSULE => {
                a[0] = rand_f(next(),-3.0, 3.0);
                a[1] = rand_f(next(),-3.0, 3.0);
                a[2] = rand_f(next(),-3.0, 3.0);
                a[3] = rand_f(next(),-3.0, 3.0);
                a[4] = rand_f(next(),0.1, 1.0);
            }
            _ => unreachable!(),
        }
        match tb {
            C2_TYPE_CIRCLE => {
                b[0] = rand_f(next(),-3.0, 3.0);
                b[1] = rand_f(next(),-3.0, 3.0);
                b[2] = rand_f(next(),0.1, 1.5);
            }
            C2_TYPE_AABB => {
                let cx = rand_f(next(),-3.0, 3.0);
                let cy = rand_f(next(),-3.0, 3.0);
                let hx = rand_f(next(),0.1, 1.5);
                let hy = rand_f(next(),0.1, 1.5);
                b[0] = cx - hx; b[1] = cy - hy;
                b[2] = cx + hx; b[3] = cy + hy;
            }
            C2_TYPE_CAPSULE => {
                b[0] = rand_f(next(),-3.0, 3.0);
                b[1] = rand_f(next(),-3.0, 3.0);
                b[2] = rand_f(next(),-3.0, 3.0);
                b[3] = rand_f(next(),-3.0, 3.0);
                b[4] = rand_f(next(),0.1, 1.0);
            }
            _ => unreachable!(),
        }

        let mc = omni_manifold_call(&c, ta, a[0], a[1], a[2], a[3], a[4],
                                    tb, b[0], b[1], b[2], b[3], b[4]);
        let mr = omni_manifold_call(&r, ta, a[0], a[1], a[2], a[3], a[4],
                                    tb, b[0], b[1], b[2], b[3], b[4]);
        assert_manifold_eq(mc, mr,
            &format!("omni_manifold ta={} a={:?} tb={} b={:?}", ta, a, tb, b));
    }
}

// Additional GJK direct tests
#[test]
fn test_c2GJK_basic() {
    let (c, r) = load_libs();
    unsafe {
        let cf: libloading::Symbol<unsafe extern "C" fn(*const std::ffi::c_void, i32, *const c2x,
                                                       *const std::ffi::c_void, i32, *const c2x,
                                                       *mut c2v, *mut c2v, i32, *mut i32,
                                                       *mut c2GJKCache) -> f32> = get(&c, b"c2GJK");
        let rf: libloading::Symbol<unsafe extern "C" fn(*const std::ffi::c_void, i32, *const c2x,
                                                       *const std::ffi::c_void, i32, *const c2x,
                                                       *mut c2v, *mut c2v, i32, *mut i32,
                                                       *mut c2GJKCache) -> f32> = get(&r, b"c2GJK");

        // Two non-overlapping circles
        let a = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 1.0 };
        let b = c2Circle { p: c2v { x: 5.0, y: 0.0 }, r: 1.0 };
        let mut ac = c2v::default(); let mut bc = c2v::default(); let mut itc: i32 = 0;
        let mut ar = c2v::default(); let mut br = c2v::default(); let mut itr: i32 = 0;
        let dc = cf(&a as *const _ as *const _, C2_TYPE_CIRCLE, std::ptr::null(),
                    &b as *const _ as *const _, C2_TYPE_CIRCLE, std::ptr::null(),
                    &mut ac, &mut bc, 0, &mut itc, std::ptr::null_mut());
        let dr = rf(&a as *const _ as *const _, C2_TYPE_CIRCLE, std::ptr::null(),
                    &b as *const _ as *const _, C2_TYPE_CIRCLE, std::ptr::null(),
                    &mut ar, &mut br, 0, &mut itr, std::ptr::null_mut());
        assert_f_eq(dc, dr, "c2GJK distance circ-circ");
        assert_v_eq(ac, ar, "c2GJK outA circ-circ");
        assert_v_eq(bc, br, "c2GJK outB circ-circ");
        assert_eq!(itc, itr, "c2GJK iterations circ-circ");
    }
}
