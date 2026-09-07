//! Phase B — valid-path differential tests for the LOW-LEVEL exports.
//!
//! Covers `CONFIGS.md` rows 1-10 and 35. Every call goes through both `.so`s;
//! all float results are compared by raw bits.

mod common;

use common::*;

const N: usize = 20_000;

/// CONFIGS.md row 1 — `c2V`: random finite pairs, then every edge scalar and
/// arbitrary bit patterns (inf / NaN / denormal / signed zero) round-tripped.
#[test]
fn row01_c2v() {
    let p = load();
    let mut rng = Rng::default_seeded();

    for _ in 0..N {
        let (x, y) = (rng.coord(), rng.coord());
        let (c, rs) = unsafe { ((p.c.c2V)(x, y), (p.rs.c2V)(x, y)) };
        assert_v_bits_eq(&format!("c2V({x:?},{y:?})"), c, rs);
    }

    for &x in EDGE_F32 {
        for &y in EDGE_F32 {
            let (c, rs) = unsafe { ((p.c.c2V)(x, y), (p.rs.c2V)(x, y)) };
            assert_v_bits_eq(&format!("c2V edge({x:?},{y:?})"), c, rs);
        }
    }

    for _ in 0..N {
        let (x, y) = (rng.any_bits_f32(), rng.any_bits_f32());
        let (c, rs) = unsafe { ((p.c.c2V)(x, y), (p.rs.c2V)(x, y)) };
        assert_v_bits_eq("c2V random-bits", c, rs);
    }
}

/// CONFIGS.md row 2 — `c2Sub`: random pairs, equal operands (exact zero),
/// `inf - inf`, denormal cancellation, all edge-scalar combinations.
#[test]
fn row02_c2sub() {
    let p = load();
    let mut rng = Rng::default_seeded();

    for _ in 0..N {
        let (a, b) = (rng.vec_coord(), rng.vec_coord());
        let (c, rs) = unsafe { ((p.c.c2Sub)(a, b), (p.rs.c2Sub)(a, b)) };
        assert_v_bits_eq("c2Sub random", c, rs);
    }

    // Equal operands -> exact zero (and the sign of that zero matters).
    for _ in 0..N / 4 {
        let a = rng.vec_any();
        let (c, rs) = unsafe { ((p.c.c2Sub)(a, a), (p.rs.c2Sub)(a, a)) };
        assert_v_bits_eq("c2Sub self", c, rs);
    }

    for &ax in EDGE_F32 {
        for &bx in EDGE_F32 {
            let a = v(ax, bx);
            let b = v(bx, ax);
            let (c, rs) = unsafe { ((p.c.c2Sub)(a, b), (p.rs.c2Sub)(a, b)) };
            assert_v_bits_eq(&format!("c2Sub edge({ax:?},{bx:?})"), c, rs);
        }
    }

    for _ in 0..N {
        let (a, b) = (rng.vec_any(), rng.vec_any());
        let (c, rs) = unsafe { ((p.c.c2Sub)(a, b), (p.rs.c2Sub)(a, b)) };
        assert_v_bits_eq("c2Sub random-bits", c, rs);
    }
}

/// CONFIGS.md row 3 — `c2Dot`: random, orthogonal (exact cancellation to zero),
/// antiparallel, magnitudes spanning 1e-30..1e30 (over/underflow of the sum).
#[test]
fn row03_c2dot() {
    let p = load();
    let mut rng = Rng::default_seeded();

    for _ in 0..N {
        let (a, b) = (rng.vec_coord(), rng.vec_coord());
        let (c, rs) = unsafe { ((p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b)) };
        assert_f32_bits_eq("c2Dot random", c, rs);
    }

    // Orthogonal: x*y + y*(-x) cancels exactly.
    for _ in 0..N / 4 {
        let a = rng.vec_coord();
        let b = v(-a.y, a.x);
        let (c, rs) = unsafe { ((p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b)) };
        assert_f32_bits_eq("c2Dot orthogonal", c, rs);
        // Antiparallel.
        let b2 = v(-a.x, -a.y);
        let (c, rs) = unsafe { ((p.c.c2Dot)(a, b2), (p.rs.c2Dot)(a, b2)) };
        assert_f32_bits_eq("c2Dot antiparallel", c, rs);
    }

    // Extreme magnitudes -> products overflow / underflow.
    for e in [-30i32, -20, -10, -1, 0, 1, 10, 20, 30, 38] {
        let m = 10f32.powi(e);
        for _ in 0..500 {
            let a = v(rng.range(-1.0, 1.0) * m, rng.range(-1.0, 1.0) * m);
            let b = v(rng.range(-1.0, 1.0) * m, rng.range(-1.0, 1.0) * m);
            let (c, rs) = unsafe { ((p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b)) };
            assert_f32_bits_eq(&format!("c2Dot scale 1e{e}"), c, rs);
        }
    }

    for &ax in EDGE_F32 {
        for &ay in EDGE_F32 {
            let a = v(ax, ay);
            let b = v(ay, ax);
            let (c, rs) = unsafe { ((p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b)) };
            assert_f32_bits_eq(&format!("c2Dot edge({ax:?},{ay:?})"), c, rs);
        }
    }

    for _ in 0..N {
        let (a, b) = (rng.vec_any(), rng.vec_any());
        let (c, rs) = unsafe { ((p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b)) };
        assert_f32_bits_eq("c2Dot random-bits", c, rs);
    }
}

/// CONFIGS.md row 4 — `c2Mulvs`: random, `b` = 0 / -0 / inf / NaN / denormal,
/// and overflow to inf. The C's operand order (`a.x * b`) is observable for
/// NaN x NaN, so random bit patterns are included on both sides.
#[test]
fn row04_c2mulvs() {
    let p = load();
    let mut rng = Rng::default_seeded();

    for _ in 0..N {
        let a = rng.vec_coord();
        let b = rng.coord();
        let (c, rs) = unsafe { ((p.c.c2Mulvs)(a, b), (p.rs.c2Mulvs)(a, b)) };
        assert_v_bits_eq("c2Mulvs random", c, rs);
    }

    for &b in EDGE_F32 {
        for &ax in EDGE_F32 {
            for &ay in EDGE_F32 {
                let a = v(ax, ay);
                let (c, rs) = unsafe { ((p.c.c2Mulvs)(a, b), (p.rs.c2Mulvs)(a, b)) };
                assert_v_bits_eq(&format!("c2Mulvs edge(({ax:?},{ay:?}) * {b:?})"), c, rs);
            }
        }
    }

    for _ in 0..N {
        let a = rng.vec_any();
        let b = rng.any_bits_f32();
        let (c, rs) = unsafe { ((p.c.c2Mulvs)(a, b), (p.rs.c2Mulvs)(a, b)) };
        assert_v_bits_eq("c2Mulvs random-bits", c, rs);
    }
}

/// CONFIGS.md rows 5-8 — `c2Maxv` / `c2Minv`: all four per-component ternary
/// outcomes, equality, signed zeros, and NaN in each of the four slots. The C
/// uses a bare ternary, so a false comparison yields the SECOND operand.
#[test]
fn row05_08_maxv_minv() {
    let p = load();
    let mut rng = Rng::default_seeded();

    // All four taken/not-taken combinations, driven explicitly.
    let combos: [(f32, f32, f32, f32); 4] = [
        (2.0, 1.0, 4.0, 3.0), // a.x > b.x, a.y > b.y
        (2.0, 1.0, 3.0, 4.0), // a.x > b.x, a.y < b.y
        (1.0, 2.0, 4.0, 3.0), // a.x < b.x, a.y > b.y
        (1.0, 2.0, 3.0, 4.0), // a.x < b.x, a.y < b.y
    ];
    for (ax, bx, ay, by) in combos {
        let a = v(ax, ay);
        let b = v(bx, by);
        let (c, rs) = unsafe { ((p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b)) };
        assert_v_bits_eq("c2Maxv combo", c, rs);
        let (c, rs) = unsafe { ((p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b)) };
        assert_v_bits_eq("c2Minv combo", c, rs);
    }

    // Equality and signed zeros: `>` / `<` are false, so `b` wins.
    let zeros = [0.0f32, -0.0f32, 1.0, -1.0, f32::NAN, -f32::NAN];
    for &ax in &zeros {
        for &ay in &zeros {
            for &bx in &zeros {
                for &by in &zeros {
                    let a = v(ax, ay);
                    let b = v(bx, by);
                    let (c, rs) = unsafe { ((p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b)) };
                    assert_v_bits_eq(
                        &format!("c2Maxv zero/nan(({ax:?},{ay:?}),({bx:?},{by:?}))"),
                        c,
                        rs,
                    );
                    let (c, rs) = unsafe { ((p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b)) };
                    assert_v_bits_eq(
                        &format!("c2Minv zero/nan(({ax:?},{ay:?}),({bx:?},{by:?}))"),
                        c,
                        rs,
                    );
                }
            }
        }
    }

    for &ax in EDGE_F32 {
        for &bx in EDGE_F32 {
            let a = v(ax, bx);
            let b = v(bx, ax);
            let (c, rs) = unsafe { ((p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b)) };
            assert_v_bits_eq("c2Maxv edge", c, rs);
            let (c, rs) = unsafe { ((p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b)) };
            assert_v_bits_eq("c2Minv edge", c, rs);
        }
    }

    for _ in 0..N {
        let (a, b) = (rng.vec_coord(), rng.vec_coord());
        let (c, rs) = unsafe { ((p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b)) };
        assert_v_bits_eq("c2Maxv random", c, rs);
        let (c, rs) = unsafe { ((p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b)) };
        assert_v_bits_eq("c2Minv random", c, rs);

        // Same-value operands (force the equality path frequently).
        let (c, rs) = unsafe { ((p.c.c2Maxv)(a, a), (p.rs.c2Maxv)(a, a)) };
        assert_v_bits_eq("c2Maxv equal", c, rs);
        let (c, rs) = unsafe { ((p.c.c2Minv)(a, a), (p.rs.c2Minv)(a, a)) };
        assert_v_bits_eq("c2Minv equal", c, rs);
    }

    for _ in 0..N {
        let (a, b) = (rng.vec_any(), rng.vec_any());
        let (c, rs) = unsafe { ((p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b)) };
        assert_v_bits_eq("c2Maxv random-bits", c, rs);
        let (c, rs) = unsafe { ((p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b)) };
        assert_v_bits_eq("c2Minv random-bits", c, rs);
    }
}

/// CONFIGS.md rows 9-10 — `c2Clampv`: `a` in all 9 regions relative to the
/// `lo..hi` box, degenerate box (`lo == hi`), inverted box (`lo > hi`), and
/// NaN in `a` / `lo` / `hi`.
#[test]
fn row09_10_clampv() {
    let p = load();
    let mut rng = Rng::default_seeded();

    // All 9 regions: below / inside / above on each axis.
    let lo = v(-10.0, -5.0);
    let hi = v(10.0, 5.0);
    let xs = [-100.0f32, -10.0, 0.0, 10.0, 100.0];
    let ys = [-50.0f32, -5.0, 0.0, 5.0, 50.0];
    for &x in &xs {
        for &y in &ys {
            let a = v(x, y);
            let (c, rs) = unsafe { ((p.c.c2Clampv)(a, lo, hi), (p.rs.c2Clampv)(a, lo, hi)) };
            assert_v_bits_eq(&format!("c2Clampv region({x},{y})"), c, rs);
        }
    }

    // Random boxes, `a` deliberately placed inside / outside.
    for _ in 0..N {
        let b = rng.aabb_sorted();
        let a = rng.vec_coord();
        let (c, rs) = unsafe { ((p.c.c2Clampv)(a, b.min, b.max), (p.rs.c2Clampv)(a, b.min, b.max)) };
        assert_v_bits_eq("c2Clampv random sorted", c, rs);

        // Degenerate box.
        let (c, rs) = unsafe {
            (
                (p.c.c2Clampv)(a, b.min, b.min),
                (p.rs.c2Clampv)(a, b.min, b.min),
            )
        };
        assert_v_bits_eq("c2Clampv degenerate", c, rs);

        // Inverted box (lo > hi) — accepted, unvalidated, by the C.
        let (c, rs) = unsafe { ((p.c.c2Clampv)(a, b.max, b.min), (p.rs.c2Clampv)(a, b.max, b.min)) };
        assert_v_bits_eq("c2Clampv inverted", c, rs);
    }

    // NaN in each slot.
    let nan = f32::NAN;
    let cands = [v(nan, 1.0), v(1.0, nan), v(nan, nan), v(0.0, 0.0)];
    for &a in &cands {
        for &l in &cands {
            for &h in &cands {
                let (c, rs) = unsafe { ((p.c.c2Clampv)(a, l, h), (p.rs.c2Clampv)(a, l, h)) };
                assert_v_bits_eq("c2Clampv nan", c, rs);
            }
        }
    }

    for _ in 0..N {
        let (a, l, h) = (rng.vec_any(), rng.vec_any(), rng.vec_any());
        let (c, rs) = unsafe { ((p.c.c2Clampv)(a, l, h), (p.rs.c2Clampv)(a, l, h)) };
        assert_v_bits_eq("c2Clampv random-bits", c, rs);
    }
}

/// CONFIGS.md row 35 — the bit-exactness contract itself: every `c2v`/`float`
/// returning export must agree on raw bits (not `==`) so signed zeros and NaN
/// payloads cannot slip through. Uses fully random bit patterns everywhere.
#[test]
fn row35_bit_exact_returns() {
    let p = load();
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D);

    for _ in 0..N {
        let a = rng.vec_any();
        let b = rng.vec_any();
        let s = rng.any_bits_f32();

        assert_v_bits_eq("bitexact c2V", unsafe { (p.c.c2V)(a.x, a.y) }, unsafe {
            (p.rs.c2V)(a.x, a.y)
        });
        assert_v_bits_eq("bitexact c2Sub", unsafe { (p.c.c2Sub)(a, b) }, unsafe {
            (p.rs.c2Sub)(a, b)
        });
        assert_v_bits_eq("bitexact c2Mulvs", unsafe { (p.c.c2Mulvs)(a, s) }, unsafe {
            (p.rs.c2Mulvs)(a, s)
        });
        assert_v_bits_eq("bitexact c2Maxv", unsafe { (p.c.c2Maxv)(a, b) }, unsafe {
            (p.rs.c2Maxv)(a, b)
        });
        assert_v_bits_eq("bitexact c2Minv", unsafe { (p.c.c2Minv)(a, b) }, unsafe {
            (p.rs.c2Minv)(a, b)
        });
        assert_f32_bits_eq("bitexact c2Dot", unsafe { (p.c.c2Dot)(a, b) }, unsafe {
            (p.rs.c2Dot)(a, b)
        });
    }
}
