//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md` (rows 1..=21; row 22, the exhaustive
//! sweep, lives in `phase_d_exhaustive.rs`). Every test drives BOTH `.so`
//! files through `libloading` and compares the returned `u16` byte-for-byte.
//!
//! Randomized inputs use SplitMix64 with a fixed seed, so a failure is exactly
//! reproducible.

mod common;

use common::*;

/// Sweep an inclusive exponent range for one sign, over the named boundary
/// mantissas plus `n_random` pseudo-random mantissas per exponent.
fn sweep_exponents(
    libs: &Libs,
    sign: u32,
    exps: std::ops::RangeInclusive<u32>,
    shift_hint: u32,
    n_random: u32,
    ctx: &str,
) {
    let mut rng = Rng::new(SEED ^ ((sign as u64) << 40) ^ (*exps.start() as u64));
    for exp in exps {
        for m in boundary_mantissas(shift_hint) {
            check_fields(libs, sign, exp, m, ctx);
        }
        for _ in 0..n_random {
            let m = rng.next_u32() & 0x7F_FFFF;
            check_fields(libs, sign, exp, m, ctx);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 1-2: R1 - exponent 0 (float zero and float subnormals) -> +/-0
// ---------------------------------------------------------------------------

#[test]
fn cfg_row01_r1_pos_zero_and_subnormal() {
    let libs = Libs::load();
    // +0.0 exactly.
    check_fields(&libs, 0, 0, 0, "row01 +0.0");
    // All float subnormals: boundary mantissas + many random ones.
    sweep_exponents(&libs, 0, 0..=0, 24, samples_per_cell(4000), "row01");
    // Smallest positive subnormal and largest positive subnormal.
    check_bits(&libs, 0x0000_0001, "row01 min subnormal");
    check_bits(&libs, 0x007F_FFFF, "row01 max subnormal");
}

#[test]
fn cfg_row02_r1_neg_zero_and_subnormal() {
    let libs = Libs::load();
    check_fields(&libs, 1, 0, 0, "row02 -0.0");
    sweep_exponents(&libs, 1, 0..=0, 24, samples_per_cell(4000), "row02");
    check_bits(&libs, 0x8000_0001, "row02 -min subnormal");
    check_bits(&libs, 0x807F_FFFF, "row02 -max subnormal");
}

// ---------------------------------------------------------------------------
// Rows 3-4: R2 - exponents 1..102, underflow, flush to +/-0
// ---------------------------------------------------------------------------

#[test]
fn cfg_row03_r2_pos_underflow() {
    let libs = Libs::load();
    sweep_exponents(&libs, 0, 1..=102, 24, samples_per_cell(24), "row03");
}

#[test]
fn cfg_row04_r2_neg_underflow() {
    let libs = Libs::load();
    sweep_exponents(&libs, 1, 1..=102, 24, samples_per_cell(24), "row04");
}

// ---------------------------------------------------------------------------
// Rows 5-6: R3 - exponents 103..112, each with its OWN shift (23..14).
// This is the only region where `shift` varies per index, so each exponent is
// probed against ITS own granularity boundary.
// ---------------------------------------------------------------------------

/// `m__shift[103..=112] == [23, 22, 21, 20, 19, 18, 17, 16, 15, 14]`.
fn r3_shift(exp: u32) -> u32 {
    assert!((103..=112).contains(&exp));
    23 - (exp - 103)
}

#[test]
fn cfg_row05_r3_pos_half_subnormal_per_exponent() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x0505);
    for exp in 103..=112u32 {
        let shift = r3_shift(exp);
        for m in boundary_mantissas(shift) {
            check_fields(&libs, 0, exp, m, "row05");
        }
        for _ in 0..samples_per_cell(2000) {
            let m = rng.next_u32() & 0x7F_FFFF;
            check_fields(&libs, 0, exp, m, "row05");
        }
        // Walk the exact cut points of this exponent's shift.
        for k in 0..=(23 - shift).min(23) {
            let unit = 1u32 << shift;
            for delta in [0i64, -1, 1] {
                let m = ((unit as i64) * ((1i64 << k).max(1)) + delta).clamp(0, 0x7F_FFFF) as u32;
                check_fields(&libs, 0, exp, m, "row05 cutpoint");
            }
        }
    }
}

#[test]
fn cfg_row06_r3_neg_half_subnormal_per_exponent() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x0606);
    for exp in 103..=112u32 {
        let shift = r3_shift(exp);
        for m in boundary_mantissas(shift) {
            check_fields(&libs, 1, exp, m, "row06");
        }
        for _ in 0..samples_per_cell(2000) {
            let m = rng.next_u32() & 0x7F_FFFF;
            check_fields(&libs, 1, exp, m, "row06");
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 7-8: R4 - exponent 113 exactly, the smallest half normal
// ---------------------------------------------------------------------------

#[test]
fn cfg_row07_r4_pos_smallest_normal() {
    let libs = Libs::load();
    sweep_exponents(&libs, 0, 113..=113, 13, samples_per_cell(20000), "row07");
}

#[test]
fn cfg_row08_r4_neg_smallest_normal() {
    let libs = Libs::load();
    sweep_exponents(&libs, 1, 113..=113, 13, samples_per_cell(20000), "row08");
}

// ---------------------------------------------------------------------------
// Rows 9-10: R5 - exponents 114..142, the half normal range
// ---------------------------------------------------------------------------

#[test]
fn cfg_row09_r5_pos_normal_range() {
    let libs = Libs::load();
    sweep_exponents(&libs, 0, 114..=142, 13, samples_per_cell(600), "row09");
}

#[test]
fn cfg_row10_r5_neg_normal_range() {
    let libs = Libs::load();
    sweep_exponents(&libs, 1, 114..=142, 13, samples_per_cell(600), "row10");
}

// ---------------------------------------------------------------------------
// Rows 11-12: R6 - exponents 143..254, overflow to +/-Inf, mantissa discarded
// ---------------------------------------------------------------------------

#[test]
fn cfg_row11_r6_pos_overflow_to_inf() {
    let libs = Libs::load();
    sweep_exponents(&libs, 0, 143..=254, 24, samples_per_cell(60), "row11");
}

#[test]
fn cfg_row12_r6_neg_overflow_to_inf() {
    let libs = Libs::load();
    sweep_exponents(&libs, 1, 143..=254, 24, samples_per_cell(60), "row12");
}

// ---------------------------------------------------------------------------
// Rows 13-14: R7 - exponent 255: Inf and NaN. shift == 13 here (NOT 24 as in
// R6), so the NaN payload propagates into the result. Highest-risk row.
// ---------------------------------------------------------------------------

#[test]
fn cfg_row13_r7_pos_inf_and_nan_payloads() {
    let libs = Libs::load();
    // +Inf
    check_bits(&libs, 0x7F80_0000, "row13 +Inf");
    // Canonical qNaN and sNaN
    check_bits(&libs, 0x7FC0_0000, "row13 +qNaN");
    check_bits(&libs, 0x7FA0_0000, "row13 +sNaN");
    // Payloads that are shifted away entirely (NaN -> Inf, a C quirk).
    for m in 1..=0x1FFFu32 {
        check_fields(&libs, 0, 255, m, "row13 payload shifted away");
    }
    // Every payload boundary around each multiple of 1<<13.
    for k in 0..1024u32 {
        let unit = k * 0x2000;
        for m in [unit.saturating_sub(1), unit, unit + 1] {
            if m <= 0x7F_FFFF {
                check_fields(&libs, 0, 255, m, "row13 payload cutpoint");
            }
        }
    }
    sweep_exponents(&libs, 0, 255..=255, 13, samples_per_cell(50000), "row13");
}

#[test]
fn cfg_row14_r7_neg_inf_and_nan_payloads() {
    let libs = Libs::load();
    check_bits(&libs, 0xFF80_0000, "row14 -Inf");
    check_bits(&libs, 0xFFC0_0000, "row14 -qNaN");
    check_bits(&libs, 0xFFA0_0000, "row14 -sNaN");
    check_bits(&libs, 0xFFFF_FFFF, "row14 all bits set (j=511, max mantissa)");
    for m in 1..=0x1FFFu32 {
        check_fields(&libs, 1, 255, m, "row14 payload shifted away");
    }
    for k in 0..1024u32 {
        let unit = k * 0x2000;
        for m in [unit.saturating_sub(1), unit, unit + 1] {
            if m <= 0x7F_FFFF {
                check_fields(&libs, 1, 255, m, "row14 payload cutpoint");
            }
        }
    }
    sweep_exponents(&libs, 1, 255..=255, 13, samples_per_cell(50000), "row14");
}

// ---------------------------------------------------------------------------
// Row 15: the 86 maximal constant-(base,shift) runs - first/last index of each
// ---------------------------------------------------------------------------

/// Re-derive, from the *C source text* (via `common::read_c_tables`), the
/// maximal runs of a constant `(base, shift)` pair, then test the first and
/// last index of every run. This deliberately parses the C rather than
/// trusting the Rust tables.
#[test]
fn cfg_row15_all_86_run_boundaries() {
    let libs = Libs::load();
    let (base, shift) = read_c_tables();

    // Maximal runs of a constant (base, shift) pair.
    let mut runs: Vec<(usize, usize)> = Vec::new();
    let mut start = 0usize;
    for k in 1..=512usize {
        if k == 512 || (base[k], shift[k]) != (base[start], shift[start]) {
            runs.push((start, k - 1));
            start = k;
        }
    }
    assert_eq!(
        runs.len(),
        86,
        "expected 86 maximal constant-(base,shift) runs, found {}",
        runs.len()
    );

    let mut rng = Rng::new(SEED ^ 0x1515);
    for (a, z) in runs {
        for &j in &[a, z] {
            let sign = (j >> 8) as u32;
            let exp = (j & 0xFF) as u32;
            for m in boundary_mantissas(shift[j] as u32) {
                check_fields(&libs, sign, exp, m, "row15 run boundary");
            }
            for _ in 0..samples_per_cell(64) {
                let m = rng.next_u32() & 0x7F_FFFF;
                check_fields(&libs, sign, exp, m, "row15 run boundary random");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 16-19: full 512-index sweeps
// ---------------------------------------------------------------------------

#[test]
fn cfg_row16_all_512_indices_mantissa_zero() {
    let libs = Libs::load();
    for j in 0..512u32 {
        check_fields(&libs, j >> 8, j & 0xFF, 0, "row16");
    }
}

#[test]
fn cfg_row17_all_512_indices_mantissa_max() {
    let libs = Libs::load();
    for j in 0..512u32 {
        check_fields(&libs, j >> 8, j & 0xFF, 0x7F_FFFF, "row17");
    }
}

#[test]
fn cfg_row18_all_512_indices_random_mantissas() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1818);
    for j in 0..512u32 {
        for _ in 0..samples_per_cell(64) {
            let m = rng.next_u32() & 0x7F_FFFF;
            check_fields(&libs, j >> 8, j & 0xFF, m, "row18");
        }
    }
}

#[test]
fn cfg_row19_all_512_indices_power_of_two_mantissas() {
    let libs = Libs::load();
    for j in 0..512u32 {
        let (sign, exp) = (j >> 8, j & 0xFF);
        for k in 0..23u32 {
            let unit = 1u32 << k;
            for m in [unit - 1, unit, unit + 1] {
                if m <= 0x7F_FFFF {
                    check_fields(&libs, sign, exp, m, "row19");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 20: uniformly random full 32-bit patterns
// ---------------------------------------------------------------------------

#[test]
fn cfg_row20_uniform_random_bit_patterns() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x2020);
    let n = samples_per_cell(2_000_000);
    for _ in 0..n {
        check_bits(&libs, rng.next_u32(), "row20 uniform random");
    }
}

// ---------------------------------------------------------------------------
// Row 21: realistic f32 values a consumer would actually pass
// ---------------------------------------------------------------------------

#[test]
fn cfg_row21_realistic_float_values() {
    let libs = Libs::load();

    let named: &[f32] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        -2.0,
        3.0,
        -3.0,
        0.1,
        -0.1,
        1.0 / 3.0,
        core::f32::consts::PI,
        -core::f32::consts::PI,
        core::f32::consts::E,
        65504.0,   // largest finite binary16
        -65504.0,
        65505.0,
        65519.0,
        65520.0,   // first value that overflows binary16
        65536.0,
        6.1035156e-5,  // smallest binary16 normal
        -6.1035156e-5,
        6.0975552e-5,  // largest binary16 subnormal
        5.9604645e-8,  // smallest binary16 subnormal
        2.9802322e-8,  // half of it: underflows
        1e-45,         // smallest f32 subnormal
        -1e-45,
        1e38,
        -1e38,
        3.4028235e38,  // f32::MAX
        -3.4028235e38,
        1.1754944e-38, // f32::MIN_POSITIVE
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        f32::EPSILON,
        1024.0,
        -1024.0,
        100.0,
        -100.0,
        1e-8,
        1e8,
    ];
    for &v in named {
        check_bits(&libs, v.to_bits(), "row21 named");
    }

    // Randomized "realistic" magnitudes spread across the whole exponent span,
    // built from a random mantissa and a random exponent rather than from a
    // uniform bit pattern, so the well-behaved value range is densely covered.
    let mut rng = Rng::new(SEED ^ 0x2121);
    for _ in 0..samples_per_cell(200_000) {
        let sign = rng.next_u32() & 1;
        let exp = rng.below(256);
        let mant = rng.next_u32() & 0x7F_FFFF;
        check_fields(&libs, sign, exp, mant, "row21 random realistic");
    }

    // Random values scaled by powers of ten, the way real data looks.
    for _ in 0..samples_per_cell(100_000) {
        let mag = (rng.below(77) as i32) - 38; // 1e-38 .. 1e38
        let frac = (rng.next_u32() as f64) / (u32::MAX as f64);
        let v = (frac * 10f64.powi(mag)) as f32;
        let v = if rng.next_u32() & 1 == 0 { v } else { -v };
        check_bits(&libs, v.to_bits(), "row21 decimal-scaled");
    }
}
