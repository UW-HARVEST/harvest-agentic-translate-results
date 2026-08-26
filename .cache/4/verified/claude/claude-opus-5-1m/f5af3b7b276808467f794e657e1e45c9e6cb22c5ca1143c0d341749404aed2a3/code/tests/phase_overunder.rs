// Differential tests for `overunder` -- CONFIGS.md rows C25-C40 and ERRORS.md
// rows E19-E24 + E26.
//
// `overunder` writes to stdout with libc `printf`, so each call is wrapped in an
// fd-1 redirect and the captured bytes are compared byte-for-byte in addition to
// the `int` return value. fd 1 is process-global, so this binary uses a custom
// single-threaded harness (`harness = false` in Cargo.toml) -- otherwise
// libtest's own progress output interleaves into the capture.
//
// Both implementations are reached only through `dlopen`/`dlsym`.

mod common;
use common::*;

// ===========================================================================
// Phase B -- CONFIGS.md rows C25 - C40
// ===========================================================================

fn cfg_c25_c30_modulo_arms() {
    let mut rng = Rng::for_test("C25-30");
    for rem in 0..6i32 {
        for _ in 0..12 {
            // a > 0 with a % 6 == rem
            let k = rng.range_i32(0, 100_000);
            let a = k.wrapping_mul(6).wrapping_add(rem);
            assert_eq!(a % 6, rem);
            let b = rng.range_i32(-100_000, 100_000);
            let c = rng.range_i32(-100_000, 100_000);
            let d = rng.range_i32(-100_000, 100_000);
            diff_overunder(a, b, c, d, &format!("C{}-rem{rem}", 25 + rem));
        }
    }
}

fn cfg_c31_negative_modulo_arms() {
    let mut rng = Rng::for_test("C31");
    for rem in 1..6i32 {
        for _ in 0..12 {
            let k = rng.range_i32(0, 100_000);
            let a = -(k.wrapping_mul(6).wrapping_add(rem));
            assert_eq!(a % 6, -rem, "C `%` truncates toward zero");
            diff_overunder(
                a,
                rng.range_i32(-50_000, 50_000),
                rng.range_i32(-50_000, 50_000),
                rng.range_i32(-50_000, 50_000),
                &format!("C31-rem-{rem}"),
            );
        }
    }
    // a negative but divisible by 6 -> `case 0` arm, not `default`
    diff_overunder(-6, 5, -7, 8, "C31-neg-div6");
    diff_overunder(-600, -5, 7, -8, "C31-neg-div6b");
}

fn cfg_c32_all_zero() {
    diff_overunder(0, 0, 0, 0, "C32");
}

fn cfg_c33_small_positive() {
    let mut rng = Rng::for_test("C33");
    for _ in 0..200 {
        diff_overunder(
            rng.range_i32(0, 64),
            rng.range_i32(0, 64),
            rng.range_i32(0, 64),
            rng.range_i32(0, 64),
            "C33",
        );
    }
    for a in 0..7 {
        for b in 0..3 {
            diff_overunder(a, b, a + b, a * b, "C33-exhaustive");
        }
    }
}

fn cfg_c34_sign_cross_product() {
    let mut rng = Rng::for_test("C34");
    for mask in 0..16u32 {
        for _ in 0..8 {
            let mut v = [
                rng.range_i32(1, 10_000),
                rng.range_i32(1, 10_000),
                rng.range_i32(1, 10_000),
                rng.range_i32(1, 10_000),
            ];
            for i in 0..4 {
                if mask & (1 << i) != 0 {
                    v[i] = -v[i];
                }
            }
            diff_overunder(v[0], v[1], v[2], v[3], &format!("C34-mask{mask:04b}"));
        }
    }
}

fn cfg_c35_sqrt_no_overflow() {
    let mut rng = Rng::for_test("C35");
    for _ in 0..200 {
        // |a|, |d| <= 32767 so d*d + a*a <= INT_MAX : sqrt of a non-negative value
        let a = rng.range_i32(-32767, 32767);
        let d = rng.range_i32(-32767, 32767);
        assert!((d as i64) * (d as i64) + (a as i64) * (a as i64) <= i32::MAX as i64);
        diff_overunder(a, rng.next_i32(), rng.next_i32(), d, "C35");
    }
    // exact boundary: 46340^2 = 2147395600 still fits in int
    diff_overunder(0, 1, 2, 46340, "C35-boundary");
    diff_overunder(46340, 1, 2, 0, "C35-boundary2");
    diff_overunder(-46340, 1, 2, 0, "C35-boundary3");
}

fn cfg_c36_sqrt_overflow_both_signs() {
    let mut rng = Rng::for_test("C36");
    let mut saw_negative = false;
    let mut saw_positive = false;
    for _ in 0..250 {
        let a = if rng.next_bool() {
            rng.range_i32(46341, i32::MAX)
        } else {
            rng.range_i32(i32::MIN, -46341)
        };
        let d = if rng.next_bool() {
            rng.range_i32(46341, i32::MAX)
        } else {
            rng.range_i32(i32::MIN, -46341)
        };
        if d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a)) < 0 {
            saw_negative = true;
        } else {
            saw_positive = true;
        }
        diff_overunder(a, rng.next_i32(), rng.next_i32(), d, "C36");
    }
    assert!(
        saw_negative && saw_positive,
        "both wrapped-sum signs must be exercised (neg={saw_negative}, pos={saw_positive})"
    );
    diff_overunder(0, 0, 0, 46341, "C36-nan");
    diff_overunder(46341, 0, 0, 0, "C36-nan2");
    diff_overunder(i32::MIN, 0, 0, i32::MIN, "C36-nan3");
}

fn cfg_c37_internal_clamps() {
    let mut rng = Rng::for_test("C37");
    // a * 1.5 > INT_MAX  <=>  a >= 1431655765   (1431655765 * 1.5 = 2147483647.5)
    // b * 2.7 > INT_MAX  <=>  b >=  795364315
    // a * 1.5 < INT_MIN  <=>  a <= -1431655766 (the low threshold is asymmetric)
    // b * 2.7 > INT_MAX  <=>  b >=   795364314
    // b * 2.7 < INT_MIN  <=>  b <=  -795364315
    let shapes: [(i32, i32); 4] = [
        (1_431_655_765, 795_364_315),
        (-1_431_655_766, -795_364_315),
        (1_431_655_765, -795_364_315),
        (-1_431_655_766, 795_364_315),
    ];
    for (a0, b0) in shapes {
        for _ in 0..15 {
            let jitter = rng.range_i32(0, 1000);
            let a = if a0 > 0 {
                a0.saturating_add(jitter)
            } else {
                a0.saturating_sub(jitter)
            };
            let b = if b0 > 0 {
                b0.saturating_add(jitter)
            } else {
                b0.saturating_sub(jitter)
            };
            diff_overunder(a, b, rng.next_i32(), rng.next_i32(), "C37");
        }
    }
    for a in [1_431_655_764, 1_431_655_765, -1_431_655_765, -1_431_655_766] {
        for b in [795_364_314, 795_364_315, -795_364_315, -795_364_316] {
            diff_overunder(a, b, 3, 4, "C37-threshold");
        }
    }
}

fn cfg_c38_c_division_and_ptr_shapes() {
    for c in [
        0,
        1,
        -1,
        3,
        -3,
        4,
        -4,
        i32::MAX,
        i32::MIN,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        1_073_741_823,
        1_073_741_824,
        -1_073_741_824,
        -1_073_741_825,
    ] {
        diff_overunder(7, 11, c, 13, "C38");
    }
}

fn cfg_c39_extreme_corners() {
    let extremes = [i32::MIN, i32::MAX];
    for &a in &extremes {
        for &b in &extremes {
            for &c in &extremes {
                for &d in &extremes {
                    diff_overunder(a, b, c, d, "C39");
                }
            }
        }
    }
    for &a in &[i32::MIN, i32::MIN + 1, i32::MAX - 1, i32::MAX] {
        for &b in &[i32::MIN, i32::MAX] {
            diff_overunder(a, b, i32::MAX, i32::MIN, "C39-near");
            diff_overunder(a, b, i32::MIN, i32::MAX, "C39-near2");
        }
    }
}

fn cfg_c40_fully_random_with_stdout() {
    let mut rng = Rng::for_test("C40");
    for _ in 0..1500 {
        diff_overunder_quiet(rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
    }
    let corners = i32_corners();
    for _ in 0..1500 {
        let pick = |r: &mut Rng| corners[(r.next_u32() as usize) % corners.len()];
        let a = pick(&mut rng);
        let b = pick(&mut rng);
        let c = pick(&mut rng);
        let d = pick(&mut rng);
        diff_overunder(a, b, c, d, "C40-corners");
    }
}

// ===========================================================================
// Phase C -- ERRORS.md rows E19 - E24, E26
// ===========================================================================

/// E19: `d*d + a*a` overflows to a negative int, so `sqrt(negative)` yields NaN
/// which `safe_double_to_int` maps through its `isnan` arm to 0.
fn err_e19_sqrt_domain_negative() {
    let mut rng = Rng::for_test("E19");
    let mut checked = 0usize;
    let mut tried = 0usize;
    while checked < 40 && tried < 20_000 {
        tried += 1;
        let a = rng.next_i32();
        let d = rng.next_i32();
        let wrapped = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
        if wrapped >= 0 {
            continue;
        }
        checked += 1;
        // Confirm the C really does print `conv4 == 0` for this shape.
        let (_, out) = {
            let _g = capture_lock();
            let (c, _) = both();
            capture_stdout(|| unsafe { (c.overunder)(a, 0, 0, d) })
        };
        let text = String::from_utf8_lossy(&out).to_string();
        let line = text
            .lines()
            .find(|l| l.starts_with("Converted values:"))
            .unwrap_or("");
        assert!(
            line.ends_with(", 0"),
            "E19: expected conv4 == 0 for a={a} d={d} (wrapped={wrapped}), got `{line}`"
        );
        diff_overunder(a, 0, 0, d, "E19");
    }
    assert!(checked >= 40, "E19 only found {checked} negative-sum cases");

    // Deterministic minimal witnesses.
    for (a, d) in [(46341, 0), (0, 46341), (i32::MIN, i32::MIN), (65536, 65536)] {
        diff_overunder(a, 1, 2, d, "E19-witness");
    }
}

/// E20: `a % 6` negative -> `process_with_fallthrough` `default` arm -> -1.
fn err_e20_negative_modulo_default() {
    let mut rng = Rng::for_test("E20");
    for _ in 0..40 {
        let a = loop {
            let v = rng.range_i32(i32::MIN, -1);
            if v % 6 != 0 {
                break v;
            }
        };
        let b = rng.next_i32();
        // The C must print the sentinel -1 regardless of `b`.
        let (_, out) = {
            let _g = capture_lock();
            let (c, _) = both();
            capture_stdout(|| unsafe { (c.overunder)(a, b, 0, 0) })
        };
        let text = String::from_utf8_lossy(&out).to_string();
        assert!(
            text.contains("Switch fall-through result: -1\n"),
            "E20: expected sentinel -1 for a={a} b={b}, got:\n{text}"
        );
        diff_overunder(a, b, rng.next_i32(), rng.next_i32(), "E20");
    }
}

/// E21: `a == INT_MIN` and neighbours -- `INT_MIN % 6 == -2`, `a*a`, `a*1.5`,
/// `a+b` all at the extreme.
fn err_e21_extreme_int_args() {
    assert_eq!(i32::MIN % 6, -2, "C `%` semantics assumption");
    let mut rng = Rng::for_test("E21");
    for a in [i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MIN + 6] {
        for _ in 0..8 {
            diff_overunder(a, rng.next_i32(), rng.next_i32(), rng.next_i32(), "E21");
        }
        diff_overunder(a, i32::MIN, i32::MIN, i32::MIN, "E21-all-min");
        diff_overunder(a, i32::MAX, i32::MAX, i32::MAX, "E21-max");
    }
}

/// E22: the `safe_double_to_int` clamps reached from inside `overunder`.
fn err_e22_internal_clamp() {
    // conv1 clamps high  <=>  a * 1.5 >  INT_MAX  <=>  a >=  1431655765
    //                                                   (1431655765*1.5 = 2147483647.5)
    // conv1 clamps low   <=>  a * 1.5 <  INT_MIN  <=>  a <= -1431655766
    //   note -1431655765*1.5 == -2147483647.5 is NOT < INT_MIN, so it truncates
    //   to -2147483647 instead of clamping -- the thresholds are asymmetric.
    for a in [1_431_655_765, i32::MAX, -1_431_655_766, i32::MIN] {
        let (_, out) = {
            let _g = capture_lock();
            let (c, _) = both();
            capture_stdout(|| unsafe { (c.overunder)(a, 0, 0, 0) })
        };
        let text = String::from_utf8_lossy(&out).to_string();
        let line = text
            .lines()
            .find(|l| l.starts_with("Converted values:"))
            .unwrap_or("")
            .to_string();
        let want = if a > 0 { "2147483647" } else { "-2147483648" };
        assert!(
            line.contains(want),
            "E22: expected conv1 clamped to {want} for a={a}, got `{line}`"
        );
        diff_overunder(a, 0, 0, 0, "E22-conv1");
    }
    // conv2 clamps high  <=>  b * 2.7 >  INT_MAX  <=>  b >=  795364314
    // conv2 clamps low    <=>  b * 2.7 <  INT_MIN  <=>  b <= -795364315
    for b in [795_364_314, 795_364_315, i32::MAX, -795_364_315, i32::MIN] {
        diff_overunder(1, b, 0, 0, "E22-conv2");
    }
    // ...and one step inside each threshold, where truncation applies instead
    for (a, b) in [
        (1_431_655_764, 795_364_313),
        (-1_431_655_765, -795_364_314),
    ] {
        diff_overunder(a, b, 0, 0, "E22-just-inside");
    }
    // both at once
    diff_overunder(i32::MAX, i32::MAX, 0, 0, "E22-both-high");
    diff_overunder(i32::MIN, i32::MIN, 0, 0, "E22-both-low");
    // and the "Overflow/Underflow protected conversion" lines (1e15 / -1e15) are
    // printed unconditionally on every call, so they are covered everywhere.
}

/// E23: `handle_pointer_operations(c)` overflow reached through `overunder`.
fn err_e23_ptr_op_overflow() {
    for c in [
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MAX,
        i32::MIN,
        i32::MIN / 2,
        i32::MIN / 2 - 1,
        1_073_741_774, // c*2 + 100 == INT_MAX + 1 exactly
        1_073_741_773,
    ] {
        diff_overunder(1, 2, c, 3, "E23");
    }
}

/// E24: the running `total` sum overflows `int` repeatedly.
fn err_e24_total_overflow() {
    let mut rng = Rng::for_test("E24");
    for _ in 0..60 {
        // large magnitudes in every slot force repeated wraparound in `total`
        let a = rng.range_i32(1_000_000_000, i32::MAX);
        let b = rng.range_i32(1_000_000_000, i32::MAX);
        let c = rng.range_i32(1_000_000_000, i32::MAX);
        let d = rng.range_i32(1_000_000_000, i32::MAX);
        diff_overunder(a, b, c, d, "E24-high");
        diff_overunder(-a, -b, -c, -d, "E24-low");
        diff_overunder(a, -b, c, -d, "E24-mixed");
    }
}

/// E26: the `sizeof(label) - 1` buffer bound -- `%s` must always print exactly
/// `Source`, never trailing garbage, for every input.
fn err_e26_label_buffer_bound() {
    let mut rng = Rng::for_test("E26");
    let (c, r) = both();
    for _ in 0..60 {
        let (a, b, cc, d) = (
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
        let _g = capture_lock();
        let (_, cout) = capture_stdout(|| unsafe { (c.overunder)(a, b, cc, d) });
        let (_, rout) = capture_stdout(|| unsafe { (r.overunder)(a, b, cc, d) });
        for (who, out) in [("C", &cout), ("Rust", &rout)] {
            let text = String::from_utf8_lossy(out).to_string();
            let line = text
                .lines()
                .find(|l| l.starts_with("Copied block:"))
                .unwrap_or("")
                .to_string();
            assert!(
                line.ends_with("label=Source"),
                "E26 [{who}]: label must be exactly `Source` (a={a}); got `{line}`"
            );
        }
        assert_eq!(cout, rout, "E26 stdout divergence at ({a},{b},{cc},{d})");
    }
}

/// Repeated-invocation check: no hidden global state in either library.
fn misc_repeated_invocation_stability() {
    let (c, r) = both();
    let args = (12345, -6789, 42, -99);
    let mut first_c: Option<(i32, Vec<u8>)> = None;
    let mut first_r: Option<(i32, Vec<u8>)> = None;
    for i in 0..25 {
        let _g = capture_lock();
        let (cv, cout) = capture_stdout(|| unsafe { (c.overunder)(args.0, args.1, args.2, args.3) });
        let (rv, rout) = capture_stdout(|| unsafe { (r.overunder)(args.0, args.1, args.2, args.3) });
        assert_eq!(cv, rv, "iteration {i}: return value");
        assert_eq!(cout, rout, "iteration {i}: stdout");
        match &first_c {
            None => first_c = Some((cv, cout)),
            Some(f) => assert_eq!(*f, (cv, cout), "C not deterministic at iteration {i}"),
        }
        match &first_r {
            None => first_r = Some((rv, rout)),
            Some(f) => assert_eq!(*f, (rv, rout), "Rust not deterministic at iteration {i}"),
        }
    }
}

// ===========================================================================

fn main() {
    let mut t = Runner::from_args();

    // Phase B -- CONFIGS.md
    t.run("cfg_c25_c30_modulo_arms", cfg_c25_c30_modulo_arms);
    t.run("cfg_c31_negative_modulo_arms", cfg_c31_negative_modulo_arms);
    t.run("cfg_c32_all_zero", cfg_c32_all_zero);
    t.run("cfg_c33_small_positive", cfg_c33_small_positive);
    t.run("cfg_c34_sign_cross_product", cfg_c34_sign_cross_product);
    t.run("cfg_c35_sqrt_no_overflow", cfg_c35_sqrt_no_overflow);
    t.run(
        "cfg_c36_sqrt_overflow_both_signs",
        cfg_c36_sqrt_overflow_both_signs,
    );
    t.run("cfg_c37_internal_clamps", cfg_c37_internal_clamps);
    t.run(
        "cfg_c38_c_division_and_ptr_shapes",
        cfg_c38_c_division_and_ptr_shapes,
    );
    t.run("cfg_c39_extreme_corners", cfg_c39_extreme_corners);
    t.run(
        "cfg_c40_fully_random_with_stdout",
        cfg_c40_fully_random_with_stdout,
    );

    // Phase C -- ERRORS.md
    t.run("err_e19_sqrt_domain_negative", err_e19_sqrt_domain_negative);
    t.run(
        "err_e20_negative_modulo_default",
        err_e20_negative_modulo_default,
    );
    t.run("err_e21_extreme_int_args", err_e21_extreme_int_args);
    t.run("err_e22_internal_clamp", err_e22_internal_clamp);
    t.run("err_e23_ptr_op_overflow", err_e23_ptr_op_overflow);
    t.run("err_e24_total_overflow", err_e24_total_overflow);
    t.run("err_e26_label_buffer_bound", err_e26_label_buffer_bound);

    t.run(
        "misc_repeated_invocation_stability",
        misc_repeated_invocation_stability,
    );

    t.finish();
}
