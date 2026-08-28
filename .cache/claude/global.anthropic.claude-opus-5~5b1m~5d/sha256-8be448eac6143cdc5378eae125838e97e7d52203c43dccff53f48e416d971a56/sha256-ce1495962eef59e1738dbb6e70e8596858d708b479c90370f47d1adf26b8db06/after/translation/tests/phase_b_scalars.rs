//! Phase B — CONFIGS.md rows 1..19 and 57: the lowest-level exported scalar /
//! vector helpers, compared bit-for-bit through both `.so`s.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Row 1 — c2V
// ---------------------------------------------------------------------------

#[test]
fn cfg_01_c2v() {
    let p = load();
    let mut d = Diff::new("cfg_01_c2v");
    let mut rng = Rng::new(0x0101);
    for x in specials() {
        for y in specials() {
            let c = unsafe { (p.c.c2V)(x, y) };
            let r = unsafe { (p.r.c2V)(x, y) };
            d.eq_v(|| format!("c2V({}, {})", fs(x), fs(y)), c, r);
        }
    }
    for _ in 0..20_000 {
        let x = rng.any_bits();
        let y = rng.any_bits();
        let c = unsafe { (p.c.c2V)(x, y) };
        let r = unsafe { (p.r.c2V)(x, y) };
        d.eq_v(|| format!("c2V({}, {})", fs(x), fs(y)), c, r);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 2, 3 — c2Dot
// ---------------------------------------------------------------------------

#[test]
fn cfg_02_c2dot_random() {
    let p = load();
    let mut d = Diff::new("cfg_02_c2dot_random");
    let mut rng = Rng::new(0x0202);
    for scale in [1e-30f32, 1e-3, 1.0, 1e3, 1e18, 3e38] {
        for _ in 0..20_000 {
            let a = rng.vec_uniform(scale);
            let b = rng.vec_uniform(scale);
            let c = unsafe { (p.c.c2Dot)(a, b) };
            let r = unsafe { (p.r.c2Dot)(a, b) };
            d.eq_f32(|| format!("c2Dot({}, {})", vs(a), vs(b)), c, r);
        }
    }
    d.finish();
}

#[test]
fn cfg_03_c2dot_specials() {
    let p = load();
    let mut d = Diff::new("cfg_03_c2dot_specials");
    let sp = specials();
    // Full cross product on the x lane with a fixed y lane, and vice versa,
    // plus a full 4-way sweep over a reduced pool (keeps it under ~200k calls).
    for &ax in &sp {
        for &bx in &sp {
            for &ay in &sp {
                for &by in &sp {
                    let a = v(ax, ay);
                    let b = v(bx, by);
                    let c = unsafe { (p.c.c2Dot)(a, b) };
                    let r = unsafe { (p.r.c2Dot)(a, b) };
                    d.eq_f32(|| format!("c2Dot({}, {})", vs(a), vs(b)), c, r);
                }
            }
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 4, 5 — c2Len
// ---------------------------------------------------------------------------

#[test]
fn cfg_04_05_c2len() {
    let p = load();
    let mut d = Diff::new("cfg_04_05_c2len");
    let mut rng = Rng::new(0x0405);
    for &ax in &specials() {
        for &ay in &specials() {
            let a = v(ax, ay);
            let c = unsafe { (p.c.c2Len)(a) };
            let r = unsafe { (p.r.c2Len)(a) };
            d.eq_f32(|| format!("c2Len({})", vs(a)), c, r);
        }
    }
    for scale in [1e-38f32, 1e-6, 1.0, 1e6, 3e38] {
        for _ in 0..20_000 {
            let a = rng.vec_uniform(scale);
            let c = unsafe { (p.c.c2Len)(a) };
            let r = unsafe { (p.r.c2Len)(a) };
            d.eq_f32(|| format!("c2Len({})", vs(a)), c, r);
        }
    }
    for _ in 0..50_000 {
        let a = rng.vec_bits();
        let c = unsafe { (p.c.c2Len)(a) };
        let r = unsafe { (p.r.c2Len)(a) };
        d.eq_f32(|| format!("c2Len({})", vs(a)), c, r);
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 6, 7 — c2Add / c2Sub
// ---------------------------------------------------------------------------

#[test]
fn cfg_06_07_add_sub() {
    let p = load();
    let mut d = Diff::new("cfg_06_07_add_sub");
    let mut rng = Rng::new(0x0607);
    let sp = specials();
    for &ax in &sp {
        for &bx in &sp {
            for &ay in &sp {
                for &by in &sp {
                    let a = v(ax, ay);
                    let b = v(bx, by);
                    let ca = unsafe { (p.c.c2Add)(a, b) };
                    let ra = unsafe { (p.r.c2Add)(a, b) };
                    d.eq_v(|| format!("c2Add({}, {})", vs(a), vs(b)), ca, ra);
                    let cs = unsafe { (p.c.c2Sub)(a, b) };
                    let rs = unsafe { (p.r.c2Sub)(a, b) };
                    d.eq_v(|| format!("c2Sub({}, {})", vs(a), vs(b)), cs, rs);
                }
            }
        }
    }
    for scale in [1e-30f32, 1.0, 1e20, 3e38] {
        for _ in 0..20_000 {
            let a = rng.vec_uniform(scale);
            let b = rng.vec_uniform(scale);
            d.eq_v(
                || format!("c2Add({}, {})", vs(a), vs(b)),
                unsafe { (p.c.c2Add)(a, b) },
                unsafe { (p.r.c2Add)(a, b) },
            );
            d.eq_v(
                || format!("c2Sub({}, {})", vs(a), vs(b)),
                unsafe { (p.c.c2Sub)(a, b) },
                unsafe { (p.r.c2Sub)(a, b) },
            );
        }
    }
    for _ in 0..50_000 {
        let a = rng.vec_bits();
        let b = rng.vec_bits();
        d.eq_v(
            || format!("c2Add({}, {})", vs(a), vs(b)),
            unsafe { (p.c.c2Add)(a, b) },
            unsafe { (p.r.c2Add)(a, b) },
        );
        d.eq_v(
            || format!("c2Sub({}, {})", vs(a), vs(b)),
            unsafe { (p.c.c2Sub)(a, b) },
            unsafe { (p.r.c2Sub)(a, b) },
        );
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 8, 9, 10, 11 — c2Mulvs / c2Div
// ---------------------------------------------------------------------------

#[test]
fn cfg_08_09_mulvs() {
    let p = load();
    let mut d = Diff::new("cfg_08_09_mulvs");
    let mut rng = Rng::new(0x0809);
    let sp = specials();
    for &ax in &sp {
        for &ay in &sp {
            for &b in &sp {
                let a = v(ax, ay);
                d.eq_v(
                    || format!("c2Mulvs({}, {})", vs(a), fs(b)),
                    unsafe { (p.c.c2Mulvs)(a, b) },
                    unsafe { (p.r.c2Mulvs)(a, b) },
                );
            }
        }
    }
    for _ in 0..100_000 {
        let a = rng.vec_bits();
        let b = rng.any_bits();
        d.eq_v(
            || format!("c2Mulvs({}, {})", vs(a), fs(b)),
            unsafe { (p.c.c2Mulvs)(a, b) },
            unsafe { (p.r.c2Mulvs)(a, b) },
        );
    }
    for scale in [1e-20f32, 1.0, 1e20] {
        for _ in 0..20_000 {
            let a = rng.vec_uniform(scale);
            let b = rng.uniform(scale);
            d.eq_v(
                || format!("c2Mulvs({}, {})", vs(a), fs(b)),
                unsafe { (p.c.c2Mulvs)(a, b) },
                unsafe { (p.r.c2Mulvs)(a, b) },
            );
        }
    }
    d.finish();
}

#[test]
fn cfg_10_11_div() {
    let p = load();
    let mut d = Diff::new("cfg_10_11_div");
    let mut rng = Rng::new(0x1011);
    let sp = specials();
    for &ax in &sp {
        for &ay in &sp {
            for &b in &sp {
                let a = v(ax, ay);
                d.eq_v(
                    || format!("c2Div({}, {})", vs(a), fs(b)),
                    unsafe { (p.c.c2Div)(a, b) },
                    unsafe { (p.r.c2Div)(a, b) },
                );
            }
        }
    }
    for _ in 0..100_000 {
        let a = rng.vec_bits();
        let b = rng.any_bits();
        d.eq_v(
            || format!("c2Div({}, {})", vs(a), fs(b)),
            unsafe { (p.c.c2Div)(a, b) },
            unsafe { (p.r.c2Div)(a, b) },
        );
    }
    // Emphasise the reciprocal-multiply quirk with values where `a/b` and
    // `a * (1/b)` differ in the last bit.
    for _ in 0..50_000 {
        let a = rng.vec_uniform(1e3);
        let b = rng.uniform(1e3);
        d.eq_v(
            || format!("c2Div({}, {})", vs(a), fs(b)),
            unsafe { (p.c.c2Div)(a, b) },
            unsafe { (p.r.c2Div)(a, b) },
        );
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 12, 13 — c2Norm
// ---------------------------------------------------------------------------

#[test]
fn cfg_12_13_norm() {
    let p = load();
    let mut d = Diff::new("cfg_12_13_norm");
    let mut rng = Rng::new(0x1213);
    for &ax in &specials() {
        for &ay in &specials() {
            let a = v(ax, ay);
            d.eq_v(
                || format!("c2Norm({})", vs(a)),
                unsafe { (p.c.c2Norm)(a) },
                unsafe { (p.r.c2Norm)(a) },
            );
        }
    }
    for scale in [1e-38f32, 1e-10, 1.0, 1e10, 3e38] {
        for _ in 0..20_000 {
            let a = rng.vec_uniform(scale);
            d.eq_v(
                || format!("c2Norm({})", vs(a)),
                unsafe { (p.c.c2Norm)(a) },
                unsafe { (p.r.c2Norm)(a) },
            );
        }
    }
    for _ in 0..100_000 {
        let a = rng.vec_bits();
        d.eq_v(
            || format!("c2Norm({})", vs(a)),
            unsafe { (p.c.c2Norm)(a) },
            unsafe { (p.r.c2Norm)(a) },
        );
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 14, 15 — c2Minv / c2Maxv (ternary, NOT fminf/fmaxf)
// ---------------------------------------------------------------------------

#[test]
fn cfg_14_15_minv_maxv() {
    let p = load();
    let mut d = Diff::new("cfg_14_15_minv_maxv");
    let mut rng = Rng::new(0x1415);
    let sp = specials();
    for &ax in &sp {
        for &bx in &sp {
            for &ay in &sp {
                for &by in &sp {
                    let a = v(ax, ay);
                    let b = v(bx, by);
                    d.eq_v(
                        || format!("c2Minv({}, {})", vs(a), vs(b)),
                        unsafe { (p.c.c2Minv)(a, b) },
                        unsafe { (p.r.c2Minv)(a, b) },
                    );
                    d.eq_v(
                        || format!("c2Maxv({}, {})", vs(a), vs(b)),
                        unsafe { (p.c.c2Maxv)(a, b) },
                        unsafe { (p.r.c2Maxv)(a, b) },
                    );
                }
            }
        }
    }
    for _ in 0..50_000 {
        let a = rng.vec_bits();
        let b = rng.vec_bits();
        d.eq_v(
            || format!("c2Minv({}, {})", vs(a), vs(b)),
            unsafe { (p.c.c2Minv)(a, b) },
            unsafe { (p.r.c2Minv)(a, b) },
        );
        d.eq_v(
            || format!("c2Maxv({}, {})", vs(a), vs(b)),
            unsafe { (p.c.c2Maxv)(a, b) },
            unsafe { (p.r.c2Maxv)(a, b) },
        );
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 16, 17 — c2Skew / c2CCW90 / c2Absv
// ---------------------------------------------------------------------------

#[test]
fn cfg_16_17_skew_ccw90_absv() {
    let p = load();
    let mut d = Diff::new("cfg_16_17_skew_ccw90_absv");
    let mut rng = Rng::new(0x1617);
    for &ax in &specials() {
        for &ay in &specials() {
            let a = v(ax, ay);
            d.eq_v(
                || format!("c2Skew({})", vs(a)),
                unsafe { (p.c.c2Skew)(a) },
                unsafe { (p.r.c2Skew)(a) },
            );
            d.eq_v(
                || format!("c2CCW90({})", vs(a)),
                unsafe { (p.c.c2CCW90)(a) },
                unsafe { (p.r.c2CCW90)(a) },
            );
            d.eq_v(
                || format!("c2Absv({})", vs(a)),
                unsafe { (p.c.c2Absv)(a) },
                unsafe { (p.r.c2Absv)(a) },
            );
        }
    }
    for _ in 0..100_000 {
        let a = rng.vec_bits();
        d.eq_v(
            || format!("c2Skew({})", vs(a)),
            unsafe { (p.c.c2Skew)(a) },
            unsafe { (p.r.c2Skew)(a) },
        );
        d.eq_v(
            || format!("c2CCW90({})", vs(a)),
            unsafe { (p.c.c2CCW90)(a) },
            unsafe { (p.r.c2CCW90)(a) },
        );
        d.eq_v(
            || format!("c2Absv({})", vs(a)),
            unsafe { (p.c.c2Absv)(a) },
            unsafe { (p.r.c2Absv)(a) },
        );
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 18, 19 — c2MulmvT
// ---------------------------------------------------------------------------

#[test]
fn cfg_18_19_mulmvt() {
    let p = load();
    let mut d = Diff::new("cfg_18_19_mulmvt");
    let mut rng = Rng::new(0x1819);
    let sp = specials();
    // Sweep each of the 6 slots against the special pool while the other five
    // stay at a fixed, distinguishable value (full 6-way cross product would be
    // 64 million calls).
    let base = [1.5f32, -2.25, 0.75, -3.5, 4.125, -0.625];
    for slot in 0..6usize {
        for &s in &sp {
            for &s2 in &sp {
                let mut f = base;
                f[slot] = s;
                f[(slot + 1) % 6] = s2;
                let m = c2m {
                    x: v(f[0], f[1]),
                    y: v(f[2], f[3]),
                };
                let b = v(f[4], f[5]);
                d.eq_v(
                    || format!("c2MulmvT({:?}, {})", m, vs(b)),
                    unsafe { (p.c.c2MulmvT)(m, b) },
                    unsafe { (p.r.c2MulmvT)(m, b) },
                );
            }
        }
    }
    for _ in 0..100_000 {
        let m = c2m {
            x: rng.vec_bits(),
            y: rng.vec_bits(),
        };
        let b = rng.vec_bits();
        d.eq_v(
            || format!("c2MulmvT({:?}, {})", m, vs(b)),
            unsafe { (p.c.c2MulmvT)(m, b) },
            unsafe { (p.r.c2MulmvT)(m, b) },
        );
    }
    for scale in [1e-20f32, 1.0, 1e20] {
        for _ in 0..20_000 {
            let m = c2m {
                x: rng.vec_uniform(scale),
                y: rng.vec_uniform(scale),
            };
            let b = rng.vec_uniform(scale);
            d.eq_v(
                || format!("c2MulmvT({:?}, {})", m, vs(b)),
                unsafe { (p.c.c2MulmvT)(m, b) },
                unsafe { (p.r.c2MulmvT)(m, b) },
            );
        }
    }
    d.finish();
}

// ---------------------------------------------------------------------------
// Row 57 — mixed bit-pattern fuzz over every scalar helper at once
// ---------------------------------------------------------------------------

#[test]
fn cfg_57_scalar_bitpattern_fuzz() {
    let p = load();
    let mut d = Diff::new("cfg_57_scalar_bitpattern_fuzz");
    let mut rng = Rng::new(0x5757);
    for _ in 0..200_000 {
        let a = rng.vec_bits();
        let b = rng.vec_bits();
        let s = rng.any_bits();
        d.eq_f32(
            || format!("c2Dot({}, {})", vs(a), vs(b)),
            unsafe { (p.c.c2Dot)(a, b) },
            unsafe { (p.r.c2Dot)(a, b) },
        );
        d.eq_f32(
            || format!("c2Len({})", vs(a)),
            unsafe { (p.c.c2Len)(a) },
            unsafe { (p.r.c2Len)(a) },
        );
        d.eq_v(
            || format!("c2Add({}, {})", vs(a), vs(b)),
            unsafe { (p.c.c2Add)(a, b) },
            unsafe { (p.r.c2Add)(a, b) },
        );
        d.eq_v(
            || format!("c2Sub({}, {})", vs(a), vs(b)),
            unsafe { (p.c.c2Sub)(a, b) },
            unsafe { (p.r.c2Sub)(a, b) },
        );
        d.eq_v(
            || format!("c2Mulvs({}, {})", vs(a), fs(s)),
            unsafe { (p.c.c2Mulvs)(a, s) },
            unsafe { (p.r.c2Mulvs)(a, s) },
        );
        d.eq_v(
            || format!("c2Div({}, {})", vs(a), fs(s)),
            unsafe { (p.c.c2Div)(a, s) },
            unsafe { (p.r.c2Div)(a, s) },
        );
        d.eq_v(
            || format!("c2Norm({})", vs(a)),
            unsafe { (p.c.c2Norm)(a) },
            unsafe { (p.r.c2Norm)(a) },
        );
        d.eq_v(
            || format!("c2Minv({}, {})", vs(a), vs(b)),
            unsafe { (p.c.c2Minv)(a, b) },
            unsafe { (p.r.c2Minv)(a, b) },
        );
        d.eq_v(
            || format!("c2Maxv({}, {})", vs(a), vs(b)),
            unsafe { (p.c.c2Maxv)(a, b) },
            unsafe { (p.r.c2Maxv)(a, b) },
        );
        let m = c2m { x: a, y: b };
        d.eq_v(
            || format!("c2MulmvT({:?}, {})", m, vs(a)),
            unsafe { (p.c.c2MulmvT)(m, a) },
            unsafe { (p.r.c2MulmvT)(m, a) },
        );
    }
    d.finish();
}
