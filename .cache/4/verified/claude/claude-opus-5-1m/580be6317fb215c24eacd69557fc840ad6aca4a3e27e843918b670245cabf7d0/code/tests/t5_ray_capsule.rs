//! Phase B rows B42..B56 and Phase C rows E20..E22 for `c2RaytoCapsule`.
//!
//! `c2RaytoCapsule` has ten distinct outcomes.  Instead of hand-deriving
//! geometry for each, the test re-implements the C's branch predicate using the
//! C library's OWN exported leaf functions (`c2Norm`, `c2MulmvT`,
//! `c2AABBtoPoint`, `c2CircleToPoint`, ...), classifies every randomized input,
//! and then asserts that every branch was reached many times.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::collections::HashMap;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum Branch {
    /// `c2AABBtoPoint(capsule_bb, yAp)` -> early `return 1`
    BbInside,
    /// `c2CircleToPoint(capsule_a, A.p)` -> `return 1`
    InCircA,
    /// `c2CircleToPoint(capsule_b, A.p)` -> `return 1`
    InCircB,
    /// `|yAp.x| < r`, `yAp.y < 0` -> `c2RaytoCircle(Ca)`
    CapA,
    /// `|yAp.x| < r`, `yAp.y >= 0` -> `c2RaytoCircle(Cb)`
    CapB,
    /// else-branch, `y <= 0` -> `c2RaytoCircle(Ca)`
    YLow,
    /// else-branch, `y >= yBb.y` -> `c2RaytoCircle(Cb)`
    YHigh,
    /// else-branch side hit, `c > 0` -> `out->n = M.x`
    SidePos,
    /// else-branch side hit, `c <= 0` -> `out->n = c2Skew(M.y)`
    SideNeg,
    /// big condition false -> `return 0`
    NoHit,
}

fn m_abs(a: f32) -> f32 {
    if a < 0.0 { -a } else { a }
}
fn m_min(a: f32, b: f32) -> f32 {
    if a < b { a } else { b }
}

/// Mirror of the C control flow, computed with the C library's own exports.
fn classify(api: &Api, A: c2Ray, B: c2Capsule) -> Branch {
    let my = (api.c2Norm)((api.c2Sub)(B.b, B.a));
    let mx = (api.c2CCW90)(my);
    let m = c2m { x: mx, y: my };
    let cap_n = (api.c2Sub)(B.b, B.a);
    let ybb = (api.c2MulmvT)(m, cap_n);
    let yap = (api.c2MulmvT)(m, (api.c2Sub)(A.p, B.a));
    let yad = (api.c2MulmvT)(m, A.d);
    let yae = (api.c2Add)(yap, (api.c2Mulvs)(yad, A.t));
    let bb = c2AABB {
        min: (api.c2V)(-B.r, 0.0),
        max: (api.c2V)(B.r, ybb.y),
    };
    if (api.c2AABBtoPoint)(bb, yap) != 0 {
        return Branch::BbInside;
    }
    if (api.c2CircleToPoint)(c2Circle { p: B.a, r: B.r }, A.p) != 0 {
        return Branch::InCircA;
    }
    if (api.c2CircleToPoint)(c2Circle { p: B.b, r: B.r }, A.p) != 0 {
        return Branch::InCircB;
    }
    if yae.x * yap.x < 0.0 || m_min(m_abs(yae.x), m_abs(yap.x)) < B.r {
        if m_abs(yap.x) < B.r {
            if yap.y < 0.0 {
                return Branch::CapA;
            }
            return Branch::CapB;
        }
        let c = if yap.x > 0.0 { B.r } else { -B.r };
        let dd = yae.x - yap.x;
        let t = (c - yap.x) / dd;
        let y = yap.y + (yae.y - yap.y) * t;
        if y <= 0.0 {
            return Branch::YLow;
        }
        if y >= ybb.y {
            return Branch::YHigh;
        }
        if c > 0.0 {
            return Branch::SidePos;
        }
        return Branch::SideNeg;
    }
    Branch::NoHit
}

fn both(d: &mut Diff, label: &str, a: c2Ray, b: c2Capsule) -> RayResult {
    let (c, r) = apis();
    let rc = call_capsule(c, a, b);
    let rr = call_capsule(r, a, b);
    d.ray(label, || format!("{:?} {:?}", a, b), rc, rr);
    rc
}

/// Random capsule + a ray aimed near it, so that all ten branches occur.
fn gen_case(rng: &mut Rng, style: u32) -> (c2Ray, c2Capsule) {
    let a = c2v {
        x: rng.uniform(20.0),
        y: rng.uniform(20.0),
    };
    let axis = rng.unit();
    let len = (rng.uniform(20.0)).abs() + 0.01;
    let b = c2v {
        x: a.x + axis.x * len,
        y: a.y + axis.y * len,
    };
    let r = (rng.uniform(6.0)).abs() + 0.01;
    let cap = c2Capsule { a, b, r };
    let mid = c2v {
        x: (a.x + b.x) * 0.5,
        y: (a.y + b.y) * 0.5,
    };
    let perp = c2v { x: -axis.y, y: axis.x };
    let ray = match style % 6 {
        // origin inside the rectangle part
        0 => c2Ray {
            p: c2v {
                x: mid.x + perp.x * r * 0.5,
                y: mid.y + perp.y * r * 0.5,
            },
            d: rng.unit(),
            t: rng.uniform(20.0),
        },
        // origin just outside an end cap along the axis
        1 => {
            let s = if rng.below(2) == 0 { -1.0 } else { 1.0 };
            let base = if s < 0.0 { a } else { b };
            c2Ray {
                p: c2v {
                    x: base.x + axis.x * s * r * 0.5,
                    y: base.y + axis.y * s * r * 0.5,
                },
                d: rng.unit(),
                t: rng.uniform(20.0),
            }
        }
        // origin beyond an end, laterally inside the band (|yAp.x| < r)
        2 => {
            let s = if rng.below(2) == 0 { -1.0 } else { 1.0 };
            let base = if s < 0.0 { a } else { b };
            let lateral = rng.uniform(r * 0.9);
            c2Ray {
                p: c2v {
                    x: base.x + axis.x * s * (r + 0.5 + (rng.uniform(10.0)).abs())
                        + perp.x * lateral,
                    y: base.y + axis.y * s * (r + 0.5 + (rng.uniform(10.0)).abs())
                        + perp.y * lateral,
                },
                d: rng.unit(),
                t: (rng.uniform(30.0)).abs(),
            }
        }
        // origin laterally outside, ray crossing the band
        3 => {
            let side = if rng.below(2) == 0 { -1.0 } else { 1.0 };
            let dist = r + 0.5 + (rng.uniform(10.0)).abs();
            let along = rng.uniform(len * 1.5);
            let p = c2v {
                x: a.x + axis.x * along + perp.x * side * dist,
                y: a.y + axis.y * along + perp.y * side * dist,
            };
            let target = c2v {
                x: a.x + axis.x * rng.uniform(len * 2.0),
                y: a.y + axis.y * rng.uniform(len * 2.0),
            };
            let dx = target.x - p.x;
            let dy = target.y - p.y;
            let l = (dx * dx + dy * dy).sqrt().max(1e-6);
            c2Ray {
                p,
                d: c2v { x: dx / l, y: dy / l },
                t: l * (0.5 + (rng.uniform(2.0)).abs()),
            }
        }
        // far away, pointing away
        4 => c2Ray {
            p: c2v {
                x: mid.x + perp.x * (r + 20.0),
                y: mid.y + perp.y * (r + 20.0),
            },
            d: perp,
            t: (rng.uniform(20.0)).abs(),
        },
        // fully random
        _ => c2Ray {
            p: rng.vec_nice(),
            d: rng.unit(),
            t: rng.nice(),
        },
    };
    (ray, cap)
}

/// B42..B51 + E20 + E21: every one of the ten branches, differentially tested
/// over many randomized inputs each.
#[test]
fn b42_b51_e20_e21_all_branches() {
    let (c, _) = apis();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB42);
    let mut counts: HashMap<Branch, usize> = HashMap::new();
    for i in 0..200_000u32 {
        let (a, b) = gen_case(&mut rng, i);
        let br = classify(c, a, b);
        *counts.entry(br).or_insert(0) += 1;
        both(&mut d, &format!("B42-51/{:?}", br), a, b);
    }
    let all = [
        Branch::BbInside,
        Branch::InCircA,
        Branch::InCircB,
        Branch::CapA,
        Branch::CapB,
        Branch::YLow,
        Branch::YHigh,
        Branch::SidePos,
        Branch::SideNeg,
        Branch::NoHit,
    ];
    eprintln!("branch coverage: {:?}", counts);
    for br in all {
        let n = counts.get(&br).copied().unwrap_or(0);
        assert!(
            n >= 50,
            "branch {:?} only reached {} times (need >= 50); counts={:?}",
            br,
            n,
            counts
        );
    }
    d.finish("B42-B51/E20/E21 c2RaytoCapsule all branches");
}

/// B52 + E22: degenerate capsule `a == b` => `c2Norm((0,0))` => NaN basis.
#[test]
fn b52_e22_degenerate_capsule() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB52);
    for _ in 0..20_000 {
        let p = rng.vec_nice();
        let cap = c2Capsule {
            a: p,
            b: p,
            r: (rng.uniform(10.0)).abs(),
        };
        let a = c2Ray {
            p: rng.vec_nice(),
            d: rng.unit(),
            t: rng.nice(),
        };
        both(&mut d, "B52", a, cap);
    }
    // exact-zero and signed-zero degenerate variants
    for r in [0.0f32, 1.0, -1.0, f32::NAN, f32::INFINITY] {
        for cap in [
            c2Capsule {
                a: c2v { x: 0.0, y: 0.0 },
                b: c2v { x: 0.0, y: 0.0 },
                r,
            },
            c2Capsule {
                a: c2v { x: -0.0, y: -0.0 },
                b: c2v { x: 0.0, y: 0.0 },
                r,
            },
            c2Capsule {
                a: c2v { x: 1.0, y: 2.0 },
                b: c2v { x: 1.0, y: 2.0 },
                r,
            },
        ] {
            for t in [0.0f32, 1.0, -1.0, f32::INFINITY, f32::NAN] {
                let a = c2Ray {
                    p: c2v { x: 3.0, y: 4.0 },
                    d: c2v { x: 1.0, y: 0.0 },
                    t,
                };
                both(&mut d, "B52/exact", a, cap);
            }
        }
    }
    d.finish("B52/E22 c2RaytoCapsule degenerate capsule");
}

/// B53: degenerate / inverted `capsule_bb`.
///
/// `capsule_bb = {min: (-r, 0), max: (r, yBb.y)}` and
/// `yBb.y = c2Dot(c2Norm(b-a), b-a) == |b-a| >= 0`, so a *negative* `yBb.y` is
/// UNREACHABLE for finite capsules however `a` and `b` are ordered (asserted
/// below over 40 000 trials).  The reachable degenerate forms are:
///   * `yBb.y == 0`  (`a == b`, covered by B52),
///   * `yBb.y == NaN` (`a == b` or infinite coordinates),
///   * x-inverted `capsule_bb` when `r < 0` (`min.x = -r > r = max.x`).
/// All three are exercised here in addition to the swap-order sweep.
#[test]
fn b53_inverted_capsule_bb() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB53);
    let mut inverted = 0;
    let (c, _) = apis();
    for _ in 0..20_000 {
        // Swapping a and b flips the sign of yBb.y for some configurations;
        // build both orders and keep the ones with yBb.y < 0.
        let p = rng.vec_nice();
        let axis = rng.unit();
        let len = (rng.uniform(20.0)).abs() + 0.01;
        let q = c2v {
            x: p.x + axis.x * len,
            y: p.y + axis.y * len,
        };
        for cap in [
            c2Capsule {
                a: p,
                b: q,
                r: (rng.uniform(5.0)).abs() + 0.01,
            },
            c2Capsule {
                a: q,
                b: p,
                r: (rng.uniform(5.0)).abs() + 0.01,
            },
        ] {
            let my = (c.c2Norm)((c.c2Sub)(cap.b, cap.a));
            let mm = c2m {
                x: (c.c2CCW90)(my),
                y: my,
            };
            let ybb = (c.c2MulmvT)(mm, (c.c2Sub)(cap.b, cap.a));
            if ybb.y < 0.0 {
                inverted += 1;
            }
            let a = c2Ray {
                p: rng.vec_nice(),
                d: rng.unit(),
                t: (rng.uniform(30.0)).abs(),
            };
            both(&mut d, "B53", a, cap);
        }
    }
    assert_eq!(
        inverted, 0,
        "yBb.y = |b-a| cannot be negative; the C's capsule_bb is never \
         y-inverted for finite capsules"
    );

    // x-inverted capsule_bb (r < 0) and NaN yBb.y, differentially tested.
    let mut ybb_nan = 0;
    let mut x_inverted = 0;
    for i in 0..20_000u32 {
        let (a, cap) = gen_case(&mut rng, i);
        for r in [-1.0f32, -0.0, -1e30, f32::NEG_INFINITY] {
            let cp = c2Capsule { r, ..cap };
            if -r > r {
                x_inverted += 1;
            }
            both(&mut d, "B53/x-inverted", a, cp);
        }
        for cp in [
            c2Capsule { b: cap.a, ..cap },
            c2Capsule {
                a: c2v {
                    x: f32::INFINITY,
                    y: 0.0,
                },
                ..cap
            },
            c2Capsule {
                b: c2v {
                    x: f32::NAN,
                    y: 0.0,
                },
                ..cap
            },
        ] {
            let my = (c.c2Norm)((c.c2Sub)(cp.b, cp.a));
            let mm = c2m {
                x: (c.c2CCW90)(my),
                y: my,
            };
            let ybb = (c.c2MulmvT)(mm, (c.c2Sub)(cp.b, cp.a));
            if ybb.y.is_nan() {
                ybb_nan += 1;
            }
            both(&mut d, "B53/nan-ybb", a, cp);
        }
    }
    assert!(x_inverted > 0, "no x-inverted capsule_bb cases");
    assert!(ybb_nan > 0, "no NaN yBb.y cases");
    d.finish("B53 c2RaytoCapsule degenerate/inverted bb");
}

/// B54 + E33: radius `0`, `-0.0`, negative, infinite, NaN.
#[test]
fn b54_e33_radius_variants() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB54);
    let radii = [
        0.0f32,
        -0.0,
        1e-30,
        -1.0,
        -1e30,
        1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
        f32::MIN_POSITIVE,
    ];
    for i in 0..2_000u32 {
        let (a, cap) = gen_case(&mut rng, i);
        for &r in &radii {
            both(&mut d, "B54", a, c2Capsule { r, ..cap });
        }
    }
    d.finish("B54/E33 c2RaytoCapsule radius variants");
}

/// B55: a special value in every individual field position.
#[test]
fn b55_specials_per_field() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB55);
    for i in 0..200u32 {
        let (base, cap) = gen_case(&mut rng, i);
        for &s in &SPECIALS {
            both(&mut d, "B55/A.p.x", c2Ray { p: c2v { x: s, ..base.p }, ..base }, cap);
            both(&mut d, "B55/A.p.y", c2Ray { p: c2v { y: s, ..base.p }, ..base }, cap);
            both(&mut d, "B55/A.d.x", c2Ray { d: c2v { x: s, ..base.d }, ..base }, cap);
            both(&mut d, "B55/A.d.y", c2Ray { d: c2v { y: s, ..base.d }, ..base }, cap);
            both(&mut d, "B55/A.t", c2Ray { t: s, ..base }, cap);
            both(&mut d, "B55/B.a.x", base, c2Capsule { a: c2v { x: s, ..cap.a }, ..cap });
            both(&mut d, "B55/B.a.y", base, c2Capsule { a: c2v { y: s, ..cap.a }, ..cap });
            both(&mut d, "B55/B.b.x", base, c2Capsule { b: c2v { x: s, ..cap.b }, ..cap });
            both(&mut d, "B55/B.b.y", base, c2Capsule { b: c2v { y: s, ..cap.b }, ..cap });
            both(&mut d, "B55/B.r", base, c2Capsule { r: s, ..cap });
        }
    }
    d.finish("B55 c2RaytoCapsule specials per field");
}

/// B56: unconstrained fuzz.
#[test]
fn b56_fuzz() {
    let mut d = Diff::new();
    let mut rng = Rng::new(0xB56);
    for _ in 0..20_000 {
        both(&mut d, "B56/nice", rng.ray_nice(), rng.capsule_nice());
    }
    for _ in 0..20_000 {
        both(&mut d, "B56/hostile", rng.ray_hostile(), rng.capsule_hostile());
    }
    for _ in 0..20_000 {
        both(&mut d, "B56/mix1", rng.ray_nice(), rng.capsule_hostile());
        both(&mut d, "B56/mix2", rng.ray_hostile(), rng.capsule_nice());
    }
    d.finish("B56 c2RaytoCapsule fuzz");
}
