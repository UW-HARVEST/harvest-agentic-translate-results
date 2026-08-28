//! Level 1: the leaf vector helpers — `c2V`, `c2Sub`, `c2Dot`, `c2Minv`,
//! `c2Maxv`. Everything is compared bit-for-bit through the FFI boundary.

mod common;

use common::*;

#[test]
fn c2v_constructor() {
    let (c, r) = both();
    for &x in &interesting_f32() {
        for &y in &interesting_f32() {
            let a = unsafe { (c.c2V)(x, y) };
            let b = unsafe { (r.c2V)(x, y) };
            assert_c2v_eq("c2V", a, b, &format!("x={x:?} y={y:?}"));
        }
    }
}

#[test]
fn c2v_constructor_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::new(0xC0FFEE);
    for _ in 0..200_000 {
        let x = rng.any_f32();
        let y = rng.any_f32();
        let a = unsafe { (c.c2V)(x, y) };
        let b = unsafe { (r.c2V)(x, y) };
        assert_c2v_eq(
            "c2V",
            a,
            b,
            &format!("x=0x{:08X} y=0x{:08X}", x.to_bits(), y.to_bits()),
        );
    }
}

/// Drive a `(c2v, c2v) -> c2v` pair over the interesting cross product.
fn sweep_vv(name: &str, cf: FnVV, rf: FnVV) {
    let vals = interesting_f32();
    for (i, &ax) in vals.iter().enumerate() {
        for (j, &bx) in vals.iter().enumerate() {
            let ay = vals[(i + j) % vals.len()];
            let by = vals[(i + 2 * j + 1) % vals.len()];
            let a = c2v { x: ax, y: ay };
            let b = c2v { x: bx, y: by };
            let rc = unsafe { cf(a, b) };
            let rr = unsafe { rf(a, b) };
            assert_c2v_eq(name, rc, rr, &format!("a={a:?} b={b:?}"));
        }
    }
}

#[test]
fn c2minv_matches() {
    let (c, r) = both();
    sweep_vv("c2Minv", c.c2Minv, r.c2Minv);
}

#[test]
fn c2maxv_matches() {
    let (c, r) = both();
    sweep_vv("c2Maxv", c.c2Maxv, r.c2Maxv);
}

#[test]
fn c2sub_matches() {
    let (c, r) = both();
    sweep_vv("c2Sub", c.c2Sub, r.c2Sub);
}

#[test]
fn minmax_sub_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::new(0xDEADBEEF);
    for _ in 0..150_000 {
        let a = c2v {
            x: rng.any_f32(),
            y: rng.any_f32(),
        };
        let b = c2v {
            x: rng.any_f32(),
            y: rng.any_f32(),
        };
        let ctx = format!(
            "a=(0x{:08X},0x{:08X}) b=(0x{:08X},0x{:08X})",
            a.x.to_bits(),
            a.y.to_bits(),
            b.x.to_bits(),
            b.y.to_bits()
        );
        assert_c2v_eq(
            "c2Minv",
            unsafe { (c.c2Minv)(a, b) },
            unsafe { (r.c2Minv)(a, b) },
            &ctx,
        );
        assert_c2v_eq(
            "c2Maxv",
            unsafe { (c.c2Maxv)(a, b) },
            unsafe { (r.c2Maxv)(a, b) },
            &ctx,
        );
        assert_c2v_eq(
            "c2Sub",
            unsafe { (c.c2Sub)(a, b) },
            unsafe { (r.c2Sub)(a, b) },
            &ctx,
        );
    }
}

#[test]
fn c2dot_matches_interesting() {
    let (c, r) = both();
    let vals = interesting_f32();
    for (i, &ax) in vals.iter().enumerate() {
        for (j, &bx) in vals.iter().enumerate() {
            let a = c2v {
                x: ax,
                y: vals[(i + j) % vals.len()],
            };
            let b = c2v {
                x: bx,
                y: vals[(i * 3 + j) % vals.len()],
            };
            let rc = unsafe { (c.c2Dot)(a, b) };
            let rr = unsafe { (r.c2Dot)(a, b) };
            assert_f32_bits_eq("c2Dot", rc, rr, &format!("a={a:?} b={b:?}"));
        }
    }
}

#[test]
fn c2dot_random_bits() {
    let (c, r) = both();
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for _ in 0..200_000 {
        let a = c2v {
            x: rng.any_f32(),
            y: rng.any_f32(),
        };
        let b = c2v {
            x: rng.any_f32(),
            y: rng.any_f32(),
        };
        let rc = unsafe { (c.c2Dot)(a, b) };
        let rr = unsafe { (r.c2Dot)(a, b) };
        assert_f32_bits_eq(
            "c2Dot",
            rc,
            rr,
            &format!(
                "a=(0x{:08X},0x{:08X}) b=(0x{:08X},0x{:08X})",
                a.x.to_bits(),
                a.y.to_bits(),
                b.x.to_bits(),
                b.y.to_bits()
            ),
        );
    }
}

#[test]
fn c2dot_random_coords() {
    let (c, r) = both();
    let mut rng = Rng::new(99);
    for _ in 0..200_000 {
        let a = c2v {
            x: rng.coord(),
            y: rng.coord(),
        };
        let b = c2v {
            x: rng.coord(),
            y: rng.coord(),
        };
        assert_f32_bits_eq(
            "c2Dot",
            unsafe { (c.c2Dot)(a, b) },
            unsafe { (r.c2Dot)(a, b) },
            &format!("a={a:?} b={b:?}"),
        );
    }
}

/// `c2Dot` is where fused-multiply-add would silently change results; check
/// magnitudes that only agree when *no* fusion happens.
#[test]
fn c2dot_no_fma_contraction() {
    let (c, r) = both();
    let cases = [
        (
            c2v { x: 1.0, y: 1.0 },
            c2v {
                x: 1.0,
                y: f32::from_bits(0x3380_0000),
            },
        ),
        (
            c2v {
                x: 16777216.0,
                y: 1.0,
            },
            c2v { x: 1.0, y: 1.0 },
        ),
        (c2v { x: 1e20, y: 1.0 }, c2v { x: 1e20, y: 1.0 }),
        (c2v { x: 1e-30, y: 1e-30 }, c2v { x: 1e-30, y: 1e-30 }),
        (
            c2v {
                x: f32::MAX,
                y: f32::MAX,
            },
            c2v { x: 2.0, y: -2.0 },
        ),
    ];
    for (a, b) in cases {
        assert_f32_bits_eq(
            "c2Dot",
            unsafe { (c.c2Dot)(a, b) },
            unsafe { (r.c2Dot)(a, b) },
            &format!("a={a:?} b={b:?}"),
        );
    }
}

/// NaN payload / invalid-operation propagation through `c2Dot`, enumerated
/// case by case: only one product invalid, only one operand NaN, both NaN,
/// signalling vs quiet, and `inf + -inf` in the sum.
#[test]
fn c2dot_nan_payload_cases() {
    let (c, r) = both();
    let q1 = f32::from_bits(0x7FC0_1234); // quiet NaN, payload 1
    let q2 = f32::from_bits(0xFFD5_5555); // quiet NaN, negative, payload 2
    let s1 = f32::from_bits(0x7F80_0001); // signalling NaN
    let s2 = f32::from_bits(0xFF80_00AB); // signalling NaN, negative
    let inf = f32::INFINITY;
    let ninf = f32::NEG_INFINITY;

    let cases: Vec<(c2v, c2v)> = vec![
        // x product invalid (0 * inf), y product finite.
        (c2v { x: 0.0, y: 1.0 }, c2v { x: inf, y: 1.0 }),
        (c2v { x: -0.0, y: 1.0 }, c2v { x: inf, y: 1.0 }),
        (c2v { x: 0.0, y: 1.0 }, c2v { x: ninf, y: 1.0 }),
        // y product invalid, x product finite.
        (c2v { x: 1.0, y: 0.0 }, c2v { x: 1.0, y: inf }),
        (c2v { x: 1.0, y: inf }, c2v { x: 1.0, y: 0.0 }),
        // Both products invalid.
        (c2v { x: 0.0, y: 0.0 }, c2v { x: inf, y: ninf }),
        // Sum invalid: inf + -inf.
        (c2v { x: inf, y: inf }, c2v { x: 1.0, y: -1.0 }),
        (c2v { x: 1.0, y: -1.0 }, c2v { x: inf, y: inf }),
        // A single NaN operand in the x product (either side).
        (c2v { x: q1, y: 1.0 }, c2v { x: 2.0, y: 1.0 }),
        (c2v { x: 2.0, y: 1.0 }, c2v { x: q1, y: 1.0 }),
        (c2v { x: s1, y: 1.0 }, c2v { x: 2.0, y: 1.0 }),
        (c2v { x: 2.0, y: 1.0 }, c2v { x: s2, y: 1.0 }),
        // A single NaN operand in the y product (either side).
        (c2v { x: 1.0, y: q2 }, c2v { x: 1.0, y: 2.0 }),
        (c2v { x: 1.0, y: 2.0 }, c2v { x: 1.0, y: q2 }),
        (c2v { x: 1.0, y: s1 }, c2v { x: 1.0, y: 2.0 }),
        (c2v { x: 1.0, y: 2.0 }, c2v { x: 1.0, y: s1 }),
        // Both operands of a product are NaN.
        (c2v { x: q1, y: 1.0 }, c2v { x: q2, y: 1.0 }),
        (c2v { x: 1.0, y: q1 }, c2v { x: 1.0, y: q2 }),
        (c2v { x: s1, y: s2 }, c2v { x: q1, y: q2 }),
        (c2v { x: q1, y: q2 }, c2v { x: s1, y: s2 }),
        // NaN in one product, invalid op in the other.
        (c2v { x: 0.0, y: q1 }, c2v { x: inf, y: 1.0 }),
        (c2v { x: q1, y: 0.0 }, c2v { x: 1.0, y: inf }),
        // NaN times zero / infinity.
        (c2v { x: q1, y: 1.0 }, c2v { x: 0.0, y: 1.0 }),
        (c2v { x: 0.0, y: 1.0 }, c2v { x: q1, y: 1.0 }),
        (c2v { x: q1, y: 1.0 }, c2v { x: inf, y: 1.0 }),
        // Sum of a NaN product with a finite one, both orders.
        (c2v { x: q1, y: 3.0 }, c2v { x: 1.0, y: 4.0 }),
        (c2v { x: 3.0, y: q1 }, c2v { x: 4.0, y: 1.0 }),
    ];

    for (a, b) in cases {
        assert_f32_bits_eq(
            "c2Dot",
            unsafe { (c.c2Dot)(a, b) },
            unsafe { (r.c2Dot)(a, b) },
            &format!(
                "a=(0x{:08X},0x{:08X}) b=(0x{:08X},0x{:08X})",
                a.x.to_bits(),
                a.y.to_bits(),
                b.x.to_bits(),
                b.y.to_bits()
            ),
        );
    }
}

/// Randomised sweep that mixes NaNs/infinities with ordinary coordinates, so
/// the payload rules are hit from many directions.
#[test]
fn c2dot_mixed_special_random() {
    let (c, r) = both();
    let mut rng = Rng::new(0x5A5A_1234);
    let specials = [
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0xFFD5_5555),
        f32::from_bits(0x7F80_0001),
        f32::from_bits(0xFF80_00AB),
        f32::INFINITY,
        f32::NEG_INFINITY,
        0.0,
        -0.0,
        1.0,
        f32::MAX,
    ];
    let pick = |rng: &mut Rng| -> f32 {
        if rng.next_u32().is_multiple_of(2) {
            specials[(rng.next_u32() as usize) % specials.len()]
        } else {
            rng.coord()
        }
    };
    for _ in 0..200_000 {
        let a = c2v {
            x: pick(&mut rng),
            y: pick(&mut rng),
        };
        let b = c2v {
            x: pick(&mut rng),
            y: pick(&mut rng),
        };
        assert_f32_bits_eq(
            "c2Dot",
            unsafe { (c.c2Dot)(a, b) },
            unsafe { (r.c2Dot)(a, b) },
            &format!(
                "a=(0x{:08X},0x{:08X}) b=(0x{:08X},0x{:08X})",
                a.x.to_bits(),
                a.y.to_bits(),
                b.x.to_bits(),
                b.y.to_bits()
            ),
        );
    }
}
