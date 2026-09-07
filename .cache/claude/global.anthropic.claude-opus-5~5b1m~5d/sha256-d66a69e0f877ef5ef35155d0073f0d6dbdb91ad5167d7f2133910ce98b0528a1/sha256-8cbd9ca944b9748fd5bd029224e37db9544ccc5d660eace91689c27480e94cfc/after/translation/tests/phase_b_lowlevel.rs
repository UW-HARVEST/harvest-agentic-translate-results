//! Phase B — valid-path differential tests for the LOW-LEVEL entry points.
//!
//! Covers `CONFIGS.md` rows 1–22 (`c2V` … `f2`) and 23–41 (`f3`, `f4`, `f5`,
//! `f7`). Every call goes through the dynamic symbol table of both `.so`s.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::os::raw::c_void;

// ===========================================================================
// Rows 1–9: c2V / c2Maxv / c2Minv / c2Clampv / c2Sub / c2Dot
// ===========================================================================

/// Row 1 — `c2V` with random and special values.
#[test]
fn cfg01_c2V() {
    let a = apis();
    let mut rng = Rng::new();
    let zoo = zoo_f32();
    for &x in &zoo {
        for &y in &zoo {
            unsafe {
                eq_v2("c2V", &(Bits(x), Bits(y)), (a.c.c2V)(x, y), (a.r.c2V)(x, y));
            }
        }
    }
    for _ in 0..20_000 {
        let (x, y) = (rng.any_f32(), rng.any_f32());
        unsafe {
            eq_v2("c2V", &(Bits(x), Bits(y)), (a.c.c2V)(x, y), (a.r.c2V)(x, y));
        }
    }
}

fn rand_v2(rng: &mut Rng, mode: u32) -> c2v {
    match mode {
        0 => c2v {
            x: rng.finite_f32(100.0),
            y: rng.finite_f32(100.0),
        },
        1 => c2v {
            x: rng.small_f32(),
            y: rng.small_f32(),
        },
        2 => c2v {
            x: rng.any_f32(),
            y: rng.any_f32(),
        },
        _ => {
            let zoo = zoo_f32();
            c2v {
                x: rng.pick(&zoo),
                y: rng.pick(&zoo),
            }
        }
    }
}

/// Rows 2–5 — `c2Maxv` / `c2Minv`, finite + NaN/inf/±0.
#[test]
fn cfg02_05_c2Maxv_c2Minv() {
    let a = apis();
    let zoo = zoo_f32();
    // Exhaustive over the interesting-value cross product (componentwise).
    for &ax in &zoo {
        for &bx in &zoo {
            let u = c2v { x: ax, y: bx };
            let v = c2v { x: bx, y: ax };
            let ctx = (Bits(ax), Bits(bx));
            unsafe {
                eq_v2("c2Maxv", &ctx, (a.c.c2Maxv)(u, v), (a.r.c2Maxv)(u, v));
                eq_v2("c2Minv", &ctx, (a.c.c2Minv)(u, v), (a.r.c2Minv)(u, v));
                // reversed operand order (the ternary is asymmetric on NaN)
                eq_v2("c2Maxv/rev", &ctx, (a.c.c2Maxv)(v, u), (a.r.c2Maxv)(v, u));
                eq_v2("c2Minv/rev", &ctx, (a.c.c2Minv)(v, u), (a.r.c2Minv)(v, u));
            }
        }
    }
    let mut rng = Rng::new();
    for i in 0..40_000u32 {
        let mode = i % 4;
        let (u, v) = (rand_v2(&mut rng, mode), rand_v2(&mut rng, mode));
        let ctx = (Bits(u.x), Bits(u.y), Bits(v.x), Bits(v.y));
        unsafe {
            eq_v2("c2Maxv", &ctx, (a.c.c2Maxv)(u, v), (a.r.c2Maxv)(u, v));
            eq_v2("c2Minv", &ctx, (a.c.c2Minv)(u, v), (a.r.c2Minv)(u, v));
        }
    }
}

/// Rows 6–7 — `c2Clampv`, well-formed and inverted boxes, with NaN.
#[test]
fn cfg06_07_c2Clampv() {
    let a = apis();
    let mut rng = Rng::new();
    for i in 0..60_000u32 {
        let mode = i % 4;
        let mut lo = rand_v2(&mut rng, mode);
        let mut hi = rand_v2(&mut rng, mode);
        // Half the iterations: force a well-formed box (lo <= hi).
        if i % 2 == 0 && lo.x.is_finite() && hi.x.is_finite() {
            if lo.x > hi.x {
                std::mem::swap(&mut lo.x, &mut hi.x);
            }
            if lo.y > hi.y {
                std::mem::swap(&mut lo.y, &mut hi.y);
            }
        }
        let p = rand_v2(&mut rng, mode);
        let ctx = (
            Bits(p.x),
            Bits(p.y),
            Bits(lo.x),
            Bits(lo.y),
            Bits(hi.x),
            Bits(hi.y),
        );
        unsafe {
            eq_v2(
                "c2Clampv",
                &ctx,
                (a.c.c2Clampv)(p, lo, hi),
                (a.r.c2Clampv)(p, lo, hi),
            );
        }
    }
    // On-edge / inside / outside, deterministic.
    let lo = c2v { x: -1.0, y: -2.0 };
    let hi = c2v { x: 3.0, y: 4.0 };
    for &px in &[-5.0f32, -1.0, 0.0, 3.0, 9.0, f32::NAN, f32::INFINITY] {
        for &py in &[-5.0f32, -2.0, 0.0, 4.0, 9.0, f32::NAN, f32::NEG_INFINITY] {
            let p = c2v { x: px, y: py };
            unsafe {
                eq_v2(
                    "c2Clampv/edge",
                    &(Bits(px), Bits(py)),
                    (a.c.c2Clampv)(p, lo, hi),
                    (a.r.c2Clampv)(p, lo, hi),
                );
            }
        }
    }
    // Inverted box.
    let ilo = c2v { x: 5.0, y: 6.0 };
    let ihi = c2v { x: -5.0, y: -6.0 };
    for &px in &[-10.0f32, 0.0, 10.0] {
        for &py in &[-10.0f32, 0.0, 10.0] {
            let p = c2v { x: px, y: py };
            unsafe {
                eq_v2(
                    "c2Clampv/inverted",
                    &(Bits(px), Bits(py)),
                    (a.c.c2Clampv)(p, ilo, ihi),
                    (a.r.c2Clampv)(p, ilo, ihi),
                );
            }
        }
    }
}

/// Rows 8–9 — `c2Sub` and `c2Dot`, including `inf-inf`, `0*inf`, NaN payloads.
#[test]
fn cfg08_09_c2Sub_c2Dot() {
    let a = apis();
    let zoo = zoo_f32();
    for &ax in &zoo {
        for &bx in &zoo {
            let u = c2v { x: ax, y: bx };
            let v = c2v { x: bx, y: ax };
            let ctx = (Bits(ax), Bits(bx));
            unsafe {
                eq_v2("c2Sub", &ctx, (a.c.c2Sub)(u, v), (a.r.c2Sub)(u, v));
                eq_v2("c2Sub/rev", &ctx, (a.c.c2Sub)(v, u), (a.r.c2Sub)(v, u));
                eq_f32("c2Dot", &ctx, (a.c.c2Dot)(u, v), (a.r.c2Dot)(u, v));
                eq_f32("c2Dot/rev", &ctx, (a.c.c2Dot)(v, u), (a.r.c2Dot)(v, u));
            }
        }
    }
    // Cross-product of the whole zoo in all four component positions would be
    // 31^4; sample it randomly instead but keep every value reachable.
    let mut rng = Rng::new();
    for i in 0..80_000u32 {
        let (u, v) = if i % 3 == 0 {
            (
                c2v {
                    x: rng.pick(&zoo),
                    y: rng.pick(&zoo),
                },
                c2v {
                    x: rng.pick(&zoo),
                    y: rng.pick(&zoo),
                },
            )
        } else {
            (rand_v2(&mut rng, i % 4), rand_v2(&mut rng, i % 4))
        };
        let ctx = (Bits(u.x), Bits(u.y), Bits(v.x), Bits(v.y));
        unsafe {
            eq_v2("c2Sub", &ctx, (a.c.c2Sub)(u, v), (a.r.c2Sub)(u, v));
            eq_f32("c2Dot", &ctx, (a.c.c2Dot)(u, v), (a.r.c2Dot)(u, v));
        }
    }
}

// ===========================================================================
// Rows 10–18: the three shape-vs-shape predicates
// ===========================================================================

fn rand_circle(rng: &mut Rng, mode: u32) -> c2Circle {
    let p = rand_v2(rng, mode);
    let r = match mode {
        0 => rng.finite_f32(50.0).abs(),
        1 => rng.small_f32(),
        2 => rng.any_f32(),
        _ => {
            let zoo = zoo_f32();
            rng.pick(&zoo)
        }
    };
    c2Circle { p, r }
}

fn rand_aabb(rng: &mut Rng, mode: u32, wellformed: bool) -> c2AABB {
    let mut min = rand_v2(rng, mode);
    let mut max = rand_v2(rng, mode);
    if wellformed {
        if min.x > max.x {
            std::mem::swap(&mut min.x, &mut max.x);
        }
        if min.y > max.y {
            std::mem::swap(&mut min.y, &mut max.y);
        }
    }
    c2AABB { min, max }
}

/// Rows 10–12 — `c2CircletoCircle`: overlapping, disjoint, touching, degenerate r.
#[test]
fn cfg10_12_c2CircletoCircle() {
    let a = apis();
    let mut rng = Rng::new();
    for i in 0..60_000u32 {
        let mode = i % 4;
        let mut A = rand_circle(&mut rng, mode);
        let mut B = rand_circle(&mut rng, mode);
        // Every third iteration: place B so the circles overlap or exactly touch.
        if i % 3 == 0 && mode <= 1 {
            A.r = A.r.abs() + 1.0;
            B.r = B.r.abs() + 1.0;
            let d = A.r + B.r;
            let f = rng.pick(&[0.0f32, 0.5, 0.999_999, 1.0, 1.000_001, 2.0]);
            B.p = c2v {
                x: A.p.x + d * f,
                y: A.p.y,
            };
        }
        let ctx = (
            Bits(A.p.x),
            Bits(A.p.y),
            Bits(A.r),
            Bits(B.p.x),
            Bits(B.p.y),
            Bits(B.r),
        );
        unsafe {
            eq_i32(
                "c2CircletoCircle",
                &ctx,
                (a.c.c2CircletoCircle)(A, B),
                (a.r.c2CircletoCircle)(A, B),
            );
        }
    }
    // Exact-touch and zero/negative radius, deterministic.
    for &ra in &[0.0f32, -0.0, 1.0, -1.0, -3.0, f32::NAN, f32::INFINITY] {
        for &rb in &[0.0f32, 1.0, -1.0, 2.0, f32::NAN, f32::NEG_INFINITY] {
            for &dx in &[0.0f32, 1.0, 2.0, 3.0, 4.0] {
                let A = c2Circle {
                    p: c2v { x: 0.0, y: 0.0 },
                    r: ra,
                };
                let B = c2Circle {
                    p: c2v { x: dx, y: 0.0 },
                    r: rb,
                };
                unsafe {
                    eq_i32(
                        "c2CircletoCircle/exact",
                        &(Bits(ra), Bits(rb), Bits(dx)),
                        (a.c.c2CircletoCircle)(A, B),
                        (a.r.c2CircletoCircle)(A, B),
                    );
                }
            }
        }
    }
}

/// Rows 13–15 — `c2CircletoAABB`: inside, all 8 outside regions, degenerate boxes.
#[test]
fn cfg13_15_c2CircletoAABB() {
    let a = apis();
    let mut rng = Rng::new();
    for i in 0..60_000u32 {
        let mode = i % 4;
        let A = rand_circle(&mut rng, mode);
        let B = rand_aabb(&mut rng, mode, i % 2 == 0);
        let ctx = (
            Bits(A.p.x),
            Bits(A.p.y),
            Bits(A.r),
            Bits(B.min.x),
            Bits(B.min.y),
            Bits(B.max.x),
            Bits(B.max.y),
        );
        unsafe {
            eq_i32(
                "c2CircletoAABB",
                &ctx,
                (a.c.c2CircletoAABB)(A, B),
                (a.r.c2CircletoAABB)(A, B),
            );
        }
    }
    // Box [0,10]x[0,10]; centre in all 9 regions (inside + 4 edges + 4 corners).
    let B = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 10.0, y: 10.0 },
    };
    for &cx in &[-5.0f32, 0.0, 5.0, 10.0, 15.0, f32::NAN] {
        for &cy in &[-5.0f32, 0.0, 5.0, 10.0, 15.0, f32::NAN] {
            for &r in &[0.0f32, 1.0, 5.0, 7.072, -3.0, f32::NAN, f32::INFINITY] {
                let A = c2Circle {
                    p: c2v { x: cx, y: cy },
                    r,
                };
                unsafe {
                    eq_i32(
                        "c2CircletoAABB/regions",
                        &(Bits(cx), Bits(cy), Bits(r)),
                        (a.c.c2CircletoAABB)(A, B),
                        (a.r.c2CircletoAABB)(A, B),
                    );
                }
            }
        }
    }
    // Degenerate (min == max) and inverted boxes.
    for &(mnx, mny, mxx, mxy) in &[
        (1.0f32, 1.0f32, 1.0f32, 1.0f32),
        (5.0, 6.0, -5.0, -6.0),
        (f32::NAN, 0.0, 1.0, 1.0),
        (0.0, f32::NAN, 1.0, 1.0),
        (0.0, 0.0, f32::NAN, 1.0),
        (0.0, 0.0, 1.0, f32::NAN),
    ] {
        let Bx = c2AABB {
            min: c2v { x: mnx, y: mny },
            max: c2v { x: mxx, y: mxy },
        };
        for &r in &[0.0f32, 1.0, 100.0, f32::NAN] {
            let A = c2Circle {
                p: c2v { x: 0.5, y: 0.5 },
                r,
            };
            unsafe {
                eq_i32(
                    "c2CircletoAABB/degenerate",
                    &(Bits(mnx), Bits(mny), Bits(mxx), Bits(mxy), Bits(r)),
                    (a.c.c2CircletoAABB)(A, Bx),
                    (a.r.c2CircletoAABB)(A, Bx),
                );
            }
        }
    }
}

/// Rows 16–18 — `c2AABBtoAABB`: overlapping, disjoint per axis, touching, NaN.
#[test]
fn cfg16_18_c2AABBtoAABB() {
    let a = apis();
    let mut rng = Rng::new();
    for i in 0..60_000u32 {
        let mode = i % 4;
        let A = rand_aabb(&mut rng, mode, i % 2 == 0);
        let B = rand_aabb(&mut rng, mode, i % 2 == 0);
        let ctx = (
            Bits(A.min.x),
            Bits(A.min.y),
            Bits(A.max.x),
            Bits(A.max.y),
            Bits(B.min.x),
            Bits(B.min.y),
            Bits(B.max.x),
            Bits(B.max.y),
        );
        unsafe {
            eq_i32(
                "c2AABBtoAABB",
                &ctx,
                (a.c.c2AABBtoAABB)(A, B),
                (a.r.c2AABBtoAABB)(A, B),
            );
        }
    }
    // Deterministic: A = [0,10]^2, B slid along x and y including exact touch.
    let A = c2AABB {
        min: c2v { x: 0.0, y: 0.0 },
        max: c2v { x: 10.0, y: 10.0 },
    };
    let offs = [-20.0f32, -10.0, -5.0, 0.0, 5.0, 10.0, 20.0, f32::NAN];
    for &ox in &offs {
        for &oy in &offs {
            let B = c2AABB {
                min: c2v { x: ox, y: oy },
                max: c2v { x: ox + 10.0, y: oy + 10.0 },
            };
            unsafe {
                eq_i32(
                    "c2AABBtoAABB/slide",
                    &(Bits(ox), Bits(oy)),
                    (a.c.c2AABBtoAABB)(A, B),
                    (a.r.c2AABBtoAABB)(A, B),
                );
            }
        }
    }
}

// ===========================================================================
// Rows 19–22: f2 dispatch, all four valid type pairs (incl. the swapped path)
// ===========================================================================

#[test]
fn cfg19_22_f2_all_type_pairs() {
    let a = apis();
    let mut rng = Rng::new();
    for i in 0..60_000u32 {
        let mode = i % 4;
        let ci = rand_circle(&mut rng, mode);
        let cj = rand_circle(&mut rng, mode);
        let bi = rand_aabb(&mut rng, mode, i % 2 == 0);
        let bj = rand_aabb(&mut rng, mode, i % 2 == 0);

        let pc = &ci as *const c2Circle as *const c_void;
        let pc2 = &cj as *const c2Circle as *const c_void;
        let pb = &bi as *const c2AABB as *const c_void;
        let pb2 = &bj as *const c2AABB as *const c_void;

        unsafe {
            // Row 19: CIRCLE, CIRCLE
            eq_i32(
                "f2(C,C)",
                &i,
                (a.c.f2)(pc, C2_TYPE_CIRCLE, pc2, C2_TYPE_CIRCLE),
                (a.r.f2)(pc, C2_TYPE_CIRCLE, pc2, C2_TYPE_CIRCLE),
            );
            // Row 20: CIRCLE, AABB
            eq_i32(
                "f2(C,A)",
                &i,
                (a.c.f2)(pc, C2_TYPE_CIRCLE, pb, C2_TYPE_AABB),
                (a.r.f2)(pc, C2_TYPE_CIRCLE, pb, C2_TYPE_AABB),
            );
            // Row 21: AABB, CIRCLE — the swapped-argument path
            eq_i32(
                "f2(A,C)",
                &i,
                (a.c.f2)(pb, C2_TYPE_AABB, pc, C2_TYPE_CIRCLE),
                (a.r.f2)(pb, C2_TYPE_AABB, pc, C2_TYPE_CIRCLE),
            );
            // Row 22: AABB, AABB
            eq_i32(
                "f2(A,A)",
                &i,
                (a.c.f2)(pb, C2_TYPE_AABB, pb2, C2_TYPE_AABB),
                (a.r.f2)(pb, C2_TYPE_AABB, pb2, C2_TYPE_AABB),
            );
        }
    }
}

/// `f2` must agree with the low-level predicate it dispatches to (including the
/// argument swap in the AABB/CIRCLE case), on both libraries.
#[test]
fn cfg19_22_f2_matches_lowlevel() {
    let a = apis();
    let mut rng = Rng::new();
    for i in 0..20_000u32 {
        let mode = i % 4;
        let ci = rand_circle(&mut rng, mode);
        let bi = rand_aabb(&mut rng, mode, true);
        let pc = &ci as *const c2Circle as *const c_void;
        let pb = &bi as *const c2AABB as *const c_void;
        unsafe {
            let via_f2 = (a.c.f2)(pb, C2_TYPE_AABB, pc, C2_TYPE_CIRCLE);
            let direct = (a.c.c2CircletoAABB)(ci, bi);
            assert_eq!(via_f2, direct, "C: f2(A,C) must swap args");
            let via_f2r = (a.r.f2)(pb, C2_TYPE_AABB, pc, C2_TYPE_CIRCLE);
            let directr = (a.r.c2CircletoAABB)(ci, bi);
            assert_eq!(via_f2r, directr, "Rust: f2(A,C) must swap args");
        }
    }
}

// ===========================================================================
// Rows 23–30: f3, floored division across every sign quadrant
// ===========================================================================

#[test]
fn cfg23_30_f3_exhaustive_specials() {
    let a = apis();
    // Full cross product of the interesting i32 values.
    for &v1 in SPECIAL_I32 {
        for &v2 in SPECIAL_I32 {
            unsafe {
                eq_i32("f3", &(v1, v2), (a.c.f3)(v1, v2), (a.r.f3)(v1, v2));
            }
        }
    }
    // Dense sweep of small magnitudes (hits r==0, r>0, r<0 in every quadrant).
    for v1 in -80i32..=80 {
        for v2 in -80i32..=80 {
            unsafe {
                eq_i32("f3/small", &(v1, v2), (a.c.f3)(v1, v2), (a.r.f3)(v1, v2));
            }
        }
    }
}

#[test]
fn cfg23_30_f3_randomized() {
    let a = apis();
    let mut rng = Rng::new();
    for i in 0..300_000u32 {
        let (v1, v2) = match i % 5 {
            // full-range random
            0 => (rng.next_i32(), rng.next_i32()),
            // random numerator, small divisor
            1 => (rng.next_i32(), rng.next_i32() % 1000),
            // small numerator, full-range divisor
            2 => (rng.next_i32() % 1000, rng.next_i32()),
            // near the INT_MIN edge
            3 => (
                i32::MIN.wrapping_add((rng.next_u32() % 4) as i32),
                rng.next_i32(),
            ),
            // special x random
            _ => (rng.pick(SPECIAL_I32), rng.next_i32()),
        };
        unsafe {
            eq_i32("f3/rand", &(v1, v2), (a.c.f3)(v1, v2), (a.r.f3)(v1, v2));
        }
    }
    // Explicit v1 == INT_MIN sweeps (rows 27, 28, 29).
    for &v1 in &[i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        for v2 in -200i32..=200 {
            unsafe {
                eq_i32("f3/intmin", &(v1, v2), (a.c.f3)(v1, v2), (a.r.f3)(v1, v2));
            }
        }
        for _ in 0..20_000 {
            let v2 = rng.next_i32();
            unsafe {
                eq_i32(
                    "f3/intmin-rand",
                    &(v1, v2),
                    (a.c.f3)(v1, v2),
                    (a.r.f3)(v1, v2),
                );
            }
        }
    }
    // Exact multiples: r == 0 path.
    for _ in 0..40_000 {
        let q = rng.next_i32() % 100_000;
        let d = {
            let d = rng.next_i32() % 10_000;
            if d == 0 {
                1
            } else {
                d
            }
        };
        let v1 = q.wrapping_mul(d);
        unsafe {
            eq_i32("f3/exact", &(v1, d), (a.c.f3)(v1, d), (a.r.f3)(v1, d));
        }
    }
}

// ===========================================================================
// Rows 31–33: f4, xorshift128+ — including the mutated state
// ===========================================================================

#[test]
fn cfg31_33_f4_single_call_and_state() {
    let a = apis();
    let mut rng = Rng::new();

    let check = |s0: u64, s1: u64| {
        let mut sc = cn_rnd_t { state: [s0, s1] };
        let mut sr = cn_rnd_t { state: [s0, s1] };
        let (vc, vr) = unsafe { ((a.c.f4)(&mut sc), (a.r.f4)(&mut sr)) };
        eq_f64("f4", &(s0, s1), vc, vr);
        assert_eq!(
            sc.state, sr.state,
            "f4 must mutate cn_rnd_t identically (seed {s0:#x},{s1:#x})"
        );
        // Row 13/14 sanity: the algorithm can never produce NaN, and the value
        // is always in [0, 1) — assert on the C side, the ground truth.
        assert!(!vc.is_nan(), "C f4 produced NaN for seed {s0:#x},{s1:#x}");
        assert!(
            (0.0..1.0).contains(&vc),
            "C f4 out of [0,1): {vc} for {s0:#x},{s1:#x}"
        );
    };

    // Row 33: fixed special seeds (cross product).
    for &s0 in SPECIAL_U64 {
        for &s1 in SPECIAL_U64 {
            check(s0, s1);
        }
    }
    // Row 31: random seeds.
    for _ in 0..50_000 {
        let (s0, s1) = (rng.next_u64(), rng.next_u64());
        check(s0, s1);
    }
    // One-word-zero shapes.
    for _ in 0..5_000 {
        check(0, rng.next_u64());
        check(rng.next_u64(), 0);
    }
}

/// Row 32 — stateful sequence: 256 successive calls on the same generator.
#[test]
fn cfg32_f4_long_sequence() {
    let a = apis();
    let mut rng = Rng::new();
    let mut seeds: Vec<(u64, u64)> = vec![(0, 0), (1, 0), (0, 1), (u64::MAX, u64::MAX)];
    for _ in 0..200 {
        seeds.push((rng.next_u64(), rng.next_u64()));
    }
    for (s0, s1) in seeds {
        let mut sc = cn_rnd_t { state: [s0, s1] };
        let mut sr = cn_rnd_t { state: [s0, s1] };
        for step in 0..256 {
            let (vc, vr) = unsafe { ((a.c.f4)(&mut sc), (a.r.f4)(&mut sr)) };
            eq_f64("f4/seq", &(s0, s1, step), vc, vr);
            assert_eq!(
                sc.state, sr.state,
                "f4 state diverged at step {step} for seed {s0:#x},{s1:#x}"
            );
        }
    }
}

// ===========================================================================
// Rows 34–35: f5, 16-bit bit reversal on a 32-bit value
// ===========================================================================

#[test]
fn cfg34_35_f5_exhaustive_low16() {
    let a = apis();
    // Row 35: exhaustive over the whole low 16-bit space.
    for v in 0u32..=0xFFFF {
        unsafe {
            eq_u32("f5", &v, (a.c.f5)(v), (a.r.f5)(v));
        }
    }
    // High-bit-only patterns and full-range randoms (Row 34).
    let mut rng = Rng::new();
    for hi in 0u32..=0xFF {
        let v = hi << 24;
        unsafe {
            eq_u32("f5/hi", &v, (a.c.f5)(v), (a.r.f5)(v));
        }
    }
    for &v in SPECIAL_U32 {
        unsafe {
            eq_u32("f5/special", &v, (a.c.f5)(v), (a.r.f5)(v));
        }
    }
    for _ in 0..200_000 {
        let v = rng.next_u32();
        unsafe {
            eq_u32("f5/rand", &v, (a.c.f5)(v), (a.r.f5)(v));
        }
    }
}

// ===========================================================================
// Rows 36–41: f7, all (channels, bitdepth) predicate combinations
// ===========================================================================

#[test]
fn cfg36_41_f7_all_combinations() {
    let a = apis();
    let mut rng = Rng::new();

    let ch_vals: &[u32] = &[0, 1, 2, 3, 4, 8, 16, 255, 0xFFFF, 0x1_0000, u32::MAX];
    let bd_vals: &[u32] = &[0, 1, 4, 8, 12, 16, 20, 24, 31, 32, 33, 64, u32::MAX];
    let bs_vals: &[u32] = &[
        0,
        1,
        2,
        7,
        8,
        16,
        192,
        4096,
        65535,
        65536,
        0x00FF_FFFF,
        0x8000_0000,
        u32::MAX,
        u32::MAX - 1,
    ];

    // Rows 36–39 + 41: full cross product of the branch-relevant values.
    for &bs in bs_vals {
        for &ch in ch_vals {
            for &bd in bd_vals {
                unsafe {
                    eq_u32(
                        "f7",
                        &(bs, ch, bd),
                        (a.c.f7)(bs, ch, bd),
                        (a.r.f7)(bs, ch, bd),
                    );
                }
            }
        }
    }
    // Row 40: fully random, guaranteed to overflow u32 constantly.
    for _ in 0..300_000 {
        let (bs, ch, bd) = (rng.next_u32(), rng.next_u32(), rng.next_u32());
        unsafe {
            eq_u32(
                "f7/rand",
                &(bs, ch, bd),
                (a.c.f7)(bs, ch, bd),
                (a.r.f7)(bs, ch, bd),
            );
        }
    }
    // Realistic FLAC parameters, plus channels pinned to 2 and bitdepth to 32.
    for _ in 0..100_000 {
        let bs = rng.below(65536);
        let ch = rng.below(9);
        let bd = rng.pick(&[8u32, 12, 16, 20, 24, 32]);
        unsafe {
            eq_u32(
                "f7/realistic",
                &(bs, ch, bd),
                (a.c.f7)(bs, ch, bd),
                (a.r.f7)(bs, ch, bd),
            );
        }
        let (bs2, bd2) = (rng.next_u32(), rng.next_u32());
        unsafe {
            eq_u32(
                "f7/ch2",
                &(bs2, 2u32, bd2),
                (a.c.f7)(bs2, 2, bd2),
                (a.r.f7)(bs2, 2, bd2),
            );
            eq_u32(
                "f7/bd32",
                &(bs2, bd2, 32u32),
                (a.c.f7)(bs2, bd2, 32),
                (a.r.f7)(bs2, bd2, 32),
            );
        }
    }
}
