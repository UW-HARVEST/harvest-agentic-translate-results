use libloading::{Library, Symbol};
use std::os::raw::c_int;

// Mirror the C struct layouts exactly
#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct C2v { x: f32, y: f32 }

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct C2r { c: f32, s: f32 }

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct C2x { p: C2v, r: C2r }

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct C2Circle { p: C2v, r: f32 }

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct C2AABB { min: C2v, max: C2v }

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct C2Capsule { a: C2v, b: C2v, r: f32 }

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct C2Proxy { radius: f32, count: c_int, verts: [C2v; 8] }

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct C2sv { s_a: C2v, s_b: C2v, p: C2v, u: f32, i_a: c_int, i_b: c_int }

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct C2Simplex { a: C2sv, b: C2sv, c: C2sv, d: C2sv, div: f32, count: c_int }

#[derive(Clone, Copy, Debug)]
#[repr(C)]
struct C2GJKCache { metric: f32, count: c_int, i_a: [c_int; 3], i_b: [c_int; 3], div: f32 }

const C2_TYPE_CIRCLE: c_int = 0;
const C2_TYPE_AABB: c_int = 1;
const C2_TYPE_CAPSULE: c_int = 2;

fn v(x: f32, y: f32) -> C2v { C2v { x, y } }

fn assert_v_eq(label: &str, a: C2v, b: C2v) {
    assert!(a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits(),
        "{label}: C=({}, {}) Rust=({}, {})", a.x, a.y, b.x, b.y);
}

fn assert_f_eq(label: &str, a: f32, b: f32) {
    assert!(a.to_bits() == b.to_bits(), "{label}: C={a} Rust={b}");
}

struct Libs { c: Library, rs: Library }

impl Libs {
    fn load() -> Self {
        let c_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("c_src/build/libtranslated_rust.so");
        let rs_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("target/debug/libaabb_lib.so");
        unsafe {
            Libs {
                c: Library::new(&c_path).expect("load C .so"),
                rs: Library::new(&rs_path).expect("load Rust .so"),
            }
        }
    }
}

// ===== Level 0: Basic vector math =====

#[test]
fn test_c2v() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(f32, f32) -> C2v;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2V").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2V").unwrap() };
    for (x, y) in [(0.0, 0.0), (1.5, -2.3), (f32::MAX, f32::MIN), (f32::NAN, 0.0)] {
        let c = unsafe { c_fn(x, y) };
        let r = unsafe { r_fn(x, y) };
        assert_v_eq("c2V", c, r);
    }
}

#[test]
fn test_c2_basic_ops() {
    let libs = Libs::load();
    let pairs = [
        (v(1.0, 2.0), v(3.0, 4.0)),
        (v(-1.0, 0.0), v(0.0, -1.0)),
        (v(100.5, -200.3), v(-50.1, 75.9)),
        (v(0.0, 0.0), v(0.0, 0.0)),
    ];

    // c2Add
    {
        type Fn = unsafe extern "C" fn(C2v, C2v) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Add").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Add").unwrap() };
        for &(a, b) in &pairs {
            assert_v_eq("c2Add", unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) });
        }
    }
    // c2Sub
    {
        type Fn = unsafe extern "C" fn(C2v, C2v) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Sub").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Sub").unwrap() };
        for &(a, b) in &pairs {
            assert_v_eq("c2Sub", unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) });
        }
    }
    // c2Dot
    {
        type Fn = unsafe extern "C" fn(C2v, C2v) -> f32;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Dot").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Dot").unwrap() };
        for &(a, b) in &pairs {
            assert_f_eq("c2Dot", unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) });
        }
    }
    // c2Neg
    {
        type Fn = unsafe extern "C" fn(C2v) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Neg").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Neg").unwrap() };
        for &(a, _) in &pairs {
            assert_v_eq("c2Neg", unsafe { c_fn(a) }, unsafe { r_fn(a) });
        }
    }
    // c2Maxv
    {
        type Fn = unsafe extern "C" fn(C2v, C2v) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Maxv").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Maxv").unwrap() };
        for &(a, b) in &pairs {
            assert_v_eq("c2Maxv", unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) });
        }
    }
    // c2Minv
    {
        type Fn = unsafe extern "C" fn(C2v, C2v) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Minv").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Minv").unwrap() };
        for &(a, b) in &pairs {
            assert_v_eq("c2Minv", unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) });
        }
    }
    // c2Mulvs
    {
        type Fn = unsafe extern "C" fn(C2v, f32) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Mulvs").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Mulvs").unwrap() };
        for &(a, _) in &pairs {
            for s in [0.0f32, 1.0, -2.5, 0.001] {
                assert_v_eq("c2Mulvs", unsafe { c_fn(a, s) }, unsafe { r_fn(a, s) });
            }
        }
    }
    // c2Skew
    {
        type Fn = unsafe extern "C" fn(C2v) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Skew").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Skew").unwrap() };
        for &(a, _) in &pairs {
            assert_v_eq("c2Skew", unsafe { c_fn(a) }, unsafe { r_fn(a) });
        }
    }
    // c2CCW90
    {
        type Fn = unsafe extern "C" fn(C2v) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2CCW90").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2CCW90").unwrap() };
        for &(a, _) in &pairs {
            assert_v_eq("c2CCW90", unsafe { c_fn(a) }, unsafe { r_fn(a) });
        }
    }
}

#[test]
fn test_c2_len_det2_norm_div() {
    let libs = Libs::load();
    let vecs = [v(3.0, 4.0), v(1.0, 0.0), v(-5.0, 12.0), v(0.1, 0.2)];

    // c2Len
    {
        type Fn = unsafe extern "C" fn(C2v) -> f32;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Len").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Len").unwrap() };
        for &a in &vecs {
            assert_f_eq("c2Len", unsafe { c_fn(a) }, unsafe { r_fn(a) });
        }
    }
    // c2Det2
    {
        type Fn = unsafe extern "C" fn(C2v, C2v) -> f32;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Det2").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Det2").unwrap() };
        for i in 0..vecs.len() {
            for j in 0..vecs.len() {
                assert_f_eq("c2Det2", unsafe { c_fn(vecs[i], vecs[j]) }, unsafe { r_fn(vecs[i], vecs[j]) });
            }
        }
    }
    // c2Norm
    {
        type Fn = unsafe extern "C" fn(C2v) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Norm").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Norm").unwrap() };
        for &a in &vecs {
            assert_v_eq("c2Norm", unsafe { c_fn(a) }, unsafe { r_fn(a) });
        }
    }
    // c2Div
    {
        type Fn = unsafe extern "C" fn(C2v, f32) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Div").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Div").unwrap() };
        for &a in &vecs {
            for d in [1.0f32, 2.0, -3.0, 0.5] {
                assert_v_eq("c2Div", unsafe { c_fn(a, d) }, unsafe { r_fn(a, d) });
            }
        }
    }
}

#[test]
fn test_c2_clampv() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(C2v, C2v, C2v) -> C2v;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Clampv").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Clampv").unwrap() };
    let cases = [
        (v(5.0, 5.0), v(0.0, 0.0), v(10.0, 10.0)),
        (v(-5.0, 15.0), v(0.0, 0.0), v(10.0, 10.0)),
        (v(0.0, 0.0), v(-1.0, -1.0), v(1.0, 1.0)),
    ];
    for (a, lo, hi) in cases {
        assert_v_eq("c2Clampv", unsafe { c_fn(a, lo, hi) }, unsafe { r_fn(a, lo, hi) });
    }
}

// ===== Level 1: Rotation/transform math =====

#[test]
fn test_c2_rot_transform() {
    let libs = Libs::load();
    // c2RotIdentity
    {
        type Fn = unsafe extern "C" fn() -> C2r;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2RotIdentity").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2RotIdentity").unwrap() };
        let c = unsafe { c_fn() };
        let r = unsafe { r_fn() };
        assert_f_eq("c2RotIdentity.c", c.c, r.c);
        assert_f_eq("c2RotIdentity.s", c.s, r.s);
    }
    // c2xIdentity
    {
        type Fn = unsafe extern "C" fn() -> C2x;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2xIdentity").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2xIdentity").unwrap() };
        let c = unsafe { c_fn() };
        let r = unsafe { r_fn() };
        assert_v_eq("c2xIdentity.p", c.p, r.p);
        assert_f_eq("c2xIdentity.r.c", c.r.c, r.r.c);
        assert_f_eq("c2xIdentity.r.s", c.r.s, r.r.s);
    }
    // c2Mulrv
    {
        type Fn = unsafe extern "C" fn(C2r, C2v) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Mulrv").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Mulrv").unwrap() };
        let rots = [C2r { c: 1.0, s: 0.0 }, C2r { c: 0.0, s: 1.0 }, C2r { c: 0.707, s: 0.707 }];
        let vecs = [v(1.0, 0.0), v(0.0, 1.0), v(3.0, -4.0)];
        for &rot in &rots {
            for &vec in &vecs {
                assert_v_eq("c2Mulrv", unsafe { c_fn(rot, vec) }, unsafe { r_fn(rot, vec) });
            }
        }
    }
    // c2MulrvT
    {
        type Fn = unsafe extern "C" fn(C2r, C2v) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2MulrvT").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2MulrvT").unwrap() };
        let rot = C2r { c: 0.866, s: 0.5 };
        for &vec in &[v(1.0, 0.0), v(0.0, 1.0), v(-3.0, 7.0)] {
            assert_v_eq("c2MulrvT", unsafe { c_fn(rot, vec) }, unsafe { r_fn(rot, vec) });
        }
    }
    // c2Mulxv
    {
        type Fn = unsafe extern "C" fn(C2x, C2v) -> C2v;
        let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Mulxv").unwrap() };
        let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Mulxv").unwrap() };
        let xf = C2x { p: v(10.0, 20.0), r: C2r { c: 0.0, s: 1.0 } };
        for &vec in &[v(1.0, 0.0), v(0.0, 0.0), v(-5.0, 3.0)] {
            assert_v_eq("c2Mulxv", unsafe { c_fn(xf, vec) }, unsafe { r_fn(xf, vec) });
        }
    }
}

// ===== Level 2: Geometry helpers =====

#[test]
fn test_c2bb_verts() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*mut C2v, *mut C2AABB);
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2BBVerts").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2BBVerts").unwrap() };
    let bbs = [
        C2AABB { min: v(-1.0, -2.0), max: v(3.0, 4.0) },
        C2AABB { min: v(0.0, 0.0), max: v(10.0, 10.0) },
    ];
    for mut bb in bbs {
        let mut c_out = [v(0.0, 0.0); 4];
        let mut r_out = [v(0.0, 0.0); 4];
        unsafe { c_fn(c_out.as_mut_ptr(), &mut bb); }
        unsafe { r_fn(r_out.as_mut_ptr(), &mut bb); }
        for i in 0..4 {
            assert_v_eq(&format!("c2BBVerts[{i}]"), c_out[i], r_out[i]);
        }
    }
}

#[test]
fn test_c2make_proxy() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const u8, c_int, *mut C2Proxy);
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2MakeProxy").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2MakeProxy").unwrap() };

    // Circle
    {
        let circle = C2Circle { p: v(1.0, 2.0), r: 5.0 };
        let mut cp = std::mem::MaybeUninit::<C2Proxy>::zeroed();
        let mut rp = std::mem::MaybeUninit::<C2Proxy>::zeroed();
        unsafe {
            c_fn(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, cp.as_mut_ptr());
            r_fn(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, rp.as_mut_ptr());
            let cp = cp.assume_init();
            let rp = rp.assume_init();
            assert_f_eq("proxy.radius", cp.radius, rp.radius);
            assert_eq!(cp.count, rp.count, "proxy.count");
            assert_v_eq("proxy.verts[0]", cp.verts[0], rp.verts[0]);
        }
    }
    // AABB
    {
        let aabb = C2AABB { min: v(-10.0, -20.0), max: v(10.0, 20.0) };
        let mut cp = std::mem::MaybeUninit::<C2Proxy>::zeroed();
        let mut rp = std::mem::MaybeUninit::<C2Proxy>::zeroed();
        unsafe {
            c_fn(&aabb as *const _ as *const u8, C2_TYPE_AABB, cp.as_mut_ptr());
            r_fn(&aabb as *const _ as *const u8, C2_TYPE_AABB, rp.as_mut_ptr());
            let cp = cp.assume_init();
            let rp = rp.assume_init();
            assert_f_eq("proxy.radius", cp.radius, rp.radius);
            assert_eq!(cp.count, rp.count, "proxy.count");
            for i in 0..4 {
                assert_v_eq(&format!("proxy.verts[{i}]"), cp.verts[i], rp.verts[i]);
            }
        }
    }
    // Capsule
    {
        let cap = C2Capsule { a: v(0.0, 0.0), b: v(10.0, 0.0), r: 3.0 };
        let mut cp = std::mem::MaybeUninit::<C2Proxy>::zeroed();
        let mut rp = std::mem::MaybeUninit::<C2Proxy>::zeroed();
        unsafe {
            c_fn(&cap as *const _ as *const u8, C2_TYPE_CAPSULE, cp.as_mut_ptr());
            r_fn(&cap as *const _ as *const u8, C2_TYPE_CAPSULE, rp.as_mut_ptr());
            let cp = cp.assume_init();
            let rp = rp.assume_init();
            assert_f_eq("proxy.radius", cp.radius, rp.radius);
            assert_eq!(cp.count, rp.count, "proxy.count");
            for i in 0..2 {
                assert_v_eq(&format!("proxy.verts[{i}]"), cp.verts[i], rp.verts[i]);
            }
        }
    }
}

#[test]
fn test_c2support() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const C2v, c_int, C2v) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Support").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Support").unwrap() };
    let verts = [v(0.0, 0.0), v(10.0, 0.0), v(5.0, 10.0), v(-5.0, 5.0)];
    let dirs = [v(1.0, 0.0), v(0.0, 1.0), v(-1.0, 0.0), v(0.0, -1.0), v(1.0, 1.0)];
    for &d in &dirs {
        let c = unsafe { c_fn(verts.as_ptr(), 4, d) };
        let r = unsafe { r_fn(verts.as_ptr(), 4, d) };
        assert_eq!(c, r, "c2Support dir=({}, {})", d.x, d.y);
    }
}

// ===== Level 3: Simplex operations =====

fn zero_sv() -> C2sv {
    C2sv { s_a: v(0.0, 0.0), s_b: v(0.0, 0.0), p: v(0.0, 0.0), u: 0.0, i_a: 0, i_b: 0 }
}

fn zero_simplex() -> C2Simplex {
    C2Simplex { a: zero_sv(), b: zero_sv(), c: zero_sv(), d: zero_sv(), div: 0.0, count: 0 }
}

fn assert_simplex_eq(label: &str, c: &C2Simplex, r: &C2Simplex) {
    assert_eq!(c.count, r.count, "{label}.count");
    assert_f_eq(&format!("{label}.div"), c.div, r.div);
    // Compare active vertices
    let svs_c = [&c.a, &c.b, &c.c];
    let svs_r = [&r.a, &r.b, &r.c];
    for i in 0..c.count as usize {
        assert_v_eq(&format!("{label}[{i}].p"), svs_c[i].p, svs_r[i].p);
        assert_f_eq(&format!("{label}[{i}].u"), svs_c[i].u, svs_r[i].u);
        assert_v_eq(&format!("{label}[{i}].sA"), svs_c[i].s_a, svs_r[i].s_a);
        assert_v_eq(&format!("{label}[{i}].sB"), svs_c[i].s_b, svs_r[i].s_b);
        assert_eq!(svs_c[i].i_a, svs_r[i].i_a, "{label}[{i}].iA");
        assert_eq!(svs_c[i].i_b, svs_r[i].i_b, "{label}[{i}].iB");
    }
}

#[test]
fn test_c2_gjk_simplex_metric() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*mut C2Simplex) -> f32;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2GJKSimplexMetric").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2GJKSimplexMetric").unwrap() };

    // count=1
    {
        let mut s = zero_simplex();
        s.count = 1;
        s.a.p = v(5.0, 3.0);
        let mut s2 = s;
        assert_f_eq("metric_1", unsafe { c_fn(&mut s) }, unsafe { r_fn(&mut s2) });
    }
    // count=2
    {
        let mut s = zero_simplex();
        s.count = 2;
        s.a.p = v(0.0, 0.0);
        s.b.p = v(3.0, 4.0);
        let mut s2 = s;
        assert_f_eq("metric_2", unsafe { c_fn(&mut s) }, unsafe { r_fn(&mut s2) });
    }
    // count=3
    {
        let mut s = zero_simplex();
        s.count = 3;
        s.a.p = v(0.0, 0.0);
        s.b.p = v(4.0, 0.0);
        s.c.p = v(0.0, 3.0);
        let mut s2 = s;
        assert_f_eq("metric_3", unsafe { c_fn(&mut s) }, unsafe { r_fn(&mut s2) });
    }
}

#[test]
fn test_c22() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*mut C2Simplex);
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c22").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c22").unwrap() };

    let cases = [
        // v <= 0 case
        (v(1.0, 0.0), v(-1.0, 0.0)),
        // u <= 0 case
        (v(-1.0, 0.0), v(1.0, 0.0)),
        // general case
        (v(-1.0, 0.0), v(1.0, 1.0)),
        (v(0.5, 0.5), v(-0.5, 0.5)),
    ];
    for (pa, pb) in cases {
        let mut cs = zero_simplex();
        cs.count = 2; cs.div = 1.0;
        cs.a.p = pa; cs.a.s_a = pa; cs.a.s_b = v(0.0, 0.0);
        cs.b.p = pb; cs.b.s_a = pb; cs.b.s_b = v(0.0, 0.0);
        let mut rs = cs;
        unsafe { c_fn(&mut cs); r_fn(&mut rs); }
        assert_simplex_eq("c22", &cs, &rs);
    }
}

#[test]
fn test_c23() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*mut C2Simplex);
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c23").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c23").unwrap() };

    let cases = [
        (v(-1.0, -1.0), v(1.0, -1.0), v(0.0, 1.0)),
        (v(1.0, 0.0), v(0.0, 1.0), v(-1.0, -1.0)),
        (v(2.0, 0.0), v(0.0, 2.0), v(-2.0, -2.0)),
        (v(0.1, 0.1), v(0.2, 0.0), v(0.0, 0.2)),
    ];
    for (pa, pb, pc) in cases {
        let mut cs = zero_simplex();
        cs.count = 3; cs.div = 1.0;
        cs.a.p = pa; cs.a.s_a = pa;
        cs.b.p = pb; cs.b.s_a = pb;
        cs.c.p = pc; cs.c.s_a = pc;
        let mut rs = cs;
        unsafe { c_fn(&mut cs); r_fn(&mut rs); }
        assert_simplex_eq("c23", &cs, &rs);
    }
}

#[test]
fn test_c2d() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2D").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2D").unwrap() };

    // count=1
    {
        let mut s = zero_simplex();
        s.count = 1; s.a.p = v(3.0, -4.0);
        let mut s2 = s;
        assert_v_eq("c2D_1", unsafe { c_fn(&mut s) }, unsafe { r_fn(&mut s2) });
    }
    // count=2 (skew branch)
    {
        let mut s = zero_simplex();
        s.count = 2; s.a.p = v(-1.0, 0.0); s.b.p = v(1.0, 0.0);
        let mut s2 = s;
        assert_v_eq("c2D_2a", unsafe { c_fn(&mut s) }, unsafe { r_fn(&mut s2) });
    }
    // count=2 (ccw90 branch)
    {
        let mut s = zero_simplex();
        s.count = 2; s.a.p = v(1.0, 0.0); s.b.p = v(-1.0, 0.0);
        let mut s2 = s;
        assert_v_eq("c2D_2b", unsafe { c_fn(&mut s) }, unsafe { r_fn(&mut s2) });
    }
    // count=3
    {
        let mut s = zero_simplex();
        s.count = 3;
        let mut s2 = s;
        assert_v_eq("c2D_3", unsafe { c_fn(&mut s) }, unsafe { r_fn(&mut s2) });
    }
}

#[test]
fn test_c2_witness() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*mut C2Simplex, *mut C2v, *mut C2v);
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Witness").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Witness").unwrap() };

    // count=1
    {
        let mut s = zero_simplex();
        s.count = 1; s.div = 1.0;
        s.a.s_a = v(1.0, 2.0); s.a.s_b = v(3.0, 4.0); s.a.u = 1.0;
        let mut s2 = s;
        let (mut ca, mut cb) = (v(0.0, 0.0), v(0.0, 0.0));
        let (mut ra, mut rb) = (v(0.0, 0.0), v(0.0, 0.0));
        unsafe { c_fn(&mut s, &mut ca, &mut cb); r_fn(&mut s2, &mut ra, &mut rb); }
        assert_v_eq("witness1.a", ca, ra);
        assert_v_eq("witness1.b", cb, rb);
    }
    // count=2
    {
        let mut s = zero_simplex();
        s.count = 2; s.div = 3.0;
        s.a.s_a = v(1.0, 0.0); s.a.s_b = v(0.0, 1.0); s.a.u = 2.0;
        s.b.s_a = v(3.0, 0.0); s.b.s_b = v(0.0, 3.0); s.b.u = 1.0;
        let mut s2 = s;
        let (mut ca, mut cb) = (v(0.0, 0.0), v(0.0, 0.0));
        let (mut ra, mut rb) = (v(0.0, 0.0), v(0.0, 0.0));
        unsafe { c_fn(&mut s, &mut ca, &mut cb); r_fn(&mut s2, &mut ra, &mut rb); }
        assert_v_eq("witness2.a", ca, ra);
        assert_v_eq("witness2.b", cb, rb);
    }
}

#[test]
fn test_c2l() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*mut C2Simplex) -> C2v;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2L").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2L").unwrap() };

    // count=1
    {
        let mut s = zero_simplex();
        s.count = 1; s.div = 1.0; s.a.p = v(5.0, 7.0);
        let mut s2 = s;
        assert_v_eq("c2L_1", unsafe { c_fn(&mut s) }, unsafe { r_fn(&mut s2) });
    }
    // count=2
    {
        let mut s = zero_simplex();
        s.count = 2; s.div = 4.0;
        s.a.p = v(2.0, 0.0); s.a.u = 3.0;
        s.b.p = v(0.0, 2.0); s.b.u = 1.0;
        let mut s2 = s;
        assert_v_eq("c2L_2", unsafe { c_fn(&mut s) }, unsafe { r_fn(&mut s2) });
    }
}

// ===== Level 4: Collision detection functions =====

#[test]
fn test_c2circle_to_circle() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(C2Circle, C2Circle) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2CircletoCircle").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2CircletoCircle").unwrap() };
    let cases = [
        (C2Circle { p: v(0.0, 0.0), r: 5.0 }, C2Circle { p: v(3.0, 0.0), r: 5.0 }),  // overlap
        (C2Circle { p: v(0.0, 0.0), r: 1.0 }, C2Circle { p: v(10.0, 0.0), r: 1.0 }), // no overlap
        (C2Circle { p: v(0.0, 0.0), r: 5.0 }, C2Circle { p: v(0.0, 0.0), r: 5.0 }),  // same
    ];
    for (a, b) in cases {
        let c = unsafe { c_fn(a, b) };
        let r = unsafe { r_fn(a, b) };
        assert_eq!(c, r, "c2CircletoCircle");
    }
}

#[test]
fn test_c2circle_to_aabb() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(C2Circle, C2AABB) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2CircletoAABB").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2CircletoAABB").unwrap() };
    let cases = [
        (C2Circle { p: v(0.0, 0.0), r: 5.0 }, C2AABB { min: v(3.0, -1.0), max: v(6.0, 1.0) }),
        (C2Circle { p: v(0.0, 0.0), r: 1.0 }, C2AABB { min: v(10.0, 10.0), max: v(20.0, 20.0) }),
        (C2Circle { p: v(5.0, 5.0), r: 2.0 }, C2AABB { min: v(0.0, 0.0), max: v(10.0, 10.0) }),
    ];
    for (a, b) in cases {
        assert_eq!(unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) }, "c2CircletoAABB");
    }
}

#[test]
fn test_c2circle_to_capsule() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(C2Circle, C2Capsule) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2CircletoCapsule").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2CircletoCapsule").unwrap() };
    let cases = [
        (C2Circle { p: v(0.0, 0.0), r: 5.0 }, C2Capsule { a: v(3.0, 0.0), b: v(10.0, 0.0), r: 1.0 }),
        (C2Circle { p: v(0.0, 0.0), r: 1.0 }, C2Capsule { a: v(10.0, 10.0), b: v(20.0, 10.0), r: 1.0 }),
        // da < 0 branch
        (C2Circle { p: v(-5.0, 0.0), r: 2.0 }, C2Capsule { a: v(0.0, 0.0), b: v(10.0, 0.0), r: 1.0 }),
        // db >= 0 branch
        (C2Circle { p: v(15.0, 0.0), r: 2.0 }, C2Capsule { a: v(0.0, 0.0), b: v(10.0, 0.0), r: 1.0 }),
    ];
    for (a, b) in cases {
        assert_eq!(unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) }, "c2CircletoCapsule");
    }
}

#[test]
fn test_c2aabb_to_aabb() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(C2AABB, C2AABB) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2AABBtoAABB").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2AABBtoAABB").unwrap() };
    let cases = [
        (C2AABB { min: v(0.0, 0.0), max: v(10.0, 10.0) }, C2AABB { min: v(5.0, 5.0), max: v(15.0, 15.0) }),
        (C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }, C2AABB { min: v(5.0, 5.0), max: v(6.0, 6.0) }),
        (C2AABB { min: v(-40.0, -40.0), max: v(-15.0, -15.0) }, C2AABB { min: v(-100.0, -100.0), max: v(100.0, 100.0) }),
    ];
    for (a, b) in cases {
        assert_eq!(unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) }, "c2AABBtoAABB");
    }
}

#[test]
fn test_c2aabb_to_capsule() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(C2AABB, C2Capsule) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2AABBtoCapsule").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2AABBtoCapsule").unwrap() };
    let cases = [
        (C2AABB { min: v(0.0, 0.0), max: v(10.0, 10.0) }, C2Capsule { a: v(5.0, 5.0), b: v(15.0, 5.0), r: 1.0 }),
        (C2AABB { min: v(0.0, 0.0), max: v(1.0, 1.0) }, C2Capsule { a: v(50.0, 50.0), b: v(60.0, 50.0), r: 1.0 }),
    ];
    for (a, b) in cases {
        assert_eq!(unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) }, "c2AABBtoCapsule");
    }
}

#[test]
fn test_c2capsule_to_capsule() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(C2Capsule, C2Capsule) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2CapsuletoCapsule").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2CapsuletoCapsule").unwrap() };
    let cases = [
        (C2Capsule { a: v(0.0, 0.0), b: v(10.0, 0.0), r: 2.0 }, C2Capsule { a: v(5.0, 1.0), b: v(15.0, 1.0), r: 2.0 }),
        (C2Capsule { a: v(0.0, 0.0), b: v(1.0, 0.0), r: 0.5 }, C2Capsule { a: v(50.0, 50.0), b: v(51.0, 50.0), r: 0.5 }),
    ];
    for (a, b) in cases {
        assert_eq!(unsafe { c_fn(a, b) }, unsafe { r_fn(a, b) }, "c2CapsuletoCapsule");
    }
}

// ===== Level 5: c2Collided and c2GJK =====

#[test]
fn test_c2collided() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(*const u8, c_int, *const u8, c_int) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2Collided").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2Collided").unwrap() };

    let circle = C2Circle { p: v(-70.0, 0.0), r: 20.0 };
    let aabb = C2AABB { min: v(-40.0, -40.0), max: v(-15.0, -15.0) };
    let capsule = C2Capsule { a: v(-40.0, 40.0), b: v(-20.0, 100.0), r: 10.0 };
    let aabb_in = C2AABB { min: v(-80.0, -10.0), max: v(-60.0, 10.0) };

    // Circle vs AABB
    let c = unsafe { c_fn(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, &aabb_in as *const _ as *const u8, C2_TYPE_AABB) };
    let r = unsafe { r_fn(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, &aabb_in as *const _ as *const u8, C2_TYPE_AABB) };
    assert_eq!(c, r, "c2Collided circle-aabb");

    // AABB vs AABB
    let c = unsafe { c_fn(&aabb as *const _ as *const u8, C2_TYPE_AABB, &aabb_in as *const _ as *const u8, C2_TYPE_AABB) };
    let r = unsafe { r_fn(&aabb as *const _ as *const u8, C2_TYPE_AABB, &aabb_in as *const _ as *const u8, C2_TYPE_AABB) };
    assert_eq!(c, r, "c2Collided aabb-aabb");

    // Capsule vs AABB
    let c = unsafe { c_fn(&capsule as *const _ as *const u8, C2_TYPE_CAPSULE, &aabb_in as *const _ as *const u8, C2_TYPE_AABB) };
    let r = unsafe { r_fn(&capsule as *const _ as *const u8, C2_TYPE_CAPSULE, &aabb_in as *const _ as *const u8, C2_TYPE_AABB) };
    assert_eq!(c, r, "c2Collided capsule-aabb");

    // Circle vs Circle
    let c2 = C2Circle { p: v(-60.0, 0.0), r: 5.0 };
    let c = unsafe { c_fn(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, &c2 as *const _ as *const u8, C2_TYPE_CIRCLE) };
    let r = unsafe { r_fn(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, &c2 as *const _ as *const u8, C2_TYPE_CIRCLE) };
    assert_eq!(c, r, "c2Collided circle-circle");

    // Circle vs Capsule
    let c = unsafe { c_fn(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, &capsule as *const _ as *const u8, C2_TYPE_CAPSULE) };
    let r = unsafe { r_fn(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, &capsule as *const _ as *const u8, C2_TYPE_CAPSULE) };
    assert_eq!(c, r, "c2Collided circle-capsule");

    // Capsule vs Capsule
    let cap2 = C2Capsule { a: v(-30.0, 50.0), b: v(-10.0, 90.0), r: 5.0 };
    let c = unsafe { c_fn(&capsule as *const _ as *const u8, C2_TYPE_CAPSULE, &cap2 as *const _ as *const u8, C2_TYPE_CAPSULE) };
    let r = unsafe { r_fn(&capsule as *const _ as *const u8, C2_TYPE_CAPSULE, &cap2 as *const _ as *const u8, C2_TYPE_CAPSULE) };
    assert_eq!(c, r, "c2Collided capsule-capsule");
}

#[test]
fn test_c2gjk() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(
        *const u8, c_int, *const C2x,
        *const u8, c_int, *const C2x,
        *mut C2v, *mut C2v,
        c_int, *mut c_int, *mut C2GJKCache,
    ) -> f32;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"c2GJK").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"c2GJK").unwrap() };

    // Test: two AABBs, no radius
    {
        let a = C2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) };
        let b = C2AABB { min: v(5.0, 0.0), max: v(7.0, 2.0) };
        let (mut c_oa, mut c_ob) = (v(0.0, 0.0), v(0.0, 0.0));
        let (mut r_oa, mut r_ob) = (v(0.0, 0.0), v(0.0, 0.0));
        let mut c_iter = 0i32;
        let mut r_iter = 0i32;
        let cd = unsafe { c_fn(&a as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &b as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &mut c_oa, &mut c_ob, 0, &mut c_iter, std::ptr::null_mut()) };
        let rd = unsafe { r_fn(&a as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &b as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &mut r_oa, &mut r_ob, 0, &mut r_iter, std::ptr::null_mut()) };
        assert_f_eq("c2GJK dist", cd, rd);
        assert_v_eq("c2GJK outA", c_oa, r_oa);
        assert_v_eq("c2GJK outB", c_ob, r_ob);
        assert_eq!(c_iter, r_iter, "c2GJK iterations");
    }

    // Test: circle vs capsule, with radius
    {
        let circle = C2Circle { p: v(0.0, 0.0), r: 3.0 };
        let capsule = C2Capsule { a: v(10.0, 0.0), b: v(20.0, 0.0), r: 2.0 };
        let (mut c_oa, mut c_ob) = (v(0.0, 0.0), v(0.0, 0.0));
        let (mut r_oa, mut r_ob) = (v(0.0, 0.0), v(0.0, 0.0));
        let mut c_iter = 0i32;
        let mut r_iter = 0i32;
        let cd = unsafe { c_fn(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, std::ptr::null(),
            &capsule as *const _ as *const u8, C2_TYPE_CAPSULE, std::ptr::null(),
            &mut c_oa, &mut c_ob, 1, &mut c_iter, std::ptr::null_mut()) };
        let rd = unsafe { r_fn(&circle as *const _ as *const u8, C2_TYPE_CIRCLE, std::ptr::null(),
            &capsule as *const _ as *const u8, C2_TYPE_CAPSULE, std::ptr::null(),
            &mut r_oa, &mut r_ob, 1, &mut r_iter, std::ptr::null_mut()) };
        assert_f_eq("c2GJK dist radius", cd, rd);
        assert_v_eq("c2GJK outA radius", c_oa, r_oa);
        assert_v_eq("c2GJK outB radius", c_ob, r_ob);
    }

    // Test: overlapping shapes (hit case)
    {
        let a = C2AABB { min: v(0.0, 0.0), max: v(10.0, 10.0) };
        let b = C2AABB { min: v(5.0, 5.0), max: v(15.0, 15.0) };
        let (mut c_oa, mut c_ob) = (v(0.0, 0.0), v(0.0, 0.0));
        let (mut r_oa, mut r_ob) = (v(0.0, 0.0), v(0.0, 0.0));
        let cd = unsafe { c_fn(&a as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &b as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &mut c_oa, &mut c_ob, 0, std::ptr::null_mut(), std::ptr::null_mut()) };
        let rd = unsafe { r_fn(&a as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &b as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &mut r_oa, &mut r_ob, 0, std::ptr::null_mut(), std::ptr::null_mut()) };
        assert_f_eq("c2GJK overlap dist", cd, rd);
        assert_v_eq("c2GJK overlap outA", c_oa, r_oa);
        assert_v_eq("c2GJK overlap outB", c_ob, r_ob);
    }

    // Test: with cache
    {
        let a = C2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) };
        let b = C2AABB { min: v(5.0, 0.0), max: v(7.0, 2.0) };
        let mut c_cache = C2GJKCache { metric: 0.0, count: 0, i_a: [0; 3], i_b: [0; 3], div: 0.0 };
        let mut r_cache = c_cache;
        // First call populates cache
        unsafe { c_fn(&a as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &b as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut c_cache); }
        unsafe { r_fn(&a as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &b as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            std::ptr::null_mut(), std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut r_cache); }
        assert_f_eq("cache.metric", c_cache.metric, r_cache.metric);
        assert_eq!(c_cache.count, r_cache.count, "cache.count");
        assert_f_eq("cache.div", c_cache.div, r_cache.div);
        for i in 0..c_cache.count as usize {
            assert_eq!(c_cache.i_a[i], r_cache.i_a[i], "cache.iA[{i}]");
            assert_eq!(c_cache.i_b[i], r_cache.i_b[i], "cache.iB[{i}]");
        }
        // Second call uses cache
        let (mut c_oa, mut c_ob) = (v(0.0, 0.0), v(0.0, 0.0));
        let (mut r_oa, mut r_ob) = (v(0.0, 0.0), v(0.0, 0.0));
        let cd = unsafe { c_fn(&a as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &b as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &mut c_oa, &mut c_ob, 0, std::ptr::null_mut(), &mut c_cache) };
        let rd = unsafe { r_fn(&a as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &b as *const _ as *const u8, C2_TYPE_AABB, std::ptr::null(),
            &mut r_oa, &mut r_ob, 0, std::ptr::null_mut(), &mut r_cache) };
        assert_f_eq("c2GJK cached dist", cd, rd);
        assert_v_eq("c2GJK cached outA", c_oa, r_oa);
        assert_v_eq("c2GJK cached outB", c_ob, r_ob);
    }

    // Test: with transform
    {
        let a = C2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) };
        let b = C2AABB { min: v(0.0, 0.0), max: v(2.0, 2.0) };
        let ax = C2x { p: v(0.0, 0.0), r: C2r { c: 1.0, s: 0.0 } };
        let bx = C2x { p: v(10.0, 0.0), r: C2r { c: 0.707, s: 0.707 } };
        let (mut c_oa, mut c_ob) = (v(0.0, 0.0), v(0.0, 0.0));
        let (mut r_oa, mut r_ob) = (v(0.0, 0.0), v(0.0, 0.0));
        let cd = unsafe { c_fn(&a as *const _ as *const u8, C2_TYPE_AABB, &ax,
            &b as *const _ as *const u8, C2_TYPE_AABB, &bx,
            &mut c_oa, &mut c_ob, 0, std::ptr::null_mut(), std::ptr::null_mut()) };
        let rd = unsafe { r_fn(&a as *const _ as *const u8, C2_TYPE_AABB, &ax,
            &b as *const _ as *const u8, C2_TYPE_AABB, &bx,
            &mut r_oa, &mut r_ob, 0, std::ptr::null_mut(), std::ptr::null_mut()) };
        assert_f_eq("c2GJK xform dist", cd, rd);
        assert_v_eq("c2GJK xform outA", c_oa, r_oa);
        assert_v_eq("c2GJK xform outB", c_ob, r_ob);
    }
}

// ===== Level 6: Top-level aabb function =====

#[test]
fn test_aabb() {
    let libs = Libs::load();
    type Fn = unsafe extern "C" fn(f32, f32, f32, f32) -> c_int;
    let c_fn: Symbol<Fn> = unsafe { libs.c.get(b"aabb").unwrap() };
    let r_fn: Symbol<Fn> = unsafe { libs.rs.get(b"aabb").unwrap() };

    let cases = [
        // Large box covering everything
        (-100.0f32, -100.0, 100.0, 100.0),
        // Small box far away
        (200.0, 200.0, 300.0, 300.0),
        // Box near circle
        (-80.0, -10.0, -60.0, 10.0),
        // Box near AABB
        (-35.0, -35.0, -20.0, -20.0),
        // Box near capsule
        (-30.0, 50.0, -10.0, 80.0),
        // Zero-size box
        (0.0, 0.0, 0.0, 0.0),
        // Negative coords
        (-50.0, -50.0, -45.0, -45.0),
        // Edge cases from the C code's hardcoded shapes
        (-90.0, -20.0, -50.0, 20.0),
        (-55.0, -25.0, -30.0, -10.0),
        (-45.0, 35.0, -15.0, 105.0),
    ];
    for (min_x, min_y, max_x, max_y) in cases {
        let c = unsafe { c_fn(min_x, min_y, max_x, max_y) };
        let r = unsafe { r_fn(min_x, min_y, max_x, max_y) };
        assert_eq!(c, r, "aabb({min_x}, {min_y}, {max_x}, {max_y}): C={c} Rust={r}");
    }
}
