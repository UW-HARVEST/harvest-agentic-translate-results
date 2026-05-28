// Integration test that loads BOTH the C .so and the Rust .so via libloading
// and compares every exported function's outputs byte-for-byte.

use libloading::{Library, Symbol};
use std::os::raw::{c_char, c_int, c_void};
use std::path::PathBuf;

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
struct C2v {
    x: f32,
    y: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
struct C2r {
    c: f32,
    s: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
struct C2x {
    p: C2v,
    r: C2r,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
struct C2Circle {
    p: C2v,
    r: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
struct C2AABB {
    min: C2v,
    max: C2v,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
struct C2Capsule {
    a: C2v,
    b: C2v,
    r: f32,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(C)]
struct C2GJKCache {
    metric: f32,
    count: c_int,
    iA: [c_int; 3],
    iB: [c_int; 3],
    div: f32,
}

#[derive(Copy, Clone)]
#[repr(C)]
struct C2Proxy {
    radius: f32,
    count: c_int,
    verts: [C2v; 8],
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
struct C2sv {
    sA: C2v,
    sB: C2v,
    p: C2v,
    u: f32,
    iA: c_int,
    iB: c_int,
}

impl Default for C2sv {
    fn default() -> Self {
        C2sv {
            sA: C2v { x: 0.0, y: 0.0 },
            sB: C2v { x: 0.0, y: 0.0 },
            p: C2v { x: 0.0, y: 0.0 },
            u: 0.0,
            iA: 0,
            iB: 0,
        }
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
struct C2Simplex {
    a: C2sv,
    b: C2sv,
    c: C2sv,
    d: C2sv,
    div: f32,
    count: c_int,
}

impl Default for C2Simplex {
    fn default() -> Self {
        C2Simplex {
            a: C2sv::default(),
            b: C2sv::default(),
            c: C2sv::default(),
            d: C2sv::default(),
            div: 0.0,
            count: 0,
        }
    }
}

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

fn c_lib_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("c_src/build/libtranslated_rust.so")
}

fn rust_lib_path() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // CARGO_TARGET_DIR or default target/debug
    let target = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir.join("target"));
    // Try debug then release
    for profile in &["debug", "release"] {
        let p = target.join(profile).join("libgjk_cache_lib.so");
        if p.exists() {
            return p;
        }
    }
    target.join("debug/libgjk_cache_lib.so")
}

unsafe fn load_libs() -> (Library, Library) {
    unsafe {
        let c = Library::new(c_lib_path()).expect("load C lib");
        let r = Library::new(rust_lib_path()).expect("load Rust lib");
        (c, r)
    }
}

fn fbits(f: f32) -> u32 {
    f.to_bits()
}

fn vbits(v: C2v) -> (u32, u32) {
    (fbits(v.x), fbits(v.y))
}

fn assert_v_eq(c: C2v, r: C2v, ctx: &str) {
    assert_eq!(vbits(c), vbits(r), "c2v mismatch [{ctx}]: C={c:?} R={r:?}");
}

fn assert_f_eq(c: f32, r: f32, ctx: &str) {
    assert_eq!(fbits(c), fbits(r), "f32 mismatch [{ctx}]: C={c} R={r}");
}

fn vecs() -> Vec<C2v> {
    vec![
        C2v { x: 0.0, y: 0.0 },
        C2v { x: 1.0, y: 0.0 },
        C2v { x: 0.0, y: 1.0 },
        C2v { x: -1.0, y: -1.0 },
        C2v { x: 3.5, y: -2.25 },
        C2v { x: 100.0, y: -25.0 },
        C2v { x: 75.0, y: 100.0 },
        C2v { x: -1e-7, y: 1e-7 },
        C2v { x: 1e8, y: -1e8 },
        C2v { x: 1.7320508, y: 0.5 },
    ]
}

#[test]
fn test_c2v_constructors() {
    unsafe {
        let (c, r) = load_libs();
        type Fn2 = unsafe extern "C" fn(f32, f32) -> C2v;
        let cf: Symbol<Fn2> = c.get(b"c2V").unwrap();
        let rf: Symbol<Fn2> = r.get(b"c2V").unwrap();
        for x in [0.0_f32, 1.0, -1.5, 1e6, -3.14159] {
            for y in [0.0_f32, 1.0, -2.5, 1e6, -1e-6] {
                let cv = cf(x, y);
                let rv = rf(x, y);
                assert_v_eq(cv, rv, &format!("c2V({x},{y})"));
            }
        }
    }
}

#[test]
fn test_c2_unary_vec() {
    unsafe {
        let (c, r) = load_libs();
        type Fn1 = unsafe extern "C" fn(C2v) -> C2v;
        for name in [b"c2Neg" as &[u8], b"c2Skew", b"c2CCW90", b"c2Norm"] {
            let cf: Symbol<Fn1> = c.get(name).unwrap();
            let rf: Symbol<Fn1> = r.get(name).unwrap();
            for v in vecs() {
                if std::str::from_utf8(name).unwrap() == "c2Norm" {
                    if v.x == 0.0 && v.y == 0.0 {
                        continue;
                    }
                }
                let cv = cf(v);
                let rv = rf(v);
                let nm = std::str::from_utf8(name).unwrap();
                assert_v_eq(cv, rv, &format!("{nm}({v:?})"));
            }
        }
    }
}

#[test]
fn test_c2_binary_vec_vec_to_vec() {
    unsafe {
        let (c, r) = load_libs();
        type Fn2 = unsafe extern "C" fn(C2v, C2v) -> C2v;
        for name in [
            b"c2Maxv" as &[u8],
            b"c2Minv",
            b"c2Sub",
            b"c2Add",
        ] {
            let cf: Symbol<Fn2> = c.get(name).unwrap();
            let rf: Symbol<Fn2> = r.get(name).unwrap();
            for a in vecs() {
                for b in vecs() {
                    let cv = cf(a, b);
                    let rv = rf(a, b);
                    let nm = std::str::from_utf8(name).unwrap();
                    assert_v_eq(cv, rv, &format!("{nm}({a:?},{b:?})"));
                }
            }
        }
    }
}

#[test]
fn test_c2_binary_vec_vec_to_scalar() {
    unsafe {
        let (c, r) = load_libs();
        type Fn2 = unsafe extern "C" fn(C2v, C2v) -> f32;
        for name in [b"c2Dot" as &[u8], b"c2Det2"] {
            let cf: Symbol<Fn2> = c.get(name).unwrap();
            let rf: Symbol<Fn2> = r.get(name).unwrap();
            for a in vecs() {
                for b in vecs() {
                    let cv = cf(a, b);
                    let rv = rf(a, b);
                    let nm = std::str::from_utf8(name).unwrap();
                    assert_f_eq(cv, rv, &format!("{nm}({a:?},{b:?})"));
                }
            }
        }
    }
}

#[test]
fn test_c2_len() {
    unsafe {
        let (c, r) = load_libs();
        type Fn1 = unsafe extern "C" fn(C2v) -> f32;
        let cf: Symbol<Fn1> = c.get(b"c2Len").unwrap();
        let rf: Symbol<Fn1> = r.get(b"c2Len").unwrap();
        for v in vecs() {
            assert_f_eq(cf(v), rf(v), &format!("c2Len({v:?})"));
        }
    }
}

#[test]
fn test_c2_clampv() {
    unsafe {
        let (c, r) = load_libs();
        type Fn3 = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
        let cf: Symbol<Fn3> = c.get(b"c2Clampv").unwrap();
        let rf: Symbol<Fn3> = r.get(b"c2Clampv").unwrap();
        for a in vecs() {
            for lo in vecs() {
                for hi in vecs() {
                    let cv = cf(a, lo, hi);
                    let rv = rf(a, lo, hi);
                    assert_v_eq(cv, rv, "c2Clampv");
                }
            }
        }
    }
}

#[test]
fn test_c2_mulvs_div() {
    unsafe {
        let (c, r) = load_libs();
        type Fn = unsafe extern "C" fn(C2v, f32) -> C2v;
        for name in [b"c2Mulvs" as &[u8], b"c2Div"] {
            let cf: Symbol<Fn> = c.get(name).unwrap();
            let rf: Symbol<Fn> = r.get(name).unwrap();
            for v in vecs() {
                for s in [1.0_f32, -1.0, 0.5, 2.0, 1e-5, 1e5] {
                    let cv = cf(v, s);
                    let rv = rf(v, s);
                    let nm = std::str::from_utf8(name).unwrap();
                    assert_v_eq(cv, rv, &format!("{nm}({v:?},{s})"));
                }
            }
        }
    }
}

#[test]
fn test_c2_rot_identity() {
    unsafe {
        let (c, r) = load_libs();
        type Fn0 = unsafe extern "C" fn() -> C2r;
        let cf: Symbol<Fn0> = c.get(b"c2RotIdentity").unwrap();
        let rf: Symbol<Fn0> = r.get(b"c2RotIdentity").unwrap();
        let cv = cf();
        let rv = rf();
        assert_eq!(fbits(cv.c), fbits(rv.c));
        assert_eq!(fbits(cv.s), fbits(rv.s));
    }
}

#[test]
fn test_c2_x_identity() {
    unsafe {
        let (c, r) = load_libs();
        type Fn0 = unsafe extern "C" fn() -> C2x;
        let cf: Symbol<Fn0> = c.get(b"c2xIdentity").unwrap();
        let rf: Symbol<Fn0> = r.get(b"c2xIdentity").unwrap();
        let cv = cf();
        let rv = rf();
        assert_v_eq(cv.p, rv.p, "c2xIdentity.p");
        assert_eq!(fbits(cv.r.c), fbits(rv.r.c));
        assert_eq!(fbits(cv.r.s), fbits(rv.r.s));
    }
}

#[test]
fn test_c2_mulrv_mulrvT() {
    unsafe {
        let (c, r) = load_libs();
        type Fn2 = unsafe extern "C" fn(C2r, C2v) -> C2v;
        for name in [b"c2Mulrv" as &[u8], b"c2MulrvT"] {
            let cf: Symbol<Fn2> = c.get(name).unwrap();
            let rf: Symbol<Fn2> = r.get(name).unwrap();
            for rot in [
                C2r { c: 1.0, s: 0.0 },
                C2r { c: 0.0, s: 1.0 },
                C2r { c: 0.7071, s: 0.7071 },
                C2r { c: -0.5, s: 0.866 },
            ] {
                for v in vecs() {
                    let cv = cf(rot, v);
                    let rv = rf(rot, v);
                    let nm = std::str::from_utf8(name).unwrap();
                    assert_v_eq(cv, rv, &format!("{nm}({rot:?},{v:?})"));
                }
            }
        }
    }
}

#[test]
fn test_c2_mulxv() {
    unsafe {
        let (c, r) = load_libs();
        type Fn2 = unsafe extern "C" fn(C2x, C2v) -> C2v;
        let cf: Symbol<Fn2> = c.get(b"c2Mulxv").unwrap();
        let rf: Symbol<Fn2> = r.get(b"c2Mulxv").unwrap();
        let xs = [
            C2x {
                p: C2v { x: 0.0, y: 0.0 },
                r: C2r { c: 1.0, s: 0.0 },
            },
            C2x {
                p: C2v { x: 1.0, y: -2.0 },
                r: C2r { c: 0.7071, s: 0.7071 },
            },
            C2x {
                p: C2v { x: 100.0, y: 50.0 },
                r: C2r { c: 0.0, s: 1.0 },
            },
        ];
        for x in xs {
            for v in vecs() {
                assert_v_eq(cf(x, v), rf(x, v), "c2Mulxv");
            }
        }
    }
}

#[test]
fn test_c2_bbverts() {
    unsafe {
        let (c, r) = load_libs();
        type Fn2 = unsafe extern "C" fn(*mut C2v, *mut C2AABB);
        let cf: Symbol<Fn2> = c.get(b"c2BBVerts").unwrap();
        let rf: Symbol<Fn2> = r.get(b"c2BBVerts").unwrap();
        let bbs = [
            C2AABB {
                min: C2v { x: 0.0, y: 0.0 },
                max: C2v { x: 10.0, y: 10.0 },
            },
            C2AABB {
                min: C2v { x: -5.0, y: -3.0 },
                max: C2v { x: 5.0, y: 3.0 },
            },
        ];
        for mut bb in bbs {
            let mut co = [C2v { x: 0.0, y: 0.0 }; 4];
            let mut ro = [C2v { x: 0.0, y: 0.0 }; 4];
            cf(co.as_mut_ptr(), &mut bb as *mut _);
            rf(ro.as_mut_ptr(), &mut bb as *mut _);
            for i in 0..4 {
                assert_v_eq(co[i], ro[i], &format!("c2BBVerts[{i}]"));
            }
        }
    }
}

#[test]
fn test_c2_make_proxy() {
    unsafe {
        let (c, r) = load_libs();
        type Fn3 = unsafe extern "C" fn(*const c_void, c_int, *mut C2Proxy);
        let cf: Symbol<Fn3> = c.get(b"c2MakeProxy").unwrap();
        let rf: Symbol<Fn3> = r.get(b"c2MakeProxy").unwrap();

        let circle = C2Circle {
            p: C2v { x: 1.0, y: 2.0 },
            r: 5.0,
        };
        let mut cp = std::mem::zeroed::<C2Proxy>();
        let mut rp = std::mem::zeroed::<C2Proxy>();
        cf(&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut cp);
        rf(&circle as *const _ as *const c_void, C2_TYPE_CIRCLE, &mut rp);
        assert_eq!(fbits(cp.radius), fbits(rp.radius));
        assert_eq!(cp.count, rp.count);
        assert_v_eq(cp.verts[0], rp.verts[0], "circle vert0");

        let bb = C2AABB {
            min: C2v { x: -1.0, y: -1.0 },
            max: C2v { x: 2.0, y: 3.0 },
        };
        let mut cp = std::mem::zeroed::<C2Proxy>();
        let mut rp = std::mem::zeroed::<C2Proxy>();
        cf(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut cp);
        rf(&bb as *const _ as *const c_void, C2_TYPE_AABB, &mut rp);
        assert_eq!(fbits(cp.radius), fbits(rp.radius));
        assert_eq!(cp.count, rp.count);
        for i in 0..4 {
            assert_v_eq(cp.verts[i], rp.verts[i], &format!("aabb vert{i}"));
        }

        let cap = C2Capsule {
            a: C2v { x: 0.0, y: 0.0 },
            b: C2v { x: 10.0, y: 5.0 },
            r: 2.0,
        };
        let mut cp = std::mem::zeroed::<C2Proxy>();
        let mut rp = std::mem::zeroed::<C2Proxy>();
        cf(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut cp);
        rf(&cap as *const _ as *const c_void, C2_TYPE_CAPSULE, &mut rp);
        assert_eq!(fbits(cp.radius), fbits(rp.radius));
        assert_eq!(cp.count, rp.count);
        for i in 0..2 {
            assert_v_eq(cp.verts[i], rp.verts[i], &format!("cap vert{i}"));
        }
    }
}

#[test]
fn test_c2_support() {
    unsafe {
        let (c, r) = load_libs();
        type Fn3 = unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int;
        let cf: Symbol<Fn3> = c.get(b"c2Support").unwrap();
        let rf: Symbol<Fn3> = r.get(b"c2Support").unwrap();
        let verts = [
            C2v { x: 0.0, y: 0.0 },
            C2v { x: 10.0, y: 0.0 },
            C2v { x: 10.0, y: 10.0 },
            C2v { x: 0.0, y: 10.0 },
        ];
        for d in vecs() {
            let cv = cf(verts.as_ptr(), 4, d);
            let rv = rf(verts.as_ptr(), 4, d);
            assert_eq!(cv, rv, "c2Support({d:?})");
        }
    }
}

fn make_simplex_count2() -> C2Simplex {
    C2Simplex {
        a: C2sv {
            sA: C2v { x: 0.0, y: 0.0 },
            sB: C2v { x: 1.0, y: 0.0 },
            p: C2v { x: 1.0, y: 0.0 },
            u: 0.5,
            iA: 0,
            iB: 0,
        },
        b: C2sv {
            sA: C2v { x: 0.0, y: 0.0 },
            sB: C2v { x: 0.0, y: 1.0 },
            p: C2v { x: 0.0, y: 1.0 },
            u: 0.5,
            iA: 0,
            iB: 1,
        },
        c: C2sv::default(),
        d: C2sv::default(),
        div: 1.0,
        count: 2,
    }
}

fn make_simplex_count3() -> C2Simplex {
    let mut s = make_simplex_count2();
    s.c = C2sv {
        sA: C2v { x: 0.0, y: 0.0 },
        sB: C2v { x: 1.0, y: 1.0 },
        p: C2v { x: -1.0, y: -1.0 },
        u: 0.33,
        iA: 0,
        iB: 2,
    };
    s.div = 1.5;
    s.count = 3;
    s
}

#[test]
fn test_c2_simplex_metric() {
    unsafe {
        let (c, r) = load_libs();
        type Fn1 = unsafe extern "C" fn(*mut C2Simplex) -> f32;
        let cf: Symbol<Fn1> = c.get(b"c2GJKSimplexMetric").unwrap();
        let rf: Symbol<Fn1> = r.get(b"c2GJKSimplexMetric").unwrap();
        for mut s in [C2Simplex::default(), make_simplex_count2(), make_simplex_count3()] {
            let mut s2 = s;
            assert_f_eq(cf(&mut s), rf(&mut s2), "c2GJKSimplexMetric");
        }
    }
}

#[test]
fn test_c2_d() {
    unsafe {
        let (c, r) = load_libs();
        type Fn1 = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
        let cf: Symbol<Fn1> = c.get(b"c2D").unwrap();
        let rf: Symbol<Fn1> = r.get(b"c2D").unwrap();
        let mut s1 = make_simplex_count2();
        s1.count = 1;
        for mut s in [s1, make_simplex_count2(), make_simplex_count3()] {
            let mut s2 = s;
            assert_v_eq(cf(&mut s), rf(&mut s2), "c2D");
        }
    }
}

#[test]
fn test_c2_l() {
    unsafe {
        let (c, r) = load_libs();
        type Fn1 = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
        let cf: Symbol<Fn1> = c.get(b"c2L").unwrap();
        let rf: Symbol<Fn1> = r.get(b"c2L").unwrap();
        let mut s1 = make_simplex_count2();
        s1.count = 1;
        for mut s in [s1, make_simplex_count2(), make_simplex_count3()] {
            let mut s2 = s;
            assert_v_eq(cf(&mut s), rf(&mut s2), "c2L");
        }
    }
}

#[test]
fn test_c2_witness() {
    unsafe {
        let (c, r) = load_libs();
        type Fn3 = unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v);
        let cf: Symbol<Fn3> = c.get(b"c2Witness").unwrap();
        let rf: Symbol<Fn3> = r.get(b"c2Witness").unwrap();
        let mut s1 = make_simplex_count2();
        s1.count = 1;
        for mut s in [s1, make_simplex_count2(), make_simplex_count3()] {
            let mut s2 = s;
            let mut ca = C2v { x: 0.0, y: 0.0 };
            let mut cb = C2v { x: 0.0, y: 0.0 };
            let mut ra = C2v { x: 0.0, y: 0.0 };
            let mut rb = C2v { x: 0.0, y: 0.0 };
            cf(&mut s, &mut ca, &mut cb);
            rf(&mut s2, &mut ra, &mut rb);
            assert_v_eq(ca, ra, "witness a");
            assert_v_eq(cb, rb, "witness b");
        }
    }
}

#[test]
fn test_c22_c23() {
    unsafe {
        let (c, r) = load_libs();
        type Fn1 = unsafe extern "C" fn(*mut C2Simplex);

        let cf: Symbol<Fn1> = c.get(b"c22").unwrap();
        let rf: Symbol<Fn1> = r.get(b"c22").unwrap();
        let mut s = make_simplex_count2();
        let mut s2 = s;
        cf(&mut s);
        rf(&mut s2);
        assert_eq!(s.count, s2.count);
        assert_eq!(fbits(s.div), fbits(s2.div));
        assert_eq!(fbits(s.a.u), fbits(s2.a.u));
        assert_eq!(fbits(s.b.u), fbits(s2.b.u));

        let cf3: Symbol<Fn1> = c.get(b"c23").unwrap();
        let rf3: Symbol<Fn1> = r.get(b"c23").unwrap();
        let mut s = make_simplex_count3();
        let mut s2 = s;
        cf3(&mut s);
        rf3(&mut s2);
        assert_eq!(s.count, s2.count);
        assert_eq!(fbits(s.div), fbits(s2.div));
        assert_eq!(fbits(s.a.u), fbits(s2.a.u));
        assert_eq!(fbits(s.b.u), fbits(s2.b.u));
        assert_eq!(fbits(s.c.u), fbits(s2.c.u));
    }
}

#[test]
fn test_c2_gjk() {
    unsafe {
        let (c, r) = load_libs();
        type FnGJK = unsafe extern "C" fn(
            *const c_void,
            c_int,
            *const C2x,
            *const c_void,
            c_int,
            *const C2x,
            *mut C2v,
            *mut C2v,
            c_int,
            *mut c_int,
            *mut C2GJKCache,
        ) -> f32;
        let cf: Symbol<FnGJK> = c.get(b"c2GJK").unwrap();
        let rf: Symbol<FnGJK> = r.get(b"c2GJK").unwrap();

        // Circle vs Capsule (same as in gjk_cache)
        let a = C2Circle {
            p: C2v { x: 0.0, y: 0.0 },
            r: 15.0,
        };
        let b = C2Capsule {
            a: C2v { x: 100.0, y: -25.0 },
            b: C2v { x: 75.0, y: 100.0 },
            r: 10.0,
        };
        let mut c_cache = C2GJKCache {
            metric: 0.0,
            count: 0,
            iA: [0; 3],
            iB: [0; 3],
            div: 0.0,
        };
        let mut r_cache = c_cache;

        let mut ca = C2v { x: 0.0, y: 0.0 };
        let mut cb = C2v { x: 0.0, y: 0.0 };
        let mut ra = C2v { x: 0.0, y: 0.0 };
        let mut rb = C2v { x: 0.0, y: 0.0 };
        let mut citer = 0;
        let mut riter = 0;
        let cd = cf(
            &a as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            &b as *const _ as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut ca,
            &mut cb,
            1,
            &mut citer,
            &mut c_cache,
        );
        let rd = rf(
            &a as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            &b as *const _ as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut ra,
            &mut rb,
            1,
            &mut riter,
            &mut r_cache,
        );
        assert_f_eq(cd, rd, "c2GJK dist");
        assert_v_eq(ca, ra, "c2GJK a");
        assert_v_eq(cb, rb, "c2GJK b");
        assert_eq!(citer, riter);
        assert_eq!(c_cache.count, r_cache.count);
        assert_eq!(fbits(c_cache.metric), fbits(r_cache.metric));
        assert_eq!(fbits(c_cache.div), fbits(r_cache.div));
        assert_eq!(c_cache.iA, r_cache.iA);
        assert_eq!(c_cache.iB, r_cache.iB);

        // Re-call (using cache)
        let mut ca2 = C2v { x: 0.0, y: 0.0 };
        let mut cb2 = C2v { x: 0.0, y: 0.0 };
        let mut ra2 = C2v { x: 0.0, y: 0.0 };
        let mut rb2 = C2v { x: 0.0, y: 0.0 };
        let mut citer2 = 0;
        let mut riter2 = 0;
        let cd2 = cf(
            &a as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            &b as *const _ as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut ca2,
            &mut cb2,
            1,
            &mut citer2,
            &mut c_cache,
        );
        let rd2 = rf(
            &a as *const _ as *const c_void,
            C2_TYPE_CIRCLE,
            std::ptr::null(),
            &b as *const _ as *const c_void,
            C2_TYPE_CAPSULE,
            std::ptr::null(),
            &mut ra2,
            &mut rb2,
            1,
            &mut riter2,
            &mut r_cache,
        );
        assert_f_eq(cd2, rd2, "c2GJK cached dist");
        assert_v_eq(ca2, ra2, "c2GJK cached a");
        assert_v_eq(cb2, rb2, "c2GJK cached b");
        assert_eq!(citer2, riter2);
    }
}

#[test]
fn test_gjk_cache_high_level() {
    unsafe {
        let (c, r) = load_libs();
        type FnGjkCache = unsafe extern "C" fn(
            c_char,
            *mut C2v,
            *mut C2v,
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
        let cf: Symbol<FnGjkCache> = c.get(b"gjk_cache").unwrap();
        let rf: Symbol<FnGjkCache> = r.get(b"gjk_cache").unwrap();
        // The C function does not write to a9/b9 (they're unused in the impl
        // beyond the parameter list), but exercise it across a few inputs
        // including the reverse=0/1 paths. Both libs should not crash.
        let cases = [
            (
                0i8, 0.0_f32, 0.0, 10.0, 10.0, 50.0, 50.0, 60.0, 60.0, 5.0,
            ),
            (1i8, -10.0, -10.0, 5.0, 5.0, 0.0, 0.0, 20.0, 20.0, 2.0),
            (0i8, 0.0, 0.0, 100.0, 100.0, 50.0, 50.0, 75.0, 75.0, 1.0),
        ];
        for (rev, a1, a2, a3, a4, b1, b2, b3, b4, b5) in cases {
            let mut a9c = C2v { x: 0.0, y: 0.0 };
            let mut b9c = C2v { x: 0.0, y: 0.0 };
            let mut a9r = C2v { x: 0.0, y: 0.0 };
            let mut b9r = C2v { x: 0.0, y: 0.0 };
            cf(rev, &mut a9c, &mut b9c, a1, a2, a3, a4, b1, b2, b3, b4, b5);
            rf(rev, &mut a9r, &mut b9r, a1, a2, a3, a4, b1, b2, b3, b4, b5);
            // No assert on outputs because the function does not write to
            // a9/b9 in the C impl; the test verifies symbol presence and
            // non-crash behaviour.
        }
    }
}
