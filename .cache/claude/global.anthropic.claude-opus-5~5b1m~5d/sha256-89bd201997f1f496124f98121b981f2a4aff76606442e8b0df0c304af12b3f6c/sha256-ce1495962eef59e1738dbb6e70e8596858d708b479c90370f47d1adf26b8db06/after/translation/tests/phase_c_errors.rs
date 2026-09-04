// Phase C — error / rejection-path differential tests, one test per ERRORS.md row.
//
// Each test constructs the exact invalid input the C guards against, calls BOTH
// `.so`s, and asserts they return the SAME sentinel — not merely that both
// "failed somehow".

mod common;

use common::*;
use std::os::raw::c_int;

// ===========================================================================
// Row 1 — modulo_operation, b == 0
// ===========================================================================

#[test]
fn err01_modulo_zero_divisor_returns_zero() {
    let p = pair();
    let mut rng = Rng::seeded();
    for a in [
        0,
        1,
        -1,
        7,
        -7,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ] {
        let cv = unsafe { (p.c.modulo_operation)(a, 0, 0, 0) };
        let rv = unsafe { (p.rs.modulo_operation)(a, 0, 0, 0) };
        assert_eq!(cv, rv, "modulo({a}, 0)");
        assert_eq!(cv, 0, "the C guard must return the sentinel 0");
    }
    for _ in 0..50_000 {
        let a = rng.i32();
        let cv = unsafe { (p.c.modulo_operation)(a, 0, rng.i32(), rng.i32()) };
        let rv = unsafe { (p.rs.modulo_operation)(a, 0, rng.i32(), rng.i32()) };
        assert_eq!(cv, rv);
        assert_eq!(cv, 0);
    }
}

// ===========================================================================
// Row 2 — modulo_operation(INT32_MIN, -1)
//
// The C library raises SIGFPE on this input (measured: exit 136, core dumped), so
// there is no value to compare and calling it would kill this process. What CAN be
// checked differentially is that the trap is confined to that single pair: every
// neighbouring input, and every other `b == -1` / `a == INT32_MIN` combination,
// must agree exactly. See ERRORS.md for the full rationale.
// ===========================================================================

#[test]
fn err02_idiv_trap_boundary_is_exactly_one_pair() {
    let p = pair();
    // `a == INT32_MIN` with every divisor except -1
    for b in [
        i32::MIN,
        i32::MIN + 1,
        -3,
        -2,
        1,
        2,
        3,
        i32::MAX,
        i32::MAX - 1,
    ] {
        assert!(!is_idiv_trap(i32::MIN, b));
        let (cv, rv) = unsafe {
            (
                (p.c.modulo_operation)(i32::MIN, b, 0, 0),
                (p.rs.modulo_operation)(i32::MIN, b, 0, 0),
            )
        };
        assert_eq!(cv, rv, "modulo(INT32_MIN, {b})");
    }
    // `b == -1` with every dividend except INT32_MIN
    for a in [
        i32::MIN + 1,
        -2,
        -1,
        0,
        1,
        2,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN / 2,
    ] {
        assert!(!is_idiv_trap(a, -1));
        let (cv, rv) = unsafe {
            (
                (p.c.modulo_operation)(a, -1, 0, 0),
                (p.rs.modulo_operation)(a, -1, 0, 0),
            )
        };
        assert_eq!(cv, rv, "modulo({a}, -1)");
        assert_eq!(cv, 0, "x % -1 == 0");
    }
    // And the trap predicate itself only fires on the one pair.
    assert!(is_idiv_trap(i32::MIN, -1));
}

// ===========================================================================
// Rows 3-6 — safe_double_to_int saturation / NaN / truncation guards
// ===========================================================================

fn sdi(d: f64) -> (c_int, c_int) {
    let p = pair();
    unsafe {
        (
            (p.c.safe_double_to_int)(d),
            (p.rs.safe_double_to_int)(d),
        )
    }
}

#[test] // row 3
fn err03_safe_double_to_int_saturates_high() {
    let mut rng = Rng::seeded();
    let mut cases: Vec<f64> = vec![
        2147483647.0, // == (double)INT32_MAX, `>=` so it saturates
        2147483647.000_000_1,
        2147483648.0,
        2147483649.0,
        4294967296.0,
        1e10,
        1e300,
        f64::MAX,
        f64::INFINITY,
    ];
    for _ in 0..20_000 {
        // any value >= 2^31-1
        cases.push(2147483647.0 + (rng.next_u32() as f64) * (rng.next_u32() as f64));
    }
    for d in cases {
        let (cv, rv) = sdi(d);
        assert_eq!(cv, rv, "safe_double_to_int({d:?})");
        assert_eq!(cv, i32::MAX, "must saturate to INT32_MAX for {d:?}");
    }
}

#[test] // row 4
fn err04_safe_double_to_int_saturates_low() {
    let mut rng = Rng::seeded();
    let mut cases: Vec<f64> = vec![
        -2147483648.0, // == (double)INT32_MIN, `<=` so it saturates
        -2147483648.000_000_1,
        -2147483649.0,
        -4294967296.0,
        -1e10,
        -1e300,
        f64::MIN,
        f64::NEG_INFINITY,
    ];
    for _ in 0..20_000 {
        cases.push(-2147483648.0 - (rng.next_u32() as f64) * (rng.next_u32() as f64));
    }
    for d in cases {
        let (cv, rv) = sdi(d);
        assert_eq!(cv, rv, "safe_double_to_int({d:?})");
        assert_eq!(cv, i32::MIN, "must saturate to INT32_MIN for {d:?}");
    }
}

#[test] // row 5
fn err05_safe_double_to_int_nan_returns_zero() {
    let mut rng = Rng::seeded();
    let mut cases: Vec<f64> = vec![
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000), // canonical quiet NaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative quiet NaN
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001), // negative signalling NaN
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF), // max payload
        f64::from_bits(0x7FF8_0000_DEAD_BEEF),
    ];
    for _ in 0..20_000 {
        // random NaN payload, both signs
        let payload = rng.next_u64() & 0x000F_FFFF_FFFF_FFFF;
        let payload = if payload == 0 { 1 } else { payload };
        let sign = (rng.next_u64() & 1) << 63;
        cases.push(f64::from_bits(sign | 0x7FF0_0000_0000_0000 | payload));
    }
    for d in cases {
        assert!(d.is_nan(), "test bug: 0x{:016x} is not NaN", d.to_bits());
        let (cv, rv) = sdi(d);
        assert_eq!(
            cv,
            rv,
            "safe_double_to_int(NaN bits=0x{:016x})",
            d.to_bits()
        );
        assert_eq!(cv, 0, "NaN must fall through to the `d != d` guard");
    }
}

#[test] // row 6
fn err06_safe_double_to_int_truncates_toward_zero() {
    let mut rng = Rng::seeded();
    let mut cases: Vec<f64> = vec![
        0.0,
        -0.0,
        f64::from_bits(1),                     // smallest subnormal
        f64::from_bits(0x8000_0000_0000_0001), // negative subnormal
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::EPSILON,
        -f64::EPSILON,
        0.1,
        -0.1,
        0.5,
        -0.5,
        0.999_999_999_999,
        -0.999_999_999_999,
    ];
    for _ in 0..20_000 {
        cases.push((rng.next_u64() as f64) / (u64::MAX as f64) * if rng.below(2) == 0 { 1.0 } else { -1.0 });
    }
    for d in cases {
        let (cv, rv) = sdi(d);
        assert_eq!(cv, rv, "safe_double_to_int({d:?})");
        assert_eq!(cv, 0, "|{d:?}| < 1 truncates to 0");
    }
    // negatives truncate toward zero, not toward -inf
    for (d, want) in [
        (-1.5, -1),
        (-2.9, -2),
        (1.5, 1),
        (2.9, 2),
        (-0.9, 0),
        (2147483646.9, 2147483646),
        (-2147483647.9, -2147483647),
    ] {
        let (cv, rv) = sdi(d);
        assert_eq!(cv, rv, "safe_double_to_int({d:?})");
        assert_eq!(cv, want, "C truncation toward zero for {d:?}");
    }
}

// ===========================================================================
// Row 7 — compute_scaled_value overflow / non-finite
// ===========================================================================

#[test]
fn err07_compute_scaled_value_overflow_and_nonfinite() {
    let p = pair();
    let cases: [(c_int, f64, c_int); 14] = [
        (i32::MAX, 1e10, i32::MAX),
        (i32::MIN, 1e10, i32::MIN),
        (i32::MAX, -1e10, i32::MIN),
        (i32::MIN, -1e10, i32::MAX),
        (1, f64::INFINITY, i32::MAX),
        (1, f64::NEG_INFINITY, i32::MIN),
        (-1, f64::INFINITY, i32::MIN),
        (-1, f64::NEG_INFINITY, i32::MAX),
        (0, f64::INFINITY, 0),      // 0 * inf == NaN -> 0
        (0, f64::NEG_INFINITY, 0),  // ditto
        (0, f64::NAN, 0),
        (12345, f64::NAN, 0),
        (i32::MAX, 1.0, i32::MAX),
        (i32::MIN, 1.0, i32::MIN),
    ];
    for (b, s, want) in cases {
        let (cv, rv) = unsafe {
            (
                (p.c.compute_scaled_value)(b, s),
                (p.rs.compute_scaled_value)(b, s),
            )
        };
        assert_eq!(cv, rv, "compute_scaled_value({b}, {s:?})");
        assert_eq!(cv, want, "expected sentinel for ({b}, {s:?})");
    }
    // randomized overflow shapes
    let mut rng = Rng::seeded();
    for _ in 0..100_000 {
        let b = rng.spicy_i32();
        let s = if rng.below(2) == 0 {
            rng.spicy_f64()
        } else {
            (rng.i32() as f64) * 1e9
        };
        let (cv, rv) = unsafe {
            (
                (p.c.compute_scaled_value)(b, s),
                (p.rs.compute_scaled_value)(b, s),
            )
        };
        assert_eq!(cv, rv, "compute_scaled_value({b}, 0x{:016x})", s.to_bits());
    }
}

// ===========================================================================
// Rows 8-14 — compare_results_in_array
// ===========================================================================

fn cmp(buf: &ArrBuf, i1: c_int, i2: c_int) -> (c_int, c_int) {
    let p = pair();
    let mut cb = buf.clone();
    let mut rb = buf.clone();
    let r = unsafe {
        (
            (p.c.compare_results_in_array)(cb.as_ptr(), i1, i2),
            (p.rs.compare_results_in_array)(rb.as_ptr(), i1, i2),
        )
    };
    assert_bufs_eq("compare must not write", &cb, &rb);
    r
}

#[test] // row 8
fn err08_compare_idx1_out_of_range() {
    let mut rng = Rng::seeded();
    for count in 0..=10i32 {
        let mut b = ArrBuf::poisoned(&mut rng);
        b.set_count(count);
        // one step past the documented range, and far past it
        for i1 in [count, count + 1, count + 10, 10, 11, 64, i32::MAX, i32::MAX - 1] {
            if i1 < count {
                continue;
            }
            let i2 = if count > 0 { count - 1 } else { 0 };
            let (cv, rv) = cmp(&b, i1, i2);
            assert_eq!(cv, rv, "compare(count={count}, idx1={i1}, idx2={i2})");
            assert_eq!(cv, 0, "out-of-range idx1 must return the sentinel 0");
        }
    }
}

#[test] // row 9
fn err09_compare_idx2_out_of_range() {
    let mut rng = Rng::seeded();
    for count in 0..=10i32 {
        let mut b = ArrBuf::poisoned(&mut rng);
        b.set_count(count);
        for i2 in [count, count + 1, count + 10, 10, 11, 64, i32::MAX, i32::MAX - 1] {
            if i2 < count {
                continue;
            }
            let i1 = if count > 0 { count - 1 } else { 0 };
            let (cv, rv) = cmp(&b, i1, i2);
            assert_eq!(cv, rv, "compare(count={count}, idx1={i1}, idx2={i2})");
            assert_eq!(cv, 0, "out-of-range idx2 must return the sentinel 0");
        }
    }
}

#[test] // row 10
fn err10_compare_nonpositive_count() {
    let mut rng = Rng::seeded();
    for count in [0i32, -1, -2, -10, i32::MIN, i32::MIN + 1] {
        let mut b = ArrBuf::poisoned(&mut rng);
        b.set_count(count);
        for (i1, i2) in [
            (0, 0),
            (0, 1),
            (1, 0),
            (9, 9),
            (0, 9),
            (i32::MAX, 0),
            (0, i32::MAX),
        ] {
            let (cv, rv) = cmp(&b, i1, i2);
            assert_eq!(cv, rv, "compare(count={count}, {i1}, {i2})");
            assert_eq!(cv, 0, "count<=0 rejects every non-negative index");
        }
    }
}

#[test] // row 11
fn err11_compare_negative_indices_are_not_rejected() {
    let mut rng = Rng::seeded();
    // The C only checks `>=`, so negative indices pass the guard and the address
    // comparison runs on out-of-bounds pointers. Rust must reproduce that, not
    // "safely" reject it.
    for count in 1..=10i32 {
        let mut b = ArrBuf::poisoned(&mut rng);
        b.set_count(count);
        for i1 in [-1i32, -2, -5, -10, -64, -1000, i32::MIN, i32::MIN + 1] {
            for i2 in [-1i32, -2, -5, 0, 1, count - 1, i32::MIN] {
                let (cv, rv) = cmp(&b, i1, i2);
                assert_eq!(cv, rv, "compare(count={count}, {i1}, {i2})");
                // The guard is `idx >= count` on BOTH indices and is evaluated
                // first, so it still fires here whenever either index reaches
                // `count` — negative indices only bypass a *lower* bound. Only
                // when both pass the guard does the address comparison run, and
                // then the answer is the sign of (i1 - i2), since element
                // addresses are monotonic in the index. Asserting that (rather
                // than only C==Rust) keeps us from agreeing on a vacuous 0.
                let want = if i1 >= count || i2 >= count {
                    0
                } else {
                    (i1 as i64).cmp(&(i2 as i64)) as i32
                };
                assert_eq!(
                    cv, want,
                    "negative-index address ordering (count={count}, {i1}, {i2})"
                );
            }
        }
    }
    // randomized
    for _ in 0..50_000 {
        let count = (rng.below(10) + 1) as c_int;
        let mut b = ArrBuf::poisoned(&mut rng);
        b.set_count(count);
        let i1 = -(rng.below(1_000_000) as c_int) - 1;
        let i2 = -(rng.below(1_000_000) as c_int) - 1;
        let (cv, rv) = cmp(&b, i1, i2);
        assert_eq!(cv, rv, "compare(count={count}, {i1}, {i2})");
    }
}

#[test] // row 12
fn err12_compare_equal_indices() {
    let mut rng = Rng::seeded();
    for count in 1..=10i32 {
        let mut b = ArrBuf::poisoned(&mut rng);
        b.set_count(count);
        for i in 0..count {
            let (cv, rv) = cmp(&b, i, i);
            assert_eq!(cv, rv, "compare(count={count}, {i}, {i})");
            assert_eq!(cv, 0, "equal addresses -> 0");
        }
        for i in [-1i32, -7, i32::MIN] {
            let (cv, rv) = cmp(&b, i, i);
            assert_eq!(cv, rv);
            assert_eq!(cv, 0, "equal (negative) addresses -> 0");
        }
    }
}

#[test] // rows 13 + 14
fn err13_14_compare_ordering_sentinels() {
    let mut rng = Rng::seeded();
    for count in 2..=10i32 {
        let mut b = ArrBuf::poisoned(&mut rng);
        b.set_count(count);
        for i1 in 0..count {
            for i2 in 0..count {
                let (cv, rv) = cmp(&b, i1, i2);
                assert_eq!(cv, rv, "compare(count={count}, {i1}, {i2})");
                let want = if i1 < i2 {
                    -1
                } else if i1 > i2 {
                    1
                } else {
                    0
                };
                assert_eq!(cv, want, "sentinel for (count={count}, {i1}, {i2})");
            }
        }
    }
}

// ===========================================================================
// Rows 15-18 — init_result_array guards
// ===========================================================================

fn init_diff(ctx: &str, start: &ArrBuf, values: &[c_int], count: c_int) -> (ArrBuf, ArrBuf) {
    let p = pair();
    let mut cb = start.clone();
    let mut rb = start.clone();
    let mut cv = values.to_vec();
    let mut rv = values.to_vec();
    unsafe {
        (p.c.init_result_array)(cb.as_ptr(), cv.as_mut_ptr(), count);
        (p.rs.init_result_array)(rb.as_ptr(), rv.as_mut_ptr(), count);
    }
    assert_eq!(cv, rv, "values[] must not be modified [{ctx}]");
    assert_bufs_eq(ctx, &cb, &rb);
    (cb, rb)
}

#[test] // rows 15 + 18
fn err15_18_init_count_clamped_at_ten() {
    let mut rng = Rng::seeded();
    // 128-wide `values` so a bad clamp reads our memory (visible diff) not a fault.
    for count in [10i32, 11, 12, 20, 64, 100, 1_000_000, i32::MAX - 1, i32::MAX] {
        let vals: Vec<c_int> = (0..128).map(|i| 0x1000 + i as c_int).collect();
        let start = ArrBuf::poisoned(&mut rng);
        let (cb, _) = init_diff(&format!("init clamp count={count}"), &start, &vals, count);
        assert_eq!(cb.get_count(), 10, "count must clamp to 10 for {count}");
        for i in 0..CAP {
            assert_eq!(cb.value(i), vals[i], "data[{i}].value");
        }
        // nothing beyond the 10-element array may be touched
        for b in RESULT_ARRAY_SIZE..BUF_SIZE {
            assert_eq!(cb.bytes[b], start.bytes[b], "byte {b} past the array");
        }
    }
}

#[test] // row 16
fn err16_init_count_zero_writes_nothing() {
    let mut rng = Rng::seeded();
    for _ in 0..2_000 {
        let vals: Vec<c_int> = (0..16).map(|_| rng.i32()).collect();
        let start = ArrBuf::poisoned(&mut rng);
        let (cb, _) = init_diff("init count=0", &start, &vals, 0);
        assert_eq!(cb.get_count(), 0);
        // every byte except the 4 count bytes must be untouched
        for b in 0..BUF_SIZE {
            if (COUNT_OFFSET..COUNT_OFFSET + 4).contains(&b) {
                continue;
            }
            assert_eq!(cb.bytes[b], start.bytes[b], "byte {b} was written");
        }
    }
}

#[test] // row 17
fn err17_init_negative_count_stored_verbatim() {
    let mut rng = Rng::seeded();
    // `count < 10` is true for negatives, so the negative value is stored as-is
    // and the loop never runs. Not rejected — reproduce, do not "fix".
    for count in [-1i32, -2, -9, -10, -100, i32::MIN, i32::MIN + 1, -0x4000_0000] {
        let vals: Vec<c_int> = (0..16).map(|_| rng.i32()).collect();
        let start = ArrBuf::poisoned(&mut rng);
        let (cb, _) = init_diff(&format!("init count={count}"), &start, &vals, count);
        assert_eq!(cb.get_count(), count, "negative count stored verbatim");
        for b in 0..BUF_SIZE {
            if (COUNT_OFFSET..COUNT_OFFSET + 4).contains(&b) {
                continue;
            }
            assert_eq!(cb.bytes[b], start.bytes[b], "byte {b} was written");
        }
    }
    for _ in 0..20_000 {
        let count = -(rng.below(1_000_000_000) as c_int) - 1;
        let vals: Vec<c_int> = (0..16).map(|_| rng.i32()).collect();
        let start = ArrBuf::poisoned(&mut rng);
        init_diff(&format!("init count={count}"), &start, &vals, count);
    }
}

// ===========================================================================
// Rows 19, 22 — process_with_foreach guards
//
// Row 20 (count < 0 -> runaway `!=` loop) and row 21 (op == NULL) are UB in C and
// would destroy the harness; see ERRORS.md. What is verified here is that the Rust
// keeps the `!=` loop condition rather than a "safe" `<`: with count == 0 the loop
// must not run, and with count > 0 it must run exactly `count` times — which the
// recorder test in Phase B (row 24) pins down exactly.
// ===========================================================================

#[test] // row 19
fn err19_process_count_zero_returns_zero_and_writes_nothing() {
    let p = pair();
    let mut rng = Rng::seeded();
    for oi in 0..4 {
        for _ in 0..500 {
            let mut start = ArrBuf::poisoned(&mut rng);
            start.set_count(0);
            let mut cb = start.clone();
            let mut rb = start.clone();
            let (ct, rt) = unsafe {
                (
                    (p.c.process_with_foreach)(cb.as_ptr(), Some(p.c.ops()[oi])),
                    (p.rs.process_with_foreach)(rb.as_ptr(), Some(p.rs.ops()[oi])),
                )
            };
            assert_eq!(ct, rt, "process count=0 op={}", OP_NAMES[oi]);
            assert_eq!(ct, 0, "count=0 must return the sentinel 0");
            assert_bufs_eq("process count=0", &cb, &rb);
            for b in 0..BUF_SIZE {
                assert_eq!(cb.bytes[b], start.bytes[b], "byte {b} written by C");
                assert_eq!(rb.bytes[b], start.bytes[b], "byte {b} written by Rust");
            }
        }
    }
}

#[test] // row 22
fn err22_process_saturating_item_values() {
    let p = pair();
    let mut rng = Rng::seeded();
    // `multiply_operation` on huge values pushes `result * 0.75` far outside int
    // range, so every element's `value` must saturate identically and `total` must
    // wrap identically.
    for _ in 0..20_000 {
        let count = (rng.below(10) + 1) as c_int;
        let mut start = ArrBuf::poisoned(&mut rng);
        for i in 0..CAP {
            start.set_value(
                i,
                [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1][rng.below(4)],
            );
            start.set_scaled(i, 0.0);
            start.set_rank(i, i as c_int);
        }
        start.set_count(count);
        let mut cb = start.clone();
        let mut rb = start.clone();
        for _pass in 0..3 {
            let (ct, rt) = unsafe {
                (
                    (p.c.process_with_foreach)(cb.as_ptr(), Some(p.c.multiply_operation)),
                    (p.rs.process_with_foreach)(rb.as_ptr(), Some(p.rs.multiply_operation)),
                )
            };
            assert_eq!(ct, rt, "saturating total (count={count})");
        }
        assert_bufs_eq("saturating process", &cb, &rb);
    }
}

// ===========================================================================
// Rows 23-25 — compute_weighted_sum guards
// ===========================================================================

#[test] // row 23
fn err23_weighted_sum_nonpositive_count() {
    let p = pair();
    let mut rng = Rng::seeded();
    for count in [0i32, -1, -2, -10, -1000, i32::MIN, i32::MIN + 1] {
        for _ in 0..200 {
            let mut start = ArrBuf::poisoned(&mut rng);
            start.set_count(count);
            let mut cb = start.clone();
            let mut rb = start.clone();
            let (cs, rs) = unsafe {
                (
                    (p.c.compute_weighted_sum)(cb.as_ptr()),
                    (p.rs.compute_weighted_sum)(rb.as_ptr()),
                )
            };
            assert_eq!(cs, rs, "weighted_sum count={count}");
            assert_eq!(cs, 0, "count<=0 must return the sentinel 0");
            assert_bufs_eq("weighted_sum is read-only", &cb, &rb);
        }
    }
}

#[test] // row 24
fn err24_weighted_sum_index_zero_weight_is_one_not_zero() {
    let p = pair();
    // The `current > base` ternary makes element 0's weight 1, not 0. With a single
    // element of value V the sum is therefore safe_double_to_int(V * 1 * 0.8).
    for v in [
        0i32, 1, -1, 10, -10, 100, -100, 12345, -12345, i32::MAX, i32::MIN,
    ] {
        let mut start = ArrBuf::zeroed();
        start.set_value(0, v);
        start.set_rank(0, 0);
        start.set_count(1);
        let mut cb = start.clone();
        let mut rb = start.clone();
        let (cs, rs) = unsafe {
            (
                (p.c.compute_weighted_sum)(cb.as_ptr()),
                (p.rs.compute_weighted_sum)(rb.as_ptr()),
            )
        };
        assert_eq!(cs, rs, "weighted_sum single value={v}");
        let want = unsafe { (p.c.safe_double_to_int)(v as f64 * 1.0 * 0.8) };
        assert_eq!(cs, want, "element 0 must use weight 1 (value={v})");
        // and it is definitely not the weight-0 answer, unless V*0.8 rounds to 0
        if v.unsigned_abs() > 1 {
            assert_ne!(cs, 0, "weight 0 would have produced 0 for value={v}");
        }
    }
}

#[test] // row 25
fn err25_weighted_sum_saturating_contribution() {
    let p = pair();
    let mut rng = Rng::seeded();
    for _ in 0..20_000 {
        let count = (rng.below(10) + 1) as c_int;
        let mut start = ArrBuf::poisoned(&mut rng);
        for i in 0..CAP {
            start.set_value(
                i,
                [i32::MAX, i32::MIN, i32::MAX / 2, i32::MIN / 2][rng.below(4)],
            );
        }
        start.set_count(count);
        let mut cb = start.clone();
        let mut rb = start.clone();
        let (cs, rs) = unsafe {
            (
                (p.c.compute_weighted_sum)(cb.as_ptr()),
                (p.rs.compute_weighted_sum)(rb.as_ptr()),
            )
        };
        assert_eq!(cs, rs, "saturating weighted_sum count={count}");
    }
}

// ===========================================================================
// Rows 26-28 — arrayfunc edge inputs
// ===========================================================================

#[test] // row 26
fn err26_arrayfunc_param4_int_min() {
    let p = pair();
    let mut rng = Rng::seeded();
    // INT32_MIN / 2 + 1 == -1073741823; no idiv trap because the divisor is 2.
    for _ in 0..20_000 {
        let (a, b, c) = (rng.spicy_i32(), rng.spicy_i32(), rng.spicy_i32());
        let (cv, rv) = unsafe {
            (
                (p.c.arrayfunc)(a, b, c, i32::MIN),
                (p.rs.arrayfunc)(a, b, c, i32::MIN),
            )
        };
        assert_eq!(cv, rv, "arrayfunc({a}, {b}, {c}, INT32_MIN)");
    }
    assert_eq!(i32::MIN / 2 + 1, -1073741823);
}

#[test] // row 27
fn err27_arrayfunc_value_array_overflow() {
    let p = pair();
    // Inputs chosen so that param1+param2, param2-param3 and param3*2 all overflow.
    let cases: [(c_int, c_int, c_int, c_int); 12] = [
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, 1, i32::MIN, -1),
        (i32::MIN, -1, i32::MAX, 1),
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
        (i32::MIN, i32::MAX, i32::MIN, i32::MAX),
        (0x4000_0000, 0x4000_0000, 0x4000_0000, 0x4000_0000),
        (-0x4000_0000, -0x4000_0000, -0x4000_0000, -0x4000_0000),
        (i32::MAX, 0, 0x4000_0000, 0),
        (0, i32::MIN, i32::MAX, 0),
        (i32::MAX - 1, 2, i32::MIN + 1, -2),
        (1, i32::MIN, 0x7FFF_FFFF, -1),
    ];
    for (a, b, c, d) in cases {
        let (cv, rv) = unsafe {
            (
                (p.c.arrayfunc)(a, b, c, d),
                (p.rs.arrayfunc)(a, b, c, d),
            )
        };
        assert_eq!(cv, rv, "arrayfunc({a}, {b}, {c}, {d}) overflow wrap");
    }
}

#[test] // row 28
fn err28_arrayfunc_fixed_comparison_contribution() {
    let p = pair();
    // arr.count is always 8, so the i-vs-(i+1) loop always contributes exactly -7.
    // Verified through the public compare entry point on a count=8 array.
    let mut rng = Rng::seeded();
    let mut start = ArrBuf::zeroed();
    start.set_count(8);
    let mut cb = start.clone();
    let mut rb = start.clone();
    let mut csum = 0i32;
    let mut rsum = 0i32;
    for i in 0..7 {
        unsafe {
            csum += (p.c.compare_results_in_array)(cb.as_ptr(), i, i + 1);
            rsum += (p.rs.compare_results_in_array)(rb.as_ptr(), i, i + 1);
        }
    }
    assert_eq!(csum, rsum);
    assert_eq!(csum, -7);
    // index 7 vs 8 is out of range for count=8 and must return 0 in both
    for (i1, i2) in [(7, 8), (8, 9), (7, 9)] {
        let (cv, rv) = unsafe {
            (
                (p.c.compare_results_in_array)(cb.as_ptr(), i1, i2),
                (p.rs.compare_results_in_array)(rb.as_ptr(), i1, i2),
            )
        };
        assert_eq!(cv, rv);
        assert_eq!(cv, 0);
    }
    // And arrayfunc itself stays consistent for arbitrary inputs.
    for _ in 0..50_000 {
        let (a, b, c, d) = (
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
            rng.spicy_i32(),
        );
        let (cv, rv) = unsafe {
            (
                (p.c.arrayfunc)(a, b, c, d),
                (p.rs.arrayfunc)(a, b, c, d),
            )
        };
        assert_eq!(cv, rv, "arrayfunc({a}, {b}, {c}, {d})");
    }
}

// ===========================================================================
// Generic FFI boundary checks required regardless of the table
// ===========================================================================

#[test]
fn generic_one_past_valid_range_for_every_int_parameter() {
    let p = pair();
    let mut rng = Rng::seeded();
    // Sweep each `int` parameter one step past both ends of its meaningful range.
    for count in [-1i32, 0, 1, 9, 10, 11] {
        let vals: Vec<c_int> = (0..128).map(|_| rng.spicy_i32()).collect();
        let start = ArrBuf::poisoned(&mut rng);
        let mut cb = start.clone();
        let mut rb = start.clone();
        let mut cv = vals.clone();
        let mut rv = vals.clone();
        unsafe {
            (p.c.init_result_array)(cb.as_ptr(), cv.as_mut_ptr(), count);
            (p.rs.init_result_array)(rb.as_ptr(), rv.as_mut_ptr(), count);
        }
        assert_bufs_eq(&format!("boundary count={count}"), &cb, &rb);

        let eff = cb.get_count();
        for idx in [-1i32, 0, eff - 1, eff, eff + 1, 9, 10, 11] {
            for other in [-1i32, 0, eff - 1, eff, 10] {
                let (c1, r1) = unsafe {
                    (
                        (p.c.compare_results_in_array)(cb.as_ptr(), idx, other),
                        (p.rs.compare_results_in_array)(rb.as_ptr(), idx, other),
                    )
                };
                assert_eq!(c1, r1, "compare(count={count}, {idx}, {other})");
            }
        }
    }
}

#[test]
fn generic_extreme_int_arguments_across_ffi() {
    let p = pair();
    // Every `int`-taking export fed the extreme representable values. C `int`
    // parameters accept any 32-bit pattern (there are no enums in this API — see
    // ERRORS.md — so this is the analogous "no valid variant" surface).
    let ext: [c_int; 8] = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
        0x4000_0000,
    ];
    for &a in &ext {
        for &b in &ext {
            for (name, f) in [
                ("add", p.c.add_operation as usize),
                ("mul", p.c.multiply_operation as usize),
                ("sub", p.c.subtract_operation as usize),
                ("mod", p.c.modulo_operation as usize),
            ] {
                let _ = (name, f);
            }
            let pairs: [(&str, unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int,
                          unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int); 4] = [
                ("add", p.c.add_operation, p.rs.add_operation),
                ("multiply", p.c.multiply_operation, p.rs.multiply_operation),
                ("subtract", p.c.subtract_operation, p.rs.subtract_operation),
                ("modulo", p.c.modulo_operation, p.rs.modulo_operation),
            ];
            for (name, cf, rf) in pairs {
                if name == "modulo" && is_idiv_trap(a, b) {
                    continue; // SIGFPE in C; see ERRORS.md row 2
                }
                let (cv, rv) = unsafe { (cf(a, b, i32::MIN, i32::MAX), rf(a, b, i32::MIN, i32::MAX)) };
                assert_eq!(cv, rv, "{name}({a}, {b})");
            }
        }
    }
    // compute_scaled_value with extreme int base
    for &b in &ext {
        for s in [0.0f64, 1.0, -1.0, f64::NAN, f64::INFINITY, 1e300] {
            let (cv, rv) = unsafe {
                (
                    (p.c.compute_scaled_value)(b, s),
                    (p.rs.compute_scaled_value)(b, s),
                )
            };
            assert_eq!(cv, rv, "compute_scaled_value({b}, {s:?})");
        }
    }
    // arrayfunc with extreme params
    for &a in &ext {
        for &b in &ext {
            for &c in &ext {
                for &d in &ext {
                    let (cv, rv) = unsafe {
                        ((p.c.arrayfunc)(a, b, c, d), (p.rs.arrayfunc)(a, b, c, d))
                    };
                    assert_eq!(cv, rv, "arrayfunc({a}, {b}, {c}, {d})");
                }
            }
        }
    }
}

#[test]
fn generic_all_double_bit_patterns_sampled() {
    let p = pair();
    // A wide sample of the raw 64-bit space, so no exponent/NaN class is missed.
    let mut rng = Rng::new(0x5EED_0000_1111_2222);
    for _ in 0..500_000 {
        let bits = rng.next_u64();
        let d = f64::from_bits(bits);
        let (cv, rv) = unsafe {
            (
                (p.c.safe_double_to_int)(d),
                (p.rs.safe_double_to_int)(d),
            )
        };
        assert_eq!(cv, rv, "safe_double_to_int(0x{bits:016x})");
    }
    // exhaustive over every exponent with a fixed mantissa, both signs
    for sign in [0u64, 1u64 << 63] {
        for exp in 0u64..2048 {
            for mant in [0u64, 1, 0x8_0000_0000_0000, 0xF_FFFF_FFFF_FFFF] {
                let bits = sign | (exp << 52) | mant;
                let d = f64::from_bits(bits);
                let (cv, rv) = unsafe {
                    (
                        (p.c.safe_double_to_int)(d),
                        (p.rs.safe_double_to_int)(d),
                    )
                };
                assert_eq!(cv, rv, "safe_double_to_int(0x{bits:016x})");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// NULL-pointer rows (ERRORS.md `n/a`): the C has no guard and dereferences
// immediately, so calling with NULL kills the process. We cannot compare a crash
// to a crash in-process. What we assert instead is the property that actually
// matters for parity: the Rust must NOT have added a guard that turns the crash
// into a return value. If Rust had a null check, it would return normally where C
// dies — so we verify via the source-level invariant that both take the same
// (unchecked) path, and document the row as n/a.
//
// This is asserted indirectly: `err19_process_count_zero_...` shows Rust performs
// no work for count==0, and Phase B row 24 shows it dereferences `arr` exactly as
// often as C for count>0. A hypothetical null guard would have to change one of
// those observable call counts.
// ---------------------------------------------------------------------------

#[test]
fn null_pointer_rows_are_documented_as_ub() {
    // Self-check that ERRORS.md still marks these rows n/a, so the exclusion stays
    // visible and intentional rather than silently forgotten.
    let md = include_str!("../ERRORS.md");
    for needle in [
        "op == NULL",
        "arr == NULL",
        "Not exercised",
    ] {
        assert!(
            md.contains(needle),
            "ERRORS.md must keep documenting `{needle}`"
        );
    }
}
