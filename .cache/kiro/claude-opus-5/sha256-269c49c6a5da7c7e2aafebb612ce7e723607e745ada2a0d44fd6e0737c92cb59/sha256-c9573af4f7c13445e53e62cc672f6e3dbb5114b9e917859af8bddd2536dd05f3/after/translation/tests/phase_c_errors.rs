//! Phase C — ERRORS.md rows 1–33 and 38–42: one differential test per distinct
//! rejection in the C source.
//!
//! Every test (a) constructs the exact invalid input/condition, (b) ASSERTS that
//! the C really took that branch (so a test cannot silently stop covering its
//! row), and (c) asserts the C and Rust libraries return the same result — the
//! same `int` and the same bytes in `*out`, not merely "both failed somehow".

#![allow(non_snake_case)]

mod common;
use common::*;

const SEED: u64 = 0xE770_2A11;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn v(x: f32, y: f32) -> c2v {
    c2v { x, y }
}

fn call_circle(ray: c2Ray, c: c2Circle) -> (i32, c2Raycast, i32, c2Raycast) {
    both_ray(|l, r, s, o| unsafe { (l.c2RaytoCircle)(r, s, o) }, ray, c)
}
fn call_aabb(ray: c2Ray, b: c2AABB) -> (i32, c2Raycast, i32, c2Raycast) {
    both_ray(|l, r, s, o| unsafe { (l.c2RaytoAABB)(r, s, o) }, ray, b)
}
fn call_capsule(ray: c2Ray, c: c2Capsule) -> (i32, c2Raycast, i32, c2Raycast) {
    both_ray(|l, r, s, o| unsafe { (l.c2RaytoCapsule)(r, s, o) }, ray, c)
}

/// `disc`, `t` and `c` as the C computes them, using the C library's primitives.
fn circle_internals(ray: c2Ray, c: c2Circle) -> (f32, f32) {
    let l = libs();
    unsafe {
        let m = (l.c.c2Sub)(ray.p, c.p);
        let cc = (l.c.c2Dot)(m, m) - c.r * c.r;
        let b = (l.c.c2Dot)(m, ray.d);
        let disc = b * b - cc;
        (disc, -b - disc.sqrt())
    }
}

/// `d` (the separating-axis value) as `c2RaytoAABB` computes it.
fn aabb_sat_d(ray: c2Ray, b: c2AABB) -> f32 {
    let l = libs();
    unsafe {
        let p0 = ray.p;
        let p1 = (l.c.c2Add)(ray.p, (l.c.c2Mulvs)(ray.d, ray.t));
        let ab = (l.c.c2Sub)(p1, p0);
        let n = (l.c.c2Skew)(ab);
        let abs_n = (l.c.c2Absv)(n);
        let half = (l.c.c2Mulvs)((l.c.c2Sub)(b.max, b.min), 0.5);
        let centre = (l.c.c2Mulvs)((l.c.c2Add)(b.min, b.max), 0.5);
        let dot = (l.c.c2Dot)(n, (l.c.c2Sub)(p0, centre));
        let adot = if dot < 0.0 { -dot } else { dot };
        adot - (l.c.c2Dot)(abs_n, half)
    }
}

/// `c2RaytoAABB`'s ray bounding box, as the C builds it.
fn aabb_ray_box(ray: c2Ray) -> c2AABB {
    let l = libs();
    unsafe {
        let p1 = (l.c.c2Add)(ray.p, (l.c.c2Mulvs)(ray.d, ray.t));
        c2AABB {
            min: (l.c.c2Minv)(ray.p, p1),
            max: (l.c.c2Maxv)(ray.p, p1),
        }
    }
}

// ===========================================================================
// c2RaytoCircle — rows 1-5
// ===========================================================================

/// Row 1 — `disc < 0`: the ray LINE misses the circle. Must return 0 and leave
/// `*out` untouched.
#[test]
fn err_01_circle_disc_negative() {
    let mut rng = Rng::new(SEED ^ 1);
    let mut d = Diff::new("row01 circle disc < 0");
    let mut fired = 0usize;
    for _ in 0..4000 {
        let c = c2Circle {
            p: v(rng.coord(), rng.coord()),
            r: rng.range(0.1, 5.0),
        };
        // Direction perpendicular to the offset, origin far to the side: the
        // line passes well outside the circle.
        let off = rng.range(c.r * 3.0, c.r * 40.0) * if rng.bool() { 1.0 } else { -1.0 };
        let ray = c2Ray {
            p: v(c.p.x + off, c.p.y - 100.0),
            d: v(0.0, 1.0),
            t: 1000.0,
        };
        let (disc, _) = circle_internals(ray, c);
        if !(disc < 0.0) {
            continue;
        }
        fired += 1;
        let (cr, co, rr, ro) = call_circle(ray, c);
        assert_eq!(cr, 0, "disc<0 must return 0");
        assert!(rc_eq(co, POISON), "disc<0 must leave *out untouched");
        d.check_ray(cr, co, rr, ro, || format!("disc={} ", fmt_f(disc)));
    }
    assert!(fired > 1000, "row01 trigger fired only {fired} times");
    eprintln!("    row01: {fired} cases with disc < 0");
    d.finish();
}

/// Row 2 — `t < 0`: the ray line hits, but behind the origin.
#[test]
fn err_02_circle_t_negative() {
    let mut rng = Rng::new(SEED ^ 2);
    let mut d = Diff::new("row02 circle t < 0");
    let mut fired = 0usize;
    for _ in 0..4000 {
        let c = c2Circle {
            p: v(rng.coord(), rng.coord()),
            r: rng.range(0.5, 20.0),
        };
        // Origin PAST the circle, aiming further away: both roots are negative.
        let dist = c.r + rng.range(1.0, 50.0);
        let ray = c2Ray {
            p: v(c.p.x + dist, c.p.y),
            d: v(1.0, 0.0),
            t: 1000.0,
        };
        let (disc, t) = circle_internals(ray, c);
        if disc < 0.0 || !(t < 0.0) {
            continue;
        }
        fired += 1;
        let (cr, co, rr, ro) = call_circle(ray, c);
        assert_eq!(cr, 0, "t<0 must return 0");
        assert!(rc_eq(co, POISON), "t<0 must leave *out untouched");
        d.check_ray(cr, co, rr, ro, || format!("t={}", fmt_f(t)));
    }
    // Origin INSIDE the circle: t = -b - sqrt(disc) is negative there too.
    for _ in 0..4000 {
        let c = c2Circle {
            p: v(rng.coord(), rng.coord()),
            r: rng.range(1.0, 20.0),
        };
        let ang = rng.range(0.0, 6.283_185_5);
        let rr_ = rng.range(0.0, c.r * 0.9);
        let ray = c2Ray {
            p: v(c.p.x + rr_ * ang.cos(), c.p.y + rr_ * ang.sin()),
            d: v(ang.cos(), ang.sin()),
            t: 1000.0,
        };
        let (disc, t) = circle_internals(ray, c);
        if disc < 0.0 || !(t < 0.0) {
            continue;
        }
        fired += 1;
        let (cr, co, rr, ro) = call_circle(ray, c);
        assert_eq!(cr, 0);
        d.check_ray(cr, co, rr, ro, || format!("inside, t={}", fmt_f(t)));
    }
    assert!(fired > 1000, "row02 trigger fired only {fired} times");
    eprintln!("    row02: {fired} cases with t < 0");
    d.finish();
}

/// Row 3 — `t > A.t`: hit exists but lies beyond the ray's length. Includes the
/// exact boundary `t == A.t` (must be ACCEPTED) and one ULP past it.
#[test]
fn err_03_circle_t_beyond_len() {
    let mut rng = Rng::new(SEED ^ 3);
    let mut d = Diff::new("row03 circle t > A.t");
    let mut fired = 0usize;
    let mut boundary_accepted = 0usize;
    for _ in 0..4000 {
        let c = c2Circle {
            p: v(rng.coord(), rng.coord()),
            r: rng.range(0.5, 20.0),
        };
        let dist = c.r + rng.range(5.0, 80.0);
        let ray_far = c2Ray {
            p: v(c.p.x - dist, c.p.y),
            d: v(1.0, 0.0),
            t: 1e9,
        };
        let (disc, t) = circle_internals(ray_far, c);
        if disc < 0.0 || t < 0.0 {
            continue;
        }
        // A.t exactly at t -> accepted; one ULP below -> rejected.
        for (at, expect_hit) in [
            (t, true),
            (f32::from_bits(t.to_bits() - 1), false),
            (t * 0.5, false),
            (0.0, false),
        ] {
            let ray = c2Ray { p: ray_far.p, d: ray_far.d, t: at };
            let (cr, co, rr, ro) = call_circle(ray, c);
            if expect_hit {
                if cr == 1 {
                    boundary_accepted += 1;
                }
            } else if cr == 0 {
                fired += 1;
                assert!(rc_eq(co, POISON), "t>A.t must leave *out untouched");
            }
            d.check_ray(cr, co, rr, ro, || {
                format!("t={} A.t={}", fmt_f(t), fmt_f(at))
            });
        }
    }
    assert!(fired > 1000, "row03 trigger fired only {fired} times");
    assert!(
        boundary_accepted > 500,
        "the `t <= A.t` boundary was never accepted ({boundary_accepted}); \
         the test is not probing the boundary"
    );
    eprintln!("    row03: {fired} rejections, {boundary_accepted} boundary acceptances");
    d.finish();
}

/// Row 4 — NaN inputs: `NaN < 0` is false so `disc < 0` does NOT reject, and the
/// rejection instead happens at `t >= 0`. The C must return 0 with `*out`
/// untouched, and the Rust must agree.
#[test]
fn err_04_circle_nan_inputs() {
    let mut d = Diff::new("row04 circle NaN inputs");
    let base_ray = c2Ray {
        p: v(-10.0, 0.0),
        d: v(1.0, 0.0),
        t: 100.0,
    };
    let base_c = c2Circle { p: v(0.0, 0.0), r: 3.0 };
    let mut fired = 0usize;
    for &nb in NAN_BITS {
        let s = f32::from_bits(nb);
        for field in 0..8 {
            let mut ray = base_ray;
            let mut c = base_c;
            match field {
                0 => ray.p.x = s,
                1 => ray.p.y = s,
                2 => ray.d.x = s,
                3 => ray.d.y = s,
                4 => ray.t = s,
                5 => c.p.x = s,
                6 => c.p.y = s,
                _ => c.r = s,
            }
            let (disc, _) = circle_internals(ray, c);
            let (cr, co, rr, ro) = call_circle(ray, c);
            if disc.is_nan() {
                fired += 1;
                assert!(
                    !(disc < 0.0),
                    "NaN disc must not satisfy `disc < 0` -- C semantics"
                );
                assert_eq!(cr, 0, "a NaN disc must end in `return 0`");
                assert!(rc_eq(co, POISON), "NaN path must leave *out untouched");
            }
            d.check_ray(cr, co, rr, ro, || {
                format!("NaN {nb:#x} in field {field}, disc={}", fmt_f(disc))
            });
        }
    }
    assert!(fired > 10, "row04 NaN-disc trigger fired only {fired} times");
    eprintln!("    row04: {fired} cases with a NaN disc");
    d.finish();
}

/// Row 5 — negative radius. `c = dot(m,m) - r*r` squares `r`, so `-r` must behave
/// exactly like `+r` in BOTH libraries.
#[test]
fn err_05_circle_negative_radius() {
    let mut rng = Rng::new(SEED ^ 5);
    let mut d = Diff::new("row05 circle negative radius");
    let l = libs();
    for _ in 0..6000 {
        let p = v(rng.coord(), rng.coord());
        let r = rng.range(0.1, 30.0);
        let origin = v(rng.coord(), rng.coord());
        let target = v(p.x + rng.range(-r * 2.0, r * 2.0), p.y + rng.range(-r * 2.0, r * 2.0));
        let dir = unsafe { (l.c.c2Norm)((l.c.c2Sub)(target, origin)) };
        let ray = c2Ray { p: origin, d: dir, t: rng.range(0.0, 300.0) };

        let pos = c2Circle { p, r };
        let neg = c2Circle { p, r: -r };
        let (cr_p, co_p, rr_p, ro_p) = call_circle(ray, pos);
        let (cr_n, co_n, rr_n, ro_n) = call_circle(ray, neg);
        // C vs Rust for each sign.
        d.check_ray(cr_p, co_p, rr_p, ro_p, || format!("r=+{}", fmt_f(r)));
        d.check_ray(cr_n, co_n, rr_n, ro_n, || format!("r=-{}", fmt_f(r)));
        // And the C's own +r/-r equivalence, which the Rust must mirror.
        d.check(cr_p == cr_n && rc_eq(co_p, co_n), || {
            format!("C: r=+{} and r=-{} disagree", fmt_f(r), fmt_f(r))
        });
        d.check(rr_p == rr_n && rc_eq(ro_p, ro_n), || {
            format!("Rust: r=+{} and r=-{} disagree", fmt_f(r), fmt_f(r))
        });
    }
    d.finish();
}

// ===========================================================================
// c2AABBtoAABB — rows 6-11
// ===========================================================================

/// Rows 6-9 — the four separating conditions `d0`..`d3`, each in isolation.
#[test]
fn err_06_09_aabbaabb_each_separation() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 6);
    let a = c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
    let labels = ["row06 d0 (B.max.x < A.min.x)", "row07 d1 (A.max.x < B.min.x)",
                  "row08 d2 (B.max.y < A.min.y)", "row09 d3 (A.max.y < B.min.y)"];
    for which in 0..4 {
        let mut d = Diff::new(labels[which]);
        let mut fired = 0usize;
        for _ in 0..2000 {
            let gap = rng.range(0.001, 50.0);
            let span = rng.range(0.001, 40.0);
            // Also sweep the y (resp. x) extent so only the intended d_i fires.
            let other_lo = rng.range(-1.0, 1.0);
            let other_hi = other_lo + rng.range(0.0, 2.0);
            let b = match which {
                0 => c2AABB { min: v(-1.0 - gap - span, other_lo), max: v(-1.0 - gap, other_hi) },
                1 => c2AABB { min: v(1.0 + gap, other_lo), max: v(1.0 + gap + span, other_hi) },
                2 => c2AABB { min: v(other_lo, -1.0 - gap - span), max: v(other_hi, -1.0 - gap) },
                _ => c2AABB { min: v(other_lo, 1.0 + gap), max: v(other_hi, 1.0 + gap + span) },
            };
            // Verify exactly the intended condition is the one that fires.
            let d0 = b.max.x < a.min.x;
            let d1 = a.max.x < b.min.x;
            let d2 = b.max.y < a.min.y;
            let d3 = a.max.y < b.min.y;
            let flags = [d0, d1, d2, d3];
            if !flags[which] {
                continue;
            }
            fired += 1;
            let cr = unsafe { (l.c.c2AABBtoAABB)(a, b) };
            let rr = unsafe { (l.r.c2AABBtoAABB)(a, b) };
            assert_eq!(cr, 0, "{}: separation must return 0", labels[which]);
            d.check_i(cr, rr, || format!("{} flags={flags:?}", labels[which]));
        }
        assert!(fired > 500, "{} fired only {fired} times", labels[which]);
        eprintln!("    {}: {fired} cases", labels[which]);
        d.finish();
    }
}

/// Row 10 — inverted boxes. The C never validates `min <= max`; it just evaluates
/// the four `<`. An inverted box typically reports overlap.
#[test]
fn err_10_aabbaabb_inverted() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 10);
    let mut d = Diff::new("row10 c2AABBtoAABB inverted boxes");
    let mut got_true = 0usize;
    for _ in 0..8000 {
        let p = c2AABB {
            min: v(rng.coord(), rng.coord()),
            max: v(rng.coord(), rng.coord()),
        };
        let inv = c2AABB { min: p.max, max: p.min };
        let q = c2AABB {
            min: v(rng.coord(), rng.coord()),
            max: v(rng.coord(), rng.coord()),
        };
        for (x, y) in [(inv, q), (q, inv), (inv, inv)] {
            let cr = unsafe { (l.c.c2AABBtoAABB)(x, y) };
            let rr = unsafe { (l.r.c2AABBtoAABB)(x, y) };
            if cr != 0 {
                got_true += 1;
            }
            d.check_i(cr, rr, || "inverted box".into());
        }
    }
    assert!(got_true > 100, "inverted boxes never reported overlap");
    eprintln!("    row10: {got_true} inverted-box overlaps reported by C");
    d.finish();
}

/// Row 11 — NaN coordinate: every `<` is false, so `!(0)` ⇒ returns 1.
#[test]
fn err_11_aabbaabb_nan() {
    let l = libs();
    let mut d = Diff::new("row11 c2AABBtoAABB NaN -> 1");
    let base = c2AABB { min: v(-1.0, -1.0), max: v(1.0, 1.0) };
    let far = c2AABB { min: v(1000.0, 1000.0), max: v(1001.0, 1001.0) };
    let mut fired = 0usize;
    for &nb in NAN_BITS {
        let s = f32::from_bits(nb);
        for field in 0..4 {
            let mut b = far;
            match field {
                0 => b.min.x = s,
                1 => b.min.y = s,
                2 => b.max.x = s,
                _ => b.max.y = s,
            }
            for (x, y) in [(base, b), (b, base)] {
                let cr = unsafe { (l.c.c2AABBtoAABB)(x, y) };
                let rr = unsafe { (l.r.c2AABBtoAABB)(x, y) };
                d.check_i(cr, rr, || format!("NaN {nb:#x} field {field}"));
                fired += 1;
            }
        }
        // All-NaN box: no comparison can be true -> must be 1.
        let allnan = c2AABB { min: v(s, s), max: v(s, s) };
        let cr = unsafe { (l.c.c2AABBtoAABB)(base, allnan) };
        let rr = unsafe { (l.r.c2AABBtoAABB)(base, allnan) };
        assert_eq!(cr, 1, "an all-NaN box must report overlap (all `<` false)");
        d.check_i(cr, rr, || "all-NaN box".into());
    }
    assert!(fired > 20);
    d.finish();
}

// ===========================================================================
// c2RayToPlane_OneDimensional (rows 12-13) and c2RaytoAABB (rows 14-18)
// ===========================================================================

/// Row 12 — `da < 0` inside the plane helper ⇒ that plane contributes `t == 0`.
/// Row 13 — the `d != 0` guard. Note the guard is only REACHABLE when
/// `da == db == 0`: if `da == db != 0` then `da*db > 0` returns `1.0f` first, so
/// `d == 0` requires the ray to lie exactly ON that plane's line.
///
/// Both helpers are `static inline` (not exported), so they are driven through
/// `c2RaytoAABB` and detected by recomputing their inputs.
#[test]
fn err_12_13_plane_helper_zero_returns() {
    let mut rng = Rng::new(SEED ^ 12);
    let mut d = Diff::new("row12/13 plane helper -> 0");
    let mut da_neg = 0usize;
    let mut parallel_zero = 0usize;

    let plane = |p: f32, n: f32, dd: f32| p * n - dd * n;

    for _ in 0..8000 {
        let cx = rng.coord();
        let cy = rng.coord();
        let b = c2AABB {
            min: v(cx - rng.range(0.5, 20.0), cy - rng.range(0.5, 20.0)),
            max: v(cx + rng.range(0.5, 20.0), cy + rng.range(0.5, 20.0)),
        };
        // To hit `da == db == 0` the ray must lie EXACTLY on one of the box's
        // four edge lines, so use the box's own coordinates as the origin and an
        // axis-aligned direction.
        let origins = [
            v(rng.range(cx - 60.0, cx + 60.0), b.min.y),
            v(rng.range(cx - 60.0, cx + 60.0), b.max.y),
            v(b.min.x, rng.range(cy - 60.0, cy + 60.0)),
            v(b.max.x, rng.range(cy - 60.0, cy + 60.0)),
            v(rng.range(cx - 60.0, cx + 60.0), rng.range(cy - 60.0, cy + 60.0)),
        ];
        let dirs = [v(1.0, 0.0), v(-1.0, 0.0), v(0.0, 1.0), v(0.0, -1.0)];
        for origin in origins {
            for dir in dirs {
                let ray = c2Ray {
                    p: origin,
                    d: dir,
                    t: rng.range(0.0, 200.0),
                };
                let rb = aabb_ray_box(ray);
                let l = libs();
                if unsafe { (l.c.c2AABBtoAABB)(rb, b) } == 0 || aabb_sat_d(ray, b) > 0.0 {
                    continue; // rejected earlier; the helper is not reached
                }
                let p1 = unsafe { (l.c.c2Add)(ray.p, (l.c.c2Mulvs)(ray.d, ray.t)) };
                let pairs = [
                    (plane(ray.p.x, -1.0, b.min.x), plane(p1.x, -1.0, b.min.x)),
                    (plane(ray.p.x, 1.0, b.max.x), plane(p1.x, 1.0, b.max.x)),
                    (plane(ray.p.y, -1.0, b.min.y), plane(p1.y, -1.0, b.min.y)),
                    (plane(ray.p.y, 1.0, b.max.y), plane(p1.y, 1.0, b.max.y)),
                ];
                for (da, db) in pairs {
                    if da < 0.0 {
                        da_neg += 1;
                    } else if !(da * db > 0.0) && da - db == 0.0 {
                        parallel_zero += 1;
                    }
                }
                let (cr, co, rr, ro) = call_aabb(ray, b);
                d.check_ray(cr, co, rr, ro, || format!("axis-aligned pairs={pairs:?}"));
            }
        }
    }
    assert!(da_neg > 100, "row12 `da < 0` fired only {da_neg} times");
    assert!(
        parallel_zero > 100,
        "row13 `da == db == 0` (the /0 guard) fired only {parallel_zero} times"
    );
    eprintln!("    row12: {da_neg} `da < 0`;  row13: {parallel_zero} `da - db == 0`");
    d.finish();
}

/// Row 14 — the ray's bounding box is disjoint from `B` ⇒ early `return 0`,
/// `*out` untouched.
#[test]
fn err_14_aabb_bbox_reject() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 14);
    let mut d = Diff::new("row14 c2RaytoAABB bbox reject");
    let mut fired = 0usize;
    for _ in 0..6000 {
        let b = c2AABB {
            min: v(rng.coord(), rng.coord()),
            max: v(rng.coord() + 100.0, rng.coord() + 100.0),
        };
        // Ray entirely on one side, pointing away.
        let ray = c2Ray {
            p: v(b.min.x - rng.range(10.0, 500.0), rng.coord()),
            d: v(-1.0, 0.0),
            t: rng.range(0.0, 100.0),
        };
        let rb = aabb_ray_box(ray);
        if unsafe { (l.c.c2AABBtoAABB)(rb, b) } != 0 {
            continue;
        }
        fired += 1;
        let (cr, co, rr, ro) = call_aabb(ray, b);
        assert_eq!(cr, 0, "bbox reject must return 0");
        assert!(rc_eq(co, POISON), "bbox reject must leave *out untouched");
        d.check_ray(cr, co, rr, ro, || "bbox reject".into());
    }
    assert!(fired > 2000, "row14 fired only {fired} times");
    eprintln!("    row14: {fired} bbox rejections");
    d.finish();
}

/// Row 15 — SAT rejection `d > 0`: the ray's bbox overlaps but its LINE misses.
#[test]
fn err_15_aabb_sat_reject() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 15);
    let mut d = Diff::new("row15 c2RaytoAABB SAT reject (d > 0)");
    let mut fired = 0usize;
    for _ in 0..40000 {
        let cx = rng.coord();
        let cy = rng.coord();
        let b = c2AABB {
            min: v(cx - rng.range(1.0, 20.0), cy - rng.range(1.0, 20.0)),
            max: v(cx + rng.range(1.0, 20.0), cy + rng.range(1.0, 20.0)),
        };
        let origin = v(rng.range(cx - 40.0, cx + 40.0), rng.range(cy - 40.0, cy + 40.0));
        let target = v(rng.range(cx - 40.0, cx + 40.0), rng.range(cy - 40.0, cy + 40.0));
        let dir = unsafe { (l.c.c2Norm)((l.c.c2Sub)(target, origin)) };
        let len = unsafe { (l.c.c2Len)((l.c.c2Sub)(target, origin)) };
        let ray = c2Ray { p: origin, d: dir, t: len };
        let rb = aabb_ray_box(ray);
        if unsafe { (l.c.c2AABBtoAABB)(rb, b) } == 0 {
            continue;
        }
        if !(aabb_sat_d(ray, b) > 0.0) {
            continue;
        }
        fired += 1;
        let (cr, co, rr, ro) = call_aabb(ray, b);
        assert_eq!(cr, 0, "SAT reject must return 0");
        assert!(rc_eq(co, POISON), "SAT reject must leave *out untouched");
        d.check_ray(cr, co, rr, ro, || {
            format!("SAT d={}", fmt_f(aabb_sat_d(ray, b)))
        });
        if fired >= 6000 {
            break;
        }
    }
    assert!(fired > 1000, "row15 fired only {fired} times");
    eprintln!("    row15: {fired} SAT rejections");
    d.finish();
}

/// Row 16 — `hit == 0`: all four `tN > 1.0f`. Reachable when the `tN` are NaN
/// (`NaN <= 1.0f` is false), which happens for a NaN ray direction.
/// Row 18 — the same path reached from `c2Norm` of a zero vector.
#[test]
fn err_16_18_aabb_no_plane_hit() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 16);
    let mut d = Diff::new("row16/18 c2RaytoAABB no plane hit");
    let mut fired = 0usize;

    let nan_dir = unsafe { (l.c.c2Norm)(v(0.0, 0.0)) };
    assert!(nan_dir.x.is_nan() && nan_dir.y.is_nan(), "c2Norm(0,0) must be NaN");

    let plane = |p: f32, n: f32, dd: f32| p * n - dd * n;
    let helper = |da: f32, db: f32| {
        if da < 0.0 {
            0.0f32
        } else if da * db > 0.0 {
            1.0
        } else {
            let dd = da - db;
            if dd != 0.0 { da / dd } else { 0.0 }
        }
    };

    let mut dirs = vec![nan_dir];
    for &nb in NAN_BITS {
        let s = f32::from_bits(nb);
        dirs.push(v(s, s));
        dirs.push(v(s, 1.0));
        dirs.push(v(1.0, s));
    }
    // With a well-formed box at least one of `da0`/`da1` is negative (the origin
    // cannot be on the outside of both the `-x` and the `+x` plane), so that
    // plane yields `t == 0` and `hit` is always true. `hit == 0` therefore needs
    // either NaN box coordinates (all four `da` NaN ⇒ all `tN` NaN ⇒
    // `NaN <= 1.0f` false) or an INVERTED box (`min > max`, so the origin can be
    // outside both opposing planes at once).
    let mut boxes: Vec<c2AABB> = Vec::new();
    for &nb in NAN_BITS {
        let s = f32::from_bits(nb);
        boxes.push(c2AABB { min: v(s, s), max: v(s, s) });
        boxes.push(c2AABB { min: v(s, -5.0), max: v(s, 5.0) });
        boxes.push(c2AABB { min: v(-5.0, s), max: v(5.0, s) });
        boxes.push(c2AABB { min: v(-5.0, -5.0), max: v(s, s) });
    }
    for _ in 0..600 {
        let cx = rng.coord();
        let cy = rng.coord();
        let hx = rng.range(1.0, 20.0);
        let hy = rng.range(1.0, 20.0);
        // Inverted box: min > max on both axes.
        boxes.push(c2AABB {
            min: v(cx + hx, cy + hy),
            max: v(cx - hx, cy - hy),
        });
        // Inverted on one axis only.
        boxes.push(c2AABB {
            min: v(cx + hx, cy - hy),
            max: v(cx - hx, cy + hy),
        });
        boxes.push(c2AABB {
            min: v(cx - hx, cy - hy),
            max: v(cx + hx, cy + hy),
        });
    }

    let mut all_dirs = dirs.clone();
    all_dirs.push(v(1.0, 0.0));
    all_dirs.push(v(0.0, 1.0));
    all_dirs.push(v(0.707, 0.707));

    for b in boxes {
        for &dir in &all_dirs {
            for t in [0.0f32, 1.0, 100.0, f32::INFINITY, f32::NAN] {
                for origin in [
                    v((b.min.x + b.max.x) * 0.5, (b.min.y + b.max.y) * 0.5),
                    v(b.min.x, b.min.y),
                    v(rng.coord(), rng.coord()),
                ] {
                    let ray = c2Ray { p: origin, d: dir, t };
                    let rb = aabb_ray_box(ray);
                    if unsafe { (l.c.c2AABBtoAABB)(rb, b) } == 0 || aabb_sat_d(ray, b) > 0.0 {
                        continue;
                    }
                    let p1 = unsafe { (l.c.c2Add)(ray.p, (l.c.c2Mulvs)(ray.d, ray.t)) };
                    let ts = [
                        helper(plane(ray.p.x, -1.0, b.min.x), plane(p1.x, -1.0, b.min.x)),
                        helper(plane(ray.p.x, 1.0, b.max.x), plane(p1.x, 1.0, b.max.x)),
                        helper(plane(ray.p.y, -1.0, b.min.y), plane(p1.y, -1.0, b.min.y)),
                        helper(plane(ray.p.y, 1.0, b.max.y), plane(p1.y, 1.0, b.max.y)),
                    ];
                    let no_hit = ts.iter().all(|&x| !(x <= 1.0));
                    let (cr, co, rr, ro) = call_aabb(ray, b);
                    if no_hit {
                        fired += 1;
                        assert_eq!(cr, 0, "hit==0 must return 0");
                        assert!(rc_eq(co, POISON), "hit==0 must leave *out untouched");
                    }
                    d.check_ray(cr, co, rr, ro, || format!("ts={ts:?}"));
                }
            }
        }
    }
    assert!(fired > 100, "row16/18 `hit == 0` fired only {fired} times");
    eprintln!("    row16/18: {fired} cases with hit == 0");
    d.finish();
}

/// Row 17 — zero-length ray (`A.t == 0`): `p1 == p0`, `n == 0`, so `d == 0` and
/// the SAT test does NOT reject; `out->t` becomes `0 * A.t`.
#[test]
fn err_17_aabb_zero_length_ray() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 17);
    let mut d = Diff::new("row17 c2RaytoAABB zero-length ray");
    let mut reached = 0usize;
    for _ in 0..6000 {
        let cx = rng.coord();
        let cy = rng.coord();
        let b = c2AABB {
            min: v(cx - rng.range(1.0, 20.0), cy - rng.range(1.0, 20.0)),
            max: v(cx + rng.range(1.0, 20.0), cy + rng.range(1.0, 20.0)),
        };
        for t in [0.0f32, -0.0f32] {
            // Origin inside the box: the ray bbox is a point inside B, so the
            // bbox test passes and the SAT `d` is exactly 0 (not > 0).
            let ray = c2Ray {
                p: v(rng.range(b.min.x, b.max.x), rng.range(b.min.y, b.max.y)),
                d: v(rng.coord(), rng.coord()),
                t,
            };
            let rb = aabb_ray_box(ray);
            let bbox_ok = unsafe { (l.c.c2AABBtoAABB)(rb, b) } != 0;
            let sat = aabb_sat_d(ray, b);
            if bbox_ok && !(sat > 0.0) {
                reached += 1;
            }
            let (cr, co, rr, ro) = call_aabb(ray, b);
            d.check_ray(cr, co, rr, ro, || {
                format!("zero-length ray t={} sat_d={}", fmt_f(t), fmt_f(sat))
            });
            // Outside the box too.
            let ray2 = c2Ray {
                p: v(b.max.x + rng.range(1.0, 50.0), cy),
                d: v(rng.coord(), rng.coord()),
                t,
            };
            let (cr, co, rr, ro) = call_aabb(ray2, b);
            d.check_ray(cr, co, rr, ro, || "zero-length ray outside".into());
        }
    }
    assert!(reached > 1000, "row17 reached the plane block only {reached} times");
    eprintln!("    row17: {reached} zero-length rays reached the plane block");
    d.finish();
}

// ===========================================================================
// c2AABBtoPoint — rows 19-23
// ===========================================================================

/// Rows 19-22 — each of the four rejecting comparisons in isolation.
/// Row 23 — a NaN point makes all four false ⇒ returns 1.
#[test]
fn err_19_23_aabb_to_point() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 19);
    let b = c2AABB { min: v(-2.0, -3.0), max: v(4.0, 5.0) };
    let labels = [
        "row19 B.x < A.min.x",
        "row20 B.y < A.min.y",
        "row21 B.x > A.max.x",
        "row22 B.y > A.max.y",
    ];
    for which in 0..4 {
        let mut d = Diff::new(labels[which]);
        let mut fired = 0usize;
        for _ in 0..3000 {
            let off = rng.range(0.0001, 100.0);
            // Keep the other coordinate strictly inside so only one flag fires.
            let inside_x = rng.range(-2.0, 4.0);
            let inside_y = rng.range(-3.0, 5.0);
            let p = match which {
                0 => v(-2.0 - off, inside_y),
                1 => v(inside_x, -3.0 - off),
                2 => v(4.0 + off, inside_y),
                _ => v(inside_x, 5.0 + off),
            };
            let flags = [
                p.x < b.min.x,
                p.y < b.min.y,
                p.x > b.max.x,
                p.y > b.max.y,
            ];
            if !flags[which] || flags.iter().filter(|x| **x).count() != 1 {
                continue;
            }
            fired += 1;
            let cr = unsafe { (l.c.c2AABBtoPoint)(b, p) };
            let rr = unsafe { (l.r.c2AABBtoPoint)(b, p) };
            assert_eq!(cr, 0, "{} must return 0", labels[which]);
            d.check_i(cr, rr, || format!("{} p={}", labels[which], fmt_v(p)));
        }
        assert!(fired > 500, "{} fired only {fired} times", labels[which]);
        eprintln!("    {}: {fired} cases", labels[which]);
        d.finish();
    }

    // Row 23 -- a NaN coordinate makes the two comparisons on that axis false.
    // It only yields 1 when the OTHER coordinate is inside, so the non-NaN
    // component is kept in range here.
    let mut d = Diff::new("row23 c2AABBtoPoint NaN");
    for &nb in NAN_BITS {
        let s = f32::from_bits(nb);
        for p in [v(s, 0.0), v(0.0, s), v(s, s), v(s, 4.0), v(-1.0, s)] {
            let cr = unsafe { (l.c.c2AABBtoPoint)(b, p) };
            let rr = unsafe { (l.r.c2AABBtoPoint)(b, p) };
            assert_eq!(
                cr, 1,
                "with the other coordinate inside, a NaN makes all four \
                 comparisons false -> 1 (p={})",
                fmt_v(p)
            );
            d.check_i(cr, rr, || format!("NaN point {}", fmt_v(p)));
        }
        // And a NaN paired with an out-of-range coordinate: the surviving
        // comparison still fires, so the result is 0.
        for p in [v(s, 1e9), v(-1e9, s), v(s, -1e9), v(1e9, s)] {
            let cr = unsafe { (l.c.c2AABBtoPoint)(b, p) };
            let rr = unsafe { (l.r.c2AABBtoPoint)(b, p) };
            assert_eq!(cr, 0, "the non-NaN axis is out of range -> 0 (p={})", fmt_v(p));
            d.check_i(cr, rr, || format!("NaN + out-of-range {}", fmt_v(p)));
        }
        // NaN in the box instead.
        for field in 0..4 {
            let mut bb = b;
            match field {
                0 => bb.min.x = s,
                1 => bb.min.y = s,
                2 => bb.max.x = s,
                _ => bb.max.y = s,
            }
            let cr = unsafe { (l.c.c2AABBtoPoint)(bb, v(0.0, 0.0)) };
            let rr = unsafe { (l.r.c2AABBtoPoint)(bb, v(0.0, 0.0)) };
            d.check_i(cr, rr, || format!("NaN box field {field}"));
        }
    }
    d.finish();
}

// ===========================================================================
// c2CircleToPoint — rows 24-26
// ===========================================================================

/// Row 24 — the boundary is OUTSIDE: `d2 < r*r` is strict, so a point exactly at
/// distance `r` must be rejected.
#[test]
fn err_24_circlepoint_on_boundary() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 24);
    let mut d = Diff::new("row24 c2CircleToPoint exactly on boundary -> 0");
    let mut exact = 0usize;
    for i in 1..4000u32 {
        let r = i as f32 * 0.011 + rng.range(0.0, 0.001);
        let c = c2Circle { p: v(rng.coord(), rng.coord()), r };
        // Axis-aligned offsets give an EXACT d2 == r*r.
        for p in [
            v(c.p.x + r, c.p.y),
            v(c.p.x - r, c.p.y),
            v(c.p.x, c.p.y + r),
            v(c.p.x, c.p.y - r),
        ] {
            let n = unsafe { (l.c.c2Sub)(c.p, p) };
            let d2 = unsafe { (l.c.c2Dot)(n, n) };
            let cr = unsafe { (l.c.c2CircleToPoint)(c, p) };
            let rr = unsafe { (l.r.c2CircleToPoint)(c, p) };
            if d2 == r * r {
                exact += 1;
                assert_eq!(cr, 0, "d2 == r*r must be REJECTED (strict `<`)");
            }
            d.check_i(cr, rr, || {
                format!("r={} d2={} r*r={}", fmt_f(r), fmt_f(d2), fmt_f(r * r))
            });
        }
        // One ULP inside and outside.
        for delta in [-1i32, 1] {
            let rr_ = if delta < 0 {
                f32::from_bits(r.to_bits() - 1)
            } else {
                f32::from_bits(r.to_bits() + 1)
            };
            let p = v(c.p.x + rr_, c.p.y);
            let cr = unsafe { (l.c.c2CircleToPoint)(c, p) };
            let rrv = unsafe { (l.r.c2CircleToPoint)(c, p) };
            d.check_i(cr, rrv, || format!("one ULP {delta}"));
        }
    }
    assert!(exact > 1000, "only {exact} exact-boundary cases");
    eprintln!("    row24: {exact} exact d2 == r*r cases, all rejected");
    d.finish();
}

/// Row 25 — `r == 0` ⇒ `d2 < 0` is impossible ⇒ always 0, even at the centre.
#[test]
fn err_25_circlepoint_zero_radius() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 25);
    let mut d = Diff::new("row25 c2CircleToPoint r == 0 -> always 0");
    for _ in 0..3000 {
        let p = v(rng.coord(), rng.coord());
        for r in [0.0f32, -0.0f32] {
            let c = c2Circle { p, r };
            for q in [p, v(p.x, p.y), v(-0.0 + p.x, p.y), v(p.x + 1.0, p.y)] {
                let cr = unsafe { (l.c.c2CircleToPoint)(c, q) };
                let rr = unsafe { (l.r.c2CircleToPoint)(c, q) };
                assert_eq!(cr, 0, "r == 0 must always return 0, even at the centre");
                d.check_i(cr, rr, || format!("r={} q={}", fmt_f(r), fmt_v(q)));
            }
        }
        // Subnormal radius: r*r underflows to exactly 0 -> also always 0.
        let c = c2Circle { p, r: 1e-30 };
        let cr = unsafe { (l.c.c2CircleToPoint)(c, p) };
        let rr = unsafe { (l.r.c2CircleToPoint)(c, p) };
        assert_eq!(cr, 0, "r*r underflowing to 0 must also reject");
        d.check_i(cr, rr, || "underflowing r*r".into());
    }
    d.finish();
}

/// Row 26 — NaN ⇒ `d2` is NaN ⇒ `NaN < r*r` is false ⇒ 0.
#[test]
fn err_26_circlepoint_nan() {
    let l = libs();
    let mut d = Diff::new("row26 c2CircleToPoint NaN -> 0");
    for &nb in NAN_BITS {
        let s = f32::from_bits(nb);
        let cases = [
            (c2Circle { p: v(0.0, 0.0), r: 2.0 }, v(s, 0.0)),
            (c2Circle { p: v(0.0, 0.0), r: 2.0 }, v(0.0, s)),
            (c2Circle { p: v(s, 0.0), r: 2.0 }, v(0.0, 0.0)),
            (c2Circle { p: v(0.0, s), r: 2.0 }, v(0.0, 0.0)),
            (c2Circle { p: v(0.0, 0.0), r: s }, v(0.0, 0.0)),
            (c2Circle { p: v(s, s), r: s }, v(s, s)),
        ];
        for (c, p) in cases {
            let cr = unsafe { (l.c.c2CircleToPoint)(c, p) };
            let rr = unsafe { (l.r.c2CircleToPoint)(c, p) };
            assert_eq!(cr, 0, "any NaN must make `d2 < r*r` false -> 0");
            d.check_i(cr, rr, || format!("NaN {nb:#x}"));
        }
    }
    d.finish();
}
