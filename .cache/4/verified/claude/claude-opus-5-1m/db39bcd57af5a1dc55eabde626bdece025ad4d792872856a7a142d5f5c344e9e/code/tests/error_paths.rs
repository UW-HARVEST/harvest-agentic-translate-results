//! Phase C — error-path / rejection differential tests.
//!
//! One `#[test]` per row of `ERRORS.md`. The C source contains ZERO rejection
//! sites (no error return, no assert, no range check — see `ERRORS.md` for the
//! grep evidence), so `max_size_frame` is a total function. Each row therefore
//! asserts that the Rust `.so` *also* accepts the degenerate/extreme/invalid
//! input and returns the **same** 32-bit value, rather than panicking,
//! overflow-trapping, saturating, or clamping.
//!
//! Every call goes through `libloading` into the two shared objects.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Row 1 — the function is total: no input is rejected
// ---------------------------------------------------------------------------

#[test]
fn row01_function_is_total_never_rejects() {
    let l = libs();
    let mut rng = Rng::new(101);
    // Hammer the whole 96-bit input space; any panic/abort in either library
    // (a Rust overflow panic in a debug build included) fails this test.
    for _ in 0..100_000 {
        let bs = rng.next_u32();
        let ch = rng.next_u32();
        let bd = rng.next_u32();
        assert_same_triple(l, bs, ch, bd);
    }
    // Plus the full boundary cross product: 18^3 = 5832 calls, all must return.
    for &bs in BOUNDARY_VALUES {
        for &ch in BOUNDARY_VALUES {
            for &bd in BOUNDARY_VALUES {
                assert_same_triple(l, bs, ch, bd);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 2-7 — zero / degenerate inputs ("zero length" analogues)
// ---------------------------------------------------------------------------

#[test]
fn row02_blocksize_zero() {
    let l = libs();
    let mut rng = Rng::new(102);
    for _ in 0..20_000 {
        let ch = rng.next_u32();
        let bd = rng.next_u32();
        // Documented: result == 18 + channels (mod 2^32).
        assert_same_and_eq(l, 0, ch, bd, 18u32.wrapping_add(ch));
    }
    for &ch in BOUNDARY_VALUES {
        for &bd in BOUNDARY_VALUES {
            assert_same_and_eq(l, 0, ch, bd, 18u32.wrapping_add(ch));
        }
    }
}

#[test]
fn row03_channels_zero() {
    let l = libs();
    let mut rng = Rng::new(103);
    for _ in 0..20_000 {
        let bs = rng.next_u32();
        let bd = rng.next_u32();
        // Documented: result == 18 exactly, for every blocksize/bitdepth.
        assert_same_and_eq(l, bs, 0, bd, 18);
    }
    for &bs in BOUNDARY_VALUES {
        for &bd in BOUNDARY_VALUES {
            assert_same_and_eq(l, bs, 0, bd, 18);
        }
    }
}

#[test]
fn row04_bitdepth_zero_stereo() {
    let l = libs();
    let mut rng = Rng::new(104);
    for _ in 0..20_000 {
        let bs = rng.next_u32();
        // T3 == blocksize * (0 + 1) * 1 == blocksize  =>  20 + (bs + 7) / 8
        let expected = 20u32.wrapping_add(bs.wrapping_add(7) / 8);
        assert_same_and_eq(l, bs, 2, 0, expected);
    }
}

#[test]
fn row05_bitdepth_zero_non_stereo() {
    let l = libs();
    let mut rng = Rng::new(105);
    for _ in 0..20_000 {
        let bs = rng.next_u32();
        let mut ch = rng.next_u32();
        if ch == 2 {
            ch = 3;
        }
        // All terms vanish: result == 18 + channels.
        assert_same_and_eq(l, bs, ch, 0, 18u32.wrapping_add(ch));
    }
}

#[test]
fn row06_all_args_zero() {
    let l = libs();
    assert_same_and_eq(l, 0, 0, 0, 18);
}

#[test]
fn row07_zero_zero_max() {
    let l = libs();
    assert_same_and_eq(l, 0, 0, u32::MAX, 18);
}

// ---------------------------------------------------------------------------
// Rows 8-13 — overflow / "oversized value" inputs
// ---------------------------------------------------------------------------

#[test]
fn row08_channels_uint32_max_outer_overflow() {
    let l = libs();
    // 18 + 0xFFFFFFFF wraps to 17.
    assert_same_and_eq(l, 0, u32::MAX, 0, 17);

    let mut rng = Rng::new(108);
    for _ in 0..20_000 {
        let bs = rng.next_u32();
        let bd = rng.next_u32();
        assert_same_triple(l, bs, u32::MAX, bd);
    }
}

#[test]
fn row09_bitdepth_uint32_max_inner_overflow() {
    let l = libs();
    // bitdepth + (bitdepth != 32) == 0xFFFFFFFF + 1 == 0, so T3 == 0 and
    // T2 == 1 * 0xFFFFFFFF == 0xFFFFFFFF; (0xFFFFFFFF + 7) wraps to 6; 6/8 == 0.
    assert_same_and_eq(l, 1, 2, u32::MAX, 20);

    let mut rng = Rng::new(109);
    for _ in 0..20_000 {
        let bs = rng.next_u32();
        assert_same_triple(l, bs, 2, u32::MAX);
        let mut ch = rng.next_u32();
        if ch == 2 {
            ch = 3;
        }
        assert_same_triple(l, bs, ch, u32::MAX);
    }
}

#[test]
fn row10_blocksize_uint32_max() {
    let l = libs();
    let mut rng = Rng::new(110);
    for _ in 0..20_000 {
        let ch = rng.next_u32();
        let bd = rng.next_u32();
        assert_same_triple(l, u32::MAX, ch, bd);
    }
    for &ch in BOUNDARY_VALUES {
        for &bd in BOUNDARY_VALUES {
            assert_same_triple(l, u32::MAX, ch, bd);
        }
    }
}

#[test]
fn row11_all_args_uint32_max() {
    let l = libs();
    assert_same_triple(l, u32::MAX, u32::MAX, u32::MAX);
}

#[test]
fn row12_product_overflow() {
    let l = libs();
    // 0x10000 * 0x10000 * 1 == 2^32 -> truncates to 0; numerator == 7; 7/8 == 0.
    assert_same_and_eq(l, 0x10000, 1, 0x10000, 19);

    // A spread of exactly-overflowing and just-past-overflowing products.
    // (Every entry is a genuine 32-bit value; pairs such as 0x10000*0x10000 and
    // 0xFFFF*0x10001 land exactly on 2^32 and 2^32-1 respectively.)
    let interesting: &[u32] = &[
        0x1_0000, 0x1_0001, 0x8000, 0x2_0000, 0x400, 0x10, 0xFFFF, 0x8000_0000, 0xFFFF_FFFF,
    ];
    for &bs in interesting {
        for &bd in interesting {
            for ch in [0u32, 1, 2, 3, 4, 8, 0x1_0000] {
                assert_same_triple(l, bs, ch, bd);
            }
        }
    }

    // Randomized products that are guaranteed to exceed 32 bits.
    let mut rng = Rng::new(112);
    for _ in 0..20_000 {
        let bs = rng.range(1 << 16, u32::MAX);
        let bd = rng.range(1 << 16, u32::MAX);
        let ch = rng.range(0, 8);
        assert_same_triple(l, bs, ch, bd);
    }
}

#[test]
fn row13_numerator_plus_seven_overflow() {
    let l = libs();
    // channels == 1, bitdepth == 1 => numerator == blocksize + 7.
    // For blocksize in [MAX-6, MAX] the +7 wraps, so the quotient is tiny.
    for bs in (u32::MAX - 6)..=u32::MAX {
        let expected = 19u32.wrapping_add(bs.wrapping_add(7) / 8);
        assert_same_and_eq(l, bs, 1, 1, expected);
    }
    // blocksize == MAX: (0xFFFFFFFF + 7) == 6 -> 6/8 == 0 -> 18 + 1 + 0 == 19.
    assert_same_and_eq(l, u32::MAX, 1, 1, 19);
    // The stereo instance of the same wrap.
    assert_same_and_eq(l, 1, 2, u32::MAX, 20);
}

// ---------------------------------------------------------------------------
// Rows 14-18 — one step past the only range-like constants in the source
// ---------------------------------------------------------------------------

#[test]
fn row14_channels_one_below_two() {
    let l = libs();
    let mut rng = Rng::new(114);
    for _ in 0..20_000 {
        let bs = rng.next_u32();
        let bd = rng.next_u32();
        assert_same_triple(l, bs, 1, bd);
    }
}

#[test]
fn row15_channels_one_above_two() {
    let l = libs();
    let mut rng = Rng::new(115);
    for _ in 0..20_000 {
        let bs = rng.next_u32();
        let bd = rng.next_u32();
        assert_same_triple(l, bs, 3, bd);
    }
}

#[test]
fn row16_bitdepth_adjacent_to_32() {
    let l = libs();
    let mut rng = Rng::new(116);
    for _ in 0..20_000 {
        let bs = rng.next_u32();
        for ch in [0u32, 1, 2, 3, 8] {
            for bd in [31u32, 33] {
                assert_same_triple(l, bs, ch, bd);
            }
        }
    }
}

#[test]
fn row17_bitdepth_exactly_32() {
    let l = libs();
    let mut rng = Rng::new(117);
    for _ in 0..20_000 {
        let bs = rng.next_u32();
        let ch = rng.next_u32();
        assert_same_triple(l, bs, ch, 32);
    }
    // The 31 / 32 / 33 discontinuity must be reproduced exactly.
    for bs in 1..=512u32 {
        for bd in [31u32, 32, 33] {
            assert_same_triple(l, bs, 2, bd);
        }
    }
}

#[test]
fn row18_channels_exactly_2() {
    let l = libs();
    let mut rng = Rng::new(118);
    for _ in 0..20_000 {
        let bs = rng.next_u32();
        let bd = rng.next_u32();
        assert_same_triple(l, bs, 2, bd);
    }
    // The 1 / 2 / 3 discontinuity must be reproduced exactly.
    for bs in 1..=512u32 {
        for ch in [1u32, 2, 3] {
            assert_same_triple(l, bs, ch, 16);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 19-21 — FFI-boundary robustness (enum / null / signedness analogues)
// ---------------------------------------------------------------------------

/// Stand-in for "out-of-range enum value passed across FFI". No parameter is an
/// enum here: all three are `uint32_t`, so *every* one of the 2^32 bit patterns
/// is a legal input with no invalid variant, and none may be rejected. This
/// sweeps hostile bit patterns (all-ones, alternating, single-bit, sign-bit).
#[test]
fn row19_arbitrary_bit_patterns_no_invalid_variant() {
    let l = libs();

    let mut patterns: Vec<u32> = vec![
        0x0000_0000,
        0xFFFF_FFFF,
        0xAAAA_AAAA,
        0x5555_5555,
        0xDEAD_BEEF,
        0xCAFE_BABE,
        0x8000_0000,
        0x7FFF_FFFF,
        0xFFFF_0000,
        0x0000_FFFF,
        0xFF00_FF00,
        0x00FF_00FF,
    ];
    // Every single-bit pattern.
    for shift in 0..32u32 {
        patterns.push(1u32 << shift);
        patterns.push(!(1u32 << shift));
    }

    for &bs in &patterns {
        for &ch in &patterns {
            for &bd in &patterns {
                assert_same_triple(l, bs, ch, bd);
            }
        }
    }

    // The structured list above is not a claim about all 2^32 patterns, so back
    // it with uniform random 32-bit patterns as well (the full-range sweep of a
    // whole axis lives in tests/exhaustive_axis.rs).
    let mut rng = Rng::new(119);
    for _ in 0..200_000 {
        let bs = rng.next_u32();
        let ch = rng.next_u32();
        let bd = rng.next_u32();
        assert_same_triple(l, bs, ch, bd);
    }
}

/// ABI analogue of passing `NULL`: the all-zero-bits argument triple.
#[test]
fn row20_all_zero_bits_null_analogue() {
    let l = libs();
    assert_same_and_eq(l, 0, 0, 0, 18);
    // Zero in each argument position individually, with hostile values elsewhere.
    assert_same_triple(l, 0, u32::MAX, u32::MAX);
    assert_same_triple(l, u32::MAX, 0, u32::MAX);
    assert_same_triple(l, u32::MAX, u32::MAX, 0);
}

/// Values above `INT32_MAX` must be treated as unsigned: a signed
/// misinterpretation (or a `c_int`-typed wrapper) would sign-extend and diverge.
#[test]
fn row21_values_above_int32_max_no_sign_extension() {
    let l = libs();
    let high: &[u32] = &[
        0x8000_0000,
        0x8000_0001,
        0xC000_0000,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
        0x9999_9999,
    ];
    for &bs in high {
        for &ch in high {
            for &bd in high {
                assert_same_triple(l, bs, ch, bd);
            }
        }
    }
    // Mixed high/low so a bad widening of any single parameter shows up.
    for &v in high {
        for low in [0u32, 1, 2, 3, 16, 32] {
            assert_same_triple(l, v, low, low);
            assert_same_triple(l, low, v, low);
            assert_same_triple(l, low, low, v);
        }
    }
    // The result must also never be interpreted as signed: check a case whose
    // return value has the top bit set.
    let big = l.c(0xFFFF_FFFF, 0x8000_0000, 1);
    assert_eq!(big, l.rust(0xFFFF_FFFF, 0x8000_0000, 1));
    assert!(big & 0x8000_0000 != 0, "expected a result with the sign bit set, got {big:#X}");
}
