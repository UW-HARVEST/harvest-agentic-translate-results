//! Phase B — CONFIGS.md rows 1..13: the low-level vector primitives.
//!
//! Every call goes through both `.so` files' exported symbols.

mod common;
use common::*;

const N: usize = 20_000;

// --- row 1: c2V ------------------------------------------------------------
#[test]
fn row01_c2V() {
    let l = libs();
    let mut rng = Rng::new(0x0101);
    for i in 0..N {
        let (x, y) = if i % 3 == 0 {
            (rng.any_bits(), rng.any_bits())
        } else {
            (rng.wide(20), rng.wide(20))
        };
        diff_eq!(
            format!("c2V({:#x},{:#x})", fb(x), fb(y)),
            vb((l.c.c2V)(x, y)),
            vb((l.r.c2V)(x, y))
        );
    }
    for &x in &specials_nan_payloads() {
        for &y in &specials_nan_payloads() {
            diff_eq!(
                format!("c2V special({:#x},{:#x})", fb(x), fb(y)),
                vb((l.c.c2V)(x, y)),
                vb((l.r.c2V)(x, y))
            );
        }
    }
}

// --- rows 2,3: c2Dot -------------------------------------------------------
#[test]
fn row02_c2Dot_finite() {
    let l = libs();
    let mut rng = Rng::new(0x0202);
    for _ in 0..N {
        let a = C2v {
            x: rng.wide(18),
            y: rng.wide(18),
        };
        let b = C2v {
            x: rng.wide(18),
            y: rng.wide(18),
        };
        diff_eq!(
            format!("c2Dot({a:?},{b:?})"),
            fb((l.c.c2Dot)(a, b)),
            fb((l.r.c2Dot)(a, b))
        );
    }
}

#[test]
fn row03_c2Dot_specials() {
    let l = libs();
    let s = specials_nan_payloads();
    // Full 4-way cross over the special set: exercises NaN operand ordering.
    for &ax in &s {
        for &ay in &s {
            for &bx in &s {
                for &by in &s {
                    let a = C2v { x: ax, y: ay };
                    let b = C2v { x: bx, y: by };
                    diff_eq!(
                        format!(
                            "c2Dot bits ({:#x},{:#x})·({:#x},{:#x})",
                            fb(ax),
                            fb(ay),
                            fb(bx),
                            fb(by)
                        ),
                        fb((l.c.c2Dot)(a, b)),
                        fb((l.r.c2Dot)(a, b))
                    );
                }
            }
        }
    }
    let mut rng = Rng::new(0x0303);
    for _ in 0..N {
        let a = rng.v_special(1e6);
        let b = rng.v_special(1e6);
        diff_eq!(
            format!("c2Dot rnd-special({a:?},{b:?})"),
            fb((l.c.c2Dot)(a, b)),
            fb((l.r.c2Dot)(a, b))
        );
    }
}

// --- row 4: c2Len ----------------------------------------------------------
#[test]
fn row04_c2Len() {
    let l = libs();
    let mut rng = Rng::new(0x0404);
    for i in 0..N {
        let a = match i % 3 {
            0 => rng.v(1e3),
            1 => rng.v_special(1e20),
            _ => C2v {
                x: rng.any_bits(),
                y: rng.any_bits(),
            },
        };
        diff_eq!(
            format!("c2Len({:#x},{:#x})", fb(a.x), fb(a.y)),
            fb((l.c.c2Len)(a)),
            fb((l.r.c2Len)(a))
        );
    }
    for &x in &specials_nan_payloads() {
        for &y in &specials_nan_payloads() {
            let a = C2v { x, y };
            diff_eq!(
                format!("c2Len special({:#x},{:#x})", fb(x), fb(y)),
                fb((l.c.c2Len)(a)),
                fb((l.r.c2Len)(a))
            );
        }
    }
}

// --- rows 5,6,10: binary vector ops ---------------------------------------
#[test]
fn rows05_06_10_binary_vec_ops() {
    let l = libs();
    let s = specials_nan_payloads();
    macro_rules! check {
        ($name:literal, $cf:expr, $rf:expr) => {{
            for &ax in &s {
                for &ay in &s {
                    for &bx in &s {
                        for &by in &s {
                            let a = C2v { x: ax, y: ay };
                            let b = C2v { x: bx, y: by };
                            diff_eq!(
                                format!(
                                    "{} ({:#x},{:#x}) ({:#x},{:#x})",
                                    $name,
                                    fb(ax),
                                    fb(ay),
                                    fb(bx),
                                    fb(by)
                                ),
                                vb($cf(a, b)),
                                vb($rf(a, b))
                            );
                        }
                    }
                }
            }
            let mut rng = Rng::new(0x0506);
            for i in 0..N {
                let (a, b) = if i % 2 == 0 {
                    (rng.v(1e4), rng.v(1e4))
                } else {
                    (rng.v_special(1e20), rng.v_special(1e20))
                };
                diff_eq!(
                    format!("{} rnd {:?} {:?}", $name, a, b),
                    vb($cf(a, b)),
                    vb($rf(a, b))
                );
            }
        }};
    }
    check!("c2Add", l.c.c2Add, l.r.c2Add);
    check!("c2Sub", l.c.c2Sub, l.r.c2Sub);
    check!("c2Minv", l.c.c2Minv, l.r.c2Minv);
    check!("c2Maxv", l.c.c2Maxv, l.r.c2Maxv);
}

// --- rows 7,8: c2Mulvs / c2Div --------------------------------------------
#[test]
fn rows07_08_scalar_vec_ops() {
    let l = libs();
    let s = specials_nan_payloads();
    macro_rules! check {
        ($name:literal, $cf:expr, $rf:expr) => {{
            for &ax in &s {
                for &ay in &s {
                    for &b in &s {
                        let a = C2v { x: ax, y: ay };
                        diff_eq!(
                            format!(
                                "{} ({:#x},{:#x}) * {:#x}",
                                $name,
                                fb(ax),
                                fb(ay),
                                fb(b)
                            ),
                            vb($cf(a, b)),
                            vb($rf(a, b))
                        );
                    }
                }
            }
            let mut rng = Rng::new(0x0708);
            for i in 0..N {
                let a = if i % 2 == 0 {
                    rng.v(1e4)
                } else {
                    rng.v_special(1e20)
                };
                let b = if i % 3 == 0 {
                    rng.special(1e10)
                } else {
                    rng.wide(20)
                };
                diff_eq!(
                    format!("{} rnd {:?} * {:#x}", $name, a, fb(b)),
                    vb($cf(a, b)),
                    vb($rf(a, b))
                );
            }
        }};
    }
    check!("c2Mulvs", l.c.c2Mulvs, l.r.c2Mulvs);
    check!("c2Div", l.c.c2Div, l.r.c2Div);
}

// --- rows 9,11,12: unary vector ops ---------------------------------------
#[test]
fn rows09_11_12_unary_vec_ops() {
    let l = libs();
    let s = specials_nan_payloads();
    macro_rules! check {
        ($name:literal, $cf:expr, $rf:expr) => {{
            for &x in &s {
                for &y in &s {
                    let a = C2v { x, y };
                    diff_eq!(
                        format!("{} ({:#x},{:#x})", $name, fb(x), fb(y)),
                        vb($cf(a)),
                        vb($rf(a))
                    );
                }
            }
            // full-bit-space fuzz
            let mut rng = Rng::new(0x0912);
            for i in 0..N {
                let a = match i % 3 {
                    0 => rng.v(1e3),
                    1 => rng.v_special(1e20),
                    _ => C2v {
                        x: rng.any_bits(),
                        y: rng.any_bits(),
                    },
                };
                diff_eq!(
                    format!("{} rnd ({:#x},{:#x})", $name, fb(a.x), fb(a.y)),
                    vb($cf(a)),
                    vb($rf(a))
                );
            }
        }};
    }
    check!("c2Norm", l.c.c2Norm, l.r.c2Norm);
    check!("c2Skew", l.c.c2Skew, l.r.c2Skew);
    check!("c2CCW90", l.c.c2CCW90, l.r.c2CCW90);
    check!("c2Absv", l.c.c2Absv, l.r.c2Absv);
}

// --- row 13: c2MulmvT ------------------------------------------------------
#[test]
fn row13_c2MulmvT() {
    let l = libs();
    let mut rng = Rng::new(0x1313);
    // random matrices / vectors, including the all-NaN basis a degenerate
    // capsule produces.
    for i in 0..N {
        let (m, v) = match i % 4 {
            0 => (
                C2m {
                    x: rng.v(1e3),
                    y: rng.v(1e3),
                },
                rng.v(1e3),
            ),
            1 => {
                let d = rng.unit_dir();
                (
                    C2m {
                        x: C2v { x: d.y, y: -d.x },
                        y: d,
                    },
                    rng.v(1e3),
                )
            }
            2 => (
                C2m {
                    x: rng.v_special(1e10),
                    y: rng.v_special(1e10),
                },
                rng.v_special(1e10),
            ),
            _ => (
                C2m {
                    x: C2v {
                        x: f32::NAN,
                        y: f32::NAN,
                    },
                    y: C2v {
                        x: f32::NAN,
                        y: f32::NAN,
                    },
                },
                rng.v_special(1e3),
            ),
        };
        diff_eq!(
            format!("c2MulmvT({m:?},{v:?})"),
            vb((l.c.c2MulmvT)(m, v)),
            vb((l.r.c2MulmvT)(m, v))
        );
    }
    // small exhaustive cross on the specials for each of the 5 scalars would be
    // 19^5; sample the corners of that space instead with a dense sub-cross.
    let s: Vec<f32> = specials_nan_payloads();
    for &mxx in &s {
        for &myy in &s {
            for &vx in &s {
                for &vy in &s {
                    let m = C2m {
                        x: C2v { x: mxx, y: 1.0 },
                        y: C2v { x: -1.0, y: myy },
                    };
                    let v = C2v { x: vx, y: vy };
                    diff_eq!(
                        format!("c2MulmvT cross {m:?} {v:?}"),
                        vb((l.c.c2MulmvT)(m, v)),
                        vb((l.r.c2MulmvT)(m, v))
                    );
                }
            }
        }
    }
}
