//! Phase B, `CONFIGS.md` rows 1–14: the tier-0 scalar / vector / rotation
//! primitives, driven through both `.so`s.
//!
//! These are the lowest-level entry points; every higher tier is built out of
//! them, so any operand-order or NaN-payload divergence shows up here first.

#![allow(non_snake_case)]
#![allow(clippy::useless_format, clippy::manual_range_patterns, clippy::needless_late_init, clippy::excessive_precision, clippy::too_many_arguments, clippy::needless_range_loop)]

#[macro_use]
mod common;

use common::*;

const N: usize = 40_000;

// ---------------------------------------------------------------------------
// row 1 — c2V
// ---------------------------------------------------------------------------

#[test]
fn row01_c2V() {
    let (c, r) = fnpair!("c2V", FnV);
    let mut rng = Rng::new(SEED ^ 1);
    for i in 0..N {
        let (x, y) = (rng.any_f32(), rng.any_f32());
        eq_raw(&format!("c2V #{i} ({x:?},{y:?})"), &c(x, y), &r(x, y));
    }
    // exhaustive over the oddball bit patterns
    for &bx in ODDBALLS.iter() {
        for &by in ODDBALLS.iter() {
            let (x, y) = (f32::from_bits(bx), f32::from_bits(by));
            eq_raw(
                &format!("c2V odd (0x{bx:08x},0x{by:08x})"),
                &c(x, y),
                &r(x, y),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 2 — c2Mulvs
// ---------------------------------------------------------------------------

#[test]
fn row02_c2Mulvs() {
    let (c, r) = fnpair!("c2Mulvs", FnVvF);
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..N {
        let a = rng.any_v();
        let b = rng.any_f32();
        eq_raw(&format!("c2Mulvs #{i} {a:?} * {b:?}"), &c(a, b), &r(a, b));
    }
    for &bs in SPECIALS.iter() {
        for &bx in ODDBALLS.iter() {
            for &by in ODDBALLS.iter() {
                let a = c2v {
                    x: f32::from_bits(bx),
                    y: f32::from_bits(by),
                };
                eq_raw(&format!("c2Mulvs odd {a:?} * {bs:?}"), &c(a, bs), &r(a, bs));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 3 — c2Maxv / c2Minv   (ternary select: NaN operand choice matters)
// ---------------------------------------------------------------------------

#[test]
fn row03_c2Maxv_c2Minv() {
    let (cmax, rmax) = fnpair!("c2Maxv", FnVvv);
    let (cmin, rmin) = fnpair!("c2Minv", FnVvv);
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..N {
        let a = rng.any_v();
        let b = if rng.below(4) == 0 { a } else { rng.any_v() };
        eq_raw(
            &format!("c2Maxv #{i} {a:?} {b:?}"),
            &cmax(a, b),
            &rmax(a, b),
        );
        eq_raw(
            &format!("c2Minv #{i} {a:?} {b:?}"),
            &cmin(a, b),
            &rmin(a, b),
        );
    }
    // all oddball x-pairs (NaN vs number, ±0 vs ∓0, inf vs inf)
    for &bax in ODDBALLS.iter() {
        for &bbx in ODDBALLS.iter() {
            let a = c2v {
                x: f32::from_bits(bax),
                y: f32::from_bits(bbx),
            };
            let b = c2v {
                x: f32::from_bits(bbx),
                y: f32::from_bits(bax),
            };
            eq_raw(&format!("c2Maxv odd {a:?} {b:?}"), &cmax(a, b), &rmax(a, b));
            eq_raw(&format!("c2Minv odd {a:?} {b:?}"), &cmin(a, b), &rmin(a, b));
        }
    }
}

// ---------------------------------------------------------------------------
// row 4 — c2Clampv   (incl. lo > hi, which the C never validates)
// ---------------------------------------------------------------------------

#[test]
fn row04_c2Clampv() {
    let (c, r) = fnpair!("c2Clampv", FnVvvv);
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..N {
        let a = rng.any_v();
        let (mut lo, mut hi) = (rng.any_v(), rng.any_v());
        match rng.below(5) {
            // well-formed box
            0 | 1 | 2 => {
                if lo.x > hi.x {
                    std::mem::swap(&mut lo.x, &mut hi.x);
                }
                if lo.y > hi.y {
                    std::mem::swap(&mut lo.y, &mut hi.y);
                }
            }
            // degenerate
            3 => hi = lo,
            // inverted: leave as-is
            _ => {}
        }
        eq_raw(
            &format!("c2Clampv #{i} a={a:?} lo={lo:?} hi={hi:?}"),
            &c(a, lo, hi),
            &r(a, lo, hi),
        );
    }
    // a below / inside / above, with NaN injected in each argument slot
    for &s in SPECIALS.iter() {
        for slot in 0..3 {
            let mut a = c2v { x: 1.0, y: 2.0 };
            let mut lo = c2v { x: -1.0, y: -1.0 };
            let mut hi = c2v { x: 5.0, y: 5.0 };
            match slot {
                0 => a.x = s,
                1 => lo.y = s,
                _ => hi.x = s,
            }
            eq_raw(
                &format!("c2Clampv special slot={slot} s={s:?}"),
                &c(a, lo, hi),
                &r(a, lo, hi),
            );
        }
    }
}

// ---------------------------------------------------------------------------
// row 5 — c2Sub / c2Add
// ---------------------------------------------------------------------------

#[test]
fn row05_c2Sub_c2Add() {
    let (csub, rsub) = fnpair!("c2Sub", FnVvv);
    let (cadd, radd) = fnpair!("c2Add", FnVvv);
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..N {
        let a = rng.any_v();
        // exercise exact cancellation a-a and overflow a+a
        let b = match rng.below(4) {
            0 => a,
            1 => c2v { x: -a.x, y: -a.y },
            _ => rng.any_v(),
        };
        eq_raw(
            &format!("c2Sub #{i} {a:?} {b:?}"),
            &csub(a, b),
            &rsub(a, b),
        );
        eq_raw(
            &format!("c2Add #{i} {a:?} {b:?}"),
            &cadd(a, b),
            &radd(a, b),
        );
    }
    for &bax in ODDBALLS.iter() {
        for &bbx in ODDBALLS.iter() {
            let a = c2v {
                x: f32::from_bits(bax),
                y: f32::from_bits(bbx),
            };
            let b = c2v {
                x: f32::from_bits(bbx),
                y: f32::from_bits(bax),
            };
            eq_raw(&format!("c2Sub odd {a:?} {b:?}"), &csub(a, b), &rsub(a, b));
            eq_raw(&format!("c2Add odd {a:?} {b:?}"), &cadd(a, b), &radd(a, b));
        }
    }
}

// ---------------------------------------------------------------------------
// row 6 — c2Dot   (a.x*b.x + a.y*b.y : operand order / NaN payload sensitive)
// ---------------------------------------------------------------------------

#[test]
fn row06_c2Dot() {
    let (c, r) = fnpair!("c2Dot", FnFvv);
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..N {
        let a = rng.any_v();
        let b = match rng.below(4) {
            0 => a,
            // catastrophic cancellation: a.x*b.x == -(a.y*b.y)
            1 => c2v { x: a.y, y: -a.x },
            _ => rng.any_v(),
        };
        eq_f32(&format!("c2Dot #{i} {a:?} {b:?}"), c(a, b), r(a, b));
    }
    // every oddball × oddball pair: inf*0 -> NaN, NaN+NaN payload choice, ...
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            for &s in ODDBALLS.iter() {
                let a = c2v {
                    x: f32::from_bits(p),
                    y: f32::from_bits(q),
                };
                let b = c2v {
                    x: f32::from_bits(s),
                    y: f32::from_bits(p),
                };
                eq_f32(&format!("c2Dot odd {a:?} {b:?}"), c(a, b), r(a, b));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 7 — c2Det2
// ---------------------------------------------------------------------------

#[test]
fn row07_c2Det2() {
    let (c, r) = fnpair!("c2Det2", FnFvv);
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..N {
        let a = rng.any_v();
        let b = match rng.below(4) {
            // collinear -> det == 0
            0 => c2v { x: a.x, y: a.y },
            1 => c2v {
                x: a.x * 2.0,
                y: a.y * 2.0,
            },
            _ => rng.any_v(),
        };
        eq_f32(&format!("c2Det2 #{i} {a:?} {b:?}"), c(a, b), r(a, b));
    }
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            for &s in ODDBALLS.iter() {
                let a = c2v {
                    x: f32::from_bits(p),
                    y: f32::from_bits(q),
                };
                let b = c2v {
                    x: f32::from_bits(s),
                    y: f32::from_bits(q),
                };
                eq_f32(&format!("c2Det2 odd {a:?} {b:?}"), c(a, b), r(a, b));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// row 8 — c2Len
// ---------------------------------------------------------------------------

#[test]
fn row08_c2Len() {
    let (c, r) = fnpair!("c2Len", FnFv);
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..N {
        let a = rng.any_v();
        eq_f32(&format!("c2Len #{i} {a:?}"), c(a), r(a));
    }
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let a = c2v {
                x: f32::from_bits(p),
                y: f32::from_bits(q),
            };
            eq_f32(&format!("c2Len odd {a:?}"), c(a), r(a));
        }
    }
    for &s in SPECIALS.iter() {
        let a = c2v { x: s, y: s };
        eq_f32(&format!("c2Len special {a:?}"), c(a), r(a));
    }
}

// ---------------------------------------------------------------------------
// row 9 — c2Neg / c2Skew / c2CCW90
// ---------------------------------------------------------------------------

#[test]
fn row09_c2Neg_c2Skew_c2CCW90() {
    let (cn, rn) = fnpair!("c2Neg", FnVv);
    let (cs, rs) = fnpair!("c2Skew", FnVv);
    let (cw, rw) = fnpair!("c2CCW90", FnVv);
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..N {
        let a = rng.any_v();
        eq_raw(&format!("c2Neg #{i} {a:?}"), &cn(a), &rn(a));
        eq_raw(&format!("c2Skew #{i} {a:?}"), &cs(a), &rs(a));
        eq_raw(&format!("c2CCW90 #{i} {a:?}"), &cw(a), &rw(a));
    }
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let a = c2v {
                x: f32::from_bits(p),
                y: f32::from_bits(q),
            };
            eq_raw(&format!("c2Neg odd {a:?}"), &cn(a), &rn(a));
            eq_raw(&format!("c2Skew odd {a:?}"), &cs(a), &rs(a));
            eq_raw(&format!("c2CCW90 odd {a:?}"), &cw(a), &rw(a));
        }
    }
}

// ---------------------------------------------------------------------------
// row 10 — c2Div   (1.0f/b then multiply: rounding is observable)
// ---------------------------------------------------------------------------

#[test]
fn row10_c2Div() {
    let (c, r) = fnpair!("c2Div", FnVvF);
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..N {
        let a = rng.any_v();
        let b = rng.any_f32();
        eq_raw(&format!("c2Div #{i} {a:?} / {b:?}"), &c(a, b), &r(a, b));
    }
    for &s in SPECIALS.iter() {
        for &p in ODDBALLS.iter() {
            let a = c2v {
                x: f32::from_bits(p),
                y: -f32::from_bits(p),
            };
            eq_raw(&format!("c2Div odd {a:?} / {s:?}"), &c(a, s), &r(a, s));
        }
    }
    for &q in ODDBALLS.iter() {
        let b = f32::from_bits(q);
        let a = c2v { x: 3.0, y: -7.5 };
        eq_raw(&format!("c2Div byodd {a:?} / {b:?}"), &c(a, b), &r(a, b));
    }
}

// ---------------------------------------------------------------------------
// row 11 — c2Norm
// ---------------------------------------------------------------------------

#[test]
fn row11_c2Norm() {
    let (c, r) = fnpair!("c2Norm", FnVv);
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..N {
        let a = rng.any_v();
        eq_raw(&format!("c2Norm #{i} {a:?}"), &c(a), &r(a));
    }
    // zero vector (0/0 -> NaN), huge (overflow in c2Len), tiny (underflow)
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 1e30, y: 1e30 },
        c2v { x: 1e-30, y: 1e-30 },
        c2v {
            x: f32::MIN_POSITIVE,
            y: 0.0,
        },
        c2v { x: 1.0, y: 0.0 },
        c2v { x: 3.0, y: 4.0 },
    ] {
        eq_raw(&format!("c2Norm edge {a:?}"), &c(a), &r(a));
    }
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let a = c2v {
                x: f32::from_bits(p),
                y: f32::from_bits(q),
            };
            eq_raw(&format!("c2Norm odd {a:?}"), &c(a), &r(a));
        }
    }
}

// ---------------------------------------------------------------------------
// row 12 — c2RotIdentity / c2xIdentity   (no-arg aggregate returns)
// ---------------------------------------------------------------------------

#[test]
fn row12_identities() {
    let (cr, rr) = fnpair!("c2RotIdentity", FnR);
    let (cx, rx) = fnpair!("c2xIdentity", FnX);
    for i in 0..64 {
        eq_raw(&format!("c2RotIdentity #{i}"), &cr(), &rr());
        eq_raw(&format!("c2xIdentity #{i}"), &cx(), &rx());
    }
    // and the documented values
    let v = cr();
    assert_eq!(v.c.to_bits(), 1.0f32.to_bits());
    assert_eq!(v.s.to_bits(), 0.0f32.to_bits());
}

// ---------------------------------------------------------------------------
// row 13 — c2Mulrv / c2MulrvT
// ---------------------------------------------------------------------------

#[test]
fn row13_c2Mulrv_c2MulrvT() {
    let (cm, rm) = fnpair!("c2Mulrv", FnVrv);
    let (ct, rt) = fnpair!("c2MulrvT", FnVrv);
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..N {
        let a = rng.r();
        let b = rng.any_v();
        eq_raw(
            &format!("c2Mulrv #{i} r={a:?} v={b:?}"),
            &cm(a, b),
            &rm(a, b),
        );
        eq_raw(
            &format!("c2MulrvT #{i} r={a:?} v={b:?}"),
            &ct(a, b),
            &rt(a, b),
        );
    }
    // non-unit and pathological rotations
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let a = c2r {
                c: f32::from_bits(p),
                s: f32::from_bits(q),
            };
            let b = c2v {
                x: f32::from_bits(q),
                y: f32::from_bits(p),
            };
            eq_raw(&format!("c2Mulrv odd {a:?} {b:?}"), &cm(a, b), &rm(a, b));
            eq_raw(&format!("c2MulrvT odd {a:?} {b:?}"), &ct(a, b), &rt(a, b));
        }
    }
    // exact -0.0 handling of the `-a.s` in c2MulrvT
    for s in [0.0f32, -0.0f32] {
        for x in [0.0f32, -0.0f32, 1.0, -1.0] {
            let a = c2r { c: 1.0, s };
            let b = c2v { x, y: 0.0 };
            eq_raw(&format!("c2MulrvT zero {a:?} {b:?}"), &ct(a, b), &rt(a, b));
        }
    }
}

// ---------------------------------------------------------------------------
// row 14 — c2Mulxv   (16-byte c2x passed in two xmm registers)
// ---------------------------------------------------------------------------

#[test]
fn row14_c2Mulxv() {
    let (c, r) = fnpair!("c2Mulxv", FnVxv);
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..N {
        let a = rng.x();
        let b = rng.any_v();
        eq_raw(
            &format!("c2Mulxv #{i} x={a:?} v={b:?}"),
            &c(a, b),
            &r(a, b),
        );
    }
    for &p in ODDBALLS.iter() {
        for &q in ODDBALLS.iter() {
            let a = c2x {
                p: c2v {
                    x: f32::from_bits(p),
                    y: f32::from_bits(q),
                },
                r: c2r {
                    c: f32::from_bits(q),
                    s: f32::from_bits(p),
                },
            };
            let b = c2v {
                x: f32::from_bits(p),
                y: f32::from_bits(p),
            };
            eq_raw(&format!("c2Mulxv odd {a:?} {b:?}"), &c(a, b), &r(a, b));
        }
    }
}
