//! Phase B rows C38..C40: the `inner` dispatcher of `c_src/src/main.c`.

mod common;

use common::{classify_nonpow2, Diff, Nonpow2Class, Rng, SPECIAL_F32, SPECIAL_INTS, SPECIAL_WRAPS};

#[allow(clippy::too_many_arguments)]
fn compare(
    d: &mut Diff,
    which: i32,
    x: f32,
    y: f32,
    z: f32,
    xw: i32,
    yw: i32,
    zw: i32,
    seed: i32,
    lac: f32,
    gain: f32,
    offset: f32,
    octaves: i32,
) {
    // `which == 5` forwards to the only function that can read out of bounds.
    if which == 5
        && classify_nonpow2(x, y, z, xw, yw, zw, seed as u8) != Nonpow2Class::Reproducible
    {
        return;
    }
    let (c, r) = (common::c_api(), common::rust_api());
    d.check(
        format_args!(
            "which={which} x={:#010x} y={:#010x} z={:#010x} wraps=({xw},{yw},{zw}) seed={seed} lac={:#010x} gain={:#010x} offset={:#010x} octaves={octaves}",
            x.to_bits(), y.to_bits(), z.to_bits(), lac.to_bits(), gain.to_bits(), offset.to_bits()
        ),
        unsafe { (c.inner)(which, x, y, z, xw, yw, zw, seed, lac, gain, offset, octaves) },
        unsafe { (r.inner)(which, x, y, z, xw, yw, zw, seed, lac, gain, offset, octaves) },
    );
}

/// C38: every `which` case with the arguments that case actually forwards.
#[test]
fn c38_inner_each_case() {
    let mut d = Diff::new("C38 inner which=0..=5");
    let mut rng = Rng::new(0x38);
    for which in 0..=5 {
        for _ in 0..3000 {
            let (x, y, z) = (rng.coord(64), rng.coord(64), rng.coord(64));
            let (xw, yw, zw) = match which {
                5 => (rng.range(0, 256), rng.range(0, 256), rng.range(0, 256)),
                _ => (
                    *rng.pick(SPECIAL_WRAPS),
                    *rng.pick(SPECIAL_WRAPS),
                    *rng.pick(SPECIAL_WRAPS),
                ),
            };
            let seed = rng.next_i32();
            let (lac, gain, offset) = (rng.lac_gain(), rng.lac_gain(), rng.lac_gain());
            let octaves = rng.range(0, 8);
            compare(
                &mut d, which, x, y, z, xw, yw, zw, seed, lac, gain, offset, octaves,
            );
        }
    }
    d.finish();
}

/// C39: all twelve arguments randomised at once.
#[test]
fn c39_inner_random_all_args() {
    let mut d = Diff::new("C39 inner all arguments randomised");
    let mut rng = Rng::new(0x39);
    for _ in 0..20000 {
        let which = rng.range(0, 5);
        let f = |rng: &mut Rng| match rng.below(4) {
            0 => rng.coord(32),
            1 => rng.finite_f32(),
            2 => *rng.pick(SPECIAL_F32),
            _ => rng.any_f32(),
        };
        let (x, y, z) = (f(&mut rng), f(&mut rng), f(&mut rng));
        let w = |rng: &mut Rng| match rng.below(3) {
            0 => *rng.pick(SPECIAL_WRAPS),
            1 => rng.range(-512, 512),
            _ => rng.next_i32(),
        };
        let (xw, yw, zw) = (w(&mut rng), w(&mut rng), w(&mut rng));
        let seed = *rng.pick(SPECIAL_INTS);
        let (lac, gain, offset) = (f(&mut rng), f(&mut rng), f(&mut rng));
        let octaves = rng.range(-2, 9);
        compare(
            &mut d, which, x, y, z, xw, yw, zw, seed, lac, gain, offset, octaves,
        );
    }
    d.finish();
}

/// C40: every `which` crossed with special floats and wrap/seed extremes.
#[test]
fn c40_inner_special_floats() {
    let mut d = Diff::new("C40 inner special floats");
    let mut rng = Rng::new(0x40);
    for which in 0..=5 {
        for &x in SPECIAL_F32 {
            for &y in SPECIAL_F32 {
                let z = *rng.pick(SPECIAL_F32);
                let (xw, yw, zw) = match which {
                    5 => (0, rng.range(1, 256), rng.range(1, 256)),
                    _ => (
                        *rng.pick(SPECIAL_WRAPS),
                        *rng.pick(SPECIAL_WRAPS),
                        *rng.pick(SPECIAL_WRAPS),
                    ),
                };
                let seed = *rng.pick(SPECIAL_INTS);
                let (lac, gain, offset) = (
                    *rng.pick(SPECIAL_F32),
                    *rng.pick(SPECIAL_F32),
                    *rng.pick(SPECIAL_F32),
                );
                let octaves = rng.range(0, 6);
                compare(
                    &mut d, which, x, y, z, xw, yw, zw, seed, lac, gain, offset, octaves,
                );
            }
        }
    }
    d.finish();
}

/// Extra: `inner` must agree with calling the individual entry points, and the
/// unused arguments of a case must not influence its result.
#[test]
fn inner_matches_direct_calls_and_ignores_unused_args() {
    let (c, r) = (common::c_api(), common::rust_api());
    let mut d = Diff::new("inner dispatch equals the direct calls");
    let mut rng = Rng::new(0x41);
    for _ in 0..3000 {
        let (x, y, z) = (rng.coord(32), rng.coord(32), rng.coord(32));
        let (xw, yw, zw) = (
            *rng.pick(SPECIAL_WRAPS),
            *rng.pick(SPECIAL_WRAPS),
            *rng.pick(SPECIAL_WRAPS),
        );
        let seed = rng.next_i32();
        let (lac, gain, offset) = (rng.lac_gain(), rng.lac_gain(), rng.lac_gain());
        let octaves = rng.range(0, 6);
        let ctx = format_args!("wraps=({xw},{yw},{zw}) seed={seed} octaves={octaves}");
        d.check(ctx, unsafe { (c.inner)(0, x, y, z, xw, yw, zw, seed, lac, gain, offset, octaves) }, unsafe {
            (r.noise3)(x, y, z, xw, yw, zw)
        });
        d.check(ctx, unsafe { (c.inner)(1, x, y, z, xw, yw, zw, seed, lac, gain, offset, octaves) }, unsafe {
            (r.noise3_seed)(x, y, z, xw, yw, zw, seed)
        });
        d.check(ctx, unsafe { (c.inner)(2, x, y, z, xw, yw, zw, seed, lac, gain, offset, octaves) }, unsafe {
            (r.ridge)(x, y, z, lac, gain, offset, octaves)
        });
        d.check(ctx, unsafe { (c.inner)(3, x, y, z, xw, yw, zw, seed, lac, gain, offset, octaves) }, unsafe {
            (r.fbm)(x, y, z, lac, gain, octaves)
        });
        d.check(ctx, unsafe { (c.inner)(4, x, y, z, xw, yw, zw, seed, lac, gain, offset, octaves) }, unsafe {
            (r.turbulence)(x, y, z, lac, gain, octaves)
        });
        // Unused arguments must not matter: re-run case 0 with different
        // fractal parameters and expect the very same result.
        d.check(
            ctx,
            unsafe { (c.inner)(0, x, y, z, xw, yw, zw, seed, lac, gain, offset, octaves) },
            unsafe { (r.inner)(0, x, y, z, xw, yw, zw, !seed, -lac, -gain, -offset, octaves ^ 3) },
        );
    }
    d.finish();
}
