//! Phase B — differential tests for the leaf vector-maths entry points.
//! CONFIGS.md rows 1–19.

mod common;
use common::*;

const N: usize = 4000;

macro_rules! wild_cases {
    ($rng:expr, $n:expr) => {
        (0..$n)
    };
}

#[test]
fn row01_c2V() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 1);
    for i in wild_cases!(rng, N) {
        let (x, y) = (rng.wild(), rng.wild());
        assert_veq(&format!("row01 c2V #{i} ({x:?},{y:?})"), (c.c2V)(x, y), (r.c2V)(x, y));
    }
}

#[test]
fn row02_c2Mulvs() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..N {
        let a = rng.wild_v();
        let b = rng.wild();
        assert_veq(
            &format!("row02 c2Mulvs #{i} a={a:?} b={b:?}"),
            (c.c2Mulvs)(a, b),
            (r.c2Mulvs)(a, b),
        );
    }
}

#[test]
fn row03_c2Maxv() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..N {
        let (a, b) = (rng.wild_v(), rng.wild_v());
        assert_veq(
            &format!("row03 c2Maxv #{i} a={a:?} b={b:?}"),
            (c.c2Maxv)(a, b),
            (r.c2Maxv)(a, b),
        );
    }
    // NaN in each of the four component slots: `a>b ? a : b` must yield b.
    for slot in 0..4 {
        let mut a = c2v { x: 1.0, y: 2.0 };
        let mut b = c2v { x: 3.0, y: 4.0 };
        match slot {
            0 => a.x = f32::NAN,
            1 => a.y = f32::NAN,
            2 => b.x = f32::NAN,
            _ => b.y = f32::NAN,
        }
        assert_veq(
            &format!("row03 c2Maxv nan slot {slot}"),
            (c.c2Maxv)(a, b),
            (r.c2Maxv)(a, b),
        );
    }
}

#[test]
fn row04_c2Minv() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..N {
        let (a, b) = (rng.wild_v(), rng.wild_v());
        assert_veq(
            &format!("row04 c2Minv #{i} a={a:?} b={b:?}"),
            (c.c2Minv)(a, b),
            (r.c2Minv)(a, b),
        );
    }
    for slot in 0..4 {
        let mut a = c2v { x: 1.0, y: 2.0 };
        let mut b = c2v { x: 3.0, y: 4.0 };
        match slot {
            0 => a.x = f32::NAN,
            1 => a.y = f32::NAN,
            2 => b.x = f32::NAN,
            _ => b.y = f32::NAN,
        }
        assert_veq(
            &format!("row04 c2Minv nan slot {slot}"),
            (c.c2Minv)(a, b),
            (r.c2Minv)(a, b),
        );
    }
}

#[test]
fn row05_c2Clampv() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 5);
    for i in 0..N {
        let a = rng.wild_v();
        let (p, q) = (rng.wild_v(), rng.wild_v());
        // mixture of proper, inverted and equal ranges
        let (lo, hi) = match i % 3 {
            0 => (
                c2v {
                    x: p.x.min(q.x),
                    y: p.y.min(q.y),
                },
                c2v {
                    x: p.x.max(q.x),
                    y: p.y.max(q.y),
                },
            ),
            1 => (q, p), // possibly inverted
            _ => (p, p), // lo == hi
        };
        assert_veq(
            &format!("row05 c2Clampv #{i} a={a:?} lo={lo:?} hi={hi:?}"),
            (c.c2Clampv)(a, lo, hi),
            (r.c2Clampv)(a, lo, hi),
        );
    }
}

#[test]
fn row06_c2Sub_row15_c2Add() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..N {
        let (a, b) = (rng.wild_v(), rng.wild_v());
        assert_veq(&format!("row06 c2Sub #{i}"), (c.c2Sub)(a, b), (r.c2Sub)(a, b));
        assert_veq(&format!("row15 c2Add #{i}"), (c.c2Add)(a, b), (r.c2Add)(a, b));
    }
    // inf - inf, inf + (-inf), ±0 combinations
    let specials = [0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY];
    for &x1 in &specials {
        for &y1 in &specials {
            for &x2 in &specials {
                for &y2 in &specials {
                    let a = c2v { x: x1, y: y1 };
                    let b = c2v { x: x2, y: y2 };
                    assert_veq("row06 c2Sub special", (c.c2Sub)(a, b), (r.c2Sub)(a, b));
                    assert_veq("row15 c2Add special", (c.c2Add)(a, b), (r.c2Add)(a, b));
                }
            }
        }
    }
}

#[test]
fn row07_c2Dot_row08_c2Det2() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 7);
    for i in 0..N {
        let (a, b) = (rng.wild_v(), rng.wild_v());
        assert_feq(&format!("row07 c2Dot #{i} {a:?} {b:?}"), (c.c2Dot)(a, b), (r.c2Dot)(a, b));
        assert_feq(&format!("row08 c2Det2 #{i} {a:?} {b:?}"), (c.c2Det2)(a, b), (r.c2Det2)(a, b));
    }
    // collinear pairs => det exactly 0, and overflow pairs
    for i in 0..N {
        let a = rng.coord_v();
        let k = rng.coord();
        let b = c2v { x: a.x * k, y: a.y * k };
        assert_feq(&format!("row08 collinear #{i}"), (c.c2Det2)(a, b), (r.c2Det2)(a, b));
        let big = c2v {
            x: f32::MAX * 0.5,
            y: f32::MAX * 0.5,
        };
        assert_feq("row08 overflow", (c.c2Det2)(big, a), (r.c2Det2)(big, a));
        assert_feq("row07 overflow", (c.c2Dot)(big, big), (r.c2Dot)(big, big));
    }
}

#[test]
fn row09_c2Len() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..N {
        let a = rng.wild_v();
        assert_feq(&format!("row09 c2Len #{i} {a:?}"), (c.c2Len)(a), (r.c2Len)(a));
    }
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::INFINITY, y: 1.0 },
        c2v { x: f32::NAN, y: 1.0 },
        c2v { x: f32::MIN_POSITIVE, y: 0.0 },
    ] {
        assert_feq(&format!("row09 c2Len special {a:?}"), (c.c2Len)(a), (r.c2Len)(a));
    }
}

#[test]
fn row10_c2Neg_row11_c2Skew_row12_c2CCW90() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..N {
        let a = rng.wild_v();
        assert_veq(&format!("row10 c2Neg #{i} {a:?}"), (c.c2Neg)(a), (r.c2Neg)(a));
        assert_veq(&format!("row11 c2Skew #{i} {a:?}"), (c.c2Skew)(a), (r.c2Skew)(a));
        assert_veq(&format!("row12 c2CCW90 #{i} {a:?}"), (c.c2CCW90)(a), (r.c2CCW90)(a));
    }
    // signed-zero round trip
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: 0.0 },
        c2v { x: 0.0, y: -0.0 },
        c2v { x: -0.0, y: -0.0 },
    ] {
        assert_veq("row10 c2Neg signed zero", (c.c2Neg)(a), (r.c2Neg)(a));
        assert_veq("row11 c2Skew signed zero", (c.c2Skew)(a), (r.c2Skew)(a));
        assert_veq("row12 c2CCW90 signed zero", (c.c2CCW90)(a), (r.c2CCW90)(a));
    }
}

#[test]
fn row13_c2Div() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 13);
    for i in 0..N {
        let a = rng.wild_v();
        let b = rng.wild();
        assert_veq(
            &format!("row13 c2Div #{i} a={a:?} b={b:?}"),
            (c.c2Div)(a, b),
            (r.c2Div)(a, b),
        );
    }
    for b in [
        0.0f32,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MIN_POSITIVE,
        f32::MAX,
    ] {
        for a in [
            c2v { x: 1.0, y: -1.0 },
            c2v { x: 0.0, y: -0.0 },
            c2v { x: f32::MAX, y: f32::MIN },
        ] {
            assert_veq(
                &format!("row13 c2Div special a={a:?} b={b:?}"),
                (c.c2Div)(a, b),
                (r.c2Div)(a, b),
            );
        }
    }
}

#[test]
fn row14_c2Norm() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..N {
        let a = rng.wild_v();
        assert_veq(&format!("row14 c2Norm #{i} {a:?}"), (c.c2Norm)(a), (r.c2Norm)(a));
    }
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: f32::MAX, y: f32::MAX },
        c2v { x: f32::MIN_POSITIVE, y: 0.0 },
        c2v { x: f32::from_bits(1), y: 0.0 },
        c2v { x: f32::INFINITY, y: f32::INFINITY },
    ] {
        assert_veq(&format!("row14 c2Norm special {a:?}"), (c.c2Norm)(a), (r.c2Norm)(a));
    }
}

#[test]
fn row16_identities() {
    let (c, r) = (&libs().c, &libs().r);
    assert!(
        req((c.c2RotIdentity)(), (r.c2RotIdentity)()),
        "row16 c2RotIdentity: C={:?} Rust={:?}",
        (c.c2RotIdentity)(),
        (r.c2RotIdentity)()
    );
    assert!(
        xeq((c.c2xIdentity)(), (r.c2xIdentity)()),
        "row16 c2xIdentity: C={:?} Rust={:?}",
        (c.c2xIdentity)(),
        (r.c2xIdentity)()
    );
    // The C source spells FLT_MAX / FLT_EPSILON out as literals; make sure the
    // Rust constants round to the same f32 by driving them through c2GJK-visible
    // behaviour is indirect, so assert the literals here instead.
    assert_eq!(f32::MAX.to_bits(), 3.402_823_466_385_288_6e38f32.to_bits());
    assert_eq!(
        1.192_092_9e-7f32.to_bits(),
        1.192_092_895_507_812_5e-7f32.to_bits()
    );
}

#[test]
fn row17_c2Mulrv_row18_c2MulrvT() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..N {
        let rot = rng.wild_r();
        let v = rng.wild_v();
        assert_veq(
            &format!("row17 c2Mulrv #{i} r={rot:?} v={v:?}"),
            (c.c2Mulrv)(rot, v),
            (r.c2Mulrv)(rot, v),
        );
        assert_veq(
            &format!("row18 c2MulrvT #{i} r={rot:?} v={v:?}"),
            (c.c2MulrvT)(rot, v),
            (r.c2MulrvT)(rot, v),
        );
    }
}

#[test]
fn row19_c2Mulxv() {
    let (c, r) = (&libs().c, &libs().r);
    let mut rng = Rng::new(SEED ^ 19);
    for i in 0..N {
        let x = rng.wild_x();
        let v = rng.wild_v();
        assert_veq(
            &format!("row19 c2Mulxv #{i} x={x:?} v={v:?}"),
            (c.c2Mulxv)(x, v),
            (r.c2Mulxv)(x, v),
        );
    }
    let ident = (c.c2xIdentity)();
    for i in 0..N {
        let v = rng.wild_v();
        assert_veq(
            &format!("row19 c2Mulxv identity #{i}"),
            (c.c2Mulxv)(ident, v),
            (r.c2Mulxv)(ident, v),
        );
    }
}
