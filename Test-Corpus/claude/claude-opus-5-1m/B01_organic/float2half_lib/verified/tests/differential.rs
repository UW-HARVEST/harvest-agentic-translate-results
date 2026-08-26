//! Phase B — valid-path differential tests, one test per axis of `CONFIGS.md`.
//!
//! Every assertion calls the C `.so` and the Rust `.so` through `libloading`
//! and compares the returned `uint16_t` bit-for-bit.

mod common;

use common::{bits_from, libs, Rng, CONFIG_ROWS, MANT_SHAPES};

/// Sanity: both `.so` files really do export the symbol and are callable.
#[test]
fn both_shared_objects_export_and_answer() {
    let l = libs();
    // 1.0f32 -> 0x3C00 in binary16.
    assert_eq!(l.c(1.0), l.rust(1.0));
    assert_eq!(l.c(1.0), 0x3C00, "C reference sanity value");
}

/// CONFIGS.md rows 1..86: every distinct `(m__base, m__shift)` class, crossed
/// with every boundary mantissa shape, at both ends and the middle of each run.
#[test]
fn config_rows_all_86_base_shift_classes() {
    let l = libs();
    let mut checked = 0u64;
    for (row, lo, hi) in CONFIG_ROWS {
        // Ends and midpoint of the run are the representative j values.
        let js = [lo, lo + (hi - lo) / 2, hi];
        for j in js {
            for mant in MANT_SHAPES {
                l.assert_same_bits(bits_from(j, mant), &format!("CONFIGS row {row}"));
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 86 * 3 * 8);
}

/// CONFIGS.md rows 1..86 with randomized mantissas AND randomized `j` inside
/// each run (fixed seed => reproducible). A single scalar per row would only
/// hit one value-dependent path; this sweeps many.
#[test]
fn config_rows_randomized_mantissas() {
    let l = libs();
    let mut rng = Rng::new(0xC0FF_EE_12_34_56_78);
    const PER_ROW: usize = 2000;
    for (row, lo, hi) in CONFIG_ROWS {
        let span = hi - lo + 1;
        for _ in 0..PER_ROW {
            let j = lo + rng.below(span);
            let mant = rng.next_u32() & 0x007f_ffff;
            l.assert_same_bits(bits_from(j, mant), &format!("CONFIGS row {row} (random)"));
        }
    }
}

/// Every one of the 512 reachable `j` values (not just one per run), crossed
/// with the boundary mantissa shapes. This covers each individual `m__base`
/// entry, including the 41+41 unique per-exponent base values.
#[test]
fn every_index_j_with_boundary_mantissas() {
    let l = libs();
    for j in 0u32..512 {
        for mant in MANT_SHAPES {
            l.assert_same_bits(bits_from(j, mant), &format!("j={j} boundary mantissa"));
        }
    }
}

/// Every `j`, sweeping every distinct value of the *surviving* mantissa field.
/// For shift `s`, the result only depends on `mant >> s`, which ranges over
/// `0 ..= (0x7FFFFF >> s)`. This enumerates each attainable output for the
/// small-shift classes and both endpoints of each quantisation bucket, i.e.
/// it proves the truncation boundary is identical.
#[test]
fn every_index_j_full_quantisation_buckets() {
    let l = libs();
    for j in 0u32..512 {
        // Largest shift is 24 -> 1 bucket; smallest is 13 -> 1024 buckets.
        for q in 0u32..=0x3FF {
            let base = q << 13;
            // first, middle and last mantissa in this bucket of 8192 values
            for mant in [base, base + 4095, base + 8191] {
                l.assert_same_bits(bits_from(j, mant), &format!("j={j} bucket q={q}"));
            }
        }
    }
}

/// Uniformly random 32-bit patterns interpreted as floats: the fully
/// unstructured property test over the whole input domain.
#[test]
fn randomized_raw_bit_patterns() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0000_0000_0001);
    for _ in 0..2_000_000 {
        l.assert_same_bits(rng.next_u32(), "uniform random bits");
    }
}

/// Randomized *realistic* float values (as a numeric consumer would pass),
/// spanning magnitudes far below and far above the half-precision range.
#[test]
fn randomized_realistic_float_values() {
    let l = libs();
    let mut rng = Rng::new(0xABCD_1234_5678_9F01);
    for _ in 0..500_000 {
        let mantissa = (rng.next_u32() & 0x007f_ffff) as f32 / 8_388_608.0;
        // exponents from 2^-60 up to 2^+60 straddle underflow and overflow
        let e = rng.below(121) as i32 - 60;
        let sign = if rng.next_u32() & 1 == 0 { 1.0 } else { -1.0 };
        let v = sign * (1.0 + mantissa) * (2.0f32).powi(e);
        l.assert_same_bits(v.to_bits(), "realistic value");
    }
}

/// Values that a consumer is most likely to convert: small integers, powers of
/// two, and the exactly-representable half-precision grid.
#[test]
fn representative_numeric_values() {
    let l = libs();
    let mut vals: Vec<f32> = Vec::new();
    for i in -2048i32..=2048 {
        vals.push(i as f32);
        vals.push(i as f32 + 0.5);
        vals.push(i as f32 / 3.0);
    }
    for e in -70i32..=70 {
        vals.push((2.0f32).powi(e));
        vals.push(-(2.0f32).powi(e));
        vals.push((2.0f32).powi(e) * 1.5);
    }
    vals.extend_from_slice(&[
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MIN,
        f32::MAX,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::EPSILON,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        -f32::NAN,
        65504.0,  // largest finite binary16
        -65504.0,
        65520.0,  // first value that overflows binary16
        6.1035156e-5,  // smallest normal binary16
        5.9604645e-8,  // smallest subnormal binary16
    ]);
    for v in vals {
        l.assert_same_bits(v.to_bits(), "representative value");
    }
}

/// Determinism / no hidden state: the same input must give the same answer on
/// repeat calls, and interleaving C and Rust calls must not perturb either
/// (guards against the Rust `static` tables being mutated).
#[test]
fn repeated_and_interleaved_calls_are_stateless() {
    let l = libs();
    let mut rng = Rng::new(7);
    let probes: Vec<u32> = (0..1000).map(|_| rng.next_u32()).collect();
    let c_first: Vec<u16> = probes.iter().map(|&b| l.c(f32::from_bits(b))).collect();
    let r_first: Vec<u16> = probes.iter().map(|&b| l.rust(f32::from_bits(b))).collect();
    for _round in 0..50 {
        for (i, &b) in probes.iter().enumerate() {
            let x = f32::from_bits(b);
            assert_eq!(l.c(x), c_first[i], "C not deterministic at 0x{b:08X}");
            assert_eq!(l.rust(x), r_first[i], "Rust not deterministic at 0x{b:08X}");
            assert_eq!(l.rust(x), l.c(x), "interleaved divergence at 0x{b:08X}");
        }
    }
    assert_eq!(c_first, r_first);
}
