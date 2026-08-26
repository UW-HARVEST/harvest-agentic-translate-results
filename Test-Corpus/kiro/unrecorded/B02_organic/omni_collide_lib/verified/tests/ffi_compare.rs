use libloading::{Library, Symbol};
use std::ffi::c_int;

const C_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so");
const RS_LIB: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libomni_collide_lib.so");

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

#[repr(C)]
#[derive(Clone, Copy)]
struct C2GJKCache { metric: f32, count: i32, i_a: [i32; 3], i_b: [i32; 3], div: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Proxy { radius: f32, count: i32, verts: [C2v; 8] }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2sv { s_a: C2v, s_b: C2v, p: C2v, u: f32, i_a: i32, i_b: i32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct C2Simplex { a: C2sv, b: C2sv, c: C2sv, d: C2sv, div: f32, count: i32 }

const C2_TYPE_CAPSULE: c_int = 0;
const C2_TYPE_CIRCLE: c_int = 1;
const C2_TYPE_AABB: c_int = 2;

fn v(x: f32, y: f32) -> C2v { C2v { x, y } }

fn assert_v_eq(a: C2v, b: C2v, ctx: &str) {
    assert!(a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits(),
        "{ctx}: C=({},{}) Rust=({},{})", a.x, a.y, b.x, b.y);
}

fn assert_f_eq(a: f32, b: f32, ctx: &str) {
    assert!(a.to_bits() == b.to_bits(), "{ctx}: C={a} Rust={b}");
}

#[test]
fn test_c2v() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();
        let cf: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = c.get(b"c2V").unwrap();
        let rf: Symbol<unsafe extern "C" fn(f32, f32) -> C2v> = r.get(b"c2V").unwrap();
        for (x, y) in [(0.0, 0.0), (1.5, -2.3), (f32::MAX, f32::MIN), (f32::INFINITY, f32::NEG_INFINITY)] {
            assert_v_eq(cf(x, y), rf(x, y), &format!("c2V({x},{y})"));
        }
    }
}

#[test]
fn test_c2_vector_ops() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();
        type VV = unsafe extern "C" fn(C2v, C2v) -> C2v;
        type VF = unsafe extern "C" fn(C2v, C2v) -> f32;
        type VS = unsafe extern "C" fn(C2v, f32) -> C2v;
        type UV = unsafe extern "C" fn(C2v) -> C2v;
        type UF = unsafe extern "C" fn(C2v) -> f32;

        let pairs: &[(C2v, C2v)] = &[
            (v(1.0, 2.0), v(3.0, 4.0)),
            (v(-1.0, 0.0), v(0.0, -1.0)),
            (v(100.0, -50.0), v(0.5, 0.5)),
            (v(0.0, 0.0), v(0.0, 0.0)),
        ];

        // c2Add, c2Sub, c2Maxv, c2Minv
        for name in [b"c2Add" as &[u8], b"c2Sub", b"c2Maxv", b"c2Minv"] {
            let cf: Symbol<VV> = c.get(name).unwrap();
            let rf: Symbol<VV> = r.get(name).unwrap();
            for &(a, b_) in pairs {
                assert_v_eq(cf(a, b_), rf(a, b_), &format!("{}({:?},{:?})", std::str::from_utf8(name).unwrap(), a, b_));
            }
        }

        // c2Dot, c2Det2
        for name in [b"c2Dot" as &[u8], b"c2Det2"] {
            let cf: Symbol<VF> = c.get(name).unwrap();
            let rf: Symbol<VF> = r.get(name).unwrap();
            for &(a, b_) in pairs {
                assert_f_eq(cf(a, b_), rf(a, b_), &format!("{}({:?},{:?})", std::str::from_utf8(name).unwrap(), a, b_));
            }
        }

        // c2Mulvs
        let cm: Symbol<VS> = c.get(b"c2Mulvs").unwrap();
        let rm: Symbol<VS> = r.get(b"c2Mulvs").unwrap();
        for &(a, _) in pairs {
            for s in [0.0f32, 1.0, -1.0, 2.5, 0.001] {
                assert_v_eq(cm(a, s), rm(a, s), &format!("c2Mulvs({a:?},{s})"));
            }
        }

        // c2Neg, c2Skew, c2CCW90
        for name in [b"c2Neg" as &[u8], b"c2Skew", b"c2CCW90"] {
            let cf: Symbol<UV> = c.get(name).unwrap();
            let rf: Symbol<UV> = r.get(name).unwrap();
            for &(a, _) in pairs {
                assert_v_eq(cf(a), rf(a), &format!("{}({:?})", std::str::from_utf8(name).unwrap(), a));
            }
        }

        // c2Len
        let cl: Symbol<UF> = c.get(b"c2Len").unwrap();
        let rl: Symbol<UF> = r.get(b"c2Len").unwrap();
        for &(a, _) in pairs {
            assert_f_eq(cl(a), rl(a), &format!("c2Len({a:?})"));
        }

        // c2Norm (skip zero vector)
        let cn: Symbol<UV> = c.get(b"c2Norm").unwrap();
        let rn: Symbol<UV> = r.get(b"c2Norm").unwrap();
        for &(a, _) in pairs.iter().filter(|(a, _)| a.x != 0.0 || a.y != 0.0) {
            assert_v_eq(cn(a), rn(a), &format!("c2Norm({a:?})"));
        }

        // c2Div (skip zero divisor)
        let cd: Symbol<VS> = c.get(b"c2Div").unwrap();
        let rd: Symbol<VS> = r.get(b"c2Div").unwrap();
        for &(a, _) in pairs {
            for s in [1.0f32, -1.0, 2.5, 0.001] {
                assert_v_eq(cd(a, s), rd(a, s), &format!("c2Div({a:?},{s})"));
            }
        }

        // c2Clampv
        let cc: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> = c.get(b"c2Clampv").unwrap();
        let rc: Symbol<unsafe extern "C" fn(C2v, C2v, C2v) -> C2v> = r.get(b"c2Clampv").unwrap();
        let lo = v(-1.0, -1.0);
        let hi = v(1.0, 1.0);
        for &(a, _) in pairs {
            assert_v_eq(cc(a, lo, hi), rc(a, lo, hi), &format!("c2Clampv({a:?})"));
        }
    }
}

#[test]
fn test_c2_rot_and_transform() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();

        // c2RotIdentity
        let cri: Symbol<unsafe extern "C" fn() -> C2r> = c.get(b"c2RotIdentity").unwrap();
        let rri: Symbol<unsafe extern "C" fn() -> C2r> = r.get(b"c2RotIdentity").unwrap();
        let cr = cri(); let rr = rri();
        assert_f_eq(cr.c, rr.c, "c2RotIdentity.c");
        assert_f_eq(cr.s, rr.s, "c2RotIdentity.s");

        // c2xIdentity
        let cxi: Symbol<unsafe extern "C" fn() -> C2x> = c.get(b"c2xIdentity").unwrap();
        let rxi: Symbol<unsafe extern "C" fn() -> C2x> = r.get(b"c2xIdentity").unwrap();
        let cx = cxi(); let rx = rxi();
        assert_v_eq(cx.p, rx.p, "c2xIdentity.p");
        assert_f_eq(cx.r.c, rx.r.c, "c2xIdentity.r.c");
        assert_f_eq(cx.r.s, rx.r.s, "c2xIdentity.r.s");

        // c2Mulrv, c2MulrvT
        let rots = [C2r { c: 1.0, s: 0.0 }, C2r { c: 0.0, s: 1.0 }, C2r { c: 0.7071, s: 0.7071 }];
        let vecs = [v(1.0, 0.0), v(0.0, 1.0), v(3.0, -4.0)];
        for name in [b"c2Mulrv" as &[u8], b"c2MulrvT"] {
            let cf: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = c.get(name).unwrap();
            let rf: Symbol<unsafe extern "C" fn(C2r, C2v) -> C2v> = r.get(name).unwrap();
            for &rot in &rots {
                for &vec in &vecs {
                    assert_v_eq(cf(rot, vec), rf(rot, vec),
                        &format!("{}({:?},{:?})", std::str::from_utf8(name).unwrap(), rot, vec));
                }
            }
        }

        // c2Mulxv
        let cf: Symbol<unsafe extern "C" fn(C2x, C2v) -> C2v> = c.get(b"c2Mulxv").unwrap();
        let rf: Symbol<unsafe extern "C" fn(C2x, C2v) -> C2v> = r.get(b"c2Mulxv").unwrap();
        let xforms = [
            C2x { p: v(0.0, 0.0), r: C2r { c: 1.0, s: 0.0 } },
            C2x { p: v(10.0, -5.0), r: C2r { c: 0.0, s: 1.0 } },
        ];
        for &x in &xforms {
            for &vec in &vecs {
                assert_v_eq(cf(x, vec), rf(x, vec), &format!("c2Mulxv({x:?},{vec:?})"));
            }
        }
    }
}

#[test]
fn test_c2_bb_verts() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();
        let cf: Symbol<unsafe extern "C" fn(*mut C2v, *mut C2AABB)> = c.get(b"c2BBVerts").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*mut C2v, *mut C2AABB)> = r.get(b"c2BBVerts").unwrap();
        let mut bb = C2AABB { min: v(-1.0, -2.0), max: v(3.0, 4.0) };
        let mut c_out = [v(0.0, 0.0); 4];
        let mut r_out = [v(0.0, 0.0); 4];
        cf(c_out.as_mut_ptr(), &mut bb);
        rf(r_out.as_mut_ptr(), &mut bb);
        for i in 0..4 {
            assert_v_eq(c_out[i], r_out[i], &format!("c2BBVerts[{i}]"));
        }
    }
}

#[test]
fn test_c2_make_proxy() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();
        let cf: Symbol<unsafe extern "C" fn(*const u8, c_int, *mut C2Proxy)> = c.get(b"c2MakeProxy").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const u8, c_int, *mut C2Proxy)> = r.get(b"c2MakeProxy").unwrap();

        // Circle
        let circle = C2Circle { p: v(1.0, 2.0), r: 3.0 };
        let mut cp = std::mem::zeroed::<C2Proxy>();
        let mut rp = std::mem::zeroed::<C2Proxy>();
        cf(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, &mut cp);
        rf(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, &mut rp);
        assert_f_eq(cp.radius, rp.radius, "proxy circle radius");
        assert_eq!(cp.count, rp.count, "proxy circle count");
        assert_v_eq(cp.verts[0], rp.verts[0], "proxy circle vert0");

        // AABB
        let aabb = C2AABB { min: v(-1.0, -2.0), max: v(3.0, 4.0) };
        cf(&aabb as *const _ as *const u8, C2_TYPE_AABB, &mut cp);
        rf(&aabb as *const _ as *const u8, C2_TYPE_AABB, &mut rp);
        assert_f_eq(cp.radius, rp.radius, "proxy aabb radius");
        assert_eq!(cp.count, rp.count, "proxy aabb count");
        for i in 0..4 {
            assert_v_eq(cp.verts[i], rp.verts[i], &format!("proxy aabb vert{i}"));
        }

        // Capsule
        let cap = C2Capsule { a: v(0.0, 0.0), b: v(5.0, 5.0), r: 1.0 };
        cf(&cap as *const _ as *const u8, C2_TYPE_CAPSULE, &mut cp);
        rf(&cap as *const _ as *const u8, C2_TYPE_CAPSULE, &mut rp);
        assert_f_eq(cp.radius, rp.radius, "proxy capsule radius");
        assert_eq!(cp.count, rp.count, "proxy capsule count");
        for i in 0..2 {
            assert_v_eq(cp.verts[i], rp.verts[i], &format!("proxy capsule vert{i}"));
        }
    }
}

fn zero_sv() -> C2sv {
    C2sv { s_a: v(0.0,0.0), s_b: v(0.0,0.0), p: v(0.0,0.0), u: 0.0, i_a: 0, i_b: 0 }
}

fn zero_simplex() -> C2Simplex {
    C2Simplex { a: zero_sv(), b: zero_sv(), c: zero_sv(), d: zero_sv(), div: 0.0, count: 0 }
}

fn assert_simplex_eq(cs: &C2Simplex, rs: &C2Simplex, ctx: &str) {
    assert_eq!(cs.count, rs.count, "{ctx} count");
    assert_f_eq(cs.div, rs.div, &format!("{ctx} div"));
    assert_v_eq(cs.a.p, rs.a.p, &format!("{ctx} a.p"));
    assert_f_eq(cs.a.u, rs.a.u, &format!("{ctx} a.u"));
    if cs.count >= 2 {
        assert_v_eq(cs.b.p, rs.b.p, &format!("{ctx} b.p"));
        assert_f_eq(cs.b.u, rs.b.u, &format!("{ctx} b.u"));
    }
    if cs.count >= 3 {
        assert_v_eq(cs.c.p, rs.c.p, &format!("{ctx} c.p"));
        assert_f_eq(cs.c.u, rs.c.u, &format!("{ctx} c.u"));
    }
}

#[test]
fn test_c22_c23() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();
        let c22c: Symbol<unsafe extern "C" fn(*mut C2Simplex)> = c.get(b"c22").unwrap();
        let c22r: Symbol<unsafe extern "C" fn(*mut C2Simplex)> = r.get(b"c22").unwrap();
        let c23c: Symbol<unsafe extern "C" fn(*mut C2Simplex)> = c.get(b"c23").unwrap();
        let c23r: Symbol<unsafe extern "C" fn(*mut C2Simplex)> = r.get(b"c23").unwrap();

        // Test c22 with various 2-simplex configs
        let test_pairs = [
            (v(1.0, 0.0), v(-1.0, 0.0)),
            (v(0.0, 0.0), v(1.0, 1.0)),
            (v(-2.0, 1.0), v(3.0, -1.0)),
            (v(0.5, 0.5), v(0.5, -0.5)),
        ];
        for &(pa, pb) in &test_pairs {
            let mut cs = zero_simplex();
            cs.a.p = pa; cs.b.p = pb; cs.count = 2; cs.div = 1.0;
            let mut rs = cs;
            c22c(&mut cs);
            c22r(&mut rs);
            assert_simplex_eq(&cs, &rs, &format!("c22({pa:?},{pb:?})"));
        }

        // Test c23 with various 3-simplex configs
        let test_triples = [
            (v(1.0, 0.0), v(0.0, 1.0), v(-1.0, -1.0)),
            (v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0)),
            (v(-1.0, -1.0), v(2.0, 0.0), v(0.0, 2.0)),
        ];
        for &(pa, pb, pc) in &test_triples {
            let mut cs = zero_simplex();
            cs.a.p = pa; cs.b.p = pb; cs.c.p = pc; cs.count = 3; cs.div = 1.0;
            let mut rs = cs;
            c23c(&mut cs);
            c23r(&mut rs);
            assert_simplex_eq(&cs, &rs, &format!("c23({pa:?},{pb:?},{pc:?})"));
        }
    }
}

#[test]
fn test_c2d_c2l_c2gjk_simplex_metric() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();
        let c_d: Symbol<unsafe extern "C" fn(*mut C2Simplex) -> C2v> = c.get(b"c2D").unwrap();
        let r_d: Symbol<unsafe extern "C" fn(*mut C2Simplex) -> C2v> = r.get(b"c2D").unwrap();
        let c_l: Symbol<unsafe extern "C" fn(*mut C2Simplex) -> C2v> = c.get(b"c2L").unwrap();
        let r_l: Symbol<unsafe extern "C" fn(*mut C2Simplex) -> C2v> = r.get(b"c2L").unwrap();
        let c_m: Symbol<unsafe extern "C" fn(*mut C2Simplex) -> f32> = c.get(b"c2GJKSimplexMetric").unwrap();
        let r_m: Symbol<unsafe extern "C" fn(*mut C2Simplex) -> f32> = r.get(b"c2GJKSimplexMetric").unwrap();

        // count=1
        let mut s = zero_simplex();
        s.a.p = v(3.0, 4.0); s.count = 1; s.div = 1.0; s.a.u = 1.0;
        let mut s2 = s;
        assert_v_eq(c_d(&mut s), r_d(&mut s2), "c2D count=1");
        assert_v_eq(c_l(&mut s), r_l(&mut s2), "c2L count=1");
        assert_f_eq(c_m(&mut s), r_m(&mut s2), "metric count=1");

        // count=2
        s.a.p = v(1.0, 0.0); s.b.p = v(-1.0, 1.0); s.count = 2; s.div = 2.0; s.a.u = 1.0; s.b.u = 1.0;
        s2 = s;
        assert_v_eq(c_d(&mut s), r_d(&mut s2), "c2D count=2");
        assert_v_eq(c_l(&mut s), r_l(&mut s2), "c2L count=2");
        assert_f_eq(c_m(&mut s), r_m(&mut s2), "metric count=2");

        // count=3
        s.a.p = v(1.0, 0.0); s.b.p = v(0.0, 1.0); s.c.p = v(-1.0, -1.0);
        s.count = 3; s.div = 3.0; s.a.u = 1.0; s.b.u = 1.0; s.c.u = 1.0;
        s2 = s;
        assert_v_eq(c_d(&mut s), r_d(&mut s2), "c2D count=3");
        assert_f_eq(c_m(&mut s), r_m(&mut s2), "metric count=3");
    }
}

#[test]
fn test_c2_support() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();
        let cf: Symbol<unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int> = c.get(b"c2Support").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int> = r.get(b"c2Support").unwrap();
        let verts = [v(0.0, 0.0), v(1.0, 0.0), v(0.0, 1.0), v(-1.0, -1.0)];
        let dirs = [v(1.0, 0.0), v(0.0, 1.0), v(-1.0, -1.0), v(1.0, 1.0), v(-1.0, 0.0)];
        for &d in &dirs {
            let cv = cf(verts.as_ptr(), 4, d);
            let rv = rf(verts.as_ptr(), 4, d);
            assert_eq!(cv, rv, "c2Support dir={d:?}");
        }
    }
}

#[test]
fn test_c2_witness() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();
        let cf: Symbol<unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v)> = c.get(b"c2Witness").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v)> = r.get(b"c2Witness").unwrap();

        // count=1
        let mut s = zero_simplex();
        s.a.s_a = v(1.0, 2.0); s.a.s_b = v(3.0, 4.0); s.a.u = 1.0; s.div = 1.0; s.count = 1;
        let (mut ca, mut cb, mut ra, mut rb) = (v(0.0,0.0), v(0.0,0.0), v(0.0,0.0), v(0.0,0.0));
        let mut s2 = s;
        cf(&mut s, &mut ca, &mut cb);
        rf(&mut s2, &mut ra, &mut rb);
        assert_v_eq(ca, ra, "witness1 a");
        assert_v_eq(cb, rb, "witness1 b");

        // count=2
        s.b.s_a = v(5.0, 6.0); s.b.s_b = v(7.0, 8.0); s.b.u = 1.0; s.div = 2.0; s.count = 2;
        s2 = s;
        cf(&mut s, &mut ca, &mut cb);
        rf(&mut s2, &mut ra, &mut rb);
        assert_v_eq(ca, ra, "witness2 a");
        assert_v_eq(cb, rb, "witness2 b");
    }
}

#[test]
fn test_collision_functions() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();

        // c2CircletoCircle
        {
            let cf: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> c_int> = c.get(b"c2CircletoCircle").unwrap();
            let rf: Symbol<unsafe extern "C" fn(C2Circle, C2Circle) -> c_int> = r.get(b"c2CircletoCircle").unwrap();
            let cases = [
                (C2Circle { p: v(0.0, 0.0), r: 1.0 }, C2Circle { p: v(0.5, 0.0), r: 1.0 }, "overlap"),
                (C2Circle { p: v(0.0, 0.0), r: 1.0 }, C2Circle { p: v(3.0, 0.0), r: 1.0 }, "apart"),
                (C2Circle { p: v(0.0, 0.0), r: 1.0 }, C2Circle { p: v(2.0, 0.0), r: 1.0 }, "touching"),
            ];
            for (a, b_, name) in &cases {
                assert_eq!(cf(*a, *b_), rf(*a, *b_), "c2CircletoCircle {name}");
            }
        }

        // c2CircletoAABB
        {
            let cf: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> c_int> = c.get(b"c2CircletoAABB").unwrap();
            let rf: Symbol<unsafe extern "C" fn(C2Circle, C2AABB) -> c_int> = r.get(b"c2CircletoAABB").unwrap();
            let circle = C2Circle { p: v(0.0, 0.0), r: 1.0 };
            let boxes = [
                (C2AABB { min: v(-0.5, -0.5), max: v(0.5, 0.5) }, "inside"),
                (C2AABB { min: v(5.0, 5.0), max: v(6.0, 6.0) }, "far"),
                (C2AABB { min: v(0.5, -0.5), max: v(1.5, 0.5) }, "edge"),
            ];
            for (bb, name) in &boxes {
                assert_eq!(cf(circle, *bb), rf(circle, *bb), "c2CircletoAABB {name}");
            }
        }

        // c2CircletoCapsule
        {
            let cf: Symbol<unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int> = c.get(b"c2CircletoCapsule").unwrap();
            let rf: Symbol<unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int> = r.get(b"c2CircletoCapsule").unwrap();
            let circle = C2Circle { p: v(0.0, 0.0), r: 1.0 };
            let caps = [
                (C2Capsule { a: v(-1.0, 0.0), b: v(1.0, 0.0), r: 0.5 }, "overlap"),
                (C2Capsule { a: v(5.0, 5.0), b: v(6.0, 6.0), r: 0.1 }, "far"),
            ];
            for (cap, name) in &caps {
                assert_eq!(cf(circle, *cap), rf(circle, *cap), "c2CircletoCapsule {name}");
            }
        }

        // c2AABBtoAABB
        {
            let cf: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> = c.get(b"c2AABBtoAABB").unwrap();
            let rf: Symbol<unsafe extern "C" fn(C2AABB, C2AABB) -> c_int> = r.get(b"c2AABBtoAABB").unwrap();
            let cases = [
                (C2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) }, C2AABB { min: v(1.0, 1.0), max: v(3.0, 3.0) }, "overlap"),
                (C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }, C2AABB { min: v(5.0, 5.0), max: v(6.0, 6.0) }, "apart"),
            ];
            for (a, b_, name) in &cases {
                assert_eq!(cf(*a, *b_), rf(*a, *b_), "c2AABBtoAABB {name}");
            }
        }

        // c2AABBtoCapsule
        {
            let cf: Symbol<unsafe extern "C" fn(C2AABB, C2Capsule) -> c_int> = c.get(b"c2AABBtoCapsule").unwrap();
            let rf: Symbol<unsafe extern "C" fn(C2AABB, C2Capsule) -> c_int> = r.get(b"c2AABBtoCapsule").unwrap();
            let bb = C2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) };
            let caps = [
                (C2Capsule { a: v(1.0, 1.0), b: v(3.0, 3.0), r: 0.5 }, "overlap"),
                (C2Capsule { a: v(10.0, 10.0), b: v(11.0, 11.0), r: 0.1 }, "far"),
            ];
            for (cap, name) in &caps {
                assert_eq!(cf(bb, *cap), rf(bb, *cap), "c2AABBtoCapsule {name}");
            }
        }

        // c2CapsuletoCapsule
        {
            let cf: Symbol<unsafe extern "C" fn(C2Capsule, C2Capsule) -> c_int> = c.get(b"c2CapsuletoCapsule").unwrap();
            let rf: Symbol<unsafe extern "C" fn(C2Capsule, C2Capsule) -> c_int> = r.get(b"c2CapsuletoCapsule").unwrap();
            let cases = [
                (C2Capsule { a: v(0.0, 0.0), b: v(2.0, 0.0), r: 1.0 }, C2Capsule { a: v(1.0, 0.0), b: v(3.0, 0.0), r: 1.0 }, "overlap"),
                (C2Capsule { a: v(0.0, 0.0), b: v(1.0, 0.0), r: 0.1 }, C2Capsule { a: v(5.0, 5.0), b: v(6.0, 5.0), r: 0.1 }, "far"),
            ];
            for (a, b_, name) in &cases {
                assert_eq!(cf(*a, *b_), rf(*a, *b_), "c2CapsuletoCapsule {name}");
            }
        }
    }
}

#[test]
fn test_c2_collided() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();
        let cf: Symbol<unsafe extern "C" fn(*const u8, c_int, *const u8, c_int) -> c_int> = c.get(b"c2Collided").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const u8, c_int, *const u8, c_int) -> c_int> = r.get(b"c2Collided").unwrap();

        // Circle vs Circle
        let a = C2Circle { p: v(0.0, 0.0), r: 1.0 };
        let b = C2Circle { p: v(0.5, 0.0), r: 1.0 };
        assert_eq!(
            cf(&a as *const _ as *const u8, C2_TYPE_CIRCLE, &b as *const _ as *const u8, C2_TYPE_CIRCLE),
            rf(&a as *const _ as *const u8, C2_TYPE_CIRCLE, &b as *const _ as *const u8, C2_TYPE_CIRCLE),
            "c2Collided circle-circle"
        );

        // AABB vs Capsule
        let aabb = C2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) };
        let cap = C2Capsule { a: v(1.0, 1.0), b: v(3.0, 3.0), r: 0.5 };
        assert_eq!(
            cf(&aabb as *const _ as *const u8, C2_TYPE_AABB, &cap as *const _ as *const u8, C2_TYPE_CAPSULE),
            rf(&aabb as *const _ as *const u8, C2_TYPE_AABB, &cap as *const _ as *const u8, C2_TYPE_CAPSULE),
            "c2Collided aabb-capsule"
        );
    }
}

#[test]
fn test_c2_gjk() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();
        let cf: Symbol<unsafe extern "C" fn(*const u8, c_int, *const C2x, *const u8, c_int, *const C2x, *mut C2v, *mut C2v, c_int, *mut c_int, *mut C2GJKCache) -> f32> = c.get(b"c2GJK").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const u8, c_int, *const C2x, *const u8, c_int, *const C2x, *mut C2v, *mut C2v, c_int, *mut c_int, *mut C2GJKCache) -> f32> = r.get(b"c2GJK").unwrap();

        // Two circles, no radius
        let a = C2Circle { p: v(0.0, 0.0), r: 1.0 };
        let b = C2Circle { p: v(3.0, 0.0), r: 1.0 };
        let (mut ca, mut cb, mut ra, mut rb) = (v(0.0,0.0), v(0.0,0.0), v(0.0,0.0), v(0.0,0.0));
        let (mut ci, mut ri) = (0i32, 0i32);
        let cd = cf(&a as *const _ as *const u8, C2_TYPE_CIRCLE, std::ptr::null(), &b as *const _ as *const u8, C2_TYPE_CIRCLE, std::ptr::null(), &mut ca, &mut cb, 0, &mut ci, std::ptr::null_mut());
        let rd = rf(&a as *const _ as *const u8, C2_TYPE_CIRCLE, std::ptr::null(), &b as *const _ as *const u8, C2_TYPE_CIRCLE, std::ptr::null(), &mut ra, &mut rb, 0, &mut ri, std::ptr::null_mut());
        assert_f_eq(cd, rd, "c2GJK circle-circle dist");
        assert_v_eq(ca, ra, "c2GJK circle-circle outA");
        assert_v_eq(cb, rb, "c2GJK circle-circle outB");

        // With radius
        let cd = cf(&a as *const _ as *const u8, C2_TYPE_CIRCLE, std::ptr::null(), &b as *const _ as *const u8, C2_TYPE_CIRCLE, std::ptr::null(), &mut ca, &mut cb, 1, &mut ci, std::ptr::null_mut());
        let rd = rf(&a as *const _ as *const u8, C2_TYPE_CIRCLE, std::ptr::null(), &b as *const _ as *const u8, C2_TYPE_CIRCLE, std::ptr::null(), &mut ra, &mut rb, 1, &mut ri, std::ptr::null_mut());
        assert_f_eq(cd, rd, "c2GJK circle-circle radius dist");
        assert_v_eq(ca, ra, "c2GJK circle-circle radius outA");
        assert_v_eq(cb, rb, "c2GJK circle-circle radius outB");

        // AABB vs Capsule
        let aabb = C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) };
        let cap = C2Capsule { a: v(3.0, 0.0), b: v(4.0, 1.0), r: 0.5 };
        let cd = cf(&aabb as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(), &cap as *const _ as *const u8, C2_TYPE_CAPSULE, std::ptr::null(), &mut ca, &mut cb, 1, &mut ci, std::ptr::null_mut());
        let rd = rf(&aabb as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(), &cap as *const _ as *const u8, C2_TYPE_CAPSULE, std::ptr::null(), &mut ra, &mut rb, 1, &mut ri, std::ptr::null_mut());
        assert_f_eq(cd, rd, "c2GJK aabb-capsule dist");
        assert_v_eq(ca, ra, "c2GJK aabb-capsule outA");
        assert_v_eq(cb, rb, "c2GJK aabb-capsule outB");
    }
}

#[test]
fn test_ptr_from_parts() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();
        let cf: Symbol<unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut u8> = c.get(b"ptr_from_parts").unwrap();
        let rf: Symbol<unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32) -> *mut u8> = r.get(b"ptr_from_parts").unwrap();

        // Circle
        let cp = cf(C2_TYPE_CIRCLE, 1.0, 2.0, 3.0, 0.0, 0.0);
        let rp = rf(C2_TYPE_CIRCLE, 1.0, 2.0, 3.0, 0.0, 0.0);
        let cc = &*(cp as *const C2Circle);
        let rc = &*(rp as *const C2Circle);
        assert_v_eq(cc.p, rc.p, "ptr_from_parts circle p");
        assert_f_eq(cc.r, rc.r, "ptr_from_parts circle r");
        libc_free(cp); libc_free(rp);

        // AABB
        let cp = cf(C2_TYPE_AABB, 1.0, 2.0, 3.0, 4.0, 0.0);
        let rp = rf(C2_TYPE_AABB, 1.0, 2.0, 3.0, 4.0, 0.0);
        let ca = &*(cp as *const C2AABB);
        let ra = &*(rp as *const C2AABB);
        assert_v_eq(ca.min, ra.min, "ptr_from_parts aabb min");
        assert_v_eq(ca.max, ra.max, "ptr_from_parts aabb max");
        libc_free(cp); libc_free(rp);

        // Capsule
        let cp = cf(C2_TYPE_CAPSULE, 1.0, 2.0, 3.0, 4.0, 5.0);
        let rp = rf(C2_TYPE_CAPSULE, 1.0, 2.0, 3.0, 4.0, 5.0);
        let cc = &*(cp as *const C2Capsule);
        let rc = &*(rp as *const C2Capsule);
        assert_v_eq(cc.a, rc.a, "ptr_from_parts capsule a");
        assert_v_eq(cc.b, rc.b, "ptr_from_parts capsule b");
        assert_f_eq(cc.r, rc.r, "ptr_from_parts capsule r");
        libc_free(cp); libc_free(rp);
    }
}

// C uses malloc, Rust uses Box. We can't free C's malloc with Rust's dealloc.
// For the test, just leak the small allocations (they're tiny).
unsafe fn libc_free(_ptr: *mut u8) {
    // Intentionally leak - mixing C malloc and Rust dealloc is UB
}

#[test]
fn test_omni_collide() {
    unsafe {
        let c = Library::new(C_LIB).unwrap();
        let r = Library::new(RS_LIB).unwrap();
        type OC = unsafe extern "C" fn(c_int, f32, f32, f32, f32, f32, c_int, f32, f32, f32, f32, f32) -> c_int;
        let cf: Symbol<OC> = c.get(b"omni_collide").unwrap();
        let rf: Symbol<OC> = r.get(b"omni_collide").unwrap();

        // All 9 type combinations
        let shapes: [(c_int, f32, f32, f32, f32, f32); 3] = [
            (C2_TYPE_CIRCLE, 0.0, 0.0, 1.0, 0.0, 0.0),   // circle at origin r=1
            (C2_TYPE_AABB, -1.0, -1.0, 1.0, 1.0, 0.0),    // aabb [-1,-1] to [1,1]
            (C2_TYPE_CAPSULE, -1.0, 0.0, 1.0, 0.0, 0.5),  // capsule from (-1,0) to (1,0) r=0.5
        ];
        let shapes_far: [(c_int, f32, f32, f32, f32, f32); 3] = [
            (C2_TYPE_CIRCLE, 10.0, 10.0, 0.1, 0.0, 0.0),
            (C2_TYPE_AABB, 10.0, 10.0, 11.0, 11.0, 0.0),
            (C2_TYPE_CAPSULE, 10.0, 10.0, 11.0, 11.0, 0.1),
        ];

        for &(ta, a1, a2, a3, a4, a5) in &shapes {
            for &(tb, b1, b2, b3, b4, b5) in &shapes {
                let cv = cf(ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5);
                let rv = rf(ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5);
                assert_eq!(cv, rv, "omni_collide near ({ta},{tb})");
            }
            for &(tb, b1, b2, b3, b4, b5) in &shapes_far {
                let cv = cf(ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5);
                let rv = rf(ta, a1, a2, a3, a4, a5, tb, b1, b2, b3, b4, b5);
                assert_eq!(cv, rv, "omni_collide far ({ta},{tb})");
            }
        }
    }
}
