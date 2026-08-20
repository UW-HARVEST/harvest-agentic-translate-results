//! Phase B rows C19..C29: the three fractal entry points.

mod common;

use common::{Diff, Rng, SPECIAL_F32};

const CANON_LAC: f32 = 2.0;
const CANON_GAIN: f32 = 0.5;

/// C19: ridge with canonical parameters.
#[test]
fn c19_ridge_canonical() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C19 ridge canonical lac=2 gain=0.5 offset=1");
    let mut rng = Rng::new(0x19);
    for octaves in 0..=8 {
        for _ in 0..300 {
            let (x, y, z) = (rng.coord(16), rng.coord(16), rng.coord(16));
            d.check(
                format_args!("octaves={octaves} x={x:e} y={y:e} z={z:e}"),
                unsafe { (c.ridge)(x, y, z, CANON_LAC, CANON_GAIN, 1.0, octaves) },
                unsafe { (r.ridge)(x, y, z, CANON_LAC, CANON_GAIN, 1.0, octaves) },
            );
        }
    }
    d.finish();
}

/// C20: ridge offset shapes.
#[test]
fn c20_ridge_offset_shapes() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C20 ridge offset shapes");
    let offsets = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        8.0,
        1e30,
        -1e30,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::MIN_POSITIVE,
    ];
    let mut rng = Rng::new(0x20);
    for &offset in &offsets {
        for octaves in [1, 2, 6] {
            for _ in 0..60 {
                let (x, y, z) = (rng.coord(8), rng.coord(8), rng.coord(8));
                d.check(
                    format_args!(
                        "offset={:#010x} octaves={octaves} x={x:e} y={y:e} z={z:e}",
                        offset.to_bits()
                    ),
                    unsafe { (c.ridge)(x, y, z, CANON_LAC, CANON_GAIN, offset, octaves) },
                    unsafe { (r.ridge)(x, y, z, CANON_LAC, CANON_GAIN, offset, octaves) },
                );
            }
        }
    }
    d.finish();
}

/// C21: ridge with randomised lacunarity/gain.
#[test]
fn c21_ridge_random_lac_gain() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C21 ridge random lacunarity/gain");
    let mut rng = Rng::new(0x21);
    for _ in 0..6000 {
        let (x, y, z) = (rng.coord(32), rng.coord(32), rng.coord(32));
        let (lac, gain) = (rng.lac_gain(), rng.lac_gain());
        let offset = rng.lac_gain();
        let octaves = rng.range(0, 8);
        d.check(
            format_args!("lac={lac:e} gain={gain:e} offset={offset:e} octaves={octaves} x={x:e} y={y:e} z={z:e}"),
            unsafe { (c.ridge)(x, y, z, lac, gain, offset, octaves) },
            unsafe { (r.ridge)(x, y, z, lac, gain, offset, octaves) },
        );
    }
    d.finish();
}

/// C22: ridge with octave counts that overflow frequency/amplitude.
#[test]
fn c22_ridge_extreme_octaves() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C22 ridge extreme octaves");
    let mut rng = Rng::new(0x22);
    for octaves in [16, 32, 64, 130, 300] {
        for _ in 0..60 {
            let (x, y, z) = (rng.coord(8), rng.coord(8), rng.coord(8));
            let (lac, gain) = (rng.lac_gain(), rng.lac_gain());
            let offset = rng.lac_gain();
            d.check(
                format_args!(
                    "octaves={octaves} lac={lac:e} gain={gain:e} offset={offset:e} x={x:e} y={y:e} z={z:e}"
                ),
                unsafe { (c.ridge)(x, y, z, lac, gain, offset, octaves) },
                unsafe { (r.ridge)(x, y, z, lac, gain, offset, octaves) },
            );
        }
    }
    d.finish();
}

/// C23: degenerate lacunarity/gain (0, 1, huge, special floats).
#[test]
fn c23_ridge_degenerate_lac_gain() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C23 ridge degenerate lacunarity/gain");
    let vals = [
        0.0f32,
        -0.0,
        1.0,
        -1.0,
        1e30,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::MIN_POSITIVE,
        1e-45,
    ];
    let mut rng = Rng::new(0x23);
    for &lac in &vals {
        for &gain in &vals {
            for octaves in [1, 3, 6] {
                let (x, y, z) = (rng.coord(2000), rng.coord(2000), rng.coord(2000));
                let offset = *rng.pick(SPECIAL_F32);
                d.check(
                    format_args!(
                        "lac={:#010x} gain={:#010x} offset={:#010x} octaves={octaves} x={x:e} y={y:e} z={z:e}",
                        lac.to_bits(), gain.to_bits(), offset.to_bits()
                    ),
                    unsafe { (c.ridge)(x, y, z, lac, gain, offset, octaves) },
                    unsafe { (r.ridge)(x, y, z, lac, gain, offset, octaves) },
                );
            }
        }
    }
    d.finish();
}

/// C24: fbm with canonical parameters.
#[test]
fn c24_fbm_canonical() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C24 fbm canonical");
    let mut rng = Rng::new(0x24);
    for octaves in 0..=8 {
        for _ in 0..300 {
            let (x, y, z) = (rng.coord(16), rng.coord(16), rng.coord(16));
            d.check(
                format_args!("octaves={octaves} x={x:e} y={y:e} z={z:e}"),
                unsafe { (c.fbm)(x, y, z, CANON_LAC, CANON_GAIN, octaves) },
                unsafe { (r.fbm)(x, y, z, CANON_LAC, CANON_GAIN, octaves) },
            );
        }
    }
    d.finish();
}

/// C25: fbm with randomised lacunarity/gain.
#[test]
fn c25_fbm_random_lac_gain() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C25 fbm random lacunarity/gain");
    let mut rng = Rng::new(0x25);
    for _ in 0..6000 {
        let (x, y, z) = (rng.coord(64), rng.coord(64), rng.coord(64));
        let (lac, gain) = (rng.lac_gain(), rng.lac_gain());
        let octaves = rng.range(0, 8);
        d.check(
            format_args!("lac={lac:e} gain={gain:e} octaves={octaves} x={x:e} y={y:e} z={z:e}"),
            unsafe { (c.fbm)(x, y, z, lac, gain, octaves) },
            unsafe { (r.fbm)(x, y, z, lac, gain, octaves) },
        );
    }
    d.finish();
}

/// C26: fbm with extreme octaves / overflowing lacunarity.
#[test]
fn c26_fbm_extreme() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C26 fbm extreme octaves/lacunarity");
    let lacs = [1e30f32, -2.0, f32::MAX, f32::INFINITY, f32::NAN, 0.0, 1.0];
    let gains = [0.5f32, 1e30, -1.0, f32::INFINITY, f32::NAN, 0.0];
    let mut rng = Rng::new(0x26);
    for octaves in [16, 32, 64, 130, 300] {
        for &lac in &lacs {
            for &gain in &gains {
                let (x, y, z) = (rng.coord(8), rng.coord(8), rng.coord(8));
                d.check(
                    format_args!(
                        "octaves={octaves} lac={:#010x} gain={:#010x} x={x:e} y={y:e} z={z:e}",
                        lac.to_bits(),
                        gain.to_bits()
                    ),
                    unsafe { (c.fbm)(x, y, z, lac, gain, octaves) },
                    unsafe { (r.fbm)(x, y, z, lac, gain, octaves) },
                );
            }
        }
    }
    d.finish();
}

/// C27: turbulence with canonical parameters.
#[test]
fn c27_turbulence_canonical() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C27 turbulence canonical");
    let mut rng = Rng::new(0x27);
    for octaves in 0..=8 {
        for _ in 0..300 {
            let (x, y, z) = (rng.coord(16), rng.coord(16), rng.coord(16));
            d.check(
                format_args!("octaves={octaves} x={x:e} y={y:e} z={z:e}"),
                unsafe { (c.turbulence)(x, y, z, CANON_LAC, CANON_GAIN, octaves) },
                unsafe { (r.turbulence)(x, y, z, CANON_LAC, CANON_GAIN, octaves) },
            );
        }
    }
    d.finish();
}

/// C28: turbulence with randomised lacunarity/gain.
#[test]
fn c28_turbulence_random_lac_gain() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C28 turbulence random lacunarity/gain");
    let mut rng = Rng::new(0x28);
    for _ in 0..6000 {
        let (x, y, z) = (rng.coord(64), rng.coord(64), rng.coord(64));
        let (lac, gain) = (rng.lac_gain(), rng.lac_gain());
        let octaves = rng.range(0, 8);
        d.check(
            format_args!("lac={lac:e} gain={gain:e} octaves={octaves} x={x:e} y={y:e} z={z:e}"),
            unsafe { (c.turbulence)(x, y, z, lac, gain, octaves) },
            unsafe { (r.turbulence)(x, y, z, lac, gain, octaves) },
        );
    }
    d.finish();
}

/// C29: turbulence with extreme octaves and inf/NaN parameters.
#[test]
fn c29_turbulence_extreme() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("C29 turbulence extreme");
    let mut rng = Rng::new(0x29);
    for octaves in [16, 32, 64, 130, 300] {
        for _ in 0..120 {
            let f = |rng: &mut Rng| match rng.below(3) {
                0 => *rng.pick(SPECIAL_F32),
                1 => rng.lac_gain(),
                _ => rng.any_f32(),
            };
            let (x, y, z) = (f(&mut rng), f(&mut rng), f(&mut rng));
            let (lac, gain) = (f(&mut rng), f(&mut rng));
            d.check(
                format_args!(
                    "octaves={octaves} lac={:#010x} gain={:#010x} x={:#010x} y={:#010x} z={:#010x}",
                    lac.to_bits(),
                    gain.to_bits(),
                    x.to_bits(),
                    y.to_bits(),
                    z.to_bits()
                ),
                unsafe { (c.turbulence)(x, y, z, lac, gain, octaves) },
                unsafe { (r.turbulence)(x, y, z, lac, gain, octaves) },
            );
        }
    }
    d.finish();
}

/// Extra: all three fractal functions over fully random inputs (the octave
/// counter is the `unsigned char` seed, so this also sweeps seeds 0..octaves).
#[test]
fn fractal_fully_random() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("fractal fully randomised");
    let mut rng = Rng::new(0x2A);
    for _ in 0..6000 {
        let f = |rng: &mut Rng| match rng.below(4) {
            0 => rng.coord(16),
            1 => rng.finite_f32(),
            2 => *rng.pick(SPECIAL_F32),
            _ => rng.any_f32(),
        };
        let (x, y, z) = (f(&mut rng), f(&mut rng), f(&mut rng));
        let (lac, gain, offset) = (f(&mut rng), f(&mut rng), f(&mut rng));
        let octaves = rng.range(-3, 10);
        let ctx = format_args!(
            "x={:#010x} y={:#010x} z={:#010x} lac={:#010x} gain={:#010x} offset={:#010x} octaves={octaves}",
            x.to_bits(), y.to_bits(), z.to_bits(), lac.to_bits(), gain.to_bits(), offset.to_bits()
        );
        d.check(ctx, unsafe { (c.ridge)(x, y, z, lac, gain, offset, octaves) }, unsafe {
            (r.ridge)(x, y, z, lac, gain, offset, octaves)
        });
        d.check(ctx, unsafe { (c.fbm)(x, y, z, lac, gain, octaves) }, unsafe {
            (r.fbm)(x, y, z, lac, gain, octaves)
        });
        d.check(ctx, unsafe { (c.turbulence)(x, y, z, lac, gain, octaves) }, unsafe {
            (r.turbulence)(x, y, z, lac, gain, octaves)
        });
    }
    d.finish();
}
