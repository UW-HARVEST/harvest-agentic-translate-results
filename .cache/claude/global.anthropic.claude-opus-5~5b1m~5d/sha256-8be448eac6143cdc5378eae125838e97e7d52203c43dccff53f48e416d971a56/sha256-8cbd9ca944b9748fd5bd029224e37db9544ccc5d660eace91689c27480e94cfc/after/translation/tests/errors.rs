//! Phase C — one differential test per row of `ERRORS.md`, plus the generic
//! C-API boundaries (null pointers, out-of-range enum values across FFI,
//! one-step-past-range values).

mod common;
use common::*;
use std::ffi::{c_int, c_void};

fn v(x: f32, y: f32) -> C2v {
    C2v { x, y }
}

/// Runs `c2RaytoCircle` on both libraries and asserts the return code AND the
/// out-struct match bit-for-bit. Also asserts the C's write/no-write decision
/// (checked via the sentinel) equals the expected one.
fn expect_circle(d: &mut Diffs, label: &str, ray: C2Ray, c: C2Circle, want_ret: c_int, want_untouched: bool) {
    let l = libs();
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoCircle)(ray, c, &mut oc) };
    let rr = unsafe { (l.r.c2RaytoCircle)(ray, c, &mut or) };
    d.check(rc == rr && rceq(oc, or), || {
        format!("[{label}] C ret={rc} out={} | R ret={rr} out={}", rcs(oc), rcs(or))
    });
    d.check(rc == want_ret, || {
        format!("[{label}] C returned {rc}, ERRORS.md says {want_ret}")
    });
    d.check(rceq(oc, SENTINEL) == want_untouched, || {
        format!(
            "[{label}] C out-untouched={} but ERRORS.md says {want_untouched} (out={})",
            rceq(oc, SENTINEL),
            rcs(oc)
        )
    });
}

fn expect_aabb(d: &mut Diffs, label: &str, ray: C2Ray, b: C2AABB, want_ret: c_int, want_untouched: bool) {
    let l = libs();
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoAABB)(ray, b, &mut oc) };
    let rr = unsafe { (l.r.c2RaytoAABB)(ray, b, &mut or) };
    d.check(rc == rr && rceq(oc, or), || {
        format!("[{label}] C ret={rc} out={} | R ret={rr} out={}", rcs(oc), rcs(or))
    });
    d.check(rc == want_ret, || {
        format!("[{label}] C returned {rc}, ERRORS.md says {want_ret}")
    });
    d.check(rceq(oc, SENTINEL) == want_untouched, || {
        format!(
            "[{label}] C out-untouched={} but ERRORS.md says {want_untouched} (out={})",
            rceq(oc, SENTINEL),
            rcs(oc)
        )
    });
}

fn expect_capsule(d: &mut Diffs, label: &str, ray: C2Ray, b: C2Capsule, want_ret: c_int, want_untouched: bool) {
    let l = libs();
    let mut oc = SENTINEL;
    let mut or = SENTINEL;
    let rc = unsafe { (l.c.c2RaytoCapsule)(ray, b, &mut oc) };
    let rr = unsafe { (l.r.c2RaytoCapsule)(ray, b, &mut or) };
    d.check(rc == rr && rceq(oc, or), || {
        format!("[{label}] C ret={rc} out={} | R ret={rr} out={}", rcs(oc), rcs(or))
    });
    d.check(rc == want_ret, || {
        format!("[{label}] C returned {rc}, ERRORS.md says {want_ret}")
    });
    d.check(rceq(oc, SENTINEL) == want_untouched, || {
        format!(
            "[{label}] C out-untouched={} but ERRORS.md says {want_untouched} (out={})",
            rceq(oc, SENTINEL),
            rcs(oc)
        )
    });
}

// ===========================================================================
// ERRORS.md rows 1-5 — c2RaytoCircle rejections
// ===========================================================================

#[test]
fn errors_rows01_05_circle_rejections() {
    let mut d = Diffs::new("ERRORS rows 1-5: c2RaytoCircle");
    let c = C2Circle { p: v(0.0, 0.0), r: 5.0 };

    // row 1: disc < 0 (ray line misses the circle)
    expect_circle(
        &mut d,
        "row1 disc<0",
        C2Ray { p: v(-10.0, 20.0), d: v(1.0, 0.0), t: 100.0 },
        c,
        0,
        true,
    );
    // row 2: t < 0 (circle behind the ray origin)
    expect_circle(
        &mut d,
        "row2 t<0 (pointing away)",
        C2Ray { p: v(-10.0, 0.0), d: v(-1.0, 0.0), t: 100.0 },
        c,
        0,
        true,
    );
    expect_circle(
        &mut d,
        "row2 t<0 (origin inside)",
        C2Ray { p: v(0.0, 0.0), d: v(1.0, 0.0), t: 100.0 },
        c,
        0,
        true,
    );
    // row 3: t > A.t
    expect_circle(
        &mut d,
        "row3 t>A.t",
        C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: 1.0 },
        c,
        0,
        true,
    );
    // row 4: disc is NaN
    expect_circle(
        &mut d,
        "row4 disc NaN via NaN p",
        C2Ray { p: v(f32::NAN, 0.0), d: v(1.0, 0.0), t: 100.0 },
        c,
        0,
        true,
    );
    expect_circle(
        &mut d,
        "row4 disc NaN via NaN r",
        C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: 100.0 },
        C2Circle { p: v(0.0, 0.0), r: f32::NAN },
        0,
        true,
    );
    expect_circle(
        &mut d,
        "row4 disc NaN via inf-inf",
        C2Ray { p: v(f32::INFINITY, 0.0), d: v(1.0, 0.0), t: 100.0 },
        C2Circle { p: v(f32::INFINITY, 0.0), r: 5.0 },
        0,
        true,
    );
    expect_circle(
        &mut d,
        "row4 disc NaN via NaN d",
        C2Ray { p: v(-10.0, 0.0), d: v(f32::NAN, f32::NAN), t: 100.0 },
        c,
        0,
        true,
    );
    // row 5: A.t is NaN, otherwise a valid hit
    expect_circle(
        &mut d,
        "row5 A.t NaN",
        C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: f32::NAN },
        c,
        0,
        true,
    );
    d.finish();
}

// ===========================================================================
// ERRORS.md rows 6-10 — c2AABBtoAABB
// ===========================================================================

#[test]
fn errors_rows06_10_aabb_to_aabb() {
    let l = libs();
    let mut d = Diffs::new("ERRORS rows 6-10: c2AABBtoAABB");
    let a = C2AABB { min: v(0.0, 0.0), max: v(10.0, 10.0) };
    let cases: &[(&str, C2AABB, c_int)] = &[
        ("row6 d0 B left of A", C2AABB { min: v(-30.0, 0.0), max: v(-20.0, 10.0) }, 0),
        ("row7 d1 B right of A", C2AABB { min: v(20.0, 0.0), max: v(30.0, 10.0) }, 0),
        ("row8 d2 B below A", C2AABB { min: v(0.0, -30.0), max: v(10.0, -20.0) }, 0),
        ("row9 d3 B above A", C2AABB { min: v(0.0, 20.0), max: v(10.0, 30.0) }, 0),
        // row 10: NaN makes all four `<` false, so the C ACCEPTS
        ("row10 NaN accepts (min)", C2AABB { min: v(f32::NAN, f32::NAN), max: v(f32::NAN, f32::NAN) }, 1),
        // A single NaN coordinate only neutralises the comparisons that READ
        // it: here d0 (`B.max.x < A.min.x` = `-20 < 0`) is still true, so the C
        // rejects. Only an all-NaN corner set makes every `<` false.
        ("row10 one NaN coord still rejects via d0", C2AABB { min: v(f32::NAN, 0.0), max: v(-20.0, 10.0) }, 0),
        ("row10 NaN accepts (max all NaN)", C2AABB { min: v(0.0, 0.0), max: v(f32::NAN, f32::NAN) }, 1),
    ];
    for (label, b, want) in cases {
        let rc = (l.c.c2AABBtoAABB)(a, *b);
        let rr = (l.r.c2AABBtoAABB)(a, *b);
        d.check(rc == rr, || format!("[{label}] C {rc} vs R {rr}"));
        d.check(rc == *want, || format!("[{label}] C returned {rc}, ERRORS.md says {want}"));
    }
    d.finish();
}

// ===========================================================================
// ERRORS.md rows 11-15 — c2RaytoAABB rejections
// ===========================================================================

#[test]
fn errors_rows11_15_aabb_rejections() {
    let l = libs();
    let mut d = Diffs::new("ERRORS rows 11-15: c2RaytoAABB");
    let b = C2AABB { min: v(-5.0, -5.0), max: v(5.0, 5.0) };
    // row 11: ray bounding box misses B
    expect_aabb(
        &mut d,
        "row11 bbox miss",
        C2Ray { p: v(50.0, 50.0), d: v(1.0, 0.0), t: 1.0 },
        b,
        0,
        true,
    );
    // row 12: SAT reject d > 0
    expect_aabb(
        &mut d,
        "row12 SAT reject",
        C2Ray { p: v(-20.0, 9.0), d: (l.c.c2Norm)(v(1.0, -1.0)), t: 40.0 },
        b,
        0,
        true,
    );
    // row 13: `hit == 0`, i.e. all four `t > 1.0f`.
    //
    // This needs ALL FOUR `da` values to be NaN (`NaN <= 1.0f` is false), which
    // in turn needs a NaN in BOTH lanes of the ray origin: `da0`/`da1` come from
    // `p0.x` and `da2`/`da3` from `p0.y`, so a NaN in only one lane still leaves
    // the other axis with `da < 0` -> `t = 0` -> `hit = 1`.
    expect_aabb(
        &mut d,
        "row13 hit==0 (NaN in both lanes of A.p)",
        C2Ray { p: v(f32::NAN, f32::NAN), d: v(1.0, 0.0), t: 10.0 },
        b,
        0,
        true,
    );
    expect_aabb(
        &mut d,
        "row13 hit==0 (NaN A.p, NaN A.t)",
        C2Ray { p: v(f32::NAN, f32::NAN), d: v(1.0, 0.0), t: f32::NAN },
        b,
        0,
        true,
    );
    // row 15: NaN `A.d` / NaN `A.t` make `p1` NaN. Note this does NOT reach
    // `hit == 0`: `da0..da3` are computed from `p0`, which is still finite, so
    // the C reports a hit with a NaN `out->t`. Pure differential comparison —
    // no assumption about the outcome.
    for ray in [
        C2Ray { p: v(0.0, 0.0), d: v(f32::NAN, f32::NAN), t: 10.0 },
        C2Ray { p: v(0.0, 0.0), d: v(1.0, 0.0), t: f32::NAN },
        C2Ray { p: v(-20.0, 0.0), d: v(f32::INFINITY, 0.0), t: 1.0 },
        C2Ray { p: v(-20.0, 0.0), d: v(1.0, 0.0), t: f32::INFINITY },
    ] {
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe { (l.c.c2RaytoAABB)(ray, b, &mut oc) };
        let rr = unsafe { (l.r.c2RaytoAABB)(ray, b, &mut or) };
        d.check(rc == rr && rceq(oc, or), || {
            format!(
                "[row15 NaN p1] {} -> C ret={rc} out={} | R ret={rr} out={}",
                rays(ray), rcs(oc), rcs(or)
            )
        });
    }
    // row 14: inverted box (min > max) — no validation in C, just compare
    for ray in [
        C2Ray { p: v(-20.0, 0.0), d: v(1.0, 0.0), t: 100.0 },
        C2Ray { p: v(0.0, 0.0), d: v(0.0, 1.0), t: 100.0 },
        C2Ray { p: v(-20.0, -20.0), d: (l.c.c2Norm)(v(1.0, 1.0)), t: 100.0 },
    ] {
        let inv = C2AABB { min: v(5.0, 5.0), max: v(-5.0, -5.0) };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe { (l.c.c2RaytoAABB)(ray, inv, &mut oc) };
        let rr = unsafe { (l.r.c2RaytoAABB)(ray, inv, &mut or) };
        d.check(rc == rr && rceq(oc, or), || {
            format!(
                "[row14 inverted box] {} -> C ret={rc} out={} | R ret={rr} out={}",
                rays(ray),
                rcs(oc),
                rcs(or)
            )
        });
    }
    d.finish();
}

// ===========================================================================
// ERRORS.md rows 16-18 — c2RayToPlane_OneDimensional (static; reached through
// c2RaytoAABB, which is the only caller)
// ===========================================================================

#[test]
fn errors_rows16_18_ray_to_plane_static_helper() {
    let l = libs();
    let mut d = Diffs::new("ERRORS rows 16-18: c2RayToPlane_OneDimensional via c2RaytoAABB");
    let b = C2AABB { min: v(-5.0, -5.0), max: v(5.0, 5.0) };
    // row 16 (`da < 0` -> 0.0) is taken for the planes the origin is already
    // behind: any ray starting inside B hits it on all four axes.
    // row 17 (`d == 0`, ray parallel to a plane): an axis-parallel ray makes
    // da == db for the two planes perpendicular to the other axis.
    // row 18 (`da`/`db` NaN -> NaN result).
    let cases: &[(&str, C2Ray)] = &[
        ("row16 origin inside B (da<0 on every axis)", C2Ray { p: v(0.0, 0.0), d: v(1.0, 0.0), t: 100.0 }),
        ("row17 axis-parallel +x (da==db in y)", C2Ray { p: v(-20.0, 1.0), d: v(1.0, 0.0), t: 100.0 }),
        ("row17 axis-parallel +y (da==db in x)", C2Ray { p: v(1.0, -20.0), d: v(0.0, 1.0), t: 100.0 }),
        ("row17 zero-length ray (da==db on all axes)", C2Ray { p: v(0.0, 0.0), d: v(0.0, 0.0), t: 0.0 }),
        ("row17 A.t == 0 (p0 == p1)", C2Ray { p: v(0.0, 0.0), d: v(1.0, 0.0), t: 0.0 }),
        ("row18 NaN da/db", C2Ray { p: v(f32::NAN, 0.0), d: v(1.0, 0.0), t: 10.0 }),
        ("row18 inf p1", C2Ray { p: v(0.0, 0.0), d: v(f32::INFINITY, 0.0), t: 1.0 }),
    ];
    for (label, ray) in cases {
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe { (l.c.c2RaytoAABB)(*ray, b, &mut oc) };
        let rr = unsafe { (l.r.c2RaytoAABB)(*ray, b, &mut or) };
        d.check(rc == rr && rceq(oc, or), || {
            format!("[{label}] C ret={rc} out={} | R ret={rr} out={}", rcs(oc), rcs(or))
        });
    }
    d.finish();
}

// ===========================================================================
// ERRORS.md rows 19-23 — c2AABBtoPoint
// ===========================================================================

#[test]
fn errors_rows19_23_aabb_to_point() {
    let l = libs();
    let mut d = Diffs::new("ERRORS rows 19-23: c2AABBtoPoint");
    let b = C2AABB { min: v(0.0, 0.0), max: v(10.0, 10.0) };
    let cases: &[(&str, C2v, c_int)] = &[
        ("row19 d0 x<min.x", v(-1.0, 5.0), 0),
        ("row20 d1 y<min.y", v(5.0, -1.0), 0),
        ("row21 d2 x>max.x", v(11.0, 5.0), 0),
        ("row22 d3 y>max.y", v(5.0, 11.0), 0),
        ("row23 NaN accepts", v(f32::NAN, f32::NAN), 1),
        ("row23 NaN x accepts", v(f32::NAN, 5.0), 1),
        ("boundary: exactly min", v(0.0, 0.0), 1),
        ("boundary: exactly max", v(10.0, 10.0), 1),
        ("boundary: one ulp below min", v(-f32::from_bits(1), 0.0), 0),
        ("boundary: one ulp above max", v(f32::from_bits(10f32.to_bits() + 1), 10.0), 0),
    ];
    for (label, p, want) in cases {
        let rc = (l.c.c2AABBtoPoint)(b, *p);
        let rr = (l.r.c2AABBtoPoint)(b, *p);
        d.check(rc == rr, || format!("[{label}] {} C {rc} vs R {rr}", vs(*p)));
        d.check(rc == *want, || format!("[{label}] C returned {rc}, ERRORS.md says {want}"));
    }
    // row 29-related: inverted capsule_bb shape (min.x > max.x) always rejects
    let inv = C2AABB { min: v(3.0, 0.0), max: v(-3.0, 20.0) };
    for p in [v(0.0, 10.0), v(-1.0, 10.0), v(1.0, 10.0)] {
        let rc = (l.c.c2AABBtoPoint)(inv, p);
        let rr = (l.r.c2AABBtoPoint)(inv, p);
        d.check(rc == rr, || format!("[inverted bb] C {rc} vs R {rr}"));
        d.check(rc == 0, || format!("[inverted bb] expected reject, got {rc}"));
    }
    d.finish();
}

// ===========================================================================
// ERRORS.md rows 24-26 — c2CircleToPoint
// ===========================================================================

#[test]
fn errors_rows24_26_circle_to_point() {
    let l = libs();
    let mut d = Diffs::new("ERRORS rows 24-26: c2CircleToPoint");
    let c = |r: f32| C2Circle { p: v(0.0, 0.0), r };
    let cases: &[(&str, C2Circle, C2v, c_int)] = &[
        // row 24: strict `<`, so exactly on the rim is REJECTED
        ("row24 exactly on rim", c(5.0), v(5.0, 0.0), 0),
        ("row24 outside", c(5.0), v(6.0, 0.0), 0),
        ("row24 inside", c(5.0), v(4.9, 0.0), 1),
        ("row24 r==0 at centre", c(0.0), v(0.0, 0.0), 0),
        // row 25: negative radius behaves like |r|
        ("row25 r<0 inside |r|", c(-5.0), v(1.0, 1.0), 1),
        ("row25 r<0 outside |r|", c(-5.0), v(6.0, 0.0), 0),
        // row 26: NaN radius / NaN point
        ("row26 NaN r", c(f32::NAN), v(0.0, 0.0), 0),
        ("row26 NaN point", c(5.0), v(f32::NAN, f32::NAN), 0),
        ("row26 inf point", c(5.0), v(f32::INFINITY, 0.0), 0),
        // r == inf does NOT accept a far-away point: `d2` overflows to +inf too,
        // and `inf < inf` is false (strict `<`).
        ("r==inf, d2 overflows -> inf<inf is false", c(f32::INFINITY), v(1e30, 1e30), 0),
        ("r==inf with a finite d2 accepts", c(f32::INFINITY), v(1.0, 1.0), 1),
        // one step past the valid range: 1 ulp inside / outside the rim
        ("1 ulp inside rim", c(5.0), v(f32::from_bits(5f32.to_bits() - 1), 0.0), 1),
        ("1 ulp outside rim", c(5.0), v(f32::from_bits(5f32.to_bits() + 1), 0.0), 0),
    ];
    for (label, circ, p, want) in cases {
        let rc = (l.c.c2CircleToPoint)(*circ, *p);
        let rr = (l.r.c2CircleToPoint)(*circ, *p);
        d.check(rc == rr, || format!("[{label}] C {rc} vs R {rr}"));
        d.check(rc == *want, || format!("[{label}] C returned {rc}, ERRORS.md says {want}"));
    }
    d.finish();
}

// ===========================================================================
// ERRORS.md rows 27-31 — c2RaytoCapsule rejections
// ===========================================================================

#[test]
fn errors_rows27_31_capsule_rejections() {
    let l = libs();
    let mut d = Diffs::new("ERRORS rows 27-31: c2RaytoCapsule");
    let cap = C2Capsule { a: v(0.0, 0.0), b: v(0.0, 20.0), r: 3.0 };

    // row 27: fall-through miss — `out` HAS already been written (t=0,
    // n=c2Norm(cap_n)), so `want_untouched` is FALSE. That asymmetry with every
    // other miss path is exactly what this row is about.
    expect_capsule(
        &mut d,
        "row27 fall-through miss (out IS written)",
        C2Ray { p: v(30.0, 10.0), d: v(1.0, 0.0), t: 5.0 },
        cap,
        0,
        false,
    );
    expect_capsule(
        &mut d,
        "row27 fall-through miss, other side",
        C2Ray { p: v(-30.0, 10.0), d: v(-1.0, 0.0), t: 5.0 },
        cap,
        0,
        false,
    );
    // and confirm the exact values the C left there
    {
        let mut oc = SENTINEL;
        let ray = C2Ray { p: v(30.0, 10.0), d: v(1.0, 0.0), t: 5.0 };
        let rc = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut oc) };
        let want_n = (l.c.c2Norm)((l.c.c2Sub)(cap.b, cap.a));
        d.check(rc == 0 && feq(oc.t, 0.0) && veq(oc.n, want_n), || {
            format!("[row27] expected out = {{t:0, n:c2Norm(b-a)={}}}, got {}", vs(want_n), rcs(oc))
        });
    }

    // row 28: degenerate capsule a == b (c2Norm of the zero vector -> NaN)
    for r in [3.0f32, 0.0, -3.0] {
        let deg = C2Capsule { a: v(1.0, 2.0), b: v(1.0, 2.0), r };
        for ray in [
            C2Ray { p: v(-20.0, 2.0), d: v(1.0, 0.0), t: 60.0 },
            C2Ray { p: v(1.0, 2.0), d: v(1.0, 0.0), t: 60.0 },
            C2Ray { p: v(1.0, 2.0), d: v(0.0, 0.0), t: 0.0 },
        ] {
            let mut oc = SENTINEL;
            let mut or = SENTINEL;
            let rc = unsafe { (l.c.c2RaytoCapsule)(ray, deg, &mut oc) };
            let rr = unsafe { (l.r.c2RaytoCapsule)(ray, deg, &mut or) };
            d.check(rc == rr && rceq(oc, or), || {
                format!(
                    "[row28 a==b] {} {} -> C ret={rc} out={} | R ret={rr} out={}",
                    rays(ray),
                    caps(deg),
                    rcs(oc),
                    rcs(or)
                )
            });
        }
    }

    // row 29: negative radius -> inverted capsule_bb
    for ray in [
        C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: 60.0 },
        C2Ray { p: v(0.0, 10.0), d: v(1.0, 0.0), t: 60.0 },
        C2Ray { p: v(0.0, -10.0), d: v(0.0, 1.0), t: 60.0 },
    ] {
        let neg = C2Capsule { a: v(0.0, 0.0), b: v(0.0, 20.0), r: -3.0 };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe { (l.c.c2RaytoCapsule)(ray, neg, &mut oc) };
        let rr = unsafe { (l.r.c2RaytoCapsule)(ray, neg, &mut or) };
        d.check(rc == rr && rceq(oc, or), || {
            format!("[row29 r<0] {} -> C ret={rc} out={} | R ret={rr} out={}", rays(ray), rcs(oc), rcs(or))
        });
    }

    // row 30: yAe.x == yAp.x  ->  d == 0, no guard, division by zero at L278
    for ray in [
        C2Ray { p: v(-20.0, 10.0), d: v(0.0, 1.0), t: 60.0 },   // motion purely along the axis
        C2Ray { p: v(-20.0, 10.0), d: v(0.0, -1.0), t: 60.0 },
        C2Ray { p: v(-20.0, 10.0), d: v(0.0, 0.0), t: 60.0 },   // zero direction
        C2Ray { p: v(-20.0, 10.0), d: v(1.0, 0.0), t: 0.0 },    // zero length
        C2Ray { p: v(20.0, 10.0), d: v(0.0, 1.0), t: 60.0 },    // other side (c < 0)
    ] {
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut oc) };
        let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut or) };
        d.check(rc == rr && rceq(oc, or), || {
            format!("[row30 d==0] {} -> C ret={rc} out={} | R ret={rr} out={}", rays(ray), rcs(oc), rcs(or))
        });
    }

    // row 31: the delegate c2RaytoCircle rejects; `out` still holds the
    // L243-244 values (so NOT the sentinel), and the return is 0.
    expect_capsule(
        &mut d,
        "row31 delegate c2RaytoCircle rejects (t>A.t)",
        C2Ray { p: v(1.0, -40.0), d: v(0.0, 1.0), t: 1.0 },
        cap,
        0,
        false,
    );
    expect_capsule(
        &mut d,
        "row31 delegate rejects (pointing away)",
        C2Ray { p: v(1.0, -40.0), d: v(0.0, -1.0), t: 60.0 },
        cap,
        0,
        false,
    );
    d.finish();
}

// ===========================================================================
// ERRORS.md rows 32-33 — c2CastRay: dead code, and the out-of-range enum
// ===========================================================================

/// Row 32: the `return 0;` after `case C2_TYPE_CAPSULE:` is dead code — the
/// preceding `return c2RaytoCapsule(...)` always fires. Proven by showing that
/// `c2CastRay(.., C2_TYPE_CAPSULE, ..)` is always exactly equal to a direct
/// `c2RaytoCapsule` call (never 0-when-it-should-hit).
#[test]
fn errors_row32_cast_ray_capsule_dead_code_unreachable() {
    let l = libs();
    let mut rng = Rng::new(0xE440_0032);
    let mut d = Diffs::new("ERRORS row 32: c2CastRay CAPSULE dead `return 0` is unreachable");
    let mut hits = 0usize;
    for _ in 0..20_000 {
        let ray = C2Ray { p: rng.geo_v(40.0), d: (l.c.c2Norm)(rng.geo_v(1.0)), t: rng.range(0.0, 60.0) };
        let a = rng.geo_v(25.0);
        let capsule = C2Capsule { a, b: (l.c.c2Add)(a, rng.geo_v(25.0)), r: rng.range(0.05, 8.0) };
        let mut o_direct = SENTINEL;
        let mut o_cast = SENTINEL;
        let mut o_rust = SENTINEL;
        let r_direct = unsafe { (l.c.c2RaytoCapsule)(ray, capsule, &mut o_direct) };
        let r_cast = unsafe {
            (l.c.c2CastRay)(ray, &capsule as *const C2Capsule as *const c_void, C2_TYPE_CAPSULE, &mut o_cast)
        };
        let r_rust = unsafe {
            (l.r.c2CastRay)(ray, &capsule as *const C2Capsule as *const c_void, C2_TYPE_CAPSULE, &mut o_rust)
        };
        if r_direct != 0 {
            hits += 1;
        }
        d.check(r_direct == r_cast && rceq(o_direct, o_cast), || {
            "c2CastRay(CAPSULE) diverged from c2RaytoCapsule inside the C library".to_string()
        });
        d.check(r_cast == r_rust && rceq(o_cast, o_rust), || {
            format!("c2CastRay(CAPSULE) C ret={r_cast} out={} | R ret={r_rust} out={}", rcs(o_cast), rcs(o_rust))
        });
    }
    assert!(hits > 100, "expected many capsule hits, got {hits}");
    d.finish();
}

/// Row 33: `typeB` with no valid enum variant. C enums accept any `int`, and
/// the C `switch` has no `default:` and the function has no trailing `return`,
/// so control falls off the end of a non-`void` function — undefined behaviour,
/// returning whatever the caller left in `%eax`.
///
/// What IS observable and well-defined is asserted here:
///  * neither library crashes,
///  * neither library writes through `out` (compared bit-for-bit against the
///    pre-call sentinel).
/// The return value is recorded but not required to be equal, because no value
/// can match "whatever happened to be in a register".
#[test]
fn errors_row33_cast_ray_out_of_range_enum() {
    let l = libs();
    let mut d = Diffs::new("ERRORS row 33: c2CastRay out-of-range C2_TYPE");
    let ray = C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: 100.0 };
    let circle = C2Circle { p: v(0.0, 0.0), r: 5.0 };
    let bad: &[c_int] = &[
        3,              // one past the last valid variant
        4,
        -1,             // one before the first
        -2,
        100,
        i32::MAX,
        i32::MIN,
        0x7fff_fffe,
        i32::MIN + 1,
    ];
    for &t in bad {
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe {
            (l.c.c2CastRay)(ray, &circle as *const C2Circle as *const c_void, t, &mut oc)
        };
        let rr = unsafe {
            (l.r.c2CastRay)(ray, &circle as *const C2Circle as *const c_void, t, &mut or)
        };
        eprintln!("  typeB={t}: C ret={rc}, Rust ret={rr} (UB: value unspecified)");
        // The well-defined part: `out` is untouched by BOTH.
        d.check(rceq(oc, SENTINEL), || {
            format!("typeB={t}: C wrote through out ({}), expected untouched", rcs(oc))
        });
        d.check(rceq(or, SENTINEL), || {
            format!("typeB={t}: Rust wrote through out ({}), expected untouched", rcs(or))
        });
        d.check(rceq(oc, or), || {
            format!("typeB={t}: out differs, C={} R={}", rcs(oc), rcs(or))
        });
    }
    // Valid variants immediately adjacent to the invalid range must still work,
    // and must agree between the two libraries.
    for t in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let (rc, rr) = match t {
            0 => (
                unsafe { (l.c.c2CastRay)(ray, &circle as *const C2Circle as *const c_void, t, &mut oc) },
                unsafe { (l.r.c2CastRay)(ray, &circle as *const C2Circle as *const c_void, t, &mut or) },
            ),
            1 => {
                let b = C2AABB { min: v(-5.0, -5.0), max: v(5.0, 5.0) };
                (
                    unsafe { (l.c.c2CastRay)(ray, &b as *const C2AABB as *const c_void, t, &mut oc) },
                    unsafe { (l.r.c2CastRay)(ray, &b as *const C2AABB as *const c_void, t, &mut or) },
                )
            }
            _ => {
                let cp = C2Capsule { a: v(0.0, -5.0), b: v(0.0, 5.0), r: 3.0 };
                (
                    unsafe { (l.c.c2CastRay)(ray, &cp as *const C2Capsule as *const c_void, t, &mut oc) },
                    unsafe { (l.r.c2CastRay)(ray, &cp as *const C2Capsule as *const c_void, t, &mut or) },
                )
            }
        };
        d.check(rc == rr && rceq(oc, or), || {
            format!("typeB={t} (valid): C ret={rc} out={} | R ret={rr} out={}", rcs(oc), rcs(or))
        });
        d.check(rc == 1, || format!("typeB={t} (valid): expected a hit, got {rc}"));
    }
    d.finish();
}

// ===========================================================================
// ERRORS.md rows 34-38 — degenerate scalar arithmetic
// ===========================================================================

#[test]
fn errors_rows34_38_degenerate_scalar_arithmetic() {
    let l = libs();
    let mut d = Diffs::new("ERRORS rows 34-38: c2Div / c2Norm / c2Len degeneracies");
    // rows 34-35: c2Div by +0.0 / -0.0 / inf / subnormal
    for s in [0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY, 1e-45, -1e-45, f32::NAN] {
        for u in [v(3.0, 4.0), v(0.0, 0.0), v(-0.0, 0.0), v(f32::INFINITY, 1.0), v(1e38, -1e38)] {
            let a = (l.c.c2Div)(u, s);
            let b = (l.r.c2Div)(u, s);
            d.check(veq(a, b), || format!("c2Div({}, {}) -> C {} vs R {}", vs(u), fs(s), vs(a), vs(b)));
        }
    }
    // row 34: c2Norm of the zero vector -> 1/0 = inf, 0*inf = NaN
    for u in [v(0.0, 0.0), v(-0.0, -0.0), v(0.0, -0.0), v(1e-45, 0.0)] {
        let a = (l.c.c2Norm)(u);
        let b = (l.r.c2Norm)(u);
        d.check(veq(a, b), || format!("c2Norm({}) -> C {} vs R {}", vs(u), vs(a), vs(b)));
        // and the C really does produce a non-finite result here
        if u.x == 0.0 && u.y == 0.0 {
            d.check(a.x.is_nan() && a.y.is_nan(), || {
                format!("expected NaN from c2Norm(zero), C gave {}", vs(a))
            });
        }
    }
    // rows 36-38: c2Len overflow / NaN / negative-argument sqrtf
    for u in [
        v(1e38, 1e38),                 // c2Dot overflows to +inf -> sqrtf(inf) = inf
        v(3.4028235e38, 3.4028235e38),
        v(f32::INFINITY, 0.0),
        v(f32::INFINITY, f32::NAN),
        v(f32::INFINITY, f32::NEG_INFINITY),
        v(f32::NAN, 0.0),
        v(-f32::NAN, 0.0),
        v(1e-45, 1e-45),               // c2Dot underflows to 0
    ] {
        let a = (l.c.c2Len)(u);
        let b = (l.r.c2Len)(u);
        d.check(feq(a, b), || format!("c2Len({}) -> C {} vs R {}", vs(u), fs(a), fs(b)));
    }
    // row 38: sqrtf of a negative would give the -NaN indefinite; the only way
    // c2Dot(a,a) goes negative is via an infinite product of opposite signs.
    {
        let u = v(f32::INFINITY, f32::INFINITY);
        let a = (l.c.c2Len)(u);
        let b = (l.r.c2Len)(u);
        d.check(feq(a, b), || format!("c2Len(inf,inf) -> C {} vs R {}", fs(a), fs(b)));
    }
    d.finish();
}

// ===========================================================================
// ERRORS.md rows 39-41 — gen_ray degeneracies and out-pointer contract
// ===========================================================================

#[test]
fn errors_rows39_41_gen_ray() {
    let l = libs();
    let mut d = Diffs::new("ERRORS rows 39-41: gen_ray");
    let run = |d: &mut Diffs, label: &str, p: [f32; 16], want_ret: Option<c_int>| {
        let mut c1 = SENTINEL;
        let mut c2 = SENTINEL;
        let mut c3 = SENTINEL;
        let mut r1 = SENTINEL;
        let mut r2 = SENTINEL;
        let mut r3 = SENTINEL;
        let rc = unsafe {
            (l.c.gen_ray)(
                &mut c1, &mut c2, &mut c3, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8],
                p[9], p[10], p[11], p[12], p[13], p[14], p[15],
            )
        };
        let rr = unsafe {
            (l.r.gen_ray)(
                &mut r1, &mut r2, &mut r3, p[0], p[1], p[2], p[3], p[4], p[5], p[6], p[7], p[8],
                p[9], p[10], p[11], p[12], p[13], p[14], p[15],
            )
        };
        d.check(rc == rr && rceq(c1, r1) && rceq(c2, r2) && rceq(c3, r3), || {
            format!(
                "[{label}] C ret={rc} {} {} {} | R ret={rr} {} {} {}",
                rcs(c1), rcs(c2), rcs(c3), rcs(r1), rcs(r2), rcs(r3)
            )
        });
        if let Some(w) = want_ret {
            d.check(rc == w, || format!("[{label}] C returned {rc}, ERRORS.md says {w}"));
        }
        // row 41: cast2 is ALWAYS written (c2RaytoCapsule L243-244), unlike
        // cast1/cast3 which are only written on a hit.
        d.check(!rceq(c2, SENTINEL), || {
            format!("[{label}] cast2 was NOT written; the C always writes it")
        });
        rc
    };
    // row 39: mp == ray.p  ->  c2Norm of the zero vector  ->  NaN ray.d and NaN
    // ray.t. The circle and capsule casts then reject, but the AABB cast still
    // reports a HIT: `da0..da3` in c2RaytoAABB are computed from `p0` (= the
    // still-finite ray origin), not from the NaN `p1`, so `da < 0` holds on both
    // axes, every `t` is 0 and `hit` is 1. The result is therefore 4 (bit 2),
    // not 0, and `cast3->t` is `0 * NaN` = NaN.
    run(
        &mut d,
        "row39 mp == ray.p",
        [1.0, 2.0, 1.0, 2.0, 0.0, 0.0, 4.0, 3.0, -6.0, 3.0, 6.0, 1.5, 6.0, -2.0, 9.0, 2.0],
        Some(4),
    );
    // row 40: everything far away -> all miss
    run(
        &mut d,
        "row40 all miss",
        [20.0, 0.0, -20.0, 0.0, 9e3, 9e3, 4.0, 9e3, -6.0, 9e3, 6.0, 1.5, 9e3, -2.0, 9e3 + 3.0, 2.0],
        Some(0),
    );
    // row 41 (positive control): all three hit -> ret == 7, all three written
    let ret = run(
        &mut d,
        "row41 all three hit",
        [20.0, 0.0, -20.0, 0.0, 0.0, 0.0, 4.0, 3.0, -6.0, 3.0, 6.0, 1.5, 6.0, -2.0, 9.0, 2.0],
        None,
    );
    eprintln!("  row41 all-hit scenario returned {ret}");
    // NaN / inf injected into each parameter
    for i in 0..16 {
        for s in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let mut p = [20.0f32, 0.0, -20.0, 0.0, 0.0, 0.0, 4.0, 3.0, -6.0, 3.0, 6.0, 1.5, 6.0, -2.0, 9.0, 2.0];
            p[i] = s;
            run(&mut d, "row39/40 special injected", p, None);
        }
    }
    d.finish();
}

// ===========================================================================
// ERRORS.md row 42 + generic C-API boundaries: NULL out pointers
// ===========================================================================

/// Row 42: the C has no NULL check. On the paths where it provably does NOT
/// write through `out`, passing NULL is harmless, and both libraries must
/// behave identically (same return, no crash). The paths where the C *does*
/// write are deliberately NOT called with NULL — that is a guaranteed SIGSEGV
/// in both, which is the identical (non-)behaviour by construction.
#[test]
fn errors_row42_null_out_pointer_on_nonwriting_paths() {
    let l = libs();
    let mut d = Diffs::new("ERRORS row 42: NULL out on non-writing paths");
    let nul: *mut C2Raycast = std::ptr::null_mut();

    // c2RaytoCircle: every rejection path leaves `out` untouched (rows 1-5).
    let circle = C2Circle { p: v(0.0, 0.0), r: 5.0 };
    for (label, ray) in [
        ("disc<0", C2Ray { p: v(-10.0, 20.0), d: v(1.0, 0.0), t: 100.0 }),
        ("t<0", C2Ray { p: v(-10.0, 0.0), d: v(-1.0, 0.0), t: 100.0 }),
        ("t>A.t", C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: 1.0 }),
        ("disc NaN", C2Ray { p: v(f32::NAN, 0.0), d: v(1.0, 0.0), t: 100.0 }),
    ] {
        let rc = unsafe { (l.c.c2RaytoCircle)(ray, circle, nul) };
        let rr = unsafe { (l.r.c2RaytoCircle)(ray, circle, nul) };
        d.check(rc == rr && rc == 0, || {
            format!("[c2RaytoCircle NULL {label}] C {rc} vs R {rr}")
        });
    }

    // c2RaytoAABB: every rejection path leaves `out` untouched (rows 11-13).
    let b = C2AABB { min: v(-5.0, -5.0), max: v(5.0, 5.0) };
    for (label, ray) in [
        ("bbox miss", C2Ray { p: v(50.0, 50.0), d: v(1.0, 0.0), t: 1.0 }),
        ("SAT reject", C2Ray { p: v(-20.0, 9.0), d: (l.c.c2Norm)(v(1.0, -1.0)), t: 40.0 }),
        // `hit == 0` needs a NaN in BOTH lanes of the origin (see row 13).
        ("hit==0 via NaN in both lanes of A.p", C2Ray { p: v(f32::NAN, f32::NAN), d: v(1.0, 0.0), t: 10.0 }),
    ] {
        let rc = unsafe { (l.c.c2RaytoAABB)(ray, b, nul) };
        let rr = unsafe { (l.r.c2RaytoAABB)(ray, b, nul) };
        d.check(rc == rr && rc == 0, || {
            format!("[c2RaytoAABB NULL {label}] C {rc} vs R {rr}")
        });
    }

    // c2CastRay with an out-of-range typeB never touches `out` (row 33), so a
    // NULL `out` — and even a NULL shape pointer — is safe in both.
    for t in [3, -1, i32::MAX, i32::MIN] {
        let rc = unsafe { (l.c.c2CastRay)(C2Ray::default(), std::ptr::null(), t, nul) };
        let rr = unsafe { (l.r.c2CastRay)(C2Ray::default(), std::ptr::null(), t, nul) };
        eprintln!("  c2CastRay(NULL,{t},NULL): C={rc} R={rr} (UB return, out untouched)");
        d.check(true, String::new);
    }

    // c2CastRay dispatching to circle/AABB on a rejecting path: `out` untouched.
    let rc = unsafe {
        (l.c.c2CastRay)(
            C2Ray { p: v(-10.0, 20.0), d: v(1.0, 0.0), t: 100.0 },
            &circle as *const C2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            nul,
        )
    };
    let rr = unsafe {
        (l.r.c2CastRay)(
            C2Ray { p: v(-10.0, 20.0), d: v(1.0, 0.0), t: 100.0 },
            &circle as *const C2Circle as *const c_void,
            C2_TYPE_CIRCLE,
            nul,
        )
    };
    d.check(rc == rr && rc == 0, || format!("[c2CastRay CIRCLE NULL miss] C {rc} vs R {rr}"));
    d.finish();
}

// ===========================================================================
// Generic boundaries: values one step past every documented range
// ===========================================================================

#[test]
fn errors_generic_one_ulp_boundaries() {
    let l = libs();
    let mut d = Diffs::new("generic: one-ulp boundaries on every comparison");
    let step = |x: f32, up: bool| {
        let b = x.to_bits();
        f32::from_bits(if up { b + 1 } else { b - 1 })
    };
    let circle = C2Circle { p: v(0.0, 0.0), r: 5.0 };

    // c2RaytoCircle: t exactly == A.t, and one ulp either side.
    // The exact hit at x=-5 from p=(-10,0) gives t == 5.
    for at in [5.0f32, step(5.0, false), step(5.0, true), 0.0, step(0.0, true)] {
        let ray = C2Ray { p: v(-10.0, 0.0), d: v(1.0, 0.0), t: at };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe { (l.c.c2RaytoCircle)(ray, circle, &mut oc) };
        let rr = unsafe { (l.r.c2RaytoCircle)(ray, circle, &mut or) };
        d.check(rc == rr && rceq(oc, or), || {
            format!("[circle A.t={}] C ret={rc} out={} | R ret={rr} out={}", fs(at), rcs(oc), rcs(or))
        });
    }
    // c2RaytoCircle: tangent exactly (disc == 0) and one ulp either side.
    for y in [5.0f32, step(5.0, false), step(5.0, true)] {
        let ray = C2Ray { p: v(-10.0, y), d: v(1.0, 0.0), t: 100.0 };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe { (l.c.c2RaytoCircle)(ray, circle, &mut oc) };
        let rr = unsafe { (l.r.c2RaytoCircle)(ray, circle, &mut or) };
        d.check(rc == rr && rceq(oc, or), || {
            format!("[circle tangent y={}] C ret={rc} out={} | R ret={rr} out={}", fs(y), rcs(oc), rcs(or))
        });
    }
    // c2AABBtoAABB / c2AABBtoPoint: exactly-touching and one ulp either side.
    let a = C2AABB { min: v(0.0, 0.0), max: v(10.0, 10.0) };
    for x in [0.0f32, step(0.0, true), -f32::from_bits(1), 10.0, step(10.0, true), step(10.0, false)] {
        let bb = C2AABB { min: v(x - 10.0, 0.0), max: v(x, 10.0) };
        let rc = (l.c.c2AABBtoAABB)(a, bb);
        let rr = (l.r.c2AABBtoAABB)(a, bb);
        d.check(rc == rr, || format!("[aabb-aabb x={}] C {rc} vs R {rr}", fs(x)));
        let rc = (l.c.c2AABBtoPoint)(a, v(x, 5.0));
        let rr = (l.r.c2AABBtoPoint)(a, v(x, 5.0));
        d.check(rc == rr, || format!("[aabb-point x={}] C {rc} vs R {rr}", fs(x)));
    }
    // c2CircleToPoint: exactly on the rim and one ulp either side, for many radii.
    for r in [0.0f32, 1e-45, 1.0, 5.0, 1e30, 3.4028235e38, f32::INFINITY] {
        for x in [r, step(r, true), if r == 0.0 { -0.0 } else { step(r, false) }] {
            let rc = (l.c.c2CircleToPoint)(C2Circle { p: v(0.0, 0.0), r }, v(x, 0.0));
            let rr = (l.r.c2CircleToPoint)(C2Circle { p: v(0.0, 0.0), r }, v(x, 0.0));
            d.check(rc == rr, || {
                format!("[circle-point r={} x={}] C {rc} vs R {rr}", fs(r), fs(x))
            });
        }
    }
    // c2RaytoCapsule: |yAp.x| exactly == B.r and one ulp either side (the
    // `abs(yAp.x) < B.r` and `min(..) < B.r` boundaries).
    let cap = C2Capsule { a: v(0.0, 0.0), b: v(0.0, 20.0), r: 3.0 };
    for x in [3.0f32, step(3.0, false), step(3.0, true), -3.0, step(-3.0, true), step(-3.0, false)] {
        for ray in [
            C2Ray { p: v(x, -10.0), d: v(0.0, 1.0), t: 60.0 },
            C2Ray { p: v(x, 10.0), d: v(0.0, 1.0), t: 60.0 },
            C2Ray { p: v(x, 30.0), d: v(0.0, -1.0), t: 60.0 },
        ] {
            let mut oc = SENTINEL;
            let mut or = SENTINEL;
            let rc = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut oc) };
            let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut or) };
            d.check(rc == rr && rceq(oc, or), || {
                format!("[capsule x={}] {} C ret={rc} out={} | R ret={rr} out={}", fs(x), rays(ray), rcs(oc), rcs(or))
            });
        }
    }
    // Capsule: y exactly == 0 and y exactly == yBb.y (the crossA / crossB
    // boundaries), and one ulp either side.
    for y0 in [0.0f32, step(0.0, true), -f32::from_bits(1), 20.0, step(20.0, true), step(20.0, false)] {
        let ray = C2Ray { p: v(-10.0, y0), d: v(1.0, 0.0), t: 60.0 };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe { (l.c.c2RaytoCapsule)(ray, cap, &mut oc) };
        let rr = unsafe { (l.r.c2RaytoCapsule)(ray, cap, &mut or) };
        d.check(rc == rr && rceq(oc, or), || {
            format!("[capsule y0={}] C ret={rc} out={} | R ret={rr} out={}", fs(y0), rcs(oc), rcs(or))
        });
    }
    // c2RaytoAABB: the four-way `t0 >= t1 && ...` tie-break, hit exactly on a
    // corner (so two t values are equal), and A.t at the exact 1.0 boundary.
    let b = C2AABB { min: v(-5.0, -5.0), max: v(5.0, 5.0) };
    for at in [
        (200.0f32).sqrt() / 2.0,
        step((200.0f32).sqrt() / 2.0, true),
        step((200.0f32).sqrt() / 2.0, false),
        14.142136,
        step(14.142136, true),
    ] {
        let ray = C2Ray { p: v(-10.0, -10.0), d: (l.c.c2Norm)(v(1.0, 1.0)), t: at };
        let mut oc = SENTINEL;
        let mut or = SENTINEL;
        let rc = unsafe { (l.c.c2RaytoAABB)(ray, b, &mut oc) };
        let rr = unsafe { (l.r.c2RaytoAABB)(ray, b, &mut or) };
        d.check(rc == rr && rceq(oc, or), || {
            format!("[aabb corner A.t={}] C ret={rc} out={} | R ret={rr} out={}", fs(at), rcs(oc), rcs(or))
        });
    }
    d.finish();
}
