//! Phase C -- error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`.  Each test constructs the *exact* invalid
//! input or degenerate condition described by the row, calls BOTH `.so`s, and
//! asserts that they return the same sentinel/error code **and** leave `*out`
//! in the same state (bit-for-bit).

#![allow(non_snake_case)]

mod common;
use common::*;

use std::os::raw::c_void;

const SEED: u64 = 0x0BAD_F00D_DEAD_BEEF;
/// Base iteration count for the randomized sweeps; scaled by `$DIFF_SCALE`.
fn n() -> usize { iters(4_000) }

/// Assert both libraries agree AND that the C library really produced the
/// expected rejection code (so the row tests the branch it claims to test).
#[track_caller]
fn diff_and_expect_circle(ctx: &str, ray: c2Ray, c: c2Circle, expect: i32) {
    let l = libs();
    let mut co = SENTINEL;
    let mut ro = SENTINEL;
    let cr = unsafe { (l.c.c2RaytoCircle)(ray, c, &mut co) };
    let rr = unsafe { (l.r.c2RaytoCircle)(ray, c, &mut ro) };
    same(&format!("{ctx} ray={ray:?} c={c:?}"), (cr, co), (rr, ro));
    assert_eq!(cr, expect, "[{ctx}] C returned {cr}, row expects {expect}");
}

/* =============== rows 1..6 : c2RaytoCircle rejections ==================== */

#[test]
fn err_raytocircle_disc_negative() {
    let mut g = Rng::new(SEED ^ 1);
    for i in 0..n() {
        let ctr = c2v {
            x: g.coord(),
            y: g.coord(),
        };
        let r = 0.5 + g.uniform(4.0).abs();
        let c = c2Circle { p: ctr, r };
        // Ray parallel to +x, offset in y by more than r  => disc < 0.
        let off = r + 1.0 + g.uniform(50.0).abs();
        let ray = c2Ray {
            p: v(ctr.x - 1000.0, ctr.y + off),
            d: v(1.0, 0.0),
            t: 1.0e6,
        };
        diff_and_expect_circle(&format!("row1 i={i}"), ray, c, 0);
        // and the mirrored offset
        let ray = c2Ray {
            p: v(ctr.x - 1000.0, ctr.y - off),
            d: v(1.0, 0.0),
            t: 1.0e6,
        };
        diff_and_expect_circle(&format!("row1 i={i} mirror"), ray, c, 0);
        // sentinel must be untouched
        let l = libs();
        let mut co = SENTINEL;
        unsafe { (l.c.c2RaytoCircle)(ray, c, &mut co) };
        assert_eq!(co.bits(), SENTINEL.bits(), "row1: *out must be untouched");
    }
}

#[test]
fn err_raytocircle_t_negative() {
    let mut g = Rng::new(SEED ^ 2);
    for i in 0..n() {
        let ctr = c2v {
            x: g.coord(),
            y: g.coord(),
        };
        let r = 0.5 + g.uniform(4.0).abs();
        let c = c2Circle { p: ctr, r };
        // Origin past the circle, travelling further away => t = -b - sqrt < 0.
        let dist = r + 1.0 + g.uniform(50.0).abs();
        let ray = c2Ray {
            p: v(ctr.x + dist, ctr.y),
            d: v(1.0, 0.0),
            t: 1.0e6,
        };
        diff_and_expect_circle(&format!("row2 i={i}"), ray, c, 0);
        let l = libs();
        let mut co = SENTINEL;
        unsafe { (l.c.c2RaytoCircle)(ray, c, &mut co) };
        assert_eq!(co.bits(), SENTINEL.bits(), "row2: *out must be untouched");
    }
}

#[test]
fn err_raytocircle_t_beyond_len() {
    let mut g = Rng::new(SEED ^ 3);
    for i in 0..n() {
        let ctr = c2v {
            x: g.coord(),
            y: g.coord(),
        };
        let r = 0.5 + g.uniform(4.0).abs();
        let c = c2Circle { p: ctr, r };
        let dist = r + 5.0 + g.uniform(50.0).abs();
        // Ask the C library itself what the exact impact `t` is, then set A.t
        // to one ULP *below* it: `t <= A.t` must then be false.
        let probe_ray = c2Ray {
            p: v(ctr.x - dist, ctr.y),
            d: v(1.0, 0.0),
            t: 1.0e9,
        };
        let l = libs();
        let mut probe = SENTINEL;
        let hit = unsafe { (l.c.c2RaytoCircle)(probe_ray, c, &mut probe) };
        assert_eq!(hit, 1, "row3 precondition: the long ray must hit");
        let ray = c2Ray {
            t: next_down(probe.t),
            ..probe_ray
        };
        diff_and_expect_circle(&format!("row3 i={i} ulp"), ray, c, 0);
        // exactly at the boundary it must be accepted (proves the ULP is tight)
        diff_and_expect_circle(
            &format!("row3 i={i} boundary"),
            c2Ray { t: probe.t, ..probe_ray },
            c,
            1,
        );
        // and grossly too short
        let ray = c2Ray {
            t: probe.t * 0.5,
            ..probe_ray
        };
        diff_and_expect_circle(&format!("row3 i={i} half"), ray, c, 0);
        let l = libs();
        let mut co = SENTINEL;
        unsafe { (l.c.c2RaytoCircle)(ray, c, &mut co) };
        assert_eq!(co.bits(), SENTINEL.bits(), "row3: *out must be untouched");
    }
}

#[test]
fn err_raytocircle_nan_t() {
    let mut g = Rng::new(SEED ^ 4);
    for i in 0..n() {
        let ctr = c2v {
            x: g.coord(),
            y: g.coord(),
        };
        let r = 0.5 + g.uniform(4.0).abs();
        let c = c2Circle { p: ctr, r };
        let dist = r + 5.0;
        for t in [f32::NAN, -f32::NAN, f32::from_bits(0x7f80_0001)] {
            let ray = c2Ray {
                p: v(ctr.x - dist, ctr.y),
                d: v(1.0, 0.0),
                t,
            };
            // disc >= 0 (a real geometric hit) but `t <= A.t` is false.
            diff_and_expect_circle(&format!("row4 i={i} t={t}"), ray, c, 0);
            let l = libs();
            let mut co = SENTINEL;
            unsafe { (l.c.c2RaytoCircle)(ray, c, &mut co) };
            assert_eq!(co.bits(), SENTINEL.bits(), "row4: *out must be untouched");
        }
    }
}

#[test]
fn err_raytocircle_nan_inputs() {
    let mut g = Rng::new(SEED ^ 5);
    let l = libs();
    for i in 0..n() {
        // Every combination of "one field is NaN".
        let base_ray = c2Ray {
            p: v(-10.0, 0.0),
            d: v(1.0, 0.0),
            t: 100.0,
        };
        let base_c = c2Circle {
            p: v(0.0, 0.0),
            r: 2.0,
        };
        let nan = if i % 2 == 0 { f32::NAN } else { -f32::NAN };
        let variants: [(&str, c2Ray, c2Circle); 6] = [
            ("ray.p.x", c2Ray { p: v(nan, 0.0), ..base_ray }, base_c),
            ("ray.p.y", c2Ray { p: v(-10.0, nan), ..base_ray }, base_c),
            ("ray.d.x", c2Ray { d: v(nan, 0.0), ..base_ray }, base_c),
            ("ray.d.y", c2Ray { d: v(1.0, nan), ..base_ray }, base_c),
            ("c.p", base_ray, c2Circle { p: v(nan, nan), ..base_c }),
            ("c.r", base_ray, c2Circle { r: nan, ..base_c }),
        ];
        for (name, ray, c) in variants {
            let mut co = SENTINEL;
            let mut ro = SENTINEL;
            let cr = unsafe { (l.c.c2RaytoCircle)(ray, c, &mut co) };
            let rr = unsafe { (l.r.c2RaytoCircle)(ray, c, &mut ro) };
            same(&format!("row5 {name} i={i}"), (cr, co), (rr, ro));
            assert_eq!(cr, 0, "row5 {name}: NaN inputs must be rejected");
        }
        // fully wild
        let ray = c2Ray {
            p: g.wild_v(),
            d: g.wild_v(),
            t: g.wild(),
        };
        let c = c2Circle {
            p: g.wild_v(),
            r: g.wild(),
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.c2RaytoCircle)(ray, c, &mut co) };
        let rr = unsafe { (l.r.c2RaytoCircle)(ray, c, &mut ro) };
        same(&format!("row5 wild i={i}"), (cr, co), (rr, ro));
    }
}

#[test]
fn err_raytocircle_zero_radius() {
    let mut g = Rng::new(SEED ^ 6);
    let l = libs();
    for i in 0..n() {
        let ctr = c2v {
            x: (g.below(41) as f32) - 20.0,
            y: (g.below(41) as f32) - 20.0,
        };
        for r in [0.0f32, -0.0] {
            let c = c2Circle { p: ctr, r };
            let dist = (g.below(20) + 1) as f32;
            // ray passing exactly through the centre => disc == 0, t == dist
            for t in [dist, dist - 1.0, dist + 1.0, 0.0] {
                let ray = c2Ray {
                    p: v(ctr.x - dist, ctr.y),
                    d: v(1.0, 0.0),
                    t,
                };
                let mut co = SENTINEL;
                let mut ro = SENTINEL;
                let cr = unsafe { (l.c.c2RaytoCircle)(ray, c, &mut co) };
                let rr = unsafe { (l.r.c2RaytoCircle)(ray, c, &mut ro) };
                same(&format!("row6 i={i} r={r} t={t}"), (cr, co), (rr, ro));
            }
        }
    }
}

/* ============ rows 7..12 : c2AABBtoAABB separation tests ================= */

#[track_caller]
fn diff_aabb2(ctx: &str, a: c2AABB, b: c2AABB, expect: i32) {
    let l = libs();
    let cr = unsafe { (l.c.c2AABBtoAABB)(a, b) };
    let rr = unsafe { (l.r.c2AABBtoAABB)(a, b) };
    same(&format!("{ctx} a={a:?} b={b:?}"), cr, rr);
    assert_eq!(cr, expect, "[{ctx}] C returned {cr}, row expects {expect}");
}

fn unit_box_at(x: f32, y: f32) -> c2AABB {
    c2AABB {
        min: v(x, y),
        max: v(x + 1.0, y + 1.0),
    }
}

#[test]
fn err_aabbtoaabb_d0() {
    let mut g = Rng::new(SEED ^ 7);
    for i in 0..n() {
        let a = unit_box_at(g.coord(), g.coord());
        // B entirely left of A: B.max.x < A.min.x
        let gap = 1.0 + g.uniform(20.0).abs();
        let b = c2AABB {
            min: v(a.min.x - gap - 1.0, a.min.y),
            max: v(a.min.x - gap, a.max.y),
        };
        diff_aabb2(&format!("row7 i={i}"), a, b, 0);
    }
}

#[test]
fn err_aabbtoaabb_d1() {
    let mut g = Rng::new(SEED ^ 8);
    for i in 0..n() {
        let a = unit_box_at(g.coord(), g.coord());
        let gap = 1.0 + g.uniform(20.0).abs();
        let b = c2AABB {
            min: v(a.max.x + gap, a.min.y),
            max: v(a.max.x + gap + 1.0, a.max.y),
        };
        diff_aabb2(&format!("row8 i={i}"), a, b, 0);
    }
}

#[test]
fn err_aabbtoaabb_d2() {
    let mut g = Rng::new(SEED ^ 9);
    for i in 0..n() {
        let a = unit_box_at(g.coord(), g.coord());
        let gap = 1.0 + g.uniform(20.0).abs();
        let b = c2AABB {
            min: v(a.min.x, a.min.y - gap - 1.0),
            max: v(a.max.x, a.min.y - gap),
        };
        diff_aabb2(&format!("row9 i={i}"), a, b, 0);
    }
}

#[test]
fn err_aabbtoaabb_d3() {
    let mut g = Rng::new(SEED ^ 10);
    for i in 0..n() {
        let a = unit_box_at(g.coord(), g.coord());
        let gap = 1.0 + g.uniform(20.0).abs();
        let b = c2AABB {
            min: v(a.min.x, a.max.y + gap),
            max: v(a.max.x, a.max.y + gap + 1.0),
        };
        diff_aabb2(&format!("row10 i={i}"), a, b, 0);
    }
}

#[test]
fn err_aabbtoaabb_nan() {
    let mut g = Rng::new(SEED ^ 11);
    let unit = unit_box_at(0.0, 0.0);
    let all_nan = c2AABB {
        min: v(f32::NAN, f32::NAN),
        max: v(f32::NAN, f32::NAN),
    };
    // Every `<` is false when a NaN is involved => !(0|0|0|0) == 1.
    diff_aabb2("row11 nan-vs-unit", all_nan, unit, 1);
    diff_aabb2("row11 unit-vs-nan", unit, all_nan, 1);
    diff_aabb2("row11 nan-vs-nan", all_nan, all_nan, 1);
    let l = libs();
    for i in 0..n() {
        let a = c2AABB {
            min: g.wild_v(),
            max: g.wild_v(),
        };
        let b = c2AABB {
            min: g.wild_v(),
            max: g.wild_v(),
        };
        same(
            &format!("row11 wild i={i} a={a:?} b={b:?}"),
            unsafe { (l.c.c2AABBtoAABB)(a, b) },
            unsafe { (l.r.c2AABBtoAABB)(a, b) },
        );
    }
}

#[test]
fn err_aabbtoaabb_inverted() {
    let mut g = Rng::new(SEED ^ 12);
    let l = libs();
    for i in 0..n() {
        let a = unit_box_at(g.coord(), g.coord());
        let inv = c2AABB {
            min: a.max,
            max: a.min,
        };
        // No validation whatsoever: the four compares decide.
        for (k, (x, y)) in [(inv, a), (a, inv), (inv, inv)].iter().enumerate() {
            same(
                &format!("row12 i={i} k={k} x={x:?} y={y:?}"),
                unsafe { (l.c.c2AABBtoAABB)(*x, *y) },
                unsafe { (l.r.c2AABBtoAABB)(*x, *y) },
            );
        }
        // an inverted box vs a far-away box: still 0
        let far = unit_box_at(a.min.x + 1000.0, a.min.y);
        diff_aabb2(&format!("row12 i={i} inv-vs-far"), inv, far, 0);
    }
}

/* ============ rows 13..17 : c2AABBtoPoint rejections ===================== */

#[track_caller]
fn diff_abp(ctx: &str, a: c2AABB, p: c2v, expect: i32) {
    let l = libs();
    let cr = unsafe { (l.c.c2AABBtoPoint)(a, p) };
    let rr = unsafe { (l.r.c2AABBtoPoint)(a, p) };
    same(&format!("{ctx} a={a:?} p={p:?}"), cr, rr);
    assert_eq!(cr, expect, "[{ctx}] C returned {cr}, row expects {expect}");
}

#[test]
fn err_aabbtopoint_d0() {
    let mut g = Rng::new(SEED ^ 13);
    for i in 0..n() {
        let a = unit_box_at(g.coord(), g.coord());
        diff_abp(
            &format!("row13 i={i}"),
            a,
            v(a.min.x - 1.0 - g.uniform(20.0).abs(), a.min.y + 0.5),
            0,
        );
        // one ULP below min.x  =>  d0 fires
        diff_abp(
            &format!("row13 i={i} ulp"),
            a,
            v(next_down(a.min.x), a.min.y + 0.5),
            0,
        );
        // exactly at min.x  =>  `<` is strict, so this is still inside
        diff_abp(
            &format!("row13 i={i} boundary"),
            a,
            v(a.min.x, a.min.y + 0.5),
            1,
        );
    }
}

#[test]
fn err_aabbtopoint_d1() {
    let mut g = Rng::new(SEED ^ 14);
    for i in 0..n() {
        let a = unit_box_at(g.coord(), g.coord());
        diff_abp(
            &format!("row14 i={i}"),
            a,
            v(a.min.x + 0.5, a.min.y - 1.0 - g.uniform(20.0).abs()),
            0,
        );
    }
}

#[test]
fn err_aabbtopoint_d2() {
    let mut g = Rng::new(SEED ^ 15);
    for i in 0..n() {
        let a = unit_box_at(g.coord(), g.coord());
        diff_abp(
            &format!("row15 i={i}"),
            a,
            v(a.max.x + 1.0 + g.uniform(20.0).abs(), a.min.y + 0.5),
            0,
        );
    }
}

#[test]
fn err_aabbtopoint_d3() {
    let mut g = Rng::new(SEED ^ 16);
    for i in 0..n() {
        let a = unit_box_at(g.coord(), g.coord());
        diff_abp(
            &format!("row16 i={i}"),
            a,
            v(a.min.x + 0.5, a.max.y + 1.0 + g.uniform(20.0).abs()),
            0,
        );
    }
}

#[test]
fn err_aabbtopoint_nan() {
    let mut g = Rng::new(SEED ^ 17);
    let a = unit_box_at(0.0, 0.0);
    // no `<`/`>` is true for NaN => !(0) == 1
    diff_abp("row17 nan-x", a, v(f32::NAN, 0.5), 1);
    diff_abp("row17 nan-y", a, v(0.5, f32::NAN), 1);
    diff_abp("row17 nan-both", a, v(f32::NAN, -f32::NAN), 1);
    let l = libs();
    for i in 0..n() {
        let bb = c2AABB {
            min: g.wild_v(),
            max: g.wild_v(),
        };
        let p = g.wild_v();
        same(
            &format!("row17 wild i={i} bb={bb:?} p={p:?}"),
            unsafe { (l.c.c2AABBtoPoint)(bb, p) },
            unsafe { (l.r.c2AABBtoPoint)(bb, p) },
        );
    }
}

/* ============ rows 18..20 : c2CircleToPoint rejections =================== */

#[track_caller]
fn diff_ctp(ctx: &str, c: c2Circle, p: c2v, expect: i32) {
    let l = libs();
    let cr = unsafe { (l.c.c2CircleToPoint)(c, p) };
    let rr = unsafe { (l.r.c2CircleToPoint)(c, p) };
    same(&format!("{ctx} c={c:?} p={p:?}"), cr, rr);
    assert_eq!(cr, expect, "[{ctx}] C returned {cr}, row expects {expect}");
}

#[test]
fn err_circletopoint_outside_and_on_rim() {
    let mut g = Rng::new(SEED ^ 18);
    for i in 0..n() {
        let ctr = v((g.below(41) as f32) - 20.0, (g.below(41) as f32) - 20.0);
        // r == 5, point (3,4) away => d2 == 25 == r*r exactly => strict < fails
        let c = c2Circle { p: ctr, r: 5.0 };
        diff_ctp(&format!("row18 i={i} exact-rim-345"), c, v(ctr.x + 3.0, ctr.y + 4.0), 0);
        diff_ctp(&format!("row18 i={i} exact-rim-axis"), c, v(ctr.x + 5.0, ctr.y), 0);
        diff_ctp(&format!("row18 i={i} exact-rim-neg"), c, v(ctr.x, ctr.y - 5.0), 0);
        // strictly outside
        diff_ctp(
            &format!("row18 i={i} outside"),
            c,
            v(ctr.x + 5.0 + g.uniform(20.0).abs() + 0.001, ctr.y),
            0,
        );
        // strictly inside for contrast
        diff_ctp(&format!("row18 i={i} inside"), c, v(ctr.x + 3.0, ctr.y + 3.0), 1);
    }
}

#[test]
fn err_circletopoint_negative_radius() {
    let mut g = Rng::new(SEED ^ 19);
    for i in 0..n() {
        let ctr = v((g.below(41) as f32) - 20.0, (g.below(41) as f32) - 20.0);
        // r = -5 => r*r = 25 => behaves exactly like r = +5 (no validation)
        let neg = c2Circle { p: ctr, r: -5.0 };
        let pos = c2Circle { p: ctr, r: 5.0 };
        let l = libs();
        for p in [ctr, v(ctr.x + 3.0, ctr.y + 3.0), v(ctr.x + 3.0, ctr.y + 4.0), v(ctr.x + 9.0, ctr.y)] {
            let cn = unsafe { (l.c.c2CircleToPoint)(neg, p) };
            let rn = unsafe { (l.r.c2CircleToPoint)(neg, p) };
            same(&format!("row19 i={i} neg p={p:?}"), cn, rn);
            let cp = unsafe { (l.c.c2CircleToPoint)(pos, p) };
            assert_eq!(cn, cp, "row19: C must treat r=-5 exactly like r=+5");
        }
        // r == 0 / -0: nothing is inside (d2 < 0 is impossible)
        for r in [0.0f32, -0.0] {
            diff_ctp(&format!("row19 i={i} r={r}"), c2Circle { p: ctr, r }, ctr, 0);
        }
        // denormal radius: r*r underflows to 0 => nothing inside
        diff_ctp(
            &format!("row19 i={i} denorm"),
            c2Circle {
                p: ctr,
                r: f32::from_bits(1),
            },
            ctr,
            0,
        );
    }
}

#[test]
fn err_circletopoint_nan() {
    let mut g = Rng::new(SEED ^ 20);
    let ctr = v(0.0, 0.0);
    diff_ctp("row20 nan-r", c2Circle { p: ctr, r: f32::NAN }, ctr, 0);
    diff_ctp("row20 nan-p", c2Circle { p: v(f32::NAN, 0.0), r: 5.0 }, ctr, 0);
    diff_ctp("row20 nan-point", c2Circle { p: ctr, r: 5.0 }, v(f32::NAN, 0.0), 0);
    let l = libs();
    for i in 0..n() {
        let c = c2Circle {
            p: g.wild_v(),
            r: g.wild(),
        };
        let p = g.wild_v();
        same(
            &format!("row20 wild i={i} c={c:?} p={p:?}"),
            unsafe { (l.c.c2CircleToPoint)(c, p) },
            unsafe { (l.r.c2CircleToPoint)(c, p) },
        );
    }
}

/* ============ rows 21..28 : c2RaytoAABB rejections ======================= */

#[track_caller]
fn diff_and_expect_aabb(ctx: &str, ray: c2Ray, bb: c2AABB, expect: i32) {
    let l = libs();
    let mut co = SENTINEL;
    let mut ro = SENTINEL;
    let cr = unsafe { (l.c.c2RaytoAABB)(ray, bb, &mut co) };
    let rr = unsafe { (l.r.c2RaytoAABB)(ray, bb, &mut ro) };
    same(&format!("{ctx} ray={ray:?} bb={bb:?}"), (cr, co), (rr, ro));
    assert_eq!(cr, expect, "[{ctx}] C returned {cr}, row expects {expect}");
    if expect == 0 {
        assert_eq!(co.bits(), SENTINEL.bits(), "[{ctx}] *out must be untouched");
    }
}

#[test]
fn err_raytoaabb_sweep_box_disjoint() {
    let mut g = Rng::new(SEED ^ 21);
    for i in 0..n() {
        let bb = unit_box_at(g.coord(), g.coord());
        // A short ray far to the left: the swept AABB cannot touch bb.
        let gap = 10.0 + g.uniform(100.0).abs();
        let ray = c2Ray {
            p: v(bb.min.x - gap - 5.0, bb.min.y + 0.5),
            d: v(-1.0, 0.0),
            t: 1.0,
        };
        diff_and_expect_aabb(&format!("row21 i={i} left"), ray, bb, 0);
        // above
        let ray = c2Ray {
            p: v(bb.min.x + 0.5, bb.max.y + gap),
            d: v(0.0, 1.0),
            t: 1.0,
        };
        diff_and_expect_aabb(&format!("row21 i={i} above"), ray, bb, 0);
    }
}

#[test]
fn err_raytoaabb_sat_reject() {
    let mut g = Rng::new(SEED ^ 22);
    let l = libs();
    let mut hits = 0usize;
    for i in 0..n() {
        let bb = unit_box_at(g.coord(), g.coord());
        // A long diagonal ray whose swept AABB overlaps bb but which passes
        // outside the box: d > 0 => reject.
        let ray = c2Ray {
            p: v(bb.min.x - 5.0, bb.max.y + 0.25),
            d: v(0.70710678, 0.70710678),
            t: 20.0,
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.c2RaytoAABB)(ray, bb, &mut co) };
        let rr = unsafe { (l.r.c2RaytoAABB)(ray, bb, &mut ro) };
        same(&format!("row22 i={i} ray={ray:?} bb={bb:?}"), (cr, co), (rr, ro));
        if cr == 0 {
            hits += 1;
            assert_eq!(co.bits(), SENTINEL.bits(), "row22: *out untouched");
        }
    }
    assert!(hits > 0, "row22 never actually exercised the SAT reject");
}

#[test]
fn err_raytoaabb_no_plane_hit() {
    let mut g = Rng::new(SEED ^ 23);
    let l = libs();
    // Sweep a large randomized space and require that the `hit == 0` exit is
    // actually taken at least once, with C and Rust agreeing every time.
    let mut zero_out_rejects = 0usize;
    for i in 0..n() * 4 {
        let bb = c2AABB {
            min: g.coord_v(),
            max: g.coord_v(),
        };
        let ray = c2Ray {
            p: g.coord_v(),
            d: g.coord_v(),
            t: g.coord(),
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.c2RaytoAABB)(ray, bb, &mut co) };
        let rr = unsafe { (l.r.c2RaytoAABB)(ray, bb, &mut ro) };
        same(&format!("row23 i={i} ray={ray:?} bb={bb:?}"), (cr, co), (rr, ro));
        if cr == 0 && co.bits() == SENTINEL.bits() {
            zero_out_rejects += 1;
        }
    }
    assert!(zero_out_rejects > 0, "row23 never rejected");
}

#[test]
fn err_raytoaabb_zero_length_ray() {
    let mut g = Rng::new(SEED ^ 24);
    let l = libs();
    for i in 0..n() {
        let bb = unit_box_at(g.coord(), g.coord());
        for t in [0.0f32, -0.0] {
            for p in [
                v(bb.min.x + 0.5, bb.min.y + 0.5), // inside
                bb.min,                            // corner
                v(bb.min.x - 5.0, bb.min.y),       // outside
            ] {
                for d in [v(1.0, 0.0), v(0.0, 0.0), g.coord_v()] {
                    let ray = c2Ray { p, d, t };
                    let mut co = SENTINEL;
                    let mut ro = SENTINEL;
                    let cr = unsafe { (l.c.c2RaytoAABB)(ray, bb, &mut co) };
                    let rr = unsafe { (l.r.c2RaytoAABB)(ray, bb, &mut ro) };
                    same(&format!("row24 i={i} ray={ray:?} bb={bb:?}"), (cr, co), (rr, ro));
                }
            }
        }
    }
}

#[test]
fn err_raytoaabb_nan_dir() {
    let mut g = Rng::new(SEED ^ 25);
    let l = libs();
    for i in 0..n() {
        let bb = unit_box_at(g.coord(), g.coord());
        let nans = [f32::NAN, -f32::NAN, f32::from_bits(0x7fa0_0000)];
        let nan = nans[(i % 3) as usize];
        let rays = [
            c2Ray { p: v(bb.min.x - 5.0, bb.min.y + 0.5), d: v(nan, 0.0), t: 20.0 },
            c2Ray { p: v(bb.min.x - 5.0, bb.min.y + 0.5), d: v(1.0, nan), t: 20.0 },
            c2Ray { p: v(nan, bb.min.y + 0.5), d: v(1.0, 0.0), t: 20.0 },
            c2Ray { p: v(bb.min.x - 5.0, bb.min.y + 0.5), d: v(1.0, 0.0), t: nan },
            // a degenerate c2Norm result, exactly as spec_ray could produce
            c2Ray {
                p: v(bb.min.x - 5.0, bb.min.y + 0.5),
                d: unsafe { (l.c.c2Norm)(v(0.0, 0.0)) },
                t: 20.0,
            },
        ];
        for (k, ray) in rays.iter().enumerate() {
            let mut co = SENTINEL;
            let mut ro = SENTINEL;
            let cr = unsafe { (l.c.c2RaytoAABB)(*ray, bb, &mut co) };
            let rr = unsafe { (l.r.c2RaytoAABB)(*ray, bb, &mut ro) };
            same(&format!("row25 i={i} k={k} ray={ray:?} bb={bb:?}"), (cr, co), (rr, ro));
        }
        // NaN box bounds
        for (k, bb) in [
            c2AABB { min: v(nan, 0.0), max: v(1.0, 1.0) },
            c2AABB { min: v(0.0, 0.0), max: v(nan, nan) },
        ]
        .iter()
        .enumerate()
        {
            let ray = c2Ray { p: v(-5.0, 0.5), d: v(1.0, 0.0), t: 20.0 };
            let mut co = SENTINEL;
            let mut ro = SENTINEL;
            let cr = unsafe { (l.c.c2RaytoAABB)(ray, *bb, &mut co) };
            let rr = unsafe { (l.r.c2RaytoAABB)(ray, *bb, &mut ro) };
            same(&format!("row25 i={i} nanbox={k}"), (cr, co), (rr, ro));
        }
    }
}

#[test]
fn err_raytoplane_da_negative() {
    // da < 0 => that plane's t is 0 => hitN == 1, so the ray is reported as
    // hitting with t == 0.  Reached whenever the ray origin is inside the slab.
    let mut g = Rng::new(SEED ^ 26);
    let l = libs();
    for i in 0..n() {
        let bb = unit_box_at(g.coord(), g.coord());
        // origin inside the box => all four da are negative
        let ray = c2Ray {
            p: v(bb.min.x + 0.5, bb.min.y + 0.5),
            d: v(1.0, 0.0),
            t: 10.0,
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.c2RaytoAABB)(ray, bb, &mut co) };
        let rr = unsafe { (l.r.c2RaytoAABB)(ray, bb, &mut ro) };
        same(&format!("row26 i={i}"), (cr, co), (rr, ro));
        assert_eq!(cr, 1, "row26: origin inside the box must report a hit");
        assert_eq!(co.t, 0.0, "row26: t must be 0 when da < 0 on the winning axis");
    }
}

#[test]
fn err_raytoplane_same_side() {
    // da * db > 0 => return 1.0f => hitN is still 1 (1.0f <= 1.0f).
    let mut g = Rng::new(SEED ^ 27);
    let l = libs();
    for i in 0..n() {
        let bb = unit_box_at(g.coord(), g.coord());
        // A ray that stays entirely on the +x side of B.max.x while crossing in y
        let ray = c2Ray {
            p: v(bb.max.x + 0.25, bb.min.y - 3.0),
            d: v(0.0, 1.0),
            t: 6.0,
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.c2RaytoAABB)(ray, bb, &mut co) };
        let rr = unsafe { (l.r.c2RaytoAABB)(ray, bb, &mut ro) };
        same(&format!("row27 i={i}"), (cr, co), (rr, ro));
    }
}

#[test]
fn err_raytoplane_d_zero() {
    // da == db => d == 0 => the explicit `d != 0` guard returns 0 instead of
    // dividing by zero.  Any axis-aligned ray produces this on the other axis.
    let mut g = Rng::new(SEED ^ 28);
    let l = libs();
    for i in 0..n() {
        let bb = unit_box_at(g.coord(), g.coord());
        for d in [v(1.0, 0.0), v(-1.0, 0.0), v(0.0, 1.0), v(0.0, -1.0)] {
            for p in [
                v(bb.min.x - 3.0, bb.min.y + 0.5),
                v(bb.min.x + 0.5, bb.min.y - 3.0),
                v(bb.min.x + 0.5, bb.min.y + 0.5),
            ] {
                let ray = c2Ray { p, d, t: 6.0 };
                let mut co = SENTINEL;
                let mut ro = SENTINEL;
                let cr = unsafe { (l.c.c2RaytoAABB)(ray, bb, &mut co) };
                let rr = unsafe { (l.r.c2RaytoAABB)(ray, bb, &mut ro) };
                same(&format!("row28 i={i} d={d:?} p={p:?}"), (cr, co), (rr, ro));
                // no inf / NaN may leak out of the guarded division
                if cr == 1 {
                    assert!(
                        co.t.is_finite() || co.t.is_nan(),
                        "row28: unexpected out->t {}",
                        co.t
                    );
                }
            }
        }
    }
}

/* ============ rows 29..37 : c2RaytoCapsule rejections ==================== */

fn nice_capsule(g: &mut Rng) -> c2Capsule {
    let a = g.coord_v();
    let ang = g.uniform(3.14159265);
    let len = 1.0 + g.uniform(20.0).abs();
    c2Capsule {
        a,
        b: v(a.x + len * ang.cos(), a.y + len * ang.sin()),
        r: 0.25 + g.uniform(4.0).abs(),
    }
}

fn frame(cap: &c2Capsule) -> (f32, f32, f32, f32, f32) {
    let len = (cap.b.x - cap.a.x).hypot(cap.b.y - cap.a.y);
    let (ux, uy) = ((cap.b.x - cap.a.x) / len, (cap.b.y - cap.a.y) / len);
    (ux, uy, -uy, ux, len)
}

#[test]
fn err_raytocapsule_miss_still_writes_out() {
    let mut g = Rng::new(SEED ^ 29);
    let l = libs();
    let mut misses = 0usize;
    for i in 0..n() {
        let cap = nice_capsule(&mut g);
        let (ux, uy, px, py, len) = frame(&cap);
        // parallel ray far off to the side => neither trigger condition holds
        let off = cap.r * 50.0 + 100.0;
        let ray = c2Ray {
            p: v(cap.a.x + px * off, cap.a.y + py * off),
            d: v(ux, uy),
            t: len,
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut co) };
        let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut ro) };
        same(&format!("row29 i={i} ray={ray:?} cap={cap:?}"), (cr, co), (rr, ro));
        if cr == 0 {
            misses += 1;
            assert_ne!(
                co.bits(),
                SENTINEL.bits(),
                "row29: C writes *out even on the miss path"
            );
            let expect_n = unsafe { (l.c.c2Norm)((l.c.c2Sub)(cap.b, cap.a)) };
            assert_eq!(co.t.to_bits(), 0.0f32.to_bits(), "row29: out->t must be 0");
            assert_eq!(
                co.n.bits(),
                expect_n.bits(),
                "row29: out->n must be normalize(b - a)"
            );
        }
    }
    assert!(misses > 0, "row29 never took the final `return 0`");
}

#[test]
fn err_raytocapsule_origin_in_slab() {
    let mut g = Rng::new(SEED ^ 30);
    let l = libs();
    for i in 0..n() {
        let cap = nice_capsule(&mut g);
        let mid = v((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
        let ray = c2Ray {
            p: mid,
            d: v(1.0, 0.0),
            t: 10.0,
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut co) };
        let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut ro) };
        same(&format!("row30 i={i}"), (cr, co), (rr, ro));
        assert_eq!(cr, 1, "row30: origin in the slab must return 1");
        assert_eq!(co.t.to_bits(), 0.0f32.to_bits(), "row30: t must be 0");
    }
}

#[test]
fn err_raytocapsule_origin_in_cap_a() {
    let mut g = Rng::new(SEED ^ 31);
    let l = libs();
    let mut used = 0usize;
    for i in 0..n() {
        let cap = nice_capsule(&mut g);
        let (ux, uy, _, _, _) = frame(&cap);
        // strictly beyond a along -u but within r => outside the slab, inside Ca
        let f = cap.r * 0.5;
        let ray = c2Ray {
            p: v(cap.a.x - ux * f, cap.a.y - uy * f),
            d: v(1.0, 0.0),
            t: 10.0,
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut co) };
        let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut ro) };
        same(&format!("row31 i={i}"), (cr, co), (rr, ro));
        if cr == 1 && co.t.to_bits() == 0.0f32.to_bits() {
            used += 1;
        }
    }
    assert!(used > 0, "row31 never hit the cap-a early return");
}

#[test]
fn err_raytocapsule_origin_in_cap_b() {
    let mut g = Rng::new(SEED ^ 32);
    let l = libs();
    let mut used = 0usize;
    for i in 0..n() {
        let cap = nice_capsule(&mut g);
        let (ux, uy, _, _, _) = frame(&cap);
        let f = cap.r * 0.5;
        let ray = c2Ray {
            p: v(cap.b.x + ux * f, cap.b.y + uy * f),
            d: v(1.0, 0.0),
            t: 10.0,
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut co) };
        let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut ro) };
        same(&format!("row32 i={i}"), (cr, co), (rr, ro));
        if cr == 1 && co.t.to_bits() == 0.0f32.to_bits() {
            used += 1;
        }
    }
    assert!(used > 0, "row32 never hit the cap-b early return");
}

#[test]
fn err_raytocapsule_degenerate_ab() {
    let mut g = Rng::new(SEED ^ 33);
    let l = libs();
    for i in 0..n() {
        let p = g.coord_v();
        for r in [0.0f32, 1.0, g.radius()] {
            let cap = c2Capsule { a: p, b: p, r };
            for ray in [
                c2Ray { p: v(p.x - 5.0, p.y), d: v(1.0, 0.0), t: 10.0 },
                c2Ray { p, d: v(1.0, 0.0), t: 10.0 },
                c2Ray { p: g.coord_v(), d: g.coord_v(), t: g.coord() },
            ] {
                let mut co = SENTINEL;
                let mut ro = SENTINEL;
                let cr = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut co) };
                let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut ro) };
                same(&format!("row33 i={i} r={r} ray={ray:?}"), (cr, co), (rr, ro));
                // out->n is always written first, and here it is NaN
                assert!(
                    co.n.x.is_nan() && co.n.y.is_nan(),
                    "row33: c2Norm((0,0)) must give a NaN normal, got {:?}",
                    co.n
                );
            }
        }
    }
}

#[test]
fn err_raytocapsule_zero_radius() {
    let mut g = Rng::new(SEED ^ 34);
    let l = libs();
    for i in 0..n() {
        let base = nice_capsule(&mut g);
        for r in [0.0f32, -0.0] {
            let cap = c2Capsule { r, ..base };
            let (ux, uy, px, py, len) = frame(&cap);
            for ray in [
                // aimed at the middle from the side
                c2Ray {
                    p: v(
                        (cap.a.x + cap.b.x) * 0.5 + px * 10.0,
                        (cap.a.y + cap.b.y) * 0.5 + py * 10.0,
                    ),
                    d: v(-px, -py),
                    t: 20.0,
                },
                // along the axis
                c2Ray {
                    p: v(cap.a.x - ux * 5.0, cap.a.y - uy * 5.0),
                    d: v(ux, uy),
                    t: len + 10.0,
                },
                c2Ray { p: g.coord_v(), d: g.coord_v(), t: g.coord() },
            ] {
                let mut co = SENTINEL;
                let mut ro = SENTINEL;
                let cr = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut co) };
                let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut ro) };
                same(&format!("row34 i={i} r={r} ray={ray:?} cap={cap:?}"), (cr, co), (rr, ro));
            }
        }
    }
}

#[test]
fn err_raytocapsule_negative_radius() {
    let mut g = Rng::new(SEED ^ 35);
    let l = libs();
    for i in 0..n() {
        let base = nice_capsule(&mut g);
        for r in [-1.0f32, -5.0, -base.r] {
            let cap = c2Capsule { r, ..base };
            let mid = v((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
            for ray in [
                c2Ray { p: mid, d: v(1.0, 0.0), t: 10.0 },
                c2Ray { p: v(mid.x + 50.0, mid.y), d: v(-1.0, 0.0), t: 100.0 },
                c2Ray { p: g.coord_v(), d: g.coord_v(), t: g.coord() },
            ] {
                let mut co = SENTINEL;
                let mut ro = SENTINEL;
                let cr = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut co) };
                let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut ro) };
                same(&format!("row35 i={i} r={r} ray={ray:?} cap={cap:?}"), (cr, co), (rr, ro));
            }
        }
    }
}

#[test]
fn err_raytocapsule_div_by_zero() {
    // yAe.x == yAp.x  =>  d == 0  =>  (c - yAp.x)/0 => +-inf or NaN.
    let mut g = Rng::new(SEED ^ 36);
    let l = libs();
    let mut reached = 0usize;
    for i in 0..n() {
        let cap = nice_capsule(&mut g);
        let (ux, uy, px, py, len) = frame(&cap);
        // A ray exactly parallel to the axis has constant local x, so
        // yAe.x - yAp.x == 0.  Offsets >= r skip the early exits.
        for off in [cap.r, cap.r * 1.5, cap.r * 3.0, cap.r + 1.0] {
            for s in [1.0f32, -1.0] {
                let ray = c2Ray {
                    p: v(
                        cap.a.x + px * off * s - ux * 3.0,
                        cap.a.y + py * off * s - uy * 3.0,
                    ),
                    d: v(ux, uy),
                    t: len + 6.0,
                };
                let mut co = SENTINEL;
                let mut ro = SENTINEL;
                let cr = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut co) };
                let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut ro) };
                same(
                    &format!("row36 i={i} off={off} s={s} ray={ray:?} cap={cap:?}"),
                    (cr, co),
                    (rr, ro),
                );
                reached += 1;
            }
        }
        // t == 0 gives yAe == yAp exactly on both components
        let ray = c2Ray {
            p: v(cap.a.x + px * (cap.r + 1.0), cap.a.y + py * (cap.r + 1.0)),
            d: v(ux, uy),
            t: 0.0,
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut co) };
        let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut ro) };
        same(&format!("row36 i={i} t=0"), (cr, co), (rr, ro));
    }
    assert!(reached > 0);
}

#[test]
fn err_raytocapsule_delegate_returns_zero() {
    // y >= yBb.y (or y <= 0) => c2RaytoCircle on an end cap, which itself
    // returns 0 when the ray is too short.
    let mut g = Rng::new(SEED ^ 37);
    let l = libs();
    let mut zeros = 0usize;
    for i in 0..n() {
        let cap = nice_capsule(&mut g);
        let (ux, uy, px, py, _len) = frame(&cap);
        for (k, (ex, ey, s)) in [(cap.a.x, cap.a.y, -1.0f32), (cap.b.x, cap.b.y, 1.0f32)]
            .iter()
            .enumerate()
        {
            let tgt = v(ex + ux * s * cap.r * 0.5, ey + uy * s * cap.r * 0.5);
            let dist = cap.r + 5.0 + g.uniform(20.0).abs();
            let o = v(tgt.x + px * dist, tgt.y + py * dist);
            let delta = unsafe { (l.c.c2Sub)(tgt, o) };
            let d = unsafe { (l.c.c2Norm)(delta) };
            // ray far too short to reach the cap => the delegated call returns 0
            let ray = c2Ray {
                p: o,
                d,
                t: dist * 0.05,
            };
            let mut co = SENTINEL;
            let mut ro = SENTINEL;
            let cr = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut co) };
            let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut ro) };
            same(&format!("row37 i={i} end={k}"), (cr, co), (rr, ro));
            if cr == 0 {
                zeros += 1;
                // the pre-written values survive the failed delegation
                assert_eq!(co.t.to_bits(), 0.0f32.to_bits());
            }
        }
    }
    assert!(zeros > 0, "row37 never observed a delegated 0");
}

/* ============ rows 38..39 : out-of-range C2_TYPE enum ==================== */

#[test]
fn err_castray_out_of_range_enum() {
    let mut g = Rng::new(SEED ^ 38);
    let l = libs();
    let shape = c2Circle {
        p: v(0.0, 0.0),
        r: 2.0,
    };
    let sp = &shape as *const c2Circle as *const c_void;
    let bad_types: [i32; 12] = [
        3,
        4,
        -1,
        -2,
        100,
        1000,
        i32::MAX,
        i32::MIN,
        0x7fff_ffff,
        -0x8000_0000,
        255,
        0x0001_0000, // low 16 bits zero but not a valid enum
    ];
    for &ty in &bad_types {
        for i in 0..200 {
            let ray = c2Ray {
                p: g.coord_v(),
                d: g.coord_v(),
                t: g.coord(),
            };
            let mut co = SENTINEL;
            let mut ro = SENTINEL;
            // The C switch has no `default:` and no trailing return, so the
            // returned value is indeterminate (whatever is in %eax).  The one
            // well-defined, observable property is that `*out` is untouched --
            // that is what both libraries must agree on.
            let _cr = unsafe { (l.c.c2CastRay)(ray, sp, ty, &mut co) };
            let rr = unsafe { (l.r.c2CastRay)(ray, sp, ty, &mut ro) };
            assert_eq!(
                co.bits(),
                SENTINEL.bits(),
                "row38/39: C c2CastRay(type={ty}) must not write *out (i={i})"
            );
            assert_eq!(
                ro.bits(),
                SENTINEL.bits(),
                "row38/39: Rust c2CastRay(type={ty}) must not write *out (i={i})"
            );
            same(
                &format!("row38/39 out-state ty={ty} i={i}"),
                (0i32, co),
                (0i32, ro),
            );
            // Rust returns a defined 0 where C is UB.
            assert_eq!(rr, 0, "row38/39: Rust must return a defined 0");
        }
    }
    // A NULL shape pointer is also never dereferenced for a bad type.
    for &ty in &bad_types {
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let ray = c2Ray {
            p: v(0.0, 0.0),
            d: v(1.0, 0.0),
            t: 1.0,
        };
        let _ = unsafe { (l.c.c2CastRay)(ray, std::ptr::null(), ty, &mut co) };
        let rr = unsafe { (l.r.c2CastRay)(ray, std::ptr::null(), ty, &mut ro) };
        assert_eq!(co.bits(), SENTINEL.bits());
        assert_eq!(ro.bits(), SENTINEL.bits());
        assert_eq!(rr, 0);
    }
    // Sanity: the three *valid* enum values are still dispatched.
    for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
        let ray = c2Ray {
            p: v(-10.0, 0.0),
            d: v(1.0, 0.0),
            t: 100.0,
        };
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cap = c2Capsule {
            a: v(0.0, -1.0),
            b: v(0.0, 1.0),
            r: 1.0,
        };
        let bb = c2AABB {
            min: v(-1.0, -1.0),
            max: v(1.0, 1.0),
        };
        let p: *const c_void = match ty {
            C2_TYPE_CIRCLE => &shape as *const c2Circle as *const c_void,
            C2_TYPE_AABB => &bb as *const c2AABB as *const c_void,
            _ => &cap as *const c2Capsule as *const c_void,
        };
        let cr = unsafe { (l.c.c2CastRay)(ray, p, ty, &mut co) };
        let rr = unsafe { (l.r.c2CastRay)(ray, p, ty, &mut ro) };
        same(&format!("row38/39 valid ty={ty}"), (cr, co), (rr, ro));
        assert_eq!(cr, 1, "valid dispatch for ty={ty} should hit");
    }
}

/* ============ rows 40..41 : NULL pointers =============================== */

#[test]
fn err_null_out_miss_paths_are_safe() {
    // On the miss paths of c2RaytoCircle / c2RaytoAABB the C code never touches
    // `out`, so a NULL out-pointer is safe there -- and must be equally safe
    // (and return the same value) in Rust.
    let mut g = Rng::new(SEED ^ 41);
    let l = libs();
    let null: *mut c2Raycast = std::ptr::null_mut();
    let mut checked = 0usize;
    for i in 0..n() {
        let ctr = c2v {
            x: g.coord(),
            y: g.coord(),
        };
        let r = 0.5 + g.uniform(4.0).abs();
        let c = c2Circle { p: ctr, r };
        // guaranteed disc < 0
        let ray = c2Ray {
            p: v(ctr.x - 1000.0, ctr.y + r + 10.0),
            d: v(1.0, 0.0),
            t: 1.0e6,
        };
        let mut probe = SENTINEL;
        assert_eq!(
            unsafe { (l.c.c2RaytoCircle)(ray, c, &mut probe) },
            0,
            "precondition: this must be a miss"
        );
        let cr = unsafe { (l.c.c2RaytoCircle)(ray, c, null) };
        let rr = unsafe { (l.r.c2RaytoCircle)(ray, c, null) };
        same(&format!("row41 circle-miss i={i}"), cr, rr);
        assert_eq!(cr, 0);

        // guaranteed swept-box reject for c2RaytoAABB
        let bb = unit_box_at(g.coord(), g.coord());
        let ray = c2Ray {
            p: v(bb.min.x - 1000.0, bb.min.y),
            d: v(-1.0, 0.0),
            t: 1.0,
        };
        let mut probe = SENTINEL;
        assert_eq!(
            unsafe { (l.c.c2RaytoAABB)(ray, bb, &mut probe) },
            0,
            "precondition: this must be a miss"
        );
        let cr = unsafe { (l.c.c2RaytoAABB)(ray, bb, null) };
        let rr = unsafe { (l.r.c2RaytoAABB)(ray, bb, null) };
        same(&format!("row41 aabb-miss i={i}"), cr, rr);
        assert_eq!(cr, 0);

        // c2CastRay with an out-of-range type never touches out either
        let cr = unsafe { (l.c.c2CastRay)(ray, std::ptr::null(), 7, null) };
        let _ = cr;
        let rr = unsafe { (l.r.c2CastRay)(ray, std::ptr::null(), 7, null) };
        assert_eq!(rr, 0);
        checked += 1;
    }
    assert!(checked > 0);
}

/* -- the faulting NULL subset, run in a child process ---------------------- */

const NULL_CASES: [&str; 4] = [
    "raytocircle_hit",
    "raytoaabb_hit",
    "raytocapsule_any",
    "spec_ray_hit",
];

#[test]
#[ignore = "helper: crashes on purpose, driven by err_null_out_faulting_paths"]
fn null_deref_child() {
    let which = std::env::var("NULL_LIB").expect("NULL_LIB");
    let case = std::env::var("NULL_CASE").expect("NULL_CASE");
    let l = libs();
    let s = if which == "c" { &l.c } else { &l.r };
    let null: *mut c2Raycast = std::ptr::null_mut();
    let ray = c2Ray {
        p: v(-10.0, 0.0),
        d: v(1.0, 0.0),
        t: 100.0,
    };
    let code = match case.as_str() {
        "raytocircle_hit" => unsafe {
            (s.c2RaytoCircle)(
                ray,
                c2Circle {
                    p: v(0.0, 0.0),
                    r: 2.0,
                },
                null,
            )
        },
        "raytoaabb_hit" => unsafe {
            (s.c2RaytoAABB)(
                ray,
                c2AABB {
                    min: v(-1.0, -1.0),
                    max: v(1.0, 1.0),
                },
                null,
            )
        },
        "raytocapsule_any" => unsafe {
            (s.c2RaytoCapsule)(
                ray,
                c2Capsule {
                    a: v(0.0, -1.0),
                    b: v(0.0, 1.0),
                    r: 1.0,
                },
                null,
            )
        },
        "spec_ray_hit" => unsafe { (s.spec_ray)(null, 0.0, 0.0, 0.0, 0.0, 2.0, -10.0, 0.0) },
        other => panic!("unknown case {other}"),
    };
    println!("SURVIVED {which}/{case} -> {code}");
}

#[test]
fn err_null_out_faulting_paths() {
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    let exe = std::env::current_exe().expect("current_exe");
    for case in NULL_CASES {
        let mut results = Vec::new();
        for lib in ["c", "r"] {
            let out = Command::new(&exe)
                .args(["--exact", "--ignored", "--nocapture", "null_deref_child"])
                .env("NULL_LIB", lib)
                .env("NULL_CASE", case)
                .output()
                .expect("spawn child");
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            results.push((
                out.status.signal(),
                out.status.code(),
                stdout.contains("SURVIVED"),
                stderr.contains("null pointer dereference"),
            ));
        }
        let (c, r) = (results[0], results[1]);
        assert_eq!(
            c.2, r.2,
            "case {case}: C survived={} but Rust survived={}",
            c.2, r.2
        );
        if c.2 {
            // Both survived (the C code never touched `out` on this path)
            // => the return values must match too.
            assert_eq!(c.1, r.1, "case {case}: exit code mismatch");
            continue;
        }
        assert_eq!(
            c.0,
            Some(11),
            "case {case}: expected SIGSEGV from the C NULL write, got {c:?}"
        );
        if r.3 {
            // The Rust `.so` under test was built with `-C debug-assertions`
            // (i.e. `cargo build` without `--release`).  In that configuration
            // rustc emits an explicit "null pointer dereference occurred"
            // check that aborts *before* the store, so the process dies with
            // SIGABRT rather than SIGSEGV.  That is a property of the debug
            // build profile, not of the translation: the shipping cdylib
            // (`cargo build --release`) segfaults exactly like C.  Accept it,
            // but only when the message proves that is what happened.
            assert_eq!(
                r.0,
                Some(6),
                "case {case}: debug-assertions Rust build should SIGABRT, got {r:?}"
            );
            eprintln!(
                "note: case {case}: Rust .so has debug-assertions; \
                 SIGABRT(null pointer dereference) accepted as equivalent to C's SIGSEGV"
            );
        } else {
            assert_eq!(
                r.0, c.0,
                "case {case}: C died with signal {:?} but Rust with {:?}",
                c.0, r.0
            );
        }
    }
}

/* ============ rows 42..46 : unguarded divisions / c2Norm ================= */

#[test]
fn err_div_by_zero() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 42);
    for b in [0.0f32, -0.0] {
        for i in 0..n() {
            let a = if i % 4 == 0 { v(0.0, 0.0) } else { g.wild_v() };
            let cr = unsafe { (l.c.c2Div)(a, b) };
            let rr = unsafe { (l.r.c2Div)(a, b) };
            same(&format!("row42/43 a={a:?} b={b} i={i}"), cr, rr);
        }
        // the sign of the zero divisor must be honoured
        let cr = unsafe { (l.c.c2Div)(v(1.0, -1.0), b) };
        assert!(
            cr.x.is_infinite() && cr.y.is_infinite(),
            "row42/43: expected +-inf, got {cr:?}"
        );
        let expect_sign = if b.is_sign_negative() { -1.0 } else { 1.0 };
        assert_eq!(cr.x.signum(), expect_sign, "row43: sign of 1/{b}");
        // 0 / 0 => 0 * inf => NaN
        let cr = unsafe { (l.c.c2Div)(v(0.0, 0.0), b) };
        let rr = unsafe { (l.r.c2Div)(v(0.0, 0.0), b) };
        same(&format!("row42 zero-over-zero b={b}"), cr, rr);
        assert!(cr.x.is_nan(), "row42: 0/0 must be NaN, got {cr:?}");
    }
}

#[test]
fn err_norm_zero_vector() {
    let l = libs();
    for a in [
        v(0.0, 0.0),
        v(-0.0, -0.0),
        v(0.0, -0.0),
        v(-0.0, 0.0),
    ] {
        let cr = unsafe { (l.c.c2Norm)(a) };
        let rr = unsafe { (l.r.c2Norm)(a) };
        same(&format!("row44 a={a:?}"), cr, rr);
        assert!(
            cr.x.is_nan() && cr.y.is_nan(),
            "row44: c2Norm({a:?}) must be (NaN, NaN), got {cr:?}"
        );
    }
    // c2Len of the zero vector is exactly 0
    for a in [v(0.0, 0.0), v(-0.0, -0.0)] {
        same(
            &format!("row44 len a={a:?}"),
            unsafe { (l.c.c2Len)(a) },
            unsafe { (l.r.c2Len)(a) },
        );
    }
}

#[test]
fn err_norm_inf_vector() {
    let l = libs();
    let inf = f32::INFINITY;
    for a in [
        v(inf, 0.0),
        v(-inf, 0.0),
        v(0.0, inf),
        v(inf, inf),
        v(inf, -inf),
        v(f32::MAX, f32::MAX),
        v(1e30, 1e30),
    ] {
        let cr = unsafe { (l.c.c2Norm)(a) };
        let rr = unsafe { (l.r.c2Norm)(a) };
        same(&format!("row45 a={a:?}"), cr, rr);
    }
    // c2Norm(inf, 0) => len = inf => 1/inf = 0 => inf*0 = NaN
    let cr = unsafe { (l.c.c2Norm)(v(inf, 0.0)) };
    assert!(cr.x.is_nan(), "row45: inf*0 must be NaN, got {cr:?}");
}

#[test]
fn err_len_overflow() {
    let l = libs();
    for a in [
        v(1e30, 0.0),
        v(1e30, 1e30),
        v(f32::MAX, f32::MAX),
        v(f32::MAX, 0.0),
        v(-1e30, -1e30),
    ] {
        let cr = unsafe { (l.c.c2Len)(a) };
        let rr = unsafe { (l.r.c2Len)(a) };
        same(&format!("row46 a={a:?}"), cr, rr);
    }
    assert!(
        unsafe { (l.c.c2Len)(v(1e30, 1e30)) }.is_infinite(),
        "row46: the dot product must overflow to +inf"
    );
    // negative "squares" cannot happen, but sqrt of a NaN dot must agree
    for a in [v(f32::NAN, 0.0), v(f32::INFINITY, f32::NEG_INFINITY)] {
        same(
            &format!("row46 nan a={a:?}"),
            unsafe { (l.c.c2Len)(a) },
            unsafe { (l.r.c2Len)(a) },
        );
    }
}

/* ============ rows 47..51 : spec_ray rejections ========================== */

#[test]
fn err_spec_ray_mp_equals_ray_origin() {
    let mut g = Rng::new(SEED ^ 47);
    let l = libs();
    for i in 0..n() {
        let (px, py) = (g.coord(), g.coord());
        let (cx, cy) = (g.coord(), g.coord());
        let r = g.radius();
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.spec_ray)(&mut co, px, py, cx, cy, r, px, py) };
        let rr = unsafe { (l.r.spec_ray)(&mut ro, px, py, cx, cy, r, px, py) };
        same(&format!("row47 i={i} p=({px},{py})"), (cr, co), (rr, ro));
        assert_eq!(cr, 0, "row47: a NaN ray direction must be rejected");
        assert_eq!(
            co.bits(),
            SENTINEL.bits(),
            "row47: *cast must be untouched"
        );
    }
    // signed-zero flavours of "equal"
    for (a, b) in [(0.0f32, -0.0f32), (-0.0, 0.0), (0.0, 0.0), (-0.0, -0.0)] {
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.spec_ray)(&mut co, a, a, 5.0, 5.0, 2.0, b, b) };
        let rr = unsafe { (l.r.spec_ray)(&mut ro, a, a, 5.0, 5.0, 2.0, b, b) };
        same(&format!("row47 zeros a={a} b={b}"), (cr, co), (rr, ro));
    }
}

#[test]
fn err_spec_ray_negative_radius() {
    let mut g = Rng::new(SEED ^ 48);
    let l = libs();
    for i in 0..n() {
        let (cx, cy) = (g.coord(), g.coord());
        let r = 1.0 + g.uniform(10.0).abs();
        let (rx, ry) = (cx - (r + 10.0), cy);
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.spec_ray)(&mut co, cx, cy, cx, cy, -r, rx, ry) };
        let rr = unsafe { (l.r.spec_ray)(&mut ro, cx, cy, cx, cy, -r, rx, ry) };
        same(&format!("row48 i={i} r={r}"), (cr, co), (rr, ro));
        // -r behaves exactly like +r because only r*r is used
        let mut po = SENTINEL;
        let pr = unsafe { (l.c.spec_ray)(&mut po, cx, cy, cx, cy, r, rx, ry) };
        assert_eq!(pr, cr, "row48: C must treat -r like +r");
        assert_eq!(po.bits(), co.bits(), "row48: -r must give identical output");
    }
}

#[test]
fn err_spec_ray_zero_radius() {
    let mut g = Rng::new(SEED ^ 49);
    let l = libs();
    for i in 0..n() {
        let cx = (g.below(41) as f32) - 20.0;
        let cy = (g.below(41) as f32) - 20.0;
        let dist = (g.below(20) + 1) as f32;
        for r in [0.0f32, -0.0] {
            // mouse point exactly at the centre => tangent-only hit
            let mut co = SENTINEL;
            let mut ro = SENTINEL;
            let cr = unsafe { (l.c.spec_ray)(&mut co, cx, cy, cx, cy, r, cx - dist, cy) };
            let rr = unsafe { (l.r.spec_ray)(&mut ro, cx, cy, cx, cy, r, cx - dist, cy) };
            same(&format!("row49 i={i} r={r} aimed"), (cr, co), (rr, ro));
            // mouse point off to the side => miss
            let mut co = SENTINEL;
            let mut ro = SENTINEL;
            let cr = unsafe { (l.c.spec_ray)(&mut co, cx, cy + 5.0, cx, cy, r, cx - dist, cy) };
            let rr = unsafe { (l.r.spec_ray)(&mut ro, cx, cy + 5.0, cx, cy, r, cx - dist, cy) };
            same(&format!("row49 i={i} r={r} aside"), (cr, co), (rr, ro));
        }
    }
}

#[test]
fn err_spec_ray_nan_inf_args() {
    let mut g = Rng::new(SEED ^ 50);
    let l = libs();
    let specials = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001), // signalling NaN
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        f32::MIN_POSITIVE,
        f32::MAX,
        f32::MIN,
    ];
    // one special per slot, the rest a clean hit configuration
    for slot in 0..7 {
        for &s in &specials {
            let mut a = [5.0f32, 5.0, 5.0, 5.0, 2.0, -10.0, 5.0];
            a[slot] = s;
            let mut co = SENTINEL;
            let mut ro = SENTINEL;
            let cr = unsafe { (l.c.spec_ray)(&mut co, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
            let rr = unsafe { (l.r.spec_ray)(&mut ro, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
            same(&format!("row50/51 slot={slot} s={s:e}"), (cr, co), (rr, ro));
        }
    }
    // all-special cross products (bounded) + fully random bit patterns
    for i in 0..n() * 4 {
        let a = [
            g.wild(),
            g.wild(),
            g.wild(),
            g.wild(),
            g.wild(),
            g.wild(),
            g.wild(),
        ];
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.spec_ray)(&mut co, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
        let rr = unsafe { (l.r.spec_ray)(&mut ro, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
        same(&format!("row50/51 wild i={i} a={a:?}"), (cr, co), (rr, ro));
    }
    // an explicitly negative ray.t (mouse point behind the origin)
    for i in 0..1000 {
        let inf = f32::INFINITY;
        let sets: [[f32; 7]; 4] = [
            [inf, 0.0, 0.0, 0.0, 2.0, -10.0, 0.0],
            [-inf, 0.0, 0.0, 0.0, 2.0, -10.0, 0.0],
            [0.0, 0.0, 0.0, 0.0, 2.0, inf, 0.0],
            [0.0, 0.0, 0.0, 0.0, 2.0, -inf, 0.0],
        ];
        let a = sets[i % 4];
        let mut co = SENTINEL;
        let mut ro = SENTINEL;
        let cr = unsafe { (l.c.spec_ray)(&mut co, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
        let rr = unsafe { (l.r.spec_ray)(&mut ro, a[0], a[1], a[2], a[3], a[4], a[5], a[6]) };
        same(&format!("row51 i={i} a={a:?}"), (cr, co), (rr, ro));
    }
}
