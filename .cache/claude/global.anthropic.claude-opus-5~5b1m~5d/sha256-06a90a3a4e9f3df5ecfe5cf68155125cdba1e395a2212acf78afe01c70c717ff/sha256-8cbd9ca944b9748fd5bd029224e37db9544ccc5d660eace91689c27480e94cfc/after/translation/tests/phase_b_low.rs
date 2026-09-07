//! Phase B -- valid-path differential tests, CONFIGS.md rows 1..=29
//! (leaf vector helpers + boolean overlap predicates).
//!
//! Every call goes through the exported symbols of BOTH `.so`s.

#![allow(non_snake_case)]

mod common;
use common::*;

const SEED: u64 = 0x2545_F491_4F6C_DD1D;
/// Base iteration count for the randomized sweeps; scaled by `$DIFF_SCALE`.
fn n() -> usize { iters(20_000) }

/* ============================ row 1 / 2 : c2V ============================ */

#[test]
fn cfg01_c2V_random_bit_patterns() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 1);
    for i in 0..n() {
        let (x, y) = (g.wild(), g.wild());
        let c = unsafe { (l.c.c2V)(x, y) };
        let r = unsafe { (l.r.c2V)(x, y) };
        same(&format!("row1 i={i} x={x:e} y={y:e}"), c, r);
    }
}

#[test]
fn cfg02_c2V_library_constants() {
    let l = libs();
    for (x, y) in [
        (-1.0f32, 0.0f32),
        (1.0, 0.0),
        (0.0, -1.0),
        (0.0, 1.0),
        (-0.0, 0.0),
        (0.0, -0.0),
        (-5.5, 3.25),
        (f32::NAN, 0.0),
    ] {
        let c = unsafe { (l.c.c2V)(x, y) };
        let r = unsafe { (l.r.c2V)(x, y) };
        same(&format!("row2 ({x},{y})"), c, r);
    }
}

/* ========================= rows 3..5 : c2Dot ============================= */

#[test]
fn cfg03_c2Dot_finite_normals() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 3);
    for i in 0..n() {
        let (a, b) = (g.coord_v(), g.coord_v());
        let c = unsafe { (l.c.c2Dot)(a, b) };
        let r = unsafe { (l.r.c2Dot)(a, b) };
        same(&format!("row3 i={i} a={a:?} b={b:?}"), c, r);
    }
}

#[test]
fn cfg04_c2Dot_mixed_classes_and_cancellation() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 4);
    // exact cancellation: a.x*b.x == -(a.y*b.y)
    for i in 0..2000 {
        let k = g.below(64) as f32 + 1.0;
        for (a, b) in [
            (v(k, k), v(k, -k)),
            (v(k, -k), v(k, k)),
            (v(0.0, k), v(-0.0, k)),
            (v(-0.0, -0.0), v(0.0, 0.0)),
            (v(f32::MIN_POSITIVE, f32::MIN_POSITIVE), v(k, -k)),
        ] {
            let c = unsafe { (l.c.c2Dot)(a, b) };
            let r = unsafe { (l.r.c2Dot)(a, b) };
            same(&format!("row4 i={i} a={a:?} b={b:?}"), c, r);
        }
    }
    for i in 0..n() {
        let (a, b) = (g.wild_v(), g.wild_v());
        let c = unsafe { (l.c.c2Dot)(a, b) };
        let r = unsafe { (l.r.c2Dot)(a, b) };
        same(&format!("row4-wild i={i} a={a:?} b={b:?}"), c, r);
    }
}

#[test]
fn cfg05_c2Dot_overflow_and_inf_times_zero() {
    let l = libs();
    for (a, b) in [
        (v(1e30, 1e30), v(1e30, 1e30)),
        (v(1e30, -1e30), v(1e30, 1e30)),
        (v(f32::INFINITY, 0.0), v(0.0, 1.0)),
        (v(f32::INFINITY, 1.0), v(f32::NEG_INFINITY, 1.0)),
        (v(f32::MAX, f32::MAX), v(2.0, 2.0)),
        (v(f32::INFINITY, f32::NEG_INFINITY), v(1.0, 1.0)),
    ] {
        let c = unsafe { (l.c.c2Dot)(a, b) };
        let r = unsafe { (l.r.c2Dot)(a, b) };
        same(&format!("row5 a={a:?} b={b:?}"), c, r);
    }
}

/* ========================= rows 6..7 : c2Len ============================= */

#[test]
fn cfg06_c2Len_finite() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 6);
    let mut cases: Vec<c2v> = vec![
        v(3.0, 4.0),
        v(-3.0, -4.0),
        v(0.0, 0.0),
        v(-0.0, -0.0),
        v(1.0, 0.0),
        v(5.0, 12.0),
    ];
    for _ in 0..n() {
        cases.push(g.coord_v());
    }
    for (i, a) in cases.iter().enumerate() {
        let c = unsafe { (l.c.c2Len)(*a) };
        let r = unsafe { (l.r.c2Len)(*a) };
        same(&format!("row6 i={i} a={a:?}"), c, r);
    }
}

#[test]
fn cfg07_c2Len_specials() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 7);
    let mut cases: Vec<c2v> = vec![
        v(0.0, 0.0),
        v(f32::INFINITY, 0.0),
        v(f32::NEG_INFINITY, f32::INFINITY),
        v(f32::NAN, 1.0),
        v(f32::MIN_POSITIVE, f32::MIN_POSITIVE),
        v(f32::from_bits(1), f32::from_bits(1)),
        v(1e30, 1e30),
        v(f32::MAX, f32::MAX),
    ];
    for _ in 0..n() {
        cases.push(g.wild_v());
    }
    for (i, a) in cases.iter().enumerate() {
        let c = unsafe { (l.c.c2Len)(*a) };
        let r = unsafe { (l.r.c2Len)(*a) };
        same(&format!("row7 i={i} a={a:?}"), c, r);
    }
}

/* ====================== rows 8..9 : c2Add / c2Sub ======================== */

#[test]
fn cfg08_c2Add_c2Sub_signed_zeros() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 8);
    let zeros = [0.0f32, -0.0f32];
    for &ax in &zeros {
        for &ay in &zeros {
            for &bx in &zeros {
                for &by in &zeros {
                    let (a, b) = (v(ax, ay), v(bx, by));
                    same(
                        &format!("row8 add a={a:?} b={b:?}"),
                        unsafe { (l.c.c2Add)(a, b) },
                        unsafe { (l.r.c2Add)(a, b) },
                    );
                    same(
                        &format!("row8 sub a={a:?} b={b:?}"),
                        unsafe { (l.c.c2Sub)(a, b) },
                        unsafe { (l.r.c2Sub)(a, b) },
                    );
                }
            }
        }
    }
    for i in 0..n() {
        let (a, b) = (g.coord_v(), g.coord_v());
        same(
            &format!("row8 add i={i} a={a:?} b={b:?}"),
            unsafe { (l.c.c2Add)(a, b) },
            unsafe { (l.r.c2Add)(a, b) },
        );
        same(
            &format!("row8 sub i={i} a={a:?} b={b:?}"),
            unsafe { (l.c.c2Sub)(a, b) },
            unsafe { (l.r.c2Sub)(a, b) },
        );
    }
}

#[test]
fn cfg09_c2Add_c2Sub_inf_nan_overflow() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 9);
    let mut cases: Vec<(c2v, c2v)> = vec![
        (v(f32::INFINITY, 1.0), v(f32::INFINITY, 1.0)),
        (v(f32::INFINITY, 1.0), v(f32::NEG_INFINITY, 1.0)),
        (v(f32::MAX, f32::MAX), v(f32::MAX, f32::MAX)),
        (v(f32::MIN, f32::MIN), v(f32::MIN, f32::MIN)),
        (v(f32::NAN, f32::NAN), v(1.0, 2.0)),
    ];
    for _ in 0..n() {
        cases.push((g.wild_v(), g.wild_v()));
    }
    for (i, (a, b)) in cases.iter().enumerate() {
        same(
            &format!("row9 add i={i} a={a:?} b={b:?}"),
            unsafe { (l.c.c2Add)(*a, *b) },
            unsafe { (l.r.c2Add)(*a, *b) },
        );
        same(
            &format!("row9 sub i={i} a={a:?} b={b:?}"),
            unsafe { (l.c.c2Sub)(*a, *b) },
            unsafe { (l.r.c2Sub)(*a, *b) },
        );
    }
}

/* ========================== row 10 : c2Mulvs ============================= */

#[test]
fn cfg10_c2Mulvs_all_scalar_classes() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 10);
    let scalars = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        0.5,
        f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        1e30,
        1e-30,
    ];
    for &s in &scalars {
        for _ in 0..500 {
            let a = g.wild_v();
            same(
                &format!("row10 a={a:?} s={s:e}"),
                unsafe { (l.c.c2Mulvs)(a, s) },
                unsafe { (l.r.c2Mulvs)(a, s) },
            );
        }
    }
    for i in 0..n() {
        let (a, s) = (g.wild_v(), g.wild());
        same(
            &format!("row10-wild i={i} a={a:?} s={s:e}"),
            unsafe { (l.c.c2Mulvs)(a, s) },
            unsafe { (l.r.c2Mulvs)(a, s) },
        );
    }
}

/* ====================== rows 11..12 : c2Div ============================== */

#[test]
fn cfg11_c2Div_zero_inf_denormal_divisors() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 11);
    let divisors = [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::NAN,
        1e-30,
        1e30,
        f32::MAX,
    ];
    for &b in &divisors {
        for _ in 0..500 {
            let a = g.wild_v();
            same(
                &format!("row11 a={a:?} b={b:e}"),
                unsafe { (l.c.c2Div)(a, b) },
                unsafe { (l.r.c2Div)(a, b) },
            );
        }
        for a in [v(0.0, 0.0), v(-0.0, 1.0), v(1.0, -1.0)] {
            same(
                &format!("row11 fixed a={a:?} b={b:e}"),
                unsafe { (l.c.c2Div)(a, b) },
                unsafe { (l.r.c2Div)(a, b) },
            );
        }
    }
}

#[test]
fn cfg12_c2Div_is_reciprocal_multiply_not_division() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 12);
    // Divisors where a*(1/b) != a/b, so the exact C formula matters.
    let divisors: Vec<f32> = (1..200).map(|k| k as f32).chain([3.0, 7.0, 49.0]).collect();
    for &b in &divisors {
        for _ in 0..60 {
            let a = g.coord_v();
            same(
                &format!("row12 a={a:?} b={b}"),
                unsafe { (l.c.c2Div)(a, b) },
                unsafe { (l.r.c2Div)(a, b) },
            );
        }
    }
    for i in 0..n() {
        let (a, b) = (g.coord_v(), g.coord());
        same(
            &format!("row12-rand i={i} a={a:?} b={b:e}"),
            unsafe { (l.c.c2Div)(a, b) },
            unsafe { (l.r.c2Div)(a, b) },
        );
    }
}

/* ====================== rows 13..14 : c2Norm ============================= */

#[test]
fn cfg13_c2Norm_finite() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 13);
    for i in 0..n() {
        let a = g.coord_v();
        same(
            &format!("row13 i={i} a={a:?}"),
            unsafe { (l.c.c2Norm)(a) },
            unsafe { (l.r.c2Norm)(a) },
        );
    }
}

#[test]
fn cfg14_c2Norm_degenerate_inf_overflow_denormal() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 14);
    let mut cases = vec![
        v(0.0, 0.0),
        v(-0.0, -0.0),
        v(0.0, -0.0),
        v(f32::INFINITY, 0.0),
        v(f32::INFINITY, f32::INFINITY),
        v(f32::NEG_INFINITY, 1.0),
        v(1e30, 1e30),
        v(f32::MAX, f32::MAX),
        v(f32::MIN_POSITIVE, f32::MIN_POSITIVE),
        v(f32::from_bits(1), 0.0),
        v(f32::NAN, 1.0),
        v(1.0, f32::NAN),
    ];
    for _ in 0..n() {
        cases.push(g.wild_v());
    }
    for (i, a) in cases.iter().enumerate() {
        same(
            &format!("row14 i={i} a={a:?}"),
            unsafe { (l.c.c2Norm)(*a) },
            unsafe { (l.r.c2Norm)(*a) },
        );
    }
}

/* ================== rows 15..16 : c2Minv / c2Maxv ======================== */

#[test]
fn cfg15_c2Minv_c2Maxv_ties_and_signed_zero() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 15);
    let mut cases: Vec<(c2v, c2v)> = vec![
        (v(1.0, 2.0), v(1.0, 2.0)),   // exact ties -> else branch
        (v(0.0, 0.0), v(-0.0, -0.0)), // +0 vs -0 (differs from fminf)
        (v(-0.0, -0.0), v(0.0, 0.0)),
        (v(-0.0, 0.0), v(0.0, -0.0)),
        (v(5.0, -5.0), v(5.0, -5.0)),
    ];
    for _ in 0..n() {
        let k = g.coord();
        cases.push((v(k, k), v(k, k))); // ties on random values
        cases.push((g.coord_v(), g.coord_v()));
    }
    for (i, (a, b)) in cases.iter().enumerate() {
        same(
            &format!("row15 min i={i} a={a:?} b={b:?}"),
            unsafe { (l.c.c2Minv)(*a, *b) },
            unsafe { (l.r.c2Minv)(*a, *b) },
        );
        same(
            &format!("row15 max i={i} a={a:?} b={b:?}"),
            unsafe { (l.c.c2Maxv)(*a, *b) },
            unsafe { (l.r.c2Maxv)(*a, *b) },
        );
    }
}

#[test]
fn cfg16_c2Minv_c2Maxv_nan_semantics() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 16);
    let mut cases: Vec<(c2v, c2v)> = vec![
        (v(f32::NAN, f32::NAN), v(1.0, 2.0)),
        (v(1.0, 2.0), v(f32::NAN, f32::NAN)),
        (v(f32::NAN, 1.0), v(f32::NAN, 1.0)),
        (v(-f32::NAN, f32::INFINITY), v(f32::NAN, f32::NEG_INFINITY)),
        (v(f32::INFINITY, f32::NEG_INFINITY), v(f32::NEG_INFINITY, f32::INFINITY)),
    ];
    for _ in 0..n() {
        cases.push((g.wild_v(), g.wild_v()));
    }
    for (i, (a, b)) in cases.iter().enumerate() {
        same(
            &format!("row16 min i={i} a={a:?} b={b:?}"),
            unsafe { (l.c.c2Minv)(*a, *b) },
            unsafe { (l.r.c2Minv)(*a, *b) },
        );
        same(
            &format!("row16 max i={i} a={a:?} b={b:?}"),
            unsafe { (l.c.c2Maxv)(*a, *b) },
            unsafe { (l.r.c2Maxv)(*a, *b) },
        );
    }
}

/* ================== row 17 : c2Skew / c2CCW90 ============================ */

#[test]
fn cfg17_c2Skew_c2CCW90() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 17);
    let mut cases = vec![
        v(0.0, 0.0),
        v(-0.0, -0.0),
        v(0.0, -0.0),
        v(f32::NAN, 1.0),
        v(-f32::NAN, -1.0),
        v(f32::INFINITY, f32::NEG_INFINITY),
        v(1.0, 2.0),
    ];
    for _ in 0..n() {
        cases.push(g.wild_v());
    }
    for (i, a) in cases.iter().enumerate() {
        same(
            &format!("row17 skew i={i} a={a:?}"),
            unsafe { (l.c.c2Skew)(*a) },
            unsafe { (l.r.c2Skew)(*a) },
        );
        same(
            &format!("row17 ccw90 i={i} a={a:?}"),
            unsafe { (l.c.c2CCW90)(*a) },
            unsafe { (l.r.c2CCW90)(*a) },
        );
    }
}

/* ========================== row 18 : c2Absv ============================== */

#[test]
fn cfg18_c2Absv_ternary_not_fabsf() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 18);
    let mut cases = vec![
        v(-0.0, -0.0), // ternary keeps -0.0 (fabsf would give +0.0)
        v(0.0, -0.0),
        v(-f32::NAN, -f32::NAN), // ternary keeps the sign bit of a NaN
        v(f32::NAN, -f32::NAN),
        v(f32::NEG_INFINITY, f32::INFINITY),
        v(-1.0, 1.0),
        v(f32::MIN, f32::MAX),
        v(-f32::MIN_POSITIVE, f32::MIN_POSITIVE),
    ];
    for _ in 0..n() {
        cases.push(g.wild_v());
    }
    for (i, a) in cases.iter().enumerate() {
        same(
            &format!("row18 i={i} a={a:?}"),
            unsafe { (l.c.c2Absv)(*a) },
            unsafe { (l.r.c2Absv)(*a) },
        );
    }
}

/* ===================== rows 19..20 : c2MulmvT ============================ */

#[test]
fn cfg19_c2MulmvT_random() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 19);
    for i in 0..n() {
        let m = c2m {
            x: g.coord_v(),
            y: g.coord_v(),
        };
        let b = g.coord_v();
        same(
            &format!("row19 i={i} m={m:?} b={b:?}"),
            unsafe { (l.c.c2MulmvT)(m, b) },
            unsafe { (l.r.c2MulmvT)(m, b) },
        );
    }
    for i in 0..n() {
        let m = c2m {
            x: g.wild_v(),
            y: g.wild_v(),
        };
        let b = g.wild_v();
        same(
            &format!("row19-wild i={i} m={m:?} b={b:?}"),
            unsafe { (l.c.c2MulmvT)(m, b) },
            unsafe { (l.r.c2MulmvT)(m, b) },
        );
    }
}

#[test]
fn cfg20_c2MulmvT_matrices_the_library_actually_builds() {
    let l = libs();
    let mut g = Rng::new(SEED ^ 20);
    for i in 0..5000 {
        // exactly what c2RaytoCapsule builds: M.y = Norm(b-a), M.x = CCW90(M.y)
        let (a, b) = (g.coord_v(), g.coord_v());
        let dir = unsafe { (l.c.c2Sub)(b, a) };
        let y = unsafe { (l.c.c2Norm)(dir) };
        let x = unsafe { (l.c.c2CCW90)(y) };
        let m = c2m { x, y };
        for arg in [dir, unsafe { (l.c.c2Sub)(g.coord_v(), a) }, g.coord_v()] {
            same(
                &format!("row20 i={i} m={m:?} arg={arg:?}"),
                unsafe { (l.c.c2MulmvT)(m, arg) },
                unsafe { (l.r.c2MulmvT)(m, arg) },
            );
        }
    }
    // the NaN matrix from a degenerate capsule (a == b)
    let nanv = unsafe { (l.c.c2Norm)(v(0.0, 0.0)) };
    let m = c2m {
        x: unsafe { (l.c.c2CCW90)(nanv) },
        y: nanv,
    };
    for arg in [v(0.0, 0.0), v(1.0, 2.0), v(f32::NAN, 1.0)] {
        same(
            &format!("row20 nan-matrix arg={arg:?}"),
            unsafe { (l.c.c2MulmvT)(m, arg) },
            unsafe { (l.r.c2MulmvT)(m, arg) },
        );
    }
}

/* ================== rows 21..25 : c2AABBtoAABB =========================== */

fn diff_aabbtoaabb(ctx: &str, a: c2AABB, b: c2AABB) {
    let l = libs();
    same(
        &format!("c2AABBtoAABB {ctx} a={a:?} b={b:?}"),
        unsafe { (l.c.c2AABBtoAABB)(a, b) },
        unsafe { (l.r.c2AABBtoAABB)(a, b) },
    );
}

fn rand_box(g: &mut Rng) -> c2AABB {
    let (x0, y0) = (g.coord(), g.coord());
    let (w, h) = (g.radius(), g.radius());
    c2AABB {
        min: v(x0, y0),
        max: v(x0 + w, y0 + h),
    }
}

#[test]
fn cfg21_aabbtoaabb_overlapping() {
    let mut g = Rng::new(SEED ^ 21);
    for i in 0..n() {
        let a = rand_box(&mut g);
        // build b so that it overlaps a most of the time
        let jx = g.uniform(2.0);
        let jy = g.uniform(2.0);
        let b = c2AABB {
            min: v(a.min.x + jx, a.min.y + jy),
            max: v(a.max.x + jx, a.max.y + jy),
        };
        diff_aabbtoaabb(&format!("row21 i={i}"), a, b);
    }
}

#[test]
fn cfg22_aabbtoaabb_disjoint_each_axis() {
    let mut g = Rng::new(SEED ^ 22);
    for i in 0..n() / 4 {
        let a = rand_box(&mut g);
        let w = (a.max.x - a.min.x).abs() + 1.0;
        let h = (a.max.y - a.min.y).abs() + 1.0;
        let shifts = [(-2.0 * w, 0.0), (2.0 * w, 0.0), (0.0, -2.0 * h), (0.0, 2.0 * h)];
        for (k, (dx, dy)) in shifts.iter().enumerate() {
            let b = c2AABB {
                min: v(a.min.x + dx, a.min.y + dy),
                max: v(a.max.x + dx, a.max.y + dy),
            };
            diff_aabbtoaabb(&format!("row22 i={i} axis={k}"), a, b);
            diff_aabbtoaabb(&format!("row22 i={i} axis={k} swapped"), b, a);
        }
        // diagonal (two axes at once)
        let b = c2AABB {
            min: v(a.min.x + 2.0 * w, a.min.y + 2.0 * h),
            max: v(a.max.x + 2.0 * w, a.max.y + 2.0 * h),
        };
        diff_aabbtoaabb(&format!("row22 i={i} diagonal"), a, b);
    }
}

#[test]
fn cfg23_aabbtoaabb_touching_edges_and_corners() {
    let mut g = Rng::new(SEED ^ 23);
    for i in 0..n() / 2 {
        let a = rand_box(&mut g);
        let w = a.max.x - a.min.x;
        let h = a.max.y - a.min.y;
        for (k, (dx, dy)) in [(w, 0.0), (-w, 0.0), (0.0, h), (0.0, -h), (w, h), (-w, -h)]
            .iter()
            .enumerate()
        {
            let b = c2AABB {
                min: v(a.min.x + dx, a.min.y + dy),
                max: v(a.max.x + dx, a.max.y + dy),
            };
            diff_aabbtoaabb(&format!("row23 i={i} touch={k}"), a, b);
        }
    }
}

#[test]
fn cfg24_aabbtoaabb_contained_degenerate_inverted() {
    let mut g = Rng::new(SEED ^ 24);
    for i in 0..n() / 2 {
        let a = rand_box(&mut g);
        // fully contained
        let inner = c2AABB {
            min: v((a.min.x + a.max.x) * 0.5, (a.min.y + a.max.y) * 0.5),
            max: v(
                (a.min.x + a.max.x) * 0.5 + 0.01,
                (a.min.y + a.max.y) * 0.5 + 0.01,
            ),
        };
        diff_aabbtoaabb(&format!("row24 i={i} contained"), a, inner);
        diff_aabbtoaabb(&format!("row24 i={i} containing"), inner, a);
        // zero area
        let p = g.coord_v();
        let zero = c2AABB { min: p, max: p };
        diff_aabbtoaabb(&format!("row24 i={i} zero-vs-a"), zero, a);
        diff_aabbtoaabb(&format!("row24 i={i} a-vs-zero"), a, zero);
        diff_aabbtoaabb(&format!("row24 i={i} zero-vs-zero"), zero, zero);
        // inverted (min > max)
        let inv = c2AABB {
            min: a.max,
            max: a.min,
        };
        diff_aabbtoaabb(&format!("row24 i={i} inv-vs-a"), inv, a);
        diff_aabbtoaabb(&format!("row24 i={i} a-vs-inv"), a, inv);
        diff_aabbtoaabb(&format!("row24 i={i} inv-vs-inv"), inv, inv);
    }
}

#[test]
fn cfg25_aabbtoaabb_nan_inf() {
    let mut g = Rng::new(SEED ^ 25);
    for i in 0..n() {
        let a = c2AABB {
            min: g.wild_v(),
            max: g.wild_v(),
        };
        let b = c2AABB {
            min: g.wild_v(),
            max: g.wild_v(),
        };
        diff_aabbtoaabb(&format!("row25 i={i}"), a, b);
    }
    let nanb = c2AABB {
        min: v(f32::NAN, f32::NAN),
        max: v(f32::NAN, f32::NAN),
    };
    let infb = c2AABB {
        min: v(f32::NEG_INFINITY, f32::NEG_INFINITY),
        max: v(f32::INFINITY, f32::INFINITY),
    };
    let unit = c2AABB {
        min: v(0.0, 0.0),
        max: v(1.0, 1.0),
    };
    for (k, (a, b)) in [
        (nanb, unit),
        (unit, nanb),
        (nanb, nanb),
        (infb, unit),
        (unit, infb),
        (infb, nanb),
    ]
    .iter()
    .enumerate()
    {
        diff_aabbtoaabb(&format!("row25 fixed={k}"), *a, *b);
    }
}

/* ================== rows 26..27 : c2AABBtoPoint ========================== */

fn diff_aabbtopoint(ctx: &str, a: c2AABB, p: c2v) {
    let l = libs();
    same(
        &format!("c2AABBtoPoint {ctx} a={a:?} p={p:?}"),
        unsafe { (l.c.c2AABBtoPoint)(a, p) },
        unsafe { (l.r.c2AABBtoPoint)(a, p) },
    );
}

#[test]
fn cfg26_aabbtopoint_inside_outside_on_edges() {
    let mut g = Rng::new(SEED ^ 26);
    for i in 0..n() / 2 {
        let a = rand_box(&mut g);
        let cx = (a.min.x + a.max.x) * 0.5;
        let cy = (a.min.y + a.max.y) * 0.5;
        let w = a.max.x - a.min.x;
        let h = a.max.y - a.min.y;
        let pts = [
            v(cx, cy),                            // inside
            v(a.min.x, a.min.y),                  // corner (exact)
            v(a.max.x, a.max.y),                  // corner (exact)
            v(a.min.x, a.max.y),
            v(a.max.x, a.min.y),
            v(a.min.x, cy),                       // on left edge
            v(a.max.x, cy),                       // on right edge
            v(cx, a.min.y),
            v(cx, a.max.y),
            v(a.min.x - w - 1.0, cy),             // outside each side
            v(a.max.x + w + 1.0, cy),
            v(cx, a.min.y - h - 1.0),
            v(cx, a.max.y + h + 1.0),
            g.coord_v(),                          // random
        ];
        for (k, p) in pts.iter().enumerate() {
            diff_aabbtopoint(&format!("row26 i={i} p={k}"), a, *p);
        }
    }
}

#[test]
fn cfg27_aabbtopoint_capsule_slab_degenerate_nan() {
    let mut g = Rng::new(SEED ^ 27);
    for i in 0..n() / 2 {
        // the exact shape c2RaytoCapsule builds: {(-r, 0), (r, yBb.y)}
        let r = g.radius();
        for yb in [
            g.coord(),
            -g.coord().abs(), // inverted slab (yBb.y < 0)
            0.0,
        ] {
            let slab = c2AABB {
                min: v(-r, 0.0),
                max: v(r, yb),
            };
            for p in [v(0.0, 0.0), v(r, yb), v(-r, 0.0), g.coord_v(), g.wild_v()] {
                diff_aabbtopoint(&format!("row27 i={i} slab={slab:?}"), slab, p);
            }
        }
        // degenerate / inverted / wild boxes
        let p0 = g.coord_v();
        diff_aabbtopoint(
            &format!("row27 i={i} zero-area"),
            c2AABB { min: p0, max: p0 },
            p0,
        );
        let b = rand_box(&mut g);
        diff_aabbtopoint(
            &format!("row27 i={i} inverted"),
            c2AABB {
                min: b.max,
                max: b.min,
            },
            g.coord_v(),
        );
        diff_aabbtopoint(
            &format!("row27 i={i} wild"),
            c2AABB {
                min: g.wild_v(),
                max: g.wild_v(),
            },
            g.wild_v(),
        );
    }
}

/* ================== rows 28..29 : c2CircleToPoint ======================== */

fn diff_circletopoint(ctx: &str, c: c2Circle, p: c2v) {
    let l = libs();
    same(
        &format!("c2CircleToPoint {ctx} c={c:?} p={p:?}"),
        unsafe { (l.c.c2CircleToPoint)(c, p) },
        unsafe { (l.r.c2CircleToPoint)(c, p) },
    );
}

#[test]
fn cfg28_circletopoint_inside_outside_on_rim() {
    let mut g = Rng::new(SEED ^ 28);
    for i in 0..n() {
        let ctr = g.coord_v();
        let r = g.radius();
        let c = c2Circle { p: ctr, r };
        let pts = [
            ctr,                                       // centre
            v(ctr.x + r, ctr.y),                       // exactly on the rim
            v(ctr.x - r, ctr.y),
            v(ctr.x, ctr.y + r),
            v(ctr.x, ctr.y - r),
            v(ctr.x + r * 0.5, ctr.y),                 // inside
            v(ctr.x + r + 1.0, ctr.y),                 // outside
            // Pythagorean rim point: exact d2 == r*r for r == 5
            v(ctr.x + 3.0, ctr.y + 4.0),
            g.coord_v(),
        ];
        for (k, p) in pts.iter().enumerate() {
            diff_circletopoint(&format!("row28 i={i} p={k}"), c, *p);
        }
        // r == 5 exactly, rim point (3,4) => d2 == 25 == r*r (strict < rejects)
        let c5 = c2Circle { p: ctr, r: 5.0 };
        diff_circletopoint(&format!("row28 i={i} exact-rim"), c5, v(ctr.x + 3.0, ctr.y + 4.0));
    }
}

#[test]
fn cfg29_circletopoint_degenerate_radii() {
    let mut g = Rng::new(SEED ^ 29);
    let radii = [
        0.0f32,
        -0.0,
        -1.0,
        -5.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN_POSITIVE,
        f32::from_bits(1), // r*r underflows to 0
        1e-30,
        1e30,
        f32::MAX,
    ];
    for &r in &radii {
        for _ in 0..500 {
            let ctr = g.coord_v();
            let c = c2Circle { p: ctr, r };
            for p in [ctr, g.coord_v(), g.wild_v()] {
                diff_circletopoint(&format!("row29 r={r:e}"), c, p);
            }
        }
    }
    for i in 0..n() {
        let c = c2Circle {
            p: g.wild_v(),
            r: g.wild(),
        };
        let p = g.wild_v();
        diff_circletopoint(&format!("row29-wild i={i}"), c, p);
    }
}
