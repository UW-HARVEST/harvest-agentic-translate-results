//! Phase B — valid-path differential tests for the vector primitives.
//!
//! Covers `CONFIGS.md` rows 1–28: `c2V`, `c2Minv`, `c2Maxv`, `c2Clampv`,
//! `c2Dot`, `c2Sub`.
//!
//! Every call is dispatched through BOTH `.so` files via `libloading`; every
//! float result is compared by raw bit pattern so NaN payloads and the sign of
//! zero are part of the assertion.

mod common;
use common::*;

const SEED: u64 = 0x0BAD_C0DE_1234_5678;

// ===========================================================================
// c2V — rows 1, 2, 32
// ===========================================================================

#[test]
fn row01_c2v_random_normals() {
    let l = libs();
    let mut rng = Rng::new(SEED);
    for _ in 0..20_000 {
        let (x, y) = (rng.signed(1.0e6), rng.signed(1.0e6));
        assert_v_bits_eq!("row01", l.c.c2V(x, y), l.rs.c2V(x, y), "c2V({x}, {y})");
    }
}

#[test]
fn row02_c2v_special_and_raw_bits() {
    let l = libs();

    // Exhaustive cross-product of the interesting patterns.
    for &x in SPECIAL_F32.iter() {
        for &y in SPECIAL_F32.iter() {
            assert_v_bits_eq!("row02/special", l.c.c2V(x, y), l.rs.c2V(x, y), "c2V");
        }
    }
    // All NaN patterns, in each slot. c2V is a pure field copy: a signalling
    // NaN must come back UNQUIETED from both implementations.
    for i in 0..NAN_F32_BITS.len() {
        for &other in SPECIAL_F32.iter() {
            let n = nan(i);
            assert_v_bits_eq!("row02/nan.x", l.c.c2V(n, other), l.rs.c2V(n, other), "c2V");
            assert_v_bits_eq!("row02/nan.y", l.c.c2V(other, n), l.rs.c2V(other, n), "c2V");
        }
        for j in 0..NAN_F32_BITS.len() {
            let (a, b) = (nan(i), nan(j));
            assert_v_bits_eq!("row02/nan.both", l.c.c2V(a, b), l.rs.c2V(a, b), "c2V");
        }
    }
    // Fully random bit patterns.
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..50_000 {
        let (x, y) = (rng.raw_f32(), rng.raw_f32());
        assert_v_bits_eq!(
            "row02/raw",
            l.c.c2V(x, y),
            l.rs.c2V(x, y),
            "c2V(0x{:08x}, 0x{:08x})",
            x.to_bits(),
            y.to_bits()
        );
    }
}

// ===========================================================================
// c2Minv / c2Maxv — rows 3–12, 17, 18
// ===========================================================================

/// Helper: run both min and max over a supplied pair, asserting bit equality.
fn check_minmax(l: &Pair, row: &str, a: c2v, b: c2v) {
    assert_v_bits_eq!(
        row,
        l.c.c2Minv(a, b),
        l.rs.c2Minv(a, b),
        "c2Minv({}, {})",
        showv(a),
        showv(b)
    );
    assert_v_bits_eq!(
        row,
        l.c.c2Maxv(a, b),
        l.rs.c2Maxv(a, b),
        "c2Maxv({}, {})",
        showv(a),
        showv(b)
    );
}

#[test]
fn row03_08_minmax_random_normals_all_orderings() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..30_000 {
        // Independent random components give every ordering of x and of y.
        let a = v(rng.signed(100.0), rng.signed(100.0));
        let b = v(rng.signed(100.0), rng.signed(100.0));
        check_minmax(&l, "row03+08", a, b);
    }
    // Explicitly force each of the 4 (x-order × y-order) quadrants.
    for &(ax, bx) in &[(1.0f32, 2.0f32), (2.0, 1.0)] {
        for &(ay, by) in &[(3.0f32, 4.0f32), (4.0, 3.0)] {
            check_minmax(&l, "row03+08/quadrant", v(ax, ay), v(bx, by));
        }
    }
}

#[test]
fn row04_09_minmax_equal_components_returns_b() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..5_000 {
        let x = rng.signed(50.0);
        let y = rng.signed(50.0);
        // a.x == b.x: `a > b` and `a < b` are both false, so C yields b.x.
        check_minmax(&l, "row04+09/x", v(x, y), v(x, rng.signed(50.0)));
        check_minmax(&l, "row04+09/y", v(rng.signed(50.0), y), v(x, y));
        check_minmax(&l, "row04+09/both", v(x, y), v(x, y));
    }
}

#[test]
fn row05_10_minmax_signed_zeros() {
    let l = libs();
    let zeros = [0.0f32, -0.0f32];
    for &ax in &zeros {
        for &ay in &zeros {
            for &bx in &zeros {
                for &by in &zeros {
                    check_minmax(&l, "row05+10", v(ax, ay), v(bx, by));
                }
            }
        }
    }
    // Signed zeros mixed with tiny values of either sign.
    for &z in &zeros {
        for &t in &[f32::MIN_POSITIVE, -f32::MIN_POSITIVE, 1e-45, -1e-45] {
            check_minmax(&l, "row05+10/tiny", v(z, t), v(t, z));
            check_minmax(&l, "row05+10/tiny", v(t, z), v(z, t));
        }
    }
}

#[test]
fn row06_11_minmax_nan_all_subsets() {
    let l = libs();
    // All 15 non-empty subsets of {a.x, a.y, b.x, b.y} replaced by a NaN.
    let base = [1.0f32, 2.0, 3.0, 4.0];
    for mask in 1u32..16 {
        for ni in 0..NAN_F32_BITS.len() {
            let mut c = base;
            for k in 0..4 {
                if mask & (1 << k) != 0 {
                    // vary the payload per slot so propagation is observable
                    c[k] = nan(ni + k);
                }
            }
            check_minmax(&l, "row06+11", v(c[0], c[1]), v(c[2], c[3]));
        }
    }
}

#[test]
fn row07_12_minmax_infinities() {
    let l = libs();
    let vals = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MAX,
        f32::MIN,
    ];
    for &ax in &vals {
        for &bx in &vals {
            for &ay in &vals {
                for &by in &vals {
                    check_minmax(&l, "row07+12", v(ax, ay), v(bx, by));
                }
            }
        }
    }
}

#[test]
fn row_minmax_fully_random_bits() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..100_000 {
        check_minmax(&l, "minmax/raw", rng.raw_v(), rng.raw_v());
    }
    for _ in 0..100_000 {
        check_minmax(&l, "minmax/nasty", rng.nasty_v(), rng.nasty_v());
    }
}

// ===========================================================================
// c2Clampv — rows 13–18
// ===========================================================================

fn check_clamp(l: &Pair, row: &str, a: c2v, lo: c2v, hi: c2v) {
    assert_v_bits_eq!(
        row,
        l.c.c2Clampv(a, lo, hi),
        l.rs.c2Clampv(a, lo, hi),
        "c2Clampv({}, {}, {})",
        showv(a),
        showv(lo),
        showv(hi)
    );
}

#[test]
fn row13_clamp_inside_interval() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..20_000 {
        let lo = v(rng.signed(100.0), rng.signed(100.0));
        let hi = v(lo.x + rng.signed(50.0).abs(), lo.y + rng.signed(50.0).abs());
        let a = v(
            lo.x + (hi.x - lo.x) * 0.5,
            lo.y + (hi.y - lo.y) * rng.signed(0.5).abs(),
        );
        check_clamp(&l, "row13", a, lo, hi);
    }
}

#[test]
fn row14_clamp_below_above_and_exactly_on_bounds() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..5_000 {
        let lo = v(rng.signed(20.0), rng.signed(20.0));
        let hi = v(lo.x + 10.0, lo.y + 10.0);
        // 3x3 grid: {below lo, == lo, inside, == hi, above hi} per component.
        let xs = [lo.x - 1.0, lo.x, (lo.x + hi.x) * 0.5, hi.x, hi.x + 1.0];
        let ys = [lo.y - 1.0, lo.y, (lo.y + hi.y) * 0.5, hi.y, hi.y + 1.0];
        for &x in &xs {
            for &y in &ys {
                check_clamp(&l, "row14", v(x, y), lo, hi);
            }
        }
    }
}

#[test]
fn row15_clamp_degenerate_interval() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..5_000 {
        let p = v(rng.signed(100.0), rng.signed(100.0));
        for &d in &[-1.0f32, 0.0, 1.0] {
            check_clamp(&l, "row15", v(p.x + d, p.y - d), p, p);
        }
    }
    // lo == hi == ±0.0
    for &z in &[0.0f32, -0.0f32] {
        for &w in &[0.0f32, -0.0f32] {
            check_clamp(&l, "row15/zero", v(w, w), v(z, z), v(z, z));
        }
    }
}

#[test]
fn row16_clamp_inverted_interval() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..20_000 {
        // lo strictly greater than hi — the C validates nothing.
        let hi = v(rng.signed(100.0), rng.signed(100.0));
        let lo = v(hi.x + rng.signed(50.0).abs() + 1.0, hi.y + 1.0);
        let a = v(rng.signed(200.0), rng.signed(200.0));
        check_clamp(&l, "row16", a, lo, hi);
    }
}

#[test]
fn row17_clamp_nan_and_inf() {
    let l = libs();
    // Every subset of the 6 components replaced by a NaN or an infinity.
    let base = [1.0f32, 2.0, -5.0, -4.0, 5.0, 6.0];
    let injects = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        nan(0),
        nan(2),
        nan(3),
        nan(4),
    ];
    for mask in 1u32..64 {
        for (ii, &inj) in injects.iter().enumerate() {
            let mut c = base;
            for k in 0..6 {
                if mask & (1 << k) != 0 {
                    c[k] = if inj.is_nan() { nan(ii + k) } else { inj };
                }
            }
            check_clamp(&l, "row17", v(c[0], c[1]), v(c[2], c[3]), v(c[4], c[5]));
        }
    }
}

#[test]
fn row18_clamp_signed_zero_endpoints() {
    let l = libs();
    let zs = [0.0f32, -0.0f32];
    for &lo in &zs {
        for &hi in &zs {
            for &a in &[0.0f32, -0.0, 1.0, -1.0, f32::MIN_POSITIVE, -1e-45] {
                check_clamp(&l, "row18", v(a, a), v(lo, hi), v(hi, lo));
            }
        }
    }
}

#[test]
fn row_clamp_fully_random_bits() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..100_000 {
        check_clamp(&l, "clamp/raw", rng.raw_v(), rng.raw_v(), rng.raw_v());
    }
    for _ in 0..100_000 {
        check_clamp(&l, "clamp/nasty", rng.nasty_v(), rng.nasty_v(), rng.nasty_v());
    }
}

// ===========================================================================
// c2Dot — rows 19–25
// ===========================================================================

fn check_dot(l: &Pair, row: &str, a: c2v, b: c2v) {
    assert_f32_bits_eq!(
        row,
        l.c.c2Dot(a, b),
        l.rs.c2Dot(a, b),
        "c2Dot({}, {})",
        showv(a),
        showv(b)
    );
}

#[test]
fn row19_dot_random_unit_scale() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..50_000 {
        check_dot(
            &l,
            "row19",
            v(rng.signed(1.0), rng.signed(1.0)),
            v(rng.signed(1.0), rng.signed(1.0)),
        );
    }
}

#[test]
fn row20_dot_full_exponent_range() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..100_000 {
        // Random exponents in [-60, 60] give products spanning the whole range
        // and exercise every rounding case of the mul+add chain.
        let s = |rng: &mut Rng| {
            let e = (rng.below(121) as i32) - 60;
            rng.signed(1.0) * (2.0f32).powi(e)
        };
        let a = v(s(&mut rng), s(&mut rng));
        let b = v(s(&mut rng), s(&mut rng));
        check_dot(&l, "row20", a, b);
    }
}

#[test]
fn row21_dot_overflow_and_underflow() {
    let l = libs();
    let big = [f32::MAX, -f32::MAX, 3.0e38, -3.0e38, 1.0e30, 2.0e19];
    let tiny = [
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        1.0e-30,
        -1.0e-30,
        1e-45,
        1e-20,
    ];
    for &p in &big {
        for &q in &big {
            check_dot(&l, "row21/overflow", v(p, q), v(q, p));
            check_dot(&l, "row21/overflow", v(p, p), v(q, q));
        }
    }
    for &p in &tiny {
        for &q in &tiny {
            check_dot(&l, "row21/underflow", v(p, q), v(q, p));
            check_dot(&l, "row21/underflow", v(p, p), v(q, q));
        }
    }
    // Mixed: a huge product cancelled by a tiny one, and vice versa.
    for &p in &big {
        for &q in &tiny {
            check_dot(&l, "row21/mixed", v(p, q), v(p, q));
            check_dot(&l, "row21/mixed", v(p, q), v(q, p));
        }
    }
}

#[test]
fn row22_dot_exact_cancellation_sign_of_zero() {
    let l = libs();
    // a.x*b.x == -(a.y*b.y) exactly ⇒ the sum is a zero; its sign matters.
    let cases: &[(c2v, c2v)] = &[
        (v(1.0, 1.0), v(3.0, -3.0)),
        (v(1.0, -1.0), v(3.0, 3.0)),
        (v(-1.0, 1.0), v(3.0, 3.0)),
        (v(2.0, 4.0), v(8.0, -4.0)),
        (v(0.0, 0.0), v(0.0, 0.0)),
        (v(-0.0, 0.0), v(0.0, 0.0)),
        (v(0.0, -0.0), v(-0.0, 0.0)),
        (v(-0.0, -0.0), v(-0.0, -0.0)),
        (v(1.0, -1.0), v(-0.0, -0.0)),
        (v(f32::MAX, f32::MAX), v(1.0, -1.0)),
        (v(1e-40, 1e-40), v(1.0, -1.0)),
    ];
    for &(a, b) in cases {
        check_dot(&l, "row22", a, b);
    }
    // Randomised exact cancellation.
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..20_000 {
        let p = rng.signed(1000.0);
        let q = rng.signed(1000.0);
        check_dot(&l, "row22/rand", v(p, p), v(q, -q));
        check_dot(&l, "row22/rand", v(p, -p), v(q, q));
    }
}

#[test]
fn row23_dot_infinities_and_invalid_ops() {
    let l = libs();
    let vals = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MAX,
    ];
    for &ax in &vals {
        for &ay in &vals {
            for &bx in &vals {
                for &by in &vals {
                    check_dot(&l, "row23", v(ax, ay), v(bx, by));
                }
            }
        }
    }
}

#[test]
fn row24_dot_nan_all_subsets() {
    let l = libs();
    let base = [1.5f32, -2.5, 3.5, -4.5];
    for mask in 1u32..16 {
        for ni in 0..NAN_F32_BITS.len() {
            let mut c = base;
            for k in 0..4 {
                if mask & (1 << k) != 0 {
                    c[k] = nan(ni + k);
                }
            }
            check_dot(&l, "row24", v(c[0], c[1]), v(c[2], c[3]));
        }
    }
    // Also with zeros/infinities in the non-NaN slots (0*inf interactions).
    for mask in 1u32..16 {
        for &filler in &[0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY] {
            let mut c = [filler; 4];
            for k in 0..4 {
                if mask & (1 << k) != 0 {
                    c[k] = nan(k);
                }
            }
            check_dot(&l, "row24/mixed", v(c[0], c[1]), v(c[2], c[3]));
        }
    }
}

#[test]
fn row25_dot_fully_random_bits() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 25);
    for _ in 0..100_000 {
        check_dot(&l, "row25/raw", rng.raw_v(), rng.raw_v());
    }
    for _ in 0..100_000 {
        check_dot(&l, "row25/nasty", rng.nasty_v(), rng.nasty_v());
    }
}

// ===========================================================================
// c2Sub — rows 26–28
// ===========================================================================

fn check_sub(l: &Pair, row: &str, a: c2v, b: c2v) {
    assert_v_bits_eq!(
        row,
        l.c.c2Sub(a, b),
        l.rs.c2Sub(a, b),
        "c2Sub({}, {})",
        showv(a),
        showv(b)
    );
}

#[test]
fn row26_sub_random_and_signed_zero_results() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..50_000 {
        check_sub(
            &l,
            "row26",
            v(rng.signed(1.0e6), rng.signed(1.0e6)),
            v(rng.signed(1.0e6), rng.signed(1.0e6)),
        );
    }
    // x - x == +0.0; (-0) - (+0) == -0.0; (+0) - (-0) == +0.0
    for _ in 0..5_000 {
        let p = v(rng.signed(1000.0), rng.signed(1000.0));
        check_sub(&l, "row26/self", p, p);
    }
    for &z in &[0.0f32, -0.0f32] {
        for &w in &[0.0f32, -0.0f32] {
            check_sub(&l, "row26/zeros", v(z, w), v(w, z));
        }
    }
}

#[test]
fn row27_sub_infinities_and_overflow() {
    let l = libs();
    let vals = [
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MIN_POSITIVE,
    ];
    for &ax in &vals {
        for &ay in &vals {
            for &bx in &vals {
                for &by in &vals {
                    check_sub(&l, "row27", v(ax, ay), v(bx, by));
                }
            }
        }
    }
}

#[test]
fn row28_sub_nan_all_subsets_and_random() {
    let l = libs();
    let base = [1.0f32, -2.0, 3.0, -4.0];
    for mask in 1u32..16 {
        for ni in 0..NAN_F32_BITS.len() {
            let mut c = base;
            for k in 0..4 {
                if mask & (1 << k) != 0 {
                    c[k] = nan(ni + k);
                }
            }
            check_sub(&l, "row28", v(c[0], c[1]), v(c[2], c[3]));
        }
    }
    let mut rng = Rng::new(SEED ^ 28);
    for _ in 0..100_000 {
        check_sub(&l, "row28/raw", rng.raw_v(), rng.raw_v());
    }
    for _ in 0..100_000 {
        check_sub(&l, "row28/nasty", rng.nasty_v(), rng.nasty_v());
    }
}
