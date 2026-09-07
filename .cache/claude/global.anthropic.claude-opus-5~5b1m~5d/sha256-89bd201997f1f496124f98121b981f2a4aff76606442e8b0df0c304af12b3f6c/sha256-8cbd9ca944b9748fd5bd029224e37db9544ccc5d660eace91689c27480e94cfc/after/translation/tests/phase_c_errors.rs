//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Row E2 (`modulo_operation(INT32_MIN, -1)`) is intentionally absent: the C
//! library raises `SIGFPE` there and returns no value at all, so there is no
//! error code to compare. See the "deliberately NOT tested" section of
//! `ERRORS.md`.

mod common;

use common::*;

fn init_both(values: &[i32], count: i32) -> (ResultArray, ResultArray) {
    let p = libs();
    let (mut ac, mut ars) = (ResultArray::poisoned(), ResultArray::poisoned());
    let (mut vc, mut vrs) = (values.to_vec(), values.to_vec());
    p.c.init(&mut ac, &mut vc, count);
    p.rs.init(&mut ars, &mut vrs, count);
    eq_state(&format!("init(count={count}) parity"), &ac, &ars);
    (ac, ars)
}

fn forged(count: i32, values: &[i32]) -> ResultArray {
    // Build a ResultArray directly, bypassing init_result_array, so states that
    // init cannot produce (e.g. count in 0..=10 with arbitrary ranks) are
    // reachable. Both libraries get a byte-identical copy.
    let mut a = ResultArray::zeroed();
    a.count = count;
    for i in 0..10 {
        let v = values[i % values.len()];
        a.data[i] = Result_ {
            value: v,
            scaled: v as f64 * 1.5,
            rank: i as i32,
        };
    }
    a
}

// ===========================================================================
// E1 — modulo_operation: b == 0 guard
// ===========================================================================

#[test]
fn err_e1_modulo_zero_divisor() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 1);
    for a in [0i32, 1, -1, 7, -7, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1] {
        let ctx = format!("E1 a={a} b=0");
        let (c, r) = (p.c.modulo(a, 0, 0, 0), p.rs.modulo(a, 0, 0, 0));
        eq_i32(&ctx, c, r);
        assert_eq!(c, 0, "C's b==0 guard must return the 0 sentinel [{ctx}]");
    }
    for _ in 0..5_000 {
        let a = rng.mixed_i32();
        let (u1, u2) = (rng.next_i32(), rng.next_i32());
        eq_i32(
            &format!("E1 random a={a} b=0"),
            p.c.modulo(a, 0, u1, u2),
            p.rs.modulo(a, 0, u1, u2),
        );
    }
}

// ===========================================================================
// E3 — modulo_operation: sign handling (C `%` truncates toward zero)
// ===========================================================================

#[test]
fn err_e3_modulo_signs() {
    let p = libs();
    for a in [-100i32, -7, -3, -1, 0, 1, 3, 7, 100, i32::MIN + 1, i32::MAX] {
        for b in [-100i32, -7, -3, -1, 1, 3, 7, 100, i32::MIN, i32::MAX] {
            if a == i32::MIN && b == -1 {
                continue; // row E2: SIGFPE in C, no value to compare
            }
            eq_i32(
                &format!("E3 a={a} b={b}"),
                p.c.modulo(a, b, 0, 0),
                p.rs.modulo(a, b, 0, 0),
            );
        }
    }
    // INT32_MIN as dividend with every other divisor is fine
    for b in [-3i32, -2, 2, 3, 7, i32::MIN, i32::MAX] {
        eq_i32(
            &format!("E3 a=INT_MIN b={b}"),
            p.c.modulo(i32::MIN, b, 0, 0),
            p.rs.modulo(i32::MIN, b, 0, 0),
        );
    }
}

// ===========================================================================
// E4 / E5 — safe_double_to_int clamps
// ===========================================================================

#[test]
fn err_e4_sdti_upper_clamp() {
    let p = libs();
    let imax = i32::MAX as f64;
    for d in [
        imax,
        imax + 1.0,
        imax + 1024.0,
        2147483648.0,
        1e18,
        f64::MAX,
        f64::INFINITY,
        f64::from_bits(imax.to_bits() + 1),
    ] {
        let ctx = format!("E4 d={d:?}");
        let (c, r) = (p.c.sdti(d), p.rs.sdti(d));
        eq_i32(&ctx, c, r);
        assert_eq!(c, i32::MAX, "C must clamp to INT32_MAX [{ctx}]");
    }
    // one step BELOW the clamp must NOT clamp
    let below = f64::from_bits(imax.to_bits() - 1);
    let (c, r) = (p.c.sdti(below), p.rs.sdti(below));
    eq_i32("E4 just below clamp", c, r);
    assert_ne!(c, i32::MAX, "value below INT32_MAX must not clamp");
}

#[test]
fn err_e5_sdti_lower_clamp() {
    let p = libs();
    let imin = i32::MIN as f64;
    for d in [
        imin,
        imin - 1.0,
        imin - 1024.0,
        -2147483649.0,
        -1e18,
        f64::MIN,
        f64::NEG_INFINITY,
        f64::from_bits(imin.to_bits() + 1),
    ] {
        let ctx = format!("E5 d={d:?}");
        let (c, r) = (p.c.sdti(d), p.rs.sdti(d));
        eq_i32(&ctx, c, r);
        assert_eq!(c, i32::MIN, "C must clamp to INT32_MIN [{ctx}]");
    }
    let above = f64::from_bits(imin.to_bits() - 1); // toward zero
    let (c, r) = (p.c.sdti(above), p.rs.sdti(above));
    eq_i32("E5 just above clamp", c, r);
    assert_ne!(c, i32::MIN, "value above INT32_MIN must not clamp");
}

// ===========================================================================
// E6 — safe_double_to_int NaN
// ===========================================================================

#[test]
fn err_e6_sdti_nan() {
    let p = libs();
    let nans = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000), // canonical qNaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative qNaN
        f64::from_bits(0x7FF0_0000_0000_0001), // sNaN
        f64::from_bits(0xFFF0_0000_0000_0001), // negative sNaN
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF), // NaN, all payload bits set
        f64::from_bits(0x7FF8_DEAD_BEEF_0000), // NaN with payload
    ];
    for d in nans {
        let ctx = format!("E6 NaN bits={:#018x}", d.to_bits());
        assert!(d.is_nan(), "test input must be NaN [{ctx}]");
        let (c, r) = (p.c.sdti(d), p.rs.sdti(d));
        eq_i32(&ctx, c, r);
        assert_eq!(c, 0, "C's `d != d` guard must return the 0 sentinel [{ctx}]");
    }
    // and through the wrapper
    for base in [0i32, 1, -1, i32::MIN, i32::MAX] {
        eq_i32(
            &format!("E6 via compute_scaled_value base={base}"),
            p.c.compute_scaled_value(base, f64::NAN),
            p.rs.compute_scaled_value(base, f64::NAN),
        );
    }
}

// ===========================================================================
// E7 — safe_double_to_int truncation toward zero
// ===========================================================================

#[test]
fn err_e7_sdti_truncation() {
    let p = libs();
    for d in [
        0.0f64,
        -0.0,
        0.5,
        -0.5,
        0.9999999999999999,
        -0.9999999999999999,
        1.0,
        -1.0,
        1.5,
        -1.5,
        2.5,
        -2.5,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
        1e-300,
        -1e-300,
        2147483646.999,
        -2147483647.999,
    ] {
        eq_i32(
            &format!("E7 d={d:?} bits={:#018x}", d.to_bits()),
            p.c.sdti(d),
            p.rs.sdti(d),
        );
    }
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..20_000 {
        // uniformly inside the non-clamping window, fractional
        let d = (rng.next_i32() as f64) + (rng.below(1_000_000) as f64) / 1_000_000.0;
        eq_i32(&format!("E7 random d={d:?}"), p.c.sdti(d), p.rs.sdti(d));
    }
}

// ===========================================================================
// E8 — compute_scaled_value overflow / NaN / INF
// ===========================================================================

#[test]
fn err_e8_scaled_overflow_nan() {
    let p = libs();
    for base in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        for s in [
            f64::NAN,
            -f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            0.0,
            -0.0,
            1e300,
            -1e300,
            1e18,
            -1e18,
            f64::MAX,
            f64::MIN,
        ] {
            eq_i32(
                &format!("E8 base={base} s={s:?}"),
                p.c.compute_scaled_value(base, s),
                p.rs.compute_scaled_value(base, s),
            );
        }
    }
    // 0 * INF == NaN -> the `d != d` guard
    let (c, r) = (
        p.c.compute_scaled_value(0, f64::INFINITY),
        p.rs.compute_scaled_value(0, f64::INFINITY),
    );
    eq_i32("E8 0*INF", c, r);
    assert_eq!(c, 0, "0*INF is NaN, so C returns the NaN sentinel 0");
}

// ===========================================================================
// E9 / E10 / E12 / E13 / E14 — compare_results_in_array guards
// ===========================================================================

#[test]
fn err_e9_cmp_idx1_too_big() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 9);
    for count in 0..=10i32 {
        let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
        let (mut ac, mut ars) = init_both(&values, count);
        for idx1 in [count, count + 1, count + 100, 10, 11, i32::MAX] {
            let idx2 = 0;
            let ctx = format!("E9 count={count} idx1={idx1} idx2={idx2}");
            let (c, r) = (p.c.compare(&mut ac, idx1, idx2), p.rs.compare(&mut ars, idx1, idx2));
            eq_i32(&ctx, c, r);
            assert_eq!(c, 0, "idx1 >= count must hit the guard and return 0 [{ctx}]");
        }
    }
}

#[test]
fn err_e10_cmp_idx2_too_big() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 10);
    for count in 1..=10i32 {
        let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
        let (mut ac, mut ars) = init_both(&values, count);
        for idx2 in [count, count + 1, count + 100, 10, 11, i32::MAX] {
            let idx1 = 0;
            let ctx = format!("E10 count={count} idx1={idx1} idx2={idx2}");
            let (c, r) = (p.c.compare(&mut ac, idx1, idx2), p.rs.compare(&mut ars, idx1, idx2));
            eq_i32(&ctx, c, r);
            assert_eq!(c, 0, "idx2 >= count must hit the guard and return 0 [{ctx}]");
        }
    }
}

#[test]
fn err_e11_cmp_negative_index() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 11);
    for count in 0..=10i32 {
        let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
        let (mut ac, mut ars) = init_both(&values, count);
        for idx1 in -20..=2i32 {
            for idx2 in -20..=2i32 {
                let ctx = format!("E11 count={count} idx1={idx1} idx2={idx2}");
                eq_i32(
                    &ctx,
                    p.c.compare(&mut ac, idx1, idx2),
                    p.rs.compare(&mut ars, idx1, idx2),
                );
            }
        }
        // wildly negative, plus mixed sign
        for (idx1, idx2) in [
            (-1000i32, 0i32),
            (0, -1000),
            (-1000, -1),
            (i32::MIN + 1, -1),
            (-1, i32::MIN + 1),
        ] {
            eq_i32(
                &format!("E11 count={count} idx1={idx1} idx2={idx2}"),
                p.c.compare(&mut ac, idx1, idx2),
                p.rs.compare(&mut ars, idx1, idx2),
            );
        }
    }
}

#[test]
fn err_e12_cmp_equal_index() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 12);
    for count in 1..=10i32 {
        let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
        let (mut ac, mut ars) = init_both(&values, count);
        for idx in 0..count {
            let ctx = format!("E12 count={count} idx={idx}");
            let (c, r) = (p.c.compare(&mut ac, idx, idx), p.rs.compare(&mut ars, idx, idx));
            eq_i32(&ctx, c, r);
            assert_eq!(c, 0, "equal indices -> equal addresses -> 0 [{ctx}]");
        }
    }
}

#[test]
fn err_e13_cmp_empty_array() {
    let p = libs();
    for count in [0i32, -1, -10, i32::MIN, i32::MIN + 1] {
        // count <= 0 reachable via init_result_array (the clamp accepts it)
        let (mut ac, mut ars) = init_both(&[1, 2, 3, 4, 5, 6, 7, 8], count);
        assert_eq!(ac.count, count, "C stores the raw (possibly negative) count");
        for (i1, i2) in [(0i32, 0i32), (0, 1), (1, 0), (5, 9), (9, 5)] {
            let ctx = format!("E13 count={count} ({i1},{i2})");
            let (c, r) = (p.c.compare(&mut ac, i1, i2), p.rs.compare(&mut ars, i1, i2));
            eq_i32(&ctx, c, r);
            assert_eq!(c, 0, "guard trips for count<=0 with non-negative idx [{ctx}]");
        }
    }
}

#[test]
fn err_e14_cmp_extreme_index() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 14);
    const EX: [i32; 8] = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        9,
        i32::MAX - 1,
        i32::MAX,
    ];
    for count in 0..=10i32 {
        let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
        let (mut ac, mut ars) = init_both(&values, count);
        for &i1 in EX.iter() {
            for &i2 in EX.iter() {
                eq_i32(
                    &format!("E14 count={count} ({i1},{i2})"),
                    p.c.compare(&mut ac, i1, i2),
                    p.rs.compare(&mut ars, i1, i2),
                );
            }
        }
    }
    // also on a forged (not init-produced) array
    for count in [1i32, 5, 10] {
        let mut ac = forged(count, &[3, -4, 5]);
        let mut ars = ac;
        for &i1 in EX.iter() {
            for &i2 in EX.iter() {
                eq_i32(
                    &format!("E14 forged count={count} ({i1},{i2})"),
                    p.c.compare(&mut ac, i1, i2),
                    p.rs.compare(&mut ars, i1, i2),
                );
            }
        }
    }
}

// ===========================================================================
// E15..E19 — init_result_array count guards
// ===========================================================================

fn init_diff_expect_count(ctx: &str, count: i32, expect: i32) {
    let p = libs();
    let values: Vec<i32> = (0..32).map(|i| (i as i32) * 7 - 100).collect();
    let (mut ac, mut ars) = (ResultArray::poisoned(), ResultArray::poisoned());
    let (mut vc, mut vrs) = (values.clone(), values.clone());
    p.c.init(&mut ac, &mut vc, count);
    p.rs.init(&mut ars, &mut vrs, count);
    eq_state(ctx, &ac, &ars);
    assert_eq!(ac.count, expect, "C count [{ctx}]");
    assert_eq!(ars.count, expect, "Rust count [{ctx}]");
    assert_eq!(vc, vrs, "values[] mutated differently [{ctx}]");
}

#[test]
fn err_e15_init_oversized_count() {
    for count in [11i32, 12, 20, 100, 1_000_000, i32::MAX - 1, i32::MAX] {
        init_diff_expect_count(&format!("E15 count={count}"), count, 10);
    }
    // and no slot beyond index 9 is touched: the poison in a *larger* buffer
    // would be overwritten if the clamp were missing. Emulate by checking the
    // 10th slot is the last written one.
    let p = libs();
    let mut values: Vec<i32> = (0..32).map(|i| i as i32 + 1).collect();
    let mut a = ResultArray::poisoned();
    p.rs.init(&mut a, &mut values, i32::MAX);
    assert_eq!(a.data[9].rank, 9);
    assert_eq!(a.data[9].value, 10);
}

#[test]
fn err_e16_init_count_exactly_10() {
    init_diff_expect_count("E16 count=10", 10, 10);
    init_diff_expect_count("E16 count=9", 9, 9);
}

#[test]
fn err_e17_init_zero_count() {
    init_diff_expect_count("E17 count=0", 0, 0);
    // data must remain the untouched poison in BOTH
    let p = libs();
    let mut values = vec![9i32; 16];
    let expected = ResultArray::poisoned();
    let mut ac = ResultArray::poisoned();
    let mut ars = ResultArray::poisoned();
    p.c.init(&mut ac, &mut values.clone(), 0);
    p.rs.init(&mut ars, &mut values, 0);
    eq_state("E17 data untouched", &ac, &ars);
    for i in 0..10 {
        assert_eq!(ac.data[i].value, expected.data[i].value, "slot {i} was written");
    }
}

#[test]
fn err_e18_init_negative_count() {
    for count in [-1i32, -2, -10, -11, -1000, -1_000_000] {
        init_diff_expect_count(&format!("E18 count={count}"), count, count);
    }
    // `values` must never be dereferenced when count <= 0: pass a zero-length
    // slice (dangling-but-aligned pointer) and prove neither library touches it.
    let p = libs();
    for count in [-1i32, -100, 0] {
        let mut ac = ResultArray::poisoned();
        let mut ars = ResultArray::poisoned();
        let mut empty: [i32; 0] = [];
        p.c.init(&mut ac, &mut empty, count);
        p.rs.init(&mut ars, &mut empty, count);
        eq_state(&format!("E18 null-ish values count={count}"), &ac, &ars);
    }
}

#[test]
fn err_e19_init_intmin_count() {
    init_diff_expect_count("E19 count=INT32_MIN", i32::MIN, i32::MIN);
    init_diff_expect_count("E19 count=INT32_MIN+1", i32::MIN + 1, i32::MIN + 1);
}

// ===========================================================================
// E20..E22 — process_with_foreach guards
// ===========================================================================

#[test]
fn err_e20_foreach_zero_count() {
    let p = libs();
    for which in 0..4usize {
        let (mut ac, mut ars) = init_both(&[1, 2, 3, 4, 5, 6, 7, 8], 0);
        let ctx = format!("E20 op={which} count=0");
        let (c, r) = (
            p.c.foreach(&mut ac, p.c.own_op(which)),
            p.rs.foreach(&mut ars, p.rs.own_op(which)),
        );
        eq_i32(&ctx, c, r);
        assert_eq!(c, 0, "empty array -> total 0 [{ctx}]");
        eq_state(&format!("{ctx} (state untouched)"), &ac, &ars);
        // and the poison is still there
        let poison = ResultArray::poisoned();
        for i in 0..10 {
            assert_eq!(ac.data[i].value, poison.data[i].value, "slot {i} written");
        }
    }
}

#[test]
fn err_e21_foreach_op_overflow() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 21);
    // multiply_operation on huge values -> int overflow, then *0.75 -> clamp
    for count in 1..=10i32 {
        for it in 0..300u32 {
            let values: Vec<i32> = (0..16)
                .map(|_| match rng.below(3) {
                    0 => i32::MIN,
                    1 => i32::MAX,
                    _ => rng.next_i32() | 0x4000_0000,
                })
                .collect();
            let (mut ac, mut ars) = init_both(&values, count);
            let ctx = format!("E21 count={count} it={it} values={values:?}");
            eq_i32(
                &ctx,
                p.c.foreach(&mut ac, p.c.own_op(1)),
                p.rs.foreach(&mut ars, p.rs.own_op(1)),
            );
            eq_state(&format!("{ctx} (state)"), &ac, &ars);
            // clamping actually happened somewhere
            eq_i32(
                &format!("{ctx} second pass"),
                p.c.foreach(&mut ac, p.c.own_op(1)),
                p.rs.foreach(&mut ars, p.rs.own_op(1)),
            );
            eq_state(&format!("{ctx} second pass (state)"), &ac, &ars);
        }
    }
}

// Ops supplied by the *test* (an arbitrary caller) that drive `total` to wrap.
unsafe extern "C" fn op_ret_int_max(_a: i32, _b: i32, _c: i32, _d: i32) -> i32 {
    i32::MAX
}
unsafe extern "C" fn op_ret_int_min(_a: i32, _b: i32, _c: i32, _d: i32) -> i32 {
    i32::MIN
}
unsafe extern "C" fn op_ret_neg_one(_a: i32, _b: i32, _c: i32, _d: i32) -> i32 {
    -1
}
unsafe extern "C" fn op_identity_a(a: i32, _b: i32, _c: i32, _d: i32) -> i32 {
    a
}

#[test]
fn err_e22_foreach_extreme_op() {
    let p = libs();
    let ops: [(&str, OperationFunc); 4] = [
        ("INT32_MAX", Some(op_ret_int_max)),
        ("INT32_MIN", Some(op_ret_int_min)),
        ("-1", Some(op_ret_neg_one)),
        ("identity(a)", Some(op_identity_a)),
    ];
    let mut rng = Rng::new(SEED ^ 22);
    for (label, op) in ops {
        for count in 0..=10i32 {
            for it in 0..50u32 {
                let values: Vec<i32> = (0..16).map(|_| rng.mixed_i32()).collect();
                let (mut ac, mut ars) = init_both(&values, count);
                for pass in 1..=3 {
                    let ctx = format!("E22 op={label} count={count} it={it} pass={pass}");
                    eq_i32(&ctx, p.c.foreach(&mut ac, op), p.rs.foreach(&mut ars, op));
                    eq_state(&format!("{ctx} (state)"), &ac, &ars);
                }
            }
        }
    }
}

// ===========================================================================
// E23..E25 — compute_weighted_sum guards
// ===========================================================================

#[test]
fn err_e23_weighted_nonpositive_count() {
    let p = libs();
    for count in [0i32, -1, -10, i32::MIN, i32::MIN + 1] {
        let (mut ac, mut ars) = init_both(&[1, 2, 3, 4, 5, 6, 7, 8], count);
        let ctx = format!("E23 count={count}");
        let (c, r) = (p.c.weighted(&mut ac), p.rs.weighted(&mut ars));
        eq_i32(&ctx, c, r);
        assert_eq!(c, 0, "count<=0 -> loop never runs -> 0 [{ctx}]");
        eq_state(&format!("{ctx} (state)"), &ac, &ars);
    }
}

#[test]
fn err_e24_weighted_index0_weight1() {
    let p = libs();
    // count == 1 isolates the i==0 branch: the result must be
    // safe_double_to_int(value * 1 * 0.8), NOT 0 (which a weight of 0 would give).
    for v in [1i32, -1, 100, -100, 12345, i32::MIN, i32::MAX, 0] {
        let values = vec![v; 16];
        let (mut ac, mut ars) = init_both(&values, 1);
        let ctx = format!("E24 count=1 value={v}");
        let (c, r) = (p.c.weighted(&mut ac), p.rs.weighted(&mut ars));
        eq_i32(&ctx, c, r);
        let expect = p.c.sdti(v as f64 * 1.0 * 0.8);
        assert_eq!(c, expect, "i==0 must use weight 1 [{ctx}]");
    }
    // forged arrays too (weight comes from address arithmetic, not from `rank`)
    for count in 1..=10i32 {
        let mut ac = forged(count, &[7, -9, 1_000_000, i32::MIN, i32::MAX]);
        let mut ars = ac;
        eq_i32(
            &format!("E24 forged count={count}"),
            p.c.weighted(&mut ac),
            p.rs.weighted(&mut ars),
        );
        eq_state(&format!("E24 forged count={count} (state)"), &ac, &ars);
    }
}

#[test]
fn err_e25_weighted_overflow() {
    let p = libs();
    let mut rng = Rng::new(SEED ^ 25);
    for count in 1..=10i32 {
        for it in 0..500u32 {
            let values: Vec<i32> = (0..16)
                .map(|_| match rng.below(4) {
                    0 => i32::MIN,
                    1 => i32::MAX,
                    2 => i32::MIN + 1,
                    _ => rng.next_i32(),
                })
                .collect();
            let (mut ac, mut ars) = init_both(&values, count);
            let ctx = format!("E25 count={count} it={it} values={values:?}");
            eq_i32(&ctx, p.c.weighted(&mut ac), p.rs.weighted(&mut ars));
            eq_state(&format!("{ctx} (state)"), &ac, &ars);
        }
    }
}

// ===========================================================================
// E26..E28 — arrayfunc integer edge cases
// ===========================================================================

#[test]
fn err_e26_arrayfunc_overflow_inputs() {
    let p = libs();
    const V: [i32; 8] = [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        -1,
        0,
        1,
        i32::MAX / 2 + 1,
        i32::MAX,
    ];
    for &a in V.iter() {
        for &b in V.iter() {
            for &c in V.iter() {
                for &d in V.iter() {
                    eq_i32(
                        &format!("E26 ({a},{b},{c},{d})"),
                        p.c.arrayfunc(a, b, c, d),
                        p.rs.arrayfunc(a, b, c, d),
                    );
                }
            }
        }
    }
}

#[test]
fn err_e27_arrayfunc_param4_intmin() {
    let p = libs();
    for d in [i32::MIN, i32::MIN + 1, i32::MIN + 2] {
        for (a, b, c) in [(0i32, 0i32, 0i32), (1, 2, 3), (-1, -2, -3), (i32::MAX, i32::MIN, 7)] {
            eq_i32(
                &format!("E27 ({a},{b},{c},{d})"),
                p.c.arrayfunc(a, b, c, d),
                p.rs.arrayfunc(a, b, c, d),
            );
        }
    }
    // sanity: INT_MIN / 2 + 1 in C is -1073741823 (no trap; divisor is 2)
    assert_eq!((i32::MIN / 2).wrapping_add(1), -1_073_741_823);
}

#[test]
fn err_e28_arrayfunc_param4_neg_odd() {
    let p = libs();
    for d in [-1i32, -3, -5, -7, -9, -11, -1001, 1, 3, 5, 1001] {
        for (a, b, c) in [(0i32, 0i32, 0i32), (2, -2, 2), (-7, 11, -13)] {
            eq_i32(
                &format!("E28 ({a},{b},{c},{d})"),
                p.c.arrayfunc(a, b, c, d),
                p.rs.arrayfunc(a, b, c, d),
            );
        }
    }
    // C `/` truncates toward zero, so -3/2 == -1 (not -2)
    assert_eq!(-3i32 / 2, -1);
}

// ===========================================================================
// E29 — "out-of-range enum" analogue: the library has no enum, so every `int`
// parameter of every entry point is fed patterns with no distinguished meaning.
// ===========================================================================

#[test]
fn err_e29_no_enum_all_int_patterns() {
    let p = libs();
    const PAT: [i32; 10] = [
        i32::MIN,
        i32::MIN + 1,
        -65537,
        -256,
        -1,
        0,
        1,
        256,
        i32::MAX - 1,
        i32::MAX,
    ];

    for &x in PAT.iter() {
        for &y in PAT.iter() {
            // every scalar entry point
            eq_i32(&format!("E29 add({x},{y})"), p.c.add(x, y, x, y), p.rs.add(x, y, x, y));
            eq_i32(&format!("E29 mul({x},{y})"), p.c.mul(x, y, x, y), p.rs.mul(x, y, x, y));
            eq_i32(&format!("E29 sub({x},{y})"), p.c.sub(x, y, x, y), p.rs.sub(x, y, x, y));
            if !(x == i32::MIN && y == -1) {
                eq_i32(
                    &format!("E29 mod({x},{y})"),
                    p.c.modulo(x, y, x, y),
                    p.rs.modulo(x, y, x, y),
                );
            }
            eq_i32(
                &format!("E29 csv({x}, {y} as f64)"),
                p.c.compute_scaled_value(x, y as f64),
                p.rs.compute_scaled_value(x, y as f64),
            );
            eq_i32(
                &format!("E29 sdti({x} as f64)"),
                p.c.sdti(x as f64),
                p.rs.sdti(x as f64),
            );
            eq_i32(
                &format!("E29 arrayfunc({x},{y},{x},{y})"),
                p.c.arrayfunc(x, y, x, y),
                p.rs.arrayfunc(x, y, x, y),
            );
        }
    }

    // init_result_array's `count` and compare_results_in_array's indices with
    // the same patterns (guard-side already covered; this is the cross-product)
    for &count in PAT.iter() {
        let (mut ac, mut ars) = init_both(&[1, -2, 3, -4, 5, -6, 7, -8, 9, -10, 11, -12], count);
        for &i1 in PAT.iter() {
            for &i2 in PAT.iter() {
                eq_i32(
                    &format!("E29 compare(count={count}, {i1}, {i2})"),
                    p.c.compare(&mut ac, i1, i2),
                    p.rs.compare(&mut ars, i1, i2),
                );
            }
        }
    }
}
