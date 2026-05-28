// Integration tests comparing the C library and Rust library outputs through
// their FFI exports. Both .so files are loaded via libloading and called via
// their exported symbols.

use libloading::{Library, Symbol};
use std::ffi::c_int;

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
struct C2GJKCache {
    metric: f32,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
struct C2sv {
    sA: C2v,
    sB: C2v,
    p: C2v,
    u: f32,
    iA: c_int,
    iB: c_int,
}

impl Default for C2v {
    fn default() -> Self {
        C2v { x: 0.0, y: 0.0 }
    }
}

impl Default for C2r {
    fn default() -> Self {
        C2r { c: 0.0, s: 0.0 }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq, Default)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct C2Proxy {
    radius: f32,
    count: c_int,
    verts: [C2v; 8],
}

impl Default for C2Proxy {
    fn default() -> Self {
        C2Proxy {
            radius: 0.0,
            count: 0,
            verts: [C2v::default(); 8],
        }
    }
}

fn c_lib_path() -> &'static str {
    "c_src/build/libtranslated_rust.so"
}

fn rust_lib_path() -> &'static str {
    "target/release/libcapsule_lib.so"
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("Failed to load C library");
        let r = Library::new(rust_lib_path()).expect("Failed to load Rust library");
        (c, r)
    }
}

// Helper that compares two f32 values exactly (bit-pattern). NaN-aware.
fn f32_eq_bits(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan())
}

fn assert_f32_eq_bits(a: f32, b: f32, ctx: &str) {
    assert!(
        f32_eq_bits(a, b),
        "{}: {} ({:#x}) != {} ({:#x})",
        ctx,
        a,
        a.to_bits(),
        b,
        b.to_bits()
    );
}

fn assert_c2v_eq(a: C2v, b: C2v, ctx: &str) {
    assert_f32_eq_bits(a.x, b.x, &format!("{}.x", ctx));
    assert_f32_eq_bits(a.y, b.y, &format!("{}.y", ctx));
}

// =========== Low-level vector functions ===========

#[test]
fn test_c2V() {
    let (c, r) = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = c.get(b"c2V").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = r.get(b"c2V").unwrap();
        for &(x, y) in &[(0.0, 0.0), (1.0, 2.0), (-3.5, 4.25), (1e10, -1e10)] {
            let cv = f_c(x, y);
            let rv = f_r(x, y);
            assert_c2v_eq(cv, rv, "c2V");
        }
    }
}

#[test]
fn test_c2Mulvs() {
    let (c, r) = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = c.get(b"c2Mulvs").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = r.get(b"c2Mulvs").unwrap();
        let cases = [
            (C2v { x: 1.0, y: 2.0 }, 3.0),
            (C2v { x: -1.5, y: 4.25 }, 0.5),
            (C2v { x: 0.0, y: 0.0 }, 100.0),
        ];
        for &(a, b) in &cases {
            assert_c2v_eq(f_c(a, b), f_r(a, b), "c2Mulvs");
        }
    }
}

#[test]
fn test_c2Maxv_Minv_Clampv() {
    let (c, r) = load_libs();
    unsafe {
        let max_c: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Maxv").unwrap();
        let max_r: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Maxv").unwrap();
        let min_c: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Minv").unwrap();
        let min_r: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Minv").unwrap();
        let clamp_c: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> =
            c.get(b"c2Clampv").unwrap();
        let clamp_r: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> =
            r.get(b"c2Clampv").unwrap();

        let cases = [
            (C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: -1.0 }),
            (C2v { x: -5.0, y: -5.0 }, C2v { x: 5.0, y: 5.0 }),
        ];
        for &(a, b) in &cases {
            assert_c2v_eq(max_c(a, b), max_r(a, b), "c2Maxv");
            assert_c2v_eq(min_c(a, b), min_r(a, b), "c2Minv");
        }
        let lo = C2v { x: -1.0, y: -1.0 };
        let hi = C2v { x: 1.0, y: 1.0 };
        for &p in &[
            C2v { x: 2.0, y: 0.5 },
            C2v { x: -3.0, y: 0.0 },
            C2v { x: 0.5, y: 0.5 },
        ] {
            assert_c2v_eq(clamp_c(p, lo, hi), clamp_r(p, lo, hi), "c2Clampv");
        }
    }
}

#[test]
fn test_c2Sub_Add_Dot_Det2_Len() {
    let (c, r) = load_libs();
    unsafe {
        let sub_c: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Sub").unwrap();
        let sub_r: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Sub").unwrap();
        let add_c: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = c.get(b"c2Add").unwrap();
        let add_r: Symbol<unsafe extern "C" fn(C2v, C2v) -> C2v> = r.get(b"c2Add").unwrap();
        let dot_c: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = c.get(b"c2Dot").unwrap();
        let dot_r: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = r.get(b"c2Dot").unwrap();
        let det_c: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = c.get(b"c2Det2").unwrap();
        let det_r: Symbol<unsafe extern "C" fn(C2v, C2v) -> f32> = r.get(b"c2Det2").unwrap();
        let len_c: Symbol<unsafe extern "C" fn(C2v) -> f32> = c.get(b"c2Len").unwrap();
        let len_r: Symbol<unsafe extern "C" fn(C2v) -> f32> = r.get(b"c2Len").unwrap();

        let cases = [
            (C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: 4.0 }),
            (C2v { x: -1.5, y: 0.25 }, C2v { x: 7.5, y: -2.0 }),
        ];
        for &(a, b) in &cases {
            assert_c2v_eq(sub_c(a, b), sub_r(a, b), "c2Sub");
            assert_c2v_eq(add_c(a, b), add_r(a, b), "c2Add");
            assert_f32_eq_bits(dot_c(a, b), dot_r(a, b), "c2Dot");
            assert_f32_eq_bits(det_c(a, b), det_r(a, b), "c2Det2");
            assert_f32_eq_bits(len_c(a), len_r(a), "c2Len");
        }
    }
}

#[test]
fn test_c2RotIdentity_xIdentity() {
    let (c, r) = load_libs();
    unsafe {
        let ri_c: Symbol<unsafe extern "C" fn() -> C2r> = c.get(b"c2RotIdentity").unwrap();
        let ri_r: Symbol<unsafe extern "C" fn() -> C2r> = r.get(b"c2RotIdentity").unwrap();
        let xi_c: Symbol<unsafe extern "C" fn() -> C2x> = c.get(b"c2xIdentity").unwrap();
        let xi_r: Symbol<unsafe extern "C" fn() -> C2x> = r.get(b"c2xIdentity").unwrap();
        let r_c = ri_c();
        let r_r = ri_r();
        assert_f32_eq_bits(r_c.c, r_r.c, "c2RotIdentity.c");
        assert_f32_eq_bits(r_c.s, r_r.s, "c2RotIdentity.s");
        let x_c = xi_c();
        let x_r = xi_r();
        assert_f32_eq_bits(x_c.p.x, x_r.p.x, "c2xIdentity.p.x");
        assert_f32_eq_bits(x_c.p.y, x_r.p.y, "c2xIdentity.p.y");
        assert_f32_eq_bits(x_c.r.c, x_r.r.c, "c2xIdentity.r.c");
        assert_f32_eq_bits(x_c.r.s, x_r.r.s, "c2xIdentity.r.s");
    }
}

#[test]
fn test_c2Mulrv_MulrvT_Mulxv() {
    let (c, r) = load_libs();
    unsafe {
        let mulrv_c: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = c.get(b"c2Mulrv").unwrap();
        let mulrv_r: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = r.get(b"c2Mulrv").unwrap();
        let mulrvt_c: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = c.get(b"c2MulrvT").unwrap();
        let mulrvt_r: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = r.get(b"c2MulrvT").unwrap();
        let mulxv_c: Symbol<unsafe extern "C" fn(C2x, C2v) -> C2v> = c.get(b"c2Mulxv").unwrap();
        let mulxv_r: Symbol<unsafe extern "C" fn(C2x, C2v) -> C2v> = r.get(b"c2Mulxv").unwrap();

        let rot = C2r { c: 0.7, s: 0.6 };
        let v = C2v { x: 1.5, y: -2.5 };
        assert_c2v_eq(mulrv_c(rot, v), mulrv_r(rot, v), "c2Mulrv");
        assert_c2v_eq(mulrvt_c(rot, v), mulrvt_r(rot, v), "c2MulrvT");
        let x = C2x {
            p: C2v { x: 2.0, y: 3.0 },
            r: rot,
        };
        assert_c2v_eq(mulxv_c(x, v), mulxv_r(x, v), "c2Mulxv");
    }
}

#[test]
fn test_c2Neg_Skew_CCW90_Norm_Div() {
    let (c, r) = load_libs();
    unsafe {
        let neg_c: Symbol<unsafe extern "C" fn(C2v) -> C2v> = c.get(b"c2Neg").unwrap();
        let neg_r: Symbol<unsafe extern "C" fn(C2v) -> C2v> = r.get(b"c2Neg").unwrap();
        let skew_c: Symbol<unsafe extern "C" fn(C2v) -> C2v> = c.get(b"c2Skew").unwrap();
        let skew_r: Symbol<unsafe extern "C" fn(C2v) -> C2v> = r.get(b"c2Skew").unwrap();
        let ccw_c: Symbol<unsafe extern "C" fn(C2v) -> C2v> = c.get(b"c2CCW90").unwrap();
        let ccw_r: Symbol<unsafe extern "C" fn(C2v) -> C2v> = r.get(b"c2CCW90").unwrap();
        let norm_c: Symbol<unsafe extern "C" fn(C2v) -> C2v> = c.get(b"c2Norm").unwrap();
        let norm_r: Symbol<unsafe extern "C" fn(C2v) -> C2v> = r.get(b"c2Norm").unwrap();
        let div_c: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = c.get(b"c2Div").unwrap();
        let div_r: Symbol<unsafe extern "C" fn(C2v, f32) -> C2v> = r.get(b"c2Div").unwrap();

        for &v in &[
            C2v { x: 1.0, y: 2.0 },
            C2v { x: -3.0, y: 4.0 },
            C2v { x: 0.5, y: -0.5 },
        ] {
            assert_c2v_eq(neg_c(v), neg_r(v), "c2Neg");
            assert_c2v_eq(skew_c(v), skew_r(v), "c2Skew");
            assert_c2v_eq(ccw_c(v), ccw_r(v), "c2CCW90");
            assert_c2v_eq(norm_c(v), norm_r(v), "c2Norm");
            assert_c2v_eq(div_c(v, 2.0), div_r(v, 2.0), "c2Div");
        }
    }
}

#[test]
fn test_c2BBVerts() {
    let (c, r) = load_libs();
    unsafe {
        let bb_c: Symbol<unsafe extern "C" fn(*mut C2v, *const C2AABB)> =
            c.get(b"c2BBVerts").unwrap();
        let bb_r: Symbol<unsafe extern "C" fn(*mut C2v, *const C2AABB)> =
            r.get(b"c2BBVerts").unwrap();
        let aabb = C2AABB {
            min: C2v { x: -1.0, y: -2.0 },
            max: C2v { x: 3.0, y: 4.0 },
        };
        let mut out_c = [C2v::default(); 4];
        let mut out_r = [C2v::default(); 4];
        bb_c(out_c.as_mut_ptr(), &aabb);
        bb_r(out_r.as_mut_ptr(), &aabb);
        for i in 0..4 {
            assert_c2v_eq(out_c[i], out_r[i], &format!("bbverts[{}]", i));
        }
    }
}

#[test]
fn test_c2Support() {
    let (c, r) = load_libs();
    unsafe {
        let s_c: Symbol<unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int> =
            c.get(b"c2Support").unwrap();
        let s_r: Symbol<unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int> =
            r.get(b"c2Support").unwrap();
        let verts = [
            C2v { x: 0.0, y: 0.0 },
            C2v { x: 1.0, y: 0.0 },
            C2v { x: 1.0, y: 1.0 },
            C2v { x: 0.0, y: 1.0 },
        ];
        for &d in &[
            C2v { x: 1.0, y: 0.0 },
            C2v { x: 0.0, y: 1.0 },
            C2v { x: -1.0, y: 0.0 },
            C2v { x: 1.0, y: 1.0 },
        ] {
            let cc = s_c(verts.as_ptr(), 4, d);
            let rr = s_r(verts.as_ptr(), 4, d);
            assert_eq!(cc, rr, "c2Support({:?})", d);
        }
    }
}

// =========== Higher-level shape collisions ===========

#[test]
fn test_c2CircletoCircle() {
    let (c, r) = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> c_int> =
            c.get(b"c2CircletoCircle").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> c_int> =
            r.get(b"c2CircletoCircle").unwrap();
        let cases = [
            (
                C2Circle {
                    p: C2v { x: 0.0, y: 0.0 },
                    r: 1.0,
                },
                C2Circle {
                    p: C2v { x: 1.0, y: 0.0 },
                    r: 1.0,
                },
            ),
            (
                C2Circle {
                    p: C2v { x: 0.0, y: 0.0 },
                    r: 1.0,
                },
                C2Circle {
                    p: C2v { x: 5.0, y: 0.0 },
                    r: 1.0,
                },
            ),
        ];
        for &(a, b) in &cases {
            assert_eq!(f_c(a, b), f_r(a, b), "c2CircletoCircle");
        }
    }
}

#[test]
fn test_c2CircletoAABB() {
    let (c, r) = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> c_int> =
            c.get(b"c2CircletoAABB").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> c_int> =
            r.get(b"c2CircletoAABB").unwrap();
        let aabb = C2AABB {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v { x: 1.0, y: 1.0 },
        };
        let cases = [
            C2Circle {
                p: C2v { x: 0.5, y: 0.5 },
                r: 0.1,
            },
            C2Circle {
                p: C2v { x: 5.0, y: 5.0 },
                r: 0.1,
            },
            C2Circle {
                p: C2v { x: -0.05, y: 0.5 },
                r: 0.1,
            },
        ];
        for &cir in &cases {
            assert_eq!(f_c(cir, aabb), f_r(cir, aabb), "c2CircletoAABB");
        }
    }
}

#[test]
fn test_c2CircletoCapsule() {
    let (c, r) = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int> =
            c.get(b"c2CircletoCapsule").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int> =
            r.get(b"c2CircletoCapsule").unwrap();
        let cap = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 4.0, y: 0.0 },
            r: 0.5,
        };
        let cases = [
            C2Circle {
                p: C2v { x: 2.0, y: 0.0 },
                r: 0.1,
            },
            C2Circle {
                p: C2v { x: 2.0, y: 5.0 },
                r: 0.1,
            },
            C2Circle {
                p: C2v { x: -1.0, y: 0.0 },
                r: 0.6,
            },
        ];
        for &cir in &cases {
            assert_eq!(f_c(cir, cap), f_r(cir, cap), "c2CircletoCapsule");
        }
    }
}

#[test]
fn test_c2AABBtoAABB() {
    let (c, r) = load_libs();
    unsafe {
        let f_c: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> =
            c.get(b"c2AABBtoAABB").unwrap();
        let f_r: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> =
            r.get(b"c2AABBtoAABB").unwrap();
        let cases = [
            (
                C2AABB {
                    min: C2v { x: 0.0, y: 0.0 },
                    max: C2v { x: 1.0, y: 1.0 },
                },
                C2AABB {
                    min: C2v { x: 0.5, y: 0.5 },
                    max: C2v { x: 1.5, y: 1.5 },
                },
            ),
            (
                C2AABB {
                    min: C2v { x: 0.0, y: 0.0 },
                    max: C2v { x: 1.0, y: 1.0 },
                },
                C2AABB {
                    min: C2v { x: 2.0, y: 2.0 },
                    max: C2v { x: 3.0, y: 3.0 },
                },
            ),
        ];
        for &(a, b) in &cases {
            assert_eq!(f_c(a, b), f_r(a, b), "c2AABBtoAABB");
        }
    }
}

#[test]
fn test_c2AABBtoCapsule_CapsuletoCapsule() {
    let (c, r) = load_libs();
    unsafe {
        let abc_c: Symbol<unsafe extern "C" fn(C2AABB, C2Capsule) -> c_int> =
            c.get(b"c2AABBtoCapsule").unwrap();
        let abc_r: Symbol<unsafe extern "C" fn(C2AABB, C2Capsule) -> c_int> =
            r.get(b"c2AABBtoCapsule").unwrap();
        let cc_c: Symbol<unsafe extern "C" fn(C2Capsule, C2Capsule) -> c_int> =
            c.get(b"c2CapsuletoCapsule").unwrap();
        let cc_r: Symbol<unsafe extern "C" fn(C2Capsule, C2Capsule) -> c_int> =
            r.get(b"c2CapsuletoCapsule").unwrap();
        let aabb = C2AABB {
            min: C2v { x: 0.0, y: 0.0 },
            max: C2v { x: 1.0, y: 1.0 },
        };
        let cap = C2Capsule {
            a: C2v { x: 0.5, y: 0.5 },
            b: C2v { x: 5.0, y: 0.5 },
            r: 0.1,
        };
        assert_eq!(abc_c(aabb, cap), abc_r(aabb, cap), "c2AABBtoCapsule");

        let cap2 = C2Capsule {
            a: C2v { x: 10.0, y: 10.0 },
            b: C2v { x: 11.0, y: 10.0 },
            r: 0.1,
        };
        assert_eq!(cc_c(cap, cap2), cc_r(cap, cap2), "c2CapsuletoCapsule_far");
        assert_eq!(cc_c(cap, cap), cc_r(cap, cap), "c2CapsuletoCapsule_self");
    }
}

#[test]
fn test_c2GJK_distance() {
    let (c, r) = load_libs();
    unsafe {
        let gjk_c: Symbol<
            unsafe extern "C" fn(
                *const std::ffi::c_void,
                c_int,
                *const C2x,
                *const std::ffi::c_void,
                c_int,
                *const C2x,
                *mut C2v,
                *mut C2v,
                c_int,
                *mut c_int,
                *mut C2GJKCache,
            ) -> f32,
        > = c.get(b"c2GJK").unwrap();
        let gjk_r: Symbol<
            unsafe extern "C" fn(
                *const std::ffi::c_void,
                c_int,
                *const C2x,
                *const std::ffi::c_void,
                c_int,
                *const C2x,
                *mut C2v,
                *mut C2v,
                c_int,
                *mut c_int,
                *mut C2GJKCache,
            ) -> f32,
        > = r.get(b"c2GJK").unwrap();

        let cap = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 4.0, y: 0.0 },
            r: 0.5,
        };
        let cap2 = C2Capsule {
            a: C2v { x: 0.0, y: 5.0 },
            b: C2v { x: 4.0, y: 5.0 },
            r: 0.5,
        };
        const C2_TYPE_CAPSULE: c_int = 2;

        let mut a_c = C2v::default();
        let mut b_c = C2v::default();
        let mut iter_c: c_int = 0;
        let mut a_r = C2v::default();
        let mut b_r = C2v::default();
        let mut iter_r: c_int = 0;
        let dist_c = gjk_c(
            &cap as *const _ as *const std::ffi::c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &cap2 as *const _ as *const std::ffi::c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut a_c,
            &mut b_c,
            1,
            &mut iter_c,
            std::ptr::null_mut(),
        );
        let dist_r = gjk_r(
            &cap as *const _ as *const std::ffi::c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &cap2 as *const _ as *const std::ffi::c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut a_r,
            &mut b_r,
            1,
            &mut iter_r,
            std::ptr::null_mut(),
        );
        assert_f32_eq_bits(dist_c, dist_r, "c2GJK dist");
        assert_c2v_eq(a_c, a_r, "c2GJK outA");
        assert_c2v_eq(b_c, b_r, "c2GJK outB");
        assert_eq!(iter_c, iter_r, "c2GJK iterations");
    }
}

#[test]
fn test_capsule_top_level() {
    let (c, r) = load_libs();
    unsafe {
        let cap_c: Symbol<unsafe extern "C" fn(f32, f32, f32, f32, f32) -> c_int> =
            c.get(b"capsule").unwrap();
        let cap_r: Symbol<unsafe extern "C" fn(f32, f32, f32, f32, f32) -> c_int> =
            r.get(b"capsule").unwrap();
        // Cases that cover all collision tests in the function.
        let cases: &[(f32, f32, f32, f32, f32)] = &[
            (-50.0, 0.0, -60.0, 5.0, 5.0),    // near circle
            (-30.0, -30.0, -20.0, -20.0, 1.0), // inside aabb
            (-30.0, 50.0, -25.0, 80.0, 5.0),   // near capsule
            (100.0, 100.0, 110.0, 110.0, 1.0), // none
            (-70.0, 0.0, 100.0, 100.0, 30.0),  // many overlaps
            (0.0, 0.0, 0.0, 0.0, 0.0),
        ];
        for &(a, b, cc, d, e) in cases {
            let rc = cap_c(a, b, cc, d, e);
            let rr = cap_r(a, b, cc, d, e);
            assert_eq!(
                rc, rr,
                "capsule({}, {}, {}, {}, {}) c={}, r={}",
                a, b, cc, d, e, rc, rr
            );
        }
    }
}
