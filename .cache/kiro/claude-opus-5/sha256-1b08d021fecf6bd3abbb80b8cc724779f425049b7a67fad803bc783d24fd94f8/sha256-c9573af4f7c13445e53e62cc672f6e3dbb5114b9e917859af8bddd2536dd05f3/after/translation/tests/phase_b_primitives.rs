//! Phase B rows 1–17: the low-level vector/scalar primitives.
//!
//! These are the bottom of the call hierarchy, so every divergence here would
//! show up (obscured) in every higher-level function. Each row is driven with
//! many randomized inputs from a fixed seed, mixing uniform finite values with
//! the special float classes (`±0`, subnormal, `±inf`, `NaN` with payloads).

mod common;
use common::*;

const N: usize = 60_000;

#[test]
fn row01_c2V() {
    let (c, r) = pair();
    let mut g = Rng::new(0x0101);
    let mut d = Diff::new("1: c2V");
    for _ in 0..N {
        let (x, y) = (g.mixed_f32(1e6), g.mixed_f32(1e6));
        let cv = unsafe { (c.c2V)(x, y) };
        let rv = unsafe { (r.c2V)(x, y) };
        d.v_bits(
            || format!("c2V({:#010x},{:#010x})", x.to_bits(), y.to_bits()),
            cv,
            rv,
        );
    }
    // Exhaustive over the special table cross-product.
    let mut g2 = Rng::new(0x0102);
    for _ in 0..4000 {
        let (x, y) = (g2.special_f32(), g2.special_f32());
        d.v_bits(
            || format!("c2V special({:#010x},{:#010x})", x.to_bits(), y.to_bits()),
            unsafe { (c.c2V)(x, y) },
            unsafe { (r.c2V)(x, y) },
        );
    }
    d.finish();
}

#[test]
fn row02_c2Dot_finite() {
    let (c, r) = pair();
    let mut g = Rng::new(0x0201);
    let mut d = Diff::new("2: c2Dot finite, magnitudes 1e-30..1e30");
    for i in 0..N {
        // Sweep the magnitude so products underflow, are normal, and overflow.
        let m = [1e-30f32, 1e-10, 1.0, 1e10, 1e30, 1e19][i % 6];
        let a = g.v(m);
        let b = g.v(m);
        d.f32_bits(
            || format!("c2Dot({},{})", fv(a), fv(b)),
            unsafe { (c.c2Dot)(a, b) },
            unsafe { (r.c2Dot)(a, b) },
        );
    }
    d.finish();
}

#[test]
fn row03_c2Dot_specials() {
    let (c, r) = pair();
    let mut g = Rng::new(0x0301);
    let mut d = Diff::new("3: c2Dot special classes / NaN payload order");
    for _ in 0..N {
        let a = g.v_special();
        let b = g.v_special();
        d.f32_bits(
            || format!("c2Dot({},{})", fv(a), fv(b)),
            unsafe { (c.c2Dot)(a, b) },
            unsafe { (r.c2Dot)(a, b) },
        );
    }
    // Fully random bit patterns, which is where distinct NaN payloads collide.
    let mut g2 = Rng::new(0x0302);
    for _ in 0..N {
        let a = c2v {
            x: g2.any_bits_f32(),
            y: g2.any_bits_f32(),
        };
        let b = c2v {
            x: g2.any_bits_f32(),
            y: g2.any_bits_f32(),
        };
        d.f32_bits(
            || format!("c2Dot random-bits({},{})", fv(a), fv(b)),
            unsafe { (c.c2Dot)(a, b) },
            unsafe { (r.c2Dot)(a, b) },
        );
    }
    d.finish();
}

#[test]
fn row04_c2Len() {
    let (c, r) = pair();
    let mut g = Rng::new(0x0401);
    let mut d = Diff::new("4: c2Len (libm sqrtf vs sqrtss)");
    for i in 0..N {
        let m = [1e-20f32, 1.0, 1e10, 1e19, 1e30, f32::MAX][i % 6];
        let a = g.v(m);
        d.f32_bits(
            || format!("c2Len({})", fv(a)),
            unsafe { (c.c2Len)(a) },
            unsafe { (r.c2Len)(a) },
        );
    }
    let mut g2 = Rng::new(0x0402);
    for _ in 0..N {
        let a = if g2.below(2) == 0 {
            g2.v_special()
        } else {
            c2v {
                x: g2.any_bits_f32(),
                y: g2.any_bits_f32(),
            }
        };
        d.f32_bits(
            || format!("c2Len({})", fv(a)),
            unsafe { (c.c2Len)(a) },
            unsafe { (r.c2Len)(a) },
        );
    }
    // Explicit zero / -0 cases.
    for a in [
        c2v { x: 0.0, y: 0.0 },
        c2v { x: -0.0, y: -0.0 },
        c2v { x: -0.0, y: 0.0 },
    ] {
        d.f32_bits(
            || format!("c2Len({})", fv(a)),
            unsafe { (c.c2Len)(a) },
            unsafe { (r.c2Len)(a) },
        );
    }
    d.finish();
}

macro_rules! vvv_row {
    ($name:ident, $row:literal, $field:ident, $seed:literal) => {
        #[test]
        fn $name() {
            let (c, r) = pair();
            let mut g = Rng::new($seed);
            let mut d = Diff::new($row);
            for i in 0..N {
                let m = [1e-30f32, 1.0, 1e10, 1e38, 1e19, 1e-38][i % 6];
                let a = g.v(m);
                let b = g.v(m);
                d.v_bits(
                    || format!("{}({},{})", $row, fv(a), fv(b)),
                    unsafe { (c.$field)(a, b) },
                    unsafe { (r.$field)(a, b) },
                );
            }
            let mut g2 = Rng::new($seed + 1);
            for _ in 0..N {
                let (a, b) = match g2.below(3) {
                    0 => (g2.v_special(), g2.v_special()),
                    1 => (
                        c2v {
                            x: g2.any_bits_f32(),
                            y: g2.any_bits_f32(),
                        },
                        c2v {
                            x: g2.any_bits_f32(),
                            y: g2.any_bits_f32(),
                        },
                    ),
                    _ => (g2.v_mixed(1e3), g2.v_mixed(1e3)),
                };
                d.v_bits(
                    || format!("{}({},{})", $row, fv(a), fv(b)),
                    unsafe { (c.$field)(a, b) },
                    unsafe { (r.$field)(a, b) },
                );
            }
            // ties and signed zeros, explicitly
            for (a, b) in [
                (c2v { x: 0.0, y: 0.0 }, c2v { x: -0.0, y: -0.0 }),
                (c2v { x: -0.0, y: -0.0 }, c2v { x: 0.0, y: 0.0 }),
                (c2v { x: 1.0, y: 2.0 }, c2v { x: 1.0, y: 2.0 }),
                (
                    c2v { x: f32::NAN, y: 1.0 },
                    c2v { x: 1.0, y: f32::NAN },
                ),
                (
                    c2v { x: f32::INFINITY, y: f32::NEG_INFINITY },
                    c2v { x: f32::NEG_INFINITY, y: f32::INFINITY },
                ),
            ] {
                d.v_bits(
                    || format!("{} edge({},{})", $row, fv(a), fv(b)),
                    unsafe { (c.$field)(a, b) },
                    unsafe { (r.$field)(a, b) },
                );
            }
            d.finish();
        }
    };
}

vvv_row!(row05_c2Add, "5: c2Add", c2Add, 0x0501);
vvv_row!(row06_c2Sub, "6: c2Sub", c2Sub, 0x0601);
vvv_row!(row12_c2Minv, "12: c2Minv (ternary <)", c2Minv, 0x1201);
vvv_row!(row13_c2Maxv, "13: c2Maxv (ternary >)", c2Maxv, 0x1301);

macro_rules! vf_row {
    ($name:ident, $row:literal, $field:ident, $seed:literal) => {
        #[test]
        fn $name() {
            let (c, r) = pair();
            let mut g = Rng::new($seed);
            let mut d = Diff::new($row);
            for i in 0..N {
                let m = [1e-30f32, 1.0, 1e10, 1e38, 1e19][i % 5];
                let a = g.v(m);
                let s = g.sym(m);
                d.v_bits(
                    || format!("{}({}, {:#010x})", $row, fv(a), s.to_bits()),
                    unsafe { (c.$field)(a, s) },
                    unsafe { (r.$field)(a, s) },
                );
            }
            let mut g2 = Rng::new($seed + 1);
            for _ in 0..N {
                let a = match g2.below(3) {
                    0 => g2.v_special(),
                    1 => c2v {
                        x: g2.any_bits_f32(),
                        y: g2.any_bits_f32(),
                    },
                    _ => g2.v_mixed(1e3),
                };
                let s = match g2.below(3) {
                    0 => g2.special_f32(),
                    1 => g2.any_bits_f32(),
                    _ => g2.mixed_f32(1e3),
                };
                d.v_bits(
                    || format!("{}({}, {:#010x})", $row, fv(a), s.to_bits()),
                    unsafe { (c.$field)(a, s) },
                    unsafe { (r.$field)(a, s) },
                );
            }
            for s in [
                0.0f32,
                -0.0,
                1.0,
                -1.0,
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NAN,
                -f32::NAN,
                f32::MIN_POSITIVE,
                f32::from_bits(1),
                f32::MAX,
                f32::MIN,
                3.0,
                7.0,
            ] {
                for a in [
                    c2v { x: 1.0, y: -1.0 },
                    c2v { x: 0.0, y: -0.0 },
                    c2v { x: f32::INFINITY, y: f32::NAN },
                    c2v { x: 1e30, y: -1e-30 },
                ] {
                    d.v_bits(
                        || format!("{} edge({}, {:#010x})", $row, fv(a), s.to_bits()),
                        unsafe { (c.$field)(a, s) },
                        unsafe { (r.$field)(a, s) },
                    );
                }
            }
            d.finish();
        }
    };
}

vf_row!(row07_c2Mulvs, "7: c2Mulvs", c2Mulvs, 0x0701);
vf_row!(row08_09_c2Div, "8/9: c2Div (reciprocal-multiply)", c2Div, 0x0801);

macro_rules! vv_row {
    ($name:ident, $row:literal, $field:ident, $seed:literal) => {
        #[test]
        fn $name() {
            let (c, r) = pair();
            let mut g = Rng::new($seed);
            let mut d = Diff::new($row);
            for i in 0..N {
                let m = [1e-30f32, 1.0, 1e10, 1e19, 1e38][i % 5];
                let a = g.v(m);
                d.v_bits(
                    || format!("{}({})", $row, fv(a)),
                    unsafe { (c.$field)(a) },
                    unsafe { (r.$field)(a) },
                );
            }
            let mut g2 = Rng::new($seed + 1);
            for _ in 0..N {
                let a = match g2.below(3) {
                    0 => g2.v_special(),
                    1 => c2v {
                        x: g2.any_bits_f32(),
                        y: g2.any_bits_f32(),
                    },
                    _ => g2.v_mixed(1e3),
                };
                d.v_bits(
                    || format!("{}({})", $row, fv(a)),
                    unsafe { (c.$field)(a) },
                    unsafe { (r.$field)(a) },
                );
            }
            for a in [
                c2v { x: 0.0, y: 0.0 },
                c2v { x: -0.0, y: -0.0 },
                c2v { x: -0.0, y: 0.0 },
                c2v { x: 0.0, y: -0.0 },
                c2v { x: f32::NAN, y: -f32::NAN },
                c2v { x: f32::INFINITY, y: 1.0 },
                c2v { x: f32::from_bits(1), y: f32::from_bits(0x8000_0001) },
                c2v { x: f32::MAX, y: f32::MAX },
            ] {
                d.v_bits(
                    || format!("{} edge({})", $row, fv(a)),
                    unsafe { (c.$field)(a) },
                    unsafe { (r.$field)(a) },
                );
            }
            d.finish();
        }
    };
}

vv_row!(row10_11_c2Norm, "10/11: c2Norm", c2Norm, 0x1001);
vv_row!(row14_c2Skew, "14: c2Skew", c2Skew, 0x1401);
vv_row!(row15_c2Absv, "15: c2Absv (ternary abs keeps -0.0)", c2Absv, 0x1501);
vv_row!(row16_c2CCW90, "16: c2CCW90", c2CCW90, 0x1601);

#[test]
fn row17_c2MulmvT() {
    let (c, r) = pair();
    let mut g = Rng::new(0x1701);
    let mut d = Diff::new("17: c2MulmvT");
    for i in 0..N {
        let m = [1e-30f32, 1.0, 1e10, 1e19, 1e38][i % 5];
        let a = c2m {
            x: g.v(m),
            y: g.v(m),
        };
        let b = g.v(m);
        d.v_bits(
            || format!("c2MulmvT({{{},{}}},{})", fv(a.x), fv(a.y), fv(b)),
            unsafe { (c.c2MulmvT)(a, b) },
            unsafe { (r.c2MulmvT)(a, b) },
        );
    }
    let mut g2 = Rng::new(0x1702);
    for _ in 0..N {
        let pick = |g: &mut Rng| match g.below(3) {
            0 => g.v_special(),
            1 => c2v {
                x: g.any_bits_f32(),
                y: g.any_bits_f32(),
            },
            _ => g.v_mixed(1e3),
        };
        let a = c2m {
            x: pick(&mut g2),
            y: pick(&mut g2),
        };
        let b = pick(&mut g2);
        d.v_bits(
            || format!("c2MulmvT({{{},{}}},{})", fv(a.x), fv(a.y), fv(b)),
            unsafe { (c.c2MulmvT)(a, b) },
            unsafe { (r.c2MulmvT)(a, b) },
        );
    }
    d.finish();
}
