//! Phase C — error-path differential tests. One test per `ERRORS.md` row.
//!
//! Rows whose "expected C result" is a hardware trap (SIGFPE / SIGSEGV) cannot
//! be asserted in-process, so they are run in a child process and the C and
//! Rust children's termination signals are compared.

mod common;

use common::*;
use std::ffi::c_int;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

// ===========================================================================
// Out-of-process harness
// ===========================================================================

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Exited(i32),
    Signalled(i32),
    TimedOut,
}

fn run_child(spec: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let mut child = Command::new(exe)
        .args(["--exact", "child_harness", "--test-threads=1", "--nocapture"])
        .env("HARVEST_CHILD", spec)
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn child");

    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match child.try_wait().expect("try_wait") {
            Some(st) => {
                return match st.signal() {
                    Some(s) => Outcome::Signalled(s),
                    None => Outcome::Exited(st.code().unwrap_or(-1)),
                };
            }
            None => {
                if Instant::now() > deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Outcome::TimedOut;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
}

/// Runs the same scenario against the C `.so` and the Rust `.so`, each in its
/// own process, and asserts the two processes end identically.
#[track_caller]
fn same_trap(row: &str, scenario: &str) -> Outcome {
    let co = run_child(&format!("c:{scenario}"));
    let ro = run_child(&format!("r:{scenario}"));
    assert_eq!(
        co, ro,
        "[{row}] scenario `{scenario}`: C child ended {co:?} but Rust child ended {ro:?}"
    );
    co
}

/// The child side. Returns immediately (test passes trivially) unless
/// `HARVEST_CHILD` is set by `run_child`.
#[test]
fn child_harness() {
    let spec = match std::env::var("HARVEST_CHILD") {
        Ok(s) => s,
        Err(_) => return,
    };
    let (which, scenario) = spec.split_once(':').expect("spec is `<lib>:<scenario>`");
    let lib: &Lib = match which {
        "c" => c(),
        "r" => r(),
        other => panic!("bad lib selector {other}"),
    };

    let null_arr: *mut ResultArray = std::ptr::null_mut();

    match scenario {
        // E2 — INT_MIN % -1 : idiv overflow
        "modulo_intmin_neg1" => {
            let v = (lib.modulo_operation)(i32::MIN, -1, 0, 0);
            println!("survived: {v}");
        }
        // E25 — null `arr` at each entry point
        "null_arr_cmp" => {
            let v = (lib.compare_results_in_array)(null_arr, 0, 1);
            println!("survived: {v}");
        }
        "null_arr_init" => {
            let mut vals = [1, 2, 3, 4];
            (lib.init_result_array)(null_arr, vals.as_mut_ptr(), 4);
            println!("survived");
        }
        "null_arr_foreach" => {
            let v = (lib.process_with_foreach)(null_arr, lib.add_operation);
            println!("survived: {v}");
        }
        "null_arr_weighted" => {
            let v = (lib.compute_weighted_sum)(null_arr);
            println!("survived: {v}");
        }
        // E26 — null `values` with count > 0
        "null_values" => {
            let mut arr = ResultArray::poisoned();
            (lib.init_result_array)(&mut arr, std::ptr::null_mut(), 4);
            println!("survived: count={}", arr.count);
        }
        // E27 — null `op`
        "null_op" => {
            let mut vals = [1, 2, 3, 4];
            let mut arr = ResultArray::poisoned();
            (lib.init_result_array)(&mut arr, vals.as_mut_ptr(), 4);
            // Deliberately a NULL function pointer: the C signature accepts any
            // `operation_func`, and a C caller passing NULL is a real input. The
            // `invalid_value` lint is exactly what we are testing here.
            // `Option<fn(..)>` is null-pointer-optimised, so this produces the
            // NULL function pointer a C caller could pass, without tripping the
            // `transmute_null_to_fn` / `invalid_value` lints on a literal 0.
            let bad: OperationFunc =
                unsafe { std::mem::transmute::<Option<OperationFunc>, OperationFunc>(None) };
            let v = (lib.process_with_foreach)(&mut arr, bad);
            println!("survived: {v}");
        }
        // E18 — negative count drives the FOREACH `!=` guard off the end
        "neg_count_foreach" => {
            let mut vals = [1, 2, 3, 4];
            // Heap-allocated so the runaway write leaves the mapping quickly.
            let mut boxed = Box::new(ResultArray::poisoned());
            (lib.init_result_array)(&mut *boxed, vals.as_mut_ptr(), -1);
            assert_eq!(boxed.count, -1, "negative count was not stored");
            let v = (lib.process_with_foreach)(&mut *boxed, lib.add_operation);
            println!("survived: {v}");
        }
        other => panic!("unknown scenario {other}"),
    }
    // Reached only if the call did not trap.
    std::process::exit(0);
}

// ===========================================================================
// E1 — modulo_operation, b == 0
// ===========================================================================

#[test]
fn err_e1_modulo_zero_divisor() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    let mut cases: Vec<c_int> = boundary_i32().to_vec();
    cases.extend([2, -2, 7, -7, i32::MAX - 1, i32::MIN + 1]);
    for _ in 0..2000 {
        cases.push(rng.next_i32());
    }
    for a in cases {
        let cv = (c.modulo_operation)(a, 0, 0, 0);
        let rv = (r.modulo_operation)(a, 0, 0, 0);
        eq_int(&format!("E1 a={a}"), cv, rv);
        assert_eq!(cv, 0, "E1: C must return the 0 sentinel for b==0 (a={a})");
    }
}

// ===========================================================================
// E2 — INT_MIN % -1 traps identically (out of process)
// ===========================================================================

#[test]
fn err_e2_modulo_intmin_by_neg1_traps() {
    let out = same_trap("E2", "modulo_intmin_neg1");
    assert_eq!(
        out,
        Outcome::Signalled(8),
        "E2: expected both children to die with SIGFPE (8); got {out:?}"
    );
    // Sanity: every other divisor of INT_MIN is fine and agrees.
    let (c, r) = both();
    for b in [-2, 1, 2, -3, i32::MAX, i32::MIN] {
        eq_int(
            &format!("E2 ok-divisor b={b}"),
            (c.modulo_operation)(i32::MIN, b, 0, 0),
            (r.modulo_operation)(i32::MIN, b, 0, 0),
        );
    }
}

// ===========================================================================
// E3..E6 — safe_double_to_int clamps / NaN / one-step-inside
// ===========================================================================

#[test]
fn err_e3_sdti_upper_clamp() {
    let (c, r) = both();
    for d in [
        INT_MAX_D,
        2147483647.000_000_1,
        2147483647.5,
        2147483648.0,
        3e9,
        1e300,
        f64::MAX,
        f64::INFINITY,
    ] {
        let cv = (c.safe_double_to_int)(d);
        let rv = (r.safe_double_to_int)(d);
        eq_int(&format!("E3 d={d}"), cv, rv);
        assert_eq!(cv, i32::MAX, "E3: C must clamp to INT32_MAX at d={d}");
    }
}

#[test]
fn err_e4_sdti_lower_clamp() {
    let (c, r) = both();
    for d in [
        INT_MIN_D,
        -2147483648.5,
        -2147483649.0,
        -3e9,
        -1e300,
        f64::MIN,
        f64::NEG_INFINITY,
    ] {
        let cv = (c.safe_double_to_int)(d);
        let rv = (r.safe_double_to_int)(d);
        eq_int(&format!("E4 d={d}"), cv, rv);
        assert_eq!(cv, i32::MIN, "E4: C must clamp to INT32_MIN at d={d}");
    }
}

#[test]
fn err_e5_sdti_nan() {
    let (c, r) = both();
    let nans = [
        f64::NAN,
        -f64::NAN,
        f64::INFINITY * 0.0,
        f64::INFINITY - f64::INFINITY,
        f64::from_bits(0x7FF0_0000_0000_0001),
        f64::from_bits(0xFFF0_0000_0000_0001),
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF),
    ];
    for d in nans {
        assert!(d.is_nan());
        let cv = (c.safe_double_to_int)(d);
        let rv = (r.safe_double_to_int)(d);
        eq_int(&format!("E5 bits={:#018x}", d.to_bits()), cv, rv);
        assert_eq!(cv, 0, "E5: C must return 0 for NaN");
    }
}

#[test]
fn err_e6_sdti_one_step_inside() {
    let (c, r) = both();
    // One representable step inside each clamp, i.e. one step past the
    // documented "invalid" range in the valid direction.
    let cases: [(f64, c_int); 6] = [
        (nextafter_toward_zero(INT_MAX_D), 2147483646),
        (2147483646.0, 2147483646),
        (2147483646.9999995, 2147483646),
        (nextafter_toward_zero(INT_MIN_D), -2147483647),
        (-2147483647.0, -2147483647),
        (-2147483647.9999995, -2147483647),
    ];
    for (d, want) in cases {
        let cv = (c.safe_double_to_int)(d);
        let rv = (r.safe_double_to_int)(d);
        eq_int(&format!("E6 d={d} bits={:#018x}", d.to_bits()), cv, rv);
        assert_eq!(cv, want, "E6: unexpected C truncation for d={d}");
    }
    // And one step OUTSIDE, which must clamp.
    assert_eq!((c.safe_double_to_int)(INT_MAX_D), i32::MAX);
    assert_eq!((r.safe_double_to_int)(INT_MAX_D), i32::MAX);
    assert_eq!((c.safe_double_to_int)(INT_MIN_D), i32::MIN);
    assert_eq!((r.safe_double_to_int)(INT_MIN_D), i32::MIN);
}

fn nextafter_toward_zero(x: f64) -> f64 {
    let b = x.to_bits();
    f64::from_bits(b - 1) // |x| decreases for finite non-zero x
}

// ===========================================================================
// E7 — compute_scaled_value overflow / underflow / NaN
// ===========================================================================

#[test]
fn err_e7_csv_overflow_underflow_nan() {
    let (c, r) = both();
    let cases: [(c_int, f64, c_int); 9] = [
        (i32::MAX, 1e10, i32::MAX),
        (1, 1e300, i32::MAX),
        (i32::MAX, f64::INFINITY, i32::MAX),
        (i32::MIN, 1e10, i32::MIN),
        (1, -1e300, i32::MIN),
        (i32::MAX, f64::NEG_INFINITY, i32::MIN),
        (0, f64::INFINITY, 0),          // 0 * inf == NaN -> 0
        (0, f64::NEG_INFINITY, 0),      // ditto
        (7, f64::NAN, 0),
    ];
    for (base, s, want) in cases {
        let cv = (c.compute_scaled_value)(base, s);
        let rv = (r.compute_scaled_value)(base, s);
        eq_int(&format!("E7 base={base} s={s}"), cv, rv);
        assert_eq!(cv, want, "E7: unexpected C result for base={base} s={s}");
    }
}

// ===========================================================================
// E8..E13 — compare_results_in_array guards
// ===========================================================================

fn make_pair(count: c_int) -> (ResultArray, ResultArray) {
    let (c, r) = both();
    let mut vals: Vec<c_int> = (0..16).collect();
    let mut ca = ResultArray::poisoned();
    let mut ra = ResultArray::poisoned();
    (c.init_result_array)(&mut ca, vals.as_mut_ptr(), count.clamp(0, 10));
    (r.init_result_array)(&mut ra, vals.as_mut_ptr(), count.clamp(0, 10));
    ca.count = count;
    ra.count = count;
    (ca, ra)
}

#[test]
fn err_e8_cmp_idx1_out_of_range() {
    let (c, r) = both();
    for count in 1..=10i32 {
        let (mut ca, mut ra) = make_pair(count);
        for idx1 in [count, count + 1, count + 100, i32::MAX] {
            let cv = (c.compare_results_in_array)(&mut ca, idx1, 0);
            let rv = (r.compare_results_in_array)(&mut ra, idx1, 0);
            eq_int(&format!("E8 count={count} idx1={idx1}"), cv, rv);
            assert_eq!(cv, 0, "E8: C must reject idx1={idx1} with 0");
        }
    }
}

#[test]
fn err_e9_cmp_idx2_out_of_range() {
    let (c, r) = both();
    for count in 1..=10i32 {
        let (mut ca, mut ra) = make_pair(count);
        for idx2 in [count, count + 1, count + 100, i32::MAX] {
            let cv = (c.compare_results_in_array)(&mut ca, 0, idx2);
            let rv = (r.compare_results_in_array)(&mut ra, 0, idx2);
            eq_int(&format!("E9 count={count} idx2={idx2}"), cv, rv);
            assert_eq!(cv, 0, "E9: C must reject idx2={idx2} with 0");
        }
    }
}

#[test]
fn err_e10_cmp_count_zero() {
    let (c, r) = both();
    let (mut ca, mut ra) = make_pair(0);
    for i1 in 0..5i32 {
        for i2 in 0..5i32 {
            let cv = (c.compare_results_in_array)(&mut ca, i1, i2);
            let rv = (r.compare_results_in_array)(&mut ra, i1, i2);
            eq_int(&format!("E10 ({i1},{i2})"), cv, rv);
            assert_eq!(cv, 0, "E10: count==0 must always reject");
        }
    }
}

/// There is NO lower-bound check in the C, so negative indices slip past the
/// guard and an out-of-bounds address is formed (never dereferenced) and
/// compared. The Rust must reproduce the resulting ordering exactly.
#[test]
fn err_e11_cmp_negative_index_unchecked() {
    let (c, r) = both();
    for count in 1..=10i32 {
        let (mut ca, mut ra) = make_pair(count);
        for i1 in [-1i32, -2, -5, -100, -1000] {
            for i2 in [-1i32, -2, 0, 1, count - 1] {
                let cv = (c.compare_results_in_array)(&mut ca, i1, i2);
                let rv = (r.compare_results_in_array)(&mut ra, i1, i2);
                eq_int(&format!("E11 count={count} ({i1},{i2})"), cv, rv);
                // The guard only has an UPPER bound, so a negative i1 gets
                // through; but i2 must still be < count.
                let want = if i1 >= count || i2 >= count {
                    0
                } else if i1 < i2 {
                    -1
                } else if i1 > i2 {
                    1
                } else {
                    0
                };
                assert_eq!(cv, want, "E11: C ordering for ({i1},{i2}) count={count}");
            }
        }
    }
}

#[test]
fn err_e12_cmp_equal_index() {
    let (c, r) = both();
    for count in 1..=10i32 {
        let (mut ca, mut ra) = make_pair(count);
        for i in 0..count {
            let cv = (c.compare_results_in_array)(&mut ca, i, i);
            let rv = (r.compare_results_in_array)(&mut ra, i, i);
            eq_int(&format!("E12 count={count} i={i}"), cv, rv);
            assert_eq!(cv, 0, "E12: equal indices must return 0");
        }
    }
}

/// `count` larger than the real array: the guard passes and addresses far past
/// `data[10]` are compared. No dereference happens, so this is deterministic.
#[test]
fn err_e13_cmp_count_lies() {
    let (c, r) = both();
    for count in [11i32, 100, 100_000, i32::MAX] {
        let (mut ca, mut ra) = make_pair(count);
        for (i1, i2) in [(10, 11), (11, 10), (1000, 1000), (0, 1000), (1000, 0), (10, 10)] {
            let cv = (c.compare_results_in_array)(&mut ca, i1, i2);
            let rv = (r.compare_results_in_array)(&mut ra, i1, i2);
            eq_int(&format!("E13 count={count} ({i1},{i2})"), cv, rv);
            let want = if i1 >= count || i2 >= count {
                0
            } else if i1 < i2 {
                -1
            } else if i1 > i2 {
                1
            } else {
                0
            };
            assert_eq!(cv, want, "E13 ordering count={count} ({i1},{i2})");
        }
    }
}

// ===========================================================================
// E14..E16 — init_result_array lengths
// ===========================================================================

#[test]
fn err_e14_init_count_clamped() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for count in [11i32, 12, 100, 1000, i32::MAX - 1, i32::MAX] {
        for _ in 0..100 {
            let mut vals: Vec<c_int> = (0..12).map(|_| rng.next_i32_spicy()).collect();
            let mut ca = ResultArray::poisoned();
            let mut ra = ResultArray::poisoned();
            (c.init_result_array)(&mut ca, vals.as_mut_ptr(), count);
            (r.init_result_array)(&mut ra, vals.as_mut_ptr(), count);
            assert_eq!(ca.count, 10, "E14: C must clamp count to 10");
            eq_struct(&format!("E14 count={count}"), &ca, &ra);
        }
    }
}

/// `count == 0` never dereferences `values`, so even a null pointer is fine.
/// If the Rust translation eagerly read `values[0]`, this would segfault.
#[test]
fn err_e15_init_count_zero_null_values_ok() {
    let (c, r) = both();
    let mut ca = ResultArray::poisoned();
    let mut ra = ResultArray::poisoned();
    (c.init_result_array)(&mut ca, std::ptr::null_mut(), 0);
    (r.init_result_array)(&mut ra, std::ptr::null_mut(), 0);
    assert_eq!(ca.count, 0);
    eq_struct("E15 count=0 values=NULL", &ca, &ra);
}

/// A negative `count` passes `count < 10`, so it is stored verbatim.
#[test]
fn err_e16_init_negative_count_poisons() {
    let (c, r) = both();
    for count in [-1i32, -2, -10, -1000, i32::MIN] {
        let mut vals: Vec<c_int> = (0..12).collect();
        let mut ca = ResultArray::poisoned();
        let mut ra = ResultArray::poisoned();
        (c.init_result_array)(&mut ca, vals.as_mut_ptr(), count);
        (r.init_result_array)(&mut ra, vals.as_mut_ptr(), count);
        assert_eq!(ca.count, count, "E16: C stores the negative count verbatim");
        eq_struct(&format!("E16 count={count}"), &ca, &ra);
        // Also safe with a null `values`, since the loop body never runs.
        let mut cb = ResultArray::poisoned();
        let mut rb = ResultArray::poisoned();
        (c.init_result_array)(&mut cb, std::ptr::null_mut(), count);
        (r.init_result_array)(&mut rb, std::ptr::null_mut(), count);
        eq_struct(&format!("E16 count={count} values=NULL"), &cb, &rb);
    }
}

// ===========================================================================
// E17..E22 — process_with_foreach / compute_weighted_sum edge counts
// ===========================================================================

#[test]
fn err_e17_foreach_count_zero() {
    let (c, r) = both();
    for pick in [
        (|l: &Lib| l.add_operation) as fn(&Lib) -> OperationFunc,
        |l: &Lib| l.multiply_operation,
        |l: &Lib| l.subtract_operation,
        |l: &Lib| l.modulo_operation,
    ] {
        let mut ca = ResultArray::poisoned();
        let mut ra = ResultArray::poisoned();
        ca.count = 0;
        ra.count = 0;
        let before = ca.observable_bytes();
        let cv = (c.process_with_foreach)(&mut ca, pick(c));
        let rv = (r.process_with_foreach)(&mut ra, pick(r));
        eq_int("E17", cv, rv);
        assert_eq!(cv, 0, "E17: empty array must total 0");
        assert_eq!(ca.observable_bytes(), before, "E17: array must be untouched");
        eq_struct("E17 struct", &ca, &ra);
    }
}

/// E18 — negative count makes the FOREACH `count_iter != size` guard never
/// terminate. Undefined behaviour: both implementations run off the end of the
/// struct. Verified out-of-process: both children must end the same way.
#[test]
fn err_e18_foreach_negative_count_runs_away() {
    let out = same_trap("E18", "neg_count_foreach");
    assert_ne!(
        out,
        Outcome::Exited(0),
        "E18: neither child should complete normally; got {out:?}"
    );
    println!("E18: both C and Rust children ended identically: {out:?}");
}

#[test]
fn err_e19_weighted_count_zero() {
    let (c, r) = both();
    let mut ca = ResultArray::poisoned();
    let mut ra = ResultArray::poisoned();
    ca.count = 0;
    ra.count = 0;
    let cv = (c.compute_weighted_sum)(&mut ca);
    let rv = (r.compute_weighted_sum)(&mut ra);
    eq_int("E19", cv, rv);
    assert_eq!(cv, 0, "E19: empty array sums to 0");
}

/// Unlike the FOREACH loop (E18), `compute_weighted_sum` uses `i < count`, so a
/// negative count terminates immediately and returns 0.
#[test]
fn err_e20_weighted_negative_count() {
    let (c, r) = both();
    for count in [-1i32, -7, -1000, i32::MIN] {
        let mut ca = ResultArray::poisoned();
        let mut ra = ResultArray::poisoned();
        ca.count = count;
        ra.count = count;
        let cv = (c.compute_weighted_sum)(&mut ca);
        let rv = (r.compute_weighted_sum)(&mut ra);
        eq_int(&format!("E20 count={count}"), cv, rv);
        assert_eq!(cv, 0, "E20: negative count must return 0, not trap");
    }
}

/// Element 0 uses `weight = 1` (not 0), because `current > base` is false.
#[test]
fn err_e21_weighted_index0_weight_is_one() {
    let (c, r) = both();
    let mut vals = [1000, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let mut ca = ResultArray::poisoned();
    let mut ra = ResultArray::poisoned();
    (c.init_result_array)(&mut ca, vals.as_mut_ptr(), 1);
    (r.init_result_array)(&mut ra, vals.as_mut_ptr(), 1);
    let cv = (c.compute_weighted_sum)(&mut ca);
    let rv = (r.compute_weighted_sum)(&mut ra);
    eq_int("E21", cv, rv);
    // 1000 * 1 * 0.8 == 800, NOT 0 (which a weight of 0 would give).
    assert_eq!(cv, 800, "E21: element 0 must use weight 1");
}

#[test]
fn err_e22_weighted_saturates_then_wraps() {
    let (c, r) = both();
    for vals in [[i32::MAX; 10], [i32::MIN; 10]] {
        let mut v = vals;
        let mut ca = ResultArray::poisoned();
        let mut ra = ResultArray::poisoned();
        (c.init_result_array)(&mut ca, v.as_mut_ptr(), 10);
        (r.init_result_array)(&mut ra, v.as_mut_ptr(), 10);
        let cv = (c.compute_weighted_sum)(&mut ca);
        let rv = (r.compute_weighted_sum)(&mut ra);
        eq_int(&format!("E22 vals[0]={}", vals[0]), cv, rv);
    }
    // Also every count, to catch a wrap that only shows at a particular length.
    for count in 0..=10i32 {
        let mut v = [i32::MAX; 10];
        let mut ca = ResultArray::poisoned();
        let mut ra = ResultArray::poisoned();
        (c.init_result_array)(&mut ca, v.as_mut_ptr(), count);
        (r.init_result_array)(&mut ra, v.as_mut_ptr(), count);
        eq_int(
            &format!("E22 count={count}"),
            (c.compute_weighted_sum)(&mut ca),
            (r.compute_weighted_sum)(&mut ra),
        );
    }
}

// ===========================================================================
// E23..E24 — arrayfunc boundaries
// ===========================================================================

#[test]
fn err_e23_arrayfunc_intmin_params() {
    let (c, r) = both();
    for slot in 0..4 {
        for &v in &[i32::MIN, i32::MIN + 1, -1, 1, i32::MAX] {
            let mut p = [0i32; 4];
            p[slot] = v;
            eq_int(
                &format!("E23 slot={slot} v={v}"),
                (c.arrayfunc)(p[0], p[1], p[2], p[3]),
                (r.arrayfunc)(p[0], p[1], p[2], p[3]),
            );
        }
    }
    // param4 / 2 for every odd/even sign combination.
    for p4 in [-3i32, -2, -1, 0, 1, 2, 3, i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        eq_int(
            &format!("E23 p4={p4}"),
            (c.arrayfunc)(1, 2, 3, p4),
            (r.arrayfunc)(1, 2, 3, p4),
        );
    }
}

#[test]
fn err_e24_arrayfunc_overflow_in_values() {
    let (c, r) = both();
    let cases: [[c_int; 4]; 10] = [
        [i32::MAX, 1, 0, 0],
        [i32::MIN, -1, 0, 0],
        [1, i32::MIN, 1, 0],
        [1, i32::MAX, -1, 0],
        [0, 0, i32::MAX, 0],
        [0, 0, i32::MIN, 0],
        [0, 0, i32::MAX / 2 + 1, 0],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [i32::MAX, i32::MIN, i32::MIN, i32::MAX],
    ];
    for p in cases {
        eq_int(
            &format!("E24 {p:?}"),
            (c.arrayfunc)(p[0], p[1], p[2], p[3]),
            (r.arrayfunc)(p[0], p[1], p[2], p[3]),
        );
    }
}

// ===========================================================================
// E25..E27 — null pointers (out of process)
// ===========================================================================

#[test]
fn err_e25_null_arr_segv_all_entry_points() {
    for scenario in [
        "null_arr_cmp",
        "null_arr_init",
        "null_arr_foreach",
        "null_arr_weighted",
    ] {
        let out = same_trap("E25", scenario);
        assert_eq!(
            out,
            Outcome::Signalled(11),
            "E25 `{scenario}`: expected SIGSEGV (11) in both; got {out:?}"
        );
    }
}

#[test]
fn err_e26_null_values_segv() {
    let out = same_trap("E26", "null_values");
    assert_eq!(
        out,
        Outcome::Signalled(11),
        "E26: expected SIGSEGV (11) in both; got {out:?}"
    );
}

#[test]
fn err_e27_null_op_segv() {
    let out = same_trap("E27", "null_op");
    assert_eq!(
        out,
        Outcome::Signalled(11),
        "E27: expected SIGSEGV (11) in both; got {out:?}"
    );
}

// ===========================================================================
// E28..E30
// ===========================================================================

extern "C" fn cb_max(_: c_int, _: c_int, _: c_int, _: c_int) -> c_int {
    i32::MAX
}
extern "C" fn cb_min(_: c_int, _: c_int, _: c_int, _: c_int) -> c_int {
    i32::MIN
}
extern "C" fn cb_neg(a: c_int, _: c_int, _: c_int, _: c_int) -> c_int {
    a.wrapping_neg()
}

#[test]
fn err_e28_foreach_arbitrary_callback() {
    let (c, r) = both();
    let mut rng = Rng::seeded();
    for cb in [
        cb_max as OperationFunc,
        cb_min as OperationFunc,
        cb_neg as OperationFunc,
    ] {
        for count in 0..=10i32 {
            for _ in 0..50 {
                let mut vals: Vec<c_int> = (0..12).map(|_| rng.next_i32_spicy()).collect();
                let mut ca = ResultArray::poisoned();
                let mut ra = ResultArray::poisoned();
                (c.init_result_array)(&mut ca, vals.as_mut_ptr(), count);
                (r.init_result_array)(&mut ra, vals.as_mut_ptr(), count);
                let cv = (c.process_with_foreach)(&mut ca, cb);
                let rv = (r.process_with_foreach)(&mut ra, cb);
                eq_int(&format!("E28 count={count}"), cv, rv);
                eq_struct(&format!("E28 count={count} struct"), &ca, &ra);
            }
        }
    }
}

/// There is no caller-controlled enum in this API: `arrayfunc` indexes its own
/// `operations[]` with a literal `0..4`. The corresponding FFI hazard is the raw
/// `operation_func`, covered by E27 (null) and E28 (arbitrary). This test pins
/// the documented fact that exactly four selectors exist and that they are the
/// four exported operations, so the row is closed rather than skipped.
#[test]
fn err_e29_no_caller_controlled_enum() {
    let (c, r) = both();
    assert_eq!(c.operations().len(), 4);
    assert_eq!(r.operations().len(), 4);
    // Each selector index maps to the same behaviour in both libraries.
    let mut rng = Rng::seeded();
    for k in 0..4usize {
        for _ in 0..200 {
            let a = rng.next_i32_spicy();
            let b = (rng.below(10)) as c_int; // rank-like second arg
            eq_int(
                &format!("E29 selector={k} a={a} b={b}"),
                (c.operations()[k])(a, b, 0, 0),
                (r.operations()[k])(a, b, 0, 0),
            );
        }
    }
    // An out-of-range selector cannot be expressed: the array has 4 entries and
    // `arrayfunc` is the only caller. Assert `arrayfunc` really uses all four by
    // checking it against the manual composition (done in C31) for one input.
    eq_int("E29 arrayfunc", (c.arrayfunc)(3, 5, 7, 11), (r.arrayfunc)(3, 5, 7, 11));
}

#[test]
fn err_e30_modulo_sign_follows_dividend() {
    let (c, r) = both();
    let expect: [(c_int, c_int, c_int); 8] = [
        (7, 3, 1),
        (-7, 3, -1),
        (7, -3, 1),
        (-7, -3, -1),
        (1, i32::MIN, 1),
        (-1, i32::MIN, -1),
        (i32::MAX, -2, 1),
        (i32::MIN + 1, 2, -1),
    ];
    for (a, b, want) in expect {
        let cv = (c.modulo_operation)(a, b, 0, 0);
        let rv = (r.modulo_operation)(a, b, 0, 0);
        eq_int(&format!("E30 a={a} b={b}"), cv, rv);
        assert_eq!(cv, want, "E30: C `%` semantics changed for {a} % {b}");
    }
}
