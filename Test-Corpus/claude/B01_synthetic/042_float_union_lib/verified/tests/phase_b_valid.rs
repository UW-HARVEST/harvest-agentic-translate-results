// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md. Every test drives BOTH the C `.so` and the
// Rust `.so` through their exported `driver` symbol and requires byte-identical
// stdout. Randomized rows use a fixed-seed SplitMix64 so failures reproduce.

mod common;

use common::{
    assert_same, from_fields, ulp_neighbourhood, with_both_signs, Rng, SEED,
};

/// Exact power of two `2^e` for e in -1074..=1023 (subnormal powers included).
fn pow2(e: i32) -> f64 {
    if e >= -1022 {
        from_fields(false, (e + 1023) as u64, 0)
    } else {
        f64::from_bits(1u64 << (e + 1074))
    }
}

// ---------------------------------------------------------------------------
// C1 / C2 — zeros
// ---------------------------------------------------------------------------

#[test]
fn config_c1_positive_zero() {
    assert_same("C1 +0.0", &[0.0]);
}

#[test]
fn config_c2_negative_zero() {
    // Sign bit set, payload zero: %.4f must print `-0.0000`, %a `-0x0p+0`.
    assert_same("C2 -0.0", &[-0.0]);
    assert_same("C2 -0.0 via bits", &[f64::from_bits(0x8000_0000_0000_0000)]);
}

// ---------------------------------------------------------------------------
// C3 — exact small binary fractions
// ---------------------------------------------------------------------------

#[test]
fn config_c3_exact_small_binary_fractions() {
    let mags = [
        1.0, 2.0, 4.0, 8.0, 0.5, 0.25, 0.125, 0.0625, 1.5, 2.5, 3.5, 1.25, 3.75,
        0.75, 0.375,
    ];
    assert_same("C3 exact binary fractions", &with_both_signs(&mags));
}

// ---------------------------------------------------------------------------
// C4 — every exact power of two, both signs
// ---------------------------------------------------------------------------

#[test]
fn config_c4_all_powers_of_two_both_signs() {
    let mut inputs = Vec::new();
    for e in -1074..=1023 {
        let p = pow2(e);
        inputs.push(p);
        inputs.push(-p);
    }
    assert_same("C4 all powers of two", &inputs);
}

// ---------------------------------------------------------------------------
// C5 — full biased-exponent sweep with randomized mantissas
// ---------------------------------------------------------------------------

#[test]
fn config_c5_exponent_sweep_random_mantissa() {
    let mut rng = Rng::new(SEED ^ 0xC5);
    let mut inputs = Vec::new();
    for exp in 0u64..=2047 {
        for _ in 0..4 {
            let m = rng.next_mantissa();
            inputs.push(from_fields(false, exp, m));
            inputs.push(from_fields(true, exp, m));
        }
    }
    assert_same("C5 exponent sweep", &inputs);
}

// ---------------------------------------------------------------------------
// C6 / C7 / C8 — subnormals
// ---------------------------------------------------------------------------

#[test]
fn config_c6_subnormal_extremes() {
    let min_sub = f64::from_bits(1);
    let max_sub = f64::from_bits(0x000F_FFFF_FFFF_FFFF);
    assert_same(
        "C6 subnormal extremes",
        &with_both_signs(&[min_sub, max_sub, f64::from_bits(2), f64::from_bits(3)]),
    );
}

#[test]
fn config_c7_random_subnormals() {
    let mut rng = Rng::new(SEED ^ 0xC7);
    let mut inputs = Vec::new();
    for _ in 0..2000 {
        let mut m = rng.next_mantissa();
        if m == 0 {
            m = 1; // keep it a subnormal rather than a zero
        }
        inputs.push(from_fields(false, 0, m));
        inputs.push(from_fields(true, 0, m));
    }
    assert_same("C7 random subnormals", &inputs);
}

#[test]
fn config_c8_subnormal_normal_boundary() {
    let mut inputs = Vec::new();
    // One step either side of the subnormal/normal transition, both signs.
    for bits in [
        0x000F_FFFF_FFFF_FFFDu64,
        0x000F_FFFF_FFFF_FFFE,
        0x000F_FFFF_FFFF_FFFF,
        0x0010_0000_0000_0000,
        0x0010_0000_0000_0001,
        0x0010_0000_0000_0002,
    ] {
        inputs.push(f64::from_bits(bits));
        inputs.push(f64::from_bits(bits | 0x8000_0000_0000_0000));
    }
    assert_same("C8 subnormal/normal boundary", &inputs);
}

// ---------------------------------------------------------------------------
// C9 — normal extremes
// ---------------------------------------------------------------------------

#[test]
fn config_c9_normal_extremes() {
    let mags = [
        f64::MIN_POSITIVE,
        f64::MAX,
        f64::from_bits(0x7FEF_FFFF_FFFF_FFFF), // == f64::MAX
        f64::from_bits(0x0010_0000_0000_0000), // == MIN_POSITIVE
        f64::from_bits(0x7FEF_FFFF_FFFF_FFFE),
    ];
    assert_same("C9 normal extremes", &with_both_signs(&mags));
}

// ---------------------------------------------------------------------------
// C10 — randomized normals in the unit interval
// ---------------------------------------------------------------------------

#[test]
fn config_c10_random_unit_interval() {
    let mut rng = Rng::new(SEED ^ 0xC10);
    let inputs: Vec<f64> = (0..4000).map(|_| rng.next_signed_unit()).collect();
    assert_same("C10 random unit interval", &inputs);
}

// ---------------------------------------------------------------------------
// C11 — randomized values scaled across every decimal decade
// ---------------------------------------------------------------------------

#[test]
fn config_c11_random_scaled_decades() {
    let mut rng = Rng::new(SEED ^ 0xC11);
    let mut inputs = Vec::new();
    for k in -320..=308 {
        let scale = 10f64.powi(k);
        for _ in 0..3 {
            let v = rng.next_signed_unit() * scale;
            inputs.push(v);
        }
    }
    assert_same("C11 random scaled decimal decades", &inputs);
}

// ---------------------------------------------------------------------------
// C12 — randomized values scaled across every binary decade
// ---------------------------------------------------------------------------

#[test]
fn config_c12_random_scaled_binary_decades() {
    let mut rng = Rng::new(SEED ^ 0xC12);
    let mut inputs = Vec::new();
    for e in -1080..=1020 {
        // Multiply in two steps so the scaling itself does not overflow.
        let m = rng.next_signed_unit();
        let v = if e >= -1022 {
            m * pow2(e)
        } else {
            (m * pow2(-1000)) * pow2(e + 1000)
        };
        inputs.push(v);
    }
    assert_same("C12 random scaled binary decades", &inputs);
}

// ---------------------------------------------------------------------------
// C13 — %.4f rounding ties
// ---------------------------------------------------------------------------

#[test]
fn config_c13_rounding_ties_4th_decimal() {
    let mut inputs = Vec::new();
    for n in 0..40 {
        let base = n as f64;
        for delta in [0.00005f64, -0.00005, 0.00015, -0.00015, 0.00025, 0.00035] {
            inputs.push(base + delta);
            inputs.push(-(base + delta));
        }
    }
    // Explicit `x.xxxx5` half-way cases.
    for v in [
        0.00005f64, 0.00015, 0.00025, 0.00035, 0.00045, 0.12345, 1.00005, 2.00005,
        1.23455, 9.87655, 0.55555, 0.66665,
    ] {
        inputs.push(v);
        inputs.push(-v);
    }
    assert_same("C13 rounding ties", &inputs);
}

// ---------------------------------------------------------------------------
// C14 — the 0.00005 cliff and its ULP neighbourhood
// ---------------------------------------------------------------------------

#[test]
fn config_c14_rounding_cliff_neighbourhood() {
    let mut inputs = Vec::new();
    for anchor in [0.00005f64, 0.0001, 0.00004999, 0.00005001, 0.000049999999] {
        for v in ulp_neighbourhood(anchor, 3) {
            inputs.push(v);
            inputs.push(-v);
        }
    }
    // Magnitudes far below the cliff must collapse to 0.0000 / -0.0000.
    for v in [1e-5f64, 1e-6, 1e-10, 1e-100, 1e-300, f64::MIN_POSITIVE] {
        inputs.push(v);
        inputs.push(-v);
    }
    assert_same("C14 rounding cliff", &inputs);
}

// ---------------------------------------------------------------------------
// C15 — rounding that carries into (and past) the integer part
// ---------------------------------------------------------------------------

#[test]
fn config_c15_rounding_carry_propagation() {
    let mags = [
        0.99995f64,
        0.99999,
        0.9999999,
        9.99995,
        9.99999,
        99.99995,
        999.99995,
        9999.99995,
        0.99994999,
        1.99995,
        1e15 - 0.00005,
        1e16 - 0.5,
    ];
    assert_same("C15 rounding carry", &with_both_signs(&mags));
}

// ---------------------------------------------------------------------------
// C16 — exact integers of increasing width
// ---------------------------------------------------------------------------

#[test]
fn config_c16_exact_integers_increasing_width() {
    let mut mags = Vec::new();
    let mut v = 1.0f64;
    for _ in 0..=22 {
        mags.push(v);
        v *= 10.0;
    }
    mags.push(9007199254740992.0); // 2^53
    mags.push(9007199254740994.0); // 2^53 + 2
    mags.push(4503599627370495.0); // 2^52 - 1
    mags.push(123456789.0);
    mags.push(1e23);
    mags.push(1e100);
    mags.push(1e308);
    assert_same("C16 exact integers", &with_both_signs(&mags));
}

// ---------------------------------------------------------------------------
// C17 — %a mantissa trailing-zero trimming extremes
// ---------------------------------------------------------------------------

#[test]
fn config_c17_mantissa_trimming_extremes() {
    let mut inputs = Vec::new();
    for exp in [1u64, 500, 1022, 1023, 1024, 1500, 2046] {
        for m in [
            0x0000_0000_0000_0000u64,
            0x000F_FFFF_FFFF_FFFF,
            0x0000_0000_0000_0001,
            0x0008_0000_0000_0000,
            0x000F_0000_0000_0000,
            0x0000_00FF_FF00_0000,
        ] {
            inputs.push(from_fields(false, exp, m));
            inputs.push(from_fields(true, exp, m));
        }
    }
    assert_same("C17 mantissa trimming", &inputs);
}

// ---------------------------------------------------------------------------
// C18 / C19 / C20 — non-finite values
// ---------------------------------------------------------------------------

#[test]
fn config_c18_infinities() {
    assert_same("C18 infinities", &[f64::INFINITY, f64::NEG_INFINITY]);
}

#[test]
fn config_c19_quiet_nans() {
    assert_same(
        "C19 quiet NaNs",
        &[
            f64::NAN,
            -f64::NAN,
            f64::from_bits(0x7FF8_0000_0000_0000),
            f64::from_bits(0xFFF8_0000_0000_0000),
        ],
    );
}

#[test]
fn config_c20_nan_payload_sweep() {
    let mut rng = Rng::new(SEED ^ 0xC20);
    let mut inputs = Vec::new();

    // Deterministic payloads, quiet (mantissa MSB set) and signaling (clear).
    for payload in [
        0x0000_0000_0000_0001u64,
        0x0000_0000_0000_00FF,
        0x0007_FFFF_FFFF_FFFF,
        0x0004_0000_0000_0000,
        0x000F_FFFF_FFFF_FFFF,
        0x0008_0000_0000_0001,
    ] {
        for sign in [false, true] {
            inputs.push(from_fields(sign, 0x7FF, payload));
        }
    }

    // Randomized payloads; mantissa forced non-zero so it stays a NaN.
    for _ in 0..1000 {
        let mut m = rng.next_mantissa();
        if m == 0 {
            m = 1;
        }
        inputs.push(from_fields(false, 0x7FF, m));
        inputs.push(from_fields(true, 0x7FF, m));
        // Explicitly signaling variant: clear the quiet bit, keep payload non-zero.
        let sig = (m & !(1u64 << 51)) | 1;
        inputs.push(from_fields(false, 0x7FF, sig));
        inputs.push(from_fields(true, 0x7FF, sig));
    }

    assert_same("C20 NaN payload sweep", &inputs);
}

// ---------------------------------------------------------------------------
// C21 — fully randomized bit patterns (property-style catch-all)
// ---------------------------------------------------------------------------

#[test]
fn config_c21_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 0xC21);
    let inputs: Vec<f64> = (0..20000).map(|_| rng.next_bit_pattern()).collect();
    assert_same("C21 random bit patterns", &inputs);
}

// ---------------------------------------------------------------------------
// C22 — ULP neighbourhoods around notable anchors
// ---------------------------------------------------------------------------

#[test]
fn config_c22_ulp_neighbourhoods() {
    let anchors = [
        0.0f64,
        1.0,
        -1.0,
        2.0,
        0.1,
        0.5,
        10.0,
        f64::MIN_POSITIVE,
        f64::MAX,
        -f64::MAX,
        1e16,
        1e-16,
    ];
    let mut inputs = Vec::new();
    for a in anchors {
        inputs.extend(ulp_neighbourhood(a, 4));
    }
    assert_same("C22 ULP neighbourhoods", &inputs);
}

// ---------------------------------------------------------------------------
// C23 — decimals that are not exactly representable
// ---------------------------------------------------------------------------

#[test]
fn config_c23_inexact_decimals() {
    let mags = [
        0.1f64,
        0.2,
        0.3,
        0.7,
        1.0 / 3.0,
        2.0 / 3.0,
        1e-5,
        1e-4,
        1e-3,
        3.14159265358979,
        2.718281828459045,
        1.4142135623730951,
        0.30000000000000004,
        123.456,
        1e-7,
    ];
    assert_same("C23 inexact decimals", &with_both_signs(&mags));
}

// ---------------------------------------------------------------------------
// C24 — many consecutive calls in one capture (buffering / pipeline)
// ---------------------------------------------------------------------------

#[test]
fn config_c24_bulk_sequential_calls() {
    let mut rng = Rng::new(SEED ^ 0xC24);
    let mut inputs = Vec::with_capacity(4096);
    for i in 0..4096 {
        // Interleave very long %.4f lines with very short ones so that output
        // records straddle glibc's internal buffer boundary.
        inputs.push(match i % 4 {
            0 => rng.next_signed_unit(),
            1 => rng.next_signed_unit() * 1e300,
            2 => rng.next_bit_pattern(),
            _ => 0.0,
        });
    }
    assert_same("C24 bulk sequential calls", &inputs);
}
