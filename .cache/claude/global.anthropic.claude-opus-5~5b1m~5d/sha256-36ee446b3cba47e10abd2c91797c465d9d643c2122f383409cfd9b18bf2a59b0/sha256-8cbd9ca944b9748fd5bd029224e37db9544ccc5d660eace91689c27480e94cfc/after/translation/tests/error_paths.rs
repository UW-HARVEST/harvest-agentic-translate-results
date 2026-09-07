//! Phase C — error-path differential tests, one test per ERRORS.md row.
//!
//! `encode_quant` has no rejection surface (no pointer parameters, no error
//! return, no assert, no range check), so these rows cover the *implicit*
//! boundary/overflow conditions the C relies on: exactly the inputs where a
//! non-wrapping Rust translation would panic instead of returning the C's
//! wrapping value. Each test asserts the two `.so`s return the SAME `int` and
//! that neither traps.

mod common;
use common::{check, check_args, pair, Rng, LSBIT_MODES};

const MIN: i32 = i32::MIN;
const MAX: i32 = i32::MAX;

/// Recompute the C's `diff`/`p0` for a candidate, used ONLY to *detect* which
/// randomized inputs reach a given overflow edge — never as the oracle.
fn p_of(u: i32, step: i32, pred: i32) -> i32 {
    let m = 2i32.wrapping_mul(u & 7).wrapping_add(1);
    let mut diff = m.wrapping_mul(step) / 8;
    if (u & 8) != 0 {
        diff = diff.wrapping_neg();
    }
    pred.wrapping_add(diff)
}

fn xor_abs(x: i32) -> i32 {
    x ^ (x >> 31)
}

/// Multiplicative inverse of an ODD `m` modulo 2^32 (Newton iteration). Used to
/// solve for a `step` that makes `m * step` wrap to a chosen product, so the
/// narrow overflow windows can be reached deterministically.
fn inv_mod_2_32(m: u32) -> u32 {
    assert_eq!(m & 1, 1, "only odd values are invertible mod 2^32");
    let mut inv: u32 = 1;
    for _ in 0..5 {
        inv = inv.wrapping_mul(2u32.wrapping_sub(m.wrapping_mul(inv)));
    }
    debug_assert_eq!(m.wrapping_mul(inv), 1);
    inv
}

// -------------------------------------------------------------- row 1
// No pointer arguments exist, so there is no null-pointer rejection to
// mirror. The degenerate all-zero call must still return a value.

#[test]
fn row01_no_pointer_args_all_zero_call() {
    let p = pair();
    let c = unsafe { (p.c)(0, 0, 0, 0, 0, 0) };
    let r = unsafe { (p.rust)(0, 0, 0, 0, 0, 0) };
    assert_eq!(c, r, "all-zero call diverged");
    assert_eq!(c, 0, "C returns 0 for the all-zero call");
}

// -------------------------------------------------------------- row 2
// uni = INT_MAX  =>  uni1 = uni + 1 overflows (C line 6).

#[test]
fn row02_uni_int_max_plus_one_overflow() {
    let mut rng = Rng::new(0x0002);
    for lsbit in LSBIT_MODES {
        for _ in 0..256 {
            let [step, pred, tgt, tgt2] = rng.interesting4();
            check(MAX, step, pred, tgt, tgt2, lsbit);
        }
    }
    for uni in (MAX - 4)..=MAX {
        for lsbit in LSBIT_MODES {
            check(uni, 1, 0, 0, 0, lsbit);
            check(uni, MAX, MAX, MIN, MAX, lsbit);
            check(uni, MIN, MIN, MAX, MIN, lsbit);
        }
    }
}

// -------------------------------------------------------------- row 3
// uni = INT_MIN  =>  uni2 = uni - 1 overflows (C line 7).

#[test]
fn row03_uni_int_min_minus_one_overflow() {
    let mut rng = Rng::new(0x0003);
    for lsbit in LSBIT_MODES {
        for _ in 0..256 {
            let [step, pred, tgt, tgt2] = rng.interesting4();
            check(MIN, step, pred, tgt, tgt2, lsbit);
        }
    }
    for uni in MIN..=(MIN + 4) {
        for lsbit in LSBIT_MODES {
            check(uni, 1, 0, 0, 0, lsbit);
            check(uni, MIN, MIN, MAX, MIN, lsbit);
            check(uni, MAX, MAX, MIN, MAX, lsbit);
        }
    }
}

// -------------------------------------------------------------- row 4
// step = INT_MAX  =>  (2*(uni&7)+1)*step overflows (C line 30).

#[test]
fn row04_step_int_max_multiply_overflow() {
    let mut rng = Rng::new(0x0004);
    for low3 in 0..8i32 {
        for lsbit in LSBIT_MODES {
            for _ in 0..64 {
                let uni = rng.uni_low3(low3);
                let [pred, tgt, tgt2, _] = rng.interesting4();
                check(uni, MAX, pred, tgt, tgt2, lsbit);
            }
        }
    }
    for step in (MAX - 3)..=MAX {
        for uni in 0..16i32 {
            check(uni, step, 0, 0, 0, 0);
            check(uni, step, MAX, MIN, MAX, 4);
            check(uni, step, MIN, MAX, MIN, 1);
        }
    }
}

// -------------------------------------------------------------- row 5
// step = INT_MIN  =>  overflow AND diff = -diff applied to a value that may
// be INT_MIN (C lines 30-32).

#[test]
fn row05_step_int_min_overflow_and_negate() {
    let mut rng = Rng::new(0x0005);
    for low4 in 0..16i32 {
        for lsbit in LSBIT_MODES {
            for _ in 0..64 {
                let uni = rng.uni_low4(low4 & 7, low4 & 8 != 0);
                let [pred, tgt, tgt2, _] = rng.interesting4();
                check(uni, MIN, pred, tgt, tgt2, lsbit);
            }
        }
    }
    for step in MIN..=(MIN + 3) {
        for uni in 0..16i32 {
            check(uni, step, 0, 0, 0, 0);
            check(uni, step, MIN, MAX, MIN, 4);
            check(uni, step, MAX, MIN, MAX, 1);
        }
    }
}

// -------------------------------------------------------------- row 6
// `diff = -diff` when `uni & 8` is set (C line 32).
//
// NOTE (verified by exhaustive/40M-sample search over the (uni&7, step) space):
// `diff == INT_MIN` is UNREACHABLE. `diff` is `(m * step) / 8` where the
// wrapped product is an i32, so the `/ 8` bounds `diff` to exactly
// [-268435456, 268435455] == [INT_MIN/8, INT_MAX/8]. Hence the unary minus on
// line 32 can never overflow. This row therefore pins the ATTAINABLE extremes
// of `diff` and asserts the negation matches there, rather than asserting an
// impossible condition.

#[test]
fn row06_negate_diff_at_attainable_extremes() {
    const DIFF_MIN: i32 = MIN / 8; // -268435456
    const DIFF_MAX: i32 = MAX / 8; //  268435455

    // Confirm the bound holds across a broad random sweep.
    let mut rng = Rng::new(0x0006);
    for _ in 0..200_000 {
        let uni = rng.range_i32(8, 15); // bit 3 set => negation happens
        let step = rng.next_i32();
        let m = 2i32.wrapping_mul(uni & 7).wrapping_add(1);
        let diff = m.wrapping_mul(step) / 8;
        assert!(
            (DIFF_MIN..=DIFF_MAX).contains(&diff),
            "diff {diff} escaped [{DIFF_MIN}, {DIFF_MAX}] for uni={uni} step={step}"
        );
        check(uni, step, 0, 0, 0, 0);
    }

    // DETERMINISTICALLY realise both extremes for every multiplier. The window
    // of products mapping to DIFF_MIN is only 8 wide out of 2^32, so random
    // search cannot find it; instead SOLVE for `step`. Every multiplier
    // m = 2*(uni&7)+1 is odd, hence invertible mod 2^32, so
    // step = product * m^-1 makes `m * step` wrap to exactly `product`.
    let mut hit_min = 0usize;
    let mut hit_max = 0usize;
    for low3 in 0..8i32 {
        let m = 2 * low3 + 1;
        let inv = inv_mod_2_32(m as u32);
        for &product in &[MIN, MAX, MIN + 7, MAX - 7] {
            let step = (product as u32).wrapping_mul(inv) as i32;
            assert_eq!(
                m.wrapping_mul(step),
                product,
                "modular inverse failed for m={m}"
            );
            let diff = product / 8;
            if diff == DIFF_MIN {
                hit_min += 1;
            }
            if diff == DIFF_MAX {
                hit_max += 1;
            }
            // Drive it with bit 3 both set (negation runs) and clear.
            for bit3 in [0i32, 8] {
                let uni = low3 | bit3;
                for pred in [MIN, -1, 0, 1, MAX] {
                    for tgt in [MIN, 0, MAX] {
                        for tgt2 in [MIN, 0, MAX] {
                            for lsbit in LSBIT_MODES {
                                check(uni, step, pred, tgt, tgt2, lsbit);
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(hit_min > 0, "row 6: never realised diff == {DIFF_MIN}");
    assert!(hit_max > 0, "row 6: never realised diff == {DIFF_MAX}");

    // Deterministically drive both extremes for the multiplier m == 1
    // (uni & 7 == 0), where diff == step / 8 exactly.
    for &uni in &[8i32, 0] {
        for step in [MIN, MIN + 1, MIN + 7, MAX - 7, MAX - 1, MAX] {
            for pred in [MIN, 0, MAX] {
                for lsbit in LSBIT_MODES {
                    check(uni, step, pred, MIN, MAX, lsbit);
                    check(uni, step, pred, MAX, MIN, lsbit);
                }
            }
        }
    }
}

// -------------------------------------------------------------- row 7
// pred + diff overflows (C lines 33/39/45).

#[test]
fn row07_pred_plus_diff_overflow() {
    let mut rng = Rng::new(0x0007);
    for pred in [MIN, MIN + 1, MIN + 7, -1, 0, 1, MAX - 7, MAX - 1, MAX] {
        for lsbit in LSBIT_MODES {
            for _ in 0..128 {
                let uni = rng.interesting_i32();
                let step = rng.next_i32();
                let tgt = rng.interesting_i32();
                let tgt2 = rng.interesting_i32();
                check(uni, step, pred, tgt, tgt2, lsbit);
            }
        }
    }
}

// -------------------------------------------------------------- row 8
// tgt - p0 overflows: tgt = INT_MAX with p0 < 0, and the mirror
// (C lines 34/40/46).

#[test]
fn row08_tgt_minus_p_overflow() {
    let mut rng = Rng::new(0x0008);
    for &tgt in &[MIN, MIN + 1, MAX - 1, MAX] {
        for lsbit in LSBIT_MODES {
            for _ in 0..128 {
                // pred of the opposite sign so the subtraction overflows.
                let off = rng.range_i32(0, 64);
                let pred = if tgt > 0 { MIN + off } else { MAX - off };
                let uni = rng.interesting_i32();
                let step = rng.next_i32();
                let tgt2 = rng.interesting_i32();
                check(uni, step, pred, tgt, tgt2, lsbit);
            }
        }
    }
    // Deterministic: step == 0 => p0 == pred, so tgt - p0 overflows exactly.
    for lsbit in LSBIT_MODES {
        check(0, 0, MIN, MAX, 0, lsbit);
        check(0, 0, MAX, MIN, 0, lsbit);
        check(0, 0, -1, MAX, 0, lsbit);
        check(0, 0, 1, MIN, 0, lsbit);
    }
}

// -------------------------------------------------------------- row 9
// tgt2 - p0 overflows (C lines 48/51/54).

#[test]
fn row09_tgt2_minus_p_overflow() {
    let mut rng = Rng::new(0x0009);
    for &tgt2 in &[MIN, MIN + 1, MAX - 1, MAX] {
        for lsbit in LSBIT_MODES {
            for _ in 0..128 {
                let off = rng.range_i32(0, 64);
                let pred = if tgt2 > 0 { MIN + off } else { MAX - off };
                let uni = rng.interesting_i32();
                let step = rng.next_i32();
                let tgt = rng.interesting_i32();
                check(uni, step, pred, tgt, tgt2, lsbit);
            }
        }
    }
    for lsbit in LSBIT_MODES {
        check(0, 0, MIN, 0, MAX, lsbit);
        check(0, 0, MAX, 0, MIN, lsbit);
        check(0, 0, -1, 0, MAX, lsbit);
        check(0, 0, 1, 0, MIN, lsbit);
    }
}

// -------------------------------------------------------------- row 10
// d ^ (d >> 31) where d == INT_MIN  =>  INT_MAX, not INT_MIN (C line 35).
// This is the abs-via-xor idiom, which cannot represent +2^31.

#[test]
fn row10_xor_abs_of_int_min() {
    // Deterministic constructions: step == 0 => p0 == p1 == p2 == pred, so
    // tgt - p0 == INT_MIN exactly when pred == 0 and tgt == INT_MIN.
    for lsbit in LSBIT_MODES {
        check(0, 0, 0, MIN, MIN, lsbit);
        check(8, 0, 0, MIN, 0, lsbit);
        check(0, 0, 0, 0, MIN, lsbit);
        check(0, 0, 1, MIN + 1, MIN + 1, lsbit);
        check(0, 0, -1, MAX, MAX, lsbit);
    }

    // DETERMINISTIC construction of d == INT_MIN. A uniform random sweep hits
    // `tgt - p0 == INT_MIN` with probability 2^-32 per draw, so instead solve
    // for it: pick uni/step/pred freely, compute p0, then set
    // tgt = p0 + INT_MIN (wrapping), which forces tgt - p0 == INT_MIN exactly.
    let mut rng = Rng::new(0x000a);
    let mut hits = 0usize;
    for _ in 0..20_000 {
        let uni = rng.range_i32(-64, 64);
        let step = rng.next_i32();
        let pred = rng.next_i32();

        let p0 = p_of(uni, step, pred);
        let tgt = p0.wrapping_add(MIN);
        assert_eq!(
            tgt.wrapping_sub(p0),
            MIN,
            "construction failed for uni={uni} step={step} pred={pred}"
        );
        hits += 1;

        // Same trick for the secondary target, so `d3 == INT_MIN` too.
        let tgt2 = p0.wrapping_add(MIN);
        for lsbit in LSBIT_MODES {
            check(uni, step, pred, tgt, tgt2, lsbit);
            check(uni, step, pred, tgt, rng_free(&mut rng), lsbit);
        }
    }
    assert!(hits > 0, "row 10: never produced d == INT_MIN");

    // Also force d1 / d2 (not just d0) to INT_MIN, via the uni+1 / uni-1
    // candidates, so the edge is hit on every candidate.
    let mut rng = Rng::new(0xA10A);
    for _ in 0..20_000 {
        let uni = rng.range_i32(-64, 64);
        let step = rng.next_i32();
        let pred = rng.next_i32();
        let mut u1 = uni.wrapping_add(1);
        let mut u2 = uni.wrapping_sub(1);
        if ((uni ^ u1) & !7) != 0 {
            u1 = uni;
        }
        if ((uni ^ u2) & !7) != 0 {
            u2 = uni;
        }
        for cand in [u1, u2] {
            let p = p_of(cand, step, pred);
            let tgt = p.wrapping_add(MIN);
            for lsbit in LSBIT_MODES {
                check(uni, step, pred, tgt, tgt, lsbit);
            }
        }
    }
}

/// A free random `i32`, used where the value is unconstrained.
fn rng_free(rng: &mut Rng) -> i32 {
    rng.next_i32()
}

// -------------------------------------------------------------- row 11
// d3 >> 5 must be an ARITHMETIC (sign-propagating) shift (C lines 50/53/56).
// After the xor-abs, d3 is non-negative for every input EXCEPT d3 == INT_MIN,
// which becomes INT_MAX; the shift semantics still matter for the sign bit,
// so sweep the whole range plus that exact edge.

#[test]
fn row11_d3_shift_right_five_arithmetic() {
    let mut rng = Rng::new(0x000b);
    for _ in 0..100_000 {
        // Force large |tgt2 - p| so the >>5 term is significant.
        let pred = rng.range_i32(-(1 << 28), 1 << 28);
        let tgt2 = rng.next_i32();
        let uni = rng.range_i32(-32, 32);
        let step = rng.next_i32();
        let tgt = rng.next_i32();
        let lsbit = rng.lsbit();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
    // The exact INT_MIN -> INT_MAX >> 5 case.
    for lsbit in LSBIT_MODES {
        check(0, 0, 0, 0, MIN, lsbit);
        check(0, 0, MAX, 0, MIN, lsbit);
        check(0, 0, MIN, 0, MAX, lsbit);
        check(0, 0, 0, MIN, MIN, lsbit);
    }
    // Every |d3| in 0..64 so each distinct d3>>5 result (0 and 1) is covered
    // on both sides of zero.
    for delta in -64..=64i32 {
        for lsbit in LSBIT_MODES {
            check(0, 0, 0, 0, delta, lsbit);
            check(0, 0, delta, 0, 0, lsbit);
        }
    }
}

// -------------------------------------------------------------- row 12
// d0 += d3 >> 5 overflows (C lines 50/53/56).

#[test]
fn row12_penalty_add_overflow() {
    let mut rng = Rng::new(0x000c);
    let mut hits = 0usize;
    for _ in 0..200_000 {
        let uni = rng.range_i32(-16, 16);
        let step = rng.next_i32();
        let pred = rng.next_i32();
        let tgt = rng.next_i32();
        let tgt2 = rng.next_i32();
        let lsbit = rng.lsbit();
        check(uni, step, pred, tgt, tgt2, lsbit);

        let p0 = p_of(uni, step, pred);
        let d0 = xor_abs(tgt.wrapping_sub(p0));
        let d3 = xor_abs(tgt2.wrapping_sub(p0));
        if d0.checked_add(d3 >> 5).is_none() {
            hits += 1;
            for m in LSBIT_MODES {
                check(uni, step, pred, tgt, tgt2, m);
            }
        }
    }
    assert!(hits > 0, "row 12: never triggered the penalty-add overflow");
}

// -------------------------------------------------------------- row 13
// ((2*(uni&7)+1)*step)/8 with a negative numerator must TRUNCATE TOWARD ZERO,
// not floor (C line 30).

#[test]
fn row13_division_truncates_toward_zero() {
    // For every low-3-bit multiplier, construct numerators with a nonzero
    // remainder mod 8, where floor-division would differ from C truncation.
    for low3 in 0..8i32 {
        let m = 2 * low3 + 1;
        for r in 1..8i32 {
            for k in 0..8i32 {
                let target = -(k * 8 + r);
                if target % m == 0 {
                    let step = target / m;
                    for bit3 in [0i32, 8] {
                        for lsbit in LSBIT_MODES {
                            check(low3 | bit3, step, 0, 0, 0, lsbit);
                            check(low3 | bit3, step, 100, -100, 5000, lsbit);
                        }
                    }
                }
            }
        }
    }
    // Dense small-negative-step sweep: every step in -64..0 against every
    // low-4-bit uni, so every (multiplier, remainder) pair is covered.
    for step in -64..0i32 {
        for uni in 0..16i32 {
            for lsbit in LSBIT_MODES {
                check(uni, step, 0, 0, 0, lsbit);
            }
        }
    }
    // Broad randomized negative-step sweep.
    let mut rng = Rng::new(0x000d);
    for _ in 0..100_000 {
        let step = -rng.range_i32(1, 1 << 30);
        let uni = rng.range_i32(-64, 64);
        let [pred, tgt, tgt2, _] = rng.interesting4();
        let lsbit = rng.lsbit();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// -------------------------------------------------------------- row 14
// uni negative => uni & 7 / uni & 8 operate on the two's-complement pattern
// (e.g. -1 & 7 == 7, not -1).

#[test]
fn row14_negative_uni_masks() {
    for uni in -128..0i32 {
        for lsbit in LSBIT_MODES {
            for step in [-17i32, -1, 0, 1, 17, MAX, MIN] {
                check(uni, step, 0, 0, 0, lsbit);
                check(uni, step, -5, 9, -9999, lsbit);
            }
        }
    }
    let mut rng = Rng::new(0x000e);
    for _ in 0..100_000 {
        let uni = -rng.range_i32(1, MAX);
        let step = rng.next_i32();
        let pred = rng.next_i32();
        let tgt = rng.next_i32();
        let tgt2 = rng.next_i32();
        let lsbit = rng.lsbit();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
}

// -------------------------------------------------------------- row 15
// uni >> 1 and uni >> 2 with uni negative in the lsbit == 4 dither path
// must be ARITHMETIC shifts (C line 17).

#[test]
fn row15_lsbit4_negative_arithmetic_shift() {
    for uni in -256..=0i32 {
        for step in [-9i32, -1, 0, 1, 9, 64, MAX, MIN] {
            check(uni, step, 0, 0, 0, 4);
            check(uni, step, 77, -77, 123_456, 4);
        }
    }
    for &uni in &[MIN, MIN + 1, MIN + 2, MAX - 2, MAX - 1, MAX] {
        for step in [0i32, 1, -1, MAX, MIN] {
            check(uni, step, 0, 0, 0, 4);
            check(uni, step, MIN, MAX, MIN, 4);
        }
    }
    let mut rng = Rng::new(0x000f);
    for _ in 0..100_000 {
        let uni = -rng.range_i32(1, MAX);
        let step = rng.next_i32();
        let pred = rng.next_i32();
        let tgt = rng.next_i32();
        let tgt2 = rng.next_i32();
        check(uni, step, pred, tgt, tgt2, 4);
    }
}

// -------------------------------------------------------------- row 16
// lsbit == 4 exactly takes the dither branch, NOT the generic even branch.

#[test]
fn row16_lsbit_exactly_four() {
    let mut rng = Rng::new(0x0010);
    for _ in 0..20_000 {
        let [uni, step, pred, tgt, tgt2] = rng.interesting5();
        check(uni, step, pred, tgt, tgt2, 4);
    }
    // Prove lsbit==4 is genuinely distinguishable from lsbit==2 (an even
    // neighbour), and that both impls agree on the distinction.
    let p = pair();
    let mut differs = false;
    let mut rng = Rng::new(0x1004);
    for _ in 0..50_000 {
        let u = rng.range_i32(-64, 64);
        let s = rng.range_i32(-64, 64);
        let pr = rng.range_i32(-64, 64);
        let t = rng.range_i32(-64, 64);
        let t2 = rng.range_i32(-4096, 4096);
        check(u, s, pr, t, t2, 4);
        check(u, s, pr, t, t2, 2);
        let a = unsafe { (p.c)(u, s, pr, t, t2, 4) };
        let b = unsafe { (p.c)(u, s, pr, t, t2, 2) };
        if a != b {
            differs = true;
        }
    }
    assert!(
        differs,
        "lsbit==4 branch is never distinguishable from lsbit==2"
    );
}

// -------------------------------------------------------------- row 17
// lsbit odd (and != 4), including negative odd and INT_MAX.

#[test]
fn row17_lsbit_odd_including_negative_and_int_max() {
    let odds: Vec<i32> = (-11..=11i32)
        .filter(|v| v % 2 != 0)
        .chain([MAX, MIN + 1, -0x7FFF_FFFF])
        .collect();
    let mut rng = Rng::new(0x0011);
    for &lsbit in &odds {
        assert_ne!(lsbit & 1, 0, "{lsbit} must be odd");
        assert_ne!(lsbit, 4);
        for _ in 0..512 {
            let [uni, step, pred, tgt, tgt2] = rng.interesting5();
            check(uni, step, pred, tgt, tgt2, lsbit);
        }
    }
}

// -------------------------------------------------------------- row 18
// lsbit even, nonzero, != 4, including negative even and INT_MIN.

#[test]
fn row18_lsbit_even_nonzero_including_negative_and_int_min() {
    let evens: Vec<i32> = (-12..=12i32)
        .filter(|v| *v != 0 && *v != 4 && v % 2 == 0)
        .chain([MIN, MAX - 1, -0x7FFF_FFFE])
        .collect();
    let mut rng = Rng::new(0x0012);
    for &lsbit in &evens {
        assert_eq!(lsbit & 1, 0, "{lsbit} must be even");
        assert_ne!(lsbit, 0);
        assert_ne!(lsbit, 4);
        for _ in 0..512 {
            let [uni, step, pred, tgt, tgt2] = rng.interesting5();
            check(uni, step, pred, tgt, tgt2, lsbit);
        }
    }
}

// -------------------------------------------------------------- row 19
// lsbit == 0 skips the whole conditioning block.

#[test]
fn row19_lsbit_zero_skips_block() {
    let mut rng = Rng::new(0x0013);
    for _ in 0..50_000 {
        let [uni, step, pred, tgt, tgt2] = rng.interesting5();
        check(uni, step, pred, tgt, tgt2, 0);
    }
}

// -------------------------------------------------------------- row 20
// Out-of-enum-range lsbit. `lsbit` is a plain C int, so ANY 32-bit value is a
// real input the C handles; assert the Rust classifies each into the same
// branch, including the values with no "documented" meaning.

#[test]
fn row20_lsbit_out_of_range_extremes() {
    let p = pair();
    let mut rng = Rng::new(0x0014);
    for _ in 0..20_000 {
        let [u, s, pr, t, t2] = rng.interesting5();
        for &lsbit in &[MIN, MIN + 1, MAX, MAX - 1, -1, -2, 5, 6, 100, 101] {
            check(u, s, pr, t, t2, lsbit);
        }
        // INT_MAX is odd => must behave exactly like lsbit = 1.
        let a = unsafe { (p.c)(u, s, pr, t, t2, MAX) };
        let b = unsafe { (p.c)(u, s, pr, t, t2, 1) };
        assert_eq!(a, b, "C: INT_MAX lsbit should act like 1");
        let ar = unsafe { (p.rust)(u, s, pr, t, t2, MAX) };
        assert_eq!(a, ar, "Rust diverged for lsbit = INT_MAX");
        // INT_MIN is even and != 4 => must behave exactly like lsbit = 2.
        let c = unsafe { (p.c)(u, s, pr, t, t2, MIN) };
        let d = unsafe { (p.c)(u, s, pr, t, t2, 2) };
        assert_eq!(c, d, "C: INT_MIN lsbit should act like 2");
        let cr = unsafe { (p.rust)(u, s, pr, t, t2, MIN) };
        assert_eq!(c, cr, "Rust diverged for lsbit = INT_MIN");
    }
}

// -------------------------------------------------------------- row 21
// uni&7 == 7 => uni1 clamped back to uni (C line 8).

#[test]
fn row21_uni1_clamped_at_group_top() {
    let mut rng = Rng::new(0x0015);
    for _ in 0..50_000 {
        let uni = (rng.next_i32() & !7) | 7;
        let step = rng.next_i32();
        let pred = rng.next_i32();
        let tgt = rng.next_i32();
        let tgt2 = rng.next_i32();
        let lsbit = rng.lsbit();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
    // Including the INT_MAX group top (0x7FFFFFFF has low3 == 7) and -1.
    for lsbit in LSBIT_MODES {
        check(MAX, 1, 0, 0, 0, lsbit);
        check(MAX, MAX, MAX, MIN, MAX, lsbit);
        check(-1, 1, 0, 0, 0, lsbit);
        check(-1, MIN, MIN, MAX, MIN, lsbit);
    }
}

// -------------------------------------------------------------- row 22
// uni&7 == 0 => uni2 clamped back to uni (C line 10).

#[test]
fn row22_uni2_clamped_at_group_bottom() {
    let mut rng = Rng::new(0x0016);
    for _ in 0..50_000 {
        let uni = rng.next_i32() & !7;
        let step = rng.next_i32();
        let pred = rng.next_i32();
        let tgt = rng.next_i32();
        let tgt2 = rng.next_i32();
        let lsbit = rng.lsbit();
        check(uni, step, pred, tgt, tgt2, lsbit);
    }
    // Including the INT_MIN group bottom (0x80000000 has low3 == 0).
    for lsbit in LSBIT_MODES {
        check(MIN, 1, 0, 0, 0, lsbit);
        check(MIN, MIN, MIN, MAX, MIN, lsbit);
        check(0, 1, 0, 0, 0, lsbit);
        check(0, MAX, MAX, MIN, MAX, lsbit);
    }
}

// -------------------------------------------------------------- row 23
// step == 0 => all diffs zero, every candidate ties.

#[test]
fn row23_step_zero_no_candidate_wins() {
    let mut rng = Rng::new(0x0017);
    for _ in 0..50_000 {
        let uni = rng.interesting_i32();
        let pred = rng.interesting_i32();
        let tgt = rng.interesting_i32();
        let tgt2 = rng.interesting_i32();
        let lsbit = rng.lsbit();
        check(uni, 0, pred, tgt, tgt2, lsbit);
    }
    for uni in -64..=64i32 {
        for lsbit in LSBIT_MODES {
            check(uni, 0, 7, -7, 999, lsbit);
            check(uni, 0, MIN, MAX, MIN, lsbit);
        }
    }
}

// -------------------------------------------------------------- row 24
// Tie: d1 == d0 and d2 == d0 => the strict `<` (C lines 57/59) keeps uni.

#[test]
fn row24_strict_less_than_keeps_uni_on_ties() {
    // step == 0 forces p0 == p1 == p2, hence exact ties everywhere.
    let mut rng = Rng::new(0x0018);
    for _ in 0..20_000 {
        let uni = rng.range_i32(-1024, 1024);
        let pred = rng.range_i32(-1024, 1024);
        for lsbit in LSBIT_MODES {
            check(uni, 0, pred, pred, pred, lsbit);
        }
    }
    // Small |step| that truncates to 0 for every multiplier also ties.
    for uni in -64..=64i32 {
        for step in 1..8i32 {
            for lsbit in LSBIT_MODES {
                check(uni, step, 0, 0, 0, lsbit);
            }
        }
    }
}

// -------------------------------------------------------------- row 25
// Both d1 < d0 and d2 < d0 => the SECOND `if` wins (returns uni2), even when
// d1 < d2. This is the C's quirk; assert the Rust reproduces it exactly.

// NOTE: with SMALL, non-overflowing inputs this case is unreachable: d(p) is a
// convex function of p and p0 lies strictly between p1 and p2, so
// d0 <= max(d1, d2). It becomes reachable once the subtractions overflow (which
// breaks the convexity argument). A 20M-sample full-range search finds it ~18%
// of the time, so this test draws from the FULL i32 range, not a small window.

#[test]
fn row25_both_better_second_if_overwrites() {
    let p = pair();
    let mut rng = Rng::new(0x0019);
    let mut confirmed = 0usize;
    for _ in 0..400_000 {
        // Full-range draws: the both-better case only occurs with overflow.
        let uni = rng.next_i32();
        let step = rng.next_i32();
        let pred = rng.next_i32();
        let tgt = rng.next_i32();
        let tgt2 = rng.next_i32();
        let c = unsafe { (p.c)(uni, step, pred, tgt, tgt2, 0) };
        let r = unsafe { (p.rust)(uni, step, pred, tgt, tgt2, 0) };
        assert_eq!(c, r, "diverged at ({uni},{step},{pred},{tgt},{tgt2},0)");

        let (d0, d1, d2, u1, u2) = distortions(uni, step, pred, tgt, tgt2);
        if d1 < d0 && d2 < d0 && u1 != uni && u2 != uni {
            // Both candidates beat uni, so the SECOND `if` must win: uni2.
            assert_eq!(
                c, u2,
                "C should return uni2 when both are better \
                 (d0={d0} d1={d1} d2={d2}) at ({uni},{step},{pred},{tgt},{tgt2})"
            );
            confirmed += 1;
        }
    }
    assert!(
        confirmed > 0,
        "row 25: never hit the both-better case in 400k full-range draws"
    );

    // Deterministic examples found by the offline reachability search, where
    // d1 < d0 AND d2 < d0 both hold; uni2 must win in every one.
    const BOTH_BETTER: [(i32, i32, i32, i32, i32); 5] = [
        (1, -1073741824, MIN, MIN, 100000),
        (1, -1073741824, MIN, MIN, 1073741823),
        (1, -1073741824, MIN, MIN + 1, 100000),
        (1, -1073741824, MIN, MIN + 1, 1073741823),
        (1, -1073741824, MIN, -100000, 1073741823),
    ];
    for &(uni, step, pred, tgt, tgt2) in &BOTH_BETTER {
        let (d0, d1, d2, _u1, u2) = distortions(uni, step, pred, tgt, tgt2);
        assert!(
            d1 < d0 && d2 < d0,
            "fixture ({uni},{step},{pred},{tgt},{tgt2}) is not both-better: \
             d0={d0} d1={d1} d2={d2}"
        );
        let c = unsafe { (p.c)(uni, step, pred, tgt, tgt2, 0) };
        let r = unsafe { (p.rust)(uni, step, pred, tgt, tgt2, 0) };
        assert_eq!(c, r, "diverged at ({uni},{step},{pred},{tgt},{tgt2},0)");
        assert_eq!(c, u2, "second `if` must overwrite the first");
    }
}

/// Recompute the C's three distortions (lsbit == 0 path) for classification.
fn distortions(uni: i32, step: i32, pred: i32, tgt: i32, tgt2: i32) -> (i32, i32, i32, i32, i32) {
    let mut u1 = uni.wrapping_add(1);
    let mut u2 = uni.wrapping_sub(1);
    if ((uni ^ u1) & !7) != 0 {
        u1 = uni;
    }
    if ((uni ^ u2) & !7) != 0 {
        u2 = uni;
    }
    let (p0, p1, p2) = (
        p_of(uni, step, pred),
        p_of(u1, step, pred),
        p_of(u2, step, pred),
    );
    let d0 = xor_abs(tgt.wrapping_sub(p0)).wrapping_add(xor_abs(tgt2.wrapping_sub(p0)) >> 5);
    let d1 = xor_abs(tgt.wrapping_sub(p1)).wrapping_add(xor_abs(tgt2.wrapping_sub(p1)) >> 5);
    let d2 = xor_abs(tgt.wrapping_sub(p2)).wrapping_add(xor_abs(tgt2.wrapping_sub(p2)) >> 5);
    (d0, d1, d2, u1, u2)
}

// -------------------------------------------------------------- row 26
// All six args INT_MIN simultaneously (every overflow at once).

#[test]
fn row26_all_args_int_min() {
    let p = pair();
    let c = unsafe { (p.c)(MIN, MIN, MIN, MIN, MIN, MIN) };
    let r = unsafe { (p.rust)(MIN, MIN, MIN, MIN, MIN, MIN) };
    assert_eq!(c, r, "all-INT_MIN diverged: C={c:#x} Rust={r:#x}");
    // Every subset of the six args set to INT_MIN, the rest 0.
    for mask in 0u32..64 {
        let v = |bit: u32| if mask & (1 << bit) != 0 { MIN } else { 0 };
        check(v(0), v(1), v(2), v(3), v(4), v(5));
    }
}

// -------------------------------------------------------------- row 27
// All six args INT_MAX, plus the full extreme-value cross-product.

#[test]
fn row27_all_args_int_max_and_extreme_cross_product() {
    let p = pair();
    let c = unsafe { (p.c)(MAX, MAX, MAX, MAX, MAX, MAX) };
    let r = unsafe { (p.rust)(MAX, MAX, MAX, MAX, MAX, MAX) };
    assert_eq!(c, r, "all-INT_MAX diverged: C={c:#x} Rust={r:#x}");

    // 6^6 = 46656 calls over the extreme set, in every parameter position.
    const EX: [i32; 6] = [MIN, MIN + 1, -1, 0, 1, MAX];
    for &a in &EX {
        for &b in &EX {
            for &cc in &EX {
                for &d in &EX {
                    for &e in &EX {
                        for &f in &EX {
                            check(a, b, cc, d, e, f);
                        }
                    }
                }
            }
        }
    }
    // And the 4/7/8-boundary set, which drives the lsbit and mask branches.
    const B2: [i32; 7] = [-8, -7, -4, 2, 4, 7, 8];
    for &a in &B2 {
        for &b in &B2 {
            for &cc in &B2 {
                for &f in &B2 {
                    check(a, b, cc, 0, 0, f);
                    check(a, b, cc, MIN, MAX, f);
                }
            }
        }
    }
}

// ---------------------------------------------------- generic boundaries

/// Zero / oversized / one-past-range values in every parameter position,
/// swept independently while the other five are randomized.
#[test]
fn generic_per_parameter_boundary_sweep() {
    const BOUNDS: [i32; 15] = [
        MIN,
        MIN + 1,
        MIN + 2,
        -8,
        -7,
        -1,
        0,
        1,
        2,
        4,
        5,
        7,
        8,
        MAX - 1,
        MAX,
    ];
    let mut rng = Rng::new(0xB0DE);
    for pos in 0..6usize {
        for &b in &BOUNDS {
            for _ in 0..256 {
                let mut a = rng.interesting_args();
                a[pos] = b;
                check_args(a);
            }
        }
    }
}
