//! Targeted coverage for the two arithmetic sites whose NaN operand selection
//! is observable but only reachable through a narrow conjunction of conditions:
//!
//!   * `c2RaytoAABB`:    `out->t = tK * A.t`
//!   * `c2RaytoCapsule`: `y = yAp.y + (yAe.y - yAp.y) * t`  and  `out->t = t * A.t`
//!
//! A random sweep reaches "both operands are distinct NaNs" here only rarely,
//! which let a wrong operand choice survive mutation testing. These tests build
//! the required states directly, using NaNs whose payloads identify their
//! source, so the surviving payload pins the choice.

#![allow(non_snake_case)]

mod common;
use common::*;

/// NaN carrying an identifiable payload.
fn tag(id: u32, neg: bool) -> f32 {
    f32::from_bits((if neg { 0xffc0_0000 } else { 0x7fc0_0000 }) | id)
}

/// `c2RaytoAABB` writes `out->t` only when at least one `t_i <= 1.0f`. With a
/// NaN `A.t` every `t_i` derived from `p1` is NaN, so the ONLY way to get there
/// is via the `da < 0 -> return 0.0f` exit of `c2RayToPlane_OneDimensional`,
/// which depends on `p0` alone. Placing `p0` outside the box on one axis
/// supplies that, while `A.t` and `A.d` stay NaN with different payloads.
#[test]
fn aabb_out_t_operand_selection() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut reached = 0usize;
    unsafe {
        let boxes = [
            c2AABB { min: c2v { x: -1.0, y: -1.0 }, max: c2v { x: 1.0, y: 1.0 } },
            c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 4.0, y: 2.0 } },
            c2AABB { min: c2v { x: -8.0, y: 3.0 }, max: c2v { x: -2.0, y: 9.0 } },
            c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } },
        ];
        // p0 placed on each side of the box (so some `da` is negative), and on
        // the corners / centre for good measure.
        let origins = |B: &c2AABB| {
            let cx = (B.min.x + B.max.x) * 0.5;
            let cy = (B.min.y + B.max.y) * 0.5;
            vec![
                c2v { x: B.min.x - 3.0, y: cy },
                c2v { x: B.max.x + 3.0, y: cy },
                c2v { x: cx, y: B.min.y - 3.0 },
                c2v { x: cx, y: B.max.y + 3.0 },
                c2v { x: cx, y: cy },
                B.min,
                B.max,
                c2v { x: B.min.x - 3.0, y: B.max.y + 3.0 },
            ]
        };
        // every pairing of two distinguishable NaNs, in both slots
        let nans = [
            tag(0x1111, false),
            tag(0x1111, true),
            tag(0x2222, false),
            tag(0x2222, true),
            f32::NAN,
            -f32::NAN,
            f32::from_bits(0x7f80_5555), // SNaN
            f32::from_bits(0xff80_5555),
        ];
        for B in boxes {
            for o in origins(&B) {
                for &tn in &nans {
                    for &dn in &nans {
                        for dpat in 0..4 {
                            let dir = match dpat {
                                0 => c2v { x: dn, y: dn },
                                1 => c2v { x: dn, y: 1.0 },
                                2 => c2v { x: 1.0, y: dn },
                                _ => c2v { x: 0.0, y: dn },
                            };
                            let A = c2Ray { p: o, d: dir, t: tn };
                            let cr = call_aabb(&p.c, A, B);
                            if cr.0 != 0 {
                                reached += 1;
                            }
                            d.ray("aabb out.t operand selection", cr, call_aabb(&p.rs, A, B));
                        }
                    }
                }
                // finite A.t with NaN direction, and vice versa
                for &n in &nans {
                    for t in [0.0f32, 0.5, 1.0, 2.0, 1.0e30] {
                        let A = c2Ray { p: o, d: c2v { x: n, y: n }, t };
                        let cr = call_aabb(&p.c, A, B);
                        if cr.0 != 0 {
                            reached += 1;
                        }
                        d.ray("aabb out.t (finite t)", cr, call_aabb(&p.rs, A, B));
                    }
                    let A = c2Ray { p: c2v { x: n, y: o.y }, d: c2v { x: 1.0, y: 0.0 }, t: n };
                    let cr = call_aabb(&p.c, A, B);
                    if cr.0 != 0 {
                        reached += 1;
                    }
                    d.ray("aabb out.t (NaN p0.x)", cr, call_aabb(&p.rs, A, B));
                }
            }
        }
    }
    assert!(
        reached > 0,
        "never reached the `out->t = tK * A.t` write with pathological inputs"
    );
    eprintln!("aabb: reached the out->t write {reached} times");
    d.finish("targeted: c2RaytoAABB out->t operand selection");
}

/// `c2RaytoCapsule`'s side-wall branch computes
/// `y = yAp.y + (yAe.y - yAp.y) * t` and then `out->t = t * A.t`. Both are
/// commutative sites with independently derived operands. A real capsule plus a
/// ray whose origin is finite (so the branch is entered) but whose direction
/// and/or `A.t` are distinguishable NaNs drives them.
#[test]
fn capsule_wall_operand_selection() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut reached = 0usize;
    let mut rng = Rng::new(0x0A11_0000);
    unsafe {
        let nans = [
            tag(0x1111, false),
            tag(0x1111, true),
            tag(0x2222, false),
            tag(0x2222, true),
            f32::NAN,
            -f32::NAN,
            f32::from_bits(0x7f80_5555),
            f32::from_bits(0xff80_5555),
        ];
        let caps = [
            c2Capsule { a: c2v { x: -2.0, y: 0.0 }, b: c2v { x: 2.0, y: 0.0 }, r: 1.0 },
            c2Capsule { a: c2v { x: 0.0, y: -3.0 }, b: c2v { x: 0.0, y: 3.0 }, r: 0.5 },
            c2Capsule { a: c2v { x: -1.0, y: -1.0 }, b: c2v { x: 4.0, y: 2.5 }, r: 2.0 },
            c2Capsule { a: c2v { x: 0.0, y: 0.0 }, b: c2v { x: 1.0, y: 0.0 }, r: 4.0 },
        ];
        for B in caps {
            // origins on the far side of the wall (|local x| > r) so the
            // side-wall branch is the one taken
            let origins = [
                c2v { x: B.a.x, y: B.a.y + 6.0 },
                c2v { x: B.a.x, y: B.a.y - 6.0 },
                c2v { x: B.a.x + 6.0, y: B.a.y },
                c2v { x: B.a.x - 6.0, y: B.a.y },
                c2v { x: B.b.x + 6.0, y: B.b.y + 6.0 },
            ];
            for o in origins {
                for &tn in &nans {
                    for &dn in &nans {
                        for dpat in 0..5 {
                            let dir = match dpat {
                                0 => c2v { x: dn, y: dn },
                                1 => c2v { x: dn, y: 1.0 },
                                2 => c2v { x: 1.0, y: dn },
                                3 => c2v { x: -1.0, y: dn },
                                _ => c2v { x: dn, y: -1.0 },
                            };
                            for t in [tn, 1.0, 10.0] {
                                let A = c2Ray { p: o, d: dir, t };
                                let cr = call_capsule(&p.c, A, B);
                                if cr.0 != 0 {
                                    reached += 1;
                                }
                                d.ray("capsule wall operand selection", cr, call_capsule(&p.rs, A, B));
                            }
                        }
                    }
                }
                // finite geometry that genuinely hits the wall, then one input
                // at a time replaced by a NaN
                let toward = c2v { x: B.a.x - o.x, y: B.a.y - o.y };
                let l = (toward.x * toward.x + toward.y * toward.y).sqrt();
                let dirn = c2v { x: toward.x / l, y: toward.y / l };
                for &n in &nans {
                    let variants = [
                        c2Ray { p: o, d: dirn, t: n },
                        c2Ray { p: o, d: c2v { x: n, y: dirn.y }, t: l },
                        c2Ray { p: o, d: c2v { x: dirn.x, y: n }, t: l },
                        c2Ray { p: c2v { x: n, y: o.y }, d: dirn, t: l },
                        c2Ray { p: c2v { x: o.x, y: n }, d: dirn, t: n },
                    ];
                    for A in variants {
                        let cr = call_capsule(&p.c, A, B);
                        if cr.0 != 0 {
                            reached += 1;
                        }
                        d.ray("capsule wall (one NaN)", cr, call_capsule(&p.rs, A, B));
                    }
                }
            }
        }
        // plus a randomized wall-hit sweep with occasional NaN injection
        for _ in 0..60_000 {
            let a = rng.v_small();
            let ang = rng.range(-7.0, 7.0);
            let len = rng.range(0.3, 10.0);
            let B = c2Capsule {
                a,
                b: c2v { x: a.x + len * ang.cos(), y: a.y + len * ang.sin() },
                r: rng.range(0.05, 4.0),
            };
            let side = if rng.bool() { 1.0f32 } else { -1.0 };
            let mx = c2v { x: ang.sin(), y: -ang.cos() };
            let o = c2v {
                x: a.x + mx.x * side * (B.r + rng.range(0.1, 10.0)),
                y: a.y + mx.y * side * (B.r + rng.range(0.1, 10.0)),
            };
            let mut dir = rng.v_dir();
            let mut t = rng.range(0.0, 40.0);
            match rng.below(4) {
                0 => dir.x = nans[rng.below(nans.len())],
                1 => dir.y = nans[rng.below(nans.len())],
                2 => t = nans[rng.below(nans.len())],
                _ => {}
            }
            let A = c2Ray { p: o, d: dir, t };
            let cr = call_capsule(&p.c, A, B);
            if cr.0 != 0 {
                reached += 1;
            }
            d.ray("capsule wall (randomized + NaN)", cr, call_capsule(&p.rs, A, B));
        }
    }
    assert!(reached > 0, "never returned a hit from the capsule wall branch");
    eprintln!("capsule: reached a hit {reached} times");
    d.finish("targeted: c2RaytoCapsule wall operand selection");
}
