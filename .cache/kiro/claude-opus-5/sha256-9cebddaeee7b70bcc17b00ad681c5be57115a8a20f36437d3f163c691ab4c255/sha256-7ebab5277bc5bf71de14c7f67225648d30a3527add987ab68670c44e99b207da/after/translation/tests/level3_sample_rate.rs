//! Level 3: the sample-rate pipeline.
//!
//! The C does:
//! ```c
//! conv64.u = desc->sample_rate;                          // numeric double -> u64
//! conv64.u = ima_btoh64(*(const ima_u64_t *)&conv64.u);  // byte swap
//! info->sample_rate = conv64.f;                          // bit reinterpretation
//! ```
//! The first line is a *value* conversion, not a bit cast, and is UB in C for
//! NaN / negative / out-of-range inputs. Whatever the compiler actually does
//! must be reproduced, so this file hammers the full 64-bit input space.
//!
//! `desc->sample_rate` is read as a host (little-endian) `double` out of the
//! buffer, so to make the C observe the double `x` the file must contain
//! `x.to_bits().to_le_bytes()`.

mod harness;
use harness::*;

fn stream_with_rate_bytes(raw: [u8; 8]) -> Vec<u8> {
    let desc = desc_body(raw, FOURCC_IMA4, 0, 34, 64, 2, 16);
    Caf::new()
        .valid_header()
        .chunk(FOURCC_DESC, &desc)
        .chunk(FOURCC_PAKT, &pakt_body(0, 0, 0, 0))
        .chunk(FOURCC_DATA, &data_body(0, &[0u8; 34]))
        .build()
}

/// Feeds a raw 64-bit pattern; the C will interpret it as a host double.
#[track_caller]
fn check_bits(label: &str, bits: u64) {
    let bytes = stream_with_rate_bytes(bits.to_le_bytes());
    let out = assert_same(label, &bytes);
    assert_eq!(out.ret, 0, "{label}");
}

/// Feeds a value such that the C reads exactly the double `v`.
#[track_caller]
fn check_double(v: f64) {
    check_bits(&format!("double {v:?} (bits {:#018x})", v.to_bits()), v.to_bits());
}

// ---------------------------------------------------------------------------

#[test]
fn realistic_sample_rates_as_stored_big_endian() {
    // What a real CAF file contains: the rate in big-endian. The C reads it
    // natively, so these are all garbage doubles -- exactly the bug to keep.
    for rate in [
        8000.0f64, 11025.0, 16000.0, 22050.0, 24000.0, 32000.0, 44100.0, 48000.0, 88200.0,
        96000.0, 176400.0, 192000.0, 384000.0, 1.0, 0.5, 0.0, -0.0, -44100.0,
    ] {
        let bytes = stream_with_rate_bytes(rate.to_bits().to_be_bytes());
        let out = assert_same(&format!("BE rate {rate}"), &bytes);
        assert_eq!(out.ret, 0);
    }
}

#[test]
fn special_doubles() {
    let specials: &[f64] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.4999999999999999,
        1.5,
        -1.5,
        2.5,
        -2.5,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::EPSILON,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        44100.0,
        48000.0,
        1e300,
        -1e300,
        1e-300,
        -1e-300,
        // Around 2^63 (the signed/unsigned pivot in the generated code).
        9223372036854775808.0,       // 2^63
        9223372036854774784.0,       // largest double < 2^63
        9223372036854777856.0,       // next double above 2^63
        -9223372036854775808.0,      // -2^63
        -9223372036854777856.0,      // just below -2^63
        // Around 2^64.
        18446744073709549568.0,      // largest double < 2^64
        18446744073709551616.0,      // 2^64
        18446744073709555712.0,      // just above 2^64
        // Around 2^32 / 2^31.
        2147483647.0,
        2147483648.0,
        -2147483648.0,
        -2147483649.0,
        4294967295.0,
        4294967296.0,
        // Around 2^53 (exact-integer limit).
        9007199254740992.0,
        9007199254740993.0,
        -9007199254740992.0,
    ];
    for &v in specials {
        check_double(v);
    }
}

/// Every NaN / infinity payload class, plus signalling NaNs.
#[test]
fn nan_and_infinity_payloads() {
    let mut rng = Rng::new(0x4E_414E);
    for sign in [0u64, 1u64 << 63] {
        // infinity
        check_bits("inf", sign | 0x7FF0_0000_0000_0000);
        // quiet + signalling NaNs with assorted payloads
        for payload in [
            1u64,
            2,
            0x7FFFF,
            0x8_0000_0000_0000,
            0xF_FFFF_FFFF_FFFF,
            0x5_5555_5555_5555,
        ] {
            check_bits("nan", sign | 0x7FF0_0000_0000_0000 | payload);
        }
        for _ in 0..64 {
            let payload = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
            if payload == 0 {
                continue;
            }
            check_bits("nan rand", sign | 0x7FF0_0000_0000_0000 | payload);
        }
    }
}

/// Sweep every one of the 2048 biased exponents, both signs, several
/// mantissas. This covers subnormals, all in-range magnitudes, the huge
/// out-of-range magnitudes and the NaN/Inf exponent.
#[test]
fn full_exponent_sweep() {
    const MANTISSAS: [u64; 6] = [
        0,
        1,
        0x8_0000_0000_0000,
        0xF_FFFF_FFFF_FFFF,
        0x5_5555_5555_5555,
        0xA_AAAA_AAAA_AAAA,
    ];
    for exp in 0u64..2048 {
        for &m in &MANTISSAS {
            for sign in [0u64, 1u64 << 63] {
                let bits = sign | (exp << 52) | m;
                check_bits(&format!("exp={exp} m={m:#x} sign={}", sign >> 63), bits);
            }
        }
    }
}

/// Dense sweep of the exponents that straddle the interesting boundaries:
/// values just below / at / above 2^63 and 2^64, and fractional values around
/// zero where truncation matters.
#[test]
fn boundary_exponent_dense_sweep() {
    // Biased exponents 1010..1090 cover roughly 2^-13 .. 2^67.
    for exp in 1010u64..=1090 {
        for m in 0u64..64 {
            for sign in [0u64, 1u64 << 63] {
                // spread the mantissa bits across the field
                let mant = (m << 46) | (m << 20) | m;
                let bits = sign | (exp << 52) | (mant & 0x000F_FFFF_FFFF_FFFF);
                check_bits(&format!("dense exp={exp} m={m} sign={}", sign >> 63), bits);
            }
        }
    }
}

/// Exhaustively walk the top 16 bits (sign + exponent + 5 mantissa bits) with
/// a fixed low half, then again with the low half set.
#[test]
fn top_bits_exhaustive() {
    for hi in 0u64..=0xFFFF {
        for low in [0u64, 0xFFFF_FFFF_FFFF, 0x1] {
            let bits = (hi << 48) | low;
            check_bits(&format!("hi={hi:#06x} low={low:#x}"), bits);
        }
    }
}

#[test]
fn random_bit_patterns() {
    let mut rng = Rng::new(0xDEAD_BEEF_1234);
    for i in 0..50_000 {
        check_bits(&format!("rand #{i}"), rng.next_u64());
    }
}

/// Random patterns biased toward "plausible double" magnitudes so the
/// conversion actually lands inside the u64 range often.
#[test]
fn random_in_range_magnitudes() {
    let mut rng = Rng::new(0x01E_4A46E_u64);
    for i in 0..20_000 {
        let exp = 1023 + (rng.below(130) as i64 - 5); // ~2^-5 .. 2^124
        let mant = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        let sign = (rng.next_u64() & 1) << 63;
        let bits = sign | ((exp as u64) << 52) | mant;
        check_bits(&format!("in-range rand #{i}"), bits);
    }
}

/// Integral doubles across the whole representable integer range, both signs.
#[test]
fn integral_doubles_powers_of_two() {
    for p in 0i32..=1030 {
        let v = (2.0f64).powi(p);
        check_double(v);
        check_double(-v);
        check_double(v + 1.0);
        check_double(v - 1.0);
        check_double(-(v + 1.0));
        check_double(-(v - 1.0));
        check_double(v * 1.5);
        check_double(-(v * 1.5));
    }
    for p in -1080i32..0 {
        let v = (2.0f64).powi(p);
        check_double(v);
        check_double(-v);
    }
}

/// The byte-swap-then-reinterpret tail: confirm the written 8 bytes are the
/// big-endian image of the converted integer for a few hand-computed cases,
/// so the test would catch a *matching pair* of wrong implementations.
#[test]
fn conversion_pipeline_is_swapped_big_endian() {
    // 5.0 truncates to 5; the stored image is the big-endian byte order of 5.
    let bytes = stream_with_rate_bytes(5.0f64.to_bits().to_le_bytes());
    let out = assert_same("5.0", &bytes);
    assert_eq!(out.ret, 0);
    assert_eq!(out.info.sample_rate_bits(), 5u64.swap_bytes());

    // 0.9 truncates to 0.
    let bytes = stream_with_rate_bytes(0.9f64.to_bits().to_le_bytes());
    let out = assert_same("0.9", &bytes);
    assert_eq!(out.info.sample_rate_bits(), 0);

    // -1.0 -> cvttsd2si -> -1 -> 0xFFFF...FF -> swap is still all ones.
    let bytes = stream_with_rate_bytes((-1.0f64).to_bits().to_le_bytes());
    let out = assert_same("-1.0", &bytes);
    assert_eq!(out.info.sample_rate_bits(), u64::MAX);

    // An exactly representable integer with distinct bytes.
    let v = 72623859706101760.0f64; // 0x0102030400000000
    assert_eq!(v as u64, 0x0102_0304_0000_0000u64);
    let bytes = stream_with_rate_bytes(v.to_bits().to_le_bytes());
    let out = assert_same("0x0102030400000000", &bytes);
    assert_eq!(
        out.info.sample_rate_bits(),
        0x0102_0304_0000_0000u64.swap_bytes()
    );
}
