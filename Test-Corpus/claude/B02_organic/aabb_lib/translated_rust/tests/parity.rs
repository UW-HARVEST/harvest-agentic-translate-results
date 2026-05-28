// Parity tests: load both C and Rust .so libraries via libloading
// and compare results byte-for-byte.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use libloading::{Library, Symbol};
use std::os::raw::{c_int, c_void};
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
struct c2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
struct c2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, PartialEq, Debug, Default)]
struct c2GJKCache {
    metric: f32,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct c2Proxy {
    radius: f32,
    count: c_int,
    verts: [c2v; 8],
}

impl Default for c2Proxy {
    fn default() -> Self {
        c2Proxy {
            radius: 0.0,
            count: 0,
            verts: [c2v { x: 0.0, y: 0.0 }; 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
struct c2sv {
    sA: c2v,
    sB: c2v,
    p: c2v,
    u: f32,
    iA: c_int,
    iB: c_int,
}

impl Default for c2v {
    fn default() -> Self {
        c2v { x: 0.0, y: 0.0 }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
struct c2Simplex {
    a: c2sv,
    b: c2sv,
    c: c2sv,
    d: c2sv,
    div: f32,
    count: c_int,
}

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn c_lib_path() -> PathBuf {
    workspace_root().join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    workspace_root().join("target/release/libaabb_lib.so")
}

fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");
        (c, r)
    }
}

// helper: get a symbol from both libs
unsafe fn get_pair<'a, T: 'a>(c: &'a Library, r: &'a Library, name: &[u8]) -> (Symbol<'a, T>, Symbol<'a, T>) {
    let cs = c.get::<T>(name).unwrap_or_else(|e| panic!("C sym {}: {}", String::from_utf8_lossy(name), e));
    let rs = r.get::<T>(name).unwrap_or_else(|e| panic!("Rust sym {}: {}", String::from_utf8_lossy(name), e));
    (cs, rs)
}

fn bits_eq_f32(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

fn bits_eq_v(a: c2v, b: c2v) -> bool {
    bits_eq_f32(a.x, b.x) && bits_eq_f32(a.y, b.y)
}

#[test]
fn test_c2V() {
    unsafe {
        let (c, r) = load_libs();
        type F = unsafe extern "C" fn(f32, f32) -> c2v;
        let (cf, rf) = get_pair::<F>(&c, &r, b"c2V");
        for &(x, y) in &[(0.0_f32, 0.0_f32), (1.5, -2.5), (-3.0, 4.0), (f32::INFINITY, 0.0)] {
            let a = cf(x, y);
            let b = rf(x, y);
            assert!(bits_eq_v(a, b), "c2V({},{}) C={:?} Rust={:?}", x, y, a, b);
        }
    }
}

#[test]
fn test_c2Mulvs() {
    unsafe {
        let (c, r) = load_libs();
        type F = unsafe extern "C" fn(c2v, f32) -> c2v;
        let (cf, rf) = get_pair::<F>(&c, &r, b"c2Mulvs");
        for &(x, y, s) in &[(1.0_f32, 2.0_f32, 3.0_f32), (-1.0, 5.0, -2.0), (0.0, 0.0, 1.0)] {
            let v = c2v { x, y };
            let a = cf(v, s);
            let b = rf(v, s);
            assert!(bits_eq_v(a, b));
        }
    }
}

#[test]
fn test_c2_arith_pair() {
    unsafe {
        let (c, r) = load_libs();
        type F = unsafe extern "C" fn(c2v, c2v) -> c2v;
        for name in [b"c2Maxv".as_ref(), b"c2Minv".as_ref(), b"c2Sub".as_ref(), b"c2Add".as_ref()] {
            let (cf, rf) = get_pair::<F>(&c, &r, name);
            for &(ax, ay, bx, by) in &[
                (1.0_f32, 2.0_f32, 3.0_f32, 4.0_f32),
                (-1.0, -2.0, 1.5, -2.5),
                (0.0, 0.0, 0.0, 0.0),
                (10.0, -5.0, -10.0, 5.0),
            ] {
                let va = c2v { x: ax, y: ay };
                let vb = c2v { x: bx, y: by };
                let ra = cf(va, vb);
                let rb = rf(va, vb);
                assert!(bits_eq_v(ra, rb), "{} mismatch", String::from_utf8_lossy(name));
            }
        }
    }
}

#[test]
fn test_c2Dot_Det2_Len() {
    unsafe {
        let (c, r) = load_libs();
        type F2 = unsafe extern "C" fn(c2v, c2v) -> f32;
        type F1 = unsafe extern "C" fn(c2v) -> f32;
        let (cdot, rdot) = get_pair::<F2>(&c, &r, b"c2Dot");
        let (cdet, rdet) = get_pair::<F2>(&c, &r, b"c2Det2");
        let (clen, rlen) = get_pair::<F1>(&c, &r, b"c2Len");
        for &(ax, ay, bx, by) in &[
            (1.0_f32, 2.0_f32, 3.0_f32, 4.0_f32),
            (-1.0, -2.0, 1.5, -2.5),
            (0.0, 0.0, 0.0, 0.0),
            (10.0, -5.0, -10.0, 5.0),
        ] {
            let va = c2v { x: ax, y: ay };
            let vb = c2v { x: bx, y: by };
            assert!(bits_eq_f32(cdot(va, vb), rdot(va, vb)));
            assert!(bits_eq_f32(cdet(va, vb), rdet(va, vb)));
            assert!(bits_eq_f32(clen(va), rlen(va)));
        }
    }
}

#[test]
fn test_c2Clampv() {
    unsafe {
        let (c, r) = load_libs();
        type F = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
        let (cf, rf) = get_pair::<F>(&c, &r, b"c2Clampv");
        let cases = [
            ((5.0_f32, 5.0_f32), (0.0_f32, 0.0_f32), (10.0_f32, 10.0_f32)),
            ((-5.0, 15.0), (0.0, 0.0), (10.0, 10.0)),
        ];
        for &(p, lo, hi) in &cases {
            let pv = c2v { x: p.0, y: p.1 };
            let lov = c2v { x: lo.0, y: lo.1 };
            let hiv = c2v { x: hi.0, y: hi.1 };
            assert!(bits_eq_v(cf(pv, lov, hiv), rf(pv, lov, hiv)));
        }
    }
}

#[test]
fn test_c2RotIdentity_xIdentity() {
    unsafe {
        let (c, r) = load_libs();
        type F1 = unsafe extern "C" fn() -> c2r;
        type F2 = unsafe extern "C" fn() -> c2x;
        let (cf, rf) = get_pair::<F1>(&c, &r, b"c2RotIdentity");
        let a = cf();
        let b = rf();
        assert!(bits_eq_f32(a.c, b.c) && bits_eq_f32(a.s, b.s));

        let (cf, rf) = get_pair::<F2>(&c, &r, b"c2xIdentity");
        let a = cf();
        let b = rf();
        assert!(bits_eq_v(a.p, b.p) && bits_eq_f32(a.r.c, b.r.c) && bits_eq_f32(a.r.s, b.r.s));
    }
}

#[test]
fn test_c2Neg_Skew_CCW90_Norm() {
    unsafe {
        let (c, r) = load_libs();
        type F = unsafe extern "C" fn(c2v) -> c2v;
        for name in [b"c2Neg".as_ref(), b"c2Skew".as_ref(), b"c2CCW90".as_ref()] {
            let (cf, rf) = get_pair::<F>(&c, &r, name);
            for &(x, y) in &[(1.0_f32, 2.0_f32), (-3.0, 4.0), (0.0, 0.0)] {
                let v = c2v { x, y };
                assert!(bits_eq_v(cf(v), rf(v)), "{}", String::from_utf8_lossy(name));
            }
        }
        let (cf, rf) = get_pair::<F>(&c, &r, b"c2Norm");
        for &(x, y) in &[(1.0_f32, 2.0_f32), (-3.0, 4.0), (10.0, 0.0)] {
            let v = c2v { x, y };
            assert!(bits_eq_v(cf(v), rf(v)));
        }

        type FD = unsafe extern "C" fn(c2v, f32) -> c2v;
        let (cf, rf) = get_pair::<FD>(&c, &r, b"c2Div");
        let v = c2v { x: 6.0, y: -4.0 };
        assert!(bits_eq_v(cf(v, 2.0), rf(v, 2.0)));
    }
}

#[test]
fn test_c2Mulrv_MulrvT_Mulxv() {
    unsafe {
        let (c, r) = load_libs();
        type FR = unsafe extern "C" fn(c2r, c2v) -> c2v;
        type FX = unsafe extern "C" fn(c2x, c2v) -> c2v;
        let rot = c2r { c: 0.6, s: 0.8 };
        let v = c2v { x: 3.0, y: 4.0 };
        let (cf, rf) = get_pair::<FR>(&c, &r, b"c2Mulrv");
        assert!(bits_eq_v(cf(rot, v), rf(rot, v)));
        let (cf, rf) = get_pair::<FR>(&c, &r, b"c2MulrvT");
        assert!(bits_eq_v(cf(rot, v), rf(rot, v)));
        let xform = c2x { p: c2v { x: 1.0, y: 2.0 }, r: rot };
        let (cf, rf) = get_pair::<FX>(&c, &r, b"c2Mulxv");
        assert!(bits_eq_v(cf(xform, v), rf(xform, v)));
    }
}

#[test]
fn test_c2BBVerts() {
    unsafe {
        let (c, r) = load_libs();
        type F = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
        let (cf, rf) = get_pair::<F>(&c, &r, b"c2BBVerts");
        let mut bb = c2AABB {
            min: c2v { x: -1.0, y: -2.0 },
            max: c2v { x: 3.0, y: 4.0 },
        };
        let mut out_c = [c2v::default(); 4];
        let mut out_r = [c2v::default(); 4];
        cf(out_c.as_mut_ptr(), &mut bb);
        rf(out_r.as_mut_ptr(), &mut bb);
        for i in 0..4 {
            assert!(bits_eq_v(out_c[i], out_r[i]), "vert {} differs", i);
        }
    }
}

#[test]
fn test_c2MakeProxy() {
    unsafe {
        let (c, r) = load_libs();
        type F = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
        let (cf, rf) = get_pair::<F>(&c, &r, b"c2MakeProxy");

        let circle = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 5.0 };
        let mut pc = c2Proxy::default();
        let mut pr = c2Proxy::default();
        cf(&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut pc);
        rf(&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut pr);
        assert_eq!(pc.count, pr.count);
        assert!(bits_eq_f32(pc.radius, pr.radius));
        for i in 0..pc.count as usize {
            assert!(bits_eq_v(pc.verts[i], pr.verts[i]));
        }

        let aabb = c2AABB { min: c2v { x: -1.0, y: -2.0 }, max: c2v { x: 3.0, y: 4.0 } };
        let mut pc = c2Proxy::default();
        let mut pr = c2Proxy::default();
        cf(&aabb as *const _ as *const c_void, C2_TYPE_AABB, &mut pc);
        rf(&aabb as *const _ as *const c_void, C2_TYPE_AABB, &mut pr);
        assert_eq!(pc.count, pr.count);
        assert!(bits_eq_f32(pc.radius, pr.radius));
        for i in 0..pc.count as usize {
            assert!(bits_eq_v(pc.verts[i], pr.verts[i]));
        }

        let cap = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: 2.0 };
        let mut pc = c2Proxy::default();
        let mut pr = c2Proxy::default();
        cf(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut pc);
        rf(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut pr);
        assert_eq!(pc.count, pr.count);
        assert!(bits_eq_f32(pc.radius, pr.radius));
        for i in 0..pc.count as usize {
            assert!(bits_eq_v(pc.verts[i], pr.verts[i]));
        }
    }
}

#[test]
fn test_c2Support() {
    unsafe {
        let (c, r) = load_libs();
        type F = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
        let (cf, rf) = get_pair::<F>(&c, &r, b"c2Support");
        let verts = [
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: -1.0, y: 0.0 },
            c2v { x: 0.0, y: -1.0 },
        ];
        for &d in &[c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }, c2v { x: -1.0, y: -1.0 }] {
            let a = cf(verts.as_ptr(), 4, d);
            let b = rf(verts.as_ptr(), 4, d);
            assert_eq!(a, b);
        }
    }
}

#[test]
fn test_c2GJKSimplexMetric_c22_c23_cD_cL_cWitness() {
    unsafe {
        let (c, r) = load_libs();
        type FM = unsafe extern "C" fn(*mut c2Simplex) -> f32;
        type FS = unsafe extern "C" fn(*mut c2Simplex);
        type FV = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
        type FW = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);

        let make_simplex = || -> c2Simplex {
            let mut s = c2Simplex::default();
            s.a.p = c2v { x: 1.0, y: 0.0 };
            s.b.p = c2v { x: 0.0, y: 1.0 };
            s.c.p = c2v { x: -1.0, y: -1.0 };
            s.a.sA = c2v { x: 0.5, y: 0.0 };
            s.b.sA = c2v { x: 0.0, y: 0.5 };
            s.c.sA = c2v { x: -0.5, y: -0.5 };
            s.a.sB = c2v { x: -0.5, y: 0.0 };
            s.b.sB = c2v { x: 0.0, y: -0.5 };
            s.c.sB = c2v { x: 0.5, y: 0.5 };
            s.a.u = 1.0;
            s.b.u = 1.0;
            s.c.u = 1.0;
            s.div = 3.0;
            s
        };

        for count in 1..=3 {
            let mut s_c = make_simplex();
            s_c.count = count;
            let mut s_r = s_c;
            let (cm, rm) = get_pair::<FM>(&c, &r, b"c2GJKSimplexMetric");
            assert!(bits_eq_f32(cm(&mut s_c), rm(&mut s_r)), "metric count={}", count);

            // c2D
            let (cd, rd) = get_pair::<FV>(&c, &r, b"c2D");
            assert!(bits_eq_v(cd(&mut s_c), rd(&mut s_r)), "cD count={}", count);

            // c2L
            let (cl, rl) = get_pair::<FV>(&c, &r, b"c2L");
            assert!(bits_eq_v(cl(&mut s_c), rl(&mut s_r)), "cL count={}", count);

            // c2Witness
            let (cw, rw) = get_pair::<FW>(&c, &r, b"c2Witness");
            let mut ac = c2v::default();
            let mut bc = c2v::default();
            let mut ar = c2v::default();
            let mut br = c2v::default();
            cw(&mut s_c, &mut ac, &mut bc);
            rw(&mut s_r, &mut ar, &mut br);
            assert!(bits_eq_v(ac, ar) && bits_eq_v(bc, br), "witness count={}", count);
        }

        // c22
        let mut s_c = make_simplex();
        s_c.count = 2;
        let mut s_r = s_c;
        let (cf, rf) = get_pair::<FS>(&c, &r, b"c22");
        cf(&mut s_c);
        rf(&mut s_r);
        assert_eq!(s_c.count, s_r.count);
        assert!(bits_eq_f32(s_c.div, s_r.div));

        // c23
        let mut s_c = make_simplex();
        s_c.count = 3;
        let mut s_r = s_c;
        let (cf, rf) = get_pair::<FS>(&c, &r, b"c23");
        cf(&mut s_c);
        rf(&mut s_r);
        assert_eq!(s_c.count, s_r.count);
        assert!(bits_eq_f32(s_c.div, s_r.div));
    }
}

#[test]
fn test_c2AABBtoAABB() {
    unsafe {
        let (c, r) = load_libs();
        type F = unsafe extern "C" fn(c2AABB, c2AABB) -> c_int;
        let (cf, rf) = get_pair::<F>(&c, &r, b"c2AABBtoAABB");
        let cases = [
            (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
             c2AABB { min: c2v { x: 0.5, y: 0.5 }, max: c2v { x: 1.5, y: 1.5 } }),
            (c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
             c2AABB { min: c2v { x: 2.0, y: 2.0 }, max: c2v { x: 3.0, y: 3.0 } }),
        ];
        for &(a, b) in &cases {
            assert_eq!(cf(a, b), rf(a, b));
        }
    }
}

#[test]
fn test_c2CircletoCircle_AABB_Capsule() {
    unsafe {
        let (c, r) = load_libs();
        type FCC = unsafe extern "C" fn(c2Circle, c2Circle) -> c_int;
        type FCA = unsafe extern "C" fn(c2Circle, c2AABB) -> c_int;
        type FCAP = unsafe extern "C" fn(c2Circle, c2Capsule) -> c_int;

        let circle1 = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 5.0 };
        let circle2 = c2Circle { p: c2v { x: 3.0, y: 0.0 }, r: 5.0 };
        let circle3 = c2Circle { p: c2v { x: 100.0, y: 0.0 }, r: 1.0 };
        let aabb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
        let cap = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: 1.0 };

        let (cf, rf) = get_pair::<FCC>(&c, &r, b"c2CircletoCircle");
        assert_eq!(cf(circle1, circle2), rf(circle1, circle2));
        assert_eq!(cf(circle1, circle3), rf(circle1, circle3));

        let (cf, rf) = get_pair::<FCA>(&c, &r, b"c2CircletoAABB");
        assert_eq!(cf(circle1, aabb), rf(circle1, aabb));
        assert_eq!(cf(circle3, aabb), rf(circle3, aabb));

        let (cf, rf) = get_pair::<FCAP>(&c, &r, b"c2CircletoCapsule");
        assert_eq!(cf(circle1, cap), rf(circle1, cap));
        assert_eq!(cf(circle3, cap), rf(circle3, cap));
    }
}

#[test]
fn test_c2GJK() {
    unsafe {
        let (c, r) = load_libs();
        type F = unsafe extern "C" fn(
            *const c_void, c_int, *const c2x,
            *const c_void, c_int, *const c2x,
            *mut c2v, *mut c2v, c_int,
            *mut c_int, *mut c2GJKCache,
        ) -> f32;
        let (cf, rf) = get_pair::<F>(&c, &r, b"c2GJK");

        let aabb1 = c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } };
        let cap = c2Capsule { a: c2v { x: 5.0, y: 0.5 }, b: c2v { x: 6.0, y: 0.5 }, r: 0.5 };
        let mut outA_c = c2v::default();
        let mut outB_c = c2v::default();
        let mut outA_r = c2v::default();
        let mut outB_r = c2v::default();
        let mut iter_c: c_int = 0;
        let mut iter_r: c_int = 0;
        let dc = cf(
            &aabb1 as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
            &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
            &mut outA_c, &mut outB_c, 1, &mut iter_c, std::ptr::null_mut(),
        );
        let dr = rf(
            &aabb1 as *const _ as *const c_void, C2_TYPE_AABB, std::ptr::null(),
            &cap as *const _ as *const c_void, C2_TYPE_CAPSULE, std::ptr::null(),
            &mut outA_r, &mut outB_r, 1, &mut iter_r, std::ptr::null_mut(),
        );
        assert!(bits_eq_f32(dc, dr), "c2GJK distance: {} vs {}", dc, dr);
        assert!(bits_eq_v(outA_c, outA_r));
        assert!(bits_eq_v(outB_c, outB_r));
        assert_eq!(iter_c, iter_r);
    }
}

#[test]
fn test_c2Collided() {
    unsafe {
        let (c, r) = load_libs();
        type F = unsafe extern "C" fn(*const c_void, c_int, *const c_void, c_int) -> c_int;
        let (cf, rf) = get_pair::<F>(&c, &r, b"c2Collided");

        let circle = c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 5.0 };
        let aabb = c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } };
        let cap = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 10.0, y: 0.0 }, r: 1.0 };

        let pairs: &[(*const c_void, c_int, *const c_void, c_int)] = &[
            (&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, &aabb as *const _ as *const c_void, C2_TYPE_AABB),
            (&aabb as *const _ as *const c_void, C2_TYPE_AABB, &cap as *const _ as *const c_void, C2_TYPE_CAPSULE),
            (&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &circle as *const _ as *const c_void, C2_TYPE_CIRCLE),
        ];
        for &(a, ta, b, tb) in pairs {
            assert_eq!(cf(a, ta, b, tb), rf(a, ta, b, tb));
        }
    }
}

#[test]
fn test_aabb_top_level() {
    unsafe {
        let (c, r) = load_libs();
        type F = unsafe extern "C" fn(f32, f32, f32, f32) -> c_int;
        let (cf, rf) = get_pair::<F>(&c, &r, b"aabb");
        let cases = [
            (-1.0_f32, -1.0_f32, 1.0_f32, 1.0_f32),
            (-100.0, -100.0, 100.0, 100.0),
            (-50.0, -50.0, 0.0, 0.0),
            (-25.0, 50.0, 0.0, 110.0),
            (0.0, 0.0, 1.0, 1.0),
        ];
        for &(a, b, c2, d) in &cases {
            let cv = cf(a, b, c2, d);
            let rv = rf(a, b, c2, d);
            assert_eq!(cv, rv, "aabb({},{},{},{}) C={} Rust={}", a, b, c2, d, cv, rv);
        }
    }
}
