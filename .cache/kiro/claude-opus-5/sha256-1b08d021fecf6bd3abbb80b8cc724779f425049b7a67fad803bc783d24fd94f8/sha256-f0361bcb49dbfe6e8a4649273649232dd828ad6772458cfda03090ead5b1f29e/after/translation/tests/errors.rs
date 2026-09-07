//! Phase C — error / rejection-path differential tests.
//!
//! One `#[test]` per row of `ERRORS.md`, in the same order. Each test
//! *constructs the exact rejecting condition*, asserts the C really does take
//! that path (so the row is not vacuously green), and then asserts the Rust
//! rejects with the same value and the same `*out` state.
//!
//! The C has no error enum: rejection is expressed as `return 0`, as an
//! `inf`/`NaN` sentinel from an unguarded divide, or as falling off the end of
//! a non-void function. All three are checked as exact bit patterns, not as
//! "both failed somehow".

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;

/// Runs a raycast on both libraries and additionally asserts the C returned
/// `expect_ret`, proving the intended branch was taken.
fn expect_ray<S: Copy + std::fmt::Debug>(
    what: &str,
    cfn: unsafe extern "C" fn(C2Ray, S, *mut C2Raycast) -> i32,
    rfn: unsafe extern "C" fn(C2Ray, S, *mut C2Raycast) -> i32,
    ray: C2Ray,
    shape: S,
    expect_ret: i32,
    ctx: &str,
) {
    let mut oc = C2Raycast::poison();
    let rc = unsafe { cfn(ray, shape, &mut oc) };
    assert_eq!(
        rc, expect_ret,
        "{what}: expected the C to return {expect_ret} for [{ctx}] but it returned {rc}; \
         the test no longer constructs the intended condition. shape={shape:?}"
    );
    cmp_ray(what, cfn, rfn, ray, shape, ctx);
}

/// Asserts `*out` was left exactly as the caller poisoned it, in BOTH
/// libraries -- i.e. the rejection happened before any write.
fn assert_out_untouched<S: Copy + std::fmt::Debug>(
    what: &str,
    cfn: unsafe extern "C" fn(C2Ray, S, *mut C2Raycast) -> i32,
    rfn: unsafe extern "C" fn(C2Ray, S, *mut C2Raycast) -> i32,
    ray: C2Ray,
    shape: S,
    ctx: &str,
) {
    let poison = C2Raycast::poison().bits();
    let mut oc = C2Raycast::poison();
    let mut or = C2Raycast::poison();
    let rc = unsafe { cfn(ray, shape, &mut oc) };
    let rr = unsafe { rfn(ray, shape, &mut or) };
    assert_eq!(oc.bits(), poison, "{what}: C wrote *out on a reject path [{ctx}]");
    assert_eq!(or.bits(), poison, "{what}: Rust wrote *out on a reject path [{ctx}]");
    assert_eq!(rc, rr, "{what}: ret mismatch [{ctx}]");
}

fn rc(ray: C2Ray, circle: C2Circle, ctx: &str) {
    let a = apis();
    cmp_ray("c2RaytoCircle", a.c.c2RaytoCircle, a.rust.c2RaytoCircle, ray, circle, ctx);
}
fn rb(ray: C2Ray, b: C2AABB, ctx: &str) {
    let a = apis();
    cmp_ray("c2RaytoAABB", a.c.c2RaytoAABB, a.rust.c2RaytoAABB, ray, b, ctx);
}
fn rk(ray: C2Ray, cap: C2Capsule, ctx: &str) {
    let a = apis();
    cmp_ray("c2RaytoCapsule", a.c.c2RaytoCapsule, a.rust.c2RaytoCapsule, ray, cap, ctx);
}

// ===========================================================================
// Rows 1-10 — c2RaytoCircle rejections
// ===========================================================================

#[test]
fn err_01_raytocircle_disc_negative() {
    let a = apis();
    let mut rng = Rng::new(101);
    // Ray travelling +x, offset perpendicular by more than r: the line misses,
    // so disc = b*b - c < 0.
    for i in 0..2000 {
        let r = rng.range(0.1, 5.0);
        let off = r + rng.range(0.01, 20.0);
        let ray = C2Ray { p: C2v::new(-50.0, off), d: C2v::new(1.0, 0.0), t: 200.0 };
        let circle = C2Circle { p: C2v::new(0.0, 0.0), r };
        expect_ray(
            "c2RaytoCircle",
            a.c.c2RaytoCircle,
            a.rust.c2RaytoCircle,
            ray,
            circle,
            0,
            &format!("disc<0 iter {i}"),
        );
        assert_out_untouched(
            "c2RaytoCircle",
            a.c.c2RaytoCircle,
            a.rust.c2RaytoCircle,
            ray,
            circle,
            &format!("disc<0 iter {i}"),
        );
    }
    // One ULP past tangency in both directions.
    for &r in &[1.0f32, 0.25, 7.5] {
        for n in [1i32, 2, 3] {
            let off = step(r, n);
            rc(
                C2Ray { p: C2v::new(-10.0, off), d: C2v::new(1.0, 0.0), t: 100.0 },
                C2Circle { p: C2v::new(0.0, 0.0), r },
                &format!("one-ulp-past-tangent r={} n={n}", f(r)),
            );
        }
    }
}

#[test]
fn err_02_raytocircle_t_negative() {
    let a = apis();
    let mut rng = Rng::new(102);
    // Origin strictly inside the circle: -b - sqrt(disc) is negative.
    for i in 0..2000 {
        let r = rng.range(0.2, 5.0);
        let frac = rng.range(0.0, 0.98);
        let ang = rng.range(0.0, 6.2831855);
        let ray = C2Ray {
            p: C2v::new(r * frac * ang.cos(), r * frac * ang.sin()),
            d: C2v::new(ang.cos(), ang.sin()),
            t: 100.0,
        };
        let circle = C2Circle { p: C2v::new(0.0, 0.0), r };
        expect_ray(
            "c2RaytoCircle",
            a.c.c2RaytoCircle,
            a.rust.c2RaytoCircle,
            ray,
            circle,
            0,
            &format!("t<0 inside iter {i}"),
        );
        assert_out_untouched(
            "c2RaytoCircle",
            a.c.c2RaytoCircle,
            a.rust.c2RaytoCircle,
            ray,
            circle,
            &format!("t<0 inside iter {i}"),
        );
        // Origin past the circle, still on the line: both roots behind.
        let d = rng.range(1.05, 10.0);
        let ray2 = C2Ray { p: C2v::new(r * d, 0.0), d: C2v::new(1.0, 0.0), t: 100.0 };
        expect_ray(
            "c2RaytoCircle",
            a.c.c2RaytoCircle,
            a.rust.c2RaytoCircle,
            ray2,
            circle,
            0,
            &format!("t<0 past iter {i}"),
        );
    }
    // One ULP inside the surface: t is a tiny negative, so it must reject.
    for &r in &[1.0f32, 3.0] {
        rc(
            C2Ray { p: C2v::new(step(r, -1), 0.0), d: C2v::new(1.0, 0.0), t: 100.0 },
            C2Circle { p: C2v::new(0.0, 0.0), r },
            "one-ulp-inside outbound",
        );
    }
}

#[test]
fn err_03_raytocircle_t_beyond_len() {
    let a = apis();
    let mut rng = Rng::new(103);
    for i in 0..2000 {
        let r = rng.range(0.1, 4.0);
        let dist = r + rng.range(1.0, 20.0);
        let hit_t = dist - r;
        let ray = C2Ray { p: C2v::new(-dist, 0.0), d: C2v::new(1.0, 0.0), t: hit_t * rng.range(0.0, 0.99) };
        let circle = C2Circle { p: C2v::new(0.0, 0.0), r };
        expect_ray(
            "c2RaytoCircle",
            a.c.c2RaytoCircle,
            a.rust.c2RaytoCircle,
            ray,
            circle,
            0,
            &format!("t>A.t iter {i}"),
        );
        assert_out_untouched(
            "c2RaytoCircle",
            a.c.c2RaytoCircle,
            a.rust.c2RaytoCircle,
            ray,
            circle,
            &format!("t>A.t iter {i}"),
        );
        // Exactly one ULP short of the hit distance -- the reject side of the
        // inclusive `t <= A.t` boundary.
        rc(
            C2Ray { p: C2v::new(-dist, 0.0), d: C2v::new(1.0, 0.0), t: step(hit_t, -1) },
            circle,
            &format!("one-ulp-short {i}"),
        );
    }
}

#[test]
fn err_04_raytocircle_zero_radius() {
    let mut rng = Rng::new(104);
    for i in 0..1500 {
        let circle = C2Circle { p: C2v::new(rng.range(-10.0, 10.0), rng.range(-10.0, 10.0)), r: 0.0 };
        let start = C2v::new(rng.range(-20.0, 20.0), rng.range(-20.0, 20.0));
        let (dx, dy) = (circle.p.x - start.x, circle.p.y - start.y);
        let l = (dx * dx + dy * dy).sqrt();
        // Aimed exactly at the zero-radius centre: disc == 0 exactly, so this
        // is the one input where a zero-radius circle can be hit.
        rc(
            C2Ray { p: start, d: C2v::new(dx / l, dy / l), t: l * 2.0 },
            circle,
            &format!("r=0 aimed {i}"),
        );
        rc(
            C2Ray { p: start, d: C2v::new(dx / l, dy / l), t: l * 0.5 },
            circle,
            &format!("r=0 aimed short {i}"),
        );
        // Not aimed: rejects.
        let ang = rng.range(0.0, 6.2831855);
        rc(
            C2Ray { p: start, d: C2v::new(ang.cos(), ang.sin()), t: 100.0 },
            circle,
            &format!("r=0 random dir {i}"),
        );
        // -0.0 radius.
        rc(
            C2Ray { p: start, d: C2v::new(dx / l, dy / l), t: l * 2.0 },
            C2Circle { p: circle.p, r: -0.0 },
            &format!("r=-0.0 aimed {i}"),
        );
    }
}


#[test]
fn err_05_raytocircle_negative_radius() {
    // The C computes B.r * B.r, so -r behaves exactly like +r. Assert that
    // equivalence holds in BOTH libraries, and that they agree with each other.
    let a = apis();
    let mut rng = Rng::new(105);
    for i in 0..2000 {
        let r = rng.range(0.05, 5.0);
        let centre = C2v::new(rng.range(-10.0, 10.0), rng.range(-10.0, 10.0));
        let start = C2v::new(rng.range(-20.0, 20.0), rng.range(-20.0, 20.0));
        let ang = rng.range(0.0, 6.2831855);
        let ray = C2Ray { p: start, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(0.0, 60.0) };
        let pos = C2Circle { p: centre, r };
        let neg = C2Circle { p: centre, r: -r };
        rc(ray, neg, &format!("neg r {i}"));
        let mut o1 = C2Raycast::poison();
        let mut o2 = C2Raycast::poison();
        let r1 = unsafe { (a.c.c2RaytoCircle)(ray, pos, &mut o1) };
        let r2 = unsafe { (a.c.c2RaytoCircle)(ray, neg, &mut o2) };
        assert_eq!(
            (r1, o1.bits()),
            (r2, o2.bits()),
            "C: +r and -r must behave identically (r*r), iter {i}"
        );
    }
}

#[test]
fn err_06_raytocircle_nonunit_dir() {
    let mut rng = Rng::new(106);
    for i in 0..2000 {
        let r = rng.range(0.1, 4.0);
        let dist = r + rng.range(0.5, 15.0);
        let circle = C2Circle { p: C2v::new(0.0, 0.0), r };
        for &s in &[0.0f32, -0.0, 1e-6, 0.5, 2.0, 1e6, -1.0, -3.0, f32::MAX, f32::INFINITY, f32::NAN] {
            rc(
                C2Ray { p: C2v::new(-dist, 0.0), d: C2v::new(s, 0.0), t: rng.range(0.0, 40.0) },
                circle,
                &format!("scale={} iter {i}", f(s)),
            );
        }
        // Zero direction vector: b == 0, disc == -c, so it hits only when the
        // origin is inside, where t < 0 rejects anyway.
        rc(
            C2Ray { p: C2v::new(0.0, 0.0), d: C2v::new(0.0, 0.0), t: 10.0 },
            circle,
            &format!("zero dir at centre {i}"),
        );
    }
}

#[test]
fn err_07_raytocircle_negative_t() {
    let a = apis();
    let mut rng = Rng::new(107);
    for i in 0..2000 {
        let r = rng.range(0.1, 4.0);
        let dist = r + rng.range(0.5, 15.0);
        let circle = C2Circle { p: C2v::new(0.0, 0.0), r };
        for &t in &[-0.0f32, -1e-6, -1.0, -1e6, -f32::MAX, f32::NEG_INFINITY] {
            let ray = C2Ray { p: C2v::new(-dist, 0.0), d: C2v::new(1.0, 0.0), t };
            // `t_hit >= 0 && t_hit <= A.t` cannot hold for a negative A.t
            // unless t_hit is -0.0, which needs dist == r exactly.
            expect_ray(
                "c2RaytoCircle",
                a.c.c2RaytoCircle,
                a.rust.c2RaytoCircle,
                ray,
                circle,
                0,
                &format!("A.t={} iter {i}", f(t)),
            );
            assert_out_untouched(
                "c2RaytoCircle",
                a.c.c2RaytoCircle,
                a.rust.c2RaytoCircle,
                ray,
                circle,
                &format!("A.t={} iter {i}", f(t)),
            );
        }
        // A.t == -0.0 with the origin exactly on the surface: t_hit == -0.0,
        // and `-0.0 >= 0 && -0.0 <= -0.0` is TRUE, so this accepts. A
        // "negative length always rejects" shortcut would get this wrong.
        rc(
            C2Ray { p: C2v::new(-r, 0.0), d: C2v::new(1.0, 0.0), t: -0.0 },
            circle,
            &format!("A.t=-0.0 on surface {i}"),
        );
    }
}

#[test]
fn err_08_raytocircle_nan_inputs() {
    let a = apis();
    let poison = C2Raycast::poison().bits();
    let mut rng = Rng::new(108);
    let nans = exotic_f32();
    for i in 0..1200 {
        let base_ray = C2Ray { p: C2v::new(-5.0, 0.0), d: C2v::new(1.0, 0.0), t: 20.0 };
        let base_circle = C2Circle { p: C2v::new(0.0, 0.0), r: 1.0 };
        // Poison one field at a time with each NaN pattern.
        for &n in &nans {
            if !n.is_nan() {
                continue;
            }
            let variants: [(C2Ray, C2Circle, &str); 7] = [
                (C2Ray { p: C2v::new(n, 0.0), ..base_ray }, base_circle, "p.x"),
                (C2Ray { p: C2v::new(-5.0, n), ..base_ray }, base_circle, "p.y"),
                (C2Ray { d: C2v::new(n, 0.0), ..base_ray }, base_circle, "d.x"),
                (C2Ray { d: C2v::new(1.0, n), ..base_ray }, base_circle, "d.y"),
                (C2Ray { t: n, ..base_ray }, base_circle, "t"),
                (base_ray, C2Circle { p: C2v::new(n, 0.0), r: 1.0 }, "c.p.x"),
                (base_ray, C2Circle { p: C2v::new(0.0, 0.0), r: n }, "c.r"),
            ];
            for (ray, circle, which) in variants {
                let mut oc = C2Raycast::poison();
                let rcv = unsafe { (a.c.c2RaytoCircle)(ray, circle, &mut oc) };
                assert_eq!(rcv, 0, "NaN in {which} must reject in C (iter {i})");
                assert_eq!(oc.bits(), poison, "NaN in {which}: C wrote *out");
                cmp_ray(
                    "c2RaytoCircle",
                    a.c.c2RaytoCircle,
                    a.rust.c2RaytoCircle,
                    ray,
                    circle,
                    &format!("NaN in {which} payload {:#010x}", n.to_bits()),
                );
            }
        }
        // Random all-NaN-ish shotgun.
        rc(
            C2Ray { p: rng.v_path(), d: rng.v_path(), t: rng.pathological() },
            C2Circle { p: rng.v_path(), r: rng.pathological() },
            &format!("nan shotgun {i}"),
        );
    }
}

#[test]
fn err_09_raytocircle_t_exactly_zero() {
    let a = apis();
    // Origin exactly on the surface, ray pointing outward: t_hit == 0, so the
    // C ACCEPTS, then impact == A.p and out->n = c2Norm(0,0) -- a NaN/inf
    // normal that must match bit-for-bit.
    let mut rng = Rng::new(109);
    for i in 0..1500 {
        let r = rng.range(0.1, 6.0);
        let cx = rng.range(-8.0, 8.0);
        let circle = C2Circle { p: C2v::new(cx, 0.0), r };
        for &(px, dx) in &[(cx - r, -1.0f32), (cx + r, 1.0f32)] {
            let ray = C2Ray { p: C2v::new(px, 0.0), d: C2v::new(dx, 0.0), t: 10.0 };
            let mut oc = C2Raycast::poison();
            let ret = unsafe { (a.c.c2RaytoCircle)(ray, circle, &mut oc) };
            if ret == 1 {
                assert_eq!(oc.t.to_bits(), 0u32, "t should be +0.0 when on-surface, iter {i}");
                assert!(
                    oc.n.x.is_nan() || oc.n.x.is_infinite(),
                    "c2Norm(0,0) should be non-finite, got {}",
                    f(oc.n.x)
                );
            }
            cmp_ray(
                "c2RaytoCircle",
                a.c.c2RaytoCircle,
                a.rust.c2RaytoCircle,
                ray,
                circle,
                &format!("t==0 iter {i} ret={ret}"),
            );
        }
        // Zero-length ray starting on the surface: A.t == 0 and t_hit == 0.
        rc(
            C2Ray { p: C2v::new(cx - r, 0.0), d: C2v::new(1.0, 0.0), t: 0.0 },
            circle,
            &format!("A.t==0 on surface {i}"),
        );
    }
}

#[test]
fn err_10_raytocircle_t_exactly_len() {
    let a = apis();
    let mut rng = Rng::new(110);
    for i in 0..2000 {
        // Integer geometry so `dist - r` is exact and `t == A.t` really happens.
        let r = (rng.next_u32() % 5 + 1) as f32;
        let dist = r + (rng.next_u32() % 20 + 1) as f32;
        let circle = C2Circle { p: C2v::new(0.0, 0.0), r };
        let hit_t = dist - r;
        let ray = C2Ray { p: C2v::new(-dist, 0.0), d: C2v::new(1.0, 0.0), t: hit_t };
        let mut oc = C2Raycast::poison();
        let ret = unsafe { (a.c.c2RaytoCircle)(ray, circle, &mut oc) };
        assert_eq!(
            ret, 1,
            "t == A.t is the INCLUSIVE side of `t <= A.t` and must be accepted (iter {i})"
        );
        cmp_ray("c2RaytoCircle", a.c.c2RaytoCircle, a.rust.c2RaytoCircle, ray, circle, &format!("t==A.t {i}"));
        for n in [-2i32, -1, 1, 2] {
            rc(
                C2Ray { p: C2v::new(-dist, 0.0), d: C2v::new(1.0, 0.0), t: step(hit_t, n) },
                circle,
                &format!("t==A.t{n:+} {i}"),
            );
        }
    }
}

// ===========================================================================
// Rows 11-20 — c2RaytoAABB rejections and the slab helper
// ===========================================================================

#[test]
fn err_11_raytoaabb_broadphase_reject() {
    let a = apis();
    let mut rng = Rng::new(111);
    // The swept AABB of [p0, p1] is disjoint from B, so `!c2AABBtoAABB` fires
    // at line 145 before any other work.
    for i in 0..2000 {
        let b = C2AABB { min: C2v::new(0.0, 0.0), max: C2v::new(1.0, 1.0) };
        let off = 2.0 + rng.range(0.0, 50.0);
        let rays = [
            C2Ray { p: C2v::new(-off - 5.0, 0.5), d: C2v::new(-1.0, 0.0), t: 4.0 }, // travels away, -x
            C2Ray { p: C2v::new(off, 0.5), d: C2v::new(1.0, 0.0), t: 4.0 },         // away, +x
            C2Ray { p: C2v::new(0.5, off), d: C2v::new(0.0, 1.0), t: 4.0 },         // away, +y
            C2Ray { p: C2v::new(0.5, -off), d: C2v::new(0.0, -1.0), t: 4.0 },       // away, -y
        ];
        for (j, &ray) in rays.iter().enumerate() {
            expect_ray(
                "c2RaytoAABB",
                a.c.c2RaytoAABB,
                a.rust.c2RaytoAABB,
                ray,
                b,
                0,
                &format!("broadphase {j} iter {i}"),
            );
            assert_out_untouched(
                "c2RaytoAABB",
                a.c.c2RaytoAABB,
                a.rust.c2RaytoAABB,
                ray,
                b,
                &format!("broadphase {j} iter {i}"),
            );
        }
    }
}

#[test]
fn err_12_raytoaabb_sat_reject() {
    let a = apis();
    let mut rng = Rng::new(112);
    let poison = C2Raycast::poison().bits();
    let mut found = 0usize;
    // Search for inputs that pass the broad phase but fail the separating-axis
    // test (`d > 0` at line 156). `aabb_stages` tells us which branch fired, so
    // this row cannot be vacuously green.
    for i in 0..60000 {
        let g = |r: &mut Rng| (r.next_u32() % 13) as f32 - 6.0;
        let ray = C2Ray {
            p: C2v::new(g(&mut rng), g(&mut rng)),
            d: C2v::new(g(&mut rng), g(&mut rng)),
            t: g(&mut rng),
        };
        let (x0, y0) = (g(&mut rng), g(&mut rng));
        let (w, h) = (rng.range(0.1, 6.0), rng.range(0.1, 6.0));
        let b = C2AABB { min: C2v::new(x0, y0), max: C2v::new(x0 + w, y0 + h) };
        let (bp, sat_d, ..) = aabb_stages(ray, b);
        if bp != 0 && sat_d > 0.0 {
            found += 1;
            let mut oc = C2Raycast::poison();
            let ret = unsafe { (a.c.c2RaytoAABB)(ray, b, &mut oc) };
            assert_eq!(ret, 0, "the `d > 0` SAT branch must reject (iter {i})");
            assert_eq!(oc.bits(), poison, "the SAT reject must not write *out (iter {i})");
            assert_out_untouched(
                "c2RaytoAABB",
                a.c.c2RaytoAABB,
                a.rust.c2RaytoAABB,
                ray,
                b,
                &format!("sat reject {i}"),
            );
        }
        cmp_ray("c2RaytoAABB", a.c.c2RaytoAABB, a.rust.c2RaytoAABB, ray, b, &format!("sat {i}"));
    }
    assert!(found > 100, "only {found} inputs reached the `d > 0` separating-axis reject; row 12 needs a better generator");
}

#[test]
fn err_13_raytoaabb_no_slab_hit() {
    let a = apis();
    // `hit == 0` needs all four `t <= 1.0` to be FALSE.
    //
    // For finite inputs that cannot happen: `c2RayToPlane_OneDimensional`
    // returns 0, or 1.0f, or `da/(da-db)` with `da > 0` and `db <= 0` (so
    // `da - db >= da` and the quotient is in [0,1]). Every outcome satisfies
    // `t <= 1.0`. The branch is therefore only reachable when some `t` is NaN,
    // which needs a NaN or an inf-minus-inf in the inputs. The generator below
    // includes those, and `aabb_stages` confirms line 195 is really the branch
    // being taken.
    let mut rng = Rng::new(113);
    let poison = C2Raycast::poison().bits();
    let mut found = 0usize;
    let b_fixed = C2AABB { min: C2v::new(-2.0, -2.0), max: C2v::new(2.0, 2.0) };
    for i in 0..40000 {
        let g = |r: &mut Rng| (r.next_u32() % 11) as f32 - 5.0;
        let (ray, b) = match i % 4 {
            // NaN origin with a finite endpoint inside the box: a_box collapses
            // onto p1 (the ternary min/max pick the non-NaN operand), so the
            // broad phase passes, `d > 0` is false for NaN, and all four
            // t values are NaN.
            0 => (
                C2Ray { p: C2v::new(f32::NAN, f32::NAN), d: rng.v(), t: rng.interesting() },
                b_fixed,
            ),
            1 => (
                C2Ray { p: C2v::new(-f32::NAN, f32::from_bits(0x7FC0_0002)), d: rng.v(), t: rng.interesting() },
                b_fixed,
            ),
            2 => (
                C2Ray { p: rng.v_path(), d: rng.v_path(), t: rng.pathological() },
                C2AABB { min: rng.v_path(), max: rng.v_path() },
            ),
            _ => (
                C2Ray {
                    p: C2v::new(g(&mut rng), g(&mut rng)),
                    d: C2v::new(g(&mut rng), g(&mut rng)),
                    t: g(&mut rng),
                },
                C2AABB {
                    min: C2v::new(g(&mut rng), g(&mut rng)),
                    max: C2v::new(g(&mut rng), g(&mut rng)),
                },
            ),
        };
        let (bp, sat_d, _t, hit, ..) = aabb_stages(ray, b);
        if bp != 0 && !(sat_d > 0.0) && !hit.iter().any(|&h| h) {
            found += 1;
            let mut oc = C2Raycast::poison();
            let ret = unsafe { (a.c.c2RaytoAABB)(ray, b, &mut oc) };
            assert_eq!(ret, 0, "the `hit == 0` branch must reject (iter {i})");
            assert_eq!(oc.bits(), poison, "the hit==0 reject must not write *out (iter {i})");
            assert_out_untouched(
                "c2RaytoAABB",
                a.c.c2RaytoAABB,
                a.rust.c2RaytoAABB,
                ray,
                b,
                &format!("no-slab-hit {i}"),
            );
        }
        cmp_ray("c2RaytoAABB", a.c.c2RaytoAABB, a.rust.c2RaytoAABB, ray, b, &format!("slab {i}"));
    }
    assert!(
        found > 100,
        "only {found} inputs reached the `hit == 0` reject at line 195; row 13 needs a better generator"
    );
}

#[test]
fn err_14_raytoaabb_inverted_box() {
    let mut rng = Rng::new(114);
    for i in 0..2500 {
        let (cx, cy) = (rng.range(-8.0, 8.0), rng.range(-8.0, 8.0));
        let (hw, hh) = (rng.range(0.1, 5.0), rng.range(0.1, 5.0));
        let inverted = [
            C2AABB { min: C2v::new(cx + hw, cy - hh), max: C2v::new(cx - hw, cy + hh) },
            C2AABB { min: C2v::new(cx - hw, cy + hh), max: C2v::new(cx + hw, cy - hh) },
            C2AABB { min: C2v::new(cx + hw, cy + hh), max: C2v::new(cx - hw, cy - hh) },
        ];
        let ang = rng.range(0.0, 6.2831855);
        for (j, &b) in inverted.iter().enumerate() {
            rb(
                C2Ray {
                    p: C2v::new(cx - 3.0 * hw, cy),
                    d: C2v::new(1.0, 0.0),
                    t: 6.0 * hw,
                },
                b,
                &format!("inverted {j} axis-aligned {i}"),
            );
            rb(
                C2Ray {
                    p: C2v::new(rng.range(-20.0, 20.0), rng.range(-20.0, 20.0)),
                    d: C2v::new(ang.cos(), ang.sin()),
                    t: rng.range(0.0, 50.0),
                },
                b,
                &format!("inverted {j} random {i}"),
            );
        }
    }
}

#[test]
fn err_15_raytoaabb_degenerate_box() {
    let mut rng = Rng::new(115);
    for i in 0..2500 {
        let (cx, cy) = (rng.range(-8.0, 8.0), rng.range(-8.0, 8.0));
        let boxes = [
            C2AABB { min: C2v::new(cx, cy), max: C2v::new(cx, cy) },
            C2AABB { min: C2v::new(cx, cy - 2.0), max: C2v::new(cx, cy + 2.0) },
            C2AABB { min: C2v::new(cx - 2.0, cy), max: C2v::new(cx + 2.0, cy) },
            C2AABB { min: C2v::new(-0.0, -0.0), max: C2v::new(0.0, 0.0) },
        ];
        for (j, &b) in boxes.iter().enumerate() {
            // Aimed exactly at the degenerate geometry.
            rb(
                C2Ray { p: C2v::new(cx - 5.0, cy), d: C2v::new(1.0, 0.0), t: 10.0 },
                b,
                &format!("degenerate {j} aimed {i}"),
            );
            rb(
                C2Ray { p: C2v::new(cx, cy - 5.0), d: C2v::new(0.0, 1.0), t: 10.0 },
                b,
                &format!("degenerate {j} aimed-y {i}"),
            );
            // Starting exactly on it.
            rb(
                C2Ray { p: C2v::new(cx, cy), d: C2v::new(1.0, 1.0), t: 5.0 },
                b,
                &format!("degenerate {j} on {i}"),
            );
            let ang = rng.range(0.0, 6.2831855);
            rb(
                C2Ray {
                    p: C2v::new(rng.range(-15.0, 15.0), rng.range(-15.0, 15.0)),
                    d: C2v::new(ang.cos(), ang.sin()),
                    t: rng.range(0.0, 40.0),
                },
                b,
                &format!("degenerate {j} random {i}"),
            );
        }
    }
}

#[test]
fn err_16_raytoaabb_zero_length_ray() {
    let a = apis();
    let mut rng = Rng::new(116);
    // A.t == 0 makes p1 == p0, so ab == 0, n == 0, abs_n == 0 and
    // d = |dot(0, ...)| - dot(0, half) = 0 - 0 = 0, which is NOT > 0, so the
    // C continues into the slab stage rather than rejecting.
    for i in 0..2500 {
        let b = C2AABB { min: C2v::new(-2.0, -2.0), max: C2v::new(2.0, 2.0) };
        let inside = C2v::new(rng.range(-1.9, 1.9), rng.range(-1.9, 1.9));
        for &t in &[0.0f32, -0.0] {
            let ray = C2Ray { p: inside, d: C2v::new(rng.range(-3.0, 3.0), rng.range(-3.0, 3.0)), t };
            let mut oc = C2Raycast::poison();
            let ret = unsafe { (a.c.c2RaytoAABB)(ray, b, &mut oc) };
            assert_eq!(
                ret, 1,
                "A.t == {} with the origin inside must still report a hit (iter {i})",
                f(t)
            );
            assert_eq!(oc.t.to_bits() & 0x7FFF_FFFF, 0, "out->t should be a zero, got {}", f(oc.t));
            cmp_ray("c2RaytoAABB", a.c.c2RaytoAABB, a.rust.c2RaytoAABB, ray, b, &format!("A.t=0 inside {i}"));
            // Origin outside: the broad phase rejects instead.
            let outside = C2v::new(rng.range(5.0, 20.0), rng.range(5.0, 20.0));
            rb(C2Ray { p: outside, d: C2v::new(1.0, 0.0), t }, b, &format!("A.t=0 outside {i}"));
        }
    }
}

#[test]
fn err_17_raytoaabb_nan_inputs() {
    let a = apis();
    let mut rng = Rng::new(117);
    for i in 0..1000 {
        let base_ray = C2Ray { p: C2v::new(-5.0, 0.0), d: C2v::new(1.0, 0.0), t: 20.0 };
        let base_box = C2AABB { min: C2v::new(-1.0, -1.0), max: C2v::new(1.0, 1.0) };
        for &n in exotic_f32().iter().filter(|v| v.is_nan()) {
            let variants: [(C2Ray, C2AABB, &str); 9] = [
                (C2Ray { p: C2v::new(n, 0.0), ..base_ray }, base_box, "p.x"),
                (C2Ray { p: C2v::new(-5.0, n), ..base_ray }, base_box, "p.y"),
                (C2Ray { d: C2v::new(n, 0.0), ..base_ray }, base_box, "d.x"),
                (C2Ray { d: C2v::new(1.0, n), ..base_ray }, base_box, "d.y"),
                (C2Ray { t: n, ..base_ray }, base_box, "t"),
                (base_ray, C2AABB { min: C2v::new(n, -1.0), ..base_box }, "min.x"),
                (base_ray, C2AABB { min: C2v::new(-1.0, n), ..base_box }, "min.y"),
                (base_ray, C2AABB { max: C2v::new(n, 1.0), ..base_box }, "max.x"),
                (base_ray, C2AABB { max: C2v::new(1.0, n), ..base_box }, "max.y"),
            ];
            for (ray, b, which) in variants {
                cmp_ray(
                    "c2RaytoAABB",
                    a.c.c2RaytoAABB,
                    a.rust.c2RaytoAABB,
                    ray,
                    b,
                    &format!("NaN in {which} payload {:#010x} iter {i}", n.to_bits()),
                );
            }
        }
        rb(
            C2Ray { p: rng.v_path(), d: rng.v_path(), t: rng.pathological() },
            C2AABB { min: rng.v_path(), max: rng.v_path() },
            &format!("nan shotgun {i}"),
        );
    }
}

#[test]
fn err_18_ray2plane_da_negative() {
    // c2SignedDistPointToPlane_OneDimensional(p, -1, min) = -p + min, so
    // da0 < 0 whenever p0.x > B.min.x. Place the origin inside the x-slab and
    // above/below in y so multiple slabs take the `da < 0 -> 0` branch, and
    // vary which ones.
    let mut rng = Rng::new(118);
    let b = C2AABB { min: C2v::new(-1.0, -1.0), max: C2v::new(1.0, 1.0) };
    for i in 0..3000 {
        // Origin strictly inside: da0 = -p.x + (-1) < 0 and da1 = p.x - 1 < 0
        // simultaneously, so BOTH x slabs return 0.
        let p = C2v::new(rng.range(-0.99, 0.99), rng.range(-0.99, 0.99));
        let ang = rng.range(0.0, 6.2831855);
        rb(
            C2Ray { p, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(0.0, 10.0) },
            b,
            &format!("da<0 both-x {i}"),
        );
        // Origin outside on exactly one side, so exactly one da is negative.
        for &(px, py) in &[(2.0f32, 0.0f32), (-2.0, 0.0), (0.0, 2.0), (0.0, -2.0)] {
            rb(
                C2Ray { p: C2v::new(px, py), d: C2v::new(-px * 0.5, -py * 0.5), t: 4.0 },
                b,
                &format!("da<0 one-side ({px},{py}) {i}"),
            );
        }
        // Exactly on a plane: da == 0, which is NOT < 0, so it falls to the
        // next branch -- the boundary of the `da < 0` test.
        for &px in &[-1.0f32, 1.0] {
            rb(
                C2Ray { p: C2v::new(px, 0.0), d: C2v::new(-px, 0.0), t: 2.0 },
                b,
                &format!("da==0 px={px} {i}"),
            );
            rb(
                C2Ray { p: C2v::new(step(px, 1), 0.0), d: C2v::new(-px, 0.0), t: 2.0 },
                b,
                &format!("da==0+ulp px={px} {i}"),
            );
            rb(
                C2Ray { p: C2v::new(step(px, -1), 0.0), d: C2v::new(-px, 0.0), t: 2.0 },
                b,
                &format!("da==0-ulp px={px} {i}"),
            );
        }
    }
}

#[test]
fn err_19_ray2plane_same_side() {
    // `da * db > 0` returns 1.0f. This branch is REACHABLE in the helper but
    // UNREACHABLE from `c2RaytoAABB`, and the test asserts that structural
    // property rather than pretending to exercise it:
    //
    //   da0 > 0 && db0 > 0  <=>  p0.x < B.min.x && p1.x < B.min.x
    //                       =>  a_box.max.x < B.min.x
    //                       =>  c2AABBtoAABB sets d1, so the broad phase at
    //                           line 145 already returned 0.
    //
    // Symmetrically for the `B.max.x` / `B.min.y` / `B.max.y` planes. So the
    // `1.0f` return can never influence an output. The helper is `static
    // inline` and has no exported symbol, so it cannot be called directly;
    // this is the strongest available check.
    let a = apis();
    let mut rng = Rng::new(119);
    let mut fired_in_helper = 0usize;
    let mut fired_after_broadphase = 0usize;
    let searched = 60000;
    for i in 0..searched {
        let g = |r: &mut Rng| (r.next_u32() % 11) as f32 - 5.0;
        let (ray, b) = match i % 3 {
            0 => (
                C2Ray {
                    p: C2v::new(g(&mut rng), g(&mut rng)),
                    d: C2v::new(g(&mut rng), g(&mut rng)),
                    t: g(&mut rng),
                },
                C2AABB {
                    min: C2v::new(g(&mut rng), g(&mut rng)),
                    max: C2v::new(g(&mut rng), g(&mut rng)),
                },
            ),
            // Inverted boxes, where the "at most one da > 0 per axis" argument
            // for proper boxes no longer holds.
            1 => {
                let (mx, my) = (rng.range(-3.0, 3.0), rng.range(-3.0, 3.0));
                (
                    C2Ray {
                        p: C2v::new(mx, my),
                        d: C2v::new(rng.range(-4.0, 4.0), rng.range(-4.0, 4.0)),
                        t: rng.range(-4.0, 4.0),
                    },
                    C2AABB {
                        min: C2v::new(mx + rng.range(0.1, 4.0), my + rng.range(0.1, 4.0)),
                        max: C2v::new(mx - rng.range(0.1, 4.0), my - rng.range(0.1, 4.0)),
                    },
                )
            }
            _ => (
                C2Ray { p: rng.v_path(), d: rng.v_path(), t: rng.pathological() },
                C2AABB { min: rng.v_path(), max: rng.v_path() },
            ),
        };
        let (bp, sat_d, t, hit, same_side, _) = aabb_stages(ray, b);
        if same_side.iter().any(|&s| s) {
            fired_in_helper += 1;
            if bp != 0 && !(sat_d > 0.0) {
                fired_after_broadphase += 1;
                // If the analysis above is ever wrong, still hold the two
                // libraries to the same result and check the 1.0f effect.
                let w = aabb_winner(t, hit);
                if same_side[w] {
                    let mut oc = C2Raycast::poison();
                    let ret = unsafe { (a.c.c2RaytoAABB)(ray, b, &mut oc) };
                    assert_eq!(ret, 1, "a winning same-side slab means hit (iter {i})");
                }
            }
        }
        cmp_ray("c2RaytoAABB", a.c.c2RaytoAABB, a.rust.c2RaytoAABB, ray, b, &format!("same-side {i}"));
    }
    assert!(
        fired_in_helper > 1000,
        "the generator only produced {fired_in_helper} same-side conditions out of {searched}; \
         it is not actually probing the branch"
    );
    println!(
        "row 19: `da*db > 0` arose {fired_in_helper} times in {searched} inputs, \
         {fired_after_broadphase} of them after the broad phase"
    );
    assert_eq!(
        fired_after_broadphase, 0,
        "the `da*db > 0 -> 1.0f` branch was believed unreachable after the broad phase, \
         but fired {fired_after_broadphase} times out of {searched}. The reachability \
         argument in this test and in ERRORS.md row 19 must be revisited."
    );
}

#[test]
fn err_20_ray2plane_zero_denominator() {
    // `d = da - db == 0` is the library's ONLY explicit zero-divisor guard.
    // da - db = (p0*n - dn) - (p1*n - dn) = (p0 - p1)*n, so it is exactly zero
    // whenever the segment does not move along that axis -- i.e. an
    // axis-aligned ray, or A.t == 0, or a zero direction.
    let a = apis();
    let b = C2AABB { min: C2v::new(-1.0, -1.0), max: C2v::new(1.0, 1.0) };
    let mut rng = Rng::new(120);
    for i in 0..3000 {
        let cases = [
            // Pure +x: the two y slabs have da - db == 0.
            C2Ray { p: C2v::new(-3.0, rng.range(-0.9, 0.9)), d: C2v::new(1.0, 0.0), t: 6.0 },
            // Pure +y: the two x slabs have da - db == 0.
            C2Ray { p: C2v::new(rng.range(-0.9, 0.9), -3.0), d: C2v::new(0.0, 1.0), t: 6.0 },
            // Zero direction inside: ALL FOUR slabs have da - db == 0.
            C2Ray { p: C2v::new(0.0, 0.0), d: C2v::new(0.0, 0.0), t: 5.0 },
            // A.t == 0 inside: same, via p1 == p0.
            C2Ray { p: C2v::new(0.0, 0.0), d: C2v::new(1.0, 1.0), t: 0.0 },
            // Zero direction, origin exactly on a corner.
            C2Ray { p: C2v::new(1.0, 1.0), d: C2v::new(0.0, 0.0), t: 1.0 },
            // Denormal direction: p1 - p0 underflows to exactly zero.
            C2Ray { p: C2v::new(0.0, 0.0), d: C2v::new(f32::from_bits(1), 0.0), t: f32::from_bits(1) },
            // Signed-zero direction.
            C2Ray { p: C2v::new(0.0, 0.5), d: C2v::new(-0.0, -0.0), t: 3.0 },
        ];
        for (j, &ray) in cases.iter().enumerate() {
            cmp_ray(
                "c2RaytoAABB",
                a.c.c2RaytoAABB,
                a.rust.c2RaytoAABB,
                ray,
                b,
                &format!("zero-denominator case {j} iter {i}"),
            );
        }
        // Without the guard, `da / 0` would be ±inf or NaN and out->t would be
        // non-finite. Assert the C's out->t IS finite on the axis-aligned case,
        // proving the guard is live and that the Rust must reproduce it.
        let ray = C2Ray { p: C2v::new(-3.0, 0.0), d: C2v::new(1.0, 0.0), t: 6.0 };
        let mut oc = C2Raycast::poison();
        let ret = unsafe { (a.c.c2RaytoAABB)(ray, b, &mut oc) };
        assert_eq!(ret, 1, "axis-aligned ray through the box should hit");
        assert!(oc.t.is_finite(), "the d != 0 guard should keep out->t finite, got {}", f(oc.t));
    }
}

// ===========================================================================
// Rows 21-26 — predicate rejections
// ===========================================================================

#[test]
fn err_21_aabbtoaabb_nan() {
    let a = apis();
    // Every comparison in c2AABBtoAABB is `<`, which is false for NaN, so
    // d0..d3 are all 0 and the function reports "overlapping" (1). That is the
    // opposite of the intuitive answer and must be reproduced.
    let sane = C2AABB { min: C2v::new(0.0, 0.0), max: C2v::new(1.0, 1.0) };
    let far = C2AABB { min: C2v::new(100.0, 100.0), max: C2v::new(101.0, 101.0) };
    for &n in exotic_f32().iter().filter(|v| v.is_nan()) {
        let all_nan = C2AABB { min: C2v::new(n, n), max: C2v::new(n, n) };
        for (name, x, y) in [
            ("all-nan vs sane", all_nan, sane),
            ("sane vs all-nan", sane, all_nan),
            ("all-nan vs far", all_nan, far),
            ("all-nan vs all-nan", all_nan, all_nan),
        ] {
            let c = unsafe { (a.c.c2AABBtoAABB)(x, y) };
            let r = unsafe { (a.rust.c2AABBtoAABB)(x, y) };
            assert_eq!(c, 1, "{name}: NaN makes every `<` false, so C must return 1");
            assert_eq!(c, r, "{name}: C={c} Rust={r}");
        }
        // One NaN field at a time, crossed with a disjoint partner.
        let fields: [C2AABB; 4] = [
            C2AABB { min: C2v::new(n, 0.0), max: C2v::new(1.0, 1.0) },
            C2AABB { min: C2v::new(0.0, n), max: C2v::new(1.0, 1.0) },
            C2AABB { min: C2v::new(0.0, 0.0), max: C2v::new(n, 1.0) },
            C2AABB { min: C2v::new(0.0, 0.0), max: C2v::new(1.0, n) },
        ];
        for (j, &x) in fields.iter().enumerate() {
            for &y in &[sane, far] {
                let c = unsafe { (a.c.c2AABBtoAABB)(x, y) };
                let r = unsafe { (a.rust.c2AABBtoAABB)(x, y) };
                assert_eq!(c, r, "one-nan field {j} payload {:#010x}: C={c} Rust={r}", n.to_bits());
                let c = unsafe { (a.c.c2AABBtoAABB)(y, x) };
                let r = unsafe { (a.rust.c2AABBtoAABB)(y, x) };
                assert_eq!(c, r, "one-nan field {j} reversed: C={c} Rust={r}");
            }
        }
    }
    let mut rng = Rng::new(121);
    for i in 0..4000 {
        let x = C2AABB { min: rng.v_path(), max: rng.v_path() };
        let y = C2AABB { min: rng.v_path(), max: rng.v_path() };
        assert_eq!(
            unsafe { (a.c.c2AABBtoAABB)(x, y) },
            unsafe { (a.rust.c2AABBtoAABB)(x, y) },
            "shotgun {i}: A={x:?} B={y:?}"
        );
    }
}

#[test]
fn err_22_aabbtopoint_outside() {
    let a = apis();
    let b = C2AABB { min: C2v::new(-1.0, -2.0), max: C2v::new(3.0, 4.0) };
    // One row per side of the `!(d0|d1|d2|d3)` rejection.
    let outside = [
        ("d0 x<min", C2v::new(step(-1.0, -1), 0.0)),
        ("d1 y<min", C2v::new(0.0, step(-2.0, -1))),
        ("d2 x>max", C2v::new(step(3.0, 1), 0.0)),
        ("d3 y>max", C2v::new(0.0, step(4.0, 1))),
        ("far -x", C2v::new(-1e6, 0.0)),
        ("far +x", C2v::new(1e6, 0.0)),
        ("far -y", C2v::new(0.0, -1e6)),
        ("far +y", C2v::new(0.0, 1e6)),
        ("-inf x", C2v::new(f32::NEG_INFINITY, 0.0)),
        ("+inf y", C2v::new(0.0, f32::INFINITY)),
        ("outside corner", C2v::new(-5.0, -5.0)),
    ];
    for (name, p) in outside {
        let c = unsafe { (a.c.c2AABBtoPoint)(b, p) };
        let r = unsafe { (a.rust.c2AABBtoPoint)(b, p) };
        assert_eq!(c, 0, "{name}: the C must reject a point outside the box");
        assert_eq!(c, r, "{name}: C={c} Rust={r}");
    }
    // Exactly on each edge: the comparisons are strict, so these are ACCEPTED.
    for (name, p) in [
        ("on x=min", C2v::new(-1.0, 0.0)),
        ("on y=min", C2v::new(0.0, -2.0)),
        ("on x=max", C2v::new(3.0, 0.0)),
        ("on y=max", C2v::new(0.0, 4.0)),
        ("min corner", C2v::new(-1.0, -2.0)),
        ("max corner", C2v::new(3.0, 4.0)),
    ] {
        let c = unsafe { (a.c.c2AABBtoPoint)(b, p) };
        let r = unsafe { (a.rust.c2AABBtoPoint)(b, p) };
        assert_eq!(c, 1, "{name}: the boundary is inclusive and must be accepted");
        assert_eq!(c, r, "{name}: C={c} Rust={r}");
    }
    let mut rng = Rng::new(122);
    for i in 0..4000 {
        let bb = C2AABB { min: rng.v_path(), max: rng.v_path() };
        let p = rng.v_path();
        assert_eq!(
            unsafe { (a.c.c2AABBtoPoint)(bb, p) },
            unsafe { (a.rust.c2AABBtoPoint)(bb, p) },
            "shotgun {i}: box={bb:?} p=({},{})",
            f(p.x),
            f(p.y)
        );
    }
}

#[test]
fn err_23_aabbtopoint_nan() {
    let a = apis();
    let b = C2AABB { min: C2v::new(0.0, 0.0), max: C2v::new(1.0, 1.0) };
    for &n in exotic_f32().iter().filter(|v| v.is_nan()) {
        for (name, p) in [
            ("nan x", C2v::new(n, 0.5)),
            ("nan y", C2v::new(0.5, n)),
            ("nan both", C2v::new(n, n)),
        ] {
            let c = unsafe { (a.c.c2AABBtoPoint)(b, p) };
            let r = unsafe { (a.rust.c2AABBtoPoint)(b, p) };
            assert_eq!(c, 1, "{name}: NaN makes all 4 comparisons false, so C returns 1");
            assert_eq!(c, r, "{name}: C={c} Rust={r}");
        }
        // NaN in the box instead of the point.
        for (name, bb) in [
            ("nan min.x", C2AABB { min: C2v::new(n, 0.0), max: C2v::new(1.0, 1.0) }),
            ("nan max.y", C2AABB { min: C2v::new(0.0, 0.0), max: C2v::new(1.0, n) }),
        ] {
            for &p in &[C2v::new(0.5, 0.5), C2v::new(-9.0, -9.0), C2v::new(9.0, 9.0)] {
                let c = unsafe { (a.c.c2AABBtoPoint)(bb, p) };
                let r = unsafe { (a.rust.c2AABBtoPoint)(bb, p) };
                assert_eq!(c, r, "{name} p=({},{}): C={c} Rust={r}", f(p.x), f(p.y));
            }
        }
    }
}

#[test]
fn err_24_circletopoint_on_or_outside() {
    let a = apis();
    // `d2 < A.r * A.r` is EXCLUSIVE, so a point exactly on the rim is rejected.
    let mut rng = Rng::new(124);
    for i in 0..3000 {
        // Integer radii and axis-aligned offsets so `d2 == r*r` is exact.
        let r = (rng.next_u32() % 8 + 1) as f32;
        let ci = C2Circle { p: C2v::new(0.0, 0.0), r };
        for &p in &[
            C2v::new(r, 0.0),
            C2v::new(-r, 0.0),
            C2v::new(0.0, r),
            C2v::new(0.0, -r),
        ] {
            let c = unsafe { (a.c.c2CircleToPoint)(ci, p) };
            let rr = unsafe { (a.rust.c2CircleToPoint)(ci, p) };
            assert_eq!(c, 0, "exactly on the rim must be REJECTED (r={}, iter {i})", f(r));
            assert_eq!(c, rr, "on-rim: C={c} Rust={rr}");
        }
        // Strictly outside, and one ULP outside.
        for &p in &[
            C2v::new(step(r, 1), 0.0),
            C2v::new(r * 1.5, 0.0),
            C2v::new(1e9, 0.0),
            C2v::new(f32::INFINITY, 0.0),
        ] {
            let c = unsafe { (a.c.c2CircleToPoint)(ci, p) };
            let rr = unsafe { (a.rust.c2CircleToPoint)(ci, p) };
            assert_eq!(c, 0, "outside must be rejected (r={}, iter {i})", f(r));
            assert_eq!(c, rr, "outside: C={c} Rust={rr}");
        }
        // One ULP inside is accepted.
        let p = C2v::new(step(r, -1), 0.0);
        let c = unsafe { (a.c.c2CircleToPoint)(ci, p) };
        let rr = unsafe { (a.rust.c2CircleToPoint)(ci, p) };
        assert_eq!(c, 1, "one ULP inside the rim must be accepted (r={}, iter {i})", f(r));
        assert_eq!(c, rr, "one-ulp-inside: C={c} Rust={rr}");
    }
}

#[test]
fn err_25_circletopoint_zero_radius() {
    let a = apis();
    // r == 0 makes the test `d2 < 0`, which is never true -- not even at the
    // centre, where d2 == 0.
    let mut rng = Rng::new(125);
    for i in 0..2000 {
        let centre = C2v::new(rng.range(-10.0, 10.0), rng.range(-10.0, 10.0));
        for &r in &[0.0f32, -0.0] {
            let ci = C2Circle { p: centre, r };
            for &p in &[centre, C2v::new(centre.x, centre.y), rng.v(), C2v::new(0.0, 0.0)] {
                let c = unsafe { (a.c.c2CircleToPoint)(ci, p) };
                let rr = unsafe { (a.rust.c2CircleToPoint)(ci, p) };
                assert_eq!(c, 0, "r={} must reject every point, including the centre (iter {i})", f(r));
                assert_eq!(c, rr, "r={}: C={c} Rust={rr}", f(r));
            }
        }
        // Denormal radius: r*r underflows to exactly 0, so it also rejects the
        // centre -- a "positive radius always contains its centre" assumption
        // would be wrong here.
        let ci = C2Circle { p: centre, r: f32::from_bits(1) };
        let c = unsafe { (a.c.c2CircleToPoint)(ci, centre) };
        let rr = unsafe { (a.rust.c2CircleToPoint)(ci, centre) };
        assert_eq!(c, rr, "denormal radius at centre: C={c} Rust={rr}");
    }
}

#[test]
fn err_26_circletopoint_nan() {
    let a = apis();
    let mut rng = Rng::new(126);
    for &n in exotic_f32().iter().filter(|v| v.is_nan()) {
        let variants = [
            ("nan p.x", C2Circle { p: C2v::new(0.0, 0.0), r: 1.0 }, C2v::new(n, 0.0)),
            ("nan p.y", C2Circle { p: C2v::new(0.0, 0.0), r: 1.0 }, C2v::new(0.0, n)),
            ("nan centre", C2Circle { p: C2v::new(n, n), r: 1.0 }, C2v::new(0.0, 0.0)),
            ("nan radius", C2Circle { p: C2v::new(0.0, 0.0), r: n }, C2v::new(0.0, 0.0)),
            ("nan radius far", C2Circle { p: C2v::new(0.0, 0.0), r: n }, C2v::new(1e9, 1e9)),
        ];
        for (name, ci, p) in variants {
            let c = unsafe { (a.c.c2CircleToPoint)(ci, p) };
            let rr = unsafe { (a.rust.c2CircleToPoint)(ci, p) };
            assert_eq!(c, 0, "{name}: `d2 < r*r` is false for NaN, so C must return 0");
            assert_eq!(c, rr, "{name}: C={c} Rust={rr}");
        }
    }
    for i in 0..4000 {
        let ci = C2Circle { p: rng.v_path(), r: rng.pathological() };
        let p = rng.v_path();
        assert_eq!(
            unsafe { (a.c.c2CircleToPoint)(ci, p) },
            unsafe { (a.rust.c2CircleToPoint)(ci, p) },
            "shotgun {i}: circle={ci:?} p=({},{})",
            f(p.x),
            f(p.y)
        );
    }
}

// ===========================================================================
// Rows 27-33 — c2RaytoCapsule rejections
// ===========================================================================

/// Capsule-local frame, mirroring `M` in the C, so a test can place the ray
/// origin in a chosen region and predict which branch fires.
fn cap_frame(cap: C2Capsule) -> (C2v, C2v, f32) {
    let (dx, dy) = (cap.b.x - cap.a.x, cap.b.y - cap.a.y);
    let l = (dx * dx + dy * dy).sqrt();
    let my = C2v::new(dx / l, dy / l);
    let mx = C2v::new(my.y, -my.x);
    (mx, my, l)
}

fn cap_local_to_world(cap: C2Capsule, mx: C2v, my: C2v, u: f32, v: f32) -> C2v {
    C2v::new(cap.a.x + u * mx.x + v * my.x, cap.a.y + u * mx.y + v * my.y)
}

fn rand_cap(rng: &mut Rng) -> C2Capsule {
    let (cx, cy) = (rng.range(-15.0, 15.0), rng.range(-15.0, 15.0));
    let ang = rng.range(0.0, 6.2831855);
    let half = rng.range(0.2, 8.0);
    let (ux, uy) = (ang.cos(), ang.sin());
    C2Capsule {
        a: C2v::new(cx - ux * half, cy - uy * half),
        b: C2v::new(cx + ux * half, cy + uy * half),
        r: rng.range(0.05, 4.0),
    }
}

#[test]
fn err_27_raytocapsule_fallthrough_writes_out() {
    let a = apis();
    // The `return 0` at line 291 is a REJECTION, but lines 243-244 already
    // wrote `out->n = c2Norm(b - a)` and `out->t = 0`. So unlike the other two
    // raycasts, a capsule miss still mutates *out -- and the Rust must mutate
    // it identically. A translation that returns early before writing would
    // pass a return-value-only test and fail here.
    let mut rng = Rng::new(127);
    let poison = C2Raycast::poison().bits();
    let mut found = 0usize;
    for i in 0..8000 {
        let cap = rand_cap(&mut rng);
        let (mx, my, l) = cap_frame(cap);
        // Origin far to one side, moving parallel to the axis: never crosses
        // +-B.r, so the condition at line 260 is false and it falls through.
        let u = (cap.r + rng.range(1.0, 20.0)) * if rng.next_u32() % 2 == 0 { 1.0 } else { -1.0 };
        let p = cap_local_to_world(cap, mx, my, u, l * rng.range(-1.0, 2.0));
        let ray = C2Ray {
            p,
            d: C2v::new(my.x, my.y), // parallel to the capsule axis
            t: rng.range(0.0, 20.0),
        };
        let mut oc = C2Raycast::poison();
        let ret = unsafe { (a.c.c2RaytoCapsule)(ray, cap, &mut oc) };
        if ret == 0 {
            found += 1;
            assert_ne!(
                oc.bits(),
                poison,
                "the capsule fall-through must still have written *out (iter {i})"
            );
            assert_eq!(oc.t.to_bits() & 0x7FFF_FFFF, 0, "out->t should be a zero, got {}", f(oc.t));
            let expect_n = unsafe { (a.c.c2Norm)((a.c.c2Sub)(cap.b, cap.a)) };
            assert_eq!(
                (oc.n.x.to_bits(), oc.n.y.to_bits()),
                (expect_n.x.to_bits(), expect_n.y.to_bits()),
                "out->n should be c2Norm(b - a) (iter {i})"
            );
        }
        rk(ray, cap, &format!("fallthrough {i}"));
    }
    assert!(found > 100, "only {found} inputs reached the capsule fall-through; row 27 needs a better generator");
}

#[test]
fn err_28_raytocapsule_degenerate_ab() {
    let a = apis();
    // B.a == B.b -> c2Sub is (0,0) -> c2Norm divides by zero -> M.y is
    // NaN/inf, which poisons M, yBb and yAp. out->n is written with that
    // non-finite value before any return, so it is observable on every path.
    let mut rng = Rng::new(128);
    for i in 0..4000 {
        let p0 = C2v::new(rng.range(-10.0, 10.0), rng.range(-10.0, 10.0));
        for &r in &[1.0f32, 0.0, -1.0, 1e-6, 1e6, f32::NAN, f32::INFINITY] {
            let cap = C2Capsule { a: p0, b: p0, r };
            let ang = rng.range(0.0, 6.2831855);
            let ray = C2Ray {
                p: C2v::new(rng.range(-20.0, 20.0), rng.range(-20.0, 20.0)),
                d: C2v::new(ang.cos(), ang.sin()),
                t: rng.range(0.0, 40.0),
            };
            let mut oc = C2Raycast::poison();
            let _ = unsafe { (a.c.c2RaytoCapsule)(ray, cap, &mut oc) };
            assert!(
                !oc.n.x.is_finite() || !oc.n.y.is_finite() || oc.n.x == 0.0,
                "a degenerate axis should give a non-finite (or zero) out->n, got ({},{})",
                f(oc.n.x),
                f(oc.n.y)
            );
            rk(ray, cap, &format!("degenerate r={} iter {i}", f(r)));
            // Origin exactly at the degenerate point.
            rk(
                C2Ray { p: p0, d: C2v::new(1.0, 0.0), t: 5.0 },
                cap,
                &format!("degenerate-at r={} iter {i}", f(r)),
            );
        }
        // Signed-zero difference: (0,0) - (-0,-0) is (0,0), same divide by zero.
        rk(
            C2Ray { p: C2v::new(-3.0, 0.0), d: C2v::new(1.0, 0.0), t: 6.0 },
            C2Capsule { a: C2v::new(0.0, 0.0), b: C2v::new(-0.0, -0.0), r: 1.0 },
            &format!("signed-zero axis {i}"),
        );
    }
}

#[test]
fn err_29_raytocapsule_zero_radius() {
    let a = apis();
    // r == 0: capsule_bb is a zero-width slab (min.x == -0.0, max.x == 0.0),
    // `abs(yAp.x) < 0` is false, and `min(|yAe.x|,|yAp.x|) < 0` is false. So the
    // early accepts and the cap delegates are all unreachable and the capsule
    // degenerates to its axis SEGMENT: a ray hits only if it crosses x == 0 at
    // a y inside [0, yBb.y]. A ray aimed at the midpoint therefore still hits,
    // which is why this row checks both outcomes rather than assuming a reject.
    let mut rng = Rng::new(129);
    let mut rejected = 0usize;
    let mut accepted = 0usize;
    for i in 0..5000 {
        let base = rand_cap(&mut rng);
        for &r in &[0.0f32, -0.0] {
            let cap = C2Capsule { r, ..base };
            let mid = C2v::new((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
            let start = C2v::new(rng.range(-25.0, 25.0), rng.range(-25.0, 25.0));
            // Aimed exactly at the midpoint: crosses the axis inside the
            // segment, so this ACCEPTS even with a zero radius.
            let (dx, dy) = (mid.x - start.x, mid.y - start.y);
            let l = (dx * dx + dy * dy).sqrt();
            let aimed = C2Ray { p: start, d: C2v::new(dx / l, dy / l), t: l * 2.0 };
            let mut oc = C2Raycast::poison();
            if unsafe { (a.c.c2RaytoCapsule)(aimed, cap, &mut oc) } == 1 {
                accepted += 1;
            }
            rk(aimed, cap, &format!("r={} aimed {i}", f(r)));
            // Random bearing: usually crosses x == 0 outside [0, yBb.y], or not
            // at all, so it rejects.
            let ang = rng.range(0.0, 6.2831855);
            let rnd = C2Ray { p: start, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(0.0, 60.0) };
            let mut oc2 = C2Raycast::poison();
            if unsafe { (a.c.c2RaytoCapsule)(rnd, cap, &mut oc2) } == 0 {
                rejected += 1;
            }
            rk(rnd, cap, &format!("r={} random {i}", f(r)));
            // Origin exactly on the axis: yAp.x == 0, but `0 < 0` is false, and
            // c2CircleToPoint with r == 0 also rejects, so the early accepts
            // still do not fire.
            rk(
                C2Ray { p: mid, d: C2v::new(1.0, 0.0), t: 10.0 },
                cap,
                &format!("r={} on-axis {i}", f(r)),
            );
            rk(
                C2Ray { p: cap.a, d: C2v::new(1.0, 0.0), t: 10.0 },
                cap,
                &format!("r={} at-a {i}", f(r)),
            );
            // Aimed just past an end cap: crosses x == 0 outside the segment.
            let (mx, myv, ll) = cap_frame(cap);
            let beyond = cap_local_to_world(cap, mx, myv, 0.0, ll + rng.range(0.5, 5.0));
            let (bx, by) = (beyond.x - start.x, beyond.y - start.y);
            let bl = (bx * bx + by * by).sqrt();
            rk(
                C2Ray { p: start, d: C2v::new(bx / bl, by / bl), t: bl * 2.0 },
                cap,
                &format!("r={} aimed-beyond {i}", f(r)),
            );
        }
    }
    assert!(rejected > 100, "only {rejected} zero-radius capsule casts rejected; row 29 is weak");
    assert!(accepted > 100, "only {accepted} zero-radius axis crossings accepted; row 29 is weak");
}

#[test]
fn err_30_raytocapsule_negative_radius() {
    // r < 0 makes capsule_bb inverted (min.x = -r > 0 = ... > max.x = r), while
    // c2CircleToPoint squares r and so treats it as |r|. The two halves of the
    // function therefore disagree about the capsule's size; no rejection or
    // normalisation happens.
    let mut rng = Rng::new(130);
    for i in 0..5000 {
        let base = rand_cap(&mut rng);
        for &r in &[-base.r, -1.0f32, -1e-6, -1e6, -f32::MAX, f32::NEG_INFINITY] {
            let cap = C2Capsule { r, ..base };
            let mid = C2v::new((cap.a.x + cap.b.x) * 0.5, (cap.a.y + cap.b.y) * 0.5);
            let start = C2v::new(rng.range(-25.0, 25.0), rng.range(-25.0, 25.0));
            let (dx, dy) = (mid.x - start.x, mid.y - start.y);
            let l = (dx * dx + dy * dy).sqrt();
            rk(
                C2Ray { p: start, d: C2v::new(dx / l, dy / l), t: l * 2.0 },
                cap,
                &format!("r={} aimed {i}", f(r)),
            );
            let ang = rng.range(0.0, 6.2831855);
            rk(
                C2Ray { p: start, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(-5.0, 50.0) },
                cap,
                &format!("r={} random {i}", f(r)),
            );
            // Origin inside |r| of the axis, where the two halves disagree.
            let (mx, my, ll) = cap_frame(cap);
            let p = cap_local_to_world(cap, mx, my, r * 0.5, ll * 0.5);
            rk(
                C2Ray { p, d: C2v::new(1.0, 0.0), t: 10.0 },
                cap,
                &format!("r={} inside-|r| {i}", f(r)),
            );
        }
    }
}

#[test]
fn err_31_raytocapsule_origin_inside() {
    let a = apis();
    // The three early `return 1`s (lines 246, 255, 257) all report a hit with
    // out->t == 0 and out->n == c2Norm(b - a) -- NOT the surface normal. That
    // is a quirk, so assert the exact values, not just the return code.
    let mut rng = Rng::new(131);
    let mut in_slab = 0usize;
    let mut in_cap = 0usize;
    for i in 0..5000 {
        let cap = rand_cap(&mut rng);
        let (mx, my, l) = cap_frame(cap);
        let expect_n = unsafe { (a.c.c2Norm)((a.c.c2Sub)(cap.b, cap.a)) };
        // Inside the slab rectangle.
        let p = cap_local_to_world(cap, mx, my, cap.r * rng.range(-0.9, 0.9), l * rng.range(0.05, 0.95));
        let ang = rng.range(0.0, 6.2831855);
        let ray = C2Ray { p, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(0.0, 20.0) };
        let mut oc = C2Raycast::poison();
        let ret = unsafe { (a.c.c2RaytoCapsule)(ray, cap, &mut oc) };
        if ret == 1 && oc.t.to_bits() & 0x7FFF_FFFF == 0 {
            in_slab += 1;
            assert_eq!(
                (oc.n.x.to_bits(), oc.n.y.to_bits()),
                (expect_n.x.to_bits(), expect_n.y.to_bits()),
                "an early capsule hit reports c2Norm(b - a), not the surface normal (iter {i})"
            );
        }
        rk(ray, cap, &format!("in-slab {i}"));
        // Strictly inside end cap A (v < 0, within r of point a).
        let rr = cap.r * rng.range(0.0, 0.9);
        let ca = cap_local_to_world(cap, mx, my, rr * 0.3, -rr * 0.9);
        let ray2 = C2Ray { p: ca, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(0.0, 20.0) };
        let mut oc2 = C2Raycast::poison();
        let ret2 = unsafe { (a.c.c2RaytoCapsule)(ray2, cap, &mut oc2) };
        if ret2 == 1 && oc2.t.to_bits() & 0x7FFF_FFFF == 0 {
            in_cap += 1;
        }
        rk(ray2, cap, &format!("in-cap-a {i}"));
        // Strictly inside end cap B.
        let cb = cap_local_to_world(cap, mx, my, rr * 0.3, l + rr * 0.9);
        rk(
            C2Ray { p: cb, d: C2v::new(ang.cos(), ang.sin()), t: rng.range(0.0, 20.0) },
            cap,
            &format!("in-cap-b {i}"),
        );
    }
    assert!(in_slab > 100, "only {in_slab} in-slab early hits; row 31 is weak");
    assert!(in_cap > 10, "only {in_cap} in-cap early hits; row 31 is weak");
}

#[test]
fn err_32_raytocapsule_zero_denominator() {
    let a = apis();
    // Line 278 computes `t = (c - yAp.x) / (yAe.x - yAp.x)` with NO zero guard,
    // unlike line 132.
    //
    // An EXACT zero denominator turns out to be unreachable here: reaching line
    // 278 requires `abs(yAp.x) >= B.r` together with either `yAe.x*yAp.x < 0`
    // (opposite signs, so the difference is non-zero) or
    // `min(|yAe.x|,|yAp.x|) < B.r` (so |yAe.x| < B.r <= |yAp.x|, again
    // different). What IS reachable is a NON-FINITE denominator (inf - inf, or
    // a NaN operand), which is the same missing-guard defect. This test drives
    // that and asserts a non-finite `t` really occurs.
    let mut rng = Rng::new(132);
    let mut nonfinite_t = 0usize;
    let mut exact_zero_denom = 0usize;
    for i in 0..30000 {
        let cap = if i % 3 == 0 {
            C2Capsule { a: rng.v_path(), b: rng.v_path(), r: rng.pathological() }
        } else {
            let base = rand_cap(&mut rng);
            C2Capsule { r: rng.pathological(), ..base }
        };
        let ray = C2Ray {
            p: rng.v_path(),
            d: rng.v_path(),
            t: rng.pathological(),
        };
        // Recompute yAp/yAe with the C's own exported helpers to see which
        // denominator the C would form.
        let cap_n = unsafe { (a.c.c2Sub)(cap.b, cap.a) };
        let my = unsafe { (a.c.c2Norm)(cap_n) };
        let m = C2m { x: unsafe { (a.c.c2CCW90)(my) }, y: my };
        let yap = unsafe { (a.c.c2MulmvT)(m, (a.c.c2Sub)(ray.p, cap.a)) };
        let yad = unsafe { (a.c.c2MulmvT)(m, ray.d) };
        let yae = unsafe { (a.c.c2Add)(yap, (a.c.c2Mulvs)(yad, ray.t)) };
        let denom = yae.x - yap.x;
        if denom == 0.0 {
            exact_zero_denom += 1;
        }
        if !denom.is_finite() {
            nonfinite_t += 1;
        }
        rk(ray, cap, &format!("denominator probe {i}"));
        // Direction exactly parallel to the axis: yAd.x == 0, so yAe.x == yAp.x
        // and the denominator is exactly zero -- but line 260 then rejects, so
        // line 278 is not reached. Verified as a differential case regardless.
        let (mx, myv, l) = cap_frame(cap);
        if l.is_finite() && l > 0.0 {
            let u = cap.r * 3.0;
            if u.is_finite() {
                let p = cap_local_to_world(cap, mx, myv, u, l * 0.5);
                rk(
                    C2Ray { p, d: myv, t: 5.0 },
                    cap,
                    &format!("parallel-to-axis {i}"),
                );
                // A.t == 0: yAe == yAp exactly.
                rk(
                    C2Ray { p, d: C2v::new(1.0, 0.0), t: 0.0 },
                    cap,
                    &format!("A.t==0 {i}"),
                );
            }
        }
    }
    assert!(
        nonfinite_t > 100,
        "only {nonfinite_t} non-finite denominators produced; row 32 is not probing the missing guard"
    );
    // Informational: exact zeros do occur upstream of line 278, they just never
    // reach it (see the comment above). Recorded so the claim is evidence-based.
    assert!(
        exact_zero_denom > 0,
        "expected some exactly-zero denominators upstream, saw none"
    );
}

#[test]
fn err_33_raytocapsule_delegate_rejects() {
    let a = apis();
    // When c2RaytoCapsule delegates to c2RaytoCircle and the CIRCLE rejects,
    // the capsule returns 0 -- and *out still holds the line 243-244 values,
    // because c2RaytoCircle leaves it alone on its reject paths. Both facts
    // must hold in the Rust.
    let mut rng = Rng::new(133);
    let mut found = 0usize;
    for i in 0..8000 {
        let cap = rand_cap(&mut rng);
        let (mx, my, l) = cap_frame(cap);
        let expect_n = unsafe { (a.c.c2Norm)((a.c.c2Sub)(cap.b, cap.a)) };
        // Origin within r of the axis line but beyond the end cap, aimed AWAY
        // from the capsule: the delegate's `t >= 0` test fails.
        let u = cap.r * rng.range(-0.9, 0.9);
        let v = -(cap.r + rng.range(2.0, 20.0));
        let p = cap_local_to_world(cap, mx, my, u, v);
        let away = C2v::new(-my.x, -my.y);
        let ray = C2Ray { p, d: away, t: rng.range(0.0, 30.0) };
        let mut oc = C2Raycast::poison();
        let ret = unsafe { (a.c.c2RaytoCapsule)(ray, cap, &mut oc) };
        if ret == 0 {
            found += 1;
            assert_eq!(
                (oc.t.to_bits() & 0x7FFF_FFFF, oc.n.x.to_bits(), oc.n.y.to_bits()),
                (0, expect_n.x.to_bits(), expect_n.y.to_bits()),
                "after a delegate rejection, *out must still hold the line 243-244 values (iter {i})"
            );
        }
        rk(ray, cap, &format!("delegate-reject away {i}"));
        // Aimed at the capsule but with A.t too short for the delegate to reach.
        let toward = C2v::new(my.x, my.y);
        rk(
            C2Ray { p, d: toward, t: rng.range(0.0, 0.5) },
            cap,
            &format!("delegate-reject short {i}"),
        );
        // Same, past the far cap.
        let p2 = cap_local_to_world(cap, mx, my, u, l + cap.r + rng.range(2.0, 20.0));
        rk(
            C2Ray { p: p2, d: toward, t: rng.range(0.0, 0.5) },
            cap,
            &format!("delegate-reject far-short {i}"),
        );
    }
    assert!(found > 100, "only {found} delegate rejections observed; row 33 needs a better generator");
}

// ===========================================================================
// Rows 34-37 — c2CastRay dispatch and the missing null checks
// ===========================================================================

#[test]
fn err_34_castray_out_of_range_tag() {
    let a = apis();
    // C enums accept any int, so a tag outside {0,1,2} is a real input. The
    // `switch` has no `default` and no trailing `return`, so the compiled C
    // executes a bare `leave; ret` that never writes %eax: the caller observes
    // its OWN leftover %eax. The Rust reproduces this by reading %eax without
    // writing it, so both agree from an identical call site -- which is the
    // only sense in which this UB can be matched.
    let poison = C2Raycast::poison().bits();
    let mut rng = Rng::new(134);
    let tags: [u32; 12] = [3, 4, 5, 7, 8, 100, 255, 256, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFE, 0xFFFF_FFFF];
    for i in 0..2000 {
        let ray = C2Ray { p: rng.v(), d: rng.v(), t: rng.interesting() };
        let circle = C2Circle { p: rng.v(), r: rng.interesting() };
        let aabb = C2AABB { min: rng.v(), max: rng.v() };
        let cap = C2Capsule { a: rng.v(), b: rng.v(), r: rng.interesting() };
        for &tag in &tags {
            // Same shape object behind B for both calls, and the same call site
            // shape, so %eax at the boundary is the same for C and Rust.
            let mut oc = C2Raycast::poison();
            let mut or = C2Raycast::poison();
            let pc = &circle as *const _ as *const c_void;
            let rcv = unsafe { (a.c.c2CastRay)(ray, pc, tag, &mut oc) };
            let rrv = unsafe { (a.rust.c2CastRay)(ray, pc, tag, &mut or) };
            assert_eq!(
                oc.bits(),
                poison,
                "tag {tag}: the C must not write *out when it falls off the end (iter {i})"
            );
            assert_eq!(
                or.bits(),
                poison,
                "tag {tag}: the Rust must not write *out either (iter {i})"
            );
            assert_eq!(
                rcv, rrv,
                "tag {tag}: fall-through return mismatch, C={rcv} Rust={rrv} (iter {i})"
            );
            // Repeat with the other two payload types behind B; the tag is what
            // matters, not the object.
            for p in [
                &aabb as *const _ as *const c_void,
                &cap as *const _ as *const c_void,
                std::ptr::null(),
            ] {
                let mut o1 = C2Raycast::poison();
                let mut o2 = C2Raycast::poison();
                let x = unsafe { (a.c.c2CastRay)(ray, p, tag, &mut o1) };
                let y = unsafe { (a.rust.c2CastRay)(ray, p, tag, &mut o2) };
                assert_eq!(x, y, "tag {tag} with alt payload: C={x} Rust={y} (iter {i})");
                assert_eq!(o1.bits(), poison, "tag {tag}: C wrote *out");
                assert_eq!(o2.bits(), poison, "tag {tag}: Rust wrote *out");
            }
        }
        // The three VALID tags must still dispatch correctly, so this test also
        // pins that an out-of-range guard was not added too eagerly.
        cmp_castray(ray, &circle, C2_TYPE_CIRCLE, &format!("valid circle {i}"));
        cmp_castray(ray, &aabb, C2_TYPE_AABB, &format!("valid aabb {i}"));
        cmp_castray(ray, &cap, C2_TYPE_CAPSULE, &format!("valid capsule {i}"));
    }
}

#[test]
fn err_36_castray_tag_shape_mismatch() {
    // c2CastRay does no size or tag validation: it just casts `B`. Passing a
    // tag that claims a LARGER object than the allocation reads past the end.
    // To keep the test itself defined, `B` always points into a buffer big
    // enough for the largest shape (c2Capsule, 20 bytes) while the tag and the
    // meaningful prefix disagree.
    let mut rng = Rng::new(136);
    for i in 0..3000 {
        // A 20-byte buffer whose first 12 bytes are a valid c2Circle and whose
        // tail is deterministic filler. Reading it as a c2AABB or c2Capsule is
        // exactly what the C would do.
        #[repr(C)]
        #[derive(Clone, Copy, Debug)]
        struct Big {
            f: [f32; 5],
        }
        let big = Big {
            f: [
                rng.interesting(),
                rng.interesting(),
                rng.interesting(),
                rng.interesting(),
                rng.interesting(),
            ],
        };
        let ray = C2Ray { p: rng.v(), d: rng.v(), t: rng.interesting() };
        for &tag in &[C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
            cmp_castray(ray, &big, tag, &format!("tag/shape mismatch tag={tag} iter {i}"));
        }
        // Same buffer read through each concrete type directly, to confirm the
        // dispatcher's reinterpretation matches a direct call in both libraries.
        let as_circle: C2Circle = unsafe { std::mem::transmute_copy(&big) };
        let as_aabb: C2AABB = unsafe { std::mem::transmute_copy(&big) };
        let as_cap: C2Capsule = unsafe { std::mem::transmute_copy(&big) };
        rc(ray, as_circle, &format!("reinterpreted circle {i}"));
        rb(ray, as_aabb, &format!("reinterpreted aabb {i}"));
        rk(ray, as_cap, &format!("reinterpreted capsule {i}"));
    }
}

#[test]
fn err_37_null_out_on_early_return_paths() {
    let a = apis();
    // There is NO null check anywhere in the C. On paths that return before
    // writing, `out == NULL` is harmless and both libraries must return the
    // same value. Paths that DO write are deliberately not exercised with NULL:
    // the C segfaults there, which is not a comparable observable.
    let mut rng = Rng::new(137);
    for i in 0..3000 {
        // c2RaytoCircle rows 1-3: disc < 0, t < 0, t > A.t.
        let r = rng.range(0.1, 4.0);
        let circle = C2Circle { p: C2v::new(0.0, 0.0), r };
        let no_write_circle = [
            // disc < 0
            C2Ray { p: C2v::new(-20.0, r + 5.0), d: C2v::new(1.0, 0.0), t: 100.0 },
            // t < 0 (origin inside)
            C2Ray { p: C2v::new(0.0, 0.0), d: C2v::new(1.0, 0.0), t: 100.0 },
            // t > A.t
            C2Ray { p: C2v::new(-20.0, 0.0), d: C2v::new(1.0, 0.0), t: 0.1 },
        ];
        for (j, &ray) in no_write_circle.iter().enumerate() {
            let mut probe = C2Raycast::poison();
            let ret = unsafe { (a.c.c2RaytoCircle)(ray, circle, &mut probe) };
            assert_eq!(ret, 0, "case {j} should reject (iter {i})");
            assert_eq!(probe.bits(), C2Raycast::poison().bits(), "case {j} should not write");
            cmp_ray_null_out(
                "c2RaytoCircle",
                a.c.c2RaytoCircle,
                a.rust.c2RaytoCircle,
                ray,
                circle,
                &format!("NULL out, circle case {j} iter {i}"),
            );
        }
        // c2RaytoAABB rows 11-13.
        let b = C2AABB { min: C2v::new(0.0, 0.0), max: C2v::new(1.0, 1.0) };
        let no_write_aabb = [
            // broad-phase reject: travelling away
            C2Ray { p: C2v::new(50.0, 0.5), d: C2v::new(1.0, 0.0), t: 4.0 },
            C2Ray { p: C2v::new(0.5, -50.0), d: C2v::new(0.0, -1.0), t: 4.0 },
            // hit == 0 via NaN origin with a finite endpoint outside
            C2Ray { p: C2v::new(f32::NAN, f32::NAN), d: C2v::new(1e9, 1e9), t: 1e9 },
        ];
        for (j, &ray) in no_write_aabb.iter().enumerate() {
            let mut probe = C2Raycast::poison();
            let ret = unsafe { (a.c.c2RaytoAABB)(ray, b, &mut probe) };
            if ret == 0 && probe.bits() == C2Raycast::poison().bits() {
                cmp_ray_null_out(
                    "c2RaytoAABB",
                    a.c.c2RaytoAABB,
                    a.rust.c2RaytoAABB,
                    ray,
                    b,
                    &format!("NULL out, aabb case {j} iter {i}"),
                );
            }
        }
        // c2CastRay with an out-of-range tag never touches *out at all.
        for &tag in &[3u32, 99, 0xFFFF_FFFF] {
            let x = unsafe { (a.c.c2CastRay)(C2Ray::default(), std::ptr::null(), tag, std::ptr::null_mut()) };
            let y = unsafe { (a.rust.c2CastRay)(C2Ray::default(), std::ptr::null(), tag, std::ptr::null_mut()) };
            assert_eq!(x, y, "NULL out + tag {tag}: C={x} Rust={y} (iter {i})");
        }
        // spec_ray whose delegate rejects: also never writes.
        let cx = rng.range(-10.0, 10.0);
        let args = [cx - 30.0, 0.0, cx, 0.0, 0.5, cx - 40.0, 20.0];
        let mut probe = C2Raycast::poison();
        let ret = unsafe {
            (a.c.spec_ray)(&mut probe, args[0], args[1], args[2], args[3], args[4], args[5], args[6])
        };
        if ret == 0 && probe.bits() == C2Raycast::poison().bits() {
            let x = unsafe {
                (a.c.spec_ray)(std::ptr::null_mut(), args[0], args[1], args[2], args[3], args[4], args[5], args[6])
            };
            let y = unsafe {
                (a.rust.spec_ray)(std::ptr::null_mut(), args[0], args[1], args[2], args[3], args[4], args[5], args[6])
            };
            assert_eq!(x, y, "NULL out spec_ray: C={x} Rust={y} (iter {i})");
        }
    }
    // Row 35 of ERRORS.md -- the `return 0;` after the c2RaytoCapsule case at
    // line 302 -- is provably dead: the preceding `return` always executes for
    // tag 2, and any other tag skips the case entirely. It is documented in the
    // table and covered by row 34's fall-through behaviour; no separate test is
    // possible because no input can reach it.
}

// ===========================================================================
// Rows 38-40 — spec_ray rejections
// ===========================================================================

#[test]
fn err_38_specray_mp_equals_origin() {
    let a = apis();
    // mp == ray.p -> c2Sub is (0,0) -> c2Norm divides by zero -> ray.d is
    // NaN (0 * inf) -> ray.t is NaN. Every comparison in c2RaytoCircle is then
    // false, so it rejects without writing.
    let poison = C2Raycast::poison().bits();
    let mut rng = Rng::new(138);
    for i in 0..3000 {
        let px = rng.range(-20.0, 20.0);
        let py = rng.range(-20.0, 20.0);
        let cx = rng.range(-20.0, 20.0);
        let cy = rng.range(-20.0, 20.0);
        let r = rng.range(0.1, 5.0);
        let mut oc = C2Raycast::poison();
        let ret = unsafe { (a.c.spec_ray)(&mut oc, px, py, cx, cy, r, px, py) };
        assert_eq!(ret, 0, "mp == ray.p makes d and t NaN, so it must reject (iter {i})");
        assert_eq!(oc.bits(), poison, "the NaN reject must not write *out (iter {i})");
        cmp_specray([px, py, cx, cy, r, px, py], &format!("mp==origin {i}"));
        // Signed-zero variant: (0,0) - (-0,-0) is still (0,0).
        cmp_specray([0.0, 0.0, cx, cy, r, -0.0, -0.0], &format!("mp==origin signed zero {i}"));
        // Origin inside the circle AND mp == origin.
        cmp_specray([cx, cy, cx, cy, r, cx, cy], &format!("mp==origin==centre {i}"));
    }
}

#[test]
fn err_39_specray_ray_too_short() {
    let a = apis();
    // ray.t = dot(mp, d) - dot(ray.p, d) is the distance to the mouse point, so
    // a mouse point short of the circle makes the delegate's `t <= A.t` fail.
    let mut rng = Rng::new(139);
    let poison = C2Raycast::poison().bits();
    let mut found = 0usize;
    for i in 0..4000 {
        let origin = C2v::new(rng.range(-20.0, 20.0), rng.range(-20.0, 20.0));
        let ang = rng.range(0.0, 6.2831855);
        let r = rng.range(0.1, 4.0);
        let d_centre = r + rng.range(2.0, 15.0);
        let centre = C2v::new(origin.x + d_centre * ang.cos(), origin.y + d_centre * ang.sin());
        let d_short = (d_centre - r) * rng.range(0.0, 0.98);
        let mp = C2v::new(origin.x + d_short * ang.cos(), origin.y + d_short * ang.sin());
        let mut oc = C2Raycast::poison();
        let ret = unsafe { (a.c.spec_ray)(&mut oc, mp.x, mp.y, centre.x, centre.y, r, origin.x, origin.y) };
        if ret == 0 {
            found += 1;
            assert_eq!(oc.bits(), poison, "a too-short ray must not write *out (iter {i})");
        }
        cmp_specray([mp.x, mp.y, centre.x, centre.y, r, origin.x, origin.y], &format!("short {i}"));
        // Mouse point exactly at the near surface: `t == A.t`, the ACCEPTING
        // side of the inclusive comparison.
        let d_exact = d_centre - r;
        let mpe = C2v::new(origin.x + d_exact * ang.cos(), origin.y + d_exact * ang.sin());
        cmp_specray([mpe.x, mpe.y, centre.x, centre.y, r, origin.x, origin.y], &format!("exact {i}"));
    }
    assert!(found > 100, "only {found} too-short rejections; row 39 is weak");
}

#[test]
fn err_40_specray_degenerate_scalars() {
    let a = apis();
    let mut rng = Rng::new(140);
    let poison = C2Raycast::poison().bits();
    for i in 0..2000 {
        let origin = C2v::new(rng.range(-10.0, 10.0), rng.range(-10.0, 10.0));
        let ang = rng.range(0.0, 6.2831855);
        let d_centre = rng.range(2.0, 15.0);
        let centre = C2v::new(origin.x + d_centre * ang.cos(), origin.y + d_centre * ang.sin());
        let mp = C2v::new(origin.x + 3.0 * d_centre * ang.cos(), origin.y + 3.0 * d_centre * ang.sin());
        // Zero and negative radii.
        for &r in &[0.0f32, -0.0, -1.0, -d_centre, f32::MIN_POSITIVE, f32::from_bits(1)] {
            cmp_specray([mp.x, mp.y, centre.x, centre.y, r, origin.x, origin.y], &format!("r={} {i}", f(r)));
        }
        // NaN in each of the seven scalars: the C must reject and leave *out
        // alone in every case.
        for &n in exotic_f32().iter().filter(|v| v.is_nan()) {
            let base = [mp.x, mp.y, centre.x, centre.y, 1.0, origin.x, origin.y];
            for k in 0..7 {
                let mut args = base;
                args[k] = n;
                let mut oc = C2Raycast::poison();
                let ret = unsafe {
                    (a.c.spec_ray)(&mut oc, args[0], args[1], args[2], args[3], args[4], args[5], args[6])
                };
                assert_eq!(ret, 0, "NaN in argument {k} must reject (iter {i})");
                assert_eq!(oc.bits(), poison, "NaN in argument {k} must not write *out (iter {i})");
                cmp_specray(args, &format!("NaN arg {k} payload {:#010x} iter {i}", n.to_bits()));
            }
        }
        // Infinities in each argument.
        for &v in &[f32::INFINITY, f32::NEG_INFINITY, f32::MAX, -f32::MAX] {
            let base = [mp.x, mp.y, centre.x, centre.y, 1.0, origin.x, origin.y];
            for k in 0..7 {
                let mut args = base;
                args[k] = v;
                cmp_specray(args, &format!("inf/max arg {k} = {} iter {i}", f(v)));
            }
        }
    }
}

// ===========================================================================
// Rows 41-46 — L0 arithmetic sentinels and ternary semantics
// ===========================================================================

#[test]
fn err_41_div_norm_by_zero() {
    let a = apis();
    // `c2Div` is `c2Mulvs(a, 1.0f / b)` with no guard, so b == 0 gives +-inf
    // components, and 0 * inf gives NaN. `c2Norm` inherits this via c2Len.
    for &b in &[0.0f32, -0.0] {
        for &v in &[
            C2v::new(1.0, 1.0),
            C2v::new(-1.0, 2.0),
            C2v::new(0.0, 0.0),
            C2v::new(-0.0, 0.0),
            C2v::new(f32::MAX, -f32::MAX),
            C2v::new(f32::MIN_POSITIVE, 1.0),
        ] {
            let c = unsafe { (a.c.c2Div)(v, b) };
            let r = unsafe { (a.rust.c2Div)(v, b) };
            assert_eq!(
                c.bits(),
                r.bits(),
                "c2Div by {}: v=({},{}) C=({},{}) Rust=({},{})",
                f(b), f(v.x), f(v.y), f(c.x), f(c.y), f(r.x), f(r.y)
            );
            assert!(
                !c.x.is_finite() || c.x == 0.0,
                "dividing by {} should not give a finite non-zero, got {}",
                f(b),
                f(c.x)
            );
        }
    }
    // c2Norm of the zero vector: c2Len is 0, so 1/0 is inf and 0 * inf is NaN.
    for &v in &[
        C2v::new(0.0, 0.0),
        C2v::new(-0.0, -0.0),
        C2v::new(0.0, -0.0),
        C2v::new(-0.0, 0.0),
    ] {
        let c = unsafe { (a.c.c2Norm)(v) };
        let r = unsafe { (a.rust.c2Norm)(v) };
        assert_eq!(
            c.bits(),
            r.bits(),
            "c2Norm of the zero vector ({},{}): C=({},{}) Rust=({},{})",
            f(v.x), f(v.y), f(c.x), f(c.y), f(r.x), f(r.y)
        );
        assert!(c.x.is_nan(), "c2Norm(0,0).x should be NaN (0 * inf), got {}", f(c.x));
    }
    // Denormal lengths, where the reciprocal overflows to inf.
    let mut rng = Rng::new(141);
    for i in 0..4000 {
        let tiny = f32::from_bits(rng.next_u32() % 0x0080_0000);
        let v = C2v::new(tiny, tiny);
        cmp_v1("c2Norm", a.c.c2Norm, a.rust.c2Norm, v, &format!("denormal {i}"));
        cmp_vs("c2Div", a.c.c2Div, a.rust.c2Div, rng.v_path(), tiny, &format!("denormal divisor {i}"));
    }
}

#[test]
fn err_42_div_norm_nonfinite() {
    let a = apis();
    // 1/inf == 0 so c2Div by inf gives a (signed) zero vector; 1/NaN == NaN so
    // it gives a NaN vector. Neither is an error return -- there is nothing to
    // return one through.
    for &b in &[f32::INFINITY, f32::NEG_INFINITY, f32::NAN, -f32::NAN] {
        for &v in &[
            C2v::new(1.0, -1.0),
            C2v::new(0.0, -0.0),
            C2v::new(f32::MAX, f32::MIN_POSITIVE),
            C2v::new(f32::INFINITY, f32::NEG_INFINITY),
            C2v::new(f32::NAN, 1.0),
        ] {
            cmp_vs("c2Div", a.c.c2Div, a.rust.c2Div, v, b, &format!("b={}", f(b)));
        }
    }
    for &v in &[
        C2v::new(f32::INFINITY, 0.0),
        C2v::new(f32::INFINITY, f32::INFINITY),
        C2v::new(f32::NEG_INFINITY, 1.0),
        C2v::new(f32::NAN, 0.0),
        C2v::new(f32::MAX, f32::MAX),
        C2v::new(-f32::MAX, f32::MAX),
    ] {
        cmp_v1("c2Norm", a.c.c2Norm, a.rust.c2Norm, v, "nonfinite");
    }
    let mut rng = Rng::new(142);
    for i in 0..4000 {
        cmp_vs("c2Div", a.c.c2Div, a.rust.c2Div, rng.v_path(), rng.pathological(), &format!("rand {i}"));
        cmp_v1("c2Norm", a.c.c2Norm, a.rust.c2Norm, rng.v_path(), &format!("rand {i}"));
    }
}

#[test]
fn err_43_len_overflow() {
    let a = apis();
    // c2Dot(a, a) overflows to inf long before c2Len would, so sqrtf(inf) is
    // inf: c2Len saturates rather than reporting anything.
    for &v in &[
        C2v::new(f32::MAX, 0.0),
        C2v::new(f32::MAX, f32::MAX),
        C2v::new(-f32::MAX, f32::MAX),
        C2v::new(1e30, 1e30),
        C2v::new(f32::INFINITY, 0.0),
        C2v::new(f32::INFINITY, f32::NEG_INFINITY),
    ] {
        let c = unsafe { (a.c.c2Len)(v) };
        let r = unsafe { (a.rust.c2Len)(v) };
        assert_eq!(c.to_bits(), r.to_bits(), "c2Len({},{}): C={} Rust={}", f(v.x), f(v.y), f(c), f(r));
    }
    let c = unsafe { (a.c.c2Len)(C2v::new(f32::MAX, 0.0)) };
    assert_eq!(c, f32::INFINITY, "c2Len(f32::MAX, 0) should saturate to inf, got {}", f(c));
    // Underflow: squaring a denormal gives exactly 0, so c2Len reports 0 for a
    // non-zero vector.
    let tiny = C2v::new(f32::from_bits(1), f32::from_bits(1));
    let c = unsafe { (a.c.c2Len)(tiny) };
    let r = unsafe { (a.rust.c2Len)(tiny) };
    assert_eq!(c.to_bits(), r.to_bits(), "c2Len of a denormal: C={} Rust={}", f(c), f(r));
    assert_eq!(c, 0.0, "c2Len of a denormal underflows to 0, got {}", f(c));
}

#[test]
fn err_44_len_nan() {
    let a = apis();
    // sqrtf's argument is dot(a,a), which is never negative, so the only NaN
    // route is a NaN input. There is no errno/domain path to reproduce.
    for &n in exotic_f32().iter().filter(|v| v.is_nan()) {
        for &v in &[
            C2v::new(n, 0.0),
            C2v::new(0.0, n),
            C2v::new(n, n),
            C2v::new(n, f32::INFINITY),
        ] {
            let c = unsafe { (a.c.c2Len)(v) };
            let r = unsafe { (a.rust.c2Len)(v) };
            assert_eq!(
                c.to_bits(),
                r.to_bits(),
                "c2Len with NaN payload {:#010x}: C={} Rust={}",
                n.to_bits(),
                f(c),
                f(r)
            );
        }
    }
    // inf + -inf inside the dot product also yields NaN without a NaN input.
    let v = C2v::new(f32::INFINITY, f32::INFINITY);
    let c = unsafe { (a.c.c2Dot)(v, C2v::new(1.0, -1.0)) };
    let r = unsafe { (a.rust.c2Dot)(v, C2v::new(1.0, -1.0)) };
    assert_eq!(c.to_bits(), r.to_bits(), "inf - inf in c2Dot: C={} Rust={}", f(c), f(r));
    assert!(c.is_nan(), "inf + -inf should be NaN, got {}", f(c));
}

#[test]
fn err_45_ternary_nan_semantics() {
    let a = apis();
    // The C spells min/max/abs as ternaries, so NaN does NOT behave like
    // fminf/fmaxf/fabsf: `NaN < x` is false, so c2Minv returns the SECOND
    // operand, and c2Absv returns NaN with its sign bit unchanged.
    for &n in exotic_f32().iter().filter(|v| v.is_nan()) {
        let finite = C2v::new(1.0, -2.0);
        let nanv = C2v::new(n, n);
        // c2Minv(NaN, finite) must return the finite operand ...
        let c = unsafe { (a.c.c2Minv)(nanv, finite) };
        let r = unsafe { (a.rust.c2Minv)(nanv, finite) };
        assert_eq!(c.bits(), r.bits(), "c2Minv(NaN, finite)");
        assert_eq!(
            c.bits(),
            finite.bits(),
            "the ternary picks the second operand when the first is NaN, got ({},{})",
            f(c.x),
            f(c.y)
        );
        // ... and c2Minv(finite, NaN) must return the NaN operand.
        let c = unsafe { (a.c.c2Minv)(finite, nanv) };
        let r = unsafe { (a.rust.c2Minv)(finite, nanv) };
        assert_eq!(c.bits(), r.bits(), "c2Minv(finite, NaN)");
        assert_eq!(c.bits(), nanv.bits(), "the ternary picks the NaN second operand");
        // Same asymmetry for c2Maxv.
        let c = unsafe { (a.c.c2Maxv)(nanv, finite) };
        let r = unsafe { (a.rust.c2Maxv)(nanv, finite) };
        assert_eq!(c.bits(), r.bits(), "c2Maxv(NaN, finite)");
        assert_eq!(c.bits(), finite.bits(), "c2Maxv also picks the second operand");
        let c = unsafe { (a.c.c2Maxv)(finite, nanv) };
        let r = unsafe { (a.rust.c2Maxv)(finite, nanv) };
        assert_eq!(c.bits(), r.bits(), "c2Maxv(finite, NaN)");
        // c2Absv(NaN) keeps the payload AND the sign bit.
        let c = unsafe { (a.c.c2Absv)(nanv) };
        let r = unsafe { (a.rust.c2Absv)(nanv) };
        assert_eq!(c.bits(), r.bits(), "c2Absv(NaN) payload/sign");
        assert_eq!(
            c.x.to_bits(),
            n.to_bits(),
            "c2Absv leaves a NaN untouched (sign bit included); fabsf would clear it"
        );
    }
}

#[test]
fn err_46_ternary_signed_zero() {
    let a = apis();
    // `-0.0 < 0` is false, so c2Absv(-0.0) returns -0.0 -- fabsf would give
    // +0.0. And c2Minv(-0.0, +0.0) returns the SECOND operand because
    // `-0.0 < 0.0` is false.
    let c = unsafe { (a.c.c2Absv)(C2v::new(-0.0, 0.0)) };
    let r = unsafe { (a.rust.c2Absv)(C2v::new(-0.0, 0.0)) };
    assert_eq!(c.bits(), r.bits(), "c2Absv signed zero");
    assert_eq!(
        c.x.to_bits(),
        0x8000_0000,
        "c2Absv(-0.0) must stay -0.0 (the ternary compares `< 0`), got {}",
        f(c.x)
    );
    for (an, bn) in [(-0.0f32, 0.0f32), (0.0, -0.0), (0.0, 0.0), (-0.0, -0.0)] {
        let p = C2v::new(an, an);
        let q = C2v::new(bn, bn);
        let c = unsafe { (a.c.c2Minv)(p, q) };
        let r = unsafe { (a.rust.c2Minv)(p, q) };
        assert_eq!(c.bits(), r.bits(), "c2Minv({}, {})", f(an), f(bn));
        assert_eq!(
            c.x.to_bits(),
            bn.to_bits(),
            "c2Minv({}, {}) must yield the second operand", f(an), f(bn)
        );
        let c = unsafe { (a.c.c2Maxv)(p, q) };
        let r = unsafe { (a.rust.c2Maxv)(p, q) };
        assert_eq!(c.bits(), r.bits(), "c2Maxv({}, {})", f(an), f(bn));
        assert_eq!(
            c.x.to_bits(),
            bn.to_bits(),
            "c2Maxv({}, {}) must also yield the second operand", f(an), f(bn)
        );
        // c2Skew / c2CCW90 negate, which flips the sign of a zero.
        cmp_v1("c2Skew", a.c.c2Skew, a.rust.c2Skew, p, "signed zero");
        cmp_v1("c2CCW90", a.c.c2CCW90, a.rust.c2CCW90, p, "signed zero");
    }
}
