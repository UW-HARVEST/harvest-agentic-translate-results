//! Cross-library parity tests.
//!
//! Each test loads BOTH the C reference shared library and the Rust
//! translation via libloading, calls the same function with identical inputs
//! through both .so files, and asserts that the byte-level outputs match.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use libloading::{Library, Symbol};
use std::ffi::{c_int, c_void};

// ----- Mirrored C ABI structs -----
#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct c2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct c2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct c2x {
    p: c2v,
    r: c2r,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct c2Circle {
    p: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct c2AABB {
    min: c2v,
    max: c2v,
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
struct c2Capsule {
    a: c2v,
    b: c2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
struct c2GJKCache {
    metric: f32,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: f32,
}

// C2_TYPE enum (4-byte int per the C definition)
const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

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

// Bytewise float equality so we can compare NaNs / signed zeros etc.
fn bits_eq_f32(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

fn vec_eq(a: c2v, b: c2v) -> bool {
    bits_eq_f32(a.x, b.x) && bits_eq_f32(a.y, b.y)
}

fn rot_eq(a: c2r, b: c2r) -> bool {
    bits_eq_f32(a.c, b.c) && bits_eq_f32(a.s, b.s)
}

fn xform_eq(a: c2x, b: c2x) -> bool {
    vec_eq(a.p, b.p) && rot_eq(a.r, b.r)
}

fn proxy_eq(a: &c2Proxy, b: &c2Proxy) -> bool {
    if !bits_eq_f32(a.radius, b.radius) {
        return false;
    }
    if a.count != b.count {
        return false;
    }
    for i in 0..8 {
        if !vec_eq(a.verts[i], b.verts[i]) {
            return false;
        }
    }
    true
}

fn simplex_eq(a: &c2Simplex, b: &c2Simplex) -> bool {
    let sv_eq = |x: &c2sv, y: &c2sv| {
        vec_eq(x.sA, y.sA)
            && vec_eq(x.sB, y.sB)
            && vec_eq(x.p, y.p)
            && bits_eq_f32(x.u, y.u)
            && x.iA == y.iA
            && x.iB == y.iB
    };
    sv_eq(&a.a, &b.a)
        && sv_eq(&a.b, &b.b)
        && sv_eq(&a.c, &b.c)
        && sv_eq(&a.d, &b.d)
        && bits_eq_f32(a.div, b.div)
        && a.count == b.count
}

fn cache_eq(a: &c2GJKCache, b: &c2GJKCache) -> bool {
    bits_eq_f32(a.metric, b.metric)
        && a.count == b.count
        && a.iA == b.iA
        && a.iB == b.iB
        && bits_eq_f32(a.div, b.div)
}

// ----- Library loading -----
struct Libs {
    c: Library,
    r: Library,
}

fn load_libs() -> Libs {
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_path = manifest.join("c_src/build/libtranslated_rust.so");
    // Cargo puts the integration-test binary's deps in target/<profile>/deps/.
    // The cdylib lives in target/<profile>/. CARGO_TARGET_DIR may or may not
    // be set; rely on CARGO_MANIFEST_DIR + standard layout.
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| manifest.join("target"));

    // Try debug first since `cargo test` builds in debug, then release.
    let candidates = [
        target_dir.join("debug").join("libgjk_lib.so"),
        target_dir.join("release").join("libgjk_lib.so"),
    ];
    let r_path = candidates
        .iter()
        .find(|p| p.exists())
        .unwrap_or_else(|| panic!("Could not find libgjk_lib.so under {:?}", target_dir))
        .clone();

    unsafe {
        Libs {
            c: Library::new(&c_path)
                .unwrap_or_else(|e| panic!("loading C lib at {:?}: {}", c_path, e)),
            r: Library::new(&r_path)
                .unwrap_or_else(|e| panic!("loading Rust lib at {:?}: {}", r_path, e)),
        }
    }
}

// Helper to fetch a typed symbol from each library.
unsafe fn sym<'lib, T>(lib: &'lib Library, name: &[u8]) -> Symbol<'lib, T> {
    unsafe { lib.get(name).expect("symbol lookup") }
}

// ===== Tests for individual exported functions =====

fn rng_pairs() -> Vec<(f32, f32)> {
    vec![
        (0.0, 0.0),
        (1.0, 0.0),
        (-1.0, 1.0),
        (1.5, -2.5),
        (3.4e10, -1.2e-5),
        (-1e-7, 1e-7),
        (123.456, -789.012),
        (0.5, 0.5),
        (-0.5, -0.5),
        (1e30, 1e30),
    ]
}

fn vec_samples() -> Vec<c2v> {
    rng_pairs().into_iter().map(|(x, y)| c2v { x, y }).collect()
}

#[test]
fn test_c2V() {
    let libs = load_libs();
    type Fn_ = unsafe extern "C" fn(f32, f32) -> c2v;
    unsafe {
        let f_c: Symbol<Fn_> = sym(&libs.c, b"c2V");
        let f_r: Symbol<Fn_> = sym(&libs.r, b"c2V");
        for (x, y) in rng_pairs() {
            let rc = f_c(x, y);
            let rr = f_r(x, y);
            assert!(vec_eq(rc, rr), "c2V({},{}): C={:?} R={:?}", x, y, rc, rr);
        }
    }
}

#[test]
fn test_c2Mulvs() {
    let libs = load_libs();
    type Fn_ = unsafe extern "C" fn(c2v, f32) -> c2v;
    unsafe {
        let f_c: Symbol<Fn_> = sym(&libs.c, b"c2Mulvs");
        let f_r: Symbol<Fn_> = sym(&libs.r, b"c2Mulvs");
        for v in vec_samples() {
            for s in [-2.5f32, 0.0, 1.0, 0.5, 1e10, -1e-6] {
                let rc = f_c(v, s);
                let rr = f_r(v, s);
                assert!(vec_eq(rc, rr), "c2Mulvs({:?},{}): C={:?} R={:?}", v, s, rc, rr);
            }
        }
    }
}

#[test]
fn test_c2Maxv_Minv_Clampv() {
    let libs = load_libs();
    type Fn2 = unsafe extern "C" fn(c2v, c2v) -> c2v;
    type Fn3 = unsafe extern "C" fn(c2v, c2v, c2v) -> c2v;
    unsafe {
        let max_c: Symbol<Fn2> = sym(&libs.c, b"c2Maxv");
        let max_r: Symbol<Fn2> = sym(&libs.r, b"c2Maxv");
        let min_c: Symbol<Fn2> = sym(&libs.c, b"c2Minv");
        let min_r: Symbol<Fn2> = sym(&libs.r, b"c2Minv");
        let clp_c: Symbol<Fn3> = sym(&libs.c, b"c2Clampv");
        let clp_r: Symbol<Fn3> = sym(&libs.r, b"c2Clampv");
        for a in vec_samples() {
            for b in vec_samples() {
                assert!(vec_eq(max_c(a, b), max_r(a, b)));
                assert!(vec_eq(min_c(a, b), min_r(a, b)));
                let lo = c2v {
                    x: a.x.min(b.x),
                    y: a.y.min(b.y),
                };
                let hi = c2v {
                    x: a.x.max(b.x),
                    y: a.y.max(b.y),
                };
                let mid = c2v {
                    x: (a.x + b.x) / 2.0,
                    y: (a.y + b.y) / 2.0,
                };
                assert!(vec_eq(clp_c(mid, lo, hi), clp_r(mid, lo, hi)));
            }
        }
    }
}

#[test]
fn test_c2Sub_Add_Dot() {
    let libs = load_libs();
    type FnVV = unsafe extern "C" fn(c2v, c2v) -> c2v;
    type FnVF = unsafe extern "C" fn(c2v, c2v) -> f32;
    unsafe {
        let sub_c: Symbol<FnVV> = sym(&libs.c, b"c2Sub");
        let sub_r: Symbol<FnVV> = sym(&libs.r, b"c2Sub");
        let add_c: Symbol<FnVV> = sym(&libs.c, b"c2Add");
        let add_r: Symbol<FnVV> = sym(&libs.r, b"c2Add");
        let dot_c: Symbol<FnVF> = sym(&libs.c, b"c2Dot");
        let dot_r: Symbol<FnVF> = sym(&libs.r, b"c2Dot");
        for a in vec_samples() {
            for b in vec_samples() {
                assert!(vec_eq(sub_c(a, b), sub_r(a, b)));
                assert!(vec_eq(add_c(a, b), add_r(a, b)));
                assert!(bits_eq_f32(dot_c(a, b), dot_r(a, b)));
            }
        }
    }
}

#[test]
fn test_c2RotIdentity_xIdentity() {
    let libs = load_libs();
    type FnR = unsafe extern "C" fn() -> c2r;
    type FnX = unsafe extern "C" fn() -> c2x;
    unsafe {
        let r_c: Symbol<FnR> = sym(&libs.c, b"c2RotIdentity");
        let r_r: Symbol<FnR> = sym(&libs.r, b"c2RotIdentity");
        assert!(rot_eq(r_c(), r_r()));
        let x_c: Symbol<FnX> = sym(&libs.c, b"c2xIdentity");
        let x_r: Symbol<FnX> = sym(&libs.r, b"c2xIdentity");
        assert!(xform_eq(x_c(), x_r()));
    }
}

#[test]
fn test_c2BBVerts() {
    let libs = load_libs();
    type Fn_ = unsafe extern "C" fn(*mut c2v, *mut c2AABB);
    unsafe {
        let f_c: Symbol<Fn_> = sym(&libs.c, b"c2BBVerts");
        let f_r: Symbol<Fn_> = sym(&libs.r, b"c2BBVerts");
        let bbs = [
            c2AABB {
                min: c2v { x: 0.0, y: 0.0 },
                max: c2v { x: 1.0, y: 1.0 },
            },
            c2AABB {
                min: c2v { x: -5.0, y: -3.0 },
                max: c2v { x: 5.0, y: 3.0 },
            },
            c2AABB {
                min: c2v { x: 1.0, y: 2.0 },
                max: c2v { x: 1.0, y: 2.0 },
            },
        ];
        for mut bb in bbs {
            let mut out_c = [c2v::default(); 4];
            let mut out_r = [c2v::default(); 4];
            f_c(out_c.as_mut_ptr(), &mut bb as *mut c2AABB);
            f_r(out_r.as_mut_ptr(), &mut bb as *mut c2AABB);
            for i in 0..4 {
                assert!(vec_eq(out_c[i], out_r[i]), "vert {}: {:?} vs {:?}", i, out_c[i], out_r[i]);
            }
        }
    }
}

#[test]
fn test_c2MakeProxy() {
    let libs = load_libs();
    type Fn_ = unsafe extern "C" fn(*const c_void, c_int, *mut c2Proxy);
    unsafe {
        let f_c: Symbol<Fn_> = sym(&libs.c, b"c2MakeProxy");
        let f_r: Symbol<Fn_> = sym(&libs.r, b"c2MakeProxy");

        let circle = c2Circle {
            p: c2v { x: 1.0, y: 2.0 },
            r: 0.5,
        };
        let mut pc = c2Proxy::default();
        let mut pr = c2Proxy::default();
        f_c(&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut pc);
        f_r(&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut pr);
        assert!(proxy_eq(&pc, &pr));

        let bb = c2AABB {
            min: c2v { x: -1.0, y: -2.0 },
            max: c2v { x: 3.0, y: 4.0 },
        };
        let mut pc = c2Proxy::default();
        let mut pr = c2Proxy::default();
        f_c(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut pc);
        f_r(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut pr);
        assert!(proxy_eq(&pc, &pr));

        let cap = c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 1.0, y: 0.0 },
            r: 0.25,
        };
        let mut pc = c2Proxy::default();
        let mut pr = c2Proxy::default();
        f_c(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut pc);
        f_r(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut pr);
        assert!(proxy_eq(&pc, &pr));
    }
}

#[test]
fn test_c2Len_Det2() {
    let libs = load_libs();
    type Fn1 = unsafe extern "C" fn(c2v) -> f32;
    type Fn2 = unsafe extern "C" fn(c2v, c2v) -> f32;
    unsafe {
        let l_c: Symbol<Fn1> = sym(&libs.c, b"c2Len");
        let l_r: Symbol<Fn1> = sym(&libs.r, b"c2Len");
        let d_c: Symbol<Fn2> = sym(&libs.c, b"c2Det2");
        let d_r: Symbol<Fn2> = sym(&libs.r, b"c2Det2");
        for a in vec_samples() {
            assert!(bits_eq_f32(l_c(a), l_r(a)));
            for b in vec_samples() {
                assert!(bits_eq_f32(d_c(a, b), d_r(a, b)));
            }
        }
    }
}

#[test]
fn test_c2Mulrv_Mulxv_MulrvT() {
    let libs = load_libs();
    type FnRV = unsafe extern "C" fn(c2r, c2v) -> c2v;
    type FnXV = unsafe extern "C" fn(c2x, c2v) -> c2v;
    unsafe {
        let mr_c: Symbol<FnRV> = sym(&libs.c, b"c2Mulrv");
        let mr_r: Symbol<FnRV> = sym(&libs.r, b"c2Mulrv");
        let mrt_c: Symbol<FnRV> = sym(&libs.c, b"c2MulrvT");
        let mrt_r: Symbol<FnRV> = sym(&libs.r, b"c2MulrvT");
        let mx_c: Symbol<FnXV> = sym(&libs.c, b"c2Mulxv");
        let mx_r: Symbol<FnXV> = sym(&libs.r, b"c2Mulxv");
        let rots = [
            c2r { c: 1.0, s: 0.0 },
            c2r { c: 0.0, s: 1.0 },
            c2r {
                c: 0.7071068,
                s: 0.7071068,
            },
            c2r { c: -0.5, s: 0.866 },
        ];
        for r in rots {
            for v in vec_samples() {
                assert!(vec_eq(mr_c(r, v), mr_r(r, v)));
                assert!(vec_eq(mrt_c(r, v), mrt_r(r, v)));
                let xform = c2x {
                    p: c2v { x: 1.0, y: -2.0 },
                    r,
                };
                assert!(vec_eq(mx_c(xform, v), mx_r(xform, v)));
            }
        }
    }
}

#[test]
fn test_c2Neg_Skew_CCW90_Div_Norm() {
    let libs = load_libs();
    type Fn1 = unsafe extern "C" fn(c2v) -> c2v;
    type FnS = unsafe extern "C" fn(c2v, f32) -> c2v;
    unsafe {
        let n_c: Symbol<Fn1> = sym(&libs.c, b"c2Neg");
        let n_r: Symbol<Fn1> = sym(&libs.r, b"c2Neg");
        let sk_c: Symbol<Fn1> = sym(&libs.c, b"c2Skew");
        let sk_r: Symbol<Fn1> = sym(&libs.r, b"c2Skew");
        let cc_c: Symbol<Fn1> = sym(&libs.c, b"c2CCW90");
        let cc_r: Symbol<Fn1> = sym(&libs.r, b"c2CCW90");
        let nm_c: Symbol<Fn1> = sym(&libs.c, b"c2Norm");
        let nm_r: Symbol<Fn1> = sym(&libs.r, b"c2Norm");
        let dv_c: Symbol<FnS> = sym(&libs.c, b"c2Div");
        let dv_r: Symbol<FnS> = sym(&libs.r, b"c2Div");
        for v in vec_samples() {
            assert!(vec_eq(n_c(v), n_r(v)));
            assert!(vec_eq(sk_c(v), sk_r(v)));
            assert!(vec_eq(cc_c(v), cc_r(v)));
            // skip zero vector for normalize
            if v.x != 0.0 || v.y != 0.0 {
                assert!(vec_eq(nm_c(v), nm_r(v)));
            }
            for s in [-1.0f32, 0.5, 2.0, 100.0] {
                assert!(vec_eq(dv_c(v, s), dv_r(v, s)));
            }
        }
    }
}

fn build_simplex_count1() -> c2Simplex {
    let mut s = c2Simplex::default();
    s.a.sA = c2v { x: 1.0, y: 1.0 };
    s.a.sB = c2v { x: 5.0, y: 1.0 };
    s.a.p = c2v { x: 4.0, y: 0.0 };
    s.a.u = 1.0;
    s.a.iA = 0;
    s.a.iB = 0;
    s.div = 1.0;
    s.count = 1;
    s
}

fn build_simplex_count2() -> c2Simplex {
    let mut s = build_simplex_count1();
    s.b.sA = c2v { x: -1.0, y: 0.5 };
    s.b.sB = c2v { x: 4.0, y: 0.5 };
    s.b.p = c2v { x: 5.0, y: 0.0 };
    s.b.u = 0.5;
    s.b.iA = 1;
    s.b.iB = 1;
    s.div = 1.5;
    s.count = 2;
    s
}

fn build_simplex_count3() -> c2Simplex {
    let mut s = build_simplex_count2();
    s.c.sA = c2v { x: 0.0, y: -1.0 };
    s.c.sB = c2v { x: 4.0, y: -1.0 };
    s.c.p = c2v { x: 4.0, y: 0.0 };
    s.c.u = 0.25;
    s.c.iA = 2;
    s.c.iB = 2;
    s.div = 1.75;
    s.count = 3;
    s
}

#[test]
fn test_c2GJKSimplexMetric() {
    let libs = load_libs();
    type Fn_ = unsafe extern "C" fn(*mut c2Simplex) -> f32;
    unsafe {
        let f_c: Symbol<Fn_> = sym(&libs.c, b"c2GJKSimplexMetric");
        let f_r: Symbol<Fn_> = sym(&libs.r, b"c2GJKSimplexMetric");
        for mut s in [
            build_simplex_count1(),
            build_simplex_count2(),
            build_simplex_count3(),
        ] {
            let rc = f_c(&mut s);
            let rr = f_r(&mut s);
            assert!(bits_eq_f32(rc, rr), "metric c={} r={}", rc, rr);
        }
    }
}

#[test]
fn test_c22_c23() {
    let libs = load_libs();
    type Fn_ = unsafe extern "C" fn(*mut c2Simplex);
    unsafe {
        let c22_c: Symbol<Fn_> = sym(&libs.c, b"c22");
        let c22_r: Symbol<Fn_> = sym(&libs.r, b"c22");
        let c23_c: Symbol<Fn_> = sym(&libs.c, b"c23");
        let c23_r: Symbol<Fn_> = sym(&libs.r, b"c23");

        // c22 — try several count-2 simplexes.
        let mut bases = vec![build_simplex_count2()];
        // mutate to exercise different branches
        let mut alt = build_simplex_count2();
        alt.a.p = c2v { x: -1.0, y: -1.0 };
        alt.b.p = c2v { x: 1.0, y: 1.0 };
        bases.push(alt);
        let mut alt2 = build_simplex_count2();
        alt2.a.p = c2v { x: 5.0, y: 5.0 };
        alt2.b.p = c2v { x: 4.0, y: 4.0 };
        bases.push(alt2);

        for mut s in bases {
            let mut sc = s;
            let mut sr = s;
            c22_c(&mut sc);
            c22_r(&mut sr);
            assert!(simplex_eq(&sc, &sr), "c22 mismatch");
            let _ = &mut s;
        }

        // c23 — try several count-3 simplexes, plus a couple where we vary a/b/c.
        let mut bases = vec![build_simplex_count3()];
        let mut alt = build_simplex_count3();
        alt.a.p = c2v { x: 0.0, y: 1.0 };
        alt.b.p = c2v { x: 1.0, y: 0.0 };
        alt.c.p = c2v { x: -1.0, y: 0.0 };
        bases.push(alt);
        let mut alt2 = build_simplex_count3();
        alt2.a.p = c2v { x: 2.0, y: 2.0 };
        alt2.b.p = c2v { x: 3.0, y: 0.0 };
        alt2.c.p = c2v { x: 0.0, y: 3.0 };
        bases.push(alt2);
        let mut alt3 = build_simplex_count3();
        alt3.a.p = c2v { x: -3.0, y: -3.0 };
        alt3.b.p = c2v { x: -1.0, y: -1.0 };
        alt3.c.p = c2v { x: -2.0, y: 0.5 };
        bases.push(alt3);

        for s in bases {
            let mut sc = s;
            let mut sr = s;
            c23_c(&mut sc);
            c23_r(&mut sr);
            assert!(simplex_eq(&sc, &sr), "c23 mismatch");
        }
    }
}

#[test]
fn test_c2D() {
    let libs = load_libs();
    type Fn_ = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
    unsafe {
        let f_c: Symbol<Fn_> = sym(&libs.c, b"c2D");
        let f_r: Symbol<Fn_> = sym(&libs.r, b"c2D");
        for mut s in [
            build_simplex_count1(),
            build_simplex_count2(),
            build_simplex_count3(),
        ] {
            assert!(vec_eq(f_c(&mut s), f_r(&mut s)));
        }
    }
}

#[test]
fn test_c2Support() {
    let libs = load_libs();
    type Fn_ = unsafe extern "C" fn(*const c2v, c_int, c2v) -> c_int;
    unsafe {
        let f_c: Symbol<Fn_> = sym(&libs.c, b"c2Support");
        let f_r: Symbol<Fn_> = sym(&libs.r, b"c2Support");
        let verts = [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 1.0, y: 1.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: -2.0, y: -2.0 },
        ];
        for d in vec_samples() {
            assert_eq!(
                f_c(verts.as_ptr(), verts.len() as c_int, d),
                f_r(verts.as_ptr(), verts.len() as c_int, d),
            );
        }
    }
}

#[test]
fn test_c2Witness_c2L() {
    let libs = load_libs();
    type FnW = unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v);
    type FnL = unsafe extern "C" fn(*mut c2Simplex) -> c2v;
    unsafe {
        let w_c: Symbol<FnW> = sym(&libs.c, b"c2Witness");
        let w_r: Symbol<FnW> = sym(&libs.r, b"c2Witness");
        let l_c: Symbol<FnL> = sym(&libs.c, b"c2L");
        let l_r: Symbol<FnL> = sym(&libs.r, b"c2L");
        for mut s in [
            build_simplex_count1(),
            build_simplex_count2(),
            build_simplex_count3(),
        ] {
            let mut ac = c2v::default();
            let mut bc = c2v::default();
            let mut ar = c2v::default();
            let mut br = c2v::default();
            w_c(&mut s, &mut ac, &mut bc);
            w_r(&mut s, &mut ar, &mut br);
            assert!(vec_eq(ac, ar));
            assert!(vec_eq(bc, br));
            assert!(vec_eq(l_c(&mut s), l_r(&mut s)));
        }
    }
}

// ----- High-level c2GJK / gjk parity -----

#[test]
fn test_c2GJK_aabb_capsule() {
    let libs = load_libs();
    type Fn_ = unsafe extern "C" fn(
        *const c_void,
        c_int,
        *const c2x,
        *const c_void,
        c_int,
        *const c2x,
        *mut c2v,
        *mut c2v,
        c_int,
        *mut c_int,
        *mut c2GJKCache,
    ) -> f32;
    unsafe {
        let f_c: Symbol<Fn_> = sym(&libs.c, b"c2GJK");
        let f_r: Symbol<Fn_> = sym(&libs.r, b"c2GJK");

        let cases: Vec<(c2AABB, c2Capsule)> = vec![
            (
                c2AABB {
                    min: c2v { x: 0.0, y: 0.0 },
                    max: c2v { x: 1.0, y: 1.0 },
                },
                c2Capsule {
                    a: c2v { x: 3.0, y: 0.5 },
                    b: c2v { x: 4.0, y: 0.5 },
                    r: 0.25,
                },
            ),
            (
                c2AABB {
                    min: c2v { x: -1.0, y: -1.0 },
                    max: c2v { x: 1.0, y: 1.0 },
                },
                c2Capsule {
                    a: c2v { x: 0.0, y: 0.5 },
                    b: c2v { x: 0.0, y: 0.0 },
                    r: 0.5,
                },
            ),
            (
                c2AABB {
                    min: c2v { x: -2.0, y: -2.0 },
                    max: c2v { x: -1.0, y: -1.0 },
                },
                c2Capsule {
                    a: c2v { x: 1.0, y: 1.0 },
                    b: c2v { x: 2.0, y: 2.0 },
                    r: 0.5,
                },
            ),
            (
                c2AABB {
                    min: c2v { x: 0.0, y: 0.0 },
                    max: c2v { x: 10.0, y: 10.0 },
                },
                c2Capsule {
                    a: c2v { x: -5.0, y: -5.0 },
                    b: c2v { x: -3.0, y: -3.0 },
                    r: 1.0,
                },
            ),
            (
                c2AABB {
                    min: c2v { x: 0.0, y: 0.0 },
                    max: c2v { x: 1.0, y: 1.0 },
                },
                c2Capsule {
                    a: c2v { x: 1.5, y: 0.5 },
                    b: c2v { x: 1.6, y: 0.5 },
                    r: 0.1,
                },
            ),
        ];
        for (bb, cap) in cases {
            for use_radius in [0_i32, 1] {
                let mut ac = c2v::default();
                let mut bc = c2v::default();
                let mut ar = c2v::default();
                let mut br = c2v::default();
                let mut iters_c: c_int = 0;
                let mut iters_r: c_int = 0;
                let dist_c = f_c(
                    &bb as *const _ as *const c_void,
                    C2_TYPE_AABB,
                    std::ptr::null(),
                    &cap as *const _ as *const c_void,
                    C2_TYPE_CAPSULE,
                    std::ptr::null(),
                    &mut ac,
                    &mut bc,
                    use_radius,
                    &mut iters_c,
                    std::ptr::null_mut(),
                );
                let dist_r = f_r(
                    &bb as *const _ as *const c_void,
                    C2_TYPE_AABB,
                    std::ptr::null(),
                    &cap as *const _ as *const c_void,
                    C2_TYPE_CAPSULE,
                    std::ptr::null(),
                    &mut ar,
                    &mut br,
                    use_radius,
                    &mut iters_r,
                    std::ptr::null_mut(),
                );
                assert!(
                    bits_eq_f32(dist_c, dist_r),
                    "dist mismatch use_radius={} bb={:?} cap={:?}: C={} R={}",
                    use_radius,
                    bb,
                    cap,
                    dist_c,
                    dist_r
                );
                assert!(vec_eq(ac, ar), "outA mismatch");
                assert!(vec_eq(bc, br), "outB mismatch");
                assert_eq!(iters_c, iters_r, "iter mismatch");
            }
        }
    }
}

#[test]
fn test_c2GJK_with_cache() {
    let libs = load_libs();
    type Fn_ = unsafe extern "C" fn(
        *const c_void,
        c_int,
        *const c2x,
        *const c_void,
        c_int,
        *const c2x,
        *mut c2v,
        *mut c2v,
        c_int,
        *mut c_int,
        *mut c2GJKCache,
    ) -> f32;
    unsafe {
        let f_c: Symbol<Fn_> = sym(&libs.c, b"c2GJK");
        let f_r: Symbol<Fn_> = sym(&libs.r, b"c2GJK");

        let bb = c2AABB {
            min: c2v { x: 0.0, y: 0.0 },
            max: c2v { x: 2.0, y: 2.0 },
        };
        let cap = c2Capsule {
            a: c2v { x: 5.0, y: 1.0 },
            b: c2v { x: 6.0, y: 1.0 },
            r: 0.5,
        };
        let mut cache_c = c2GJKCache::default();
        let mut cache_r = c2GJKCache::default();
        for _ in 0..3 {
            let mut ac = c2v::default();
            let mut bc = c2v::default();
            let mut ar = c2v::default();
            let mut br = c2v::default();
            let dist_c = f_c(
                &bb as *const _ as *const c_void,
                C2_TYPE_AABB,
                std::ptr::null(),
                &cap as *const _ as *const c_void,
                C2_TYPE_CAPSULE,
                std::ptr::null(),
                &mut ac,
                &mut bc,
                1,
                std::ptr::null_mut(),
                &mut cache_c,
            );
            let dist_r = f_r(
                &bb as *const _ as *const c_void,
                C2_TYPE_AABB,
                std::ptr::null(),
                &cap as *const _ as *const c_void,
                C2_TYPE_CAPSULE,
                std::ptr::null(),
                &mut ar,
                &mut br,
                1,
                std::ptr::null_mut(),
                &mut cache_r,
            );
            assert!(bits_eq_f32(dist_c, dist_r));
            assert!(vec_eq(ac, ar));
            assert!(vec_eq(bc, br));
            assert!(cache_eq(&cache_c, &cache_r));
        }
    }
}

#[test]
fn test_gjk_top_level() {
    let libs = load_libs();
    type Fn_ = unsafe extern "C" fn(
        std::ffi::c_char,
        *mut c2v,
        *mut c2v,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
        f32,
    );
    unsafe {
        let f_c: Symbol<Fn_> = sym(&libs.c, b"gjk");
        let f_r: Symbol<Fn_> = sym(&libs.r, b"gjk");
        let cases: Vec<(f32, f32, f32, f32, f32, f32, f32, f32, f32)> = vec![
            (0.0, 0.0, 1.0, 1.0, 3.0, 0.5, 4.0, 0.5, 0.25),
            (-1.0, -1.0, 1.0, 1.0, 0.0, 0.5, 0.0, 0.0, 0.5),
            (-2.0, -2.0, -1.0, -1.0, 1.0, 1.0, 2.0, 2.0, 0.5),
            (0.0, 0.0, 10.0, 10.0, -5.0, -5.0, -3.0, -3.0, 1.0),
            (0.0, 0.0, 1.0, 1.0, 1.5, 0.5, 1.6, 0.5, 0.1),
            (0.0, 0.0, 5.0, 5.0, 2.5, 2.5, 2.6, 2.6, 0.1),
        ];
        for (a1, a2, a3, a4, b1, b2, b3, b4, b5) in cases {
            for reverse in [0i8, 1] {
                let mut ac = c2v::default();
                let mut bc = c2v::default();
                let mut ar = c2v::default();
                let mut br = c2v::default();
                f_c(reverse as _, &mut ac, &mut bc, a1, a2, a3, a4, b1, b2, b3, b4, b5);
                f_r(reverse as _, &mut ar, &mut br, a1, a2, a3, a4, b1, b2, b3, b4, b5);
                assert!(
                    vec_eq(ac, ar),
                    "gjk a-out mismatch reverse={} bb=({},{}-{},{}) cap=({},{}-{}, {} r={}): C={:?} R={:?}",
                    reverse, a1, a2, a3, a4, b1, b2, b3, b4, b5, ac, ar,
                );
                assert!(vec_eq(bc, br), "gjk b-out mismatch");
            }
        }
    }
}
