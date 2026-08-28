//! Phase B — valid-path differential tests, GATED on `CONFIGS.md`.
//!
//! One `#[test]` per row of `CONFIGS.md`. Every test loads both the C `.so` and
//! the Rust `.so` via `libloading` and compares results bit-for-bit. Randomized
//! rows use a fixed seed (derived from the row number) so failures reproduce.

mod common;

use common::{Pair, Rng, SAMPLES_PER_ROW, assert_bits_eq, half_from};

/// Run one `h` through both libraries and assert bit equality.
fn check(pair: &Pair, h: u16, ctx: &str) {
    let c = pair.c_half2float();
    let r = pair.rust_half2float();
    // SAFETY: signature matches `float half2float(uint16_t)` from lib.h.
    let (cv, rv) = unsafe { (c(h), r(h)) };
    assert_bits_eq(h, cv, rv, ctx);
}

/// Sweep a fixed exponent-field index `n` across mantissa boundary values plus
/// randomized mantissas.
fn check_region(pair: &Pair, n: u32, seed: u64, ctx: &str) {
    // Boundary mantissas first: 0, 1, midpoint, and max.
    for m in [0x000u32, 0x001, 0x200, 0x3FF] {
        check(pair, half_from(n, m), ctx);
    }
    // Then randomized mantissas across the whole 10-bit field.
    let mut rng = Rng::new(seed);
    for _ in 0..SAMPLES_PER_ROW {
        let m = rng.below(0x400);
        check(pair, half_from(n, m), ctx);
    }
}

// ---------------------------------------------------------------------------
// Rows 1-4: n = 0 -> positive zero / positive subnormal (offset 0x0000)
// ---------------------------------------------------------------------------

#[test]
fn row01_n0_mantissa_zero_is_positive_zero() {
    let pair = Pair::load();
    let h = half_from(0, 0x000);
    check(&pair, h, "row 1: n=0 mantissa=0x000 (+0.0)");

    // Pin down that this really is +0.0 and not -0.0: compare raw bits, since
    // `+0.0 == -0.0` would hide a sign-bit divergence.
    let c = pair.c_half2float();
    let cv = unsafe { c(h) };
    assert_eq!(cv.to_bits(), 0x0000_0000, "C must produce exactly +0.0 bits");
}

#[test]
fn row02_n0_smallest_positive_subnormal() {
    let pair = Pair::load();
    check(&pair, half_from(0, 0x001), "row 2: smallest +subnormal");
}

#[test]
fn row03_n0_largest_positive_subnormal() {
    let pair = Pair::load();
    check(&pair, half_from(0, 0x3FF), "row 3: largest +subnormal");
}

#[test]
fn row04_n0_positive_subnormal_region_randomized() {
    let pair = Pair::load();
    check_region(&pair, 0, 0x0000_0004, "row 4: n=0 randomized");
}

// ---------------------------------------------------------------------------
// Rows 5-7: positive normals (offset 0x0400)
// ---------------------------------------------------------------------------

#[test]
fn row05_n1_smallest_positive_normal_exponent() {
    let pair = Pair::load();
    check_region(&pair, 1, 0x0000_0005, "row 5: n=1");
}

#[test]
fn row06_interior_positive_normals_randomized() {
    let pair = Pair::load();
    // Every interior exponent gets covered deterministically...
    for n in 2..=29 {
        check_region(&pair, n, 0x0600 + n as u64, "row 6: interior +normal");
    }
    // ...plus a randomized cross-product of (n, mantissa) within the region.
    let mut rng = Rng::new(0x0000_0006);
    for _ in 0..SAMPLES_PER_ROW {
        let n = rng.in_range(2, 29);
        let m = rng.below(0x400);
        check(&pair, half_from(n, m), "row 6: randomized (n,mantissa)");
    }
}

#[test]
fn row07_n30_largest_finite_positive() {
    let pair = Pair::load();
    check_region(&pair, 30, 0x0000_0007, "row 7: n=30");
    // 0x7BFF is the largest finite positive half.
    check(&pair, 0x7BFF, "row 7: largest finite positive half");
}

// ---------------------------------------------------------------------------
// Rows 8-12: n = 31 -> +Inf / +NaN (the anomalous exponent entry 0x47800000)
// ---------------------------------------------------------------------------

#[test]
fn row08_n31_positive_infinity() {
    let pair = Pair::load();
    check(&pair, 0x7C00, "row 8: +Inf");
}

#[test]
fn row09_n31_positive_signalling_nan_payload() {
    let pair = Pair::load();
    // Payload bits must survive the xmm0 return unchanged and identically.
    check(&pair, 0x7C01, "row 9: +sNaN 0x7C01");
}

#[test]
fn row10_n31_positive_quiet_nan() {
    let pair = Pair::load();
    check(&pair, 0x7E00, "row 10: +qNaN 0x7E00");
}

#[test]
fn row11_n31_top_of_positive_nan_range() {
    let pair = Pair::load();
    check(&pair, 0x7FFF, "row 11: 0x7FFF");
}

#[test]
fn row12_n31_region_randomized() {
    let pair = Pair::load();
    check_region(&pair, 31, 0x0000_0012, "row 12: n=31 randomized");
}

// ---------------------------------------------------------------------------
// Rows 13-16: n = 32 -> -0.0 / negative subnormal (offset drops back to 0x0000)
// ---------------------------------------------------------------------------

#[test]
fn row13_n32_mantissa_zero_is_negative_zero() {
    let pair = Pair::load();
    let h = half_from(32, 0x000);
    check(&pair, h, "row 13: n=32 mantissa=0x000 (-0.0)");

    let c = pair.c_half2float();
    let r = pair.rust_half2float();
    let (cv, rv) = unsafe { (c(h), r(h)) };
    // Both must be *negative* zero, i.e. sign bit set. `== 0.0` cannot see this.
    assert_eq!(cv.to_bits(), 0x8000_0000, "C must produce exactly -0.0 bits");
    assert_eq!(rv.to_bits(), 0x8000_0000, "Rust must produce exactly -0.0 bits");
    // And -0.0 must be bit-distinct from the +0.0 of row 1.
    let plus_zero = unsafe { c(half_from(0, 0)) };
    assert_ne!(
        cv.to_bits(),
        plus_zero.to_bits(),
        "+0.0 and -0.0 must be bit-distinct"
    );
}

#[test]
fn row14_n32_smallest_negative_subnormal() {
    let pair = Pair::load();
    check(&pair, half_from(32, 0x001), "row 14: smallest -subnormal");
}

#[test]
fn row15_n32_largest_negative_subnormal() {
    let pair = Pair::load();
    check(&pair, half_from(32, 0x3FF), "row 15: largest -subnormal");
}

#[test]
fn row16_n32_negative_subnormal_region_randomized() {
    let pair = Pair::load();
    // This is the row that catches a wrong `m__offset[32]`: the offset returns
    // to 0x0000 here, so the LOW half of m__mantissa is reused with a negative
    // exponent. An off-by-one in the offset table shows up immediately.
    check_region(&pair, 32, 0x0000_0016, "row 16: n=32 randomized");
}

// ---------------------------------------------------------------------------
// Rows 17-19: negative normals
// ---------------------------------------------------------------------------

#[test]
fn row17_n33_smallest_negative_normal_exponent() {
    let pair = Pair::load();
    check_region(&pair, 33, 0x0000_0017, "row 17: n=33");
}

#[test]
fn row18_interior_negative_normals_randomized() {
    let pair = Pair::load();
    for n in 34..=61 {
        check_region(&pair, n, 0x1800 + n as u64, "row 18: interior -normal");
    }
    let mut rng = Rng::new(0x0000_0018);
    for _ in 0..SAMPLES_PER_ROW {
        let n = rng.in_range(34, 61);
        let m = rng.below(0x400);
        check(&pair, half_from(n, m), "row 18: randomized (n,mantissa)");
    }
}

#[test]
fn row19_n62_largest_finite_negative() {
    let pair = Pair::load();
    check_region(&pair, 62, 0x0000_0019, "row 19: n=62");
    check(&pair, 0xFBFF, "row 19: largest-magnitude finite negative half");
}

// ---------------------------------------------------------------------------
// Rows 20-23: n = 63 -> -Inf / -NaN (exponent 0xC7800000, near-overflow region)
// ---------------------------------------------------------------------------

#[test]
fn row20_n63_negative_infinity() {
    let pair = Pair::load();
    check(&pair, 0xFC00, "row 20: -Inf");
}

#[test]
fn row21_n63_negative_signalling_nan_payload() {
    let pair = Pair::load();
    check(&pair, 0xFC01, "row 21: -sNaN 0xFC01");
}

#[test]
fn row22_n63_maximum_input_value() {
    let pair = Pair::load();
    check(&pair, 0xFFFF, "row 22: h = 0xFFFF (max input)");
}

#[test]
fn row23_n63_region_randomized_near_u32_overflow() {
    let pair = Pair::load();
    // m__exponent[63] = 0xC7800000 is the largest exponent addend, so this is
    // where `m__mantissa[i] + m__exponent[n]` comes closest to wrapping u32.
    // Exercising the whole region pins down the wrapping-add semantics.
    check_region(&pair, 63, 0x0000_0023, "row 23: n=63 randomized");
}

// ---------------------------------------------------------------------------
// Row 24: both m__offset values back-to-back (mis-shared index base)
// ---------------------------------------------------------------------------

#[test]
fn row24_offset_table_transitions() {
    let pair = Pair::load();
    // The only two places m__offset changes value are n=0->1 and n=31->32->33.
    // Interleave across each transition with identical mantissa fields so a
    // mixed-up offset produces an immediately visible mismatch.
    for m in [0x000u32, 0x001, 0x1FF, 0x200, 0x3FE, 0x3FF] {
        for n in [0u32, 1, 30, 31, 32, 33, 62, 63] {
            check(&pair, half_from(n, m), "row 24: offset transition");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 25: fully randomized over the whole domain
// ---------------------------------------------------------------------------

#[test]
fn row25_fully_randomized_domain() {
    let pair = Pair::load();
    let mut rng = Rng::new(0x0000_0025);
    for _ in 0..(SAMPLES_PER_ROW * 16) {
        let h = rng.next_u16();
        check(&pair, h, "row 25: fully randomized h");
    }
}

// ---------------------------------------------------------------------------
// Row 26: EXHAUSTIVE - all 65536 inputs. Subsumes every row above.
// ---------------------------------------------------------------------------

#[test]
fn row26_exhaustive_all_65536_inputs() {
    let pair = Pair::load();
    let c = pair.c_half2float();
    let r = pair.rust_half2float();

    let mut mismatches: Vec<(u16, u32, u32)> = Vec::new();
    for h in 0u16..=u16::MAX {
        let (cv, rv) = unsafe { (c(h), r(h)) };
        let (cb, rb) = (cv.to_bits(), rv.to_bits());
        if cb != rb {
            mismatches.push((h, cb, rb));
        }
        if h == u16::MAX {
            break;
        }
    }

    assert!(
        mismatches.is_empty(),
        "{} of 65536 inputs diverged; first 16: {:?}",
        mismatches.len(),
        &mismatches[..mismatches.len().min(16)]
    );
}

// ---------------------------------------------------------------------------
// Row 27: statelessness / order independence within one loaded session
// ---------------------------------------------------------------------------

#[test]
fn row27_stateless_and_order_independent() {
    let pair = Pair::load();
    let c = pair.c_half2float();
    let r = pair.rust_half2float();

    // Record a baseline in ascending order.
    let probes: Vec<u16> = {
        let mut rng = Rng::new(0x0000_0027);
        (0..1024).map(|_| rng.next_u16()).collect()
    };
    let baseline: Vec<(u32, u32)> = probes
        .iter()
        .map(|&h| unsafe { (c(h).to_bits(), r(h).to_bits()) })
        .collect();

    // Replay in reverse, and repeatedly, interleaving the two libraries. A
    // stateful implementation (e.g. a cached last result) would drift.
    for _round in 0..4 {
        for (idx, &h) in probes.iter().enumerate().rev() {
            let (cv, rv) = unsafe { (c(h), r(h)) };
            assert_bits_eq(h, cv, rv, "row 27: reversed replay");
            assert_eq!(
                (cv.to_bits(), rv.to_bits()),
                baseline[idx],
                "row 27: result for h={h:#06x} changed between calls (not stateless)"
            );
        }
    }
}
