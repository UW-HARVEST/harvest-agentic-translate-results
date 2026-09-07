//! Phase C — error / rejection-path differential tests, one test per
//! `ERRORS.md` row.
//!
//! This library has no error codes, so "same rejection" means: the same
//! silently-declined no-op, the same value returned for a degenerate/out-of-
//! range argument, the same two's-complement wrap, or the same fatal signal.
//! Rows that are undefined behaviour in C (NULL deref, allocation failure) are
//! run in a forked child and compared by termination status, so a crash in one
//! implementation but not the other is a detected divergence.

mod common;

use common::*;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// Rows 1-8: shift_array_data rejections
// ---------------------------------------------------------------------------

/// Asserts C and Rust agree, and (when `expect_noop`) that the buffer really is
/// untouched relative to the input.
fn shift_expect(p: &Pair, data: &[c_int], size: c_int, shift_by: c_int, expect_noop: bool) {
    let mut cb = data.to_vec();
    let mut rb = data.to_vec();
    unsafe { (p.c.shift_array_data)(cb.as_mut_ptr(), size, shift_by) };
    unsafe { (p.r.shift_array_data)(rb.as_mut_ptr(), size, shift_by) };
    let ctx = (size, shift_by);
    eq_bytes("shift_array_data", ctx, &cb, &rb);
    if expect_noop {
        eq_bytes("shift_array_data must be a no-op", ctx, data, &cb);
    }
}

#[test]
fn err01_shift_by_zero_is_noop() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 101);
    for _ in 0..200 {
        let size = rng.range(0, 64);
        let data: Vec<c_int> = (0..64).map(|_| rng.i32_mixed()).collect();
        shift_expect(p, &data, size, 0, true);
    }
}

#[test]
fn err02_shift_by_negative_is_noop() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 102);
    for _ in 0..200 {
        let size = rng.range(0, 64);
        let data: Vec<c_int> = (0..64).map(|_| rng.i32_mixed()).collect();
        for &shift_by in &[-1, -2, -64, -65, i32::MIN, i32::MIN + 1] {
            shift_expect(p, &data, size, shift_by, true);
        }
    }
}

#[test]
fn err03_shift_by_equals_size_is_noop() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 103);
    for _ in 0..200 {
        let size = rng.range(0, 64);
        let data: Vec<c_int> = (0..64).map(|_| rng.i32_mixed()).collect();
        shift_expect(p, &data, size, size, true);
    }
}

#[test]
fn err04_shift_by_greater_than_size_is_noop() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 104);
    for _ in 0..200 {
        let size = rng.range(0, 64);
        let data: Vec<c_int> = (0..64).map(|_| rng.i32_mixed()).collect();
        for &d in &[1, 2, 64, 1000] {
            shift_expect(p, &data, size, size.saturating_add(d), true);
        }
        shift_expect(p, &data, size, i32::MAX, true);
    }
}

#[test]
fn err05_size_nonpositive_is_noop() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 105);
    for _ in 0..200 {
        let data: Vec<c_int> = (0..64).map(|_| rng.i32_mixed()).collect();
        for &size in &[0, -1, -2, -64, i32::MIN, i32::MIN + 1] {
            for &shift_by in &[1, 2, 63, i32::MAX] {
                shift_expect(p, &data, size, shift_by, true);
            }
        }
    }
}

#[test]
fn err06_size_int_min_shift_one_is_noop() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 106);
    let data: Vec<c_int> = (0..64).map(|_| rng.i32_mixed()).collect();
    shift_expect(p, &data, i32::MIN, 1, true);
    shift_expect(p, &data, i32::MIN, i32::MAX, true);
    shift_expect(p, &data, i32::MIN + 1, 1, true);
}

#[test]
fn err07_null_arr_with_guard_false_returns_normally() {
    let (_g, p) = locked();
    // Guard is false, so `arr` is never dereferenced: both must return cleanly.
    for &(size, shift_by) in
        &[(0, 0), (0, 1), (0, -1), (1, 0), (1, 1), (1, 2), (-1, 5), (i32::MIN, 1), (10, 10)]
    {
        let c = outcome_of(|| unsafe { (p.c.shift_array_data)(null_int(), size, shift_by) });
        let r = outcome_of(|| unsafe { (p.r.shift_array_data)(null_int(), size, shift_by) });
        assert_eq!(c, r, "shift_array_data(NULL, {size}, {shift_by}) outcome diverged");
        assert_eq!(c, Outcome::Exited(0), "expected clean return for ({size}, {shift_by})");
    }
}

#[test]
fn err08_null_arr_with_guard_true_faults_identically() {
    let (_g, p) = locked();
    for &(size, shift_by) in &[(10, 3), (2, 1), (64, 63), (i32::MAX, 1)] {
        let c = outcome_of(|| unsafe { (p.c.shift_array_data)(null_int(), size, shift_by) });
        let r = outcome_of(|| unsafe { (p.r.shift_array_data)(null_int(), size, shift_by) });
        assert_eq!(c, r, "shift_array_data(NULL, {size}, {shift_by}) outcome diverged");
        assert!(
            matches!(c, Outcome::Signaled(_)),
            "expected a fatal signal for ({size}, {shift_by}), got {c:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 9-16: manipulate_records rejections
// ---------------------------------------------------------------------------

fn rand_records(rng: &mut Rng, n: usize) -> Vec<DataRecord> {
    (0..n)
        .map(|i| {
            let mut name = [0i8; 32];
            for b in name.iter_mut() {
                *b = rng.i8_any();
            }
            DataRecord { id: i as c_int, value: rng.i32_mixed(), timestamp: rng.i64_any(), name }
        })
        .collect()
}

/// Over-allocates the buffer to `slots` records so that even the out-of-range
/// reads the C performs stay inside one allocation and are therefore
/// deterministic and identical for both libraries.
fn records_expect(
    p: &Pair,
    base: &[DataRecord],
    num_records: c_int,
    shift: c_int,
    expect: Option<c_int>,
) -> c_int {
    let mut cb = base.to_vec();
    let mut rb = base.to_vec();
    let cr = unsafe { (p.c.manipulate_records)(cb.as_mut_ptr(), num_records, shift) };
    let rr = unsafe { (p.r.manipulate_records)(rb.as_mut_ptr(), num_records, shift) };
    let ctx = (num_records, shift);
    eq_i32("manipulate_records", ctx, cr, rr);
    eq_bytes("manipulate_records buffer", ctx, as_raw_bytes(&cb), as_raw_bytes(&rb));
    if let Some(e) = expect {
        assert_eq!(cr, e, "C returned {cr} for {ctx:?}, table says {e}");
    }
    cr
}

#[test]
fn err09_shift_equals_num_records_returns_zero() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 109);
    for _ in 0..200 {
        let n = rng.range(1, 16);
        let recs = rand_records(&mut rng, 16);
        records_expect(p, &recs, n, n, Some(0));
    }
}

#[test]
fn err10_shift_greater_than_num_records_returns_zero() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 110);
    for _ in 0..200 {
        let n = rng.range(0, 16);
        let recs = rand_records(&mut rng, 16);
        for &d in &[1, 2, 16, 1000] {
            records_expect(p, &recs, n, n + d, Some(0));
        }
        records_expect(p, &recs, n, i32::MAX, Some(0));
    }
}

#[test]
fn err11_negative_shift_reads_past_logical_end() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 111);
    // C skips the memmove (`shift > 0` false) but its loop bound is
    // `num_records - shift`, which for shift<0 is LARGER than num_records — so
    // it sums records past the logical end. Buffer is over-allocated to 32
    // records so the read stays inside the allocation.
    for _ in 0..200 {
        let recs = rand_records(&mut rng, 32);
        for &(n, shift) in &[(5, -2), (5, -1), (1, -4), (0, -8), (8, -8), (2, -30), (16, -16)] {
            // expected value derived from the C semantics: sum of the first
            // (n - shift) `value` fields, no memmove
            let limit = (n as i64 - shift as i64) as usize;
            let mut want: c_int = 0;
            for i in 0..limit {
                want = want.wrapping_add(recs[i].value);
            }
            let got = records_expect(p, &recs, n, shift, Some(want));
            assert_eq!(got, want);
            // and the buffer must be untouched (no memmove happened)
            let mut cb = recs.clone();
            unsafe { (p.c.manipulate_records)(cb.as_mut_ptr(), n, shift) };
            eq_bytes(
                "negative shift must not memmove",
                (n, shift),
                as_raw_bytes(&recs),
                as_raw_bytes(&cb),
            );
        }
    }
}

#[test]
fn err12_num_records_zero_returns_zero() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 112);
    for _ in 0..200 {
        let recs = rand_records(&mut rng, 16);
        records_expect(p, &recs, 0, 0, Some(0));
    }
}

#[test]
fn err13_num_records_negative_returns_zero() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 113);
    for _ in 0..200 {
        let recs = rand_records(&mut rng, 16);
        for &n in &[-1, -2, -16, i32::MIN, i32::MIN + 1] {
            records_expect(p, &recs, n, 0, Some(0));
        }
    }
}

#[test]
fn err14_num_records_minus_shift_overflows() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 114);
    let recs = rand_records(&mut rng, 16);
    // `num_records - shift` overflows int. gcc wraps it; the guard
    // `shift > 0 && shift < num_records` and the loop bound then disagree with
    // any "sane" reading, and the Rust must reproduce the wrapped bound exactly.
    // Expected values are derived from the C semantics with a wrapping subtract.
    for &(n, shift) in &[
        (1, i32::MIN),          // 1 - INT_MIN wraps to INT_MIN+1 (negative) -> 0
        (0, i32::MIN),          // 0 - INT_MIN wraps to INT_MIN     (negative) -> 0
        (5, i32::MIN),
        (i32::MAX, i32::MIN),   // wraps to -1 -> 0
        (i32::MIN, i32::MAX),   // wraps to +1 -> sums recs[0]
        (i32::MIN, 1),          // guard: 1 < INT_MIN false; bound INT_MIN-1 wraps to INT_MAX
    ] {
        let bound = n.wrapping_sub(shift);
        // The C also skips the memmove whenever `shift <= 0 || shift >= n`.
        let memmoved = shift > 0 && shift < n;
        assert!(!memmoved, "these cases must all skip the memmove");
        let mut want: c_int = 0;
        if bound > 0 {
            // Only bounds that stay inside the over-allocated buffer are safe
            // to actually run; INT_MAX would read off the end.
            if (bound as usize) <= recs.len() {
                for i in 0..bound as usize {
                    want = want.wrapping_add(recs[i].value);
                }
                let got = records_expect(p, &recs, n, shift, Some(want));
                assert_eq!(got, want);
            }
            // else: bound is astronomically large; the C would read out of
            // bounds indefinitely. Not runnable, and not a distinguishable
            // rejection — skip (documented in ERRORS.md row 14).
        } else {
            records_expect(p, &recs, n, shift, Some(0));
        }
    }
}

#[test]
fn err15_null_records_guard_false_returns_zero() {
    let (_g, p) = locked();
    for &(n, shift) in &[(0, 0), (0, 1), (1, 1), (1, 2), (-1, 0), (5, 5), (5, 9)] {
        let c = outcome_of(|| {
            let v = unsafe { (p.c.manipulate_records)(null_rec(), n, shift) };
            assert_eq!(v, 0);
        });
        let r = outcome_of(|| {
            let v = unsafe { (p.r.manipulate_records)(null_rec(), n, shift) };
            assert_eq!(v, 0);
        });
        assert_eq!(c, r, "manipulate_records(NULL, {n}, {shift}) outcome diverged");
        assert_eq!(c, Outcome::Exited(0), "expected clean 0 for ({n}, {shift})");
    }
}

#[test]
fn err16_null_records_guard_true_faults_identically() {
    let (_g, p) = locked();
    for &(n, shift) in &[(5, 2), (2, 1), (32, 31), (i32::MAX, 1)] {
        let c = outcome_of(|| {
            blackhole(unsafe { (p.c.manipulate_records)(null_rec(), n, shift) })
        });
        let r = outcome_of(|| {
            blackhole(unsafe { (p.r.manipulate_records)(null_rec(), n, shift) })
        });
        assert_eq!(c, r, "manipulate_records(NULL, {n}, {shift}) outcome diverged");
        assert!(matches!(c, Outcome::Signaled(_)), "expected fatal signal, got {c:?}");
    }
}

// ---------------------------------------------------------------------------
// Rows 17-20: compute_with_dynamic_memory rejections
// ---------------------------------------------------------------------------

#[test]
fn err17_count_zero_returns_zero() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 117);
    for _ in 0..200 {
        let base = rng.i32_mixed();
        let c = unsafe { (p.c.compute_with_dynamic_memory)(base, 0) };
        let r = unsafe { (p.r.compute_with_dynamic_memory)(base, 0) };
        eq_i32("compute_with_dynamic_memory(count=0)", base, c, r);
        assert_eq!(c, 0, "table says count==0 returns 0");
    }
}

#[test]
fn err18_count_negative_returns_zero() {
    let (_g, p) = locked();
    let mut rng = Rng::new(SEED ^ 118);
    for _ in 0..200 {
        let base = rng.i32_mixed();
        for &count in &[-1, -2, -8, -1000, -(1 << 20)] {
            let c = unsafe { (p.c.compute_with_dynamic_memory)(base, count) };
            let r = unsafe { (p.r.compute_with_dynamic_memory)(base, count) };
            eq_i32("compute_with_dynamic_memory(count<0)", (base, count), c, r);
            assert_eq!(c, 0, "table says count<0 returns 0 (malloc fails, loops skipped)");
        }
    }
}

#[test]
fn err19_count_int_min_returns_zero() {
    let (_g, p) = locked();
    for &base in &[0, 1, -1, i32::MIN, i32::MAX] {
        for &count in &[i32::MIN, i32::MIN + 1] {
            let c = unsafe { (p.c.compute_with_dynamic_memory)(base, count) };
            let r = unsafe { (p.r.compute_with_dynamic_memory)(base, count) };
            eq_i32("compute_with_dynamic_memory(count=INT_MIN)", (base, count), c, r);
            assert_eq!(c, 0);
        }
    }
}

#[test]
fn err20_allocation_failure_faults_identically() {
    let (_g, p) = locked();
    // Constrain the child's address space so `malloc` genuinely fails, then let
    // the unchecked fill loop write through the returned NULL. Both libraries
    // must die the same way.
    const AS_LIMIT: u64 = 512 * 1024 * 1024;
    const COUNT: c_int = 400_000_000; // 1.6 GB request

    let c = outcome_of_limited(AS_LIMIT, || {
        blackhole(unsafe { (p.c.compute_with_dynamic_memory)(7, COUNT) })
    });
    let r = outcome_of_limited(AS_LIMIT, || {
        blackhole(unsafe { (p.r.compute_with_dynamic_memory)(7, COUNT) })
    });
    assert_eq!(c, r, "compute_with_dynamic_memory allocation-failure outcome diverged");
    assert!(matches!(c, Outcome::Signaled(_)), "expected fatal signal, got {c:?}");

    // INT_MAX (8 GB request) under the same limit.
    let c = outcome_of_limited(AS_LIMIT, || {
        blackhole(unsafe { (p.c.compute_with_dynamic_memory)(-3, i32::MAX) })
    });
    let r = outcome_of_limited(AS_LIMIT, || {
        blackhole(unsafe { (p.r.compute_with_dynamic_memory)(-3, i32::MAX) })
    });
    assert_eq!(c, r, "compute_with_dynamic_memory(INT_MAX) outcome diverged");
    assert!(matches!(c, Outcome::Signaled(_)), "expected fatal signal, got {c:?}");
}

// ---------------------------------------------------------------------------
// Rows 21-22: NULL data pointer and NULL function pointer
// ---------------------------------------------------------------------------

#[test]
fn err21_process_pointer_data_null_faults_identically() {
    let (_g, p) = locked();
    for &m in &[0, 1, -1, i32::MIN, i32::MAX] {
        let c = outcome_of(|| blackhole(unsafe { (p.c.process_pointer_data)(null_int(), m) }));
        let r = outcome_of(|| blackhole(unsafe { (p.r.process_pointer_data)(null_int(), m) }));
        assert_eq!(c, r, "process_pointer_data(NULL, {m}) outcome diverged");
        assert!(matches!(c, Outcome::Signaled(_)), "expected fatal signal, got {c:?}");
    }
}

#[test]
fn err22_apply_operation_null_op_faults_identically() {
    let (_g, p) = locked();
    // A C `operation_func` accepts any pointer value, including one with no
    // valid target — the FFI analogue of an out-of-range enum. Both sides must
    // fault the same way rather than one of them, say, panicking or returning.
    for &(a, b, c) in &[(0, 0, 0), (1, 2, 3), (i32::MIN, i32::MAX, -1)] {
        let co = outcome_of(|| blackhole(unsafe { (p.c.apply_operation)(None, a, b, c) }));
        let ro = outcome_of(|| blackhole(unsafe { (p.r.apply_operation)(None, a, b, c) }));
        assert_eq!(co, ro, "apply_operation(NULL, {a}, {b}, {c}) outcome diverged");
        assert!(matches!(co, Outcome::Signaled(_)), "expected fatal signal, got {co:?}");
    }
}

// ---------------------------------------------------------------------------
// Rows 23-24: get_time_based_value int overflow
// ---------------------------------------------------------------------------

#[test]
fn err23_time_based_value_int_max_overflow() {
    let (_g, p) = locked();
    for &seed in &[i32::MAX, i32::MAX - 1, 596_524, 596_525, 1 << 20, 1 << 30] {
        eq_i32(
            "get_time_based_value(positive overflow)",
            seed,
            unsafe { (p.c.get_time_based_value)(seed) },
            unsafe { (p.r.get_time_based_value)(seed) },
        );
    }
}

#[test]
fn err24_time_based_value_int_min_overflow() {
    let (_g, p) = locked();
    for &seed in &[i32::MIN, i32::MIN + 1, -596_524, -596_525, -(1 << 20), -(1 << 30)] {
        eq_i32(
            "get_time_based_value(negative overflow)",
            seed,
            unsafe { (p.c.get_time_based_value)(seed) },
            unsafe { (p.r.get_time_based_value)(seed) },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 25-27: arithmetic overflow in the pure functions
// ---------------------------------------------------------------------------

#[test]
fn err25_add_three_overflow() {
    let (_g, p) = locked();
    for &(a, b, c) in &[
        (i32::MAX, 1, 0),
        (i32::MAX, 0, 1),
        (i32::MIN, -1, 0),
        (i32::MIN, 0, -1),
        (i32::MAX, i32::MAX, 0),
        (i32::MIN, i32::MIN, 0),
        (i32::MAX, i32::MIN, i32::MAX),
    ] {
        eq_i32("add_three overflow", (a, b, c), unsafe { (p.c.add_three)(a, b, c) }, unsafe {
            (p.r.add_three)(a, b, c)
        });
    }
}

#[test]
fn err26_multiply_add_overflow() {
    let (_g, p) = locked();
    for &(a, b, c) in &[
        (i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN),
        (i32::MIN, -1, 0),
        (-1, i32::MIN, 0),
        (65536, 65536, 0),
        (46341, 46341, 0),
        (i32::MAX, 2, 1),
    ] {
        eq_i32("multiply_add overflow", (a, b, c), unsafe { (p.c.multiply_add)(a, b, c) }, unsafe {
            (p.r.multiply_add)(a, b, c)
        });
    }
}

#[test]
fn err27_complex_calc_overflow() {
    let (_g, p) = fresh();
    for &(a, b, c) in &[
        (i32::MIN, i32::MAX, i32::MAX),
        (i32::MAX, i32::MIN, i32::MAX),
        (i32::MIN, 1, 1),
        (0, i32::MIN, 1),
        (0, i32::MIN, -1),
        (i32::MIN, i32::MIN, i32::MIN),
    ] {
        eq_i32("complex_calc overflow", (a, b, c), unsafe { (p.c.complex_calc)(a, b, c) }, unsafe {
            (p.r.complex_calc)(a, b, c)
        });
    }
    // and again with global_counter itself at INT_MAX so the final `+` wraps
    unsafe { (p.c.increment_counter)(i32::MAX, 0) };
    unsafe { (p.r.increment_counter)(i32::MAX, 0) };
    for &(a, b, c) in &[(0, 0, 0), (1, 0, 1), (i32::MIN, i32::MAX, i32::MAX)] {
        eq_i32(
            "complex_calc overflow w/ counter=INT_MAX",
            (a, b, c),
            unsafe { (p.c.complex_calc)(a, b, c) },
            unsafe { (p.r.complex_calc)(a, b, c) },
        );
    }
}

// ---------------------------------------------------------------------------
// Rows 28-29: overflow of the two file-scope statics
// ---------------------------------------------------------------------------

#[test]
fn err28_global_counter_overflow() {
    let (_g, p) = fresh();
    let steps = [i32::MAX, 1, 1, 1, i32::MAX, i32::MAX, i32::MIN, i32::MIN, -1];
    for (i, &v) in steps.iter().enumerate() {
        unsafe { (p.c.increment_counter)(v, 0) };
        unsafe { (p.r.increment_counter)(v, 0) };
        eq_i32(
            "global_counter after overflow",
            (i, v),
            Pair::counter(&p.c),
            Pair::counter(&p.r),
        );
    }
}

#[test]
fn err29_global_accumulator_overflow() {
    let (_g, p) = fresh();
    let steps = [1, i32::MAX, i32::MAX, i32::MIN, i32::MIN, 0, -1, i32::MAX];
    for (i, &v) in steps.iter().enumerate() {
        unsafe { (p.c.update_accumulator)(v, 0) };
        unsafe { (p.r.update_accumulator)(v, 0) };
        eq_i32(
            "global_accumulator after overflow",
            (i, v),
            Pair::accumulator(&p.c),
            Pair::accumulator(&p.r),
        );
    }
    // 32 doublings from 1 must wrap the static all the way to 0
    p.reset();
    unsafe { (p.c.update_accumulator)(1, 0) };
    unsafe { (p.r.update_accumulator)(1, 0) };
    for i in 0..40 {
        unsafe { (p.c.update_accumulator)(0, 0) };
        unsafe { (p.r.update_accumulator)(0, 0) };
        eq_i32("global_accumulator doubling", i, Pair::accumulator(&p.c), Pair::accumulator(&p.r));
    }
}

// ---------------------------------------------------------------------------
// Row 30: hatch at the integer boundaries, including resulting static state
// ---------------------------------------------------------------------------

#[test]
fn err30_hatch_boundary_params_and_resulting_state() {
    let (_g, p) = locked();
    const V: [i32; 6] = [0, 1, -1, i32::MIN, i32::MAX, i32::MIN + 1];
    for &a in &V {
        for &b in &V {
            for &c in &V {
                for &d in &V {
                    p.reset();
                    let cv = unsafe { (p.c.hatch)(a, b, c, d) };
                    let rv = unsafe { (p.r.hatch)(a, b, c, d) };
                    eq_i32("hatch(boundary)", (a, b, c, d), cv, rv);
                    // the statics hatch leaves behind must match too
                    eq_i32(
                        "global_counter after hatch",
                        (a, b, c, d),
                        Pair::counter(&p.c),
                        Pair::counter(&p.r),
                    );
                    eq_i32(
                        "global_accumulator after hatch",
                        (a, b, c, d),
                        Pair::accumulator(&p.c),
                        Pair::accumulator(&p.r),
                    );
                }
            }
        }
    }
}
