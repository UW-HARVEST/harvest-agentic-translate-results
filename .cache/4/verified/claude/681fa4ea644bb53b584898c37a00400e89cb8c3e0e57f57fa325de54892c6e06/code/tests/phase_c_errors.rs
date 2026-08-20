//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`.  Each test
//!  1. constructs the exact invalid input / rejection condition,
//!  2. calls BOTH the C `.so` and the Rust `.so` through their exports,
//!  3. asserts they return the *same* rejection value (`0` / `1`) **and** the
//!     same bytes in the `c2Raycast` out-parameter (pre-filled with a sentinel,
//!     so "did not write" is distinguishable from "wrote something"),
//!  4. asserts that the C really produced the documented result, so the row is
//!     provably exercised and not silently mis-constructed.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::{c_int, c_void};

const M: usize = 600;

fn perp(u: C2v) -> C2v {
    v(-u.y, u.x)
}

/// Differential call + "the C must reject/accept exactly like this" assertion.
fn expect_circle(d: &mut Diff, ray: C2Ray, circ: C2Circle, want_rc: c_int) {
    let (c, r) = apis();
    let cc = call_raytocircle(c, ray, circ);
    let cr = call_raytocircle(r, ray, circ);
    assert_eq!(
        cc.0,
        want_rc,
        "row `{}`: C returned {} for {} {}, expected {want_rc}",
        d.row,
        cc.0,
        rayshow(&ray),
        circshow(&circ)
    );
    d.check_call(|| format!("{} {}", rayshow(&ray), circshow(&circ)), cc, cr);
}

fn expect_aabb(d: &mut Diff, ray: C2Ray, b: C2AABB, want_rc: c_int) {
    let (c, r) = apis();
    let cc = call_raytoaabb(c, ray, b);
    let cr = call_raytoaabb(r, ray, b);
    assert_eq!(
        cc.0,
        want_rc,
        "row `{}`: C returned {} for {} {}, expected {want_rc}",
        d.row,
        cc.0,
        rayshow(&ray),
        aabbshow(&b)
    );
    d.check_call(|| format!("{} {}", rayshow(&ray), aabbshow(&b)), cc, cr);
}

fn expect_capsule(d: &mut Diff, ray: C2Ray, cap: C2Capsule, want_rc: c_int) {
    let (c, r) = apis();
    let cc = call_raytocapsule(c, ray, cap);
    let cr = call_raytocapsule(r, ray, cap);
    assert_eq!(
        cc.0,
        want_rc,
        "row `{}`: C returned {} for {} {}, expected {want_rc}",
        d.row,
        cc.0,
        rayshow(&ray),
        capshow(&cap)
    );
    d.check_call(|| format!("{} {}", rayshow(&ray), capshow(&cap)), cc, cr);
}

/* =============== rows 1..6 — c2RaytoCircle rejection branches =============== */

#[test]
fn err01_circle_disc_negative() {
    let mut rng = Rng::new(0xE01);
    let mut d = Diff::new("1: c2RaytoCircle disc < 0");
    for _ in 0..M {
        let center = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let rad = rng.range(0.1, 10.0);
        let u = rng.dir();
        let s = rad * rng.range(1.2, 6.0) * if rng.chance(2) { 1.0 } else { -1.0 };
        let ray = C2Ray {
            p: vadd(vadd(center, vscale(perp(u), s)), vscale(u, -20.0)),
            d: u,
            t: 100.0,
        };
        expect_circle(&mut d, ray, C2Circle { p: center, r: rad }, 0);
    }
    d.finish();
}

#[test]
fn err02_circle_t_negative() {
    let mut rng = Rng::new(0xE02);
    let mut d = Diff::new("2: c2RaytoCircle t < 0 (origin inside / circle behind)");
    for i in 0..M {
        let center = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let rad = rng.range(0.5, 10.0);
        let u = rng.dir();
        let ray = if i % 2 == 0 {
            C2Ray {
                p: vadd(center, vscale(rng.dir(), rng.range(0.0, 0.9) * rad)),
                d: u,
                t: 100.0,
            }
        } else {
            C2Ray {
                p: vadd(center, vscale(u, rad + rng.range(0.1, 30.0))),
                d: u,
                t: 100.0,
            }
        };
        expect_circle(&mut d, ray, C2Circle { p: center, r: rad }, 0);
    }
    d.finish();
}

#[test]
fn err03_circle_t_beyond_ray_length() {
    let (c, _r) = apis();
    let mut rng = Rng::new(0xE03);
    let mut d = Diff::new("3: c2RaytoCircle t > A.t");
    for _ in 0..M {
        let center = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let rad = rng.range(0.1, 10.0);
        let u = rng.dir();
        let dist = rad + rng.range(1.0, 30.0);
        let base = C2Ray {
            p: vadd(center, vscale(u, -dist)),
            d: u,
            t: 1e9,
        };
        let circ = C2Circle { p: center, r: rad };
        let (rc, out) = call_raytocircle(c, base, circ);
        assert_eq!(rc, 1);
        // exactly one ulp short of the hit distance => rejected
        let ray = C2Ray {
            t: next_down(out.t),
            ..base
        };
        expect_circle(&mut d, ray, circ, 0);
        // and exactly at the hit distance => accepted (inclusive bound)
        let ray_ok = C2Ray { t: out.t, ..base };
        expect_circle(&mut d, ray_ok, circ, 1);
    }
    d.finish();
}

#[test]
fn err04_circle_disc_nan() {
    let mut rng = Rng::new(0xE04);
    let mut d = Diff::new("4: c2RaytoCircle disc == NaN (no early-out, t >= 0 false)");
    let nan = f32::NAN;
    for i in 0..M {
        let center = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let rad = rng.range(0.1, 10.0);
        let u = rng.dir();
        let mut ray = C2Ray {
            p: vadd(center, vscale(u, -20.0)),
            d: u,
            t: 100.0,
        };
        let mut circ = C2Circle { p: center, r: rad };
        match i % 5 {
            0 => ray.p.x = nan,
            1 => ray.p.y = nan,
            2 => circ.p.x = nan,
            3 => circ.r = nan,
            _ => ray.d.x = nan,
        }
        expect_circle(&mut d, ray, circ, 0);
    }
    d.finish();
}

#[test]
fn err05_circle_negative_radius_behaves_like_abs() {
    let (c, _r) = apis();
    let mut rng = Rng::new(0xE05);
    let mut d = Diff::new("5: c2RaytoCircle B.r < 0 (no validation)");
    for _ in 0..M {
        let center = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let rad = rng.range(0.1, 10.0);
        let u = rng.dir();
        let ray = C2Ray {
            p: vadd(center, vscale(u, -(rad + rng.range(0.5, 20.0)))),
            d: u,
            t: 100.0,
        };
        let pos = C2Circle { p: center, r: rad };
        let neg = C2Circle { p: center, r: -rad };
        let a = call_raytocircle(c, ray, pos);
        let b = call_raytocircle(c, ray, neg);
        assert_eq!(a.0, b.0, "C: -r must behave like +r");
        assert!(cast_eq_bits(&a.1, &b.1), "C: -r must behave like +r");
        expect_circle(&mut d, ray, neg, a.0);
    }
    d.finish();
}

#[test]
fn err06_circle_negative_ray_length() {
    let mut rng = Rng::new(0xE06);
    let mut d = Diff::new("6: c2RaytoCircle A.t < 0");
    for i in 0..M {
        let center = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let rad = rng.range(0.1, 10.0);
        let u = rng.dir();
        let ray = C2Ray {
            p: vadd(center, vscale(u, -(rad + rng.range(0.5, 20.0)))),
            d: u,
            t: if i % 3 == 0 {
                -0.0
            } else {
                -rng.range(0.0, 100.0)
            },
        };
        // t == 0 is accepted when the hit distance is exactly 0, which cannot
        // happen here (the origin is outside), so every case must be rejected.
        expect_circle(&mut d, ray, C2Circle { p: center, r: rad }, 0);
    }
    d.finish();
}

/* ================ rows 7..11 — c2AABBtoAABB rejection branches ============= */

#[test]
fn err07_10_aabbtoaabb_each_separating_axis() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE07);
    let mut d = Diff::new("7-10: c2AABBtoAABB d0/d1/d2/d3");
    for i in 0..M * 4 {
        let x0 = rng.range(-30.0, 30.0);
        let y0 = rng.range(-30.0, 30.0);
        let a = C2AABB {
            min: v(x0, y0),
            max: v(x0 + rng.range(0.1, 20.0), y0 + rng.range(0.1, 20.0)),
        };
        let gap = rng.range(0.001, 20.0);
        let w = rng.range(0.1, 20.0);
        let h = rng.range(0.1, 20.0);
        let b = match i % 4 {
            // d0: B.max.x < A.min.x
            0 => C2AABB {
                min: v(a.min.x - gap - w, a.min.y),
                max: v(a.min.x - gap, a.max.y),
            },
            // d1: A.max.x < B.min.x
            1 => C2AABB {
                min: v(a.max.x + gap, a.min.y),
                max: v(a.max.x + gap + w, a.max.y),
            },
            // d2: B.max.y < A.min.y
            2 => C2AABB {
                min: v(a.min.x, a.min.y - gap - h),
                max: v(a.max.x, a.min.y - gap),
            },
            // d3: A.max.y < B.min.y
            _ => C2AABB {
                min: v(a.min.x, a.max.y + gap),
                max: v(a.max.x, a.max.y + gap + h),
            },
        };
        let rc = unsafe { (c.c2AABBtoAABB)(a, b) };
        let rr = unsafe { (r.c2AABBtoAABB)(a, b) };
        assert_eq!(
            rc,
            0,
            "axis {}: C should report no overlap for {} {}",
            i % 4,
            aabbshow(&a),
            aabbshow(&b)
        );
        d.check(rc == rr, || {
            format!(
                "c2AABBtoAABB({}, {}): C {rc} vs RUST {rr}",
                aabbshow(&a),
                aabbshow(&b)
            )
        });
    }
    d.finish();
}

#[test]
fn err11_aabbtoaabb_nan_reports_overlap() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE11);
    let mut d = Diff::new("11: c2AABBtoAABB with NaN => returns 1");
    let nans = [
        f32::NAN,
        f32::from_bits(0xFFC0_0000),
        f32::from_bits(0x7FA0_0000),
    ];
    for i in 0..M {
        let x0 = rng.range(-30.0, 30.0);
        let a = C2AABB {
            min: v(x0, x0),
            max: v(x0 + 5.0, x0 + 5.0),
        };
        let nan = nans[i % nans.len()];
        // separated on every axis *except* that one coordinate is NaN
        let mut b = C2AABB {
            min: v(a.max.x + 10.0, a.max.y + 10.0),
            max: v(a.max.x + 20.0, a.max.y + 20.0),
        };
        match i % 4 {
            0 => b.min.x = nan,
            1 => b.min.y = nan,
            2 => b.max.x = nan,
            _ => {
                b.min = v(nan, nan);
                b.max = v(nan, nan);
            }
        }
        let rc = unsafe { (c.c2AABBtoAABB)(a, b) };
        let rr = unsafe { (r.c2AABBtoAABB)(a, b) };
        if i % 4 == 3 {
            assert_eq!(rc, 1, "C must report overlap for an all-NaN box");
        }
        d.check(rc == rr, || {
            format!(
                "c2AABBtoAABB({}, {}): C {rc} vs RUST {rr}",
                aabbshow(&a),
                aabbshow(&b)
            )
        });
    }
    d.finish();
}

/* ================ rows 12..18 — c2RaytoAABB rejection branches ============= */

#[test]
fn err12_raytoaabb_bbox_reject() {
    let mut rng = Rng::new(0xE12);
    let mut d = Diff::new("12: c2RaytoAABB swept-bbox reject");
    for _ in 0..M {
        let x0 = rng.range(-30.0, 30.0);
        let y0 = rng.range(-30.0, 30.0);
        let b = C2AABB {
            min: v(x0, y0),
            max: v(x0 + rng.range(0.1, 20.0), y0 + rng.range(0.1, 20.0)),
        };
        let away = match rng.below(4) {
            0 => v(-1.0, 0.0),
            1 => v(1.0, 0.0),
            2 => v(0.0, -1.0),
            _ => v(0.0, 1.0),
        };
        let ray = C2Ray {
            p: vadd(v(x0, y0), vscale(away, 200.0)),
            d: away,
            t: rng.range(0.0, 50.0),
        };
        expect_aabb(&mut d, ray, b, 0);
    }
    d.finish();
}

#[test]
fn err13_raytoaabb_separating_axis_reject() {
    let (c, _r) = apis();
    let mut rng = Rng::new(0xE13);
    let mut d = Diff::new("13: c2RaytoAABB separating axis d > 0");
    let mut n = 0;
    for _ in 0..M * 4 {
        let x0 = rng.range(-30.0, 30.0);
        let y0 = rng.range(-30.0, 30.0);
        let b = C2AABB {
            min: v(x0, y0),
            max: v(x0 + rng.range(1.0, 20.0), y0 + rng.range(1.0, 20.0)),
        };
        let (corner, outward) = match rng.below(4) {
            0 => (b.min, vnorm(v(-1.0, -1.0))),
            1 => (b.max, vnorm(v(1.0, 1.0))),
            2 => (v(b.min.x, b.max.y), vnorm(v(-1.0, 1.0))),
            _ => (v(b.max.x, b.min.y), vnorm(v(1.0, -1.0))),
        };
        let q = vadd(corner, vscale(outward, rng.range(0.05, 2.0)));
        let dir = perp(outward);
        let ray = C2Ray {
            p: vadd(q, vscale(dir, -40.0)),
            d: dir,
            t: 80.0,
        };
        let (rc, out) = call_raytoaabb(c, ray, b);
        if classify_aabb(c, ray, b, rc, &out) != AabbBranch::SepAxisReject {
            continue;
        }
        n += 1;
        expect_aabb(&mut d, ray, b, 0);
    }
    assert!(n > 100, "only {n} separating-axis rejections generated");
    d.finish();
}

#[test]
fn err14_raytoaabb_no_plane_hit() {
    let (c, _r) = apis();
    let mut rng = Rng::new(0xE14);
    let mut d = Diff::new("14: c2RaytoAABB hit == 0 (all t0..t3 > 1)");
    for _ in 0..M {
        // an all-NaN box passes the bbox test (row 11) and the `d > 0` test
        // (d is NaN), and then every plane ratio is NaN => hit == 0.
        let b = C2AABB {
            min: v(f32::NAN, f32::NAN),
            max: v(f32::NAN, f32::NAN),
        };
        let u = rng.dir();
        let ray = C2Ray {
            p: v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0)),
            d: u,
            t: rng.range(0.0, 50.0),
        };
        let (rc, out) = call_raytoaabb(c, ray, b);
        assert_eq!(
            classify_aabb(c, ray, b, rc, &out),
            AabbBranch::NoPlaneHit,
            "expected the `hit == 0` branch"
        );
        expect_aabb(&mut d, ray, b, 0);
    }
    d.finish();
}

/// `c2SignedDistPointToPlane_OneDimensional` — same formula as the C, used only
/// to classify which branch of `c2RayToPlane_OneDimensional` a case reaches.
fn sd(p: f32, n: f32, dd: f32) -> f32 {
    p * n - dd * n
}

#[test]
fn err15_18_raytoplane_onedimensional_branches() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE15);
    let mut d = Diff::new("15-18: c2RayToPlane_OneDimensional da<0 / da*db>0 / d==0 / overflow");
    let mut seen = [0usize; 4];
    for i in 0..M * 8 {
        let x0 = rng.range(-30.0, 30.0);
        let y0 = rng.range(-30.0, 30.0);
        let b = C2AABB {
            min: v(x0, y0),
            max: v(x0 + rng.range(0.1, 20.0), y0 + rng.range(0.1, 20.0)),
        };
        let mut b = b;
        let ray = match i % 6 {
            // axis-parallel: for the two planes of the other axis da == db
            0 => C2Ray {
                p: v(b.min.x - 5.0, b.min.y + 1.0),
                d: v(1.0, 0.0),
                t: 50.0,
            },
            // zero-length sweep: p1 == p0 => da == db for all four planes
            1 => C2Ray {
                p: v(b.min.x + 1.0, b.min.y + 1.0),
                d: rng.dir(),
                t: 0.0,
            },
            // huge coordinates / huge sweep
            2 => C2Ray {
                p: v(-3.0e38, b.min.y + 1.0),
                d: v(1.0, 0.0),
                t: 3.0e38,
            },
            // origin exactly ON a plane and parallel to it => da == db == 0,
            // i.e. `d == 0` inside c2RayToPlane_OneDimensional
            3 => C2Ray {
                p: v(b.min.x, b.min.y - 3.0),
                d: v(0.0, 1.0),
                t: 30.0,
            },
            4 => C2Ray {
                p: v(b.max.x, b.min.y - 3.0),
                d: v(0.0, 1.0),
                t: 30.0,
            },
            // NaN plane => `da / d` is NaN, so `t <= 1.0` is false
            _ => {
                b.min.x = f32::NAN;
                C2Ray {
                    p: v(b.min.y - 5.0, b.min.y + rng.range(0.0, 1.0)),
                    d: vnorm(v(1.0, rng.range(-0.5, 0.5))),
                    t: 60.0,
                }
            }
        };
        let b = b;
        // classify the four plane tests with the C's own formula
        let p0 = ray.p;
        let p1 = unsafe { (c.c2Add)(ray.p, (c.c2Mulvs)(ray.d, ray.t)) };
        for (da, db) in [
            (sd(p0.x, -1.0, b.min.x), sd(p1.x, -1.0, b.min.x)),
            (sd(p0.x, 1.0, b.max.x), sd(p1.x, 1.0, b.max.x)),
            (sd(p0.y, -1.0, b.min.y), sd(p1.y, -1.0, b.min.y)),
            (sd(p0.y, 1.0, b.max.y), sd(p1.y, 1.0, b.max.y)),
        ] {
            if da < 0.0 {
                seen[0] += 1;
            } else if da * db > 0.0 {
                seen[1] += 1;
            } else if da - db == 0.0 {
                seen[2] += 1;
            } else if !(da / (da - db) <= 1.0) {
                seen[3] += 1;
            }
        }
        let cc = call_raytoaabb(c, ray, b);
        let cr = call_raytoaabb(r, ray, b);
        d.check_call(|| format!("{} {}", rayshow(&ray), aabbshow(&b)), cc, cr);
    }
    for (k, name) in ["da<0", "da*db>0", "d==0", "ratio>1/NaN"].iter().enumerate() {
        assert!(seen[k] > 0, "branch `{name}` was never exercised: {seen:?}");
    }
    // Note: for finite inputs `da >= 0` and `da*db <= 0` imply `da - db >= da`,
    // so `da/d <= 1` always holds; the "ratio > 1" case is reachable only
    // through NaN (row 18).
    println!("plane-branch coverage: {seen:?}");
    d.finish();
}

/* ============== rows 19..24 — c2AABBtoPoint rejection branches ============= */

#[test]
fn err19_22_aabbtopoint_each_rejection() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE19);
    let mut d = Diff::new("19-22: c2AABBtoPoint d0/d1/d2/d3");
    for i in 0..M * 4 {
        let x0 = rng.range(-30.0, 30.0);
        let y0 = rng.range(-30.0, 30.0);
        let a = C2AABB {
            min: v(x0, y0),
            max: v(x0 + rng.range(0.1, 20.0), y0 + rng.range(0.1, 20.0)),
        };
        let gap = rng.range(0.001, 20.0);
        let p = match i % 4 {
            0 => v(a.min.x - gap, a.min.y + 1.0),
            1 => v(a.min.x + 1.0, a.min.y - gap),
            2 => v(a.max.x + gap, a.min.y + 1.0),
            _ => v(a.min.x + 1.0, a.max.y + gap),
        };
        let rc = unsafe { (c.c2AABBtoPoint)(a, p) };
        let rr = unsafe { (r.c2AABBtoPoint)(a, p) };
        assert_eq!(rc, 0, "C should reject {} in {}", vshow(p), aabbshow(&a));
        d.check(rc == rr, || {
            format!(
                "c2AABBtoPoint({}, {}): C {rc} vs RUST {rr}",
                aabbshow(&a),
                vshow(p)
            )
        });
    }
    d.finish();
}

#[test]
fn err23_aabbtopoint_nan_reports_inside() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE23);
    let mut d = Diff::new("23: c2AABBtoPoint with NaN => returns 1");
    for i in 0..M {
        let x0 = rng.range(-30.0, 30.0);
        let a = C2AABB {
            min: v(x0, x0),
            max: v(x0 + 5.0, x0 + 5.0),
        };
        let p = match i % 3 {
            0 => v(f32::NAN, f32::NAN),
            1 => v(f32::NAN, x0 + 1.0),
            _ => v(x0 + 1.0, f32::from_bits(0xFFC0_0000)),
        };
        let rc = unsafe { (c.c2AABBtoPoint)(a, p) };
        let rr = unsafe { (r.c2AABBtoPoint)(a, p) };
        assert_eq!(rc, 1, "C must report `inside` for a NaN point");
        d.check(rc == rr, || {
            format!(
                "c2AABBtoPoint({}, {}): C {rc} vs RUST {rr}",
                aabbshow(&a),
                vshow(p)
            )
        });
    }
    d.finish();
}

#[test]
fn err24_aabbtopoint_inverted_box() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE24);
    let mut d = Diff::new("24: c2AABBtoPoint inverted box (min > max)");
    let mut inside = 0;
    for _ in 0..M * 2 {
        let x0 = rng.range(-30.0, 30.0);
        let y0 = rng.range(-30.0, 30.0);
        let a = C2AABB {
            min: v(x0, y0),
            max: v(x0 - rng.range(0.1, 20.0), y0 - rng.range(0.1, 20.0)),
        };
        let p = v(
            rng.range(x0 - 25.0, x0 + 5.0),
            rng.range(y0 - 25.0, y0 + 5.0),
        );
        let rc = unsafe { (c.c2AABBtoPoint)(a, p) };
        let rr = unsafe { (r.c2AABBtoPoint)(a, p) };
        if rc != 0 {
            inside += 1;
        }
        d.check(rc == rr, || {
            format!(
                "c2AABBtoPoint({}, {}): C {rc} vs RUST {rr}",
                aabbshow(&a),
                vshow(p)
            )
        });
    }
    println!("inverted-box `inside` results: {inside}");
    d.finish();
}

/* ============= rows 25..27 — c2CircleToPoint rejection branches ============ */

#[test]
fn err25_circletopoint_on_rim_is_a_miss() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE25);
    let mut d = Diff::new("25: c2CircleToPoint d2 >= r*r (strict <)");
    for i in 0..M {
        // dyadic coordinates so that `center +- rad` is exact in f32 and the
        // point really lands on the rim (d2 == r*r bit-exactly)
        let center = v(
            (rng.below(64) as f32) - 32.0,
            (rng.below(64) as f32) - 32.0,
        );
        let rad = (rng.below(160) as f32 + 1.0) * 0.0625;
        let circ = C2Circle { p: center, r: rad };
        let p = match i % 4 {
            0 => v(center.x + rad, center.y),
            1 => v(center.x - rad, center.y),
            2 => v(center.x, center.y + rad),
            _ => v(center.x, center.y - rad),
        };
        let rc = unsafe { (c.c2CircleToPoint)(circ, p) };
        let rr = unsafe { (r.c2CircleToPoint)(circ, p) };
        assert_eq!(rc, 0, "on-rim must be a miss for {} {}", circshow(&circ), vshow(p));
        d.check(rc == rr, || {
            format!(
                "c2CircleToPoint({}, {}): C {rc} vs RUST {rr}",
                circshow(&circ),
                vshow(p)
            )
        });
    }
    d.finish();
}

#[test]
fn err26_circletopoint_zero_radius_never_hits() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE26);
    let mut d = Diff::new("26: c2CircleToPoint r == 0 => always 0");
    for i in 0..M {
        let center = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let circ = C2Circle {
            p: center,
            r: if i % 2 == 0 { 0.0 } else { -0.0 },
        };
        let p = if i % 3 == 0 {
            center
        } else {
            vadd(center, vscale(rng.dir(), rng.range(0.0, 5.0)))
        };
        let rc = unsafe { (c.c2CircleToPoint)(circ, p) };
        let rr = unsafe { (r.c2CircleToPoint)(circ, p) };
        assert_eq!(rc, 0, "r == 0 must never contain a point");
        d.check(rc == rr, || {
            format!(
                "c2CircleToPoint({}, {}): C {rc} vs RUST {rr}",
                circshow(&circ),
                vshow(p)
            )
        });
    }
    d.finish();
}

#[test]
fn err27_circletopoint_nan_is_a_miss() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE27);
    let mut d = Diff::new("27: c2CircleToPoint NaN => 0");
    for i in 0..M {
        let center = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let mut circ = C2Circle {
            p: center,
            r: rng.range(0.1, 10.0),
        };
        let mut p = center;
        match i % 4 {
            0 => p.x = f32::NAN,
            1 => p.y = f32::from_bits(0x7FA0_0000),
            2 => circ.p.x = f32::NAN,
            _ => circ.r = f32::NAN,
        }
        let rc = unsafe { (c.c2CircleToPoint)(circ, p) };
        let rr = unsafe { (r.c2CircleToPoint)(circ, p) };
        assert_eq!(rc, 0, "NaN must be a miss");
        d.check(rc == rr, || {
            format!(
                "c2CircleToPoint({}, {}): C {rc} vs RUST {rr}",
                circshow(&circ),
                vshow(p)
            )
        });
    }
    d.finish();
}

/* ============= rows 28..33 — c2RaytoCapsule rejection branches ============= */

#[test]
fn err28_capsule_outside_slab() {
    let (c, _r) = apis();
    let mut rng = Rng::new(0xE28);
    let mut d = Diff::new("28: c2RaytoCapsule ray outside the slab => 0 (out already written)");
    for _ in 0..M {
        let a = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let dir = rng.dir();
        let cap = C2Capsule {
            a,
            b: vadd(a, vscale(dir, rng.range(1.0, 30.0))),
            r: rng.range(0.1, 8.0),
        };
        let len = cap_len(&cap);
        let lx = (cap.r + rng.range(0.1, 5.0)) * if rng.chance(2) { 1.0 } else { -1.0 };
        let ray = C2Ray {
            p: cap_local_point(&cap, lx, rng.range(0.1, 0.9) * len),
            d: cap_local_dir(&cap, 0.0, if rng.chance(2) { 1.0 } else { -1.0 }),
            t: rng.range(0.0, 30.0),
        };
        if classify_capsule(c, ray, cap) != CapBranch::Outside {
            continue;
        }
        expect_capsule(&mut d, ray, cap, 0);
        // the pre-write at lib.c:243-244 must have happened even though the
        // function returns 0
        let (_, out) = call_raytocapsule(c, ray, cap);
        assert_eq!(out.t.to_bits(), 0, "out->t must be +0.0, not the sentinel");
        let expect_n = unsafe { (c.c2Norm)((c.c2Sub)(cap.b, cap.a)) };
        assert!(
            v_eq_bits(out.n, expect_n),
            "out->n must be c2Norm(b - a): {} vs {}",
            vshow(out.n),
            vshow(expect_n)
        );
    }
    assert!(d.checked > M / 4, "too few Outside cases: {}", d.checked);
    d.finish();
}

#[test]
fn err29_capsule_degenerate_a_equals_b() {
    let (c, _r) = apis();
    let mut rng = Rng::new(0xE29);
    let mut d = Diff::new("29: c2RaytoCapsule a == b (c2Norm of the zero vector)");
    for i in 0..M {
        let a = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let cap = C2Capsule {
            a,
            b: a,
            r: if i % 4 == 0 { 0.0 } else { rng.range(0.1, 8.0) },
        };
        let u = rng.dir();
        let ray = C2Ray {
            p: vadd(a, vscale(u, -rng.range(0.0, 40.0))),
            d: u,
            t: rng.range(0.0, 80.0),
        };
        let (rc, out) = call_raytocapsule(c, ray, cap);
        // NaNs make c2AABBtoPoint report `inside`, so the C returns 1 with a
        // NaN normal and t == 0
        assert_eq!(rc, 1, "degenerate capsule must return 1");
        assert!(out.n.x.is_nan() && out.n.y.is_nan(), "normal must be NaN");
        assert_eq!(out.t.to_bits(), 0);
        expect_capsule(&mut d, ray, cap, 1);
    }
    d.finish();
}

#[test]
fn err30_capsule_inverted_slab_box() {
    let (c, _r) = apis();
    let mut rng = Rng::new(0xE30);
    let mut d = Diff::new("30: c2RaytoCapsule yBb.y < 0 is impossible => inverted bb via r < 0");
    let mut neg_y = 0;
    for _ in 0..M * 2 {
        let a = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let dir = rng.dir();
        let cap = C2Capsule {
            a,
            b: vadd(a, vscale(dir, rng.range(1.0, 30.0))),
            // negative radius makes capsule_bb.min.x = -r > 0 = ... inverted on x
            r: -rng.range(0.1, 8.0),
        };
        let len = cap_len(&cap);
        let ray = C2Ray {
            p: cap_local_point(&cap, rng.range(-5.0, 5.0), rng.range(-0.5, 1.5) * len),
            d: rng.dir(),
            t: rng.range(0.0, 40.0),
        };
        let ybb = unsafe {
            let my = (c.c2Norm)((c.c2Sub)(cap.b, cap.a));
            let mx = (c.c2CCW90)(my);
            (c.c2MulmvT)(C2m { x: mx, y: my }, (c.c2Sub)(cap.b, cap.a))
        };
        if ybb.y < 0.0 {
            neg_y += 1;
        }
        let (rc, _) = call_raytocapsule(c, ray, cap);
        expect_capsule(&mut d, ray, cap, rc);
    }
    println!("cases with yBb.y < 0: {neg_y}");
    d.finish();
}

#[test]
fn err31_capsule_infinite_denominator() {
    let (c, _r) = apis();
    let mut rng = Rng::new(0xE31);
    let mut d = Diff::new("31: c2RaytoCapsule d = yAe.x - yAp.x = +-inf / NaN");
    let mut n = 0;
    for _ in 0..M * 2 {
        let a = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let dir = rng.dir();
        let cap = C2Capsule {
            a,
            b: vadd(a, vscale(dir, rng.range(1.0, 30.0))),
            r: rng.range(0.1, 8.0),
        };
        let len = cap_len(&cap);
        let lx = cap.r + rng.range(0.1, 4.0);
        let ray = C2Ray {
            p: cap_local_point(&cap, lx, rng.range(0.1, 0.9) * len),
            d: cap_local_dir(&cap, -1.0, rng.range(-0.5, 0.5)),
            t: f32::INFINITY,
        };
        let br = classify_capsule(c, ray, cap);
        if br != CapBranch::SidePos && br != CapBranch::SideNeg {
            continue;
        }
        n += 1;
        let (rc, out) = call_raytocapsule(c, ray, cap);
        assert_eq!(rc, 1);
        assert!(out.t.is_nan(), "expected a NaN out->t, got {}", fshow(out.t));
        expect_capsule(&mut d, ray, cap, 1);
    }
    assert!(n > 50, "only {n} infinite-denominator cases");
    d.finish();
}

#[test]
fn err32_capsule_near_axis_delegation_misses() {
    let (c, _r) = apis();
    let mut rng = Rng::new(0xE32);
    let mut d = Diff::new("32: c2RaytoCapsule |yAp.x| < r delegation that misses");
    let mut n = 0;
    for _ in 0..M * 4 {
        let a = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let dir = rng.dir();
        let cap = C2Capsule {
            a,
            b: vadd(a, vscale(dir, rng.range(1.0, 30.0))),
            r: rng.range(0.1, 8.0),
        };
        let len = cap_len(&cap);
        let below = rng.chance(2);
        let ly = if below {
            -(cap.r + rng.range(1.0, 20.0))
        } else {
            len + cap.r + rng.range(1.0, 20.0)
        };
        // aim AWAY from the capsule so the delegated circle cast rejects
        let ray = C2Ray {
            p: cap_local_point(&cap, rng.range(-0.8, 0.8) * cap.r, ly),
            d: cap_local_dir(&cap, rng.range(-0.2, 0.2), if below { -1.0 } else { 1.0 }),
            t: rng.range(0.0, 20.0),
        };
        let br = classify_capsule(c, ray, cap);
        if br != CapBranch::NearAxisCa && br != CapBranch::NearAxisCb {
            continue;
        }
        let (rc, out) = call_raytocapsule(c, ray, cap);
        if rc != 0 {
            continue;
        }
        n += 1;
        // *out still holds the pre-written values
        assert_eq!(out.t.to_bits(), 0);
        expect_capsule(&mut d, ray, cap, 0);
    }
    assert!(n > 50, "only {n} delegated misses");
    d.finish();
}

#[test]
fn err33_capsule_cross_delegation_misses() {
    let (c, _r) = apis();
    let mut rng = Rng::new(0xE33);
    let mut d = Diff::new("33: c2RaytoCapsule y<=0 / y>=yBb.y delegation that misses");
    let mut n = 0;
    for _ in 0..M * 8 {
        let a = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let dir = rng.dir();
        let cap = C2Capsule {
            a,
            b: vadd(a, vscale(dir, rng.range(1.0, 30.0))),
            r: rng.range(0.1, 8.0),
        };
        let len = cap_len(&cap);
        let lx = cap.r + rng.range(0.1, 3.0);
        let ray = C2Ray {
            p: cap_local_point(&cap, lx, rng.range(-0.2, 1.2) * len),
            d: cap_local_dir(&cap, -1.0, rng.range(-6.0, 6.0)),
            t: rng.range(0.5, 2.0) * lx,
        };
        let br = classify_capsule(c, ray, cap);
        if br != CapBranch::CrossCa && br != CapBranch::CrossCb {
            continue;
        }
        let (rc, out) = call_raytocapsule(c, ray, cap);
        if rc != 0 {
            continue;
        }
        n += 1;
        assert_eq!(out.t.to_bits(), 0, "out->t must still be the pre-written +0.0");
        expect_capsule(&mut d, ray, cap, 0);
    }
    assert!(n > 20, "only {n} crossing-delegation misses");
    d.finish();
}

/* =============== rows 34..35 — c2CastRay dispatch edge cases ============== */

#[test]
fn err34_castray_out_of_range_enum_values() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE34);
    let mut d = Diff::new("34: c2CastRay typeB outside {0,1,2} (C falls off the switch)");
    let types: [c_int; 10] = [
        3,
        4,
        5,
        255,
        256,
        -1,
        -2,
        -1000,
        c_int::MAX,
        c_int::MIN,
    ];
    for &ty in &types {
        for _ in 0..M / 10 + 1 {
            let circ = C2Circle {
                p: rng.v_ordinary(),
                r: rng.range(0.1, 10.0),
            };
            let ray = C2Ray {
                p: rng.v_ordinary(),
                d: rng.dir(),
                t: rng.range(0.0, 50.0),
            };
            let mut oc = sentinel();
            let mut or_ = sentinel();
            let rc = unsafe {
                (c.c2CastRay)(
                    ray,
                    &circ as *const C2Circle as *const c_void,
                    ty,
                    &mut oc,
                )
            };
            let rr = unsafe {
                (r.c2CastRay)(
                    ray,
                    &circ as *const C2Circle as *const c_void,
                    ty,
                    &mut or_,
                )
            };
            // Well-defined part: neither implementation may touch *out, and
            // neither may crash.  The *return value* is undefined in C (the
            // function falls off the end of a non-void function, so gcc returns
            // whatever happened to be in EAX at the call site); the Rust
            // translation returns the source's dead `return 0;`.
            d.check(
                oc.t.to_bits() == SENT_T
                    && oc.n.x.to_bits() == SENT_NX
                    && oc.n.y.to_bits() == SENT_NY,
                || format!("C wrote to *out for typeB = {ty}: {}", castshow(&oc)),
            );
            d.check(cast_eq_bits(&oc, &or_), || {
                format!(
                    "typeB = {ty}: C out {} vs RUST out {}",
                    castshow(&oc),
                    castshow(&or_)
                )
            });
            d.check(rr == 0, || {
                format!("RUST returned {rr} for typeB = {ty}, expected the dead `return 0`")
            });
            // record what the C did, for the record in ERRORS.md
            if rc != 0 {
                d.tag("c_returned_garbage");
            } else {
                d.tag("c_returned_zero");
            }
        }
    }
    println!("c2CastRay invalid-enum tags: {:?}", d.tags);
    d.finish();
}

#[test]
fn err35_castray_shape_reinterpretation() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE35);
    let mut d = Diff::new("35: c2CastRay B reinterpreted as a larger shape");
    // A 32-byte buffer so that reading 20 bytes (c2Capsule) from it is always
    // in-bounds: both libraries must interpret the same bytes identically.
    for _ in 0..M {
        let buf: [f32; 8] = [
            rng.range(-20.0, 20.0),
            rng.range(-20.0, 20.0),
            rng.range(0.1, 10.0),
            rng.range(-20.0, 20.0),
            rng.range(-20.0, 20.0),
            rng.range(0.1, 10.0),
            rng.range(-20.0, 20.0),
            rng.range(-20.0, 20.0),
        ];
        let ray = C2Ray {
            p: rng.v_ordinary(),
            d: rng.dir(),
            t: rng.range(0.0, 50.0),
        };
        for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            let mut oc = sentinel();
            let mut or_ = sentinel();
            let p = buf.as_ptr() as *const c_void;
            let rc = unsafe { (c.c2CastRay)(ray, p, ty, &mut oc) };
            let rr = unsafe { (r.c2CastRay)(ray, p, ty, &mut or_) };
            d.check_call(
                || format!("typeB = {ty}, buf = {buf:?}, {}", rayshow(&ray)),
                (rc, oc),
                (rr, or_),
            );
        }
    }
    d.finish();
}

/* ============== rows 38..44 — degenerate math and spec_ray ================= */

#[test]
fn err38_div_and_norm_by_zero() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE38);
    let mut d = Diff::new("38: c2Div / c2Norm with a zero denominator");
    for i in 0..M * 2 {
        let a = match i % 4 {
            0 => rng.v_ordinary(),
            1 => v(0.0, 0.0),
            2 => v(-0.0, 0.0),
            _ => v(rng.range(-1e38, 1e38), rng.range(-1e38, 1e38)),
        };
        let b = match i % 5 {
            0 => 0.0,
            1 => -0.0,
            2 => f32::INFINITY,
            3 => f32::from_bits(1),
            _ => f32::NEG_INFINITY,
        };
        let dc = unsafe { (c.c2Div)(a, b) };
        let dr = unsafe { (r.c2Div)(a, b) };
        d.check(v_eq_bits(dc, dr), || {
            format!(
                "c2Div({}, {}): C {} vs RUST {}",
                vshow(a),
                fshow(b),
                vshow(dc),
                vshow(dr)
            )
        });
        let nc = unsafe { (c.c2Norm)(a) };
        let nr = unsafe { (r.c2Norm)(a) };
        d.check(v_eq_bits(nc, nr), || {
            format!("c2Norm({}): C {} vs RUST {}", vshow(a), vshow(nc), vshow(nr))
        });
    }
    // the C really does produce NaN for the zero vector
    let z = unsafe { (c.c2Norm)(v(0.0, 0.0)) };
    assert!(z.x.is_nan() && z.y.is_nan(), "c2Norm(0,0) must be NaN");
    d.finish();
}

#[test]
fn err39_41_len_overflow_and_nan() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE39);
    let mut d = Diff::new("39-41: c2Len overflow / NaN / sqrtf domain");
    let mut min_dot = f32::INFINITY;
    for i in 0..M * 4 {
        let a = match i % 4 {
            0 => v(rng.range(1e30, 3.4e38), rng.range(1e30, 3.4e38)),
            1 => v(f32::INFINITY, rng.ordinary()),
            2 => rng.v_special(),
            _ => rng.v_any_bits(),
        };
        let dot = unsafe { (c.c2Dot)(a, a) };
        if !dot.is_nan() && dot < min_dot {
            min_dot = dot;
        }
        let lc = unsafe { (c.c2Len)(a) };
        let lr = unsafe { (r.c2Len)(a) };
        d.check(f_eq_bits(lc, lr), || {
            format!("c2Len({}): C {} vs RUST {}", vshow(a), fshow(lc), fshow(lr))
        });
    }
    // row 41: sqrtf can never see a negative finite value (dot(a,a) >= 0)
    assert!(
        min_dot >= 0.0,
        "c2Dot(a,a) produced a negative value: {}",
        fshow(min_dot)
    );
    let big = unsafe { (c.c2Len)(v(3.0e38, 3.0e38)) };
    assert!(big.is_infinite(), "c2Len must overflow to +inf, got {}", fshow(big));
    d.finish();
}

#[test]
fn err42_spec_ray_degenerate_direction() {
    let (c, _r) = apis();
    let mut rng = Rng::new(0xE42);
    let mut d = Diff::new("42: spec_ray mp == r_p => NaN direction => 0");
    for _ in 0..M {
        let p = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let cp = v(rng.range(-30.0, 30.0), rng.range(-30.0, 30.0));
        let cr = rng.range(0.1, 10.0);
        let (rc, out) = call_spec_ray(c, p.x, p.y, cp.x, cp.y, cr, p.x, p.y);
        assert_eq!(rc, 0, "NaN direction must miss");
        assert_eq!(out.t.to_bits(), SENT_T, "*cast must be untouched");
        let (c_rc, c_out) = call_spec_ray(c, p.x, p.y, cp.x, cp.y, cr, p.x, p.y);
        let (r_rc, r_out) = call_spec_ray(rust_api(), p.x, p.y, cp.x, cp.y, cr, p.x, p.y);
        d.check_call(
            || format!("spec_ray(mp = r_p = {}, c_p = {}, c_r = {})", vshow(p), vshow(cp), fshow(cr)),
            (c_rc, c_out),
            (r_rc, r_out),
        );
    }
    d.finish();
}

#[test]
fn err44_spec_ray_zero_radius_can_still_hit() {
    let (c, r) = apis();
    let mut rng = Rng::new(0xE44);
    let mut d = Diff::new("44: spec_ray c_r <= 0");
    let mut hits = 0;
    for i in 0..M * 4 {
        let cp = v(
            (rng.below(64) as f32) - 32.0,
            (rng.below(64) as f32) - 32.0,
        );
        let cr = match i % 3 {
            0 => 0.0,
            1 => -0.0,
            _ => -((rng.below(64) as f32 + 1.0) * 0.0625),
        };
        // ray origin and mouse point exactly collinear with the centre
        let u = match rng.below(4) {
            0 => v(1.0, 0.0),
            1 => v(-1.0, 0.0),
            2 => v(0.0, 1.0),
            _ => v(0.0, -1.0),
        };
        let dist = (rng.below(32) as f32) + 1.0;
        let rp = vadd(cp, vscale(u, -dist));
        let mp = vadd(cp, vscale(u, (rng.below(32) as f32) + 1.0));
        let cc = call_spec_ray(c, mp.x, mp.y, cp.x, cp.y, cr, rp.x, rp.y);
        let cr_ = call_spec_ray(r, mp.x, mp.y, cp.x, cp.y, cr, rp.x, rp.y);
        if cc.0 == 1 {
            hits += 1;
        }
        d.check_call(
            || {
                format!(
                    "spec_ray(mp = {}, c_p = {}, c_r = {}, r_p = {})",
                    vshow(mp),
                    vshow(cp),
                    fshow(cr),
                    vshow(rp)
                )
            },
            cc,
            cr_,
        );
    }
    assert!(hits > 0, "expected some hits with c_r <= 0 (disc == 0 case)");
    println!("spec_ray with c_r <= 0: {hits} hits");
    d.finish();
}
