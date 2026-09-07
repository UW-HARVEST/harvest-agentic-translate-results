//! Phase C — error-path / boundary differential tests.
//!
//! One test per row of `ERRORS.md`. `float2half` has no error return (every
//! `uint16_t` is a valid result), so each row asserts that the C and Rust
//! exports agree on the exact returned value for a condition where the Rust
//! translation could otherwise panic, trap, or canonicalise instead of
//! returning what the C returns.

mod harness;

use harness::{bits_from, Pair, Rng, MANTISSA_EDGES, RUNS};

/// ERRORS.md row 1 — table index overrun. The only guard is `& 0x1ff`.
#[test]
fn err01_table_index_never_overruns() {
    let p = Pair::load();
    // The adversarial patterns: all-ones, top-half all-ones, and every j at
    // the very end of each table half.
    for bits in [0xFFFF_FFFFu32, 0x7FFF_FFFF, 0xFF80_0000, 0x7F80_0000] {
        p.assert_bits(bits, "ERRORS row 1 (index overrun probe)");
    }
    // j == 255 and j == 511 (the last element of each table half) with every
    // mantissa edge, plus a randomized sweep over their full mantissa space.
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_F00D);
    for j in [255u32, 511] {
        for &m in MANTISSA_EDGES.iter() {
            p.assert_bits(bits_from(j, m), "ERRORS row 1 (last table element)");
        }
        for _ in 0..200_000 {
            let m = rng.next_u32() & 0x007f_ffff;
            p.assert_bits(bits_from(j, m), "ERRORS row 1 (last table element, random)");
        }
    }
    // And every single j, to prove no index is out of range for any input.
    for j in 0u32..512 {
        p.assert_bits(bits_from(j, 0x007f_ffff), "ERRORS row 1 (all indices)");
    }
}

/// ERRORS.md row 2 — widest shift in the table (0x18 = 24) must not overflow.
#[test]
fn err02_widest_shift_is_well_defined() {
    let p = Pair::load();
    let mut count = 0u32;
    for &(lo, hi, shift, label) in RUNS.iter() {
        if shift != 0x18 {
            continue;
        }
        let what = format!("ERRORS row 2 (shift 0x18 in [{label}])");
        for j in lo..=hi {
            // Maximum mantissa maximises the shifted operand.
            p.assert_bits(bits_from(j, 0x007f_ffff), &what);
            p.assert_bits(bits_from(j, 0x0040_0000), &what);
            p.assert_bits(bits_from(j, 0x0000_0001), &what);
            count += 1;
        }
    }
    assert!(count >= 400, "expected the four shift-0x18 runs, saw {count} j values");
}

/// ERRORS.md row 3 — `uint16_t` truncation of the `uint32_t` sum; worst case
/// `j == 511`, mantissa `0x7fffff` -> `0xfc00 + 0x3ff == 0xffff`.
#[test]
fn err03_u16_truncation_of_sum() {
    let p = Pair::load();
    let bits = bits_from(511, 0x007f_ffff);
    p.assert_bits(bits, "ERRORS row 3 (max sum)");
    let (c, r) = p.both_bits(bits);
    assert_eq!(c, 0xFFFF, "C should produce the maximal 0xFFFF here");
    assert_eq!(r, 0xFFFF);

    // The other maximal-sum candidates.
    for j in [255u32, 398, 142, 511] {
        p.assert_bits(bits_from(j, 0x007f_ffff), "ERRORS row 3 (near-max sum)");
    }
}

/// ERRORS.md row 4 — signalling NaN payloads must survive the FFI unchanged.
#[test]
fn err04_signalling_nan_payloads() {
    let p = Pair::load();
    for bits in [
        0x7FBF_FFFFu32, // +sNaN, max payload
        0xFFBF_FFFF,    // -sNaN, max payload
        0x7F80_0001,    // +sNaN, min payload
        0xFF80_0001,    // -sNaN, min payload
        0x7FA0_0000,
        0xFFA0_0000,
    ] {
        p.assert_bits(bits, "ERRORS row 4 (signalling NaN)");
    }
    let (c, r) = p.both_bits(0x7FBF_FFFF);
    assert_eq!((c, r), (0x7DFF, 0x7DFF), "sNaN payload was altered somewhere");
    let (c, r) = p.both_bits(0xFFBF_FFFF);
    assert_eq!((c, r), (0xFDFF, 0xFDFF));
}

/// ERRORS.md row 5 — quiet NaN payloads, including the payload that aliases
/// onto the infinity encoding. This aliasing is what the C does; do not "fix".
#[test]
fn err05_quiet_nan_payloads_including_inf_alias() {
    let p = Pair::load();
    // A NaN whose payload shifts down to zero collides with +Inf's encoding.
    let (c, r) = p.both_bits(0x7F80_0001);
    assert_eq!(
        (c, r),
        (0x7C00, 0x7C00),
        "the C aliases small-payload NaN onto the Inf encoding; Rust must too"
    );
    let (c, r) = p.both_bits(0xFF80_0001);
    assert_eq!((c, r), (0xFC00, 0xFC00));

    // Exhaustive over the full 2^23 payload space of both NaN halves.
    for m in 0u32..0x0080_0000 {
        p.assert_bits(bits_from(255, m), "ERRORS row 5 (all +NaN payloads)");
        p.assert_bits(bits_from(511, m), "ERRORS row 5 (all -NaN payloads)");
    }
}

/// ERRORS.md row 6 — infinities.
#[test]
fn err06_infinities() {
    let p = Pair::load();
    let (c, r) = p.both_bits(0x7F80_0000);
    assert_eq!((c, r), (0x7C00, 0x7C00), "+Inf");
    let (c, r) = p.both_bits(0xFF80_0000);
    assert_eq!((c, r), (0xFC00, 0xFC00), "-Inf");
    p.assert_val(f32::INFINITY, "ERRORS row 6 (+Inf via value)");
    p.assert_val(f32::NEG_INFINITY, "ERRORS row 6 (-Inf via value)");
}

/// ERRORS.md row 7 — finite overflow, one step past the last representable
/// exponent: `j == 143` / `j == 399`.
#[test]
fn err07_finite_overflow_saturates() {
    let p = Pair::load();
    let (c, r) = p.both_bits(bits_from(143, 0));
    assert_eq!((c, r), (0x7C00, 0x7C00), "j=143 saturates to +Inf encoding");
    let (c, r) = p.both_bits(bits_from(399, 0));
    assert_eq!((c, r), (0xFC00, 0xFC00), "j=399 saturates to -Inf encoding");

    // Mantissa is discarded in this run — check that across the whole mantissa
    // range for both signs.
    let mut rng = Rng::new(0x0BAD_F00D_1234_5678);
    for _ in 0..200_000 {
        let m = rng.next_u32() & 0x007f_ffff;
        p.assert_bits(bits_from(143, m), "ERRORS row 7 (+overflow, random mantissa)");
        p.assert_bits(bits_from(399, m), "ERRORS row 7 (-overflow, random mantissa)");
    }
}

/// ERRORS.md row 8 — underflow to zero one step below the first representable
/// half subnormal: `j == 102` / `j == 358`, sign preserved.
#[test]
fn err08_underflow_preserves_sign() {
    let p = Pair::load();
    let (c, r) = p.both_bits(bits_from(102, 0x007f_ffff));
    assert_eq!((c, r), (0x0000, 0x0000), "+tiny flushes to +0");
    let (c, r) = p.both_bits(bits_from(358, 0x007f_ffff));
    assert_eq!((c, r), (0x8000, 0x8000), "-tiny flushes to -0, not +0");

    let mut rng = Rng::new(0xFEED_FACE_5EED_1111);
    for _ in 0..200_000 {
        let m = rng.next_u32() & 0x007f_ffff;
        p.assert_bits(bits_from(102, m), "ERRORS row 8 (+underflow)");
        p.assert_bits(bits_from(358, m), "ERRORS row 8 (-underflow)");
    }
}

/// ERRORS.md row 9 — signed zero and float subnormals.
#[test]
fn err09_signed_zero_and_float_subnormals() {
    let p = Pair::load();
    let cases: [(u32, u16, &str); 6] = [
        (0x0000_0000, 0x0000, "+0"),
        (0x8000_0000, 0x8000, "-0"),
        (0x0000_0001, 0x0000, "smallest +subnormal f32"),
        (0x8000_0001, 0x8000, "smallest -subnormal f32"),
        (0x007F_FFFF, 0x0000, "largest +subnormal f32"),
        (0x807F_FFFF, 0x8000, "largest -subnormal f32"),
    ];
    for (bits, want, label) in cases {
        p.assert_bits(bits, &format!("ERRORS row 9 ({label})"));
        let (c, r) = p.both_bits(bits);
        assert_eq!(c, want, "unexpected C result for {label}");
        assert_eq!(r, want, "unexpected Rust result for {label}");
    }
}

/// ERRORS.md row 10 — the `j == 102` / `j == 103` (and 358/359) discontinuity,
/// across *all* 2^23 mantissas of each side.
#[test]
fn err10_flush_to_graduated_boundary() {
    let p = Pair::load();
    for j in [102u32, 103, 358, 359] {
        for m in 0u32..0x0080_0000 {
            p.assert_bits(bits_from(j, m), "ERRORS row 10 (flush/graduated boundary)");
        }
    }
}

/// ERRORS.md row 11 — the graduated-underflow / normal boundary
/// (`j == 112` vs `113`, `368` vs `369`), all mantissas.
#[test]
fn err11_graduated_to_normal_boundary() {
    let p = Pair::load();
    for j in [112u32, 113, 368, 369] {
        for m in 0u32..0x0080_0000 {
            p.assert_bits(bits_from(j, m), "ERRORS row 11 (graduated/normal boundary)");
        }
    }
}

/// ERRORS.md row 12 — the last-finite-normal / saturation boundary
/// (`j == 142` vs `143`, `398` vs `399`), all mantissas.
#[test]
fn err12_normal_to_saturation_boundary() {
    let p = Pair::load();
    for j in [142u32, 143, 398, 399] {
        for m in 0u32..0x0080_0000 {
            p.assert_bits(bits_from(j, m), "ERRORS row 12 (normal/saturation boundary)");
        }
    }
}

/// ERRORS.md row 13 — the saturation / Inf-NaN single-element discontinuity at
/// the end of each table half (`j == 254` vs `255`, `510` vs `511`).
#[test]
fn err13_saturation_to_infnan_boundary() {
    let p = Pair::load();
    for j in [254u32, 255, 510, 511] {
        for m in 0u32..0x0080_0000 {
            p.assert_bits(bits_from(j, m), "ERRORS row 13 (saturation/InfNaN boundary)");
        }
    }
    // The defining contrast: mantissa ignored at j=254, honoured at j=255.
    let (c254, r254) = p.both_bits(bits_from(254, 0x007f_ffff));
    let (c255, r255) = p.both_bits(bits_from(255, 0x007f_ffff));
    assert_eq!((c254, r254), (0x7C00, 0x7C00));
    assert_eq!((c255, r255), (0x7FFF, 0x7FFF));
}

/// ERRORS.md rows 14-16 — the generic C-API boundaries, documented as
/// structurally inapplicable and asserted as such against the real header.
///
/// * row 14 (null pointer): `float2half` has no pointer parameter.
/// * row 15 (zero/oversized length): no length parameter.
/// * row 16 (out-of-range enum): no enum/flag parameter.
///
/// The nearest real analogue is "an argument bit pattern with no valid
/// interpretation". Because the whole argument is 32 bits, that class is
/// covered by passing *every* pattern (see `phase_b_exhaustive.rs`). Here we
/// assert the header really does declare a single `float` parameter, so these
/// rows cannot be silently hiding an untested input.
#[test]
fn err14_15_16_generic_boundaries_are_structurally_inapplicable() {
    let header = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src")
            .join("include")
            .join("lib.h"),
    )
    .expect("read c_src/include/lib.h");

    let decls: Vec<&str> = header
        .lines()
        .map(str::trim)
        .filter(|l| l.contains('(') && l.ends_with(';'))
        .collect();
    assert_eq!(
        decls,
        vec!["uint16_t float2half(float flt);"],
        "the public C API changed; rows 14-16 must be re-derived"
    );
    assert!(!header.contains('*'), "a pointer parameter appeared: row 14 now applies");
    assert!(!header.contains("enum"), "an enum parameter appeared: row 16 now applies");
    assert!(
        !header.contains("size") && !header.contains("len"),
        "a length parameter appeared: row 15 now applies"
    );

    // A "no valid variant" argument still crosses the boundary fine on both
    // sides: pass the non-canonical patterns a C caller could hand us.
    let p = Pair::load();
    for bits in [
        0xFFFF_FFFFu32,
        0x7FFF_FFFF,
        0x8000_0000,
        0x0000_0000,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x8080_8080,
        0x5555_5555,
        0xAAAA_AAAA,
    ] {
        p.assert_bits(bits, "ERRORS rows 14-16 (arbitrary argument bit pattern)");
    }
}
