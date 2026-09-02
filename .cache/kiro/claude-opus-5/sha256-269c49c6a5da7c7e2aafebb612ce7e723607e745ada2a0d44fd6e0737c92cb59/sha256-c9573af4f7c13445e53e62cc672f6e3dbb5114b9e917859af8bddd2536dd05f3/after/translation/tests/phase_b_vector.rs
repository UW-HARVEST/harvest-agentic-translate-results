//! Phase B — valid-path differential tests for the low-level vector helpers.
//! Covers CONFIGS.md rows 1–12.
//!
//! Every call goes through `dlopen`/`dlsym` on both `.so`s; no Rust function is
//! called directly.

mod common;
use common::*;

const SEED: u64 = 0x5EED_C2A1;
const N: usize = 20_000;

/// Row 1 — `c2V`
#[test]
fn cfg_01_c2V() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 1);
    let mut d = Diff::new("row01 c2V");

    // All special classes, exhaustively crossed.
    for &x in SPECIALS {
        for &y in SPECIALS {
            let cv = unsafe { (l.c.c2V)(x, y) };
            let rv = unsafe { (l.r.c2V)(x, y) };
            d.check_v(cv, rv, || format!("c2V({}, {})", fmt_f(x), fmt_f(y)));
        }
    }
    // Every distinguishable NaN payload must survive the return-in-xmm0 path.
    for &bx in NAN_BITS {
        for &by in NAN_BITS {
            let (x, y) = (f32::from_bits(bx), f32::from_bits(by));
            let cv = unsafe { (l.c.c2V)(x, y) };
            let rv = unsafe { (l.r.c2V)(x, y) };
            d.check_v(cv, rv, || format!("c2V(nan {bx:#x}, nan {by:#x})"));
        }
    }
    for _ in 0..N {
        let (x, y) = (rng.any_bits(), rng.any_bits());
        let cv = unsafe { (l.c.c2V)(x, y) };
        let rv = unsafe { (l.r.c2V)(x, y) };
        d.check_v(cv, rv, || format!("c2V({}, {})", fmt_f(x), fmt_f(y)));
    }
    d.finish();
}

/// Rows 2 & 3 — `c2Dot`: finite mixed magnitudes, then special classes.
#[test]
fn cfg_02_03_c2Dot() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 2);
    let mut d = Diff::new("row02/03 c2Dot");

    for _ in 0..N {
        let (a, b) = (rng.vec_coord(), rng.vec_coord());
        d.check_f(
            unsafe { (l.c.c2Dot)(a, b) },
            unsafe { (l.r.c2Dot)(a, b) },
            || format!("c2Dot({}, {})", fmt_v(a), fmt_v(b)),
        );
    }
    // Wide magnitudes: forces overflow to inf and cancellation to inf-inf=NaN.
    for _ in 0..N {
        let a = c2v {
            x: rng.wide(),
            y: rng.wide(),
        };
        let b = c2v {
            x: rng.wide(),
            y: rng.wide(),
        };
        d.check_f(
            unsafe { (l.c.c2Dot)(a, b) },
            unsafe { (l.r.c2Dot)(a, b) },
            || format!("c2Dot({}, {})", fmt_v(a), fmt_v(b)),
        );
    }
    // Row 3: special classes. `inf*0` -> NaN, `inf + -inf` -> NaN; operand
    // order decides which NaN is returned.
    for &ax in SPECIALS {
        for &by in SPECIALS {
            let a = c2v { x: ax, y: by };
            let b = c2v { x: by, y: ax };
            d.check_f(
                unsafe { (l.c.c2Dot)(a, b) },
                unsafe { (l.r.c2Dot)(a, b) },
                || format!("c2Dot({}, {})", fmt_v(a), fmt_v(b)),
            );
        }
    }
    for _ in 0..N {
        let (a, b) = (rng.vec_spicy(), rng.vec_spicy());
        d.check_f(
            unsafe { (l.c.c2Dot)(a, b) },
            unsafe { (l.r.c2Dot)(a, b) },
            || format!("c2Dot({}, {})", fmt_v(a), fmt_v(b)),
        );
    }
    d.finish();
}

/// Row 4 — `c2Len`
#[test]
fn cfg_04_c2Len() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 4);
    let mut d = Diff::new("row04 c2Len");

    for &x in SPECIALS {
        for &y in SPECIALS {
            let a = c2v { x, y };
            d.check_f(unsafe { (l.c.c2Len)(a) }, unsafe { (l.r.c2Len)(a) }, || {
                format!("c2Len({})", fmt_v(a))
            });
        }
    }
    for _ in 0..N {
        let a = rng.vec_coord();
        d.check_f(unsafe { (l.c.c2Len)(a) }, unsafe { (l.r.c2Len)(a) }, || {
            format!("c2Len({})", fmt_v(a))
        });
    }
    for _ in 0..N {
        let a = rng.vec_spicy();
        d.check_f(unsafe { (l.c.c2Len)(a) }, unsafe { (l.r.c2Len)(a) }, || {
            format!("c2Len({})", fmt_v(a))
        });
    }
    d.finish();
}

/// Row 5 — `c2Add` / `c2Sub`
#[test]
fn cfg_05_c2Add_c2Sub() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 5);
    let mut d = Diff::new("row05 c2Add/c2Sub");

    let mut pairs: Vec<(c2v, c2v)> = Vec::new();
    for &ax in SPECIALS {
        for &bx in SPECIALS {
            pairs.push((c2v { x: ax, y: bx }, c2v { x: bx, y: ax }));
        }
    }
    // NaN payload cross-product: `addss` keeps the destination NaN, so operand
    // order is observable.
    for &na in NAN_BITS {
        for &v in &[1.0f32, -1.0, 0.0, f32::INFINITY] {
            let n = f32::from_bits(na);
            pairs.push((c2v { x: n, y: v }, c2v { x: v, y: n }));
            pairs.push((c2v { x: v, y: n }, c2v { x: n, y: v }));
        }
        for &nb in NAN_BITS {
            pairs.push((
                c2v {
                    x: f32::from_bits(na),
                    y: f32::from_bits(nb),
                },
                c2v {
                    x: f32::from_bits(nb),
                    y: f32::from_bits(na),
                },
            ));
        }
    }
    for _ in 0..N {
        pairs.push((rng.vec_coord(), rng.vec_coord()));
    }
    for _ in 0..N {
        pairs.push((rng.vec_spicy(), rng.vec_spicy()));
    }

    for (a, b) in pairs {
        d.check_v(unsafe { (l.c.c2Add)(a, b) }, unsafe { (l.r.c2Add)(a, b) }, || {
            format!("c2Add({}, {})", fmt_v(a), fmt_v(b))
        });
        d.check_v(unsafe { (l.c.c2Sub)(a, b) }, unsafe { (l.r.c2Sub)(a, b) }, || {
            format!("c2Sub({}, {})", fmt_v(a), fmt_v(b))
        });
    }
    d.finish();
}

/// Row 6 — `c2Mulvs`
#[test]
fn cfg_06_c2Mulvs() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 6);
    let mut d = Diff::new("row06 c2Mulvs");

    for &x in SPECIALS {
        for &s in SPECIALS {
            let a = c2v { x, y: -x };
            d.check_v(
                unsafe { (l.c.c2Mulvs)(a, s) },
                unsafe { (l.r.c2Mulvs)(a, s) },
                || format!("c2Mulvs({}, {})", fmt_v(a), fmt_f(s)),
            );
        }
    }
    for &nb in NAN_BITS {
        let s = f32::from_bits(nb);
        for &v in &[0.0f32, -0.0, 1.0, f32::INFINITY, f32::NEG_INFINITY] {
            let a = c2v { x: v, y: -v };
            d.check_v(
                unsafe { (l.c.c2Mulvs)(a, s) },
                unsafe { (l.r.c2Mulvs)(a, s) },
                || format!("c2Mulvs({}, nan {nb:#x})", fmt_v(a)),
            );
            let a2 = c2v {
                x: s,
                y: f32::from_bits(nb ^ 0x8000_0000),
            };
            d.check_v(
                unsafe { (l.c.c2Mulvs)(a2, v) },
                unsafe { (l.r.c2Mulvs)(a2, v) },
                || format!("c2Mulvs(nan-vec, {})", fmt_f(v)),
            );
        }
    }
    for _ in 0..N {
        let (a, s) = (rng.vec_coord(), rng.coord());
        d.check_v(
            unsafe { (l.c.c2Mulvs)(a, s) },
            unsafe { (l.r.c2Mulvs)(a, s) },
            || format!("c2Mulvs({}, {})", fmt_v(a), fmt_f(s)),
        );
    }
    for _ in 0..N {
        let (a, s) = (rng.vec_spicy(), rng.spicy());
        d.check_v(
            unsafe { (l.c.c2Mulvs)(a, s) },
            unsafe { (l.r.c2Mulvs)(a, s) },
            || format!("c2Mulvs({}, {})", fmt_v(a), fmt_f(s)),
        );
    }
    d.finish();
}

/// Row 7 — `c2Div` (reciprocal-then-multiply, including divide-by-zero)
#[test]
fn cfg_07_c2Div() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 7);
    let mut d = Diff::new("row07 c2Div");

    for &x in SPECIALS {
        for &b in SPECIALS {
            let a = c2v { x, y: -x };
            d.check_v(unsafe { (l.c.c2Div)(a, b) }, unsafe { (l.r.c2Div)(a, b) }, || {
                format!("c2Div({}, {})", fmt_v(a), fmt_f(b))
            });
        }
    }
    // Explicit division by both zeros: 1/0 == inf, then 0*inf == NaN.
    for &b in &[0.0f32, -0.0f32] {
        for &x in &[0.0f32, -0.0, 1.0, -1.0, f32::INFINITY] {
            let a = c2v { x, y: 0.0 };
            d.check_v(unsafe { (l.c.c2Div)(a, b) }, unsafe { (l.r.c2Div)(a, b) }, || {
                format!("c2Div({}, {})", fmt_v(a), fmt_f(b))
            });
        }
    }
    for _ in 0..N {
        let (a, b) = (rng.vec_coord(), rng.coord());
        d.check_v(unsafe { (l.c.c2Div)(a, b) }, unsafe { (l.r.c2Div)(a, b) }, || {
            format!("c2Div({}, {})", fmt_v(a), fmt_f(b))
        });
    }
    for _ in 0..N {
        let (a, b) = (rng.vec_spicy(), rng.spicy());
        d.check_v(unsafe { (l.c.c2Div)(a, b) }, unsafe { (l.r.c2Div)(a, b) }, || {
            format!("c2Div({}, {})", fmt_v(a), fmt_f(b))
        });
    }
    d.finish();
}

/// Row 8 — `c2Norm`, including the unguarded zero vector.
#[test]
fn cfg_08_c2Norm() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 8);
    let mut d = Diff::new("row08 c2Norm");

    let mut inputs = vec![
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: 0.0, y: 5.0 },
        c2v { x: 5.0, y: 0.0 },
        c2v { x: -0.0, y: 5.0 },
        c2v { x: 1e30, y: 1e30 },   // c2Len overflows to inf
        c2v { x: 1e-30, y: 1e-30 }, // c2Dot underflows to 0
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::MIN_POSITIVE, y: 0.0 },
        c2v { x: 1e-45, y: 1e-45 },
    ];
    for &x in SPECIALS {
        for &y in SPECIALS {
            inputs.push(c2v { x, y });
        }
    }
    for &nb in NAN_BITS {
        inputs.push(c2v {
            x: f32::from_bits(nb),
            y: 1.0,
        });
        inputs.push(c2v {
            x: 1.0,
            y: f32::from_bits(nb),
        });
    }
    for _ in 0..N {
        inputs.push(rng.vec_coord());
    }
    for _ in 0..N {
        inputs.push(rng.vec_spicy());
    }

    for a in inputs {
        d.check_v(unsafe { (l.c.c2Norm)(a) }, unsafe { (l.r.c2Norm)(a) }, || {
            format!("c2Norm({})", fmt_v(a))
        });
    }
    d.finish();
}

/// Row 9 — `c2Minv` / `c2Maxv`: the ternary idiom, NOT `f32::min`/`max`.
#[test]
fn cfg_09_c2Minv_c2Maxv() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 9);
    let mut d = Diff::new("row09 c2Minv/c2Maxv");

    let mut pairs: Vec<(c2v, c2v)> = Vec::new();
    // Full special x special cross product on both components.
    for &ax in SPECIALS {
        for &bx in SPECIALS {
            pairs.push((c2v { x: ax, y: bx }, c2v { x: bx, y: ax }));
            pairs.push((c2v { x: ax, y: ax }, c2v { x: bx, y: bx }));
        }
    }
    // +0.0 vs -0.0: `a < b` is false, so the ternary returns `b` -- the sign of
    // the returned zero is observable and differs from f32::min.
    pairs.push((c2v { x: 0.0, y: -0.0 }, c2v { x: -0.0, y: 0.0 }));
    pairs.push((c2v { x: -0.0, y: 0.0 }, c2v { x: 0.0, y: -0.0 }));
    // NaN on the left vs the right side of the comparison.
    for &nb in NAN_BITS {
        let n = f32::from_bits(nb);
        pairs.push((c2v { x: n, y: n }, c2v { x: 1.0, y: -1.0 }));
        pairs.push((c2v { x: 1.0, y: -1.0 }, c2v { x: n, y: n }));
    }
    // Equal components (neither `<` nor `>` fires).
    for _ in 0..1000 {
        let v = rng.coord();
        pairs.push((c2v { x: v, y: v }, c2v { x: v, y: v }));
    }
    for _ in 0..N {
        pairs.push((rng.vec_coord(), rng.vec_coord()));
    }
    for _ in 0..N {
        pairs.push((rng.vec_spicy(), rng.vec_spicy()));
    }

    for (a, b) in pairs {
        d.check_v(
            unsafe { (l.c.c2Minv)(a, b) },
            unsafe { (l.r.c2Minv)(a, b) },
            || format!("c2Minv({}, {})", fmt_v(a), fmt_v(b)),
        );
        d.check_v(
            unsafe { (l.c.c2Maxv)(a, b) },
            unsafe { (l.r.c2Maxv)(a, b) },
            || format!("c2Maxv({}, {})", fmt_v(a), fmt_v(b)),
        );
    }
    d.finish();
}

/// Row 10 — `c2Skew` / `c2CCW90`: float negation, sign of zero and of NaN.
#[test]
fn cfg_10_c2Skew_c2CCW90() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 10);
    let mut d = Diff::new("row10 c2Skew/c2CCW90");

    let mut inputs = Vec::new();
    for &x in SPECIALS {
        for &y in SPECIALS {
            inputs.push(c2v { x, y });
        }
    }
    for &nb in NAN_BITS {
        inputs.push(c2v {
            x: f32::from_bits(nb),
            y: f32::from_bits(nb ^ 0x8000_0000),
        });
    }
    for _ in 0..N {
        inputs.push(rng.vec_coord());
    }
    for _ in 0..N {
        inputs.push(rng.vec_spicy());
    }
    for a in inputs {
        d.check_v(unsafe { (l.c.c2Skew)(a) }, unsafe { (l.r.c2Skew)(a) }, || {
            format!("c2Skew({})", fmt_v(a))
        });
        d.check_v(unsafe { (l.c.c2CCW90)(a) }, unsafe { (l.r.c2CCW90)(a) }, || {
            format!("c2CCW90({})", fmt_v(a))
        });
    }
    d.finish();
}

/// Row 11 — `c2Absv`: `a < 0 ? -a : a`, which keeps `-0.0` and a NaN's sign.
#[test]
fn cfg_11_c2Absv() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 11);
    let mut d = Diff::new("row11 c2Absv");

    let mut inputs = vec![
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
    ];
    for &x in SPECIALS {
        for &y in SPECIALS {
            inputs.push(c2v { x, y });
        }
    }
    for &nb in NAN_BITS {
        inputs.push(c2v {
            x: f32::from_bits(nb),
            y: f32::from_bits(nb ^ 0x8000_0000),
        });
    }
    for _ in 0..N {
        inputs.push(rng.vec_coord());
    }
    for _ in 0..N {
        inputs.push(rng.vec_spicy());
    }
    for a in inputs {
        d.check_v(unsafe { (l.c.c2Absv)(a) }, unsafe { (l.r.c2Absv)(a) }, || {
            format!("c2Absv({})", fmt_v(a))
        });
    }
    d.finish();
}

/// Row 12 — `c2MulmvT` (c2m is a 16-byte all-float struct: xmm0+xmm1)
#[test]
fn cfg_12_c2MulmvT() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 12);
    let mut d = Diff::new("row12 c2MulmvT");

    let mut cases: Vec<(c2m, c2v)> = vec![
        (
            c2m {
                x: c2v { x: 1.0, y: 0.0 },
                y: c2v { x: 0.0, y: 1.0 },
            },
            c2v { x: 3.0, y: -4.0 },
        ),
        (
            c2m {
                x: c2v { x: 0.0, y: 0.0 },
                y: c2v { x: 0.0, y: 0.0 },
            },
            c2v {
                x: f32::INFINITY,
                y: 1.0,
            },
        ),
    ];
    for &v in SPECIALS {
        cases.push((
            c2m {
                x: c2v { x: v, y: -v },
                y: c2v { x: -v, y: v },
            },
            c2v { x: 1.0, y: -1.0 },
        ));
        cases.push((
            c2m {
                x: c2v { x: 1.0, y: 2.0 },
                y: c2v { x: 3.0, y: 4.0 },
            },
            c2v { x: v, y: -v },
        ));
    }
    for &nb in NAN_BITS {
        let n = f32::from_bits(nb);
        cases.push((
            c2m {
                x: c2v { x: n, y: 1.0 },
                y: c2v { x: 1.0, y: n },
            },
            c2v { x: 2.0, y: n },
        ));
    }
    for _ in 0..N {
        cases.push((
            c2m {
                x: rng.vec_coord(),
                y: rng.vec_coord(),
            },
            rng.vec_coord(),
        ));
    }
    for _ in 0..N {
        cases.push((
            c2m {
                x: rng.vec_spicy(),
                y: rng.vec_spicy(),
            },
            rng.vec_spicy(),
        ));
    }
    for (m, v) in cases {
        d.check_v(
            unsafe { (l.c.c2MulmvT)(m, v) },
            unsafe { (l.r.c2MulmvT)(m, v) },
            || {
                format!(
                    "c2MulmvT({{x:{}, y:{}}}, {})",
                    fmt_v(m.x),
                    fmt_v(m.y),
                    fmt_v(v)
                )
            },
        );
    }
    d.finish();
}
