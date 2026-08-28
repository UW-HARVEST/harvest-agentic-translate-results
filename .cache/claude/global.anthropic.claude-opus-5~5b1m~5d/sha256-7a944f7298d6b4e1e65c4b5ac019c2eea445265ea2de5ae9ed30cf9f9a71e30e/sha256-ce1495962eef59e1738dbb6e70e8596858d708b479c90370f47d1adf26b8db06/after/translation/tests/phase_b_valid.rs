//! Phase B — valid-path differential tests, one test (or test group) per row of
//! `CONFIGS.md`.
//!
//! Every call goes through `dlopen`/`dlsym` on BOTH shared objects; results are
//! compared **bit-for-bit** (`f32::to_bits`), never with an epsilon.

#![allow(non_snake_case)]

mod common;

use common::*;
use std::ffi::c_int;

const ITERS: usize = 4000;

// ===========================================================================
// Row 1 — c2V
// ===========================================================================

#[test]
fn row01_c2v() {
    let l = libs();
    let (c, r) = l.pair::<FnV2>("c2V");
    let mut rng = Rng::new(0x1001);
    for i in 0..ITERS {
        let (x, y) = (rng.wild(), rng.wild());
        let cv = unsafe { c(x, y) };
        let rv = unsafe { r(x, y) };
        assert!(veq(cv, rv), "iter {i}: c2V({}, {}) C={} R={}", fdesc(x), fdesc(y), vdesc(cv), vdesc(rv));
    }
}

// ===========================================================================
// Row 2 — c2Mulvs
// ===========================================================================

#[test]
fn row02_c2mulvs() {
    let l = libs();
    let (c, r) = l.pair::<FnVecScalar>("c2Mulvs");
    let mut rng = Rng::new(0x1002);
    for i in 0..ITERS {
        let a = rng.vec_wild();
        let b = rng.wild();
        let cv = unsafe { c(a, b) };
        let rv = unsafe { r(a, b) };
        assert!(veq(cv, rv), "iter {i}: c2Mulvs({}, {}) C={} R={}", vdesc(a), fdesc(b), vdesc(cv), vdesc(rv));
    }
}

// ===========================================================================
// Row 3 — c2Add / c2Sub
// ===========================================================================

#[test]
fn row03_c2add_c2sub() {
    let l = libs();
    let (ca, ra) = l.pair::<FnVecVec>("c2Add");
    let (cs, rs) = l.pair::<FnVecVec>("c2Sub");
    let mut rng = Rng::new(0x1003);
    for i in 0..ITERS {
        let a = rng.vec_wild();
        let b = if rng.below(8) == 0 { a } else { rng.vec_wild() };
        let (x, y) = (unsafe { ca(a, b) }, unsafe { ra(a, b) });
        assert!(veq(x, y), "iter {i}: c2Add({}, {}) C={} R={}", vdesc(a), vdesc(b), vdesc(x), vdesc(y));
        let (x, y) = (unsafe { cs(a, b) }, unsafe { rs(a, b) });
        assert!(veq(x, y), "iter {i}: c2Sub({}, {}) C={} R={}", vdesc(a), vdesc(b), vdesc(x), vdesc(y));
    }
}

// ===========================================================================
// Row 4 — c2Dot / c2Det2 (incl. cancellation cases)
// ===========================================================================

#[test]
fn row04_c2dot_c2det2() {
    let l = libs();
    let (cd, rd) = l.pair::<FnVecVecF>("c2Dot");
    let (ce, re) = l.pair::<FnVecVecF>("c2Det2");
    let mut rng = Rng::new(0x1004);
    for i in 0..ITERS {
        let a = rng.vec_wild();
        let b = match rng.below(6) {
            0 => a,                                        // dot = |a|^2, det = 0
            1 => c2v { x: -a.x, y: -a.y },                 // dot = -|a|^2, det = 0
            2 => c2v { x: -a.y, y: a.x },                  // dot = 0 (orthogonal)
            3 => c2v { x: a.y, y: -a.x },                  // dot = 0
            _ => rng.vec_wild(),
        };
        let (x, y) = (unsafe { cd(a, b) }, unsafe { rd(a, b) });
        assert!(feq(x, y), "iter {i}: c2Dot({}, {}) C={} R={}", vdesc(a), vdesc(b), fdesc(x), fdesc(y));
        let (x, y) = (unsafe { ce(a, b) }, unsafe { re(a, b) });
        assert!(feq(x, y), "iter {i}: c2Det2({}, {}) C={} R={}", vdesc(a), vdesc(b), fdesc(x), fdesc(y));
    }
}

// ===========================================================================
// Row 5 — c2Maxv / c2Minv (?: semantics, NaN and signed-zero asymmetry)
// ===========================================================================

#[test]
fn row05_c2maxv_c2minv() {
    let l = libs();
    let (cmax, rmax) = l.pair::<FnVecVec>("c2Maxv");
    let (cmin, rmin) = l.pair::<FnVecVec>("c2Minv");
    let mut rng = Rng::new(0x1005);

    // Hand-picked signed-zero / NaN pairs first.
    let picks: [(c2v, c2v); 8] = [
        (c2v { x: 0.0, y: -0.0 }, c2v { x: -0.0, y: 0.0 }),
        (c2v { x: -0.0, y: 0.0 }, c2v { x: 0.0, y: -0.0 }),
        (c2v { x: f32::NAN, y: 1.0 }, c2v { x: 2.0, y: f32::NAN }),
        (c2v { x: 2.0, y: f32::NAN }, c2v { x: f32::NAN, y: 1.0 }),
        (c2v { x: f32::NAN, y: f32::NAN }, c2v { x: f32::NAN, y: f32::NAN }),
        (c2v { x: f32::INFINITY, y: f32::NEG_INFINITY }, c2v { x: f32::NEG_INFINITY, y: f32::INFINITY }),
        (c2v { x: 1.0, y: 1.0 }, c2v { x: 1.0, y: 1.0 }),
        (c2v { x: -f32::NAN, y: 0.0 }, c2v { x: 0.0, y: -f32::NAN }),
    ];
    for (i, &(a, b)) in picks.iter().enumerate() {
        let (x, y) = (unsafe { cmax(a, b) }, unsafe { rmax(a, b) });
        assert!(veq(x, y), "pick {i}: c2Maxv({}, {}) C={} R={}", vdesc(a), vdesc(b), vdesc(x), vdesc(y));
        let (x, y) = (unsafe { cmin(a, b) }, unsafe { rmin(a, b) });
        assert!(veq(x, y), "pick {i}: c2Minv({}, {}) C={} R={}", vdesc(a), vdesc(b), vdesc(x), vdesc(y));
    }

    for i in 0..ITERS {
        let a = rng.vec_wild();
        let b = if rng.below(8) == 0 { a } else { rng.vec_wild() };
        let (x, y) = (unsafe { cmax(a, b) }, unsafe { rmax(a, b) });
        assert!(veq(x, y), "iter {i}: c2Maxv({}, {}) C={} R={}", vdesc(a), vdesc(b), vdesc(x), vdesc(y));
        let (x, y) = (unsafe { cmin(a, b) }, unsafe { rmin(a, b) });
        assert!(veq(x, y), "iter {i}: c2Minv({}, {}) C={} R={}", vdesc(a), vdesc(b), vdesc(x), vdesc(y));
    }
}

// ===========================================================================
// Row 6 — c2Clampv (inside / outside on every side, lo == hi, inverted box)
// ===========================================================================

#[test]
fn row06_c2clampv() {
    let l = libs();
    let (c, r) = l.pair::<FnVecVecVec>("c2Clampv");
    let mut rng = Rng::new(0x1006);
    for i in 0..ITERS {
        let lo = rng.vec_coord(64.0);
        let hi = match rng.below(4) {
            0 => lo,                                                             // lo == hi
            1 => c2v { x: lo.x - rng.range(0.0, 32.0), y: lo.y - rng.range(0.0, 32.0) }, // inverted
            _ => c2v { x: lo.x + rng.range(0.0, 64.0), y: lo.y + rng.range(0.0, 64.0) },
        };
        // Deliberately sample inside, left, right, above, below and wild.
        let a = match rng.below(7) {
            0 => c2v { x: lo.x - 8.0, y: lo.y - 8.0 },
            1 => c2v { x: hi.x + 8.0, y: hi.y + 8.0 },
            2 => c2v { x: lo.x, y: hi.y },
            3 => c2v { x: hi.x, y: lo.y },
            4 => rng.vec_wild(),
            _ => rng.vec_coord(64.0),
        };
        let (x, y) = (unsafe { c(a, lo, hi) }, unsafe { r(a, lo, hi) });
        assert!(
            veq(x, y),
            "iter {i}: c2Clampv({}, {}, {}) C={} R={}",
            vdesc(a), vdesc(lo), vdesc(hi), vdesc(x), vdesc(y)
        );
    }
}

// ===========================================================================
// Row 7 — c2Len
// ===========================================================================

#[test]
fn row07_c2len() {
    let l = libs();
    let (c, r) = l.pair::<FnVecF>("c2Len");
    let mut rng = Rng::new(0x1007);
    let picks = [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 3.0, y: 4.0 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::MIN_POSITIVE, y: 0.0 },
        c2v { x: f32::from_bits(1), y: f32::from_bits(1) },
        c2v { x: f32::NAN, y: 0.0 },
        c2v { x: -f32::NAN, y: 0.0 },
        c2v { x: f32::from_bits(0x7fc0_1234), y: 0.0 },
        c2v { x: f32::INFINITY, y: f32::NAN },
        c2v { x: f32::NEG_INFINITY, y: 0.0 },
    ];
    for (i, &a) in picks.iter().enumerate() {
        let (x, y) = (unsafe { c(a) }, unsafe { r(a) });
        assert!(feq(x, y), "pick {i}: c2Len({}) C={} R={}", vdesc(a), fdesc(x), fdesc(y));
    }
    for i in 0..ITERS {
        let a = rng.vec_wild();
        let (x, y) = (unsafe { c(a) }, unsafe { r(a) });
        assert!(feq(x, y), "iter {i}: c2Len({}) C={} R={}", vdesc(a), fdesc(x), fdesc(y));
    }
}

// ===========================================================================
// Row 8 — c2Div / c2Norm
// ===========================================================================

#[test]
fn row08_c2div_c2norm() {
    let l = libs();
    let (cd, rd) = l.pair::<FnVecScalar>("c2Div");
    let (cn, rn) = l.pair::<FnVec>("c2Norm");
    let mut rng = Rng::new(0x1008);
    for i in 0..ITERS {
        let a = rng.vec_wild();
        let b = rng.wild();
        let (x, y) = (unsafe { cd(a, b) }, unsafe { rd(a, b) });
        assert!(veq(x, y), "iter {i}: c2Div({}, {}) C={} R={}", vdesc(a), fdesc(b), vdesc(x), vdesc(y));
        let (x, y) = (unsafe { cn(a) }, unsafe { rn(a) });
        assert!(veq(x, y), "iter {i}: c2Norm({}) C={} R={}", vdesc(a), vdesc(x), vdesc(y));
    }
    // Unit-length and axis-aligned inputs.
    for &a in &[
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 0.0, y: -1.0 },
        c2v { x: 0.6, y: 0.8 },
        c2v { x: 1.0e-30, y: 1.0e-30 },
        c2v { x: 1.0e30, y: 1.0e30 },
    ] {
        let (x, y) = (unsafe { cn(a) }, unsafe { rn(a) });
        assert!(veq(x, y), "c2Norm({}) C={} R={}", vdesc(a), vdesc(x), vdesc(y));
    }
}

// ===========================================================================
// Row 9 — c2Neg / c2Skew / c2CCW90 (sign-bit propagation)
// ===========================================================================

#[test]
fn row09_c2neg_c2skew_c2ccw90() {
    let l = libs();
    let (cneg, rneg) = l.pair::<FnVec>("c2Neg");
    let (csk, rsk) = l.pair::<FnVec>("c2Skew");
    let (ccw, rcw) = l.pair::<FnVec>("c2CCW90");
    let mut rng = Rng::new(0x1009);
    let picks = [
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: f32::NAN, y: -f32::NAN },
        c2v { x: f32::from_bits(0x7fc0_1234), y: f32::from_bits(0xffc0_4321) },
        c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
    ];
    let mut all: Vec<c2v> = picks.to_vec();
    for _ in 0..ITERS {
        all.push(rng.vec_wild());
    }
    for (i, &a) in all.iter().enumerate() {
        let (x, y) = (unsafe { cneg(a) }, unsafe { rneg(a) });
        assert!(veq(x, y), "{i}: c2Neg({}) C={} R={}", vdesc(a), vdesc(x), vdesc(y));
        let (x, y) = (unsafe { csk(a) }, unsafe { rsk(a) });
        assert!(veq(x, y), "{i}: c2Skew({}) C={} R={}", vdesc(a), vdesc(x), vdesc(y));
        let (x, y) = (unsafe { ccw(a) }, unsafe { rcw(a) });
        assert!(veq(x, y), "{i}: c2CCW90({}) C={} R={}", vdesc(a), vdesc(x), vdesc(y));
    }
}

// ===========================================================================
// Row 10 — c2RotIdentity / c2xIdentity
// ===========================================================================

#[test]
fn row10_identities() {
    let l = libs();
    let (cr, rr) = l.pair::<FnRotIdentity>("c2RotIdentity");
    let (cx, rx) = l.pair::<FnXIdentity>("c2xIdentity");
    for _ in 0..64 {
        let (a, b) = (unsafe { cr() }, unsafe { rr() });
        assert!(req(a, b), "c2RotIdentity C=({},{}) R=({},{})", fdesc(a.c), fdesc(a.s), fdesc(b.c), fdesc(b.s));
        let (a, b) = (unsafe { cx() }, unsafe { rx() });
        assert!(xeq(a, b), "c2xIdentity C={a:?} R={b:?}");
    }
}

// ===========================================================================
// Rows 11-12 — c2Mulrv / c2MulrvT / c2Mulxv
// ===========================================================================

#[test]
fn row11_c2mulrv_c2mulrvt() {
    let l = libs();
    let (cf, rf) = l.pair::<FnMulrv>("c2Mulrv");
    let (ct, rt) = l.pair::<FnMulrv>("c2MulrvT");
    let mut rng = Rng::new(0x1011);
    for i in 0..ITERS {
        let rot = match rng.below(6) {
            0 => c2r { c: 1.0, s: 0.0 },
            1 => c2r { c: 0.0, s: 0.0 },
            2 => c2r { c: -1.0, s: -0.0 },
            3 => {
                let a = rng.range(-3.15, 3.15);
                c2r { c: a.cos(), s: a.sin() }
            }
            4 => c2r { c: rng.wild(), s: rng.wild() },
            _ => c2r { c: rng.range(-4.0, 4.0), s: rng.range(-4.0, 4.0) },
        };
        let v = rng.vec_wild();
        let (x, y) = (unsafe { cf(rot, v) }, unsafe { rf(rot, v) });
        assert!(
            veq(x, y),
            "iter {i}: c2Mulrv(({},{}), {}) C={} R={}",
            fdesc(rot.c), fdesc(rot.s), vdesc(v), vdesc(x), vdesc(y)
        );
        let (x, y) = (unsafe { ct(rot, v) }, unsafe { rt(rot, v) });
        assert!(
            veq(x, y),
            "iter {i}: c2MulrvT(({},{}), {}) C={} R={}",
            fdesc(rot.c), fdesc(rot.s), vdesc(v), vdesc(x), vdesc(y)
        );
        // Round-trip through both libraries independently.
        let cc = unsafe { ct(rot, cf(rot, v)) };
        let rr2 = unsafe { rt(rot, rf(rot, v)) };
        assert!(veq(cc, rr2), "iter {i}: round-trip C={} R={}", vdesc(cc), vdesc(rr2));
    }
}

#[test]
fn row12_c2mulxv() {
    let l = libs();
    let (c, r) = l.pair::<FnMulxv>("c2Mulxv");
    let mut rng = Rng::new(0x1012);
    for i in 0..ITERS {
        let scale = SCALES[rng.below(SCALES.len())];
        let tx = gen_x(&mut rng, scale);
        let v = if rng.below(4) == 0 { rng.vec_wild() } else { rng.vec_coord(scale) };
        let (x, y) = (unsafe { c(tx, v) }, unsafe { r(tx, v) });
        assert!(veq(x, y), "iter {i}: c2Mulxv({tx:?}, {}) C={} R={}", vdesc(v), vdesc(x), vdesc(y));
    }
}

// ===========================================================================
// Row 13 — c2BBVerts
// ===========================================================================

#[test]
fn row13_c2bbverts() {
    let l = libs();
    let (c, r) = l.pair::<FnBBVerts>("c2BBVerts");
    let mut rng = Rng::new(0x1013);
    for i in 0..ITERS {
        let scale = SCALES[rng.below(SCALES.len())];
        let bb = if rng.below(6) == 0 {
            c2AABB { min: rng.vec_wild(), max: rng.vec_wild() }
        } else {
            gen_aabb(&mut rng, scale)
        };
        // Pre-fill both output buffers with the same garbage so that a missing
        // write is detected.
        let sentinel = c2v { x: -1.234e9, y: 5.678e9 };
        let mut co = [sentinel; 6];
        let mut ro = [sentinel; 6];
        let mut bb_c = bb;
        let mut bb_r = bb;
        unsafe { c(co.as_mut_ptr(), &mut bb_c) };
        unsafe { r(ro.as_mut_ptr(), &mut bb_r) };
        for k in 0..6 {
            assert!(
                veq(co[k], ro[k]),
                "iter {i}: c2BBVerts out[{k}] for {bb:?}: C={} R={}",
                vdesc(co[k]), vdesc(ro[k])
            );
        }
        // The input AABB must not be modified by either implementation.
        assert!(veq(bb_c.min, bb_r.min) && veq(bb_c.max, bb_r.max));
    }
}

// ===========================================================================
// Rows 14-17 — c2MakeProxy for each valid type, clean and pre-dirtied output
// ===========================================================================

fn make_proxy_row(seed: u64, ty: c_int, dirty: bool) {
    let l = libs();
    let (c, r) = l.pair::<FnMakeProxy>("c2MakeProxy");
    let mut rng = Rng::new(seed);
    for i in 0..ITERS {
        let scale = SCALES[rng.below(SCALES.len())];
        let shape = gen_shape(&mut rng, ty, scale);
        let mut cp = c2Proxy::default();
        let mut rp = c2Proxy::default();
        if dirty {
            // Identical non-zero garbage in both output buffers.
            cp.radius = -7.5;
            cp.count = 12345;
            for k in 0..8 {
                cp.verts[k] = c2v { x: (k as f32) * 3.5 - 9.0, y: -(k as f32) * 2.25 + 4.0 };
            }
            rp = cp;
        }
        unsafe { c(shape.as_ptr(), ty, &mut cp) };
        unsafe { r(shape.as_ptr(), ty, &mut rp) };
        assert!(
            proxy_eq(&cp, &rp),
            "iter {i}: c2MakeProxy(ty={ty}, dirty={dirty}) {shape:?}\nC:\n{}\nR:\n{}",
            proxy_desc(&cp), proxy_desc(&rp)
        );
    }
}

#[test]
fn row14_makeproxy_circle() {
    make_proxy_row(0x1014, C2_TYPE_CIRCLE, false);
}

#[test]
fn row15_makeproxy_aabb() {
    make_proxy_row(0x1015, C2_TYPE_AABB, false);
}

#[test]
fn row16_makeproxy_capsule() {
    make_proxy_row(0x1016, C2_TYPE_CAPSULE, false);
}

#[test]
fn row17_makeproxy_dirty_output() {
    make_proxy_row(0x1017, C2_TYPE_CIRCLE, true);
    make_proxy_row(0x1018, C2_TYPE_AABB, true);
    make_proxy_row(0x1019, C2_TYPE_CAPSULE, true);
}

// ===========================================================================
// Row 18 — c2GJKSimplexMetric for count 1/2/3
// ===========================================================================

#[test]
fn row18_gjk_simplex_metric() {
    let l = libs();
    let (c, r) = l.pair::<FnSimplexF>("c2GJKSimplexMetric");
    let mut rng = Rng::new(0x1020);
    for count in 1..=3 {
        for i in 0..ITERS {
            let scale = SCALES[rng.below(SCALES.len())];
            let wild = rng.below(6) == 0;
            let s = gen_simplex(&mut rng, count, scale, wild);
            let mut cs = s;
            let mut rs = s;
            let (x, y) = (unsafe { c(&mut cs) }, unsafe { r(&mut rs) });
            assert!(
                feq(x, y),
                "count={count} iter {i}: c2GJKSimplexMetric C={} R={}\n{}",
                fdesc(x), fdesc(y), simplex_desc(&s)
            );
            // The function must not mutate the simplex.
            assert!(simplex_eq(&cs, &rs), "count={count} iter {i}: simplex mutated differently");
        }
    }
}

// ===========================================================================
// Helpers for the simplex-shape rows
// ===========================================================================

/// Tag each vertex with `iA = iB = index` so the arm taken by `c22`/`c23` can be
/// identified from the (re-arranged) output simplex.
fn tag(s: &mut c2Simplex) {
    for i in 0..4 {
        s.verts[i].iA = i as c_int;
        s.verts[i].iB = i as c_int;
    }
}

/// Random point positions in `p` for a `count`-vertex simplex; `sA`/`sB` are
/// randomized independently so `c2Witness` also sees interesting data.
fn simplex_points(rng: &mut Rng, pts: &[c2v]) -> c2Simplex {
    let mut s = c2Simplex::default();
    for (i, &p) in pts.iter().enumerate() {
        s.verts[i].p = p;
        s.verts[i].sA = rng.vec_coord(64.0);
        s.verts[i].sB = rng.vec_coord(64.0);
        s.verts[i].u = rng.range(-2.0, 2.0);
    }
    s.div = 1.0;
    s.count = pts.len() as c_int;
    tag(&mut s);
    s
}

/// Run `c22`/`c23` through both `.so`s and return the arm signature taken
/// (derived from the C result): `(count, iA of verts[0], iA of verts[1])`.
fn run_simplex_fn(name: &str, s: &c2Simplex) -> (c_int, c_int, c_int) {
    let l = libs();
    let (c, r) = l.pair::<FnSimplexVoid>(name);
    let mut cs = *s;
    let mut rs = *s;
    unsafe { c(&mut cs) };
    unsafe { r(&mut rs) };
    assert!(
        simplex_eq(&cs, &rs),
        "{name} diverged\nINPUT:\n{}\nC:\n{}\nRUST:\n{}",
        simplex_desc(s),
        simplex_desc(&cs),
        simplex_desc(&rs)
    );
    (cs.count, cs.verts[0].iA, cs.verts[1].iA)
}

// ===========================================================================
// Rows 19-21 — c22 (all 3 arms)
// ===========================================================================

#[test]
fn row19_c22_random() {
    let mut rng = Rng::new(0x1030);
    let mut arms = std::collections::BTreeSet::new();
    for _ in 0..ITERS {
        let scale = SCALES[rng.below(SCALES.len())];
        let pts = [rng.vec_coord(scale), rng.vec_coord(scale)];
        let s = simplex_points(&mut rng, &pts);
        arms.insert(run_simplex_fn("c22", &s));
    }
    // Arms: (1,0,_) keep a, (1,1,_) take b, (2,0,1) edge.
    assert!(arms.len() >= 3, "c22 arms covered: {arms:?}");
}

#[test]
fn row20_c22_edge_region() {
    // Origin strictly inside the segment: pick a random direction and place
    // `a` and `b` on opposite sides of the foot of the perpendicular.
    let mut rng = Rng::new(0x1031);
    let mut edge_hits = 0usize;
    for _ in 0..ITERS {
        let ang = rng.range(-3.15, 3.15);
        let (dx, dy) = (ang.cos(), ang.sin());
        let off = rng.range(-4.0, 4.0);
        let (nx, ny) = (-dy * off, dx * off); // perpendicular offset
        let ta = -rng.range(0.25, 40.0);
        let tb = rng.range(0.25, 40.0);
        let pts = [
            c2v { x: nx + dx * ta, y: ny + dy * ta },
            c2v { x: nx + dx * tb, y: ny + dy * tb },
        ];
        let s = simplex_points(&mut rng, &pts);
        let (count, _, _) = run_simplex_fn("c22", &s);
        if count == 2 {
            edge_hits += 1;
        }
    }
    assert!(edge_hits > ITERS / 2, "edge arm hit {edge_hits}/{ITERS}");
}

#[test]
fn row21_c22_vertex_regions() {
    // Both points on the same side of the origin along a random direction:
    // the origin then projects outside the segment, exercising the `v <= 0`
    // and `u <= 0` collapse arms depending on which end is nearer.
    let mut rng = Rng::new(0x1032);
    let mut keep_a = 0usize;
    let mut take_b = 0usize;
    for _ in 0..ITERS {
        let ang = rng.range(-3.15, 3.15);
        let (dx, dy) = (ang.cos(), ang.sin());
        let t0 = rng.range(1.0, 40.0);
        let t1 = t0 + rng.range(0.25, 40.0);
        let (near, far) = if rng.boolean() { (t0, t1) } else { (t1, t0) };
        let pts = [
            c2v { x: dx * near, y: dy * near },
            c2v { x: dx * far, y: dy * far },
        ];
        let s = simplex_points(&mut rng, &pts);
        let (count, i0, _) = run_simplex_fn("c22", &s);
        if count == 1 && i0 == 0 {
            keep_a += 1;
        }
        if count == 1 && i0 == 1 {
            take_b += 1;
        }
    }
    assert!(keep_a > 0 && take_b > 0, "keep_a={keep_a} take_b={take_b}");
}

#[test]
fn row21b_c22_wild_values() {
    // Same entry point, but with non-finite / extreme `p` values so the
    // comparison arms are driven by NaN/inf too.
    let mut rng = Rng::new(0x1033);
    for _ in 0..ITERS {
        let pts = [rng.vec_wild(), rng.vec_wild()];
        let mut s = simplex_points(&mut rng, &pts);
        s.div = rng.wild();
        run_simplex_fn("c22", &s);
    }
}

// ===========================================================================
// Rows 22-26 — c23 (all 7 arms)
// ===========================================================================

fn c23_arm_sweep(seed: u64, genfn: impl Fn(&mut Rng) -> [c2v; 3], iters: usize) -> std::collections::BTreeMap<(c_int, c_int, c_int), usize> {
    let mut rng = Rng::new(seed);
    let mut arms = std::collections::BTreeMap::new();
    for _ in 0..iters {
        let pts = genfn(&mut rng);
        let s = simplex_points(&mut rng, &pts);
        *arms.entry(run_simplex_fn("c23", &s)).or_insert(0usize) += 1;
    }
    arms
}

#[test]
fn row22_c23_random() {
    let arms = c23_arm_sweep(
        0x1040,
        |rng| {
            let scale = SCALES[rng.below(SCALES.len())];
            [rng.vec_coord(scale), rng.vec_coord(scale), rng.vec_coord(scale)]
        },
        ITERS,
    );
    assert!(arms.len() >= 4, "c23 arms from uniform triangles: {arms:?}");
}

#[test]
fn row23_c23_origin_inside() {
    // Triangle spanning the origin: three radii at ~120 degrees apart.
    let arms = c23_arm_sweep(
        0x1041,
        |rng| {
            let base = rng.range(-3.15, 3.15);
            let step = 2.0 * std::f32::consts::PI / 3.0;
            let mut out = [c2v::default(); 3];
            for k in 0..3 {
                let a = base + step * k as f32 + rng.range(-0.4, 0.4);
                let r = rng.range(1.0, 60.0);
                out[k] = c2v { x: a.cos() * r, y: a.sin() * r };
            }
            out
        },
        ITERS,
    );
    let interior = arms.iter().filter(|(k, _)| k.0 == 3).map(|(_, v)| *v).sum::<usize>();
    assert!(interior > ITERS / 8, "interior arm hit {interior}/{ITERS}: {arms:?}");
}

#[test]
fn row24_c23_vertex_regions() {
    // Small cluster far from the origin => the closest feature is a vertex.
    let arms = c23_arm_sweep(
        0x1042,
        |rng| {
            let ang = rng.range(-3.15, 3.15);
            let dist = rng.range(20.0, 400.0);
            let cx = ang.cos() * dist;
            let cy = ang.sin() * dist;
            let mut out = [c2v::default(); 3];
            for k in 0..3 {
                out[k] = c2v {
                    x: cx + rng.range(-3.0, 3.0),
                    y: cy + rng.range(-3.0, 3.0),
                };
            }
            out
        },
        ITERS,
    );
    let v0 = arms.iter().filter(|(k, _)| k.0 == 1 && k.1 == 0).count();
    let v1 = arms.iter().filter(|(k, _)| k.0 == 1 && k.1 == 1).count();
    let v2 = arms.iter().filter(|(k, _)| k.0 == 1 && k.1 == 2).count();
    assert!(v0 > 0 && v1 > 0 && v2 > 0, "vertex arms a/b/c: {v0}/{v1}/{v2}: {arms:?}");
}

#[test]
fn row25_c23_edge_regions() {
    // Two points straddling the origin ray, third far to one side => closest
    // feature is an edge. Rotating which pair straddles hits all three edges.
    let arms = c23_arm_sweep(
        0x1043,
        |rng| {
            let ang = rng.range(-3.15, 3.15);
            let (dx, dy) = (ang.cos(), ang.sin());
            let (px, py) = (-dy, dx);
            let d = rng.range(4.0, 60.0);
            let w = rng.range(4.0, 60.0);
            let far = rng.range(80.0, 400.0);
            let mut out = [
                c2v { x: dx * d + px * w, y: dy * d + py * w },
                c2v { x: dx * d - px * w, y: dy * d - py * w },
                c2v { x: dx * far, y: dy * far },
            ];
            // Rotate the roles so every edge (ab, bc, ca) becomes the closest.
            let rot = rng.below(3);
            out.rotate_left(rot);
            out
        },
        ITERS,
    );
    let e: Vec<_> = arms.keys().filter(|k| k.0 == 2).collect();
    assert!(e.len() >= 3, "edge arms covered: {e:?} (all: {arms:?})");
}

#[test]
fn row26_c23_winding_and_degenerate() {
    let mut rng = Rng::new(0x1044);
    let mut arms = std::collections::BTreeSet::new();
    for _ in 0..ITERS {
        let scale = SCALES[rng.below(SCALES.len())];
        let a = rng.vec_coord(scale);
        let b = rng.vec_coord(scale);
        let c = match rng.below(5) {
            0 => a,                                             // repeated vertex
            1 => b,                                             // repeated vertex
            2 => c2v { x: a.x + (b.x - a.x) * 2.0, y: a.y + (b.y - a.y) * 2.0 }, // collinear
            _ => rng.vec_coord(scale),
        };
        // Both windings of the same point set.
        for pts in [[a, b, c], [a, c, b]] {
            let s = simplex_points(&mut rng, &pts);
            arms.insert(run_simplex_fn("c23", &s));
        }
    }
    assert!(!arms.is_empty());
}

#[test]
fn row26b_c23_wild_values() {
    let mut rng = Rng::new(0x1045);
    for _ in 0..ITERS {
        let pts = [rng.vec_wild(), rng.vec_wild(), rng.vec_wild()];
        let mut s = simplex_points(&mut rng, &pts);
        s.div = rng.wild();
        run_simplex_fn("c23", &s);
    }
}

// ===========================================================================
// Row 27 — c2D
// ===========================================================================

#[test]
fn row27_c2d() {
    let l = libs();
    let (c, r) = l.pair::<FnSimplexV>("c2D");
    let mut rng = Rng::new(0x1050);
    let mut seen_skew = 0usize;
    let mut seen_ccw = 0usize;
    for count in [1, 2, 3] {
        for i in 0..ITERS {
            let scale = SCALES[rng.below(SCALES.len())];
            let wild = rng.below(6) == 0;
            let mut s = gen_simplex(&mut rng, count, scale, wild);
            if !wild {
                for k in 0..3 {
                    s.verts[k].p = rng.vec_coord(scale);
                }
            }
            let mut cs = s;
            let mut rs = s;
            let (x, y) = (unsafe { c(&mut cs) }, unsafe { r(&mut rs) });
            assert!(
                veq(x, y),
                "count={count} iter {i}: c2D C={} R={}\n{}",
                vdesc(x), vdesc(y), simplex_desc(&s)
            );
            if count == 2 {
                // c2Skew(ab) = (-ab.y, ab.x); c2CCW90(ab) = (ab.y, -ab.x).
                let abx = s.verts[1].p.x - s.verts[0].p.x;
                let aby = s.verts[1].p.y - s.verts[0].p.y;
                if feq(x.x, -aby) && feq(x.y, abx) {
                    seen_skew += 1;
                } else if feq(x.x, aby) && feq(x.y, -abx) {
                    seen_ccw += 1;
                }
            }
        }
    }
    assert!(seen_skew > 0 && seen_ccw > 0, "skew={seen_skew} ccw={seen_ccw}");
}

// ===========================================================================
// Row 28 — c2L
// ===========================================================================

#[test]
fn row28_c2l() {
    let l = libs();
    let (c, r) = l.pair::<FnSimplexV>("c2L");
    let mut rng = Rng::new(0x1051);
    for count in [1, 2, 3] {
        for i in 0..ITERS {
            let scale = SCALES[rng.below(SCALES.len())];
            let wild = rng.below(4) == 0;
            let s = gen_simplex(&mut rng, count, scale, wild);
            let mut cs = s;
            let mut rs = s;
            let (x, y) = (unsafe { c(&mut cs) }, unsafe { r(&mut rs) });
            assert!(
                veq(x, y),
                "count={count} iter {i}: c2L C={} R={}\n{}",
                vdesc(x), vdesc(y), simplex_desc(&s)
            );
        }
    }
}

// ===========================================================================
// Row 29 — c2Witness
// ===========================================================================

#[test]
fn row29_c2witness() {
    let l = libs();
    let (c, r) = l.pair::<FnWitness>("c2Witness");
    let mut rng = Rng::new(0x1052);
    for count in [1, 2, 3] {
        for i in 0..ITERS {
            let scale = SCALES[rng.below(SCALES.len())];
            let wild = rng.below(4) == 0;
            let s = gen_simplex(&mut rng, count, scale, wild);
            let mut cs = s;
            let mut rs = s;
            let sentinel = c2v { x: 1.5e9, y: -2.5e9 };
            let (mut ca, mut cb) = (sentinel, sentinel);
            let (mut ra, mut rb) = (sentinel, sentinel);
            unsafe { c(&mut cs, &mut ca, &mut cb) };
            unsafe { r(&mut rs, &mut ra, &mut rb) };
            assert!(
                veq(ca, ra) && veq(cb, rb),
                "count={count} iter {i}: c2Witness C=({}, {}) R=({}, {})\n{}",
                vdesc(ca), vdesc(cb), vdesc(ra), vdesc(rb), simplex_desc(&s)
            );
        }
    }
}

// ===========================================================================
// Rows 30-33 — c2Support at every proxy width
// ===========================================================================

fn support_row(seed: u64, count: c_int, len: usize) {
    let l = libs();
    let (c, r) = l.pair::<FnSupport>("c2Support");
    let mut rng = Rng::new(seed);
    for i in 0..ITERS {
        let scale = SCALES[rng.below(SCALES.len())];
        let mut verts = vec![c2v::default(); len];
        for k in 0..len {
            verts[k] = match rng.below(8) {
                0 => rng.vec_wild(),
                1 if k > 0 => verts[k - 1], // exact duplicate => tie
                _ => rng.vec_coord(scale),
            };
        }
        let d = match rng.below(6) {
            0 => c2v { x: 0.0, y: 0.0 },
            1 => rng.vec_wild(),
            _ => rng.vec_coord(scale),
        };
        let cv = unsafe { c(verts.as_ptr(), count, d) };
        let rv = unsafe { r(verts.as_ptr(), count, d) };
        assert_eq!(cv, rv, "iter {i}: c2Support(count={count}, d={}) verts={verts:?}", vdesc(d));
    }
}

#[test]
fn row30_support_count1() {
    support_row(0x1060, 1, 8);
}

#[test]
fn row31_support_count2() {
    support_row(0x1061, 2, 8);
}

#[test]
fn row32_support_count4() {
    support_row(0x1062, 4, 8);
}

#[test]
fn row33_support_other_counts() {
    support_row(0x1063, 3, 8);
    support_row(0x1064, 5, 8);
    support_row(0x1065, 8, 8);
    support_row(0x1066, 16, 16);
}

// ===========================================================================
// Shape generators that actually make the two shapes interact
// ===========================================================================

/// Quantised jitter around `c` with step `size/8` so that exact touching,
/// exact containment and exact coincidence occur frequently.
fn jitter(rng: &mut Rng, c: c2v, size: f32) -> c2v {
    let q = |rng: &mut Rng| ((rng.next_u32() % 33) as i32 - 16) as f32 * size / 8.0;
    c2v { x: c.x + q(rng), y: c.y + q(rng) }
}

fn radius_pick(rng: &mut Rng, size: f32) -> f32 {
    match rng.below(6) {
        0 => 0.0,
        1 => size,
        2 => size / 2.0,
        3 => -size / 2.0, // never validated by the C
        _ => ((rng.next_u32() % 17) as f32) * size / 8.0,
    }
}

fn shape_at(rng: &mut Rng, ty: c_int, c: c2v, size: f32) -> Shape {
    match ty {
        C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
            p: jitter(rng, c, size),
            r: radius_pick(rng, size),
        }),
        C2_TYPE_AABB => {
            let a = jitter(rng, c, size);
            match rng.below(6) {
                0 => Shape::Aabb(c2AABB { min: a, max: a }),
                1 => Shape::Aabb(c2AABB {
                    min: c2v { x: a.x + size, y: a.y + size },
                    max: a,
                }),
                _ => {
                    let w = ((rng.next_u32() % 17) as f32) * size / 8.0;
                    let h = ((rng.next_u32() % 17) as f32) * size / 8.0;
                    Shape::Aabb(c2AABB { min: a, max: c2v { x: a.x + w, y: a.y + h } })
                }
            }
        }
        _ => {
            let a = jitter(rng, c, size);
            let b = match rng.below(6) {
                0 => a,
                1 => c2v { x: a.x, y: a.y + size },
                2 => c2v { x: a.x + size, y: a.y },
                _ => jitter(rng, c, size),
            };
            Shape::Capsule(c2Capsule { a, b, r: radius_pick(rng, size) })
        }
    }
}

/// A pair of shapes placed close enough together that separation, touching,
/// shallow overlap, deep overlap and containment all occur.
fn shape_pair(rng: &mut Rng, tyA: c_int, tyB: c_int, size: f32) -> (Shape, Shape) {
    let a = shape_at(rng, tyA, c2v { x: 0.0, y: 0.0 }, size);
    let off = jitter(rng, c2v { x: 0.0, y: 0.0 }, size * 2.0);
    let b = shape_at(rng, tyB, off, size);
    (a, b)
}

// ===========================================================================
// Rows 34-39 — the direct shape-vs-shape predicates
// ===========================================================================

#[test]
fn row34_aabb_to_aabb() {
    let l = libs();
    let (c, r) = l.pair::<FnAABBtoAABB>("c2AABBtoAABB");
    let mut rng = Rng::new(0x1070);
    let mut hits = 0usize;
    for i in 0..ITERS * 4 {
        let size = SCALES[rng.below(SCALES.len())];
        let (a, b) = shape_pair(&mut rng, C2_TYPE_AABB, C2_TYPE_AABB, size);
        let (Shape::Aabb(a), Shape::Aabb(b)) = (a, b) else { unreachable!() };
        let (x, y) = (unsafe { c(a, b) }, unsafe { r(a, b) });
        assert_eq!(x, y, "iter {i}: c2AABBtoAABB({a:?}, {b:?})");
        hits += (x != 0) as usize;
        // Also with wild (NaN/inf) coordinates.
        let wa = c2AABB { min: rng.vec_wild(), max: rng.vec_wild() };
        let wb = c2AABB { min: rng.vec_wild(), max: rng.vec_wild() };
        let (x, y) = (unsafe { c(wa, wb) }, unsafe { r(wa, wb) });
        assert_eq!(x, y, "iter {i}: c2AABBtoAABB({wa:?}, {wb:?})");
    }
    assert!(hits > 0 && hits < ITERS * 4, "both outcomes must occur: {hits}");
}

#[test]
fn row35_circle_to_circle() {
    let l = libs();
    let (c, r) = l.pair::<FnCircletoCircle>("c2CircletoCircle");
    let mut rng = Rng::new(0x1071);
    let mut hits = 0usize;
    for i in 0..ITERS * 4 {
        let size = SCALES[rng.below(SCALES.len())];
        let (a, b) = shape_pair(&mut rng, C2_TYPE_CIRCLE, C2_TYPE_CIRCLE, size);
        let (Shape::Circle(a), Shape::Circle(b)) = (a, b) else { unreachable!() };
        let (x, y) = (unsafe { c(a, b) }, unsafe { r(a, b) });
        assert_eq!(x, y, "iter {i}: c2CircletoCircle({a:?}, {b:?})");
        hits += (x != 0) as usize;
        let wa = c2Circle { p: rng.vec_wild(), r: rng.wild() };
        let wb = c2Circle { p: rng.vec_wild(), r: rng.wild() };
        let (x, y) = (unsafe { c(wa, wb) }, unsafe { r(wa, wb) });
        assert_eq!(x, y, "iter {i}: c2CircletoCircle({wa:?}, {wb:?})");
    }
    assert!(hits > 0 && hits < ITERS * 4, "both outcomes must occur: {hits}");
}

#[test]
fn row36_circle_to_aabb() {
    let l = libs();
    let (c, r) = l.pair::<FnCircletoAABB>("c2CircletoAABB");
    let mut rng = Rng::new(0x1072);
    let mut hits = 0usize;
    for i in 0..ITERS * 4 {
        let size = SCALES[rng.below(SCALES.len())];
        let (a, b) = shape_pair(&mut rng, C2_TYPE_CIRCLE, C2_TYPE_AABB, size);
        let (Shape::Circle(a), Shape::Aabb(b)) = (a, b) else { unreachable!() };
        let (x, y) = (unsafe { c(a, b) }, unsafe { r(a, b) });
        assert_eq!(x, y, "iter {i}: c2CircletoAABB({a:?}, {b:?})");
        hits += (x != 0) as usize;
        let wa = c2Circle { p: rng.vec_wild(), r: rng.wild() };
        let wb = c2AABB { min: rng.vec_wild(), max: rng.vec_wild() };
        let (x, y) = (unsafe { c(wa, wb) }, unsafe { r(wa, wb) });
        assert_eq!(x, y, "iter {i}: c2CircletoAABB({wa:?}, {wb:?})");
    }
    assert!(hits > 0 && hits < ITERS * 4, "both outcomes must occur: {hits}");
}

#[test]
fn row37_circle_to_capsule() {
    let l = libs();
    let (c, r) = l.pair::<FnCircletoCapsule>("c2CircletoCapsule");
    let mut rng = Rng::new(0x1073);
    let mut hits = 0usize;
    for i in 0..ITERS * 4 {
        let size = SCALES[rng.below(SCALES.len())];
        let (a, b) = shape_pair(&mut rng, C2_TYPE_CIRCLE, C2_TYPE_CAPSULE, size);
        let (Shape::Circle(a), Shape::Capsule(b)) = (a, b) else { unreachable!() };
        let (x, y) = (unsafe { c(a, b) }, unsafe { r(a, b) });
        assert_eq!(x, y, "iter {i}: c2CircletoCapsule({a:?}, {b:?})");
        hits += (x != 0) as usize;
        let wa = c2Circle { p: rng.vec_wild(), r: rng.wild() };
        let wb = c2Capsule { a: rng.vec_wild(), b: rng.vec_wild(), r: rng.wild() };
        let (x, y) = (unsafe { c(wa, wb) }, unsafe { r(wa, wb) });
        assert_eq!(x, y, "iter {i}: c2CircletoCapsule({wa:?}, {wb:?})");
    }
    assert!(hits > 0 && hits < ITERS * 4, "both outcomes must occur: {hits}");
}

#[test]
fn row38_aabb_to_capsule() {
    let l = libs();
    let (c, r) = l.pair::<FnAABBtoCapsule>("c2AABBtoCapsule");
    let mut rng = Rng::new(0x1074);
    let mut hits = 0usize;
    for i in 0..ITERS * 2 {
        let size = SCALES[rng.below(SCALES.len())];
        let (a, b) = shape_pair(&mut rng, C2_TYPE_AABB, C2_TYPE_CAPSULE, size);
        let (Shape::Aabb(a), Shape::Capsule(b)) = (a, b) else { unreachable!() };
        let (x, y) = (unsafe { c(a, b) }, unsafe { r(a, b) });
        assert_eq!(x, y, "iter {i}: c2AABBtoCapsule({a:?}, {b:?})");
        hits += (x != 0) as usize;
    }
    assert!(hits > 0 && hits < ITERS * 2, "both outcomes must occur: {hits}");
}

#[test]
fn row39_capsule_to_capsule() {
    let l = libs();
    let (c, r) = l.pair::<FnCapsuletoCapsule>("c2CapsuletoCapsule");
    let mut rng = Rng::new(0x1075);
    let mut hits = 0usize;
    for i in 0..ITERS * 2 {
        let size = SCALES[rng.below(SCALES.len())];
        let (a, b) = shape_pair(&mut rng, C2_TYPE_CAPSULE, C2_TYPE_CAPSULE, size);
        let (Shape::Capsule(a), Shape::Capsule(b)) = (a, b) else { unreachable!() };
        let (x, y) = (unsafe { c(a, b) }, unsafe { r(a, b) });
        assert_eq!(x, y, "iter {i}: c2CapsuletoCapsule({a:?}, {b:?})");
        hits += (x != 0) as usize;
    }
    assert!(hits > 0 && hits < ITERS * 2, "both outcomes must occur: {hits}");
}

// ===========================================================================
// Rows 40-48 — c2Collided over the full 3x3 type matrix
// ===========================================================================

fn collided_row(seed: u64, tyA: c_int, tyB: c_int) {
    let l = libs();
    let (c, r) = l.pair::<FnCollided>("c2Collided");
    let mut rng = Rng::new(seed);
    let mut hits = 0usize;
    let n = ITERS * 2;
    for i in 0..n {
        let size = SCALES[rng.below(SCALES.len())];
        let (a, b) = shape_pair(&mut rng, tyA, tyB, size);
        let x = unsafe { c(a.as_ptr(), tyA, b.as_ptr(), tyB) };
        let y = unsafe { r(a.as_ptr(), tyA, b.as_ptr(), tyB) };
        assert_eq!(x, y, "iter {i}: c2Collided({tyA}, {tyB}) A={a:?} B={b:?}");
        hits += (x != 0) as usize;
    }
    assert!(hits > 0 && hits < n, "({tyA},{tyB}) both outcomes must occur: {hits}/{n}");
}

#[test]
fn row40_collided_circle_circle() { collided_row(0x1080, C2_TYPE_CIRCLE, C2_TYPE_CIRCLE); }
#[test]
fn row41_collided_circle_aabb() { collided_row(0x1081, C2_TYPE_CIRCLE, C2_TYPE_AABB); }
#[test]
fn row42_collided_circle_capsule() { collided_row(0x1082, C2_TYPE_CIRCLE, C2_TYPE_CAPSULE); }
#[test]
fn row43_collided_aabb_circle() { collided_row(0x1083, C2_TYPE_AABB, C2_TYPE_CIRCLE); }
#[test]
fn row44_collided_aabb_aabb() { collided_row(0x1084, C2_TYPE_AABB, C2_TYPE_AABB); }
#[test]
fn row45_collided_aabb_capsule() { collided_row(0x1085, C2_TYPE_AABB, C2_TYPE_CAPSULE); }
#[test]
fn row46_collided_capsule_circle() { collided_row(0x1086, C2_TYPE_CAPSULE, C2_TYPE_CIRCLE); }
#[test]
fn row47_collided_capsule_aabb() { collided_row(0x1087, C2_TYPE_CAPSULE, C2_TYPE_AABB); }
#[test]
fn row48_collided_capsule_capsule() { collided_row(0x1088, C2_TYPE_CAPSULE, C2_TYPE_CAPSULE); }

// ===========================================================================
// c2GJK differential driver (rows 49-63)
// ===========================================================================

#[derive(Clone, Copy, Debug)]
pub struct GjkOpts {
    pub ax: Option<c2x>,
    pub bx: Option<c2x>,
    pub use_radius: c_int,
    pub want_out_a: bool,
    pub want_out_b: bool,
    pub want_iters: bool,
}

impl Default for GjkOpts {
    fn default() -> Self {
        GjkOpts {
            ax: None,
            bx: None,
            use_radius: 1,
            want_out_a: true,
            want_out_b: true,
            want_iters: true,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct GjkOut {
    pub dist: f32,
    pub a: c2v,
    pub b: c2v,
    pub iters: c_int,
    pub cache: Option<c2GJKCache>,
}

const SENTINEL_V: c2v = c2v { x: -1.111e9, y: 2.222e9 };
const SENTINEL_I: c_int = -999_999;

/// Invoke `c2GJK` through one `.so`.
unsafe fn call_gjk(
    f: FnGJK,
    a: &Shape,
    b: &Shape,
    o: &GjkOpts,
    cache: Option<c2GJKCache>,
) -> GjkOut {
    let mut out_a = SENTINEL_V;
    let mut out_b = SENTINEL_V;
    let mut iters = SENTINEL_I;
    let mut cache_buf = cache.unwrap_or_default();
    let ax = o.ax;
    let bx = o.bx;
    let ax_ptr = match &ax {
        Some(v) => v as *const c2x,
        None => std::ptr::null(),
    };
    let bx_ptr = match &bx {
        Some(v) => v as *const c2x,
        None => std::ptr::null(),
    };
    let dist = unsafe {
        f(
            a.as_ptr(),
            a.ty(),
            ax_ptr,
            b.as_ptr(),
            b.ty(),
            bx_ptr,
            if o.want_out_a { &mut out_a } else { std::ptr::null_mut() },
            if o.want_out_b { &mut out_b } else { std::ptr::null_mut() },
            o.use_radius,
            if o.want_iters { &mut iters } else { std::ptr::null_mut() },
            if cache.is_some() { &mut cache_buf } else { std::ptr::null_mut() },
        )
    };
    GjkOut {
        dist,
        a: out_a,
        b: out_b,
        iters,
        cache: cache.map(|_| cache_buf),
    }
}

fn assert_gjk_eq(ctx: &str, a: &Shape, b: &Shape, o: &GjkOpts, cv: &GjkOut, rv: &GjkOut) {
    let ok = feq(cv.dist, rv.dist)
        && veq(cv.a, rv.a)
        && veq(cv.b, rv.b)
        && cv.iters == rv.iters
        && match (&cv.cache, &rv.cache) {
            (Some(x), Some(y)) => cache_eq(x, y),
            (None, None) => true,
            _ => false,
        };
    assert!(
        ok,
        "{ctx}\n  A={a:?}\n  B={b:?}\n  opts={o:?}\n  C   : dist={} a={} b={} iters={} cache={}\n  RUST: dist={} a={} b={} iters={} cache={}",
        fdesc(cv.dist), vdesc(cv.a), vdesc(cv.b), cv.iters,
        cv.cache.as_ref().map(cache_desc).unwrap_or_else(|| "-".into()),
        fdesc(rv.dist), vdesc(rv.a), vdesc(rv.b), rv.iters,
        rv.cache.as_ref().map(cache_desc).unwrap_or_else(|| "-".into()),
    );
}

/// Sweep all 9 `(typeA, typeB)` combinations with a fixed option set.
fn gjk_matrix(seed: u64, iters_per_pair: usize, mk_opts: impl Fn(&mut Rng) -> GjkOpts, with_cache: bool) {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(seed);
    let mut hit_count = 0usize;
    let mut nonzero = 0usize;
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for i in 0..iters_per_pair {
                let size = SCALES[rng.below(SCALES.len())];
                let (a, b) = shape_pair(&mut rng, tyA, tyB, size);
                let o = mk_opts(&mut rng);
                let cache = if with_cache { Some(c2GJKCache::default()) } else { None };
                let cv = unsafe { call_gjk(cf, &a, &b, &o, cache) };
                let rv = unsafe { call_gjk(rf, &a, &b, &o, cache) };
                assert_gjk_eq(
                    &format!("c2GJK seed={seed:#x} ({tyA},{tyB}) iter {i}"),
                    &a, &b, &o, &cv, &rv,
                );
                if cv.dist == 0.0 { hit_count += 1 } else { nonzero += 1 }
            }
        }
    }
    assert!(hit_count > 0 && nonzero > 0, "need both touching and separated: {hit_count}/{nonzero}");
}

#[test]
fn row49_gjk_default_options() {
    gjk_matrix(0x1090, 900, |_| GjkOpts::default(), false);
}

#[test]
fn row50_gjk_use_radius_zero() {
    gjk_matrix(0x1091, 900, |_| GjkOpts { use_radius: 0, ..Default::default() }, false);
}

#[test]
fn row51_gjk_explicit_identity_transforms() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let ident = c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: 1.0, s: 0.0 } };
    let mut rng = Rng::new(0x1092);
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for i in 0..600 {
                let size = SCALES[rng.below(SCALES.len())];
                let (a, b) = shape_pair(&mut rng, tyA, tyB, size);
                let o = GjkOpts { ax: Some(ident), bx: Some(ident), ..Default::default() };
                let cv = unsafe { call_gjk(cf, &a, &b, &o, None) };
                let rv = unsafe { call_gjk(rf, &a, &b, &o, None) };
                assert_gjk_eq(&format!("explicit identity ({tyA},{tyB}) iter {i}"), &a, &b, &o, &cv, &rv);
                // An explicit identity must give the same answer as NULL.
                let o_null = GjkOpts::default();
                let cn = unsafe { call_gjk(cf, &a, &b, &o_null, None) };
                assert!(
                    feq(cv.dist, cn.dist) && veq(cv.a, cn.a) && veq(cv.b, cn.b) && cv.iters == cn.iters,
                    "identity vs NULL differ in the C itself ({tyA},{tyB}) iter {i}"
                );
            }
        }
    }
}

#[test]
fn row52_gjk_rotation_only() {
    gjk_matrix(
        0x1093,
        700,
        |rng| {
            let ang = rng.range(-3.15, 3.15);
            GjkOpts {
                ax: Some(c2x { p: c2v { x: 0.0, y: 0.0 }, r: c2r { c: ang.cos(), s: ang.sin() } }),
                bx: None,
                ..Default::default()
            }
        },
        false,
    );
}

#[test]
fn row53_gjk_translation_only() {
    gjk_matrix(
        0x1094,
        700,
        |rng| GjkOpts {
            ax: Some(c2x { p: rng.vec_coord(4.0), r: c2r { c: 1.0, s: 0.0 } }),
            bx: Some(c2x { p: rng.vec_coord(4.0), r: c2r { c: 1.0, s: 0.0 } }),
            ..Default::default()
        },
        false,
    );
}

#[test]
fn row54_gjk_rotation_and_translation() {
    gjk_matrix(
        0x1095,
        700,
        |rng| {
            let a = rng.range(-3.15, 3.15);
            let b = rng.range(-3.15, 3.15);
            GjkOpts {
                ax: Some(c2x { p: rng.vec_coord(4.0), r: c2r { c: a.cos(), s: a.sin() } }),
                bx: Some(c2x { p: rng.vec_coord(4.0), r: c2r { c: b.cos(), s: b.sin() } }),
                use_radius: if rng.boolean() { 1 } else { 0 },
                ..Default::default()
            }
        },
        false,
    );
}

#[test]
fn row55_gjk_non_unit_rotation() {
    gjk_matrix(
        0x1096,
        700,
        |rng| GjkOpts {
            ax: Some(gen_x(rng, 4.0)),
            bx: Some(gen_x(rng, 4.0)),
            use_radius: if rng.boolean() { 1 } else { 0 },
            ..Default::default()
        },
        false,
    );
}

#[test]
fn row56_gjk_cold_cache() {
    gjk_matrix(0x1097, 700, |_| GjkOpts::default(), true);
}

#[test]
fn row57_gjk_warm_cache_replay() {
    // Carry one cache across several calls with slightly moved shapes: the
    // second and later calls take the `cache_was_read` path.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x1098);
    let mut warm_calls = 0usize;
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for i in 0..300 {
                let size = SCALES[rng.below(SCALES.len())];
                let mut c_cache = c2GJKCache::default();
                let mut r_cache = c2GJKCache::default();
                for step in 0..4 {
                    let (a, b) = shape_pair(&mut rng, tyA, tyB, size);
                    let o = GjkOpts::default();
                    let cv = unsafe { call_gjk(cf, &a, &b, &o, Some(c_cache)) };
                    let rv = unsafe { call_gjk(rf, &a, &b, &o, Some(r_cache)) };
                    assert_gjk_eq(
                        &format!("warm cache ({tyA},{tyB}) iter {i} step {step}"),
                        &a, &b, &o, &cv, &rv,
                    );
                    c_cache = cv.cache.unwrap();
                    r_cache = rv.cache.unwrap();
                    if step > 0 && c_cache.count != 0 {
                        warm_calls += 1;
                    }
                }
            }
        }
    }
    assert!(warm_calls > 0, "no warm-cache call was exercised");
}

#[test]
fn row58_gjk_warm_cache_changing_transforms() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x1099);
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for i in 0..300 {
                let size = SCALES[rng.below(SCALES.len())];
                let (a, b) = shape_pair(&mut rng, tyA, tyB, size);
                let mut c_cache = c2GJKCache::default();
                let mut r_cache = c2GJKCache::default();
                for step in 0..4 {
                    let ang = rng.range(-3.15, 3.15);
                    let o = GjkOpts {
                        ax: Some(c2x {
                            p: rng.vec_coord(size),
                            r: c2r { c: ang.cos(), s: ang.sin() },
                        }),
                        bx: if rng.boolean() { Some(gen_x(&mut rng, size)) } else { None },
                        use_radius: if rng.boolean() { 1 } else { 0 },
                        ..Default::default()
                    };
                    let cv = unsafe { call_gjk(cf, &a, &b, &o, Some(c_cache)) };
                    let rv = unsafe { call_gjk(rf, &a, &b, &o, Some(r_cache)) };
                    assert_gjk_eq(
                        &format!("warm cache + transforms ({tyA},{tyB}) iter {i} step {step}"),
                        &a, &b, &o, &cv, &rv,
                    );
                    c_cache = cv.cache.unwrap();
                    r_cache = rv.cache.unwrap();
                }
            }
        }
    }
}

#[test]
fn row59_gjk_relations() {
    // Separated far / touching exactly / deeply overlapping, per type pair.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x109a);
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for i in 0..800 {
                let size = 8.0f32;
                let a = shape_at(&mut rng, tyA, c2v { x: 0.0, y: 0.0 }, size);
                let d = match rng.below(4) {
                    0 => 0.0,                      // coincident
                    1 => size,                     // touching-ish
                    2 => size * 4.0,               // far
                    _ => size / 2.0,               // overlapping
                };
                let b = shape_at(&mut rng, tyB, c2v { x: d, y: 0.0 }, size);
                for &use_radius in &[0, 1] {
                    let o = GjkOpts { use_radius, ..Default::default() };
                    let cv = unsafe { call_gjk(cf, &a, &b, &o, Some(c2GJKCache::default())) };
                    let rv = unsafe { call_gjk(rf, &a, &b, &o, Some(c2GJKCache::default())) };
                    assert_gjk_eq(&format!("relations ({tyA},{tyB}) iter {i}"), &a, &b, &o, &cv, &rv);
                }
            }
        }
    }
}

#[test]
fn row60_gjk_degenerate_proxies() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x109b);
    let degenerates: [Shape; 6] = [
        Shape::Circle(c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 }),
        Shape::Circle(c2Circle { p: c2v { x: 3.0, y: -2.0 }, r: 0.0 }),
        Shape::Aabb(c2AABB { min: c2v { x: 1.0, y: 1.0 }, max: c2v { x: 1.0, y: 1.0 } }),
        Shape::Aabb(c2AABB { min: c2v { x: 4.0, y: 4.0 }, max: c2v { x: -4.0, y: -4.0 } }),
        Shape::Capsule(c2Capsule { a: c2v { x: 2.0, y: 2.0 }, b: c2v { x: 2.0, y: 2.0 }, r: 0.0 }),
        Shape::Capsule(c2Capsule { a: c2v { x: -1.0, y: 5.0 }, b: c2v { x: -1.0, y: 5.0 }, r: 3.0 }),
    ];
    for a in &degenerates {
        for b in &degenerates {
            for &use_radius in &[0, 1] {
                for _ in 0..8 {
                    let o = GjkOpts {
                        ax: if rng.boolean() { Some(gen_x(&mut rng, 4.0)) } else { None },
                        bx: if rng.boolean() { Some(gen_x(&mut rng, 4.0)) } else { None },
                        use_radius,
                        ..Default::default()
                    };
                    let cv = unsafe { call_gjk(cf, a, b, &o, Some(c2GJKCache::default())) };
                    let rv = unsafe { call_gjk(rf, a, b, &o, Some(c2GJKCache::default())) };
                    assert_gjk_eq("degenerate proxies", a, b, &o, &cv, &rv);
                }
            }
        }
    }
}

#[test]
fn row61_gjk_magnitude_classes() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x109c);
    for &tyA in &ALL_TYPES {
        for &tyB in &ALL_TYPES {
            for &sa in &SCALES {
                for &sb in &SCALES {
                    for i in 0..60 {
                        let a = shape_at(&mut rng, tyA, c2v { x: 0.0, y: 0.0 }, sa);
                        let centre = jitter(&mut rng, c2v { x: 0.0, y: 0.0 }, sb);
                        let b = shape_at(&mut rng, tyB, centre, sb);
                        let o = GjkOpts {
                            use_radius: if rng.boolean() { 1 } else { 0 },
                            ..Default::default()
                        };
                        let cv = unsafe { call_gjk(cf, &a, &b, &o, Some(c2GJKCache::default())) };
                        let rv = unsafe { call_gjk(rf, &a, &b, &o, Some(c2GJKCache::default())) };
                        assert_gjk_eq(
                            &format!("magnitudes ({tyA},{tyB}) sa={sa:e} sb={sb:e} iter {i}"),
                            &a, &b, &o, &cv, &rv,
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn row62_gjk_null_output_subsets() {
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x109d);
    for mask in 0..16u32 {
        let want_out_a = mask & 1 != 0;
        let want_out_b = mask & 2 != 0;
        let want_iters = mask & 4 != 0;
        let with_cache = mask & 8 != 0;
        for &tyA in &ALL_TYPES {
            for &tyB in &ALL_TYPES {
                for i in 0..40 {
                    let size = SCALES[rng.below(SCALES.len())];
                    let (a, b) = shape_pair(&mut rng, tyA, tyB, size);
                    let o = GjkOpts {
                        use_radius: if rng.boolean() { 1 } else { 0 },
                        want_out_a,
                        want_out_b,
                        want_iters,
                        ..Default::default()
                    };
                    let cache = if with_cache { Some(c2GJKCache::default()) } else { None };
                    let cv = unsafe { call_gjk(cf, &a, &b, &o, cache) };
                    let rv = unsafe { call_gjk(rf, &a, &b, &o, cache) };
                    assert_gjk_eq(&format!("null subsets mask={mask} ({tyA},{tyB}) iter {i}"), &a, &b, &o, &cv, &rv);
                    // Skipped stores must leave the sentinel untouched.
                    if !want_out_a {
                        assert!(veq(cv.a, SENTINEL_V) && veq(rv.a, SENTINEL_V));
                    }
                    if !want_out_b {
                        assert!(veq(cv.b, SENTINEL_V) && veq(rv.b, SENTINEL_V));
                    }
                    if !want_iters {
                        assert_eq!(cv.iters, SENTINEL_I);
                        assert_eq!(rv.iters, SENTINEL_I);
                    }
                }
            }
        }
    }
}

#[test]
fn row63_gjk_aabb_vs_aabb_wide_proxy() {
    // AABB proxies have 4 vertices, the widest `c2Support` search this library
    // performs, and they are the pairing that needs the most GJK iterations.
    let l = libs();
    let (cf, rf) = l.pair::<FnGJK>("c2GJK");
    let mut rng = Rng::new(0x109e);
    let mut max_iters = 0;
    for i in 0..ITERS * 2 {
        let size = SCALES[rng.below(SCALES.len())];
        let (a, b) = shape_pair(&mut rng, C2_TYPE_AABB, C2_TYPE_AABB, size);
        let o = GjkOpts {
            ax: if rng.boolean() { Some(gen_x(&mut rng, size)) } else { None },
            bx: if rng.boolean() { Some(gen_x(&mut rng, size)) } else { None },
            use_radius: if rng.boolean() { 1 } else { 0 },
            ..Default::default()
        };
        let cv = unsafe { call_gjk(cf, &a, &b, &o, Some(c2GJKCache::default())) };
        let rv = unsafe { call_gjk(rf, &a, &b, &o, Some(c2GJKCache::default())) };
        assert_gjk_eq(&format!("wide proxy iter {i}"), &a, &b, &o, &cv, &rv);
        max_iters = max_iters.max(cv.iters);
    }
    assert!(max_iters >= 2, "GJK never iterated more than {max_iters} times");
}

// ===========================================================================
// Rows 64-65 — the `aabb` entry point declared in include/lib.h
// ===========================================================================

#[test]
fn row64_aabb_random() {
    let l = libs();
    let (c, r) = l.pair::<FnAabb>("aabb");
    let mut rng = Rng::new(0x10a0);
    let mut seen = std::collections::BTreeSet::new();
    for i in 0..ITERS * 8 {
        let (a, b, cc, d) = match rng.below(4) {
            0 => (rng.wild(), rng.wild(), rng.wild(), rng.wild()),
            1 => {
                // Around the three hard-coded shapes.
                let x = rng.range(-140.0, 40.0);
                let y = rng.range(-80.0, 140.0);
                (x, y, x + rng.range(-10.0, 60.0), y + rng.range(-10.0, 60.0))
            }
            2 => {
                let x = rng.coord(8.0);
                let y = rng.coord(8.0);
                (x, y, x, y) // degenerate
            }
            _ => (rng.coord(8.0), rng.coord(8.0), rng.coord(8.0), rng.coord(8.0)),
        };
        let cv = unsafe { c(a, b, cc, d) };
        let rv = unsafe { r(a, b, cc, d) };
        assert_eq!(cv, rv, "iter {i}: aabb({}, {}, {}, {})", fdesc(a), fdesc(b), fdesc(cc), fdesc(d));
        seen.insert(cv);
    }
    assert!(seen.len() >= 4, "aabb result values observed: {seen:?}");
}

#[test]
fn row65_aabb_grid_sweep() {
    let l = libs();
    let (c, r) = l.pair::<FnAabb>("aabb");
    let mut seen = std::collections::BTreeSet::new();
    // Grid over the region occupied by circle(-70,0,r20), aabb(-40..-15) and
    // capsule((-40,40)-(-20,100), r10).
    let mut x = -120i32;
    while x <= 40 {
        let mut y = -80i32;
        while y <= 140 {
            for &(w, h) in &[(0i32, 0i32), (5, 5), (20, 20), (60, 60), (200, 200)] {
                let (a, b, cc, d) = (x as f32, y as f32, (x + w) as f32, (y + h) as f32);
                let cv = unsafe { c(a, b, cc, d) };
                let rv = unsafe { r(a, b, cc, d) };
                assert_eq!(cv, rv, "aabb({a}, {b}, {cc}, {d})");
                seen.insert(cv);
            }
            y += 5;
        }
        x += 5;
    }
    assert!(seen.len() >= 6, "aabb bitmask values observed: {seen:?}");
}
