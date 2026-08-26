// Phase C — error-path differential tests.
//
// One test per ERRORS.md row, all executed from a single `#[test]` function
// (the harness redirects the process-wide fds 1/2, so nothing else may run
// concurrently).  The per-row summary printed at the end is the check-off
// evidence for ERRORS.md.

mod harness;

use harness::*;
use std::ffi::{c_int, CString};

extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// Runs `f` in a forked child and returns a printable description of how the
/// child terminated (exit code or fatal signal).
fn child_outcome<F: FnOnce()>(f: F) -> String {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut status: c_int = 0;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        let sig = status & 0x7f;
        if sig == 0 {
            format!("exit({})", (status >> 8) & 0xff)
        } else {
            format!("signal({sig})")
        }
    }
}

#[test]
fn phase_c_errors() {
    let _guard = GLOBAL.lock().unwrap();
    let (c, r) = load_impls();
    println!("C   : {}", c.path.display());
    println!("RUST: {}", r.path.display());

    let mut fails: Vec<String> = Vec::new();
    let mut rows = Rows::new("ERRORS.md");
    let mut rng = Rng::new(SEED ^ 0x1234_5678);
    let mut n = 0usize;
    clear_prog_env();

    let mut cap = Capture::new("phase-c");
    for imp in [&c, &r] {
        if let Err(e) = self_check_capture(&mut cap, imp) {
            drop(cap);
            panic!("{e}");
        }
    }
    clear_prog_env();

    let base = CString::new("PROG_BASE_OFFSET").unwrap();
    let mult = CString::new("PROG_MULTIPLIER").unwrap();

    macro_rules! row {
        ($num:expr, $name:expr, $body:block) => {{
            let before_f = fails.len();
            let before_n = n;
            $body
            rows.add($num, $name, n - before_n, fails.len() - before_f);
        }};
    }

    // ------------------------------------------------------------------
    // Row 1 — variable absent -> default_val, nothing printed.
    // ------------------------------------------------------------------
    row!(1, "err_01_env_absent_returns_default", {
        put_env("PROG_BASE_OFFSET", None);
        for d in [
            0,
            1,
            -1,
            0o100,
            0o12,
            i32::MAX,
            i32::MIN,
            0x5555_5555,
            -0x5555_5555,
        ] {
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row1 absent default={d}"),
                |imp| call_parse(imp, &base, d),
            );
            n += 1;
        }
        for _ in 0..64 {
            let d = rng.next_i32();
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row1 absent random default={d}"),
                |imp| call_parse(imp, &base, d),
            );
            n += 1;
        }
    });

    // ------------------------------------------------------------------
    // Row 2 — value contains ',' -> stderr warning + default.
    // Also exercises unusual env *names* in the "%s" of the warning.
    // ------------------------------------------------------------------
    row!(2, "err_02_comma_warns_and_returns_default", {
        let long_name = "N".repeat(300);
        let names: [&str; 6] = [
            "PROG_BASE_OFFSET",
            "PROG_MULTIPLIER",
            "DIFFTEST_WEIRD",
            "A=B",
            "x y",
            &long_name,
        ];
        let values = [
            ",", "1,", ",1", "1,2", "a,b", ",,,", "1,2,3", "-1,",
            "2147483647,", ",abc", "1 , 2",
        ];
        for name in names {
            let cname = CString::new(name).unwrap();
            for v in values {
                put_env(name, Some(v));
                for d in [0, -1, 64, i32::MIN] {
                    differential(
                        &mut fails,
                        &mut cap,
                        &c,
                        &r,
                        &format!("row2 name={name:?} value={v:?} default={d}"),
                        |imp| call_parse(imp, &cname, d),
                    );
                    n += 1;
                }
                put_env(name, None);
            }
        }
    });

    // ------------------------------------------------------------------
    // Row 3 — value contains ';' (and no ',') -> stderr warning + default.
    // ------------------------------------------------------------------
    row!(3, "err_03_semicolon_warns_and_returns_default", {
        for v in [
            ";", "1;", ";1", "1;2", ";;;", "-5;", "abc;", "2147483648;", "1 ; 2",
        ] {
            put_env("PROG_MULTIPLIER", Some(v));
            for d in [0, -1, 10, i32::MAX] {
                differential(
                    &mut fails,
                    &mut cap,
                    &c,
                    &r,
                    &format!("row3 value={v:?} default={d}"),
                    |imp| call_parse(imp, &mult, d),
                );
                n += 1;
            }
        }
        put_env("PROG_MULTIPLIER", None);
    });

    // ------------------------------------------------------------------
    // Row 4 — both ',' and ';': the comma check runs first.
    // ------------------------------------------------------------------
    row!(4, "err_04_comma_wins_over_semicolon", {
        for v in [",;", ";,", "1;2,3", "1,2;3", ";1,", ",1;"] {
            put_env("PROG_BASE_OFFSET", Some(v));
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row4 value={v:?}"),
                |imp| call_parse(imp, &base, 64),
            );
            n += 1;
        }
        put_env("PROG_BASE_OFFSET", None);
    });

    // ------------------------------------------------------------------
    // Row 5 — non-numeric value: atoi returns 0, NOT default_val.
    // ------------------------------------------------------------------
    row!(5, "err_05_non_numeric_atoi_zero", {
        for v in [
            "abc", "+", "-", "0x10", " ", "--5", ".", "e5", "\t", "\n", "z9",
            "NaN", "inf", "++1", "- 1", "/", ":",
        ] {
            put_env("PROG_BASE_OFFSET", Some(v));
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row5 value={v:?}"),
                |imp| call_parse(imp, &base, 777),
            );
            n += 1;
        }
        put_env("PROG_BASE_OFFSET", None);
    });

    // ------------------------------------------------------------------
    // Row 6 — empty value: present but "" -> 0.
    // ------------------------------------------------------------------
    row!(6, "err_06_empty_value_is_zero", {
        put_env("PROG_BASE_OFFSET", Some(""));
        for d in [0, 1, -1, 64, i32::MIN, i32::MAX] {
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row6 empty default={d}"),
                |imp| call_parse(imp, &base, d),
            );
            n += 1;
        }
        put_env("PROG_BASE_OFFSET", None);
    });

    // ------------------------------------------------------------------
    // Row 7 — atoi overflow / clamping.
    // ------------------------------------------------------------------
    row!(7, "err_07_atoi_overflow_truncation", {
        let big40 = "9".repeat(40);
        let negbig40 = format!("-{}", "8".repeat(40));
        for v in [
            "2147483648",
            "-2147483649",
            "4294967296",
            "9999999999",
            "-9999999999",
            "9223372036854775807",
            "9223372036854775808",
            "-9223372036854775808",
            "-9223372036854775809",
            "18446744073709551616",
            &big40,
            &negbig40,
            "000000000000002147483648",
        ] {
            put_env("PROG_BASE_OFFSET", Some(v));
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row7 overflow value={v:?}"),
                |imp| call_parse(imp, &base, 64),
            );
            n += 1;
        }
        put_env("PROG_BASE_OFFSET", None);
    });

    // ------------------------------------------------------------------
    // Row 8 — trailing garbage: atoi parses the prefix.
    // ------------------------------------------------------------------
    row!(8, "err_08_trailing_garbage_prefix_parsed", {
        for v in [
            "12abc", "12 34", "12.", "007x", "-0-", "3+4", "-7q", "  8zz", "0nope",
            "2147483647x", "-2147483648y",
        ] {
            put_env("PROG_BASE_OFFSET", Some(v));
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row8 value={v:?}"),
                |imp| call_parse(imp, &base, 64),
            );
            n += 1;
        }
        put_env("PROG_BASE_OFFSET", None);
    });

    // ------------------------------------------------------------------
    // Row 9 — empty env *name*: getenv("") == NULL -> default.
    // ------------------------------------------------------------------
    row!(9, "err_09_empty_env_name", {
        let empty = CString::new("").unwrap();
        let eq = CString::new("=").unwrap();
        for name in [&empty, &eq] {
            for d in [0, -1, 64, i32::MAX, i32::MIN] {
                differential(
                    &mut fails,
                    &mut cap,
                    &c,
                    &r,
                    &format!("row9 name={name:?} default={d}"),
                    |imp| call_parse(imp, name, d),
                );
                n += 1;
            }
        }
    });

    // ------------------------------------------------------------------
    // Rows 10-13 — NULL pointer arguments must fail identically.
    // Each call runs in a forked child so the harness survives the signal.
    // ------------------------------------------------------------------
    row!(10, "err_10_null_env_name_segv", {
        let c_out = child_outcome(|| {
            let _ = unsafe { (c.parse_env_numeric)(std::ptr::null(), 5) };
        });
        let r_out = child_outcome(|| {
            let _ = unsafe { (r.parse_env_numeric)(std::ptr::null(), 5) };
        });
        cap.discard();
        if c_out != r_out {
            fails.push(format!(
                "MISMATCH [row10 parse_env_numeric(NULL, 5)] C = {c_out}, RUST = {r_out}"
            ));
        }
        n += 1;
    });

    row!(11, "err_11_null_flags_segv", {
        let c_out = child_outcome(|| unsafe { (c.init_config_from_env)(std::ptr::null_mut()) });
        let r_out = child_outcome(|| unsafe { (r.init_config_from_env)(std::ptr::null_mut()) });
        cap.discard();
        if c_out != r_out {
            fails.push(format!(
                "MISMATCH [row11 init_config_from_env(NULL)] C = {c_out}, RUST = {r_out}"
            ));
        }
        n += 1;
    });

    row!(12, "err_12_null_flags_segv", {
        for (v1, v2) in [(0, 0), (7, 9), (i32::MIN, i32::MAX)] {
            let c_out =
                child_outcome(|| {
                    let _ = unsafe { (c.perform_operation)(v1, v2, std::ptr::null_mut()) };
                });
            let r_out =
                child_outcome(|| {
                    let _ = unsafe { (r.perform_operation)(v1, v2, std::ptr::null_mut()) };
                });
            cap.discard();
            if c_out != r_out {
                fails.push(format!(
                    "MISMATCH [row12 perform_operation({v1},{v2},NULL)] C = {c_out}, RUST = {r_out}"
                ));
            }
            n += 1;
        }
    });

    row!(13, "err_13_null_flags_segv", {
        for v in [0, 1, -1, i32::MIN] {
            let c_out = child_outcome(|| {
                let _ = unsafe { (c.apply_bit_operations)(v, std::ptr::null_mut()) };
            });
            let r_out = child_outcome(|| {
                let _ = unsafe { (r.apply_bit_operations)(v, std::ptr::null_mut()) };
            });
            cap.discard();
            if c_out != r_out {
                fails.push(format!(
                    "MISMATCH [row13 apply_bit_operations({v},NULL)] C = {c_out}, RUST = {r_out}"
                ));
            }
            n += 1;
        }
    });

    // ------------------------------------------------------------------
    // Row 14 — perform_operation with every out-of-range flag bit pattern
    // (log_level 0..7 incl. the values init_config_from_env never produces,
    // reserved bit set, garbage padding).
    // ------------------------------------------------------------------
    row!(14, "err_14_flag_bit_patterns", {
        for low in 0..256u32 {
            for pad in [0x0000_0000u32, 0xFFFF_FF00, 0x8000_0100 & 0xFFFF_FF00] {
                let bits = pad | low;
                for (v1, v2) in [(1, 2), (i32::MAX, i32::MIN), (-7, -9)] {
                    differential(
                        &mut fails,
                        &mut cap,
                        &c,
                        &r,
                        &format!("row14 bits=0x{bits:08x} v=({v1},{v2})"),
                        |imp| call_perform(imp, v1, v2, bits),
                    );
                    n += 1;
                }
            }
        }
    });

    // ------------------------------------------------------------------
    // Row 15 — apply_bit_operations with every out-of-range flag pattern.
    // ------------------------------------------------------------------
    row!(15, "err_15_flag_bit_patterns", {
        for low in 0..256u32 {
            for pad in [0x0000_0000u32, 0xFFFF_FF00] {
                let bits = pad | low;
                for v in [0, 1, -1, i32::MAX, i32::MIN] {
                    differential(
                        &mut fails,
                        &mut cap,
                        &c,
                        &r,
                        &format!("row15 bits=0x{bits:08x} value={v}"),
                        |imp| call_apply(imp, v, bits),
                    );
                    n += 1;
                }
            }
        }
    });

    // ------------------------------------------------------------------
    // Row 16 — non-optimized branch overflow (val1 * log_level + val2/2).
    // ------------------------------------------------------------------
    row!(16, "err_16_perform_operation_overflow", {
        for log_level in 0..8u32 {
            let bits = flags(false, false, false, true, log_level);
            for v1 in [
                i32::MAX,
                i32::MIN,
                i32::MAX / 2,
                i32::MIN / 2,
                0x4000_0000,
                -0x4000_0000,
                715_827_883,
                -715_827_883,
            ] {
                for v2 in [0, i32::MAX, i32::MIN, 1, -1] {
                    differential(
                        &mut fails,
                        &mut cap,
                        &c,
                        &r,
                        &format!("row16 log_level={log_level} v=({v1},{v2})"),
                        |imp| call_perform(imp, v1, v2, bits),
                    );
                    n += 1;
                }
            }
        }
    });

    // ------------------------------------------------------------------
    // Row 17 — optimized branch overflow (val1 + val2).
    // ------------------------------------------------------------------
    row!(17, "err_17_add_overflow", {
        let bits = flags(false, false, true, true, 3);
        for (v1, v2) in [
            (i32::MAX, 1),
            (i32::MAX, i32::MAX),
            (i32::MIN, -1),
            (i32::MIN, i32::MIN),
            (1, i32::MAX),
            (-1, i32::MIN),
            (0x4000_0000, 0x4000_0000),
        ] {
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row17 v=({v1},{v2})"),
                |imp| call_perform(imp, v1, v2, bits),
            );
            n += 1;
        }
    });

    // ------------------------------------------------------------------
    // Row 18 — val2 == INT_MIN in `val2 / 2`.
    // ------------------------------------------------------------------
    row!(18, "err_18_val2_int_min_div", {
        for log_level in 0..8u32 {
            let bits = flags(false, false, false, true, log_level);
            for v1 in [0, 1, -1, i32::MIN, i32::MAX] {
                for v2 in [i32::MIN, i32::MIN + 1, -1, -3, 1, 3] {
                    differential(
                        &mut fails,
                        &mut cap,
                        &c,
                        &r,
                        &format!("row18 log_level={log_level} v=({v1},{v2})"),
                        |imp| call_perform(imp, v1, v2, bits),
                    );
                    n += 1;
                }
            }
        }
    });

    // ------------------------------------------------------------------
    // Row 19 — `value << 1` shifting into / past the sign bit.
    // ------------------------------------------------------------------
    row!(19, "err_19_shift_sign_overflow", {
        for cache in [false, true] {
            let bits = flags(true, false, false, cache, 3);
            for v in [
                0x4000_0000,
                0x4000_0001,
                0x7FFF_FFFF,
                -1,
                i32::MIN,
                i32::MIN + 1,
                -0x4000_0000,
                -0x4000_0001,
                0x3FFF_FFFF,
            ] {
                differential(
                    &mut fails,
                    &mut cap,
                    &c,
                    &r,
                    &format!("row19 cache={cache} value={v}"),
                    |imp| call_apply(imp, v, bits),
                );
                n += 1;
            }
        }
    });

    // ------------------------------------------------------------------
    // Row 20 — envy: param3 * multiplier overflow (multiplier from the env).
    // ------------------------------------------------------------------
    row!(20, "err_20_param3_mul_overflow", {
        for m in [
            "2147483647",
            "-2147483648",
            "65536",
            "1000000",
            "-1000000",
            "3",
        ] {
            put_env("PROG_MULTIPLIER", Some(m));
            for p3 in [i32::MAX, i32::MIN, 65537, -65537, 0x4000_0000, 3] {
                for p1 in [0, 5, -5] {
                    differential(
                        &mut fails,
                        &mut cap,
                        &c,
                        &r,
                        &format!("row20 mult={m:?} p3={p3} p1={p1}"),
                        |imp| call_envy(imp, p1, 2, p3, 0),
                    );
                    n += 1;
                }
            }
        }
        put_env("PROG_MULTIPLIER", None);
    });

    // ------------------------------------------------------------------
    // Row 21 — envy: param4 >> 2 with negative / boundary param4.
    // ------------------------------------------------------------------
    row!(21, "err_21_param4_arithmetic_shift", {
        for p4 in [
            i32::MIN,
            i32::MIN + 1,
            -1,
            -2,
            -3,
            -4,
            -5,
            i32::MAX,
            1,
            2,
            3,
            0x4000_0000,
        ] {
            for opt in [None, Some("1")] {
                put_env("PROG_OPTIMIZE", opt);
                differential(
                    &mut fails,
                    &mut cap,
                    &c,
                    &r,
                    &format!("row21 p4={p4} optimize={opt:?}"),
                    |imp| call_envy(imp, 1, 2, 0, p4),
                );
                n += 1;
            }
        }
        put_env("PROG_OPTIMIZE", None);
    });

    // ------------------------------------------------------------------
    // Row 22 — result < 0 restores the backup and returns param1.
    // Row 23 — ... even when param1 itself is negative.
    // ------------------------------------------------------------------
    row!(22, "err_22_negative_result_restores_backup", {
        put_env("PROG_BASE_OFFSET", Some("-2000000000"));
        for v in VERBOSE_STATES {
            put_env("PROG_VERBOSE", v);
            for p1 in [0, 1, 2, 7, 1000, i32::MAX, 0x4000_0000] {
                differential(
                    &mut fails,
                    &mut cap,
                    &c,
                    &r,
                    &format!("row22 verbose={v:?} p1={p1}"),
                    |imp| call_envy(imp, p1, 0, 0, 0),
                );
                n += 1;
            }
        }
        clear_prog_env();
    });

    row!(23, "err_23_negative_backup_returned", {
        put_env("PROG_BASE_OFFSET", Some("-2000000000"));
        for v in VERBOSE_STATES {
            put_env("PROG_VERBOSE", v);
            for p1 in [-1, -2, -1000, i32::MIN, -0x4000_0000] {
                differential(
                    &mut fails,
                    &mut cap,
                    &c,
                    &r,
                    &format!("row23 verbose={v:?} p1={p1}"),
                    |imp| call_envy(imp, p1, 0, 0, 0),
                );
                n += 1;
            }
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Row 24 — the rejections above, but reached from inside `envy`
    // (including the stderr warning it forwards).
    // ------------------------------------------------------------------
    row!(24, "err_24_envy_env_rejection_defaults", {
        for bad in [
            None,
            Some(""),
            Some("abc"),
            Some("1,2"),
            Some("3;4"),
            Some("9999999999"),
            Some(",;"),
        ] {
            for which in 0..3 {
                clear_prog_env();
                match which {
                    0 => {
                        put_env("PROG_BASE_OFFSET", bad);
                    }
                    1 => {
                        put_env("PROG_MULTIPLIER", bad);
                    }
                    _ => {
                        put_env("PROG_BASE_OFFSET", bad);
                        put_env("PROG_MULTIPLIER", bad);
                    }
                }
                for verbose in [None, Some("1")] {
                    put_env("PROG_VERBOSE", verbose);
                    differential(
                        &mut fails,
                        &mut cap,
                        &c,
                        &r,
                        &format!("row24 bad={bad:?} which={which} verbose={verbose:?}"),
                        |imp| call_envy(imp, 3, 5, 7, 9),
                    );
                    n += 1;
                }
            }
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Row 25/26 — the two "colon not found" branches are unreachable:
    // the messages that depend on them must always be printed.
    // ------------------------------------------------------------------
    row!(25, "err_25_colon_always_found", {
        clear_prog_env();
        put_env("PROG_VERBOSE", Some("1"));
        for p1 in [0, 1, -1, i32::MAX, i32::MIN, 123456] {
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row25 p1={p1}"),
                |imp| call_envy(imp, p1, p1, p1, p1),
            );
            n += 1;
            // Extra (non-differential) check: prove the branch really is taken.
            let _ = call_envy(&c, p1, p1, p1, p1);
            let (out, _err) = cap.take();
            if !String::from_utf8_lossy(&out).contains("Found colon at position: 6\n") {
                fails.push(format!(
                    "row25: C did not report the first colon for p1={p1}: {:?}",
                    String::from_utf8_lossy(&out)
                ));
            }
            n += 1;
        }
        clear_prog_env();
    });

    row!(26, "err_26_second_colon_always_found", {
        clear_prog_env();
        put_env("PROG_DEBUG", Some("1"));
        for p1 in [0, 1, -1, i32::MAX, i32::MIN, 987654] {
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row26 p1={p1}"),
                |imp| call_envy(imp, p1, p1, p1, p1),
            );
            n += 1;
            let _ = call_envy(&c, p1, p1, p1, p1);
            let (out, _err) = cap.take();
            if !String::from_utf8_lossy(&out)
                .contains("Debug: Result string format validated\n")
            {
                fails.push(format!(
                    "row26: C did not validate the result string for p1={p1}: {:?}",
                    String::from_utf8_lossy(&out)
                ));
            }
            n += 1;
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Row 27 — VERBOSE/DEBUG present but without a '1' -> flag cleared.
    // ------------------------------------------------------------------
    row!(27, "err_27_verbose_debug_need_a_one", {
        for v in [
            None,
            Some(""),
            Some("0"),
            Some("true"),
            Some("yes"),
            Some("2"),
            Some("11"),
            Some("x1"),
            Some("1"),
            Some("0001"),
            Some("-1"),
        ] {
            put_env("PROG_VERBOSE", v);
            put_env("PROG_DEBUG", v);
            for prefill in [0u32, 0xFFFF_FFFF, 0xA5A5_A5A5] {
                differential(
                    &mut fails,
                    &mut cap,
                    &c,
                    &r,
                    &format!("row27 V=D={v:?} prefill=0x{prefill:08x}"),
                    |imp| call_init(imp, prefill),
                );
                n += 1;
            }
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row27 envy V=D={v:?}"),
                |imp| call_envy(imp, 11, 22, 33, 44),
            );
            n += 1;
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Row 28 — OPTIMIZE: presence alone sets the flag, value is ignored.
    // ------------------------------------------------------------------
    row!(28, "err_28_optimize_presence_only", {
        for o in [
            None,
            Some(""),
            Some("0"),
            Some("no"),
            Some("false"),
            Some("1"),
            Some("-1"),
        ] {
            put_env("PROG_OPTIMIZE", o);
            for prefill in [0u32, 0xFFFF_FFFF] {
                differential(
                    &mut fails,
                    &mut cap,
                    &c,
                    &r,
                    &format!("row28 O={o:?} prefill=0x{prefill:08x}"),
                    |imp| call_init(imp, prefill),
                );
                n += 1;
            }
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row28 envy O={o:?}"),
                |imp| call_envy(imp, 5, 6, 7, 8),
            );
            n += 1;
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Row 29 — padding bits 8..31 must be preserved by the read-modify-write.
    // ------------------------------------------------------------------
    row!(29, "err_29_padding_bits_preserved", {
        for v in [None, Some("1")] {
            for d in [None, Some("1")] {
                for o in [None, Some("")] {
                    put_env("PROG_VERBOSE", v);
                    put_env("PROG_DEBUG", d);
                    put_env("PROG_OPTIMIZE", o);
                    for _ in 0..40 {
                        let prefill = rng.next_u32();
                        differential(
                            &mut fails,
                            &mut cap,
                            &c,
                            &r,
                            &format!("row29 V={v:?} D={d:?} O={o:?} prefill=0x{prefill:08x}"),
                            |imp| call_init(imp, prefill),
                        );
                        n += 1;
                        // Extra check: the C implementation itself must leave
                        // bits 8..31 untouched (documents the expectation).
                        let got = call_init(&c, prefill) as u32;
                        cap.discard();
                        if got & 0xFFFF_FF00 != prefill & 0xFFFF_FF00 {
                            fails.push(format!(
                                "row29: C changed the padding bits: prefill=0x{prefill:08x} -> 0x{got:08x}"
                            ));
                        }
                        n += 1;
                    }
                }
            }
        }
        clear_prog_env();
    });

    // ------------------------------------------------------------------
    // Row 30 — misaligned `struct ConfigFlags*` (offsets 1, 2, 3 into a byte
    // buffer).  x86-64 allows the unaligned access, so the call must succeed
    // and both implementations must touch exactly the same bytes.
    // ------------------------------------------------------------------
    row!(30, "err_30_misaligned_flags_pointer", {
        for off in [0usize, 1, 2, 3] {
            for bits in [0x0000_0000u32, 0xFFFF_FFFF, 0x0000_003B, 0xDEAD_BEEF] {
                for (v1, v2) in [(1, 2), (i32::MAX, i32::MIN), (-7, 9)] {
                    differential(
                        &mut fails,
                        &mut cap,
                        &c,
                        &r,
                        &format!("row30 perform off={off} bits=0x{bits:08x} v=({v1},{v2})"),
                        |imp| call_perform_unaligned(imp, v1, v2, bits, off),
                    );
                    n += 1;
                }
                for v in [0, -1, i32::MIN, 0x4000_0000] {
                    differential(
                        &mut fails,
                        &mut cap,
                        &c,
                        &r,
                        &format!("row30 apply off={off} bits=0x{bits:08x} value={v}"),
                        |imp| call_apply_unaligned(imp, v, bits, off),
                    );
                    n += 1;
                }
                for (verbose, debug, opt) in [
                    (None, None, None),
                    (Some("1"), Some("1"), Some("")),
                    (Some("0"), Some("1"), None),
                ] {
                    put_env("PROG_VERBOSE", verbose);
                    put_env("PROG_DEBUG", debug);
                    put_env("PROG_OPTIMIZE", opt);
                    differential(
                        &mut fails,
                        &mut cap,
                        &c,
                        &r,
                        &format!("row30 init off={off} bits=0x{bits:08x} env={verbose:?}/{debug:?}/{opt:?}"),
                        |imp| call_init_unaligned(imp, bits, off),
                    );
                    n += 1;
                }
                clear_prog_env();
            }
        }
    });

    // ------------------------------------------------------------------
    // Row 31 — non-UTF-8 and very large environment values.
    // ------------------------------------------------------------------
    row!(31, "err_31_non_utf8_and_huge_values", {
        let raw_values: [&[u8]; 10] = [
            b"\xff\xfe",
            b"1\xff",
            b"\xff1",
            b"\x80,\x81",
            b"\x80;\x81",
            b"-\xff5",
            b"\xc3",
            b"12\xff34",
            b"\x01\x02\x03",
            b"\x7f",
        ];
        for v in raw_values {
            if !put_env_bytes("PROG_BASE_OFFSET", v) {
                fails.push(format!("row31: setenv failed for {v:?}"));
                continue;
            }
            for d in [0, -1, 64] {
                differential(
                    &mut fails,
                    &mut cap,
                    &c,
                    &r,
                    &format!("row31 raw value={v:?} default={d}"),
                    |imp| call_parse(imp, &base, d),
                );
                n += 1;
            }
        }

        // Huge values: 64 KiB of digits, with the poison character (if any) in
        // the very last byte so the whole string has to be scanned.
        let digits = "1234567890".repeat(6554); // 65540 bytes
        for (tag, v) in [
            ("digits", digits.clone()),
            ("digits+comma", format!("{digits},")),
            ("digits+semicolon", format!("{digits};")),
            ("comma+digits", format!(",{digits}")),
            ("spaces+digits", format!("{}{}", " ".repeat(4096), digits)),
        ] {
            put_env("PROG_MULTIPLIER", Some(&v));
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row31 huge {tag} (len={})", v.len()),
                |imp| call_parse(imp, &mult, 10),
            );
            n += 1;
            // ... and through `envy`, where the warning is printed.
            differential(
                &mut fails,
                &mut cap,
                &c,
                &r,
                &format!("row31 huge {tag} via envy"),
                |imp| call_envy(imp, 3, 4, 5, 6),
            );
            n += 1;
        }
        clear_prog_env();
    });

    drop(cap);
    rows.print();
    rows.assert_covers(&(1..=31u32).collect::<Vec<u32>>());
    report(fails, n, "phase C (error paths)");
}
