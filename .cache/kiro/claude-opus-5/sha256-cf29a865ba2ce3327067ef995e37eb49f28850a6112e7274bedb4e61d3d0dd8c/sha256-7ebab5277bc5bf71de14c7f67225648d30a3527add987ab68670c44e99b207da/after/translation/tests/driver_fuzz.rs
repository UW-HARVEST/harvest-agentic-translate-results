//! High-volume sweeps for `driver`. These are the same comparison as
//! `driver_diff.rs`, just at a scale that makes an accidental disagreement in a
//! rarely taken formatting branch very unlikely to survive.
//!
//! Inputs are streamed through in chunks so memory stays bounded.

mod common;

use common::*;

const CHUNK: usize = 100_000;

/// Uniformly random 64-bit patterns: every exponent, every mantissa, every NaN
/// payload, both signs.
#[test]
fn fuzz_uniform_bit_patterns() {
    let mut rng = SplitMix64(0x243f_6a88_85a3_08d3);
    assert_same_chunked((0..1_000_000).map(|_| f64::from_bits(rng.next_u64())), CHUNK);
}

/// Random values whose exponents land in the range where `%.4f` prints a mix of
/// integer and fractional digits — the region uniform bit fuzzing almost never
/// reaches, and where decimal rounding is decided.
#[test]
fn fuzz_moderate_exponents() {
    let mut rng = SplitMix64(0x1319_8a2e_0370_7344);
    assert_same_chunked(
        (0..1_000_000).map(|_| {
            let r = rng.next_u64();
            let exp = (1023i64 - 45 + ((r >> 52) % 106) as i64) as u64;
            let sign = (r >> 51) & 1;
            f64::from_bits((sign << 63) | (exp << 52) | (r & 0x000f_ffff_ffff_ffff))
        }),
        CHUNK,
    );
}

/// Systematic grid: every one of the 2048 exponent fields crossed with the top
/// nine mantissa bits, for both signs. Covers normals, subnormals, infinities
/// and NaNs uniformly rather than by chance.
#[test]
fn sweep_exponent_by_high_mantissa() {
    let inputs = (0..0x800u64).flat_map(|exp| {
        (0..512u64).flat_map(move |hi| {
            let m = hi << 43;
            [
                f64::from_bits((exp << 52) | m),
                f64::from_bits(0x8000_0000_0000_0000 | (exp << 52) | m),
            ]
        })
    });
    assert_same_chunked(inputs, CHUNK);
}

/// Systematic grid over the low mantissa bits, which drive `%a`'s trailing-zero
/// trimming and the length of the exact `%.4f` expansion.
#[test]
fn sweep_low_mantissa_bits() {
    let exps: [u64; 12] = [0, 1, 2, 0x3fd, 0x3fe, 0x3ff, 0x400, 0x40f, 0x433, 0x7fd, 0x7fe, 0x7ff];
    let inputs = exps.into_iter().flat_map(|exp| {
        (0..0x1_0000u64).flat_map(move |lo| {
            let m = lo | ((lo & 0xf) << 40);
            [
                f64::from_bits((exp << 52) | m),
                f64::from_bits(0x8000_0000_0000_0000 | (exp << 52) | m),
            ]
        })
    });
    assert_same_chunked(inputs, CHUNK);
}

/// Every `f32` value widened to `f64`, sampled across the whole `f32` range.
/// These have long-zero mantissa tails, so they stress `%a`'s trimming while
/// still spanning ~80 decades of magnitude.
#[test]
fn sweep_widened_f32_values() {
    let inputs = (0..u32::MAX / 512).map(|i| {
        let bits = i.wrapping_mul(512);
        f32::from_bits(bits) as f64
    });
    assert_same_chunked(inputs, CHUNK);
}

/// Dense sweep of exact decimal-tie candidates: every multiple of 2^-k that can
/// sit exactly on a `%.4f` rounding boundary, over several decades.
#[test]
fn sweep_decimal_rounding_ties() {
    let inputs = (5..=30u32).flat_map(|k| {
        let denom = (1u64 << k) as f64;
        (0..20_000u64).flat_map(move |n| {
            let v = (2 * n + 1) as f64 / denom;
            [v, -v]
        })
    });
    assert_same_chunked(inputs, CHUNK);
}
