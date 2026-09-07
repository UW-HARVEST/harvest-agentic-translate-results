//! Differential tests: C `.so` vs Rust `.so`, both loaded via `libloading`.
//!
//! Phase B (valid paths) covers every row of `CONFIGS.md`.
//! Phase C (error paths) covers `ERRORS.md` (empty table) plus the generic
//! boundaries every C API has.
//!
//! All comparisons use raw `f32` bit patterns, so NaN payloads and signed
//! zeroes are distinguished byte-for-byte.

mod common;

use common::{assert_same, both, Lcg};

// ---------------------------------------------------------------------------
// Sanity: both libraries actually loaded and export the symbol.
// ---------------------------------------------------------------------------

#[test]
fn both_libraries_load_and_export_half2float() {
    let (c, r) = both();
    // 1.0h == 0x3C00 -> 1.0f
    assert_eq!(c.call(0x3C00), 1.0f32, "C half2float(0x3C00) should be 1.0");
    assert_eq!(r.call(0x3C00), 1.0f32, "Rust half2float(0x3C00) should be 1.0");
    println!("loaded C   from {}", common::c_so_path().display());
    println!("loaded Rust from {}", common::rust_so_path().display());
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 12 — EXHAUSTIVE. The input domain is only 65536 wide, so this
// is a complete proof of behavioural equivalence.
// ---------------------------------------------------------------------------

#[test]
fn exhaustive_all_65536_inputs() {
    let (c, r) = both();
    let mut mismatches: Vec<(u16, u32, u32)> = Vec::new();

    for h in 0u16..=0xFFFF {
        let cb = c.call_bits(h);
        let rb = r.call_bits(h);
        if cb != rb {
            mismatches.push((h, cb, rb));
        }
    }

    if !mismatches.is_empty() {
        let shown: Vec<String> = mismatches
            .iter()
            .take(20)
            .map(|(h, cb, rb)| {
                format!(
                    "h=0x{h:04X} (n={}, low=0x{:03X}) C=0x{cb:08X} Rust=0x{rb:08X}",
                    h >> 10,
                    h & 0x3FF
                )
            })
            .collect();
        panic!(
            "{} / 65536 inputs diverged. First {}:\n{}",
            mismatches.len(),
            shown.len(),
            shown.join("\n")
        );
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 1 & 2 — offset==0x0000 rows (zero / subnormal), n==0, n==32.
// ---------------------------------------------------------------------------

#[test]
fn row1_n0_zero_and_subnormals_positive() {
    let (c, r) = both();
    for low in 0u16..=0x3FF {
        assert_same(&c, &r, low, "row1 n=0 offset=0 positive zero/subnormal");
    }
}

#[test]
fn row2_n32_zero_and_subnormals_negative() {
    let (c, r) = both();
    for low in 0u16..=0x3FF {
        let h = (32u16 << 10) | low;
        assert_same(&c, &r, h, "row2 n=32 offset=0 negative zero/subnormal");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 3 & 4 — offset==0x0400 rows (normals), n in 1..=30 / 33..=62.
// ---------------------------------------------------------------------------

#[test]
fn row3_positive_normals_n1_to_n30() {
    let (c, r) = both();
    for n in 1u16..=30 {
        for low in 0u16..=0x3FF {
            let h = (n << 10) | low;
            assert_same(&c, &r, h, "row3 positive normal offset=0x400");
        }
    }
}

#[test]
fn row4_negative_normals_n33_to_n62() {
    let (c, r) = both();
    for n in 33u16..=62 {
        for low in 0u16..=0x3FF {
            let h = (n << 10) | low;
            assert_same(&c, &r, h, "row4 negative normal offset=0x400");
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 5 & 6 — the Inf/NaN rows, where m__exponent is the quirky
// 0x47800000 / 0xC7800000 rather than a regular exponent step.
// ---------------------------------------------------------------------------

#[test]
fn row5_n31_positive_inf_and_nan() {
    let (c, r) = both();
    for low in 0u16..=0x3FF {
        let h = (31u16 << 10) | low;
        assert_same(&c, &r, h, "row5 n=31 exponent=0x47800000 +Inf/NaN");
    }
}

#[test]
fn row6_n63_negative_inf_and_nan() {
    let (c, r) = both();
    for low in 0u16..=0x3FF {
        let h = (63u16 << 10) | low;
        assert_same(&c, &r, h, "row6 n=63 exponent=0xC7800000 -Inf/NaN");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md rows 7 & 8 — low field pinned to its extremes across all 64 n.
// ---------------------------------------------------------------------------

#[test]
fn row7_low_field_zero_across_all_n() {
    let (c, r) = both();
    for n in 0u16..64 {
        assert_same(&c, &r, n << 10, "row7 low=0x000 across all n");
    }
}

#[test]
fn row8_low_field_max_across_all_n() {
    let (c, r) = both();
    for n in 0u16..64 {
        assert_same(&c, &r, (n << 10) | 0x3FF, "row8 low=0x3FF across all n");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 9 / ERRORS.md generic boundaries — min, max, one-past-range,
// and every special encoding.
// ---------------------------------------------------------------------------

#[test]
fn row9_boundary_values() {
    let (c, r) = both();
    let cases: &[(u16, &str)] = &[
        (0x0000, "minimum uint16 / +0.0"),
        (0x0001, "smallest positive subnormal"),
        (0x03FE, "second largest subnormal"),
        (0x03FF, "largest subnormal (last of offset=0 block)"),
        (0x0400, "one past largest subnormal: smallest normal, n flips to 1"),
        (0x0401, "smallest normal + 1"),
        (0x3BFF, "just below 1.0"),
        (0x3C00, "exactly 1.0"),
        (0x3C01, "just above 1.0"),
        (0x7BFF, "largest finite half (65504)"),
        (0x7C00, "+Inf"),
        (0x7C01, "smallest +NaN payload"),
        (0x7DFF, "mid +NaN payload"),
        (0x7E00, "+quiet NaN"),
        (0x7FFF, "largest +NaN payload"),
        (0x8000, "-0.0 (sign bit only)"),
        (0x8001, "smallest negative subnormal"),
        (0x83FF, "largest negative subnormal"),
        (0x8400, "smallest negative normal"),
        (0xBC00, "exactly -1.0"),
        (0xFBFF, "most negative finite half"),
        (0xFC00, "-Inf"),
        (0xFC01, "smallest -NaN payload"),
        (0xFFFE, "second largest -NaN payload"),
        (0xFFFF, "maximum uint16 / largest -NaN payload"),
    ];
    for (h, what) in cases {
        assert_same(&c, &r, *h, what);
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 10 — the wrapping-add path. For n >= 32 the exponent table
// entry has bit 31 set, so `mantissa + exponent` in C is an unsigned add that
// must wrap identically in Rust (`wrapping_add`, not `+` with overflow panic).
// ---------------------------------------------------------------------------

#[test]
fn row10_wrapping_add_high_bit_paths() {
    let (c, r) = both();
    // Every n whose exponent entry has the top bit set (n >= 32), plus n==31
    // whose entry (0x47800000) is the other irregular one.
    for n in [31u16].into_iter().chain(32u16..64) {
        for low in 0u16..=0x3FF {
            let h = (n << 10) | low;
            assert_same(&c, &r, h, "row10 wrapping add / bit-31 exponent");
        }
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 11 — randomized property sweep with a FIXED seed.
// ---------------------------------------------------------------------------

#[test]
fn row11_randomized_property_sweep_fixed_seed() {
    let (c, r) = both();
    let mut rng = Lcg::new(0xC0FFEE_1234_5678);
    for i in 0..200_000 {
        let h = rng.next_u16();
        let cb = c.call_bits(h);
        let rb = r.call_bits(h);
        assert_eq!(
            cb, rb,
            "row11 randomized draw #{i} h=0x{h:04X} (n={}, low=0x{:03X}): \
             C=0x{cb:08X} vs Rust=0x{rb:08X}",
            h >> 10,
            h & 0x3FF
        );
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 13 — statelessness / purity. The C has no global mutable
// state; interleaving inputs must not change results on either side.
// ---------------------------------------------------------------------------

#[test]
fn row13_stateless_and_order_independent() {
    let (c, r) = both();
    let probes: [u16; 8] = [0x0000, 0x03FF, 0x0400, 0x3C00, 0x7C00, 0x7FFF, 0x8000, 0xFFFF];

    // Baseline, measured in isolation.
    let base_c: Vec<u32> = probes.iter().map(|&h| c.call_bits(h)).collect();
    let base_r: Vec<u32> = probes.iter().map(|&h| r.call_bits(h)).collect();
    assert_eq!(base_c, base_r, "row13 baseline must already agree");

    // Now re-probe after interleaving lots of unrelated traffic.
    let mut rng = Lcg::new(0xDEADBEEF);
    for _ in 0..5_000 {
        let noise = rng.next_u16();
        let _ = c.call_bits(noise);
        let _ = r.call_bits(noise);
    }
    for (i, &h) in probes.iter().enumerate() {
        assert_eq!(
            c.call_bits(h), base_c[i],
            "row13 C not stateless at h=0x{h:04X}"
        );
        assert_eq!(
            r.call_bits(h), base_r[i],
            "row13 Rust not stateless at h=0x{h:04X}"
        );
    }

    // Reverse order, same answers.
    for (i, &h) in probes.iter().enumerate().rev() {
        assert_same(&c, &r, h, "row13 reverse-order probe");
        assert_eq!(c.call_bits(h), base_c[i], "row13 C reverse-order drift");
        assert_eq!(r.call_bits(h), base_r[i], "row13 Rust reverse-order drift");
    }
}

// ---------------------------------------------------------------------------
// CONFIGS.md row 14 — NaN payloads must be preserved bit-exactly. A test that
// compared with `==` would silently pass for ANY two different NaNs, so this
// asserts on raw bits and additionally verifies both sides really are NaN.
// ---------------------------------------------------------------------------

#[test]
fn row14_nan_payloads_bit_exact() {
    let (c, r) = both();
    let mut nan_count = 0usize;

    for n in [31u16, 63u16] {
        for low in 1u16..=0x3FF {
            let h = (n << 10) | low;
            let cb = c.call_bits(h);
            let rb = r.call_bits(h);
            assert_eq!(
                cb, rb,
                "row14 NaN payload differs for h=0x{h:04X}: \
                 C=0x{cb:08X} vs Rust=0x{rb:08X}"
            );
            // Guard against a vacuous test: confirm float `==` would NOT have
            // caught a difference here, i.e. these really are NaNs.
            if f32::from_bits(cb).is_nan() {
                nan_count += 1;
                assert!(
                    f32::from_bits(rb).is_nan(),
                    "row14 C produced NaN but Rust did not for h=0x{h:04X}"
                );
                assert_ne!(
                    f32::from_bits(cb) == f32::from_bits(rb),
                    true,
                    "row14 sanity: NaN == NaN must be false, so bit comparison is essential"
                );
            }
        }
    }
    assert!(
        nan_count > 0,
        "row14 expected to exercise NaN results, but saw none — test would be vacuous"
    );
    println!("row14 compared {nan_count} NaN results by raw bits");
}

// ---------------------------------------------------------------------------
// Signed zero must not be collapsed: +0.0 == -0.0 under float `==`, so this
// is another place a naive comparison would be vacuous.
// ---------------------------------------------------------------------------

#[test]
fn signed_zero_bit_exact() {
    let (c, r) = both();
    let pos = c.call_bits(0x0000);
    let neg = c.call_bits(0x8000);
    assert_eq!(pos, r.call_bits(0x0000), "+0.0 bits must match");
    assert_eq!(neg, r.call_bits(0x8000), "-0.0 bits must match");
    // Sanity: the two really are distinguishable only by bits.
    assert_ne!(pos, neg, "+0.0 and -0.0 must have different bit patterns");
    assert!(
        f32::from_bits(pos) == f32::from_bits(neg),
        "sanity: +0.0 == -0.0 under float equality, so bitwise comparison is essential"
    );
}

// ---------------------------------------------------------------------------
// ERRORS.md — the error table is empty (the C has zero branches and zero error
// returns). This test documents and enforces that: every one of the 65536
// inputs is accepted by BOTH sides and yields a value, with no side rejecting,
// trapping, or returning a sentinel that the other does not.
// ---------------------------------------------------------------------------

#[test]
fn errors_md_total_function_no_rejection_path() {
    let (c, r) = both();

    // Classify every input identically on both sides. If either implementation
    // had a rejection path (sentinel, trap, differing NaN-vs-finite), the
    // classification vectors would diverge.
    #[derive(PartialEq, Debug)]
    enum Kind {
        Zero,
        Subnormal,
        Normal,
        Infinite,
        Nan,
    }
    fn kind(x: f32) -> Kind {
        if x.is_nan() {
            Kind::Nan
        } else if x.is_infinite() {
            Kind::Infinite
        } else if x == 0.0 {
            Kind::Zero
        } else if x.is_subnormal() {
            Kind::Subnormal
        } else {
            Kind::Normal
        }
    }

    let mut counts = [0usize; 5];
    for h in 0u16..=0xFFFF {
        let kc = kind(c.call(h));
        let kr = kind(r.call(h));
        assert_eq!(
            kc, kr,
            "classification diverged at h=0x{h:04X}: C={kc:?} Rust={kr:?}"
        );
        counts[match kc {
            Kind::Zero => 0,
            Kind::Subnormal => 1,
            Kind::Normal => 2,
            Kind::Infinite => 3,
            Kind::Nan => 4,
        }] += 1;
    }

    let total: usize = counts.iter().sum();
    assert_eq!(total, 65536, "every input must produce a classified result");
    // No input is rejected: all 65536 returned a value on both sides.
    println!(
        "ERRORS.md: 65536/65536 inputs accepted by both. \
         zero={} subnormal={} normal={} inf={} nan={}",
        counts[0], counts[1], counts[2], counts[3], counts[4]
    );
}
