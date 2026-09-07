// Phase C -- error / rejection-path differential tests, one test per ERRORS.md row.
// Both libraries are driven exclusively through their `.so` exports.

mod common;
use common::*;

// ---------------------------------------------------------------- row 1
#[test]
fn err_01_classify_mode_unknown() {
    let l = libs();
    let cases: &[&str] = &[
        "STANDARD",
        "Standard",
        "standar",
        "standardx",
        "enhance",
        "enhancedd",
        "TURBO",
        "turb",
        "turboo",
        "extrem",
        "extremee",
        "unknown",
        "mode",
        "0",
        " standard",
        "standard ",
        "\tstandard",
    ];
    for &s in cases {
        let buf = cstring(s);
        let (c, r) = (l.c.classify_mode_bytes(&buf), l.rust.classify_mode_bytes(&buf));
        eq_i32(format!("classify_mode({s:?})"), c, r);
        assert_eq!(c, 0x00, "C should reject {s:?} with 0x00");
    }
}

// ---------------------------------------------------------------- row 2
#[test]
fn err_02_classify_mode_empty() {
    let l = libs();
    let buf = cstring("");
    let (c, r) = (l.c.classify_mode_bytes(&buf), l.rust.classify_mode_bytes(&buf));
    eq_i32("classify_mode(\"\")", c, r);
    assert_eq!(c, 0x00);
}

// ---------------------------------------------------------------- row 3
#[test]
fn err_03_classify_mode_prefix_superstring() {
    let l = libs();
    for m in MODES {
        // every strict prefix
        for len in 0..m.len() {
            let buf = cstring(&m[..len]);
            let (c, r) = (l.c.classify_mode_bytes(&buf), l.rust.classify_mode_bytes(&buf));
            eq_i32(format!("classify_mode(prefix {:?})", &m[..len]), c, r);
            assert_eq!(c, 0x00, "prefix {:?} must not match", &m[..len]);
        }
        // superstrings
        for suffix in ["x", "0", " ", "\u{7f}", "standard"] {
            let buf = cstring(&format!("{m}{suffix}"));
            let (c, r) = (l.c.classify_mode_bytes(&buf), l.rust.classify_mode_bytes(&buf));
            eq_i32(format!("classify_mode({m:?}+{suffix:?})"), c, r);
            assert_eq!(c, 0x00);
        }
        // embedded NUL: only the bytes before the NUL are compared, so this DOES
        // match -- verify both agree on that too.
        let mut buf = m.as_bytes().to_vec();
        buf.push(0);
        buf.extend_from_slice(b"junk\0");
        eq_i32(
            format!("classify_mode({m:?} + NUL + junk)"),
            l.c.classify_mode_bytes(&buf),
            l.rust.classify_mode_bytes(&buf),
        );
    }
}

// ---------------------------------------------------------------- row 4
#[test]
fn err_04_classify_mode_high_bytes() {
    let l = libs();
    // strcmp compares as `unsigned char`; feed bytes >= 0x80 in every position of
    // each literal to catch a sign-extension mistranslation.
    for m in MODES {
        for pos in 0..m.len() {
            for hb in [0x80u8, 0xA0, 0xFE, 0xFF] {
                let mut bytes = m.as_bytes().to_vec();
                bytes[pos] = hb;
                bytes.push(0);
                let (c, r) = (l.c.classify_mode_bytes(&bytes), l.rust.classify_mode_bytes(&bytes));
                eq_i32(format!("classify_mode({m:?} byte {pos} -> 0x{hb:02X})"), c, r);
                assert_eq!(c, 0x00);
            }
        }
    }
    for bytes in [
        vec![0xFFu8, 0],
        vec![0x80, 0x81, 0x82, 0],
        vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0],
    ] {
        eq_i32(
            format!("classify_mode({bytes:02X?})"),
            l.c.classify_mode_bytes(&bytes),
            l.rust.classify_mode_bytes(&bytes),
        );
    }
}

// ---------------------------------------------------------------- row 5
#[test]
fn err_05_classify_mode_null_ptr_both_crash() {
    let l = libs();
    let c_outcome = run_isolated(|| {
        let _ = l.c.classify_mode(std::ptr::null());
    });
    let r_outcome = run_isolated(|| {
        let _ = l.rust.classify_mode(std::ptr::null());
    });
    assert_eq!(
        c_outcome, r_outcome,
        "classify_mode(NULL): C terminated as {c_outcome:?} but Rust as {r_outcome:?}"
    );
    // Document the actual behaviour: glibc strcmp faults on a null argument.
    assert_eq!(
        c_outcome,
        Child::Signaled(libc::SIGSEGV),
        "expected SIGSEGV from strcmp(NULL, ..), got {c_outcome:?}"
    );
}

// ---------------------------------------------------------------- row 6
#[test]
fn err_06_apply_multiplier_out_of_range_level() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 106);
    let mut levels: Vec<i32> = vec![-1, 5, 6, 7, 100, -100, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
    for _ in 0..2048 {
        let v = rng.next_i32();
        if !(0..=4).contains(&v) {
            levels.push(v);
        }
    }
    let bases = [0i32, 0xA0, -1, i32::MIN, i32::MAX, 12345];
    for &level in &levels {
        for &base in &bases {
            let (c, r) = (l.c.apply_multiplier(base, level), l.rust.apply_multiplier(base, level));
            eq_i32(format!("apply_multiplier({base}, {level})"), c, r);
            assert_eq!(
                c, 0xDEAD,
                "C default branch must yield 0xDEAD for level {level}, base {base}"
            );
        }
    }
}

// ---------------------------------------------------------------- row 7
#[test]
fn err_07_apply_multiplier_one_past_range() {
    let l = libs();
    for level in [-1i32, 5] {
        for base in [0i32, 0xA0, i32::MIN, i32::MAX] {
            let (c, r) = (l.c.apply_multiplier(base, level), l.rust.apply_multiplier(base, level));
            eq_i32(format!("apply_multiplier({base}, {level})"), c, r);
            assert_eq!(c, 0xDEAD);
        }
    }
    // and the in-range endpoints, which must NOT be 0xDEAD-by-accident
    for level in [0i32, 4] {
        let (c, r) = (l.c.apply_multiplier(0xA0, level), l.rust.apply_multiplier(0xA0, level));
        eq_i32(format!("apply_multiplier(0xA0, {level})"), c, r);
        assert_ne!(c, 0xDEAD);
    }
}

// ---------------------------------------------------------------- row 8
#[test]
fn err_08_apply_multiplier_signed_overflow() {
    let l = libs();
    // Each level accumulates a different total; probe bases that overflow at
    // every individual `+=` step.
    let totals = [0x05i32, 0x21, 0x9F, 0x14A, 0x249];
    let mut bases: Vec<i32> = vec![i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1];
    for t in totals {
        bases.push(i32::MAX.wrapping_sub(t));
        bases.push(i32::MAX.wrapping_sub(t).wrapping_add(1));
        bases.push(i32::MIN.wrapping_sub(t));
        bases.push(i32::MIN.wrapping_sub(t).wrapping_sub(1));
    }
    for &base in &bases {
        for level in 0..=4 {
            eq_i32(
                format!("apply_multiplier({base}, {level}) [overflow]"),
                l.c.apply_multiplier(base, level),
                l.rust.apply_multiplier(base, level),
            );
        }
    }
}

// ---------------------------------------------------------------- row 9
#[test]
fn err_09_convert_time_factor_overflow() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 109);
    let mut vals: Vec<f64> = vec![
        1.0, -1.0, 1e8, -1e8, 1e30, -1e30, 1e300, -1e300, 2.2e-3, -2.2e-3, 1e-2, -1e-2,
    ];
    for _ in 0..2048 {
        let sign = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        let mantissa = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        vals.push(sign * (1.0 + mantissa) * 2f64.powi(rng.below(200) as i32));
    }
    for &v in &vals {
        let (c, r) = (l.c.convert_time_factor(v), l.rust.convert_time_factor(v));
        eq_i32(format!("convert_time_factor({v:e}) [overflow]"), c, r);
        assert_eq!(
            c,
            i32::MIN,
            "C must yield the integer-indefinite sentinel for {v:e}"
        );
    }
}

// ---------------------------------------------------------------- row 10
#[test]
fn err_10_convert_time_factor_nan() {
    let l = libs();
    let nans = [
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0001),
        f64::from_bits(0xFFF8_0000_0000_0001),
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFFF_FFFF_FFFF_FFFF),
    ];
    for &v in &nans {
        let (c, r) = (l.c.convert_time_factor(v), l.rust.convert_time_factor(v));
        eq_i32(format!("convert_time_factor(NaN bits 0x{:016X})", v.to_bits()), c, r);
        assert_eq!(c, i32::MIN);
    }
}

// ---------------------------------------------------------------- row 11
#[test]
fn err_11_convert_time_factor_inf() {
    let l = libs();
    for &v in &[f64::INFINITY, f64::NEG_INFINITY, f64::MAX, f64::MIN] {
        let (c, r) = (l.c.convert_time_factor(v), l.rust.convert_time_factor(v));
        eq_i32(format!("convert_time_factor({v:e})"), c, r);
        assert_eq!(c, i32::MIN);
    }
}

// ---------------------------------------------------------------- row 12
#[test]
fn err_12_convert_time_factor_zero_denormal() {
    let l = libs();
    let vals = [
        0.0f64,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1), // smallest denormal
        f64::from_bits(0x8000_0000_0000_0001),
        1e-320,
        -1e-320,
        1e-13,
        -1e-13,
    ];
    for &v in &vals {
        let (c, r) = (l.c.convert_time_factor(v), l.rust.convert_time_factor(v));
        eq_i32(format!("convert_time_factor({v:e}) [zero/denormal]"), c, r);
        assert_eq!(c, 0, "expected truncation to 0 for {v:e}");
    }
}

// ---------------------------------------------------------------- row 13
#[test]
fn err_13_convert_time_factor_int_boundaries() {
    let l = libs();
    // factor = target / 1e12 so that factor*1e12 lands near the int limits.
    let targets: [f64; 12] = [
        2147483646.0,
        2147483647.0,
        2147483647.5,
        2147483648.0,
        2147483649.0,
        4294967296.0,
        -2147483647.0,
        -2147483648.0,
        -2147483648.5,
        -2147483649.0,
        -2147483650.0,
        -4294967296.0,
    ];
    for &t in &targets {
        let factor = t / 1e12;
        eq_i32(
            format!("convert_time_factor({factor:e}) [target {t}]"),
            l.c.convert_time_factor(factor),
            l.rust.convert_time_factor(factor),
        );
    }
    // Also hit the exact boundaries by scanning every ULP around them.
    for base in [2147483647.0f64, -2147483648.0] {
        let mut f = base / 1e12;
        for _ in 0..64 {
            eq_i32(
                format!("convert_time_factor ULP scan {f:e}"),
                l.c.convert_time_factor(f),
                l.rust.convert_time_factor(f),
            );
            f = f64::from_bits(f.to_bits().wrapping_add(1));
        }
        let mut f = base / 1e12;
        for _ in 0..64 {
            eq_i32(
                format!("convert_time_factor ULP scan down {f:e}"),
                l.c.convert_time_factor(f),
                l.rust.convert_time_factor(f),
            );
            f = f64::from_bits(f.to_bits().wrapping_sub(1));
        }
    }
}

// ---------------------------------------------------------------- row 14
#[test]
fn err_14_convert_negative_overflow_underflow() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 114);
    let mut vals: Vec<f64> = vec![1.0, -1.0, 1e-5, -1e-5, 1e7, -1e7, 1e200, -1e200];
    for _ in 0..2048 {
        let sign = if rng.next_u64() & 1 == 0 { 1.0 } else { -1.0 };
        let mantissa = (rng.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        vals.push(sign * (1.0 + mantissa) * 2f64.powi(rng.below(200) as i32));
    }
    for &v in &vals {
        let (c, r) = (
            l.c.convert_negative_overflow(v),
            l.rust.convert_negative_overflow(v),
        );
        eq_i32(format!("convert_negative_overflow({v:e}) [underflow]"), c, r);
        assert_eq!(c, i32::MIN, "C must yield the sentinel for {v:e}");
    }
}

// ---------------------------------------------------------------- row 15
#[test]
fn err_15_convert_negative_overflow_nan_inf() {
    let l = libs();
    let vals = [
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(0x7FF8_0000_0000_0001),
        f64::from_bits(0xFFF0_0000_0000_0001),
        f64::MAX,
        f64::MIN,
    ];
    for &v in &vals {
        let (c, r) = (
            l.c.convert_negative_overflow(v),
            l.rust.convert_negative_overflow(v),
        );
        eq_i32(
            format!("convert_negative_overflow(bits 0x{:016X})", v.to_bits()),
            c,
            r,
        );
        assert_eq!(c, i32::MIN);
    }
    // ULP scan around the int limits for this function too.
    for base in [2147483647.0f64, -2147483648.0] {
        let mut v = base / -1e15;
        for _ in 0..64 {
            eq_i32(
                format!("convert_negative_overflow ULP {v:e}"),
                l.c.convert_negative_overflow(v),
                l.rust.convert_negative_overflow(v),
            );
            v = f64::from_bits(v.to_bits().wrapping_add(1));
        }
    }
}

// ---------------------------------------------------------------- row 16
#[test]
fn err_16_convert_negative_overflow_signed_zero() {
    let l = libs();
    for &v in &[
        0.0f64,
        -0.0,
        f64::from_bits(1),
        f64::from_bits(0x8000_0000_0000_0001),
        1e-320,
        -1e-320,
        1e-16,
        -1e-16,
    ] {
        let (c, r) = (
            l.c.convert_negative_overflow(v),
            l.rust.convert_negative_overflow(v),
        );
        eq_i32(format!("convert_negative_overflow({v:e}) [signed zero]"), c, r);
        assert_eq!(c, 0);
    }
}

// ---------------------------------------------------------------- row 17
#[test]
fn err_17_get_modified_time_int_overflow() {
    let l = libs();
    let days = [
        i32::MIN,
        i32::MIN + 1,
        -24856,
        -24855,
        24855,
        24856,
        i32::MAX,
        i32::MAX - 1,
        1000000,
        -1000000,
    ];
    for &d in &days {
        for &h in &[0i32, 1, -1, 23] {
            eq_i64(
                format!("get_modified_time({d}, {h}) [days overflow]"),
                l.c.get_modified_time(d, h),
                l.rust.get_modified_time(d, h),
            );
        }
    }
}

// ---------------------------------------------------------------- row 18
#[test]
fn err_18_get_modified_time_hours_overflow() {
    let l = libs();
    let hours = [
        i32::MIN,
        i32::MIN + 1,
        -596524,
        -596523,
        596523,
        596524,
        i32::MAX,
        i32::MAX - 1,
    ];
    for &h in &hours {
        for &d in &[0i32, 1, -1, 7] {
            eq_i64(
                format!("get_modified_time({d}, {h}) [hours overflow]"),
                l.c.get_modified_time(d, h),
                l.rust.get_modified_time(d, h),
            );
        }
    }
}

// ---------------------------------------------------------------- row 19
#[test]
fn err_19_get_modified_time_sum_overflow() {
    let l = libs();
    // days*86400 and hours*3600 each fit, but their sum does not.
    let pairs = [
        (24000i32, 500000i32),
        (24855, 596523),
        (-24855, -596523),
        (20000, 596523),
        (24855, 100000),
        (-24000, -500000),
        (24855, -596523),
        (-24855, 596523),
    ];
    for &(d, h) in &pairs {
        eq_i64(
            format!("get_modified_time({d}, {h}) [sum overflow]"),
            l.c.get_modified_time(d, h),
            l.rust.get_modified_time(d, h),
        );
    }
}

// ---------------------------------------------------------------- row 20
#[test]
fn err_20_hash_time_value_negative_and_overflow() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 120);
    let mut vals: Vec<i64> = vec![
        -1,
        i64::MIN,
        i64::MIN + 1,
        i64::MAX,
        -2,
        -256,
        -0x8000_0000,
        -0x1_0000_0000_0000_0000i128 as i64,
    ];
    for _ in 0..4096 {
        vals.push(-(rng.next_i64().abs().max(1)));
    }
    for &t in &vals {
        eq_i32(
            format!("hash_time_value({t}) [negative/overflow]"),
            l.c.hash_time_value(t),
            l.rust.hash_time_value(t),
        );
    }
}

// ---------------------------------------------------------------- row 21
#[test]
fn err_21_hash_time_value_zero() {
    let l = libs();
    let (c, r) = (l.c.hash_time_value(0), l.rust.hash_time_value(0));
    eq_i32("hash_time_value(0)", c, r);
    assert!(c >= 0, "result is masked with 0x7FFFFFFF so it is non-negative");
}

// ---------------------------------------------------------------- row 22
#[test]
fn err_22_modeselect_negative_selector_documented() {
    let l = libs();
    // `mode_selector % 4` is negative in C, so `modes[mode_index]` reads before
    // the array: undefined behaviour. The loaded pointer is stack garbage, so the
    // outcome depends on the caller's frame, the optimisation level and the ABI --
    // the *C* library itself is not self-consistent here (linked into a small
    // driver it faults with SIGSEGV for -1/-2/-3, while inside this test process
    // it happens to survive). Therefore no equality can be required; the test
    // records the outcomes and only asserts that neither library hangs and that
    // the *in-bounds* negative input (INT_MIN, whose `% 4` is 0) matches exactly.
    for ms in [-1i32, -2, -3, -4, -5, -7, i32::MIN + 1] {
        let co = run_isolated(|| {
            let _ = l.c.modeselect(ms, 0, 0, 0);
        });
        let ro = run_isolated(|| {
            let _ = l.rust.modeselect(ms, 0, 0, 0);
        });
        eprintln!("modeselect({ms}, 0, 0, 0) [UB, OOB read]: C {co:?}, Rust {ro:?}");
        for (who, o) in [("C", &co), ("Rust", &ro)] {
            match o {
                // Either a normal return or a memory fault from dereferencing the
                // garbage pointer is an acceptable manifestation of this UB.
                Child::Exited(0) => {}
                Child::Signaled(s) if *s == libc::SIGSEGV || *s == libc::SIGBUS => {}
                other => panic!("{who} modeselect({ms}) terminated unexpectedly: {other:?}"),
            }
        }
    }
    // `INT_MIN % 4 == 0` in C, so INT_MIN is actually an in-bounds selector and
    // its full result MUST match.
    eq_i32(
        "modeselect(INT_MIN, 0, 0, 0)",
        l.c.modeselect(i32::MIN, 0, 0, 0),
        l.rust.modeselect(i32::MIN, 0, 0, 0),
    );
}

// ---------------------------------------------------------------- row 23
#[test]
fn err_23_modeselect_negative_complexity() {
    let l = libs();
    // complexity < 0 => complexity % 5 is negative => apply_multiplier default
    // branch => multiplier 0xDEAD.
    for cx in [-1i32, -2, -3, -4, -5, -6, -100, i32::MIN, i32::MIN + 1] {
        let level = cx % 5;
        if level != 0 {
            assert_eq!(
                l.c.apply_multiplier(0xA0, level),
                0xDEAD,
                "level {level} should hit the default branch"
            );
        }
        for ms in 0..4 {
            eq_i32(
                format!("modeselect({ms}, 0, {cx}, 0) [negative complexity]"),
                l.c.modeselect(ms, 0, cx, 0),
                l.rust.modeselect(ms, 0, cx, 0),
            );
        }
    }
}

// ---------------------------------------------------------------- row 24
#[test]
fn err_24_modeselect_int_min_params() {
    let l = libs();
    // INT_MIN % 4 == 0, so the selector stays in bounds; every other parameter
    // is pushed to INT_MIN in turn and then all at once.
    let cases = [
        (i32::MIN, 0, 0, 0),
        (0, i32::MIN, 0, 0),
        (0, 0, i32::MIN, 0),
        (0, 0, 0, i32::MIN),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MIN, i32::MIN + 1, i32::MIN, i32::MIN + 1),
        (0, i32::MIN + 1, i32::MIN + 1, i32::MIN + 1),
    ];
    for &(a, b, c, d) in &cases {
        eq_i32(
            format!("modeselect({a}, {b}, {c}, {d}) [INT_MIN]"),
            l.c.modeselect(a, b, c, d),
            l.rust.modeselect(a, b, c, d),
        );
    }
}

// ---------------------------------------------------------------- row 25
#[test]
fn err_25_modeselect_int_max_params() {
    let l = libs();
    let cases = [
        (i32::MAX, 0, 0, 0),
        (0, i32::MAX, 0, 0),
        (0, 0, i32::MAX, 0),
        (0, 0, 0, i32::MAX),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MAX - 1, i32::MAX - 1, i32::MAX - 1, i32::MAX - 1),
        (i32::MAX, i32::MIN, i32::MAX, i32::MIN),
    ];
    for &(a, b, c, d) in &cases {
        eq_i32(
            format!("modeselect({a}, {b}, {c}, {d}) [INT_MAX]"),
            l.c.modeselect(a, b, c, d),
            l.rust.modeselect(a, b, c, d),
        );
    }
}
