use libloading::{Library, Symbol};
use std::os::raw::c_char;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2v { x: f32, y: f32 }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2r { c: f32, s: f32 }

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct c2x { p: c2v, r: c2r }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2AABB { min: c2v, max: c2v }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Capsule { a: c2v, b: c2v, r: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Circle { p: c2v, r: f32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Proxy { radius: f32, count: i32, verts: [c2v; 8] }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2sv { sA: c2v, sB: c2v, p: c2v, u: f32, iA: i32, iB: i32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2Simplex { a: c2sv, b: c2sv, c: c2sv, d: c2sv, div: f32, count: i32 }

#[repr(C)]
#[derive(Clone, Copy)]
struct c2GJKCache { metric: f32, count: i32, iA: [i32; 3], iB: [i32; 3], div: f32 }

fn c_lib() -> &'static str {
    concat!(env!("CARGO_MANIFEST_DIR"), "/c_src/build/libtranslated_rust.so")
}

fn rust_lib() -> &'static str {
    // The Rust cdylib is built alongside tests
    concat!(env!("CARGO_MANIFEST_DIR"), "/target/debug/libgjk_lib.so")
}

fn assert_v_eq(label: &str, a: c2v, b: c2v) {
    assert!(a.x.to_bits() == b.x.to_bits() && a.y.to_bits() == b.y.to_bits(),
        "{label}: C=({}, {}) Rust=({}, {})", a.x, a.y, b.x, b.y);
}

fn assert_f_eq(label: &str, a: f32, b: f32) {
    assert!(a.to_bits() == b.to_bits(), "{label}: C={a} Rust={b}");
}

#[test]
fn test_c2v() {
    unsafe {
        let c = Library::new(c_lib()).unwrap();
        let r = Library::new(rust_lib()).unwrap();
        let cf: Symbol<unsafe extern "C" fn(f32, f32) -> c2v> = c.get(b"c2V").unwrap();
        let rf: Symbol<unsafe extern "C" fn(f32, f32) -> c2v> = r.get(b"c2V").unwrap();
        for &(x, y) in &[(0.0f32, 0.0), (1.5, -2.3), (-100.0, 100.0), (f32::INFINITY, f32::NAN)] {
            assert_v_eq("c2V", cf(x, y), rf(x, y));
        }
    }
}

#[test]
fn test_c2_arithmetic() {
    unsafe {
        let c = Library::new(c_lib()).unwrap();
        let r = Library::new(rust_lib()).unwrap();

        let c_add: Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = c.get(b"c2Add").unwrap();
        let r_add: Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = r.get(b"c2Add").unwrap();
        let c_sub: Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = c.get(b"c2Sub").unwrap();
        let r_sub: Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = r.get(b"c2Sub").unwrap();
        let c_neg: Symbol<unsafe extern "C" fn(c2v) -> c2v> = c.get(b"c2Neg").unwrap();
        let r_neg: Symbol<unsafe extern "C" fn(c2v) -> c2v> = r.get(b"c2Neg").unwrap();
        let c_mulvs: Symbol<unsafe extern "C" fn(c2v, f32) -> c2v> = c.get(b"c2Mulvs").unwrap();
        let r_mulvs: Symbol<unsafe extern "C" fn(c2v, f32) -> c2v> = r.get(b"c2Mulvs").unwrap();
        let c_div: Symbol<unsafe extern "C" fn(c2v, f32) -> c2v> = c.get(b"c2Div").unwrap();
        let r_div: Symbol<unsafe extern "C" fn(c2v, f32) -> c2v> = r.get(b"c2Div").unwrap();
        let c_dot: Symbol<unsafe extern "C" fn(c2v, c2v) -> f32> = c.get(b"c2Dot").unwrap();
        let r_dot: Symbol<unsafe extern "C" fn(c2v, c2v) -> f32> = r.get(b"c2Dot").unwrap();
        let c_det2: Symbol<unsafe extern "C" fn(c2v, c2v) -> f32> = c.get(b"c2Det2").unwrap();
        let r_det2: Symbol<unsafe extern "C" fn(c2v, c2v) -> f32> = r.get(b"c2Det2").unwrap();
        let c_len: Symbol<unsafe extern "C" fn(c2v) -> f32> = c.get(b"c2Len").unwrap();
        let r_len: Symbol<unsafe extern "C" fn(c2v) -> f32> = r.get(b"c2Len").unwrap();
        let c_norm: Symbol<unsafe extern "C" fn(c2v) -> c2v> = c.get(b"c2Norm").unwrap();
        let r_norm: Symbol<unsafe extern "C" fn(c2v) -> c2v> = r.get(b"c2Norm").unwrap();

        let vecs = [
            c2v { x: 1.0, y: 2.0 }, c2v { x: -3.0, y: 4.0 },
            c2v { x: 0.0, y: 0.0 }, c2v { x: 100.5, y: -0.001 },
        ];
        for i in 0..vecs.len() {
            let a = vecs[i];
            assert_v_eq("c2Neg", c_neg(a), r_neg(a));
            assert_f_eq("c2Len", c_len(a), r_len(a));
            if a.x != 0.0 || a.y != 0.0 {
                assert_v_eq("c2Norm", c_norm(a), r_norm(a));
            }
            for j in 0..vecs.len() {
                let b = vecs[j];
                assert_v_eq("c2Add", c_add(a, b), r_add(a, b));
                assert_v_eq("c2Sub", c_sub(a, b), r_sub(a, b));
                assert_f_eq("c2Dot", c_dot(a, b), r_dot(a, b));
                assert_f_eq("c2Det2", c_det2(a, b), r_det2(a, b));
            }
            for &s in &[0.0f32, 1.0, -2.5, 100.0] {
                assert_v_eq("c2Mulvs", c_mulvs(a, s), r_mulvs(a, s));
                if s != 0.0 {
                    assert_v_eq("c2Div", c_div(a, s), r_div(a, s));
                }
            }
        }
    }
}

#[test]
fn test_c2_vector_ops() {
    unsafe {
        let c = Library::new(c_lib()).unwrap();
        let r = Library::new(rust_lib()).unwrap();

        let c_maxv: Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = c.get(b"c2Maxv").unwrap();
        let r_maxv: Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = r.get(b"c2Maxv").unwrap();
        let c_minv: Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = c.get(b"c2Minv").unwrap();
        let r_minv: Symbol<unsafe extern "C" fn(c2v, c2v) -> c2v> = r.get(b"c2Minv").unwrap();
        let c_clamp: Symbol<unsafe extern "C" fn(c2v, c2v, c2v) -> c2v> = c.get(b"c2Clampv").unwrap();
        let r_clamp: Symbol<unsafe extern "C" fn(c2v, c2v, c2v) -> c2v> = r.get(b"c2Clampv").unwrap();
        let c_skew: Symbol<unsafe extern "C" fn(c2v) -> c2v> = c.get(b"c2Skew").unwrap();
        let r_skew: Symbol<unsafe extern "C" fn(c2v) -> c2v> = r.get(b"c2Skew").unwrap();
        let c_ccw: Symbol<unsafe extern "C" fn(c2v) -> c2v> = c.get(b"c2CCW90").unwrap();
        let r_ccw: Symbol<unsafe extern "C" fn(c2v) -> c2v> = r.get(b"c2CCW90").unwrap();

        let vecs = [
            c2v { x: 1.0, y: 2.0 }, c2v { x: -3.0, y: 4.0 },
            c2v { x: 0.0, y: 0.0 }, c2v { x: 5.0, y: -1.0 },
        ];
        for a in &vecs {
            assert_v_eq("c2Skew", c_skew(*a), r_skew(*a));
            assert_v_eq("c2CCW90", c_ccw(*a), r_ccw(*a));
            for b in &vecs {
                assert_v_eq("c2Maxv", c_maxv(*a, *b), r_maxv(*a, *b));
                assert_v_eq("c2Minv", c_minv(*a, *b), r_minv(*a, *b));
            }
        }
        let lo = c2v { x: -1.0, y: -1.0 };
        let hi = c2v { x: 1.0, y: 1.0 };
        for a in &vecs {
            assert_v_eq("c2Clampv", c_clamp(*a, lo, hi), r_clamp(*a, lo, hi));
        }
    }
}

#[test]
fn test_c2_rotation() {
    unsafe {
        let c = Library::new(c_lib()).unwrap();
        let r = Library::new(rust_lib()).unwrap();

        let c_rot: Symbol<unsafe extern "C" fn() -> c2r> = c.get(b"c2RotIdentity").unwrap();
        let r_rot: Symbol<unsafe extern "C" fn() -> c2r> = r.get(b"c2RotIdentity").unwrap();
        let c_xi: Symbol<unsafe extern "C" fn() -> c2x> = c.get(b"c2xIdentity").unwrap();
        let r_xi: Symbol<unsafe extern "C" fn() -> c2x> = r.get(b"c2xIdentity").unwrap();
        let c_mulrv: Symbol<unsafe extern "C" fn(c2r, c2v) -> c2v> = c.get(b"c2Mulrv").unwrap();
        let r_mulrv: Symbol<unsafe extern "C" fn(c2r, c2v) -> c2v> = r.get(b"c2Mulrv").unwrap();
        let c_mulrvt: Symbol<unsafe extern "C" fn(c2r, c2v) -> c2v> = c.get(b"c2MulrvT").unwrap();
        let r_mulrvt: Symbol<unsafe extern "C" fn(c2r, c2v) -> c2v> = r.get(b"c2MulrvT").unwrap();
        let c_mulxv: Symbol<unsafe extern "C" fn(c2x, c2v) -> c2v> = c.get(b"c2Mulxv").unwrap();
        let r_mulxv: Symbol<unsafe extern "C" fn(c2x, c2v) -> c2v> = r.get(b"c2Mulxv").unwrap();

        let cr = c_rot(); let rr = r_rot();
        assert_f_eq("RotIdentity.c", cr.c, rr.c);
        assert_f_eq("RotIdentity.s", cr.s, rr.s);

        let cx = c_xi(); let rx = r_xi();
        assert_v_eq("xIdentity.p", cx.p, rx.p);
        assert_f_eq("xIdentity.r.c", cx.r.c, rx.r.c);
        assert_f_eq("xIdentity.r.s", cx.r.s, rx.r.s);

        let rots = [c2r { c: 1.0, s: 0.0 }, c2r { c: 0.0, s: 1.0 }, c2r { c: 0.707, s: 0.707 }];
        let vecs = [c2v { x: 1.0, y: 0.0 }, c2v { x: 0.0, y: 1.0 }, c2v { x: 3.0, y: -4.0 }];
        for rot in &rots {
            for v in &vecs {
                assert_v_eq("c2Mulrv", c_mulrv(*rot, *v), r_mulrv(*rot, *v));
                assert_v_eq("c2MulrvT", c_mulrvt(*rot, *v), r_mulrvt(*rot, *v));
            }
        }
        let xforms = [
            c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } },
            c2x { p: c2v { x: 5.0, y: -3.0 }, r: c2r { c: 0.0, s: 1.0 } },
        ];
        for xf in &xforms {
            for v in &vecs {
                assert_v_eq("c2Mulxv", c_mulxv(*xf, *v), r_mulxv(*xf, *v));
            }
        }
    }
}

#[test]
fn test_c2bbverts() {
    unsafe {
        let c = Library::new(c_lib()).unwrap();
        let r = Library::new(rust_lib()).unwrap();
        let cf: Symbol<unsafe extern "C" fn(*mut c2v, *mut c2AABB)> = c.get(b"c2BBVerts").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*mut c2v, *mut c2AABB)> = r.get(b"c2BBVerts").unwrap();

        let mut bb = c2AABB { min: c2v { x: -1.0, y: -2.0 }, max: c2v { x: 3.0, y: 4.0 } };
        let mut c_out = [c2v { x: 0.0, y: 0.0 }; 4];
        let mut r_out = [c2v { x: 0.0, y: 0.0 }; 4];
        cf(c_out.as_mut_ptr(), &mut bb);
        rf(r_out.as_mut_ptr(), &mut bb);
        for i in 0..4 {
            assert_v_eq(&format!("c2BBVerts[{i}]"), c_out[i], r_out[i]);
        }
    }
}

#[test]
fn test_c2makeproxy() {
    unsafe {
        let c = Library::new(c_lib()).unwrap();
        let r = Library::new(rust_lib()).unwrap();
        let cf: Symbol<unsafe extern "C" fn(*const u8, i32, *mut c2Proxy)> = c.get(b"c2MakeProxy").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const u8, i32, *mut c2Proxy)> = r.get(b"c2MakeProxy").unwrap();

        // Circle (type 0)
        let circle = c2Circle { p: c2v { x: 1.0, y: 2.0 }, r: 3.0 };
        let mut cp = std::mem::zeroed::<c2Proxy>();
        let mut rp = std::mem::zeroed::<c2Proxy>();
        cf(&circle as *const _ as *const u8, 0, &mut cp);
        rf(&circle as *const _ as *const u8, 0, &mut rp);
        assert_f_eq("proxy_circle.radius", cp.radius, rp.radius);
        assert_eq!(cp.count, rp.count, "proxy_circle.count");
        assert_v_eq("proxy_circle.verts[0]", cp.verts[0], rp.verts[0]);

        // AABB (type 1)
        let bb = c2AABB { min: c2v { x: -1.0, y: -2.0 }, max: c2v { x: 3.0, y: 4.0 } };
        let mut cp = std::mem::zeroed::<c2Proxy>();
        let mut rp = std::mem::zeroed::<c2Proxy>();
        cf(&bb as *const _ as *const u8, 1, &mut cp);
        rf(&bb as *const _ as *const u8, 1, &mut rp);
        assert_f_eq("proxy_aabb.radius", cp.radius, rp.radius);
        assert_eq!(cp.count, rp.count, "proxy_aabb.count");
        for i in 0..4 {
            assert_v_eq(&format!("proxy_aabb.verts[{i}]"), cp.verts[i], rp.verts[i]);
        }

        // Capsule (type 2)
        let cap = c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 5.0, y: 5.0 }, r: 1.5 };
        let mut cp = std::mem::zeroed::<c2Proxy>();
        let mut rp = std::mem::zeroed::<c2Proxy>();
        cf(&cap as *const _ as *const u8, 2, &mut cp);
        rf(&cap as *const _ as *const u8, 2, &mut rp);
        assert_f_eq("proxy_cap.radius", cp.radius, rp.radius);
        assert_eq!(cp.count, rp.count, "proxy_cap.count");
        for i in 0..2 {
            assert_v_eq(&format!("proxy_cap.verts[{i}]"), cp.verts[i], rp.verts[i]);
        }
    }
}

fn zero_sv() -> c2sv {
    c2sv { sA: c2v{x:0.0,y:0.0}, sB: c2v{x:0.0,y:0.0}, p: c2v{x:0.0,y:0.0}, u: 0.0, iA: 0, iB: 0 }
}

fn make_simplex_2() -> c2Simplex {
    c2Simplex {
        a: c2sv { sA: c2v{x:1.0,y:0.0}, sB: c2v{x:3.0,y:0.0}, p: c2v{x:2.0,y:0.0}, u: 0.0, iA: 0, iB: 0 },
        b: c2sv { sA: c2v{x:0.0,y:1.0}, sB: c2v{x:0.0,y:3.0}, p: c2v{x:0.0,y:2.0}, u: 0.0, iA: 1, iB: 1 },
        c: zero_sv(), d: zero_sv(), div: 1.0, count: 2,
    }
}

fn make_simplex_3() -> c2Simplex {
    c2Simplex {
        a: c2sv { sA: c2v{x:0.0,y:0.0}, sB: c2v{x:2.0,y:0.0}, p: c2v{x:1.0,y:0.0}, u: 0.0, iA: 0, iB: 0 },
        b: c2sv { sA: c2v{x:0.0,y:0.0}, sB: c2v{x:0.0,y:2.0}, p: c2v{x:0.0,y:1.0}, u: 0.0, iA: 1, iB: 1 },
        c: c2sv { sA: c2v{x:0.0,y:0.0}, sB: c2v{x:-2.0,y:0.0}, p: c2v{x:-1.0,y:0.0}, u: 0.0, iA: 2, iB: 2 },
        d: zero_sv(), div: 1.0, count: 3,
    }
}

fn assert_simplex_eq(label: &str, a: &c2Simplex, b: &c2Simplex) {
    assert_eq!(a.count, b.count, "{label}.count");
    assert_f_eq(&format!("{label}.div"), a.div, b.div);
    // Compare active vertices
    let svs_a = [&a.a, &a.b, &a.c, &a.d];
    let svs_b = [&b.a, &b.b, &b.c, &b.d];
    for i in 0..a.count as usize {
        assert_v_eq(&format!("{label}[{i}].p"), svs_a[i].p, svs_b[i].p);
        assert_v_eq(&format!("{label}[{i}].sA"), svs_a[i].sA, svs_b[i].sA);
        assert_v_eq(&format!("{label}[{i}].sB"), svs_a[i].sB, svs_b[i].sB);
        assert_f_eq(&format!("{label}[{i}].u"), svs_a[i].u, svs_b[i].u);
        assert_eq!(svs_a[i].iA, svs_b[i].iA, "{label}[{i}].iA");
        assert_eq!(svs_a[i].iB, svs_b[i].iB, "{label}[{i}].iB");
    }
}

#[test]
fn test_simplex_ops() {
    unsafe {
        let c = Library::new(c_lib()).unwrap();
        let r = Library::new(rust_lib()).unwrap();

        let c_metric: Symbol<unsafe extern "C" fn(*mut c2Simplex) -> f32> = c.get(b"c2GJKSimplexMetric").unwrap();
        let r_metric: Symbol<unsafe extern "C" fn(*mut c2Simplex) -> f32> = r.get(b"c2GJKSimplexMetric").unwrap();
        let c_d: Symbol<unsafe extern "C" fn(*mut c2Simplex) -> c2v> = c.get(b"c2D").unwrap();
        let r_d: Symbol<unsafe extern "C" fn(*mut c2Simplex) -> c2v> = r.get(b"c2D").unwrap();
        let c_l: Symbol<unsafe extern "C" fn(*mut c2Simplex) -> c2v> = c.get(b"c2L").unwrap();
        let r_l: Symbol<unsafe extern "C" fn(*mut c2Simplex) -> c2v> = r.get(b"c2L").unwrap();
        let c_witness: Symbol<unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v)> = c.get(b"c2Witness").unwrap();
        let r_witness: Symbol<unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v)> = r.get(b"c2Witness").unwrap();
        let c_c22: Symbol<unsafe extern "C" fn(*mut c2Simplex)> = c.get(b"c22").unwrap();
        let r_c22: Symbol<unsafe extern "C" fn(*mut c2Simplex)> = r.get(b"c22").unwrap();
        let c_c23: Symbol<unsafe extern "C" fn(*mut c2Simplex)> = c.get(b"c23").unwrap();
        let r_c23: Symbol<unsafe extern "C" fn(*mut c2Simplex)> = r.get(b"c23").unwrap();

        // Test with count=1 simplex
        let mut s1 = c2Simplex {
            a: c2sv { sA: c2v{x:1.0,y:2.0}, sB: c2v{x:3.0,y:4.0}, p: c2v{x:2.0,y:2.0}, u: 1.0, iA: 0, iB: 0 },
            b: zero_sv(), c: zero_sv(), d: zero_sv(), div: 1.0, count: 1,
        };
        let mut s1r = s1;
        assert_f_eq("metric_1", c_metric(&mut s1), r_metric(&mut s1r));
        assert_v_eq("D_1", c_d(&mut s1), r_d(&mut s1r));
        assert_v_eq("L_1", c_l(&mut s1), r_l(&mut s1r));
        let (mut ca, mut cb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let (mut ra, mut rb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        c_witness(&mut s1, &mut ca, &mut cb);
        r_witness(&mut s1r, &mut ra, &mut rb);
        assert_v_eq("witness_1_a", ca, ra);
        assert_v_eq("witness_1_b", cb, rb);

        // Test with count=2 simplex
        let mut s2c = make_simplex_2();
        let mut s2r = make_simplex_2();
        assert_f_eq("metric_2", c_metric(&mut s2c), r_metric(&mut s2r));
        assert_v_eq("D_2", c_d(&mut s2c), r_d(&mut s2r));
        assert_v_eq("L_2", c_l(&mut s2c), r_l(&mut s2r));

        // c22
        let mut s2c = make_simplex_2();
        let mut s2r = make_simplex_2();
        c_c22(&mut s2c);
        r_c22(&mut s2r);
        assert_simplex_eq("c22", &s2c, &s2r);

        // Test with count=3 simplex
        let mut s3c = make_simplex_3();
        let mut s3r = make_simplex_3();
        assert_f_eq("metric_3", c_metric(&mut s3c), r_metric(&mut s3r));

        // c23
        let mut s3c = make_simplex_3();
        let mut s3r = make_simplex_3();
        c_c23(&mut s3c);
        r_c23(&mut s3r);
        assert_simplex_eq("c23", &s3c, &s3r);
    }
}

#[test]
fn test_c2support() {
    unsafe {
        let c = Library::new(c_lib()).unwrap();
        let r = Library::new(rust_lib()).unwrap();
        let cf: Symbol<unsafe extern "C" fn(*const c2v, i32, c2v) -> i32> = c.get(b"c2Support").unwrap();
        let rf: Symbol<unsafe extern "C" fn(*const c2v, i32, c2v) -> i32> = r.get(b"c2Support").unwrap();

        let verts = [
            c2v{x:0.0,y:0.0}, c2v{x:1.0,y:0.0}, c2v{x:1.0,y:1.0}, c2v{x:0.0,y:1.0},
            c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0},
        ];
        let dirs = [
            c2v{x:1.0,y:0.0}, c2v{x:0.0,y:1.0}, c2v{x:-1.0,y:0.0},
            c2v{x:1.0,y:1.0}, c2v{x:-1.0,y:-1.0},
        ];
        for d in &dirs {
            let ci = cf(verts.as_ptr(), 4, *d);
            let ri = rf(verts.as_ptr(), 4, *d);
            assert_eq!(ci, ri, "c2Support dir=({},{})", d.x, d.y);
        }
    }
}

#[test]
fn test_c2gjk() {
    unsafe {
        let c = Library::new(c_lib()).unwrap();
        let r = Library::new(rust_lib()).unwrap();
        type GjkFn = unsafe extern "C" fn(*const u8, i32, *const c2x, *const u8, i32, *const c2x,
            *mut c2v, *mut c2v, i32, *mut i32, *mut c2GJKCache) -> f32;
        let cf: Symbol<GjkFn> = c.get(b"c2GJK").unwrap();
        let rf: Symbol<GjkFn> = r.get(b"c2GJK").unwrap();

        // AABB vs Capsule, no transform, use_radius=1
        let bb = c2AABB { min: c2v{x:-1.0,y:-1.0}, max: c2v{x:1.0,y:1.0} };
        let cap = c2Capsule { a: c2v{x:3.0,y:0.0}, b: c2v{x:5.0,y:0.0}, r: 0.5 };
        let (mut ca, mut cb, mut ra2, mut rb2) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let mut c_iter = 0i32; let mut r_iter = 0i32;
        let cd = cf(&bb as *const _ as *const u8, 1, std::ptr::null(),
                     &cap as *const _ as *const u8, 2, std::ptr::null(),
                     &mut ca, &mut cb, 1, &mut c_iter, std::ptr::null_mut());
        let rd = rf(&bb as *const _ as *const u8, 1, std::ptr::null(),
                     &cap as *const _ as *const u8, 2, std::ptr::null(),
                     &mut ra2, &mut rb2, 1, &mut r_iter, std::ptr::null_mut());
        assert_f_eq("c2GJK_dist", cd, rd);
        assert_v_eq("c2GJK_outA", ca, ra2);
        assert_v_eq("c2GJK_outB", cb, rb2);
        assert_eq!(c_iter, r_iter, "c2GJK_iter");

        // Overlapping case
        let bb2 = c2AABB { min: c2v{x:-1.0,y:-1.0}, max: c2v{x:1.0,y:1.0} };
        let cap2 = c2Capsule { a: c2v{x:0.0,y:0.0}, b: c2v{x:0.5,y:0.0}, r: 0.5 };
        let (mut ca, mut cb, mut ra2, mut rb2) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let cd = cf(&bb2 as *const _ as *const u8, 1, std::ptr::null(),
                     &cap2 as *const _ as *const u8, 2, std::ptr::null(),
                     &mut ca, &mut cb, 1, std::ptr::null_mut(), std::ptr::null_mut());
        let rd = rf(&bb2 as *const _ as *const u8, 1, std::ptr::null(),
                     &cap2 as *const _ as *const u8, 2, std::ptr::null(),
                     &mut ra2, &mut rb2, 1, std::ptr::null_mut(), std::ptr::null_mut());
        assert_f_eq("c2GJK_overlap_dist", cd, rd);
        assert_v_eq("c2GJK_overlap_outA", ca, ra2);
        assert_v_eq("c2GJK_overlap_outB", cb, rb2);

        // Circle vs AABB
        let circ = c2Circle { p: c2v{x:5.0,y:5.0}, r: 1.0 };
        let bb3 = c2AABB { min: c2v{x:0.0,y:0.0}, max: c2v{x:2.0,y:2.0} };
        let (mut ca, mut cb, mut ra2, mut rb2) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
        let cd = cf(&circ as *const _ as *const u8, 0, std::ptr::null(),
                     &bb3 as *const _ as *const u8, 1, std::ptr::null(),
                     &mut ca, &mut cb, 1, std::ptr::null_mut(), std::ptr::null_mut());
        let rd = rf(&circ as *const _ as *const u8, 0, std::ptr::null(),
                     &bb3 as *const _ as *const u8, 1, std::ptr::null(),
                     &mut ra2, &mut rb2, 1, std::ptr::null_mut(), std::ptr::null_mut());
        assert_f_eq("c2GJK_circ_dist", cd, rd);
        assert_v_eq("c2GJK_circ_outA", ca, ra2);
        assert_v_eq("c2GJK_circ_outB", cb, rb2);
    }
}

#[test]
fn test_gjk_wrapper() {
    unsafe {
        let c = Library::new(c_lib()).unwrap();
        let r = Library::new(rust_lib()).unwrap();
        type GjkWrap = unsafe extern "C" fn(c_char, *mut c2v, *mut c2v, f32,f32,f32,f32, f32,f32,f32,f32,f32);
        let cf: Symbol<GjkWrap> = c.get(b"gjk").unwrap();
        let rf: Symbol<GjkWrap> = r.get(b"gjk").unwrap();

        let cases: &[(i8, f32,f32,f32,f32, f32,f32,f32,f32,f32)] = &[
            // (reverse, a1,a2,a3,a4, b1,b2,b3,b4,b5)
            (0, -1.0,-1.0,1.0,1.0, 3.0,0.0,5.0,0.0,0.5),   // separated
            (1, -1.0,-1.0,1.0,1.0, 3.0,0.0,5.0,0.0,0.5),   // reversed
            (0, 0.0,0.0,2.0,2.0, 1.0,1.0,3.0,1.0,0.5),     // touching/overlap
            (1, 0.0,0.0,2.0,2.0, 1.0,1.0,3.0,1.0,0.5),     // reversed overlap
            (0, -10.0,-10.0,10.0,10.0, 0.0,0.0,0.0,0.0,1.0), // contained
            (0, 0.0,0.0,1.0,1.0, 100.0,100.0,200.0,200.0,0.1), // far apart
        ];

        for (i, &(rev, a1,a2,a3,a4, b1,b2,b3,b4,b5)) in cases.iter().enumerate() {
            let (mut ca, mut cb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
            let (mut ra, mut rb) = (c2v{x:0.0,y:0.0}, c2v{x:0.0,y:0.0});
            cf(rev, &mut ca, &mut cb, a1,a2,a3,a4, b1,b2,b3,b4,b5);
            rf(rev, &mut ra, &mut rb, a1,a2,a3,a4, b1,b2,b3,b4,b5);
            assert_v_eq(&format!("gjk[{i}].a"), ca, ra);
            assert_v_eq(&format!("gjk[{i}].b"), cb, rb);
        }
    }
}
