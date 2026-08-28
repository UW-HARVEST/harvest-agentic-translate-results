//! Differential tests for `hsv_to_rgb`.
//!
//! The C shared library is the ground truth; the Rust `cdylib` must produce
//! byte-identical results. Both are loaded with `libloading` and invoked
//! exclusively through their exported `hsv_to_rgb` symbol.

mod common;

use common::{Implementations, Rng, assert_same, fuzz_iterations};

/// Sanity check: both libraries were found and both export the symbol.
#[test]
fn both_libraries_export_hsv_to_rgb() {
    let impls = Implementations::load();
    println!("C   : {}", impls.c_path.display());
    println!("Rust: {}", impls.rust_path.display());

    // A trivially valid conversion, just to prove both are callable.
    assert_same(&impls, [0.0, 1.0, 1.0]);
}

/// `s == 0` takes the early-return path and copies `v` into all three slots.
/// This includes `s == -0.0`, which compares equal to zero in C.
#[test]
fn achromatic_early_return() {
    let impls = Implementations::load();

    let hues = [
        -1.0e30, -720.0, -60.0, -1.0, -0.0, 0.0, 1.0, 59.0, 60.0, 180.0, 359.999, 360.0, 720.0,
        1.0e30, f32::INFINITY, f32::NEG_INFINITY, f32::NAN,
    ];
    let values = [
        -1.0e30,
        -1.0,
        -0.0,
        0.0,
        f32::from_bits(1), // smallest positive subnormal
        0.5,
        1.0,
        255.0,
        f32::MAX,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
    ];

    for &s in &[0.0f32, -0.0f32] {
        for &h in &hues {
            for &v in &values {
                assert_same(&impls, [h, s, v]);
            }
        }
    }
}

/// Each arm of the `switch (i)` statement, including the `default` arm reached
/// from both large-positive and negative sector indices.
#[test]
fn every_switch_sector() {
    let impls = Implementations::load();

    // i = -3 .. 12 covers cases 0-4 plus `default` on both sides of the range.
    for sector in -3..=12 {
        let base = sector as f32 * 60.0;
        for offset in [0.0f32, 1.0, 15.0, 30.0, 45.0, 59.0, 59.9999] {
            for &s in &[0.25f32, 0.5, 1.0] {
                for &v in &[0.25f32, 1.0, 255.0] {
                    assert_same(&impls, [base + offset, s, v]);
                }
            }
        }
    }
}

/// Values of `h` that land exactly on, or one ULP away from, a sector boundary.
/// These exercise the `floorf` / `(int)` rounding behaviour.
#[test]
fn sector_boundaries_to_the_ulp() {
    let impls = Implementations::load();

    for sector in -8..=8 {
        let boundary = sector as f32 * 60.0;
        let mut candidates = vec![boundary];
        let mut x = boundary;
        for _ in 0..4 {
            x = next_up(x);
            candidates.push(x);
        }
        let mut x = boundary;
        for _ in 0..4 {
            x = next_down(x);
            candidates.push(x);
        }

        for h in candidates {
            for &s in &[0.125f32, 1.0] {
                for &v in &[1.0f32, 100.0] {
                    assert_same(&impls, [h, s, v]);
                }
            }
        }
    }
}

/// Out-of-range and non-finite `h`, where the C cast `(int)floorf(h)` leaves
/// the range of `int`. The Rust translation must reproduce whatever the C build
/// does here, not saturate.
#[test]
fn hue_outside_int_range() {
    let impls = Implementations::load();

    let hues = [
        // h/60 straddling INT_MAX and INT_MIN.
        2_147_483_520.0f32 * 60.0,
        2_147_483_648.0f32 * 60.0,
        -2_147_483_648.0f32 * 60.0,
        -2_147_483_904.0f32 * 60.0,
        1.0e18,
        -1.0e18,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7FC0_0001),
        f32::from_bits(0x7F80_0001), // signalling NaN
        f32::from_bits(0xFF80_0001),
    ];

    for h in hues {
        for &s in &[0.5f32, 1.0, -1.0, 2.0] {
            for &v in &[1.0f32, 0.0, -3.0, f32::INFINITY, f32::NAN] {
                assert_same(&impls, [h, s, v]);
            }
        }
    }
}

/// Extreme / degenerate `s` and `v`, including values outside the nominal
/// `[0, 1]` domain and non-finite ones, which produce infinities and NaNs.
#[test]
fn saturation_and_value_extremes() {
    let impls = Implementations::load();

    let specials = [
        f32::NAN,
        -f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(1),
        f32::from_bits(0x8000_0001),
        -0.0,
        1.0e-30,
        -1.0e-30,
        1.0e30,
        -1.0e30,
        1.0,
        -1.0,
        2.0,
        -2.0,
        1e7,
    ];

    for &h in &[-30.0f32, 0.0, 45.0, 90.0, 150.0, 210.0, 270.0, 330.0, 400.0] {
        for &s in &specials {
            for &v in &specials {
                assert_same(&impls, [h, s, v]);
            }
        }
    }
}

/// Dense sweep of the nominal domain: `h` in `[0, 360)`, `s`/`v` in `[0, 1]`.
#[test]
fn dense_nominal_domain_sweep() {
    let impls = Implementations::load();

    for hi in 0..=720 {
        let h = hi as f32 * 0.5;
        for si in 0..=16 {
            let s = si as f32 / 16.0;
            for vi in 0..=16 {
                let v = vi as f32 / 16.0;
                assert_same(&impls, [h, s, v]);
            }
        }
    }
}

/// Randomised sweep over the nominal domain.
#[test]
fn random_nominal_domain() {
    let impls = Implementations::load();
    let mut rng = Rng::new(0x5EED_1234_ABCD_0001);

    for _ in 0..fuzz_iterations(200_000) {
        let h = rng.next_range(-720.0, 1080.0);
        let s = rng.next_unit();
        let v = rng.next_unit();
        assert_same(&impls, [h, s, v]);
    }
}

/// Randomised sweep over wide magnitudes, still finite.
#[test]
fn random_wide_magnitudes() {
    let impls = Implementations::load();
    let mut rng = Rng::new(0xC0FF_EE00_1357_9BDF);

    for _ in 0..fuzz_iterations(200_000) {
        let h = random_scaled(&mut rng);
        let s = random_scaled(&mut rng);
        let v = random_scaled(&mut rng);
        assert_same(&impls, [h, s, v]);
    }
}

/// Randomised sweep over completely arbitrary bit patterns, so NaN payloads,
/// subnormals and infinities all get exercised.
#[test]
fn random_arbitrary_bit_patterns() {
    let impls = Implementations::load();
    let mut rng = Rng::new(0x1BAD_B002_DEAD_C0DE);

    for _ in 0..fuzz_iterations(200_000) {
        let h = f32::from_bits(rng.next_u32());
        let s = f32::from_bits(rng.next_u32());
        let v = f32::from_bits(rng.next_u32());
        assert_same(&impls, [h, s, v]);
    }
}

/// Aliasing case the C code tolerates: `dest` and `src` may overlap because
/// every input is read into a local before any store happens.
#[test]
fn aliased_source_and_destination() {
    let impls = Implementations::load();

    let inputs: [[f32; 3]; 8] = [
        [0.0, 0.0, 0.5],
        [30.0, 1.0, 1.0],
        [90.0, 0.5, 0.25],
        [150.0, 0.75, 2.0],
        [210.0, 0.9, 0.1],
        [270.0, 0.3, 0.7],
        [330.0, 1.0, 255.0],
        [-45.0, 0.6, 1.0],
    ];

    for input in inputs {
        let mut c_buf = input;
        let mut rust_buf = input;
        unsafe {
            (impls.c)(c_buf.as_mut_ptr(), c_buf.as_ptr());
            (impls.rust)(rust_buf.as_mut_ptr(), rust_buf.as_ptr());
        }
        assert_eq!(
            c_buf.map(f32::to_bits),
            rust_buf.map(f32::to_bits),
            "aliased in/out mismatch for {input:?}"
        );
    }
}

/// A finite float spanning a broad range of exponents, both signs.
fn random_scaled(rng: &mut Rng) -> f32 {
    let mantissa = rng.next_unit() * 2.0 - 1.0;
    // Exponents from 2^-40 up to 2^30.
    let exponent = (rng.next_u32() % 71) as i32 - 40;
    let scaled = mantissa * 2.0f32.powi(exponent);
    if scaled.is_finite() { scaled } else { 0.0 }
}

fn next_up(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(1);
    }
    let bits = x.to_bits();
    if x > 0.0 {
        f32::from_bits(bits + 1)
    } else {
        f32::from_bits(bits - 1)
    }
}

fn next_down(x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return f32::from_bits(0x8000_0001);
    }
    let bits = x.to_bits();
    if x > 0.0 {
        f32::from_bits(bits - 1)
    } else {
        f32::from_bits(bits + 1)
    }
}
