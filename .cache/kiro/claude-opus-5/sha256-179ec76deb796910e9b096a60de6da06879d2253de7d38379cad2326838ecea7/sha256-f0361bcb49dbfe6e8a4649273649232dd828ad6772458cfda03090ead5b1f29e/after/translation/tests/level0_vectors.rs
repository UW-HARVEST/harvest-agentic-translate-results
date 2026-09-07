//! Phase B — CONFIGS.md rows 1-14: pure vector / rotation primitives.
//!
//! Every call goes through both `.so` exports; results compared bit-for-bit.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 20_000;

/// Row 1 — `c2V` over every float class.
#[test]
fn row01_c2V() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 1);
    for i in 0..N {
        let (x, y) = (g.wild(), g.wild());
        eq_v(
            &format!("row01 c2V #{i} ({x:?},{y:?})"),
            (c.c2V)(x, y),
            (r.c2V)(x, y),
        );
    }
}

/// Row 2 — `c2Sub` / `c2Add`, including `inf - inf` and `±0` sign rules.
#[test]
fn row02_add_sub() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 2);
    for i in 0..N {
        let (a, b) = (g.wild_v(), g.wild_v());
        eq_v(
            &format!("row02 c2Sub #{i} {a:?} {b:?}"),
            (c.c2Sub)(a, b),
            (r.c2Sub)(a, b),
        );
        eq_v(
            &format!("row02 c2Add #{i} {a:?} {b:?}"),
            (c.c2Add)(a, b),
            (r.c2Add)(a, b),
        );
    }
    // Explicit sign-of-zero and inf cases.
    for &(a, b) in &[
        (c2v { x: 0.0, y: 0.0 }, c2v { x: 0.0, y: 0.0 }),
        (c2v { x: 0.0, y: 0.0 }, c2v { x: -0.0, y: -0.0 }),
        (c2v { x: -0.0, y: -0.0 }, c2v { x: 0.0, y: 0.0 }),
        (
            c2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
            c2v {
                x: f32::INFINITY,
                y: f32::NEG_INFINITY,
            },
        ),
    ] {
        eq_v("row02 zero/inf c2Sub", (c.c2Sub)(a, b), (r.c2Sub)(a, b));
        eq_v("row02 zero/inf c2Add", (c.c2Add)(a, b), (r.c2Add)(a, b));
    }
}

/// Row 3 — `c2Mulvs`, including `0 * inf`.
#[test]
fn row03_mulvs() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 3);
    for i in 0..N {
        let a = g.wild_v();
        let s = g.wild();
        eq_v(
            &format!("row03 c2Mulvs #{i} {a:?} * {s:?}"),
            (c.c2Mulvs)(a, s),
            (r.c2Mulvs)(a, s),
        );
    }
    let z = c2v { x: 0.0, y: -0.0 };
    eq_v(
        "row03 0*inf",
        (c.c2Mulvs)(z, f32::INFINITY),
        (r.c2Mulvs)(z, f32::INFINITY),
    );
}

/// Row 4 — `c2Dot`.
#[test]
fn row04_dot() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 4);
    for i in 0..N {
        let (a, b) = (g.wild_v(), g.wild_v());
        eq_f32(
            &format!("row04 c2Dot #{i} {a:?} {b:?}"),
            (c.c2Dot)(a, b),
            (r.c2Dot)(a, b),
        );
    }
    // inf*1 + (-inf)*1 => NaN
    let a = c2v {
        x: f32::INFINITY,
        y: f32::NEG_INFINITY,
    };
    let b = c2v { x: 1.0, y: 1.0 };
    eq_f32("row04 inf-inf", (c.c2Dot)(a, b), (r.c2Dot)(a, b));
}

/// Row 5 — `c2Det2`.
#[test]
fn row05_det2() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 5);
    for i in 0..N {
        let (a, b) = (g.wild_v(), g.wild_v());
        eq_f32(
            &format!("row05 c2Det2 #{i} {a:?} {b:?}"),
            (c.c2Det2)(a, b),
            (r.c2Det2)(a, b),
        );
    }
    // Exact cancellation and collinear inputs.
    for k in 1..64u32 {
        let a = c2v {
            x: k as f32,
            y: 2.0 * k as f32,
        };
        let b = c2v {
            x: 3.0 * k as f32,
            y: 6.0 * k as f32,
        };
        eq_f32("row05 collinear", (c.c2Det2)(a, b), (r.c2Det2)(a, b));
    }
}

/// Row 6 — `c2Maxv` / `c2Minv`: ties, `±0`, and NaN (the C uses a bare
/// `>` / `<` ternary, so a NaN comparison is false and the *second* argument
/// wins).
#[test]
fn row06_max_min() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 6);
    for i in 0..N {
        let (a, b) = (g.wild_v(), g.wild_v());
        eq_v(
            &format!("row06 c2Maxv #{i} {a:?} {b:?}"),
            (c.c2Maxv)(a, b),
            (r.c2Maxv)(a, b),
        );
        eq_v(
            &format!("row06 c2Minv #{i} {a:?} {b:?}"),
            (c.c2Minv)(a, b),
            (r.c2Minv)(a, b),
        );
    }
    let specials = [0.0f32, -0.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1.0];
    for &ax in &specials {
        for &ay in &specials {
            for &bx in &specials {
                for &by in &specials {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    eq_v("row06 special max", (c.c2Maxv)(a, b), (r.c2Maxv)(a, b));
                    eq_v("row06 special min", (c.c2Minv)(a, b), (r.c2Minv)(a, b));
                }
            }
        }
    }
}

/// Row 7 — `c2Clampv`, including the inverted `lo > hi` case the C never
/// validates, and NaN in each of the three arguments.
#[test]
fn row07_clampv() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 7);
    for i in 0..N {
        let a = g.wild_v();
        let lo = g.wild_v();
        let hi = g.wild_v();
        eq_v(
            &format!("row07 c2Clampv #{i} {a:?} lo={lo:?} hi={hi:?}"),
            (c.c2Clampv)(a, lo, hi),
            (r.c2Clampv)(a, lo, hi),
        );
    }
    // Explicit inverted range.
    let a = c2v { x: 5.0, y: 5.0 };
    let lo = c2v { x: 10.0, y: 10.0 };
    let hi = c2v { x: 0.0, y: 0.0 };
    eq_v(
        "row07 inverted",
        (c.c2Clampv)(a, lo, hi),
        (r.c2Clampv)(a, lo, hi),
    );
    // NaN in each slot.
    let n = c2v {
        x: f32::NAN,
        y: f32::NAN,
    };
    eq_v("row07 nan a", (c.c2Clampv)(n, lo, hi), (r.c2Clampv)(n, lo, hi));
    eq_v("row07 nan lo", (c.c2Clampv)(a, n, hi), (r.c2Clampv)(a, n, hi));
    eq_v("row07 nan hi", (c.c2Clampv)(a, lo, n), (r.c2Clampv)(a, lo, n));
}

/// Row 8 — `c2Len`: zero vector, overflow to `inf`, NaN.
#[test]
fn row08_len() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 8);
    for i in 0..N {
        let a = g.wild_v();
        eq_f32(
            &format!("row08 c2Len #{i} {a:?}"),
            (c.c2Len)(a),
            (r.c2Len)(a),
        );
    }
    for &a in &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 3.0, y: 4.0 },
        c2v {
            x: f32::MAX,
            y: f32::MAX,
        },
        c2v { x: -1.0, y: 0.0 },
    ] {
        eq_f32("row08 special", (c.c2Len)(a), (r.c2Len)(a));
    }
}

/// Row 9 — `c2Div`, including division by zero.
#[test]
fn row09_div() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 9);
    for i in 0..N {
        let a = g.wild_v();
        let b = g.wild();
        eq_v(
            &format!("row09 c2Div #{i} {a:?} / {b:?}"),
            (c.c2Div)(a, b),
            (r.c2Div)(a, b),
        );
    }
    for &b in &[0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
        let a = c2v { x: 1.0, y: -1.0 };
        eq_v("row09 special div", (c.c2Div)(a, b), (r.c2Div)(a, b));
        let z = c2v { x: 0.0, y: 0.0 };
        eq_v("row09 zero/special", (c.c2Div)(z, b), (r.c2Div)(z, b));
    }
}

/// Row 10 — `c2Norm`: zero vector (`0/0`), huge vector (`inf` length), NaN.
#[test]
fn row10_norm() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 10);
    for i in 0..N {
        let a = g.wild_v();
        eq_v(
            &format!("row10 c2Norm #{i} {a:?}"),
            (c.c2Norm)(a),
            (r.c2Norm)(a),
        );
    }
    for &a in &[
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v {
            x: f32::MAX,
            y: f32::MAX,
        },
        c2v {
            x: f32::from_bits(1),
            y: 0.0,
        },
    ] {
        eq_v("row10 special", (c.c2Norm)(a), (r.c2Norm)(a));
    }
}

/// Row 11 — `c2Neg` / `c2Skew` / `c2CCW90`, incl. `±0` sign flips.
#[test]
fn row11_neg_skew_ccw() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 11);
    for i in 0..N {
        let a = g.wild_v();
        eq_v(
            &format!("row11 c2Neg #{i} {a:?}"),
            (c.c2Neg)(a),
            (r.c2Neg)(a),
        );
        eq_v(
            &format!("row11 c2Skew #{i} {a:?}"),
            (c.c2Skew)(a),
            (r.c2Skew)(a),
        );
        eq_v(
            &format!("row11 c2CCW90 #{i} {a:?}"),
            (c.c2CCW90)(a),
            (r.c2CCW90)(a),
        );
    }
    let z = c2v { x: 0.0, y: -0.0 };
    eq_v("row11 neg of zeros", (c.c2Neg)(z), (r.c2Neg)(z));
    eq_v("row11 skew of zeros", (c.c2Skew)(z), (r.c2Skew)(z));
    eq_v("row11 ccw of zeros", (c.c2CCW90)(z), (r.c2CCW90)(z));
}

/// Row 12 — the nullary identity constructors.
#[test]
fn row12_identities() {
    let (c, r) = apis();
    eq_r(
        "row12 c2RotIdentity",
        (c.c2RotIdentity)(),
        (r.c2RotIdentity)(),
    );
    eq_x("row12 c2xIdentity", (c.c2xIdentity)(), (r.c2xIdentity)());
    // And against the literal C values.
    let ci = (c.c2RotIdentity)();
    assert_eq!((ci.c.to_bits(), ci.s.to_bits()), (1.0f32.to_bits(), 0u32));
}

/// Row 13 — `c2Mulrv` and its transpose `c2MulrvT`, with unit, non-unit, zero
/// and NaN rotations (the C never normalizes `c2r`).
#[test]
fn row13_mulrv() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 13);
    for i in 0..N {
        let rot = g.rot();
        let v = g.wild_v();
        eq_v(
            &format!("row13 c2Mulrv #{i} {rot:?} {v:?}"),
            (c.c2Mulrv)(rot, v),
            (r.c2Mulrv)(rot, v),
        );
        eq_v(
            &format!("row13 c2MulrvT #{i} {rot:?} {v:?}"),
            (c.c2MulrvT)(rot, v),
            (r.c2MulrvT)(rot, v),
        );
    }
    for &rot in &[
        c2r { c: 1.0, s: 0.0 },
        c2r { c: 0.0, s: 1.0 },
        c2r { c: 0.0, s: 0.0 },
        c2r { c: -1.0, s: -0.0 },
        c2r {
            c: f32::NAN,
            s: 1.0,
        },
        c2r {
            c: 1e30,
            s: -1e30,
        },
    ] {
        for &v in &[
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v {
                x: f32::INFINITY,
                y: 1.0,
            },
        ] {
            eq_v("row13 special rv", (c.c2Mulrv)(rot, v), (r.c2Mulrv)(rot, v));
            eq_v(
                "row13 special rvT",
                (c.c2MulrvT)(rot, v),
                (r.c2MulrvT)(rot, v),
            );
        }
    }
}

/// Row 14 — `c2Mulxv` across identity / translation / rotation / both /
/// non-unit transforms.
#[test]
fn row14_mulxv() {
    let (c, r) = apis();
    let mut g = Rng::new(SEED ^ 14);
    for i in 0..N {
        let x = g.xform();
        let v = g.wild_v();
        eq_v(
            &format!("row14 c2Mulxv #{i} {x:?} {v:?}"),
            (c.c2Mulxv)(x, v),
            (r.c2Mulxv)(x, v),
        );
    }
    let ident = (c.c2xIdentity)();
    let mut cases = vec![ident];
    cases.push(c2x {
        p: c2v { x: 3.0, y: -4.0 },
        r: c2r { c: 1.0, s: 0.0 },
    }); // pure translation
    cases.push(c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 0.0, s: 1.0 },
    }); // pure 90-degree rotation
    cases.push(c2x {
        p: c2v { x: 1.5, y: 2.5 },
        r: c2r { c: 0.6, s: 0.8 },
    }); // rotation + translation
    cases.push(c2x {
        p: c2v { x: 0.0, y: 0.0 },
        r: c2r { c: 0.0, s: 0.0 },
    }); // degenerate rotation
    for x in cases {
        for &v in &[
            c2v { x: 1.0, y: 0.0 },
            c2v { x: 0.0, y: 1.0 },
            c2v { x: 0.0, y: 0.0 },
            c2v {
                x: f32::NAN,
                y: 0.0,
            },
        ] {
            eq_v("row14 special", (c.c2Mulxv)(x, v), (r.c2Mulxv)(x, v));
        }
    }
}
