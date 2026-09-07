//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has no error enum, no `errno`,
//! and no `NULL` return: every rejection is the integer `0`. So each test asserts
//! **both** that the C and the Rust return the same `int` **and** that the
//! out-param bits agree — including the case where the C leaves the out-param
//! completely untouched, which is why every call starts from a recognisable
//! `dirty()` pattern rather than zeros.
//!
//! Rows 41-43 of `ERRORS.md` are genuine C undefined behaviour (NULL out-param,
//! `count > 8` overrun, NULL shape pointer). A differential test cannot assert
//! matching *behaviour* for UB, so they are documented there and excluded here;
//! `err_42_poly_count_gt_8_bounded` still checks the in-bounds boundary.

mod common;
use common::*;
use std::ffi::c_void;
use std::os::raw::c_int;

/// Assert C and Rust agree, and that the C really did take the rejection path.
fn expect_reject(
    d: &mut Diff,
    ctx: impl std::fmt::Debug + Copy,
    cret: c_int,
    cout: c2Raycast,
    rret: c_int,
    rout: c2Raycast,
) {
    d.eq(("ret", ctx), cret, rret);
    d.eq(("out", ctx), cb(cout), cb(rout));
    assert_eq!(cret, 0, "expected the C to REJECT for {ctx:?}, but it returned {cret}");
}

fn ray_circle(a: c2Ray, b: c2Circle) -> (c_int, c2Raycast, c_int, c2Raycast) {
    let (cf, rf) = sym!("c2RaytoCircle", FnRayCircle);
    let (mut co, mut ro) = (dirty(), dirty());
    let cret = unsafe { cf(a, b, &mut co) };
    let rret = unsafe { rf(a, b, &mut ro) };
    (cret, co, rret, ro)
}

fn ray_aabb(a: c2Ray, b: c2AABB) -> (c_int, c2Raycast, c_int, c2Raycast) {
    let (cf, rf) = sym!("c2RaytoAABB", FnRayAABB);
    let (mut co, mut ro) = (dirty(), dirty());
    let cret = unsafe { cf(a, b, &mut co) };
    let rret = unsafe { rf(a, b, &mut ro) };
    (cret, co, rret, ro)
}

fn ray_capsule(a: c2Ray, b: c2Capsule) -> (c_int, c2Raycast, c_int, c2Raycast) {
    let (cf, rf) = sym!("c2RaytoCapsule", FnRayCapsule);
    let (mut co, mut ro) = (dirty(), dirty());
    let cret = unsafe { cf(a, b, &mut co) };
    let rret = unsafe { rf(a, b, &mut ro) };
    (cret, co, rret, ro)
}

fn ray_poly(a: c2Ray, p: &c2Poly, bx: *const c2x) -> (c_int, c2Raycast, c_int, c2Raycast) {
    let (cf, rf) = sym!("c2RaytoPoly", FnRayPoly);
    let (mut co, mut ro) = (dirty(), dirty());
    let cret = unsafe { cf(a, p, bx, &mut co) };
    let rret = unsafe { rf(a, p, bx, &mut ro) };
    (cret, co, rret, ro)
}

fn cast_ray(
    a: c2Ray,
    shape: *const c_void,
    bx: *const c2x,
    ty: c_int,
) -> (c_int, c2Raycast, c_int, c2Raycast) {
    let (cf, rf) = sym!("c2CastRay", FnCastRay);
    let (mut co, mut ro) = (dirty(), dirty());
    let cret = unsafe { cf(a, shape, bx, ty, &mut co) };
    let rret = unsafe { rf(a, shape, bx, ty, &mut ro) };
    (cret, co, rret, ro)
}

// ===========================================================================
// Rows 1-5: c2RaytoCircle rejections
// ===========================================================================

/// Row 1 — `disc = b*b - c < 0`: the ray's line misses the circle entirely.
#[test]
fn err_01_circle_disc_negative() {
    let mut d = Diff::new("err_01_circle_disc_negative");
    let mut rng = Rng::new(SEED ^ 101);
    for i in 0..2048 {
        let c = c2Circle {
            p: rng.v(),
            r: 0.25 + rng.unit() * 3.0,
        };
        // Aim perpendicular-offset by MORE than the radius => the line misses.
        let th = rng.unit() * std::f32::consts::TAU;
        let dir = c2v {
            x: th.cos(),
            y: th.sin(),
        };
        let perp = c2v {
            x: -dir.y,
            y: dir.x,
        };
        let off = c.r * (1.05 + rng.unit() * 4.0);
        let dist = 5.0 + rng.unit() * 10.0;
        let p = c2v {
            x: c.p.x + perp.x * off - dir.x * dist,
            y: c.p.y + perp.y * off - dir.y * dist,
        };
        let a = c2Ray {
            p,
            d: dir,
            t: 1e6,
        };
        let (cr, co, rr, ro) = ray_circle(a, c);
        expect_reject(&mut d, ("disc<0", i), cr, co, rr, ro);
    }
    d.finish();
}

/// Row 2 — `t = -b - sqrtf(disc) < 0`: the circle is entirely BEHIND the origin.
#[test]
fn err_02_circle_t_negative() {
    let mut d = Diff::new("err_02_circle_t_negative");
    let mut rng = Rng::new(SEED ^ 102);
    for i in 0..2048 {
        let c = c2Circle {
            p: rng.v(),
            r: 0.25 + rng.unit() * 3.0,
        };
        let th = rng.unit() * std::f32::consts::TAU;
        let dir = c2v {
            x: th.cos(),
            y: th.sin(),
        };
        // Origin PAST the circle, still pointing the same way => t < 0.
        let dist = c.r + 1.0 + rng.unit() * 10.0;
        let p = c2v {
            x: c.p.x + dir.x * dist,
            y: c.p.y + dir.y * dist,
        };
        let a = c2Ray {
            p,
            d: dir,
            t: 1e6,
        };
        let (cr, co, rr, ro) = ray_circle(a, c);
        expect_reject(&mut d, ("t<0", i), cr, co, rr, ro);
    }
    d.finish();
}

/// Row 3 — `t > A.t`: a real hit, but beyond the ray's length (incl. `A.t == 0`).
#[test]
fn err_03_circle_t_beyond_len() {
    let mut d = Diff::new("err_03_circle_t_beyond_len");
    let (f_c, _) = sym!("c2RaytoCircle", FnRayCircle);
    let mut rng = Rng::new(SEED ^ 103);
    for i in 0..1024 {
        let c = c2Circle {
            p: rng.v(),
            r: 0.25 + rng.unit() * 3.0,
        };
        let th = rng.unit() * std::f32::consts::TAU;
        let dir = c2v {
            x: th.cos(),
            y: th.sin(),
        };
        let dist = c.r + 2.0 + rng.unit() * 10.0;
        let p = c2v {
            x: c.p.x - dir.x * dist,
            y: c.p.y - dir.y * dist,
        };
        // First confirm it DOES hit with an unlimited ray, then shorten it.
        let mut probe = dirty();
        let long = c2Ray {
            p,
            d: dir,
            t: 1e9,
        };
        if unsafe { f_c(long, c, &mut probe) } == 0 {
            continue;
        }
        let t_hit = probe.t;
        for short in [0.0f32, -0.0, t_hit * 0.5, t_hit * 0.999, t_hit - 1.0] {
            if short >= t_hit {
                continue;
            }
            let a = c2Ray {
                p,
                d: dir,
                t: short,
            };
            let (cr, co, rr, ro) = ray_circle(a, c);
            expect_reject(&mut d, ("t>A.t", i, fb(short)), cr, co, rr, ro);
        }
    }
    d.finish();
}

/// Row 4 — `A.t` is NaN, so `t <= A.t` is false and the hit is rejected.
#[test]
fn err_04_circle_t_nan() {
    let mut d = Diff::new("err_04_circle_t_nan");
    let mut rng = Rng::new(SEED ^ 104);
    for i in 0..1024 {
        let c = c2Circle {
            p: rng.v(),
            r: 0.25 + rng.unit() * 3.0,
        };
        let th = rng.unit() * std::f32::consts::TAU;
        let dir = c2v {
            x: th.cos(),
            y: th.sin(),
        };
        let dist = c.r + 2.0 + rng.unit() * 8.0;
        let p = c2v {
            x: c.p.x - dir.x * dist,
            y: c.p.y - dir.y * dist,
        };
        for nan in [f32::NAN, -f32::NAN, f32::from_bits(0x7F80_0001)] {
            let a = c2Ray { p, d: dir, t: nan };
            let (cr, co, rr, ro) = ray_circle(a, c);
            expect_reject(&mut d, ("A.t=NaN", i, fb(nan)), cr, co, rr, ro);
        }
    }
    d.finish();
}

/// Row 5 — NaN in the circle / ray makes `disc` NaN: `disc < 0` is false, then
/// `t >= 0` is false, so the C still returns 0.
#[test]
fn err_05_circle_r_nan() {
    let mut d = Diff::new("err_05_circle_r_nan");
    let mut rng = Rng::new(SEED ^ 105);
    let nans = [f32::NAN, -f32::NAN, f32::from_bits(0x7FC0_1234)];
    for i in 0..512 {
        let base = c2Circle {
            p: rng.v(),
            r: 1.0 + rng.unit() * 3.0,
        };
        let a = rng.ray();
        for &n in &nans {
            // NaN radius
            let (cr, co, rr, ro) = ray_circle(a, c2Circle { p: base.p, r: n });
            expect_reject(&mut d, ("r=NaN", i, fb(n)), cr, co, rr, ro);
            // NaN centre component
            for c2 in [
                c2Circle {
                    p: c2v { x: n, y: base.p.y },
                    r: base.r,
                },
                c2Circle {
                    p: c2v { x: base.p.x, y: n },
                    r: base.r,
                },
            ] {
                let (cr, co, rr, ro) = ray_circle(a, c2);
                expect_reject(&mut d, ("p=NaN", i, fb(n)), cr, co, rr, ro);
            }
            // NaN ray origin / direction
            for a2 in [
                c2Ray {
                    p: c2v { x: n, y: a.p.y },
                    d: a.d,
                    t: a.t,
                },
                c2Ray {
                    p: a.p,
                    d: c2v { x: n, y: a.d.y },
                    t: a.t,
                },
            ] {
                let (cr, co, rr, ro) = ray_circle(a2, base);
                expect_reject(&mut d, ("ray=NaN", i, fb(n)), cr, co, rr, ro);
            }
        }
    }
    d.finish();
}

// ===========================================================================
// Rows 6-10: c2RaytoAABB rejections
// ===========================================================================

/// Row 6 — the ray's own bounding box is disjoint from `B`
/// (`!c2AABBtoAABB(a_box, B)`), rejected before any plane maths.
#[test]
fn err_06_aabb_bb_reject() {
    let mut d = Diff::new("err_06_aabb_bb_reject");
    let mut rng = Rng::new(SEED ^ 106);
    for i in 0..4096 {
        let b = c2AABB {
            min: c2v { x: -1.0, y: -1.0 },
            max: c2v { x: 1.0, y: 1.0 },
        };
        // Ray entirely inside a quadrant far away from B, in each of 4 directions.
        for &(sx, sy) in &[(1.0f32, 1.0f32), (-1.0, 1.0), (1.0, -1.0), (-1.0, -1.0)] {
            let far = 5.0 + rng.unit() * 20.0;
            let len = rng.unit() * 2.0;
            let a = c2Ray {
                p: c2v {
                    x: sx * far,
                    y: sy * far,
                },
                d: c2v { x: sx, y: sy },
                t: len,
            };
            let (cr, co, rr, ro) = ray_aabb(a, b);
            expect_reject(&mut d, ("bb", i, fb(sx), fb(sy)), cr, co, rr, ro);
        }
    }
    d.finish();
}

/// Row 7 — the SAT test on the ray's own skew normal separates (`d > 0`): the
/// ray's box overlaps `B` but the ray's *line* misses it.
#[test]
fn err_07_aabb_sat_reject() {
    let mut d = Diff::new("err_07_aabb_sat_reject");
    let (bb_c, _) = sym!("c2AABBtoAABB", FnIbb);
    let (min_c, _) = sym!("c2Minv", FnVvv);
    let (max_c, _) = sym!("c2Maxv", FnVvv);
    let (add_c, _) = sym!("c2Add", FnVvv);
    let (mul_c, _) = sym!("c2Mulvs", FnVvf);
    let mut rng = Rng::new(SEED ^ 107);
    let mut hit_row = 0u32;
    for i in 0..N_C {
        let b = rng.aabb();
        let a = rng.ray();
        // Keep only inputs that PASS the bb pre-check but still return 0, i.e.
        // the rejection came from `d > 0` (or from the `hit` mask, row 8).
        let p1 = add_c(a.p, mul_c(a.d, a.t));
        let abox = c2AABB {
            min: min_c(a.p, p1),
            max: max_c(a.p, p1),
        };
        if bb_c(abox, b) == 0 {
            continue;
        }
        let (cr, co, rr, ro) = ray_aabb(a, b);
        d.eq(("sat", i), (cr, cb(co)), (rr, cb(ro)));
        if cr == 0 {
            hit_row += 1;
        }
    }
    // Plus a constructed case: a long thin diagonal ray whose AABB covers B but
    // whose line passes outside a corner.
    for i in 0..2048 {
        let s = 1.0 + rng.unit() * 5.0;
        let b = c2AABB {
            min: c2v { x: -s, y: -s },
            max: c2v { x: s, y: s },
        };
        let off = s * (2.0 + rng.unit() * 3.0);
        let a = c2Ray {
            p: c2v { x: -off, y: off * 0.999 },
            d: c2v {
                x: std::f32::consts::FRAC_1_SQRT_2,
                y: -std::f32::consts::FRAC_1_SQRT_2,
            },
            t: off * 4.0,
        };
        let (cr, co, rr, ro) = ray_aabb(a, b);
        d.eq(("diag", i), (cr, cb(co)), (rr, cb(ro)));
        if cr == 0 {
            hit_row += 1;
        }
    }
    assert!(hit_row > 0, "no `d > 0` / no-hit rejection was produced");
    d.finish();
}

const N_C: usize = 8192;

/// Row 8 — all four `t0..t3 > 1.0f`, so `hit == 0` and the function returns 0
/// from its final `else`.
///
/// Note on reachability, derived from `c2RayToPlane_OneDimensional`: for finite
/// inputs its result is ALWAYS `<= 1`. (`da < 0` gives 0; `da*db > 0` gives
/// exactly `1`; otherwise `db <= 0` so `d = da - db >= da >= 0` and hence
/// `da/d <= 1`.) The only way to get `t > 1.0f` — and therefore `hit == 0` — is
/// for `t` to be **NaN**, which makes `t <= 1.0f` false. So this row is driven
/// with NaN-injecting generators; a NaN-free search would never reach it, which
/// is exactly the blind spot this phase exists to close.
#[test]
fn err_08_aabb_no_plane_hit() {
    let (bb_c, _) = sym!("c2AABBtoAABB", FnIbb);
    let (min_c, _) = sym!("c2Minv", FnVvv);
    let (max_c, _) = sym!("c2Maxv", FnVvv);
    let (add_c, _) = sym!("c2Add", FnVvv);
    let (mul_c, _) = sym!("c2Mulvs", FnVvf);
    let (dot_c, _) = sym!("c2Dot", FnFvv);
    let (sub_c, _) = sym!("c2Sub", FnVvv);
    let (skew_c, _) = sym!("c2Skew", FnVv);
    let (absv_c, _) = sym!("c2Absv", FnVv);
    let mut d = Diff::new("err_08_aabb_no_plane_hit");
    let mut rng = Rng::new(SEED ^ 108);
    let mut reached = 0u32;
    for i in 0..N_C * 4 {
        // NaN-injecting generators: see the reachability note above.
        let mut b = rng.aabb();
        let mut a = rng.ray();
        let nan = [f32::NAN, -f32::NAN, f32::from_bits(0x7F80_0001)][rng.below(3) as usize];
        match rng.below(9) {
            0 => a.p.x = nan,
            1 => a.p.y = nan,
            2 => a.d.x = nan,
            3 => a.d.y = nan,
            4 => a.t = nan,
            5 => b.min.x = nan,
            6 => b.min.y = nan,
            7 => b.max.x = nan,
            _ => b.max.y = nan,
        }
        // Reproduce the two earlier gates with the C's own helpers so that only
        // inputs which reach the `hit` test are counted for this row.
        let p1 = add_c(a.p, mul_c(a.d, a.t));
        let abox = c2AABB {
            min: min_c(a.p, p1),
            max: max_c(a.p, p1),
        };
        if bb_c(abox, b) == 0 {
            continue;
        }
        let n = skew_c(sub_c(p1, a.p));
        let half = mul_c(sub_c(b.max, b.min), 0.5);
        let ctr = mul_c(add_c(b.min, b.max), 0.5);
        let dv = dot_c(n, sub_c(a.p, ctr));
        let dd = (if dv < 0.0 { -dv } else { dv }) - dot_c(absv_c(n), half);
        if dd > 0.0 {
            continue;
        }
        let (cr, co, rr, ro) = ray_aabb(a, b);
        d.eq(("hitmask", i), (cr, cb(co)), (rr, cb(ro)));
        if cr == 0 {
            reached += 1;
        }
    }
    assert!(reached > 0, "never reached the `hit == 0` rejection");
    eprintln!("[err_08] `hit == 0` rejection reached {reached} times");
    d.finish();
}

/// Row 9 — inverted `B` (`B.min > B.max`), giving negative half-extents.
#[test]
fn err_09_aabb_inverted() {
    let mut d = Diff::new("err_09_aabb_inverted");
    let mut rng = Rng::new(SEED ^ 109);
    for i in 0..N_C {
        let lo = rng.v();
        let hi = rng.v();
        // Deliberately swapped
        let b = c2AABB {
            min: c2v {
                x: lo.x.max(hi.x),
                y: lo.y.max(hi.y),
            },
            max: c2v {
                x: lo.x.min(hi.x),
                y: lo.y.min(hi.y),
            },
        };
        let a = rng.ray();
        let (cr, co, rr, ro) = ray_aabb(a, b);
        d.eq(("inv", i), (cr, cb(co)), (rr, cb(ro)));
        // and a fully-swapped-axes variant
        let b2 = c2AABB {
            min: c2v { x: b.max.x, y: b.min.y },
            max: c2v { x: b.min.x, y: b.max.y },
        };
        let (cr2, co2, rr2, ro2) = ray_aabb(a, b2);
        d.eq(("inv2", i), (cr2, cb(co2)), (rr2, cb(ro2)));
    }
    d.finish();
}

/// Row 10 — NaN anywhere in `A.p` / `A.d` / `A.t` / `B`.
#[test]
fn err_10_aabb_nan_inputs() {
    let mut d = Diff::new("err_10_aabb_nan_inputs");
    let mut rng = Rng::new(SEED ^ 110);
    let nans = [f32::NAN, -f32::NAN, f32::from_bits(0x7F80_0001)];
    for i in 0..1024 {
        let b0 = rng.aabb();
        let a0 = rng.ray();
        for &n in &nans {
            // one NaN in each of the 9 float slots, one at a time
            let variants: [(c2Ray, c2AABB); 9] = [
                (c2Ray { p: c2v { x: n, y: a0.p.y }, d: a0.d, t: a0.t }, b0),
                (c2Ray { p: c2v { x: a0.p.x, y: n }, d: a0.d, t: a0.t }, b0),
                (c2Ray { p: a0.p, d: c2v { x: n, y: a0.d.y }, t: a0.t }, b0),
                (c2Ray { p: a0.p, d: c2v { x: a0.d.x, y: n }, t: a0.t }, b0),
                (c2Ray { p: a0.p, d: a0.d, t: n }, b0),
                (a0, c2AABB { min: c2v { x: n, y: b0.min.y }, max: b0.max }),
                (a0, c2AABB { min: c2v { x: b0.min.x, y: n }, max: b0.max }),
                (a0, c2AABB { min: b0.min, max: c2v { x: n, y: b0.max.y } }),
                (a0, c2AABB { min: b0.min, max: c2v { x: b0.max.x, y: n } }),
            ];
            for (k, (a, b)) in variants.into_iter().enumerate() {
                let (cr, co, rr, ro) = ray_aabb(a, b);
                d.eq(("nan", i, k, fb(n)), (cr, cb(co)), (rr, cb(ro)));
            }
            // all-NaN
            let (cr, co, rr, ro) = ray_aabb(
                c2Ray {
                    p: c2v { x: n, y: n },
                    d: c2v { x: n, y: n },
                    t: n,
                },
                c2AABB {
                    min: c2v { x: n, y: n },
                    max: c2v { x: n, y: n },
                },
            );
            d.eq(("all-nan", i, fb(n)), (cr, cb(co)), (rr, cb(ro)));
        }
    }
    d.finish();
}

// ===========================================================================
// Rows 11-15: c2RaytoCapsule rejections
// ===========================================================================

/// Row 11 — the miss path. NOTE the C has ALREADY written `out->n` and
/// `out->t = 0` (lines 260-261) before it can return 0, so a correct port must
/// leave the same partially-written out-param behind. Zero-filling the out-param
/// beforehand would hide this; `dirty()` does not.
#[test]
fn err_11_capsule_miss_writes_out() {
    let mut d = Diff::new("err_11_capsule_miss_writes_out");
    let mut rng = Rng::new(SEED ^ 111);
    let mut confirmed_partial_write = 0u32;
    for i in 0..N_C {
        // Vertical capsule at the origin; a ray far to the right, moving right,
        // never approaches the axis => the final `return 0`.
        let len = 0.5 + rng.unit() * 6.0;
        let r = 0.25 + rng.unit() * 2.0;
        let b = c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 0.0, y: len },
            r,
        };
        let x0 = r + 1.0 + rng.unit() * 10.0;
        let a = c2Ray {
            p: c2v {
                x: x0,
                y: rng.range(len + 5.0),
            },
            d: c2v { x: 1.0, y: rng.range(1.0) },
            t: rng.unit() * 20.0,
        };
        let (cr, co, rr, ro) = ray_capsule(a, b);
        expect_reject(&mut d, ("miss", i), cr, co, rr, ro);
        // The out-param must NOT still hold the dirty pattern: the C pre-wrote it.
        if cb(co) != cb(dirty()) {
            confirmed_partial_write += 1;
        }
    }
    assert!(
        confirmed_partial_write > 0,
        "expected the C to pre-write out->n/out->t even on the miss path"
    );
    eprintln!("[err_11] partial out-param write confirmed {confirmed_partial_write} times");
    d.finish();
}

/// Row 12 — degenerate capsule `B.a == B.b`: `c2Norm` of the zero vector gives
/// `1/0 = inf` then `0*inf = NaN`, so `M` and `out->n` are NaN-laden.
#[test]
fn err_12_capsule_degenerate() {
    let mut d = Diff::new("err_12_capsule_degenerate");
    let mut rng = Rng::new(SEED ^ 112);
    for i in 0..N_C {
        let p = rng.v();
        for &r in &[0.0f32, -0.0, 1.0, -1.0, 5.0, f32::INFINITY, f32::NAN] {
            let b = c2Capsule { a: p, b: p, r };
            let a = rng.ray();
            let (cr, co, rr, ro) = ray_capsule(a, b);
            d.eq(("degen", i, fb(r)), (cr, cb(co)), (rr, cb(ro)));
        }
        // near-degenerate: a and b separated by one ULP / a denormal
        for &eps in &[f32::from_bits(1), f32::MIN_POSITIVE, 1e-30f32] {
            let b = c2Capsule {
                a: p,
                b: c2v { x: p.x + eps, y: p.y },
                r: 1.0,
            };
            let a = rng.ray();
            let (cr, co, rr, ro) = ray_capsule(a, b);
            d.eq(("near-degen", i, fb(eps)), (cr, cb(co)), (rr, cb(ro)));
        }
    }
    d.finish();
}

/// Row 13 — negative capsule radius: `capsule_bb.min.x = -B.r > B.r =
/// capsule_bb.max.x`, i.e. an inverted bounding box the C never validates.
#[test]
fn err_13_capsule_negative_radius() {
    let mut d = Diff::new("err_13_capsule_negative_radius");
    let mut rng = Rng::new(SEED ^ 113);
    for i in 0..N_C {
        let b = c2Capsule {
            a: rng.v(),
            b: rng.v(),
            r: -(rng.unit() * 8.0) - f32::MIN_POSITIVE,
        };
        let a = rng.ray();
        let (cr, co, rr, ro) = ray_capsule(a, b);
        d.eq(("negr", i), (cr, cb(co)), (rr, cb(ro)));
        // exactly zero and negative zero radius too
        for &r in &[0.0f32, -0.0] {
            let b2 = c2Capsule { a: b.a, b: b.b, r };
            let (cr, co, rr, ro) = ray_capsule(a, b2);
            d.eq(("zeror", i, fb(r)), (cr, cb(co)), (rr, cb(ro)));
        }
    }
    d.finish();
}

/// Row 14 — the capsule delegates to `c2RaytoCircle`, which itself rejects: the
/// `0` propagates out with the out-param still holding the pre-write from
/// lines 260-261 (NOT the dirty pattern, and NOT a circle result).
#[test]
fn err_14_capsule_delegated_reject() {
    let mut d = Diff::new("err_14_capsule_delegated_reject");
    let mut rng = Rng::new(SEED ^ 114);
    let mut delegated_rejects = 0u32;
    for i in 0..N_C {
        let len = 0.5 + rng.unit() * 6.0;
        let r = 0.25 + rng.unit() * 2.0;
        let b = c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 0.0, y: len },
            r,
        };
        // |yAp.x| < r but well below/above the caps, and A.t too short to reach.
        for sign in [-1.0f32, 1.0] {
            let a = c2Ray {
                p: c2v {
                    x: r * 0.5 * sign,
                    y: if sign < 0.0 {
                        -(r + 2.0 + rng.unit() * 10.0)
                    } else {
                        len + r + 2.0 + rng.unit() * 10.0
                    },
                },
                d: c2v { x: 0.0, y: -sign },
                t: 0.0, // zero-length ray => the delegated circle cast rejects
            };
            let (cr, co, rr, ro) = ray_capsule(a, b);
            d.eq(("deleg", i, fb(sign)), (cr, cb(co)), (rr, cb(ro)));
            if cr == 0 {
                delegated_rejects += 1;
                // the pre-write must be visible
                d.eq(("deleg not dirty", i, fb(sign)), cb(co) != cb(dirty()), true);
            }
        }
    }
    assert!(
        delegated_rejects > 0,
        "never produced a delegated c2RaytoCircle rejection"
    );
    d.finish();
}

/// Row 15 — `d = yAe.x - yAp.x == 0` in the side-plane branch, so
/// `t = (c - yAp.x) / 0 = +-inf`.
#[test]
fn err_15_capsule_zero_dx() {
    let mut d = Diff::new("err_15_capsule_zero_dx");
    let mut rng = Rng::new(SEED ^ 115);
    for i in 0..N_C {
        let len = 0.5 + rng.unit() * 6.0;
        let r = 0.25 + rng.unit() * 2.0;
        let b = c2Capsule {
            a: c2v { x: 0.0, y: 0.0 },
            b: c2v { x: 0.0, y: len },
            r,
        };
        // Ray parallel to the capsule axis at |x| >= r => yAe.x == yAp.x exactly.
        for sign in [-1.0f32, 1.0] {
            for xoff in [r, r * 1.0000001, r + 1.0, r + 10.0] {
                for t in [0.0f32, 1.0, 100.0, f32::INFINITY] {
                    let a = c2Ray {
                        p: c2v {
                            x: sign * xoff,
                            y: rng.range(len + 4.0),
                        },
                        d: c2v { x: 0.0, y: 1.0 },
                        t,
                    };
                    let (cr, co, rr, ro) = ray_capsule(a, b);
                    d.eq(("dx=0", i, fb(sign), fb(xoff), fb(t)), (cr, cb(co)), (rr, cb(ro)));
                }
            }
        }
        // and A.t == 0, which also forces yAe == yAp
        let a = c2Ray {
            p: c2v {
                x: r + 1.0,
                y: len * 0.5,
            },
            d: rng.v(),
            t: 0.0,
        };
        let (cr, co, rr, ro) = ray_capsule(a, b);
        d.eq(("t=0", i), (cr, cb(co)), (rr, cb(ro)));
    }
    d.finish();
}

// ===========================================================================
// Rows 16-25: c2RaytoPoly rejections
// ===========================================================================

/// Row 16 — `den == 0 && num < 0`: the ray is exactly parallel to a face plane
/// and on its OUTSIDE, so the C returns 0 immediately from inside the loop.
#[test]
fn err_16_poly_parallel_outside() {
    let mut d = Diff::new("err_16_poly_parallel_outside");
    let mut rng = Rng::new(SEED ^ 116);
    let mut reached = 0u32;
    for &(hw, hh) in &[(1.0f32, 1.0f32), (0.875, 11.5), (4.0, 0.5)] {
        let p = box_poly(hw, hh);
        // norms[0] = (1,0): a ray with d = (0, +-1) gives den == 0 exactly;
        // starting at x > hw gives num = dot((1,0), (hw,-hh) - (x,y)) = hw - x < 0.
        for i in 0..1024 {
            for &dir in &[c2v { x: 0.0, y: 1.0 }, c2v { x: 0.0, y: -1.0 }] {
                let x = hw + 0.001 + rng.unit() * 20.0;
                for &sx in &[1.0f32, -1.0] {
                    let a = c2Ray {
                        p: c2v {
                            x: sx * x,
                            y: rng.range(30.0),
                        },
                        d: dir,
                        t: 1e6,
                    };
                    let (cr, co, rr, ro) = ray_poly(a, &p, std::ptr::null());
                    expect_reject(&mut d, ("par-out", fb(hw), i, fb(sx)), cr, co, rr, ro);
                    reached += 1;
                }
            }
            // horizontal rays outside the +-y planes (norms[1]/[3])
            for &dir in &[c2v { x: 1.0, y: 0.0 }, c2v { x: -1.0, y: 0.0 }] {
                let y = hh + 0.001 + rng.unit() * 20.0;
                for &sy in &[1.0f32, -1.0] {
                    let a = c2Ray {
                        p: c2v {
                            x: rng.range(30.0),
                            y: sy * y,
                        },
                        d: dir,
                        t: 1e6,
                    };
                    let (cr, co, rr, ro) = ray_poly(a, &p, std::ptr::null());
                    expect_reject(&mut d, ("par-out-y", fb(hh), i, fb(sy)), cr, co, rr, ro);
                    reached += 1;
                }
            }
        }
    }
    assert!(reached > 0);
    d.finish();
}

/// Row 17 — `hi < lo` after an iteration: the entry/exit interval collapsed.
#[test]
fn err_17_poly_interval_empty() {
    let mut d = Diff::new("err_17_poly_interval_empty");
    let mut rng = Rng::new(SEED ^ 117);
    let mut zero = 0u32;
    for n in [2usize, 3, 4, 5, 8] {
        let p = ngon(n, 2.0, 0.13, c2v { x: 0.0, y: 0.0 });
        for i in 0..1024 {
            // Rays whose line misses the polygon: lo overtakes hi mid-loop.
            let th = rng.unit() * std::f32::consts::TAU;
            let dir = c2v {
                x: th.cos(),
                y: th.sin(),
            };
            let perp = c2v {
                x: -dir.y,
                y: dir.x,
            };
            let off = 2.5 + rng.unit() * 10.0; // beyond the circumradius
            let dist = 10.0 + rng.unit() * 10.0;
            let a = c2Ray {
                p: c2v {
                    x: perp.x * off - dir.x * dist,
                    y: perp.y * off - dir.y * dist,
                },
                d: dir,
                t: 1e6,
            };
            let (cr, co, rr, ro) = ray_poly(a, &p, std::ptr::null());
            d.eq(("empty", n, i), (cr, cb(co)), (rr, cb(ro)));
            if cr == 0 {
                zero += 1;
            }
        }
    }
    assert!(zero > 100, "expected many interval-collapse rejections, got {zero}");
    d.finish();
}

/// Row 18 — `index == ~0` at the end: no face ever narrowed `lo`, so the
/// function falls through to `return 0` with `*out` untouched.
#[test]
fn err_18_poly_index_unset() {
    let mut d = Diff::new("err_18_poly_index_unset");
    let mut rng = Rng::new(SEED ^ 118);
    let mut untouched = 0u32;
    for n in [1usize, 3, 4, 6, 8] {
        let p = ngon(n, 4.0, 0.0, c2v { x: 0.0, y: 0.0 });
        for i in 0..1024 {
            // Origin strictly inside the inscribed circle => every `den < 0`
            // face already satisfies `num >= lo*den` at `lo == 0`.
            let rho = rng.unit() * 4.0 * (std::f32::consts::PI / n as f32).cos() * 0.85;
            let phi = rng.unit() * std::f32::consts::TAU;
            let th = rng.unit() * std::f32::consts::TAU;
            let a = c2Ray {
                p: c2v {
                    x: rho * phi.cos(),
                    y: rho * phi.sin(),
                },
                d: c2v {
                    x: th.cos(),
                    y: th.sin(),
                },
                t: rng.unit() * 30.0,
            };
            let (cr, co, rr, ro) = ray_poly(a, &p, std::ptr::null());
            d.eq(("inside", n, i), (cr, cb(co)), (rr, cb(ro)));
            if cr == 0 && cb(co) == cb(dirty()) {
                untouched += 1;
            }
        }
    }
    assert!(
        untouched > 100,
        "expected many untouched-out-param rejections, got {untouched}"
    );
    eprintln!("[err_18] out-param left untouched {untouched} times");
    d.finish();
}

/// Row 19 — `count == 0`: the loop body never runs, so row 18's path is taken.
#[test]
fn err_19_poly_count_zero() {
    let mut d = Diff::new("err_19_poly_count_zero");
    let mut rng = Rng::new(SEED ^ 119);
    for i in 0..N_C {
        let mut p = rng.poly();
        p.count = 0;
        let a = rng.ray();
        let bx = rng.x();
        for bxp in [std::ptr::null(), &bx as *const c2x] {
            let (cr, co, rr, ro) = ray_poly(a, &p, bxp);
            expect_reject(&mut d, ("count0", i, bxp.is_null()), cr, co, rr, ro);
            // out-param must be untouched
            d.eq(("count0 untouched", i), cb(co), cb(dirty()));
        }
        // an all-zero polygon too
        let z = zero_poly();
        let (cr, co, rr, ro) = ray_poly(a, &z, std::ptr::null());
        expect_reject(&mut d, ("zeropoly", i), cr, co, rr, ro);
    }
    d.finish();
}

/// Row 20 — NEGATIVE `count` (`i < count` false immediately). `INT_MIN` included.
#[test]
fn err_20_poly_count_negative() {
    let mut d = Diff::new("err_20_poly_count_negative");
    let mut rng = Rng::new(SEED ^ 120);
    let counts: [c_int; 8] = [-1, -2, -8, -9, -1000, i32::MIN, i32::MIN + 1, -0x7FFF_FFFF];
    for i in 0..2048 {
        let base = rng.poly();
        let a = rng.ray();
        let bx = rng.x();
        for &c in &counts {
            let mut p = base;
            p.count = c;
            for bxp in [std::ptr::null(), &bx as *const c2x] {
                let (cr, co, rr, ro) = ray_poly(a, &p, bxp);
                expect_reject(&mut d, ("negcount", i, c), cr, co, rr, ro);
                d.eq(("negcount untouched", i, c), cb(co), cb(dirty()));
            }
        }
    }
    d.finish();
}

/// Row 21 — `A.t == 0` => `hi = 0`, so `num < hi*den` degenerates to `num < 0`.
#[test]
fn err_21_poly_zero_t() {
    let mut d = Diff::new("err_21_poly_zero_t");
    let mut rng = Rng::new(SEED ^ 121);
    for n in [1usize, 3, 4, 8] {
        let p = ngon(n, 2.0, 0.4, c2v { x: 0.0, y: 0.0 });
        for i in 0..1024 {
            for &t in &[0.0f32, -0.0] {
                let a = c2Ray {
                    p: rng.v(),
                    d: {
                        let th = rng.unit() * std::f32::consts::TAU;
                        c2v {
                            x: th.cos(),
                            y: th.sin(),
                        }
                    },
                    t,
                };
                let (cr, co, rr, ro) = ray_poly(a, &p, std::ptr::null());
                d.eq(("t=0", n, i, fb(t)), (cr, cb(co)), (rr, cb(ro)));
            }
        }
    }
    d.finish();
}

/// Row 22 — NEGATIVE `A.t` => `hi < lo == 0` on the first `hi` narrowing.
#[test]
fn err_22_poly_negative_t() {
    let mut d = Diff::new("err_22_poly_negative_t");
    let mut rng = Rng::new(SEED ^ 122);
    for n in [1usize, 3, 4, 8] {
        let p = ngon(n, 2.0, 0.4, c2v { x: 0.0, y: 0.0 });
        for i in 0..1024 {
            for &t in &[
                -f32::MIN_POSITIVE,
                -1.0f32,
                -1e6,
                f32::NEG_INFINITY,
                -f32::MAX,
            ] {
                let a = c2Ray {
                    p: rng.v(),
                    d: {
                        let th = rng.unit() * std::f32::consts::TAU;
                        c2v {
                            x: th.cos(),
                            y: th.sin(),
                        }
                    },
                    t,
                };
                let (cr, co, rr, ro) = ray_poly(a, &p, std::ptr::null());
                d.eq(("t<0", n, i, fb(t)), (cr, cb(co)), (rr, cb(ro)));
            }
        }
    }
    d.finish();
}

/// Row 23 — `bx_ptr == NULL`: the C substitutes `c2xIdentity()`, so the result
/// must be bit-identical to passing an explicit identity `c2x`.
#[test]
fn err_23_poly_null_bx() {
    let mut d = Diff::new("err_23_poly_null_bx");
    let (cf, rf) = sym!("c2RaytoPoly", FnRayPoly);
    let (idc, _) = sym!("c2xIdentity", FnX);
    let mut rng = Rng::new(SEED ^ 123);
    let id_from_c = idc();
    for i in 0..N_C {
        let p = rng.poly();
        let a = rng.ray();
        // C(NULL) == C(&identity)
        let (mut o1, mut o2) = (dirty(), dirty());
        let r1 = unsafe { cf(a, &p, std::ptr::null(), &mut o1) };
        let r2 = unsafe { cf(a, &p, &id_from_c, &mut o2) };
        d.eq(("C null==id", i), (r1, cb(o1)), (r2, cb(o2)));
        // Rust(NULL) == Rust(&identity)
        let (mut o3, mut o4) = (dirty(), dirty());
        let r3 = unsafe { rf(a, &p, std::ptr::null(), &mut o3) };
        let r4 = unsafe { rf(a, &p, &id_from_c, &mut o4) };
        d.eq(("RS null==id", i), (r3, cb(o3)), (r4, cb(o4)));
        // and cross-library
        d.eq(("cross null", i), (r1, cb(o1)), (r3, cb(o3)));
        d.eq(("cross id", i), (r2, cb(o2)), (r4, cb(o4)));
    }
    d.finish();
}

/// Row 24 — `bx.r` is not a unit rotation; the C never validates or normalises.
#[test]
fn err_24_poly_nonunit_rot() {
    let mut d = Diff::new("err_24_poly_nonunit_rot");
    let mut rng = Rng::new(SEED ^ 124);
    let rotors = [
        c2r { c: 0.0, s: 0.0 },
        c2r { c: -0.0, s: -0.0 },
        c2r { c: 1.0, s: 1.0 },
        c2r { c: 100.0, s: 100.0 },
        c2r { c: 1e-30, s: 1e-30 },
        c2r { c: f32::INFINITY, s: f32::INFINITY },
        c2r { c: f32::NEG_INFINITY, s: 1.0 },
        c2r { c: f32::NAN, s: f32::NAN },
        c2r { c: f32::MAX, s: f32::MAX },
        c2r { c: f32::MIN_POSITIVE, s: 0.0 },
    ];
    for i in 0..2048 {
        let p = rng.poly();
        let a = rng.ray();
        for (k, &r) in rotors.iter().enumerate() {
            for tp in [
                c2v { x: 0.0, y: 0.0 },
                rng.v(),
                c2v { x: f32::INFINITY, y: f32::NAN },
            ] {
                let bx = c2x { p: tp, r };
                let (cr, co, rr, ro) = ray_poly(a, &p, &bx);
                d.eq(("nonunit", i, k), (cr, cb(co)), (rr, cb(ro)));
            }
        }
    }
    d.finish();
}

/// Row 25 — all-zero `norms`: `den == 0 && num == 0` for every face, so neither
/// narrowing branch is taken and `index` stays `~0`.
#[test]
fn err_25_poly_zero_norms() {
    let mut d = Diff::new("err_25_poly_zero_norms");
    let mut rng = Rng::new(SEED ^ 125);
    for i in 0..N_C {
        let mut p = rng.poly();
        for j in 0..8 {
            p.norms[j] = c2v { x: 0.0, y: 0.0 };
        }
        let a = rng.ray();
        let (cr, co, rr, ro) = ray_poly(a, &p, std::ptr::null());
        expect_reject(&mut d, ("zeronorms", i), cr, co, rr, ro);
        d.eq(("zeronorms untouched", i), cb(co), cb(dirty()));
        // -0.0 norms, and NaN norms
        let mut q = p;
        for j in 0..8 {
            q.norms[j] = c2v { x: -0.0, y: -0.0 };
        }
        let (cr, co, rr, ro) = ray_poly(a, &q, std::ptr::null());
        d.eq(("negzero norms", i), (cr, cb(co)), (rr, cb(ro)));
        let mut r = p;
        for j in 0..8 {
            r.norms[j] = c2v {
                x: f32::NAN,
                y: f32::NAN,
            };
        }
        let (cr, co, rr, ro) = ray_poly(a, &r, std::ptr::null());
        d.eq(("nan norms", i), (cr, cb(co)), (rr, cb(ro)));
    }
    d.finish();
}

// ===========================================================================
// Rows 26-28: c2CastRay
// ===========================================================================

/// Row 26 — OUT-OF-RANGE ENUM VALUES. A C `enum` accepts any `int`, so a value
/// with no valid variant is a real input crossing the FFI boundary. The `switch`
/// matches nothing, so `B` is never dereferenced and `0` is returned with `*out`
/// untouched — verified by passing a NULL shape pointer, which would crash if
/// either implementation dispatched.
#[test]
fn err_26_castray_bad_enum() {
    let mut d = Diff::new("err_26_castray_bad_enum");
    let mut rng = Rng::new(SEED ^ 126);
    let bad: [c_int; 16] = [
        4, 5, 6, 7, 8, 100, 255, 256, 0x7FFF, i32::MAX, i32::MAX - 1, -1, -2, -100, i32::MIN,
        i32::MIN + 1,
    ];
    for i in 0..512 {
        let a = rng.ray();
        let bx = rng.x();
        let shape = rng.poly();
        for &ty in &bad {
            // with a real shape pointer
            let (cr, co, rr, ro) = cast_ray(
                a,
                &shape as *const c2Poly as *const c_void,
                &bx,
                ty,
            );
            expect_reject(&mut d, ("bad enum", i, ty), cr, co, rr, ro);
            d.eq(("bad enum untouched", i, ty), cb(co), cb(dirty()));
            // with NULL shape AND NULL bx: proves neither side dispatches
            let (cr, co, rr, ro) = cast_ray(a, std::ptr::null(), std::ptr::null(), ty);
            expect_reject(&mut d, ("bad enum null", i, ty), cr, co, rr, ro);
            d.eq(("bad enum null untouched", i, ty), cb(co), cb(dirty()));
        }
    }
    d.finish();
}

/// Row 27 — `typeB == C2_TYPE_POLY` with `bx == NULL`: the only path on which a
/// NULL `bx` is legal; it must equal the explicit-identity result.
#[test]
fn err_27_castray_poly_null_bx() {
    let mut d = Diff::new("err_27_castray_poly_null_bx");
    let (cf, rf) = sym!("c2CastRay", FnCastRay);
    let mut rng = Rng::new(SEED ^ 127);
    let id = identity_x();
    for i in 0..N_C {
        let p = rng.poly();
        let a = rng.ray();
        let sp = &p as *const c2Poly as *const c_void;
        let (mut o1, mut o2, mut o3, mut o4) = (dirty(), dirty(), dirty(), dirty());
        let r1 = unsafe { cf(a, sp, std::ptr::null(), C2_TYPE_POLY, &mut o1) };
        let r2 = unsafe { rf(a, sp, std::ptr::null(), C2_TYPE_POLY, &mut o2) };
        let r3 = unsafe { cf(a, sp, &id, C2_TYPE_POLY, &mut o3) };
        let r4 = unsafe { rf(a, sp, &id, C2_TYPE_POLY, &mut o4) };
        d.eq(("null cross", i), (r1, cb(o1)), (r2, cb(o2)));
        d.eq(("id cross", i), (r3, cb(o3)), (r4, cb(o4)));
        d.eq(("C null==id", i), (r1, cb(o1)), (r3, cb(o3)));
        d.eq(("RS null==id", i), (r2, cb(o2)), (r4, cb(o4)));
    }
    d.finish();
}

/// Row 28 — for CIRCLE / AABB / CAPSULE the C silently IGNORES `bx`, so the
/// result must be independent of it (including a garbage non-NULL `bx`).
#[test]
fn err_28_castray_bx_ignored() {
    let mut d = Diff::new("err_28_castray_bx_ignored");
    let (cf, rf) = sym!("c2CastRay", FnCastRay);
    let mut rng = Rng::new(SEED ^ 128);
    for i in 0..2048 {
        let a = rng.ray();
        let circle = rng.circle();
        let aabb = rng.aabb();
        let capsule = rng.capsule();
        let garbage_bx = c2x {
            p: c2v {
                x: f32::from_bits(rng.next_u32()),
                y: f32::from_bits(rng.next_u32()),
            },
            r: c2r {
                c: f32::from_bits(rng.next_u32()),
                s: f32::from_bits(rng.next_u32()),
            },
        };
        let shapes: [(*const c_void, c_int); 3] = [
            (&circle as *const c2Circle as *const c_void, C2_TYPE_CIRCLE),
            (&aabb as *const c2AABB as *const c_void, C2_TYPE_AABB),
            (&capsule as *const c2Capsule as *const c_void, C2_TYPE_CAPSULE),
        ];
        for (sp, ty) in shapes {
            let (mut o1, mut o2, mut o3, mut o4) = (dirty(), dirty(), dirty(), dirty());
            let r1 = unsafe { cf(a, sp, std::ptr::null(), ty, &mut o1) };
            let r2 = unsafe { cf(a, sp, &garbage_bx, ty, &mut o2) };
            let r3 = unsafe { rf(a, sp, std::ptr::null(), ty, &mut o3) };
            let r4 = unsafe { rf(a, sp, &garbage_bx, ty, &mut o4) };
            d.eq(("C ignores bx", i, ty), (r1, cb(o1)), (r2, cb(o2)));
            d.eq(("RS ignores bx", i, ty), (r3, cb(o3)), (r4, cb(o4)));
            d.eq(("cross null", i, ty), (r1, cb(o1)), (r3, cb(o3)));
            d.eq(("cross bx", i, ty), (r2, cb(o2)), (r4, cb(o4)));
        }
    }
    d.finish();
}

// ===========================================================================
// Rows 29-35: predicate rejections
// ===========================================================================

/// Row 29 — `c2AABBtoAABB` returns 0 via each of the 4 separating axes.
#[test]
fn err_29_aabbaabb_reject() {
    let mut d = Diff::new("err_29_aabbaabb_reject");
    let (f_c, f_r) = sym!("c2AABBtoAABB", FnIbb);
    let mut rng = Rng::new(SEED ^ 129);
    let base = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let mut axes = [0u32; 4];
    for i in 0..4096 {
        let gap = f32::MIN_POSITIVE + rng.unit() * 20.0;
        let cases = [
            // d0: B.max.x < A.min.x
            c2AABB {
                min: c2v { x: -3.0 - gap, y: -1.0 },
                max: c2v { x: -1.0 - gap, y: 1.0 },
            },
            // d1: A.max.x < B.min.x
            c2AABB {
                min: c2v { x: 1.0 + gap, y: -1.0 },
                max: c2v { x: 3.0 + gap, y: 1.0 },
            },
            // d2: B.max.y < A.min.y
            c2AABB {
                min: c2v { x: -1.0, y: -3.0 - gap },
                max: c2v { x: 1.0, y: -1.0 - gap },
            },
            // d3: A.max.y < B.min.y
            c2AABB {
                min: c2v { x: -1.0, y: 1.0 + gap },
                max: c2v { x: 1.0, y: 3.0 + gap },
            },
        ];
        for (k, b) in cases.into_iter().enumerate() {
            let cr = f_c(base, b);
            d.eq(("axis", i, k), cr, f_r(base, b));
            assert_eq!(cr, 0, "axis {k} should separate");
            axes[k] += 1;
        }
        // exact touching => NOT separated (`<` is strict)
        for b in [
            c2AABB { min: c2v { x: -3.0, y: -1.0 }, max: c2v { x: -1.0, y: 1.0 } },
            c2AABB { min: c2v { x: 1.0, y: -1.0 }, max: c2v { x: 3.0, y: 1.0 } },
        ] {
            let cr = f_c(base, b);
            d.eq(("touch", i), cr, f_r(base, b));
            assert_eq!(cr, 1, "exact touching must NOT be a rejection");
        }
    }
    assert!(axes.iter().all(|&c| c > 0));
    d.finish();
}

/// Row 30 — a NaN coordinate makes every `<` false, so `c2AABBtoAABB` reports
/// overlap (returns 1). Counter-intuitive, and exactly what the C does.
#[test]
fn err_30_aabbaabb_nan() {
    let mut d = Diff::new("err_30_aabbaabb_nan");
    let (f_c, f_r) = sym!("c2AABBtoAABB", FnIbb);
    let far = c2AABB {
        min: c2v { x: 1e9, y: 1e9 },
        max: c2v { x: 2e9, y: 2e9 },
    };
    let nan = f32::NAN;
    let all_nan = c2AABB {
        min: c2v { x: nan, y: nan },
        max: c2v { x: nan, y: nan },
    };
    let cases = [
        (all_nan, far),
        (far, all_nan),
        (all_nan, all_nan),
        (
            c2AABB { min: c2v { x: nan, y: 0.0 }, max: c2v { x: 1.0, y: 1.0 } },
            far,
        ),
        (
            far,
            c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: nan, y: 1.0 } },
        ),
    ];
    for (i, (a, b)) in cases.into_iter().enumerate() {
        let cr = f_c(a, b);
        d.eq(("nan", i), cr, f_r(a, b));
    }
    // Only the fully-NaN boxes are guaranteed to report overlap: a box with a
    // single NaN coordinate can still be separated by one of the OTHER three
    // axes, which is what cases 3-4 above exercise.
    for (i, (a, b)) in [(all_nan, far), (far, all_nan), (all_nan, all_nan)]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            f_c(a, b),
            1,
            "an all-NaN box must report overlap (every `<` is false) (case {i})"
        );
    }
    // The C-derived value is asserted rather than assumed for the mixed cases.
    let mut rng = Rng::new(SEED ^ 130);
    for i in 0..4096 {
        let mut a = rng.aabb();
        let mut b = rng.aabb();
        match rng.below(4) {
            0 => a.min.x = nan,
            1 => a.max.y = nan,
            2 => b.min.y = nan,
            _ => b.max.x = nan,
        }
        d.eq(("rand nan", i), f_c(a, b), f_r(a, b));
    }
    d.finish();
}

/// Row 31 — `c2AABBtoPoint` returns 0 on each of the 4 outside directions.
#[test]
fn err_31_aabbpoint_reject() {
    let mut d = Diff::new("err_31_aabbpoint_reject");
    let (f_c, f_r) = sym!("c2AABBtoPoint", FnIbv);
    let mut rng = Rng::new(SEED ^ 131);
    let b = c2AABB {
        min: c2v { x: -1.0, y: -2.0 },
        max: c2v { x: 3.0, y: 4.0 },
    };
    for i in 0..4096 {
        let gap = f32::MIN_POSITIVE + rng.unit() * 20.0;
        let pts = [
            c2v { x: -1.0 - gap, y: 1.0 }, // d0
            c2v { x: 1.0, y: -2.0 - gap }, // d1
            c2v { x: 3.0 + gap, y: 1.0 },  // d2
            c2v { x: 1.0, y: 4.0 + gap },  // d3
        ];
        for (k, p) in pts.into_iter().enumerate() {
            let cr = f_c(b, p);
            d.eq(("out", i, k), cr, f_r(b, p));
            assert_eq!(cr, 0, "side {k} must reject");
        }
        // boundary is INCLUSIVE here (`<` / `>` are strict), so all 4 edges hit
        for p in [
            c2v { x: -1.0, y: 1.0 },
            c2v { x: 3.0, y: 1.0 },
            c2v { x: 1.0, y: -2.0 },
            c2v { x: 1.0, y: 4.0 },
        ] {
            let cr = f_c(b, p);
            d.eq(("edge", i), cr, f_r(b, p));
            assert_eq!(cr, 1, "an exact edge point must be INSIDE");
        }
    }
    d.finish();
}

/// Row 32 — a NaN point makes all 4 comparisons false, so `c2AABBtoPoint`
/// returns 1.
#[test]
fn err_32_aabbpoint_nan() {
    let mut d = Diff::new("err_32_aabbpoint_nan");
    let (f_c, f_r) = sym!("c2AABBtoPoint", FnIbv);
    let b = c2AABB {
        min: c2v { x: -1.0, y: -1.0 },
        max: c2v { x: 1.0, y: 1.0 },
    };
    let nan = f32::NAN;
    for (i, p) in [
        c2v { x: nan, y: nan },
        c2v { x: nan, y: 0.0 },
        c2v { x: 0.0, y: nan },
        c2v { x: nan, y: 1e9 },
        c2v { x: -nan, y: -nan },
    ]
    .into_iter()
    .enumerate()
    {
        let cr = f_c(b, p);
        d.eq(("nan pt", i), cr, f_r(b, p));
    }
    assert_eq!(
        f_c(b, c2v { x: nan, y: nan }),
        1,
        "an all-NaN point must report INSIDE"
    );
    let mut rng = Rng::new(SEED ^ 132);
    for i in 0..4096 {
        let bb = rng.aabb();
        let mut p = rng.v();
        if rng.below(2) == 0 {
            p.x = nan;
        } else {
            p.y = nan;
        }
        d.eq(("rand", i), f_c(bb, p), f_r(bb, p));
    }
    d.finish();
}

/// Row 33 — `c2CircleToPoint` uses a STRICT `<`, so a point exactly on the
/// boundary is a MISS.
#[test]
fn err_33_circlepoint_reject() {
    let mut d = Diff::new("err_33_circlepoint_reject");
    let (f_c, f_r) = sym!("c2CircleToPoint", FnIcv);
    // 3-4-5 and 5-12-13 give exact boundary distances with no rounding slop.
    let exact: [(f32, f32, f32); 4] =
        [(3.0, 4.0, 5.0), (-3.0, -4.0, 5.0), (5.0, 12.0, 13.0), (0.0, 7.0, 7.0)];
    for (i, &(px, py, r)) in exact.iter().enumerate() {
        let a = c2Circle {
            p: c2v { x: 0.0, y: 0.0 },
            r,
        };
        let p = c2v { x: px, y: py };
        let cr = f_c(a, p);
        d.eq(("exact boundary", i), cr, f_r(a, p));
        assert_eq!(cr, 0, "exactly-on-boundary must be a MISS (case {i})");
    }
    let mut rng = Rng::new(SEED ^ 133);
    for i in 0..4096 {
        let a = c2Circle {
            p: rng.v(),
            r: 0.25 + rng.unit() * 6.0,
        };
        let th = rng.unit() * std::f32::consts::TAU;
        // strictly outside
        let k = 1.0 + f32::MIN_POSITIVE + rng.unit() * 5.0;
        let p = c2v {
            x: a.p.x + a.r * k * th.cos(),
            y: a.p.y + a.r * k * th.sin(),
        };
        let cr = f_c(a, p);
        d.eq(("outside", i), cr, f_r(a, p));
        assert_eq!(cr, 0);
    }
    d.finish();
}

/// Row 34 — `A.r == 0` can never contain a point (`d2 < 0` is impossible), while
/// `A.r < 0` still does because `r*r > 0`.
#[test]
fn err_34_circlepoint_zero_neg_r() {
    let mut d = Diff::new("err_34_circlepoint_zero_neg_r");
    let (f_c, f_r) = sym!("c2CircleToPoint", FnIcv);
    let mut rng = Rng::new(SEED ^ 134);
    let center = c2v { x: 1.5, y: -2.5 };
    for i in 0..4096 {
        for &r in &[0.0f32, -0.0] {
            let a = c2Circle { p: center, r };
            for p in [center, rng.v(), c2v { x: 0.0, y: 0.0 }] {
                let cr = f_c(a, p);
                d.eq(("r=0", i, fb(r)), cr, f_r(a, p));
                assert_eq!(cr, 0, "a zero-radius circle can never contain a point");
            }
        }
        // negative radius still "contains" nearby points
        let r = -(1.0 + rng.unit() * 4.0);
        let a = c2Circle { p: center, r };
        let p = c2v {
            x: center.x + r.abs() * 0.5,
            y: center.y,
        };
        let cr = f_c(a, p);
        d.eq(("r<0", i), cr, f_r(a, p));
        assert_eq!(cr, 1, "a negative radius still contains points (r*r > 0)");
    }
    d.finish();
}

/// Row 35 — NaN makes `d2 < r*r` false, so `c2CircleToPoint` returns 0.
#[test]
fn err_35_circlepoint_nan() {
    let mut d = Diff::new("err_35_circlepoint_nan");
    let (f_c, f_r) = sym!("c2CircleToPoint", FnIcv);
    let nan = f32::NAN;
    let mut rng = Rng::new(SEED ^ 135);
    for i in 0..4096 {
        let base = c2Circle {
            p: rng.v(),
            r: 1.0 + rng.unit() * 4.0,
        };
        let variants: [(c2Circle, c2v); 5] = [
            (c2Circle { p: base.p, r: nan }, base.p),
            (c2Circle { p: c2v { x: nan, y: base.p.y }, r: base.r }, base.p),
            (c2Circle { p: c2v { x: base.p.x, y: nan }, r: base.r }, base.p),
            (base, c2v { x: nan, y: base.p.y }),
            (base, c2v { x: nan, y: nan }),
        ];
        for (k, (a, p)) in variants.into_iter().enumerate() {
            let cr = f_c(a, p);
            d.eq(("nan", i, k), cr, f_r(a, p));
            assert_eq!(cr, 0, "NaN must make c2CircleToPoint return 0 (case {k})");
        }
    }
    d.finish();
}

// ===========================================================================
// Rows 36-40: arithmetic edge cases with no explicit check in the C
// ===========================================================================

/// Row 36 — `c2Div`/`c2Norm` by ZERO: `1.0f/0.0f = +inf`, then a `0` component
/// becomes `0 * inf = NaN`. No check exists in the C.
#[test]
fn err_36_div_by_zero() {
    let mut d = Diff::new("err_36_div_by_zero");
    let (div_c, div_r) = sym!("c2Div", FnVvf);
    let (nrm_c, nrm_r) = sym!("c2Norm", FnVv);
    let mut rng = Rng::new(SEED ^ 136);
    for i in 0..4096 {
        let a = rng.any_v();
        let got_c = div_c(a, 0.0);
        d.eq(("div 0", i, vb(a)), vb(got_c), vb(div_r(a, 0.0)));
        d.eq(("norm", i, vb(a)), vb(nrm_c(a)), vb(nrm_r(a)));
    }
    // Every combination of a zero / non-zero component against a zero divisor.
    for &x in &[0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NAN] {
        for &y in &[0.0f32, -0.0, 1.0, -1.0, f32::INFINITY, f32::NAN] {
            let a = c2v { x, y };
            for &b in &[0.0f32, -0.0] {
                d.eq(("exhaustive", vb(a), fb(b)), vb(div_c(a, b)), vb(div_r(a, b)));
            }
        }
    }
    // The C's actual value is the reference, but sanity-check the shape once.
    let one = c2v { x: 1.0, y: 0.0 };
    let r = div_c(one, 0.0);
    assert_eq!(fb(r.x), fb(f32::INFINITY), "1/0 must give +inf in lane x");
    assert!(r.y.is_nan(), "0 * inf must give NaN in lane y");
    d.finish();
}

/// Row 37 — divide by NEGATIVE zero: `1.0f/-0.0f = -inf`, so the signs flip.
#[test]
fn err_37_div_by_negative_zero() {
    let mut d = Diff::new("err_37_div_by_negative_zero");
    let (div_c, div_r) = sym!("c2Div", FnVvf);
    let mut rng = Rng::new(SEED ^ 137);
    for i in 0..4096 {
        let a = rng.any_v();
        d.eq(("div -0", i, vb(a)), vb(div_c(a, -0.0)), vb(div_r(a, -0.0)));
    }
    let one = c2v { x: 1.0, y: -1.0 };
    let r = div_c(one, -0.0);
    assert_eq!(fb(r.x), fb(f32::NEG_INFINITY), "1/-0 must give -inf");
    assert_eq!(fb(r.y), fb(f32::INFINITY), "-1 * -inf must give +inf");
    d.eq("shape", vb(r), vb(div_r(one, -0.0)));
    d.finish();
}

/// Row 38 — `c2Norm` of the ZERO vector: length 0, so both lanes are
/// `0 * inf = NaN`.
#[test]
fn err_38_norm_zero_vector() {
    let mut d = Diff::new("err_38_norm_zero_vector");
    let (nrm_c, nrm_r) = sym!("c2Norm", FnVv);
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
    ] {
        let got = nrm_c(a);
        d.eq(("zero", vb(a)), vb(got), vb(nrm_r(a)));
        assert!(
            got.x.is_nan() && got.y.is_nan(),
            "c2Norm of the zero vector must be NaN in both lanes, got {got:?}"
        );
    }
    // Denormal vectors whose squared length underflows to exactly zero.
    for &m in &[1e-30f32, 1e-25, f32::MIN_POSITIVE, f32::from_bits(1)] {
        for a in [
            c2v { x: m, y: 0.0 },
            c2v { x: m, y: m },
            c2v { x: -m, y: m },
        ] {
            d.eq(("underflow", vb(a)), vb(nrm_c(a)), vb(nrm_r(a)));
        }
    }
    d.finish();
}

/// Row 39 — `c2Len` on NaN / infinite / overflowing inputs.
#[test]
fn err_39_len_special() {
    let mut d = Diff::new("err_39_len_special");
    let (len_c, len_r) = sym!("c2Len", FnFv);
    for &x in SPECIALS.iter() {
        for &y in SPECIALS.iter() {
            let a = c2v { x, y };
            d.eq(("specials", vb(a)), fb(len_c(a)), fb(len_r(a)));
        }
    }
    // A signalling NaN must be quieted identically by both.
    for &bits in &[0x7F80_0001u32, 0xFF80_0001, 0x7FBF_FFFF, 0xFFBF_FFFF] {
        let s = f32::from_bits(bits);
        for a in [
            c2v { x: s, y: 0.0 },
            c2v { x: 0.0, y: s },
            c2v { x: s, y: s },
        ] {
            d.eq(("snan", bits, vb(a)), fb(len_c(a)), fb(len_r(a)));
        }
    }
    // Overflow of the squared length must give +inf, not a finite value.
    let big = c2v { x: 1e30, y: 1e30 };
    assert_eq!(fb(len_c(big)), fb(f32::INFINITY));
    d.eq("overflow", fb(len_c(big)), fb(len_r(big)));
    let mut rng = Rng::new(SEED ^ 139);
    for i in 0..4096 {
        let a = rng.any_v();
        d.eq(("rand", i), fb(len_c(a)), fb(len_r(a)));
    }
    d.finish();
}

/// Row 40 — the C's TERNARY abs/min/max, not `fabsf`/`fminf`/`fmaxf`:
/// `c2Absv(NaN)` keeps the sign bit, `c2Absv(-0.0)` returns `-0.0`, and
/// `c2Minv(NaN, x)` returns `x` while `c2Minv(x, NaN)` returns NaN.
#[test]
fn err_40_ternary_nan_negzero() {
    let mut d = Diff::new("err_40_ternary_nan_negzero");
    let (ab_c, ab_r) = sym!("c2Absv", FnVv);
    let (min_c, min_r) = sym!("c2Minv", FnVvv);
    let (max_c, max_r) = sym!("c2Maxv", FnVvv);

    // -0.0 must NOT be normalised to +0.0 by the ternary abs.
    let nz = c2v { x: -0.0, y: -0.0 };
    let got = ab_c(nz);
    d.eq("abs(-0)", vb(got), vb(ab_r(nz)));
    assert_eq!(
        fb(got.x),
        fb(-0.0f32),
        "the ternary abs must leave -0.0 as -0.0 (f32::abs would not)"
    );

    // A negative NaN must keep its sign bit.
    let nn = c2v {
        x: -f32::NAN,
        y: f32::NAN,
    };
    let got = ab_c(nn);
    d.eq("abs(-NaN)", vb(got), vb(ab_r(nn)));
    assert_eq!(
        got.x.to_bits() >> 31,
        1,
        "the ternary abs must leave a negative NaN negative (f32::abs would clear it)"
    );

    // NaN placement asymmetry in min/max.
    let nan = f32::NAN;
    let a = c2v { x: nan, y: 1.0 };
    let b = c2v { x: 1.0, y: nan };
    d.eq("min(NaN,1)", vb(min_c(a, b)), vb(min_r(a, b)));
    d.eq("min(1,NaN)", vb(min_c(b, a)), vb(min_r(b, a)));
    d.eq("max(NaN,1)", vb(max_c(a, b)), vb(max_r(a, b)));
    d.eq("max(1,NaN)", vb(max_c(b, a)), vb(max_r(b, a)));
    let m = min_c(a, b);
    assert_eq!(fb(m.x), fb(1.0f32), "ternary min(NaN, 1) must be 1");
    assert!(m.y.is_nan(), "ternary min(1, NaN) must be NaN");

    // -0.0 vs +0.0 in min/max (`<` and `>` treat them as equal, so the ternary
    // always returns the SECOND operand).
    for (ax, bx) in [(-0.0f32, 0.0f32), (0.0, -0.0)] {
        let a = c2v { x: ax, y: ax };
        let b = c2v { x: bx, y: bx };
        d.eq(("min +-0", fb(ax), fb(bx)), vb(min_c(a, b)), vb(min_r(a, b)));
        d.eq(("max +-0", fb(ax), fb(bx)), vb(max_c(a, b)), vb(max_r(a, b)));
        assert_eq!(fb(min_c(a, b).x), fb(bx), "ternary min returns b when equal");
    }

    // Exhaustive over every special pair.
    for &ax in SPECIALS.iter() {
        for &ay in SPECIALS.iter() {
            let a = c2v { x: ax, y: ay };
            d.eq(("absv", vb(a)), vb(ab_c(a)), vb(ab_r(a)));
            for &bx in SPECIALS.iter() {
                for &by in SPECIALS.iter() {
                    let b = c2v { x: bx, y: by };
                    d.eq(("minv", vb(a), vb(b)), vb(min_c(a, b)), vb(min_r(a, b)));
                    d.eq(("maxv", vb(a), vb(b)), vb(max_c(a, b)), vb(max_r(a, b)));
                }
            }
        }
    }
    d.finish();
}

// ===========================================================================
// Generic FFI boundary checks (beyond the ERRORS.md rows)
// ===========================================================================

/// Row 42 (bounded part) — `count` at the exact array capacity. `count > 8` is
/// UB in the C (it reads past `verts[8]`), so only `count <= 8` is asserted; the
/// Rust mirrors the C's unchecked raw-pointer indexing either way.
#[test]
fn err_42_poly_count_gt_8_bounded() {
    let mut d = Diff::new("err_42_poly_count_gt_8_bounded");
    let mut rng = Rng::new(SEED ^ 142);
    for i in 0..N_C {
        let mut p = rng.poly();
        let a = rng.ray();
        for c in 0..=8 {
            p.count = c;
            let (cr, co, rr, ro) = ray_poly(a, &p, std::ptr::null());
            d.eq(("count", i, c), (cr, cb(co)), (rr, cb(ro)));
        }
    }
    d.finish();
}

/// Every valid enum value, plus one step past each end of the range, driven
/// through `c2CastRay` for all four shape types.
#[test]
fn err_99_enum_range_boundaries() {
    let mut d = Diff::new("err_99_enum_range_boundaries");
    let mut rng = Rng::new(SEED ^ 199);
    for i in 0..1024 {
        let a = rng.ray();
        let circle = rng.circle();
        let aabb = rng.aabb();
        let capsule = rng.capsule();
        let poly = rng.poly();
        let bx = rng.x();
        // valid: 0..=3 with the MATCHING shape; one-past-the-end: -1 and 4.
        let entries: [(*const c_void, c_int); 4] = [
            (&circle as *const c2Circle as *const c_void, C2_TYPE_CIRCLE),
            (&aabb as *const c2AABB as *const c_void, C2_TYPE_AABB),
            (&capsule as *const c2Capsule as *const c_void, C2_TYPE_CAPSULE),
            (&poly as *const c2Poly as *const c_void, C2_TYPE_POLY),
        ];
        for (sp, ty) in entries {
            let (cr, co, rr, ro) = cast_ray(a, sp, &bx, ty);
            d.eq(("valid", i, ty), (cr, cb(co)), (rr, cb(ro)));
        }
        // one step past each end of the documented range
        for ty in [-1, 4] {
            let (cr, co, rr, ro) = cast_ray(
                a,
                &poly as *const c2Poly as *const c_void,
                &bx,
                ty,
            );
            expect_reject(&mut d, ("past-end", i, ty), cr, co, rr, ro);
        }
    }
    d.finish();
}
