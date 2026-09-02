//! Phase B — CONFIGS.md rows 1..20: the scalar / vector / transform layer.
//! Both `.so`s are loaded via `libloading`; nothing is called directly.

#![allow(non_snake_case)]

mod common;
use common::*;

const N: usize = 4000;

/// Rows 1..3 in one pass over the weird-value pool + randomized normals.
#[test]
fn row01_c2V() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x0101);
    unsafe {
        for &a in WEIRD {
            for &b in WEIRD {
                d.vec(
                    &format!("c2V({a:e},{b:e})"),
                    (p.c.c2V)(a, b),
                    (p.rs.c2V)(a, b),
                );
            }
        }
        for _ in 0..N {
            let (a, b) = (rng.f_mixed(), rng.f_mixed());
            d.vec(&format!("c2V({a:e},{b:e})"), (p.c.c2V)(a, b), (p.rs.c2V)(a, b));
        }
    }
    d.finish("row 1: c2V");
}

#[test]
fn row02_row03_c2Dot() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x0203);
    unsafe {
        // row 3: non-finite operands, full cross product of the weird pool
        for &ax in WEIRD {
            for &by in WEIRD {
                let a = c2v { x: ax, y: by };
                let b = c2v { x: by, y: ax };
                d.scalar(
                    &format!("c2Dot({}, {})", fmt_v(a), fmt_v(b)),
                    (p.c.c2Dot)(a, b),
                    (p.rs.c2Dot)(a, b),
                );
            }
        }
        // row 2: randomized magnitudes 1e-30..1e30
        for _ in 0..N {
            let a = c2v {
                x: rng.f_normal(),
                y: rng.f_normal(),
            };
            let b = c2v {
                x: rng.f_normal(),
                y: rng.f_normal(),
            };
            d.scalar("c2Dot(rand)", (p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b));
        }
        for _ in 0..N {
            let s = 10f32.powf(rng.range(-30.0, 30.0));
            let a = c2v { x: rng.sym(1.0) * s, y: rng.sym(1.0) * s };
            let b = c2v { x: rng.sym(1.0) * s, y: rng.sym(1.0) * s };
            d.scalar("c2Dot(extreme)", (p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b));
        }
        for _ in 0..N {
            let a = rng.v_weird();
            let b = rng.v_weird();
            d.scalar("c2Dot(weird)", (p.c.c2Dot)(a, b), (p.rs.c2Dot)(a, b));
        }
    }
    d.finish("rows 2-3: c2Dot");
}

#[test]
fn row04_c2Len() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x04);
    unsafe {
        for &x in WEIRD {
            for &y in WEIRD {
                let a = c2v { x, y };
                d.scalar(
                    &format!("c2Len({})", fmt_v(a)),
                    (p.c.c2Len)(a),
                    (p.rs.c2Len)(a),
                );
            }
        }
        for _ in 0..N {
            let a = rng.v_mixed();
            d.scalar(&format!("c2Len({})", fmt_v(a)), (p.c.c2Len)(a), (p.rs.c2Len)(a));
        }
        // magnitudes that overflow / underflow the squared sum
        for _ in 0..N {
            let s = 10f32.powf(rng.range(-40.0, 40.0));
            let a = c2v { x: rng.sym(1.0) * s, y: rng.sym(1.0) * s };
            d.scalar("c2Len(extreme)", (p.c.c2Len)(a), (p.rs.c2Len)(a));
        }
    }
    d.finish("row 4: c2Len");
}

#[test]
fn row05_row06_add_sub() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x0506);
    unsafe {
        for &ax in WEIRD {
            for &bx in WEIRD {
                let a = c2v { x: ax, y: bx };
                let b = c2v { x: bx, y: ax };
                d.vec("c2Add(weird)", (p.c.c2Add)(a, b), (p.rs.c2Add)(a, b));
                d.vec("c2Sub(weird)", (p.c.c2Sub)(a, b), (p.rs.c2Sub)(a, b));
            }
        }
        for _ in 0..N {
            let a = rng.v_mixed();
            let b = rng.v_mixed();
            d.vec("c2Add(rand)", (p.c.c2Add)(a, b), (p.rs.c2Add)(a, b));
            d.vec("c2Sub(rand)", (p.c.c2Sub)(a, b), (p.rs.c2Sub)(a, b));
        }
    }
    d.finish("rows 5-6: c2Add / c2Sub");
}

#[test]
fn row07_c2Mulvs() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x07);
    unsafe {
        for &vx in WEIRD {
            for &s in WEIRD {
                let a = c2v { x: vx, y: -vx };
                d.vec(
                    &format!("c2Mulvs({}, {s:e})", fmt_v(a)),
                    (p.c.c2Mulvs)(a, s),
                    (p.rs.c2Mulvs)(a, s),
                );
            }
        }
        for _ in 0..N {
            let a = rng.v_mixed();
            let s = rng.f_mixed();
            d.vec("c2Mulvs(rand)", (p.c.c2Mulvs)(a, s), (p.rs.c2Mulvs)(a, s));
        }
    }
    d.finish("row 7: c2Mulvs");
}

/// Row 8. Note the C computes `a * (1.0f/b)` — NOT `a/b` — so the reciprocal
/// rounding must be reproduced exactly.
#[test]
fn row08_c2Div() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x08);
    unsafe {
        for &vx in WEIRD {
            for &b in WEIRD {
                let a = c2v { x: vx, y: -vx };
                d.vec(
                    &format!("c2Div({}, {b:e})", fmt_v(a)),
                    (p.c.c2Div)(a, b),
                    (p.rs.c2Div)(a, b),
                );
            }
        }
        for _ in 0..N {
            let a = rng.v_mixed();
            let b = rng.f_mixed();
            d.vec("c2Div(rand)", (p.c.c2Div)(a, b), (p.rs.c2Div)(a, b));
        }
        // divisors that make 1/b overflow or underflow
        for _ in 0..N {
            let a = rng.v_normal();
            let b = 10f32.powf(rng.range(-45.0, 45.0)) * if rng.bool() { 1.0 } else { -1.0 };
            d.vec("c2Div(extreme)", (p.c.c2Div)(a, b), (p.rs.c2Div)(a, b));
        }
    }
    d.finish("row 8: c2Div");
}

#[test]
fn row09_c2Norm() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x09);
    unsafe {
        for &x in WEIRD {
            for &y in WEIRD {
                let a = c2v { x, y };
                d.vec(
                    &format!("c2Norm({})", fmt_v(a)),
                    (p.c.c2Norm)(a),
                    (p.rs.c2Norm)(a),
                );
            }
        }
        for _ in 0..N {
            let a = rng.v_mixed();
            d.vec("c2Norm(rand)", (p.c.c2Norm)(a), (p.rs.c2Norm)(a));
        }
        for _ in 0..N {
            let s = 10f32.powf(rng.range(-40.0, 40.0));
            let a = c2v { x: rng.sym(1.0) * s, y: rng.sym(1.0) * s };
            d.vec("c2Norm(extreme)", (p.c.c2Norm)(a), (p.rs.c2Norm)(a));
        }
    }
    d.finish("row 9: c2Norm");
}

/// Rows 10-11. The C uses `a < b ? a : b`, which returns `b` on NaN — not
/// `fminf` semantics. Every NaN placement is checked.
#[test]
fn row10_row11_minv_maxv() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x1011);
    unsafe {
        for &ax in WEIRD {
            for &bx in WEIRD {
                let a = c2v { x: ax, y: bx };
                let b = c2v { x: bx, y: ax };
                d.vec("c2Minv(weird)", (p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b));
                d.vec("c2Maxv(weird)", (p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b));
                // equal-value case (both orders)
                d.vec("c2Minv(eq)", (p.c.c2Minv)(a, a), (p.rs.c2Minv)(a, a));
                d.vec("c2Maxv(eq)", (p.c.c2Maxv)(a, a), (p.rs.c2Maxv)(a, a));
            }
        }
        for _ in 0..N {
            let a = rng.v_mixed();
            let b = rng.v_mixed();
            d.vec("c2Minv(rand)", (p.c.c2Minv)(a, b), (p.rs.c2Minv)(a, b));
            d.vec("c2Maxv(rand)", (p.c.c2Maxv)(a, b), (p.rs.c2Maxv)(a, b));
        }
    }
    d.finish("rows 10-11: c2Minv / c2Maxv");
}

#[test]
fn row12_row13_row14_skew_absv_ccw90() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x1214);
    unsafe {
        for &x in WEIRD {
            for &y in WEIRD {
                let a = c2v { x, y };
                let tag = fmt_v(a);
                d.vec(&format!("c2Skew({tag})"), (p.c.c2Skew)(a), (p.rs.c2Skew)(a));
                d.vec(&format!("c2Absv({tag})"), (p.c.c2Absv)(a), (p.rs.c2Absv)(a));
                d.vec(&format!("c2CCW90({tag})"), (p.c.c2CCW90)(a), (p.rs.c2CCW90)(a));
            }
        }
        for _ in 0..N {
            let a = rng.v_mixed();
            d.vec("c2Skew(rand)", (p.c.c2Skew)(a), (p.rs.c2Skew)(a));
            d.vec("c2Absv(rand)", (p.c.c2Absv)(a), (p.rs.c2Absv)(a));
            d.vec("c2CCW90(rand)", (p.c.c2CCW90)(a), (p.rs.c2CCW90)(a));
        }
    }
    // Explicit -0.0 pin: the C's `x < 0 ? -x : x` must NOT normalise -0.0.
    unsafe {
        let mz = c2v { x: -0.0, y: -0.0 };
        let ca = (p.c.c2Absv)(mz);
        let ra = (p.rs.c2Absv)(mz);
        assert_eq!(
            ca.x.to_bits(), 0x8000_0000u32,
            "C's c2Absv(-0.0) is expected to preserve -0.0 (got {:#010x})", ca.x.to_bits()
        );
        assert_eq!(ra.x.to_bits(), ca.x.to_bits());
        assert_eq!(ra.y.to_bits(), ca.y.to_bits());
    }
    d.finish("rows 12-14: c2Skew / c2Absv / c2CCW90");
}

#[test]
fn row15_c2MulmvT() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x15);
    unsafe {
        // identity + zero matrix
        let ident = c2m {
            x: c2v { x: 1.0, y: 0.0 },
            y: c2v { x: 0.0, y: 1.0 },
        };
        let zero = c2m::default();
        for &x in WEIRD {
            for &y in WEIRD {
                let b = c2v { x, y };
                d.vec("c2MulmvT(ident)", (p.c.c2MulmvT)(ident, b), (p.rs.c2MulmvT)(ident, b));
                d.vec("c2MulmvT(zero)", (p.c.c2MulmvT)(zero, b), (p.rs.c2MulmvT)(zero, b));
            }
        }
        for _ in 0..N {
            let m = c2m {
                x: rng.v_mixed(),
                y: rng.v_mixed(),
            };
            let b = rng.v_mixed();
            d.vec("c2MulmvT(rand)", (p.c.c2MulmvT)(m, b), (p.rs.c2MulmvT)(m, b));
        }
        // the shape c2RaytoCapsule actually builds: M.y = unit, M.x = CCW90(M.y)
        for _ in 0..N {
            let y = rng.v_dir();
            let m = c2m {
                x: (p.c.c2CCW90)(y),
                y,
            };
            let b = rng.v_small();
            d.vec("c2MulmvT(capsule-frame)", (p.c.c2MulmvT)(m, b), (p.rs.c2MulmvT)(m, b));
        }
    }
    d.finish("row 15: c2MulmvT");
}

#[test]
fn row16_row17_identities() {
    let p = load_pair();
    let mut d = Diff::new();
    unsafe {
        for _ in 0..64 {
            d.rot("c2RotIdentity", (p.c.c2RotIdentity)(), (p.rs.c2RotIdentity)());
            let cx = (p.c.c2xIdentity)();
            let rx = (p.rs.c2xIdentity)();
            d.vec("c2xIdentity.p", cx.p, rx.p);
            d.rot("c2xIdentity.r", cx.r, rx.r);
        }
    }
    d.finish("rows 16-17: c2RotIdentity / c2xIdentity");
}

#[test]
fn row18_row19_row20_rotations() {
    let p = load_pair();
    let mut d = Diff::new();
    let mut rng = Rng::new(0x1820);
    unsafe {
        // degenerate / weird rotations
        for &c in WEIRD {
            for &s in WEIRD {
                let r = c2r { c, s };
                let b = c2v { x: s, y: c };
                d.vec("c2Mulrv(weird)", (p.c.c2Mulrv)(r, b), (p.rs.c2Mulrv)(r, b));
                d.vec("c2MulrvT(weird)", (p.c.c2MulrvT)(r, b), (p.rs.c2MulrvT)(r, b));
                let x = c2x { p: b, r };
                d.vec("c2MulxvT(weird)", (p.c.c2MulxvT)(x, b), (p.rs.c2MulxvT)(x, b));
            }
        }
        for _ in 0..N {
            // unit rotations
            let r = rng.rot_unit();
            let b = rng.v_small();
            d.vec("c2Mulrv(unit)", (p.c.c2Mulrv)(r, b), (p.rs.c2Mulrv)(r, b));
            d.vec("c2MulrvT(unit)", (p.c.c2MulrvT)(r, b), (p.rs.c2MulrvT)(r, b));
            let x = c2x { p: rng.v_small(), r };
            d.vec("c2MulxvT(unit)", (p.c.c2MulxvT)(x, b), (p.rs.c2MulxvT)(x, b));
        }
        for _ in 0..N {
            // NON-unit rotations (the C never normalises)
            let r = c2r {
                c: rng.f_mixed(),
                s: rng.f_mixed(),
            };
            let b = rng.v_mixed();
            d.vec("c2Mulrv(nonunit)", (p.c.c2Mulrv)(r, b), (p.rs.c2Mulrv)(r, b));
            d.vec("c2MulrvT(nonunit)", (p.c.c2MulrvT)(r, b), (p.rs.c2MulrvT)(r, b));
            let x = c2x { p: rng.v_mixed(), r };
            d.vec("c2MulxvT(nonunit)", (p.c.c2MulxvT)(x, b), (p.rs.c2MulxvT)(x, b));
        }
        // zero rotation (c=0,s=0) — explicitly in CONFIGS row 49's family
        let zr = c2r { c: 0.0, s: 0.0 };
        for _ in 0..256 {
            let b = rng.v_mixed();
            d.vec("c2Mulrv(zero-rot)", (p.c.c2Mulrv)(zr, b), (p.rs.c2Mulrv)(zr, b));
            d.vec("c2MulrvT(zero-rot)", (p.c.c2MulrvT)(zr, b), (p.rs.c2MulrvT)(zr, b));
        }
    }
    d.finish("rows 18-20: c2Mulrv / c2MulrvT / c2MulxvT");
}
