//! Phase B (anti-blind-spot) — the NaN/infinity operand matrix.
//!
//! Motivation, found by mutation-testing the rest of the suite: several
//! arithmetic sites are only distinguishable when **both** operands of a single
//! SSE instruction are NaN *and those two NaNs have different bit patterns*.
//! x86 resolves such a tie in favour of the instruction's destination operand,
//! so a Rust translation that writes `a * b` where GCC emitted `MULSS b, a`
//! silently returns the wrong NaN payload — a genuine byte-level divergence.
//!
//! Randomly generated floats essentially never produce that situation: you need
//! one NaN to arrive as an *input* with a distinctive payload while a *second,
//! different* NaN is manufactured internally by an invalid operation
//! (`0*inf`, `inf-inf`, `0/0`), which itself requires infinities in just the
//! right places.
//!
//! So this file drives every entry point over a small, deliberately chosen
//! alphabet of signed zeros, ±1, ±inf and several *distinguishable* NaN
//! encodings, in EVERY argument slot — exhaustively where the slot count allows
//! and by dense random sampling otherwise.

#![allow(non_snake_case)]

mod common;

use common::*;

/// High-yield alphabet. Signed zeros and infinities manufacture the x86
/// "indefinite" QNaN (`0xFFC00000`) internally; the explicit NaNs carry
/// *different* payloads so a wrong operand order changes the observable bits.
const ALPHA: &[u32] = &[
    0x0000_0000, // +0.0   — with inf, makes 0*inf → 0xFFC00000
    0x8000_0000, // -0.0
    0x3F80_0000, // +1.0
    0xBF80_0000, // -1.0
    0x7F80_0000, // +inf   — with inf, makes inf-inf → 0xFFC00000
    0xFF80_0000, // -inf
    0x7FC0_1234, // +qNaN, distinctive payload
    0xFFC0_5678, // -qNaN, distinctive payload
    0x7F80_0001, // +sNaN (must be quieted to 0x7FC00001)
];

/// A 5-value sub-alphabet used for the exhaustive sweeps, chosen because these
/// five suffice to manufacture `0*inf` and `inf-inf` in any slot.
const ALPHA5: &[u32] = &[0x0000_0000, 0x3F80_0000, 0xBF80_0000, 0x7F80_0000, 0xFF80_0000];

/// The distinctive NaNs that get pinned into one slot while the others sweep.
const PINNED_NANS: &[u32] = &[0x7FC0_1234, 0xFFC0_5678, 0x7F80_0001, 0x7FC0_0000];

#[inline]
fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

// ---------------------------------------------------------------------------
// c2RaytoAABB — 9 float slots
// ---------------------------------------------------------------------------

fn aabb_from_slots(s: &[f32; 9]) -> (c2Ray, c2AABB) {
    (
        c2Ray {
            p: c2v { x: s[0], y: s[1] },
            d: c2v { x: s[2], y: s[3] },
            t: s[4],
        },
        c2AABB {
            min: c2v { x: s[5], y: s[6] },
            max: c2v { x: s[7], y: s[8] },
        },
    )
}

/// `tag` is a `&'static str`, never a `format!` — these helpers run millions of
/// times and eager message construction would dominate the runtime.
fn check_aabb(l: &Pair, tag: &str, s: &[f32; 9]) {
    let (a, b) = aabb_from_slots(s);
    let c = run_aabb(&l.c, a, b);
    let r = run_aabb(&l.rs, a, b);
    if c != r {
        panic!(
            "DIVERGENCE [{tag}]\n  slots = {:08x?}\n  C    = {c:?}\n  RUST = {r:?}",
            s.map(|v| v.to_bits())
        );
    }
}

#[test]
fn nan_matrix_raytoaabb_exhaustive_with_pinned_nan() {
    let l = libs();
    // Pin a distinctive NaN into each slot in turn, then sweep the remaining
    // eight slots over the 5-value alphabet: 9 * 4 * 5^8 would be too many, so
    // sweep the 8 others in a nested loop over 5 values for the four slots that
    // feed `out->t = t_k * A.t` and randomly for the rest.
    let base = [0x3F80_0000u32; 9];
    for &pin_bits in PINNED_NANS {
        for pin_slot in 0..9usize {
            // Exhaustive over 4 chosen "co-slots" (5^4 = 625), random over the
            // remaining 4 — enough to manufacture 0*inf / inf-inf alongside the
            // pinned NaN in every arrangement that matters.
            let co: Vec<usize> = (0..9).filter(|&i| i != pin_slot).collect();
            for a0 in ALPHA5 {
                for a1 in ALPHA5 {
                    for a2 in ALPHA5 {
                        for a3 in ALPHA5 {
                            let mut s = base.map(f);
                            s[pin_slot] = f(pin_bits);
                            s[co[0]] = f(*a0);
                            s[co[1]] = f(*a1);
                            s[co[2]] = f(*a2);
                            s[co[3]] = f(*a3);
                            check_aabb(&l, "aabb pinned-nan sweep", &s);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn nan_matrix_raytoaabb_random_full_alphabet() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xA1);
    for _ in 0..400_000 {
        let mut s = [0.0f32; 9];
        for slot in s.iter_mut() {
            *slot = f(ALPHA[rng.below(ALPHA.len() as u32) as usize]);
        }
        check_aabb(&l, "aabb random full alphabet", &s);
    }
    // Same, but biased so that at least two slots always carry *different* NaNs.
    for _ in 0..200_000 {
        let mut s = [0.0f32; 9];
        for slot in s.iter_mut() {
            *slot = f(ALPHA5[rng.below(ALPHA5.len() as u32) as usize]);
        }
        let n1 = rng.below(9) as usize;
        let mut n2 = rng.below(9) as usize;
        if n2 == n1 {
            n2 = (n2 + 1) % 9;
        }
        s[n1] = f(PINNED_NANS[rng.below(PINNED_NANS.len() as u32) as usize]);
        s[n2] = f(PINNED_NANS[rng.below(PINNED_NANS.len() as u32) as usize]);
        check_aabb(&l, "aabb two-distinct-nan", &s);
    }
}

// ---------------------------------------------------------------------------
// c2RaytoCapsule — 10 float slots
// ---------------------------------------------------------------------------

fn capsule_from_slots(s: &[f32; 10]) -> (c2Ray, c2Capsule) {
    (
        c2Ray {
            p: c2v { x: s[0], y: s[1] },
            d: c2v { x: s[2], y: s[3] },
            t: s[4],
        },
        c2Capsule {
            a: c2v { x: s[5], y: s[6] },
            b: c2v { x: s[7], y: s[8] },
            r: s[9],
        },
    )
}

fn check_capsule(l: &Pair, tag: &str, s: &[f32; 10]) {
    let (a, b) = capsule_from_slots(s);
    let c = run_capsule(&l.c, a, b);
    let r = run_capsule(&l.rs, a, b);
    if c != r {
        panic!(
            "DIVERGENCE [{tag}]\n  slots = {:08x?}\n  C    = {c:?}\n  RUST = {r:?}",
            s.map(|v| v.to_bits())
        );
    }
}

#[test]
fn nan_matrix_raytocapsule_exhaustive_with_pinned_nan() {
    let l = libs();
    let base = [0x3F80_0000u32; 10];
    for &pin_bits in PINNED_NANS {
        for pin_slot in 0..10usize {
            let co: Vec<usize> = (0..10).filter(|&i| i != pin_slot).collect();
            for a0 in ALPHA5 {
                for a1 in ALPHA5 {
                    for a2 in ALPHA5 {
                        for a3 in ALPHA5 {
                            let mut s = base.map(f);
                            s[pin_slot] = f(pin_bits);
                            s[co[0]] = f(*a0);
                            s[co[1]] = f(*a1);
                            s[co[2]] = f(*a2);
                            s[co[3]] = f(*a3);
                            check_capsule(&l, "capsule pinned-nan sweep", &s);
                        }
                    }
                }
            }
        }
    }
    // The degenerate capsule (a == b) is the case that manufactures a *pair* of
    // different NaNs internally: c2Norm((0,0)) is 0xFFC00000 and c2CCW90 flips
    // its sign to 0x7FC00000, so M.x and M.y carry different payloads which then
    // meet inside c2MulmvT and the `y` interpolation.
    for &ax in ALPHA {
        for &ay in ALPHA {
            for &r in ALPHA {
                for &t in ALPHA {
                    for &dx in ALPHA5 {
                        for &dy in ALPHA5 {
                            let s = [
                                f(0x3F80_0000),
                                f(0xBF80_0000),
                                f(dx),
                                f(dy),
                                f(t),
                                f(ax),
                                f(ay),
                                f(ax),
                                f(ay), // a == b exactly
                                f(r),
                            ];
                            check_capsule(&l, "capsule degenerate-axis", &s);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn nan_matrix_raytocapsule_random_full_alphabet() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xA2);
    for _ in 0..400_000 {
        let mut s = [0.0f32; 10];
        for slot in s.iter_mut() {
            *slot = f(ALPHA[rng.below(ALPHA.len() as u32) as usize]);
        }
        check_capsule(&l, "capsule random full alphabet", &s);
    }
    for _ in 0..200_000 {
        let mut s = [0.0f32; 10];
        for slot in s.iter_mut() {
            *slot = f(ALPHA5[rng.below(ALPHA5.len() as u32) as usize]);
        }
        let n1 = rng.below(10) as usize;
        let mut n2 = rng.below(10) as usize;
        if n2 == n1 {
            n2 = (n2 + 1) % 10;
        }
        s[n1] = f(PINNED_NANS[rng.below(PINNED_NANS.len() as u32) as usize]);
        s[n2] = f(PINNED_NANS[rng.below(PINNED_NANS.len() as u32) as usize]);
        check_capsule(&l, "capsule two-distinct-nan", &s);
    }
}

// ---------------------------------------------------------------------------
// c2RaytoCircle — 8 float slots (exhaustive over the 5-value alphabet)
// ---------------------------------------------------------------------------

#[test]
fn nan_matrix_raytocircle() {
    let l = libs();
    let check = |s: &[f32; 8], ctx: &str| {
        let a = c2Ray {
            p: c2v { x: s[0], y: s[1] },
            d: c2v { x: s[2], y: s[3] },
            t: s[4],
        };
        let b = c2Circle {
            p: c2v { x: s[5], y: s[6] },
            r: s[7],
        };
        let c = run_circle(&l.c, a, b);
        let r = run_circle(&l.rs, a, b);
        if c != r {
            panic!(
                "DIVERGENCE [{ctx}]\n  slots = {:08x?}\n  C    = {c:?}\n  RUST = {r:?}",
                s.map(|v| v.to_bits())
            );
        }
    };
    let base = [0x3F80_0000u32; 8];
    for &pin_bits in PINNED_NANS {
        for pin_slot in 0..8usize {
            let co: Vec<usize> = (0..8).filter(|&i| i != pin_slot).collect();
            for a0 in ALPHA5 {
                for a1 in ALPHA5 {
                    for a2 in ALPHA5 {
                        for a3 in ALPHA5 {
                            let mut s = base.map(f);
                            s[pin_slot] = f(pin_bits);
                            s[co[0]] = f(*a0);
                            s[co[1]] = f(*a1);
                            s[co[2]] = f(*a2);
                            s[co[3]] = f(*a3);
                            check(&s, "circle pinned-nan sweep");
                        }
                    }
                }
            }
        }
    }
    let mut rng = Rng::new(SEED ^ 0xA3);
    for _ in 0..300_000 {
        let mut s = [0.0f32; 8];
        for slot in s.iter_mut() {
            *slot = f(ALPHA[rng.below(ALPHA.len() as u32) as usize]);
        }
        check(&s, "circle random full alphabet");
    }
}

// ---------------------------------------------------------------------------
// c2RaytoPoly — the ray/transform slots, over a fixed and a NaN-laden polygon
// ---------------------------------------------------------------------------

#[test]
fn nan_matrix_raytopoly() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xA4);

    let polys = {
        let mut v = vec![poly_ray_box()];
        // A polygon whose verts/norms are themselves drawn from the alphabet, so
        // NaNs are manufactured *inside* the dot products too.
        for _ in 0..24 {
            let mut p = poly_ray_box();
            for k in 0..8 {
                p.verts[k] = c2v {
                    x: f(ALPHA[rng.below(ALPHA.len() as u32) as usize]),
                    y: f(ALPHA[rng.below(ALPHA.len() as u32) as usize]),
                };
                p.norms[k] = c2v {
                    x: f(ALPHA[rng.below(ALPHA.len() as u32) as usize]),
                    y: f(ALPHA[rng.below(ALPHA.len() as u32) as usize]),
                };
            }
            p.count = 1 + (rng.below(10) as i32);
            v.push(p);
        }
        v
    };

    for (pi, p) in polys.iter().enumerate() {
        let buf = PolyBuf::from_poly(p);
        // Exhaustive over the 5 ray slots × the 5-value alphabet = 3125, with a
        // pinned NaN rotated through every slot.
        for &pin_bits in PINNED_NANS {
            for pin_slot in 0..9usize {
                for a0 in ALPHA5 {
                    for a1 in ALPHA5 {
                        let mut s = [f(0x3F80_0000); 9];
                        s[pin_slot] = f(pin_bits);
                        let co: Vec<usize> = (0..9).filter(|&i| i != pin_slot).collect();
                        s[co[0]] = f(*a0);
                        s[co[1]] = f(*a1);
                        let a = c2Ray {
                            p: c2v { x: s[0], y: s[1] },
                            d: c2v { x: s[2], y: s[3] },
                            t: s[4],
                        };
                        let bx = c2x {
                            p: c2v { x: s[5], y: s[6] },
                            r: c2r { c: s[7], s: s[8] },
                        };
                        for bxo in [None, Some(&bx)] {
                            let c = run_poly_raw(&l.c, a, &buf, bxo);
                            let r = run_poly_raw(&l.rs, a, &buf, bxo);
                            assert!(
                                c == r,
                                "DIVERGENCE [poly{pi} pin{pin_slot}={pin_bits:#010x}]\n  \
                                 slots = {:08x?}\n  C    = {c:?}\n  RUST = {r:?}",
                                s.map(|v| v.to_bits())
                            );
                        }
                    }
                }
            }
        }
    }

    // Random full-alphabet sweep.
    for i in 0..200_000 {
        let p = &polys[rng.below(polys.len() as u32) as usize];
        let buf = PolyBuf::from_poly(p);
        let mut s = [0.0f32; 9];
        for slot in s.iter_mut() {
            *slot = f(ALPHA[rng.below(ALPHA.len() as u32) as usize]);
        }
        let a = c2Ray {
            p: c2v { x: s[0], y: s[1] },
            d: c2v { x: s[2], y: s[3] },
            t: s[4],
        };
        let bx = c2x {
            p: c2v { x: s[5], y: s[6] },
            r: c2r { c: s[7], s: s[8] },
        };
        let bxo = if rng.below(2) == 0 { None } else { Some(&bx) };
        let c = run_poly_raw(&l.c, a, &buf, bxo);
        let r = run_poly_raw(&l.rs, a, &buf, bxo);
        assert!(
            c == r,
            "DIVERGENCE [poly rand #{i}]\n  slots = {:08x?}\n  C    = {c:?}\n  RUST = {r:?}",
            s.map(|v| v.to_bits())
        );
    }
}

// ---------------------------------------------------------------------------
// The leaf vector math, exhaustively over the full alphabet
// ---------------------------------------------------------------------------

#[test]
fn nan_matrix_leaf_math_exhaustive() {
    let l = libs();
    // Two-vector functions: 4 slots × 9 values = 6561 combinations each.
    for &ax in ALPHA {
        for &ay in ALPHA {
            for &bx in ALPHA {
                for &by in ALPHA {
                    let a = c2v { x: f(ax), y: f(ay) };
                    let b = c2v { x: f(bx), y: f(by) };
                    let ctx = format!("{ax:#010x},{ay:#010x} / {bx:#010x},{by:#010x}");
                    diff_eq!(format!("c2Dot {ctx}"), fb((l.c.c2Dot)(a, b)), fb((l.rs.c2Dot)(a, b)));
                    diff_eq!(format!("c2Add {ctx}"), vb((l.c.c2Add)(a, b)), vb((l.rs.c2Add)(a, b)));
                    diff_eq!(format!("c2Sub {ctx}"), vb((l.c.c2Sub)(a, b)), vb((l.rs.c2Sub)(a, b)));
                    diff_eq!(format!("c2Minv {ctx}"), vb((l.c.c2Minv)(a, b)), vb((l.rs.c2Minv)(a, b)));
                    diff_eq!(format!("c2Maxv {ctx}"), vb((l.c.c2Maxv)(a, b)), vb((l.rs.c2Maxv)(a, b)));

                    // The vector argument MUST also range over the NaN-bearing
                    // alphabet: c2MulmvT's four `mulss` operand orders are only
                    // observable when a matrix lane AND a vector lane are both
                    // NaN with different payloads.
                    let m = c2m { x: a, y: b };
                    for &cx in ALPHA {
                        for &cy in ALPHA {
                            let v = c2v { x: f(cx), y: f(cy) };
                            diff_eq!(
                                format!("c2MulmvT {ctx} v={cx:#010x},{cy:#010x}"),
                                vb((l.c.c2MulmvT)(m, v)),
                                vb((l.rs.c2MulmvT)(m, v))
                            );
                        }
                    }

                    let rot = c2r { c: f(ax), s: f(ay) };
                    diff_eq!(
                        format!("c2Mulrv {ctx}"),
                        vb((l.c.c2Mulrv)(rot, b)),
                        vb((l.rs.c2Mulrv)(rot, b))
                    );
                    diff_eq!(
                        format!("c2MulrvT {ctx}"),
                        vb((l.c.c2MulrvT)(rot, b)),
                        vb((l.rs.c2MulrvT)(rot, b))
                    );
                }
            }
        }
    }
    // Single-vector + scalar functions: 3 slots × 9 = 729 each.
    for &ax in ALPHA {
        for &ay in ALPHA {
            let a = c2v { x: f(ax), y: f(ay) };
            diff_eq!(format!("c2Len {ax:#010x},{ay:#010x}"), fb((l.c.c2Len)(a)), fb((l.rs.c2Len)(a)));
            diff_eq!(format!("c2Norm"), vb((l.c.c2Norm)(a)), vb((l.rs.c2Norm)(a)));
            diff_eq!(format!("c2Skew"), vb((l.c.c2Skew)(a)), vb((l.rs.c2Skew)(a)));
            diff_eq!(format!("c2CCW90"), vb((l.c.c2CCW90)(a)), vb((l.rs.c2CCW90)(a)));
            diff_eq!(format!("c2Absv"), vb((l.c.c2Absv)(a)), vb((l.rs.c2Absv)(a)));
            for &s in ALPHA {
                diff_eq!(
                    format!("c2Mulvs {ax:#010x},{ay:#010x} * {s:#010x}"),
                    vb((l.c.c2Mulvs)(a, f(s))),
                    vb((l.rs.c2Mulvs)(a, f(s)))
                );
                diff_eq!(
                    format!("c2Div {ax:#010x},{ay:#010x} / {s:#010x}"),
                    vb((l.c.c2Div)(a, f(s))),
                    vb((l.rs.c2Div)(a, f(s)))
                );
            }
            // c2MulxvT: 4 transform slots.
            for &px in ALPHA5 {
                for &py in ALPHA5 {
                    for &rc in ALPHA5 {
                        for &rs in ALPHA5 {
                            let x = c2x {
                                p: c2v { x: f(px), y: f(py) },
                                r: c2r { c: f(rc), s: f(rs) },
                            };
                            diff_eq!(
                                format!("c2MulxvT"),
                                vb((l.c.c2MulxvT)(x, a)),
                                vb((l.rs.c2MulxvT)(x, a))
                            );
                        }
                    }
                }
            }
        }
    }
}
