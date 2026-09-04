// Phase C — error/rejection-path differential tests. One test per ERRORS.md row.
mod common;

use common::*;
use std::ffi::{c_int, CString};

const VAR: &str = "DIFF_ERR_VAR";

fn pen(imp: &Impl, name: &str, dflt: c_int) -> Call {
    let n = CString::new(name).unwrap();
    call(|| unsafe { (imp.parse_env_numeric)(n.as_ptr(), dflt) })
}

fn check_pen(ctx: &str, value: Option<&str>, dflt: c_int) -> Call {
    let (p, _g) = pair();
    apply_env(VAR, value);
    let c = pen(&p.c, VAR, dflt);
    let r = pen(&p.rs, VAR, dflt);
    unset_env(VAR);
    assert_same(&format!("{ctx} value={value:?} default={dflt}"), &c, &r);
    c
}

// ---------------------------------------------------------------------------
// Harness sanity: the fd capture must really observe library output, otherwise
// every "outputs match" assertion in this suite would be vacuous.
// ---------------------------------------------------------------------------

#[test]
fn sanity_00_capture_observes_library_output() {
    // stderr: the comma warning.
    let c = check_pen("sanity/comma", Some("1,2"), 5);
    assert_eq!(
        String::from_utf8_lossy(&c.output.err),
        format!("Warning: Invalid character in {VAR}\n"),
        "the capture harness did not observe the C warning"
    );
    assert!(c.output.out.is_empty());

    // stdout: the verbose + debug chatter of `envy`.
    let (p, _g) = pair();
    set_env("PROG_VERBOSE", "1");
    set_env("PROG_DEBUG", "1");
    let out = call(|| unsafe { (p.c.envy)(1, 2, 3, 4) });
    let out_r = call(|| unsafe { (p.rs.envy)(1, 2, 3, 4) });
    clear_prog_env();
    let s = String::from_utf8_lossy(&out.output.out).into_owned();
    for expected in [
        "Verbose mode enabled\n",
        "Base offset: 64 (from octal 0100)\n",
        "Multiplier: 10 (from octal 012)\n",
        "Debug: Created state backup using memcpy\n",
        "Debug: Backup base_value = 1\n",
        "Debug: operation_mode = 755 (octal)\n",
        "Found colon at position: 6\n",
        "Debug: Result string format validated\n",
        "Final result: ",
    ] {
        assert!(s.contains(expected), "capture missed {expected:?}; got {s:?}");
    }
    assert_same("sanity/envy-verbose-debug", &out, &out_r);
}

// ---------------------------------------------------------------------------
// rows 1..9 — parse_env_numeric
// ---------------------------------------------------------------------------

#[test]
fn err_01_unset_var_returns_default() {
    for d in BOUNDARIES {
        let c = check_pen("err01", None, d);
        assert_eq!(c.ret, d, "unset variable must return default_val verbatim");
        assert!(c.output.out.is_empty() && c.output.err.is_empty());
    }
}

#[test]
fn err_02_comma_returns_default_and_warns() {
    for d in BOUNDARIES {
        for v in ["1,2", ",", "abc,def", "1,", ",1"] {
            let c = check_pen("err02", Some(v), d);
            assert_eq!(c.ret, d, "comma must yield default_val");
            assert_eq!(
                String::from_utf8_lossy(&c.output.err),
                format!("Warning: Invalid character in {VAR}\n")
            );
        }
    }
}

#[test]
fn err_03_semicolon_returns_default_and_warns() {
    for d in BOUNDARIES {
        for v in ["1;2", ";", "abc;def", "1;", ";1"] {
            let c = check_pen("err03", Some(v), d);
            assert_eq!(c.ret, d, "semicolon must yield default_val");
            assert_eq!(
                String::from_utf8_lossy(&c.output.err),
                format!("Warning: Semicolon found in {VAR}\n")
            );
        }
    }
}

#[test]
fn err_04_comma_wins_over_semicolon() {
    for v in ["1,2;3", "1;2,3", ",;", ";,"] {
        let c = check_pen("err04", Some(v), 77);
        assert_eq!(c.ret, 77);
        assert_eq!(
            String::from_utf8_lossy(&c.output.err),
            format!("Warning: Invalid character in {VAR}\n"),
            "the comma check runs first, so only that warning may appear"
        );
    }
}

#[test]
fn err_05_separator_positions() {
    let mut rng = Rng::new(SEED ^ 5);
    for sep in [',', ';'] {
        for pos in 0..6usize {
            let mut s: Vec<char> = "12345".chars().collect();
            s.insert(pos.min(5), sep);
            let v: String = s.into_iter().collect();
            check_pen("err05", Some(&v), rng.interesting_i32());
        }
        check_pen("err05/alone", Some(&sep.to_string()), 3);
        check_pen("err05/repeat", Some(&sep.to_string().repeat(9)), 3);
    }
}

#[test]
fn err_06_non_numeric_atoi_zero() {
    for v in ["abc", "", " ", "+-3", "-+3", ".", "e", "\n", "\t\t", "%d"] {
        let c = check_pen("err06", Some(v), 12345);
        assert_eq!(c.ret, 0, "atoi({v:?}) must be 0 — default_val is NOT used");
        assert!(c.output.err.is_empty());
    }
}

#[test]
fn err_07_atoi_overflow() {
    for v in [
        "99999999999999",
        "-99999999999999",
        "2147483648",
        "-2147483649",
        "9223372036854775808",
        "-9223372036854775809",
        "184467440737095516160",
    ] {
        // Only the differential equality is asserted: glibc's overflow result is
        // whatever it is, and the Rust translation must reproduce it exactly.
        check_pen("err07", Some(v), 999);
    }
}

#[test]
fn err_08_null_env_name_same_signal() {
    let (p, _g) = pair();
    let sc = child_status(|| unsafe {
        (p.c.parse_env_numeric)(std::ptr::null(), 7);
    });
    let sr = child_status(|| unsafe {
        (p.rs.parse_env_numeric)(std::ptr::null(), 7);
    });
    assert_same_fatal("err08/NULL env_name", sc, sr);
}

#[test]
fn err_09_empty_name_returns_default() {
    let (p, _g) = pair();
    for d in BOUNDARIES {
        let c = pen(&p.c, "", d);
        let r = pen(&p.rs, "", d);
        assert_same("err09", &c, &r);
        assert_eq!(c.ret, d, "getenv(\"\") is NULL ⇒ default_val");
    }
    // A name that is merely absent behaves the same way.
    for name in ["THIS_VAR_DOES_NOT_EXIST_12345", "=", " "] {
        let c = pen(&p.c, name, -3);
        let r = pen(&p.rs, name, -3);
        assert_same("err09/absent", &c, &r);
        assert_eq!(c.ret, -3);
    }
}

// ---------------------------------------------------------------------------
// rows 10..13 — init_config_from_env
// ---------------------------------------------------------------------------

#[test]
fn err_10_init_null_flags_same_signal() {
    let (p, _g) = pair();
    clear_prog_env();
    let sc = child_status(|| unsafe { (p.c.init_config_from_env)(std::ptr::null_mut()) });
    let sr = child_status(|| unsafe { (p.rs.init_config_from_env)(std::ptr::null_mut()) });
    assert_same_fatal("err10/NULL flags", sc, sr);
    // ...and with the environment set, i.e. writing a 1 bit rather than a 0 bit.
    set_env("PROG_VERBOSE", "1");
    set_env("PROG_DEBUG", "1");
    set_env("PROG_OPTIMIZE", "1");
    let sc = child_status(|| unsafe { (p.c.init_config_from_env)(std::ptr::null_mut()) });
    let sr = child_status(|| unsafe { (p.rs.init_config_from_env)(std::ptr::null_mut()) });
    clear_prog_env();
    assert_same_fatal("err10/NULL flags, env set", sc, sr);
}

fn init_pair(prefill: [u8; 4]) -> (Flags, Flags) {
    let (p, _g) = pair();
    let mut fc = Flags(prefill);
    let mut fr = Flags(prefill);
    let ((), oc) = capture(|| unsafe { (p.c.init_config_from_env)(fc.as_ptr()) });
    let ((), or) = capture(|| unsafe { (p.rs.init_config_from_env)(fr.as_ptr()) });
    assert_same_flags("init_pair", (&fc, &oc), (&fr, &or));
    (fc, fr)
}

#[test]
fn err_11_verbose_without_one_rejected() {
    for v in ["yes", "0", "", "true", "on", "VERBOSE", "2", "\u{c4}1"] {
        clear_prog_env();
        set_env("PROG_VERBOSE", v);
        let (fc, _) = init_pair([0; 4]);
        let expect_set = v.contains('1');
        assert_eq!(
            fc.0[0] & F_VERBOSE != 0,
            expect_set,
            "PROG_VERBOSE={v:?} verbose bit"
        );
    }
    clear_prog_env();
}

#[test]
fn err_12_debug_without_one_rejected() {
    for v in ["yes", "0", "", "true", "on", "DEBUG", "2", "verbose"] {
        clear_prog_env();
        set_env("PROG_DEBUG", v);
        let (fc, _) = init_pair([0; 4]);
        assert_eq!(fc.0[0] & F_DEBUG != 0, v.contains('1'), "PROG_DEBUG={v:?} debug bit");
    }
    clear_prog_env();
}

#[test]
fn err_13_optimize_empty_is_set() {
    for (v, expect) in [
        (None, false),
        (Some(""), true),
        (Some("0"), true),
        (Some("no"), true),
        (Some("false"), true),
    ] {
        clear_prog_env();
        apply_env("PROG_OPTIMIZE", v);
        let (fc, _) = init_pair([0; 4]);
        assert_eq!(
            fc.0[0] & F_OPTIMIZE != 0,
            expect,
            "PROG_OPTIMIZE={v:?} is a presence-only test"
        );
    }
    clear_prog_env();
}

// ---------------------------------------------------------------------------
// rows 14..17 — perform_operation
// ---------------------------------------------------------------------------

fn check_perform(ctx: &str, v1: c_int, v2: c_int, flags: Flags) -> Call {
    let (p, _g) = pair();
    let mut fc = flags;
    let mut fr = flags;
    let c = call(|| unsafe { (p.c.perform_operation)(v1, v2, fc.as_ptr()) });
    let r = call(|| unsafe { (p.rs.perform_operation)(v1, v2, fr.as_ptr()) });
    assert_same(&format!("{ctx} ({v1},{v2}) {flags:?}"), &c, &r);
    assert_eq!(fc, fr, "{ctx}: flags unit diverged");
    c
}

#[test]
fn err_14_perform_null_flags_same_signal() {
    let (p, _g) = pair();
    let sc = child_status(|| unsafe {
        (p.c.perform_operation)(1, 2, std::ptr::null_mut());
    });
    let sr = child_status(|| unsafe {
        (p.rs.perform_operation)(1, 2, std::ptr::null_mut());
    });
    assert_same_fatal("err14/NULL flags in perform_operation", sc, sr);
}

#[test]
fn err_15_log_level_full_range() {
    // No range check exists: all 8 values a 3-bit field can hold are used as-is.
    let mut rng = Rng::new(SEED ^ 15);
    for log in 0u8..8 {
        for debug in [false, true] {
            let flags = Flags::new(false, debug, false, true, log);
            for _ in 0..20 {
                check_perform("err15", rng.interesting_i32(), rng.interesting_i32(), flags);
            }
            let c = check_perform("err15/fixed", 7, 9, flags);
            assert_eq!(c.ret, 7 * log as i32 + 4, "val1*log_level + val2/2");
        }
    }
}

#[test]
fn err_16_perform_signed_overflow() {
    let extremes = [i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1, 0x4000_0000u32 as i32, -1, 1];
    for log in 0u8..8 {
        for optimize in [false, true] {
            let flags = Flags::new(false, false, optimize, true, log);
            for a in extremes {
                for b in extremes {
                    check_perform("err16", a, b, flags);
                }
            }
        }
    }
}

#[test]
fn err_17_int_min_div_two() {
    for log in 0u8..8 {
        let flags = Flags::new(false, false, false, true, log);
        let c = check_perform("err17", 0, i32::MIN, flags);
        assert_eq!(c.ret, -1073741824, "INT_MIN/2 truncates toward zero");
        check_perform("err17/neg-odd", 1, -3, flags);
        check_perform("err17/neg-even", 1, -4, flags);
        check_perform("err17/max", 0, i32::MAX, flags);
    }
    // -3/2 must truncate toward zero (-1), not floor (-2).
    let flags = Flags::new(false, false, false, true, 0);
    assert_eq!(check_perform("err17/trunc", 0, -3, flags).ret, -1);
}

// ---------------------------------------------------------------------------
// rows 18..20 — apply_bit_operations
// ---------------------------------------------------------------------------

fn check_bitops(ctx: &str, value: c_int, flags: Flags) -> Call {
    let (p, _g) = pair();
    let mut fc = flags;
    let mut fr = flags;
    let c = call(|| unsafe { (p.c.apply_bit_operations)(value, fc.as_ptr()) });
    let r = call(|| unsafe { (p.rs.apply_bit_operations)(value, fr.as_ptr()) });
    assert_same(&format!("{ctx} value={value} {flags:?}"), &c, &r);
    assert_eq!(fc, fr, "{ctx}: flags unit diverged");
    c
}

#[test]
fn err_18_bitops_null_flags_same_signal() {
    let (p, _g) = pair();
    let sc = child_status(|| unsafe {
        (p.c.apply_bit_operations)(42, std::ptr::null_mut());
    });
    let sr = child_status(|| unsafe {
        (p.rs.apply_bit_operations)(42, std::ptr::null_mut());
    });
    assert_same_fatal("err18/NULL flags in apply_bit_operations", sc, sr);
}

#[test]
fn err_19_shift_overflow() {
    let vals = [
        0x4000_0000u32 as i32,
        0x4000_0001u32 as i32,
        0x7FFF_FFFF,
        -1,
        -2,
        i32::MIN,
        i32::MIN + 1,
        -0x4000_0000,
        0x2000_0000,
    ];
    for cache in [false, true] {
        for v in vals {
            check_bitops("err19", v, Flags::new(true, false, false, cache, 3));
        }
    }
}

#[test]
fn err_20_int_min_shift() {
    let c = check_bitops("err20", i32::MIN, Flags::new(true, false, false, true, 3));
    assert_eq!(c.ret, 15, "(INT_MIN << 1) | 0x0F");
    let c = check_bitops("err20/nocache", i32::MIN, Flags::new(true, false, false, false, 3));
    assert_eq!(c.ret, 0, "INT_MIN << 1");
}

// ---------------------------------------------------------------------------
// rows 21..30 — envy
// ---------------------------------------------------------------------------

fn check_envy(ctx: &str, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> Call {
    let (p, _g) = pair();
    let c = call(|| unsafe { (p.c.envy)(p1, p2, p3, p4) });
    let r = call(|| unsafe { (p.rs.envy)(p1, p2, p3, p4) });
    assert_same(&format!("{ctx} params=({p1},{p2},{p3},{p4})"), &c, &r);
    c
}

/// Compares C, Rust *and* the independent reference model.
fn check_envy_modeled(ctx: &str, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> (Call, bool) {
    let c = check_envy(ctx, p1, p2, p3, p4);
    let (expected, restored) = model_envy(&Cfg::default(), p1, p2, p3, p4);
    assert_eq!(c.ret, expected, "{ctx}: C disagrees with the reference model");
    (c, restored)
}

#[test]
fn err_21_negative_result_restores_base() {
    clear_prog_env();
    let (c, restored) = check_envy_modeled("err21", 5000, -1_000_000, 0, 0);
    assert!(restored, "these inputs must take the restore branch");
    assert_eq!(c.ret, 5000, "the restore branch returns state.base_value == param1");
    for p1 in [1, 2, 12345, 5000, 999] {
        let (c, restored) = check_envy_modeled("err21/loop", p1, -1_000_000, -1_000_000, -1_000_000);
        assert!(restored, "p1={p1}: restore branch expected");
        assert_eq!(c.ret, p1);
    }
}

#[test]
fn err_22_restore_can_return_negative() {
    clear_prog_env();
    // param1 < 0 ⇒ the restored base_value is itself negative and is returned
    // as-is: the `result < 0` guard is not re-evaluated after the restore.
    for p1 in [-1, -5000, -64, -2, -1_000_000] {
        let (c, restored) = check_envy_modeled("err22", p1, -1_000_000, 0, 0);
        assert!(restored, "p1={p1}: restore branch expected");
        assert_eq!(c.ret, p1, "restored value is returned even when negative");
        assert!(c.ret < 0, "envy really can return a negative value");
    }
    let (c, restored) = check_envy_modeled("err22/zero", 0, -1_000_000, 0, 0);
    assert!(restored);
    assert_eq!(c.ret, 0);
}

#[test]
fn err_23_param3_zero_skips_term() {
    clear_prog_env();
    // A term large enough to survive the final `| 0x0F`: multiplier is 012 == 10,
    // so param3 == 1000 contributes 10000.
    let (with, _) = check_envy_modeled("err23/with", 10, 10, 1000, 0);
    let (without, _) = check_envy_modeled("err23/without", 10, 10, 0, 0);
    assert_ne!(with.ret, without.ret, "param3 != 0 must add param3*multiplier");
    assert_eq!(with.ret - without.ret, 10000, "param3 * multiplier(=012=10)");
    // param3 == 0 must skip the term entirely, which for multiplier 10 is the
    // same as adding zero — verified against the model above.
    let mut rng = Rng::new(SEED ^ 23);
    for _ in 0..30 {
        check_envy_modeled("err23/random", rng.interesting_i32(), rng.interesting_i32(), 0, 0);
    }
}

#[test]
fn err_24_param4_zero_skips_term() {
    clear_prog_env();
    let (with, _) = check_envy_modeled("err24/with", 10, 10, 0, 4096);
    let (without, _) = check_envy_modeled("err24/without", 10, 10, 0, 0);
    assert_eq!(with.ret - without.ret, 1024, "param4 != 0 must add param4 >> 2");
    // param4 ∈ {1,2,3}: the *guard* is taken but the shifted term is 0, and the
    // low nibble is overwritten by `| 0x0F` anyway.
    for p4 in [1, 2, 3] {
        let (c, _) = check_envy_modeled("err24/small", 10, 10, 0, p4);
        assert_eq!(c.ret, without.ret);
    }
}

#[test]
fn err_25_negative_param4_arith_shift() {
    clear_prog_env();
    for p4 in [-1, -2, -3, -4, -5, -8, -9, i32::MIN, -1024, -1023, -4096, -4097] {
        check_envy_modeled("err25", 100000, 0, 0, p4);
    }
    // Arithmetic (not logical) shift: -4096 >> 2 == -1024, and -1 >> 2 == -1
    // (rounds toward −∞, so even tiny negatives lower the result).
    let (base, _) = check_envy_modeled("err25/base", 100000, 0, 0, 0);
    let (neg, _) = check_envy_modeled("err25/-4096", 100000, 0, 0, -4096);
    assert_eq!(neg.ret, base.ret - 1024, "arithmetic shift of a negative param4");
    let (tiny, _) = check_envy_modeled("err25/-1", 100000, 0, 0, -1);
    assert!(tiny.ret <= base.ret, "-1 >> 2 == -1, never 0x3FFFFFFF");
}

#[test]
fn err_26_colon_guard_always_taken() {
    // The guard at lib.c:160 can never fail; with verbose set, the position of
    // the first colon is printed and must be byte-identical.
    clear_prog_env();
    set_env("PROG_VERBOSE", "1");
    let mut rng = Rng::new(SEED ^ 26);
    for _ in 0..40 {
        let c = check_envy(
            "err26",
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        let s = String::from_utf8_lossy(&c.output.out).into_owned();
        assert!(s.contains("Found colon at position: 6\n"), "got {s:?}");
    }
    clear_prog_env();
}

#[test]
fn err_27_second_colon_guard() {
    clear_prog_env();
    set_env("PROG_DEBUG", "1");
    let mut rng = Rng::new(SEED ^ 27);
    for _ in 0..40 {
        let c = check_envy(
            "err27",
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        let s = String::from_utf8_lossy(&c.output.out).into_owned();
        assert!(s.contains("Debug: Result string format validated\n"), "got {s:?}");
    }
    clear_prog_env();
}

#[test]
fn err_28_no_snprintf_truncation() {
    // The longest possible formatted result is "Result:-2147483648:Complete"
    // (27 chars + NUL) which fits in BUFFER_SIZE == 256, so the second colon is
    // always present. Drive the widest results and verify both agree.
    clear_prog_env();
    set_env("PROG_VERBOSE", "1");
    set_env("PROG_DEBUG", "1");
    set_env("PROG_BASE_OFFSET", "-2147483648");
    for p1 in [i32::MIN, i32::MAX, 0] {
        for p2 in [i32::MIN, i32::MAX, 0] {
            let c = check_envy("err28", p1, p2, i32::MIN, i32::MAX);
            let s = String::from_utf8_lossy(&c.output.out).into_owned();
            assert!(s.contains("Debug: Result string format validated\n"), "got {s:?}");
        }
    }
    clear_prog_env();
}

#[test]
fn err_29_octal_defaults_on_rejection() {
    clear_prog_env();
    let plain = check_envy("err29/default", 10, 10, 1, 0);
    for bad in ["1,2", "3;4", ",", ";"] {
        set_env("PROG_BASE_OFFSET", bad);
        set_env("PROG_MULTIPLIER", bad);
        let c = check_envy("err29", 10, 10, 1, 0);
        assert_eq!(c.ret, plain.ret, "rejected values must fall back to 0100 / 012");
        assert!(!c.output.err.is_empty(), "a warning must be printed for {bad:?}");
        clear_prog_env();
    }
    // And the defaults really are OCTAL: 0100 == 64, 012 == 10.
    set_env("PROG_BASE_OFFSET", "64");
    set_env("PROG_MULTIPLIER", "10");
    let explicit = check_envy("err29/explicit-decimal", 10, 10, 1, 0);
    clear_prog_env();
    assert_eq!(explicit.ret, plain.ret);
    set_env("PROG_BASE_OFFSET", "100");
    let wrong = check_envy("err29/decimal-100", 10, 10, 1, 0);
    clear_prog_env();
    assert_ne!(wrong.ret, plain.ret, "0100 is 64, not 100");
}

#[test]
fn err_30_envy_extreme_overflow() {
    let mut rng = Rng::new(SEED ^ 30);
    for mult in ["2147483647", "-2147483648", "99999999999999", "0", "-1"] {
        for offset in ["2147483647", "-2147483648", "99999999999999", "0", "-1"] {
            clear_prog_env();
            set_env("PROG_MULTIPLIER", mult);
            set_env("PROG_BASE_OFFSET", offset);
            for p1 in [i32::MIN, i32::MAX] {
                for p4 in [i32::MIN, i32::MAX] {
                    check_envy("err30", p1, p1, p4, p4);
                }
            }
            for _ in 0..10 {
                check_envy(
                    "err30/random",
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                );
            }
        }
    }
    clear_prog_env();
}

// ---------------------------------------------------------------------------
// rows 31..32 — out-of-range "enum"/flag values crossing the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn err_31_garbage_upper_bits_ignored() {
    let mut rng = Rng::new(SEED ^ 31);

    // (a) All 32 bits set — no valid combination of the declared bit-fields, but
    //     a perfectly legal `int` for a C caller to hand over.
    for value in [0, 1, -1, i32::MIN, i32::MAX, 12345, -12345] {
        check_bitops("err31/all-ones", value, Flags::raw(0xFF, [0xFF, 0xFF, 0xFF]));
        check_perform("err31/all-ones", value, value, Flags::raw(0xFF, [0xFF, 0xFF, 0xFF]));
    }

    // (b) Randomized garbage in bits 8..31 must never change the result and must
    //     never be modified.
    for _ in 0..200 {
        let byte0 = rng.next_u32() as u8;
        let upper = [rng.next_u32() as u8, rng.next_u32() as u8, rng.next_u32() as u8];
        let dirty = Flags::raw(byte0, upper);
        let clean = Flags::raw(byte0, [0, 0, 0]);
        let v = rng.interesting_i32();
        let a = check_bitops("err31/dirty", v, dirty);
        let b = check_bitops("err31/clean", v, clean);
        assert_eq!(a.ret, b.ret, "bits 8..31 must not influence apply_bit_operations");
        let a = check_perform("err31/dirty", v, v, dirty);
        let b = check_perform("err31/clean", v, v, clean);
        assert_eq!(a.ret, b.ret, "bits 8..31 must not influence perform_operation");
    }

    // (c) init_config_from_env on a unit whose upper bits are garbage: whatever
    //     the C leaves there, the Rust must leave the same.
    for prefill in [[0xFFu8; 4], [0xAA; 4], [0x00; 4], [0x5A; 4]] {
        for v in [None, Some("1")] {
            clear_prog_env();
            apply_env("PROG_VERBOSE", v);
            apply_env("PROG_DEBUG", v);
            apply_env("PROG_OPTIMIZE", v);
            init_pair(prefill);
        }
    }
    clear_prog_env();
}

#[test]
fn err_32_no_length_arguments_boundary_documented() {
    // The API has no length/size parameter anywhere (see ERRORS.md row 32); the
    // only fixed-size buffer is `envy`'s 256-byte `buffer`, exercised by
    // err_28_no_snprintf_truncation. This test pins the fact that even the most
    // extreme inputs stay well inside it.
    clear_prog_env();
    set_env("PROG_VERBOSE", "1");
    let c = check_envy("err32", i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    let s = String::from_utf8_lossy(&c.output.out).into_owned();
    assert!(s.contains("Found colon at position: 6\n"));
    clear_prog_env();
}
