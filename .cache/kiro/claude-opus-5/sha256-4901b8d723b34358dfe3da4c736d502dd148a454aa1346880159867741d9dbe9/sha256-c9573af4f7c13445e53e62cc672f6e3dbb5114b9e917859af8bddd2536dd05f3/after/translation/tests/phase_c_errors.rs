//! Phase C — error / rejection-path differential tests, one `#[test]` per row
//! of ERRORS.md.
//!
//! Every test constructs the exact invalid input, calls BOTH `.so` files through
//! their exported symbols, and asserts they reject identically — the same
//! returned sentinel/default AND the same warning bytes on stderr, not merely
//! "both failed somehow".

mod common;

use common::*;
use std::ffi::{CString, c_int};

fn call_parse(lib: &Lib, name: &str, default_val: i32) -> i64 {
    let n = CString::new(name).unwrap();
    unsafe { (lib.parse_env_numeric)(n.as_ptr(), default_val) as i64 }
}

fn call_init(lib: &Lib, initial: u32) -> i64 {
    let mut storage: u32 = initial;
    unsafe { (lib.init_config_from_env)(&mut storage) };
    storage as i64
}

fn call_perform(lib: &Lib, val1: i32, val2: i32, flags: u32) -> i64 {
    let mut storage = flags;
    let r = unsafe { (lib.perform_operation)(val1, val2, &mut storage) };
    ((storage as i64) << 32) | (r as u32 as i64)
}

fn call_apply(lib: &Lib, value: i32, flags: u32) -> i64 {
    let mut storage = flags;
    let r = unsafe { (lib.apply_bit_operations)(value, &mut storage) };
    ((storage as i64) << 32) | (r as u32 as i64)
}

fn call_envy(lib: &Lib, p1: i32, p2: i32, p3: i32, p4: i32) -> i64 {
    unsafe { (lib.envy)(p1, p2, p3, p4) as i64 }
}

/// Assert that the *exact* sentinel the C returns on rejection is `default_val`,
/// and that both libraries emit the same stderr bytes.
fn assert_rejects_with_default(row: &str, var: &str, value: Option<&str>, expected_stderr: &str) {
    let (c, r) = libs();
    let name_owned = var.to_string();
    let val_owned = value.map(|s| s.to_string());

    for default_val in [0i32, 1, -1, 64, 10, i32::MIN, i32::MAX, 0x1234_5678] {
        let setup = || {
            env_clear_all();
            if let Some(v) = &val_owned {
                env_set(&name_owned, v);
            }
        };
        setup();
        let (vc, oc, ec) = capture(|| call_parse(c, &name_owned, default_val));
        setup();
        let (vr, or, er) = capture(|| call_parse(r, &name_owned, default_val));

        assert_eq!(
            vc, default_val as i64,
            "[{row}] C did not return the default_val sentinel for {var}={val_owned:?}"
        );
        assert_eq!(
            vr, vc,
            "[{row}] Rust returned {vr} where C returned {vc} for {var}={val_owned:?}"
        );
        assert_eq!(
            String::from_utf8_lossy(&ec),
            expected_stderr,
            "[{row}] unexpected C stderr for {var}={val_owned:?}"
        );
        assert_streams_eq(row, "stderr", &ec, &er);
        assert_streams_eq(row, "stdout", &oc, &or);
    }
    env_clear_all();
}

// ===========================================================================
// Rows 1–12: parse_env_numeric rejection branches
// ===========================================================================

#[test]
fn row01_env_variable_absent_returns_default() {
    let _g = lock();
    assert_rejects_with_default("ERRORS row 1", "PROG_BASE_OFFSET", None, "");
    assert_rejects_with_default("ERRORS row 1", "PROG_MULTIPLIER", None, "");
    assert_rejects_with_default("ERRORS row 1", "NO_SUCH_VAR_QQQ", None, "");
}

#[test]
fn row02_comma_in_value_warns_and_returns_default() {
    let _g = lock();
    for v in [",", "1,2", ",,,", "abc,def", "-1,"] {
        assert_rejects_with_default(
            "ERRORS row 2",
            "PROG_BASE_OFFSET",
            Some(v),
            "Warning: Invalid character in PROG_BASE_OFFSET\n",
        );
    }
    // The warning embeds env_name via %s, so a different name must appear.
    assert_rejects_with_default(
        "ERRORS row 2",
        "PROG_MULTIPLIER",
        Some("7,"),
        "Warning: Invalid character in PROG_MULTIPLIER\n",
    );
}

#[test]
fn row03_semicolon_in_value_warns_and_returns_default() {
    let _g = lock();
    for v in [";", "1;2", ";;;", "abc;def", "-1;"] {
        assert_rejects_with_default(
            "ERRORS row 3",
            "PROG_BASE_OFFSET",
            Some(v),
            "Warning: Semicolon found in PROG_BASE_OFFSET\n",
        );
    }
    assert_rejects_with_default(
        "ERRORS row 3",
        "PROG_MULTIPLIER",
        Some("7;"),
        "Warning: Semicolon found in PROG_MULTIPLIER\n",
    );
}

#[test]
fn row04_comma_is_checked_before_semicolon() {
    let _g = lock();
    // Both present: only the comma warning must be emitted, whichever order the
    // characters appear in the value.
    for v in [",;", ";,", "1,2;3", "1;2,3", ",1;", ";1,"] {
        assert_rejects_with_default(
            "ERRORS row 4",
            "PROG_MULTIPLIER",
            Some(v),
            "Warning: Invalid character in PROG_MULTIPLIER\n",
        );
    }
}

#[test]
fn row05_empty_value_is_not_rejected_and_yields_zero() {
    let _g = lock();
    let (c, r) = libs();
    for default_val in [0i32, 1, -1, 64, i32::MIN, i32::MAX] {
        env_clear_all();
        env_set("PROG_BASE_OFFSET", "");
        let (vc, oc, ec) = capture(|| call_parse(c, "PROG_BASE_OFFSET", default_val));
        env_set("PROG_BASE_OFFSET", "");
        let (vr, or, er) = capture(|| call_parse(r, "PROG_BASE_OFFSET", default_val));
        assert_eq!(
            vc, 0,
            "[ERRORS row 5] empty value must fall through to atoi(\"\") == 0, \
             NOT return default_val {default_val}"
        );
        assert_eq!(vr, vc);
        assert!(ec.is_empty() && oc.is_empty(), "no warning is emitted");
        assert_streams_eq("ERRORS row 5", "stderr", &ec, &er);
        assert_streams_eq("ERRORS row 5", "stdout", &oc, &or);
    }
    env_clear_all();
}

#[test]
fn row06_non_numeric_junk_uses_atoi_semantics() {
    let _g = lock();
    diff("ERRORS row 6", |lib| {
        let mut out = Vec::new();
        for v in [
            "abc", "++5", "--5", "0x1f", " \t-", "-", "+", ".", "e", "\x7f", "  ", "\t\n\r",
            "NaN", "inf", "!", "/", ":", "@",
        ] {
            env_clear_all();
            env_set("PROG_BASE_OFFSET", v);
            for d in [0i32, -1, 64, i32::MIN, i32::MAX] {
                out.push(call_parse(lib, "PROG_BASE_OFFSET", d));
            }
        }
        env_clear_all();
        out
    });
}

#[test]
fn row07_trailing_junk_after_digits() {
    let _g = lock();
    diff("ERRORS row 7", |lib| {
        let mut out = Vec::new();
        for v in [
            "12abc", "7 8", "-3xyz", "0009z", "1.5", "5e3", "42%", "-0-", "+12+", "  -7  q",
        ] {
            env_clear_all();
            env_set("PROG_MULTIPLIER", v);
            for d in [0i32, -1, 10, i32::MIN, i32::MAX] {
                out.push(call_parse(lib, "PROG_MULTIPLIER", d));
            }
        }
        env_clear_all();
        out
    });
}

#[test]
fn row08_value_out_of_int_range() {
    let _g = lock();
    diff("ERRORS row 8", |lib| {
        let mut out = Vec::new();
        for v in [
            "2147483648",
            "-2147483649",
            "99999999999999999999",
            "-99999999999999999999",
            "4294967296",
            "4294967295",
            "9223372036854775807",
            "9223372036854775808",
            "-9223372036854775808",
            "-9223372036854775809",
            "1000000000000",
        ] {
            env_clear_all();
            env_set("PROG_BASE_OFFSET", v);
            for d in [0i32, 64, i32::MIN, i32::MAX] {
                out.push(call_parse(lib, "PROG_BASE_OFFSET", d));
            }
            // And end to end, where the truncated value feeds the arithmetic.
            out.push(call_envy(lib, 1, 2, 3, 4));
        }
        env_clear_all();
        out
    });
}

#[test]
fn row09_value_at_int_boundaries() {
    let _g = lock();
    diff("ERRORS row 9", |lib| {
        let mut out = Vec::new();
        for v in ["2147483647", "-2147483648", "2147483646", "-2147483647"] {
            env_clear_all();
            env_set("PROG_MULTIPLIER", v);
            out.push(call_parse(lib, "PROG_MULTIPLIER", 0));
            out.push(call_envy(lib, 1, 1, 1, 1));
            out.push(call_envy(lib, -1, -1, -1, -1));
        }
        env_clear_all();
        out
    });
}

#[test]
fn row10_extreme_default_val_when_variable_unset() {
    let _g = lock();
    let _ = &();
    env_clear_all();
    let (c, r) = libs();
    for d in [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 1,
        i32::MAX,
        0o100,
        0o12,
    ] {
        let vc = call_parse(c, "NO_SUCH_VAR_QQQ", d);
        let vr = call_parse(r, "NO_SUCH_VAR_QQQ", d);
        assert_eq!(vc, d as i64, "[ERRORS row 10] C must return default_val as-is");
        assert_eq!(vr, vc, "[ERRORS row 10] Rust diverged for default_val {d}");
    }
}

#[test]
fn row11_rejected_character_at_position_zero() {
    let _g = lock();
    assert_rejects_with_default(
        "ERRORS row 11",
        "PROG_BASE_OFFSET",
        Some(",123"),
        "Warning: Invalid character in PROG_BASE_OFFSET\n",
    );
    assert_rejects_with_default(
        "ERRORS row 11",
        "PROG_BASE_OFFSET",
        Some(";123"),
        "Warning: Semicolon found in PROG_BASE_OFFSET\n",
    );
}

#[test]
fn row12_empty_env_name() {
    let _g = lock();
    env_clear_all();
    let (c, r) = libs();
    for d in [0i32, -1, 64, i32::MIN, i32::MAX] {
        let (vc, oc, ec) = capture(|| call_parse(c, "", d));
        let (vr, or, er) = capture(|| call_parse(r, "", d));
        assert_eq!(vc, d as i64, "[ERRORS row 12] getenv(\"\") must miss");
        assert_eq!(vr, vc);
        assert_streams_eq("ERRORS row 12", "stderr", &ec, &er);
        assert_streams_eq("ERRORS row 12", "stdout", &oc, &or);
    }
}

// ===========================================================================
// Rows 13–17: init_config_from_env rejection of absent / unsatisfying values
// ===========================================================================

#[test]
fn row13_verbose_absent_clears_the_bit() {
    let _g = lock();
    let (c, r) = libs();
    env_clear_all();
    let sc = call_init(c, 0) as u32;
    let sr = call_init(r, 0) as u32;
    assert_eq!(sc & 1, 0, "[ERRORS row 13] verbose must be 0 when unset");
    assert_eq!(sr, sc);
}

#[test]
fn row14_verbose_present_without_a_one_digit() {
    let _g = lock();
    let (c, r) = libs();
    for v in ["", "0", "true", "yes", "on", "2", "TRUE", "one", " ", "\t"] {
        env_clear_all();
        env_set("PROG_VERBOSE", v);
        let sc = call_init(c, 0) as u32;
        let sr = call_init(r, 0) as u32;
        assert_eq!(
            sc & 1,
            0,
            "[ERRORS row 14] presence alone must not enable verbose (value {v:?})"
        );
        assert_eq!(sr, sc, "[ERRORS row 14] diverged for PROG_VERBOSE={v:?}");
    }
    env_clear_all();
}

#[test]
fn row15_debug_absent_or_without_a_one_digit() {
    let _g = lock();
    let (c, r) = libs();
    env_clear_all();
    assert_eq!(call_init(c, 0) as u32 >> 1 & 1, 0);
    assert_eq!(call_init(r, 0), call_init(c, 0));
    for v in ["", "0", "true", "debug", "2", "on"] {
        env_clear_all();
        env_set("PROG_DEBUG", v);
        let sc = call_init(c, 0) as u32;
        let sr = call_init(r, 0) as u32;
        assert_eq!(
            sc >> 1 & 1,
            0,
            "[ERRORS row 15] debug must stay 0 for {v:?}"
        );
        assert_eq!(sr, sc);
    }
    env_clear_all();
}

#[test]
fn row16_optimize_presence_only_asymmetry() {
    let _g = lock();
    let (c, r) = libs();

    env_clear_all();
    let sc = call_init(c, 0) as u32;
    assert_eq!(sc >> 2 & 1, 0, "[ERRORS row 16] optimize 0 when absent");
    assert_eq!(call_init(r, 0) as u32, sc);

    // Present with ANY content — including values that read as "off" — enables it.
    for v in ["", "0", "false", "no", "off", "\t", "1"] {
        env_clear_all();
        env_set("PROG_OPTIMIZE", v);
        let sc = call_init(c, 0) as u32;
        let sr = call_init(r, 0) as u32;
        assert_eq!(
            sc >> 2 & 1,
            1,
            "[ERRORS row 16] optimize must be 1 for any present value ({v:?}) — \
             the C never inspects the content"
        );
        assert_eq!(sr, sc, "[ERRORS row 16] diverged for PROG_OPTIMIZE={v:?}");
    }
    env_clear_all();
}

#[test]
fn row17_init_preserves_padding_bits_over_garbage() {
    let _g = lock();
    diff("ERRORS row 17", |lib| {
        let mut out = Vec::new();
        let mut rng = Rng::new(SEED ^ 0xC17);
        for v in [None, Some("1"), Some("0")] {
            for d in [None, Some("1"), Some("0")] {
                for o in [None, Some("")] {
                    env_clear_all();
                    if let Some(x) = v {
                        env_set("PROG_VERBOSE", x);
                    }
                    if let Some(x) = d {
                        env_set("PROG_DEBUG", x);
                    }
                    if let Some(x) = o {
                        env_set("PROG_OPTIMIZE", x);
                    }
                    for pad in GARBAGE_PADDING {
                        out.push(call_init(lib, pad));
                        out.push(call_init(lib, pad | 0xFF));
                        out.push(call_init(lib, pad | 0x80));
                    }
                    for _ in 0..64 {
                        out.push(call_init(lib, rng.next_u32()));
                    }
                }
            }
        }
        env_clear_all();
        out
    });
}

// ===========================================================================
// Rows 18–22: out-of-range values arriving across the FFI boundary
// ===========================================================================

/// The `struct ConfigFlags` bitfields are the C analogue of an enum crossing the
/// FFI boundary: `log_level` has 8 representable values but
/// `init_config_from_env` only ever produces `3`, and `cache_enabled` is only
/// ever `1`. A foreign caller can supply any of the 256 low-byte patterns, so
/// every one of them is a real input the C accepts without validation.
#[test]
fn row18_all_256_out_of_range_flag_patterns() {
    let _g = lock();
    env_clear_all();
    diff("ERRORS row 18", |lib| {
        let mut out = Vec::new();
        for byte in 0u32..256 {
            for v in [
                i32::MIN,
                i32::MIN + 1,
                -1,
                0,
                1,
                2,
                3,
                7,
                0x4000_0000,
                i32::MAX - 1,
                i32::MAX,
            ] {
                out.push(call_apply(lib, v, byte));
                out.push(call_perform(lib, v, v, byte));
                out.push(call_perform(lib, v, 0, byte));
                out.push(call_perform(lib, 0, v, byte));
            }
            let mut rng = Rng::new(SEED ^ 0xE18 ^ byte as u64);
            for _ in 0..32 {
                let (a, b) = (rng.next_i32(), rng.next_i32());
                out.push(call_perform(lib, a, b, byte));
                out.push(call_apply(lib, a, byte));
            }
        }
        out
    });
}

#[test]
fn row19_garbage_in_the_padding_bits_is_ignored() {
    let _g = lock();
    env_clear_all();
    diff("ERRORS row 19", |lib| {
        let mut out = Vec::new();
        for pad in [
            0xFFFF_FF00u32,
            0x8000_0000,
            0xDEAD_BE00,
            0x0000_0100,
            0xA5A5_A500,
        ] {
            for byte in 0u32..256 {
                let fw = pad | byte;
                for v in [i32::MIN, -1, 0, 1, i32::MAX] {
                    out.push(call_apply(lib, v, fw));
                    out.push(call_perform(lib, v, v, fw));
                }
                // The same low byte with clean padding must give the same answer.
                for v in [i32::MIN, -1, 0, 1, i32::MAX] {
                    out.push(call_apply(lib, v, byte));
                    out.push(call_perform(lib, v, v, byte));
                }
            }
        }
        out
    });
}

#[test]
fn row20_division_truncation_toward_zero() {
    let _g = lock();
    env_clear_all();
    diff("ERRORS row 20", |lib| {
        let mut out = Vec::new();
        let fw = flags_word(0, 0, 0, 0, 0, 0, 0); // optimize=0, log_level=0
        for b in [
            i32::MIN,
            i32::MIN + 1,
            -7,
            -5,
            -4,
            -3,
            -2,
            -1,
            0,
            1,
            2,
            3,
            4,
            5,
            7,
            i32::MAX - 1,
            i32::MAX,
        ] {
            for a in [i32::MIN, -1, 0, 1, i32::MAX] {
                out.push(call_perform(lib, a, b, fw));
            }
        }
        let mut rng = Rng::new(SEED ^ 0xE20);
        for _ in 0..2048 {
            out.push(call_perform(lib, rng.next_i32(), rng.next_i32(), fw));
        }
        out
    });
}

#[test]
fn row21_signed_overflow_in_the_multiply_and_add() {
    let _g = lock();
    env_clear_all();
    diff("ERRORS row 21", |lib| {
        let mut out = Vec::new();
        for ll in 0u32..8 {
            let fw = flags_word(0, 0, 0, 0, ll, 0, 0);
            for a in [
                i32::MIN,
                i32::MIN + 1,
                i32::MIN / 2,
                -0x2000_0000,
                -3,
                -1,
                0,
                1,
                3,
                0x2000_0000,
                i32::MAX / 2,
                i32::MAX - 1,
                i32::MAX,
                0x1234_5678,
                0x4000_0000,
            ] {
                for b in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
                    out.push(call_perform(lib, a, b, fw));
                }
            }
        }
        // And the optimize path, where `val1 + val2` overflows.
        let fw = flags_word(0, 0, 1, 0, 0, 0, 0);
        for a in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
            for b in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
                out.push(call_perform(lib, a, b, fw));
            }
        }
        out
    });
}

#[test]
fn row22_left_shift_overflow_in_apply_bit_operations() {
    let _g = lock();
    env_clear_all();
    diff("ERRORS row 22", |lib| {
        let mut out = Vec::new();
        for cache in [0u32, 1] {
            let fw = flags_word(1, 0, 0, cache, 0, 0, 0); // verbose = 1 -> << 1
            for v in [
                i32::MIN,
                i32::MIN + 1,
                -0x4000_0001,
                -0x4000_0000,
                -1,
                0,
                1,
                0x3FFF_FFFF,
                0x4000_0000,
                0x4000_0001,
                0x7FFF_FFFE,
                i32::MAX,
            ] {
                out.push(call_apply(lib, v, fw));
            }
            let mut rng = Rng::new(SEED ^ 0xE22 ^ cache as u64);
            for _ in 0..1024 {
                out.push(call_apply(lib, rng.next_u32() as i32, fw));
            }
        }
        out
    });
}

// ===========================================================================
// Rows 23–29: envy fallback and guard branches
// ===========================================================================

#[test]
fn row23_negative_result_returns_param1() {
    let _g = lock();
    let (c, r) = libs();
    // Force a hugely negative accumulation so the `result < 0` branch fires and
    // the returned value must be exactly param1.
    env_clear_all();
    env_set("PROG_BASE_OFFSET", "-2147483648");
    env_set("PROG_MULTIPLIER", "-2000000");

    let mut fired = 0;
    for p1 in [i32::MIN, -1234, -1, 0, 1, 1234, i32::MAX] {
        for p2 in [i32::MIN, -1, 0, 1, i32::MAX] {
            let vc = call_envy(c, p1, p2, 1000, 0);
            let vr = call_envy(r, p1, p2, 1000, 0);
            assert_eq!(vr, vc, "[ERRORS row 23] diverged at ({p1}, {p2})");
            if vc == p1 as i64 {
                fired += 1;
            }
        }
    }
    assert!(
        fired > 0,
        "[ERRORS row 23] the result<0 backup-restore branch never fired — \
         the test does not exercise the row"
    );
    env_clear_all();
}

#[test]
fn row24_param3_zero_skips_the_multiplier_term() {
    let _g = lock();
    env_clear_all();
    diff("ERRORS row 24", |lib| {
        let mut out = Vec::new();
        // A multiplier that would overflow if it were ever applied.
        for m in ["2147483647", "-2147483648", "0", "1"] {
            env_clear_all();
            env_set("PROG_MULTIPLIER", m);
            for p1 in BOUNDS {
                out.push(call_envy(lib, p1, 3, 0, 0));
                out.push(call_envy(lib, p1, 3, 0, 8));
                // and the non-zero counterpart for contrast
                out.push(call_envy(lib, p1, 3, 1, 0));
                out.push(call_envy(lib, p1, 3, -1, 0));
            }
        }
        env_clear_all();
        out
    });
}

#[test]
fn row25_param4_zero_skips_the_shift_term() {
    let _g = lock();
    env_clear_all();
    diff("ERRORS row 25", |lib| {
        let mut out = Vec::new();
        for p1 in BOUNDS {
            for p2 in BOUNDS {
                out.push(call_envy(lib, p1, p2, 0, 0));
                out.push(call_envy(lib, p1, p2, 5, 0));
            }
        }
        out
    });
}

#[test]
fn row26_negative_param4_arithmetic_shift() {
    let _g = lock();
    env_clear_all();
    diff("ERRORS row 26", |lib| {
        let mut out = Vec::new();
        for p4 in [
            i32::MIN,
            i32::MIN + 1,
            i32::MIN + 3,
            -9,
            -8,
            -7,
            -6,
            -5,
            -4,
            -3,
            -2,
            -1,
        ] {
            for p1 in [0i32, 1, -1, 1_000_000, -1_000_000] {
                out.push(call_envy(lib, p1, 0, 0, p4));
                out.push(call_envy(lib, p1, 0, 1, p4));
            }
        }
        let mut rng = Rng::new(SEED ^ 0xE26);
        for _ in 0..512 {
            let p4 = -((rng.next_u32() % 1_000_000) as i32) - 1;
            out.push(call_envy(lib, rng.next_i32(), rng.next_i32(), 0, p4));
        }
        out
    });
}

#[test]
fn row27_param1_extremes_through_the_fallback() {
    let _g = lock();
    env_clear_all();
    diff("ERRORS row 27", |lib| {
        let mut out = Vec::new();
        for off in ["-2147483648", "-1", "0", "2147483647"] {
            for m in ["-2147483648", "-1", "0", "1", "2147483647"] {
                env_clear_all();
                env_set("PROG_BASE_OFFSET", off);
                env_set("PROG_MULTIPLIER", m);
                for p1 in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
                    for p3 in [0i32, 1, -1, i32::MAX, i32::MIN] {
                        out.push(call_envy(lib, p1, 0, p3, 0));
                    }
                }
            }
        }
        env_clear_all();
        out
    });
}

#[test]
fn row28_colon_is_always_found_at_index_six() {
    let _g = lock();
    let (c, r) = libs();
    env_clear_all();
    env_set("PROG_VERBOSE", "1");

    let params: Vec<(i32, i32, i32, i32)> = {
        let mut rng = Rng::new(SEED ^ 0xE28);
        let mut v = vec![
            (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
            (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
            (0, 0, 0, 0),
        ];
        for _ in 0..256 {
            v.push((
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
                rng.next_i32(),
            ));
        }
        v
    };

    let run = |lib: &Lib| {
        let mut vals = Vec::new();
        for &(a, b, cc, d) in &params {
            vals.push(call_envy(lib, a, b, cc, d));
        }
        vals
    };
    let (vc, oc, ec) = capture(|| run(c));
    let (vr, or, er) = capture(|| run(r));
    assert_eq!(vc, vr, "[ERRORS row 28] return values diverged");
    assert_streams_eq("ERRORS row 28", "stdout", &oc, &or);
    assert_streams_eq("ERRORS row 28", "stderr", &ec, &er);

    let text = String::from_utf8_lossy(&oc);
    let lines: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("Found colon at position:"))
        .collect();
    assert_eq!(
        lines.len(),
        params.len(),
        "[ERRORS row 28] strchr must succeed on every call"
    );
    for l in lines {
        assert_eq!(
            l, "Found colon at position: 6",
            "[ERRORS row 28] the ':' is always at index 6 (snprintf never truncates)"
        );
    }
    env_clear_all();
}

#[test]
fn row29_both_numeric_env_vars_invalid_emits_two_warnings_in_order() {
    let _g = lock();
    let (c, r) = libs();

    let setup = || {
        env_clear_all();
        env_set("PROG_BASE_OFFSET", ",");
        env_set("PROG_MULTIPLIER", ";");
    };
    setup();
    let (vc, oc, ec) = capture(|| call_envy(c, 1, 2, 3, 4));
    setup();
    let (vr, or, er) = capture(|| call_envy(r, 1, 2, 3, 4));

    assert_eq!(vc, vr, "[ERRORS row 29] return values diverged");
    assert_eq!(
        String::from_utf8_lossy(&ec),
        "Warning: Invalid character in PROG_BASE_OFFSET\n\
         Warning: Semicolon found in PROG_MULTIPLIER\n",
        "[ERRORS row 29] both warnings must appear, base offset first"
    );
    assert_streams_eq("ERRORS row 29", "stderr", &ec, &er);
    assert_streams_eq("ERRORS row 29", "stdout", &oc, &or);

    // Same, with every flag combination active so the warnings interleave with
    // stdout chatter.
    for v in [false, true] {
        for d in [false, true] {
            let setup = || {
                env_clear_all();
                if v {
                    env_set("PROG_VERBOSE", "1");
                }
                if d {
                    env_set("PROG_DEBUG", "1");
                }
                env_set("PROG_BASE_OFFSET", ";");
                env_set("PROG_MULTIPLIER", ",");
            };
            setup();
            let (a1, o1, e1) = capture(|| call_envy(c, -7, 9, 2, -3));
            setup();
            let (a2, o2, e2) = capture(|| call_envy(r, -7, 9, 2, -3));
            assert_eq!(a1, a2);
            assert_eq!(
                String::from_utf8_lossy(&e1),
                "Warning: Semicolon found in PROG_BASE_OFFSET\n\
                 Warning: Invalid character in PROG_MULTIPLIER\n"
            );
            assert_streams_eq("ERRORS row 29", "stderr", &e1, &e2);
            assert_streams_eq("ERRORS row 29", "stdout", &o1, &o2);
        }
    }
    env_clear_all();
}

// ===========================================================================
// Row 30: NULL pointer arguments. These fault, so each call is made in a forked
// child and the termination status is compared between the two libraries.
// ===========================================================================

unsafe extern "C" {
    fn fork() -> c_int;
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn _exit(code: c_int) -> !;
}

/// Run `f` in a forked child; return the raw wait status.
fn status_of(f: impl FnOnce()) -> c_int {
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut status: c_int = -1;
        let w = waitpid(pid, &mut status, 0);
        assert_eq!(w, pid, "waitpid failed");
        status
    }
}

fn describe(status: c_int) -> String {
    if status & 0x7f != 0 {
        format!("signal {}", status & 0x7f)
    } else {
        format!("exit {}", (status >> 8) & 0xff)
    }
}

#[test]
fn row30_null_pointer_arguments_fault_identically() {
    let _g = lock();
    env_clear_all();
    let (c, r) = libs();

    // Each entry: a label plus the NULL-argument call to make in the child.
    let cases: Vec<(&str, fn(&Lib))> = vec![
        ("parse_env_numeric(NULL, 7)", |lib: &Lib| {
            unsafe { (lib.parse_env_numeric)(std::ptr::null(), 7) };
        }),
        ("init_config_from_env(NULL)", |lib: &Lib| {
            unsafe { (lib.init_config_from_env)(std::ptr::null_mut()) };
        }),
        ("perform_operation(1, 2, NULL)", |lib: &Lib| {
            unsafe { (lib.perform_operation)(1, 2, std::ptr::null_mut()) };
        }),
        ("apply_bit_operations(1, NULL)", |lib: &Lib| {
            unsafe { (lib.apply_bit_operations)(1, std::ptr::null_mut()) };
        }),
    ];

    for (label, call) in cases {
        let sc = status_of(|| call(c));
        let sr = status_of(|| call(r));
        assert_eq!(
            describe(sc),
            describe(sr),
            "[ERRORS row 30] {label}: C terminated with {} but Rust with {}",
            describe(sc),
            describe(sr)
        );
    }
}
