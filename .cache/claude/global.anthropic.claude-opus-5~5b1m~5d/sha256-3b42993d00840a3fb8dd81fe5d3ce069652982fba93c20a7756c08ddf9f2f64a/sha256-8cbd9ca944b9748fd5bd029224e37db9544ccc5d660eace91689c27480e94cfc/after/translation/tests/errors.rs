// Phase C — error-path differential tests.
//
// One #[test] per row of ERRORS.md (E1..E36). Each constructs the exact
// invalid input / rejection condition and asserts the C .so and the Rust .so
// produce the SAME result: the same returned int, the same warning bytes on
// stderr, or — for the null-pointer rows — the same fatal signal in a forked
// child.

mod common;
use common::*;

use std::ffi::CString;

// ---------------------------------------------------------------------------
// E1 — variable unset -> default_val
// ---------------------------------------------------------------------------

#[test]
fn err_e1_unset_returns_default() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xE001);
    for name in ENV_NAMES {
        for d in BOUNDARY_I32 {
            let r = diff_parse_env(&format!("E1 {name}"), name, d);
            assert_eq!(r, d, "E1: unset var must return default_val verbatim");
        }
        for _ in 0..32 {
            let d = rng.interesting_i32();
            let cap_free = diff_parse_env(&format!("E1 {name} rnd"), name, d);
            assert_eq!(cap_free, d);
        }
    }
    // No output at all on this path.
    let l = libs();
    let f = l.c.parse_env_numeric();
    let n = CString::new("PROG_MULTIPLIER").unwrap();
    let cap = capture(|| unsafe { f(n.as_ptr(), 7) });
    assert!(cap.out.is_empty() && cap.err.is_empty(), "E1 must be silent");
}

// ---------------------------------------------------------------------------
// E2 — comma -> "Invalid character" warning + default
// ---------------------------------------------------------------------------

#[test]
fn err_e2_comma_rejected() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xE002);
    for name in ENV_NAMES {
        for i in 0..24 {
            let d = rng.interesting_i32();
            let digits: String = (0..(rng.below(8)))
                .map(|_| (b'0' + rng.below(10) as u8) as char)
                .collect();
            let pos = rng.below(digits.len() as u64 + 1) as usize;
            let mut v = digits.clone();
            v.insert(pos, ',');
            env_set(name, &v);
            let r = diff_parse_env(&format!("E2 {name}[{i}] v={v:?}"), name, d);
            assert_eq!(r, d, "E2 must return default_val");

            // The warning text and stream must match exactly.
            let l = libs();
            let cn = CString::new(name).unwrap();
            let cf = l.c.parse_env_numeric();
            let rf = l.rs.parse_env_numeric();
            let cc = capture(|| unsafe { cf(cn.as_ptr(), d) });
            let rr = capture(|| unsafe { rf(cn.as_ptr(), d) });
            let expect = format!("Warning: Invalid character in {name}\n").into_bytes();
            assert_eq!(cc.err, expect, "E2 C stderr text");
            assert_eq!(rr.err, expect, "E2 Rust stderr text");
            assert!(cc.out.is_empty() && rr.out.is_empty(), "E2 writes nothing to stdout");
        }
        env_unset(name);
    }
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E3 — semicolon (no comma) -> "Semicolon found" warning + default
// ---------------------------------------------------------------------------

#[test]
fn err_e3_semicolon_rejected() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xE003);
    for name in ENV_NAMES {
        for i in 0..24 {
            let d = rng.interesting_i32();
            let digits: String = (0..(rng.below(8)))
                .map(|_| (b'0' + rng.below(10) as u8) as char)
                .collect();
            let pos = rng.below(digits.len() as u64 + 1) as usize;
            let mut v = digits.clone();
            v.insert(pos, ';');
            assert!(!v.contains(','), "E3 setup: value must have no comma");
            env_set(name, &v);
            let r = diff_parse_env(&format!("E3 {name}[{i}] v={v:?}"), name, d);
            assert_eq!(r, d, "E3 must return default_val");

            let l = libs();
            let cn = CString::new(name).unwrap();
            let cf = l.c.parse_env_numeric();
            let rf = l.rs.parse_env_numeric();
            let cc = capture(|| unsafe { cf(cn.as_ptr(), d) });
            let rr = capture(|| unsafe { rf(cn.as_ptr(), d) });
            let expect = format!("Warning: Semicolon found in {name}\n").into_bytes();
            assert_eq!(cc.err, expect, "E3 C stderr text");
            assert_eq!(rr.err, expect, "E3 Rust stderr text");
        }
        env_unset(name);
    }
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E4 — both -> the comma check runs first, so ONLY the comma warning appears
// ---------------------------------------------------------------------------

#[test]
fn err_e4_comma_wins_over_semicolon() {
    let _g = lock();
    env_clear_all();
    let name = "PROG_BASE_OFFSET";
    let expect = format!("Warning: Invalid character in {name}\n").into_bytes();
    for v in [
        ",;", ";,", "1,2;3", "1;2,3", ";;;,,,", ",,,;;;", "a;b,c", "a,b;c",
    ] {
        env_set(name, v);
        let r = diff_parse_env(&format!("E4 v={v:?}"), name, 64);
        assert_eq!(r, 64);
        let l = libs();
        let cn = CString::new(name).unwrap();
        let cf = l.c.parse_env_numeric();
        let rf = l.rs.parse_env_numeric();
        let cc = capture(|| unsafe { cf(cn.as_ptr(), 64) });
        let rr = capture(|| unsafe { rf(cn.as_ptr(), 64) });
        assert_eq!(cc.err, expect, "E4: only the comma warning must fire (C)");
        assert_eq!(rr.err, expect, "E4: only the comma warning must fire (Rust)");
        assert!(
            !String::from_utf8_lossy(&cc.err).contains("Semicolon"),
            "E4: semicolon branch must be unreachable"
        );
    }
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E5 — set-but-empty is NOT unset: returns 0, not default_val
// ---------------------------------------------------------------------------

#[test]
fn err_e5_empty_string_is_zero() {
    let _g = lock();
    env_clear_all();
    for name in ENV_NAMES {
        env_set(name, "");
        for d in BOUNDARY_I32 {
            let r = diff_parse_env(&format!("E5 {name} d={d}"), name, d);
            assert_eq!(r, 0, "E5: empty value must yield atoi(\"\") == 0, not the default");
        }
        env_unset(name);
    }
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E6 / E7 / E8 — atoi pass-through (no rejection at all)
// ---------------------------------------------------------------------------

#[test]
fn err_e6_garbage_atoi_passthrough() {
    let _g = lock();
    env_clear_all();
    let name = "PROG_MULTIPLIER";
    for v in [
        "abc", "--3", "+", "-", "0x1f", "0X1F", "0b11", "1e9", "1.9", ".5",
        // NOTE: an embedded NUL cannot be placed in the environment at all
        // (setenv takes a C string), so it is not a reachable input.
        "+-3", "-+3", "e5", "NaN", "inf", "null", "|", "&", "%d",
        "%n", "%s", "٣", "١٢٣",
    ] {
        env_set(name, v);
        for d in [-1, 0, 1, i32::MIN, i32::MAX] {
            let l = libs();
            let cn = CString::new(name).unwrap();
            let cf = l.c.parse_env_numeric();
            let rf = l.rs.parse_env_numeric();
            let cc = capture(|| unsafe { cf(cn.as_ptr(), d) });
            let rr = capture(|| unsafe { rf(cn.as_ptr(), d) });
            assert_same(&format!("E6 v={v:?} d={d}"), &cc, &rr);
            assert!(cc.err.is_empty(), "E6: garbage is NOT rejected, no warning");
        }
        env_unset(name);
    }
    env_clear_all();
}

#[test]
fn err_e7_atoi_leading_trailing() {
    let _g = lock();
    env_clear_all();
    let name = "PROG_BASE_OFFSET";
    for v in [
        "  -42abc", " 42", "\t-7x", "\n\r\x0B\x0C 9 ", "+0007", "-0", "  +0009xyz",
        "42 43", "  ", "0000000000000000005",
    ] {
        env_set(name, v);
        diff_parse_env(&format!("E7 v={v:?}"), name, 64);
        diff_parse_env(&format!("E7 v={v:?} d=min"), name, i32::MIN);
        env_unset(name);
    }
    env_clear_all();
}

#[test]
fn err_e8_atoi_overflow() {
    let _g = lock();
    env_clear_all();
    let name = "PROG_MULTIPLIER";
    for v in [
        "2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "99999999999999999999",
        "-99999999999999999999",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551616",
        "1".repeat(64).as_str(),
    ] {
        env_set(name, v);
        // Same libc atoi in both libraries, so the (formally UB) result must be
        // bit-identical, not merely "both weird".
        diff_parse_env(&format!("E8 v={v:?}"), name, -1);
        env_unset(name);
    }
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E9 / E10 — default_val extremes; odd env names
// ---------------------------------------------------------------------------

#[test]
fn err_e9_default_extremes() {
    let _g = lock();
    env_clear_all();
    for d in [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX] {
        let r = diff_parse_env(&format!("E9 unset d={d}"), "PROG_NOT_SET_AT_ALL", d);
        assert_eq!(r, d, "E9: no clamping of default_val");
        // and on the two rejection paths, which also return default_val
        env_set("PROG_BASE_OFFSET", "1,2");
        let r = diff_parse_env(&format!("E9 comma d={d}"), "PROG_BASE_OFFSET", d);
        assert_eq!(r, d);
        env_set("PROG_BASE_OFFSET", "1;2");
        let r = diff_parse_env(&format!("E9 semi d={d}"), "PROG_BASE_OFFSET", d);
        assert_eq!(r, d);
        env_clear_all();
    }
}

#[test]
fn err_e10_empty_env_name() {
    let _g = lock();
    env_clear_all();
    for d in [i32::MIN, -1, 0, 1, i32::MAX] {
        let r = diff_parse_env("E10 name=\"\"", "", d);
        assert_eq!(r, d, "E10: getenv(\"\") is NULL -> default_val");
        let r = diff_parse_env("E10 name=\"=\"", "=", d);
        assert_eq!(r, d);
    }
}

// ---------------------------------------------------------------------------
// E11 — parse_env_numeric(NULL, d): both libraries must fault identically
// ---------------------------------------------------------------------------

#[test]
fn err_e11_null_env_name_faults_identically() {
    let _g = lock();
    env_clear_all();
    let l = libs();
    let cf = l.c.parse_env_numeric();
    let rf = l.rs.parse_env_numeric();
    let c = fork_outcome(|| unsafe { cf(std::ptr::null(), 5) });
    let r = fork_outcome(|| unsafe { rf(std::ptr::null(), 5) });
    assert_eq!(
        c, r,
        "E11: parse_env_numeric(NULL, 5) must behave the same in C and Rust \
         (C={c:?}, Rust={r:?})"
    );
}

// ---------------------------------------------------------------------------
// E12 / E13 / E14 — init_config_from_env "falsy" env values
// ---------------------------------------------------------------------------

#[test]
fn err_e12_verbose_needs_literal_1() {
    let _g = lock();
    for v in ["true", "TRUE", "yes", "on", "0", "", "22", "-", "enabled"] {
        env_apply(&[("PROG_VERBOSE", Some(v))]);
        let f = diff_init_config(&format!("E12 V={v:?}"), Flags([0; 4]));
        assert_eq!(
            f.0[0] & B_VERBOSE,
            0,
            "E12: PROG_VERBOSE={v:?} contains no '1' so verbose must be 0 (got {:#04x})",
            f.0[0]
        );
        // and the flag DOES get set when a '1' is present anywhere
        let with = format!("x{v}1y");
        env_apply(&[("PROG_VERBOSE", Some(&with))]);
        let f = diff_init_config(&format!("E12 V={with:?}"), Flags([0; 4]));
        assert_eq!(f.0[0] & B_VERBOSE, B_VERBOSE, "E12: a literal '1' must set verbose");
    }
    env_clear_all();
}

#[test]
fn err_e13_debug_needs_literal_1() {
    let _g = lock();
    for v in ["true", "TRUE", "yes", "on", "0", "", "22", "-", "enabled"] {
        env_apply(&[("PROG_DEBUG", Some(v))]);
        let f = diff_init_config(&format!("E13 D={v:?}"), Flags([0; 4]));
        assert_eq!(f.0[0] & B_DEBUG, 0, "E13: no '1' -> debug must be 0");
        let with = format!("{v}1");
        env_apply(&[("PROG_DEBUG", Some(&with))]);
        let f = diff_init_config(&format!("E13 D={with:?}"), Flags([0; 4]));
        assert_eq!(f.0[0] & B_DEBUG, B_DEBUG, "E13: '1' -> debug must be 1");
    }
    env_clear_all();
}

#[test]
fn err_e14_optimize_empty_is_true() {
    let _g = lock();
    // Only NULL-ness is tested for PROG_OPTIMIZE, so "" and "0" both enable it.
    for v in ["", "0", "no", "false", "off"] {
        env_apply(&[("PROG_OPTIMIZE", Some(v))]);
        let f = diff_init_config(&format!("E14 O={v:?}"), Flags([0; 4]));
        assert_eq!(
            f.0[0] & B_OPTIMIZE,
            B_OPTIMIZE,
            "E14: PROG_OPTIMIZE={v:?} is non-NULL so optimize must be 1"
        );
    }
    env_apply(&[("PROG_OPTIMIZE", None)]);
    let f = diff_init_config("E14 O=unset", Flags([0xFF; 4]));
    assert_eq!(f.0[0] & B_OPTIMIZE, 0, "E14: unset -> optimize must be 0");
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E15 — dirty ConfigFlags padding survives the byte-sized bit-field stores
// ---------------------------------------------------------------------------

#[test]
fn err_e15_dirty_padding_preserved() {
    let _g = lock();
    env_clear_all();
    for v in [None, Some("1")] {
        for d in [None, Some("1")] {
            for o in [None, Some("x")] {
                env_apply(&[
                    ("PROG_VERBOSE", v),
                    ("PROG_DEBUG", d),
                    ("PROG_OPTIMIZE", o),
                ]);
                let f = diff_init_config(
                    &format!("E15 V={v:?} D={d:?} O={o:?}"),
                    Flags([0xFF, 0xFF, 0xFF, 0xFF]),
                );
                assert_eq!(
                    &f.0[1..],
                    &[0xFFu8, 0xFF, 0xFF],
                    "E15: bytes 1..3 of the allocation unit must be left untouched"
                );
                // byte 0 must be fully determined regardless of its old value
                let g = diff_init_config("E15/zeroed", Flags([0x00, 0xFF, 0xFF, 0xFF]));
                assert_eq!(
                    f.0[0], g.0[0],
                    "E15: byte 0 must not depend on its previous contents"
                );
            }
        }
    }
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E16 / E17 — perform_operation with log_level 0 and with all 256 flag bytes
// ---------------------------------------------------------------------------

#[test]
fn err_e16_log_level_zero() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xE016);
    let flags = Flags::from_byte0(flag_byte(false, false, false, true, 0));
    for _ in 0..64 {
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        let r = diff_perform_op("E16", a, b, flags);
        assert_eq!(r, b.wrapping_div(2), "E16: log_level 0 -> result == val2/2");
    }
    for a in BOUNDARY_I32 {
        for b in BOUNDARY_I32 {
            let r = diff_perform_op("E16/bounds", a, b, flags);
            assert_eq!(r, b.wrapping_div(2));
        }
    }
}

#[test]
fn err_e17_all_256_flag_bytes() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xE017);
    // Every possible bit pattern, including combinations no named constant and
    // no call to init_config_from_env can produce (reserved set, cache clear,
    // log_level 4..7). None of them is validated by the C code.
    for b0 in 0u16..256 {
        let flags = Flags::from_byte0(b0 as u8);
        for _ in 0..6 {
            let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
            diff_perform_op(&format!("E17 byte0={b0:#04x}"), a, b, flags);
            diff_apply_bits(&format!("E17 byte0={b0:#04x}"), a, flags);
        }
    }
}

// ---------------------------------------------------------------------------
// E18 / E19 / E20 — signed overflow and truncating division
// ---------------------------------------------------------------------------

#[test]
fn err_e18_signed_overflow_mul() {
    let _g = lock();
    env_clear_all();
    for ll in [2u8, 3, 4, 5, 6, 7] {
        let flags = Flags::from_byte0(flag_byte(false, false, false, true, ll));
        for a in [
            i32::MAX,
            i32::MAX - 1,
            i32::MIN,
            i32::MIN + 1,
            0x4000_0000,
            -0x4000_0000,
            0x2000_0000,
            715827883,
            -715827883,
        ] {
            for b in [0, 1, -1, i32::MAX, i32::MIN] {
                diff_perform_op(&format!("E18 ll={ll}"), a, b, flags);
            }
        }
    }
    // and the add path (optimize = 1) overflowing
    let flags = Flags::from_byte0(flag_byte(false, false, true, true, 3));
    for a in [i32::MAX, i32::MIN, i32::MAX - 1, i32::MIN + 1] {
        for b in [i32::MAX, i32::MIN, 1, -1] {
            diff_perform_op("E18/add", a, b, flags);
        }
    }
}

#[test]
fn err_e19_negative_division() {
    let _g = lock();
    env_clear_all();
    let flags = Flags::from_byte0(flag_byte(false, false, false, true, 0));
    let r = diff_perform_op("E19", 0, i32::MIN, flags);
    assert_eq!(r, -1073741824, "E19: INT_MIN/2 == -1073741824");
    for b in [i32::MIN, i32::MIN + 1, -4, -3, -2, -1, 0, 1, 2, 3, i32::MAX] {
        let r = diff_perform_op(&format!("E19 b={b}"), 0, b, flags);
        assert_eq!(r, b / 2, "E19: C division truncates toward zero");
    }
}

#[test]
fn err_e20_minus_one_div_two() {
    let _g = lock();
    env_clear_all();
    let flags = Flags::from_byte0(flag_byte(false, false, false, true, 0));
    let r = diff_perform_op("E20", 0, -1, flags);
    assert_eq!(r, 0, "E20: -1/2 must truncate to 0, not floor to -1");
    let r = diff_perform_op("E20/-3", 0, -3, flags);
    assert_eq!(r, -1, "E20: -3/2 == -1");
}

// ---------------------------------------------------------------------------
// E21 / E22 / E23 — apply_bit_operations shift overflow / negative / no cache
// ---------------------------------------------------------------------------

#[test]
fn err_e21_shift_overflow() {
    let _g = lock();
    env_clear_all();
    for cache in [false, true] {
        let flags = Flags::from_byte0(flag_byte(true, false, false, cache, 3));
        for v in [
            0x4000_0000i32,
            0x4000_0001,
            0x5555_5555,
            0x7FFF_FFFE,
            0x7FFF_FFFF,
            0x8000_0000u32 as i32,
            0xC000_0000u32 as i32,
            0xFFFF_FFFFu32 as i32,
        ] {
            diff_apply_bits(&format!("E21 cache={cache}"), v, flags);
        }
    }
}

#[test]
fn err_e22_shift_negative() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xE022);
    for cache in [false, true] {
        let flags = Flags::from_byte0(flag_byte(true, false, false, cache, 3));
        for v in [-1i32, -2, -3, -16, -17, i32::MIN, i32::MIN + 1, -1073741824] {
            diff_apply_bits(&format!("E22 cache={cache}"), v, flags);
        }
        for _ in 0..64 {
            let v = -(1 + (rng.next_u32() % 0x7FFF_FFFF) as i32);
            diff_apply_bits(&format!("E22 rnd cache={cache}"), v, flags);
        }
    }
}

#[test]
fn err_e23_cache_disabled() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xE023);
    // cache_enabled == 0 is only reachable by driving the low-level export
    // directly; init_config_from_env always sets it.
    let flags = Flags::from_byte0(flag_byte(false, false, false, false, 3));
    for _ in 0..128 {
        let v = rng.interesting_i32();
        let r = diff_apply_bits("E23", v, flags);
        assert_eq!(r, v, "E23: verbose=0, cache=0 must be the identity");
    }
    for v in BOUNDARY_I32 {
        let r = diff_apply_bits("E23/bounds", v, flags);
        assert_eq!(r, v);
    }
    // verbose=1, cache=0 -> shift only, no OR
    let flags = Flags::from_byte0(flag_byte(true, false, false, false, 3));
    for v in BOUNDARY_I32 {
        let r = diff_apply_bits("E23/shift-only", v, flags);
        assert_eq!(r, ((v as u32) << 1) as i32, "E23: cache=0 must not OR 0x0F");
        assert!(r & 0x0F != 0x0F || v & 0x07 == 0x07);
    }
}

// ---------------------------------------------------------------------------
// E24 — envy rollback on result < 0
// ---------------------------------------------------------------------------

#[test]
fn err_e24_negative_result_rollback() {
    let _g = lock();
    env_clear_all();
    // With the default environment, result is always == 15 (mod 16); pick
    // values that land it strictly below zero.
    let cases: [[i32; 4]; 10] = [
        [0, -160, 0, 0],
        [5, -1000, 0, 0],
        [-1000, 0, 0, 0],
        [i32::MIN, 0, 0, 0],
        [-7, -7, -7, -7],
        [0, 0, -1000, 0],
        [0, 0, 0, i32::MIN],
        [-1, -1, -1, -1],
        [-100000, -100000, -100000, -100000],
        [12345, -1000000, 0, 0],
    ];
    for c in cases {
        let r = diff_envy("E24", c[0], c[1], c[2], c[3]);
        // Only assert the rollback semantics where the pre-check result really
        // is negative; the differential comparison above is unconditional.
        let mut pre = c[0].wrapping_mul(3).wrapping_add(c[1].wrapping_div(2));
        if c[2] != 0 {
            pre = pre.wrapping_add(c[2].wrapping_mul(10));
        }
        if c[3] != 0 {
            pre = pre.wrapping_add(c[3] >> 2);
        }
        pre = (pre | 0x0F).wrapping_add(64);
        if pre < 0 {
            assert_eq!(r, c[0], "E24: rollback must return param1 ({c:?})");
        }
    }
    // and with verbose on so the "Restored state from backup" line is compared
    env_apply(&[("PROG_VERBOSE", Some("1"))]);
    for c in cases {
        diff_envy("E24/verbose", c[0], c[1], c[2], c[3]);
    }
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E25 / E26 / E27 — envy's zero guards and the negative right shift
// ---------------------------------------------------------------------------

#[test]
fn err_e25_param3_zero_branch() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xE025);
    // A non-zero multiplier makes the skipped term observable.
    for m in ["1", "10", "-1", "2147483647"] {
        env_apply(&[("PROG_MULTIPLIER", Some(m))]);
        for _ in 0..32 {
            let (p1, p2, p4) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            diff_envy(&format!("E25 m={m}"), p1, p2, 0, p4);
        }
    }
    env_clear_all();
}

#[test]
fn err_e26_param4_zero_branch() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xE026);
    for _ in 0..64 {
        let (p1, p2, p3) = (
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
        diff_envy("E26", p1, p2, p3, 0);
    }
    // param4 == 0 vs param4 == 1/2/3, all of which shift to 0 -- the branch is
    // taken but contributes nothing, which must still match.
    for p4 in [0, 1, 2, 3] {
        diff_envy(&format!("E26 p4={p4}"), 1, 1, 1, p4);
    }
    env_clear_all();
}

#[test]
fn err_e27_negative_right_shift() {
    let _g = lock();
    env_clear_all();
    // gcc emits an arithmetic (sign-propagating) shift for `param4 >> 2`.
    for p4 in [-1i32, -2, -3, -4, -5, -6, -7, -8, -9, -16, i32::MIN, i32::MIN + 1, i32::MIN + 3] {
        diff_envy(&format!("E27 p4={p4}"), 0, 0, 0, p4);
        diff_envy(&format!("E27 p4={p4} b"), 1000, 1000, 1, p4);
    }
    env_apply(&[("PROG_VERBOSE", Some("1")), ("PROG_DEBUG", Some("1"))]);
    for p4 in [-1i32, -5, i32::MIN] {
        diff_envy(&format!("E27/loud p4={p4}"), 0, 0, 0, p4);
    }
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E28 — envy at the int extremes
// ---------------------------------------------------------------------------

#[test]
fn err_e28_param_extremes() {
    let _g = lock();
    env_clear_all();
    let ext = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &a in &ext {
        for &b in &ext {
            for &c in &ext {
                for &d in &ext {
                    diff_envy("E28", a, b, c, d);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E29 — envy with both env numerics rejected
// ---------------------------------------------------------------------------

#[test]
fn err_e29_envy_with_rejected_env() {
    let _g = lock();
    env_clear_all();
    env_apply(&[
        ("PROG_BASE_OFFSET", Some("7,7")),
        ("PROG_MULTIPLIER", Some("9;9")),
    ]);
    let l = libs();
    let cf = l.c.envy();
    let rf = l.rs.envy();
    let cc = capture(|| unsafe { cf(1, 2, 3, 4) });
    let rr = capture(|| unsafe { rf(1, 2, 3, 4) });
    assert_same("E29", &cc, &rr);
    let expect = b"Warning: Invalid character in PROG_BASE_OFFSET\n\
                   Warning: Semicolon found in PROG_MULTIPLIER\n"
        .to_vec();
    assert_eq!(cc.err, expect, "E29: both warnings, in this order (C)");
    assert_eq!(rr.err, expect, "E29: both warnings, in this order (Rust)");
    // defaults 0100 = 64 and 012 = 10 must have been used:
    // (1*3 + 2/2) + 3*10 + 4>>2 = 4 + 30 + 1 = 35 -> |0x0F = 47 -> +64 = 111
    assert_eq!(cc.ret, 111, "E29: defaults 64 and 10 must be in effect");

    let mut rng = Rng::new(0xE029);
    for _ in 0..48 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        diff_envy("E29/rnd", p[0], p[1], p[2], p[3]);
    }
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E30 — the two strchr NULL guards in envy
// ---------------------------------------------------------------------------

#[test]
fn err_e30_colons_always_present() {
    let _g = lock();
    env_clear_all();
    // snprintf writes "Result:<int>:Complete" — at most 7 + 11 + 9 = 27 bytes
    // plus the NUL, far below BUFFER_SIZE, so it never truncates and both
    // colons always exist. Verified by driving `envy` over the int extremes
    // with debug on: the "format validated" line (which requires the SECOND
    // colon) must appear for every input in both libraries.
    env_apply(&[("PROG_DEBUG", Some("1")), ("PROG_VERBOSE", Some("1"))]);
    let l = libs();
    let cf = l.c.envy();
    let rf = l.rs.envy();
    let mut rng = Rng::new(0xE030);
    let mut cases: Vec<[i32; 4]> = vec![
        [0, 0, 0, 0],
        [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
        [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
        [0, -160, 0, 0],
    ];
    for _ in 0..64 {
        cases.push([
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ]);
    }
    for p in cases {
        let cc = capture(|| unsafe { cf(p[0], p[1], p[2], p[3]) });
        let rr = capture(|| unsafe { rf(p[0], p[1], p[2], p[3]) });
        assert_same(&format!("E30 {p:?}"), &cc, &rr);
        let s = String::from_utf8_lossy(&cc.out).to_string();
        assert!(
            s.contains("Found colon at position: 6\n"),
            "E30: first colon must always be found at offset 6 ({p:?}): {s:?}"
        );
        assert!(
            s.contains("Debug: Result string format validated\n"),
            "E30: second colon must always be found ({p:?}): {s:?}"
        );
    }
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E31 / E32 — the result < 0 boundary
// ---------------------------------------------------------------------------

fn pre_rollback_default_env(p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
    let mut r = p1.wrapping_mul(3).wrapping_add(p2.wrapping_div(2));
    if p3 != 0 {
        r = r.wrapping_add(p3.wrapping_mul(10));
    }
    if p4 != 0 {
        r = r.wrapping_add(p4 >> 2);
    }
    (r | 0x0F).wrapping_add(64)
}

#[test]
fn err_e31_result_zero_boundary() {
    let _g = lock();
    env_clear_all();
    // `result == 0` cannot be produced through `envy` with the default config
    // (apply_bit_operations ORs 0x0F and base_offset 64 is a multiple of 16, so
    // result == 15 mod 16). It IS reachable by making base_offset cancel the
    // OR: base_offset = -15 gives result == 0 whenever the pre-OR value is
    // exactly 0..15.  0 must NOT trigger the rollback.
    env_apply(&[("PROG_BASE_OFFSET", Some("-15"))]);
    let mut hits = 0;
    for p2 in 0i32..64 {
        let mut r = p2.wrapping_div(2);
        r = (r | 0x0F).wrapping_sub(15);
        let got = diff_envy(&format!("E31 p2={p2}"), 0, p2, 0, 0);
        if r == 0 {
            hits += 1;
            assert_eq!(got, 0, "E31: result == 0 must NOT roll back");
        }
    }
    assert!(hits > 0, "E31: no input produced result == 0");
    // The smallest non-negative result reachable with the DEFAULT env is 15.
    env_clear_all();
    let mut hits15 = 0;
    for p2 in -400i32..=0 {
        if pre_rollback_default_env(0, p2, 0, 0) == 15 {
            hits15 += 1;
            let got = diff_envy(&format!("E31 default p2={p2}"), 0, p2, 0, 0);
            assert_eq!(got, 15);
        }
    }
    assert!(hits15 > 0);
    env_clear_all();
}

#[test]
fn err_e32_result_minus_one_boundary() {
    let _g = lock();
    env_clear_all();
    // -1 is one step past the boundary and IS reachable with the default env.
    let mut hits = 0;
    for p1 in [-3i32, -1, 0, 1, 7, 999] {
        for p2 in -600i32..=0 {
            if pre_rollback_default_env(p1, p2, 0, 0) == -1 {
                hits += 1;
                let got = diff_envy(&format!("E32 p1={p1} p2={p2}"), p1, p2, 0, 0);
                assert_eq!(got, p1, "E32: result == -1 must roll back to param1");
            }
        }
    }
    assert!(hits > 0, "E32: no input produced result == -1");
    // -1 with base_offset = -15 too (a different route to the same boundary)
    env_apply(&[("PROG_BASE_OFFSET", Some("-16"))]);
    for p2 in 0i32..64 {
        diff_envy(&format!("E32/base-16 p2={p2}"), 0, p2, 0, 0);
    }
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E33 — out-of-range "enum"/bit-field values across the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn err_e33_out_of_range_flag_bits() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xE033);
    // Every 32-bit pattern is a legal `struct ConfigFlags` object as far as C
    // is concerned: there is no validation anywhere. Sweep all 256 values of
    // the byte that actually holds the bit-fields, with random padding, and
    // also full random 4-byte patterns.
    for b0 in 0u16..256 {
        let pad = rng.next_u32().to_le_bytes();
        let flags = Flags([b0 as u8, pad[1], pad[2], pad[3]]);
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        diff_perform_op(&format!("E33 op byte0={b0:#04x}"), a, b, flags);
        diff_apply_bits(&format!("E33 bits byte0={b0:#04x}"), a, flags);
    }
    for _ in 0..256 {
        let w = rng.next_u32().to_le_bytes();
        let flags = Flags(w);
        let (a, b) = (rng.interesting_i32(), rng.interesting_i32());
        diff_perform_op("E33 op rnd", a, b, flags);
        diff_apply_bits("E33 bits rnd", a, flags);
        diff_init_config("E33 init rnd", flags);
    }
    // reserved = 1 specifically: a state no valid configuration produces.
    for ll in 0u8..8 {
        let flags = Flags::from_byte0(flag_byte(true, true, true, true, ll) | B_RESERVED);
        diff_perform_op("E33 reserved", 12345, -6789, flags);
        diff_apply_bits("E33 reserved", 12345, flags);
    }
}

// ---------------------------------------------------------------------------
// E34 — NULL struct pointer: all three consumers must fault identically
// ---------------------------------------------------------------------------

#[test]
fn err_e34_null_flags_pointer_faults_identically() {
    let _g = lock();
    env_clear_all();
    let l = libs();

    let ci = l.c.init_config_from_env();
    let ri = l.rs.init_config_from_env();
    let c = fork_outcome(|| unsafe {
        ci(std::ptr::null_mut());
        1
    });
    let r = fork_outcome(|| unsafe {
        ri(std::ptr::null_mut());
        1
    });
    assert_eq!(
        c, r,
        "E34: init_config_from_env(NULL) must behave identically (C={c:?}, Rust={r:?})"
    );

    let co = l.c.perform_operation();
    let ro = l.rs.perform_operation();
    let c = fork_outcome(|| unsafe { co(3, 4, std::ptr::null_mut()) & 0x7F });
    let r = fork_outcome(|| unsafe { ro(3, 4, std::ptr::null_mut()) & 0x7F });
    assert_eq!(
        c, r,
        "E34: perform_operation(_,_,NULL) must behave identically (C={c:?}, Rust={r:?})"
    );

    let cb = l.c.apply_bit_operations();
    let rb = l.rs.apply_bit_operations();
    let c = fork_outcome(|| unsafe { cb(3, std::ptr::null_mut()) & 0x7F });
    let r = fork_outcome(|| unsafe { rb(3, std::ptr::null_mut()) & 0x7F });
    assert_eq!(
        c, r,
        "E34: apply_bit_operations(_,NULL) must behave identically (C={c:?}, Rust={r:?})"
    );
}

// ---------------------------------------------------------------------------
// E35 — very long env value (no length limit in the C)
// ---------------------------------------------------------------------------

#[test]
fn err_e35_very_long_value() {
    let _g = lock();
    env_clear_all();
    let mut rng = Rng::new(0xE035);
    for len in [64usize, 255, 256, 257, 512, 1000, 4096] {
        let v: String = (0..len)
            .map(|_| (b'0' + rng.below(10) as u8) as char)
            .collect();
        env_set("PROG_BASE_OFFSET", &v);
        diff_parse_env(&format!("E35 len={len}"), "PROG_BASE_OFFSET", 64);
        // long value that is rejected at the very last byte
        let mut vc = v.clone();
        vc.push(',');
        env_set("PROG_BASE_OFFSET", &vc);
        diff_parse_env(&format!("E35 len={len}+comma"), "PROG_BASE_OFFSET", 64);
        let mut vs = v.clone();
        vs.push(';');
        env_set("PROG_BASE_OFFSET", &vs);
        diff_parse_env(&format!("E35 len={len}+semi"), "PROG_BASE_OFFSET", 64);
        // and through envy, where the long value also feeds the arithmetic
        env_set("PROG_BASE_OFFSET", &v);
        diff_envy(&format!("E35/envy len={len}"), 1, 2, 3, 4);
    }
    env_clear_all();
}

// ---------------------------------------------------------------------------
// E36 — env numerics at the int extremes drive wrapping inside envy
// ---------------------------------------------------------------------------

#[test]
fn err_e36_env_extremes() {
    let _g = lock();
    env_clear_all();
    let ext = ["2147483647", "-2147483648", "2147483646", "-2147483647"];
    let mut rng = Rng::new(0xE036);
    for b in ext {
        for m in ext {
            env_apply(&[
                ("PROG_BASE_OFFSET", Some(b)),
                ("PROG_MULTIPLIER", Some(m)),
            ]);
            for t in [
                [0, 0, 0, 0],
                [1, 1, 1, 1],
                [-1, -1, -1, -1],
                [i32::MAX, i32::MAX, i32::MAX, i32::MAX],
                [i32::MIN, i32::MIN, i32::MIN, i32::MIN],
                [0, 0, 2, 0],
                [0, 0, -2, 0],
            ] {
                diff_envy(&format!("E36 b={b} m={m}"), t[0], t[1], t[2], t[3]);
            }
            for _ in 0..8 {
                let p = [
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                ];
                diff_envy(&format!("E36/rnd b={b} m={m}"), p[0], p[1], p[2], p[3]);
            }
            // also with all output on, so the printed values are compared too
            env_apply(&[
                ("PROG_BASE_OFFSET", Some(b)),
                ("PROG_MULTIPLIER", Some(m)),
                ("PROG_VERBOSE", Some("1")),
                ("PROG_DEBUG", Some("1")),
            ]);
            diff_envy(&format!("E36/loud b={b} m={m}"), 1, 1, 1, 1);
            diff_envy(&format!("E36/loud b={b} m={m}"), i32::MIN, i32::MAX, -3, 7);
        }
    }
    // env numerics that overflow int, feeding envy's arithmetic
    for v in ["99999999999999999999", "-99999999999999999999", "9223372036854775808"] {
        env_apply(&[
            ("PROG_BASE_OFFSET", Some(v)),
            ("PROG_MULTIPLIER", Some(v)),
        ]);
        diff_envy(&format!("E36/atoi-overflow {v}"), 1, 2, 3, 4);
        diff_envy(&format!("E36/atoi-overflow {v}"), i32::MIN, i32::MAX, -1, -1);
    }
    env_clear_all();
}
