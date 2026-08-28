//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md` (E1..E36). Each constructs the exact invalid
//! input/condition, calls BOTH libraries through their `.so` exports, and asserts
//! they return the SAME sentinel — and that the sentinel is the specific value
//! the C source dictates, not merely "both failed somehow".
//!
//! Rows E2, E24, E25, E34 and E35 kill the process by design (SIGFPE / SIGSEGV);
//! they are verified out-of-process in `crash_probes.rs`.

mod common;

use common::*;
use std::os::raw::c_int;
use std::ptr;

/// Asserts C == Rust == `expected`.
#[track_caller]
fn eq_sentinel(ctx: &str, c: c_int, rust: c_int, expected: c_int) {
    eq_int(ctx, c, rust);
    assert_eq!(
        c, expected,
        "{ctx}: both agreed on {c} but the C source dictates {expected}"
    );
}

// ===========================================================================
// E1 — modulo_operation: b == 0
// ===========================================================================

#[test]
fn e1_modulo_by_zero_returns_zero() {
    let l = libs();
    let mut rng = Rng::new(0xE1);
    let mut a_values: Vec<c_int> = vec![0, 1, -1, i32::MAX, i32::MIN, 7, -7, 12345, -12345];
    for _ in 0..5000 {
        a_values.push(rng.interesting_i32());
    }
    for a in a_values {
        for &(u1, u2) in &[(0, 0), (1, 1), (i32::MIN, i32::MAX)] {
            unsafe {
                eq_sentinel(
                    &format!("E1 modulo_operation({a}, 0)"),
                    (l.c.modulo_operation)(a, 0, u1, u2),
                    (l.rust.modulo_operation)(a, 0, u1, u2),
                    0,
                );
            }
        }
    }
}

// ===========================================================================
// E3..E7 — safe_double_to_int clamps and NaN
// ===========================================================================

#[test]
fn e3_safe_double_to_int_at_or_above_intmax_clamps() {
    let l = libs();
    let imax = i32::MAX as f64;
    let mut cases: Vec<f64> = vec![
        imax,
        imax + 1.0,
        imax + 0.5,
        imax * 2.0,
        imax * 1e10,
        2147483648.0,
        4294967296.0,
        1e300,
        f64::MAX,
        f64::INFINITY,
    ];
    let mut rng = Rng::new(0xE3);
    for _ in 0..20_000 {
        // Anything strictly above the boundary.
        let d = imax + (rng.next_u32() as f64) * 0.5 + 1.0;
        cases.push(d);
    }
    for d in cases {
        unsafe {
            eq_sentinel(
                &format!("E3 safe_double_to_int({d:?})"),
                (l.c.safe_double_to_int)(d),
                (l.rust.safe_double_to_int)(d),
                i32::MAX,
            );
        }
    }
}

#[test]
fn e4_safe_double_to_int_exactly_intmax_is_rejected() {
    // The check is `>=`, so the representable boundary value itself clamps.
    let l = libs();
    let d = i32::MAX as f64;
    assert_eq!(d, 2147483647.0);
    unsafe {
        eq_sentinel(
            "E4 safe_double_to_int(2147483647.0)",
            (l.c.safe_double_to_int)(d),
            (l.rust.safe_double_to_int)(d),
            i32::MAX,
        );
    }
    // One representable step below must NOT clamp.
    let below = f64::from_bits(d.to_bits() - 1);
    assert!(below < d);
    unsafe {
        let c = (l.c.safe_double_to_int)(below);
        let r = (l.rust.safe_double_to_int)(below);
        eq_int("E4 one ULP below INT32_MAX", c, r);
        assert_eq!(c, 2147483646, "one ULP below must truncate, not clamp");
    }
}

#[test]
fn e5_safe_double_to_int_at_or_below_intmin_clamps() {
    let l = libs();
    let imin = i32::MIN as f64;
    let mut cases: Vec<f64> = vec![
        imin,
        imin - 1.0,
        imin - 0.5,
        imin * 2.0,
        imin * 1e10,
        -2147483649.0,
        -4294967296.0,
        -1e300,
        f64::MIN,
        f64::NEG_INFINITY,
    ];
    let mut rng = Rng::new(0xE5);
    for _ in 0..20_000 {
        cases.push(imin - (rng.next_u32() as f64) * 0.5 - 1.0);
    }
    for d in cases {
        unsafe {
            eq_sentinel(
                &format!("E5 safe_double_to_int({d:?})"),
                (l.c.safe_double_to_int)(d),
                (l.rust.safe_double_to_int)(d),
                i32::MIN,
            );
        }
    }
}

#[test]
fn e6_safe_double_to_int_exactly_intmin_is_rejected() {
    // The check is `<=`, so the boundary value itself clamps.
    let l = libs();
    let d = i32::MIN as f64;
    assert_eq!(d, -2147483648.0);
    unsafe {
        eq_sentinel(
            "E6 safe_double_to_int(-2147483648.0)",
            (l.c.safe_double_to_int)(d),
            (l.rust.safe_double_to_int)(d),
            i32::MIN,
        );
    }
    // One representable step above (towards zero) must NOT clamp.
    let above = f64::from_bits(d.to_bits() - 1); // magnitude shrinks
    assert!(above > d);
    unsafe {
        let c = (l.c.safe_double_to_int)(above);
        let r = (l.rust.safe_double_to_int)(above);
        eq_int("E6 one ULP above INT32_MIN", c, r);
        assert_eq!(c, -2147483647, "one ULP above must truncate, not clamp");
    }
}

#[test]
fn e7_safe_double_to_int_nan_returns_zero() {
    let l = libs();
    let mut nans: Vec<f64> = vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000), // canonical quiet NaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative quiet NaN
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001), // negative signalling NaN
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF), // max payload
        f64::from_bits(0x7FF4_0000_DEAD_BEEF), // arbitrary payload
        0.0 / 0.0_f64,
        f64::INFINITY - f64::INFINITY,
        f64::INFINITY * 0.0,
    ];
    let mut rng = Rng::new(0xE7);
    for _ in 0..20_000 {
        // Random NaN payloads: exponent all ones, non-zero mantissa.
        let payload = (rng.next_u64() & 0x000F_FFFF_FFFF_FFFF) | 1;
        let sign = (rng.next_u64() & 1) << 63;
        nans.push(f64::from_bits(sign | 0x7FF0_0000_0000_0000 | payload));
    }
    for d in nans {
        assert!(d.is_nan(), "test bug: 0x{:016x} is not NaN", d.to_bits());
        unsafe {
            eq_sentinel(
                &format!("E7 safe_double_to_int(NaN bits=0x{:016x})", d.to_bits()),
                (l.c.safe_double_to_int)(d),
                (l.rust.safe_double_to_int)(d),
                0,
            );
        }
    }
}

// ===========================================================================
// E8..E10 — compute_scaled_value overflow / NaN / infinity
// ===========================================================================

#[test]
fn e8_compute_scaled_value_overflow_saturates() {
    let l = libs();
    let cases: &[(c_int, f64, c_int)] = &[
        (i32::MAX, 2.0, i32::MAX),
        (i32::MAX, 1.0, i32::MAX), // MAX * 1.0 == (double)INT32_MAX -> clamps
        (i32::MIN, 2.0, i32::MIN),
        (i32::MIN, 1.0, i32::MIN), // MIN * 1.0 == (double)INT32_MIN -> clamps
        (i32::MAX, 1e10, i32::MAX),
        (i32::MIN, 1e10, i32::MIN),
        (i32::MAX, -2.0, i32::MIN),
        (i32::MIN, -2.0, i32::MAX),
        (1, 1e300, i32::MAX),
        (-1, 1e300, i32::MIN),
        (2, f64::MAX, i32::MAX),
        (1000000, 1000000.0, i32::MAX),
        (-1000000, 1000000.0, i32::MIN),
    ];
    for &(base, scale, expected) in cases {
        unsafe {
            eq_sentinel(
                &format!("E8 compute_scaled_value({base}, {scale:?})"),
                (l.c.compute_scaled_value)(base, scale),
                (l.rust.compute_scaled_value)(base, scale),
                expected,
            );
        }
    }
}

#[test]
fn e9_compute_scaled_value_nan_returns_zero() {
    let l = libs();
    // NaN scale factor, and the 0 * INFINITY -> NaN corner.
    let mut cases: Vec<(c_int, f64)> = vec![
        (0, f64::INFINITY),
        (0, f64::NEG_INFINITY),
        (0, f64::NAN),
        (1, f64::NAN),
        (-1, f64::NAN),
        (i32::MAX, f64::NAN),
        (i32::MIN, f64::NAN),
        (0, f64::from_bits(0x7FF4_0000_0000_0001)),
    ];
    let mut rng = Rng::new(0xE9);
    for _ in 0..5000 {
        cases.push((rng.interesting_i32(), f64::NAN));
    }
    for (base, scale) in cases {
        unsafe {
            eq_sentinel(
                &format!("E9 compute_scaled_value({base}, {scale:?})"),
                (l.c.compute_scaled_value)(base, scale),
                (l.rust.compute_scaled_value)(base, scale),
                0,
            );
        }
    }
}

#[test]
fn e10_compute_scaled_value_infinite_scale() {
    let l = libs();
    for base in [1, 2, -1, -2, 7, -7, i32::MAX, i32::MIN, 1 << 30] {
        for (scale, sign) in [(f64::INFINITY, 1i32), (f64::NEG_INFINITY, -1i32)] {
            let expected = if (base > 0) == (sign > 0) {
                i32::MAX
            } else {
                i32::MIN
            };
            unsafe {
                eq_sentinel(
                    &format!("E10 compute_scaled_value({base}, {scale:?})"),
                    (l.c.compute_scaled_value)(base, scale),
                    (l.rust.compute_scaled_value)(base, scale),
                    expected,
                );
            }
        }
    }
}

// ===========================================================================
// E11..E18 — compare_results_in_array guards
// ===========================================================================

fn arr_with_count(seed: u64, count: c_int) -> (ResultArray, ResultArray) {
    let a = {
        let mut a = ResultArray::dirty(seed);
        a.count = count;
        a
    };
    (a, a)
}

fn compare_both(seed: u64, count: c_int, idx1: c_int, idx2: c_int) -> (c_int, c_int) {
    let l = libs();
    let (mut ac, mut ar) = arr_with_count(seed, count);
    unsafe {
        let c = (l.c.compare_results_in_array)(&mut ac, idx1, idx2);
        let r = (l.rust.compare_results_in_array)(&mut ar, idx1, idx2);
        eq_array(
            &format!("compare must not mutate (count={count} {idx1},{idx2})"),
            &ac,
            &ar,
        );
        (c, r)
    }
}

#[test]
fn e11_compare_idx1_at_or_past_count_returns_zero() {
    for count in 0..=10i32 {
        for idx1 in count..count + 6 {
            // idx2 kept valid (or 0 when count == 0).
            let idx2 = if count > 0 { count - 1 } else { 0 };
            let (c, r) = compare_both(0xE11, count, idx1, idx2);
            eq_sentinel(
                &format!("E11 compare(count={count}, idx1={idx1}, idx2={idx2})"),
                c,
                r,
                0,
            );
        }
    }
}

#[test]
fn e12_compare_idx2_at_or_past_count_returns_zero() {
    for count in 0..=10i32 {
        for idx2 in count..count + 6 {
            let idx1 = if count > 0 { count - 1 } else { 0 };
            let (c, r) = compare_both(0xE12, count, idx1, idx2);
            eq_sentinel(
                &format!("E12 compare(count={count}, idx1={idx1}, idx2={idx2})"),
                c,
                r,
                0,
            );
        }
    }
}

#[test]
fn e13_compare_both_indices_out_of_range_returns_zero() {
    for count in 0..=10i32 {
        for d1 in 0..4i32 {
            for d2 in 0..4i32 {
                let (idx1, idx2) = (count + d1, count + d2);
                let (c, r) = compare_both(0xE13, count, idx1, idx2);
                eq_sentinel(
                    &format!("E13 compare(count={count}, {idx1}, {idx2})"),
                    c,
                    r,
                    0,
                );
            }
        }
    }
}

#[test]
fn e14_compare_empty_array_always_returns_zero() {
    // count == 0: every non-negative index is >= count.
    for idx1 in 0..12i32 {
        for idx2 in 0..12i32 {
            let (c, r) = compare_both(0xE14, 0, idx1, idx2);
            eq_sentinel(&format!("E14 compare(count=0, {idx1}, {idx2})"), c, r, 0);
        }
    }
}

#[test]
fn e15_compare_negative_indices_are_accepted_not_rejected() {
    // The C guard only checks the UPPER bound, so negative indices fall through
    // to address arithmetic and produce -1 / 0 / 1 rather than the 0 sentinel.
    for count in 1..=10i32 {
        for idx1 in -6..count {
            for idx2 in -6..count {
                let (c, r) = compare_both(0xE15, count, idx1, idx2);
                let expected = if idx1 < idx2 {
                    -1
                } else if idx1 > idx2 {
                    1
                } else {
                    0
                };
                eq_sentinel(
                    &format!("E15 compare(count={count}, {idx1}, {idx2})"),
                    c,
                    r,
                    expected,
                );
            }
        }
    }
    // Explicitly: a negative index paired with a valid one is NOT the 0 guard.
    let (c, r) = compare_both(0xE15, 5, -1, 0);
    eq_sentinel("E15 compare(count=5, -1, 0) must be -1 not 0", c, r, -1);
    let (c, r) = compare_both(0xE15, 5, 0, -1);
    eq_sentinel("E15 compare(count=5, 0, -1) must be 1 not 0", c, r, 1);
}

#[test]
fn e16_compare_negative_count_rejects_all_nonnegative_indices() {
    for count in [-1i32, -2, -10, i32::MIN, i32::MIN + 1] {
        for idx1 in 0..8i32 {
            for idx2 in 0..8i32 {
                let (c, r) = compare_both(0xE16, count, idx1, idx2);
                eq_sentinel(
                    &format!("E16 compare(count={count}, {idx1}, {idx2})"),
                    c,
                    r,
                    0,
                );
            }
        }
    }
}

#[test]
fn e17_compare_equal_indices_returns_zero() {
    for count in 1..=10i32 {
        for idx in -4..count {
            let (c, r) = compare_both(0xE17, count, idx, idx);
            eq_sentinel(&format!("E17 compare(count={count}, {idx}, {idx})"), c, r, 0);
        }
    }
}

#[test]
fn e18_compare_count_beyond_capacity_has_no_bounds_check() {
    // `count` hand-set past the 10-element `data[]`: the C performs no bounds
    // check on `data`, so indices 10..count are "in range" for the guard and the
    // address comparison proceeds.
    for count in 11..=20i32 {
        for idx1 in 8..count.min(18) {
            for idx2 in 8..count.min(18) {
                let (c, r) = compare_both(0xE18, count, idx1, idx2);
                let expected = if idx1 < idx2 {
                    -1
                } else if idx1 > idx2 {
                    1
                } else {
                    0
                };
                eq_sentinel(
                    &format!("E18 compare(count={count}, {idx1}, {idx2})"),
                    c,
                    r,
                    expected,
                );
            }
        }
    }
    // And `count == i32::MAX` accepts every index.
    let (c, r) = compare_both(0xE18, i32::MAX, 9, 10);
    eq_sentinel("E18 compare(count=INT_MAX, 9, 10)", c, r, -1);
}

// ===========================================================================
// E19..E22 — init_result_array clamping / degenerate counts
// ===========================================================================

fn init_both(seed: u64, values: &[c_int], count: c_int) -> (ResultArray, ResultArray) {
    let l = libs();
    let mut ac = ResultArray::dirty(seed);
    let mut ar = ResultArray::dirty(seed);
    let mut vc = values.to_vec();
    let mut vr = values.to_vec();
    unsafe {
        (l.c.init_result_array)(&mut ac, vc.as_mut_ptr(), count);
        (l.rust.init_result_array)(&mut ar, vr.as_mut_ptr(), count);
    }
    assert_eq!(vc, vr, "init_result_array must not write to values[]");
    (ac, ar)
}

#[test]
fn e19_init_oversized_count_clamps_to_ten() {
    let mut rng = Rng::new(0xE19);
    for count in [11i32, 12, 20, 100, 1000, 65536, i32::MAX - 1, i32::MAX] {
        for _ in 0..200 {
            let vals: Vec<c_int> = (0..16).map(|_| rng.interesting_i32()).collect();
            let (ac, ar) = init_both(0xE19, &vals, count);
            eq_array(&format!("E19 init count={count}"), &ac, &ar);
            assert_eq!(ac.count, 10, "E19 count={count} must clamp to 10");
            assert_eq!(ar.count, 10, "E19 count={count} must clamp to 10");
            for k in 0..10 {
                assert_eq!(ac.data[k].value, vals[k], "E19 element {k}");
                assert_eq!(ac.data[k].rank, k as c_int, "E19 rank {k}");
            }
        }
    }
}

#[test]
fn e20_init_count_exactly_ten_is_the_boundary() {
    // `count < 10 ? count : 10` — at exactly 10 the false branch is taken, but
    // the value is the same, so all ten elements are written.
    let mut rng = Rng::new(0xE20);
    for _ in 0..500 {
        let vals: Vec<c_int> = (0..16).map(|_| rng.interesting_i32()).collect();
        let (ac, ar) = init_both(0xE20, &vals, 10);
        eq_array("E20 init count=10", &ac, &ar);
        assert_eq!(ac.count, 10);
        for k in 0..10 {
            assert_eq!(ac.data[k].value, vals[k]);
            assert_eq!(ac.data[k].scaled, vals[k] as f64 * 1.5);
            assert_eq!(ac.data[k].rank, k as c_int);
        }
    }
    // count == 9 takes the true branch and leaves element 9 untouched.
    let vals: Vec<c_int> = (0..16).map(|v| v * 3 - 5).collect();
    let (ac, ar) = init_both(0xE20, &vals, 9);
    eq_array("E20 init count=9", &ac, &ar);
    assert_eq!(ac.count, 9);
    let pristine = ResultArray::dirty(0xE20);
    assert_eq!(
        ac.data[9].value, pristine.data[9].value,
        "E20 element 9 must be untouched when count == 9"
    );
}

#[test]
fn e21_init_negative_count_writes_nothing_and_ignores_null_values() {
    let l = libs();
    for count in [-1i32, -2, -10, -1000, i32::MIN, i32::MIN + 1] {
        // (a) with a real values[] pointer
        let (ac, ar) = init_both(0xE21, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10], count);
        eq_array(&format!("E21 init count={count}"), &ac, &ar);
        assert_eq!(ac.count, count, "E21 negative count is stored verbatim");
        assert_eq!(ar.count, count, "E21 negative count is stored verbatim");
        let pristine = ResultArray::dirty(0xE21);
        for k in 0..10 {
            assert_eq!(
                ac.data[k].value, pristine.data[k].value,
                "E21 count={count}: element {k} must be untouched"
            );
        }

        // (b) values == NULL is safe, because the loop body never runs
        let mut bc = ResultArray::dirty(0xE21);
        let mut br = ResultArray::dirty(0xE21);
        unsafe {
            (l.c.init_result_array)(&mut bc, ptr::null_mut(), count);
            (l.rust.init_result_array)(&mut br, ptr::null_mut(), count);
        }
        eq_array(&format!("E21 init NULL values count={count}"), &bc, &br);
        assert_eq!(bc.count, count);
        assert_eq!(br.count, count);
    }
}

#[test]
fn e22_init_zero_count_writes_nothing_and_ignores_null_values() {
    let l = libs();
    let (ac, ar) = init_both(0xE22, &[9, 9, 9, 9], 0);
    eq_array("E22 init count=0", &ac, &ar);
    assert_eq!(ac.count, 0);
    let pristine = ResultArray::dirty(0xE22);
    for k in 0..10 {
        assert_eq!(ac.data[k].value, pristine.data[k].value);
    }

    let mut bc = ResultArray::dirty(0xE22);
    let mut br = ResultArray::dirty(0xE22);
    unsafe {
        (l.c.init_result_array)(&mut bc, ptr::null_mut(), 0);
        (l.rust.init_result_array)(&mut br, ptr::null_mut(), 0);
    }
    eq_array("E22 init NULL values count=0", &bc, &br);
    assert_eq!(bc.count, 0);
    assert_eq!(br.count, 0);
}

// ===========================================================================
// E23 — process_with_foreach: NULL op is harmless when count == 0
// ===========================================================================

#[test]
fn e23_process_with_null_op_and_zero_count_is_safe() {
    // The FOREACH body never runs, so the C never dereferences `op`. The Rust
    // must likewise not touch the pointer before the loop.
    let l = libs();
    for seed in 0..200u64 {
        let mut ac = ResultArray::dirty(0xE23 + seed);
        let mut ar = ResultArray::dirty(0xE23 + seed);
        ac.count = 0;
        ar.count = 0;
        unsafe {
            let c = (l.c.process_with_foreach)(&mut ac, None);
            let r = (l.rust.process_with_foreach)(&mut ar, None);
            eq_sentinel("E23 process(count=0, op=NULL)", c, r, 0);
        }
        eq_array("E23 process(count=0, op=NULL) struct untouched", &ac, &ar);
        let pristine = ResultArray::dirty(0xE23 + seed);
        for k in 0..10 {
            assert_eq!(ac.data[k].value, pristine.data[k].value);
            assert_eq!(ar.data[k].value, pristine.data[k].value);
        }
    }
}

#[test]
fn e23b_process_with_zero_count_returns_zero_for_every_op() {
    let l = libs();
    for op_index in 0..4 {
        let (name, cop) = l.c.ops()[op_index];
        let (_, rop) = l.rust.ops()[op_index];
        let mut ac = ResultArray::dirty(0xE23B);
        let mut ar = ResultArray::dirty(0xE23B);
        ac.count = 0;
        ar.count = 0;
        unsafe {
            let c = (l.c.process_with_foreach)(&mut ac, Some(cop));
            let r = (l.rust.process_with_foreach)(&mut ar, Some(rop));
            eq_sentinel(&format!("E23b process(count=0, op={name})"), c, r, 0);
        }
        eq_array(&format!("E23b op={name} struct untouched"), &ac, &ar);
    }
}

// ===========================================================================
// E26 — an op returning extreme values forces the `* 0.75` clamp
// ===========================================================================

unsafe extern "C" fn op_ret_max(_a: c_int, _b: c_int, _u1: c_int, _u2: c_int) -> c_int {
    i32::MAX
}
unsafe extern "C" fn op_ret_min(_a: c_int, _b: c_int, _u1: c_int, _u2: c_int) -> c_int {
    i32::MIN
}

#[test]
fn e26_process_with_extreme_op_results_saturates() {
    let l = libs();
    for (name, cb, expect_value) in [
        // INT32_MAX * 0.75 == 1610612735.25 -> truncates to 1610612735
        ("max", op_ret_max as OperationFunc, 1_610_612_735i32),
        // INT32_MIN * 0.75 == -1610612736.0 exactly
        ("min", op_ret_min as OperationFunc, -1_610_612_736i32),
    ] {
        for count in 1..=10i32 {
            let mut ac = ResultArray::dirty(0xE26);
            ac.count = count;
            for k in 0..10 {
                ac.data[k].rank = k as c_int;
            }
            let mut ar = ac;
            unsafe {
                let c = (l.c.process_with_foreach)(&mut ac, Some(cb));
                let r = (l.rust.process_with_foreach)(&mut ar, Some(cb));
                eq_int(&format!("E26 {name} count={count}"), c, r);
            }
            eq_array(&format!("E26 {name} count={count} struct"), &ac, &ar);
            for k in 0..count as usize {
                assert_eq!(
                    ac.data[k].value, expect_value,
                    "E26 {name}: element {k} clamped value"
                );
            }
        }
    }
}

// ===========================================================================
// E27, E28, E29 — compute_weighted_sum degenerate / fallback / saturating
// ===========================================================================

#[test]
fn e27_weighted_sum_nonpositive_count_returns_zero() {
    let l = libs();
    for count in [0i32, -1, -2, -100, i32::MIN, i32::MIN + 1] {
        for seed in 0..50u64 {
            let mut ac = ResultArray::dirty(0xE27 + seed);
            ac.count = count;
            let mut ar = ac;
            unsafe {
                let c = (l.c.compute_weighted_sum)(&mut ac);
                let r = (l.rust.compute_weighted_sum)(&mut ar);
                eq_sentinel(&format!("E27 weighted_sum(count={count})"), c, r, 0);
            }
            eq_array(&format!("E27 count={count} struct untouched"), &ac, &ar);
        }
    }
}

#[test]
fn e28_weighted_sum_index_zero_uses_weight_one_not_zero() {
    // For i == 0 the pointer difference is 0, but the C `?:` substitutes 1.
    // With count == 1 the result is therefore trunc(value * 1 * 0.8), never 0.
    let l = libs();
    for value in [
        1i32, 2, 5, 10, 100, -1, -2, -5, -10, -100, 1000, -1000, i32::MAX, i32::MIN,
    ] {
        let mut ac = ResultArray::dirty(0xE28);
        ac.count = 1;
        ac.data[0].value = value;
        let mut ar = ac;
        unsafe {
            let c = (l.c.compute_weighted_sum)(&mut ac);
            let r = (l.rust.compute_weighted_sum)(&mut ar);
            eq_int(&format!("E28 weighted_sum(count=1, value={value})"), c, r);
            let expected = (l.c.safe_double_to_int)(value as f64 * 1.0 * 0.8);
            assert_eq!(
                c, expected,
                "E28 value={value}: weight must be 1 (result {c}, expected {expected})"
            );
            // Prove the fallback is 1 and not 0: with weight 0 the product would
            // be exactly 0.0 for every input. Any |value| >= 2 gives
            // |value * 1 * 0.8| >= 1.6, which truncates to a non-zero int, so a
            // non-zero result is only possible with weight == 1.
            // (|value| == 1 truncates 0.8 to 0 and cannot distinguish the two.)
            if value.unsigned_abs() >= 2 {
                assert_ne!(c, 0, "E28 value={value}: weight 0 would have given 0");
            }
        }
    }
}

#[test]
fn e29_weighted_sum_saturating_terms() {
    let l = libs();
    // value * weight * 0.8 crosses INT32_MAX for larger i but not for i == 0.
    for value in [i32::MAX, i32::MIN, i32::MAX / 2, i32::MIN / 2, 1 << 30] {
        for count in 1..=10i32 {
            let mut ac = ResultArray::dirty(0xE29);
            ac.count = count;
            for k in 0..10 {
                ac.data[k].value = value;
                ac.data[k].rank = k as c_int;
            }
            let mut ar = ac;
            unsafe {
                let c = (l.c.compute_weighted_sum)(&mut ac);
                let r = (l.rust.compute_weighted_sum)(&mut ar);
                eq_int(&format!("E29 value={value} count={count}"), c, r);

                // Recompute the expected sum through the C's own primitives.
                let mut expected: c_int = 0;
                for i in 0..count {
                    let weight = if i > 0 { i } else { 1 };
                    let w = value as f64 * weight as f64 * 0.8;
                    expected = expected.wrapping_add((l.c.safe_double_to_int)(w));
                }
                assert_eq!(c, expected, "E29 value={value} count={count} model");
            }
            eq_array(&format!("E29 value={value} count={count} struct"), &ac, &ar);
        }
    }
}

// ===========================================================================
// E30..E33 — arrayfunc overflow paths
// ===========================================================================

#[test]
fn e30_arrayfunc_param3_times_two_overflow() {
    let l = libs();
    for p3 in [
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        1 << 30,
        (1 << 30) + 1,
        -(1 << 30),
        1_073_741_824,
        2_000_000_000,
        -2_000_000_000,
    ] {
        for &(p1, p2, p4) in &[(0, 0, 0), (1, 2, 4), (-5, 7, -9), (i32::MAX, i32::MIN, 3)] {
            unsafe {
                eq_int(
                    &format!("E30 arrayfunc({p1},{p2},{p3},{p4})"),
                    (l.c.arrayfunc)(p1, p2, p3, p4),
                    (l.rust.arrayfunc)(p1, p2, p3, p4),
                );
            }
        }
    }
}

#[test]
fn e31_arrayfunc_add_sub_overflow() {
    let l = libs();
    // p1 + p2 overflows, and p2 - p3 overflows.
    let cases: &[(c_int, c_int, c_int, c_int)] = &[
        (i32::MAX, i32::MAX, 0, 0),
        (i32::MAX, 1, 0, 0),
        (i32::MIN, -1, 0, 0),
        (i32::MIN, i32::MIN, 0, 0),
        (0, i32::MAX, i32::MIN, 0),
        (0, i32::MIN, i32::MAX, 0),
        (0, i32::MIN, 1, 0),
        (0, i32::MAX, -1, 0),
        (i32::MAX, i32::MAX, i32::MIN, i32::MIN),
        (i32::MIN, i32::MIN, i32::MAX, i32::MAX),
    ];
    for &(p1, p2, p3, p4) in cases {
        unsafe {
            eq_int(
                &format!("E31 arrayfunc({p1},{p2},{p3},{p4})"),
                (l.c.arrayfunc)(p1, p2, p3, p4),
                (l.rust.arrayfunc)(p1, p2, p3, p4),
            );
        }
    }
}

#[test]
fn e32_arrayfunc_param4_intmin_division() {
    let l = libs();
    // `param4 / 2` with param4 == INT_MIN: the divisor is the literal 2, so the
    // INT_MIN / -1 trap cannot occur; the result is -1073741824, then +1.
    for p4 in [i32::MIN, i32::MIN + 1, i32::MIN + 2, -1, 1, i32::MAX, i32::MAX - 1] {
        for &(p1, p2, p3) in &[(0, 0, 0), (1, 2, 3), (i32::MAX, i32::MIN, 7)] {
            unsafe {
                eq_int(
                    &format!("E32 arrayfunc({p1},{p2},{p3},{p4})"),
                    (l.c.arrayfunc)(p1, p2, p3, p4),
                    (l.rust.arrayfunc)(p1, p2, p3, p4),
                );
            }
        }
    }
    // Odd negative values: C truncates toward zero (-7/2 == -3, not -4).
    for p4 in -21..=21i32 {
        unsafe {
            eq_int(
                &format!("E32 trunc arrayfunc(0,0,0,{p4})"),
                (l.c.arrayfunc)(0, 0, 0, p4),
                (l.rust.arrayfunc)(0, 0, 0, p4),
            );
        }
    }
}

#[test]
fn e33_arrayfunc_accumulator_overflow_then_final_clamp() {
    let l = libs();
    let mut rng = Rng::new(0xE33);
    // Large magnitudes drive both wraparound in `result` and the final
    // `* 0.333` clamp.
    for _ in 0..120_000 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        unsafe {
            eq_int(
                &format!("E33 arrayfunc({},{},{},{})", p[0], p[1], p[2], p[3]),
                (l.c.arrayfunc)(p[0], p[1], p[2], p[3]),
                (l.rust.arrayfunc)(p[0], p[1], p[2], p[3]),
            );
        }
    }
    // Deliberately huge, to hit safe_double_to_int's clamp on the final scale.
    for &(a, b, c, d) in &[
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX, i32::MIN, i32::MAX),
    ] {
        unsafe {
            eq_int(
                &format!("E33 extreme arrayfunc({a},{b},{c},{d})"),
                (l.c.arrayfunc)(a, b, c, d),
                (l.rust.arrayfunc)(a, b, c, d),
            );
        }
    }
}

// ===========================================================================
// E36 — arbitrary function pointer (the "out-of-range enum" analogue)
// ===========================================================================

unsafe extern "C" fn op_weird(a: c_int, b: c_int, u1: c_int, u2: c_int) -> c_int {
    // Deliberately depends on all four parameters so any argument-passing
    // difference is visible.
    a.wrapping_mul(3)
        .wrapping_sub(b.wrapping_mul(7))
        .wrapping_add(u1.wrapping_mul(1_000_003))
        .wrapping_add(u2.wrapping_mul(999_983))
        .rotate_left(11)
}

#[test]
fn e36_arbitrary_function_pointer_is_called_verbatim() {
    let l = libs();
    let mut rng = Rng::new(0xE36);
    for count in 0..=10i32 {
        for iter in 0..3000u64 {
            let mut ac = ResultArray::dirty(0xE36 + iter);
            for k in 0..10 {
                ac.data[k].value = rng.interesting_i32();
                ac.data[k].rank = rng.range_i32(-3, 12);
            }
            ac.count = count;
            let mut ar = ac;
            unsafe {
                let c = (l.c.process_with_foreach)(&mut ac, Some(op_weird));
                let r = (l.rust.process_with_foreach)(&mut ar, Some(op_weird));
                eq_int(&format!("E36 count={count} iter={iter}"), c, r);
            }
            eq_array(&format!("E36 count={count} iter={iter} struct"), &ac, &ar);
        }
    }

    // Also confirm the library passes literal 0,0 for unused1/unused2: with
    // u1 == u2 == 0 the weird op reduces to a*3 - b*7 rotated.
    let mut ac = ResultArray::dirty(0xE36F);
    ac.count = 1;
    ac.data[0].value = 12345;
    ac.data[0].rank = 6;
    let mut ar = ac;
    unsafe {
        let c = (l.c.process_with_foreach)(&mut ac, Some(op_weird));
        let r = (l.rust.process_with_foreach)(&mut ar, Some(op_weird));
        eq_int("E36 unused-arg probe", c, r);
        let expected = 12345i32
            .wrapping_mul(3)
            .wrapping_sub(6i32.wrapping_mul(7))
            .rotate_left(11);
        assert_eq!(c, expected, "E36: unused1/unused2 must be passed as 0, 0");
    }
}

// ===========================================================================
// Generic FFI boundary coverage (beyond the table)
// ===========================================================================

#[test]
fn generic_extreme_index_values_do_not_diverge() {
    // Values one step past every documented range, plus the absolute extremes.
    let idxs = [
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -11,
        -10,
        -1,
        0,
        1,
        9,
        10,
        11,
        1000,
        i32::MAX - 1,
        i32::MAX,
    ];
    let counts = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        9,
        10,
        11,
        1000,
        i32::MAX - 1,
        i32::MAX,
    ];
    for &count in &counts {
        for &idx1 in &idxs {
            for &idx2 in &idxs {
                // Skip index magnitudes whose *address* arithmetic would run off
                // the address space; the C guard rejects them first only when
                // they are >= count.
                if count == i32::MAX && (idx1.unsigned_abs() > 1000 || idx2.unsigned_abs() > 1000) {
                    continue;
                }
                let (c, r) = compare_both(0xBEEF, count, idx1, idx2);
                eq_int(
                    &format!("generic compare(count={count}, {idx1}, {idx2})"),
                    c,
                    r,
                );
            }
        }
    }
}

#[test]
fn generic_init_with_extreme_counts() {
    let mut rng = Rng::new(0xBEEF2);
    for count in [
        i32::MIN,
        i32::MIN + 1,
        -1000,
        -1,
        0,
        1,
        9,
        10,
        11,
        1000,
        i32::MAX - 1,
        i32::MAX,
    ] {
        let vals: Vec<c_int> = (0..16).map(|_| rng.interesting_i32()).collect();
        let (ac, ar) = init_both(0xBEEF2, &vals, count);
        eq_array(&format!("generic init count={count}"), &ac, &ar);
    }
}

#[test]
fn generic_zero_and_oversized_lengths_agree() {
    // "length" here is `count`; zero and oversized are the two degenerate ends.
    let l = libs();
    let mut rng = Rng::new(0xBEEF3);
    for count in [0i32, 10, 11, i32::MAX] {
        let mut vals: Vec<c_int> = (0..16).map(|_| rng.interesting_i32()).collect();
        let mut ac = ResultArray::dirty(0xBEEF3);
        let mut ar = ResultArray::dirty(0xBEEF3);
        unsafe {
            (l.c.init_result_array)(&mut ac, vals.as_mut_ptr(), count);
            (l.rust.init_result_array)(&mut ar, vals.as_mut_ptr(), count);
            eq_array(&format!("generic len init count={count}"), &ac, &ar);

            for op_index in 0..4 {
                let (name, cop) = l.c.ops()[op_index];
                let (_, rop) = l.rust.ops()[op_index];
                let mut bc = ac;
                let mut br = ar;
                let c = (l.c.process_with_foreach)(&mut bc, Some(cop));
                let r = (l.rust.process_with_foreach)(&mut br, Some(rop));
                eq_int(&format!("generic len process {name} count={count}"), c, r);
                eq_array(
                    &format!("generic len process {name} count={count} struct"),
                    &bc,
                    &br,
                );

                let wc = (l.c.compute_weighted_sum)(&mut bc);
                let wr = (l.rust.compute_weighted_sum)(&mut br);
                eq_int(&format!("generic len weighted {name} count={count}"), wc, wr);
            }
        }
    }
}

// ===========================================================================
// E24 (deterministic half) — identical out-of-bounds marching
// ===========================================================================
//
// A negative `count` makes `FOREACH` run away until the process dies, and the
// exact signal is nondeterministic for both implementations (see
// `crash_probes::e24_negative_count_kills_both_processes`). The *substance* of
// the row — that both implementations walk memory past the 10-element `data[]`
// with identical address arithmetic and identical per-element writes — is
// deterministic and is verified here using a large **mapped** backing buffer and
// a big positive `count`, which reaches exactly the same marching code path.

/// A `ResultArray` view over a much larger buffer, so indices far past the
/// 10-element capacity stay within mapped memory.
struct BigBuf {
    buf: Vec<Result_>,
}

impl BigBuf {
    const ELEMS: usize = 5000;

    fn new(seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let buf = (0..Self::ELEMS)
            .map(|i| Result_ {
                value: rng.next_i32(),
                scaled: f64::from_bits(rng.next_u64()),
                rank: (i as c_int).wrapping_mul(3).wrapping_sub(4),
            })
            .collect();
        BigBuf { buf }
    }

    fn as_arr(&mut self) -> *mut ResultArray {
        self.buf.as_mut_ptr().cast::<ResultArray>()
    }

    /// Byte-for-byte snapshot of every defined byte of the whole buffer.
    fn snapshot(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.buf.len() * 16);
        for e in &self.buf {
            out.extend_from_slice(&e.value.to_le_bytes());
            out.extend_from_slice(&e.scaled.to_bits().to_le_bytes());
            out.extend_from_slice(&e.rank.to_le_bytes());
        }
        out
    }

    fn first_diff(&self, other: &BigBuf) -> Option<usize> {
        self.buf.iter().zip(&other.buf).position(|(a, b)| {
            a.value != b.value || a.scaled.to_bits() != b.scaled.to_bits() || a.rank != b.rank
        })
    }
}

#[test]
fn e24_deterministic_out_of_bounds_marching_process() {
    let l = libs();
    // `count` is the field at offset 240, i.e. buf[10].value — so the loop
    // overwrites its own count field as it marches. `size` is read once by the
    // FOREACH macro, so that does not change the iteration count. Both
    // implementations must reproduce this identically.
    for &count in &[11i32, 12, 16, 64, 100, 999, 4096] {
        for op_index in 0..4 {
            let (name, cop) = l.c.ops()[op_index];
            let (_, rop) = l.rust.ops()[op_index];
            let mut bc = BigBuf::new(0x24D0);
            let mut br = BigBuf::new(0x24D0);
            assert_eq!(bc.snapshot(), br.snapshot(), "E24d precondition");
            unsafe {
                let ac = bc.as_arr();
                let ar = br.as_arr();
                (*ac).count = count;
                (*ar).count = count;
                let rc = (l.c.process_with_foreach)(ac, Some(cop));
                let rr = (l.rust.process_with_foreach)(ar, Some(rop));
                eq_int(&format!("E24d process op={name} count={count}"), rc, rr);
            }
            if bc.snapshot() != br.snapshot() {
                panic!(
                    "E24d op={name} count={count}: buffers diverge at element {:?}",
                    bc.first_diff(&br)
                );
            }
        }
    }
}

#[test]
fn e24_deterministic_out_of_bounds_marching_weighted_sum() {
    // `compute_weighted_sum` derives `weight` from the pointer difference
    // `current - base`, which for indices past the 10-element capacity keeps
    // growing as `i`. This is exactly the path where a naive Rust
    // `offset_from` would be out of bounds of the `data` field.
    let l = libs();
    for &count in &[11i32, 12, 16, 64, 100, 999, 4096] {
        let mut bc = BigBuf::new(0x24E0);
        let mut br = BigBuf::new(0x24E0);
        unsafe {
            let ac = bc.as_arr();
            let ar = br.as_arr();
            (*ac).count = count;
            (*ar).count = count;
            let rc = (l.c.compute_weighted_sum)(ac);
            let rr = (l.rust.compute_weighted_sum)(ar);
            eq_int(&format!("E24d weighted_sum count={count}"), rc, rr);
        }
        if bc.snapshot() != br.snapshot() {
            panic!(
                "E24d weighted_sum count={count}: buffers diverge at element {:?}",
                bc.first_diff(&br)
            );
        }
    }
}

#[test]
fn e24_deterministic_out_of_bounds_marching_init_and_compare() {
    let l = libs();
    let mut rng = Rng::new(0x24F0);
    // init_result_array always clamps to 10, so it can never march out of
    // bounds; assert that explicitly for large counts on a big buffer.
    for &count in &[11i32, 100, 4096, i32::MAX] {
        let mut bc = BigBuf::new(0x24F0);
        let mut br = BigBuf::new(0x24F0);
        let mut vals: Vec<c_int> = (0..64).map(|_| rng.interesting_i32()).collect();
        unsafe {
            (l.c.init_result_array)(bc.as_arr(), vals.as_mut_ptr(), count);
            (l.rust.init_result_array)(br.as_arr(), vals.as_mut_ptr(), count);
        }
        if bc.snapshot() != br.snapshot() {
            panic!(
                "E24d init count={count}: buffers diverge at element {:?}",
                bc.first_diff(&br)
            );
        }
        // Only the first 10 elements (plus the count field at buf[10].value)
        // may have changed.
        let pristine = BigBuf::new(0x24F0);
        for k in 11..BigBuf::ELEMS {
            assert_eq!(
                bc.buf[k].value, pristine.buf[k].value,
                "init_result_array wrote past element 10 (element {k})"
            );
        }
    }

    // compare_results_in_array with far out-of-range (but guard-accepted)
    // indices: pure address arithmetic, no dereference.
    for &count in &[4096i32, i32::MAX] {
        for &(i1, i2) in &[(0, 4095), (4095, 0), (1000, 1000), (10, 11), (4095, 4094)] {
            let mut bc = BigBuf::new(0x24F1);
            let mut br = BigBuf::new(0x24F1);
            unsafe {
                let ac = bc.as_arr();
                let ar = br.as_arr();
                (*ac).count = count;
                (*ar).count = count;
                let rc = (l.c.compare_results_in_array)(ac, i1, i2);
                let rr = (l.rust.compare_results_in_array)(ar, i1, i2);
                let expected = if i1 < i2 {
                    -1
                } else if i1 > i2 {
                    1
                } else {
                    0
                };
                eq_sentinel(
                    &format!("E24d compare(count={count}, {i1}, {i2})"),
                    rc,
                    rr,
                    expected,
                );
            }
        }
    }
}
