//! Level 4: NaN *payload* propagation.
//!
//! When both operands of an x86 SSE arithmetic instruction are NaN, the result
//! is the first source operand (the destination register) quieted. Which
//! source-level operand lands there is a register-allocation decision, and the
//! reference C build makes a different choice than LLVM for several
//! expressions. `src/lib.rs` pins the reference behaviour down explicitly; this
//! file is the regression test for it.
//!
//! Each test *collects* mismatches rather than failing on the first one, so a
//! regression reports the whole affected class at once.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_void;

fn nan(bits: u32) -> f32 {
    f32::from_bits(bits)
}

fn b(v: f32) -> String {
    format!("{:#010x}", v.to_bits())
}

/// Distinct NaN payloads plus the operands that make x86 generate its own
/// "indefinite" NaN (0xffc00000) via invalid operations.
fn vals() -> Vec<f32> {
    vec![
        nan(0x7fc0_0000),
        nan(0xffc0_0000),
        nan(0x7fc0_dead),
        nan(0xffc0_beef),
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -3.5,
    ]
}

struct Report {
    lines: Vec<String>,
}

impl Report {
    fn new() -> Self {
        Report { lines: Vec::new() }
    }
    fn note(&mut self, s: String) {
        if self.lines.len() < 2000 {
            self.lines.push(s);
        }
    }
    fn finish(self, what: &str) {
        if !self.lines.is_empty() {
            for l in &self.lines {
                println!("{l}");
            }
            panic!("{} mismatch(es) in {what}", self.lines.len());
        }
        println!("{what}: OK");
    }
}

#[test]
fn nan_payloads_scalar_returning_leaves() {
    let l = libs();
    let v = vals();
    let mut rep = Report::new();

    for name in ["c2Dot", "c2Det2"] {
        let (c, r) = l.pair::<unsafe extern "C" fn(c2v, c2v) -> f32>(name);
        for &ax in &v {
            for &ay in &v {
                for &bx in &v {
                    for &by in &v {
                        let p = c2v { x: ax, y: ay };
                        let q = c2v { x: bx, y: by };
                        let (cv, rv) = unsafe { (c(p, q), r(p, q)) };
                        if cv.to_bits() != rv.to_bits() {
                            rep.note(format!(
                                "{name}(({},{}),({},{})) C={} R={}",
                                b(ax),
                                b(ay),
                                b(bx),
                                b(by),
                                b(cv),
                                b(rv)
                            ));
                        }
                    }
                }
            }
        }
    }

    let (c, r) = l.pair::<unsafe extern "C" fn(c2v) -> f32>("c2Len");
    for &ax in &v {
        for &ay in &v {
            let p = c2v { x: ax, y: ay };
            let (cv, rv) = unsafe { (c(p), r(p)) };
            if cv.to_bits() != rv.to_bits() {
                rep.note(format!("c2Len(({},{})) C={} R={}", b(ax), b(ay), b(cv), b(rv)));
            }
        }
    }
    rep.finish("scalar-returning leaves");
}

#[test]
fn nan_payloads_vector_returning_leaves() {
    let l = libs();
    let v = vals();
    let mut rep = Report::new();

    for name in ["c2Maxv", "c2Minv", "c2Sub", "c2Add"] {
        let (c, r) = l.pair::<unsafe extern "C" fn(c2v, c2v) -> c2v>(name);
        for &ax in &v {
            for &ay in &v {
                for &bx in &v {
                    for &by in &v {
                        let p = c2v { x: ax, y: ay };
                        let q = c2v { x: bx, y: by };
                        let (cv, rv) = unsafe { (c(p, q), r(p, q)) };
                        if raw(&cv) != raw(&rv) {
                            rep.note(format!(
                                "{name}(({},{}),({},{})) C=({},{}) R=({},{})",
                                b(ax),
                                b(ay),
                                b(bx),
                                b(by),
                                b(cv.x),
                                b(cv.y),
                                b(rv.x),
                                b(rv.y)
                            ));
                        }
                    }
                }
            }
        }
    }

    for name in ["c2Neg", "c2Skew", "c2CCW90", "c2Norm"] {
        let (c, r) = l.pair::<unsafe extern "C" fn(c2v) -> c2v>(name);
        for &ax in &v {
            for &ay in &v {
                let p = c2v { x: ax, y: ay };
                let (cv, rv) = unsafe { (c(p), r(p)) };
                if raw(&cv) != raw(&rv) {
                    rep.note(format!(
                        "{name}(({},{})) C=({},{}) R=({},{})",
                        b(ax),
                        b(ay),
                        b(cv.x),
                        b(cv.y),
                        b(rv.x),
                        b(rv.y)
                    ));
                }
            }
        }
    }

    for name in ["c2Mulvs", "c2Div"] {
        let (c, r) = l.pair::<unsafe extern "C" fn(c2v, f32) -> c2v>(name);
        for &ax in &v {
            for &ay in &v {
                for &s in &v {
                    let p = c2v { x: ax, y: ay };
                    let (cv, rv) = unsafe { (c(p, s), r(p, s)) };
                    if raw(&cv) != raw(&rv) {
                        rep.note(format!(
                            "{name}(({},{}),{}) C=({},{}) R=({},{})",
                            b(ax),
                            b(ay),
                            b(s),
                            b(cv.x),
                            b(cv.y),
                            b(rv.x),
                            b(rv.y)
                        ));
                    }
                }
            }
        }
    }

    let (c, r) = l.pair::<unsafe extern "C" fn(c2v, c2v, c2v) -> c2v>("c2Clampv");
    for &ax in &v {
        for &bx in &v {
            for &cx in &v {
                let p = c2v { x: ax, y: bx };
                let lo = c2v { x: bx, y: cx };
                let hi = c2v { x: cx, y: ax };
                let (cv, rv) = unsafe { (c(p, lo, hi), r(p, lo, hi)) };
                if raw(&cv) != raw(&rv) {
                    rep.note(format!("c2Clampv({p:?},{lo:?},{hi:?}) C={cv:?} R={rv:?}"));
                }
            }
        }
    }
    rep.finish("vector-returning leaves");
}

#[test]
fn nan_payloads_rotation_leaves() {
    let l = libs();
    let v = vals();
    let mut rep = Report::new();

    for name in ["c2Mulrv", "c2MulrvT"] {
        let (c, r) = l.pair::<unsafe extern "C" fn(c2r, c2v) -> c2v>(name);
        for &rc in &v {
            for &rs in &v {
                for &bx in &v {
                    for &by in &v {
                        let rot = c2r { c: rc, s: rs };
                        let q = c2v { x: bx, y: by };
                        let (cv, rv) = unsafe { (c(rot, q), r(rot, q)) };
                        if raw(&cv) != raw(&rv) {
                            rep.note(format!(
                                "{name}(({},{}),({},{})) C=({},{}) R=({},{})",
                                b(rc),
                                b(rs),
                                b(bx),
                                b(by),
                                b(cv.x),
                                b(cv.y),
                                b(rv.x),
                                b(rv.y)
                            ));
                        }
                    }
                }
            }
        }
    }

    let (c, r) = l.pair::<unsafe extern "C" fn(c2x, c2v) -> c2v>("c2Mulxv");
    for &rc in &v {
        for &rs in &v {
            for &px in &v {
                for &by in &v {
                    let x = c2x { p: c2v { x: px, y: rs }, r: c2r { c: rc, s: rs } };
                    let q = c2v { x: by, y: px };
                    let (cv, rv) = unsafe { (c(x, q), r(x, q)) };
                    if raw(&cv) != raw(&rv) {
                        rep.note(format!("c2Mulxv({x:?},{q:?}) C={cv:?} R={rv:?}"));
                    }
                }
            }
        }
    }
    rep.finish("rotation leaves");
}

#[test]
fn nan_payloads_simplex_functions() {
    let l = libs();
    let v = vals();
    let mut rep = Report::new();

    let (c22c, c22r) = l.pair::<unsafe extern "C" fn(*mut c2Simplex)>("c22");
    let (c23c, c23r) = l.pair::<unsafe extern "C" fn(*mut c2Simplex)>("c23");
    let (cDc, cDr) = l.pair::<unsafe extern "C" fn(*mut c2Simplex) -> c2v>("c2D");
    let (cLc, cLr) = l.pair::<unsafe extern "C" fn(*mut c2Simplex) -> c2v>("c2L");
    let (cMc, cMr) = l.pair::<unsafe extern "C" fn(*mut c2Simplex) -> f32>("c2GJKSimplexMetric");
    let (cWc, cWr) =
        l.pair::<unsafe extern "C" fn(*mut c2Simplex, *mut c2v, *mut c2v)>("c2Witness");

    for &p0x in &v {
        for &p0y in &v {
            for &p1x in &v {
                for &p1y in &v {
                    let mut s = c2Simplex::default();
                    s.verts[0].p = c2v { x: p0x, y: p0y };
                    s.verts[1].p = c2v { x: p1x, y: p1y };
                    s.verts[2].p = c2v { x: p1y, y: p0x };
                    s.verts[3].p = c2v { x: p0y, y: p1x };
                    for k in 0..4 {
                        s.verts[k].sA = c2v { x: p0x, y: p1y };
                        s.verts[k].sB = c2v { x: p1x, y: p0y };
                        s.verts[k].u = [p0x, p0y, p1x, p1y][k];
                        s.verts[k].iA = k as i32;
                        s.verts[k].iB = (3 - k) as i32;
                    }
                    s.div = p1x;

                    for count in [0i32, 1, 2, 3, 4] {
                        s.count = count;

                        let (mut a, mut bb) = (s, s);
                        unsafe {
                            c22c(&mut a);
                            c22r(&mut bb);
                        }
                        if raw(&a) != raw(&bb) {
                            rep.note(format!("c22 in={s:?}\n     C={a:?}\n     R={bb:?}"));
                        }

                        let (mut a, mut bb) = (s, s);
                        unsafe {
                            c23c(&mut a);
                            c23r(&mut bb);
                        }
                        if raw(&a) != raw(&bb) {
                            rep.note(format!("c23 in={s:?}\n     C={a:?}\n     R={bb:?}"));
                        }

                        let (mut a, mut bb) = (s, s);
                        let (x, y) = unsafe { (cDc(&mut a), cDr(&mut bb)) };
                        if raw(&x) != raw(&y) {
                            rep.note(format!("c2D in={s:?} C={x:?} R={y:?}"));
                        }

                        let (mut a, mut bb) = (s, s);
                        let (x, y) = unsafe { (cLc(&mut a), cLr(&mut bb)) };
                        if raw(&x) != raw(&y) {
                            rep.note(format!("c2L in={s:?} C={x:?} R={y:?}"));
                        }

                        let (mut a, mut bb) = (s, s);
                        let (x, y) = unsafe { (cMc(&mut a), cMr(&mut bb)) };
                        if x.to_bits() != y.to_bits() {
                            rep.note(format!(
                                "c2GJKSimplexMetric in={s:?} C={} R={}",
                                b(x),
                                b(y)
                            ));
                        }

                        let fill = c2v { x: 7.0, y: -7.0 };
                        let (mut a, mut bb) = (s, s);
                        let (mut w1, mut w2) = (fill, fill);
                        let (mut w3, mut w4) = (fill, fill);
                        unsafe {
                            cWc(&mut a, &mut w1, &mut w2);
                            cWr(&mut bb, &mut w3, &mut w4);
                        }
                        if raw(&w1) != raw(&w3) || raw(&w2) != raw(&w4) {
                            rep.note(format!(
                                "c2Witness in={s:?} C=({w1:?},{w2:?}) R=({w3:?},{w4:?})"
                            ));
                        }
                    }
                }
            }
        }
    }
    rep.finish("simplex functions");
}

#[test]
fn nan_payloads_predicates_and_gjk() {
    let l = libs();
    let v = vals();
    let mut rep = Report::new();

    let (c_cc, r_cc) =
        l.pair::<unsafe extern "C" fn(c2Circle, c2Circle) -> i32>("c2CircletoCircle");
    let (c_ca, r_ca) = l.pair::<unsafe extern "C" fn(c2Circle, c2AABB) -> i32>("c2CircletoAABB");
    let (c_ck, r_ck) =
        l.pair::<unsafe extern "C" fn(c2Circle, c2Capsule) -> i32>("c2CircletoCapsule");
    let (c_aa, r_aa) = l.pair::<unsafe extern "C" fn(c2AABB, c2AABB) -> i32>("c2AABBtoAABB");

    for &a in &v {
        for &bq in &v {
            for &cq in &v {
                let c1 = c2Circle { p: c2v { x: a, y: bq }, r: cq };
                let c2 = c2Circle { p: c2v { x: cq, y: a }, r: bq };
                let bb = c2AABB { min: c2v { x: a, y: bq }, max: c2v { x: cq, y: a } };
                let k = c2Capsule { a: c2v { x: a, y: bq }, b: c2v { x: cq, y: bq }, r: a };
                unsafe {
                    if c_cc(c1, c2) != r_cc(c1, c2) {
                        rep.note(format!("c2CircletoCircle {c1:?} {c2:?}"));
                    }
                    if c_ca(c1, bb) != r_ca(c1, bb) {
                        rep.note(format!("c2CircletoAABB {c1:?} {bb:?}"));
                    }
                    if c_ck(c1, k) != r_ck(c1, k) {
                        rep.note(format!("c2CircletoCapsule {c1:?} {k:?}"));
                    }
                    if c_aa(bb, bb) != r_aa(bb, bb) {
                        rep.note(format!("c2AABBtoAABB {bb:?}"));
                    }
                }
            }
        }
    }

    type GjkFn = unsafe extern "C" fn(
        *const c_void,
        i32,
        *const c2x,
        *const c_void,
        i32,
        *const c2x,
        *mut c2v,
        *mut c2v,
        i32,
        *mut i32,
        *mut c2GJKCache,
    ) -> f32;
    let (cg, rg) = l.pair::<GjkFn>("c2GJK");

    for &a in &v {
        for &bq in &v {
            for &cq in &v {
                let circ = c2Circle { p: c2v { x: a, y: bq }, r: cq };
                let cap = c2Capsule { a: c2v { x: bq, y: a }, b: c2v { x: a, y: cq }, r: bq };
                let bb = c2AABB { min: c2v { x: -a, y: -bq }, max: c2v { x: cq, y: a } };
                let shapes: [(*const c_void, i32); 3] = [
                    (&circ as *const _ as *const c_void, C2_TYPE_CIRCLE),
                    (&cap as *const _ as *const c_void, C2_TYPE_CAPSULE),
                    (&bb as *const _ as *const c_void, C2_TYPE_AABB),
                ];
                for (pa, ta) in &shapes {
                    for (pb, tb) in &shapes {
                        for use_radius in [0, 1] {
                            let mut o1 = (c2v::default(), c2v::default());
                            let mut o2 = (c2v::default(), c2v::default());
                            let (mut i1, mut i2) = (-1i32, -1i32);
                            let (d1, d2) = unsafe {
                                (
                                    cg(
                                        *pa, *ta, std::ptr::null(), *pb, *tb, std::ptr::null(),
                                        &mut o1.0, &mut o1.1, use_radius, &mut i1,
                                        std::ptr::null_mut(),
                                    ),
                                    rg(
                                        *pa, *ta, std::ptr::null(), *pb, *tb, std::ptr::null(),
                                        &mut o2.0, &mut o2.1, use_radius, &mut i2,
                                        std::ptr::null_mut(),
                                    ),
                                )
                            };
                            if i1 >= 20 || i2 >= 20 {
                                rep.note(format!(
                                    "c2GJK hit iteration cap ta={ta} tb={tb} C_it={i1} R_it={i2}"
                                ));
                                continue;
                            }
                            if d1.to_bits() != d2.to_bits()
                                || raw(&o1) != raw(&o2)
                                || i1 != i2
                            {
                                rep.note(format!(
                                    "c2GJK ta={ta} tb={tb} r={use_radius} circ={circ:?} \
                                     cap={cap:?} bb={bb:?}\n     C dist={} out={o1:?} it={i1}\
                                     \n     R dist={} out={o2:?} it={i2}",
                                    b(d1),
                                    b(d2)
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    rep.finish("predicates and GJK");
}

#[test]
fn nan_payloads_capsule_entry() {
    let l = libs();
    let v = vals();
    let mut rep = Report::new();
    let (c, r) = l.pair::<unsafe extern "C" fn(f32, f32, f32, f32, f32) -> i32>("capsule");
    for &a in &v {
        for &bq in &v {
            for &cq in &v {
                for &d in &v {
                    for &e in &v {
                        let (x, y) = unsafe { (c(a, bq, cq, d, e), r(a, bq, cq, d, e)) };
                        if x != y {
                            rep.note(format!(
                                "capsule({},{},{},{},{}) C={x} R={y}",
                                b(a),
                                b(bq),
                                b(cq),
                                b(d),
                                b(e)
                            ));
                        }
                    }
                }
            }
        }
    }
    rep.finish("capsule entry point");
}
