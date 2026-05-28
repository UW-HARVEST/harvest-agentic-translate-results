//! Integration tests comparing the C reference .so against the Rust .so.
//!
//! Both libraries are loaded via `libloading` and called through their FFI
//! exports. Outputs must match byte-for-byte.

use libloading::{Library, Symbol};
use std::ffi::c_int;
use std::path::PathBuf;

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
struct C2v {
    x: f32,
    y: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
struct C2r {
    c: f32,
    s: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
struct C2GJKCache {
    metric: f32,
    count: c_int,
    i_a: [c_int; 3],
    i_b: [c_int; 3],
    div: f32,
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
        Self {
            radius: 0.0,
            count: 0,
            verts: [C2v::default(); 8],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
struct C2sv {
    s_a: C2v,
    s_b: C2v,
    p: C2v,
    u: f32,
    i_a: c_int,
    i_b: c_int,
}

#[repr(C)]
#[derive(Copy, Clone, Default, Debug, PartialEq)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: c_int,
}

fn lib_paths() -> (PathBuf, PathBuf) {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let c_so = manifest.join("c_src/build/libtranslated_rust.so");
    // pick whichever Rust .so exists (debug for `cargo test`)
    let candidates = [
        manifest.join("target/debug/libreverse_collide_lib.so"),
        manifest.join("target/release/libreverse_collide_lib.so"),
    ];
    let rust_so = candidates
        .iter()
        .find(|p| p.exists())
        .cloned()
        .unwrap_or_else(|| candidates[0].clone());
    (c_so, rust_so)
}

fn load_libs() -> (Library, Library) {
    let (c_so, rust_so) = lib_paths();
    unsafe {
        // Preload libm globally so the C .so's unresolved sqrtf, etc., bind.
        // The C build doesn't link to -lm explicitly.
        use libloading::os::unix::{Library as UnixLib, RTLD_GLOBAL, RTLD_NOW};
        let _libm = UnixLib::open(Some("libm.so.6"), RTLD_NOW | RTLD_GLOBAL)
            .or_else(|_| UnixLib::open(Some("libm.so"), RTLD_NOW | RTLD_GLOBAL))
            .expect("preload libm");
        // Leak so the symbols stay resolvable for the entire test process.
        std::mem::forget(_libm);

        let c = Library::new(&c_so).unwrap_or_else(|e| panic!("load C {:?}: {}", c_so, e));
        let r = Library::new(&rust_so).unwrap_or_else(|e| panic!("load Rust {:?}: {}", rust_so, e));
        (c, r)
    }
}

fn bits_eq_f32(a: f32, b: f32) -> bool {
    a.to_bits() == b.to_bits()
}

fn bits_eq_v(a: C2v, b: C2v) -> bool {
    bits_eq_f32(a.x, b.x) && bits_eq_f32(a.y, b.y)
}

// ---------------------------------------------------------------------------
// Lowest-level: c2V, simple math
// ---------------------------------------------------------------------------

#[test]
fn test_c2v() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(f32, f32) -> C2v;
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2V").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2V").unwrap();
        for &(x, y) in &[
            (0.0, 0.0),
            (1.0, -1.0),
            (3.14, 2.71),
            (-1e9, 1e-9),
            (f32::INFINITY, f32::NEG_INFINITY),
        ] {
            let cv = cf(x, y);
            let rv = rf(x, y);
            assert!(bits_eq_v(cv, rv), "c2V({},{}): C={:?} R={:?}", x, y, cv, rv);
        }
    }
}

#[test]
fn test_c2_simple_vec_ops() {
    let (c, r) = load_libs();
    type FvvV = unsafe extern "C" fn(C2v, C2v) -> C2v;
    type FvvF = unsafe extern "C" fn(C2v, C2v) -> f32;
    type FvfV = unsafe extern "C" fn(C2v, f32) -> C2v;
    type FvV = unsafe extern "C" fn(C2v) -> C2v;
    type FvF = unsafe extern "C" fn(C2v) -> f32;
    unsafe {
        let names_vvV: &[&[u8]] = &[b"c2Maxv", b"c2Minv", b"c2Sub", b"c2Add"];
        let inputs = [
            (C2v { x: 1.0, y: 2.0 }, C2v { x: 3.0, y: 4.0 }),
            (C2v { x: -1.5, y: 0.0 }, C2v { x: 1.5, y: -2.0 }),
            (C2v { x: 100.0, y: 100.0 }, C2v { x: 100.0, y: 100.0 }),
        ];
        for n in names_vvV {
            let cf: Symbol<FvvV> = c.get(n).unwrap();
            let rf: Symbol<FvvV> = r.get(n).unwrap();
            for &(a, b) in &inputs {
                let cv = cf(a, b);
                let rv = rf(a, b);
                assert!(
                    bits_eq_v(cv, rv),
                    "{:?}({:?},{:?}): C={:?} R={:?}",
                    String::from_utf8_lossy(n),
                    a,
                    b,
                    cv,
                    rv
                );
            }
        }
        let names_vvF: &[&[u8]] = &[b"c2Dot", b"c2Det2"];
        for n in names_vvF {
            let cf: Symbol<FvvF> = c.get(n).unwrap();
            let rf: Symbol<FvvF> = r.get(n).unwrap();
            for &(a, b) in &inputs {
                let cv = cf(a, b);
                let rv = rf(a, b);
                assert!(bits_eq_f32(cv, rv), "{:?}: C={} R={}", String::from_utf8_lossy(n), cv, rv);
            }
        }
        // c2Mulvs and c2Div
        let cf: Symbol<FvfV> = c.get(b"c2Mulvs").unwrap();
        let rf: Symbol<FvfV> = r.get(b"c2Mulvs").unwrap();
        for &(a, _) in &inputs {
            for s in [0.0f32, 1.0, -2.5, 1e9] {
                let cv = cf(a, s);
                let rv = rf(a, s);
                assert!(bits_eq_v(cv, rv), "c2Mulvs: C={:?} R={:?}", cv, rv);
            }
        }
        let cf: Symbol<FvfV> = c.get(b"c2Div").unwrap();
        let rf: Symbol<FvfV> = r.get(b"c2Div").unwrap();
        for &(a, _) in &inputs {
            for s in [1.0f32, -2.5, 100.0] {
                let cv = cf(a, s);
                let rv = rf(a, s);
                assert!(bits_eq_v(cv, rv));
            }
        }

        // unary v->v
        let names_vV: &[&[u8]] = &[b"c2Neg", b"c2Skew", b"c2CCW90", b"c2Norm"];
        for n in names_vV {
            let cf: Symbol<FvV> = c.get(n).unwrap();
            let rf: Symbol<FvV> = r.get(n).unwrap();
            let inps = [
                C2v { x: 1.0, y: 2.0 },
                C2v { x: -3.0, y: 4.0 },
                C2v { x: 5.0, y: -12.0 },
            ];
            for a in inps {
                let cv = cf(a);
                let rv = rf(a);
                assert!(
                    bits_eq_v(cv, rv),
                    "{:?}: C={:?} R={:?}",
                    String::from_utf8_lossy(n),
                    cv,
                    rv
                );
            }
        }

        // c2Len
        let cf: Symbol<FvF> = c.get(b"c2Len").unwrap();
        let rf: Symbol<FvF> = r.get(b"c2Len").unwrap();
        for a in [
            C2v { x: 3.0, y: 4.0 },
            C2v { x: 0.0, y: 0.0 },
            C2v { x: -7.0, y: 24.0 },
        ] {
            let cv = cf(a);
            let rv = rf(a);
            assert!(bits_eq_f32(cv, rv));
        }
    }
}

#[test]
fn test_c2_clampv() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2Clampv").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2Clampv").unwrap();
        let cases = [
            (
                C2v { x: 5.0, y: 5.0 },
                C2v { x: 0.0, y: 0.0 },
                C2v { x: 10.0, y: 10.0 },
            ),
            (
                C2v { x: -5.0, y: 15.0 },
                C2v { x: 0.0, y: 0.0 },
                C2v { x: 10.0, y: 10.0 },
            ),
        ];
        for (a, lo, hi) in cases {
            let cv = cf(a, lo, hi);
            let rv = rf(a, lo, hi);
            assert!(bits_eq_v(cv, rv));
        }
    }
}

#[test]
fn test_c2_rot_x_identity() {
    let (c, r) = load_libs();
    type FrR = unsafe extern "C" fn() -> C2r;
    type FxX = unsafe extern "C" fn() -> C2x;
    unsafe {
        let cf: Symbol<FrR> = c.get(b"c2RotIdentity").unwrap();
        let rf: Symbol<FrR> = r.get(b"c2RotIdentity").unwrap();
        let cv = cf();
        let rv = rf();
        assert!(bits_eq_f32(cv.c, rv.c) && bits_eq_f32(cv.s, rv.s));

        let cf: Symbol<FxX> = c.get(b"c2xIdentity").unwrap();
        let rf: Symbol<FxX> = r.get(b"c2xIdentity").unwrap();
        let cv = cf();
        let rv = rf();
        assert!(bits_eq_v(cv.p, rv.p) && bits_eq_f32(cv.r.c, rv.r.c) && bits_eq_f32(cv.r.s, rv.r.s));
    }
}

#[test]
fn test_c2_mulrv_mulrvt_mulxv() {
    let (c, r) = load_libs();
    type FrvV = unsafe extern "C" fn(C2r, C2v) -> C2v;
    type FxvV = unsafe extern "C" fn(C2x, C2v) -> C2v;
    unsafe {
        for name in [b"c2Mulrv".as_ref(), b"c2MulrvT".as_ref()] {
            let cf: Symbol<FrvV> = c.get(name).unwrap();
            let rf: Symbol<FrvV> = r.get(name).unwrap();
            let rot = C2r { c: 0.6, s: 0.8 };
            for v in [
                C2v { x: 1.0, y: 0.0 },
                C2v { x: 0.0, y: 1.0 },
                C2v { x: 3.0, y: -2.5 },
            ] {
                let cv = cf(rot, v);
                let rv = rf(rot, v);
                assert!(bits_eq_v(cv, rv));
            }
        }
        let cf: Symbol<FxvV> = c.get(b"c2Mulxv").unwrap();
        let rf: Symbol<FxvV> = r.get(b"c2Mulxv").unwrap();
        let xform = C2x {
            p: C2v { x: 5.0, y: -3.0 },
            r: C2r { c: 0.6, s: 0.8 },
        };
        for v in [C2v { x: 1.0, y: 0.0 }, C2v { x: -2.0, y: 4.0 }] {
            let cv = cf(xform, v);
            let rv = rf(xform, v);
            assert!(bits_eq_v(cv, rv));
        }
    }
}

#[test]
fn test_c2_bbverts() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(*mut C2v, *mut C2AABB);
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2BBVerts").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2BBVerts").unwrap();
        let mut bb = C2AABB {
            min: C2v { x: -5.0, y: -2.0 },
            max: C2v { x: 3.0, y: 4.0 },
        };
        let mut c_out = [C2v::default(); 4];
        let mut r_out = [C2v::default(); 4];
        cf(c_out.as_mut_ptr(), &mut bb);
        rf(r_out.as_mut_ptr(), &mut bb);
        for i in 0..4 {
            assert!(bits_eq_v(c_out[i], r_out[i]), "vert {}", i);
        }
    }
}

#[test]
fn test_c2_make_proxy() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(*const std::ffi::c_void, c_int, *mut C2Proxy);
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2MakeProxy").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2MakeProxy").unwrap();

        // CIRCLE
        let circle = C2Circle {
            p: C2v { x: 1.0, y: 2.0 },
            r: 3.0,
        };
        let mut cp = C2Proxy::default();
        let mut rp = C2Proxy::default();
        cf(&circle as *const _ as *const _, 0, &mut cp);
        rf(&circle as *const _ as *const _, 0, &mut rp);
        assert_eq!(cp.count, rp.count);
        assert!(bits_eq_f32(cp.radius, rp.radius));
        for i in 0..cp.count as usize {
            assert!(bits_eq_v(cp.verts[i], rp.verts[i]));
        }

        // AABB
        let bb = C2AABB {
            min: C2v { x: -1.0, y: -2.0 },
            max: C2v { x: 3.0, y: 4.0 },
        };
        let mut cp = C2Proxy::default();
        let mut rp = C2Proxy::default();
        cf(&bb as *const _ as *const _, 1, &mut cp);
        rf(&bb as *const _ as *const _, 1, &mut rp);
        assert_eq!(cp.count, rp.count);
        assert!(bits_eq_f32(cp.radius, rp.radius));
        for i in 0..cp.count as usize {
            assert!(bits_eq_v(cp.verts[i], rp.verts[i]));
        }

        // CAPSULE
        let cap = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 5.0, y: 5.0 },
            r: 1.5,
        };
        let mut cp = C2Proxy::default();
        let mut rp = C2Proxy::default();
        cf(&cap as *const _ as *const _, 2, &mut cp);
        rf(&cap as *const _ as *const _, 2, &mut rp);
        assert_eq!(cp.count, rp.count);
        assert!(bits_eq_f32(cp.radius, rp.radius));
        for i in 0..cp.count as usize {
            assert!(bits_eq_v(cp.verts[i], rp.verts[i]));
        }
    }
}

// ---------------------------------------------------------------------------
// Simplex helpers
// ---------------------------------------------------------------------------

fn make_simplex(count: c_int, pts: &[(f32, f32)]) -> C2Simplex {
    let mut s = C2Simplex::default();
    s.count = count;
    s.div = 1.0;
    let mk = |x: f32, y: f32| C2sv {
        s_a: C2v { x, y },
        s_b: C2v { x, y },
        p: C2v { x, y },
        u: 1.0,
        i_a: 0,
        i_b: 0,
    };
    if !pts.is_empty() {
        s.a = mk(pts[0].0, pts[0].1);
    }
    if pts.len() > 1 {
        s.b = mk(pts[1].0, pts[1].1);
    }
    if pts.len() > 2 {
        s.c = mk(pts[2].0, pts[2].1);
    }
    s
}

#[test]
fn test_c2_gjk_simplex_metric_and_simplex_funcs() {
    let (c, r) = load_libs();
    type FsF = unsafe extern "C" fn(*mut C2Simplex) -> f32;
    type FsV = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
    type Fs = unsafe extern "C" fn(*mut C2Simplex);
    unsafe {
        let cf: Symbol<FsF> = c.get(b"c2GJKSimplexMetric").unwrap();
        let rf: Symbol<FsF> = r.get(b"c2GJKSimplexMetric").unwrap();
        let cases: &[(c_int, Vec<(f32, f32)>)] = &[
            (1, vec![(1.0, 2.0)]),
            (2, vec![(0.0, 0.0), (3.0, 4.0)]),
            (3, vec![(0.0, 0.0), (1.0, 0.0), (0.0, 1.0)]),
        ];
        for (cnt, pts) in cases {
            let mut sc = make_simplex(*cnt, pts);
            let mut sr = make_simplex(*cnt, pts);
            let cv = cf(&mut sc);
            let rv = rf(&mut sr);
            assert!(bits_eq_f32(cv, rv));
        }

        // c2L (count 1, 2)
        let cf: Symbol<FsV> = c.get(b"c2L").unwrap();
        let rf: Symbol<FsV> = r.get(b"c2L").unwrap();
        for (cnt, pts) in &cases[..2] {
            let mut sc = make_simplex(*cnt, pts);
            let mut sr = make_simplex(*cnt, pts);
            let cv = cf(&mut sc);
            let rv = rf(&mut sr);
            assert!(bits_eq_v(cv, rv));
        }

        // c2D (count 1, 2, 3)
        let cf: Symbol<FsV> = c.get(b"c2D").unwrap();
        let rf: Symbol<FsV> = r.get(b"c2D").unwrap();
        for (cnt, pts) in cases {
            let mut sc = make_simplex(*cnt, pts);
            let mut sr = make_simplex(*cnt, pts);
            let cv = cf(&mut sc);
            let rv = rf(&mut sr);
            assert!(bits_eq_v(cv, rv));
        }

        // c22, c23: in-place mutation
        let cf: Symbol<Fs> = c.get(b"c22").unwrap();
        let rf: Symbol<Fs> = r.get(b"c22").unwrap();
        let pts2: Vec<(f32, f32)> = vec![(1.0, 0.0), (-1.0, 0.0)];
        let mut sc = make_simplex(2, &pts2);
        let mut sr = make_simplex(2, &pts2);
        cf(&mut sc);
        rf(&mut sr);
        assert_eq!(sc.count, sr.count);
        assert!(bits_eq_f32(sc.div, sr.div));

        let cf: Symbol<Fs> = c.get(b"c23").unwrap();
        let rf: Symbol<Fs> = r.get(b"c23").unwrap();
        let pts3: Vec<(f32, f32)> = vec![(0.0, 1.0), (-1.0, -1.0), (1.0, -1.0)];
        let mut sc = make_simplex(3, &pts3);
        let mut sr = make_simplex(3, &pts3);
        cf(&mut sc);
        rf(&mut sr);
        assert_eq!(sc.count, sr.count);
        assert!(bits_eq_f32(sc.div, sr.div));
    }
}

#[test]
fn test_c2_support() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int;
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2Support").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2Support").unwrap();
        let verts = [
            C2v { x: 0.0, y: 0.0 },
            C2v { x: 1.0, y: 0.0 },
            C2v { x: 1.0, y: 1.0 },
            C2v { x: 0.0, y: 1.0 },
        ];
        for d in [
            C2v { x: 1.0, y: 0.0 },
            C2v { x: 0.0, y: 1.0 },
            C2v { x: -1.0, y: 0.0 },
            C2v { x: 1.0, y: 1.0 },
        ] {
            let cv = cf(verts.as_ptr(), 4, d);
            let rv = rf(verts.as_ptr(), 4, d);
            assert_eq!(cv, rv);
        }
    }
}

// ---------------------------------------------------------------------------
// Shape-vs-shape predicates
// ---------------------------------------------------------------------------

#[test]
fn test_circle_circle() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2CircletoCircle").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2CircletoCircle").unwrap();
        let cases = [
            (C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 }, C2Circle { p: C2v { x: 1.0, y: 0.0 }, r: 1.0 }),
            (C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 }, C2Circle { p: C2v { x: 5.0, y: 0.0 }, r: 1.0 }),
            (C2Circle { p: C2v { x: -10.0, y: 5.0 }, r: 3.0 }, C2Circle { p: C2v { x: -8.0, y: 5.0 }, r: 1.0 }),
        ];
        for (a, b) in cases {
            assert_eq!(cf(a, b), rf(a, b));
        }
    }
}

#[test]
fn test_aabb_aabb() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(C2AABB, C2AABB) -> c_int;
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2AABBtoAABB").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2AABBtoAABB").unwrap();
        let mk = |a: (f32, f32), b: (f32, f32)| C2AABB {
            min: C2v { x: a.0, y: a.1 },
            max: C2v { x: b.0, y: b.1 },
        };
        let cases = [
            (mk((-1.0, -1.0), (1.0, 1.0)), mk((0.0, 0.0), (2.0, 2.0))),
            (mk((-1.0, -1.0), (1.0, 1.0)), mk((10.0, 10.0), (12.0, 12.0))),
            (mk((-5.0, -5.0), (5.0, 5.0)), mk((-2.0, -2.0), (2.0, 2.0))),
        ];
        for (a, b) in cases {
            assert_eq!(cf(a, b), rf(a, b));
        }
    }
}

#[test]
fn test_circle_aabb() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(C2Circle, C2AABB) -> c_int;
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2CircletoAABB").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2CircletoAABB").unwrap();
        let bb = C2AABB {
            min: C2v { x: -1.0, y: -1.0 },
            max: C2v { x: 1.0, y: 1.0 },
        };
        for c1 in [
            C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 0.5 },
            C2Circle { p: C2v { x: 5.0, y: 0.0 }, r: 1.0 },
            C2Circle { p: C2v { x: 1.5, y: 1.5 }, r: 0.5 },
            C2Circle { p: C2v { x: 1.5, y: 1.5 }, r: 1.0 },
        ] {
            assert_eq!(cf(c1, bb), rf(c1, bb));
        }
    }
}

#[test]
fn test_circle_capsule() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int;
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2CircletoCapsule").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2CircletoCapsule").unwrap();
        let cap = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 10.0, y: 0.0 },
            r: 1.0,
        };
        for c1 in [
            C2Circle { p: C2v { x: 5.0, y: 0.0 }, r: 0.5 },
            C2Circle { p: C2v { x: -5.0, y: 0.0 }, r: 0.5 },
            C2Circle { p: C2v { x: 15.0, y: 0.0 }, r: 0.5 },
            C2Circle { p: C2v { x: 5.0, y: 3.0 }, r: 1.0 },
            C2Circle { p: C2v { x: 5.0, y: 1.5 }, r: 1.0 },
        ] {
            assert_eq!(cf(c1, cap), rf(c1, cap));
        }
    }
}

#[test]
fn test_aabb_capsule() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(C2AABB, C2Capsule) -> c_int;
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2AABBtoCapsule").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2AABBtoCapsule").unwrap();
        let bb = C2AABB {
            min: C2v { x: -1.0, y: -1.0 },
            max: C2v { x: 1.0, y: 1.0 },
        };
        let caps = [
            C2Capsule { a: C2v { x: 5.0, y: 0.0 }, b: C2v { x: 10.0, y: 0.0 }, r: 1.0 },
            C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 5.0, y: 5.0 }, r: 1.0 },
            C2Capsule { a: C2v { x: 2.0, y: 2.0 }, b: C2v { x: 5.0, y: 5.0 }, r: 0.5 },
        ];
        for cap in caps {
            assert_eq!(cf(bb, cap), rf(bb, cap));
        }
    }
}

#[test]
fn test_capsule_capsule() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(C2Capsule, C2Capsule) -> c_int;
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2CapsuletoCapsule").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2CapsuletoCapsule").unwrap();
        let cap1 = C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 10.0, y: 0.0 }, r: 1.0 };
        let caps = [
            C2Capsule { a: C2v { x: 5.0, y: 1.5 }, b: C2v { x: 5.0, y: 5.0 }, r: 1.0 },
            C2Capsule { a: C2v { x: 50.0, y: 50.0 }, b: C2v { x: 60.0, y: 50.0 }, r: 1.0 },
            C2Capsule { a: C2v { x: 5.0, y: 0.0 }, b: C2v { x: 5.0, y: 10.0 }, r: 0.5 },
        ];
        for cap2 in caps {
            assert_eq!(cf(cap1, cap2), rf(cap1, cap2));
        }
    }
}

#[test]
fn test_c2_collided() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(*const std::ffi::c_void, c_int, *const std::ffi::c_void, c_int) -> c_int;
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2Collided").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2Collided").unwrap();
        let ca = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 };
        let cb = C2Circle { p: C2v { x: 1.5, y: 0.0 }, r: 1.0 };
        let bb = C2AABB { min: C2v { x: -1.0, y: -1.0 }, max: C2v { x: 1.0, y: 1.0 } };
        let cap = C2Capsule { a: C2v { x: 0.0, y: 0.0 }, b: C2v { x: 5.0, y: 0.0 }, r: 0.5 };
        // (typeA, typeB, ptrA, ptrB) — all 9 type combinations
        let test = |ta: c_int, tb: c_int, pa: *const std::ffi::c_void, pb: *const std::ffi::c_void| {
            let cv = cf(pa, ta, pb, tb);
            let rv = rf(pa, ta, pb, tb);
            assert_eq!(cv, rv, "ta={} tb={}", ta, tb);
        };
        test(0, 0, &ca as *const _ as _, &cb as *const _ as _);
        test(0, 1, &ca as *const _ as _, &bb as *const _ as _);
        test(0, 2, &ca as *const _ as _, &cap as *const _ as _);
        test(1, 0, &bb as *const _ as _, &ca as *const _ as _);
        test(1, 1, &bb as *const _ as _, &bb as *const _ as _);
        test(1, 2, &bb as *const _ as _, &cap as *const _ as _);
        test(2, 0, &cap as *const _ as _, &ca as *const _ as _);
        test(2, 1, &cap as *const _ as _, &bb as *const _ as _);
        test(2, 2, &cap as *const _ as _, &cap as *const _ as _);
    }
}

// ---------------------------------------------------------------------------
// c2GJK — full FFI surface
// ---------------------------------------------------------------------------

#[test]
fn test_c2_gjk_basic() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(
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
    ) -> f32;
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"c2GJK").unwrap();
        let rf: Symbol<Fn_> = r.get(b"c2GJK").unwrap();

        let ca = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: 1.0 };
        let cb = C2Circle { p: C2v { x: 5.0, y: 0.0 }, r: 1.0 };
        let bb = C2AABB { min: C2v { x: -2.0, y: -2.0 }, max: C2v { x: 2.0, y: 2.0 } };
        let cap = C2Capsule { a: C2v { x: 5.0, y: 0.0 }, b: C2v { x: 5.0, y: 5.0 }, r: 1.0 };

        let mut run = |use_radius: c_int,
                       pa: *const std::ffi::c_void,
                       ta: c_int,
                       pb: *const std::ffi::c_void,
                       tb: c_int,
                       label: &str| {
            for use_cache in [0, 1] {
                let mut c_out_a = C2v::default();
                let mut c_out_b = C2v::default();
                let mut c_iter: c_int = 0;
                let mut c_cache = C2GJKCache::default();

                let mut r_out_a = C2v::default();
                let mut r_out_b = C2v::default();
                let mut r_iter: c_int = 0;
                let mut r_cache = C2GJKCache::default();

                let c_dist = cf(
                    pa,
                    ta,
                    std::ptr::null(),
                    pb,
                    tb,
                    std::ptr::null(),
                    &mut c_out_a,
                    &mut c_out_b,
                    use_radius,
                    &mut c_iter,
                    if use_cache != 0 { &mut c_cache } else { std::ptr::null_mut() },
                );
                let r_dist = rf(
                    pa,
                    ta,
                    std::ptr::null(),
                    pb,
                    tb,
                    std::ptr::null(),
                    &mut r_out_a,
                    &mut r_out_b,
                    use_radius,
                    &mut r_iter,
                    if use_cache != 0 { &mut r_cache } else { std::ptr::null_mut() },
                );
                assert!(
                    bits_eq_f32(c_dist, r_dist),
                    "{} ur={} cache={}: dist C={} R={}",
                    label,
                    use_radius,
                    use_cache,
                    c_dist,
                    r_dist
                );
                assert!(bits_eq_v(c_out_a, r_out_a), "{} outA mismatch", label);
                assert!(bits_eq_v(c_out_b, r_out_b), "{} outB mismatch", label);
                assert_eq!(c_iter, r_iter, "{} iter mismatch", label);
                if use_cache != 0 {
                    assert!(bits_eq_f32(c_cache.metric, r_cache.metric));
                    assert_eq!(c_cache.count, r_cache.count);
                    assert_eq!(c_cache.i_a, r_cache.i_a);
                    assert_eq!(c_cache.i_b, r_cache.i_b);
                    assert!(bits_eq_f32(c_cache.div, r_cache.div));
                }
            }
        };

        run(0, &ca as *const _ as _, 0, &cb as *const _ as _, 0, "circle-circle");
        run(1, &ca as *const _ as _, 0, &cb as *const _ as _, 0, "circle-circle-r");
        run(1, &bb as *const _ as _, 1, &cap as *const _ as _, 2, "aabb-cap");
        run(0, &bb as *const _ as _, 1, &cap as *const _ as _, 2, "aabb-cap-no-r");

        // Overlapping shapes (hit==1)
        let cb2 = C2Circle { p: C2v { x: 0.5, y: 0.0 }, r: 1.0 };
        run(1, &ca as *const _ as _, 0, &cb2 as *const _ as _, 0, "overlap-circles");
    }
}

// ---------------------------------------------------------------------------
// Top-level: reverse_collide
// ---------------------------------------------------------------------------

#[test]
fn test_reverse_collide() {
    let (c, r) = load_libs();
    type Fn_ = unsafe extern "C" fn(f32, f32, f32) -> c_int;
    unsafe {
        let cf: Symbol<Fn_> = c.get(b"reverse_collide").unwrap();
        let rf: Symbol<Fn_> = r.get(b"reverse_collide").unwrap();
        let cases = [
            (-70.0f32, 0.0f32, 20.0f32),
            (0.0, 0.0, 5.0),
            (-30.0, -30.0, 5.0),
            (-25.0, 60.0, 8.0),
            (100.0, 100.0, 1.0),
            (-50.0, 0.0, 20.0),
            (-20.0, 80.0, 6.0),
        ];
        for (x, y, r_) in cases {
            let cv = cf(x, y, r_);
            let rv = rf(x, y, r_);
            assert_eq!(cv, rv, "reverse_collide({},{},{})", x, y, r_);
        }
    }
}
