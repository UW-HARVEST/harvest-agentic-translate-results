use libloading::{Library, Symbol};
use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2v { x: f32, y: f32 }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2r { c: f32, s: f32 }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2x { p: C2v, r: C2r }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Circle { p: C2v, r: f32 }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2AABB { min: C2v, max: C2v }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct C2Capsule { a: C2v, b: C2v, r: f32 }

fn v(x: f32, y: f32) -> C2v { C2v { x, y } }

fn libs() -> (Library, Library) {
    unsafe {
        let c = Library::new("c_src/build/libtranslated_rust.so").expect("load C .so");
        let r = Library::new("target/debug/libcapsule_lib.so").expect("load Rust .so");
        (c, r)
    }
}

fn assert_v_eq(a: C2v, b: C2v, ctx: &str) {
    assert!(a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits(),
        "{ctx}: C=({},{}) Rust=({},{})", a.x, a.y, b.x, b.y);
}

fn assert_f_eq(a: f32, b: f32, ctx: &str) {
    assert!(a.to_bits() == b.to_bits(), "{ctx}: C={a} Rust={b}");
}

#[test]
fn test_c2v() {
    let (c, r) = libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = c.get(b"c2V").unwrap();
        let rf: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = r.get(b"c2V").unwrap();
        for &(x, y) in &[(0.0f32, 0.0), (1.5, -2.3), (-100.0, 100.0), (f32::MAX, f32::MIN)] {
            assert_v_eq(cf(x, y), rf(x, y), &format!("c2V({x},{y})"));
        }
    }
}

#[test]
fn test_c2mulvs() {
    let (c, r) = libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = c.get(b"c2Mulvs").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = r.get(b"c2Mulvs").unwrap();
        for &(a, b) in &[(v(1.0, 2.0), 3.0f32), (v(-1.0, 0.5), -2.0), (v(0.0, 0.0), 100.0)] {
            assert_v_eq(cf(a, b), rf(a, b), "c2Mulvs");
        }
    }
}

#[test]
fn test_c2maxv_minv() {
    let (c, r) = libs();
    unsafe {
        let cmx: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Maxv").unwrap();
        let rmx: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Maxv").unwrap();
        let cmn: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Minv").unwrap();
        let rmn: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Minv").unwrap();
        let a = v(1.0, -3.0); let b = v(-2.0, 5.0);
        assert_v_eq(cmx(a, b), rmx(a, b), "c2Maxv");
        assert_v_eq(cmn(a, b), rmn(a, b), "c2Minv");
    }
}

#[test]
fn test_c2sub_add_dot_det2_neg_skew_ccw90() {
    let (c, r) = libs();
    unsafe {
        let pairs: &[(C2v, C2v)] = &[(v(3.0, 4.0), v(1.0, 2.0)), (v(-1.0, 0.0), v(0.0, -1.0))];
        macro_rules! cmp_vv {
            ($name:expr) => {{
                let cf: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get($name).unwrap();
                let rf: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get($name).unwrap();
                for &(a, b) in pairs { assert_v_eq(cf(a, b), rf(a, b), std::str::from_utf8($name).unwrap()); }
            }};
        }
        macro_rules! cmp_vf {
            ($name:expr) => {{
                let cf: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = c.get($name).unwrap();
                let rf: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = r.get($name).unwrap();
                for &(a, b) in pairs { assert_f_eq(cf(a, b), rf(a, b), std::str::from_utf8($name).unwrap()); }
            }};
        }
        cmp_vv!(b"c2Sub");
        cmp_vv!(b"c2Add");
        cmp_vf!(b"c2Dot");
        cmp_vf!(b"c2Det2");

        let cf_neg: Symbol<unsafe extern "C" fn(C2v) -> C2v> = c.get(b"c2Neg").unwrap();
        let rf_neg: Symbol<unsafe extern "C" fn(C2v) -> C2v> = r.get(b"c2Neg").unwrap();
        let cf_skew: Symbol<unsafe extern "C" fn(C2v) -> C2v> = c.get(b"c2Skew").unwrap();
        let rf_skew: Symbol<unsafe extern "C" fn(C2v) -> C2v> = r.get(b"c2Skew").unwrap();
        let cf_ccw: Symbol<unsafe extern "C" fn(C2v) -> C2v> = c.get(b"c2CCW90").unwrap();
        let rf_ccw: Symbol<unsafe extern "C" fn(C2v) -> C2v> = r.get(b"c2CCW90").unwrap();
        for &a in &[v(3.0, 4.0), v(-1.0, 0.0)] {
            assert_v_eq(cf_neg(a), rf_neg(a), "c2Neg");
            assert_v_eq(cf_skew(a), rf_skew(a), "c2Skew");
            assert_v_eq(cf_ccw(a), rf_ccw(a), "c2CCW90");
        }
    }
}

#[test]
fn test_c2len_div_norm_clampv() {
    let (c, r) = libs();
    unsafe {
        let cl: Symbol<unsafe extern "C" fn(C2v) -> f32> = c.get(b"c2Len").unwrap();
        let rl: Symbol<unsafe extern "C" fn(C2v) -> f32> = r.get(b"c2Len").unwrap();
        let cd: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = c.get(b"c2Div").unwrap();
        let rd: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = r.get(b"c2Div").unwrap();
        let cn: Symbol<unsafe extern "C" fn(C2v) -> C2v> = c.get(b"c2Norm").unwrap();
        let rn: Symbol<unsafe extern "C" fn(C2v) -> C2v> = r.get(b"c2Norm").unwrap();
        let ccl: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> = c.get(b"c2Clampv").unwrap();
        let rcl: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> = r.get(b"c2Clampv").unwrap();

        for &a in &[v(3.0, 4.0), v(-1.0, 0.0), v(0.0, 1.0)] {
            assert_f_eq(cl(a), rl(a), "c2Len");
            assert_v_eq(cd(a, 2.0), rd(a, 2.0), "c2Div");
            if cl(a) > 0.0 { assert_v_eq(cn(a), rn(a), "c2Norm"); }
        }
        assert_v_eq(ccl(v(5.0, -5.0), v(0.0, 0.0), v(3.0, 3.0)), rcl(v(5.0, -5.0), v(0.0, 0.0), v(3.0, 3.0)), "c2Clampv");
    }
}

#[test]
fn test_c2mulrv_mulrvt_mulxv() {
    let (c, r) = libs();
    unsafe {
        let cmr: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = c.get(b"c2Mulrv").unwrap();
        let rmr: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = r.get(b"c2Mulrv").unwrap();
        let cmt: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = c.get(b"c2MulrvT").unwrap();
        let rmt: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = r.get(b"c2MulrvT").unwrap();
        let cmx: Symbol<unsafe extern "C" fn(C2x, C2v) -> C2v> = c.get(b"c2Mulxv").unwrap();
        let rmx: Symbol<unsafe extern "C" fn(C2x, C2v) -> C2v> = r.get(b"c2Mulxv").unwrap();

        let rot = C2r { c: 0.6, s: 0.8 };
        let xf = C2x { p: v(10.0, 20.0), r: rot };
        for &bv in &[v(1.0, 0.0), v(0.0, 1.0), v(3.0, -4.0)] {
            assert_v_eq(cmr(rot, bv), rmr(rot, bv), "c2Mulrv");
            assert_v_eq(cmt(rot, bv), rmt(rot, bv), "c2MulrvT");
            assert_v_eq(cmx(xf, bv), rmx(xf, bv), "c2Mulxv");
        }
    }
}

#[test]
fn test_c2rot_identity_xidentity() {
    let (c, r) = libs();
    unsafe {
        let cri: Symbol<unsafe extern "C" fn() -> C2r> = c.get(b"c2RotIdentity").unwrap();
        let rri: Symbol<unsafe extern "C" fn() -> C2r> = r.get(b"c2RotIdentity").unwrap();
        let cr = cri(); let rr = rri();
        assert_f_eq(cr.c, rr.c, "RotIdentity.c");
        assert_f_eq(cr.s, rr.s, "RotIdentity.s");

        let cxi: Symbol<unsafe extern "C" fn() -> C2x> = c.get(b"c2xIdentity").unwrap();
        let rxi: Symbol<unsafe extern "C" fn() -> C2x> = r.get(b"c2xIdentity").unwrap();
        let cx = cxi(); let rx = rxi();
        assert_v_eq(cx.p, rx.p, "xIdentity.p");
        assert_f_eq(cx.r.c, rx.r.c, "xIdentity.r.c");
        assert_f_eq(cx.r.s, rx.r.s, "xIdentity.r.s");
    }
}

#[test]
fn test_c2bbverts() {
    let (c, r) = libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(*mut C2v, *mut C2AABB)> = c.get(b"c2BBVerts").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*mut C2v, *mut C2AABB)> = r.get(b"c2BBVerts").unwrap();
        let mut bb = C2AABB { min: v(-10.0, -20.0), max: v(30.0, 40.0) };
        let mut co = [C2v { x: 0.0, y: 0.0 }; 4];
        let mut ro = [C2v { x: 0.0, y: 0.0 }; 4];
        cf(co.as_mut_ptr(), &mut bb);
        rf(ro.as_mut_ptr(), &mut bb);
        for i in 0..4 { assert_v_eq(co[i], ro[i], &format!("c2BBVerts[{i}]")); }
    }
}

#[test]
fn test_c2support() {
    let (c, r) = libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int> = c.get(b"c2Support").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int> = r.get(b"c2Support").unwrap();
        let verts = [v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(-1.0, -1.0)];
        for &d in &[v(1.0, 0.0), v(0.0, 1.0), v(-1.0, -1.0), v(1.0, 1.0)] {
            let cv = cf(verts.as_ptr(), 4, d);
            let rv = rf(verts.as_ptr(), 4, d);
            assert_eq!(cv, rv, "c2Support d=({},{})", d.x, d.y);
        }
    }
}

#[test]
fn test_collision_circle_circle() {
    let (c, r) = libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> c_int> = c.get(b"c2CircletoCircle").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> c_int> = r.get(b"c2CircletoCircle").unwrap();
        let cases: &[(C2Circle, C2Circle)] = &[
            (C2Circle { p: v(0.0, 0.0), r: 5.0 }, C2Circle { p: v(3.0, 0.0), r: 5.0 }),  // overlap
            (C2Circle { p: v(0.0, 0.0), r: 1.0 }, C2Circle { p: v(10.0, 0.0), r: 1.0 }), // no overlap
            (C2Circle { p: v(0.0, 0.0), r: 5.0 }, C2Circle { p: v(10.0, 0.0), r: 5.0 }), // touching
        ];
        for (i, &(a, b)) in cases.iter().enumerate() {
            assert_eq!(cf(a, b), rf(a, b), "c2CircletoCircle case {i}");
        }
    }
}

#[test]
fn test_collision_circle_aabb() {
    let (c, r) = libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> c_int> = c.get(b"c2CircletoAABB").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> c_int> = r.get(b"c2CircletoAABB").unwrap();
        let cases: &[(C2Circle, C2AABB)] = &[
            (C2Circle { p: v(0.0, 0.0), r: 5.0 }, C2AABB { min: v(3.0, 3.0), max: v(6.0, 6.0) }),
            (C2Circle { p: v(0.0, 0.0), r: 1.0 }, C2AABB { min: v(10.0, 10.0), max: v(20.0, 20.0) }),
            (C2Circle { p: v(0.0, 0.0), r: 10.0 }, C2AABB { min: v(-5.0, -5.0), max: v(5.0, 5.0) }),
        ];
        for (i, &(a, b)) in cases.iter().enumerate() {
            assert_eq!(cf(a, b), rf(a, b), "c2CircletoAABB case {i}");
        }
    }
}

#[test]
fn test_collision_circle_capsule() {
    let (c, r) = libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int> = c.get(b"c2CircletoCapsule").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int> = r.get(b"c2CircletoCapsule").unwrap();
        let cases: &[(C2Circle, C2Capsule)] = &[
            (C2Circle { p: v(0.0, 0.0), r: 5.0 }, C2Capsule { a: v(3.0, 0.0), b: v(10.0, 0.0), r: 2.0 }),
            (C2Circle { p: v(0.0, 0.0), r: 1.0 }, C2Capsule { a: v(10.0, 10.0), b: v(20.0, 20.0), r: 1.0 }),
            (C2Circle { p: v(-70.0, 0.0), r: 20.0 }, C2Capsule { a: v(-50.0, -50.0), b: v(50.0, 50.0), r: 5.0 }),
        ];
        for (i, &(a, b)) in cases.iter().enumerate() {
            assert_eq!(cf(a, b), rf(a, b), "c2CircletoCapsule case {i}");
        }
    }
}

#[test]
fn test_collision_aabb_aabb() {
    let (c, r) = libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> = c.get(b"c2AABBtoAABB").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> = r.get(b"c2AABBtoAABB").unwrap();
        let cases: &[(C2AABB, C2AABB)] = &[
            (C2AABB { min: v(0.0, 0.0), max: v(5.0, 5.0) }, C2AABB { min: v(3.0, 3.0), max: v(8.0, 8.0) }),
            (C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }, C2AABB { min: v(10.0, 10.0), max: v(11.0, 11.0) }),
        ];
        for (i, &(a, b)) in cases.iter().enumerate() {
            assert_eq!(cf(a, b), rf(a, b), "c2AABBtoAABB case {i}");
        }
    }
}

#[test]
fn test_collision_aabb_capsule() {
    let (c, r) = libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2AABB, C2Capsule) -> c_int> = c.get(b"c2AABBtoCapsule").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2AABB, C2Capsule) -> c_int> = r.get(b"c2AABBtoCapsule").unwrap();
        let cases: &[(C2AABB, C2Capsule)] = &[
            (C2AABB { min: v(-40.0, -40.0), max: v(-15.0, -15.0) }, C2Capsule { a: v(-50.0, -50.0), b: v(50.0, 50.0), r: 5.0 }),
            (C2AABB { min: v(100.0, 100.0), max: v(110.0, 110.0) }, C2Capsule { a: v(0.0, 0.0), b: v(1.0, 1.0), r: 1.0 }),
        ];
        for (i, &(a, b)) in cases.iter().enumerate() {
            assert_eq!(cf(a, b), rf(a, b), "c2AABBtoCapsule case {i}");
        }
    }
}

#[test]
fn test_collision_capsule_capsule() {
    let (c, r) = libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(C2Capsule, C2Capsule) -> c_int> = c.get(b"c2CapsuletoCapsule").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2Capsule, C2Capsule) -> c_int> = r.get(b"c2CapsuletoCapsule").unwrap();
        let cases: &[(C2Capsule, C2Capsule)] = &[
            (C2Capsule { a: v(0.0, 0.0), b: v(10.0, 0.0), r: 5.0 }, C2Capsule { a: v(5.0, 5.0), b: v(15.0, 5.0), r: 5.0 }),
            (C2Capsule { a: v(0.0, 0.0), b: v(1.0, 0.0), r: 1.0 }, C2Capsule { a: v(100.0, 100.0), b: v(101.0, 100.0), r: 1.0 }),
            (C2Capsule { a: v(-40.0, 40.0), b: v(-20.0, 100.0), r: 10.0 }, C2Capsule { a: v(-50.0, -50.0), b: v(50.0, 50.0), r: 5.0 }),
        ];
        for (i, &(a, b)) in cases.iter().enumerate() {
            assert_eq!(cf(a, b), rf(a, b), "c2CapsuletoCapsule case {i}");
        }
    }
}

#[test]
fn test_capsule_top_level() {
    let (c, r) = libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(f32, f32, f32, f32, f32) -> c_int> = c.get(b"capsule").unwrap();
        let rf: Symbol<unsafe extern "C" fn(f32, f32, f32, f32, f32) -> c_int> = r.get(b"capsule").unwrap();
        let cases: &[(f32, f32, f32, f32, f32)] = &[
            (-50.0, -50.0, 50.0, 50.0, 5.0),
            (0.0, 0.0, 0.0, 0.0, 0.0),
            (-100.0, -100.0, 100.0, 100.0, 50.0),
            (100.0, 100.0, 200.0, 200.0, 1.0),
            (-70.0, 0.0, -60.0, 0.0, 15.0),
            (-40.0, -40.0, -15.0, -15.0, 1.0),
            (-40.0, 40.0, -20.0, 100.0, 5.0),
            (0.0, 0.0, 1.0, 1.0, 0.1),
            (-30.0, -30.0, -20.0, -20.0, 10.0),
            (-50.0, 50.0, -10.0, 90.0, 15.0),
        ];
        for (i, &(a, b, cx, d, e)) in cases.iter().enumerate() {
            let cv = cf(a, b, cx, d, e);
            let rv = rf(a, b, cx, d, e);
            assert_eq!(cv, rv, "capsule case {i}: args=({a},{b},{cx},{d},{e}) C={cv} Rust={rv}");
        }
    }
}

#[test]
fn test_c2collided_via_ffi() {
    let (c, r) = libs();
    unsafe {
        let cf: Symbol<unsafe extern "C" fn(*const std::ffi::c_void, c_int, *const std::ffi::c_void, c_int) -> c_int> = c.get(b"c2Collided").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const std::ffi::c_void, c_int, *const std::ffi::c_void, c_int) -> c_int> = r.get(b"c2Collided").unwrap();

        // Circle vs Capsule (type 0 vs type 2)
        let circle = C2Circle { p: v(-70.0, 0.0), r: 20.0 };
        let cap = C2Capsule { a: v(-50.0, -50.0), b: v(50.0, 50.0), r: 5.0 };
        let cv = cf(&circle as *const _ as *const _, 0, &cap as *const _ as *const _, 2);
        let rv = rf(&circle as *const _ as *const _, 0, &cap as *const _ as *const _, 2);
        assert_eq!(cv, rv, "c2Collided Circle vs Capsule");

        // AABB vs AABB (type 1 vs type 1)
        let a1 = C2AABB { min: v(0.0, 0.0), max: v(5.0, 5.0) };
        let a2 = C2AABB { min: v(3.0, 3.0), max: v(8.0, 8.0) };
        let cv = cf(&a1 as *const _ as *const _, 1, &a2 as *const _ as *const _, 1);
        let rv = rf(&a1 as *const _ as *const _, 1, &a2 as *const _ as *const _, 1);
        assert_eq!(cv, rv, "c2Collided AABB vs AABB");

        // Capsule vs Circle (type 2 vs type 0)
        let cv = cf(&cap as *const _ as *const _, 2, &circle as *const _ as *const _, 0);
        let rv = rf(&cap as *const _ as *const _, 2, &circle as *const _ as *const _, 0);
        assert_eq!(cv, rv, "c2Collided Capsule vs Circle");
    }
}
