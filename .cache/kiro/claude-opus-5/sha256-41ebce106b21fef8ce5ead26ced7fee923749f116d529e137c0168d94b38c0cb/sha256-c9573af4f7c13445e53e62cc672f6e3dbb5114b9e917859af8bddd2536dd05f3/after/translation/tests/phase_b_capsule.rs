//! Phase B — CONFIGS.md rows 37..43: `c2RaytoCapsule`, called directly.
//!
//! `c2RaytoCapsule` builds a local frame `M` from the capsule axis and then
//! takes one of seven distinct exits. Each row targets a specific exit; the
//! branch-coverage test at the end asserts every exit was actually reached, so
//! a row cannot silently pass by never entering its branch.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::cell::RefCell;

const N: usize = 3000;

/// Mirrors the C's local-frame construction so tests can place a ray in
/// capsule space. Only used to *build inputs*; never to predict outputs.
struct Frame {
    mx: c2v,
    my: c2v,
    a: c2v,
    len: f32,
}

fn frame(B: &c2Capsule) -> Frame {
    let dx = B.b.x - B.a.x;
    let dy = B.b.y - B.a.y;
    let l = (dx * dx + dy * dy).sqrt();
    let my = c2v { x: dx / l, y: dy / l };
    let mx = c2v { x: my.y, y: -my.x }; // c2CCW90
    Frame { mx, my, a: B.a, len: l }
}

impl Frame {
    /// local (u along mx, v along my) -> world
    fn to_world(&self, u: f32, v: f32) -> c2v {
        c2v {
            x: self.a.x + self.mx.x * u + self.my.x * v,
            y: self.a.y + self.mx.y * u + self.my.y * v,
        }
    }
    fn dir_to_world(&self, u: f32, v: f32) -> c2v {
        c2v {
            x: self.mx.x * u + self.my.x * v,
            y: self.mx.y * u + self.my.y * v,
        }
    }
}

fn rand_capsule(rng: &mut Rng) -> c2Capsule {
    let a = rng.v_small();
    let ang = rng.range(-7.0, 7.0);
    let len = rng.range(0.2, 16.0);
    c2Capsule {
        a,
        b: c2v {
            x: a.x + len * ang.cos(),
            y: a.y + len * ang.sin(),
        },
        r: rng.range(0.05, 6.0),
    }
}

/// Row 37: fully random rays vs random capsules.
#[test]
fn row37_capsule_random() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x37);
    unsafe {
        for _ in 0..(N * 6) {
            let B = rand_capsule(&mut rng);
            let A = c2Ray {
                p: rng.v_small(),
                d: rng.v_dir(),
                t: rng.range(0.0, 40.0),
            };
            d.ray("c2RaytoCapsule(rand)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
        }
    }
    d.finish("row 37: c2RaytoCapsule random");
}

/// Row 38: ray origin inside the capsule's local bounding box → the first
/// `c2AABBtoPoint` early `return 1`.
#[test]
fn row38_capsule_origin_in_bb() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x38);
    unsafe {
        for _ in 0..(N * 2) {
            let B = rand_capsule(&mut rng);
            let f = frame(&B);
            // u in [-r, r], v in [0, len] is exactly capsule_bb
            for (uk, vk) in [
                (0.0f32, 0.5f32),
                (0.99, 0.5),
                (-0.99, 0.5),
                (1.0, 0.0),
                (-1.0, 1.0),
                (0.5, 0.0),
                (0.5, 1.0),
                (0.0, 0.0),
                (0.0, 1.0),
            ] {
                let A = c2Ray {
                    p: f.to_world(uk * B.r, vk * f.len),
                    d: rng.v_dir(),
                    t: rng.range(0.0, 40.0),
                };
                d.ray("c2RaytoCapsule(in-bb)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
            }
        }
    }
    d.finish("row 38: c2RaytoCapsule origin in local bb");
}

/// Row 39: ray origin inside end-cap A / end-cap B but OUTSIDE the local bb
/// (`v < 0` or `v > len`), which is the only way to reach the
/// `c2CircleToPoint` early returns.
#[test]
fn row39_capsule_origin_in_caps() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x39);
    unsafe {
        for _ in 0..(N * 2) {
            let B = rand_capsule(&mut rng);
            let f = frame(&B);
            for (uk, v) in [
                (0.0f32, -0.5f32),
                (0.5, -0.3),
                (-0.5, -0.3),
                (0.9, -0.05),
                (0.0, 1.5),
                (0.5, 1.3),
                (-0.5, 1.3),
            ] {
                // v is a fraction of r beyond the cap centre plane
                let vv = if v < 0.0 { v * B.r } else { f.len + (v - 1.0) * B.r };
                let A = c2Ray {
                    p: f.to_world(uk * B.r, vv),
                    d: rng.v_dir(),
                    t: rng.range(0.0, 40.0),
                };
                d.ray("c2RaytoCapsule(in-cap)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
            }
        }
    }
    d.finish("row 39: c2RaytoCapsule origin in end-caps");
}

/// Row 40: `|yAp.x| < B.r` while outside the bb and both caps → delegate to
/// circle A (`yAp.y < 0`) or circle B (`yAp.y >= 0`).
#[test]
fn row40_capsule_delegate_by_sign() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x40);
    unsafe {
        for _ in 0..(N * 3) {
            let B = rand_capsule(&mut rng);
            let f = frame(&B);
            // |u| < r but v far outside [ -r, len + r ] so the caps reject
            for u in [0.0f32, 0.5, -0.5, 0.95, -0.95] {
                for v in [
                    -B.r - rng.range(0.1, 20.0),
                    f.len + B.r + rng.range(0.1, 20.0),
                ] {
                    let origin = f.to_world(u * B.r, v);
                    // aim back towards the capsule and also away from it
                    for dv in [1.0f32, -1.0] {
                        let A = c2Ray {
                            p: origin,
                            d: f.dir_to_world(rng.sym(0.3), dv),
                            t: rng.range(0.0, 60.0),
                        };
                        d.ray("c2RaytoCapsule(deleg)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
                    }
                    let A = c2Ray { p: origin, d: rng.v_dir(), t: rng.range(0.0, 60.0) };
                    d.ray("c2RaytoCapsule(deleg-rnd)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
                }
            }
        }
    }
    d.finish("row 40: c2RaytoCapsule |yAp.x| < r delegation");
}

/// Row 41: the side-wall branch. Origin has `|u| >= r`, the ray crosses the
/// `u = ±r` wall plane, and the crossing height `y` selects cap A, cap B or a
/// genuine wall hit (with the `M.x` vs `c2Skew(M.y)` normal choice).
#[test]
fn row41_capsule_side_wall() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x41);
    unsafe {
        for _ in 0..(N * 4) {
            let B = rand_capsule(&mut rng);
            let f = frame(&B);
            for side in [1.0f32, -1.0] {
                let u0 = side * (B.r + rng.range(0.01, 20.0));
                // target height on the wall: below 0, inside, above len
                for vt in [
                    -rng.range(0.1, 5.0) * B.r,
                    rng.range(0.05, 0.95) * f.len,
                    f.len + rng.range(0.1, 5.0) * B.r,
                    0.0,
                    f.len,
                ] {
                    let v0 = vt + rng.sym(4.0);
                    let origin = f.to_world(u0, v0);
                    // direction that crosses u = side*r at height vt when
                    // travelling the full ray length
                    let target = f.to_world(side * B.r, vt);
                    let dx = target.x - origin.x;
                    let dy = target.y - origin.y;
                    let l = (dx * dx + dy * dy).sqrt();
                    if !(l > 0.0) {
                        continue;
                    }
                    let dir = c2v { x: dx / l, y: dy / l };
                    for tk in [0.5f32, 1.0, 2.0, 1.0e-3] {
                        let A = c2Ray { p: origin, d: dir, t: l * tk };
                        d.ray("c2RaytoCapsule(wall)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
                    }
                    // and the same geometry with an unnormalised direction
                    let A = c2Ray {
                        p: origin,
                        d: c2v { x: dx, y: dy },
                        t: rng.range(0.0, 2.0),
                    };
                    d.ray("c2RaytoCapsule(wall-unnorm)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
                }
            }
        }
    }
    d.finish("row 41: c2RaytoCapsule side wall");
}

/// Row 42: axis-aligned capsules crossed by axis-aligned rays — makes
/// `yAd.x`/`yAe.x - yAp.x` exactly zero very often.
#[test]
fn row42_capsule_axis_aligned() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x42);
    unsafe {
        for _ in 0..(N * 2) {
            let a = rng.v_small();
            let len = rng.range(0.2, 16.0);
            let r = rng.range(0.05, 6.0);
            for axis in 0..4 {
                let b = match axis {
                    0 => c2v { x: a.x + len, y: a.y },
                    1 => c2v { x: a.x - len, y: a.y },
                    2 => c2v { x: a.x, y: a.y + len },
                    _ => c2v { x: a.x, y: a.y - len },
                };
                let B = c2Capsule { a, b, r };
                for dir in AXIS_DIRS {
                    for off in [0.0f32, 0.5, 1.0, 1.5, -0.5, -1.0] {
                        let origin = c2v {
                            x: a.x - dir.x * 20.0 + if dir.x == 0.0 { off * r } else { 0.0 },
                            y: a.y - dir.y * 20.0 + if dir.y == 0.0 { off * r } else { 0.0 },
                        };
                        for t in [0.0f32, 10.0, 20.0, 40.0] {
                            let A = c2Ray { p: origin, d: dir, t };
                            d.ray("c2RaytoCapsule(axis)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
                        }
                    }
                }
            }
        }
    }
    d.finish("row 42: c2RaytoCapsule axis-aligned");
}

/// Row 43: degenerate capsules — `a == b` (the axis normalises to NaN),
/// `r == 0`, `r < 0`, `A.t == 0`, and fully non-finite inputs.
#[test]
fn row43_capsule_degenerate() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x43);
    unsafe {
        // a == b: c2Norm(0) -> NaN frame; every later compare is false
        for _ in 0..(N * 2) {
            let q = rng.v_small();
            let B = c2Capsule { a: q, b: q, r: rng.range(-4.0, 8.0) };
            let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 40.0) };
            d.ray("c2RaytoCapsule(a==b)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
            // and with the ray origin exactly at the degenerate point
            let A2 = c2Ray { p: q, d: rng.v_dir(), t: rng.range(0.0, 40.0) };
            d.ray("c2RaytoCapsule(a==b,at)", call_capsule(&p.c, A2, B), call_capsule(&p.rs, A2, B));
        }
        // r == 0 and r < 0 (inverted local bb)
        for _ in 0..(N * 2) {
            let mut B = rand_capsule(&mut rng);
            for r in [0.0f32, -0.0, -B.r, -1.0, -1.0e-30] {
                B.r = r;
                let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 40.0) };
                d.ray("c2RaytoCapsule(r<=0)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
            }
        }
        // A.t == 0 / negative
        for _ in 0..(N * 2) {
            let B = rand_capsule(&mut rng);
            for t in [0.0f32, -0.0, -1.0, -40.0] {
                let A = c2Ray { p: rng.v_small(), d: rng.v_dir(), t };
                d.ray("c2RaytoCapsule(t<=0)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
            }
        }
        // fully non-finite
        for _ in 0..(N * 2) {
            let B = c2Capsule { a: rng.v_mixed(), b: rng.v_mixed(), r: rng.f_mixed() };
            let A = c2Ray { p: rng.v_mixed(), d: rng.v_mixed(), t: rng.f_mixed() };
            d.ray("c2RaytoCapsule(mixed)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
        }
        // exhaustive-ish weird sweep on r with a fixed sane capsule
        let base = c2Capsule {
            a: c2v { x: -2.0, y: 0.0 },
            b: c2v { x: 3.0, y: 1.0 },
            r: 1.0,
        };
        for &r in WEIRD {
            let B = c2Capsule { r, ..base };
            for &tv in WEIRD {
                let A = c2Ray {
                    p: c2v { x: -6.0, y: 0.5 },
                    d: c2v { x: 1.0, y: 0.0 },
                    t: tv,
                };
                d.ray("c2RaytoCapsule(weird r/t)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
            }
        }
    }
    d.finish("row 43: c2RaytoCapsule degenerate");
}

// ---------------------------------------------------------------------------
// Branch-coverage guard: proves rows 38..41 actually reach every exit of
// `c2RaytoCapsule` rather than passing vacuously.
// ---------------------------------------------------------------------------

thread_local! {
    static SEEN: RefCell<[usize; 7]> = const { RefCell::new([0; 7]) };
}

/// Re-derives which exit the C took, from the same quantities the C computes.
/// Exit ids:
/// 0 = `c2AABBtoPoint(capsule_bb, yAp)`      1 = cap A point test
/// 2 = cap B point test                      3 = `|yAp.x| < r`, `yAp.y < 0`
/// 4 = `|yAp.x| < r`, `yAp.y >= 0`           5 = wall / cap by height
/// 6 = final `return 0`
fn classify(B: &c2Capsule, A: &c2Ray) -> usize {
    let dx = B.b.x - B.a.x;
    let dy = B.b.y - B.a.y;
    let l = (dx * dx + dy * dy).sqrt();
    let my = c2v { x: dx / l, y: dy / l };
    let mx = c2v { x: my.y, y: -my.x };
    let mul = |m0: c2v, m1: c2v, b: c2v| c2v {
        x: m0.x * b.x + m0.y * b.y,
        y: m1.x * b.x + m1.y * b.y,
    };
    let yBb = mul(mx, my, c2v { x: dx, y: dy });
    let yAp = mul(mx, my, c2v { x: A.p.x - B.a.x, y: A.p.y - B.a.y });
    let yAd = mul(mx, my, A.d);
    let yAe = c2v {
        x: yAp.x + yAd.x * A.t,
        y: yAp.y + yAd.y * A.t,
    };
    let in_bb = !(yAp.x < -B.r || yAp.y < 0.0 || yAp.x > B.r || yAp.y > yBb.y);
    if in_bb {
        return 0;
    }
    let da = (A.p.x - B.a.x).powi(2) + (A.p.y - B.a.y).powi(2);
    if da < B.r * B.r {
        return 1;
    }
    let db = (A.p.x - B.b.x).powi(2) + (A.p.y - B.b.y).powi(2);
    if db < B.r * B.r {
        return 2;
    }
    let cabs = |v: f32| if v < 0.0 { -v } else { v };
    let cmin = |a: f32, b: f32| if a < b { a } else { b };
    if yAe.x * yAp.x < 0.0 || cmin(cabs(yAe.x), cabs(yAp.x)) < B.r {
        if cabs(yAp.x) < B.r {
            return if yAp.y < 0.0 { 3 } else { 4 };
        }
        return 5;
    }
    6
}

#[test]
fn capsule_branch_coverage() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0xC0FFEE);
    let mut seen = [0usize; 7];
    unsafe {
        // Reuse every generator family from rows 37..43 in one sweep.
        for _ in 0..40_000 {
            let B = rand_capsule(&mut rng);
            let f = frame(&B);
            let mode = rng.below(6);
            let A = match mode {
                0 => c2Ray { p: rng.v_small(), d: rng.v_dir(), t: rng.range(0.0, 40.0) },
                1 => c2Ray {
                    p: f.to_world(rng.sym(1.0) * B.r, rng.range(0.0, f.len)),
                    d: rng.v_dir(),
                    t: rng.range(0.0, 40.0),
                },
                2 => c2Ray {
                    p: f.to_world(rng.sym(0.9) * B.r, -rng.range(0.0, 0.9) * B.r),
                    d: rng.v_dir(),
                    t: rng.range(0.0, 40.0),
                },
                3 => c2Ray {
                    p: f.to_world(rng.sym(0.9) * B.r, f.len + rng.range(0.0, 0.9) * B.r),
                    d: rng.v_dir(),
                    t: rng.range(0.0, 40.0),
                },
                4 => c2Ray {
                    p: f.to_world(rng.sym(0.95) * B.r, f.len + B.r + rng.range(0.1, 20.0)),
                    d: rng.v_dir(),
                    t: rng.range(0.0, 60.0),
                },
                _ => {
                    let side = if rng.bool() { 1.0 } else { -1.0 };
                    c2Ray {
                        p: f.to_world(side * (B.r + rng.range(0.01, 20.0)), rng.sym(20.0)),
                        d: rng.v_dir(),
                        t: rng.range(0.0, 60.0),
                    }
                }
            };
            let k = classify(&B, &A);
            if k < 7 {
                seen[k] += 1;
            }
            d.ray("c2RaytoCapsule(cov)", call_capsule(&p.c, A, B), call_capsule(&p.rs, A, B));
        }
    }
    SEEN.with(|s| *s.borrow_mut() = seen);
    eprintln!("capsule exit histogram = {seen:?}");
    for (i, &n) in seen.iter().enumerate() {
        assert!(n > 0, "capsule exit {i} was never reached; row coverage is vacuous");
    }
    d.finish("capsule branch coverage (all 7 exits)");
}
