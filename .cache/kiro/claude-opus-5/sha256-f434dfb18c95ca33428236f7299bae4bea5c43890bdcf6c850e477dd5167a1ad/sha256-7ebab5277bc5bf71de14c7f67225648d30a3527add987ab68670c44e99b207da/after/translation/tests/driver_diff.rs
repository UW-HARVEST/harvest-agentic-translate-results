//! Differential tests for `void driver(float x)`.
//!
//! `driver` is the only symbol the C library exports; the helper
//! `static void print_hex(unsigned char *p, int len)` has internal linkage and
//! is therefore only reachable through `driver`. The tests below start with the
//! byte-formatting behaviour that `print_hex` is responsible for (single
//! values, one line at a time) and build up to large sweeps and repeated calls
//! through the public entry point.

mod common;

use common::{assert_same, assert_same_batch, capture_stdout, run_both, Libs, SplitMix64};

// ---------------------------------------------------------------------------
// Level 0: the libraries load and export the entry point at all.
// ---------------------------------------------------------------------------

#[test]
fn both_libraries_export_driver() {
    let libs = Libs::load();
    // Loading already asserts the symbol lookups succeeded; touch the pointers
    // so the compiler cannot elide them.
    assert!(!(libs.c_driver as *const ()).is_null());
    assert!(!(libs.rust_driver as *const ()).is_null());
}

// ---------------------------------------------------------------------------
// Level 1: `print_hex` formatting, observed one call at a time.
// ---------------------------------------------------------------------------

#[test]
fn matches_for_simple_values() {
    let libs = Libs::load();
    for &x in &[
        0.0f32, -0.0, 1.0, -1.0, 2.0, -2.0, 0.5, -0.5, 3.14159265, -3.14159265, 42.0, -42.0,
        1e-10, 1e10, 255.0, 256.0, 65535.0, 16777216.0,
    ] {
        assert_same(&libs, x, &format!("{:?}", x));
    }
}

#[test]
fn output_shape_is_eight_hex_digits_and_newline() {
    // Locks in the exact byte layout so a regression in either library shows up
    // as a shape failure rather than a silent match.
    let libs = Libs::load();
    let (c_out, rust_out) = run_both(&libs, 1.0f32);
    assert_eq!(c_out, rust_out);
    assert_eq!(c_out.len(), 9, "4 bytes * 2 hex digits + '\\n'");
    assert_eq!(*c_out.last().unwrap(), b'\n');
    assert!(c_out[..8]
        .iter()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)));

    // And the digits really are the native-endian bytes of the float.
    let expected: String = 1.0f32
        .to_ne_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect();
    assert_eq!(&c_out[..8], expected.as_bytes());
}

#[test]
fn matches_for_special_values() {
    let libs = Libs::load();
    let specials: &[(f32, &str)] = &[
        (f32::INFINITY, "+inf"),
        (f32::NEG_INFINITY, "-inf"),
        (f32::NAN, "default NaN"),
        (-f32::NAN, "negated default NaN"),
        (f32::MAX, "MAX"),
        (f32::MIN, "MIN"),
        (f32::MIN_POSITIVE, "MIN_POSITIVE"),
        (f32::EPSILON, "EPSILON"),
        (f32::from_bits(0x0000_0001), "smallest subnormal"),
        (f32::from_bits(0x8000_0001), "smallest negative subnormal"),
        (f32::from_bits(0x007f_ffff), "largest subnormal"),
        (f32::from_bits(0x7f80_0001), "signalling NaN"),
        (f32::from_bits(0xff80_0001), "negative signalling NaN"),
        (f32::from_bits(0x7fc0_0000), "quiet NaN"),
        (f32::from_bits(0x7fff_ffff), "all-payload NaN"),
        (f32::from_bits(0xffff_ffff), "all bits set"),
        (f32::from_bits(0x0000_0000), "all bits clear"),
    ];
    for &(x, label) in specials {
        assert_same(&libs, x, label);
    }
}

#[test]
fn matches_for_every_single_byte_pattern() {
    // Every byte value 0x00..=0xff must format identically, in each of the four
    // byte positions `print_hex` walks. This is the direct test of the
    // "%02x" / int-promotion behaviour for the full unsigned char range.
    let libs = Libs::load();
    let mut values = Vec::new();
    for pos in 0..4u32 {
        for b in 0..=0xffu32 {
            values.push(f32::from_bits(b << (8 * pos)));
        }
    }
    assert_same_batch(&libs, &values, "all byte patterns in all positions");
}

// ---------------------------------------------------------------------------
// Level 2: the public `driver` entry point over wide input sweeps.
// ---------------------------------------------------------------------------

#[test]
fn matches_for_exhaustive_exponent_and_boundary_bits() {
    let libs = Libs::load();
    let mut values = Vec::new();

    // Every exponent, with a few mantissa patterns, both signs.
    for sign in 0..2u32 {
        for exp in 0..256u32 {
            for &mant in &[0u32, 1, 0x0040_0000, 0x007f_ffff, 0x0055_5555] {
                values.push(f32::from_bits((sign << 31) | (exp << 23) | mant));
            }
        }
    }

    // Single-bit and single-bit-clear patterns.
    for bit in 0..32u32 {
        values.push(f32::from_bits(1u32 << bit));
        values.push(f32::from_bits(!(1u32 << bit)));
    }

    assert_same_batch(&libs, &values, "exponent and boundary bit sweep");
}

#[test]
fn matches_for_pseudorandom_bit_patterns() {
    let libs = Libs::load();
    let mut rng = SplitMix64(0x1234_5678_9abc_def0);
    let values: Vec<f32> = (0..200_000).map(|_| f32::from_bits(rng.next_u32())).collect();
    assert_same_batch(&libs, &values, "200k random bit patterns");
}

#[test]
fn matches_for_dense_low_bit_sweep() {
    // Contiguous bit patterns exercise carries across all four printed bytes.
    let libs = Libs::load();
    let values: Vec<f32> = (0..70_000u32).map(f32::from_bits).collect();
    assert_same_batch(&libs, &values, "bits 0x00000000..0x00011170");
}

#[test]
fn matches_for_values_near_representable_integers() {
    let libs = Libs::load();
    let mut values = Vec::new();
    for i in -2000i32..=2000 {
        let x = i as f32;
        values.push(x);
        values.push(f32::from_bits(x.to_bits().wrapping_add(1)));
        values.push(f32::from_bits(x.to_bits().wrapping_sub(1)));
    }
    assert_same_batch(&libs, &values, "integers +/- 1 ulp");
}

#[test]
fn matches_for_strided_sweep_of_whole_bit_space() {
    // A coprime stride walks the entire 2^32 bit space; ~1M samples keeps the
    // captured output at a manageable size while touching every exponent,
    // mantissa region and sign.
    let libs = Libs::load();
    const STRIDE: u32 = 4093; // prime, coprime with 2^32
    let mut values = Vec::with_capacity(1_048_576);
    let mut bits: u32 = 0;
    for _ in 0..1_048_576u32 {
        values.push(f32::from_bits(bits));
        bits = bits.wrapping_add(STRIDE);
    }
    assert_same_batch(&libs, &values, "strided sweep of the 2^32 bit space");
}

// ---------------------------------------------------------------------------
// Level 3: statefulness / repeated invocation.
// ---------------------------------------------------------------------------

#[test]
fn repeated_calls_produce_independent_lines() {
    // `driver` must be stateless: N calls yield N identical lines in both
    // libraries, with no leftover buffering differences.
    let libs = Libs::load();
    let c_out = capture_stdout(|| {
        for _ in 0..64 {
            unsafe { (libs.c_driver)(-7.5f32) }
        }
    });
    let rust_out = capture_stdout(|| {
        for _ in 0..64 {
            unsafe { (libs.rust_driver)(-7.5f32) }
        }
    });
    assert_eq!(c_out, rust_out);
    assert_eq!(c_out.iter().filter(|&&b| b == b'\n').count(), 64);
}

#[test]
fn interleaved_calls_match() {
    // Alternate between the two libraries inside one capture; the combined
    // stream must be pairs of identical lines.
    let libs = Libs::load();
    let mut rng = SplitMix64(0xdead_beef_cafe_f00d);
    let values: Vec<f32> = (0..500).map(|_| f32::from_bits(rng.next_u32())).collect();

    let out = capture_stdout(|| {
        for &x in &values {
            unsafe {
                (libs.c_driver)(x);
                (libs.rust_driver)(x);
            }
        }
    });

    let lines: Vec<&[u8]> = out.split(|&b| b == b'\n').filter(|l| !l.is_empty()).collect();
    assert_eq!(lines.len(), values.len() * 2);
    for (i, pair) in lines.chunks(2).enumerate() {
        assert_eq!(
            pair[0],
            pair[1],
            "interleaved mismatch for input bits {:#010x}",
            values[i].to_bits()
        );
    }
}
