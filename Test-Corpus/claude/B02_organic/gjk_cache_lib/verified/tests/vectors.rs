//! Phase B — CONFIGS.md rows 1..19: the lowest-level entry points
//! (vector helpers and proxy construction), driven through both `.so`s.
//!
//! Each row is tested in two tiers (see the policy comment in `common/mod.rs`):
//!   * STRICT — NaN-free inputs (incl. `±0`, `±inf`, subnormals, `±FLT_MAX`);
//!     every output bit must match, including hardware-generated NaNs.
//!   * SOFT   — inputs that already contain a NaN; NaN == NaN regardless of
//!     payload, everything else still bit-exact.
//! Functions that only copy or select (never do arithmetic on two NaNs) are
//! held to STRICT even on NaN input, which is the stronger assertion.

mod common;
use common::*;

const N: usize = 4096;

// ---------------------------------------------------------------------------
// Row 1: c2V — pure field copy, STRICT even for NaN / signalling NaN.
// ---------------------------------------------------------------------------

#[test]
fn row01_c2v() {
    let p = pair();
    let mut rng = Rng::new(0x0101);
    unsafe {
        for i in 0..N {
            let (x, y) = (rng.nasty(), rng.nasty());
            eq_v(&format!("row01[{i}] c2V({x:?},{y:?})"), (p.c.c2V)(x, y), (p.r.c2V)(x, y));
        }
        // Raw bit patterns, incl. signalling NaNs — must be preserved verbatim.
        for i in 0..N {
            let (x, y) = (rng.bits_f32(), rng.bits_f32());
            eq_v(&format!("row01-bits[{i}]"), (p.c.c2V)(x, y), (p.r.c2V)(x, y));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 2: c2Mulvs
// ---------------------------------------------------------------------------

#[test]
fn row02_c2mulvs() {
    let p = pair();
    let mut rng = Rng::new(0x0202);
    let scalars_strict =
        [0.0f32, -0.0, 1.0, -1.0, 0.5, f32::INFINITY, f32::NEG_INFINITY, f32::MAX,
         f32::MIN_POSITIVE, f32::from_bits(1)];
    unsafe {
        // STRICT tier
        for (j, &s) in scalars_strict.iter().enumerate() {
            for i in 0..256 {
                let v = rng.vec_nasty_no_nan();
                eq_v(
                    &format!("row02 strict-fixed[{j}][{i}] v={v:?} s={s:?}"),
                    (p.c.c2Mulvs)(v, s),
                    (p.r.c2Mulvs)(v, s),
                );
            }
        }
        for i in 0..N {
            let v = rng.vec_nasty_no_nan();
            let s = rng.nasty_no_nan();
            eq_v(
                &format!("row02 strict-rand[{i}] v={v:?} s={s:?}"),
                (p.c.c2Mulvs)(v, s),
                (p.r.c2Mulvs)(v, s),
            );
        }
        // SOFT tier (NaN inputs)
        for i in 0..N {
            let v = rng.vec_nasty();
            let s = rng.nasty();
            eq_v_soft(
                &format!("row02 soft[{i}] v={v:?} s={s:?}"),
                (p.c.c2Mulvs)(v, s),
                (p.r.c2Mulvs)(v, s),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3: c2Add / c2Sub
// ---------------------------------------------------------------------------

#[test]
fn row03_c2add_c2sub() {
    let p = pair();
    let mut rng = Rng::new(0x0303);
    unsafe {
        for i in 0..N {
            let a = rng.vec_nasty_no_nan();
            let b = rng.vec_nasty_no_nan();
            eq_v(&format!("row03 add[{i}] {a:?} {b:?}"), (p.c.c2Add)(a, b), (p.r.c2Add)(a, b));
            eq_v(&format!("row03 sub[{i}] {a:?} {b:?}"), (p.c.c2Sub)(a, b), (p.r.c2Sub)(a, b));
            // exact cancellation, and inf-inf -> indefinite NaN in both
            eq_v(&format!("row03 sub-self[{i}]"), (p.c.c2Sub)(a, a), (p.r.c2Sub)(a, a));
            eq_v(&format!("row03 add-self[{i}]"), (p.c.c2Add)(a, a), (p.r.c2Add)(a, a));
        }
        // inf - inf and inf + -inf: generated NaN, STRICT
        let inf = c2v { x: f32::INFINITY, y: f32::NEG_INFINITY };
        eq_v("row03 inf-inf", (p.c.c2Sub)(inf, inf), (p.r.c2Sub)(inf, inf));
        eq_v("row03 inf+inf", (p.c.c2Add)(inf, inf), (p.r.c2Add)(inf, inf));
        // SOFT tier
        for i in 0..N {
            let a = rng.vec_nasty();
            let b = rng.vec_nasty();
            eq_v_soft(&format!("row03 soft-add[{i}]"), (p.c.c2Add)(a, b), (p.r.c2Add)(a, b));
            eq_v_soft(&format!("row03 soft-sub[{i}]"), (p.c.c2Sub)(a, b), (p.r.c2Sub)(a, b));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4: c2Dot
// ---------------------------------------------------------------------------

#[test]
fn row04_c2dot() {
    let p = pair();
    let mut rng = Rng::new(0x0404);
    unsafe {
        for i in 0..N {
            let a = rng.vec_nasty_no_nan();
            let b = rng.vec_nasty_no_nan();
            eq_f32(&format!("row04[{i}] {a:?}.{b:?}"), (p.c.c2Dot)(a, b), (p.r.c2Dot)(a, b));
        }
        // orthogonal / antiparallel / overflow-to-inf
        for i in 0..N {
            let a = rng.vec_scaled(1e30);
            let orth = c2v { x: -a.y, y: a.x };
            eq_f32(&format!("row04 orth[{i}]"), (p.c.c2Dot)(a, orth), (p.r.c2Dot)(a, orth));
            let anti = c2v { x: -a.x, y: -a.y };
            eq_f32(&format!("row04 anti[{i}]"), (p.c.c2Dot)(a, anti), (p.r.c2Dot)(a, anti));
            eq_f32(&format!("row04 self[{i}]"), (p.c.c2Dot)(a, a), (p.r.c2Dot)(a, a));
        }
        // inf*0 in one term only: generated NaN, still STRICT
        for (a, b) in [
            (c2v { x: f32::INFINITY, y: 1.0 }, c2v { x: 0.0, y: 2.0 }),
            (c2v { x: f32::NEG_INFINITY, y: 1.0 }, c2v { x: 0.0, y: 2.0 }),
            (c2v { x: 1.0, y: f32::INFINITY }, c2v { x: 2.0, y: 0.0 }),
            (c2v { x: f32::INFINITY, y: f32::INFINITY }, c2v { x: 0.0, y: 0.0 }),
            (c2v { x: f32::INFINITY, y: f32::NEG_INFINITY }, c2v { x: 0.0, y: 0.0 }),
        ] {
            eq_f32(&format!("row04 invalid {a:?} {b:?}"), (p.c.c2Dot)(a, b), (p.r.c2Dot)(a, b));
        }
        // SOFT tier
        for i in 0..N {
            let a = rng.vec_nasty();
            let b = rng.vec_nasty();
            eq_f32_soft(&format!("row04 soft[{i}]"), (p.c.c2Dot)(a, b), (p.r.c2Dot)(a, b));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5: c2Det2
// ---------------------------------------------------------------------------

#[test]
fn row05_c2det2() {
    let p = pair();
    let mut rng = Rng::new(0x0505);
    unsafe {
        for i in 0..N {
            let a = rng.vec_nasty_no_nan();
            let b = rng.vec_nasty_no_nan();
            eq_f32(&format!("row05[{i}]"), (p.c.c2Det2)(a, b), (p.r.c2Det2)(a, b));
        }
        for i in 0..N {
            // collinear -> det == 0 (and the SIGN of that zero matters)
            let a = rng.vec_any_scale();
            let k = rng.scaled(4.0);
            let b = c2v { x: a.x * k, y: a.y * k };
            eq_f32(&format!("row05 collinear[{i}]"), (p.c.c2Det2)(a, b), (p.r.c2Det2)(a, b));
            eq_f32(&format!("row05 self[{i}]"), (p.c.c2Det2)(a, a), (p.r.c2Det2)(a, a));
        }
        // SOFT tier
        for i in 0..N {
            let a = rng.vec_nasty();
            let b = rng.vec_nasty();
            eq_f32_soft(&format!("row05 soft[{i}]"), (p.c.c2Det2)(a, b), (p.r.c2Det2)(a, b));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 6: c2Len
// ---------------------------------------------------------------------------

#[test]
fn row06_c2len() {
    let p = pair();
    let mut rng = Rng::new(0x0606);
    unsafe {
        for i in 0..N {
            let a = rng.vec_nasty_no_nan();
            eq_f32(&format!("row06[{i}] {a:?}"), (p.c.c2Len)(a), (p.r.c2Len)(a));
        }
        for v in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: -0.0 },
            c2v { x: 1e30, y: 1e30 },        // c2Dot overflows to +inf
            c2v { x: f32::MAX, y: f32::MAX },
            c2v { x: f32::from_bits(1), y: 0.0 },
            c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
            c2v { x: 3.0, y: 4.0 },
            c2v { x: -3.0, y: -4.0 },
        ] {
            eq_f32(&format!("row06 fixed {v:?}"), (p.c.c2Len)(v), (p.r.c2Len)(v));
        }
        // SOFT tier
        for i in 0..N {
            let a = rng.vec_nasty();
            eq_f32_soft(&format!("row06 soft[{i}] {a:?}"), (p.c.c2Len)(a), (p.r.c2Len)(a));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 7: c2Maxv / c2Minv — pure compare+select, STRICT even for NaN.
// ---------------------------------------------------------------------------

#[test]
fn row07_c2maxv_c2minv() {
    let p = pair();
    let mut rng = Rng::new(0x0707);
    unsafe {
        for i in 0..N {
            let a = rng.vec_nasty();
            let b = rng.vec_nasty();
            eq_v(&format!("row07 max[{i}] {a:?} {b:?}"), (p.c.c2Maxv)(a, b), (p.r.c2Maxv)(a, b));
            eq_v(&format!("row07 min[{i}] {a:?} {b:?}"), (p.c.c2Minv)(a, b), (p.r.c2Minv)(a, b));
            // equal components (tie -> takes b in both)
            eq_v(&format!("row07 max-eq[{i}]"), (p.c.c2Maxv)(a, a), (p.r.c2Maxv)(a, a));
            eq_v(&format!("row07 min-eq[{i}]"), (p.c.c2Minv)(a, a), (p.r.c2Minv)(a, a));
        }
        // Explicit NaN-position matrix: NaN in a only, b only, both.
        let nan = c2v { x: f32::NAN, y: f32::NAN };
        let nan2 = c2v { x: f32::from_bits(0xffc0_0000), y: f32::from_bits(0x7fc0_0001) };
        let ord = c2v { x: 1.0, y: -1.0 };
        let zpos = c2v { x: 0.0, y: 0.0 };
        let zneg = c2v { x: -0.0, y: -0.0 };
        for (n, (a, b)) in [
            (nan, ord),
            (ord, nan),
            (nan, nan),
            (nan, nan2),
            (nan2, nan),
            (zpos, zneg),
            (zneg, zpos),
        ]
        .iter()
        .enumerate()
        {
            eq_v(&format!("row07 special-max[{n}]"), (p.c.c2Maxv)(*a, *b), (p.r.c2Maxv)(*a, *b));
            eq_v(&format!("row07 special-min[{n}]"), (p.c.c2Minv)(*a, *b), (p.r.c2Minv)(*a, *b));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 8: c2Clampv, incl. the inverted lo>hi range — STRICT even for NaN.
// ---------------------------------------------------------------------------

#[test]
fn row08_c2clampv() {
    let p = pair();
    let mut rng = Rng::new(0x0808);
    unsafe {
        for i in 0..N {
            let a = rng.vec_nasty();
            let (l, h) = (rng.vec_scaled(10.0), rng.vec_scaled(10.0));
            let lo = c2v { x: l.x.min(h.x), y: l.y.min(h.y) };
            let hi = c2v { x: l.x.max(h.x), y: l.y.max(h.y) };
            // lo < hi
            eq_v(
                &format!("row08 ordered[{i}]"),
                (p.c.c2Clampv)(a, lo, hi),
                (p.r.c2Clampv)(a, lo, hi),
            );
            // lo == hi
            eq_v(&format!("row08 equal[{i}]"), (p.c.c2Clampv)(a, lo, lo), (p.r.c2Clampv)(a, lo, lo));
            // INVERTED: lo > hi (ERRORS.md row 23)
            eq_v(
                &format!("row08 inverted[{i}]"),
                (p.c.c2Clampv)(a, hi, lo),
                (p.r.c2Clampv)(a, hi, lo),
            );
            // fully nasty
            let (nl, nh) = (rng.vec_nasty(), rng.vec_nasty());
            eq_v(&format!("row08 nasty[{i}]"), (p.c.c2Clampv)(a, nl, nh), (p.r.c2Clampv)(a, nl, nh));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 9: c2Neg / c2Skew / c2CCW90 — copy + sign flip, STRICT even for NaN.
// ---------------------------------------------------------------------------

#[test]
fn row09_c2neg_skew_ccw90() {
    let p = pair();
    let mut rng = Rng::new(0x0909);
    unsafe {
        for i in 0..N {
            let a = rng.vec_nasty();
            eq_v(&format!("row09 neg[{i}] {a:?}"), (p.c.c2Neg)(a), (p.r.c2Neg)(a));
            eq_v(&format!("row09 skew[{i}] {a:?}"), (p.c.c2Skew)(a), (p.r.c2Skew)(a));
            eq_v(&format!("row09 ccw90[{i}] {a:?}"), (p.c.c2CCW90)(a), (p.r.c2CCW90)(a));
            let b = c2v { x: rng.bits_f32(), y: rng.bits_f32() };
            eq_v(&format!("row09 neg-bits[{i}]"), (p.c.c2Neg)(b), (p.r.c2Neg)(b));
            eq_v(&format!("row09 skew-bits[{i}]"), (p.c.c2Skew)(b), (p.r.c2Skew)(b));
            eq_v(&format!("row09 ccw-bits[{i}]"), (p.c.c2CCW90)(b), (p.r.c2CCW90)(b));
        }
        // signed zero must round-trip identically
        for v in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: 0.0 },
            c2v { x: 0.0, y: -0.0 },
            c2v { x: -0.0, y: -0.0 },
        ] {
            eq_v(&format!("row09 zero-neg {v:?}"), (p.c.c2Neg)(v), (p.r.c2Neg)(v));
            eq_v(&format!("row09 zero-skew {v:?}"), (p.c.c2Skew)(v), (p.r.c2Skew)(v));
            eq_v(&format!("row09 zero-ccw {v:?}"), (p.c.c2CCW90)(v), (p.r.c2CCW90)(v));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 10: c2Div (incl. divide-by-zero -> inf/NaN)
// ---------------------------------------------------------------------------

#[test]
fn row10_c2div() {
    let p = pair();
    let mut rng = Rng::new(0x0a0a);
    let divisors = [1.0f32, -1.0, 0.0, -0.0, f32::INFINITY, f32::NEG_INFINITY,
                    f32::MIN_POSITIVE, f32::MAX, 2.0, 1e-30, 1e30, f32::from_bits(1)];
    unsafe {
        // STRICT: NaN-free vectors against every interesting divisor (this covers
        // ERRORS.md rows 15 and 16: b == 0.0 and b == -0.0).
        for (j, &d) in divisors.iter().enumerate() {
            for i in 0..256 {
                let a = rng.vec_nasty_no_nan();
                eq_v(
                    &format!("row10 strict[{j}][{i}] a={a:?} d={d:?}"),
                    (p.c.c2Div)(a, d),
                    (p.r.c2Div)(a, d),
                );
            }
        }
        for i in 0..N {
            let a = rng.vec_nasty_no_nan();
            let d = rng.nasty_no_nan();
            eq_v(&format!("row10 rand[{i}] a={a:?} d={d:?}"), (p.c.c2Div)(a, d), (p.r.c2Div)(a, d));
        }
        // SOFT tier
        for i in 0..N {
            let a = rng.vec_nasty();
            let d = rng.nasty();
            eq_v_soft(&format!("row10 soft[{i}]"), (p.c.c2Div)(a, d), (p.r.c2Div)(a, d));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11: c2Norm (incl. the zero vector -> NaN)
// ---------------------------------------------------------------------------

#[test]
fn row11_c2norm() {
    let p = pair();
    let mut rng = Rng::new(0x0b0b);
    unsafe {
        for i in 0..N {
            let a = rng.vec_nasty_no_nan();
            eq_v(&format!("row11 strict[{i}] {a:?}"), (p.c.c2Norm)(a), (p.r.c2Norm)(a));
            let b = rng.vec_any_scale();
            eq_v(&format!("row11 scaled[{i}] {b:?}"), (p.c.c2Norm)(b), (p.r.c2Norm)(b));
        }
        // ERRORS.md rows 17/18: zero vector and inf vector
        for v in [
            c2v { x: 0.0, y: 0.0 },
            c2v { x: -0.0, y: 0.0 },
            c2v { x: 0.0, y: -0.0 },
            c2v { x: 3.0, y: 4.0 },
            c2v { x: 1e30, y: 1e30 },
            c2v { x: f32::from_bits(1), y: 0.0 },
            c2v { x: f32::INFINITY, y: 1.0 },
            c2v { x: f32::INFINITY, y: f32::INFINITY },
            c2v { x: f32::MAX, y: f32::MAX },
        ] {
            eq_v(&format!("row11 fixed {v:?}"), (p.c.c2Norm)(v), (p.r.c2Norm)(v));
        }
        // SOFT tier (row 19: NaN input)
        for i in 0..N {
            let a = rng.vec_nasty();
            eq_v_soft(&format!("row11 soft[{i}] {a:?}"), (p.c.c2Norm)(a), (p.r.c2Norm)(a));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 12: c2RotIdentity / c2xIdentity
// ---------------------------------------------------------------------------

#[test]
fn row12_identities() {
    let p = pair();
    unsafe {
        for _ in 0..64 {
            eq_r("row12 rot", (p.c.c2RotIdentity)(), (p.r.c2RotIdentity)());
            eq_x("row12 x", (p.c.c2xIdentity)(), (p.r.c2xIdentity)());
        }
        // Exact expected constants from the C source.
        let r = (p.r.c2RotIdentity)();
        assert_eq!((r.c.to_bits(), r.s.to_bits()), (1.0f32.to_bits(), 0.0f32.to_bits()));
        let x = (p.r.c2xIdentity)();
        assert_eq!(
            (x.p.x.to_bits(), x.p.y.to_bits(), x.r.c.to_bits(), x.r.s.to_bits()),
            (0, 0, 1.0f32.to_bits(), 0)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 13: c2Mulrv / c2MulrvT
// ---------------------------------------------------------------------------

#[test]
fn row13_c2mulrv_transpose() {
    let p = pair();
    let mut rng = Rng::new(0x0d0d);
    unsafe {
        for i in 0..N {
            let v = rng.vec_any_scale();
            // normalised rotation
            let ang = rng.unit() * std::f32::consts::PI;
            let rn = c2r { c: ang.cos(), s: ang.sin() };
            eq_v(&format!("row13 norm-fwd[{i}]"), (p.c.c2Mulrv)(rn, v), (p.r.c2Mulrv)(rn, v));
            eq_v(&format!("row13 norm-tr[{i}]"), (p.c.c2MulrvT)(rn, v), (p.r.c2MulrvT)(rn, v));
            // un-normalised (scales/skews)
            let ru = c2r { c: rng.scaled(3.0), s: rng.scaled(3.0) };
            eq_v(&format!("row13 unnorm-fwd[{i}]"), (p.c.c2Mulrv)(ru, v), (p.r.c2Mulrv)(ru, v));
            eq_v(&format!("row13 unnorm-tr[{i}]"), (p.c.c2MulrvT)(ru, v), (p.r.c2MulrvT)(ru, v));
            // extreme but NaN-free -> STRICT
            let rs = c2r { c: rng.nasty_no_nan(), s: rng.nasty_no_nan() };
            let vs = rng.vec_nasty_no_nan();
            eq_v(&format!("row13 extreme-fwd[{i}]"), (p.c.c2Mulrv)(rs, vs), (p.r.c2Mulrv)(rs, vs));
            eq_v(&format!("row13 extreme-tr[{i}]"), (p.c.c2MulrvT)(rs, vs), (p.r.c2MulrvT)(rs, vs));
            // SOFT tier
            let rx = c2r { c: rng.nasty(), s: rng.nasty() };
            let vx = rng.vec_nasty();
            eq_v_soft(&format!("row13 soft-fwd[{i}]"), (p.c.c2Mulrv)(rx, vx), (p.r.c2Mulrv)(rx, vx));
            eq_v_soft(&format!("row13 soft-tr[{i}]"), (p.c.c2MulrvT)(rx, vx), (p.r.c2MulrvT)(rx, vx));
        }
        // identity and all-zero rotation
        let id = c2r { c: 1.0, s: 0.0 };
        let zero = c2r { c: 0.0, s: 0.0 };
        for i in 0..256 {
            let v = rng.vec_scaled(100.0);
            eq_v(&format!("row13 id[{i}]"), (p.c.c2Mulrv)(id, v), (p.r.c2Mulrv)(id, v));
            eq_v(&format!("row13 zero[{i}]"), (p.c.c2Mulrv)(zero, v), (p.r.c2Mulrv)(zero, v));
            eq_v(&format!("row13 id-t[{i}]"), (p.c.c2MulrvT)(id, v), (p.r.c2MulrvT)(id, v));
            eq_v(&format!("row13 zero-t[{i}]"), (p.c.c2MulrvT)(zero, v), (p.r.c2MulrvT)(zero, v));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 14: c2Mulxv
// ---------------------------------------------------------------------------

#[test]
fn row14_c2mulxv() {
    let p = pair();
    let mut rng = Rng::new(0x0e0e);
    unsafe {
        for i in 0..N {
            let v = rng.vec_any_scale();
            let x1 = rand_transform(&mut rng, 100.0);
            eq_v(&format!("row14 norm[{i}]"), (p.c.c2Mulxv)(x1, v), (p.r.c2Mulxv)(x1, v));
            let x2 = rand_transform_unnorm(&mut rng, 100.0);
            eq_v(&format!("row14 unnorm[{i}]"), (p.c.c2Mulxv)(x2, v), (p.r.c2Mulxv)(x2, v));
            // extreme but NaN-free -> STRICT
            let x3 = c2x {
                p: rng.vec_nasty_no_nan(),
                r: c2r { c: rng.nasty_no_nan(), s: rng.nasty_no_nan() },
            };
            let v3 = rng.vec_nasty_no_nan();
            eq_v(&format!("row14 extreme[{i}]"), (p.c.c2Mulxv)(x3, v3), (p.r.c2Mulxv)(x3, v3));
            // SOFT tier
            let x4 = c2x { p: rng.vec_nasty(), r: c2r { c: rng.nasty(), s: rng.nasty() } };
            let v4 = rng.vec_nasty();
            eq_v_soft(&format!("row14 soft[{i}]"), (p.c.c2Mulxv)(x4, v4), (p.r.c2Mulxv)(x4, v4));
        }
        // identity transform
        let id = (p.c.c2xIdentity)();
        for i in 0..256 {
            let v = rng.vec_nasty_no_nan();
            eq_v(&format!("row14 identity[{i}]"), (p.c.c2Mulxv)(id, v), (p.r.c2Mulxv)(id, v));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15: c2BBVerts — pure copies, STRICT even for NaN.
// ---------------------------------------------------------------------------

#[test]
fn row15_c2bbverts() {
    let p = pair();
    let mut rng = Rng::new(0x0f0f);
    unsafe {
        // The out buffer is 8 wide (as inside c2Proxy) and poisoned, so that
        // "wrote exactly 4 verts and nothing else" is verified too.
        let poison = c2v { x: f32::from_bits(0xAAAA_AAAA), y: f32::from_bits(0x5555_5555) };
        let mut cases: Vec<c2AABB> = Vec::new();
        for _ in 0..N {
            cases.push(rng.aabb_any());
        }
        // degenerate / inverted / extreme (CONFIGS row 15)
        cases.push(c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } });
        cases.push(c2AABB { min: c2v { x: 5.0, y: 5.0 }, max: c2v { x: -5.0, y: -5.0 } });
        cases.push(c2AABB { min: c2v { x: -0.0, y: 0.0 }, max: c2v { x: 0.0, y: -0.0 } });
        cases.push(c2AABB {
            min: c2v { x: f32::NEG_INFINITY, y: f32::NAN },
            max: c2v { x: f32::INFINITY, y: 1.0 },
        });
        cases.push(c2AABB {
            min: c2v { x: -f32::MAX, y: -f32::MAX },
            max: c2v { x: f32::MAX, y: f32::MAX },
        });
        for _ in 0..256 {
            cases.push(c2AABB { min: rng.vec_nasty(), max: rng.vec_nasty() });
        }

        for (i, bb) in cases.iter().enumerate() {
            let mut oc = [poison; 8];
            let mut or = [poison; 8];
            let mut bc = *bb;
            let mut br = *bb;
            (p.c.c2BBVerts)(oc.as_mut_ptr(), &mut bc);
            (p.r.c2BBVerts)(or.as_mut_ptr(), &mut br);
            eq_bytes(&format!("row15[{i}] out {bb:?}"), &oc, &or);
            // the input AABB must not be modified by either
            eq_bytes(&format!("row15[{i}] in"), &bc, &br);
            // and verts[4..8] must still be poison
            for k in 4..8 {
                assert_eq!(oc[k].x.to_bits(), 0xAAAA_AAAA, "C clobbered out[{k}]");
                assert_eq!(or[k].x.to_bits(), 0xAAAA_AAAA, "Rust clobbered out[{k}]");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 16..19: c2MakeProxy — pure copies, STRICT even for NaN.
// ---------------------------------------------------------------------------

fn poisoned_proxy(byte: u8) -> c2Proxy {
    let mut pr = c2Proxy::default();
    unsafe {
        std::ptr::write_bytes(
            &mut pr as *mut c2Proxy as *mut u8,
            byte,
            std::mem::size_of::<c2Proxy>(),
        );
    }
    pr
}

#[test]
fn row16_makeproxy_circle() {
    let p = pair();
    let mut rng = Rng::new(0x1010);
    unsafe {
        for i in 0..N {
            let c = match rng.below(5) {
                0 => c2Circle { p: c2v { x: 0.0, y: 0.0 }, r: 0.0 },
                1 => c2Circle { p: rng.vec_scaled(100.0), r: -rng.scaled(10.0).abs() },
                2 => c2Circle { p: rng.vec_nasty(), r: rng.nasty() },
                _ => rng.circle_any(),
            };
            let mut pc = poisoned_proxy(0xAA);
            let mut pr2 = poisoned_proxy(0xAA);
            (p.c.c2MakeProxy)(&c as *const c2Circle as *const _, C2_TYPE_CIRCLE, &mut pc);
            (p.r.c2MakeProxy)(&c as *const c2Circle as *const _, C2_TYPE_CIRCLE, &mut pr2);
            eq_proxy(&format!("row16[{i}] {c:?}"), &pc, &pr2);
            // structural expectations read off the C source
            eq_i("row16 count", pc.count, 1);
            // verts[1..8] must still be poison in BOTH
            for k in 1..8 {
                assert_eq!(pc.verts[k].x.to_bits(), 0xAAAA_AAAA, "C clobbered verts[{k}]");
                assert_eq!(pr2.verts[k].x.to_bits(), 0xAAAA_AAAA, "Rust clobbered verts[{k}]");
            }
        }
    }
}

#[test]
fn row17_makeproxy_aabb() {
    let p = pair();
    let mut rng = Rng::new(0x1111);
    unsafe {
        for i in 0..N {
            let bb = match rng.below(5) {
                0 => c2AABB { min: c2v { x: 0.0, y: 0.0 }, max: c2v { x: 0.0, y: 0.0 } },
                1 => c2AABB { min: c2v { x: 9.0, y: 9.0 }, max: c2v { x: -9.0, y: -9.0 } },
                2 => c2AABB { min: rng.vec_nasty(), max: rng.vec_nasty() },
                _ => rng.aabb_any(),
            };
            let mut pc = poisoned_proxy(0xAA);
            let mut pr2 = poisoned_proxy(0xAA);
            (p.c.c2MakeProxy)(&bb as *const c2AABB as *const _, C2_TYPE_AABB, &mut pc);
            (p.r.c2MakeProxy)(&bb as *const c2AABB as *const _, C2_TYPE_AABB, &mut pr2);
            eq_proxy(&format!("row17[{i}] {bb:?}"), &pc, &pr2);
            eq_i("row17 count", pc.count, 4);
            eq_f32("row17 radius", pc.radius, 0.0);
            for k in 4..8 {
                assert_eq!(pc.verts[k].x.to_bits(), 0xAAAA_AAAA, "C clobbered verts[{k}]");
                assert_eq!(pr2.verts[k].x.to_bits(), 0xAAAA_AAAA, "Rust clobbered verts[{k}]");
            }
        }
    }
}

#[test]
fn row18_makeproxy_capsule() {
    let p = pair();
    let mut rng = Rng::new(0x1212);
    unsafe {
        for i in 0..N {
            let cap = match rng.below(5) {
                0 => {
                    let a = rng.vec_scaled(100.0);
                    c2Capsule { a, b: a, r: 0.0 } // zero-length, zero radius
                }
                1 => c2Capsule {
                    a: rng.vec_scaled(100.0),
                    b: rng.vec_scaled(100.0),
                    r: -rng.scaled(10.0).abs(),
                },
                2 => c2Capsule { a: rng.vec_nasty(), b: rng.vec_nasty(), r: rng.nasty() },
                _ => rng.capsule_any(),
            };
            let mut pc = poisoned_proxy(0xAA);
            let mut pr2 = poisoned_proxy(0xAA);
            (p.c.c2MakeProxy)(&cap as *const c2Capsule as *const _, C2_TYPE_CAPSULE, &mut pc);
            (p.r.c2MakeProxy)(&cap as *const c2Capsule as *const _, C2_TYPE_CAPSULE, &mut pr2);
            eq_proxy(&format!("row18[{i}] {cap:?}"), &pc, &pr2);
            eq_i("row18 count", pc.count, 2);
            for k in 2..8 {
                assert_eq!(pc.verts[k].x.to_bits(), 0xAAAA_AAAA, "C clobbered verts[{k}]");
                assert_eq!(pr2.verts[k].x.to_bits(), 0xAAAA_AAAA, "Rust clobbered verts[{k}]");
            }
        }
    }
}

#[test]
fn row19_makeproxy_poison_patterns() {
    // Same shapes written into proxies pre-filled with several different byte
    // patterns: verifies exactly which bytes each switch arm writes.
    let p = pair();
    let mut rng = Rng::new(0x1313);
    unsafe {
        for pat in [0x00u8, 0xFF, 0xAA, 0x55, 0x7F, 0x80] {
            for i in 0..512 {
                let shape = rng.shape_any();
                let mut pc = poisoned_proxy(pat);
                let mut pr2 = poisoned_proxy(pat);
                (p.c.c2MakeProxy)(shape.as_ptr(), shape.ty(), &mut pc);
                (p.r.c2MakeProxy)(shape.as_ptr(), shape.ty(), &mut pr2);
                eq_proxy(&format!("row19 pat={pat:#02x}[{i}] {shape:?}"), &pc, &pr2);
            }
        }
    }
}
