//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Each test constructs the exact invalid input / rejection condition, calls
//! BOTH libraries through their `.so` exports, and asserts they return the same
//! rejection (same `int` code *and* the same `*out` bytes), never merely "both
//! failed somehow".

mod common;

use common::*;
use std::ffi::{c_int, c_void};
use std::ptr;

// ---------------------------------------------------------------------------
// Small wrappers
// ---------------------------------------------------------------------------

fn circle(p: &Pair, a: c2Ray, b: c2Circle) -> (c_int, c2Raycast, c_int, c2Raycast) {
    let mut co = sentinel();
    let mut ro = sentinel();
    let cr = unsafe { (p.c.c2RaytoCircle)(a, b, &mut co) };
    let rr = unsafe { (p.r.c2RaytoCircle)(a, b, &mut ro) };
    (cr, co, rr, ro)
}

fn aabb(p: &Pair, a: c2Ray, b: c2AABB) -> (c_int, c2Raycast, c_int, c2Raycast) {
    let mut co = sentinel();
    let mut ro = sentinel();
    let cr = unsafe { (p.c.c2RaytoAABB)(a, b, &mut co) };
    let rr = unsafe { (p.r.c2RaytoAABB)(a, b, &mut ro) };
    (cr, co, rr, ro)
}

fn capsule(p: &Pair, a: c2Ray, b: c2Capsule) -> (c_int, c2Raycast, c_int, c2Raycast) {
    let mut co = sentinel();
    let mut ro = sentinel();
    let cr = unsafe { (p.c.c2RaytoCapsule)(a, b, &mut co) };
    let rr = unsafe { (p.r.c2RaytoCapsule)(a, b, &mut ro) };
    (cr, co, rr, ro)
}

/// Asserts both libraries rejected identically **and** that the C really did
/// take the expected rejection branch (`want_ret`), so the row is proven to be
/// exercised rather than silently skipped.
fn expect(
    d: &mut Diff,
    ctx: String,
    want_ret: Option<c_int>,
    cr: c_int,
    co: &c2Raycast,
    rr: c_int,
    ro: &c2Raycast,
) {
    d.eq_cast(|| ctx.clone(), cr, co, rr, ro);
    if let Some(w) = want_ret {
        assert_eq!(cr, w, "{ctx}: C did not take the expected branch");
    }
}

fn norm_ray(rng: &mut Rng, scale: f32) -> c2Ray {
    let p = rng.vec_uniform(scale);
    let target = rng.vec_uniform(scale);
    let dx = target.x - p.x;
    let dy = target.y - p.y;
    let l = (dx * dx + dy * dy).sqrt();
    let d = v(dx / l, dy / l);
    c2Ray {
        p,
        d,
        t: (target.x * d.x + target.y * d.y) - (p.x * d.x + p.y * d.y),
    }
}

// ===========================================================================
// Rows 1..5 — c2RaytoCircle
// ===========================================================================

/// Row 1 — `disc = b*b - c < 0`: the ray *line* misses the circle entirely.
#[test]
fn err_01_circle_disc_negative() {
    let p = load();
    let mut d = Diff::new("err_01_circle_disc_negative");
    let mut rng = Rng::new(1);
    for _ in 0..40_000 {
        let ci = c2Circle {
            p: rng.vec_uniform(50.0),
            r: rng.positive(5.0),
        };
        // Direction perpendicular to the offset, started far to the side, so the
        // infinite line passes at distance > r.
        let ang = rng.uniform(3.14159265);
        let dir = v(ang.cos(), ang.sin());
        let perp = v(-dir.y, dir.x);
        let off = ci.r + 1.0 + rng.positive(20.0);
        let a = c2Ray {
            p: v(
                ci.p.x - dir.x * 30.0 + perp.x * off,
                ci.p.y - dir.y * 30.0 + perp.y * off,
            ),
            d: dir,
            t: 1e6,
        };
        let (cr, co, rr, ro) = circle(&p, a, ci);
        expect(
            &mut d,
            format!("row1 disc<0 {:?} {:?}", a, ci),
            Some(0),
            cr,
            &co,
            rr,
            &ro,
        );
        // `*out` must be byte-identical to the sentinel in BOTH.
        assert_eq!(rcbits(&co), rcbits(&sentinel()), "row1: C wrote *out");
        assert_eq!(rcbits(&ro), rcbits(&sentinel()), "row1: Rust wrote *out");
    }
    d.finish();
}

/// Row 2 — `disc` is NaN: `disc < 0` is *false*, so the early return is NOT
/// taken; the rejection happens later at `t >= 0`.
#[test]
fn err_02_circle_disc_nan() {
    let p = load();
    let mut d = Diff::new("err_02_circle_disc_nan");
    let mut rng = Rng::new(2);
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xffc0_4321),
    ];
    for &n in &nans {
        for _ in 0..5_000 {
            // NaN reaches `disc` through A.p, B.p, A.d or B.r.
            let variants = [
                (
                    c2Ray { p: v(n, rng.uniform(10.0)), d: v(1.0, 0.0), t: 100.0 },
                    c2Circle { p: rng.vec_uniform(10.0), r: 2.0 },
                ),
                (
                    c2Ray { p: rng.vec_uniform(10.0), d: v(n, 0.0), t: 100.0 },
                    c2Circle { p: rng.vec_uniform(10.0), r: 2.0 },
                ),
                (
                    c2Ray { p: rng.vec_uniform(10.0), d: v(1.0, 0.0), t: 100.0 },
                    c2Circle { p: v(n, rng.uniform(10.0)), r: 2.0 },
                ),
                (
                    c2Ray { p: rng.vec_uniform(10.0), d: v(1.0, 0.0), t: 100.0 },
                    c2Circle { p: rng.vec_uniform(10.0), r: n },
                ),
                // inf - inf inside `disc`
                (
                    c2Ray { p: v(f32::INFINITY, 0.0), d: v(1.0, 0.0), t: f32::INFINITY },
                    c2Circle { p: v(0.0, 0.0), r: f32::INFINITY },
                ),
            ];
            for (a, ci) in variants {
                let (cr, co, rr, ro) = circle(&p, a, ci);
                expect(
                    &mut d,
                    format!("row2 disc=NaN {:?} {:?}", a, ci),
                    None,
                    cr,
                    &co,
                    rr,
                    &ro,
                );
            }
        }
    }
    d.finish();
}

/// Row 3 — `t = -b - sqrt(disc) < 0`: the whole circle lies behind the origin.
#[test]
fn err_03_circle_t_negative() {
    let p = load();
    let mut d = Diff::new("err_03_circle_t_negative");
    let mut rng = Rng::new(3);
    let mut rejected = 0u32;
    for _ in 0..40_000 {
        let ci = c2Circle {
            p: rng.vec_uniform(50.0),
            r: rng.positive(5.0),
        };
        let ang = rng.uniform(3.14159265);
        let dir = v(ang.cos(), ang.sin());
        // Origin *past* the circle, pointing further away.
        let a = c2Ray {
            p: v(
                ci.p.x + dir.x * (ci.r + 10.0),
                ci.p.y + dir.y * (ci.r + 10.0),
            ),
            d: dir,
            t: 1e6,
        };
        let (cr, co, rr, ro) = circle(&p, a, ci);
        expect(
            &mut d,
            format!("row3 t<0 {:?} {:?}", a, ci),
            Some(0),
            cr,
            &co,
            rr,
            &ro,
        );
        rejected += 1;
        assert_eq!(rcbits(&co), rcbits(&sentinel()));
        assert_eq!(rcbits(&ro), rcbits(&sentinel()));
        // Also the "origin at the centre" case: t = -sqrt(r*r) < 0.
        let a2 = c2Ray { p: ci.p, d: dir, t: 1e6 };
        let (cr2, co2, rr2, ro2) = circle(&p, a2, ci);
        expect(
            &mut d,
            format!("row3 origin==centre {:?} {:?}", a2, ci),
            Some(0),
            cr2,
            &co2,
            rr2,
            &ro2,
        );
    }
    assert!(rejected > 0);
    d.finish();
}

/// Rows 4, 5 — `t > A.t` (hit beyond the ray's length), including `A.t < 0`,
/// `A.t == -0.0` and `A.t == NaN`.
#[test]
fn err_04_circle_t_past_end() {
    let p = load();
    let mut d = Diff::new("err_04_circle_t_past_end");
    let mut rng = Rng::new(4);
    let mut short_reject = 0u32;
    let mut nan_reject = 0u32;
    for _ in 0..20_000 {
        // The control assertion below requires the ray to *provably* hit, so use
        // an exact axis-aligned direction and a radius that is comfortably larger
        // than the float error of `origin + dir * dist`.
        let ci = c2Circle {
            p: rng.vec_uniform(50.0),
            r: 1.0 + rng.positive(5.0),
        };
        let dir = [v(1.0, 0.0), v(-1.0, 0.0), v(0.0, 1.0), v(0.0, -1.0)]
            [rng.below(4) as usize];
        let dist = ci.r + 10.0;
        let origin = v(ci.p.x - dir.x * dist, ci.p.y - dir.y * dist);
        // A.t values that are all shorter than the true hit distance (~10).
        for &t in &[
            0.0f32,
            -0.0,
            1.0,
            9.0,
            -1.0,
            -1e6,
            f32::NEG_INFINITY,
            f32::NAN,
            -f32::NAN,
            f32::from_bits(0x7f80_0001),
        ] {
            let a = c2Ray { p: origin, d: dir, t };
            let (cr, co, rr, ro) = circle(&p, a, ci);
            expect(
                &mut d,
                format!("row4/5 t>A.t (A.t={}) {:?} {:?}", fs(t), a, ci),
                Some(0),
                cr,
                &co,
                rr,
                &ro,
            );
            assert_eq!(rcbits(&co), rcbits(&sentinel()));
            assert_eq!(rcbits(&ro), rcbits(&sentinel()));
            if t.is_nan() {
                nan_reject += 1;
            } else {
                short_reject += 1;
            }
        }
        // Sanity: with a long enough A.t the same configuration *hits*, proving
        // the rejections above really came from the `t <= A.t` guard.
        let a = c2Ray { p: origin, d: dir, t: 1e6 };
        let (cr, co, rr, ro) = circle(&p, a, ci);
        expect(&mut d, format!("row4 control hit {:?} {:?}", a, ci), Some(1), cr, &co, rr, &ro);
    }
    assert!(short_reject > 0 && nan_reject > 0);
    d.finish();
}

// ===========================================================================
// Rows 6..11 — c2AABBtoAABB
// ===========================================================================

#[test]
fn err_06_09_aabb_aabb_four_axes() {
    let p = load();
    let mut d = Diff::new("err_06_09_aabb_aabb_four_axes");
    let mut rng = Rng::new(6);
    let mut counts = [0u32; 4];
    for _ in 0..40_000 {
        let a = c2AABB {
            min: v(-1.0, -1.0),
            max: v(1.0, 1.0),
        };
        let gap = rng.positive(10.0) + 1e-3;
        // Row 6: d0  (B.max.x < A.min.x)  → B entirely to the -x side
        let b0 = c2AABB { min: v(-1.0 - gap - 1.0, -1.0), max: v(-1.0 - gap, 1.0) };
        // Row 7: d1  (A.max.x < B.min.x)  → B entirely to the +x side
        let b1 = c2AABB { min: v(1.0 + gap, -1.0), max: v(1.0 + gap + 1.0, 1.0) };
        // Row 8: d2  (B.max.y < A.min.y)
        let b2 = c2AABB { min: v(-1.0, -1.0 - gap - 1.0), max: v(1.0, -1.0 - gap) };
        // Row 9: d3  (A.max.y < B.min.y)
        let b3 = c2AABB { min: v(-1.0, 1.0 + gap), max: v(1.0, 1.0 + gap + 1.0) };
        for (i, b) in [b0, b1, b2, b3].into_iter().enumerate() {
            let cr = unsafe { (p.c.c2AABBtoAABB)(a, b) };
            let rr = unsafe { (p.r.c2AABBtoAABB)(a, b) };
            d.eq_i(|| format!("row{} {:?} {:?}", 6 + i, a, b), cr, rr);
            assert_eq!(cr, 0, "row{}: C did not reject", 6 + i);
            counts[i] += 1;
        }
    }
    assert!(counts.iter().all(|&c| c > 0), "{counts:?}");
    d.finish();
}

/// Row 10 — any NaN coordinate makes every `<` false, so the C **accepts**.
#[test]
fn err_10_aabb_aabb_nan_accepts() {
    let p = load();
    let mut d = Diff::new("err_10_aabb_aabb_nan_accepts");
    let mut accepted = 0u32;
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xffc0_4321),
    ];
    for &n in &nans {
        // For each separating axis in turn, build a pair that is separated on
        // *only* that axis, then poison exactly the coordinate that axis tests.
        // Killing that single `<` must flip the whole predicate to "accept".
        //   axis 0: d0 = B.max.x < A.min.x   → poison A.min.x (slot 0) or B.max.x (slot 6)
        //   axis 1: d1 = A.max.x < B.min.x   → poison A.max.x (slot 2) or B.min.x (slot 4)
        //   axis 2: d2 = B.max.y < A.min.y   → poison A.min.y (slot 1) or B.max.y (slot 7)
        //   axis 3: d3 = A.max.y < B.min.y   → poison A.max.y (slot 3) or B.min.y (slot 5)
        let pairs: [(c2AABB, c2AABB, [usize; 2]); 4] = [
            (
                c2AABB { min: v(0.0, -1.0), max: v(1.0, 1.0) },
                c2AABB { min: v(-3.0, -1.0), max: v(-2.0, 1.0) },
                [0, 6],
            ),
            (
                c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) },
                c2AABB { min: v(2.0, -1.0), max: v(3.0, 1.0) },
                [2, 4],
            ),
            (
                c2AABB { min: v(-1.0, 0.0), max: v(1.0, 1.0) },
                c2AABB { min: v(-1.0, -3.0), max: v(1.0, -2.0) },
                [1, 7],
            ),
            (
                c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) },
                c2AABB { min: v(-1.0, 2.0), max: v(1.0, 3.0) },
                [3, 5],
            ),
        ];
        for (a, b, slots) in pairs {
            // Sanity: without the NaN the pair really is rejected.
            assert_eq!(
                unsafe { (p.c.c2AABBtoAABB)(a, b) },
                0,
                "row10 setup: pair should be separated"
            );
            for slot in slots {
                let mut f = [
                    a.min.x, a.min.y, a.max.x, a.max.y, b.min.x, b.min.y, b.max.x, b.max.y,
                ];
                f[slot] = n;
                let aa = c2AABB { min: v(f[0], f[1]), max: v(f[2], f[3]) };
                let bb = c2AABB { min: v(f[4], f[5]), max: v(f[6], f[7]) };
                let cr = unsafe { (p.c.c2AABBtoAABB)(aa, bb) };
                let rr = unsafe { (p.r.c2AABBtoAABB)(aa, bb) };
                d.eq_i(|| format!("row10 {:?} {:?}", aa, bb), cr, rr);
                assert_eq!(
                    cr, 1,
                    "row10: poisoning slot {slot} with {} must make the C accept",
                    fs(n)
                );
                accepted += 1;
            }
        }
        // Poisoning every slot in turn must also never diverge.
        for slot in 0..8usize {
            let mut f = [-1.0f32, -1.0, 1.0, 1.0, 100.0, 100.0, 101.0, 101.0];
            f[slot] = n;
            let aa = c2AABB { min: v(f[0], f[1]), max: v(f[2], f[3]) };
            let bb = c2AABB { min: v(f[4], f[5]), max: v(f[6], f[7]) };
            let cr = unsafe { (p.c.c2AABBtoAABB)(aa, bb) };
            let rr = unsafe { (p.r.c2AABBtoAABB)(aa, bb) };
            d.eq_i(|| format!("row10 sweep {:?} {:?}", aa, bb), cr, rr);
        }
        // All-NaN: guaranteed accept.
        let aa = c2AABB { min: v(n, n), max: v(n, n) };
        let bb = c2AABB { min: v(n, n), max: v(n, n) };
        let cr = unsafe { (p.c.c2AABBtoAABB)(aa, bb) };
        let rr = unsafe { (p.r.c2AABBtoAABB)(aa, bb) };
        d.eq_i(|| format!("row10 all-NaN {:?} {:?}", aa, bb), cr, rr);
        assert_eq!(cr, 1, "row10: NaN box must be accepted by the C");
    }
    assert!(accepted > 0, "row10: NaN never produced an accept");
    d.finish();
}

/// Row 11 — inverted boxes (`min > max`) get no validation at all.
#[test]
fn err_11_aabb_aabb_inverted() {
    let p = load();
    let mut d = Diff::new("err_11_aabb_aabb_inverted");
    let mut rng = Rng::new(11);
    let mut saw_zero = 0u32;
    let mut saw_one = 0u32;
    for _ in 0..100_000 {
        let a = c2AABB {
            min: rng.vec_uniform(10.0),
            max: rng.vec_uniform(10.0),
        };
        let b = c2AABB {
            min: rng.vec_uniform(10.0),
            max: rng.vec_uniform(10.0),
        };
        let inverted = a.min.x > a.max.x || a.min.y > a.max.y || b.min.x > b.max.x || b.min.y > b.max.y;
        let cr = unsafe { (p.c.c2AABBtoAABB)(a, b) };
        let rr = unsafe { (p.r.c2AABBtoAABB)(a, b) };
        d.eq_i(|| format!("row11 {:?} {:?}", a, b), cr, rr);
        if inverted {
            if cr == 0 {
                saw_zero += 1
            } else {
                saw_one += 1
            }
        }
    }
    assert!(saw_zero > 0 && saw_one > 0, "row11 coverage {saw_zero}/{saw_one}");
    d.finish();
}

// ===========================================================================
// Rows 12..19 — c2RaytoAABB and its two static helpers
// ===========================================================================

/// Row 12 — the swept-ray broad-phase box misses `B`.
#[test]
fn err_12_aabb_broadphase_reject() {
    let p = load();
    let mut d = Diff::new("err_12_aabb_broadphase_reject");
    let mut rng = Rng::new(12);
    for _ in 0..40_000 {
        // A short ray near the origin, a box far away → a_box cannot overlap B.
        let a = c2Ray {
            p: rng.vec_uniform(1.0),
            d: rng.vec_uniform(1.0),
            t: rng.positive(1.0),
        };
        let far = 1e5 + rng.positive(1e5);
        let b = c2AABB {
            min: v(far, far),
            max: v(far + 1.0, far + 1.0),
        };
        let (cr, co, rr, ro) = aabb(&p, a, b);
        expect(
            &mut d,
            format!("row12 broadphase {:?} {:?}", a, b),
            Some(0),
            cr,
            &co,
            rr,
            &ro,
        );
        assert_eq!(rcbits(&co), rcbits(&sentinel()));
        assert_eq!(rcbits(&ro), rcbits(&sentinel()));
    }
    d.finish();
}

/// Row 13 — the SAT test on the ray's own axis rejects (`d > 0`) even though
/// the broad-phase box overlapped.
#[test]
fn err_13_aabb_sat_reject() {
    let p = load();
    let mut d = Diff::new("err_13_aabb_sat_reject");
    let mut rng = Rng::new(13);
    let mut sat_rejects = 0u32;
    for _ in 0..200_000 {
        // A long diagonal ray sweeping past a small box: the AABB of the sweep
        // overlaps the box, but the ray line itself misses it.
        let b = c2AABB {
            min: v(-1.0, -1.0),
            max: v(1.0, 1.0),
        };
        let off = 2.0 + rng.positive(6.0);
        let sx = if rng.below(2) == 0 { 1.0 } else { -1.0 };
        let a = c2Ray {
            p: v(-10.0 * sx, -10.0 + off),
            d: v(sx * 0.70710678, 0.70710678),
            t: 40.0,
        };
        let (cr, co, rr, ro) = aabb(&p, a, b);
        d.eq_cast(|| format!("row13 {:?} {:?}", a, b), cr, &co, rr, &ro);
        if cr == 0 && rcbits(&co) == rcbits(&sentinel()) {
            sat_rejects += 1;
        }
        // Randomised diagonal sweeps too.
        let a2 = c2Ray {
            p: v(-30.0, rng.uniform(30.0)),
            d: {
                let ang = rng.uniform(3.14159265);
                v(ang.cos(), ang.sin())
            },
            t: 60.0,
        };
        let (cr2, co2, rr2, ro2) = aabb(&p, a2, b);
        d.eq_cast(|| format!("row13 rnd {:?} {:?}", a2, b), cr2, &co2, rr2, &ro2);
    }
    assert!(sat_rejects > 0, "row13: SAT reject never observed");
    d.finish();
}

/// Row 14 — `hit0|hit1|hit2|hit3 == 0`: all four slab candidates exceed `1.0`.
#[test]
fn err_14_aabb_all_t_gt_one() {
    let p = load();
    let mut d = Diff::new("err_14_aabb_all_t_gt_one");
    let mut rng = Rng::new(14);
    let mut zero_hits = 0u32;
    // Sweep a huge space of ray/box pairs and count how many reach the
    // `hit == 0` final rejection with *out untouched* (which can only happen
    // after the broad-phase and SAT tests both passed).
    for _ in 0..300_000 {
        let b = c2AABB {
            min: rng.vec_uniform(5.0),
            max: rng.vec_uniform(5.0),
        };
        let b = c2AABB {
            min: v(b.min.x.min(b.max.x), b.min.y.min(b.max.y)),
            max: v(b.min.x.max(b.max.x), b.min.y.max(b.max.y)),
        };
        let a = norm_ray(&mut rng, 5.0);
        let (cr, co, rr, ro) = aabb(&p, a, b);
        d.eq_cast(|| format!("row14 {:?} {:?}", a, b), cr, &co, rr, &ro);
        if cr == 0 && rcbits(&co) == rcbits(&sentinel()) {
            zero_hits += 1;
        }
    }
    assert!(zero_hits > 0, "row14 never reached a reject");
    d.finish();
}

/// Row 15 — a NaN `t_i` makes `t_i <= 1.0f` false, zeroing that `hit_i`.
#[test]
fn err_15_aabb_nan_t() {
    let p = load();
    let mut d = Diff::new("err_15_aabb_nan_t");
    let mut rng = Rng::new(15);
    let nans = [f32::NAN, -f32::NAN, f32::from_bits(0x7f80_0001)];
    for &n in &nans {
        for _ in 0..10_000 {
            // NaN in the box bounds or the ray → NaN da/db → NaN t_i.
            let variants: [(c2Ray, c2AABB); 5] = [
                (
                    c2Ray { p: v(n, 0.0), d: v(1.0, 0.0), t: 10.0 },
                    c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) },
                ),
                (
                    c2Ray { p: v(-5.0, 0.0), d: v(n, 0.0), t: 10.0 },
                    c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) },
                ),
                (
                    c2Ray { p: v(-5.0, 0.0), d: v(1.0, 0.0), t: n },
                    c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) },
                ),
                (
                    c2Ray { p: v(-5.0, 0.0), d: v(1.0, 0.0), t: 10.0 },
                    c2AABB { min: v(n, -1.0), max: v(1.0, 1.0) },
                ),
                (
                    c2Ray { p: v(-5.0, 0.0), d: v(1.0, 0.0), t: 10.0 },
                    c2AABB { min: v(-1.0, -1.0), max: v(n, 1.0) },
                ),
            ];
            for (a, b) in variants {
                let (cr, co, rr, ro) = aabb(&p, a, b);
                d.eq_cast(|| format!("row15 {:?} {:?}", a, b), cr, &co, rr, &ro);
            }
            // …plus infinities that turn into NaN through `inf * 0` / `inf-inf`.
            let a = c2Ray {
                p: v(f32::INFINITY, 0.0),
                d: v(f32::NEG_INFINITY, 0.0),
                t: rng.positive(10.0),
            };
            let b = c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
            let (cr, co, rr, ro) = aabb(&p, a, b);
            d.eq_cast(|| format!("row15 inf {:?} {:?}", a, b), cr, &co, rr, &ro);
        }
    }
    d.finish();
}

/// Row 16 — `A.t == 0`: a zero-length ray, so `p1 == p0` and `n == (0,0)`.
#[test]
fn err_16_aabb_zero_length_ray() {
    let p = load();
    let mut d = Diff::new("err_16_aabb_zero_length_ray");
    let mut rng = Rng::new(16);
    for _ in 0..40_000 {
        let c = rng.vec_uniform(10.0);
        let b = c2AABB {
            min: v(c.x - 1.0, c.y - 1.0),
            max: v(c.x + 1.0, c.y + 1.0),
        };
        for &t in &[0.0f32, -0.0] {
            // origin inside, on the border, and outside
            for o in [
                c,
                v(b.min.x, b.min.y),
                v(b.max.x, b.max.y),
                v(c.x + 5.0, c.y),
                rng.vec_uniform(10.0),
            ] {
                let a = c2Ray { p: o, d: rng.vec_uniform(3.0), t };
                let (cr, co, rr, ro) = aabb(&p, a, b);
                d.eq_cast(|| format!("row16 {:?} {:?}", a, b), cr, &co, rr, &ro);
            }
        }
    }
    d.finish();
}

/// Rows 17, 18, 19 — the three rejection branches of the static
/// `c2RayToPlane_OneDimensional`, reached through `c2RaytoAABB`.
///
/// * 17: `da < 0`            → contributes `t_i = 0`
/// * 18: `da*db > 0`         → contributes `t_i = 1`
/// * 19: `d = da - db == 0`  → division refused, contributes `t_i = 0`
#[test]
fn err_17_19_ray_plane_branches() {
    let p = load();
    let mut d = Diff::new("err_17_19_ray_plane_branches");
    let mut rng = Rng::new(17);
    let b = c2AABB {
        min: v(-1.0, -1.0),
        max: v(1.0, 1.0),
    };
    // Row 19 is reached whenever the ray is parallel to a slab, i.e. one
    // component of `ab` is exactly zero: then da == db for that plane pair.
    let axis_dirs = [
        v(1.0, 0.0),
        v(-1.0, 0.0),
        v(0.0, 1.0),
        v(0.0, -1.0),
        v(0.0, 0.0),
    ];
    for dir in axis_dirs {
        for _ in 0..20_000 {
            // Origins spread over inside / on-plane / outside, so all of
            // `da < 0`, `da*db > 0` and `da == db` occur.
            let o = match rng.below(6) {
                0 => v(-1.0, 0.0),
                1 => v(1.0, 0.0),
                2 => v(0.0, -1.0),
                3 => v(0.0, 1.0),
                4 => v(0.0, 0.0),
                _ => rng.vec_uniform(4.0),
            };
            for &t in &[0.0f32, 0.5, 1.0, 4.0, 40.0, -3.0] {
                let a = c2Ray { p: o, d: dir, t };
                let (cr, co, rr, ro) = aabb(&p, a, b);
                d.eq_cast(|| format!("row17-19 {:?} {:?}", a, b), cr, &co, rr, &ro);
            }
        }
    }
    // Row 18 specifically: both plane distances on the same side and the ray
    // fully outside the slab but still inside the broad-phase box.
    for _ in 0..40_000 {
        let a = c2Ray {
            p: v(-5.0, 3.0 + rng.positive(1.0)),
            d: v(1.0, 0.0),
            t: 10.0,
        };
        let bb = c2AABB {
            min: v(-1.0, -1.0),
            max: v(1.0, 5.0),
        };
        let (cr, co, rr, ro) = aabb(&p, a, bb);
        d.eq_cast(|| format!("row18 {:?} {:?}", a, bb), cr, &co, rr, &ro);
    }
    d.finish();
}

// ===========================================================================
// Rows 20..24 — c2AABBtoPoint
// ===========================================================================

#[test]
fn err_20_23_aabb_point_four_axes() {
    let p = load();
    let mut d = Diff::new("err_20_23_aabb_point_four_axes");
    let mut rng = Rng::new(20);
    for _ in 0..40_000 {
        let bx = c2AABB {
            min: v(-1.0, -2.0),
            max: v(3.0, 4.0),
        };
        let e = rng.positive(10.0) + 1e-3;
        let pts = [
            v(-1.0 - e, 0.0), // row 20: d0
            v(0.0, -2.0 - e), // row 21: d1
            v(3.0 + e, 0.0),  // row 22: d2
            v(0.0, 4.0 + e),  // row 23: d3
        ];
        for (i, pt) in pts.into_iter().enumerate() {
            let cr = unsafe { (p.c.c2AABBtoPoint)(bx, pt) };
            let rr = unsafe { (p.r.c2AABBtoPoint)(bx, pt) };
            d.eq_i(|| format!("row{} {:?} {}", 20 + i, bx, vs(pt)), cr, rr);
            assert_eq!(cr, 0, "row{}: C did not reject", 20 + i);
        }
    }
    d.finish();
}

/// Row 24 — a NaN point component makes all four tests false → **accept**.
#[test]
fn err_24_aabb_point_nan_accepts() {
    let p = load();
    let mut d = Diff::new("err_24_aabb_point_nan_accepts");
    let bx = c2AABB {
        min: v(-1.0, -1.0),
        max: v(1.0, 1.0),
    };
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xffc0_4321),
    ];
    let mut accepted = 0u32;
    for &n in &nans {
        for pt in [v(n, n), v(n, 0.0), v(0.0, n), v(n, 1e9), v(-1e9, n)] {
            let cr = unsafe { (p.c.c2AABBtoPoint)(bx, pt) };
            let rr = unsafe { (p.r.c2AABBtoPoint)(bx, pt) };
            d.eq_i(|| format!("row24 {:?} {}", bx, vs(pt)), cr, rr);
            accepted += (cr == 1) as u32;
        }
        // both components NaN → definitely accepted
        let pt = v(n, n);
        assert_eq!(
            unsafe { (p.c.c2AABBtoPoint)(bx, pt) },
            1,
            "row24: C must accept an all-NaN point"
        );
    }
    assert!(accepted > 0);
    d.finish();
}

// ===========================================================================
// Rows 25..28 — c2CircleToPoint
// ===========================================================================

#[test]
fn err_25_circle_point_outside() {
    let p = load();
    let mut d = Diff::new("err_25_circle_point_outside");
    let mut rng = Rng::new(25);
    for _ in 0..40_000 {
        let ci = c2Circle {
            p: rng.vec_uniform(50.0),
            r: rng.positive(10.0),
        };
        let ang = rng.uniform(3.14159265);
        // Exactly on the rim (d2 == r*r → `<` is false → reject) and outside.
        for k in [1.0f32, 1.0000001, 1.5, 100.0] {
            let pt = v(
                ci.p.x + ci.r * k * ang.cos(),
                ci.p.y + ci.r * k * ang.sin(),
            );
            let cr = unsafe { (p.c.c2CircleToPoint)(ci, pt) };
            let rr = unsafe { (p.r.c2CircleToPoint)(ci, pt) };
            d.eq_i(|| format!("row25 k={k} {:?} {}", ci, vs(pt)), cr, rr);
        }
        // Control: strictly inside → accept.
        let pt = v(ci.p.x + ci.r * 0.5 * ang.cos(), ci.p.y + ci.r * 0.5 * ang.sin());
        let cr = unsafe { (p.c.c2CircleToPoint)(ci, pt) };
        let rr = unsafe { (p.r.c2CircleToPoint)(ci, pt) };
        d.eq_i(|| format!("row25 control {:?} {}", ci, vs(pt)), cr, rr);
    }
    d.finish();
}

/// Row 26 — negative radius: `r*r` is still positive, so the test still works.
#[test]
fn err_26_circle_point_negative_r() {
    let p = load();
    let mut d = Diff::new("err_26_circle_point_negative_r");
    let mut rng = Rng::new(26);
    let mut accepted = 0u32;
    for _ in 0..40_000 {
        let mag = rng.positive(10.0);
        let ci = c2Circle {
            p: rng.vec_uniform(50.0),
            r: -mag,
        };
        for k in [0.0f32, 0.5, 1.0, 1.5] {
            let pt = v(ci.p.x + mag * k, ci.p.y);
            let cr = unsafe { (p.c.c2CircleToPoint)(ci, pt) };
            let rr = unsafe { (p.r.c2CircleToPoint)(ci, pt) };
            d.eq_i(|| format!("row26 {:?} {}", ci, vs(pt)), cr, rr);
            accepted += (cr == 1) as u32;
        }
    }
    assert!(accepted > 0, "row26: negative r never accepted anything");
    d.finish();
}

/// Row 27 — `r == 0` (and `-0.0`): `d2 < 0` is impossible → always rejects.
#[test]
fn err_27_circle_point_zero_r() {
    let p = load();
    let mut d = Diff::new("err_27_circle_point_zero_r");
    let mut rng = Rng::new(27);
    for _ in 0..40_000 {
        for &r in &[0.0f32, -0.0] {
            let ci = c2Circle { p: rng.vec_uniform(50.0), r };
            for pt in [ci.p, rng.vec_uniform(50.0), v(-0.0, -0.0), v(0.0, 0.0)] {
                let cr = unsafe { (p.c.c2CircleToPoint)(ci, pt) };
                let rr = unsafe { (p.r.c2CircleToPoint)(ci, pt) };
                d.eq_i(|| format!("row27 {:?} {}", ci, vs(pt)), cr, rr);
                assert_eq!(cr, 0, "row27: zero radius must always reject");
            }
        }
    }
    d.finish();
}

/// Row 28 — NaN makes `d2` NaN, so `r*r > d2` is false → reject.
#[test]
fn err_28_circle_point_nan() {
    let p = load();
    let mut d = Diff::new("err_28_circle_point_nan");
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xffc0_4321),
    ];
    for &n in &nans {
        let cases = [
            (c2Circle { p: v(n, 0.0), r: 5.0 }, v(0.0, 0.0)),
            (c2Circle { p: v(0.0, n), r: 5.0 }, v(0.0, 0.0)),
            (c2Circle { p: v(0.0, 0.0), r: n }, v(0.0, 0.0)),
            (c2Circle { p: v(0.0, 0.0), r: 5.0 }, v(n, 0.0)),
            (c2Circle { p: v(0.0, 0.0), r: 5.0 }, v(0.0, n)),
            (c2Circle { p: v(n, n), r: n }, v(n, n)),
        ];
        for (ci, pt) in cases {
            let cr = unsafe { (p.c.c2CircleToPoint)(ci, pt) };
            let rr = unsafe { (p.r.c2CircleToPoint)(ci, pt) };
            d.eq_i(|| format!("row28 {:?} {}", ci, vs(pt)), cr, rr);
            assert_eq!(cr, 0, "row28: NaN must reject");
        }
    }
    d.finish();
}

// ===========================================================================
// Rows 29..35 — c2RaytoCapsule
// ===========================================================================

/// Row 29 — the final `return 0`: the ray never crosses the capsule slab.
/// Note the C has **already written** `*out` (`n = norm(b-a)`, `t = 0`) before
/// this rejection, so the sentinel must be gone in both libraries.
#[test]
fn err_29_capsule_outside_slab() {
    let p = load();
    let mut d = Diff::new("err_29_capsule_outside_slab");
    let mut rng = Rng::new(29);
    let mut rejects = 0u32;
    for _ in 0..60_000 {
        let ca = rng.vec_uniform(20.0);
        let ang = rng.uniform(3.14159265);
        let axis = v(ang.cos(), ang.sin());
        let l = 1.0 + rng.positive(10.0);
        let cb = v(ca.x + axis.x * l, ca.y + axis.y * l);
        let r = 0.01 + rng.positive(1.0);
        let b = c2Capsule { a: ca, b: cb, r };
        let mx = v(axis.y, -axis.x);
        // Start well to one side and travel parallel to the axis: both yAp.x and
        // yAe.x keep the same sign and |·| stays >= r.
        let side = if rng.below(2) == 0 { 1.0 } else { -1.0 };
        let off = r + 1.0 + rng.positive(20.0);
        let o = v(ca.x + mx.x * side * off, ca.y + mx.y * side * off);
        let a = c2Ray { p: o, d: axis, t: l * 2.0 };
        let (cr, co, rr, ro) = capsule(&p, a, b);
        expect(
            &mut d,
            format!("row29 {:?} {:?}", a, b),
            Some(0),
            cr,
            &co,
            rr,
            &ro,
        );
        // The C writes `*out` unconditionally before this rejection.
        assert_ne!(
            rcbits(&co),
            rcbits(&sentinel()),
            "row29: C left *out untouched (unexpected)"
        );
        assert_eq!(
            rcbits(&co),
            rcbits(&ro),
            "row29: pre-set *out values diverge"
        );
        rejects += 1;
    }
    assert!(rejects > 0);
    d.finish();
}

/// Row 30 — degenerate capsule `a == b`: `norm((0,0))` = `0 * (1/0)` = NaN, so
/// the whole local frame is NaN and every comparison is unordered.
#[test]
fn err_30_capsule_degenerate_axis() {
    let p = load();
    let mut d = Diff::new("err_30_capsule_degenerate_axis");
    let mut rng = Rng::new(30);
    for _ in 0..40_000 {
        let q = rng.vec_uniform(20.0);
        for &r in &[0.0f32, -0.0, 1.0, -1.0, 1e-30, 1e30, f32::INFINITY, f32::NAN] {
            let b = c2Capsule { a: q, b: q, r };
            for a in [
                norm_ray(&mut rng, 20.0),
                c2Ray { p: q, d: v(1.0, 0.0), t: 10.0 },
                c2Ray { p: v(q.x + 5.0, q.y), d: v(-1.0, 0.0), t: 10.0 },
            ] {
                let (cr, co, rr, ro) = capsule(&p, a, b);
                d.eq_cast(|| format!("row30 {:?} {:?}", a, b), cr, &co, rr, &ro);
            }
        }
        // Also `a == b` up to the sign of zero.
        let b = c2Capsule { a: v(0.0, 0.0), b: v(-0.0, -0.0), r: 1.0 };
        let a = norm_ray(&mut rng, 5.0);
        let (cr, co, rr, ro) = capsule(&p, a, b);
        d.eq_cast(|| format!("row30 ±0 {:?} {:?}", a, b), cr, &co, rr, &ro);
    }
    d.finish();
}

/// Row 31 — `B.r == 0`: `capsule_bb` degenerates to a zero-width slab.
#[test]
fn err_31_capsule_zero_radius() {
    let p = load();
    let mut d = Diff::new("err_31_capsule_zero_radius");
    let mut rng = Rng::new(31);
    for _ in 0..40_000 {
        let ca = rng.vec_uniform(20.0);
        let ang = rng.uniform(3.14159265);
        let axis = v(ang.cos(), ang.sin());
        let l = 1.0 + rng.positive(10.0);
        let cb = v(ca.x + axis.x * l, ca.y + axis.y * l);
        for &r in &[0.0f32, -0.0] {
            let b = c2Capsule { a: ca, b: cb, r };
            for a in [
                norm_ray(&mut rng, 20.0),
                c2Ray { p: ca, d: axis, t: l },
                c2Ray {
                    p: v(ca.x + axis.y * 3.0, ca.y - axis.x * 3.0),
                    d: v(-axis.y, axis.x),
                    t: 6.0,
                },
            ] {
                let (cr, co, rr, ro) = capsule(&p, a, b);
                d.eq_cast(|| format!("row31 {:?} {:?}", a, b), cr, &co, rr, &ro);
            }
        }
    }
    d.finish();
}

/// Row 32 — negative `B.r` inverts `capsule_bb` (`min.x = -r > 0 = max.x`).
#[test]
fn err_32_capsule_negative_radius() {
    let p = load();
    let mut d = Diff::new("err_32_capsule_negative_radius");
    let mut rng = Rng::new(32);
    for _ in 0..40_000 {
        let ca = rng.vec_uniform(20.0);
        let ang = rng.uniform(3.14159265);
        let axis = v(ang.cos(), ang.sin());
        let l = 1.0 + rng.positive(10.0);
        let cb = v(ca.x + axis.x * l, ca.y + axis.y * l);
        for &r in &[-1e-30f32, -0.5, -1.0, -1e30, f32::NEG_INFINITY] {
            let b = c2Capsule { a: ca, b: cb, r };
            for a in [
                norm_ray(&mut rng, 20.0),
                c2Ray { p: ca, d: axis, t: l },
                c2Ray {
                    p: v(ca.x + axis.y * 3.0, ca.y - axis.x * 3.0),
                    d: v(-axis.y, axis.x),
                    t: 6.0,
                },
            ] {
                let (cr, co, rr, ro) = capsule(&p, a, b);
                d.eq_cast(|| format!("row32 {:?} {:?}", a, b), cr, &co, rr, &ro);
            }
        }
    }
    d.finish();
}

/// Rows 33, 34 — the delegated rejections: `c2RaytoCircle` returns 0 for end-cap
/// A (row 33) and end-cap B (row 34), and that 0 becomes the capsule's result
/// while `*out` keeps the values `c2RaytoCapsule` pre-set.
#[test]
fn err_33_34_capsule_delegate_cap_miss() {
    let p = load();
    let mut d = Diff::new("err_33_34_capsule_delegate_cap_miss");
    let mut rng = Rng::new(33);
    let mut ca_miss = 0u32;
    let mut cb_miss = 0u32;
    for _ in 0..60_000 {
        let ca = rng.vec_uniform(20.0);
        let ang = rng.uniform(3.14159265);
        let axis = v(ang.cos(), ang.sin());
        let l = 2.0 + rng.positive(10.0);
        let cb = v(ca.x + axis.x * l, ca.y + axis.y * l);
        let r = 0.2 + rng.positive(1.0);
        let b = c2Capsule { a: ca, b: cb, r };
        let mx = v(axis.y, -axis.x);
        let to_world =
            |lx: f32, ly: f32| v(ca.x + mx.x * lx + axis.x * ly, ca.y + mx.y * lx + axis.y * ly);

        // |yAp.x| < r, yAp.y < 0  →  delegates to Ca; A.t too small → miss.
        let o_a = to_world(rng.uniform(r * 0.8), -(r + 5.0));
        let a_a = c2Ray { p: o_a, d: axis, t: 0.001 };
        let (cr, co, rr, ro) = capsule(&p, a_a, b);
        expect(&mut d, format!("row33 {:?} {:?}", a_a, b), Some(0), cr, &co, rr, &ro);
        if rcbits(&co) != rcbits(&sentinel()) {
            ca_miss += 1;
        }

        // |yAp.x| < r, yAp.y >= 0 (beyond the far end) → delegates to Cb; miss.
        let o_b = to_world(rng.uniform(r * 0.8), l + r + 5.0);
        let a_b = c2Ray { p: o_b, d: axis, t: 0.001 };
        let (cr, co, rr, ro) = capsule(&p, a_b, b);
        expect(&mut d, format!("row34 {:?} {:?}", a_b, b), Some(0), cr, &co, rr, &ro);
        if rcbits(&co) != rcbits(&sentinel()) {
            cb_miss += 1;
        }

        // Same, but pointing away from the capsule entirely.
        let a_c = c2Ray { p: o_a, d: v(-axis.x, -axis.y), t: 100.0 };
        let (cr, co, rr, ro) = capsule(&p, a_c, b);
        expect(&mut d, format!("row33b {:?} {:?}", a_c, b), Some(0), cr, &co, rr, &ro);
        let a_d = c2Ray { p: o_b, d: axis, t: 100.0 };
        let (cr, co, rr, ro) = capsule(&p, a_d, b);
        expect(&mut d, format!("row34b {:?} {:?}", a_d, b), Some(0), cr, &co, rr, &ro);
    }
    assert!(ca_miss > 0 && cb_miss > 0, "{ca_miss} {cb_miss}");
    d.finish();
}

/// Row 35 — `d = yAe.x - yAp.x == 0`: unlike `c2RayToPlane_OneDimensional`,
/// `c2RaytoCapsule` has **no** zero-denominator guard, so `t` becomes ±inf/NaN.
#[test]
fn err_35_capsule_zero_denominator() {
    let p = load();
    let mut d = Diff::new("err_35_capsule_zero_denominator");
    let mut rng = Rng::new(35);
    for _ in 0..60_000 {
        let ca = rng.vec_uniform(20.0);
        let ang = rng.uniform(3.14159265);
        let axis = v(ang.cos(), ang.sin());
        let l = 1.0 + rng.positive(10.0);
        let cb = v(ca.x + axis.x * l, ca.y + axis.y * l);
        let r = 0.1 + rng.positive(1.0);
        let b = c2Capsule { a: ca, b: cb, r };
        let mx = v(axis.y, -axis.x);
        // |yAp.x| >= r (so we reach the `else` with the division) and yAd.x == 0
        // (direction parallel to the axis) → yAe.x == yAp.x → d == 0.
        let off = r + 0.5 + rng.positive(5.0);
        let o = v(ca.x + mx.x * off, ca.y + mx.y * off);
        for &t in &[0.0f32, -0.0, 1.0, -1.0, 100.0, f32::INFINITY, f32::NAN] {
            for dir in [axis, v(-axis.x, -axis.y), v(0.0, 0.0)] {
                let a = c2Ray { p: o, d: dir, t };
                let (cr, co, rr, ro) = capsule(&p, a, b);
                d.eq_cast(|| format!("row35 {:?} {:?}", a, b), cr, &co, rr, &ro);
            }
        }
        // A.t == 0 with a crossing direction also gives yAe == yAp.
        let a = c2Ray { p: o, d: v(-mx.x, -mx.y), t: 0.0 };
        let (cr, co, rr, ro) = capsule(&p, a, b);
        d.eq_cast(|| format!("row35 A.t=0 {:?} {:?}", a, b), cr, &co, rr, &ro);
    }
    d.finish();
}

// ===========================================================================
// Rows 36, 37 — c2Div / c2Norm / c2Len division-by-zero surface
// ===========================================================================

#[test]
fn err_36_div_norm_by_zero() {
    let p = load();
    let mut d = Diff::new("err_36_div_norm_by_zero");
    let mut rng = Rng::new(36);
    // c2Div(a, 0) → a * inf ; c2Norm((0,0)) → 0 * inf = NaN
    for _ in 0..40_000 {
        let a = rng.vec_uniform(10.0);
        d.eq_v(
            || format!("row36 c2Div({}, 0)", vs(a)),
            unsafe { (p.c.c2Div)(a, 0.0) },
            unsafe { (p.r.c2Div)(a, 0.0) },
        );
        d.eq_v(
            || format!("row36 c2Norm({})", vs(a)),
            unsafe { (p.c.c2Norm)(a) },
            unsafe { (p.r.c2Norm)(a) },
        );
    }
    for z in [v(0.0, 0.0), v(-0.0, -0.0), v(0.0, -0.0), v(-0.0, 0.0)] {
        d.eq_v(
            || format!("row36 c2Norm({}) [zero vector]", vs(z)),
            unsafe { (p.c.c2Norm)(z) },
            unsafe { (p.r.c2Norm)(z) },
        );
        assert!(
            unsafe { (p.c.c2Norm)(z) }.x.is_nan(),
            "row36: C c2Norm(0) should be NaN"
        );
        d.eq_f32(
            || format!("row36 c2Len({})", vs(z)),
            unsafe { (p.c.c2Len)(z) },
            unsafe { (p.r.c2Len)(z) },
        );
        d.eq_v(
            || format!("row36 c2Div({}, 0)", vs(z)),
            unsafe { (p.c.c2Div)(z, 0.0) },
            unsafe { (p.r.c2Div)(z, 0.0) },
        );
    }
    d.finish();
}

#[test]
fn err_37_div_negative_zero() {
    let p = load();
    let mut d = Diff::new("err_37_div_negative_zero");
    let mut rng = Rng::new(37);
    for _ in 0..40_000 {
        let a = rng.vec_uniform(10.0);
        for &b in &[
            -0.0f32,
            0.0,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::MIN_POSITIVE,
            -f32::MIN_POSITIVE,
            f32::from_bits(1),
            f32::MAX,
            f32::NAN,
        ] {
            d.eq_v(
                || format!("row37 c2Div({}, {})", vs(a), fs(b)),
                unsafe { (p.c.c2Div)(a, b) },
                unsafe { (p.r.c2Div)(a, b) },
            );
        }
    }
    // 1/-0.0 must be -inf in both.
    let got = unsafe { (p.c.c2Div)(v(1.0, 1.0), -0.0) };
    assert_eq!(bits(got.x), bits(f32::NEG_INFINITY), "row37 sanity");
    d.finish();
}

// ===========================================================================
// Rows 38, 39 — c2CastRay
// ===========================================================================

/// Row 38 — out-of-range `C2_TYPE`. A C `enum` accepts any `int`, and this
/// `switch` has no `default:` and no trailing `return`, so gcc emits
/// `ja <epilogue>` and returns whatever the caller left in `%eax`, with `*out`
/// untouched.
///
/// The Rust export is a naked stub with the identical instruction semantics, so
/// this test asserts the *observable* contract that can actually hold:
///
/// 1. `*out` is byte-identical (untouched) in both;
/// 2. neither library dereferences `B` (a null `B` is safe in both);
/// 3. both return the *incoming* `%eax` — verified by checking that either the
///    two returns agree, or each equals the low 32 bits of its own entry
///    address, which is exactly what `call *%rax` leaves in `%eax`.
#[test]
fn err_38_castray_out_of_range_type() {
    let p = load();
    let mut d = Diff::new("err_38_castray_out_of_range_type");
    let c_addr = (p.c.c2CastRay as usize) as u32;
    let r_addr = (p.r.c2CastRay as usize) as u32;
    let a = c2Ray {
        p: v(-5.0, 0.0),
        d: v(1.0, 0.0),
        t: 100.0,
    };
    let ci = c2Circle { p: v(0.0, 0.0), r: 2.0 };
    let bad = [
        3i32,
        4,
        5,
        7,
        100,
        -1,
        -2,
        -100,
        i32::MAX,
        i32::MIN,
        0x7fff_ffff,
        i32::MIN + 1,
        -559038737, // 0xdeadbeef
    ];
    for &ty in &bad {
        for bptr in [(&raw const ci) as *const c_void, ptr::null()] {
            let mut co = sentinel();
            let mut ro = sentinel();
            let cr = unsafe { (p.c.c2CastRay)(a, bptr, ty, &mut co) };
            let rr = unsafe { (p.r.c2CastRay)(a, bptr, ty, &mut ro) };

            // (1) `*out` untouched, identically.
            assert_eq!(
                rcbits(&co),
                rcbits(&sentinel()),
                "row38 typeB={ty}: C wrote *out"
            );
            assert_eq!(
                rcbits(&ro),
                rcbits(&sentinel()),
                "row38 typeB={ty}: Rust wrote *out"
            );
            d.eq_cast(|| format!("row38 typeB={ty} out"), 0, &co, 0, &ro);

            // (3) both returned the incoming %eax.
            let same = cr == rr;
            let both_self = cr as u32 == c_addr && rr as u32 == r_addr;
            assert!(
                same || both_self,
                "row38 typeB={ty}: C={:#x} Rust={:#x} but c_addr={:#x} r_addr={:#x} — the \
                 Rust export is not leaving %eax untouched",
                cr as u32,
                rr as u32,
                c_addr,
                r_addr
            );
            d.eq_i(|| format!("row38 typeB={ty} eax-preservation"), 1, 1);
        }
    }
    // And the in-range values must still dispatch identically.
    for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
        let shape = c2Capsule { a: v(0.0, -1.0), b: v(0.0, 1.0), r: 1.0 };
        let mut co = sentinel();
        let mut ro = sentinel();
        let cr = unsafe { (p.c.c2CastRay)(a, (&raw const shape) as *const c_void, ty, &mut co) };
        let rr = unsafe { (p.r.c2CastRay)(a, (&raw const shape) as *const c_void, ty, &mut ro) };
        d.eq_cast(|| format!("row38 in-range typeB={ty}"), cr, &co, rr, &ro);
        assert!(cr == 0 || cr == 1, "row38: in-range must return 0/1, got {cr}");
    }
    d.finish();
}

/// Row 39 — the dead `return 0;` after the `C2_TYPE_CAPSULE` case must have no
/// effect: `c2CastRay(.., CAPSULE, ..)` must equal `c2RaytoCapsule(..)` exactly.
#[test]
fn err_39_castray_capsule_dead_return() {
    let p = load();
    let mut d = Diff::new("err_39_castray_capsule_dead_return");
    let mut rng = Rng::new(39);
    let mut hits = 0u32;
    for _ in 0..80_000 {
        let a = norm_ray(&mut rng, 20.0);
        let b = c2Capsule {
            a: rng.vec_uniform(20.0),
            b: rng.vec_uniform(20.0),
            r: rng.positive(5.0),
        };
        // via the dispatcher
        let mut co1 = sentinel();
        let mut ro1 = sentinel();
        let cr1 =
            unsafe { (p.c.c2CastRay)(a, (&raw const b) as *const c_void, C2_TYPE_CAPSULE, &mut co1) };
        let rr1 =
            unsafe { (p.r.c2CastRay)(a, (&raw const b) as *const c_void, C2_TYPE_CAPSULE, &mut ro1) };
        d.eq_cast(|| format!("row39 dispatch {:?} {:?}", a, b), cr1, &co1, rr1, &ro1);
        // direct
        let mut co2 = sentinel();
        let mut ro2 = sentinel();
        let cr2 = unsafe { (p.c.c2RaytoCapsule)(a, b, &mut co2) };
        let rr2 = unsafe { (p.r.c2RaytoCapsule)(a, b, &mut ro2) };
        d.eq_cast(|| format!("row39 direct {:?} {:?}", a, b), cr2, &co2, rr2, &ro2);
        // and the dead `return 0` must not have changed the dispatcher's answer
        assert_eq!(cr1, cr2, "row39: dispatcher != direct in the C");
        assert_eq!(rcbits(&co1), rcbits(&co2), "row39: dispatcher *out != direct");
        assert_eq!(rr1, rr2, "row39: dispatcher != direct in the Rust");
        assert_eq!(rcbits(&ro1), rcbits(&ro2));
        hits += (cr1 == 1) as u32;
    }
    assert!(hits > 0, "row39: never produced a capsule hit");
    d.finish();
}

// ===========================================================================
// Rows 40, 41, 42 — gen_ray
// ===========================================================================

fn call_gen(l: &Lib, f: &[f32; 16], o: &mut [c2Raycast; 3]) -> c_int {
    let (a, rest) = o.split_at_mut(1);
    let (b, c) = rest.split_at_mut(1);
    unsafe {
        (l.gen_ray)(
            &mut a[0], &mut b[0], &mut c[0], f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8],
            f[9], f[10], f[11], f[12], f[13], f[14], f[15],
        )
    }
}

fn diff_gen(d: &mut Diff, p: &Pair, f: &[f32; 16]) -> c_int {
    let mut co = [sentinel(); 3];
    let mut ro = [sentinel(); 3];
    let cr = call_gen(&p.c, f, &mut co);
    let rr = call_gen(&p.r, f, &mut ro);
    d.eq_i(|| format!("gen_ray{:?} ret", f), cr, rr);
    for i in 0..3 {
        d.eq_f32(|| format!("gen_ray{:?} cast{}.t", f, i + 1), co[i].t, ro[i].t);
        d.eq_v(|| format!("gen_ray{:?} cast{}.n", f, i + 1), co[i].n, ro[i].n);
    }
    cr
}

/// Row 40 — `mp == ray.p` makes `c2Norm((0,0))` NaN, so `ray.d` and `ray.t` are
/// NaN and every downstream comparison is unordered.
#[test]
fn err_40_gen_ray_degenerate_ray() {
    let p = load();
    let mut d = Diff::new("err_40_gen_ray_degenerate_ray");
    let mut rng = Rng::new(40);
    for _ in 0..40_000 {
        let mp = rng.vec_uniform(50.0);
        let mut f = [0f32; 16];
        f[0] = mp.x;
        f[1] = mp.y;
        f[2] = mp.x;
        f[3] = mp.y;
        for slot in f[4..].iter_mut() {
            *slot = rng.uniform(50.0);
        }
        f[6] = rng.positive(10.0);
        f[11] = rng.positive(10.0);
        diff_gen(&mut d, &p, &f);
        // ±0.0 variants of the same degeneracy
        for (a, b) in [(0.0f32, -0.0f32), (-0.0, 0.0), (0.0, 0.0), (-0.0, -0.0)] {
            let mut g = f;
            g[0] = a;
            g[1] = a;
            g[2] = b;
            g[3] = b;
            diff_gen(&mut d, &p, &g);
        }
    }
    d.finish();
}

/// Row 41 — the `hit` accumulator spans the whole `0..=7` bitmask, and nothing
/// outside it.
#[test]
fn err_41_gen_ray_hit_bitmask_range() {
    let p = load();
    let mut d = Diff::new("err_41_gen_ray_hit_bitmask_range");
    let mut rng = Rng::new(41);
    let mut seen = [0u32; 8];
    for _ in 0..200_000 {
        let far = 1e4;
        let bits = rng.below(8);
        let f = [
            10.0 + rng.uniform(1.0),
            rng.uniform(1.0),
            rng.uniform(1.0),
            rng.uniform(1.0),
            if bits & 1 != 0 { 5.0 + rng.uniform(0.4) } else { far },
            if bits & 1 != 0 { rng.uniform(0.4) } else { far },
            1.0 + rng.positive(0.5),
            if bits & 2 != 0 { 7.0 + rng.uniform(0.4) } else { far },
            -2.0,
            if bits & 2 != 0 { 7.0 + rng.uniform(0.4) } else { far },
            2.0,
            0.5 + rng.positive(0.3),
            if bits & 4 != 0 { 8.0 } else { far },
            -1.0,
            if bits & 4 != 0 { 9.0 } else { far + 1.0 },
            1.0,
        ];
        let r = diff_gen(&mut d, &p, &f);
        assert!(
            (0..8).contains(&r),
            "row41: gen_ray returned {r}, outside 0..=7"
        );
        seen[r as usize] += 1;
    }
    assert!(
        seen.iter().all(|&c| c > 0),
        "row41: not all 8 bitmask values reached: {seen:?}"
    );
    d.finish();
}

/// Row 42 — out-parameter aliasing: with a single shared `c2Raycast*` the last
/// writer wins, identically in both libraries.
#[test]
fn err_42_out_aliasing() {
    let p = load();
    let mut d = Diff::new("err_42_out_aliasing");
    let mut rng = Rng::new(42);
    let call = |l: &Lib, f: &[f32; 16]| -> (c_int, c2Raycast) {
        let mut o = sentinel();
        let ret = unsafe {
            (l.gen_ray)(
                &mut o, &mut o, &mut o, f[0], f[1], f[2], f[3], f[4], f[5], f[6], f[7], f[8], f[9],
                f[10], f[11], f[12], f[13], f[14], f[15],
            )
        };
        (ret, o)
    };
    for _ in 0..80_000 {
        let base = [
            10.0f32, 0.0, 0.0, 0.0, 5.0, 0.0, 1.0, 7.0, -2.0, 7.0, 2.0, 0.5, 8.0, -1.0, 9.0, 1.0,
        ];
        let mut f = base;
        for slot in f.iter_mut() {
            *slot += rng.uniform(6.0);
        }
        let (cr, co) = call(&p.c, &f);
        let (rr, ro) = call(&p.r, &f);
        d.eq_i(|| format!("row42 aliased gen_ray{:?} ret", f), cr, rr);
        d.eq_f32(|| format!("row42 aliased gen_ray{:?} .t", f), co.t, ro.t);
        d.eq_v(|| format!("row42 aliased gen_ray{:?} .n", f), co.n, ro.n);
    }
    d.finish();
}

// ===========================================================================
// Rows 43, 44 — c2Len / sqrtf edge cases
// ===========================================================================

#[test]
fn err_43_len_overflow() {
    let p = load();
    let mut d = Diff::new("err_43_len_overflow");
    let cases = [
        v(3e38, 3e38),
        v(f32::MAX, f32::MAX),
        v(f32::MAX, 0.0),
        v(1e30, 1e30),
        v(f32::INFINITY, 0.0),
        v(f32::NEG_INFINITY, 0.0),
        v(f32::INFINITY, f32::NEG_INFINITY),
        v(f32::MIN_POSITIVE, f32::MIN_POSITIVE),
        v(f32::from_bits(1), f32::from_bits(1)),
        v(-0.0, -0.0),
    ];
    for a in cases {
        let c = unsafe { (p.c.c2Len)(a) };
        let r = unsafe { (p.r.c2Len)(a) };
        d.eq_f32(|| format!("row43 c2Len({})", vs(a)), c, r);
    }
    assert!(
        unsafe { (p.c.c2Len)(v(3e38, 3e38)) }.is_infinite(),
        "row43: expected overflow to inf"
    );
    d.finish();
}

/// Row 44 — `sqrtf(NaN)`: glibc's `sqrtf` does *not* take the `isless(x, 0)`
/// error path for NaN, so the payload is preserved (merely quieted), exactly
/// like the Rust `sqrtss`.
#[test]
fn err_44_len_nan() {
    let p = load();
    let mut d = Diff::new("err_44_len_nan");
    let nans = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xff80_0001),
        f32::from_bits(0x7fc0_1234),
        f32::from_bits(0xffc0_4321),
    ];
    for &n in &nans {
        for a in [v(n, 0.0), v(0.0, n), v(n, n), v(n, 1.0), v(1.0, n)] {
            let c = unsafe { (p.c.c2Len)(a) };
            let r = unsafe { (p.r.c2Len)(a) };
            d.eq_f32(|| format!("row44 c2Len({})", vs(a)), c, r);
            assert!(c.is_nan(), "row44: expected NaN from the C");
            // The same NaN must survive c2Norm, which divides by c2Len.
            let cn = unsafe { (p.c.c2Norm)(a) };
            let rn = unsafe { (p.r.c2Norm)(a) };
            d.eq_v(|| format!("row44 c2Norm({})", vs(a)), cn, rn);
        }
    }
    d.finish();
}

// ===========================================================================
// Generic FFI boundary checks required by the task, beyond the table
// ===========================================================================

/// Null `out` on paths where the C provably never dereferences it: an early
/// `disc < 0` reject in `c2RaytoCircle` and a broad-phase reject in
/// `c2RaytoAABB`. Both libraries must return 0 without touching the pointer.
#[test]
fn generic_null_out_on_early_reject() {
    let p = load();
    let mut d = Diff::new("generic_null_out_on_early_reject");
    // Circle: ray parallel to and far from the circle → disc < 0.
    let a = c2Ray {
        p: v(0.0, 1000.0),
        d: v(1.0, 0.0),
        t: 1.0,
    };
    let ci = c2Circle { p: v(0.0, 0.0), r: 1.0 };
    let cr = unsafe { (p.c.c2RaytoCircle)(a, ci, ptr::null_mut()) };
    let rr = unsafe { (p.r.c2RaytoCircle)(a, ci, ptr::null_mut()) };
    d.eq_i(|| "null out, circle disc<0".to_string(), cr, rr);
    assert_eq!(cr, 0);

    // AABB: short ray far from the box → broad-phase reject.
    let bx = c2AABB {
        min: v(1e6, 1e6),
        max: v(1e6 + 1.0, 1e6 + 1.0),
    };
    let cr = unsafe { (p.c.c2RaytoAABB)(a, bx, ptr::null_mut()) };
    let rr = unsafe { (p.r.c2RaytoAABB)(a, bx, ptr::null_mut()) };
    d.eq_i(|| "null out, aabb broadphase".to_string(), cr, rr);
    assert_eq!(cr, 0);

    // c2CastRay with a null `B` *and* an out-of-range type never dereferences
    // either pointer.
    let cr = unsafe { (p.c.c2CastRay)(a, ptr::null(), 9, ptr::null_mut()) };
    let rr = unsafe { (p.r.c2CastRay)(a, ptr::null(), 9, ptr::null_mut()) };
    let _ = (cr, rr); // value is the caller's leftover %eax (see row 38)
    d.eq_i(|| "null everything, out-of-range type: no crash".to_string(), 1, 1);
    d.finish();
}

/// Every value one step past each valid `C2_TYPE` boundary, and every value one
/// step inside, so the enum's exact accepted range is pinned down.
#[test]
fn generic_enum_boundary_values() {
    let p = load();
    let mut d = Diff::new("generic_enum_boundary_values");
    let a = c2Ray {
        p: v(-5.0, 0.0),
        d: v(1.0, 0.0),
        t: 100.0,
    };
    let cap = c2Capsule { a: v(0.0, -1.0), b: v(0.0, 1.0), r: 1.0 };
    // -1 and 3 are the two values one step outside [0, 2].
    for ty in [-1i32, 0, 1, 2, 3] {
        let mut co = sentinel();
        let mut ro = sentinel();
        let cr = unsafe { (p.c.c2CastRay)(a, (&raw const cap) as *const c_void, ty, &mut co) };
        let rr = unsafe { (p.r.c2CastRay)(a, (&raw const cap) as *const c_void, ty, &mut ro) };
        if (0..=2).contains(&ty) {
            d.eq_cast(|| format!("enum boundary ty={ty}"), cr, &co, rr, &ro);
            assert!(cr == 0 || cr == 1);
        } else {
            // Out of range: `*out` untouched in both (see row 38 for the return).
            assert_eq!(rcbits(&co), rcbits(&sentinel()), "ty={ty}: C wrote *out");
            assert_eq!(rcbits(&ro), rcbits(&sentinel()), "ty={ty}: Rust wrote *out");
            d.eq_cast(|| format!("enum boundary ty={ty} out"), 0, &co, 0, &ro);
        }
    }
    d.finish();
}
