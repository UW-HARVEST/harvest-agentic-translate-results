//! Phase C — one differential test per row of ERRORS.md.
//!
//! Rows whose C behaviour is a fatal signal (NULL pointers, UB) are checked by
//! re-executing this same test binary as a child process and comparing the exact
//! termination status of the C-side child against the Rust-side child.

mod common;

use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Child-process crash harness
// ---------------------------------------------------------------------------

/// Outcome of a child run: `(exit_code, terminating_signal)`.
type Outcome = (Option<i32>, Option<i32>);

fn run_child(test_name: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .args([
            "--exact",
            test_name,
            "--ignored",
            "--test-threads=1",
            "--nocapture",
        ])
        .env("PHASE_C_CHILD", "1")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .unwrap_or_else(|e| panic!("failed to re-exec self for `{test_name}`: {e}"));
    (status.code(), status.signal())
}

#[track_caller]
fn assert_same_crash(row: &str, c_child: &str, rust_child: &str) {
    let c_out = run_child(c_child);
    let r_out = run_child(rust_child);
    assert_eq!(
        c_out, r_out,
        "{row}: C child `{c_child}` terminated with (code, signal) = {c_out:?} but Rust child \
         `{rust_child}` terminated with {r_out:?}"
    );
    assert_eq!(
        c_out.1,
        Some(11),
        "{row}: expected both children to die with SIGSEGV (11), got {c_out:?}"
    );
}

// ---------------------------------------------------------------------------
// E1 — modulo_operation with b == 0
// ---------------------------------------------------------------------------

#[test]
fn phase_c_e1_modulo_by_zero() {
    let (c, r) = both();
    let mut rng = Rng::new(0xE1);
    let mut cases: Vec<i32> = vec![i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for _ in 0..2000 {
        cases.push(rng.next_i32());
    }
    for a in cases {
        let cv = unsafe { (c.modulo_operation)(a, 0, 0, 0) };
        let rv = unsafe { (r.modulo_operation)(a, 0, 0, 0) };
        eq_i32("E1 modulo b=0", a, cv, rv);
        assert_eq!(cv, 0, "E1: C must return the sentinel 0 for b == 0, got {cv}");
    }
}

// ---------------------------------------------------------------------------
// E2/E3/E4 — safe_double_to_int saturation and NaN sentinels
// ---------------------------------------------------------------------------

#[test]
fn phase_c_e2_saturate_high() {
    let (c, r) = both();
    let imax = i32::MAX as f64;
    for d in [
        imax,
        imax + 1.0,
        imax * 2.0,
        2147483648.0,
        1e300,
        f64::MAX,
        f64::INFINITY,
    ] {
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        eq_i32("E2 saturate high", d, cv, rv);
        assert_eq!(cv, i32::MAX, "E2: C must clamp {d} to INT32_MAX, got {cv}");
    }
}

#[test]
fn phase_c_e3_saturate_low() {
    let (c, r) = both();
    let imin = i32::MIN as f64;
    for d in [
        imin,
        imin - 1.0,
        imin * 2.0,
        -2147483649.0,
        -1e300,
        f64::MIN,
        f64::NEG_INFINITY,
    ] {
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        eq_i32("E3 saturate low", d, cv, rv);
        assert_eq!(cv, i32::MIN, "E3: C must clamp {d} to INT32_MIN, got {cv}");
    }
}

#[test]
fn phase_c_e4_nan_sentinel() {
    let (c, r) = both();
    let nans = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000),
        f64::from_bits(0xFFF8_0000_0000_0000),
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling
        f64::from_bits(0xFFF0_0000_0000_0001),
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF),
        f64::from_bits(0x7FF8_DEAD_BEEF_0001),
    ];
    for d in nans {
        assert!(d.is_nan());
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        eq_i32("E4 NaN", d.to_bits(), cv, rv);
        assert_eq!(cv, 0, "E4: C must return 0 for NaN, got {cv}");
    }
}

// ---------------------------------------------------------------------------
// E5/E6/E7 — compute_scaled_value delegating to the clamps
// ---------------------------------------------------------------------------

#[test]
fn phase_c_e5_scaled_overflow_high() {
    let (c, r) = both();
    for (b, s) in [
        (i32::MAX, 2.0f64),
        (i32::MAX, 1.0000001),
        (2, 1e300),
        (-2, -1e300),
        (1, f64::INFINITY),
        (-1, f64::NEG_INFINITY),
        (1 << 30, 4.0),
        (i32::MIN, -1.0),
    ] {
        let cv = unsafe { (c.compute_scaled_value)(b, s) };
        let rv = unsafe { (r.compute_scaled_value)(b, s) };
        eq_i32("E5 scaled high", (b, s), cv, rv);
        assert_eq!(cv, i32::MAX, "E5: expected INT32_MAX for ({b}, {s})");
    }
}

#[test]
fn phase_c_e6_scaled_overflow_low() {
    let (c, r) = both();
    for (b, s) in [
        (i32::MIN, 2.0f64),
        (i32::MAX, -2.0),
        (2, -1e300),
        (-2, 1e300),
        (1, f64::NEG_INFINITY),
        (-1, f64::INFINITY),
        (i32::MIN, 1.0),
        (i32::MIN, 1.0000001),
    ] {
        let cv = unsafe { (c.compute_scaled_value)(b, s) };
        let rv = unsafe { (r.compute_scaled_value)(b, s) };
        eq_i32("E6 scaled low", (b, s), cv, rv);
        assert_eq!(cv, i32::MIN, "E6: expected INT32_MIN for ({b}, {s})");
    }
}

#[test]
fn phase_c_e7_scaled_nan() {
    let (c, r) = both();
    for (b, s) in [
        (0i32, f64::NAN),
        (1, f64::NAN),
        (i32::MIN, f64::NAN),
        (0, f64::INFINITY),      // 0 * INF == NaN
        (0, f64::NEG_INFINITY),  // 0 * -INF == NaN
        (0, f64::from_bits(0x7FF8_1234_5678_9ABC)),
    ] {
        let cv = unsafe { (c.compute_scaled_value)(b, s) };
        let rv = unsafe { (r.compute_scaled_value)(b, s) };
        eq_i32("E7 scaled NaN", (b, s.to_bits()), cv, rv);
        assert_eq!(cv, 0, "E7: expected 0 for ({b}, {s})");
    }
}

// ---------------------------------------------------------------------------
// E8..E12 — compare_results_in_array guards
// ---------------------------------------------------------------------------

fn cmp_pair(count: i32, i1: i32, i2: i32) -> i32 {
    let (c, r) = both();
    let mut ca = CResultArray::poisoned(count);
    let mut ra = CResultArray::poisoned(count);
    let cv = unsafe { (c.compare_results_in_array)(&mut ca, i1, i2) };
    let rv = unsafe { (r.compare_results_in_array)(&mut ra, i1, i2) };
    eq_i32("compare", (count, i1, i2), cv, rv);
    eq_arrays("compare/no-mutation", (count, i1, i2), &ca, &ra);
    cv
}

#[test]
fn phase_c_e8_idx1_out_of_range() {
    for count in [1i32, 2, 5, 10] {
        for i1 in [count, count + 1, count + 1000, i32::MAX] {
            // idx2 deliberately valid, so only the idx1 arm can fire.
            assert_eq!(
                cmp_pair(count, i1, 0),
                0,
                "E8: idx1={i1} >= count={count} must return 0"
            );
        }
    }
}

#[test]
fn phase_c_e9_idx2_out_of_range() {
    for count in [1i32, 2, 5, 10] {
        for i2 in [count, count + 1, count + 1000, i32::MAX] {
            assert_eq!(
                cmp_pair(count, 0, i2),
                0,
                "E9: idx2={i2} >= count={count} must return 0"
            );
        }
    }
}

#[test]
fn phase_c_e10_negative_indices_unvalidated() {
    // C never validates negative indices; the pointer comparison decides.
    assert_eq!(cmp_pair(10, -3, -1), -1, "E10");
    assert_eq!(cmp_pair(10, -1, -3), 1, "E10");
    assert_eq!(cmp_pair(10, -3, -3), 0, "E10");
    assert_eq!(cmp_pair(10, -1, 0), -1, "E10");
    assert_eq!(cmp_pair(10, 0, -1), 1, "E10");
    // With count == 0 every non-negative index is rejected, but two negative
    // indices still fall through to the pointer compare.
    assert_eq!(cmp_pair(0, -5, -2), -1, "E10 count=0");
    assert_eq!(cmp_pair(0, -2, -5), 1, "E10 count=0");
    assert_eq!(cmp_pair(0, -2, -2), 0, "E10 count=0");
    assert_eq!(cmp_pair(0, -1, 0), 0, "E10 count=0 mixed (idx2 rejected)");
    // Extreme negatives. Note that `idx >= count` is still evaluated first, so
    // with count = -1 an index of -1 is *rejected* (-1 >= -1) and the pointer
    // compare is only reached when both indices are strictly below count.
    assert_eq!(cmp_pair(-1, -1, -5), 0, "E10 idx1=-1 rejected at count=-1");
    assert_eq!(cmp_pair(-1, -5, -1), 0, "E10 idx2=-1 rejected at count=-1");
    assert_eq!(cmp_pair(-1, -5, -3), -1, "E10 both below count=-1");
    assert_eq!(cmp_pair(-1, -3, -5), 1, "E10 both below count=-1");
    assert_eq!(cmp_pair(-1, -5, -5), 0, "E10 both below count=-1, equal");
    assert_eq!(cmp_pair(-1, i32::MIN, -2), -1, "E10 extreme");
    assert_eq!(cmp_pair(-1, -2, i32::MIN), 1, "E10 extreme");
    assert_eq!(cmp_pair(i32::MIN, i32::MIN, i32::MIN), 0, "E10 idx == count");
}

#[test]
fn phase_c_e11_count_non_positive() {
    for count in [0i32, -1, -1000, i32::MIN] {
        for i in [0i32, 1, 9, 10, i32::MAX] {
            assert_eq!(cmp_pair(count, i, i), 0, "E11 count={count} idx={i}");
            assert_eq!(cmp_pair(count, i, 0), 0, "E11 count={count} idx1={i}");
            assert_eq!(cmp_pair(count, 0, i), 0, "E11 count={count} idx2={i}");
        }
    }
}

#[test]
fn phase_c_e12_equal_indices() {
    for count in [1i32, 2, 5, 10] {
        for i in 0..count {
            assert_eq!(cmp_pair(count, i, i), 0, "E12 count={count} idx={i}");
        }
    }
}

// ---------------------------------------------------------------------------
// E13..E16 — init_result_array clamp / negative count / no-read cases
// ---------------------------------------------------------------------------

fn init_pair(values: &[i32], count: i32, start: impl Fn() -> CResultArray) -> CResultArray {
    let (c, r) = both();
    let mut ca = start();
    let mut ra = start();
    unsafe { (c.init_result_array)(&mut ca, values.as_ptr(), count) };
    unsafe { (r.init_result_array)(&mut ra, values.as_ptr(), count) };
    eq_arrays("init", (count,), &ca, &ra);
    ca
}

#[test]
fn phase_c_e13_count_clamped_to_ten() {
    let vals: Vec<i32> = (0..10).map(|i| 1000 + i).collect();
    for count in [10i32, 11, 12, 100, 1000, i32::MAX, i32::MAX - 1] {
        let out = init_pair(&vals, count, || CResultArray::poisoned(-9));
        assert_eq!(out.count, 10, "E13: count={count} must clamp to 10");
        for i in 0..10 {
            assert_eq!(out.data[i].value, 1000 + i as i32, "E13 element {i}");
            assert_eq!(out.data[i].rank, i as i32, "E13 rank {i}");
        }
    }
}

#[test]
fn phase_c_e14_negative_count_stored_verbatim() {
    let poison = CResultArray::poisoned(0);
    for count in [-1i32, -2, -1000, i32::MIN, i32::MIN + 1] {
        let out = init_pair(&[42; 10], count, || CResultArray::poisoned(0));
        assert_eq!(
            out.count, count,
            "E14: negative count must be stored verbatim"
        );
        for i in 0..10 {
            assert_eq!(
                out.data[i].value, poison.data[i].value,
                "E14: element {i} must be untouched"
            );
            assert_eq!(
                out.data[i].scaled.to_bits(),
                poison.data[i].scaled.to_bits(),
                "E14: element {i} scaled must be untouched"
            );
        }
    }
}

#[test]
fn phase_c_e15_count_zero_never_reads_values() {
    let out = init_pair(&[7; 10], 0, || CResultArray::poisoned(-3));
    assert_eq!(out.count, 0, "E15");
    // `values` must not be dereferenced at all: prove it with a NULL pointer.
    let (c, r) = both();
    let mut ca = CResultArray::poisoned(-3);
    let mut ra = CResultArray::poisoned(-3);
    unsafe { (c.init_result_array)(&mut ca, std::ptr::null(), 0) };
    unsafe { (r.init_result_array)(&mut ra, std::ptr::null(), 0) };
    eq_arrays("E15 null values, count=0", (), &ca, &ra);
    assert_eq!(ca.count, 0);
    // Negative counts likewise never read `values`.
    for count in [-1i32, i32::MIN] {
        let mut ca = CResultArray::poisoned(0);
        let mut ra = CResultArray::poisoned(0);
        unsafe { (c.init_result_array)(&mut ca, std::ptr::null(), count) };
        unsafe { (r.init_result_array)(&mut ra, std::ptr::null(), count) };
        eq_arrays("E15 null values, count<0", count, &ca, &ra);
        assert_eq!(ca.count, count);
    }
}

#[test]
fn phase_c_e16_extreme_values_no_clamp_in_scaled() {
    let vals = [
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
        0,
        -1,
        1,
        1 << 30,
        -(1 << 30),
        7,
    ];
    let out = init_pair(&vals, 10, CResultArray::zeroed);
    for i in 0..10 {
        assert_eq!(out.data[i].value, vals[i], "E16 value {i}");
        assert_eq!(
            out.data[i].scaled,
            vals[i] as f64 * 1.5,
            "E16 scaled {i} must be the exact double product (no int clamp)"
        );
    }
}

// ---------------------------------------------------------------------------
// E17, E19, E20 — process_with_foreach
// ---------------------------------------------------------------------------

#[test]
fn phase_c_e17_count_zero_never_calls_op() {
    let (c, r) = both();
    // A NULL `op` is safe precisely because the loop body never runs.
    for start in [CResultArray::zeroed(), CResultArray::poisoned(0)] {
        let mut ca = start;
        let mut ra = start;
        let cv = unsafe { (c.process_with_foreach)(&mut ca, None) };
        let rv = unsafe { (r.process_with_foreach)(&mut ra, None) };
        eq_i32("E17 count=0, op=NULL", (), cv, rv);
        assert_eq!(cv, 0, "E17: must return 0");
        eq_arrays("E17 count=0 untouched", (), &ca, &ra);
    }
}

#[test]
fn phase_c_e19_writeback_saturates() {
    unsafe extern "C" fn big_pos(_a: i32, _b: i32, _c: i32, _d: i32) -> i32 {
        i32::MAX
    }
    unsafe extern "C" fn big_neg(_a: i32, _b: i32, _c: i32, _d: i32) -> i32 {
        i32::MIN
    }
    let (c, r) = both();
    for (tag, cb) in [("max", big_pos as OpFn), ("min", big_neg as OpFn)] {
        let mut ca = CResultArray::from_values(&[1, 2, 3]);
        let mut ra = CResultArray::from_values(&[1, 2, 3]);
        let cv = unsafe { (c.process_with_foreach)(&mut ca, Some(cb)) };
        let rv = unsafe { (r.process_with_foreach)(&mut ra, Some(cb)) };
        eq_i32("E19 writeback", tag, cv, rv);
        eq_arrays("E19 writeback", tag, &ca, &ra);
        // INT_MAX * 0.75 = 1610612735.25 -> 1610612735 (no clamp),
        // INT_MIN * 0.75 = -1610612736.0 -> -1610612736 (no clamp);
        // the clamp itself is proved by E2/E3, this asserts the shared path.
        let expect = if tag == "max" { 1610612735 } else { -1610612736 };
        for i in 0..3 {
            assert_eq!(ca.data[i].value, expect, "E19 element {i} ({tag})");
        }
    }
}

/// Row E20 — `op == NULL` with a non-empty array: both must die with SIGSEGV.
#[test]
fn phase_c_e20_null_op_with_nonempty_array() {
    assert_same_crash("E20 null op", "child_c_null_op", "child_rust_null_op");
}

// ---------------------------------------------------------------------------
// E21..E23 — compute_weighted_sum
// ---------------------------------------------------------------------------

fn weighted_pair(arr: CResultArray) -> i32 {
    let (c, r) = both();
    let mut ca = arr;
    let mut ra = arr;
    let cv = unsafe { (c.compute_weighted_sum)(&mut ca) };
    let rv = unsafe { (r.compute_weighted_sum)(&mut ra) };
    eq_i32("weighted", ca.count, cv, rv);
    eq_arrays("weighted/no-mutation", ca.count, &ca, &ra);
    cv
}

#[test]
fn phase_c_e21_count_non_positive_returns_zero() {
    for count in [0i32, -1, -1000, i32::MIN] {
        assert_eq!(
            weighted_pair(CResultArray::poisoned(count)),
            0,
            "E21 count={count}"
        );
    }
}

#[test]
fn phase_c_e22_index_zero_weight_is_one() {
    // count == 1: the only term is index 0, whose weight is 1 (not 0), so the
    // result is safe_double_to_int(value * 1 * 0.8).
    for v in [1000i32, -1000, 5, -5, 1, -1, 2, -2] {
        let got = weighted_pair(CResultArray::from_values(&[v]));
        let want = (v as f64 * 0.8) as i32;
        assert_eq!(got, want, "E22: value={v} must use weight 1");
        // For |v| >= 2 the weight-1 result is non-zero, which proves the weight
        // really is 1 and not 0 (`v * 0 * 0.8` would always be 0).
        if v.abs() >= 2 {
            assert_ne!(got, 0, "E22: weight must not be 0 for index 0 (value={v})");
        }
    }
}

#[test]
fn phase_c_e23_per_term_clamp_then_wrapping_accumulate() {
    // Every term clamps to INT_MAX; the int accumulation then wraps.
    let got = weighted_pair(CResultArray::from_values(&[i32::MAX; 10]));
    let mut want: i32 = 0;
    for i in 0..10i32 {
        let w = if i > 0 { i } else { 1 };
        let t = i32::MAX as f64 * w as f64 * 0.8;
        let clamped = if t >= i32::MAX as f64 {
            i32::MAX
        } else if t <= i32::MIN as f64 {
            i32::MIN
        } else {
            t as i32
        };
        want = want.wrapping_add(clamped);
    }
    assert_eq!(got, want, "E23 INT_MAX x10");
    let got = weighted_pair(CResultArray::from_values(&[i32::MIN; 10]));
    let mut want: i32 = 0;
    for i in 0..10i32 {
        let w = if i > 0 { i } else { 1 };
        let t = i32::MIN as f64 * w as f64 * 0.8;
        let clamped = if t >= i32::MAX as f64 {
            i32::MAX
        } else if t <= i32::MIN as f64 {
            i32::MIN
        } else {
            t as i32
        };
        want = want.wrapping_add(clamped);
    }
    assert_eq!(got, want, "E23 INT_MIN x10");
}

// ---------------------------------------------------------------------------
// E24..E27 — arrayfunc
// ---------------------------------------------------------------------------

#[test]
fn phase_c_e24_compare_loop_bound() {
    // `arr.count` is always 8 inside arrayfunc, so the compare loop contributes
    // exactly -7 and never forms an invalid index. Equality across a wide sweep
    // (plus the pipeline replica in phase_b_pipeline) is what pins this down.
    let (c, r) = both();
    let mut rng = Rng::new(0xE24);
    for _ in 0..20_000 {
        let p = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        let cv = unsafe { (c.arrayfunc)(p.0, p.1, p.2, p.3) };
        let rv = unsafe { (r.arrayfunc)(p.0, p.1, p.2, p.3) };
        eq_i32("E24 arrayfunc", p, cv, rv);
    }
}

#[test]
fn phase_c_e25_param4_int_min_no_trap() {
    let (c, r) = both();
    for p4 in [i32::MIN, i32::MIN + 1, -1, 1, i32::MAX] {
        for &o in &[i32::MIN, -1, 0, 1, i32::MAX] {
            let cv = unsafe { (c.arrayfunc)(o, o, o, p4) };
            let rv = unsafe { (r.arrayfunc)(o, o, o, p4) };
            eq_i32("E25 arrayfunc param4", (o, p4), cv, rv);
        }
    }
}

#[test]
fn phase_c_e26_overflowing_derived_values() {
    let (c, r) = both();
    // Inputs chosen so that param1+param2, param2-param3 and param3*2 all
    // overflow int.
    let cases = [
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MAX, 1, i32::MIN, 1),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MIN, -1, i32::MAX, -1),
        (1 << 30, 1 << 30, 1 << 30, 1 << 30),
        (-(1 << 30), -(1 << 30), -(1 << 30), -(1 << 30)),
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
    ];
    for p in cases {
        let cv = unsafe { (c.arrayfunc)(p.0, p.1, p.2, p.3) };
        let rv = unsafe { (r.arrayfunc)(p.0, p.1, p.2, p.3) };
        eq_i32("E26 arrayfunc overflow", p, cv, rv);
    }
}

#[test]
fn phase_c_e27_final_scale_shared_path() {
    let (c, r) = both();
    // The final `safe_double_to_int(result * 0.333)` is exercised by every
    // arrayfunc call; assert the helper agrees on the full int32 domain scaled
    // by 0.333 (this is the exact domain arrayfunc can reach).
    let mut rng = Rng::new(0xE27);
    for _ in 0..50_000 {
        let n = rng.next_i32();
        let d = n as f64 * 0.333;
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        eq_i32("E27 final scale", (n, d), cv, rv);
    }
    for n in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        let d = n as f64 * 0.333;
        let cv = unsafe { (c.safe_double_to_int)(d) };
        let rv = unsafe { (r.safe_double_to_int)(d) };
        eq_i32("E27 final scale bound", (n, d), cv, rv);
    }
}

// ---------------------------------------------------------------------------
// E28/E29 — NULL pointer arguments (differential crash tests)
// ---------------------------------------------------------------------------

#[test]
fn phase_c_e28_null_arr_compare_results_in_array() {
    assert_same_crash(
        "E28 NULL arr / compare_results_in_array",
        "child_c_null_arr_compare",
        "child_rust_null_arr_compare",
    );
}

#[test]
fn phase_c_e28_null_arr_init_result_array() {
    assert_same_crash(
        "E28 NULL arr / init_result_array",
        "child_c_null_arr_init",
        "child_rust_null_arr_init",
    );
}

#[test]
fn phase_c_e28_null_arr_process_with_foreach() {
    assert_same_crash(
        "E28 NULL arr / process_with_foreach",
        "child_c_null_arr_foreach",
        "child_rust_null_arr_foreach",
    );
}

#[test]
fn phase_c_e28_null_arr_compute_weighted_sum() {
    assert_same_crash(
        "E28 NULL arr / compute_weighted_sum",
        "child_c_null_arr_weighted",
        "child_rust_null_arr_weighted",
    );
}

#[test]
fn phase_c_e29_null_values_with_positive_count() {
    assert_same_crash(
        "E29 NULL values / init_result_array",
        "child_c_null_values",
        "child_rust_null_values",
    );
}

// ---------------------------------------------------------------------------
// E18 — negative count into process_with_foreach (runaway FOREACH)
// ---------------------------------------------------------------------------

/// The C `FOREACH` macro terminates on `count_iter != size`, so a negative
/// `count` walks forward off the end of the object. There is no defined C result
/// to match; the only observable, comparable behaviour is *how* each side dies.
#[test]
fn phase_c_e18_negative_count_runaway() {
    let c_out = run_child("child_c_negative_count");
    let r_out = run_child("child_rust_negative_count");
    assert_eq!(
        c_out, r_out,
        "E18: C child died with (code, signal) = {c_out:?} but Rust child died with {r_out:?} \
         — both must reproduce the same runaway `!=` loop"
    );
    assert_eq!(
        c_out.1,
        Some(11),
        "E18: expected both to run off the object and hit SIGSEGV, got {c_out:?}"
    );
}

// ---------------------------------------------------------------------------
// E30 — INT_MIN % -1 (hardware trap in C)
// ---------------------------------------------------------------------------

/// `modulo_operation(INT_MIN, -1)` is UB in C. The compiled C library issues a
/// bare `idiv`, which raises `#DE` -> `SIGFPE`, so there is no value the Rust
/// translation could return that would "match". The Rust translation uses
/// `wrapping_rem` and returns `0`. This test pins down *both* documented
/// behaviours so the divergence can never change silently.
#[test]
fn phase_c_e30_modulo_int_min_by_neg_one() {
    let c_out = run_child("child_c_mod_trap");
    let r_out = run_child("child_rust_mod_trap");

    // Rust must not crash.
    assert_eq!(
        r_out,
        (Some(0), None),
        "E30: the Rust library must return normally for INT_MIN % -1, got {r_out:?}"
    );
    let r = rs();
    assert_eq!(
        unsafe { (r.modulo_operation)(i32::MIN, -1, 0, 0) },
        0,
        "E30: Rust returns 0 (wrapping_rem)"
    );

    // C either traps (SIGFPE) or — on a target where the compiler folded the
    // division — returns 0 as well. Both are accepted; anything else is news.
    match c_out {
        (_, Some(8)) => { /* SIGFPE: the documented x86-64 behaviour */ }
        (Some(0), None) => {
            let c = common::c();
            assert_eq!(
                unsafe { (c.modulo_operation)(i32::MIN, -1, 0, 0) },
                0,
                "E30: if C does not trap it must agree with Rust"
            );
        }
        other => panic!("E30: unexpected C outcome {other:?} (expected SIGFPE or a clean 0)"),
    }
}

// ---------------------------------------------------------------------------
// Child processes. Ignored by default; each is spawned by exactly one of the
// tests above and is expected to terminate abnormally.
// ---------------------------------------------------------------------------

fn null_arr() -> *mut CResultArray {
    std::ptr::null_mut()
}

macro_rules! child {
    ($name:ident, $api:expr, $body:expr) => {
        #[test]
        #[ignore = "spawned as a child process by a phase_c_* crash test"]
        fn $name() {
            let api: &'static Api = $api;
            let f: fn(&'static Api) = $body;
            f(api);
        }
    };
}

child!(child_c_null_arr_compare, common::c(), |a| {
    let v = unsafe { (a.compare_results_in_array)(null_arr(), 0, 1) };
    println!("unexpectedly survived: {v}");
});
child!(child_rust_null_arr_compare, common::rs(), |a| {
    let v = unsafe { (a.compare_results_in_array)(null_arr(), 0, 1) };
    println!("unexpectedly survived: {v}");
});

child!(child_c_null_arr_init, common::c(), |a| {
    let vals = [1i32; 10];
    unsafe { (a.init_result_array)(null_arr(), vals.as_ptr(), 3) };
    println!("unexpectedly survived");
});
child!(child_rust_null_arr_init, common::rs(), |a| {
    let vals = [1i32; 10];
    unsafe { (a.init_result_array)(null_arr(), vals.as_ptr(), 3) };
    println!("unexpectedly survived");
});

child!(child_c_null_arr_foreach, common::c(), |a| {
    let v = unsafe { (a.process_with_foreach)(null_arr(), Some(a.add_operation)) };
    println!("unexpectedly survived: {v}");
});
child!(child_rust_null_arr_foreach, common::rs(), |a| {
    let v = unsafe { (a.process_with_foreach)(null_arr(), Some(a.add_operation)) };
    println!("unexpectedly survived: {v}");
});

child!(child_c_null_arr_weighted, common::c(), |a| {
    let v = unsafe { (a.compute_weighted_sum)(null_arr()) };
    println!("unexpectedly survived: {v}");
});
child!(child_rust_null_arr_weighted, common::rs(), |a| {
    let v = unsafe { (a.compute_weighted_sum)(null_arr()) };
    println!("unexpectedly survived: {v}");
});

child!(child_c_null_values, common::c(), |a| {
    let mut arr = CResultArray::zeroed();
    unsafe { (a.init_result_array)(&mut arr, std::ptr::null(), 3) };
    println!("unexpectedly survived: {}", arr.count);
});
child!(child_rust_null_values, common::rs(), |a| {
    let mut arr = CResultArray::zeroed();
    unsafe { (a.init_result_array)(&mut arr, std::ptr::null(), 3) };
    println!("unexpectedly survived: {}", arr.count);
});

child!(child_c_null_op, common::c(), |a| {
    let mut arr = CResultArray::from_values(&[1, 2, 3]);
    let v = unsafe { (a.process_with_foreach)(&mut arr, None) };
    println!("unexpectedly survived: {v}");
});
child!(child_rust_null_op, common::rs(), |a| {
    let mut arr = CResultArray::from_values(&[1, 2, 3]);
    let v = unsafe { (a.process_with_foreach)(&mut arr, None) };
    println!("unexpectedly survived: {v}");
});

/// The array lives on the heap so the runaway `FOREACH` walks forward through
/// mapped-then-unmapped memory in both libraries alike.
fn heap_array(count: i32) -> Box<CResultArray> {
    let mut b = Box::new(CResultArray::zeroed());
    b.count = count;
    b
}

child!(child_c_negative_count, common::c(), |a| {
    let mut arr = heap_array(-1);
    let v = unsafe { (a.process_with_foreach)(&mut *arr, Some(a.add_operation)) };
    println!("unexpectedly survived: {v}");
});
child!(child_rust_negative_count, common::rs(), |a| {
    let mut arr = heap_array(-1);
    let v = unsafe { (a.process_with_foreach)(&mut *arr, Some(a.add_operation)) };
    println!("unexpectedly survived: {v}");
});

child!(child_c_mod_trap, common::c(), |a| {
    let v = unsafe { (a.modulo_operation)(i32::MIN, -1, 0, 0) };
    println!("returned {v}");
});
child!(child_rust_mod_trap, common::rs(), |a| {
    let v = unsafe { (a.modulo_operation)(i32::MIN, -1, 0, 0) };
    println!("returned {v}");
});
