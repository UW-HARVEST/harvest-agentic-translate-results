//! Phase C — ERRORS.md error-surface tests.
//!
//! One `#[test]` per row of the ERROR-SURFACE TABLE in `ERRORS.md`.  Each test
//! constructs the *exact* invalid input / rejection condition described by the
//! row, calls both libraries through their `.so` exports, and asserts that they
//! return the same sentinel **and** leave / write `*out` identically.
//!
//! Rows 53 and 54 (unconditional null dereference in the C source) are not
//! differentially testable — a hard segfault cannot be compared as "the same
//! error code".  They are documented in `ERRORS.md`.

#![allow(non_snake_case)]

mod common;
use common::*;

use std::ffi::c_void;
use std::mem::size_of;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn ray(px: f32, py: f32, dx: f32, dy: f32, t: f32) -> C2Ray {
    C2Ray {
        p: v(px, py),
        d: v(dx, dy),
        t,
    }
}

fn bx(minx: f32, miny: f32, maxx: f32, maxy: f32) -> C2AABB {
    C2AABB {
        min: v(minx, miny),
        max: v(maxx, maxy),
    }
}

/// Differential call of `c2RaytoCircle`; returns the (agreed) C result.
#[track_caller]
fn diff_circle(A: C2Ray, B: C2Circle, what: &str) -> (i32, C2Raycast, bool) {
    let (c, r) = (c(), rs());
    let seed = 0xa5a5_a5a5u32;
    let mut oc = poison(seed);
    let mut orr = poison(seed);
    let rc = unsafe { (c.c2RaytoCircle)(A, B, &mut oc) };
    let rr = unsafe { (r.c2RaytoCircle)(A, B, &mut orr) };
    assert_eq!(rc, rr, "{what}: c2RaytoCircle C={rc} RUST={rr}");
    assert!(
        rceq(oc, orr),
        "{what}: c2RaytoCircle *out C={} RUST={}",
        rcshow(oc),
        rcshow(orr)
    );
    let touched = !rceq(oc, poison(seed));
    let r_touched = !rceq(orr, poison(seed));
    assert_eq!(touched, r_touched, "{what}: *out write differs");
    (rc, oc, touched)
}

#[track_caller]
fn diff_aabb(A: C2Ray, B: C2AABB, what: &str) -> (i32, C2Raycast, bool) {
    let (c, r) = (c(), rs());
    let seed = 0xa5a5_a5a5u32;
    let mut oc = poison(seed);
    let mut orr = poison(seed);
    let rc = unsafe { (c.c2RaytoAABB)(A, B, &mut oc) };
    let rr = unsafe { (r.c2RaytoAABB)(A, B, &mut orr) };
    assert_eq!(rc, rr, "{what}: c2RaytoAABB C={rc} RUST={rr}");
    assert!(
        rceq(oc, orr),
        "{what}: c2RaytoAABB *out C={} RUST={}",
        rcshow(oc),
        rcshow(orr)
    );
    let touched = !rceq(oc, poison(seed));
    assert_eq!(
        touched,
        !rceq(orr, poison(seed)),
        "{what}: *out write differs"
    );
    (rc, oc, touched)
}

#[track_caller]
fn diff_capsule(A: C2Ray, B: C2Capsule, what: &str) -> (i32, C2Raycast, bool) {
    let (c, r) = (c(), rs());
    let seed = 0xa5a5_a5a5u32;
    let mut oc = poison(seed);
    let mut orr = poison(seed);
    let rc = unsafe { (c.c2RaytoCapsule)(A, B, &mut oc) };
    let rr = unsafe { (r.c2RaytoCapsule)(A, B, &mut orr) };
    assert_eq!(rc, rr, "{what}: c2RaytoCapsule C={rc} RUST={rr}");
    assert!(
        rceq(oc, orr),
        "{what}: c2RaytoCapsule *out C={} RUST={}",
        rcshow(oc),
        rcshow(orr)
    );
    let touched = !rceq(oc, poison(seed));
    assert_eq!(
        touched,
        !rceq(orr, poison(seed)),
        "{what}: *out write differs"
    );
    (rc, oc, touched)
}

#[track_caller]
fn diff_poly(
    A: C2Ray,
    B: *const C2Poly,
    bxp: *const C2x,
    what: &str,
) -> (i32, C2Raycast, bool) {
    let (c, r) = (c(), rs());
    let seed = 0xa5a5_a5a5u32;
    let mut oc = poison(seed);
    let mut orr = poison(seed);
    let rc = unsafe { (c.c2RaytoPoly)(A, B, bxp, &mut oc) };
    let rr = unsafe { (r.c2RaytoPoly)(A, B, bxp, &mut orr) };
    assert_eq!(rc, rr, "{what}: c2RaytoPoly C={rc} RUST={rr}");
    assert!(
        rceq(oc, orr),
        "{what}: c2RaytoPoly *out C={} RUST={}",
        rcshow(oc),
        rcshow(orr)
    );
    let touched = !rceq(oc, poison(seed));
    assert_eq!(
        touched,
        !rceq(orr, poison(seed)),
        "{what}: *out write differs"
    );
    (rc, oc, touched)
}

#[track_caller]
fn diff_cast(
    A: C2Ray,
    B: *const c_void,
    bxp: *const C2x,
    ty: u32,
    what: &str,
) -> (i32, C2Raycast, bool) {
    let (c, r) = (c(), rs());
    let seed = 0xa5a5_a5a5u32;
    let mut oc = poison(seed);
    let mut orr = poison(seed);
    let rc = unsafe { (c.c2CastRay)(A, B, bxp, ty, &mut oc) };
    let rr = unsafe { (r.c2CastRay)(A, B, bxp, ty, &mut orr) };
    assert_eq!(rc, rr, "{what}: c2CastRay C={rc} RUST={rr}");
    assert!(
        rceq(oc, orr),
        "{what}: c2CastRay *out C={} RUST={}",
        rcshow(oc),
        rcshow(orr)
    );
    let touched = !rceq(oc, poison(seed));
    assert_eq!(
        touched,
        !rceq(orr, poison(seed)),
        "{what}: *out write differs"
    );
    (rc, oc, touched)
}

fn boxpoly(hw: f32, hh: f32) -> C2Poly {
    let mut p = C2Poly::default();
    p.count = 4;
    p.verts[0] = v(hw, -hh);
    p.verts[1] = v(hw, hh);
    p.verts[2] = v(-hw, hh);
    p.verts[3] = v(-hw, -hh);
    p.norms[0] = v(1.0, 0.0);
    p.norms[1] = v(0.0, 1.0);
    p.norms[2] = v(-1.0, 0.0);
    p.norms[3] = v(0.0, -1.0);
    p
}

const VCAP: C2Capsule = C2Capsule {
    a: C2v { x: 0.0, y: 0.0 },
    b: C2v { x: 0.0, y: 10.0 },
    r: 2.0,
};

// ===========================================================================
// c2RaytoCircle — rows 1..6
// ===========================================================================

/// ERRORS.md row 1 — `disc < 0`: the ray's supporting line misses the circle.
#[test]
fn err_row01_circle_disc_negative() {
    let (rc, _, touched) = diff_circle(
        ray(-10.0, 100.0, 1.0, 0.0, 5.0),
        C2Circle { p: v(0.0, 0.0), r: 1.0 },
        "row01",
    );
    assert_eq!(rc, 0, "row01: expected the rejection sentinel 0");
    assert!(!touched, "row01: *out must be left untouched");
    // sweep many offsets that all make disc < 0
    let mut rng = Rng::new(1);
    for _ in 0..2048 {
        let r0 = 0.25 + rng.unit(3.0).abs();
        let off = r0 + 0.5 + rng.unit(50.0).abs();
        let (rc, _, t) = diff_circle(
            ray(-100.0, off, 1.0, 0.0, 1000.0),
            C2Circle { p: v(0.0, 0.0), r: r0 },
            "row01/sweep",
        );
        assert_eq!(rc, 0);
        assert!(!t);
    }
}

/// ERRORS.md row 2 — `t < 0`: the circle is entirely behind the ray origin.
#[test]
fn err_row02_circle_behind_origin() {
    let (rc, _, touched) = diff_circle(
        ray(5.0, 0.0, 1.0, 0.0, 100.0),
        C2Circle { p: v(0.0, 0.0), r: 1.0 },
        "row02",
    );
    assert_eq!(rc, 0, "row02: expected 0");
    assert!(!touched);
    let mut rng = Rng::new(2);
    for _ in 0..2048 {
        let r0 = 0.25 + rng.unit(3.0).abs();
        let d = r0 + 0.5 + rng.unit(50.0).abs();
        for (px, py, dx, dy) in [
            (d, 0.0, 1.0, 0.0),
            (-d, 0.0, -1.0, 0.0),
            (0.0, d, 0.0, 1.0),
            (0.0, -d, 0.0, -1.0),
        ] {
            let (rc, _, t) = diff_circle(
                ray(px, py, dx, dy, 1000.0),
                C2Circle { p: v(0.0, 0.0), r: r0 },
                "row02/sweep",
            );
            assert_eq!(rc, 0);
            assert!(!t);
        }
    }
}

/// ERRORS.md row 3 — `t > A.t`: a real hit exists but is out of range.
#[test]
fn err_row03_circle_beyond_max_t() {
    let (rc, _, touched) = diff_circle(
        ray(-10.0, 0.0, 1.0, 0.0, 1.0),
        C2Circle { p: v(0.0, 0.0), r: 1.0 },
        "row03",
    );
    assert_eq!(rc, 0, "row03: expected 0");
    assert!(!touched);
    // t exactly at the hit distance must be ACCEPTED (`t <= A.t`), one ULP
    // below must be rejected — check both libraries agree on the boundary.
    let hit_t = 9.0f32; // dist 10, radius 1, exact
    let (accept, _, _) = diff_circle(
        ray(-10.0, 0.0, 1.0, 0.0, hit_t),
        C2Circle { p: v(0.0, 0.0), r: 1.0 },
        "row03/exact",
    );
    assert_eq!(accept, 1, "row03: t == hit distance must be accepted");
    let below = f32::from_bits(hit_t.to_bits() - 1);
    let (reject, _, _) = diff_circle(
        ray(-10.0, 0.0, 1.0, 0.0, below),
        C2Circle { p: v(0.0, 0.0), r: 1.0 },
        "row03/one-ulp-below",
    );
    assert_eq!(reject, 0, "row03: one ULP below must be rejected");
}

/// ERRORS.md row 4 — `disc` is NaN, so `disc < 0` is false but `t >= 0` is too.
#[test]
fn err_row04_circle_nan_inputs() {
    let nan_cases: [(C2Ray, C2Circle); 8] = [
        (ray(f32::NAN, 0.0, 1.0, 0.0, 10.0), C2Circle { p: v(0.0, 0.0), r: 1.0 }),
        (ray(0.0, f32::NAN, 1.0, 0.0, 10.0), C2Circle { p: v(0.0, 0.0), r: 1.0 }),
        (ray(-5.0, 0.0, f32::NAN, 0.0, 10.0), C2Circle { p: v(0.0, 0.0), r: 1.0 }),
        (ray(-5.0, 0.0, 1.0, f32::NAN, 10.0), C2Circle { p: v(0.0, 0.0), r: 1.0 }),
        (ray(-5.0, 0.0, 1.0, 0.0, f32::NAN), C2Circle { p: v(0.0, 0.0), r: 1.0 }),
        (ray(-5.0, 0.0, 1.0, 0.0, 10.0), C2Circle { p: v(f32::NAN, 0.0), r: 1.0 }),
        (ray(-5.0, 0.0, 1.0, 0.0, 10.0), C2Circle { p: v(0.0, 0.0), r: f32::NAN }),
        // inf - inf == NaN
        (
            ray(f32::INFINITY, 0.0, 1.0, 0.0, 10.0),
            C2Circle { p: v(0.0, 0.0), r: f32::INFINITY },
        ),
    ];
    for (i, (a, b)) in nan_cases.iter().enumerate() {
        let (rc, _, touched) = diff_circle(*a, *b, &format!("row04/{i}"));
        assert_eq!(rc, 0, "row04/{i}: expected 0 from a NaN input");
        assert!(!touched, "row04/{i}: *out must be untouched");
    }
    // and with -NaN
    for &s in [f32::NAN, -f32::NAN].iter() {
        for slot in 0..7 {
            let mut a = ray(-5.0, 0.25, 1.0, 0.0, 10.0);
            let mut b = C2Circle { p: v(0.0, 0.0), r: 1.0 };
            match slot {
                0 => a.p.x = s,
                1 => a.p.y = s,
                2 => a.d.x = s,
                3 => a.d.y = s,
                4 => a.t = s,
                5 => b.p.x = s,
                _ => b.r = s,
            }
            diff_circle(a, b, "row04/slots");
        }
    }
}

/// ERRORS.md row 5 — hit accepted but `c2Norm` divides by zero.
#[test]
fn err_row05_circle_zero_radius_norm_div0() {
    let (rc, out, touched) = diff_circle(
        ray(-3.0, 0.0, 1.0, 0.0, 10.0),
        C2Circle { p: v(0.0, 0.0), r: 0.0 },
        "row05",
    );
    assert_eq!(rc, 1, "row05: the C library reports a hit");
    assert!(touched, "row05: *out is written on a hit");
    assert!(
        out.n.x.is_nan() && out.n.y.is_nan(),
        "row05: expected c2Norm(0,0) to give NaN, got {}",
        vshow(out.n)
    );
    // more configurations that hit the exact centre with r == 0 / r == -0.0
    for r0 in [0.0f32, -0.0] {
        for (px, py, dx, dy) in [
            (-3.0f32, 0.0f32, 1.0f32, 0.0f32),
            (3.0, 0.0, -1.0, 0.0),
            (0.0, -3.0, 0.0, 1.0),
            (0.0, 3.0, 0.0, -1.0),
            (-3.0, -3.0, 1.0, 1.0),
        ] {
            diff_circle(
                ray(px, py, dx, dy, 100.0),
                C2Circle { p: v(0.0, 0.0), r: r0 },
                "row05/sweep",
            );
        }
    }
}

/// ERRORS.md row 6 — negative radius behaves like `|r|` (no validation).
#[test]
fn err_row06_circle_negative_radius() {
    let (c, r) = (c(), rs());
    let mut rng = Rng::new(6);
    for _ in 0..2048 {
        let rad = 0.25 + rng.unit(5.0).abs();
        let a = ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom());
        // differential
        diff_circle(a, C2Circle { p: v(0.0, 0.0), r: -rad }, "row06");
        // and: -r must behave identically to +r in *both* libraries
        for api in [c, r] {
            let mut o1 = poison(11);
            let mut o2 = poison(11);
            let r1 = unsafe {
                (api.c2RaytoCircle)(a, C2Circle { p: v(0.0, 0.0), r: rad }, &mut o1)
            };
            let r2 = unsafe {
                (api.c2RaytoCircle)(a, C2Circle { p: v(0.0, 0.0), r: -rad }, &mut o2)
            };
            assert_eq!(r1, r2, "{}: row06: +r != -r", api.name);
            assert!(rceq(o1, o2), "{}: row06: +r != -r (out)", api.name);
        }
    }
}

// ===========================================================================
// c2AABBtoAABB — rows 7..11
// ===========================================================================

#[track_caller]
fn diff_bb(A: C2AABB, B: C2AABB, what: &str) -> i32 {
    let (c, r) = (c(), rs());
    let rc = unsafe { (c.c2AABBtoAABB)(A, B) };
    let rr = unsafe { (r.c2AABBtoAABB)(A, B) };
    assert_eq!(rc, rr, "{what}: c2AABBtoAABB C={rc} RUST={rr}");
    rc
}

/// ERRORS.md row 7 — `B.max.x < A.min.x`.
#[test]
fn err_row07_aabbtoaabb_sep_neg_x() {
    let a = bx(0.0, 0.0, 1.0, 1.0);
    assert_eq!(diff_bb(a, bx(-3.0, 0.0, -0.5, 1.0), "row07"), 0);
    let mut rng = Rng::new(7);
    for _ in 0..2048 {
        let gap = 0.001 + rng.unit(20.0).abs();
        let w = 0.001 + rng.unit(5.0).abs();
        assert_eq!(
            diff_bb(a, bx(a.min.x - gap - w, 0.0, a.min.x - gap, 1.0), "row07/sweep"),
            0
        );
    }
}

/// ERRORS.md row 8 — `A.max.x < B.min.x`.
#[test]
fn err_row08_aabbtoaabb_sep_pos_x() {
    let a = bx(0.0, 0.0, 1.0, 1.0);
    assert_eq!(diff_bb(a, bx(1.5, 0.0, 3.0, 1.0), "row08"), 0);
    let mut rng = Rng::new(8);
    for _ in 0..2048 {
        let gap = 0.001 + rng.unit(20.0).abs();
        let w = 0.001 + rng.unit(5.0).abs();
        assert_eq!(
            diff_bb(a, bx(a.max.x + gap, 0.0, a.max.x + gap + w, 1.0), "row08/sweep"),
            0
        );
    }
}

/// ERRORS.md row 9 — `B.max.y < A.min.y`.
#[test]
fn err_row09_aabbtoaabb_sep_neg_y() {
    let a = bx(0.0, 0.0, 1.0, 1.0);
    assert_eq!(diff_bb(a, bx(0.0, -3.0, 1.0, -0.5), "row09"), 0);
    let mut rng = Rng::new(9);
    for _ in 0..2048 {
        let gap = 0.001 + rng.unit(20.0).abs();
        let h = 0.001 + rng.unit(5.0).abs();
        assert_eq!(
            diff_bb(a, bx(0.0, a.min.y - gap - h, 1.0, a.min.y - gap), "row09/sweep"),
            0
        );
    }
}

/// ERRORS.md row 10 — `A.max.y < B.min.y`.
#[test]
fn err_row10_aabbtoaabb_sep_pos_y() {
    let a = bx(0.0, 0.0, 1.0, 1.0);
    assert_eq!(diff_bb(a, bx(0.0, 1.5, 1.0, 3.0), "row10"), 0);
    let mut rng = Rng::new(10);
    for _ in 0..2048 {
        let gap = 0.001 + rng.unit(20.0).abs();
        let h = 0.001 + rng.unit(5.0).abs();
        assert_eq!(
            diff_bb(a, bx(0.0, a.max.y + gap, 1.0, a.max.y + gap + h), "row10/sweep"),
            0
        );
    }
}

/// ERRORS.md row 11 — any NaN coordinate makes all four `<` false -> `1`.
#[test]
fn err_row11_aabbtoaabb_nan() {
    let a = bx(0.0, 0.0, 1.0, 1.0);
    let far = bx(1000.0, 1000.0, 2000.0, 2000.0);
    // NaN in each slot of B, with B otherwise far away
    for slot in 0..4 {
        let mut b = far;
        match slot {
            0 => b.min.x = f32::NAN,
            1 => b.min.y = f32::NAN,
            2 => b.max.x = f32::NAN,
            _ => b.max.y = f32::NAN,
        }
        let got = diff_bb(a, b, &format!("row11/B{slot}"));
        // rows 7/9 depend on B.max, rows 8/10 on B.min: only some slots flip
        // the answer to 1 — whatever the C says, the Rust must agree (already
        // asserted).  Record the fully-NaN case explicitly:
        let _ = got;
    }
    let all_nan = bx(f32::NAN, f32::NAN, f32::NAN, f32::NAN);
    assert_eq!(
        diff_bb(a, all_nan, "row11/all-nan-B"),
        1,
        "row11: an all-NaN box must be reported as OVERLAPPING"
    );
    assert_eq!(diff_bb(all_nan, a, "row11/all-nan-A"), 1);
    assert_eq!(diff_bb(all_nan, all_nan, "row11/both-nan"), 1);
    // exhaustive: NaN in every subset of the 8 coordinates
    for mask in 0u32..256 {
        let mut aa = bx(0.0, 0.0, 1.0, 1.0);
        let mut bb = bx(5.0, 5.0, 6.0, 6.0);
        let slots: [&mut f32; 8] = [
            &mut aa.min.x,
            &mut aa.min.y,
            &mut aa.max.x,
            &mut aa.max.y,
            &mut bb.min.x,
            &mut bb.min.y,
            &mut bb.max.x,
            &mut bb.max.y,
        ];
        for (i, s) in slots.into_iter().enumerate() {
            if mask & (1 << i) != 0 {
                *s = f32::NAN;
            }
        }
        diff_bb(aa, bb, &format!("row11/mask{mask}"));
    }
}

// ===========================================================================
// c2RaytoAABB — rows 12..18
// ===========================================================================

/// ERRORS.md row 12 — the swept-ray bounding box does not overlap `B`.
#[test]
fn err_row12_raytoaabb_bb_miss() {
    let b = bx(-1.0, -1.0, 1.0, 1.0);
    let (rc, _, touched) = diff_aabb(ray(-100.0, -100.0, -1.0, -1.0, 1.0), b, "row12");
    assert_eq!(rc, 0, "row12: expected 0");
    assert!(!touched, "row12: *out must be untouched");
    let mut rng = Rng::new(12);
    for _ in 0..2048 {
        let far = 5.0 + rng.unit(100.0).abs();
        for a in [
            ray(-far, 0.0, -1.0, 0.0, 1.0),
            ray(far, 0.0, 1.0, 0.0, 1.0),
            ray(0.0, -far, 0.0, -1.0, 1.0),
            ray(0.0, far, 0.0, 1.0, 1.0),
        ] {
            let (rc, _, t) = diff_aabb(a, b, "row12/sweep");
            assert_eq!(rc, 0);
            assert!(!t);
        }
    }
}

/// ERRORS.md row 13 — `d > 0`: SAT on the ray normal separates.
#[test]
fn err_row13_raytoaabb_sat_separated() {
    let b = bx(-1.0, -1.0, 1.0, 1.0);
    let a = ray(-4.0, 2.5, 1.0, 1.0, 20.0);
    let (rc, _, touched) = diff_aabb(a, b, "row13");
    assert_eq!(rc, 0, "row13: expected 0");
    assert!(!touched, "row13: *out must be untouched");
    let mut rng = Rng::new(13);
    for _ in 0..4096 {
        // diagonal rays whose sweep box covers B but whose line misses it
        let off = 2.5 + rng.unit(6.0).abs();
        let (rc, _, t) = diff_aabb(ray(-4.0, off, 1.0, 1.0, 20.0), b, "row13/sweep");
        assert_eq!(rc, 0);
        assert!(!t);
    }
}

/// ERRORS.md row 14 — `hit == 0`, i.e. all four `t0..t3 > 1`.
#[test]
fn err_row14_raytoaabb_no_hit_flags() {
    let b = bx(-1.0, -1.0, 1.0, 1.0);
    // A ray whose sweep box and SAT test both pass but which stops short.
    let mut found = 0;
    let mut rng = Rng::new(14);
    for _ in 0..8192 {
        let a = ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom());
        let (rc, _, touched) = diff_aabb(a, b, "row14/search");
        if rc == 0 {
            assert!(!touched, "row14: *out must be untouched on any miss");
            found += 1;
        }
    }
    assert!(found > 0, "row14: no miss found in the randomized sweep");
    // explicit: ray parallel to the box, well above it, sweep-box overlapping
    let (rc, _, t) = diff_aabb(ray(-4.0, 8.0, 1.0, 0.0, 1.0), b, "row14/explicit");
    assert_eq!(rc, 0);
    assert!(!t);
}

/// ERRORS.md row 15 — `c2RayToPlane_OneDimensional` returns the `0.0f` guard
/// when `da < 0` (start point already behind the plane).
#[test]
fn err_row15_raytoplane1d_da_negative() {
    // da0 = p0.x*(-1) - B.min.x*(-1) = B.min.x - p0.x  ->  < 0 iff p0.x > min.x
    // Sweep the origin across the box so each of da0..da3 goes negative in turn.
    let b = bx(-1.0, -1.0, 1.0, 1.0);
    for px in [-4.0f32, -1.0, -0.5, 0.0, 0.5, 1.0, 4.0] {
        for py in [-4.0f32, -1.0, -0.5, 0.0, 0.5, 1.0, 4.0] {
            for (dx, dy) in [(1.0f32, 0.0f32), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0), (1.0, 1.0)] {
                for t in [0.0f32, 0.5, 1.0, 2.0, 8.0] {
                    diff_aabb(ray(px, py, dx, dy, t), b, "row15");
                }
            }
        }
    }
    // Origin strictly inside makes all four `da` negative simultaneously.
    let (rc, _, _) = diff_aabb(ray(0.0, 0.0, 1.0, 0.0, 4.0), b, "row15/inside");
    let _ = rc;
}

/// ERRORS.md row 16 — the `d = da - db == 0` division-by-zero guard.
#[test]
fn err_row16_raytoplane1d_div_guard() {
    // da == db exactly when the ray does not move along that axis.
    let b = bx(-1.0, -1.0, 1.0, 1.0);
    for x in [-2.0f32, -1.0, -0.5, 0.0, 0.5, 1.0, 2.0] {
        for t in [0.0f32, 1.0, 4.0] {
            // no movement in x -> da0 == db0 and da1 == db1
            diff_aabb(ray(x, -4.0, 0.0, 1.0, t), b, "row16/x-static");
            diff_aabb(ray(x, 4.0, 0.0, -1.0, t), b, "row16/x-static2");
            // no movement in y -> da2 == db2 and da3 == db3
            diff_aabb(ray(-4.0, x, 1.0, 0.0, t), b, "row16/y-static");
            diff_aabb(ray(4.0, x, -1.0, 0.0, t), b, "row16/y-static2");
            // no movement at all -> all four guards fire
            diff_aabb(ray(x, x, 0.0, 0.0, t), b, "row16/static");
            diff_aabb(ray(x, x, 1.0, 1.0, 0.0), b, "row16/zero-t");
        }
    }
}

/// ERRORS.md row 17 — inverted box (`min > max`), no validation at all.
#[test]
fn err_row17_raytoaabb_inverted_box() {
    let inv = bx(1.0, 1.0, -1.0, -1.0);
    let (rc, _, _) = diff_aabb(ray(-5.0, 0.0, 1.0, 0.0, 20.0), inv, "row17");
    assert_eq!(rc, 0, "row17: an inverted box never overlaps");
    let mut rng = Rng::new(17);
    for _ in 0..4096 {
        let a = rng.geom_v();
        let bb = rng.geom_v();
        let inverted = bx(
            a.x.max(bb.x) + 0.001,
            a.y.max(bb.y) + 0.001,
            a.x.min(bb.x) - 0.001,
            a.y.min(bb.y) - 0.001,
        );
        diff_aabb(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            inverted,
            "row17/sweep",
        );
    }
}

/// ERRORS.md row 18 — `A.t == 0`: `p0 == p1`, `ab == 0`, `n == (0,0)`.
#[test]
fn err_row18_raytoaabb_zero_length_ray() {
    let b = bx(-1.0, -1.0, 1.0, 1.0);
    for t in [0.0f32, -0.0] {
        for px in [-2.0f32, -1.0, 0.0, 1.0, 2.0] {
            for py in [-2.0f32, -1.0, 0.0, 1.0, 2.0] {
                for (dx, dy) in [(1.0f32, 0.0f32), (0.0, 1.0), (0.0, 0.0), (1.0e30, 1.0e30)] {
                    diff_aabb(ray(px, py, dx, dy, t), b, "row18");
                }
            }
        }
    }
    let mut rng = Rng::new(18);
    for _ in 0..2048 {
        diff_aabb(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), 0.0),
            C2AABB {
                min: rng.geom_v(),
                max: rng.geom_v(),
            },
            "row18/sweep",
        );
    }
}

// ===========================================================================
// c2AABBtoPoint — rows 19..23
// ===========================================================================

#[track_caller]
fn diff_bp(A: C2AABB, B: C2v, what: &str) -> i32 {
    let (c, r) = (c(), rs());
    let rc = unsafe { (c.c2AABBtoPoint)(A, B) };
    let rr = unsafe { (r.c2AABBtoPoint)(A, B) };
    assert_eq!(rc, rr, "{what}: c2AABBtoPoint C={rc} RUST={rr}");
    rc
}

/// ERRORS.md row 19 — `B.x < A.min.x`.
#[test]
fn err_row19_aabbtopoint_below_min_x() {
    let a = bx(0.0, 0.0, 2.0, 2.0);
    assert_eq!(diff_bp(a, v(-0.5, 1.0), "row19"), 0);
    let mut rng = Rng::new(19);
    for _ in 0..2048 {
        let e = 0.001 + rng.unit(50.0).abs();
        assert_eq!(diff_bp(a, v(a.min.x - e, 1.0), "row19/sweep"), 0);
    }
    // exactly at min.x is INSIDE (the check is strict `<`)
    assert_eq!(diff_bp(a, v(a.min.x, 1.0), "row19/boundary"), 1);
}

/// ERRORS.md row 20 — `B.y < A.min.y`.
#[test]
fn err_row20_aabbtopoint_below_min_y() {
    let a = bx(0.0, 0.0, 2.0, 2.0);
    assert_eq!(diff_bp(a, v(1.0, -0.5), "row20"), 0);
    let mut rng = Rng::new(20);
    for _ in 0..2048 {
        let e = 0.001 + rng.unit(50.0).abs();
        assert_eq!(diff_bp(a, v(1.0, a.min.y - e), "row20/sweep"), 0);
    }
    assert_eq!(diff_bp(a, v(1.0, a.min.y), "row20/boundary"), 1);
}

/// ERRORS.md row 21 — `B.x > A.max.x`.
#[test]
fn err_row21_aabbtopoint_above_max_x() {
    let a = bx(0.0, 0.0, 2.0, 2.0);
    assert_eq!(diff_bp(a, v(2.5, 1.0), "row21"), 0);
    let mut rng = Rng::new(21);
    for _ in 0..2048 {
        let e = 0.001 + rng.unit(50.0).abs();
        assert_eq!(diff_bp(a, v(a.max.x + e, 1.0), "row21/sweep"), 0);
    }
    assert_eq!(diff_bp(a, v(a.max.x, 1.0), "row21/boundary"), 1);
}

/// ERRORS.md row 22 — `B.y > A.max.y`.
#[test]
fn err_row22_aabbtopoint_above_max_y() {
    let a = bx(0.0, 0.0, 2.0, 2.0);
    assert_eq!(diff_bp(a, v(1.0, 2.5), "row22"), 0);
    let mut rng = Rng::new(22);
    for _ in 0..2048 {
        let e = 0.001 + rng.unit(50.0).abs();
        assert_eq!(diff_bp(a, v(1.0, a.max.y + e), "row22/sweep"), 0);
    }
    assert_eq!(diff_bp(a, v(1.0, a.max.y), "row22/boundary"), 1);
}

/// ERRORS.md row 23 — a NaN coordinate makes all four comparisons false -> 1.
#[test]
fn err_row23_aabbtopoint_nan() {
    let a = bx(0.0, 0.0, 2.0, 2.0);
    assert_eq!(
        diff_bp(a, v(f32::NAN, f32::NAN), "row23/point-nan"),
        1,
        "row23: an all-NaN point must be reported as INSIDE"
    );
    assert_eq!(diff_bp(a, v(f32::NAN, 1.0), "row23/x-nan"), 1);
    assert_eq!(diff_bp(a, v(1.0, f32::NAN), "row23/y-nan"), 1);
    assert_eq!(diff_bp(a, v(-f32::NAN, 1000.0), "row23/mixed"), 0);
    // exhaustive NaN masks over all 6 coordinates
    for mask in 0u32..64 {
        let mut aa = bx(0.0, 0.0, 2.0, 2.0);
        let mut p = v(5.0, 5.0);
        let slots: [&mut f32; 6] = [
            &mut aa.min.x,
            &mut aa.min.y,
            &mut aa.max.x,
            &mut aa.max.y,
            &mut p.x,
            &mut p.y,
        ];
        for (i, s) in slots.into_iter().enumerate() {
            if mask & (1 << i) != 0 {
                *s = f32::NAN;
            }
        }
        diff_bp(aa, p, &format!("row23/mask{mask}"));
    }
}

// ===========================================================================
// c2CircleToPoint — rows 24..26
// ===========================================================================

#[track_caller]
fn diff_cp(A: C2Circle, B: C2v, what: &str) -> i32 {
    let (c, r) = (c(), rs());
    let rc = unsafe { (c.c2CircleToPoint)(A, B) };
    let rr = unsafe { (r.c2CircleToPoint)(A, B) };
    assert_eq!(rc, rr, "{what}: c2CircleToPoint C={rc} RUST={rr}");
    rc
}

/// ERRORS.md row 24 — `d2 >= r*r`, with an EXCLUSIVE boundary.
#[test]
fn err_row24_circletopoint_outside_exclusive() {
    let circle = C2Circle { p: v(0.0, 0.0), r: 5.0 };
    // exactly on the rim -> rejected (3-4-5 is exact in binary floating point)
    assert_eq!(
        diff_cp(circle, v(3.0, 4.0), "row24/rim"),
        0,
        "row24: the rim is EXCLUSIVE"
    );
    assert_eq!(diff_cp(circle, v(5.0, 0.0), "row24/rim-axis"), 0);
    assert_eq!(diff_cp(circle, v(0.0, -5.0), "row24/rim-axis2"), 0);
    // just inside / just outside
    assert_eq!(diff_cp(circle, v(3.0, 3.9375), "row24/inside"), 1);
    assert_eq!(diff_cp(circle, v(5.0625, 0.0), "row24/outside"), 0);
    let mut rng = Rng::new(24);
    for _ in 0..4096 {
        let rad = 0.25 + rng.unit(8.0).abs();
        let d = rad + 0.001 + rng.unit(50.0).abs();
        let ang = rng.unit(std::f32::consts::PI);
        assert_eq!(
            diff_cp(
                C2Circle { p: v(0.0, 0.0), r: rad },
                v(ang.cos() * d, ang.sin() * d),
                "row24/sweep"
            ),
            0
        );
    }
}

/// ERRORS.md row 25 — `r == 0` can never contain a point (`d2 < 0` impossible).
#[test]
fn err_row25_circletopoint_zero_radius() {
    for r0 in [0.0f32, -0.0] {
        let circle = C2Circle { p: v(1.0, 2.0), r: r0 };
        assert_eq!(diff_cp(circle, v(1.0, 2.0), "row25/centre"), 0);
        assert_eq!(diff_cp(circle, v(0.0, 0.0), "row25/other"), 0);
        let mut rng = Rng::new(25);
        for _ in 0..2048 {
            assert_eq!(diff_cp(circle, rng.geom_v(), "row25/sweep"), 0);
        }
    }
    // smallest subnormal radius: r*r underflows to 0 -> also always 0
    let tiny = C2Circle {
        p: v(0.0, 0.0),
        r: f32::from_bits(1),
    };
    assert_eq!(diff_cp(tiny, v(0.0, 0.0), "row25/subnormal"), 0);
}

/// ERRORS.md row 26 — a NaN makes `<` false -> `0`.
#[test]
fn err_row26_circletopoint_nan() {
    assert_eq!(
        diff_cp(
            C2Circle { p: v(f32::NAN, 0.0), r: 5.0 },
            v(0.0, 0.0),
            "row26/p"
        ),
        0
    );
    assert_eq!(
        diff_cp(
            C2Circle { p: v(0.0, 0.0), r: f32::NAN },
            v(0.0, 0.0),
            "row26/r"
        ),
        0
    );
    assert_eq!(
        diff_cp(
            C2Circle { p: v(0.0, 0.0), r: 5.0 },
            v(f32::NAN, 0.0),
            "row26/B"
        ),
        0
    );
    for mask in 0u32..32 {
        let mut circle = C2Circle { p: v(0.0, 0.0), r: 5.0 };
        let mut p = v(1.0, 1.0);
        let slots: [&mut f32; 5] = [&mut circle.p.x, &mut circle.p.y, &mut circle.r, &mut p.x, &mut p.y];
        for (i, s) in slots.into_iter().enumerate() {
            if mask & (1 << i) != 0 {
                *s = f32::NAN;
            }
        }
        diff_cp(circle, p, &format!("row26/mask{mask}"));
    }
}

// ===========================================================================
// c2RaytoCapsule — rows 27..34
// ===========================================================================

/// ERRORS.md row 27 — degenerate `a == b`: `c2Norm(0)` poisons everything.
#[test]
fn err_row27_capsule_degenerate_ab() {
    let degen = C2Capsule {
        a: v(0.0, 0.0),
        b: v(0.0, 0.0),
        r: 2.0,
    };
    let (rc, out, touched) = diff_capsule(ray(-5.0, 0.0, 1.0, 0.0, 20.0), degen, "row27");
    assert!(touched, "row27: *out is written unconditionally");
    assert!(
        out.n.x.is_nan() && out.n.y.is_nan(),
        "row27: expected NaN normal from c2Norm(0,0), got {}",
        vshow(out.n)
    );
    let _ = rc;
    let mut rng = Rng::new(27);
    for _ in 0..4096 {
        let p = rng.geom_v();
        diff_capsule(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            C2Capsule { a: p, b: p, r: rng.geom() },
            "row27/sweep",
        );
    }
}

/// ERRORS.md row 28 — origin inside the local slab -> early `return 1`.
#[test]
fn err_row28_capsule_origin_in_slab() {
    let (rc, out, touched) = diff_capsule(ray(0.0, 5.0, 1.0, 0.0, 10.0), VCAP, "row28");
    assert_eq!(rc, 1, "row28: expected the early return 1");
    assert!(touched);
    assert!(feq(out.t, 0.0), "row28: t must be 0, got {}", fshow(out.t));
    assert!(
        veq(out.n, v(0.0, 1.0)),
        "row28: n must be c2Norm(b-a) = (0,1), got {}",
        vshow(out.n)
    );
    let mut rng = Rng::new(28);
    for _ in 0..4096 {
        let x = rng.unit(2.0);
        let y = rng.unit(1.0).abs() * 10.0;
        let (rc, o, _) = diff_capsule(
            ray(x, y, rng.geom(), rng.geom(), rng.geom()),
            VCAP,
            "row28/sweep",
        );
        assert_eq!(rc, 1, "row28/sweep: origin ({x},{y}) is inside the slab");
        assert!(feq(o.t, 0.0));
    }
}

/// ERRORS.md row 29 — origin inside end-cap A -> early `return 1`.
#[test]
fn err_row29_capsule_origin_in_cap_a() {
    let (rc, out, _) = diff_capsule(ray(0.0, -1.0, 1.0, 0.0, 10.0), VCAP, "row29");
    assert_eq!(rc, 1, "row29: expected the early return 1");
    assert!(feq(out.t, 0.0));
    assert!(veq(out.n, v(0.0, 1.0)));
    let mut rng = Rng::new(29);
    for _ in 0..4096 {
        // strictly inside cap A but with y < 0 so the slab test fails first
        let ang = std::f32::consts::PI + rng.unit(1.0).abs() * std::f32::consts::PI;
        let d = rng.unit(1.0).abs() * 1.99;
        let (rc, _, _) = diff_capsule(
            ray(ang.cos() * d, -(d * 0.5).abs() - 0.0001, rng.geom(), rng.geom(), rng.geom()),
            VCAP,
            "row29/sweep",
        );
        let _ = rc;
    }
}

/// ERRORS.md row 30 — origin inside end-cap B -> early `return 1`.
#[test]
fn err_row30_capsule_origin_in_cap_b() {
    let (rc, out, _) = diff_capsule(ray(0.0, 11.0, 1.0, 0.0, 10.0), VCAP, "row30");
    assert_eq!(rc, 1, "row30: expected the early return 1");
    assert!(feq(out.t, 0.0));
    assert!(veq(out.n, v(0.0, 1.0)));
    let mut rng = Rng::new(30);
    for _ in 0..4096 {
        let d = rng.unit(1.0).abs() * 1.99;
        let (rc, _, _) = diff_capsule(
            ray(0.0, 10.0 + (d * 0.5).abs() + 0.0001, rng.geom(), rng.geom(), rng.geom()),
            VCAP,
            "row30/sweep",
        );
        let _ = rc;
    }
}

/// ERRORS.md row 31 — full miss, but `*out` has already been overwritten.
#[test]
fn err_row31_capsule_miss_but_out_written() {
    let (rc, out, touched) = diff_capsule(ray(5.0, 5.0, 1.0, 0.0, 10.0), VCAP, "row31");
    assert_eq!(rc, 0, "row31: expected the rejection sentinel 0");
    assert!(
        touched,
        "row31: the C library overwrites *out even on this miss"
    );
    assert!(feq(out.t, 0.0), "row31: t must be 0, got {}", fshow(out.t));
    assert!(
        veq(out.n, v(0.0, 1.0)),
        "row31: n must be c2Norm(b-a), got {}",
        vshow(out.n)
    );
    let mut rng = Rng::new(31);
    for _ in 0..4096 {
        let x = 2.0 + 0.001 + rng.unit(50.0).abs();
        let (rc, _, t) = diff_capsule(ray(x, 5.0, 1.0, 0.0, 10.0), VCAP, "row31/sweep");
        assert_eq!(rc, 0);
        assert!(t, "row31/sweep: *out must still be written");
    }
}

/// ERRORS.md row 32 — unguarded `(c - yAp.x) / (yAe.x - yAp.x)` with a zero
/// denominator.
#[test]
fn err_row32_capsule_div_by_zero_dx() {
    // yAd.x == 0 => yAe.x == yAp.x => d == 0.
    for x in [-8.0f32, -3.0, -2.0001, 2.0001, 3.0, 8.0] {
        for y in [-5.0f32, 0.0, 5.0, 10.0, 15.0] {
            for t in [1.0f32, 10.0, 1.0e30, f32::INFINITY] {
                diff_capsule(ray(x, y, 0.0, 1.0, t), VCAP, "row32/no-x-motion");
                diff_capsule(ray(x, y, 0.0, -1.0, t), VCAP, "row32/no-x-motion2");
                diff_capsule(ray(x, y, -0.0, 1.0, t), VCAP, "row32/neg-zero-dx");
            }
            // A.t == 0 also makes yAe == yAp
            diff_capsule(ray(x, y, 1.0, 0.0, 0.0), VCAP, "row32/zero-t");
            diff_capsule(ray(x, y, 0.0, 0.0, 10.0), VCAP, "row32/zero-d");
        }
    }
}

/// ERRORS.md row 33 — the delegated `c2RaytoCircle` itself returns 0.
#[test]
fn err_row33_capsule_delegates_circle_miss() {
    // |yAp.x| < r and yAp.y < 0 -> delegate to cap A, pointing away from it.
    let (rc, out, touched) = diff_capsule(ray(1.0, -50.0, 0.0, -1.0, 10.0), VCAP, "row33");
    assert_eq!(rc, 0, "row33: the delegated circle cast must miss");
    assert!(touched, "row33: *out still carries the pre-set values");
    assert!(feq(out.t, 0.0), "row33: t stays 0, got {}", fshow(out.t));
    assert!(veq(out.n, v(0.0, 1.0)), "row33: n stays c2Norm(b-a)");
    // same for cap B
    let (rc2, _, _) = diff_capsule(ray(1.0, 60.0, 0.0, 1.0, 10.0), VCAP, "row33/capB");
    assert_eq!(rc2, 0);
    let mut rng = Rng::new(33);
    for _ in 0..4096 {
        let x = rng.unit(1.99);
        let far = 20.0 + rng.unit(200.0).abs();
        diff_capsule(ray(x, -far, 0.0, -1.0, 10.0), VCAP, "row33/sweepA");
        diff_capsule(ray(x, 10.0 + far, 0.0, 1.0, 10.0), VCAP, "row33/sweepB");
    }
}

/// ERRORS.md row 34 — `B.r < 0` gives an inverted local bounding box, so the
/// `c2AABBtoPoint` slab test can never succeed.
#[test]
fn err_row34_capsule_negative_radius() {
    let neg = C2Capsule {
        a: v(0.0, 0.0),
        b: v(0.0, 10.0),
        r: -2.0,
    };
    // origin dead centre in the shaft: with r > 0 this early-returns 1, with
    // r < 0 the inverted bb rejects it.
    let (rc, _, _) = diff_capsule(ray(0.0, 5.0, 1.0, 0.0, 10.0), neg, "row34");
    let (rc_pos, _, _) = diff_capsule(ray(0.0, 5.0, 1.0, 0.0, 10.0), VCAP, "row34/pos");
    assert_eq!(rc_pos, 1, "row34: sanity — the positive-radius capsule hits");
    let _ = rc;
    let mut rng = Rng::new(34);
    for _ in 0..4096 {
        diff_capsule(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            C2Capsule {
                a: v(0.0, 0.0),
                b: v(0.0, 10.0),
                r: -(0.001 + rng.unit(8.0).abs()),
            },
            "row34/sweep",
        );
    }
}

// ===========================================================================
// c2RaytoPoly — rows 35..42
// ===========================================================================

/// ERRORS.md row 35 — `den == 0 && num < 0`: ray parallel to a face, outside.
#[test]
fn err_row35_poly_parallel_outside() {
    let p = boxpoly(2.0, 1.0);
    // norms[1] = (0,1), verts[1] = (2,1): moving purely in x at y = 5 keeps
    // den == 0 for that face while num = 1 - 5 = -4 < 0.
    let (rc, _, touched) = diff_poly(
        ray(-5.0, 5.0, 1.0, 0.0, 20.0),
        &p,
        std::ptr::null(),
        "row35",
    );
    assert_eq!(rc, 0, "row35: expected 0");
    assert!(!touched, "row35: *out must be untouched");
    let mut rng = Rng::new(35);
    for _ in 0..4096 {
        let y = 1.0 + 0.001 + rng.unit(50.0).abs();
        let (rc, _, t) = diff_poly(
            ray(-5.0, y, 1.0, 0.0, 20.0),
            &p,
            std::ptr::null(),
            "row35/sweep",
        );
        assert_eq!(rc, 0);
        assert!(!t);
        let x = 2.0 + 0.001 + rng.unit(50.0).abs();
        let (rc2, _, t2) = diff_poly(
            ray(x, -5.0, 0.0, 1.0, 20.0),
            &p,
            std::ptr::null(),
            "row35/sweep2",
        );
        assert_eq!(rc2, 0);
        assert!(!t2);
    }
}

/// ERRORS.md row 36 — `hi < lo`: the interval collapses mid-loop.
#[test]
fn err_row36_poly_hi_lt_lo() {
    let p = boxpoly(2.0, 1.0);
    let (rc, _, touched) = diff_poly(
        ray(-5.0, 5.0, 1.0, -0.125, 20.0),
        &p,
        std::ptr::null(),
        "row36",
    );
    assert_eq!(rc, 0, "row36: expected 0");
    assert!(!touched);
    let mut rng = Rng::new(36);
    let mut misses = 0;
    for _ in 0..8192 {
        let (rc, _, t) = diff_poly(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            &p,
            std::ptr::null(),
            "row36/sweep",
        );
        if rc == 0 {
            assert!(!t, "row36: *out must be untouched on every miss");
            misses += 1;
        }
    }
    assert!(misses > 0, "row36: the randomized sweep produced no misses");
}

/// ERRORS.md row 37 — `index == ~0` after the loop (no entering face).
#[test]
fn err_row37_poly_index_unset() {
    let p = boxpoly(2.0, 1.0);
    // origin strictly inside: every `den < 0` branch has num >= lo*den, so
    // `index` is never assigned.
    let (rc, _, touched) = diff_poly(
        ray(0.0, 0.0, 1.0, 0.0, 20.0),
        &p,
        std::ptr::null(),
        "row37",
    );
    assert_eq!(rc, 0, "row37: expected 0 with the origin inside");
    assert!(!touched, "row37: *out must be untouched");
    let mut rng = Rng::new(37);
    for _ in 0..4096 {
        let ang = rng.unit(std::f32::consts::PI);
        let (rc, _, t) = diff_poly(
            C2Ray {
                p: v(rng.unit(1.9), rng.unit(0.9)),
                d: v(ang.cos(), ang.sin()),
                t: 100.0,
            },
            &p,
            std::ptr::null(),
            "row37/sweep",
        );
        assert_eq!(rc, 0);
        assert!(!t);
    }
}

/// ERRORS.md row 38 — `count <= 0`.
#[test]
fn err_row38_poly_count_zero_and_negative() {
    let mut rng = Rng::new(38);
    for count in [0i32, -1, -2, -8, -1000, i32::MIN] {
        let mut p = boxpoly(2.0, 1.0);
        p.count = count;
        let (rc, _, touched) = diff_poly(
            ray(-5.0, 0.0, 1.0, 0.0, 20.0),
            &p,
            std::ptr::null(),
            &format!("row38/count{count}"),
        );
        assert_eq!(rc, 0, "row38: count={count} must be rejected with 0");
        assert!(!touched, "row38: count={count}: *out must be untouched");
        for _ in 0..256 {
            let (rc, _, t) = diff_poly(
                ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
                &p,
                std::ptr::null(),
                "row38/sweep",
            );
            assert_eq!(rc, 0);
            assert!(!t);
        }
    }
}

/// ERRORS.md row 39 — `bx_ptr == NULL` substitutes `c2xIdentity()`.
#[test]
fn err_row39_poly_null_bx_is_identity() {
    let (c, r) = (c(), rs());
    let ident = C2x {
        p: v(0.0, 0.0),
        r: C2r { c: 1.0, s: 0.0 },
    };
    let mut rng = Rng::new(39);
    for _ in 0..4096 {
        let p = boxpoly(0.5 + rng.unit(3.0).abs(), 0.5 + rng.unit(3.0).abs());
        let a = ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom());
        diff_poly(a, &p, std::ptr::null(), "row39/null");
        diff_poly(a, &p, &ident, "row39/ident");
        for api in [c, r] {
            let mut o1 = poison(39);
            let mut o2 = poison(39);
            let r1 = unsafe { (api.c2RaytoPoly)(a, &p, std::ptr::null(), &mut o1) };
            let r2 = unsafe { (api.c2RaytoPoly)(a, &p, &ident, &mut o2) };
            assert_eq!(r1, r2, "{}: row39: NULL bx != c2xIdentity()", api.name);
            assert!(rceq(o1, o2), "{}: row39: NULL bx != c2xIdentity() out", api.name);
        }
        // c2CastRay must forward the NULL the same way
        diff_cast(
            a,
            (&p as *const C2Poly) as *const c_void,
            std::ptr::null(),
            C2_TYPE_POLY,
            "row39/cast-null",
        );
    }
}

/// ERRORS.md row 40 — non-unit `c2r` is accepted without validation.
#[test]
fn err_row40_poly_non_unit_rotation() {
    let mut rng = Rng::new(40);
    let p = boxpoly(2.0, 1.0);
    let bxs = [
        C2x { p: v(0.0, 0.0), r: C2r { c: 0.0, s: 0.0 } },
        C2x { p: v(0.0, 0.0), r: C2r { c: 2.0, s: 0.0 } },
        C2x { p: v(0.0, 0.0), r: C2r { c: 3.0, s: 4.0 } },
        C2x { p: v(1.0, -1.0), r: C2r { c: -5.0, s: 7.5 } },
        C2x { p: v(0.0, 0.0), r: C2r { c: 1.0e20, s: 1.0e20 } },
        C2x { p: v(0.0, 0.0), r: C2r { c: f32::INFINITY, s: 0.0 } },
        C2x { p: v(0.0, 0.0), r: C2r { c: f32::NAN, s: 0.0 } },
        C2x { p: v(f32::NAN, 0.0), r: C2r { c: 1.0, s: 0.0 } },
    ];
    for (i, bxv) in bxs.iter().enumerate() {
        for k in 0..16 {
            let ang = (k as f32) * std::f32::consts::TAU / 16.0;
            diff_poly(
                ray(ang.cos() * 8.0, ang.sin() * 8.0, -ang.cos(), -ang.sin(), 16.0),
                &p,
                bxv,
                &format!("row40/{i}"),
            );
        }
    }
    for _ in 0..4096 {
        let bxv = C2x {
            p: rng.geom_v(),
            r: C2r { c: rng.geom(), s: rng.geom() },
        };
        diff_poly(
            ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom()),
            &p,
            &bxv,
            "row40/sweep",
        );
    }
}

/// ERRORS.md row 41 — `count > 8` reads past `verts[8]` / `norms[8]`.
#[test]
fn err_row41_poly_count_gt_8_oob_read() {
    let (c, r) = (c(), rs());
    let words = (size_of::<C2Poly>() + 512) / 4;
    let mut rng = Rng::new(41);
    for count in [9i32, 10, 12, 16, 20, 24] {
        for trial in 0..32 {
            let mut buf: Vec<u32> = (0..words)
                .map(|i| rng.next_u32() ^ (i as u32).wrapping_mul(0xc2b2_ae35))
                .collect();
            let mut base = boxpoly(2.0, 1.0);
            base.count = count;
            unsafe {
                std::ptr::copy_nonoverlapping(
                    (&base as *const C2Poly) as *const u8,
                    buf.as_mut_ptr() as *mut u8,
                    size_of::<C2Poly>(),
                );
            }
            let pc = buf.as_ptr() as *const C2Poly;
            for k in 0..8 {
                let ang = (k as f32) * std::f32::consts::TAU / 8.0;
                let a = ray(ang.cos() * 8.0, ang.sin() * 8.0, -ang.cos(), -ang.sin(), 16.0);
                let mut oc = poison(0x1234_5678);
                let mut orr = poison(0x1234_5678);
                let rc = unsafe { (c.c2RaytoPoly)(a, pc, std::ptr::null(), &mut oc) };
                let rr = unsafe { (r.c2RaytoPoly)(a, pc, std::ptr::null(), &mut orr) };
                assert_eq!(
                    rc, rr,
                    "row41: count={count} trial={trial} k={k}: C={rc} RUST={rr}"
                );
                assert!(
                    rceq(oc, orr),
                    "row41: count={count} trial={trial} k={k}: out C={} RUST={}",
                    rcshow(oc),
                    rcshow(orr)
                );
            }
        }
    }
}

/// ERRORS.md row 42 — NaN makes `den == 0`, `den < 0`, `den > 0` and `hi < lo`
/// all false, so the loop runs to completion with `index == -1`.
#[test]
fn err_row42_poly_nan_inputs() {
    let p = boxpoly(2.0, 1.0);
    // NaN in the ray: every face comparison is false.
    let all_nan = C2Ray {
        p: v(f32::NAN, f32::NAN),
        d: v(f32::NAN, f32::NAN),
        t: f32::NAN,
    };
    let (rc, _, touched) = diff_poly(all_nan, &p, std::ptr::null(), "row42/all-nan");
    assert_eq!(rc, 0, "row42: an all-NaN ray must be rejected with 0");
    assert!(!touched, "row42: *out must be untouched");
    for &s in [f32::NAN, -f32::NAN].iter() {
        for slot in 0..5 {
            let mut a = ray(-5.0, 0.0, 1.0, 0.0, 20.0);
            match slot {
                0 => a.p.x = s,
                1 => a.p.y = s,
                2 => a.d.x = s,
                3 => a.d.y = s,
                _ => a.t = s,
            }
            diff_poly(a, &p, std::ptr::null(), "row42/ray-slot");
        }
        // NaN in the polygon data
        for slot in 0..4 {
            let mut q = boxpoly(2.0, 1.0);
            match slot {
                0 => q.norms[0].x = s,
                1 => q.norms[2].y = s,
                2 => q.verts[0].x = s,
                _ => q.verts[3].y = s,
            }
            diff_poly(ray(-5.0, 0.0, 1.0, 0.0, 20.0), &q, std::ptr::null(), "row42/poly-slot");
        }
    }
}

// ===========================================================================
// c2CastRay — rows 43..46
// ===========================================================================

/// ERRORS.md row 43 — out-of-range `C2_TYPE` value: the `switch` falls through.
#[test]
fn err_row43_castray_invalid_type_enum() {
    let circle = C2Circle { p: v(0.0, 0.0), r: 1.0 };
    let a = ray(-4.0, 0.0, 1.0, 0.0, 10.0);
    // sanity: the valid value hits
    let (ok, _, _) = diff_cast(
        a,
        (&circle as *const C2Circle) as *const c_void,
        std::ptr::null(),
        C2_TYPE_CIRCLE,
        "row43/valid",
    );
    assert_eq!(ok, 1, "row43: sanity — typeB=0 must hit this circle");
    for ty in [
        4u32,
        5,
        6,
        7,
        8,
        9,
        15,
        16,
        31,
        32,
        63,
        64,
        127,
        128,
        255,
        256,
        1024,
        0x7fff_fffe,
        0x7fff_ffff, // i32::MAX
        0x8000_0000, // i32::MIN
        0x8000_0001,
        0xffff_fffe, // -2
        0xffff_ffff, // -1
        0xdead_beef,
        0xcafe_babe,
    ] {
        let (rc, _, touched) = diff_cast(
            a,
            (&circle as *const C2Circle) as *const c_void,
            std::ptr::null(),
            ty,
            &format!("row43/{ty}"),
        );
        assert_eq!(rc, 0, "row43: typeB={ty} must return the sentinel 0");
        assert!(!touched, "row43: typeB={ty}: *out must be untouched");
        // B is never dereferenced on this path, so NULL is safe.
        let (rc2, _, t2) = diff_cast(a, std::ptr::null(), std::ptr::null(), ty, "row43/nullB");
        assert_eq!(rc2, 0);
        assert!(!t2);
    }
    // fuzz the whole 32-bit space
    let mut rng = Rng::new(43);
    for _ in 0..8192 {
        let ty = rng.next_u32();
        let (rc, _, touched) = diff_cast(a, std::ptr::null(), std::ptr::null(), ty, "row43/fuzz");
        if ty > 3 {
            assert_eq!(rc, 0, "row43/fuzz: typeB={ty}");
            assert!(!touched);
        }
    }
}

/// ERRORS.md row 44 — no type-tag validation: the same bytes are reinterpreted.
#[test]
fn err_row44_castray_type_layout_mismatch() {
    let mut rng = Rng::new(44);
    let words = (size_of::<C2Poly>() + 512) / 4;
    for trial in 0..64 {
        let mut buf: Vec<u32> = (0..words).map(|_| rng.geom().to_bits()).collect();
        buf[0] = (trial % 9) as u32; // keep `count` sane for the POLY view
        let ptr = buf.as_ptr() as *const c_void;
        for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE, C2_TYPE_POLY] {
            for k in 0..8 {
                let ang = (k as f32) * std::f32::consts::TAU / 8.0;
                diff_cast(
                    ray(ang.cos() * 7.0, ang.sin() * 7.0, -ang.cos(), -ang.sin(), 14.0),
                    ptr,
                    std::ptr::null(),
                    ty,
                    "row44",
                );
            }
        }
    }
}

/// ERRORS.md row 45 — `typeB == POLY` with `bx == NULL` is forwarded.
#[test]
fn err_row45_castray_poly_null_bx() {
    let (c, r) = (c(), rs());
    let mut rng = Rng::new(45);
    for _ in 0..4096 {
        let p = boxpoly(0.5 + rng.unit(3.0).abs(), 0.5 + rng.unit(3.0).abs());
        let a = ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom());
        diff_cast(
            a,
            (&p as *const C2Poly) as *const c_void,
            std::ptr::null(),
            C2_TYPE_POLY,
            "row45",
        );
        // must equal the direct call with a NULL bx
        for api in [c, r] {
            let mut o1 = poison(45);
            let mut o2 = poison(45);
            let r1 = unsafe { (api.c2RaytoPoly)(a, &p, std::ptr::null(), &mut o1) };
            let r2 = unsafe {
                (api.c2CastRay)(
                    a,
                    (&p as *const C2Poly) as *const c_void,
                    std::ptr::null(),
                    C2_TYPE_POLY,
                    &mut o2,
                )
            };
            assert_eq!(r1, r2, "{}: row45 mismatch", api.name);
            assert!(rceq(o1, o2), "{}: row45 out mismatch", api.name);
        }
    }
}

/// ERRORS.md row 46 — for the non-POLY types `bx` is ignored entirely.
#[test]
fn err_row46_castray_bx_ignored_for_non_poly() {
    let (c, r) = (c(), rs());
    let mut rng = Rng::new(46);
    for _ in 0..4096 {
        let a = ray(rng.geom(), rng.geom(), rng.geom(), rng.geom(), rng.geom());
        let garbage = C2x {
            p: rng.wild_v(),
            r: C2r { c: rng.wild(), s: rng.wild() },
        };
        let circle = C2Circle { p: rng.geom_v(), r: rng.geom() };
        let aabb = C2AABB { min: rng.geom_v(), max: rng.geom_v() };
        let capsule = C2Capsule { a: rng.geom_v(), b: rng.geom_v(), r: rng.geom() };
        let views: [(u32, *const c_void); 3] = [
            (C2_TYPE_CIRCLE, (&circle as *const C2Circle) as *const c_void),
            (C2_TYPE_AABB, (&aabb as *const C2AABB) as *const c_void),
            (C2_TYPE_CAPSULE, (&capsule as *const C2Capsule) as *const c_void),
        ];
        for (ty, ptr) in views {
            diff_cast(a, ptr, &garbage, ty, "row46/garbage-bx");
            for api in [c, r] {
                let mut o1 = poison(46);
                let mut o2 = poison(46);
                let r1 = unsafe { (api.c2CastRay)(a, ptr, std::ptr::null(), ty, &mut o1) };
                let r2 = unsafe { (api.c2CastRay)(a, ptr, &garbage, ty, &mut o2) };
                assert_eq!(r1, r2, "{}: row46: bx affected typeB={ty}", api.name);
                assert!(rceq(o1, o2), "{}: row46: bx affected *out for typeB={ty}", api.name);
            }
        }
    }
}

// ===========================================================================
// arithmetic guards — rows 47..51
// ===========================================================================

/// ERRORS.md row 47 — `c2Div` has no zero check.
#[test]
fn err_row47_div_by_zero() {
    let (c, r) = (c(), rs());
    for b in [0.0f32, -0.0] {
        for a in [
            v(1.0, 2.0),
            v(-1.0, -2.0),
            v(0.0, 0.0),
            v(-0.0, 0.0),
            v(f32::INFINITY, 1.0),
            v(f32::NAN, 1.0),
            v(f32::MAX, f32::MIN),
        ] {
            let ca = unsafe { (c.c2Div)(a, b) };
            let ra = unsafe { (r.c2Div)(a, b) };
            assert!(
                veq(ca, ra),
                "row47: c2Div({}, {}): C={} RUST={}",
                vshow(a),
                fshow(b),
                vshow(ca),
                vshow(ra)
            );
        }
    }
    // 1/0 == +inf, x * inf: sign and NaN behaviour must match exactly
    let ca = unsafe { (c.c2Div)(v(1.0, -1.0), 0.0) };
    let ra = unsafe { (r.c2Div)(v(1.0, -1.0), 0.0) };
    assert!(veq(ca, ra));
    assert!(
        ca.x.is_infinite() && ca.y.is_infinite(),
        "row47: expected infinities, got {}",
        vshow(ca)
    );
    let cz = unsafe { (c.c2Div)(v(0.0, 0.0), 0.0) };
    let rz = unsafe { (r.c2Div)(v(0.0, 0.0), 0.0) };
    assert!(veq(cz, rz));
    assert!(cz.x.is_nan(), "row47: 0 * inf must be NaN, got {}", fshow(cz.x));
}

/// ERRORS.md row 48 — `c2Norm((0,0))` divides by zero.
#[test]
fn err_row48_norm_zero_vector() {
    let (c, r) = (c(), rs());
    for a in [v(0.0, 0.0), v(-0.0, -0.0), v(0.0, -0.0), v(-0.0, 0.0)] {
        let ca = unsafe { (c.c2Norm)(a) };
        let ra = unsafe { (r.c2Norm)(a) };
        assert!(
            veq(ca, ra),
            "row48: c2Norm({}): C={} RUST={}",
            vshow(a),
            vshow(ca),
            vshow(ra)
        );
        assert!(
            ca.x.is_nan() && ca.y.is_nan(),
            "row48: expected NaN from 0/0, got {}",
            vshow(ca)
        );
    }
    // c2Len is 0 -> the exported c2Len must agree too
    for a in [v(0.0, 0.0), v(-0.0, -0.0)] {
        let cl = unsafe { (c.c2Len)(a) };
        let rl = unsafe { (r.c2Len)(a) };
        assert!(feq(cl, rl), "row48: c2Len({})", vshow(a));
        assert!(feq(cl, 0.0), "row48: c2Len(0) must be +0.0, got {}", fshow(cl));
    }
}

/// ERRORS.md row 49 — `c2Norm` of a vector containing `inf`.
#[test]
fn err_row49_norm_inf_vector() {
    let (c, r) = (c(), rs());
    for a in [
        v(f32::INFINITY, 0.0),
        v(f32::NEG_INFINITY, 0.0),
        v(0.0, f32::INFINITY),
        v(f32::INFINITY, f32::INFINITY),
        v(f32::INFINITY, f32::NEG_INFINITY),
        v(f32::INFINITY, 1.0),
        v(1.0, f32::NEG_INFINITY),
        v(f32::MAX, f32::MAX),
        v(1.0e30, 1.0e30),
    ] {
        let ca = unsafe { (c.c2Norm)(a) };
        let ra = unsafe { (r.c2Norm)(a) };
        assert!(
            veq(ca, ra),
            "row49: c2Norm({}): C={} RUST={}",
            vshow(a),
            vshow(ca),
            vshow(ra)
        );
    }
}

/// ERRORS.md row 50 — `c2Len` overflows to `+inf`.
#[test]
fn err_row50_len_overflow_to_inf() {
    let (c, r) = (c(), rs());
    for a in [
        v(1.0e30, 1.0e30),
        v(f32::MAX, 0.0),
        v(f32::MAX, f32::MAX),
        v(-f32::MAX, -f32::MAX),
        v(1.0e20, 1.0e20),
        v(f32::INFINITY, 0.0),
    ] {
        let cl = unsafe { (c.c2Len)(a) };
        let rl = unsafe { (r.c2Len)(a) };
        assert!(
            feq(cl, rl),
            "row50: c2Len({}): C={} RUST={}",
            vshow(a),
            fshow(cl),
            fshow(rl)
        );
    }
    let cl = unsafe { (c.c2Len)(v(f32::MAX, f32::MAX)) };
    assert!(
        cl.is_infinite() && cl > 0.0,
        "row50: expected +inf, got {}",
        fshow(cl)
    );
}

/// ERRORS.md row 51 — `sqrtf(NaN)` and `sqrtf` of a negative sum.
#[test]
fn err_row51_len_nan() {
    let (c, r) = (c(), rs());
    for a in [
        v(f32::NAN, 0.0),
        v(0.0, f32::NAN),
        v(f32::NAN, f32::NAN),
        v(-f32::NAN, 1.0),
        // inf * 0 inside the dot product
        v(f32::INFINITY, 0.0),
    ] {
        let cl = unsafe { (c.c2Len)(a) };
        let rl = unsafe { (r.c2Len)(a) };
        assert!(
            feq(cl, rl),
            "row51: c2Len({}): C={} RUST={}",
            vshow(a),
            fshow(cl),
            fshow(rl)
        );
    }
    assert!(
        unsafe { (c.c2Len)(v(f32::NAN, 0.0)) }.is_nan(),
        "row51: c2Len(NaN) must be NaN"
    );
    assert!(
        unsafe { (r.c2Len)(v(f32::NAN, 0.0)) }.is_nan(),
        "row51: the Rust c2Len(NaN) must be NaN too"
    );
}

// ===========================================================================
// row 52 — NULL `out` on the paths that never write
// ===========================================================================

/// ERRORS.md row 52 — `out == NULL` combined with an early `return 0` path.
/// The C code does not dereference `out` on those paths, so this is a real,
/// survivable input that both libraries must handle identically.
#[test]
fn err_row52_null_out_on_early_return() {
    let (c, r) = (c(), rs());
    let nul: *mut C2Raycast = std::ptr::null_mut();

    // c2RaytoCircle: disc < 0 (row 1), t < 0 (row 2), t > A.t (row 3), NaN (row 4)
    let circle = C2Circle { p: v(0.0, 0.0), r: 1.0 };
    for (name, a) in [
        ("disc<0", ray(-10.0, 100.0, 1.0, 0.0, 5.0)),
        ("t<0", ray(5.0, 0.0, 1.0, 0.0, 100.0)),
        ("t>A.t", ray(-10.0, 0.0, 1.0, 0.0, 1.0)),
        ("nan", ray(f32::NAN, 0.0, 1.0, 0.0, 10.0)),
    ] {
        let rc = unsafe { (c.c2RaytoCircle)(a, circle, nul) };
        let rr = unsafe { (r.c2RaytoCircle)(a, circle, nul) };
        assert_eq!(rc, 0, "row52/circle/{name}: expected 0 from C");
        assert_eq!(rc, rr, "row52/circle/{name}: C={rc} RUST={rr}");
    }

    // c2RaytoAABB: bb miss (row 12), SAT reject (row 13), no hit flags (row 14)
    let b = bx(-1.0, -1.0, 1.0, 1.0);
    for (name, a) in [
        ("bb-miss", ray(-100.0, -100.0, -1.0, -1.0, 1.0)),
        ("sat", ray(-4.0, 2.5, 1.0, 1.0, 20.0)),
        ("no-hit", ray(-4.0, 8.0, 1.0, 0.0, 1.0)),
    ] {
        let rc = unsafe { (c.c2RaytoAABB)(a, b, nul) };
        let rr = unsafe { (r.c2RaytoAABB)(a, b, nul) };
        assert_eq!(rc, 0, "row52/aabb/{name}: expected 0 from C");
        assert_eq!(rc, rr, "row52/aabb/{name}: C={rc} RUST={rr}");
    }

    // c2RaytoPoly: parallel-outside (row 35), hi<lo (row 36), index==-1 (row 37),
    // count<=0 (row 38)
    let p = boxpoly(2.0, 1.0);
    let mut empty = boxpoly(2.0, 1.0);
    empty.count = 0;
    let mut negative = boxpoly(2.0, 1.0);
    negative.count = -5;
    for (name, a, poly) in [
        ("parallel", ray(-5.0, 5.0, 1.0, 0.0, 20.0), &p),
        ("hi<lo", ray(-5.0, 5.0, 1.0, -0.125, 20.0), &p),
        ("index=-1", ray(0.0, 0.0, 1.0, 0.0, 20.0), &p),
        ("count=0", ray(-5.0, 0.0, 1.0, 0.0, 20.0), &empty),
        ("count<0", ray(-5.0, 0.0, 1.0, 0.0, 20.0), &negative),
    ] {
        let pc: *const C2Poly = poly;
        let rc = unsafe { (c.c2RaytoPoly)(a, pc, std::ptr::null(), nul) };
        let rr = unsafe { (r.c2RaytoPoly)(a, pc, std::ptr::null(), nul) };
        assert_eq!(rc, 0, "row52/poly/{name}: expected 0 from C");
        assert_eq!(rc, rr, "row52/poly/{name}: C={rc} RUST={rr}");
    }

    // c2CastRay with an invalid typeB never touches out or B (row 43)
    for ty in [4u32, 255, 0xffff_ffff] {
        let rc = unsafe { (c.c2CastRay)(ray(0.0, 0.0, 1.0, 0.0, 1.0), std::ptr::null(), std::ptr::null(), ty, nul) };
        let rr = unsafe { (r.c2CastRay)(ray(0.0, 0.0, 1.0, 0.0, 1.0), std::ptr::null(), std::ptr::null(), ty, nul) };
        assert_eq!(rc, 0, "row52/cast/{ty}: expected 0 from C");
        assert_eq!(rc, rr, "row52/cast/{ty}: C={rc} RUST={rr}");
    }

    // and via c2CastRay on the missing sub-shapes as well
    let rc = unsafe {
        (c.c2CastRay)(
            ray(-10.0, 100.0, 1.0, 0.0, 5.0),
            (&circle as *const C2Circle) as *const c_void,
            std::ptr::null(),
            C2_TYPE_CIRCLE,
            nul,
        )
    };
    let rr = unsafe {
        (r.c2CastRay)(
            ray(-10.0, 100.0, 1.0, 0.0, 5.0),
            (&circle as *const C2Circle) as *const c_void,
            std::ptr::null(),
            C2_TYPE_CIRCLE,
            nul,
        )
    };
    assert_eq!(rc, 0);
    assert_eq!(rc, rr, "row52/cast-circle: C={rc} RUST={rr}");
}

// ===========================================================================
// row 55 — the public entry point
// ===========================================================================

/// ERRORS.md row 55 — `poly_ray`'s bitmask return value.
#[test]
fn err_row55_poly_ray_bitmask() {
    let (c, r) = (c(), rs());
    for seed in [0u32, 1, 0xffff_ffff, 0xdead_beef, 0x5555_5555] {
        let mut c1c = poison(seed);
        let mut c2c = poison(seed ^ 0xaaaa_aaaa);
        let mut c1r = poison(seed);
        let mut c2r = poison(seed ^ 0xaaaa_aaaa);
        let rc = unsafe { (c.poly_ray)(&mut c1c, &mut c2c) };
        let rr = unsafe { (r.poly_ray)(&mut c1r, &mut c2r) };
        assert_eq!(rc, rr, "row55: poly_ray return C={rc} RUST={rr}");
        assert!(
            (0..=3).contains(&rc),
            "row55: the bitmask must be in 0..=3, got {rc}"
        );
        assert!(rceq(c1c, c1r), "row55: cast1 C={} RUST={}", rcshow(c1c), rcshow(c1r));
        assert!(rceq(c2c, c2r), "row55: cast2 C={} RUST={}", rcshow(c2c), rcshow(c2r));
        // Both casts miss on the hard-coded geometry, so neither out-param is
        // written and the mask is 0.  Assert the *measured* C behaviour.
        assert_eq!(rc, 0, "row55: measured C ground truth is 0");
        assert!(
            rceq(c1c, poison(seed)),
            "row55: the C library left cast1 untouched"
        );
        assert!(
            rceq(c1r, poison(seed)),
            "row55: the Rust library must leave cast1 untouched too, got {}",
            rcshow(c1r)
        );
        assert!(rceq(c2c, poison(seed ^ 0xaaaa_aaaa)));
        assert!(
            rceq(c2r, poison(seed ^ 0xaaaa_aaaa)),
            "row55: the Rust library must leave cast2 untouched too"
        );
    }
}
