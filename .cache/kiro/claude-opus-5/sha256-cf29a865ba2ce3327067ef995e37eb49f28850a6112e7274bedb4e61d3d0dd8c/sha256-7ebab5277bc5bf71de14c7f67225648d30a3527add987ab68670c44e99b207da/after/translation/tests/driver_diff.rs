//! Differential tests for `void driver(double)`.
//!
//! `driver` is the only public entry point (see `c_src/include/driver.h`), and
//! it is a leaf: it reinterprets the `double`'s bits and runs one `printf` with
//! `"%llx %a %.4f\n"`. The tests below therefore attack the three conversions
//! independently — raw bits (`%llx`), hex float (`%a`), and fixed decimal
//! (`%.4f`) — plus the interesting float classes and a bit-level fuzz sweep.

mod common;

use common::*;

/// Hand-picked values covering every IEEE-754 class and the obvious edges.
fn special_values() -> Vec<f64> {
    let mut v: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        2.0,
        -2.0,
        0.5,
        -0.5,
        3.0,
        -3.0,
        10.0,
        100.0,
        1e4,
        1e5,
        0.1,
        -0.1,
        0.2,
        0.3,
        1.0 / 3.0,
        2.0 / 3.0,
        std::f64::consts::PI,
        -std::f64::consts::PI,
        std::f64::consts::E,
        std::f64::consts::LN_2,
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::EPSILON,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
    ];

    // Raw bit patterns: smallest/largest subnormals, the normal boundary, all
      // -ones mantissas, quiet and signalling NaN payloads, and both NaN signs.
    let bit_patterns: [u64; 24] = [
        0x0000_0000_0000_0000, // +0
        0x8000_0000_0000_0000, // -0
        0x0000_0000_0000_0001, // smallest positive subnormal
        0x8000_0000_0000_0001, // smallest negative subnormal
        0x0000_0000_0000_0002,
        0x0000_0000_0000_00ff,
        0x0000_8000_0000_0000, // mid subnormal
        0x000f_ffff_ffff_ffff, // largest subnormal
        0x800f_ffff_ffff_ffff,
        0x0010_0000_0000_0000, // smallest normal
        0x0010_0000_0000_0001,
        0x7fef_ffff_ffff_ffff, // f64::MAX
        0xffef_ffff_ffff_ffff, // f64::MIN
        0x7ff0_0000_0000_0000, // +inf
        0xfff0_0000_0000_0000, // -inf
        0x7ff8_0000_0000_0000, // quiet NaN
        0xfff8_0000_0000_0000, // negative quiet NaN
        0x7ff0_0000_0000_0001, // signalling NaN
        0xfff0_0000_0000_0001,
        0x7ff4_0000_0000_0000,
        0x7fff_ffff_ffff_ffff,
        0xffff_ffff_ffff_ffff,
        0x3ff0_0000_0000_0001, // 1.0 + 1 ulp
        0x3fef_ffff_ffff_ffff, // 1.0 - 1 ulp
    ];
    v.extend(bit_patterns.iter().copied().map(f64::from_bits));
    v
}

#[test]
fn special_values_match() {
    assert_same_all(special_values());
}

/// `%llx` must reproduce the raw 64-bit payload; walking a single set bit
/// through all 64 positions exercises every nibble of the hex conversion,
/// including the leading-zero-suppression behaviour.
#[test]
fn single_bit_patterns_match() {
    let mut inputs = Vec::new();
    for bit in 0..64u32 {
        let b = 1u64 << bit;
        inputs.push(f64::from_bits(b));
        inputs.push(f64::from_bits(!b));
        inputs.push(f64::from_bits(b | 0x3ff0_0000_0000_0000));
    }
    assert_same_all(inputs);
}

/// Every possible biased exponent, with a few mantissas each: this covers the
/// whole `%a` exponent range (`p-1022` .. `p+1023`), the subnormal and
/// infinity/NaN exponent fields, and the full `%.4f` magnitude range.
#[test]
fn every_exponent_field_matches() {
    let mantissas: [u64; 6] = [
        0x0_0000_0000_0000,
        0x0_0000_0000_0001,
        0x8_0000_0000_0000,
        0xa_bcde_f012_3456,
        0xf_ffff_ffff_fffe,
        0xf_ffff_ffff_ffff,
    ];
    let mut inputs = Vec::new();
    for exp in 0..0x800u64 {
        for &m in &mantissas {
            inputs.push(f64::from_bits((exp << 52) | m));
            inputs.push(f64::from_bits(0x8000_0000_0000_0000 | (exp << 52) | m));
        }
    }
    assert_same_all(inputs);
}

/// Powers of two across the entire representable range, plus their negations
/// and neighbours. These are the values where `%a` drops the radix point
/// entirely and where `%.4f` flips between `0.0000` and a long digit string.
#[test]
fn powers_of_two_match() {
    let mut inputs = Vec::new();
    for e in -1074i32..=1023 {
        let v = if e < -1022 {
            f64::from_bits(1u64 << (e + 1074))
        } else {
            f64::from_bits(((e + 1023) as u64) << 52)
        };
        inputs.push(v);
        inputs.push(-v);
        inputs.push(f64::from_bits(v.to_bits() + 1));
        if v.to_bits() > 0 {
            inputs.push(f64::from_bits(v.to_bits() - 1));
        }
    }
    assert_same_all(inputs);
}

/// `%a` trims trailing zero hex digits and omits the radix point when the
/// fraction is empty. Sweeping mantissas that end in progressively more zero
/// nibbles pins that logic down for all 13 hex digits.
#[test]
fn hex_float_trailing_zero_trimming_matches() {
    let mut inputs = Vec::new();
    for nibbles in 0..=13u32 {
        for pat in [0x1u64, 0x8, 0xf, 0x9, 0xa] {
            let m = if nibbles == 0 {
                0
            } else {
                (pat << (4 * (nibbles - 1))) & 0x000f_ffff_ffff_ffff
            };
            for exp in [0u64, 1, 0x3ff, 0x400, 0x7fe] {
                inputs.push(f64::from_bits((exp << 52) | m));
                inputs.push(f64::from_bits(0x8000_0000_0000_0000 | (exp << 52) | m));
            }
        }
    }
    // Every "one nibble set, rest zero" mantissa.
    for nib in 0..13u32 {
        for val in 1..16u64 {
            let m = val << (4 * nib);
            inputs.push(f64::from_bits(0x3ff0_0000_0000_0000 | m));
            inputs.push(f64::from_bits(0x0000_0000_0000_0000 | m));
        }
    }
    assert_same_all(inputs);
}

/// Exact halfway cases for `%.4f`. A `double` with exactly five fractional bits
/// is a multiple of 1/32, and every odd multiple of 1/32 ends in `...5` at the
/// fifth decimal place — i.e. an exact tie that must be broken the same way by
/// both implementations (glibc rounds half to even).
#[test]
fn fixed_decimal_exact_ties_match() {
    let mut inputs = Vec::new();
    for k in 0..4096i64 {
        let v = k as f64 / 32.0;
        inputs.push(v);
        inputs.push(-v);
    }
    // Ties further out: multiples of 1/2^n for n = 5..=20 keep producing
    // decimals whose tail sits right at or below the rounding position.
    for n in 5..=24u32 {
        let denom = (1u64 << n) as f64;
        for k in [1i64, 3, 5, 7, 9, 11, 13, 15, 17, 31, 33, 63, 127, 255, 1023] {
            let v = k as f64 / denom;
            inputs.push(v);
            inputs.push(-v);
        }
    }
    assert_same_all(inputs);
}

/// Values clustered just above and below the `%.4f` rounding boundary, where a
/// one-ulp difference decides the printed digits.
#[test]
fn fixed_decimal_boundary_neighbours_match() {
    let mut inputs = Vec::new();
    for k in 0..2000i64 {
        // x.xxxx5 targets, approached from both sides in ulps.
        let base = (k as f64) * 1e-4 + 0.5e-4;
        for delta in -3i64..=3 {
            let bits = base.to_bits();
            let shifted = if delta >= 0 {
                bits.wrapping_add(delta as u64)
            } else {
                bits.wrapping_sub((-delta) as u64)
            };
            let v = f64::from_bits(shifted);
            inputs.push(v);
            inputs.push(-v);
        }
    }
    // Values just under 1e-4 / 5e-5, where the result flips to 0.0000.
    for k in 1..500i64 {
        let v = (k as f64) * 1e-7;
        inputs.push(v);
        inputs.push(-v);
    }
    assert_same_all(inputs);
}

/// Very large magnitudes: `%.4f` has to print the full exact integer expansion
/// (up to ~309 digits) followed by four zeros.
#[test]
fn huge_magnitudes_match() {
    let mut inputs = Vec::new();
    for e in 0..=308i32 {
        let v = 10f64.powi(e);
        inputs.push(v);
        inputs.push(-v);
        inputs.push(f64::from_bits(v.to_bits() + 1));
        inputs.push(f64::from_bits(v.to_bits() - 1));
    }
    for e in 60..=1023i32 {
        let v = f64::from_bits(((e + 1023) as u64) << 52) * 1.0;
        inputs.push(v);
        inputs.push(v * 1.5);
        inputs.push(-v);
    }
    inputs.push(f64::MAX);
    inputs.push(f64::MIN);
    inputs.push(f64::from_bits(0x7fef_ffff_ffff_fffe));
    assert_same_all(inputs);
}

/// Very small magnitudes: `%.4f` collapses these to `0.0000` / `-0.0000`, while
/// `%a` still has to spell out the subnormal exponent.
#[test]
fn tiny_magnitudes_match() {
    let mut inputs = Vec::new();
    for e in -320..=-1i32 {
        let v = 10f64.powi(e);
        inputs.push(v);
        inputs.push(-v);
    }
    for bit in 0..52u32 {
        let v = f64::from_bits(1u64 << bit);
        inputs.push(v);
        inputs.push(-v);
        inputs.push(f64::from_bits(v.to_bits() | 0xa5));
    }
    assert_same_all(inputs);
}

/// Uniform random bit patterns: covers arbitrary NaN payloads, arbitrary
/// subnormals, and arbitrary normals at every exponent.
#[test]
fn random_bit_patterns_match() {
    let mut rng = SplitMix64(0x1234_5678_9abc_def0);
    let mut inputs = Vec::with_capacity(20_000);
    for _ in 0..20_000 {
        inputs.push(f64::from_bits(rng.next_u64()));
    }
    assert_same_all(inputs);
}

/// Random values restricted to exponents where `%.4f` prints something
/// interesting (roughly 1e-8 .. 1e12), which the uniform bit sweep almost never
/// hits.
#[test]
fn random_moderate_values_match() {
    let mut rng = SplitMix64(0xdead_beef_cafe_f00d);
    let mut inputs = Vec::with_capacity(20_000);
    for _ in 0..20_000 {
        let r = rng.next_u64();
        // Biased exponent chosen from ~[2^-30, 2^40].
        let exp = 1023i64 - 30 + ((r >> 52) % 71) as i64;
        let mantissa = r & 0x000f_ffff_ffff_ffff;
        let sign = (r >> 51) & 1;
        inputs.push(f64::from_bits(
            (sign << 63) | ((exp as u64) << 52) | mantissa,
        ));
    }
    assert_same_all(inputs);
}

/// Random subnormals only — the `%a` path where the leading hex digit is `0`
/// and the exponent is pinned at `p-1022`.
#[test]
fn random_subnormals_match() {
    let mut rng = SplitMix64(0x0f0f_0f0f_5555_aaaa);
    let mut inputs = Vec::with_capacity(10_000);
    for _ in 0..10_000 {
        let r = rng.next_u64();
        let bits = (r & 0x000f_ffff_ffff_ffff) | (((r >> 63) & 1) << 63);
        inputs.push(f64::from_bits(bits));
    }
    assert_same_all(inputs);
}

/// Random NaN and infinity encodings, both signs, all payloads.
#[test]
fn random_nonfinite_match() {
    let mut rng = SplitMix64(0xabad_1dea_1234_4321);
    let mut inputs = Vec::with_capacity(4_000);
    for _ in 0..4_000 {
        let r = rng.next_u64();
        let bits = 0x7ff0_0000_0000_0000u64 | (r & 0x000f_ffff_ffff_ffff) | (((r >> 63) & 1) << 63);
        inputs.push(f64::from_bits(bits));
    }
    inputs.push(f64::from_bits(0x7ff0_0000_0000_0000));
    inputs.push(f64::from_bits(0xfff0_0000_0000_0000));
    assert_same_all(inputs);
}

/// Integers and half-integers, which `%.4f` prints with a fixed `.0000` /
/// `.5000` tail and `%a` prints with short mantissas.
#[test]
fn integral_values_match() {
    let mut inputs = Vec::new();
    for k in -1000i64..=1000 {
        inputs.push(k as f64);
        inputs.push(k as f64 + 0.5);
        inputs.push(k as f64 + 0.25);
        inputs.push(k as f64 + 0.0001);
    }
    for k in [
        1i64 << 20,
        1 << 30,
        1 << 40,
        1 << 52,
        (1i64 << 53) - 1,
        1 << 53,
        (1i64 << 53) + 2,
        i64::MAX / 2,
    ] {
        inputs.push(k as f64);
        inputs.push(-(k as f64));
    }
    assert_same_all(inputs);
}

/// The output must always be a single line with exactly three whitespace
/// separated fields, and both libraries must agree on it. This guards against a
/// translation that gets the values right but the framing wrong.
#[test]
fn output_framing_matches() {
    for f in special_values() {
        let c = c_output(f);
        let r = rust_output(f);
        assert_eq!(c, r, "bits {:#018x}", f.to_bits());
        let s = String::from_utf8(c).expect("output is UTF-8");
        assert_eq!(s.matches('\n').count(), 1, "not one line: {s:?}");
        assert!(s.ends_with('\n'), "no trailing newline: {s:?}");
        assert_eq!(
            s.trim_end().split(' ').count(),
            3,
            "not three space separated fields: {s:?}"
        );
    }
}
