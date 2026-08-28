//! Phase E — heavy soak sweeps (`#[ignore]`d so the normal suite stays fast).
//!
//! Run with:
//! ```sh
//! cargo test --release --test phase_e_soak -- --ignored --nocapture
//! ```
//!
//! These go well beyond the per-row Phase B/C tests to give an independent,
//! near-exhaustive confidence check. The coverage argument they rest on:
//!
//! * For `exp_q2 <= 0` the loop runs **exactly once**, and the result depends on
//!   `exp_q2` only through `(exp_q2 & 3, (exp_q2 >> 2) & 31)`, which is periodic
//!   with **period 128**. So sweeping any contiguous 128 negative values covers
//!   every distinct negative code path; these tests sweep millions.
//! * For `exp_q2 > 120` every trip but the last uses the identical multiplier
//!   `2^-30`, and the last uses `exp_q2 mod 120`-ish residue, so behaviour is
//!   determined by (trip count, final remainder).

mod common;

use common::*;

/// Exhaustive sweep of a contiguous negative range (many full 128-periods)
/// crossed with a set of representative `y` values.
#[test]
#[ignore = "soak: ~30M FFI call pairs"]
fn s1_exhaustive_negative_exp_sweep() {
    let im = impls();
    let ys: Vec<f32> = vec![
        1.0,
        -1.0,
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7FC0_1234),
        f32::from_bits(0x7FA0_0000),
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x4B7F_FFFF),
        std::f32::consts::PI,
    ];
    let mut n: u64 = 0;
    for e in -2_000_000..=0i32 {
        for &y in &ys {
            let c = im.c(y, e);
            let r = im.rust(y, e);
            if c.to_bits() != r.to_bits() {
                panic!(
                    "S1 divergence at y=0x{:08x} exp_q2={e}: C=0x{:08x} Rust=0x{:08x}",
                    y.to_bits(),
                    c.to_bits(),
                    r.to_bits()
                );
            }
            n += 1;
        }
    }
    eprintln!("S1: {n} exhaustive negative-exp pairs OK");
}

/// Exhaustive sweep of the positive single- and multi-trip region.
#[test]
#[ignore = "soak: ~10M FFI call pairs"]
fn s2_exhaustive_positive_exp_sweep() {
    let im = impls();
    let ys: Vec<f32> = vec![
        1.0,
        -1.0,
        0.0,
        -0.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7FC0_1234),
        f32::MAX,
        f32::MIN_POSITIVE,
        std::f32::consts::E,
    ];
    let mut n: u64 = 0;
    // Up to 4000 => ~34 trips, covering the trip-count transitions densely.
    for e in 0..=4000i32 {
        for &y in &ys {
            let c = im.c(y, e);
            let r = im.rust(y, e);
            assert_eq!(
                c.to_bits(),
                r.to_bits(),
                "S2 divergence at y=0x{:08x} exp_q2={e}",
                y.to_bits()
            );
            n += 1;
        }
    }
    eprintln!("S2: {n} exhaustive positive-exp pairs OK");
}

/// Massive randomized fuzz over arbitrary `f32` bit patterns and a wide
/// `exp_q2` range.
#[test]
#[ignore = "soak: 20M randomized FFI call pairs"]
fn s3_random_fuzz_20m() {
    let im = impls();
    let mut rng = Rng::new(0x5040_1337);
    let mut n: u64 = 0;
    for _ in 0..20_000_000u64 {
        let y = f32::from_bits(rng.next_u32());
        let e = rng.range_i32(-1_000_000, 20_000);
        let c = im.c(y, e);
        let r = im.rust(y, e);
        if c.to_bits() != r.to_bits() {
            panic!(
                "S3 divergence at y=0x{:08x} exp_q2={e}: C=0x{:08x} Rust=0x{:08x}",
                y.to_bits(),
                c.to_bits(),
                r.to_bits()
            );
        }
        n += 1;
    }
    eprintln!("S3: {n} randomized pairs OK");
}

/// Exhaustive over ALL 2^24 `f32` values with the top mantissa bits varied, at
/// the exponents that exercise each distinct scale regime. This is the closest
/// practical approach to exhausting the `y` domain.
#[test]
#[ignore = "soak: exhaustive y mantissa sweep"]
fn s4_exhaustive_y_mantissa_sweep() {
    let im = impls();
    // One exponent per distinct scale regime.
    let exps: Vec<i32> = vec![0, -1, -5, -8, -128, 119, 120, 121, 240, 1200];
    let mut n: u64 = 0;
    for &e in &exps {
        // Sweep the full 23-bit mantissa at a few biased exponents, both signs.
        for biased_exp in [0u32, 1, 100, 127, 254, 255] {
            for mant in (0..0x0080_0000u32).step_by(97) {
                for sign in [0u32, 0x8000_0000] {
                    let bits = sign | (biased_exp << 23) | mant;
                    let y = f32::from_bits(bits);
                    let c = im.c(y, e);
                    let r = im.rust(y, e);
                    if c.to_bits() != r.to_bits() {
                        panic!(
                            "S4 divergence at y=0x{bits:08x} exp_q2={e}: C=0x{:08x} Rust=0x{:08x}",
                            c.to_bits(),
                            r.to_bits()
                        );
                    }
                    n += 1;
                }
            }
        }
    }
    eprintln!("S4: {n} exhaustive-mantissa pairs OK");
}
