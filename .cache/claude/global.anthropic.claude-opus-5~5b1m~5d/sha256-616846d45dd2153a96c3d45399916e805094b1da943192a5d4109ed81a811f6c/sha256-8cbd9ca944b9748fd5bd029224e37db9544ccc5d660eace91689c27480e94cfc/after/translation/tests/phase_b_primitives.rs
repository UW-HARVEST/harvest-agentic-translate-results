//! Phase B — differential tests for the level-0/1 entry points.
//!
//! `CONFIGS.md` rows 1-22: `c2V`, `c2Sub`, `c2Dot`, `c2Mulvs`, `c2Maxv`,
//! `c2Minv`, `c2Clampv`.
//!
//! Every call goes through the `.so` exports of BOTH libraries; results are
//! compared as raw bits so `±0.0` and NaN payloads are distinguished.

mod common;
use common::*;

// ===========================================================================
// c2V — rows 1, 2
// ===========================================================================

#[test]
fn row01_c2v_random_bits() {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(1);
    let mut case = Case::new("01 c2V random bits");
    for _ in 0..20_000 {
        let (x, y) = (g.any_bits(), g.any_bits());
        let cv = unsafe { (c.c2V)(x, y) };
        let rv = unsafe { (r.c2V)(x, y) };
        case.eq(vb(cv), vb(rv), || {
            format!("c2V({:#010x}, {:#010x})", fb(x), fb(y))
        });
    }
    case.finish();
}

#[test]
fn row02_c2v_edges_exhaustive() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("02 c2V edges x edges");
    for &x in EDGES {
        for &y in EDGES {
            let cv = unsafe { (c.c2V)(x, y) };
            let rv = unsafe { (r.c2V)(x, y) };
            case.eq(vb(cv), vb(rv), || {
                format!("c2V({:#010x}, {:#010x})", fb(x), fb(y))
            });
        }
    }
    case.finish();
}

// ===========================================================================
// c2Sub — rows 3, 4, 5
// ===========================================================================

fn sub_row(row: &'static str, n: usize, seed: u64, mkg: impl Fn(&mut Rng) -> (C2v, C2v)) {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(seed);
    let mut case = Case::new(row);
    for _ in 0..n {
        let (a, b) = mkg(&mut g);
        let cv = unsafe { (c.c2Sub)(a, b) };
        let rv = unsafe { (r.c2Sub)(a, b) };
        case.eq(vb(cv), vb(rv), || {
            format!(
                "c2Sub(({:#010x},{:#010x}), ({:#010x},{:#010x}))",
                fb(a.x),
                fb(a.y),
                fb(b.x),
                fb(b.y)
            )
        });
    }
    case.finish();
}

#[test]
fn row03_c2sub_finite() {
    sub_row("03 c2Sub finite", 20_000, 3, |g| {
        (g.v_finite(1e3), g.v_finite(1e3))
    });
}

#[test]
fn row04_c2sub_random_bits() {
    sub_row("04 c2Sub random bits", 20_000, 4, |g| (g.v_any(), g.v_any()));
}

#[test]
fn row05_c2sub_edges_exhaustive() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("05 c2Sub edges^4");
    for &ax in EDGES {
        for &ay in EDGES {
            for &bx in EDGES {
                for &by in EDGES {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    let cv = unsafe { (c.c2Sub)(a, b) };
                    let rv = unsafe { (r.c2Sub)(a, b) };
                    case.eq(vb(cv), vb(rv), || {
                        format!(
                            "c2Sub(({:#010x},{:#010x}), ({:#010x},{:#010x}))",
                            fb(ax),
                            fb(ay),
                            fb(bx),
                            fb(by)
                        )
                    });
                }
            }
        }
    }
    case.finish();
}

// ===========================================================================
// c2Dot — rows 6, 7, 8, 9
// ===========================================================================

fn dot_row(row: &'static str, n: usize, seed: u64, mkg: impl Fn(&mut Rng) -> (C2v, C2v)) {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(seed);
    let mut case = Case::new(row);
    for _ in 0..n {
        let (a, b) = mkg(&mut g);
        let cv = unsafe { (c.c2Dot)(a, b) };
        let rv = unsafe { (r.c2Dot)(a, b) };
        case.eq(fb(cv), fb(rv), || {
            format!(
                "c2Dot(({:#010x},{:#010x}), ({:#010x},{:#010x}))",
                fb(a.x),
                fb(a.y),
                fb(b.x),
                fb(b.y)
            )
        });
    }
    case.finish();
}

#[test]
fn row06_c2dot_finite() {
    dot_row("06 c2Dot finite", 20_000, 6, |g| {
        (g.v_finite(1e3), g.v_finite(1e3))
    });
}

#[test]
fn row07_c2dot_overflowing() {
    // Products overflow to ±inf; opposite-sign lanes give inf + -inf = NaN.
    dot_row("07 c2Dot overflow -> inf/NaN", 20_000, 7, |g| {
        (g.v_finite(1e20), g.v_finite(1e20))
    });
}

#[test]
fn row08_c2dot_random_bits() {
    // NaN-payload / operand-order sensitive: mulss/addss keep the destination's NaN.
    dot_row("08 c2Dot random bits", 40_000, 8, |g| (g.v_any(), g.v_any()));
}

#[test]
fn row09_c2dot_nan_exhaustive() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("09 c2Dot NaNs^4");
    for &ax in NANS {
        for &ay in NANS {
            for &bx in NANS {
                for &by in NANS {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    let cv = unsafe { (c.c2Dot)(a, b) };
                    let rv = unsafe { (r.c2Dot)(a, b) };
                    case.eq(fb(cv), fb(rv), || {
                        format!(
                            "c2Dot(({:#010x},{:#010x}), ({:#010x},{:#010x}))",
                            fb(ax),
                            fb(ay),
                            fb(bx),
                            fb(by)
                        )
                    });
                }
            }
        }
    }
    case.finish();
}

// ===========================================================================
// c2Mulvs — rows 10, 11, 12
// ===========================================================================

fn mulvs_row(row: &'static str, n: usize, seed: u64, mkg: impl Fn(&mut Rng) -> (C2v, f32)) {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(seed);
    let mut case = Case::new(row);
    for _ in 0..n {
        let (a, b) = mkg(&mut g);
        let cv = unsafe { (c.c2Mulvs)(a, b) };
        let rv = unsafe { (r.c2Mulvs)(a, b) };
        case.eq(vb(cv), vb(rv), || {
            format!(
                "c2Mulvs(({:#010x},{:#010x}), {:#010x})",
                fb(a.x),
                fb(a.y),
                fb(b)
            )
        });
    }
    case.finish();
}

#[test]
fn row10_c2mulvs_finite() {
    mulvs_row("10 c2Mulvs finite", 20_000, 10, |g| {
        (g.v_finite(1e3), g.sym(1e3))
    });
}

#[test]
fn row11_c2mulvs_specials() {
    // b special (±0, ±inf, NaN) x a random bits -> 0*inf, NaN-vs-NaN ordering.
    mulvs_row("11 c2Mulvs specials", 20_000, 11, |g| {
        (g.v_any(), g.spicy())
    });
}

#[test]
fn row12_c2mulvs_nan_exhaustive() {
    let (c, r) = (&apis().c, &apis().r);
    let mut case = Case::new("12 c2Mulvs NaNs^3");
    for &b in NANS {
        for &ax in NANS {
            for &ay in NANS {
                let a = C2v { x: ax, y: ay };
                let cv = unsafe { (c.c2Mulvs)(a, b) };
                let rv = unsafe { (r.c2Mulvs)(a, b) };
                case.eq(vb(cv), vb(rv), || {
                    format!(
                        "c2Mulvs(({:#010x},{:#010x}), {:#010x})",
                        fb(ax),
                        fb(ay),
                        fb(b)
                    )
                });
            }
        }
    }
    // Also the 0 * inf pairings that produce NaN.
    for &b in EDGES {
        for &ax in EDGES {
            for &ay in EDGES {
                let a = C2v { x: ax, y: ay };
                let cv = unsafe { (c.c2Mulvs)(a, b) };
                let rv = unsafe { (r.c2Mulvs)(a, b) };
                case.eq(vb(cv), vb(rv), || {
                    format!(
                        "c2Mulvs(({:#010x},{:#010x}), {:#010x})",
                        fb(ax),
                        fb(ay),
                        fb(b)
                    )
                });
            }
        }
    }
    case.finish();
}

// ===========================================================================
// c2Maxv / c2Minv — rows 13-18
// ===========================================================================

fn binv_row(
    row: &'static str,
    pick: fn(&Api) -> FnC2Binv,
    n: usize,
    seed: u64,
    mkg: impl Fn(&mut Rng) -> (C2v, C2v),
) {
    let (c, r) = (&apis().c, &apis().r);
    let (cf, rf) = (pick(c), pick(r));
    let mut g = Rng::seeded(seed);
    let mut case = Case::new(row);
    for _ in 0..n {
        let (a, b) = mkg(&mut g);
        let cv = unsafe { cf(a, b) };
        let rv = unsafe { rf(a, b) };
        case.eq(vb(cv), vb(rv), || {
            format!(
                "(({:#010x},{:#010x}), ({:#010x},{:#010x}))",
                fb(a.x),
                fb(a.y),
                fb(b.x),
                fb(b.y)
            )
        });
    }
    case.finish();
}

fn binv_exhaustive(row: &'static str, pick: fn(&Api) -> FnC2Binv) {
    let (c, r) = (&apis().c, &apis().r);
    let (cf, rf) = (pick(c), pick(r));
    let mut case = Case::new(row);
    for &ax in EDGES {
        for &ay in EDGES {
            for &bx in EDGES {
                for &by in EDGES {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    let cv = unsafe { cf(a, b) };
                    let rv = unsafe { rf(a, b) };
                    case.eq(vb(cv), vb(rv), || {
                        format!(
                            "(({:#010x},{:#010x}), ({:#010x},{:#010x}))",
                            fb(ax),
                            fb(ay),
                            fb(bx),
                            fb(by)
                        )
                    });
                }
            }
        }
    }
    case.finish();
}

/// Generator whose lanes land on `a<b`, `a>b` and `a==b` with equal odds, and
/// which throws in `±0.0` pairs (comparison equal, ternary false -> `b` wins).
fn ordered_pair(g: &mut Rng) -> (C2v, C2v) {
    let mk = |g: &mut Rng| match g.below(4) {
        0 => {
            let v = g.sym(100.0);
            (v, v) // exactly equal
        }
        1 => (0.0, -0.0),
        2 => (-0.0, 0.0),
        _ => (g.sym(100.0), g.sym(100.0)),
    };
    let (ax, bx) = mk(g);
    let (ay, by) = mk(g);
    (C2v { x: ax, y: ay }, C2v { x: bx, y: by })
}

#[test]
fn row13_c2maxv_finite() {
    binv_row("13 c2Maxv finite/orderings", |a| a.c2Maxv, 20_000, 13, |g| {
        ordered_pair(g)
    });
}

#[test]
fn row14_c2maxv_zeros_nans() {
    binv_row("14 c2Maxv zeros/NaNs", |a| a.c2Maxv, 20_000, 14, |g| {
        (g.v_spicy(), g.v_spicy())
    });
}

#[test]
fn row15_c2maxv_edges_exhaustive() {
    binv_exhaustive("15 c2Maxv edges^4", |a| a.c2Maxv);
}

#[test]
fn row16_c2minv_finite() {
    binv_row("16 c2Minv finite/orderings", |a| a.c2Minv, 20_000, 16, |g| {
        ordered_pair(g)
    });
}

#[test]
fn row17_c2minv_zeros_nans() {
    binv_row("17 c2Minv zeros/NaNs", |a| a.c2Minv, 20_000, 17, |g| {
        (g.v_spicy(), g.v_spicy())
    });
}

#[test]
fn row18_c2minv_edges_exhaustive() {
    binv_exhaustive("18 c2Minv edges^4", |a| a.c2Minv);
}

// ===========================================================================
// c2Clampv — rows 19-22
// ===========================================================================

fn clampv_row(row: &'static str, n: usize, seed: u64, mkg: impl Fn(&mut Rng) -> (C2v, C2v, C2v)) {
    let (c, r) = (&apis().c, &apis().r);
    let mut g = Rng::seeded(seed);
    let mut case = Case::new(row);
    for _ in 0..n {
        let (a, lo, hi) = mkg(&mut g);
        let cv = unsafe { (c.c2Clampv)(a, lo, hi) };
        let rv = unsafe { (r.c2Clampv)(a, lo, hi) };
        case.eq(vb(cv), vb(rv), || {
            format!(
                "c2Clampv(a=({:#010x},{:#010x}), lo=({:#010x},{:#010x}), hi=({:#010x},{:#010x}))",
                fb(a.x),
                fb(a.y),
                fb(lo.x),
                fb(lo.y),
                fb(hi.x),
                fb(hi.y)
            )
        });
    }
    case.finish();
}

#[test]
fn row19_c2clampv_valid_range() {
    clampv_row("19 c2Clampv lo<=hi", 20_000, 19, |g| {
        // lo <= hi per lane; `a` deliberately below / inside / above.
        let mk = |g: &mut Rng| {
            let l = g.sym(50.0);
            let h = l + g.unit() * 100.0;
            let a = match g.below(3) {
                0 => l - g.unit() * 20.0,
                1 => l + g.unit() * (h - l),
                _ => h + g.unit() * 20.0,
            };
            (a, l, h)
        };
        let (ax, lx, hx) = mk(g);
        let (ay, ly, hy) = mk(g);
        (
            C2v { x: ax, y: ay },
            C2v { x: lx, y: ly },
            C2v { x: hx, y: hy },
        )
    });
}

#[test]
fn row20_c2clampv_inverted_range() {
    clampv_row("20 c2Clampv lo>hi (inverted)", 20_000, 20, |g| {
        let mk = |g: &mut Rng| {
            let h = g.sym(50.0);
            let l = h + g.unit() * 100.0 + 1.0; // lo strictly > hi
            (g.sym(120.0), l, h)
        };
        let (ax, lx, hx) = mk(g);
        let (ay, ly, hy) = mk(g);
        (
            C2v { x: ax, y: ay },
            C2v { x: lx, y: ly },
            C2v { x: hx, y: hy },
        )
    });
}

#[test]
fn row21_c2clampv_nan_zero() {
    clampv_row("21 c2Clampv NaN/±0", 20_000, 21, |g| {
        (g.v_spicy(), g.v_spicy(), g.v_spicy())
    });
}

#[test]
fn row22_c2clampv_infinite_bounds() {
    clampv_row("22 c2Clampv ±inf bounds", 20_000, 22, |g| {
        let (lo, hi) = if g.next_u32() & 1 == 0 {
            (
                C2v {
                    x: f32::NEG_INFINITY,
                    y: f32::NEG_INFINITY,
                },
                C2v {
                    x: f32::INFINITY,
                    y: f32::INFINITY,
                },
            )
        } else {
            (
                C2v {
                    x: f32::INFINITY,
                    y: f32::INFINITY,
                },
                C2v {
                    x: f32::NEG_INFINITY,
                    y: f32::NEG_INFINITY,
                },
            )
        };
        (g.v_any(), lo, hi)
    });
}
