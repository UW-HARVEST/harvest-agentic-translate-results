// Phase C — error / rejection-path differential tests.
//
// One test per row of ERRORS.md, in the same order, named `err_NN_…`.
// Each test constructs the exact invalid input or condition, calls BOTH shared
// objects, and asserts they reject it the *same* way — the same returned
// sentinel value AND the same diagnostic bytes on stderr, or (for the rows where
// the C validates nothing at all) death by the same signal.

mod common;

use common::*;
use std::ffi::c_int;
use std::ptr;

const DEF_BASE_OFFSET: c_int = 0o100; // 64
const DEF_MULTIPLIER: c_int = 0o12; // 10
const SIGSEGV: i32 = 11;

fn rng_for(tag: &str) -> Rng {
    let mut h: u64 = SEED ^ 0xE770_0000_0000_0001;
    for b in tag.as_bytes() {
        h = (h ^ *b as u64).wrapping_mul(0x100_0000_01B3);
    }
    Rng::new(h)
}

/// Assert that both implementations return exactly `expected` and that the
/// stdout/stderr bytes match each other and the expected diagnostic.
fn expect_parse(
    context: &str,
    env: &[(&str, Option<&str>)],
    name: &str,
    default_val: c_int,
    expected_ret: c_int,
    expected_stderr: &str,
) {
    let (c, r) = both();

    env_config(env);
    let got_c = capture(|| call_parse(c, name, default_val));
    env_config(env);
    let got_r = capture(|| call_parse(r, name, default_val));

    assert_eq!(
        got_c.ret, got_r.ret,
        "[{context}] C and Rust returned different values"
    );
    assert_eq!(
        String::from_utf8_lossy(&got_c.err),
        String::from_utf8_lossy(&got_r.err),
        "[{context}] C and Rust wrote different stderr"
    );
    assert_eq!(
        String::from_utf8_lossy(&got_c.out),
        String::from_utf8_lossy(&got_r.out),
        "[{context}] C and Rust wrote different stdout"
    );
    // …and pin the actual C-defined behaviour, so the test would also catch
    // "both wrong in the same way" after a future edit.
    assert_eq!(
        got_c.ret, expected_ret,
        "[{context}] unexpected return value (C reference behaviour)"
    );
    assert_eq!(
        String::from_utf8_lossy(&got_c.err),
        expected_stderr,
        "[{context}] unexpected stderr (C reference behaviour)"
    );
    assert!(
        got_c.out.is_empty(),
        "[{context}] parse_env_numeric must never write to stdout, got {:?}",
        String::from_utf8_lossy(&got_c.out)
    );
}

// ===========================================================================
// Row 1 — variable absent ⇒ default_val, silently
// ===========================================================================

#[test]
fn err_01_missing_env_returns_default() {
    let _g = lock();
    let mut rng = rng_for("err_01");
    for dv in [0, 1, -1, 42, -42, i32::MAX, i32::MIN, DEF_BASE_OFFSET, DEF_MULTIPLIER] {
        expect_parse(
            &format!("err1 default={dv}"),
            &[("PROG_ABSENT_XYZ", None)],
            "PROG_ABSENT_XYZ",
            dv,
            dv,
            "",
        );
    }
    for i in 0..300 {
        let dv = rng.interesting_i32();
        expect_parse(
            &format!("err1/rand#{i} default={dv}"),
            &[("PROG_BASE_OFFSET", None)],
            "PROG_BASE_OFFSET",
            dv,
            dv,
            "",
        );
    }
}

// ===========================================================================
// Row 2 — present but empty ⇒ atoi("") == 0, NOT default_val
// ===========================================================================

#[test]
fn err_02_empty_env_value_is_atoi_zero() {
    let _g = lock();
    for dv in [0, 1, -1, 999, i32::MAX, i32::MIN] {
        expect_parse(
            &format!("err2 default={dv}"),
            &[("PROG_BASE_OFFSET", Some(""))],
            "PROG_BASE_OFFSET",
            dv,
            0,
            "",
        );
    }
}

// ===========================================================================
// Row 3 — comma ⇒ default_val + "Warning: Invalid character in <name>"
// ===========================================================================

#[test]
fn err_03_comma_rejected_with_warning() {
    let _g = lock();
    let mut rng = rng_for("err_03");
    let fixed = [",", "1,", ",1", "1,2", "12,34", "a,b", ",,,", "  ,  "];
    for v in fixed {
        expect_parse(
            &format!("err3 value={v:?}"),
            &[("PROG_MULTIPLIER", Some(v))],
            "PROG_MULTIPLIER",
            -7,
            -7,
            "Warning: Invalid character in PROG_MULTIPLIER\n",
        );
    }
    for i in 0..300 {
        let v = random_comma_value(&mut rng);
        let dv = rng.interesting_i32();
        expect_parse(
            &format!("err3/rand#{i} value={v:?}"),
            &[("PROG_BASE_OFFSET", Some(&v))],
            "PROG_BASE_OFFSET",
            dv,
            dv,
            "Warning: Invalid character in PROG_BASE_OFFSET\n",
        );
    }
}

// ===========================================================================
// Row 4 — semicolon (no comma) ⇒ default_val + "Warning: Semicolon found in …"
// ===========================================================================

#[test]
fn err_04_semicolon_rejected_with_warning() {
    let _g = lock();
    let mut rng = rng_for("err_04");
    let fixed = [";", "1;", ";1", "1;2", "12;34", "a;b", ";;;"];
    for v in fixed {
        expect_parse(
            &format!("err4 value={v:?}"),
            &[("PROG_MULTIPLIER", Some(v))],
            "PROG_MULTIPLIER",
            123,
            123,
            "Warning: Semicolon found in PROG_MULTIPLIER\n",
        );
    }
    for i in 0..300 {
        let v = random_semicolon_value(&mut rng);
        let dv = rng.interesting_i32();
        expect_parse(
            &format!("err4/rand#{i} value={v:?}"),
            &[("PROG_BASE_OFFSET", Some(&v))],
            "PROG_BASE_OFFSET",
            dv,
            dv,
            "Warning: Semicolon found in PROG_BASE_OFFSET\n",
        );
    }
}

// ===========================================================================
// Row 5 — both ⇒ the comma check short-circuits, only its warning appears
// ===========================================================================

#[test]
fn err_05_comma_wins_over_semicolon() {
    let _g = lock();
    let mut rng = rng_for("err_05");
    // comma before semicolon, and semicolon before comma: the *order in the
    // string* is irrelevant, the order of the checks in the C is what decides.
    for v in [",;", ";,", "1,2;3", "1;2,3", ",,;;", ";;,,"] {
        expect_parse(
            &format!("err5 value={v:?}"),
            &[("PROG_MULTIPLIER", Some(v))],
            "PROG_MULTIPLIER",
            -1,
            -1,
            "Warning: Invalid character in PROG_MULTIPLIER\n",
        );
    }
    for i in 0..300 {
        let mut v = random_clean_value(&mut rng, 10);
        let p1 = rng.below(v.len() as u64 + 1) as usize;
        v.insert(p1, ',');
        let p2 = rng.below(v.len() as u64 + 1) as usize;
        v.insert(p2, ';');
        let dv = rng.interesting_i32();
        expect_parse(
            &format!("err5/rand#{i} value={v:?}"),
            &[("PROG_MULTIPLIER", Some(&v))],
            "PROG_MULTIPLIER",
            dv,
            dv,
            "Warning: Invalid character in PROG_MULTIPLIER\n",
        );
    }
}

// ===========================================================================
// Row 6 — unparseable text falls through to atoi (no default_val fallback)
// ===========================================================================

#[test]
fn err_06_unparseable_value_falls_through_to_atoi() {
    let _g = lock();
    // Expected values are `atoi`'s documented prefix-parse behaviour.
    let cases: [(&str, c_int); 16] = [
        ("abc", 0),
        ("+", 0),
        ("-", 0),
        (" ", 0),
        ("\t\n", 0),
        ("0x10", 0),
        ("1 2", 1),
        ("12abc", 12),
        ("  -34xyz", -34),
        ("--5", 0),
        ("+-5", 0),
        ("3.99", 3),
        (".5", 0),
        ("1e5", 1),
        ("z9", 0),
        ("007", 7),
    ];
    for (v, expected) in cases {
        expect_parse(
            &format!("err6 value={v:?}"),
            &[("PROG_BASE_OFFSET", Some(v))],
            "PROG_BASE_OFFSET",
            9999, // must NOT be returned
            expected,
            "",
        );
    }
}

// ===========================================================================
// Row 7 — atoi integer overflow (UB in C; both sides call the same libc atoi)
// ===========================================================================

#[test]
fn err_07_int_overflow_in_atoi() {
    let _g = lock();
    let (c, r) = both();
    let values = [
        "2147483647",
        "2147483648",
        "2147483649",
        "4294967296",
        "-2147483648",
        "-2147483649",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775809",
        "18446744073709551616",
        "99999999999999999999",
        "-99999999999999999999",
        "123456789012345678901234567890",
    ];
    for v in values {
        env_config(&[("PROG_BASE_OFFSET", Some(v))]);
        let got_c = capture(|| call_parse(c, "PROG_BASE_OFFSET", 5));
        env_config(&[("PROG_BASE_OFFSET", Some(v))]);
        let got_r = capture(|| call_parse(r, "PROG_BASE_OFFSET", 5));
        assert_eq!(
            got_c, got_r,
            "err7 value={v:?}: overflow handling diverged"
        );
        // Whatever glibc does, it must not be the default_val fallback path.
        assert_ne!(
            got_c.ret, 5,
            "err7 value={v:?}: unexpectedly took the default_val path"
        );
        assert!(got_c.err.is_empty(), "err7 value={v:?}: unexpected warning");
    }
}

// ===========================================================================
// Row 8 — empty variable *name*: getenv("") finds nothing ⇒ default_val
// ===========================================================================

#[test]
fn err_08_empty_env_name() {
    let _g = lock();
    for dv in [0, 1, -1, 777, i32::MAX, i32::MIN] {
        // `setenv("")` is EINVAL, so "" can never be present: the only
        // reachable behaviour is the `getenv == NULL` branch.
        expect_parse(&format!("err8 default={dv}"), &[], "", dv, dv, "");
    }
}

// ===========================================================================
// Row 9 — extreme default_val passes straight through, unclamped
// ===========================================================================

#[test]
fn err_09_extreme_default_val_passthrough() {
    let _g = lock();
    for dv in [i32::MIN, i32::MIN + 1, -1, 0, i32::MAX - 1, i32::MAX] {
        // trigger 1: absent
        expect_parse(
            &format!("err9/absent default={dv}"),
            &[("PROG_MULTIPLIER", None)],
            "PROG_MULTIPLIER",
            dv,
            dv,
            "",
        );
        // trigger 3: comma
        expect_parse(
            &format!("err9/comma default={dv}"),
            &[("PROG_MULTIPLIER", Some("x,y"))],
            "PROG_MULTIPLIER",
            dv,
            dv,
            "Warning: Invalid character in PROG_MULTIPLIER\n",
        );
        // trigger 4: semicolon
        expect_parse(
            &format!("err9/semi default={dv}"),
            &[("PROG_MULTIPLIER", Some("x;y"))],
            "PROG_MULTIPLIER",
            dv,
            dv,
            "Warning: Semicolon found in PROG_MULTIPLIER\n",
        );
    }
}

// ===========================================================================
// Rows 10..13 — the C validates no pointer at all. Both libraries must die the
// same way. Executed in a forked child so the harness survives.
// ===========================================================================

#[test]
fn err_10_null_env_name_crashes_identically() {
    let _g = lock();
    env_clear_prog();
    let (c, _r) = both();
    // Establish the reference behaviour from the C library first.
    let oc = run_in_child(|| unsafe {
        (c.parse_env_numeric)(ptr::null(), 7);
    });
    assert_eq!(
        oc,
        Outcome::Signaled(SIGSEGV),
        "C reference: getenv(NULL) is expected to segfault, got {oc:?}"
    );
    diff_crash("err10 parse_env_numeric(NULL, 7)", |api| unsafe {
        (api.parse_env_numeric)(ptr::null(), 7);
    });
    // also with extreme default values, in case a null check were added
    for dv in [0, i32::MIN, i32::MAX] {
        diff_crash(&format!("err10 parse_env_numeric(NULL, {dv})"), move |api| unsafe {
            (api.parse_env_numeric)(ptr::null(), dv);
        });
    }
}

#[test]
fn err_11_null_flags_init_crashes_identically() {
    let _g = lock();
    let (c, _r) = both();
    for env in [
        vec![],
        vec![("PROG_VERBOSE", Some("1")), ("PROG_DEBUG", Some("1"))],
        vec![("PROG_OPTIMIZE", Some("1"))],
    ] {
        env_config(&env);
        let oc = run_in_child(|| unsafe { (c.init_config_from_env)(ptr::null_mut()) });
        assert_eq!(
            oc,
            Outcome::Signaled(SIGSEGV),
            "C reference: init_config_from_env(NULL) should segfault, got {oc:?}"
        );
        diff_crash("err11 init_config_from_env(NULL)", |api| unsafe {
            (api.init_config_from_env)(ptr::null_mut())
        });
    }
}

#[test]
fn err_12_null_flags_perform_crashes_identically() {
    let _g = lock();
    env_clear_prog();
    let (c, _r) = both();
    let oc = run_in_child(|| unsafe {
        (c.perform_operation)(1, 2, ptr::null_mut());
    });
    assert_eq!(
        oc,
        Outcome::Signaled(SIGSEGV),
        "C reference: perform_operation(_,_,NULL) should segfault, got {oc:?}"
    );
    for (v1, v2) in [(0, 0), (1, 2), (i32::MAX, i32::MIN), (-1, -1)] {
        diff_crash(
            &format!("err12 perform_operation({v1},{v2},NULL)"),
            move |api| unsafe {
                (api.perform_operation)(v1, v2, ptr::null_mut());
            },
        );
    }
}

#[test]
fn err_13_null_flags_apply_crashes_identically() {
    let _g = lock();
    env_clear_prog();
    let (c, _r) = both();
    let oc = run_in_child(|| unsafe {
        (c.apply_bit_operations)(1, ptr::null_mut());
    });
    assert_eq!(
        oc,
        Outcome::Signaled(SIGSEGV),
        "C reference: apply_bit_operations(_,NULL) should segfault, got {oc:?}"
    );
    for v in [0, 1, -1, i32::MAX, i32::MIN] {
        diff_crash(
            &format!("err13 apply_bit_operations({v},NULL)"),
            move |api| unsafe {
                (api.apply_bit_operations)(v, ptr::null_mut());
            },
        );
    }
}

// ===========================================================================
// Row 14 — result < 0 ⇒ roll-back, returned value becomes param1
// ===========================================================================

/// Reference model of the C's `envy` for a *clean* environment
/// (log_level = 3, cache_enabled = 1, verbose = 0, debug = 0, optimize = 0,
/// base_offset = 64, multiplier = 10).
fn envy_model_clean(p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
    let mut result = p1.wrapping_mul(3).wrapping_add(p2.wrapping_div(2));
    if p3 != 0 {
        result = result.wrapping_add(p3.wrapping_mul(DEF_MULTIPLIER));
    }
    if p4 != 0 {
        result = result.wrapping_add(p4 >> 2);
    }
    result |= 0x0F;
    result = result.wrapping_add(DEF_BASE_OFFSET);
    if result < 0 {
        return p1;
    }
    result
}

#[test]
fn err_14_negative_result_rolls_back_to_param1() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = rng_for("err_14");
    let mut rollbacks = 0usize;
    for i in 0..1500 {
        // Bias hard towards the negative side so the roll-back really triggers.
        let p1 = -(rng.below(200_000) as i32);
        let p2 = rng.interesting_i32();
        let p3 = -(rng.below(200_000) as i32);
        let p4 = rng.interesting_i32();

        let expected = envy_model_clean(p1, p2, p3, p4);

        env_clear_prog();
        let got_c = capture(|| unsafe { (c.envy)(p1, p2, p3, p4) });
        env_clear_prog();
        let got_r = capture(|| unsafe { (r.envy)(p1, p2, p3, p4) });

        assert_eq!(got_c, got_r, "err14/#{i} params=({p1},{p2},{p3},{p4})");
        assert_eq!(
            got_c.ret, expected,
            "err14/#{i} params=({p1},{p2},{p3},{p4}): C diverged from the roll-back model"
        );
        if got_c.ret == p1 && p1 <= 0 {
            rollbacks += 1;
        }
    }
    assert!(
        rollbacks > 100,
        "err14: the roll-back branch was only reached {rollbacks} times — test is not exercising row 14"
    );
}

// ===========================================================================
// Row 15 — the roll-back does not re-check, so it can return a negative value
// ===========================================================================

#[test]
fn err_15_rollback_can_return_negative() {
    let _g = lock();
    let (c, r) = both();
    let mut saw_negative_return = 0usize;
    // p3 very negative drags the result below zero; p1 is then returned as-is.
    for p1 in [-1i32, -2, -1000, -123456, i32::MIN, i32::MIN + 1] {
        for p3 in [-1_000_000i32, -50_000, -7] {
            env_clear_prog();
            let got_c = capture(|| unsafe { (c.envy)(p1, 0, p3, 0) });
            env_clear_prog();
            let got_r = capture(|| unsafe { (r.envy)(p1, 0, p3, 0) });
            assert_eq!(got_c, got_r, "err15 params=({p1},0,{p3},0)");
            assert_eq!(
                got_c.ret,
                envy_model_clean(p1, 0, p3, 0),
                "err15 params=({p1},0,{p3},0): diverged from model"
            );
            if got_c.ret < 0 {
                saw_negative_return += 1;
                assert_eq!(
                    got_c.ret, p1,
                    "err15: a negative return must be exactly param1"
                );
            }
        }
    }
    assert!(
        saw_negative_return > 0,
        "err15: never observed a negative return value"
    );
}

// ===========================================================================
// Rows 16/17 — the `param3 == 0` / `param4 == 0` blocks are skipped entirely
// ===========================================================================

#[test]
fn err_16_param3_zero_skips_block() {
    let _g = lock();
    let mut rng = rng_for("err_16");
    for i in 0..400 {
        let p1 = rng.interesting_i32();
        let p2 = rng.interesting_i32();
        let p4 = rng.interesting_i32();
        // A huge multiplier would visibly change the result if the block ran.
        for mu in ["1000000", "-1000000", "2147483647"] {
            diff_with_env(
                &format!("err16/#{i} mu={mu}"),
                &[("PROG_MULTIPLIER", Some(mu))],
                move |api| unsafe { (api.envy)(p1, p2, 0, p4) },
            );
        }
        // and confirm p3 == 0 gives the same answer regardless of multiplier
        let (c, _r) = both();
        env_config(&[("PROG_MULTIPLIER", Some("1000000"))]);
        let a = capture(|| unsafe { (c.envy)(p1, p2, 0, p4) }).ret;
        env_config(&[("PROG_MULTIPLIER", Some("-999"))]);
        let b = capture(|| unsafe { (c.envy)(p1, p2, 0, p4) }).ret;
        assert_eq!(a, b, "err16/#{i}: multiplier leaked in despite param3 == 0");
    }
}

#[test]
fn err_17_param4_zero_skips_block() {
    let _g = lock();
    let mut rng = rng_for("err_17");
    for i in 0..600 {
        let p1 = rng.interesting_i32();
        let p2 = rng.interesting_i32();
        let p3 = rng.interesting_i32();
        diff_with_env(&format!("err17/#{i}"), &[], move |api| unsafe {
            (api.envy)(p1, p2, p3, 0)
        });
        // `0 >> 2 == 0`, so skipping is observationally equal here; the point of
        // the row is that both implementations *branch* the same way.
        diff_with_env(&format!("err17/#{i} verbose"), &[("PROG_VERBOSE", Some("1"))], move |api| unsafe {
            (api.envy)(p1, p2, p3, 0)
        });
    }
}

// ===========================================================================
// Row 18 — signed overflow wraps everywhere (must never panic in Rust)
// ===========================================================================

#[test]
fn err_18_signed_overflow_wraps_everywhere() {
    let _g = lock();
    let ext: [i32; 10] = [
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x4000_0000,
        -0x4000_0000,
        0x7FFF_FFFE,
        1,
        -1,
        0,
    ];

    // val1 * log_level and val2 / 2 (perform_operation, non-optimize)
    for log_level in 0u8..8 {
        for v1 in ext {
            for v2 in ext {
                let flags = Flags4::from_fields(0, 0, 0, 1, log_level, 0);
                diff_flags_with_env(
                    &format!("err18/perform ll={log_level} v1={v1} v2={v2}"),
                    &[],
                    flags,
                    move |api, p| unsafe { (api.perform_operation)(v1, v2, p) },
                );
            }
        }
    }
    // val1 + val2 (perform_operation, optimize)
    for v1 in ext {
        for v2 in ext {
            let flags = Flags4::from_fields(0, 0, 1, 1, 3, 0);
            diff_flags_with_env(
                &format!("err18/perform-opt v1={v1} v2={v2}"),
                &[],
                flags,
                move |api, p| unsafe { (api.perform_operation)(v1, v2, p) },
            );
        }
    }
    // adjusted << 1 (apply_bit_operations, verbose)
    for v in ext {
        for cache in 0u8..2 {
            let flags = Flags4::from_fields(1, 0, 0, cache, 0, 0);
            diff_flags_with_env(
                &format!("err18/apply v={v} cache={cache}"),
                &[],
                flags,
                move |api, p| unsafe { (api.apply_bit_operations)(v, p) },
            );
        }
    }
    // param3 * multiplier, result + …, result + base_offset (envy)
    for v in ext {
        for mu in ["2147483647", "-2147483648", "1073741824"] {
            for bo in ["2147483647", "-2147483648"] {
                diff_with_env(
                    &format!("err18/envy v={v} mu={mu} bo={bo}"),
                    &[
                        ("PROG_MULTIPLIER", Some(mu)),
                        ("PROG_BASE_OFFSET", Some(bo)),
                    ],
                    move |api| unsafe { (api.envy)(v, v, v, v) },
                );
            }
        }
    }
}

// ===========================================================================
// Row 19 — `param4 >> 2` is an arithmetic shift on negatives
// ===========================================================================

#[test]
fn err_19_negative_right_shift_is_arithmetic() {
    let _g = lock();
    let (c, r) = both();
    // -1 >> 2 == -1 (arithmetic), not 0x3FFFFFFF (logical).
    for p4 in [-1i32, -2, -3, -4, -5, -6, -7, -8, -9, -1000, i32::MIN, i32::MIN + 1] {
        let expected = envy_model_clean(0, 0, 0, p4);
        env_clear_prog();
        let got_c = capture(|| unsafe { (c.envy)(0, 0, 0, p4) });
        env_clear_prog();
        let got_r = capture(|| unsafe { (r.envy)(0, 0, 0, p4) });
        assert_eq!(got_c, got_r, "err19 p4={p4}");
        assert_eq!(
            got_c.ret, expected,
            "err19 p4={p4}: not an arithmetic shift"
        );
    }
    // Pin the sign-propagation explicitly. For envy(0, 0, 0, -4) on a clean
    // environment the pipeline is:
    //   result = 0*3 + 0/2          = 0
    //   param4 != 0  -> result += (-4 >> 2)
    //   cache_enabled -> result |= 0x0F
    //   result += 64
    // An *arithmetic* shift gives -4 >> 2 == -1, so
    //   result = ((0 + -1) | 0x0F) + 64 = (-1 | 0x0F) + 64 = -1 + 64 = 63.
    // A *logical* shift would give 0x3FFFFFFF, i.e. ((0x3FFFFFFF) | 0x0F) + 64
    //   = 0x4000004E = 1073741902 — wildly different.
    const ARITHMETIC: i32 = 63;
    const IF_LOGICAL: i32 = (((-4i32 as u32 >> 2) as i32) | 0x0F).wrapping_add(64);
    assert_ne!(ARITHMETIC, IF_LOGICAL, "the two shift kinds must differ here");
    env_clear_prog();
    let got_c = capture(|| unsafe { (c.envy)(0, 0, 0, -4) });
    env_clear_prog();
    let got_r = capture(|| unsafe { (r.envy)(0, 0, 0, -4) });
    assert_eq!(
        got_c.ret, ARITHMETIC,
        "C reference: `param4 >> 2` must be an arithmetic shift"
    );
    assert_eq!(
        got_r.ret, ARITHMETIC,
        "Rust used a logical shift for `param4 >> 2`"
    );
}

// ===========================================================================
// Row 20 — INT_MIN / 2 truncates toward zero without trapping
// ===========================================================================

#[test]
fn err_20_int_min_division() {
    let _g = lock();
    for log_level in 0u8..8 {
        for v1 in [0i32, 1, -1, i32::MAX, i32::MIN] {
            let flags = Flags4::from_fields(0, 0, 0, 1, log_level, 0);
            diff_flags_with_env(
                &format!("err20/perform ll={log_level} v1={v1} v2=INT_MIN"),
                &[],
                flags,
                move |api, p| unsafe { (api.perform_operation)(v1, i32::MIN, p) },
            );
        }
    }
    // and through envy
    for p1 in [0i32, 1, -1, i32::MAX, i32::MIN] {
        diff_with_env(
            &format!("err20/envy p1={p1} p2=INT_MIN"),
            &[],
            move |api| unsafe { (api.envy)(p1, i32::MIN, 0, 0) },
        );
    }
    // odd negatives truncate toward zero: -3 / 2 == -1
    for v2 in [-1i32, -3, -5, -7, -2147483647] {
        let flags = Flags4::from_fields(0, 0, 0, 1, 1, 0);
        diff_flags_with_env(
            &format!("err20/trunc v2={v2}"),
            &[],
            flags,
            move |api, p| unsafe { (api.perform_operation)(0, v2, p) },
        );
    }
}

// ===========================================================================
// Row 21 — rejected env vars while verbose: stdout and stderr interleaving
// ===========================================================================

#[test]
fn err_21_rejected_env_with_verbose_output() {
    let _g = lock();
    let (c, r) = both();
    let cases: [(&str, &str); 6] = [
        ("1,2", "3,4"),
        ("1;2", "3;4"),
        ("1,2", "3;4"),
        ("1;2", "3,4"),
        (",", ";"),
        ("a,b;c", "d;e,f"),
    ];
    for (bo, mu) in cases {
        let env = [
            ("PROG_BASE_OFFSET", Some(bo)),
            ("PROG_MULTIPLIER", Some(mu)),
            ("PROG_VERBOSE", Some("1")),
        ];
        env_config(&env);
        let got_c = capture(|| unsafe { (c.envy)(5, 6, 7, 8) });
        env_config(&env);
        let got_r = capture(|| unsafe { (r.envy)(5, 6, 7, 8) });
        assert_eq!(got_c, got_r, "err21 bo={bo:?} mu={mu:?}");

        // Both variables were rejected, so the octal defaults must be reported.
        let out = String::from_utf8_lossy(&got_c.out).to_string();
        assert!(
            out.contains("Base offset: 64 (from octal 0100)\n"),
            "err21 bo={bo:?} mu={mu:?}: default base offset not reported, out={out:?}"
        );
        assert!(
            out.contains("Multiplier: 10 (from octal 012)\n"),
            "err21 bo={bo:?} mu={mu:?}: default multiplier not reported, out={out:?}"
        );
        // Exactly two warnings, on stderr, one per variable.
        let err = String::from_utf8_lossy(&got_c.err).to_string();
        assert_eq!(
            err.lines().count(),
            2,
            "err21 bo={bo:?} mu={mu:?}: expected 2 warnings, got {err:?}"
        );
        let expect_bo = if bo.contains(',') {
            "Warning: Invalid character in PROG_BASE_OFFSET"
        } else {
            "Warning: Semicolon found in PROG_BASE_OFFSET"
        };
        let expect_mu = if mu.contains(',') {
            "Warning: Invalid character in PROG_MULTIPLIER"
        } else {
            "Warning: Semicolon found in PROG_MULTIPLIER"
        };
        assert_eq!(
            err,
            format!("{expect_bo}\n{expect_mu}\n"),
            "err21 bo={bo:?} mu={mu:?}: wrong warnings / wrong order"
        );
    }
}

// ===========================================================================
// Row 22 — extreme PROG_MULTIPLIER so param3 * multiplier overflows hard
// ===========================================================================

#[test]
fn err_22_extreme_multiplier_overflow() {
    let _g = lock();
    let mut rng = rng_for("err_22");
    let mus = [
        "2147483647",
        "-2147483648",
        "1073741824",
        "-1073741824",
        "65536",
        "-65536",
    ];
    for mu in mus {
        for p3 in [1i32, -1, 2, -2, 65536, -65536, i32::MAX, i32::MIN, 0x4000_0000] {
            diff_with_env(
                &format!("err22 mu={mu} p3={p3}"),
                &[("PROG_MULTIPLIER", Some(mu))],
                move |api| unsafe { (api.envy)(1, 1, p3, 1) },
            );
        }
        for i in 0..100 {
            let p = [
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            ];
            diff_with_env(
                &format!("err22/rand#{i} mu={mu} params={p:?}"),
                &[("PROG_MULTIPLIER", Some(mu)), ("PROG_VERBOSE", Some("1"))],
                move |api| unsafe { (api.envy)(p[0], p[1], p[2], p[3]) },
            );
        }
    }
}

// ===========================================================================
// Row 23 — bytes 1..3 of struct ConfigFlags: ignored on read, preserved on write
// ===========================================================================

#[test]
fn err_23_padding_bytes_ignored_and_preserved() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = rng_for("err_23");

    // (a) `init_config_from_env` must leave bytes 1..3 exactly as they were.
    for i in 0..400 {
        let initial = rng.flags4();
        let env: Vec<(&str, Option<&str>)> = vec![
            ("PROG_VERBOSE", if rng.bool() { Some("1") } else { None }),
            ("PROG_DEBUG", if rng.bool() { Some("1") } else { None }),
            ("PROG_OPTIMIZE", if rng.bool() { Some("1") } else { None }),
        ];
        let mut fc = initial;
        env_config(&env);
        capture(|| {
            unsafe { (c.init_config_from_env)(fc.as_mut_ptr()) };
            0
        });
        let mut fr = initial;
        env_config(&env);
        capture(|| {
            unsafe { (r.init_config_from_env)(fr.as_mut_ptr()) };
            0
        });
        assert_eq!(fc, fr, "err23a/#{i} initial={:02x?}", initial.0);
        assert_eq!(
            &fc.0[1..],
            &initial.0[1..],
            "err23a/#{i}: C clobbered the padding bytes"
        );
        assert_eq!(
            &fr.0[1..],
            &initial.0[1..],
            "err23a/#{i}: Rust clobbered the padding bytes"
        );
    }

    // (b) the two reader functions must ignore bytes 1..3 entirely and must not
    // modify the struct at all.
    for i in 0..400 {
        let b0 = rng.next_u32() as u8;
        let v1 = rng.interesting_i32();
        let v2 = rng.interesting_i32();
        let tails: [[u8; 3]; 4] = [[0, 0, 0], [0xFF, 0xFF, 0xFF], [0x5A, 0xA5, 0x3C], [1, 2, 3]];
        let mut perform_rets = Vec::new();
        let mut apply_rets = Vec::new();
        for t in tails {
            for api in [c, r] {
                let mut f = Flags4([b0, t[0], t[1], t[2]]);
                env_clear_prog();
                let got = capture(|| unsafe { (api.perform_operation)(v1, v2, f.as_mut_ptr()) });
                assert_eq!(
                    f.0,
                    [b0, t[0], t[1], t[2]],
                    "err23b/#{i}: {} perform_operation mutated the struct",
                    api.name
                );
                perform_rets.push(got.ret);

                let mut f = Flags4([b0, t[0], t[1], t[2]]);
                let got = capture(|| unsafe { (api.apply_bit_operations)(v1, f.as_mut_ptr()) });
                assert_eq!(
                    f.0,
                    [b0, t[0], t[1], t[2]],
                    "err23b/#{i}: {} apply_bit_operations mutated the struct",
                    api.name
                );
                apply_rets.push(got.ret);
            }
        }
        assert!(
            perform_rets.windows(2).all(|w| w[0] == w[1]),
            "err23b/#{i} b0={b0:#04x}: perform_operation depended on the padding bytes: {perform_rets:?}"
        );
        assert!(
            apply_rets.windows(2).all(|w| w[0] == w[1]),
            "err23b/#{i} b0={b0:#04x}: apply_bit_operations depended on the padding bytes: {apply_rets:?}"
        );
    }
}

// ===========================================================================
// Row 24 — every one of the 256 flag bit patterns, including the values
// `init_config_from_env` can never produce (log_level 4..7, reserved = 1).
// This is the FFI-boundary "out-of-range enum value" case for this library:
// the bit-fields are the only enumerated state, and C accepts any bit pattern.
// ===========================================================================

#[test]
fn err_24_all_256_flag_bit_patterns() {
    let _g = lock();
    let mut rng = rng_for("err_24");
    let mut unreachable_patterns = 0usize;
    for b0 in 0u8..=255 {
        let log_level = (b0 >> 4) & 7;
        let reserved = (b0 >> 7) & 1;
        if log_level != 3 || reserved != 0 {
            unreachable_patterns += 1;
        }
        for i in 0..4 {
            let v1 = rng.interesting_i32();
            let v2 = rng.interesting_i32();
            diff_flags_with_env(
                &format!("err24/perform b0={b0:#04x}/#{i}"),
                &[],
                Flags4([b0, 0xDE, 0xAD, 0xBE]),
                move |api, p| unsafe { (api.perform_operation)(v1, v2, p) },
            );
            diff_flags_with_env(
                &format!("err24/apply b0={b0:#04x}/#{i}"),
                &[],
                Flags4([b0, 0xDE, 0xAD, 0xBE]),
                move |api, p| unsafe { (api.apply_bit_operations)(v1, p) },
            );
            // …and init over each pattern, so the write path sees them too.
            diff_flags_with_env(
                &format!("err24/init b0={b0:#04x}/#{i}"),
                &[("PROG_VERBOSE", Some("1"))],
                Flags4([b0, 0xDE, 0xAD, 0xBE]),
                |api, p| {
                    unsafe { (api.init_config_from_env)(p) };
                    0
                },
            );
        }
    }
    assert!(
        unreachable_patterns >= 224,
        "err24: expected most of the 256 patterns to be unreachable via \
         init_config_from_env, counted {unreachable_patterns}"
    );
}

// ===========================================================================
// Row 25 — PROG_OPTIMIZE="" still enables optimize (only != NULL is tested)
// ===========================================================================

#[test]
fn err_25_empty_prog_optimize_still_enables() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = rng_for("err_25");

    // The struct byte must show optimize = 1 for an empty value.
    let mut fc = Flags4([0, 0, 0, 0]);
    env_config(&[("PROG_OPTIMIZE", Some(""))]);
    capture(|| {
        unsafe { (c.init_config_from_env)(fc.as_mut_ptr()) };
        0
    });
    assert_eq!(
        fc.0[0] & 0b100,
        0b100,
        "C reference: PROG_OPTIMIZE=\"\" must set optimize, byte0={:#04x}",
        fc.0[0]
    );
    let mut fr = Flags4([0, 0, 0, 0]);
    env_config(&[("PROG_OPTIMIZE", Some(""))]);
    capture(|| {
        unsafe { (r.init_config_from_env)(fr.as_mut_ptr()) };
        0
    });
    assert_eq!(fc, fr, "err25: init diverged for PROG_OPTIMIZE=\"\"");

    // And through envy: the empty value must select the `val1 + val2` branch,
    // i.e. give the same answer as PROG_OPTIMIZE=1 and a *different* one from
    // PROG_OPTIMIZE unset.
    for i in 0..200 {
        let p1 = rng.interesting_i32();
        let p2 = rng.interesting_i32();
        for o in ["", "0", "false", " ", "\t", "1"] {
            diff_with_env(
                &format!("err25/#{i} optimize={o:?}"),
                &[("PROG_OPTIMIZE", Some(o))],
                move |api| unsafe { (api.envy)(p1, p2, 0, 0) },
            );
        }
        env_config(&[("PROG_OPTIMIZE", Some(""))]);
        let empty = capture(|| unsafe { (c.envy)(p1, p2, 0, 0) }).ret;
        env_config(&[("PROG_OPTIMIZE", Some("1"))]);
        let one = capture(|| unsafe { (c.envy)(p1, p2, 0, 0) }).ret;
        assert_eq!(
            empty, one,
            "err25/#{i}: empty PROG_OPTIMIZE behaved differently from \"1\""
        );
    }
}

// ===========================================================================
// Row 26 — verbose/debug need the literal character '1', not truthiness
// ===========================================================================

#[test]
fn err_26_verbose_debug_require_literal_one() {
    let _g = lock();
    let (c, r) = both();
    let falsy = ["", "0", "true", "yes", "on", "TRUE", "enabled", "2", "  ", "\t", "-"];
    for v in falsy {
        assert!(!v.contains('1'));
        let env = [("PROG_VERBOSE", Some(v)), ("PROG_DEBUG", Some(v))];

        let mut fc = Flags4([0, 0, 0, 0]);
        env_config(&env);
        capture(|| {
            unsafe { (c.init_config_from_env)(fc.as_mut_ptr()) };
            0
        });
        assert_eq!(
            fc.0[0] & 0b11,
            0,
            "C reference: {v:?} must NOT enable verbose/debug, byte0={:#04x}",
            fc.0[0]
        );
        let mut fr = Flags4([0, 0, 0, 0]);
        env_config(&env);
        capture(|| {
            unsafe { (r.init_config_from_env)(fr.as_mut_ptr()) };
            0
        });
        assert_eq!(fc, fr, "err26: init diverged for {v:?}");

        // envy must stay completely silent for these values
        env_config(&env);
        let got_c = capture(|| unsafe { (c.envy)(3, 4, 5, 6) });
        env_config(&env);
        let got_r = capture(|| unsafe { (r.envy)(3, 4, 5, 6) });
        assert_eq!(got_c, got_r, "err26 envy diverged for {v:?}");
        assert!(
            got_c.out.is_empty() && got_c.err.is_empty(),
            "err26 {v:?}: expected silence, got out={:?} err={:?}",
            String::from_utf8_lossy(&got_c.out),
            String::from_utf8_lossy(&got_c.err)
        );
    }
}

// ===========================================================================
// Row 27 — '1' anywhere in the value enables the flag (substring, not equality)
// ===========================================================================

#[test]
fn err_27_one_anywhere_enables_flag() {
    let _g = lock();
    let (c, r) = both();
    let truthy = ["1", "x1", "1x", "31337", "0001", "1000", "-1", "a1b", "0.1", "  1  ", "21"];
    for v in truthy {
        assert!(v.contains('1'));
        let env = [("PROG_VERBOSE", Some(v)), ("PROG_DEBUG", Some(v))];

        let mut fc = Flags4([0, 0, 0, 0]);
        env_config(&env);
        capture(|| {
            unsafe { (c.init_config_from_env)(fc.as_mut_ptr()) };
            0
        });
        assert_eq!(
            fc.0[0] & 0b11,
            0b11,
            "C reference: {v:?} must enable both verbose and debug, byte0={:#04x}",
            fc.0[0]
        );
        let mut fr = Flags4([0, 0, 0, 0]);
        env_config(&env);
        capture(|| {
            unsafe { (r.init_config_from_env)(fr.as_mut_ptr()) };
            0
        });
        assert_eq!(fc, fr, "err27: init diverged for {v:?}");

        env_config(&env);
        let got_c = capture(|| unsafe { (c.envy)(3, 4, 5, 6) });
        env_config(&env);
        let got_r = capture(|| unsafe { (r.envy)(3, 4, 5, 6) });
        assert_eq!(got_c, got_r, "err27 envy diverged for {v:?}");
        assert!(
            !got_c.out.is_empty(),
            "err27 {v:?}: expected verbose+debug output, got nothing"
        );
    }
}

// ===========================================================================
// Generic FFI-boundary sweeps the task mandates beyond the table itself.
// ===========================================================================

/// Zero and oversized-looking lengths do not exist in this API (no buffer is
/// ever passed in), but the *value* domain of every scalar parameter can still
/// be swept one step past every documented boundary.
#[test]
fn generic_scalar_boundary_sweep() {
    let _g = lock();
    let boundaries: [i32; 16] = [
        i32::MIN,
        i32::MIN + 1,
        -0x4000_0001,
        -0x4000_0000,
        -0x3FFF_FFFF,
        -16,
        -15,
        -1,
        0,
        1,
        15,
        16,
        0x3FFF_FFFF,
        0x4000_0000,
        i32::MAX - 1,
        i32::MAX,
    ];
    // parse_env_numeric default_val
    for dv in boundaries {
        diff_with_env(
            &format!("generic/parse dv={dv}"),
            &[("PROG_BASE_OFFSET", None)],
            move |api| call_parse(api, "PROG_BASE_OFFSET", dv),
        );
    }
    // apply_bit_operations value, for all four relevant flag combinations
    for verbose in 0u8..2 {
        for cache in 0u8..2 {
            for v in boundaries {
                let flags = Flags4::from_fields(verbose, 0, 0, cache, 3, 0);
                diff_flags_with_env(
                    &format!("generic/apply v={v} verbose={verbose} cache={cache}"),
                    &[],
                    flags,
                    move |api, p| unsafe { (api.apply_bit_operations)(v, p) },
                );
            }
        }
    }
    // perform_operation val1/val2 over the full boundary cross-product
    for log_level in [0u8, 3, 7] {
        for optimize in 0u8..2 {
            for v1 in boundaries {
                for v2 in boundaries {
                    let flags = Flags4::from_fields(0, 0, optimize, 1, log_level, 0);
                    diff_flags_with_env(
                        &format!("generic/perform ll={log_level} opt={optimize} v1={v1} v2={v2}"),
                        &[],
                        flags,
                        move |api, p| unsafe { (api.perform_operation)(v1, v2, p) },
                    );
                }
            }
        }
    }
    // envy: each parameter swept while the others sit at boundaries
    for i in 0..4 {
        for v in boundaries {
            for other in [i32::MIN, 0, i32::MAX] {
                let mut p = [other; 4];
                p[i] = v;
                diff_with_env(
                    &format!("generic/envy slot={i} v={v} other={other}"),
                    &[],
                    move |api| unsafe { (api.envy)(p[0], p[1], p[2], p[3]) },
                );
            }
        }
    }
}

/// A misaligned `struct ConfigFlags*`: C only guarantees behaviour for a
/// properly aligned pointer, but x86-64 tolerates it, so both implementations
/// must still agree rather than one of them faulting.
#[test]
fn generic_misaligned_flags_pointer() {
    let _g = lock();
    let mut rng = rng_for("generic_misaligned");
    for off in 1usize..4 {
        for i in 0..40 {
            let mut buf = [0u8; 16];
            let b0 = rng.next_u32() as u8;
            buf[off] = b0;
            let v1 = rng.interesting_i32();
            let v2 = rng.interesting_i32();

            let (c, r) = both();
            let run = |api: &Api| -> (i32, i32, [u8; 16]) {
                let mut b = buf;
                env_clear_prog();
                let p = unsafe { b.as_mut_ptr().add(off) };
                let a = capture(|| unsafe { (api.perform_operation)(v1, v2, p) }).ret;
                let q = capture(|| unsafe { (api.apply_bit_operations)(v1, p) }).ret;
                (a, q, b)
            };
            let rc = run(c);
            let rr = run(r);
            assert_eq!(
                rc, rr,
                "generic/misaligned off={off} #{i} b0={b0:#04x}: diverged"
            );
        }
    }
}
