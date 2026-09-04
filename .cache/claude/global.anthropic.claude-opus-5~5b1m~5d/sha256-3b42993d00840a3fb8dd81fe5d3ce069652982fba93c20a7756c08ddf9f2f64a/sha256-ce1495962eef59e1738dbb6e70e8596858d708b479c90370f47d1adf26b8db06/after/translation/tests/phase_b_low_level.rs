// Phase B — valid-path differential tests for the LOW-LEVEL entry points.
// CONFIGS.md rows 1..36. Both implementations are reached only through their
// `.so` exports.
mod common;

use common::*;
use std::ffi::{c_int, CString};

const VAR: &str = "DIFF_TEST_VAR";

fn pen(imp: &Impl, name: &str, dflt: c_int) -> Call {
    let n = CString::new(name).unwrap();
    call(|| unsafe { (imp.parse_env_numeric)(n.as_ptr(), dflt) })
}

/// Sets `VAR` (or unsets it) and compares `parse_env_numeric` on both libraries.
fn check_pen(ctx: &str, value: Option<&str>, dflt: c_int) {
    check_pen_named(ctx, VAR, value, dflt);
}

fn check_pen_named(ctx: &str, name: &str, value: Option<&str>, dflt: c_int) {
    let (p, _g) = pair();
    apply_env(name, value);
    let c = pen(&p.c, name, dflt);
    let r = pen(&p.rs, name, dflt);
    unset_env(name);
    assert_same(&format!("{ctx} name={name} value={value:?} default={dflt}"), &c, &r);
}

// ---------------------------------------------------------------------------
// rows 1..14 — parse_env_numeric
// ---------------------------------------------------------------------------

#[test]
fn row_01_unset_variable() {
    let mut rng = Rng::new(SEED ^ 1);
    unset_env(VAR);
    for d in BOUNDARIES {
        check_pen("row01/boundary", None, d);
    }
    for _ in 0..200 {
        check_pen("row01/random", None, rng.interesting_i32());
    }
}

#[test]
fn row_02_valid_positive_decimal() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..300 {
        let v = (rng.next_u32() >> 1) as i32;
        check_pen("row02", Some(&v.to_string()), rng.interesting_i32());
    }
}

#[test]
fn row_03_valid_negative_decimal() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..300 {
        let v = -((rng.next_u32() >> 1) as i32);
        check_pen("row03", Some(&v.to_string()), rng.interesting_i32());
    }
}

#[test]
fn row_04_explicit_plus_sign() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..200 {
        let v = rng.next_u32() >> 1;
        check_pen("row04", Some(&format!("+{v}")), rng.interesting_i32());
    }
    for s in ["+0", "+1", "+2147483647", "++5", "+-5", "-+5", "--5"] {
        check_pen("row04/fixed", Some(s), 7);
    }
}

#[test]
fn row_05_leading_whitespace() {
    let mut rng = Rng::new(SEED ^ 5);
    for pre in [" ", "  ", "\t", "\n", "\r", "\x0b", "\x0c", " \t "] {
        for _ in 0..40 {
            let v = rng.next_i32();
            check_pen("row05", Some(&format!("{pre}{v}")), rng.interesting_i32());
        }
    }
}

#[test]
fn row_06_trailing_garbage() {
    let mut rng = Rng::new(SEED ^ 6);
    for suf in ["abc", " ", " 12", "x", ".5", "e9", "\t", "-", "+", "\n"] {
        for _ in 0..40 {
            let v = rng.next_i32();
            check_pen("row06", Some(&format!("{v}{suf}")), rng.interesting_i32());
        }
    }
}

#[test]
fn row_07_leading_zeros_are_decimal_not_octal() {
    for s in ["0100", "0", "00", "000000012", "007", "0000000000000000009"] {
        check_pen("row07", Some(s), 42);
    }
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..200 {
        let v = rng.below(100000);
        check_pen("row07/random", Some(&format!("0{v}")), rng.interesting_i32());
    }
}

#[test]
fn row_08_hex_and_other_bases() {
    for s in ["0x1F", "0X10", "0b101", "1_000", "1e3", "#7", "o17"] {
        check_pen("row08", Some(s), -5);
    }
}

#[test]
fn row_09_int_limits_and_one_past() {
    for s in [
        "2147483647",
        "2147483648",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "-4294967296",
        "2147483646",
        "-2147483647",
    ] {
        for d in BOUNDARIES {
            check_pen("row09", Some(s), d);
        }
    }
}

#[test]
fn row_10_comma_rejection() {
    let mut rng = Rng::new(SEED ^ 10);
    // The env NAME is interpolated into the warning with %s, so it is varied too.
    for name in ["DIFF_TEST_VAR", "A", "LONG_NAME_WITH_UNDERSCORES_1234567890", "x"] {
        for _ in 0..40 {
            let v = rng.next_i32();
            let val = format!("{v},{}", rng.next_i32());
            check_pen_named("row10", name, Some(&val), rng.interesting_i32());
        }
    }
    for val in [",", "1,", ",1", "a,b", ",,,", "12,34"] {
        check_pen("row10/fixed", Some(val), 99);
    }
}

#[test]
fn row_11_semicolon_rejection() {
    let mut rng = Rng::new(SEED ^ 11);
    for name in ["DIFF_TEST_VAR", "B", "SEMI_NAME_0987654321"] {
        for _ in 0..40 {
            let v = rng.next_i32();
            let val = format!("{v};{}", rng.next_i32());
            check_pen_named("row11", name, Some(&val), rng.interesting_i32());
        }
    }
    for val in [";", "1;", ";1", "a;b", ";;;", "12;34"] {
        check_pen("row11/fixed", Some(val), -99);
    }
}

#[test]
fn row_12_both_separators_comma_checked_first() {
    for val in ["1,2;3", "1;2,3", ",;", ";,", "a;b,c", "a,b;c"] {
        check_pen("row12", Some(val), 1234);
    }
}

#[test]
fn row_13_empty_and_whitespace_only() {
    for val in ["", " ", "   ", "\t", "\n", "abc", "+", "-", ".", "e"] {
        for d in BOUNDARIES {
            check_pen("row13", Some(val), d);
        }
    }
}

#[test]
fn row_14_very_long_values() {
    let long_digits = "9".repeat(1024);
    let long_zeros = format!("{}{}", "0".repeat(1000), 42);
    let long_with_comma = format!("{}{}", "1".repeat(1000), ",5");
    let long_with_semi = format!("{}{}", "2".repeat(1000), ";5");
    for val in [
        long_digits.as_str(),
        long_zeros.as_str(),
        long_with_comma.as_str(),
        long_with_semi.as_str(),
    ] {
        check_pen("row14", Some(val), 8);
    }
}

// ---------------------------------------------------------------------------
// rows 15..18 — init_config_from_env
// ---------------------------------------------------------------------------

fn init_both(prefill: [u8; 4]) -> ((Flags, Output), (Flags, Output)) {
    let (p, _g) = pair();
    let mut fc = Flags(prefill);
    let mut fr = Flags(prefill);
    let ((), oc) = capture(|| unsafe { (p.c.init_config_from_env)(fc.as_ptr()) });
    let ((), or) = capture(|| unsafe { (p.rs.init_config_from_env)(fr.as_ptr()) });
    ((fc, oc), (fr, or))
}

const V_STATES: [Option<&str>; 3] = [None, Some("no-one-here"), Some("1")];
const O_STATES: [Option<&str>; 3] = [None, Some(""), Some("0")];

#[test]
fn row_15_full_env_cross_product() {
    for v in V_STATES {
        for d in V_STATES {
            for o in O_STATES {
                apply_env("PROG_VERBOSE", v);
                apply_env("PROG_DEBUG", d);
                apply_env("PROG_OPTIMIZE", o);
                for prefill in [[0u8; 4], [0xFF; 4], [0xAA; 4]] {
                    let (c, r) = init_both(prefill);
                    assert_same_flags(
                        &format!("row15 v={v:?} d={d:?} o={o:?} prefill={prefill:02x?}"),
                        (&c.0, &c.1),
                        (&r.0, &r.1),
                    );
                }
                clear_prog_env();
            }
        }
    }
}

#[test]
fn row_16_prefill_patterns_and_preserved_bits() {
    let mut rng = Rng::new(SEED ^ 16);
    for v in V_STATES {
        for d in V_STATES {
            for o in O_STATES {
                apply_env("PROG_VERBOSE", v);
                apply_env("PROG_DEBUG", d);
                apply_env("PROG_OPTIMIZE", o);
                for _ in 0..12 {
                    let prefill = rng.next_u32().to_le_bytes();
                    let (c, r) = init_both(prefill);
                    assert_same_flags(
                        &format!("row16 v={v:?} d={d:?} o={o:?} prefill={prefill:02x?}"),
                        (&c.0, &c.1),
                        (&r.0, &r.1),
                    );
                }
                clear_prog_env();
            }
        }
    }
}

#[test]
fn row_17_one_character_positions() {
    let values = [
        "1", "10", "01", "11", "v1.0", "310", "a1", "1a", "aaa1aaa", "0", "yes", "true", "-1",
        "+1", "\u{31}", "21", "1;", "1,",
    ];
    for vv in values {
        for dd in values {
            apply_env("PROG_VERBOSE", Some(vv));
            apply_env("PROG_DEBUG", Some(dd));
            unset_env("PROG_OPTIMIZE");
            let (c, r) = init_both([0x5A; 4]);
            assert_same_flags(&format!("row17 v={vv:?} d={dd:?}"), (&c.0, &c.1), (&r.0, &r.1));
            clear_prog_env();
        }
    }
}

#[test]
fn row_18_repeated_calls_are_stable() {
    let (p, _g) = pair();
    for v in V_STATES {
        for o in O_STATES {
            apply_env("PROG_VERBOSE", v);
            apply_env("PROG_DEBUG", v);
            apply_env("PROG_OPTIMIZE", o);
            let mut fc = Flags([0x3C; 4]);
            let mut fr = Flags([0x3C; 4]);
            let ((), oc) = capture(|| unsafe {
                (p.c.init_config_from_env)(fc.as_ptr());
                (p.c.init_config_from_env)(fc.as_ptr());
                (p.c.init_config_from_env)(fc.as_ptr());
            });
            let ((), or) = capture(|| unsafe {
                (p.rs.init_config_from_env)(fr.as_ptr());
                (p.rs.init_config_from_env)(fr.as_ptr());
                (p.rs.init_config_from_env)(fr.as_ptr());
            });
            assert_same_flags(&format!("row18 v={v:?} o={o:?}"), (&fc, &oc), (&fr, &or));
            clear_prog_env();
        }
    }
}

// ---------------------------------------------------------------------------
// rows 19..31 — perform_operation
// ---------------------------------------------------------------------------

/// Calls `perform_operation` on both libraries and compares the return value,
/// the captured output AND the (unchanged) flags unit.
fn check_perform(ctx: &str, val1: c_int, val2: c_int, flags: Flags) {
    let (p, _g) = pair();
    let mut fc = flags;
    let mut fr = flags;
    let c = call(|| unsafe { (p.c.perform_operation)(val1, val2, fc.as_ptr()) });
    let r = call(|| unsafe { (p.rs.perform_operation)(val1, val2, fr.as_ptr()) });
    assert_same(&format!("{ctx} val1={val1} val2={val2} {flags:?}"), &c, &r);
    assert_eq!(fc, fr, "{ctx}: flags unit diverged after the call");
}

fn sweep_perform(ctx: &str, flags: Flags, seed: u64) {
    let mut rng = Rng::new(seed);
    for a in BOUNDARIES {
        for b in BOUNDARIES {
            check_perform(ctx, a, b, flags);
        }
    }
    for _ in 0..150 {
        check_perform(ctx, rng.interesting_i32(), rng.interesting_i32(), flags);
    }
}

#[test]
fn row_19_optimize_no_debug() {
    sweep_perform("row19", Flags::new(false, false, true, true, 3), SEED ^ 19);
}

#[test]
fn row_20_optimize_with_debug() {
    sweep_perform("row20", Flags::new(false, true, true, true, 3), SEED ^ 20);
}

#[test]
fn rows_21_to_28_multiply_path_all_log_levels() {
    for log in 0u8..8 {
        sweep_perform(
            &format!("row{}/log_level={log}", 21 + log),
            Flags::new(false, false, false, true, log),
            SEED ^ (0x21_00 + log as u64),
        );
    }
}

#[test]
fn row_29_multiply_path_with_debug_all_log_levels() {
    for log in 0u8..8 {
        sweep_perform(
            &format!("row29/log_level={log}"),
            Flags::new(false, true, false, true, log),
            SEED ^ (0x29_00 + log as u64),
        );
    }
}

#[test]
fn row_30_all_256_flag_byte_patterns() {
    let mut rng = Rng::new(SEED ^ 30);
    for byte0 in 0u16..256 {
        let flags = Flags::raw(byte0 as u8, [0, 0, 0]);
        for _ in 0..6 {
            check_perform("row30", rng.interesting_i32(), rng.interesting_i32(), flags);
        }
        check_perform("row30/min", i32::MIN, i32::MIN, flags);
        check_perform("row30/max", i32::MAX, i32::MAX, flags);
    }
}

#[test]
fn row_31_garbage_in_upper_bits_is_ignored() {
    let mut rng = Rng::new(SEED ^ 31);
    for byte0 in [0x00u8, 0x08, 0x0C, 0x3A, 0x7F, 0xFF, 0x38, 0x0A] {
        for _ in 0..40 {
            let upper = [rng.next_u32() as u8, rng.next_u32() as u8, rng.next_u32() as u8];
            let flags = Flags::raw(byte0, upper);
            check_perform("row31", rng.interesting_i32(), rng.interesting_i32(), flags);
        }
        // ...and the reference "no garbage" call must give the same answer.
        check_perform("row31/clean", 12345, -6789, Flags::raw(byte0, [0, 0, 0]));
        check_perform("row31/dirty", 12345, -6789, Flags::raw(byte0, [0xDE, 0xAD, 0xBE]));
    }
}

// ---------------------------------------------------------------------------
// rows 32..36 — apply_bit_operations
// ---------------------------------------------------------------------------

fn check_bitops(ctx: &str, value: c_int, flags: Flags) {
    let (p, _g) = pair();
    let mut fc = flags;
    let mut fr = flags;
    let c = call(|| unsafe { (p.c.apply_bit_operations)(value, fc.as_ptr()) });
    let r = call(|| unsafe { (p.rs.apply_bit_operations)(value, fr.as_ptr()) });
    assert_same(&format!("{ctx} value={value} {flags:?}"), &c, &r);
    assert_eq!(fc, fr, "{ctx}: flags unit diverged after the call");
}

fn sweep_bitops(ctx: &str, flags: Flags, seed: u64) {
    let mut rng = Rng::new(seed);
    for v in BOUNDARIES {
        check_bitops(ctx, v, flags);
    }
    for v in [
        0x4000_0001u32 as i32,
        0x7FFF_FFFF,
        -0x4000_0000,
        -0x4000_0001,
        0x0F,
        0x10,
        -15,
        -16,
        i32::MIN + 1,
        i32::MAX - 1,
    ] {
        check_bitops(ctx, v, flags);
    }
    for _ in 0..200 {
        check_bitops(ctx, rng.interesting_i32(), flags);
    }
}

#[test]
fn row_32_no_verbose_no_cache() {
    sweep_bitops("row32", Flags::new(false, false, false, false, 3), SEED ^ 32);
}

#[test]
fn row_33_no_verbose_with_cache() {
    sweep_bitops("row33", Flags::new(false, false, false, true, 3), SEED ^ 33);
}

#[test]
fn row_34_verbose_no_cache() {
    sweep_bitops("row34", Flags::new(true, false, false, false, 3), SEED ^ 34);
}

#[test]
fn row_35_verbose_with_cache() {
    sweep_bitops("row35", Flags::new(true, false, false, true, 3), SEED ^ 35);
}

#[test]
fn row_36_all_256_flag_byte_patterns() {
    let mut rng = Rng::new(SEED ^ 36);
    for byte0 in 0u16..256 {
        let flags = Flags::raw(byte0 as u8, [0xAA, 0x55, 0xF0]);
        for _ in 0..6 {
            check_bitops("row36", rng.interesting_i32(), flags);
        }
        check_bitops("row36/min", i32::MIN, flags);
        check_bitops("row36/max", i32::MAX, flags);
    }
}

// ---------------------------------------------------------------------------
// Extra: the env NAME is forwarded to `fprintf` as a `%s` argument, so names
// containing format specifiers or non-ASCII bytes must be reproduced verbatim
// by both implementations (rows 10/11, name axis).
// ---------------------------------------------------------------------------

#[test]
fn row_10_11_hostile_env_names() {
    let (p, _g) = pair();
    for name in [
        "%s", "%d%d%d", "%n", "%%", "A%sB", "\u{e4}\u{f6}\u{fc}", "NAME WITH SPACES",
        "tab\there", "0123", "-", "..", "a".repeat(300).as_str(),
    ] {
        for value in ["1,2", "3;4", "5", ""] {
            // getenv() with such names simply fails to find them unless set.
            apply_env(name, Some(value));
            let c = pen(&p.c, name, 4242);
            let r = pen(&p.rs, name, 4242);
            unset_env(name);
            assert_same(&format!("hostile-name name={name:?} value={value:?}"), &c, &r);
        }
    }
}
