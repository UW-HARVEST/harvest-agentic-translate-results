//! Phase B — valid-path differential tests for the LOWEST-level entry points.
//!
//! Covers `CONFIGS.md` rows 1..15: `c2V`, `c2Sub`, `c2Dot`, `c2Mulvs`, `c2Maxv`,
//! `c2Minv`, `c2Clampv`. Every call goes through `dlsym` on both `.so`s and every
//! returned `f32` is compared with STRICT bit equality (`to_bits()`), so NaN
//! payloads, NaN sign bits and the sign of zero are all in scope.

#![allow(non_snake_case)]

mod common;
use common::*;

/// Cross product of `SPECIAL_F32` and `SPECIAL_BITS` as raw f32 values.
fn specials() -> Vec<f32> {
    let mut v: Vec<f32> = SPECIAL_F32.to_vec();
    v.extend(SPECIAL_BITS.iter().map(|&b| f32::from_bits(b)));
    v
}

// ===========================================================================
// Row 1 / 2 — c2V
// ===========================================================================

#[test]
fn row01_c2v_random_normals() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(1);
    for i in 0..4096 {
        let (x, y) = (rng.coord(), rng.coord());
        unsafe {
            let cv = (c.c2V)(x, y);
            let rv = (r.c2V)(x, y);
            diff_assert!(
                v_eq(cv, rv),
                "row01 #{i} c2V({}, {}): C={} RS={}",
                show(x),
                show(y),
                show_v(cv),
                show_v(rv)
            );
        }
    }
}

#[test]
fn row02_c2v_special_floats() {
    let (c, r) = libs();
    let sp = specials();
    for &x in &sp {
        for &y in &sp {
            unsafe {
                let cv = (c.c2V)(x, y);
                let rv = (r.c2V)(x, y);
                diff_assert!(
                    v_eq(cv, rv),
                    "row02 c2V({}, {}): C={} RS={}",
                    show(x),
                    show(y),
                    show_v(cv),
                    show_v(rv)
                );
            }
        }
    }
}

// ===========================================================================
// Row 3 / 4 — c2Sub
// ===========================================================================

#[test]
fn row03_c2sub_random_normals() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(3);
    for i in 0..4096 {
        let (a, b) = (rng.vec_coord(), rng.vec_coord());
        unsafe {
            let cv = (c.c2Sub)(a, b);
            let rv = (r.c2Sub)(a, b);
            diff_assert!(
                v_eq(cv, rv),
                "row03 #{i} c2Sub({}, {}): C={} RS={}",
                show_v(a),
                show_v(b),
                show_v(cv),
                show_v(rv)
            );
        }
    }
}

#[test]
fn row04_c2sub_special_floats() {
    let (c, r) = libs();
    let sp = specials();
    for &ax in &sp {
        for &bx in &sp {
            // Pair each x-lane combination with a rotated y-lane combination so
            // both lanes see the full special x special cross product.
            let a = c2v { x: ax, y: bx };
            let b = c2v { x: bx, y: ax };
            unsafe {
                let cv = (c.c2Sub)(a, b);
                let rv = (r.c2Sub)(a, b);
                diff_assert!(
                    v_eq(cv, rv),
                    "row04 c2Sub({}, {}): C={} RS={}",
                    show_v(a),
                    show_v(b),
                    show_v(cv),
                    show_v(rv)
                );
            }
        }
    }
    // Explicit sign-of-zero and inf-inf cases.
    let cases = [
        (0.0f32, 0.0f32),
        (0.0, -0.0),
        (-0.0, 0.0),
        (-0.0, -0.0),
        (f32::INFINITY, f32::INFINITY),
        (f32::NEG_INFINITY, f32::NEG_INFINITY),
        (f32::INFINITY, f32::NEG_INFINITY),
    ];
    for &(p, q) in &cases {
        let (a, b) = (c2v { x: p, y: q }, c2v { x: q, y: p });
        unsafe {
            let (cv, rv) = ((c.c2Sub)(a, b), (r.c2Sub)(a, b));
            diff_assert!(
                v_eq(cv, rv),
                "row04 signed-zero c2Sub({}, {}): C={} RS={}",
                show_v(a),
                show_v(b),
                show_v(cv),
                show_v(rv)
            );
        }
    }
}

// ===========================================================================
// Row 5 / 6 — c2Dot
// ===========================================================================

#[test]
fn row05_c2dot_random_normals() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(5);
    for i in 0..4096 {
        let (a, b) = (rng.vec_coord(), rng.vec_coord());
        unsafe {
            let cd = (c.c2Dot)(a, b);
            let rd = (r.c2Dot)(a, b);
            diff_assert!(
                f32_eq_bits(cd, rd),
                "row05 #{i} c2Dot({}, {}): C={} RS={}",
                show_v(a),
                show_v(b),
                show(cd),
                show(rd)
            );
        }
    }
}

#[test]
fn row06_c2dot_overflow_and_nan_payloads() {
    let (c, r) = libs();
    let sp = specials();
    // Full 4-lane cross product over a reduced special set, plus the 2-lane
    // cross product over the full set, to keep the run bounded but exhaustive
    // in the lanes that determine `mulss`/`addss` operand selection.
    for &ax in &sp {
        for &ay in &sp {
            for &bx in &sp {
                for &by in &sp {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    unsafe {
                        let cd = (c.c2Dot)(a, b);
                        let rd = (r.c2Dot)(a, b);
                        diff_assert!(
                            f32_eq_bits(cd, rd),
                            "row06 c2Dot({}, {}): C={} RS={}",
                            show_v(a),
                            show_v(b),
                            show(cd),
                            show(rd)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn row06b_c2dot_random_raw_bit_patterns() {
    // Property-style: 1M fully random 32-bit patterns per lane, so NaNs,
    // subnormals, infinities and huge exponents all occur naturally.
    let (c, r) = libs();
    let mut rng = Rng::seeded(6);
    for i in 0..1_000_000u64 {
        let (a, b) = (rng.vec_raw(), rng.vec_raw());
        unsafe {
            let cd = (c.c2Dot)(a, b);
            let rd = (r.c2Dot)(a, b);
            diff_assert!(
                f32_eq_bits(cd, rd),
                "row06b #{i} c2Dot({}, {}): C={} RS={}",
                show_v(a),
                show_v(b),
                show(cd),
                show(rd)
            );
        }
    }
}

// ===========================================================================
// Row 7 / 8 — c2Mulvs
// ===========================================================================

#[test]
fn row07_c2mulvs_random_normals() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(7);
    for i in 0..4096 {
        let a = rng.vec_coord();
        let s = rng.coord();
        unsafe {
            let cv = (c.c2Mulvs)(a, s);
            let rv = (r.c2Mulvs)(a, s);
            diff_assert!(
                v_eq(cv, rv),
                "row07 #{i} c2Mulvs({}, {}): C={} RS={}",
                show_v(a),
                show(s),
                show_v(cv),
                show_v(rv)
            );
        }
    }
}

#[test]
fn row08_c2mulvs_special_floats() {
    let (c, r) = libs();
    let sp = specials();
    for &ax in &sp {
        for &ay in &sp {
            for &s in &sp {
                let a = c2v { x: ax, y: ay };
                unsafe {
                    let cv = (c.c2Mulvs)(a, s);
                    let rv = (r.c2Mulvs)(a, s);
                    diff_assert!(
                        v_eq(cv, rv),
                        "row08 c2Mulvs({}, {}): C={} RS={}",
                        show_v(a),
                        show(s),
                        show_v(cv),
                        show_v(rv)
                    );
                }
            }
        }
    }
}

#[test]
fn row08b_c2mulvs_random_raw_bit_patterns() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(8);
    for i in 0..1_000_000u64 {
        let a = rng.vec_raw();
        let s = rng.raw_f32();
        unsafe {
            let cv = (c.c2Mulvs)(a, s);
            let rv = (r.c2Mulvs)(a, s);
            diff_assert!(
                v_eq(cv, rv),
                "row08b #{i} c2Mulvs({}, {}): C={} RS={}",
                show_v(a),
                show(s),
                show_v(cv),
                show_v(rv)
            );
        }
    }
}

// ===========================================================================
// Row 9 / 10 — c2Maxv,  Row 11 / 12 — c2Minv
// ===========================================================================

#[test]
fn row09_c2maxv_random() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(9);
    for i in 0..4096 {
        let (a, b) = (rng.vec_coord(), rng.vec_coord());
        unsafe {
            let (cv, rv) = ((c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
            diff_assert!(
                v_eq(cv, rv),
                "row09 #{i} c2Maxv({}, {}): C={} RS={}",
                show_v(a),
                show_v(b),
                show_v(cv),
                show_v(rv)
            );
        }
    }
    // Force all four ternary-branch combinations deterministically.
    for &(ax, bx) in &[(1.0f32, 2.0f32), (2.0, 1.0)] {
        for &(ay, by) in &[(1.0f32, 2.0f32), (2.0, 1.0)] {
            let (a, b) = (c2v { x: ax, y: ay }, c2v { x: bx, y: by });
            unsafe {
                let (cv, rv) = ((c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
                diff_assert!(v_eq(cv, rv), "row09 branch combo {ax}/{bx} {ay}/{by}");
            }
        }
    }
}

#[test]
fn row10_c2maxv_equal_and_signed_zero_and_nan() {
    let (c, r) = libs();
    let sp = specials();
    for &p in &sp {
        for &q in &sp {
            // (p,q) vs (q,p) and the equal case (p,p) vs (p,p).
            for &(a, b) in &[
                (c2v { x: p, y: q }, c2v { x: q, y: p }),
                (c2v { x: p, y: p }, c2v { x: p, y: p }),
                (c2v { x: p, y: q }, c2v { x: p, y: q }),
            ] {
                unsafe {
                    let (cv, rv) = ((c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
                    diff_assert!(
                        v_eq(cv, rv),
                        "row10 c2Maxv({}, {}): C={} RS={}",
                        show_v(a),
                        show_v(b),
                        show_v(cv),
                        show_v(rv)
                    );
                }
            }
        }
    }
}

#[test]
fn row11_c2minv_random() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(11);
    for i in 0..4096 {
        let (a, b) = (rng.vec_coord(), rng.vec_coord());
        unsafe {
            let (cv, rv) = ((c.c2Minv)(a, b), (r.c2Minv)(a, b));
            diff_assert!(
                v_eq(cv, rv),
                "row11 #{i} c2Minv({}, {}): C={} RS={}",
                show_v(a),
                show_v(b),
                show_v(cv),
                show_v(rv)
            );
        }
    }
    for &(ax, bx) in &[(1.0f32, 2.0f32), (2.0, 1.0)] {
        for &(ay, by) in &[(1.0f32, 2.0f32), (2.0, 1.0)] {
            let (a, b) = (c2v { x: ax, y: ay }, c2v { x: bx, y: by });
            unsafe {
                let (cv, rv) = ((c.c2Minv)(a, b), (r.c2Minv)(a, b));
                diff_assert!(v_eq(cv, rv), "row11 branch combo {ax}/{bx} {ay}/{by}");
            }
        }
    }
}

#[test]
fn row12_c2minv_equal_and_signed_zero_and_nan() {
    let (c, r) = libs();
    let sp = specials();
    for &p in &sp {
        for &q in &sp {
            for &(a, b) in &[
                (c2v { x: p, y: q }, c2v { x: q, y: p }),
                (c2v { x: p, y: p }, c2v { x: p, y: p }),
                (c2v { x: p, y: q }, c2v { x: p, y: q }),
            ] {
                unsafe {
                    let (cv, rv) = ((c.c2Minv)(a, b), (r.c2Minv)(a, b));
                    diff_assert!(
                        v_eq(cv, rv),
                        "row12 c2Minv({}, {}): C={} RS={}",
                        show_v(a),
                        show_v(b),
                        show_v(cv),
                        show_v(rv)
                    );
                }
            }
        }
    }
}

#[test]
fn row09_11_maxv_minv_random_raw_bits() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(910);
    for i in 0..500_000u64 {
        let (a, b) = (rng.vec_raw(), rng.vec_raw());
        unsafe {
            let (cv, rv) = ((c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
            diff_assert!(v_eq(cv, rv), "maxv raw #{i} {} {}", show_v(a), show_v(b));
            let (cv, rv) = ((c.c2Minv)(a, b), (r.c2Minv)(a, b));
            diff_assert!(v_eq(cv, rv), "minv raw #{i} {} {}", show_v(a), show_v(b));
        }
    }
}

// ===========================================================================
// Row 13 / 14 / 15 — c2Clampv
// ===========================================================================

#[test]
fn row13_c2clampv_proper_range() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(13);
    for i in 0..4096 {
        let bb = rng.aabb_proper();
        // Sample `a` inside, below and above the box.
        let a = match i % 3 {
            0 => c2v {
                x: bb.min.x + (bb.max.x - bb.min.x) * rng.unit(),
                y: bb.min.y + (bb.max.y - bb.min.y) * rng.unit(),
            },
            1 => c2v {
                x: bb.min.x - rng.unit() * 50.0,
                y: bb.min.y - rng.unit() * 50.0,
            },
            _ => c2v {
                x: bb.max.x + rng.unit() * 50.0,
                y: bb.max.y + rng.unit() * 50.0,
            },
        };
        unsafe {
            let (cv, rv) = (
                (c.c2Clampv)(a, bb.min, bb.max),
                (r.c2Clampv)(a, bb.min, bb.max),
            );
            diff_assert!(
                v_eq(cv, rv),
                "row13 #{i} c2Clampv({}, {}, {}): C={} RS={}",
                show_v(a),
                show_v(bb.min),
                show_v(bb.max),
                show_v(cv),
                show_v(rv)
            );
        }
    }
}

#[test]
fn row14_c2clampv_inverted_range() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(14);
    for i in 0..4096 {
        let bb = rng.aabb_proper();
        // Swap lo/hi so lo > hi — the C performs no ordering check.
        let (lo, hi) = (bb.max, bb.min);
        let a = rng.vec_coord();
        unsafe {
            let (cv, rv) = ((c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi));
            diff_assert!(
                v_eq(cv, rv),
                "row14 #{i} c2Clampv({}, lo={}, hi={}): C={} RS={}",
                show_v(a),
                show_v(lo),
                show_v(hi),
                show_v(cv),
                show_v(rv)
            );
        }
    }
}

#[test]
fn row15_c2clampv_degenerate_and_special() {
    let (c, r) = libs();
    let sp = specials();
    // lo == hi (degenerate) plus every special value in each of the three args.
    for &p in &sp {
        for &q in &sp {
            let combos = [
                // lo == hi
                (
                    c2v { x: p, y: q },
                    c2v { x: q, y: p },
                    c2v { x: q, y: p },
                ),
                // ±inf bounds
                (
                    c2v { x: p, y: q },
                    c2v {
                        x: f32::NEG_INFINITY,
                        y: f32::NEG_INFINITY,
                    },
                    c2v {
                        x: f32::INFINITY,
                        y: f32::INFINITY,
                    },
                ),
                // NaN in a
                (
                    c2v { x: f32::NAN, y: p },
                    c2v { x: q, y: q },
                    c2v { x: p, y: p },
                ),
                // NaN in the bounds
                (
                    c2v { x: p, y: q },
                    c2v { x: f32::NAN, y: q },
                    c2v { x: p, y: f32::NAN },
                ),
            ];
            for &(a, lo, hi) in &combos {
                unsafe {
                    let (cv, rv) = ((c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi));
                    diff_assert!(
                        v_eq(cv, rv),
                        "row15 c2Clampv({}, {}, {}): C={} RS={}",
                        show_v(a),
                        show_v(lo),
                        show_v(hi),
                        show_v(cv),
                        show_v(rv)
                    );
                }
            }
        }
    }
}

#[test]
fn row15b_c2clampv_random_raw_bits() {
    let (c, r) = libs();
    let mut rng = Rng::seeded(15);
    for i in 0..500_000u64 {
        let (a, lo, hi) = (rng.vec_raw(), rng.vec_raw(), rng.vec_raw());
        unsafe {
            let (cv, rv) = ((c.c2Clampv)(a, lo, hi), (r.c2Clampv)(a, lo, hi));
            diff_assert!(
                v_eq(cv, rv),
                "row15b #{i} c2Clampv({}, {}, {}): C={} RS={}",
                show_v(a),
                show_v(lo),
                show_v(hi),
                show_v(cv),
                show_v(rv)
            );
        }
    }
}
