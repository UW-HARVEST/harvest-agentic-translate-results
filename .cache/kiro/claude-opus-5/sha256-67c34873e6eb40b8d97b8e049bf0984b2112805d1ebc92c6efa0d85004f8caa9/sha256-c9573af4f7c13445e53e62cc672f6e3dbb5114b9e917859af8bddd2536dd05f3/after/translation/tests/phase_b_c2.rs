//! Phase B — valid-path differential tests for the lowest-level entry points:
//! the tinyc2 vector helpers and shape tests. CONFIGS.md rows C1-C22.
//!
//! Every call goes through `dlsym` on both `.so`s; results are compared with
//! `to_bits()` so `-0.0` and every NaN payload is significant.

mod common;

use common::*;

macro_rules! bind {
    ($l:expr, $name:expr, $ty:ty) => {{
        let c: libloading::Symbol<$ty> = $l.c.get($name);
        let r: libloading::Symbol<$ty> = $l.r.get($name);
        (c, r)
    }};
}

const N: usize = 4000;

// ---------------------------------------------------------------------------
// C1, C2 — c2V
// ---------------------------------------------------------------------------

#[test]
fn c1_c2v_random_finite() {
    let l = libs();
    let (c, r) = bind!(l, "c2V", FnC2V);
    let mut g = Rng::seeded();
    for i in 0..N {
        let (x, y) = (g.finite_f32(1e6), g.finite_f32(1e6));
        unsafe { eq_vec2(&format!("C1 c2V #{i} ({x},{y})"), c(x, y), r(x, y)) }
    }
}

#[test]
fn c2_c2v_specials() {
    let l = libs();
    let (c, r) = bind!(l, "c2V", FnC2V);
    for &x in &special_f32s() {
        for &y in &special_f32s() {
            unsafe {
                eq_vec2(
                    &format!("C2 c2V(0x{:08x},0x{:08x})", x.to_bits(), y.to_bits()),
                    c(x, y),
                    r(x, y),
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C3, C4, C5 — c2Maxv / c2Minv
// ---------------------------------------------------------------------------

fn minmax_row(name: &'static str, tag: &str, gen: impl Fn(&mut Rng) -> (C2v, C2v)) {
    let l = libs();
    let (c, r) = bind!(l, name, FnC2Bin);
    let mut g = Rng::seeded();
    for i in 0..N {
        let (a, b) = gen(&mut g);
        unsafe {
            eq_vec2(
                &format!("{tag} {name} #{i} a={a:?} b={b:?}"),
                c(a, b),
                r(a, b),
            )
        }
    }
}

#[test]
fn c3_minmax_random_finite() {
    let mk = |g: &mut Rng| {
        (
            C2v { x: g.finite_f32(100.0), y: g.finite_f32(100.0) },
            C2v { x: g.finite_f32(100.0), y: g.finite_f32(100.0) },
        )
    };
    minmax_row("c2Maxv", "C3", mk);
    minmax_row("c2Minv", "C3", mk);
}

#[test]
fn c4_minmax_ties_and_signed_zero() {
    let l = libs();
    let zeros = [0.0f32, -0.0f32, 1.0f32, -1.0f32];
    for name in ["c2Maxv", "c2Minv"] {
        let (c, r) = bind!(l, name, FnC2Bin);
        for &ax in &zeros {
            for &ay in &zeros {
                for &bx in &zeros {
                    for &by in &zeros {
                        let a = C2v { x: ax, y: ay };
                        let b = C2v { x: bx, y: by };
                        unsafe {
                            eq_vec2(
                                &format!("C4 {name} a={a:?} b={b:?}"),
                                c(a, b),
                                r(a, b),
                            )
                        }
                    }
                }
            }
        }
        // exact equality (a == b) for a big random sample
        let mut g = Rng::seeded();
        for i in 0..N {
            let v = C2v { x: g.finite_f32(50.0), y: g.finite_f32(50.0) };
            unsafe { eq_vec2(&format!("C4 {name} tie #{i}"), c(v, v), r(v, v)) }
        }
    }
}

#[test]
fn c5_minmax_nan() {
    let l = libs();
    let sp = special_f32s();
    for name in ["c2Maxv", "c2Minv"] {
        let (c, r) = bind!(l, name, FnC2Bin);
        for &ax in &sp {
            for &bx in &sp {
                let a = C2v { x: ax, y: bx };
                let b = C2v { x: bx, y: ax };
                unsafe {
                    eq_vec2(
                        &format!(
                            "C5 {name} a=(0x{:08x},0x{:08x}) b=(0x{:08x},0x{:08x})",
                            ax.to_bits(),
                            bx.to_bits(),
                            bx.to_bits(),
                            ax.to_bits()
                        ),
                        c(a, b),
                        r(a, b),
                    )
                }
            }
        }
        // fully random bit patterns, so NaN meets NaN with distinct payloads
        let mut g = Rng::seeded();
        for i in 0..N {
            let a = C2v { x: g.mixed_f32(), y: g.mixed_f32() };
            let b = C2v { x: g.mixed_f32(), y: g.mixed_f32() };
            unsafe { eq_vec2(&format!("C5 {name} rnd #{i}"), c(a, b), r(a, b)) }
        }
    }
}

// ---------------------------------------------------------------------------
// C6, C7 — c2Clampv
// ---------------------------------------------------------------------------

#[test]
fn c6_clamp_inside_below_above_inverted() {
    let l = libs();
    let (c, r) = bind!(l, "c2Clampv", FnC2Clamp);
    let mut g = Rng::seeded();
    for i in 0..N {
        let lo = C2v { x: g.finite_f32(10.0), y: g.finite_f32(10.0) };
        let span = (g.range_f32(0.0, 20.0), g.range_f32(0.0, 20.0));
        let hi = C2v { x: lo.x + span.0, y: lo.y + span.1 };
        // four shapes: inside, below, above, inverted range
        let cases = [
            C2v { x: g.range_f32(lo.x, hi.x), y: g.range_f32(lo.y, hi.y) }, // inside
            C2v { x: lo.x - g.range_f32(0.1, 50.0), y: lo.y - g.range_f32(0.1, 50.0) },
            C2v { x: hi.x + g.range_f32(0.1, 50.0), y: hi.y + g.range_f32(0.1, 50.0) },
            C2v { x: g.finite_f32(50.0), y: g.finite_f32(50.0) },
        ];
        for (k, &a) in cases.iter().enumerate() {
            unsafe {
                eq_vec2(
                    &format!("C6 c2Clampv #{i}/{k} a={a:?} lo={lo:?} hi={hi:?}"),
                    c(a, lo, hi),
                    r(a, lo, hi),
                )
            }
            // inverted range (lo > hi) — no validation in C
            unsafe {
                eq_vec2(
                    &format!("C6 c2Clampv inverted #{i}/{k}"),
                    c(a, hi, lo),
                    r(a, hi, lo),
                )
            }
        }
    }
}

#[test]
fn c7_clamp_nan_and_inf() {
    let l = libs();
    let (c, r) = bind!(l, "c2Clampv", FnC2Clamp);
    let sp = special_f32s();
    for &v in &sp {
        for &w in &sp {
            let a = C2v { x: v, y: w };
            let lo = C2v { x: w, y: v };
            let hi = C2v { x: v, y: w };
            unsafe {
                eq_vec2(
                    &format!("C7 c2Clampv 0x{:08x}/0x{:08x}", v.to_bits(), w.to_bits()),
                    c(a, lo, hi),
                    r(a, lo, hi),
                )
            }
        }
    }
    let mut g = Rng::seeded();
    for i in 0..N {
        let a = C2v { x: g.mixed_f32(), y: g.mixed_f32() };
        let lo = C2v { x: g.mixed_f32(), y: g.mixed_f32() };
        let hi = C2v { x: g.mixed_f32(), y: g.mixed_f32() };
        unsafe { eq_vec2(&format!("C7 c2Clampv rnd #{i}"), c(a, lo, hi), r(a, lo, hi)) }
    }
}

// ---------------------------------------------------------------------------
// C8 — c2Sub
// ---------------------------------------------------------------------------

#[test]
fn c8_sub() {
    let l = libs();
    let (c, r) = bind!(l, "c2Sub", FnC2Bin);
    let sp = special_f32s();
    for &v in &sp {
        for &w in &sp {
            let a = C2v { x: v, y: w };
            let b = C2v { x: w, y: v };
            unsafe {
                eq_vec2(
                    &format!("C8 c2Sub 0x{:08x}-0x{:08x}", v.to_bits(), w.to_bits()),
                    c(a, b),
                    r(a, b),
                )
            }
        }
    }
    let mut g = Rng::seeded();
    for i in 0..N {
        let a = C2v { x: g.mixed_f32(), y: g.mixed_f32() };
        let b = C2v { x: g.mixed_f32(), y: g.mixed_f32() };
        unsafe { eq_vec2(&format!("C8 c2Sub rnd #{i}"), c(a, b), r(a, b)) }
    }
    for i in 0..N {
        let a = C2v { x: g.finite_f32(1e20), y: g.finite_f32(1e20) };
        let b = C2v { x: g.finite_f32(1e20), y: g.finite_f32(1e20) };
        unsafe { eq_vec2(&format!("C8 c2Sub finite #{i}"), c(a, b), r(a, b)) }
    }
}

// ---------------------------------------------------------------------------
// C9, C10, C11 — c2Dot
// ---------------------------------------------------------------------------

#[test]
fn c9_dot_random_finite() {
    let l = libs();
    let (c, r) = bind!(l, "c2Dot", FnC2Dot);
    let mut g = Rng::seeded();
    for i in 0..N {
        let a = C2v { x: g.finite_f32(1e3), y: g.finite_f32(1e3) };
        let b = C2v { x: g.finite_f32(1e3), y: g.finite_f32(1e3) };
        unsafe { eq_f32(&format!("C9 c2Dot #{i} {a:?}.{b:?}"), c(a, b), r(a, b)) }
    }
    // magnitudes that make the two products differ wildly, so the add order
    // is observable in the rounding
    for i in 0..N {
        let a = C2v { x: g.finite_f32(1e18), y: g.finite_f32(1e-18) };
        let b = C2v { x: g.finite_f32(1e18), y: g.finite_f32(1e-18) };
        unsafe { eq_f32(&format!("C9 c2Dot spread #{i}"), c(a, b), r(a, b)) }
    }
}

#[test]
fn c10_dot_dual_nan_payloads() {
    let l = libs();
    let (c, r) = bind!(l, "c2Dot", FnC2Dot);
    // Both products are NaN, with *different* payloads: only the correct
    // ADDSS src1/src2 selection reproduces the C result.
    let nans = [
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0x7FC0_0000),
        f32::from_bits(0x7FAB_CDEF),
        f32::from_bits(0xFF80_0002),
        f32::from_bits(0xFFD5_5555),
        f32::from_bits(0x7F81_2345),
    ];
    for &n1 in &nans {
        for &n2 in &nans {
            let a = C2v { x: n1, y: n2 };
            let b = C2v { x: 1.0, y: 1.0 };
            unsafe {
                eq_f32(
                    &format!("C10 c2Dot nan.x=0x{:08x} nan.y=0x{:08x}", n1.to_bits(), n2.to_bits()),
                    c(a, b),
                    r(a, b),
                )
            }
            // NaN in b instead of a (picks the other MULSS source)
            let a2 = C2v { x: 1.0, y: 1.0 };
            let b2 = C2v { x: n1, y: n2 };
            unsafe {
                eq_f32(&format!("C10 c2Dot b-nan 0x{:08x}/0x{:08x}", n1.to_bits(), n2.to_bits()), c(a2, b2), r(a2, b2))
            }
            // NaN on both sides of both products
            let a3 = C2v { x: n1, y: n1 };
            let b3 = C2v { x: n2, y: n2 };
            unsafe {
                eq_f32(&format!("C10 c2Dot both 0x{:08x}/0x{:08x}", n1.to_bits(), n2.to_bits()), c(a3, b3), r(a3, b3))
            }
        }
    }
    let mut g = Rng::seeded();
    for i in 0..N {
        let a = C2v { x: g.mixed_f32(), y: g.mixed_f32() };
        let b = C2v { x: g.mixed_f32(), y: g.mixed_f32() };
        unsafe { eq_f32(&format!("C10 c2Dot rnd #{i} {a:?} {b:?}"), c(a, b), r(a, b)) }
    }
}

#[test]
fn c11_dot_overflow_and_inf_cancellation() {
    let l = libs();
    let (c, r) = bind!(l, "c2Dot", FnC2Dot);
    let cases: &[(f32, f32, f32, f32)] = &[
        (3.0e38, 3.0e38, 1.0, 1.0),
        (3.0e38, 3.0e38, -1.0, 1.0),
        (f32::INFINITY, 0.0, 1.0, 1.0),
        (f32::INFINITY, f32::NEG_INFINITY, 1.0, 1.0),
        (f32::MAX, f32::MAX, f32::MAX, -f32::MAX),
        (1e-30, 1e-30, 1e-30, 1e-30),
        (f32::MIN_POSITIVE, f32::MIN_POSITIVE, 0.5, 0.5),
    ];
    for (i, &(ax, ay, bx, by)) in cases.iter().enumerate() {
        let a = C2v { x: ax, y: ay };
        let b = C2v { x: bx, y: by };
        unsafe { eq_f32(&format!("C11 c2Dot #{i}"), c(a, b), r(a, b)) }
    }
    let mut g = Rng::seeded();
    for i in 0..N {
        let s = if i % 2 == 0 { 3.0e38 } else { 1e-30 };
        let a = C2v { x: g.finite_f32(s), y: g.finite_f32(s) };
        let b = C2v { x: g.finite_f32(s), y: g.finite_f32(s) };
        unsafe { eq_f32(&format!("C11 c2Dot extreme #{i}"), c(a, b), r(a, b)) }
    }
}

// ---------------------------------------------------------------------------
// C12-C15 — c2CircletoCircle
// ---------------------------------------------------------------------------

fn circles(tag: &str, gen: impl Fn(&mut Rng) -> (C2Circle, C2Circle), n: usize) {
    let l = libs();
    let (c, r) = bind!(l, "c2CircletoCircle", FnCircleCircle);
    let mut g = Rng::seeded();
    for i in 0..n {
        let (a, b) = gen(&mut g);
        unsafe {
            eq_i32(
                &format!("{tag} c2CircletoCircle #{i} A={a:?} B={b:?}"),
                c(a, b),
                r(a, b),
            )
        }
    }
}

#[test]
fn c12_circle_circle_overlapping() {
    circles(
        "C12",
        |g| {
            let p = C2v { x: g.finite_f32(10.0), y: g.finite_f32(10.0) };
            let ra = g.range_f32(1.0, 5.0);
            let rb = g.range_f32(1.0, 5.0);
            // put B within (ra+rb) of A
            let d = g.range_f32(0.0, ra + rb);
            let ang = g.range_f32(0.0, 6.283185);
            (
                C2Circle { p, r: ra },
                C2Circle {
                    p: C2v { x: p.x + d * ang.cos(), y: p.y + d * ang.sin() },
                    r: rb,
                },
            )
        },
        N,
    );
}

#[test]
fn c13_circle_circle_disjoint_and_touching() {
    circles(
        "C13",
        |g| {
            let p = C2v { x: g.finite_f32(10.0), y: g.finite_f32(10.0) };
            let ra = g.range_f32(1.0, 5.0);
            let rb = g.range_f32(1.0, 5.0);
            let d = ra + rb + g.range_f32(0.0, 5.0);
            (
                C2Circle { p, r: ra },
                C2Circle { p: C2v { x: p.x + d, y: p.y }, r: rb },
            )
        },
        N,
    );
    // exactly touching: d2 == r2 (strict `<` must give 0)
    let l = libs();
    let (c, r) = bind!(l, "c2CircletoCircle", FnCircleCircle);
    for k in 1..200u32 {
        let ra = k as f32;
        let rb = (k * 2) as f32;
        let a = C2Circle { p: C2v { x: 0.0, y: 0.0 }, r: ra };
        let b = C2Circle { p: C2v { x: ra + rb, y: 0.0 }, r: rb };
        unsafe { eq_i32(&format!("C13 touching k={k}"), c(a, b), r(a, b)) }
    }
}

#[test]
fn c14_circle_circle_negative_and_zero_radius() {
    circles(
        "C14",
        |g| {
            let neg = |g: &mut Rng| match g.below(3) {
                0 => 0.0f32,
                1 => -g.range_f32(0.0, 10.0),
                _ => g.range_f32(0.0, 10.0),
            };
            (
                C2Circle {
                    p: C2v { x: g.finite_f32(10.0), y: g.finite_f32(10.0) },
                    r: neg(g),
                },
                C2Circle {
                    p: C2v { x: g.finite_f32(10.0), y: g.finite_f32(10.0) },
                    r: neg(g),
                },
            )
        },
        N,
    );
}

#[test]
fn c15_circle_circle_nonfinite() {
    circles(
        "C15",
        |g| {
            (
                C2Circle {
                    p: C2v { x: g.mixed_f32(), y: g.mixed_f32() },
                    r: g.mixed_f32(),
                },
                C2Circle {
                    p: C2v { x: g.mixed_f32(), y: g.mixed_f32() },
                    r: g.mixed_f32(),
                },
            )
        },
        N * 2,
    );
}

// ---------------------------------------------------------------------------
// C16-C19 — c2CircletoAABB
// ---------------------------------------------------------------------------

fn circle_aabb(tag: &str, gen: impl Fn(&mut Rng) -> (C2Circle, C2Aabb), n: usize) {
    let l = libs();
    let (c, r) = bind!(l, "c2CircletoAABB", FnCircleAabb);
    let mut g = Rng::seeded();
    for i in 0..n {
        let (a, b) = gen(&mut g);
        unsafe {
            eq_i32(
                &format!("{tag} c2CircletoAABB #{i} A={a:?} B={b:?}"),
                c(a, b),
                r(a, b),
            )
        }
    }
}

fn rand_box(g: &mut Rng) -> C2Aabb {
    let x0 = g.finite_f32(10.0);
    let y0 = g.finite_f32(10.0);
    C2Aabb {
        min: C2v { x: x0, y: y0 },
        max: C2v { x: x0 + g.range_f32(0.0, 20.0), y: y0 + g.range_f32(0.0, 20.0) },
    }
}

#[test]
fn c16_circle_aabb_centre_inside() {
    circle_aabb(
        "C16",
        |g| {
            let b = rand_box(g);
            let a = C2Circle {
                p: C2v {
                    x: g.range_f32(b.min.x, b.max.x),
                    y: g.range_f32(b.min.y, b.max.y),
                },
                r: g.range_f32(0.0, 5.0),
            };
            (a, b)
        },
        N,
    );
}

#[test]
fn c17_circle_aabb_all_eight_outside_regions() {
    let l = libs();
    let (c, r) = bind!(l, "c2CircletoAABB", FnCircleAabb);
    let mut g = Rng::seeded();
    // 3x3 grid of regions around the box (skipping the centre, covered by C16)
    for i in 0..N {
        let b = rand_box(&mut g);
        let w = b.max.x - b.min.x;
        let h = b.max.y - b.min.y;
        for gx in 0..3 {
            for gy in 0..3 {
                let x = match gx {
                    0 => b.min.x - g.range_f32(0.01, w + 1.0),
                    1 => g.range_f32(b.min.x, b.max.x),
                    _ => b.max.x + g.range_f32(0.01, w + 1.0),
                };
                let y = match gy {
                    0 => b.min.y - g.range_f32(0.01, h + 1.0),
                    1 => g.range_f32(b.min.y, b.max.y),
                    _ => b.max.y + g.range_f32(0.01, h + 1.0),
                };
                let a = C2Circle { p: C2v { x, y }, r: g.range_f32(0.0, 6.0) };
                unsafe {
                    eq_i32(
                        &format!("C17 c2CircletoAABB #{i} region({gx},{gy})"),
                        c(a, b),
                        r(a, b),
                    )
                }
            }
        }
    }
}

#[test]
fn c18_circle_aabb_inverted_box() {
    circle_aabb(
        "C18",
        |g| {
            let b = rand_box(g);
            let inv = C2Aabb { min: b.max, max: b.min };
            let a = C2Circle {
                p: C2v { x: g.finite_f32(20.0), y: g.finite_f32(20.0) },
                r: g.range_f32(0.0, 8.0),
            };
            (a, inv)
        },
        N,
    );
}

#[test]
fn c19_circle_aabb_nonfinite() {
    circle_aabb(
        "C19",
        |g| {
            (
                C2Circle {
                    p: C2v { x: g.mixed_f32(), y: g.mixed_f32() },
                    r: g.mixed_f32(),
                },
                C2Aabb {
                    min: C2v { x: g.mixed_f32(), y: g.mixed_f32() },
                    max: C2v { x: g.mixed_f32(), y: g.mixed_f32() },
                },
            )
        },
        N * 2,
    );
}

// ---------------------------------------------------------------------------
// C20-C22 — c2AABBtoAABB
// ---------------------------------------------------------------------------

#[test]
fn c20_aabb_aabb_overlap_and_each_axis_disjoint() {
    let l = libs();
    let (c, r) = bind!(l, "c2AABBtoAABB", FnAabbAabb);
    let mut g = Rng::seeded();
    for i in 0..N {
        let a = rand_box(&mut g);
        // overlapping
        let bo = C2Aabb {
            min: C2v {
                x: g.range_f32(a.min.x, a.max.x),
                y: g.range_f32(a.min.y, a.max.y),
            },
            max: C2v {
                x: a.max.x + g.range_f32(0.0, 5.0),
                y: a.max.y + g.range_f32(0.0, 5.0),
            },
        };
        unsafe { eq_i32(&format!("C20 overlap #{i}"), c(a, bo), r(a, bo)) }

        // d0: B.max.x < A.min.x
        let b0 = C2Aabb {
            min: C2v { x: a.min.x - 20.0, y: a.min.y },
            max: C2v { x: a.min.x - g.range_f32(0.01, 5.0), y: a.max.y },
        };
        // d1: A.max.x < B.min.x
        let b1 = C2Aabb {
            min: C2v { x: a.max.x + g.range_f32(0.01, 5.0), y: a.min.y },
            max: C2v { x: a.max.x + 30.0, y: a.max.y },
        };
        // d2: B.max.y < A.min.y
        let b2 = C2Aabb {
            min: C2v { x: a.min.x, y: a.min.y - 20.0 },
            max: C2v { x: a.max.x, y: a.min.y - g.range_f32(0.01, 5.0) },
        };
        // d3: A.max.y < B.min.y
        let b3 = C2Aabb {
            min: C2v { x: a.min.x, y: a.max.y + g.range_f32(0.01, 5.0) },
            max: C2v { x: a.max.x, y: a.max.y + 30.0 },
        };
        for (k, b) in [b0, b1, b2, b3].iter().enumerate() {
            unsafe { eq_i32(&format!("C20 disjoint d{k} #{i}"), c(a, *b), r(a, *b)) }
        }
        // fully random pairs
        let br = rand_box(&mut g);
        unsafe { eq_i32(&format!("C20 rnd #{i}"), c(a, br), r(a, br)) }
    }
}

#[test]
fn c21_aabb_aabb_edge_touching() {
    let l = libs();
    let (c, r) = bind!(l, "c2AABBtoAABB", FnAabbAabb);
    let mut g = Rng::seeded();
    for i in 0..N {
        let a = rand_box(&mut g);
        // B.max.x == A.min.x exactly -> strict `<` is false -> "collide"
        let cases = [
            C2Aabb { min: C2v { x: a.min.x - 5.0, y: a.min.y }, max: C2v { x: a.min.x, y: a.max.y } },
            C2Aabb { min: C2v { x: a.max.x, y: a.min.y }, max: C2v { x: a.max.x + 5.0, y: a.max.y } },
            C2Aabb { min: C2v { x: a.min.x, y: a.min.y - 5.0 }, max: C2v { x: a.max.x, y: a.min.y } },
            C2Aabb { min: C2v { x: a.min.x, y: a.max.y }, max: C2v { x: a.max.x, y: a.max.y + 5.0 } },
        ];
        for (k, b) in cases.iter().enumerate() {
            unsafe { eq_i32(&format!("C21 touch{k} #{i}"), c(a, *b), r(a, *b)) }
        }
    }
}

#[test]
fn c22_aabb_aabb_nonfinite() {
    let l = libs();
    let (c, r) = bind!(l, "c2AABBtoAABB", FnAabbAabb);
    let nan = f32::NAN;
    let all_nan = C2Aabb {
        min: C2v { x: nan, y: nan },
        max: C2v { x: nan, y: nan },
    };
    unsafe {
        eq_i32("C22 all-NaN vs all-NaN", c(all_nan, all_nan), r(all_nan, all_nan));
    }
    let mut g = Rng::seeded();
    for i in 0..N * 2 {
        let a = C2Aabb {
            min: C2v { x: g.mixed_f32(), y: g.mixed_f32() },
            max: C2v { x: g.mixed_f32(), y: g.mixed_f32() },
        };
        let b = C2Aabb {
            min: C2v { x: g.mixed_f32(), y: g.mixed_f32() },
            max: C2v { x: g.mixed_f32(), y: g.mixed_f32() },
        };
        unsafe { eq_i32(&format!("C22 rnd #{i} {a:?} {b:?}"), c(a, b), r(a, b)) }
    }
}
