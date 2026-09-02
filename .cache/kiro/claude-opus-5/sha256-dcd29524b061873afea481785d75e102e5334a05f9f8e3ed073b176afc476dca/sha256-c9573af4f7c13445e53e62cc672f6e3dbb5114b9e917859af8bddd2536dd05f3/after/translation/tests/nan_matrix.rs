//! NaN/inf bit-pattern matrix.
//!
//! `MULSS`/`ADDSS`/`SUBSS` return their **destination** operand when both
//! operands are QNaNs, so any instruction-selection difference between gcc and
//! LLVM (operand commutation, `fneg`-into-`fsub` folding, SLP vectorisation)
//! shows up as a flipped NaN sign bit. `ERRORS.md` rows 13/17/19/47/59/60/67
//! only sample this space; this file walks the independent cross-product of a
//! nasty value set through every arithmetic export.

mod common;

use common::*;
use std::ffi::c_int;

/// Both NaN signs are included deliberately — that is the distinguishing case.
const NASTY: [f32; 8] = [
    f32::NAN,                       // 0x7fc00000
    -f32::NAN,                      // 0xffc00000
    f32::INFINITY,
    f32::NEG_INFINITY,
    0.0,
    -0.0,
    2.0,
    -3.5,
];

#[test]
fn nan_matrix_unary() {
    let p = load_pair();
    unsafe {
        for &x in &NASTY {
            for &y in &NASTY {
                let v = c2v { x, y };
                let t = format!("({:08x},{:08x})", x.to_bits(), y.to_bits());
                eq_v(&format!("c2V {t}"), (p.c.c2V)(x, y), (p.r.c2V)(x, y));
                eq_v(&format!("c2Neg {t}"), (p.c.c2Neg)(v), (p.r.c2Neg)(v));
                eq_v(&format!("c2Skew {t}"), (p.c.c2Skew)(v), (p.r.c2Skew)(v));
                eq_v(&format!("c2CCW90 {t}"), (p.c.c2CCW90)(v), (p.r.c2CCW90)(v));
                eq_f32(&format!("c2Len {t}"), (p.c.c2Len)(v), (p.r.c2Len)(v));
                eq_v(&format!("c2Norm {t}"), (p.c.c2Norm)(v), (p.r.c2Norm)(v));
                for &s in &NASTY {
                    eq_v(
                        &format!("c2Mulvs {t} s={:08x}", s.to_bits()),
                        (p.c.c2Mulvs)(v, s),
                        (p.r.c2Mulvs)(v, s),
                    );
                    eq_v(
                        &format!("c2Div {t} s={:08x}", s.to_bits()),
                        (p.c.c2Div)(v, s),
                        (p.r.c2Div)(v, s),
                    );
                }
            }
        }
    }
}

#[test]
fn nan_matrix_binary_full_cross_product() {
    let p = load_pair();
    unsafe {
        // independent 4-tuples: 8^4 = 4096 combinations
        for &ax in &NASTY {
            for &ay in &NASTY {
                for &bx in &NASTY {
                    for &by in &NASTY {
                        let a = c2v { x: ax, y: ay };
                        let b = c2v { x: bx, y: by };
                        let r = c2r { c: ax, s: ay };
                        let t = format!(
                            "a=({:08x},{:08x}) b=({:08x},{:08x})",
                            ax.to_bits(),
                            ay.to_bits(),
                            bx.to_bits(),
                            by.to_bits()
                        );
                        eq_f32(&format!("c2Dot {t}"), (p.c.c2Dot)(a, b), (p.r.c2Dot)(a, b));
                        eq_f32(&format!("c2Det2 {t}"), (p.c.c2Det2)(a, b), (p.r.c2Det2)(a, b));
                        eq_v(&format!("c2Add {t}"), (p.c.c2Add)(a, b), (p.r.c2Add)(a, b));
                        eq_v(&format!("c2Sub {t}"), (p.c.c2Sub)(a, b), (p.r.c2Sub)(a, b));
                        eq_v(&format!("c2Maxv {t}"), (p.c.c2Maxv)(a, b), (p.r.c2Maxv)(a, b));
                        eq_v(&format!("c2Minv {t}"), (p.c.c2Minv)(a, b), (p.r.c2Minv)(a, b));
                        eq_v(&format!("c2Mulrv {t}"), (p.c.c2Mulrv)(r, b), (p.r.c2Mulrv)(r, b));
                        eq_v(&format!("c2MulrvT {t}"), (p.c.c2MulrvT)(r, b), (p.r.c2MulrvT)(r, b));
                    }
                }
            }
        }
    }
}

#[test]
fn nan_matrix_clampv() {
    let p = load_pair();
    unsafe {
        for &ax in &NASTY {
            for &lx in &NASTY {
                for &hx in &NASTY {
                    for &ay in &NASTY {
                        let a = c2v { x: ax, y: ay };
                        let lo = c2v { x: lx, y: hx };
                        let hi = c2v { x: hx, y: lx };
                        eq_v(
                            &format!(
                                "c2Clampv a=({:08x},{:08x}) lo.x={:08x} hi.x={:08x}",
                                ax.to_bits(),
                                ay.to_bits(),
                                lx.to_bits(),
                                hx.to_bits()
                            ),
                            (p.c.c2Clampv)(a, lo, hi),
                            (p.r.c2Clampv)(a, lo, hi),
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn nan_matrix_mulxv() {
    let p = load_pair();
    let mut rng = Rng::new(0x5eed);
    unsafe {
        for &c in &NASTY {
            for &s in &NASTY {
                for &px in &NASTY {
                    for &vx in &NASTY {
                        for k in 0..NASTY.len() {
                            let x = c2x {
                                p: c2v { x: px, y: NASTY[k] },
                                r: c2r { c, s },
                            };
                            let v = c2v { x: vx, y: NASTY[(k + 3) % NASTY.len()] };
                            eq_v(
                                &format!(
                                    "c2Mulxv r=({:08x},{:08x}) p.x={:08x} v.x={:08x} k={k}",
                                    c.to_bits(),
                                    s.to_bits(),
                                    px.to_bits(),
                                    vx.to_bits()
                                ),
                                (p.c.c2Mulxv)(x, v),
                                (p.r.c2Mulxv)(x, v),
                            );
                        }
                    }
                }
            }
        }
        let _ = rng.next_u64();
    }
}

#[test]
fn nan_matrix_simplex_routines() {
    let p = load_pair();
    let mut rng = Rng::new(0xC0FFEE);
    unsafe {
        for i in 0..20000 {
            let pick = |k: usize| NASTY[(i * 7 + k * 3) % NASTY.len()];
            let mut sc = c2Simplex::default();
            for k in 0..4 {
                sc.verts[k] = c2sv {
                    sA: c2v { x: pick(k), y: pick(k + 1) },
                    sB: c2v { x: pick(k + 2), y: pick(k + 3) },
                    p: c2v { x: pick(k + 4), y: pick(k + 5) },
                    u: pick(k + 6),
                    iA: rng.below(4) as c_int,
                    iB: rng.below(4) as c_int,
                };
            }
            sc.div = pick(i % NASTY.len());
            sc.count = (i % 5) as c_int;

            // c22
            let mut a = sc;
            let mut b = sc;
            a.count = 2;
            b.count = 2;
            (p.c.c22)(&mut a);
            (p.r.c22)(&mut b);
            eq_simplex(&format!("nan c22 #{i}"), &a, &b);

            // c23
            let mut a = sc;
            let mut b = sc;
            a.count = 3;
            b.count = 3;
            (p.c.c23)(&mut a);
            (p.r.c23)(&mut b);
            eq_simplex(&format!("nan c23 #{i}"), &a, &b);

            // c2D / c2L / c2GJKSimplexMetric / c2Witness at the sampled count
            let mut a = sc;
            let mut b = sc;
            eq_v(&format!("nan c2D #{i}"), (p.c.c2D)(&mut a), (p.r.c2D)(&mut b));
            eq_v(&format!("nan c2L #{i}"), (p.c.c2L)(&mut a), (p.r.c2L)(&mut b));
            eq_f32(
                &format!("nan metric #{i}"),
                (p.c.c2GJKSimplexMetric)(&mut a),
                (p.r.c2GJKSimplexMetric)(&mut b),
            );
            let mut wac = c2v { x: 1.0, y: 2.0 };
            let mut wbc = c2v { x: 3.0, y: 4.0 };
            let mut war = wac;
            let mut wbr = wbc;
            (p.c.c2Witness)(&mut a, &mut wac, &mut wbc);
            (p.r.c2Witness)(&mut b, &mut war, &mut wbr);
            eq_v(&format!("nan witness a #{i}"), wac, war);
            eq_v(&format!("nan witness b #{i}"), wbc, wbr);
            eq_simplex(&format!("nan witness struct #{i}"), &a, &b);
        }
    }
}

#[test]
fn nan_matrix_gjk_transforms_and_shapes() {
    // NaN/inf in every rotation component, translation, shape coordinate and
    // radius, with the full type cross-product.
    let p = load_pair();
    let mut rng = Rng::new(0xDEADBEEF);
    unsafe {
        for i in 0..30000usize {
            let pick = |k: usize| NASTY[(i * 5 + k * 3) % NASTY.len()];
            let ta = ALL_TYPES[i % 3];
            let tb = ALL_TYPES[(i / 3) % 3];
            let sa = match ta {
                C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                    p: c2v { x: pick(0), y: pick(1) },
                    r: pick(2),
                }),
                C2_TYPE_AABB => Shape::Aabb(c2AABB {
                    min: c2v { x: pick(0), y: pick(1) },
                    max: c2v { x: pick(2), y: pick(3) },
                }),
                _ => Shape::Capsule(c2Capsule {
                    a: c2v { x: pick(0), y: pick(1) },
                    b: c2v { x: pick(2), y: pick(3) },
                    r: pick(4),
                }),
            };
            let sb = match tb {
                C2_TYPE_CIRCLE => Shape::Circle(c2Circle {
                    p: c2v { x: pick(5), y: rng.coord() },
                    r: pick(6),
                }),
                C2_TYPE_AABB => Shape::Aabb(c2AABB {
                    min: c2v { x: pick(5), y: rng.coord() },
                    max: c2v { x: rng.coord(), y: pick(6) },
                }),
                _ => Shape::Capsule(c2Capsule {
                    a: c2v { x: pick(5), y: rng.coord() },
                    b: c2v { x: rng.coord(), y: pick(6) },
                    r: pick(7),
                }),
            };
            let ax = c2x {
                p: c2v { x: pick(1), y: pick(4) },
                r: c2r { c: pick(2), s: pick(3) },
            };
            let bx = c2x {
                p: c2v { x: pick(6), y: pick(0) },
                r: c2r { c: pick(5), s: pick(7) },
            };
            let ur = (i % 2) as c_int;
            let (axo, bxo) = match i % 3 {
                0 => (None, None),
                1 => (Some(&ax), None),
                _ => (Some(&ax), Some(&bx)),
            };
            let mut cc = c2GJKCache::default();
            let mut cr = c2GJKCache::default();
            let oc = call_gjk(&p.c, &sa, axo, &sb, bxo, ur, true, true, true, Some(&mut cc));
            let or = call_gjk(&p.r, &sa, axo, &sb, bxo, ur, true, true, true, Some(&mut cr));
            eq_gjk_out(&format!("nan gjk #{i} ta={ta} tb={tb} ur={ur}"), &oc, &or);
            eq_cache(&format!("nan gjk #{i} cache"), &cc, &cr);
        }
    }
}

#[test]
fn nan_matrix_wrapper() {
    let p = load_pair();
    unsafe {
        for i in 0..40000usize {
            let f: Vec<f32> = (0..9).map(|k| NASTY[(i * 3 + k * 5) % NASTY.len()]).collect();
            for &rev in &[0i8, 1, -1] {
                let mut ac = c2v::default();
                let mut bc = c2v::default();
                let mut ar = c2v::default();
                let mut br = c2v::default();
                (p.c.gjk)(rev, &mut ac, &mut bc, f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8]);
                (p.r.gjk)(rev, &mut ar, &mut br, f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8]);
                eq_v(&format!("nan wrapper #{i} rev={rev} a"), ac, ar);
                eq_v(&format!("nan wrapper #{i} rev={rev} b"), bc, br);
            }
        }
    }
}
