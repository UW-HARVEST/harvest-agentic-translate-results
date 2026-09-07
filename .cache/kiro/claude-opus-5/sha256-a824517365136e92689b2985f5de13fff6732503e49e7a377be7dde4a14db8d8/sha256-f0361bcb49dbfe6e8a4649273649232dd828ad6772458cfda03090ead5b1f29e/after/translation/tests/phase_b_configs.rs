//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both implementations are loaded from their `.so` files and compared
//! bit-for-bit over many randomized inputs per row (fixed seed).

mod common;

use common::{Pair, Rng, BITS_1000_0, BITS_1_0};

const N: usize = 2000;

/// The 16 sign patterns of `(a, b, c, d)`, as a bitmask 0..16.
fn signs(mask: u32) -> (bool, bool, bool, bool) {
    (
        mask & 1 != 0,
        mask & 2 != 0,
        mask & 4 != 0,
        mask & 8 != 0,
    )
}

// ---------------------------------------------------------------- C1
// A1 = all 16 sign patterns x A3 mixed decimal widths.
#[test]
fn c1_all_sign_patterns_mixed_widths() {
    let p = Pair::load();
    for mask in 0u32..16 {
        let (sa, sb, sc, sd) = signs(mask);
        p.run_row(&format!("C1/mask{mask}"), N / 4, |r| {
            // Mixed widths: pick a random decimal width 1..9 per argument.
            let mag = |r: &mut Rng| {
                let digits = r.range_i32(1, 9);
                let hi = 10i32.pow(digits as u32) - 1;
                let lo = if digits == 1 { 0 } else { 10i32.pow(digits as u32 - 1) };
                r.range_i32(lo, hi)
            };
            let a = if sa { mag(r).wrapping_neg() } else { mag(r) };
            let b = if sb { mag(r).wrapping_neg() } else { mag(r) };
            let c = if sc { mag(r).wrapping_neg() } else { mag(r) };
            let d = if sd { mag(r).wrapping_neg() } else { mag(r) };
            (a, b, c, d)
        });
    }
}

// ---------------------------------------------------------------- C2
// A2 state (i): a <= 0 -> float term omitted.
#[test]
fn c2_float_window_not_taken_a_nonpositive() {
    let p = Pair::load();
    p.run_row("C2", N, |r| {
        let a = r.range_i32(i32::MIN, 0);
        (a, r.next_i32(), r.next_i32(), r.next_i32())
    });
}

// ---------------------------------------------------------------- C3
// A2 state (ii): 1 <= a < bits(1.0f) -> taken, 0 < f < 1, (int)f == 0.
#[test]
fn c3_float_window_taken_truncates_to_zero() {
    let p = Pair::load();
    p.run_row("C3", N, |r| {
        let a = r.range_u32(1, BITS_1_0 - 1) as i32;
        (a, r.next_i32(), r.next_i32(), r.next_i32())
    });
}

// ---------------------------------------------------------------- C4
// A2 state (iii): bits(1.0f) <= a < bits(1000.0f) -> (int)f in 1..999.
#[test]
fn c4_float_window_taken_nonzero_truncation() {
    let p = Pair::load();
    p.run_row("C4", N, |r| {
        let a = r.range_u32(BITS_1_0, BITS_1000_0 - 1) as i32;
        (a, r.next_i32(), r.next_i32(), r.next_i32())
    });
}

// ---------------------------------------------------------------- C5
// A2 state (iv): a >= bits(1000.0f) -> f >= 1000 / non-finite -> omitted.
#[test]
fn c5_float_window_not_taken_too_large() {
    let p = Pair::load();
    p.run_row("C5", N, |r| {
        let a = r.range_u32(BITS_1000_0, u32::MAX / 2) as i32; // stays positive as i32
        (a, r.next_i32(), r.next_i32(), r.next_i32())
    });
}

// ---------------------------------------------------------------- C6
// A5: low bytes of b, c, d all 0x00.
#[test]
fn c6_low_bytes_all_zero() {
    let p = Pair::load();
    p.run_row("C6", N, |r| {
        let mk = |r: &mut Rng| (r.next_u32() & 0xFFFF_FF00) as i32;
        (r.next_i32(), mk(r), mk(r), mk(r))
    });
}

// ---------------------------------------------------------------- C7
// A5: low bytes of b, c, d all 0xFF -> interpret_as_int == 0x00FFFFFF.
#[test]
fn c7_low_bytes_all_ff() {
    let p = Pair::load();
    p.run_row("C7", N, |r| {
        let mk = |r: &mut Rng| ((r.next_u32() & 0xFFFF_FF00) | 0xFF) as i32;
        (r.next_i32(), mk(r), mk(r), mk(r))
    });
}

// ---------------------------------------------------------------- C8
// A5: low bytes of b, c, d all 0x80 (byte sign bit set).
#[test]
fn c8_low_bytes_all_80() {
    let p = Pair::load();
    p.run_row("C8", N, |r| {
        let mk = |r: &mut Rng| ((r.next_u32() & 0xFFFF_FF00) | 0x80) as i32;
        (r.next_i32(), mk(r), mk(r), mk(r))
    });
}

// ---------------------------------------------------------------- C9
// A6: all four low bytes equal -> complex_iteration xor-cancels to 0.
#[test]
fn c9_low_bytes_xor_cancel_all_equal() {
    let p = Pair::load();
    p.run_row("C9", N, |r| {
        let byte = r.next_u32() & 0xFF;
        let mk = |r: &mut Rng| ((r.next_u32() & 0xFFFF_FF00) | byte) as i32;
        (mk(r), mk(r), mk(r), mk(r))
    });
}

// ---------------------------------------------------------------- C10
// A6: pairwise-cancelling low bytes (a==b, c==d, but the pairs differ).
#[test]
fn c10_low_bytes_xor_cancel_pairwise() {
    let p = Pair::load();
    p.run_row("C10", N, |r| {
        let b1 = r.next_u32() & 0xFF;
        let mut b2 = r.next_u32() & 0xFF;
        if b2 == b1 {
            b2 ^= 1;
        }
        let mk = |r: &mut Rng, byte: u32| ((r.next_u32() & 0xFFFF_FF00) | byte) as i32;
        (mk(r, b1), mk(r, b1), mk(r, b2), mk(r, b2))
    });
}

// ---------------------------------------------------------------- C11
// A7: a + b + c + d overflows INT_MAX (positive wraparound).
#[test]
fn c11_sum_overflows_positive() {
    let p = Pair::load();
    p.run_row("C11", N, |r| loop {
        let a = r.range_i32(600_000_000, i32::MAX);
        let b = r.range_i32(600_000_000, i32::MAX);
        let c = r.range_i32(600_000_000, i32::MAX);
        let d = r.range_i32(600_000_000, i32::MAX);
        let s = a as i64 + b as i64 + c as i64 + d as i64;
        if s > i32::MAX as i64 {
            return (a, b, c, d);
        }
    });
}

// ---------------------------------------------------------------- C12
// A7: a + b + c + d underflows INT_MIN (negative wraparound).
#[test]
fn c12_sum_overflows_negative() {
    let p = Pair::load();
    p.run_row("C12", N, |r| loop {
        let a = r.range_i32(i32::MIN, -600_000_000);
        let b = r.range_i32(i32::MIN, -600_000_000);
        let c = r.range_i32(i32::MIN, -600_000_000);
        let d = r.range_i32(i32::MIN, -600_000_000);
        let s = a as i64 + b as i64 + c as i64 + d as i64;
        if s < i32::MIN as i64 {
            return (a, b, c, d);
        }
    });
}

// ---------------------------------------------------------------- C13
// A3/A4 longest formatted text: 51 bytes, dash_count == 7, largest buf_sum.
#[test]
fn c13_longest_buffer_all_large_negative() {
    let p = Pair::load();
    p.run_row("C13", N, |r| {
        // [INT_MIN, -1000000000] via u32 magnitudes to avoid negating INT_MIN.
        let mk = |r: &mut Rng| {
            let m = r.range_u32(1_000_000_000, 2_147_483_648);
            (m as i32).wrapping_neg()
        };
        (mk(r), mk(r), mk(r), mk(r))
    });
    // Plus the exact extreme.
    p.assert_eq_at("C13/exact", i32::MIN, i32::MIN, i32::MIN, i32::MIN);
}

// ---------------------------------------------------------------- C14
// A3 shortest formatted text: all args in -9..9.
#[test]
fn c14_shortest_buffer_single_digits() {
    let p = Pair::load();
    p.run_row("C14", N, |r| {
        (
            r.range_i32(-9, 9),
            r.range_i32(-9, 9),
            r.range_i32(-9, 9),
            r.range_i32(-9, 9),
        )
    });
    // Exhaustive over the whole -9..=9 hypercube would be 130321 calls; do the
    // full 2-arg sweep with the other two pinned, plus the all-equal diagonal.
    for a in -9..=9 {
        for b in -9..=9 {
            p.assert_eq_at("C14/sweep", a, b, 0, 0);
            p.assert_eq_at("C14/diag", a, a, b, b);
        }
    }
}

// ---------------------------------------------------------------- C15
// A3 uniform 10-digit positive magnitudes: dash_count == 3.
#[test]
fn c15_ten_digit_positives() {
    let p = Pair::load();
    p.run_row("C15", N, |r| {
        let mk = |r: &mut Rng| r.range_i32(1_000_000_000, i32::MAX);
        (mk(r), mk(r), mk(r), mk(r))
    });
}

// ---------------------------------------------------------------- C16
// Catch-all: uniform random over the whole i32^4 domain.
#[test]
fn c16_uniform_random_whole_domain() {
    let p = Pair::load();
    p.run_row("C16", 10_000, |r| {
        (r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32())
    });
}

// ---------------------------------------------------------------- C17
// A2 x A1: every float-window boundary for `a` crossed with all 16 sign
// patterns for b, c, d (and randomized magnitudes within each).
#[test]
fn c17_float_boundaries_cross_sign_patterns() {
    let p = Pair::load();
    let boundaries: [u32; 8] = [
        0,
        1,
        BITS_1_0 - 1,
        BITS_1_0,
        BITS_1000_0 - 1,
        BITS_1000_0,
        BITS_1000_0 + 1,
        0x7F7F_FFFF, // largest finite float bits
    ];
    for &bits in &boundaries {
        let a = bits as i32;
        for mask in 0u32..16 {
            let (sa, sb, sc, sd) = signs(mask);
            p.run_row(&format!("C17/{bits:#x}/{mask}"), 40, |r| {
                let mk = |r: &mut Rng, neg: bool| {
                    let m = r.range_i32(0, 2_000_000_000);
                    if neg {
                        m.wrapping_neg()
                    } else {
                        m
                    }
                };
                // `sa` toggles between the boundary value itself and its
                // negation, so the boundary is probed on both signs.
                let aa = if sa { a.wrapping_neg() } else { a };
                (aa, mk(r, sb), mk(r, sc), mk(r, sd))
            });
        }
    }
}

// ---------------------------------------------------------------- C18
// A9: identical inputs through the (u32,u32,u32,u32)->u32 FFI signature.
#[test]
fn c18_unsigned_ffi_signature() {
    let p = Pair::load();
    let mut r = Rng::new(common::FIXED_SEED ^ 0xC18);
    for _ in 0..N {
        let (a, b, c, d) = (r.next_u32(), r.next_u32(), r.next_u32(), r.next_u32());
        let gc = p.c_u(a, b, c, d);
        let gr = p.rust_u(a, b, c, d);
        assert_eq!(
            gc.to_le_bytes(),
            gr.to_le_bytes(),
            "[C18/unsigned] memchra2({a:#x}, {b:#x}, {c:#x}, {d:#x}): C = {gc:#x}, Rust = {gr:#x}"
        );
        // Same words through the signed view must agree with the unsigned view.
        let sc = p.c(a as i32, b as i32, c as i32, d as i32);
        let sr = p.rust(a as i32, b as i32, c as i32, d as i32);
        assert_eq!(sc as u32, gc, "[C18] C signed/unsigned views disagree");
        assert_eq!(sr as u32, gr, "[C18] Rust signed/unsigned views disagree");
    }
}

// ---------------------------------------------------------------- C19
// Interaction: `a` inside the 1 <= f < 1000 window AND xor-cancelling bytes.
#[test]
fn c19_float_window_times_xor_cancel() {
    let p = Pair::load();
    p.run_row("C19", N, |r| {
        let a = r.range_u32(BITS_1_0, BITS_1000_0 - 1) as i32;
        let byte = (a as u32) & 0xFF;
        let mk = |r: &mut Rng| ((r.next_u32() & 0xFFFF_FF00) | byte) as i32;
        // b, c, d share a's low byte -> xor over 4 elements cancels to 0.
        (a, mk(r), mk(r), mk(r))
    });
}

// ---------------------------------------------------------------- C20
// Interaction: wraparound x all-negative x longest buffer, together.
#[test]
fn c20_overflow_times_all_negative_longest() {
    let p = Pair::load();
    p.run_row("C20", N, |r| {
        let mk = |r: &mut Rng| {
            let m = r.range_u32(1_500_000_000, 2_147_483_648);
            (m as i32).wrapping_neg()
        };
        (mk(r), mk(r), mk(r), mk(r))
    });
}

// ---------------------------------------------------------------- C21
// Equivalent-mutant proof. `mutation_check.sh` reports two surviving mutants:
//
//   (1) snprintf cap `buffer.len() - 1` -> `- 2`
//   (2) process_buffer accumulator `(buffer[i] as i8)` -> `(buffer[i] as u8)`
//
// Both are unobservable through the only exported entry point, and this test
// proves it mechanically instead of by argument:
//
//   (1) is unobservable iff the formatted text never exceeds 62 bytes, so the
//       truncation cap is never reached for any cap >= 52. Worst case is
//       "test-2147483648--2147483648--2147483648--2147483648" = 51 bytes.
//   (2) is unobservable iff every byte of the formatted text is < 0x80, so the
//       signed and unsigned widening of `char` coincide. The text contains only
//       't','e','s','t', ASCII digits and '-' (0x2D..0x74).
//
// Verified over the full randomized domain plus the length/byte extremes.
#[test]
fn c21_unreachable_paths_are_provably_unreachable() {
    let mut r = Rng::new(common::FIXED_SEED ^ 0xC21);
    let mut max_len = 0usize;
    let mut max_byte = 0u8;

    let check = |a: i32, b: i32, c: i32, d: i32, max_len: &mut usize, max_byte: &mut u8| {
        let s = format!("test{a}-{b}-{c}-{d}");
        let bytes = s.as_bytes();
        *max_len = (*max_len).max(bytes.len());
        for &x in bytes {
            *max_byte = (*max_byte).max(x);
            assert!(
                x < 0x80,
                "byte {x:#x} >= 0x80 in {s:?}: signed/unsigned char widening \
                 would become observable"
            );
        }
        assert!(
            bytes.len() <= 51,
            "formatted length {} > 51 for ({a},{b},{c},{d}): {s:?}",
            bytes.len()
        );
    };

    // Extremes first.
    for &(a, b, c, d) in &[
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (0, 0, 0, 0),
        (i32::MIN, i32::MAX, i32::MIN, i32::MAX),
    ] {
        check(a, b, c, d, &mut max_len, &mut max_byte);
    }
    // Then 50 000 random tuples over the whole domain.
    for _ in 0..50_000 {
        let (a, b, c, d) = (r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
        check(a, b, c, d, &mut max_len, &mut max_byte);
    }

    assert_eq!(max_len, 51, "worst-case formatted length must be exactly 51");
    assert!(
        max_byte < 0x80,
        "max observed byte {max_byte:#x} must stay below 0x80"
    );
    // 51 <= 62 = (64 - 2), so the mutated cap is never reached either.
    assert!(max_len <= 62, "the mutated snprintf cap must also never bind");
}
