//! Phase B: differential tests for the leaf-level vector helpers.
//!
//! Every call goes through the exported symbol of BOTH `.so`s.

mod common;
use common::*;

const N: usize = 20_000;

/// c2V(x, y) - pure struct construction, including all hostile bit patterns.
#[test]
fn b01_c2v_construct() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x0001);
    let mut d = Diff::new();
    for i in 0..N {
        let (x, y) = if i % 2 == 0 {
            (rng.nice(), rng.nice())
        } else {
            (rng.hostile(), rng.hostile())
        };
        let a = (c.c2V)(x, y);
        let b = (r.c2V)(x, y);
        d.check(v_eq(a, b), || {
            format!(
                "c2V({}, {}): C={} RUST={}",
                fmt_f(x),
                fmt_f(y),
                fmt_v(a),
                fmt_v(b)
            )
        });
    }
    d.finish("c2V");
}

macro_rules! unary_v {
    ($name:ident, $field:ident) => {
        #[test]
        fn $name() {
            let (c, r) = apis();
            let mut rng = Rng::new(0x1000 + line!() as u64);
            let mut d = Diff::new();
            for i in 0..N {
                let a = if i % 2 == 0 {
                    rng.vec_nice()
                } else {
                    rng.vec_hostile()
                };
                let x = (c.$field)(a);
                let y = (r.$field)(a);
                d.check(v_eq(x, y), || {
                    format!(
                        "{}({}): C={} RUST={}",
                        stringify!($field),
                        fmt_v(a),
                        fmt_v(x),
                        fmt_v(y)
                    )
                });
            }
            d.finish(stringify!($field));
        }
    };
}

unary_v!(b07_c2norm, c2Norm);
unary_v!(b09_c2skew, c2Skew);
unary_v!(b09_c2absv, c2Absv);
unary_v!(b09_c2ccw90, c2CCW90);

macro_rules! binary_vv {
    ($name:ident, $field:ident) => {
        #[test]
        fn $name() {
            let (c, r) = apis();
            let mut rng = Rng::new(0x2000 + line!() as u64);
            let mut d = Diff::new();
            for i in 0..N {
                let (a, b) = match i % 4 {
                    0 => (rng.vec_nice(), rng.vec_nice()),
                    1 => (rng.vec_hostile(), rng.vec_nice()),
                    2 => (rng.vec_nice(), rng.vec_hostile()),
                    _ => (rng.vec_hostile(), rng.vec_hostile()),
                };
                let x = (c.$field)(a, b);
                let y = (r.$field)(a, b);
                d.check(v_eq(x, y), || {
                    format!(
                        "{}({}, {}): C={} RUST={}",
                        stringify!($field),
                        fmt_v(a),
                        fmt_v(b),
                        fmt_v(x),
                        fmt_v(y)
                    )
                });
            }
            d.finish(stringify!($field));
        }
    };
}

binary_vv!(b04_c2add, c2Add);
binary_vv!(b04_c2sub, c2Sub);
binary_vv!(b08_c2minv, c2Minv);
binary_vv!(b08_c2maxv, c2Maxv);

macro_rules! binary_vs {
    ($name:ident, $field:ident) => {
        #[test]
        fn $name() {
            let (c, r) = apis();
            let mut rng = Rng::new(0x3000 + line!() as u64);
            let mut d = Diff::new();
            for i in 0..N {
                let (a, s) = match i % 4 {
                    0 => (rng.vec_nice(), rng.nice()),
                    1 => (rng.vec_hostile(), rng.nice()),
                    2 => (rng.vec_nice(), rng.hostile()),
                    _ => (rng.vec_hostile(), rng.hostile()),
                };
                let x = (c.$field)(a, s);
                let y = (r.$field)(a, s);
                d.check(v_eq(x, y), || {
                    format!(
                        "{}({}, {}): C={} RUST={}",
                        stringify!($field),
                        fmt_v(a),
                        fmt_f(s),
                        fmt_v(x),
                        fmt_v(y)
                    )
                });
            }
            d.finish(stringify!($field));
        }
    };
}

binary_vs!(b05_c2mulvs, c2Mulvs);
binary_vs!(b06_c2div, c2Div);

#[test]
fn b02_c2dot() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x4001);
    let mut d = Diff::new();
    for i in 0..N {
        let (a, b) = match i % 4 {
            0 => (rng.vec_nice(), rng.vec_nice()),
            1 => (rng.vec_hostile(), rng.vec_nice()),
            2 => (rng.vec_nice(), rng.vec_hostile()),
            _ => (rng.vec_hostile(), rng.vec_hostile()),
        };
        let x = (c.c2Dot)(a, b);
        let y = (r.c2Dot)(a, b);
        d.check(f_eq(x, y), || {
            format!(
                "c2Dot({}, {}): C={} RUST={}",
                fmt_v(a),
                fmt_v(b),
                fmt_f(x),
                fmt_f(y)
            )
        });
    }
    d.finish("c2Dot");
}

#[test]
fn b03_c2len() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x4002);
    let mut d = Diff::new();
    for i in 0..N {
        let a = if i % 2 == 0 {
            rng.vec_nice()
        } else {
            rng.vec_hostile()
        };
        let x = (c.c2Len)(a);
        let y = (r.c2Len)(a);
        d.check(f_eq(x, y), || {
            format!("c2Len({}): C={} RUST={}", fmt_v(a), fmt_f(x), fmt_f(y))
        });
    }
    d.finish("c2Len");
}

#[test]
fn b10_c2mulmvt() {
    let (c, r) = apis();
    let mut rng = Rng::new(0x4003);
    let mut d = Diff::new();
    for i in 0..N {
        let (m, v) = match i % 4 {
            0 => (
                c2m {
                    x: rng.vec_nice(),
                    y: rng.vec_nice(),
                },
                rng.vec_nice(),
            ),
            1 => (
                c2m {
                    x: rng.vec_hostile(),
                    y: rng.vec_nice(),
                },
                rng.vec_nice(),
            ),
            2 => (
                c2m {
                    x: rng.vec_nice(),
                    y: rng.vec_hostile(),
                },
                rng.vec_hostile(),
            ),
            _ => (
                c2m {
                    x: rng.vec_hostile(),
                    y: rng.vec_hostile(),
                },
                rng.vec_hostile(),
            ),
        };
        let x = (c.c2MulmvT)(m, v);
        let y = (r.c2MulmvT)(m, v);
        d.check(v_eq(x, y), || {
            format!(
                "c2MulmvT(({}, {}), {}): C={} RUST={}",
                fmt_v(m.x),
                fmt_v(m.y),
                fmt_v(v),
                fmt_v(x),
                fmt_v(y)
            )
        });
    }
    d.finish("c2MulmvT");
}

/// Hand-picked edge cases that the C's ternary `min`/`max`/`abs` macros treat
/// differently from `fminf`/`fmaxf`/`fabsf`.
#[test]
fn b11_specials_cross_product() {
    let (c, r) = apis();
    let vals = [
        0.0f32,
        -0.0f32,
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        1.0,
        -1.0,
        f32::from_bits(0x7f80_0001),
        f32::from_bits(0xffc0_1234),
    ];
    let mut d = Diff::new();
    for &ax in &vals {
        for &ay in &vals {
            for &bx in &vals {
                for &by in &vals {
                    let a = c2v { x: ax, y: ay };
                    let b = c2v { x: bx, y: by };
                    d.check(v_eq((c.c2Minv)(a, b), (r.c2Minv)(a, b)), || {
                        format!(
                            "c2Minv({}, {}): C={} RUST={}",
                            fmt_v(a),
                            fmt_v(b),
                            fmt_v((c.c2Minv)(a, b)),
                            fmt_v((r.c2Minv)(a, b))
                        )
                    });
                    d.check(v_eq((c.c2Maxv)(a, b), (r.c2Maxv)(a, b)), || {
                        format!(
                            "c2Maxv({}, {}): C={} RUST={}",
                            fmt_v(a),
                            fmt_v(b),
                            fmt_v((c.c2Maxv)(a, b)),
                            fmt_v((r.c2Maxv)(a, b))
                        )
                    });
                    d.check(f_eq((c.c2Dot)(a, b), (r.c2Dot)(a, b)), || {
                        format!(
                            "c2Dot({}, {}): C={} RUST={}",
                            fmt_v(a),
                            fmt_v(b),
                            fmt_f((c.c2Dot)(a, b)),
                            fmt_f((r.c2Dot)(a, b))
                        )
                    });
                    d.check(v_eq((c.c2Add)(a, b), (r.c2Add)(a, b)), || {
                        format!(
                            "c2Add({}, {}): C={} RUST={}",
                            fmt_v(a),
                            fmt_v(b),
                            fmt_v((c.c2Add)(a, b)),
                            fmt_v((r.c2Add)(a, b))
                        )
                    });
                    d.check(v_eq((c.c2Sub)(a, b), (r.c2Sub)(a, b)), || {
                        format!(
                            "c2Sub({}, {}): C={} RUST={}",
                            fmt_v(a),
                            fmt_v(b),
                            fmt_v((c.c2Sub)(a, b)),
                            fmt_v((r.c2Sub)(a, b))
                        )
                    });
                }
                let a = c2v { x: ax, y: ay };
                d.check(v_eq((c.c2Absv)(a), (r.c2Absv)(a)), || {
                    format!(
                        "c2Absv({}): C={} RUST={}",
                        fmt_v(a),
                        fmt_v((c.c2Absv)(a)),
                        fmt_v((r.c2Absv)(a))
                    )
                });
                d.check(v_eq((c.c2Norm)(a), (r.c2Norm)(a)), || {
                    format!(
                        "c2Norm({}): C={} RUST={}",
                        fmt_v(a),
                        fmt_v((c.c2Norm)(a)),
                        fmt_v((r.c2Norm)(a))
                    )
                });
                d.check(f_eq((c.c2Len)(a), (r.c2Len)(a)), || {
                    format!(
                        "c2Len({}): C={} RUST={}",
                        fmt_v(a),
                        fmt_f((c.c2Len)(a)),
                        fmt_f((r.c2Len)(a))
                    )
                });
                d.check(v_eq((c.c2Mulvs)(a, bx), (r.c2Mulvs)(a, bx)), || {
                    format!(
                        "c2Mulvs({}, {}): C={} RUST={}",
                        fmt_v(a),
                        fmt_f(bx),
                        fmt_v((c.c2Mulvs)(a, bx)),
                        fmt_v((r.c2Mulvs)(a, bx))
                    )
                });
                d.check(v_eq((c.c2Div)(a, bx), (r.c2Div)(a, bx)), || {
                    format!(
                        "c2Div({}, {}): C={} RUST={}",
                        fmt_v(a),
                        fmt_f(bx),
                        fmt_v((c.c2Div)(a, bx)),
                        fmt_v((r.c2Div)(a, bx))
                    )
                });
            }
        }
    }
    d.finish("signed-zero/NaN macro semantics");
}
