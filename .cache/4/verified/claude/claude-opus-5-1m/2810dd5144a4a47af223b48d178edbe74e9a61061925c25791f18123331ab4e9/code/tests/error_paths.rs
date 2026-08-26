//! Phase C — error-path / boundary differential tests.
//!
//! `ERRORS.md` derives an EMPTY error-surface table: `float2half` takes one
//! `float` by value, has no `if`/`switch`/`assert`/`return -1`/`NULL`, no
//! pointer, length or enum parameter, and exactly one unconditional `return`.
//! There is therefore no input it rejects and no error code to compare.
//!
//! Accordingly this file discharges the generic boundaries Phase C mandates
//! anyway (rows G1..G11 of `ERRORS.md`): degenerate values, values one step
//! past every documented/derived range boundary, the derived index `j` at and
//! past its extremes, and the raw-bit patterns that a C caller can legally
//! hand across the FFI boundary. Any divergence here is the equivalent of an
//! error-path mismatch.

mod common;

use common::{bits_from, libs, Rng};

/// Structural re-verification of the *claim* that the error table is empty:
/// the C source really does contain no rejection construct. If someone later
/// adds one to `c_src`, this test fails and forces `ERRORS.md` to be redone.
#[test]
fn errors_md_empty_table_is_still_justified() {
    let src = std::fs::read_to_string(common::manifest_dir().join("c_src/src/lib.c"))
        .expect("read c_src/src/lib.c");
    // Strip the two big hex tables so their `0x....` entries don't confuse us.
    let code: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("0x"))
        .collect::<Vec<_>>()
        .join("\n");

    // Tokenise on non-identifier characters so that e.g. `m__shift` does not
    // look like the keyword `if`.
    let tokens: std::collections::BTreeSet<&str> = code
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|t| !t.is_empty())
        .collect();

    // Keywords / identifiers that would introduce a branch or a rejection.
    for kw in [
        "if", "else", "switch", "case", "default", "while", "for", "goto", "assert",
        "static_assert", "NULL", "nullptr", "errno", "abort", "exit", "malloc", "calloc",
        "realloc", "free", "longjmp", "raise", "perror",
    ] {
        assert!(
            !tokens.contains(kw),
            "c_src/src/lib.c now contains the token `{kw}` \
             -- ERRORS.md's empty error table is no longer valid and must be regenerated"
        );
    }
    // Character-level constructs: ternary branch, conditional compilation,
    // pointer parameters, negative (error-sentinel) returns.
    for seq in ["?", "#if", "#el", "return -", "return(-", "*"] {
        assert!(
            !code.contains(seq),
            "c_src/src/lib.c now contains `{seq}` \
             -- ERRORS.md's empty error table is no longer valid and must be regenerated"
        );
    }
    assert_eq!(
        code.matches("return").count(),
        1,
        "expected exactly one unconditional return in the C source"
    );
    // Exactly one parameter, passed by value, not a pointer.
    let hdr = std::fs::read_to_string(common::manifest_dir().join("c_src/include/lib.h")).unwrap();
    assert!(hdr.contains("uint16_t float2half(float flt);"));
    assert!(!hdr.contains('*'), "no pointer parameter => no null-pointer path");
}

/// G1 — zero and signed zero.
#[test]
fn g1_zero_and_signed_zero() {
    let l = libs();
    l.assert_same_bits(0x0000_0000, "G1 +0.0");
    l.assert_same_bits(0x8000_0000, "G1 -0.0");
    assert_eq!(l.c(0.0), 0x0000);
    assert_eq!(l.c(-0.0), 0x8000);
    assert_eq!(l.rust(0.0), 0x0000);
    assert_eq!(l.rust(-0.0), 0x8000);
}

/// G2 — the whole f32 subnormal range (exp == 0), which underflows to zero.
/// Endpoints plus a randomized sweep of the 2^23-1 subnormal payloads.
#[test]
fn g2_f32_subnormal_underflow() {
    let l = libs();
    for sign in [0u32, 0x8000_0000] {
        for mant in [1u32, 2, 0x40_0000, 0x7F_FFFE, 0x7F_FFFF] {
            l.assert_same_bits(sign | mant, "G2 subnormal endpoint");
        }
    }
    let mut rng = Rng::new(0x0002_0002);
    for _ in 0..200_000 {
        let sign = (rng.next_u32() & 1) << 31;
        let mant = (rng.next_u32() & 0x007f_ffff).max(1);
        l.assert_same_bits(sign | mant, "G2 subnormal random");
    }
}

/// G3 — infinities.
#[test]
fn g3_infinities() {
    let l = libs();
    l.assert_same_bits(0x7F80_0000, "G3 +Inf");
    l.assert_same_bits(0xFF80_0000, "G3 -Inf");
    assert_eq!(l.c(f32::INFINITY), 0x7C00);
    assert_eq!(l.c(f32::NEG_INFINITY), 0xFC00);
}

/// G4 — quiet NaN, both signs.
#[test]
fn g4_quiet_nan() {
    let l = libs();
    l.assert_same_bits(0x7FC0_0000, "G4 +qNaN");
    l.assert_same_bits(0xFFC0_0000, "G4 -qNaN");
}

/// G5 — signalling NaN and saturated NaN payloads. A translation that let the
/// FPU quiet the sNaN (or canonicalised it in `to_bits`) would diverge here.
/// Sweeps EVERY one of the 2^23-1 NaN payloads for both signs.
#[test]
fn g5_signalling_nan_and_all_payloads_not_quieted() {
    let l = libs();
    l.assert_same_bits(0x7F80_0001, "G5 +sNaN min payload");
    l.assert_same_bits(0xFF80_0001, "G5 -sNaN min payload");
    l.assert_same_bits(0x7FFF_FFFF, "G5 +NaN all-ones payload");
    l.assert_same_bits(0xFFFF_FFFF, "G5 -NaN all-ones payload");
    // Exhaustive over the NaN/Inf exponent (j = 255 and j = 511).
    for j in [255u32, 511] {
        for mant in 0u32..=0x007f_ffff {
            let bits = bits_from(j, mant);
            let c = l.c(f32::from_bits(bits));
            let r = l.rust(f32::from_bits(bits));
            assert_eq!(
                c, r,
                "G5 NaN payload divergence: bits=0x{bits:08X} C=0x{c:04X} Rust=0x{r:04X}"
            );
        }
    }
}

/// G6 — one step past the last exponent that maps to a finite half
/// (`j = 142` -> `j = 143`, i.e. the overflow-to-Inf boundary).
#[test]
fn g6_one_step_past_half_overflow_threshold() {
    let l = libs();
    for sign in [0u32, 256] {
        for (j, mant) in [
            (142, 0x00_0000),
            (142, 0x7F_FFFF),
            (143, 0x00_0000),
            (143, 0x00_0001),
            (143, 0x7F_FFFF),
            (144, 0x00_0000),
        ] {
            l.assert_same_bits(bits_from(sign + j, mant), "G6 overflow boundary");
        }
    }
    // Documented values: 65504.0 is the largest finite binary16; the next
    // representable step overflows.
    assert_eq!(l.c(65504.0), l.rust(65504.0));
    assert_eq!(l.c(65536.0), l.rust(65536.0));
    assert_eq!(l.c(65536.0), 0x7C00);
}

/// G7 — one step past the flush-to-zero range (`j = 102` -> `j = 103`).
#[test]
fn g7_one_step_past_flush_to_zero_threshold() {
    let l = libs();
    for sign in [0u32, 256] {
        for (j, mant) in [
            (101, 0x7F_FFFF),
            (102, 0x00_0000),
            (102, 0x7F_FFFF),
            (103, 0x00_0000),
            (103, 0x00_0001),
            (103, 0x7F_FFFF),
            (104, 0x00_0000),
        ] {
            l.assert_same_bits(bits_from(sign + j, mant), "G7 flush-to-zero boundary");
        }
    }
}

/// G8 — one step past the largest half subnormal into the smallest half
/// normal (`j = 112` -> `j = 113`).
#[test]
fn g8_one_step_past_smallest_half_normal() {
    let l = libs();
    for sign in [0u32, 256] {
        for (j, mant) in [
            (112, 0x00_0000),
            (112, 0x7F_FFFF),
            (113, 0x00_0000),
            (113, 0x00_0001),
            (113, 0x7F_FFFF),
        ] {
            l.assert_same_bits(bits_from(sign + j, mant), "G8 subnormal/normal boundary");
        }
    }
}

/// G9 — every adjacent `(j, j+1)` pair, plus the extremes `j = 0` and
/// `j = 511`. `j` is the ONLY index the C derives, and `& 0x1ff` provably
/// clamps it to `0..=511`; this walks every step across that whole range so
/// there is no "one past the end" left untried.
#[test]
fn g9_every_index_boundary_pair_and_extremes() {
    let l = libs();
    for j in 0u32..511 {
        for mant in [0x00_0000, 0x00_0001, 0x7F_FFFF] {
            l.assert_same_bits(bits_from(j, mant), &format!("G9 j={j}"));
            l.assert_same_bits(bits_from(j + 1, mant), &format!("G9 j={}", j + 1));
        }
    }
    // Extremes of the derived index, with extreme mantissas: the closest thing
    // this API has to an out-of-range value, and the highest bit pattern of all.
    l.assert_same_bits(bits_from(0, 0), "G9 j=0 min");
    l.assert_same_bits(bits_from(0, 0x7F_FFFF), "G9 j=0 max mant");
    l.assert_same_bits(bits_from(511, 0), "G9 j=511 min");
    l.assert_same_bits(bits_from(511, 0x7F_FFFF), "G9 j=511 max mant (0xFFFFFFFF)");
    assert_eq!(bits_from(511, 0x7F_FFFF), 0xFFFF_FFFF);
}

/// G10 — all 512 `j` values crossed with the extreme mantissa values, i.e. the
/// min/max of the only sub-field the code reads.
#[test]
fn g10_all_indices_with_extreme_mantissas() {
    let l = libs();
    for j in 0u32..512 {
        for mant in [0x00_0000u32, 0x00_0001, 0x00_0002, 0x1F_FFFF, 0x20_0000, 0x3F_FFFF,
                     0x40_0000, 0x40_0001, 0x7F_FFFD, 0x7F_FFFE, 0x7F_FFFF] {
            l.assert_same_bits(bits_from(j, mant), &format!("G10 j={j} mant=0x{mant:06X}"));
        }
    }
}

/// The one "invalid input" a caller of a float-taking C function can still
/// construct: a bit pattern that is not any normal float. All 2^32 patterns are
/// valid `float` object representations, so this asserts C and Rust agree on
/// every *class* of them, including the ones no arithmetic can produce.
#[test]
fn all_non_finite_and_pathological_bit_patterns() {
    let l = libs();
    let mut probes: Vec<u32> = vec![
        0x0000_0000, 0x0000_0001, 0x007F_FFFF, 0x0080_0000, 0x7F7F_FFFF, 0x7F80_0000,
        0x7F80_0001, 0x7FBF_FFFF, 0x7FC0_0000, 0x7FFF_FFFF, 0x8000_0000, 0x8000_0001,
        0x807F_FFFF, 0x8080_0000, 0xFF7F_FFFF, 0xFF80_0000, 0xFF80_0001, 0xFFBF_FFFF,
        0xFFC0_0000, 0xFFFF_FFFF, 0xAAAA_AAAA, 0x5555_5555, 0xDEAD_BEEF, 0xCAFE_BABE,
    ];
    // every single-bit and single-bit-cleared pattern
    for b in 0..32 {
        probes.push(1u32 << b);
        probes.push(!(1u32 << b));
    }
    for bits in probes {
        l.assert_same_bits(bits, "pathological bit pattern");
    }
}
