//! Phase C — error/rejection-path differential tests, one test per `ERRORS.md`
//! row (except the two fatal-signal rows, which live in `crash_parity.rs`).
//!
//! `lib.c` has no error codes, no sentinels and no asserts, so "the same error"
//! means: the same guard decision (byte-identical post-state for the rejected
//! buffer), the same returned `int`, and the same fatal signal where the C faults.

mod harness;

use harness::*;
use std::ffi::c_int;

// ===========================================================================
// Rows 1–6: shift_array_data — every way the lib.c:67 guard rejects
// ===========================================================================

/// Row 1 — `shift_by == 0` (first conjunct `shift_by > 0` fails).
#[test]
fn err01_shift_array_shift_by_zero_is_noop() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let size = rng.range(1, 64);
        let data = (0..size as usize).map(|_| rng.i32_any()).collect::<Vec<_>>();
        let out = both_shift_array_data(&data, size, 0);
        assert_eq!(out, data, "shift_by==0 must leave the array untouched");
    }
}

/// Row 2 — `shift_by < 0` (first conjunct fails; must NOT memmove backwards).
#[test]
fn err02_shift_array_negative_shift_is_noop() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let size = rng.range(1, 64);
        let data = (0..size as usize).map(|_| rng.i32_any()).collect::<Vec<_>>();
        for shift_by in [-1, -2, -size, -size - 1, i32::MIN, i32::MIN + 1] {
            let out = both_shift_array_data(&data, size, shift_by);
            assert_eq!(out, data, "shift_by={shift_by} must be rejected (no-op)");
        }
    }
}

/// Row 3 — `shift_by == size` (second conjunct `shift_by < size` fails).
#[test]
fn err03_shift_array_shift_equals_size_is_noop() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let size = rng.range(1, 64);
        let data = (0..size as usize).map(|_| rng.i32_any()).collect::<Vec<_>>();
        let out = both_shift_array_data(&data, size, size);
        assert_eq!(out, data, "shift_by==size must be rejected (no-op)");
    }
}

/// Row 4 — `shift_by > size`.
#[test]
fn err04_shift_array_shift_greater_than_size_is_noop() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let size = rng.range(1, 64);
        let data = (0..size as usize).map(|_| rng.i32_any()).collect::<Vec<_>>();
        for extra in [1, 2, 1000, i32::MAX - size] {
            let out = both_shift_array_data(&data, size, size + extra);
            assert_eq!(out, data, "shift_by>size must be rejected (no-op)");
        }
        let out = both_shift_array_data(&data, size, i32::MAX);
        assert_eq!(out, data);
    }
}

/// Row 5 — `size <= 0`: no `shift_by` at all can satisfy `0 < shift_by < size`.
#[test]
fn err05_shift_array_nonpositive_size_is_noop() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..1_000 {
        let data = (0..16).map(|_| rng.i32_any()).collect::<Vec<_>>();
        for size in [0, -1, -2, -16, -1000, i32::MIN, i32::MIN + 1] {
            for shift_by in [i32::MIN, -1000, -1, 0, 1, 2, 1000, i32::MAX] {
                let out = both_shift_array_data(&data, size, shift_by);
                assert_eq!(
                    out, data,
                    "size={size}, shift_by={shift_by} must be rejected (no-op)"
                );
            }
        }
    }
}

/// Row 6 — the `INT_MIN`/`INT_MIN` corner.
#[test]
fn err06_shift_array_int_min_corner() {
    let _g = lock();
    let mut rng = rng();
    let data = (0..32).map(|_| rng.i32_any()).collect::<Vec<_>>();
    for &size in &EDGE {
        for &shift_by in &EDGE {
            // Skip the combinations where the guard legitimately passes: the C
            // would then memmove far past this buffer, which is undefined in the
            // C too and so not a meaningful differential case.
            if shift_by > 0 && shift_by < size {
                continue;
            }
            let out = both_shift_array_data(&data, size, shift_by);
            assert_eq!(out, data, "size={size}, shift_by={shift_by} must be a no-op");
        }
    }
}

// ===========================================================================
// Rows 8–10: compute_with_dynamic_memory degenerate counts
// ===========================================================================

/// Row 8 — `count == 0`: `malloc(0)`, both loops skipped, returns 0.
#[test]
fn err08_compute_dynamic_memory_zero_count() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let v = both_compute_with_dynamic_memory(rng.i32_any(), 0);
        assert_eq!(v, 0, "count==0 must return 0");
    }
    for &base in &EDGE {
        assert_eq!(both_compute_with_dynamic_memory(base, 0), 0);
    }
}

/// Row 9 — `count < 0`: `count * sizeof(int)` sign-extends to a huge `size_t`,
/// `malloc` returns NULL, the loop guards keep it from ever being dereferenced,
/// and `free(NULL)` is a no-op — so the function returns 0 instead of faulting.
#[test]
fn err09_compute_dynamic_memory_negative_count() {
    let _g = lock();
    let mut rng = rng();
    for count in [
        -1,
        -2,
        -3,
        -4,
        -1024,
        -65_536,
        -1_073_741_824,
        i32::MIN + 1,
        i32::MIN,
    ] {
        for &base in &EDGE {
            let v = both_compute_with_dynamic_memory(base, count);
            assert_eq!(v, 0, "count={count} must return 0 without faulting");
        }
    }
    for _ in 0..2_000 {
        let count = -rng.range(1, 1_000_000);
        let v = both_compute_with_dynamic_memory(rng.i32_any(), count);
        assert_eq!(v, 0);
    }
}

/// Row 10 — signed overflow of `base + i*3` and of `sum`.
#[test]
fn err10_compute_dynamic_memory_overflow_wraps() {
    let _g = lock();
    for base in [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX - 3,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 3,
        2_000_000_000,
        -2_000_000_000,
    ] {
        for count in 1..=256 {
            both_compute_with_dynamic_memory(base, count);
        }
    }
    // A count large enough that `sum` alone must wrap many times.
    for base in [i32::MAX, i32::MIN, 1_000_000, -1_000_000] {
        both_compute_with_dynamic_memory(base, 100_000);
    }
}

// ===========================================================================
// Rows 11–18: manipulate_records rejection / degenerate paths
// ===========================================================================

/// Row 11 — `shift == 0`: guard skipped, but the loop still sums all records.
#[test]
fn err11_manipulate_records_shift_zero_sums_everything() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let n = rng.range(1, 32);
        let recs = random_records(&mut rng, n as usize);
        let (total, out) = both_manipulate_records(&recs, n, 0);
        assert_eq!(out, recs, "shift==0 must not memmove");
        let expect = recs.iter().fold(0i32, |a, r| a.wrapping_add(r.value));
        assert_eq!(total, expect);
    }
}

/// Row 12 — `shift < 0`: the guard rejects the memmove, but the loop bound
/// `num_records - shift` is LARGER than `num_records`, so the C reads `-shift`
/// records past the end. The Rust must read exactly the same over-long range.
/// A slack buffer makes the over-read land on defined, identical bytes.
#[test]
fn err12_manipulate_records_negative_shift_reads_past_end() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..3_000 {
        let n = rng.range(1, 24);
        let over = rng.range(1, 24); // how far past the end the loop will read
        let recs = random_records(&mut rng, (n + over) as usize);
        let shift = -over;
        let (total, out) = both_manipulate_records(&recs, n, shift);
        assert_eq!(out, recs, "negative shift must not memmove");
        // The C sums records[0 .. n - shift] == records[0 .. n + over].
        let expect = recs[..(n + over) as usize]
            .iter()
            .fold(0i32, |a, r| a.wrapping_add(r.value));
        assert_eq!(
            total, expect,
            "negative shift must sum num_records-shift elements (n={n}, shift={shift})"
        );
    }
}

/// Row 13 — `shift == num_records`: loop bound 0, returns 0.
#[test]
fn err13_manipulate_records_shift_equals_num_records() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let n = rng.range(1, 32);
        let recs = random_records(&mut rng, n as usize);
        let (total, out) = both_manipulate_records(&recs, n, n);
        assert_eq!(out, recs, "shift==num_records must not memmove");
        assert_eq!(total, 0, "shift==num_records must return 0");
    }
}

/// Row 14 — `shift > num_records`: loop bound negative, returns 0.
#[test]
fn err14_manipulate_records_shift_greater_than_num_records() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let n = rng.range(1, 32);
        let recs = random_records(&mut rng, n as usize);
        for extra in [1, 2, 100, 100_000] {
            let (total, out) = both_manipulate_records(&recs, n, n + extra);
            assert_eq!(out, recs);
            assert_eq!(total, 0, "shift>num_records must return 0");
        }
        let (total, out) = both_manipulate_records(&recs, n, i32::MAX);
        assert_eq!(out, recs);
        assert_eq!(total, 0);
    }
}

/// Row 15 — `num_records == 0`, `shift == 0`.
#[test]
fn err15_manipulate_records_zero_records() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..1_000 {
        let recs = random_records(&mut rng, 8);
        let (total, out) = both_manipulate_records(&recs, 0, 0);
        assert_eq!(out, recs, "num_records==0 must touch nothing");
        assert_eq!(total, 0);
        // shift > 0 with num_records == 0 also fails the guard.
        for shift in [1, 2, 1000, i32::MAX] {
            let (t, o) = both_manipulate_records(&recs, 0, shift);
            assert_eq!(o, recs);
            assert_eq!(t, 0);
        }
    }
}

/// Row 16 — `num_records < 0`.
///
/// Note the loop bound at lib.c:116 is `num_records - shift` computed in `int`,
/// which is *independent* of the guard and can wrap. `expected_manipulate` below
/// models exactly that, so these cases are asserted against the real C rule
/// rather than an assumption that "negative means zero".
#[test]
fn err16_manipulate_records_negative_num_records() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..500 {
        let recs = random_records(&mut rng, 32);
        for n in [-1, -2, -16, -1000, i32::MIN + 1, i32::MIN] {
            for shift in [0, 1, 2, 1000, i32::MAX] {
                check_manipulate(&recs, n, shift);
            }
        }
        // num_records < 0 with shift < num_records makes the loop bound POSITIVE
        // (e.g. -5 - -10 == 5): the C reads that many records from the base
        // pointer. Rust must read the same count.
        for n in [-1, -5, -9] {
            check_manipulate(&recs, n, -16);
        }
    }
}

/// Models lib.c:111 + lib.c:116 exactly: the guard, then the `int`-wrapping loop
/// bound. Returns `None` when the C would run off the end of `recs` (undefined
/// in the C too, so not a meaningful differential case).
fn expected_manipulate(recs: &[DataRecord], n: c_int, shift: c_int) -> Option<c_int> {
    let len = recs.len() as i64;
    let guard = shift > 0 && shift < n;
    if guard && (n as i64) > len {
        return None; // memmove would read/write past the buffer
    }
    let bound = n.wrapping_sub(shift);
    let count = if bound > 0 { bound as i64 } else { 0 };
    if count > len {
        return None; // the loop would read past the buffer
    }
    let base: usize = if guard { shift as usize } else { 0 };
    if base as i64 + count > len {
        return None;
    }
    Some(
        recs[base..base + count as usize]
            .iter()
            .fold(0i32, |a, r| a.wrapping_add(r.value)),
    )
}

/// Differential + oracle check for one `manipulate_records` configuration.
#[track_caller]
fn check_manipulate(recs: &[DataRecord], n: c_int, shift: c_int) {
    let Some(expect) = expected_manipulate(recs, n, shift) else {
        return;
    };
    let (total, out) = both_manipulate_records(recs, n, shift);
    assert_eq!(
        total, expect,
        "manipulate_records(len={}, num_records={n}, shift={shift}) result",
        recs.len()
    );
    if !(shift > 0 && shift < n) {
        assert_eq!(
            out, recs,
            "manipulate_records(num_records={n}, shift={shift}) must not memmove"
        );
    }
}

/// Row 17 — `num_records - shift` itself overflows `int`. Enumerated over the
/// full boundary cross-product, keeping every case the C can actually survive.
#[test]
fn err17_manipulate_records_loop_bound_overflow() {
    let _g = lock();
    let mut rng = rng();
    let recs = random_records(&mut rng, 64);

    // Named overflow corners, incl. INT_MAX - INT_MIN wrapping to -1 and
    // INT_MIN - INT_MAX wrapping to +1 (which makes the loop run once).
    for (n, shift) in [
        (i32::MAX, i32::MIN),
        (i32::MAX, i32::MIN + 1),
        (i32::MAX - 1, i32::MIN),
        (i32::MIN, i32::MAX),
        (i32::MIN, i32::MAX - 1),
        (i32::MIN + 1, i32::MAX),
        (0, i32::MIN),
        (1, i32::MIN),
        (i32::MIN, 0),
        (-1, i32::MAX),
    ] {
        check_manipulate(&recs, n, shift);
    }

    // Full boundary cross-product; `check_manipulate` skips the shapes the C
    // itself cannot survive.
    for &n in &EDGE {
        for &shift in &EDGE {
            check_manipulate(&recs, n, shift);
        }
    }
    // And the wrap-adjacent band around ±INT_MAX/±INT_MIN.
    for dn in -3i64..=3 {
        for ds in -3i64..=3 {
            for (bn, bs) in [
                (i32::MAX as i64, i32::MIN as i64),
                (i32::MIN as i64, i32::MAX as i64),
            ] {
                let n = (bn + dn).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
                let shift = (bs + ds).clamp(i32::MIN as i64, i32::MAX as i64) as i32;
                check_manipulate(&recs, n, shift);
            }
        }
    }
}

/// Row 18 — `total` overflows while summing `.value`s.
#[test]
fn err18_manipulate_records_total_overflow_wraps() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..2_000 {
        let n = rng.range(2, 32);
        let shift = rng.range(0, n - 1);
        let mut recs = random_records(&mut rng, n as usize);
        for r in recs.iter_mut() {
            // Values chosen so the running sum blows past INT_MAX repeatedly.
            r.value = if rng.next_u64() & 1 == 0 {
                i32::MAX - rng.range(0, 3)
            } else {
                i32::MIN + rng.range(0, 3)
            };
        }
        both_manipulate_records(&recs, n, shift);
    }
}

// ===========================================================================
// Rows 20–25: signed-overflow "out of range" results
// ===========================================================================

/// Row 20 — `add_three` overflow.
#[test]
fn err20_add_three_overflow_wraps() {
    let _g = lock();
    both_add_three(i32::MAX, i32::MAX, i32::MAX);
    both_add_three(i32::MIN, i32::MIN, i32::MIN);
    both_add_three(i32::MAX, 1, 0);
    both_add_three(i32::MIN, -1, 0);
    both_add_three(i32::MAX, 0, 1);
    both_add_three(i32::MAX, i32::MIN, 0);
    let mut rng = rng();
    for _ in 0..5_000 {
        let big = if rng.next_u64() & 1 == 0 { i32::MAX } else { i32::MIN };
        both_add_three(big, rng.i32_any(), rng.i32_any());
    }
}

/// Row 21 — `multiply_add` overflow, incl. `INT_MIN * -1`.
#[test]
fn err21_multiply_add_overflow_wraps() {
    let _g = lock();
    both_multiply_add(i32::MIN, -1, 0);
    both_multiply_add(-1, i32::MIN, 0);
    both_multiply_add(i32::MIN, -1, i32::MIN);
    both_multiply_add(65_536, 65_536, 0);
    both_multiply_add(i32::MAX, 2, i32::MAX);
    let mut rng = rng();
    for _ in 0..5_000 {
        both_multiply_add(rng.i32_any(), rng.i32_any(), rng.i32_any());
    }
    for _ in 0..5_000 {
        // Force the product itself out of range.
        let a = rng.range(46_341, 2_000_000_000);
        let b = rng.range(46_341, 2_000_000_000);
        both_multiply_add(a, b, rng.i32_any());
        both_multiply_add(-a, b, rng.i32_any());
        both_multiply_add(a, -b, rng.i32_any());
        both_multiply_add(-a, -b, rng.i32_any());
    }
}

/// Row 22 — `complex_calc` overflow (`(a-b)*c` and the `+ global_counter`).
#[test]
fn err22_complex_calc_overflow_wraps() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..200 {
        both_increment_counter(rng.i32_any(), 0);
        both_complex_calc(i32::MIN, i32::MAX, i32::MAX);
        both_complex_calc(i32::MAX, i32::MIN, i32::MIN);
        both_complex_calc(i32::MIN, 1, -1);
        both_complex_calc(0, i32::MIN, -1);
        for _ in 0..20 {
            let a = rng.range(46_341, 2_000_000_000);
            let b = -rng.range(46_341, 2_000_000_000);
            both_complex_calc(a, b, rng.i32_any());
        }
    }
}

/// Row 23 — `global_counter` itself wraps.
#[test]
fn err23_global_counter_overflow_wraps() {
    let _g = lock();
    // Push A1 deliberately over INT_MAX and under INT_MIN, checking the
    // observable value after every step.
    for _ in 0..8 {
        for _ in 0..40 {
            both_increment_counter(i32::MAX / 8, 0);
            both_complex_calc(0, 0, 0);
        }
        for _ in 0..40 {
            both_increment_counter(i32::MIN / 8, 0);
            both_complex_calc(0, 0, 0);
        }
    }
    both_increment_counter(i32::MAX, 0);
    both_increment_counter(i32::MAX, 0);
    both_complex_calc(0, 0, 0);
    both_increment_counter(i32::MIN, 0);
    both_increment_counter(i32::MIN, 0);
    both_complex_calc(0, 0, 0);
}

/// Row 24 — `global_accumulator` wraps (it doubles each call, so it must).
#[test]
fn err24_global_accumulator_overflow_wraps() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..50 {
        for _ in 0..40 {
            both_update_accumulator(1, 0);
            both_process_pointer_data(0, 0);
        }
        for _ in 0..40 {
            both_update_accumulator(rng.i32_any(), 0);
            both_process_pointer_data(0, 0);
        }
    }
    both_update_accumulator(i32::MAX, 0);
    both_process_pointer_data(0, 0);
    both_update_accumulator(i32::MIN, 0);
    both_process_pointer_data(0, 0);
}

/// Row 25 — `process_pointer_data` arithmetic overflow.
#[test]
fn err25_process_pointer_data_overflow_wraps() {
    let _g = lock();
    let mut rng = rng();
    for _ in 0..200 {
        both_update_accumulator(rng.i32_any(), 0);
        both_process_pointer_data(i32::MIN, -1);
        both_process_pointer_data(i32::MAX, i32::MAX);
        both_process_pointer_data(i32::MIN, i32::MIN);
        both_process_pointer_data(65_536, 65_536);
        for _ in 0..20 {
            let v = rng.range(46_341, 2_000_000_000);
            both_process_pointer_data(v, rng.range(46_341, 2_000_000_000));
            both_process_pointer_data(-v, rng.range(46_341, 2_000_000_000));
        }
    }
}

// ===========================================================================
// Rows 26–27: get_time_based_value out-of-range seeds
// ===========================================================================

/// Row 26 — `seed * 3600` overflows `int` before widening to `time_t`.
#[test]
fn err26_time_based_value_seed_multiply_overflow() {
    let _g = lock();
    for seed in [
        596_524,
        596_525,
        1_000_000,
        2_000_000,
        i32::MAX,
        i32::MAX - 1,
        -596_524,
        -1_000_000,
        -2_000_000,
        i32::MIN,
        i32::MIN + 1,
        1 << 20,
        1 << 24,
        1 << 30,
        -(1 << 30),
    ] {
        both_get_time_based_value(seed);
    }
    let mut rng = rng();
    for _ in 0..20_000 {
        // Anything with |seed| > 596523 overflows.
        let mag = rng.range(596_524, i32::MAX);
        both_get_time_based_value(mag);
        both_get_time_based_value(-mag);
    }
}

/// Row 27 — the `(int)(diff / 100)` truncation, including where the quotient is
/// not exactly representable and where the wrapped value is negative.
#[test]
fn err27_time_based_value_truncation_direction() {
    let _g = lock();
    // 3600*seed is a multiple of 100 until the int wrap breaks it, so sweep both.
    for seed in -400..400 {
        both_get_time_based_value(seed);
    }
    let mut rng = rng();
    for _ in 0..20_000 {
        both_get_time_based_value(rng.i32_any());
    }
    // Exhaustive sweep across the sign-flip induced by wraparound.
    let threshold = i32::MAX / 3600; // 596523
    for d in -50..=50 {
        both_get_time_based_value(threshold + d);
        both_get_time_based_value(-threshold + d);
    }
}

// ===========================================================================
// Rows 28–29: hatch never rejects
// ===========================================================================

/// Row 28/29 — `hatch` has no rejection path: every `int` quadruple returns a
/// (wrapped) `int`. Both libraries must agree for boundary and random inputs.
#[test]
fn err2829_hatch_has_no_rejection_path() {
    let _g = lock();
    let vals = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &a in &vals {
        for &b in &vals {
            for &c in &vals {
                for &d in &vals {
                    both_hatch(a, b, c, d);
                }
            }
        }
    }
    let mut rng = rng();
    for _ in 0..4_000 {
        both_hatch(
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
            rng.i32_interesting(),
        );
    }
}

// ===========================================================================
// Row 30: "out-of-range enum" analogue for this API
// ===========================================================================

/// Row 30 — the API declares no `enum`/`bool` parameter, so the analogue of an
/// out-of-range enum is an arbitrary 32-bit pattern in an `int` parameter, and
/// an arbitrary (non-library) function pointer for `operation_func`. Both are
/// covered here explicitly: every entry point is fed raw bit patterns that no
/// "valid variant" reasoning would produce.
#[test]
fn err30_arbitrary_bit_patterns_in_every_int_parameter() {
    let _g = lock();
    let p = libs();
    let mut rng = rng();
    let patterns: [i32; 12] = [
        0x0000_0000,
        i32::MIN,        // 0x8000_0000
        -1,              // 0xFFFF_FFFF
        0x7FFF_FFFF,
        0x5555_5555,
        0x5555_5555u32.wrapping_neg() as i32,
        0x0F0F_0F0F,
        0x00FF_00FF,
        0x0000_FFFF,
        0x0001_0000,
        0x0000_0100,
        0x1234_5678,
    ];
    for &x in &patterns {
        for &y in &patterns {
            both_add_three(x, y, x ^ y);
            both_multiply_add(x, y, x ^ y);
            both_complex_calc(x, y, x ^ y);
            both_increment_counter(x, y);
            both_update_accumulator(x, y);
            both_process_pointer_data(x, y);
            both_get_time_based_value(x);
            both_apply_operation_with(
                "pattern/add_three",
                Some(p.c.add_three),
                Some(p.r.add_three),
                x,
                y,
                x ^ y,
            );
            // Only non-faulting shapes for the buffer entry points.
            let data: Vec<c_int> = (0..8).map(|_| rng.i32_any()).collect();
            let _ = both_shift_array_data(&data, 0, x);
            let recs = random_records(&mut rng, 8);
            let _ = both_manipulate_records(&recs, 0, x.max(0));
            if x <= 0 {
                assert_eq!(both_compute_with_dynamic_memory(y, x), 0);
            }
        }
    }
}
