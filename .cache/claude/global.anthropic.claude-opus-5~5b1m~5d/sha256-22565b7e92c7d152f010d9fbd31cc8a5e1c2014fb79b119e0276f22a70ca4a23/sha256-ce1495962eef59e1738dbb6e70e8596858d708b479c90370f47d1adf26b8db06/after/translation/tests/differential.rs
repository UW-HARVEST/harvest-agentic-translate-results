// Phase B — valid-path differential tests.
//
// One test per row of CONFIGS.md, in the same order, named `cfg_NN_…`.
// Every test drives BOTH shared objects through their exported symbols and
// compares the return value, the stdout bytes and the stderr bytes.
//
// Randomized inputs come from the fixed-seed splitmix64 RNG in `common`, so a
// failure is always reproducible.

mod common;

use common::*;
use std::ffi::c_int;

// The two octal defaults baked into `envy` (`0100` and `012`).
const DEF_BASE_OFFSET: c_int = 0o100; // 64
const DEF_MULTIPLIER: c_int = 0o12; // 10

fn rng_for(tag: &str) -> Rng {
    // Derive a distinct but deterministic stream per test from the shared seed.
    let mut h: u64 = SEED;
    for b in tag.as_bytes() {
        h = (h ^ *b as u64).wrapping_mul(0x100_0000_01B3);
    }
    Rng::new(h)
}

// ===========================================================================
// parse_env_numeric — rows 1..11
// ===========================================================================

/// Row 1: variable absent from the environment × randomized `default_val`.
#[test]
fn cfg_01_parse_missing_env_random_defaults() {
    let _g = lock();
    let mut rng = rng_for("cfg_01");
    let names = ["PROG_BASE_OFFSET", "PROG_MULTIPLIER", "PROG_NOT_SET_AT_ALL"];
    for i in 0..600 {
        let name = *rng.pick(&names);
        let dv = rng.interesting_i32();
        diff_with_env(
            &format!("row1/#{i} {name} unset default={dv}"),
            &[(name, None)],
            |api| call_parse(api, name, dv),
        );
    }
}

/// Row 2: value is a run of plain decimal digits.
#[test]
fn cfg_02_parse_plain_decimal() {
    let _g = lock();
    let mut rng = rng_for("cfg_02");
    for i in 0..600 {
        let digits = 1 + rng.below(12) as usize;
        let mut v = String::new();
        for _ in 0..digits {
            v.push((b'0' + rng.below(10) as u8) as char);
        }
        let dv = rng.interesting_i32();
        diff_with_env(
            &format!("row2/#{i} value={v:?} default={dv}"),
            &[("PROG_BASE_OFFSET", Some(&v))],
            |api| call_parse(api, "PROG_BASE_OFFSET", dv),
        );
    }
}

/// Row 3: signed decimal values.
#[test]
fn cfg_03_parse_signed_decimal() {
    let _g = lock();
    let mut rng = rng_for("cfg_03");
    for i in 0..600 {
        let n = rng.interesting_i32();
        let v = if rng.bool() {
            format!("{n}")
        } else {
            format!("{:+}", n)
        };
        let dv = rng.interesting_i32();
        diff_with_env(
            &format!("row3/#{i} value={v:?} default={dv}"),
            &[("PROG_MULTIPLIER", Some(&v))],
            |api| call_parse(api, "PROG_MULTIPLIER", dv),
        );
    }
}

/// Row 4: leading whitespace / trailing garbage — `atoi` prefix parsing.
#[test]
fn cfg_04_parse_whitespace_and_trailing_garbage() {
    let _g = lock();
    let mut rng = rng_for("cfg_04");
    let fixed = [
        "  42", "\t-7", " +19abc", "12 34", "0x10", "007", "3.99", "-0", "+0", " - 5", "--5",
        "1e5", "  \t  ", "9,9", "8;8", "2147483647x", "\t\t+2147483646zz",
    ];
    for (i, v) in fixed.iter().enumerate() {
        diff_with_env(
            &format!("row4/fixed#{i} value={v:?}"),
            &[("PROG_BASE_OFFSET", Some(v))],
            |api| call_parse(api, "PROG_BASE_OFFSET", 1234),
        );
    }
    for i in 0..600 {
        let v = random_numeric_value(&mut rng);
        let dv = rng.interesting_i32();
        diff_with_env(
            &format!("row4/rand#{i} value={v:?} default={dv}"),
            &[("PROG_BASE_OFFSET", Some(&v))],
            |api| call_parse(api, "PROG_BASE_OFFSET", dv),
        );
    }
}

/// Row 5: value contains `,` at a random position ⇒ default + stderr warning.
#[test]
fn cfg_05_parse_comma_rejection() {
    let _g = lock();
    let mut rng = rng_for("cfg_05");
    for i in 0..600 {
        let v = random_comma_value(&mut rng);
        let dv = rng.interesting_i32();
        let name = if rng.bool() {
            "PROG_BASE_OFFSET"
        } else {
            "PROG_MULTIPLIER"
        };
        diff_with_env(
            &format!("row5/#{i} value={v:?} default={dv}"),
            &[(name, Some(&v))],
            |api| call_parse(api, name, dv),
        );
    }
}

/// Row 6: value contains `;` and no `,` ⇒ default + the semicolon warning.
#[test]
fn cfg_06_parse_semicolon_rejection() {
    let _g = lock();
    let mut rng = rng_for("cfg_06");
    for i in 0..600 {
        let v = random_semicolon_value(&mut rng);
        assert!(!v.contains(','));
        let dv = rng.interesting_i32();
        diff_with_env(
            &format!("row6/#{i} value={v:?} default={dv}"),
            &[("PROG_MULTIPLIER", Some(&v))],
            |api| call_parse(api, "PROG_MULTIPLIER", dv),
        );
    }
}

/// Row 7: both `,` and `;` present, in random order ⇒ the comma branch must win.
#[test]
fn cfg_07_parse_comma_and_semicolon() {
    let _g = lock();
    let mut rng = rng_for("cfg_07");
    for i in 0..600 {
        let mut v = random_clean_value(&mut rng, 10);
        let p1 = rng.below(v.len() as u64 + 1) as usize;
        v.insert(p1, ',');
        let p2 = rng.below(v.len() as u64 + 1) as usize;
        v.insert(p2, ';');
        let dv = rng.interesting_i32();
        diff_with_env(
            &format!("row7/#{i} value={v:?} default={dv}"),
            &[("PROG_BASE_OFFSET", Some(&v))],
            |api| call_parse(api, "PROG_BASE_OFFSET", dv),
        );
    }
}

/// Row 8: present but empty value ⇒ `atoi("") == 0`, *not* `default_val`.
#[test]
fn cfg_08_parse_empty_value() {
    let _g = lock();
    let mut rng = rng_for("cfg_08");
    for i in 0..200 {
        let dv = rng.interesting_i32();
        diff_with_env(
            &format!("row8/#{i} default={dv}"),
            &[("PROG_BASE_OFFSET", Some(""))],
            |api| call_parse(api, "PROG_BASE_OFFSET", dv),
        );
    }
}

/// Row 9: octal-looking values — `atoi` is decimal, so `0100` is 100 not 64.
#[test]
fn cfg_09_parse_octal_looking_is_decimal() {
    let _g = lock();
    let mut rng = rng_for("cfg_09");
    let fixed = [
        "0100", "012", "0755", "00", "000000012", "-0100", "+0755", "08", "09", "0o12",
    ];
    for (i, v) in fixed.iter().enumerate() {
        let dv = rng.interesting_i32();
        diff_with_env(
            &format!("row9/#{i} value={v:?} default={dv}"),
            &[("PROG_MULTIPLIER", Some(v))],
            |api| call_parse(api, "PROG_MULTIPLIER", dv),
        );
    }
    for i in 0..300 {
        let v = format!("0{}", rng.below(100_000_000));
        diff_with_env(
            &format!("row9/rand#{i} value={v:?}"),
            &[("PROG_MULTIPLIER", Some(&v))],
            |api| call_parse(api, "PROG_MULTIPLIER", 7),
        );
    }
}

/// Row 10: `INT_MAX`/`INT_MIN`-adjacent and beyond-64-bit digit strings.
#[test]
fn cfg_10_parse_overflowing_values() {
    let _g = lock();
    let mut rng = rng_for("cfg_10");
    let fixed = [
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483648",
        "-2147483649",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "-9223372036854775809",
        "18446744073709551616",
        "99999999999999999999999999999999",
        "-99999999999999999999999999999999",
        "000000000000000002147483648",
    ];
    for (i, v) in fixed.iter().enumerate() {
        let dv = rng.interesting_i32();
        diff_with_env(
            &format!("row10/#{i} value={v:?} default={dv}"),
            &[("PROG_BASE_OFFSET", Some(v))],
            |api| call_parse(api, "PROG_BASE_OFFSET", dv),
        );
    }
    for i in 0..400 {
        let digits = 10 + rng.below(15) as usize;
        let mut v = String::new();
        if rng.bool() {
            v.push('-');
        }
        for _ in 0..digits {
            v.push((b'0' + rng.below(10) as u8) as char);
        }
        diff_with_env(
            &format!("row10/rand#{i} value={v:?}"),
            &[("PROG_BASE_OFFSET", Some(&v))],
            |api| call_parse(api, "PROG_BASE_OFFSET", -5),
        );
    }
}

/// Row 11: randomized variable *names* — the name is echoed through `%s` in
/// both warning messages, so a long or empty name is an output-formatting case.
#[test]
fn cfg_11_parse_random_env_names() {
    let _g = lock();
    let mut rng = rng_for("cfg_11");

    let long_name = "P".repeat(200);
    let fixed_names: Vec<String> = vec![
        String::new(),
        "P".to_string(),
        "PROG_VERBOSE".to_string(),
        "PATH".to_string(),
        long_name.clone(),
        "PROG_WITH_A_QUITE_LONG_NAME_0123456789".to_string(),
    ];
    for (i, name) in fixed_names.iter().enumerate() {
        // absent — this is the only reachable state for a name `setenv` rejects
        // (empty, or containing `=`), and it is still a valid `getenv` input.
        diff_with_env(
            &format!("row11/absent#{i} name={name:?}"),
            &[(name.as_str(), None)],
            |api| call_parse(api, name, 99),
        );
        if !env_name_is_settable(name) {
            continue;
        }
        // present with a comma so the name reaches stderr through `%s`
        diff_with_env(
            &format!("row11/comma#{i} name={name:?}"),
            &[(name.as_str(), Some("1,2"))],
            |api| call_parse(api, name, 99),
        );
        // present with a semicolon
        diff_with_env(
            &format!("row11/semi#{i} name={name:?}"),
            &[(name.as_str(), Some("1;2"))],
            |api| call_parse(api, name, 99),
        );
    }
    for i in 0..300 {
        let len = 1 + rng.below(24) as usize;
        let mut name = String::from("Z");
        for _ in 0..len {
            name.push(*rng.pick(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789") as char);
        }
        let v = match rng.below(4) {
            0 => random_comma_value(&mut rng),
            1 => random_semicolon_value(&mut rng),
            2 => random_numeric_value(&mut rng),
            _ => random_value(&mut rng, 10),
        };
        let dv = rng.interesting_i32();
        diff_with_env(
            &format!("row11/rand#{i} name={name:?} value={v:?}"),
            &[(name.as_str(), Some(&v))],
            |api| call_parse(api, &name, dv),
        );
    }
}

// ===========================================================================
// init_config_from_env — rows 12..15
// ===========================================================================

/// The three states each of `PROG_VERBOSE` / `PROG_DEBUG` can be in, and the
/// two states of `PROG_OPTIMIZE` — exactly what `lib.c:70-76` distinguishes.
const VERBOSE_STATES: [Option<&str>; 3] = [None, Some("1"), Some("0")];
const DEBUG_STATES: [Option<&str>; 3] = [None, Some("1"), Some("0")];
const OPTIMIZE_STATES: [Option<&str>; 2] = [None, Some("yes")];

fn env_flag_matrix() -> Vec<Vec<(&'static str, Option<&'static str>)>> {
    let mut out = Vec::new();
    for v in VERBOSE_STATES {
        for d in DEBUG_STATES {
            for o in OPTIMIZE_STATES {
                out.push(vec![
                    ("PROG_VERBOSE", v),
                    ("PROG_DEBUG", d),
                    ("PROG_OPTIMIZE", o),
                ]);
            }
        }
    }
    out
}

/// Row 12: all 18 env-flag combinations, struct starting from zero.
#[test]
fn cfg_12_init_all_env_flag_combinations() {
    let _g = lock();
    for (i, env) in env_flag_matrix().into_iter().enumerate() {
        diff_flags_with_env(
            &format!("row12/#{i}"),
            &env,
            Flags4([0, 0, 0, 0]),
            |api, p| {
                unsafe { (api.init_config_from_env)(p) };
                0
            },
        );
    }
}

/// Row 13: same 18 combinations, but the struct pre-filled with garbage — this
/// is what proves the bit-field read-modify-write and the padding preservation
/// are byte-identical.
#[test]
fn cfg_13_init_over_garbage_preserves_padding() {
    let _g = lock();
    let mut rng = rng_for("cfg_13");
    for (i, env) in env_flag_matrix().into_iter().enumerate() {
        for j in 0..60 {
            let initial = rng.flags4();
            diff_flags_with_env(
                &format!("row13/#{i}.{j} initial={:02x?}", initial.0),
                &env,
                initial,
                |api, p| {
                    unsafe { (api.init_config_from_env)(p) };
                    0
                },
            );
        }
        // and every one of the 256 byte-0 patterns with fixed garbage tail
        for b0 in 0u8..=255 {
            let initial = Flags4([b0, 0x5A, 0xA5, 0x3C]);
            diff_flags_with_env(
                &format!("row13/#{i}.b0={b0:#04x}"),
                &env,
                initial,
                |api, p| {
                    unsafe { (api.init_config_from_env)(p) };
                    0
                },
            );
        }
    }
}

/// Row 14: `PROG_OPTIMIZE=""` still enables (only `!= NULL` is tested), and
/// `'1'` in a random position of a random string enables verbose/debug.
#[test]
fn cfg_14_init_empty_optimize_and_embedded_one() {
    let _g = lock();
    let mut rng = rng_for("cfg_14");

    for (i, o) in ["", "0", "false", "no", " ", "\t"].iter().enumerate() {
        diff_flags_with_env(
            &format!("row14/optimize#{i}={o:?}"),
            &[("PROG_OPTIMIZE", Some(o))],
            Flags4([0xFF, 0x11, 0x22, 0x33]),
            |api, p| {
                unsafe { (api.init_config_from_env)(p) };
                0
            },
        );
    }

    for i in 0..400 {
        let mut v = random_value(&mut rng, 12).replace('1', "8");
        if rng.bool() {
            let pos = rng.below(v.len() as u64 + 1) as usize;
            v.insert(pos, '1');
        }
        let mut d = random_value(&mut rng, 12).replace('1', "8");
        if rng.bool() {
            let pos = rng.below(d.len() as u64 + 1) as usize;
            d.insert(pos, '1');
        }
        let initial = rng.flags4();
        diff_flags_with_env(
            &format!("row14/rand#{i} verbose={v:?} debug={d:?}"),
            &[
                ("PROG_VERBOSE", Some(&v)),
                ("PROG_DEBUG", Some(&d)),
                ("PROG_OPTIMIZE", if rng.bool() { Some("x") } else { None }),
            ],
            initial,
            |api, p| {
                unsafe { (api.init_config_from_env)(p) };
                0
            },
        );
    }
}

/// Row 15: repeated calls on the same struct must be idempotent, and calling on
/// an already-populated struct must not accumulate state.
#[test]
fn cfg_15_init_is_idempotent() {
    let _g = lock();
    let mut rng = rng_for("cfg_15");
    for (i, env) in env_flag_matrix().into_iter().enumerate() {
        for j in 0..20 {
            let initial = rng.flags4();
            let n = 1 + rng.below(5);
            diff_flags_with_env(
                &format!("row15/#{i}.{j} x{n}"),
                &env,
                initial,
                move |api, p| {
                    for _ in 0..n {
                        unsafe { (api.init_config_from_env)(p) };
                    }
                    0
                },
            );
        }
    }
}

// ===========================================================================
// perform_operation — rows 16..21
// ===========================================================================

/// Row 16: `optimize = 1` ⇒ wrapping `val1 + val2`, over the extremes.
#[test]
fn cfg_16_perform_optimize_addition() {
    let _g = lock();
    let mut rng = rng_for("cfg_16");
    for i in 0..1500 {
        let v1 = rng.interesting_i32();
        let v2 = rng.interesting_i32();
        // optimize = 1, everything else randomized (must not matter except debug)
        let log_level = rng.below(8) as u8;
        let flags = Flags4::from_fields(
            rng.below(2) as u8,
            0,
            1,
            rng.below(2) as u8,
            log_level,
            rng.below(2) as u8,
        );
        diff_flags_with_env(
            &format!("row16/#{i} v1={v1} v2={v2} flags={:02x?}", flags.0),
            &[],
            flags,
            move |api, p| unsafe { (api.perform_operation)(v1, v2, p) },
        );
    }
}

/// Row 17: `optimize = 0` ⇒ `val1 * log_level + val2 / 2`, for every one of the
/// eight `log_level` values the 3-bit field can hold.
#[test]
fn cfg_17_perform_non_optimize_all_log_levels() {
    let _g = lock();
    let mut rng = rng_for("cfg_17");
    for log_level in 0u8..8 {
        for i in 0..400 {
            let v1 = rng.interesting_i32();
            let v2 = rng.interesting_i32();
            let flags = Flags4::from_fields(
                rng.below(2) as u8,
                0,
                0,
                rng.below(2) as u8,
                log_level,
                rng.below(2) as u8,
            );
            diff_flags_with_env(
                &format!("row17/ll={log_level}/#{i} v1={v1} v2={v2}"),
                &[],
                flags,
                move |api, p| unsafe { (api.perform_operation)(v1, v2, p) },
            );
        }
    }
}

/// Row 18: division truncation toward zero for odd/negative/`INT_MIN` `val2`.
#[test]
fn cfg_18_perform_division_truncation() {
    let _g = lock();
    let mut rng = rng_for("cfg_18");
    let v2s: [i32; 17] = [
        0,
        1,
        -1,
        2,
        -2,
        3,
        -3,
        5,
        -5,
        7,
        -7,
        99,
        -99,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ];
    for log_level in 0u8..8 {
        for (i, v2) in v2s.iter().enumerate() {
            let v1 = rng.interesting_i32();
            let flags = Flags4::from_fields(0, 0, 0, 1, log_level, 0);
            let v2 = *v2;
            diff_flags_with_env(
                &format!("row18/ll={log_level}/#{i} v1={v1} v2={v2}"),
                &[],
                flags,
                move |api, p| unsafe { (api.perform_operation)(v1, v2, p) },
            );
        }
    }
}

/// Row 19: `debug = 1` ⇒ the two debug lines (`%o` of `0755`, `%d` of result).
#[test]
fn cfg_19_perform_debug_output() {
    let _g = lock();
    let mut rng = rng_for("cfg_19");
    for optimize in 0u8..2 {
        for log_level in 0u8..8 {
            for i in 0..120 {
                let v1 = rng.interesting_i32();
                let v2 = rng.interesting_i32();
                let flags =
                    Flags4::from_fields(rng.below(2) as u8, 1, optimize, 1, log_level, 0);
                diff_flags_with_env(
                    &format!("row19/opt={optimize}/ll={log_level}/#{i} v1={v1} v2={v2}"),
                    &[],
                    flags,
                    move |api, p| unsafe { (api.perform_operation)(v1, v2, p) },
                );
            }
        }
    }
}

/// Row 20: **all 256 byte-0 bit patterns** — the out-of-range-flag sweep.
#[test]
fn cfg_20_perform_all_256_flag_patterns() {
    let _g = lock();
    let mut rng = rng_for("cfg_20");
    for b0 in 0u8..=255 {
        for i in 0..6 {
            let v1 = rng.interesting_i32();
            let v2 = rng.interesting_i32();
            let flags = Flags4([b0, 0, 0, 0]);
            diff_flags_with_env(
                &format!("row20/b0={b0:#04x}/#{i} v1={v1} v2={v2}"),
                &[],
                flags,
                move |api, p| unsafe { (api.perform_operation)(v1, v2, p) },
            );
        }
    }
}

/// Row 21: garbage in bytes 1..3 must not influence the result.
#[test]
fn cfg_21_perform_ignores_padding_garbage() {
    let _g = lock();
    let mut rng = rng_for("cfg_21");
    for i in 0..1200 {
        let v1 = rng.interesting_i32();
        let v2 = rng.interesting_i32();
        let flags = rng.flags4();
        diff_flags_with_env(
            &format!("row21/#{i} flags={:02x?} v1={v1} v2={v2}", flags.0),
            &[],
            flags,
            move |api, p| unsafe { (api.perform_operation)(v1, v2, p) },
        );
    }
}

// ===========================================================================
// apply_bit_operations — rows 22..27
// ===========================================================================

fn apply_case(tag: &str, verbose: u8, cache: u8, iters: usize, extra: &[i32]) {
    let mut rng = rng_for(tag);
    for (i, value) in extra.iter().enumerate() {
        let flags = Flags4::from_fields(verbose, 0, 0, cache, 0, 0);
        let value = *value;
        diff_flags_with_env(
            &format!("{tag}/fixed#{i} value={value}"),
            &[],
            flags,
            move |api, p| unsafe { (api.apply_bit_operations)(value, p) },
        );
    }
    for i in 0..iters {
        let value = rng.interesting_i32();
        let flags = Flags4::from_fields(
            verbose,
            rng.below(2) as u8,
            rng.below(2) as u8,
            cache,
            rng.below(8) as u8,
            rng.below(2) as u8,
        );
        diff_flags_with_env(
            &format!("{tag}/rand#{i} value={value}"),
            &[],
            flags,
            move |api, p| unsafe { (api.apply_bit_operations)(value, p) },
        );
    }
}

const SHIFT_EDGE: [i32; 14] = [
    0,
    1,
    -1,
    2,
    -2,
    15,
    16,
    -16,
    0x3FFF_FFFF,
    0x4000_0000,
    0x7FFF_FFFF,
    i32::MIN,
    i32::MIN + 1,
    -0x4000_0000,
];

/// Row 22: `verbose=0, cache_enabled=0` ⇒ identity.
#[test]
fn cfg_22_apply_identity() {
    let _g = lock();
    apply_case("row22", 0, 0, 800, &SHIFT_EDGE);
}

/// Row 23: `verbose=0, cache_enabled=1` ⇒ `value | 0x0F`.
#[test]
fn cfg_23_apply_mask_only() {
    let _g = lock();
    apply_case("row23", 0, 1, 800, &SHIFT_EDGE);
}

/// Row 24: `verbose=1, cache_enabled=0` ⇒ `value << 1`, incl. overflow.
#[test]
fn cfg_24_apply_shift_only() {
    let _g = lock();
    apply_case("row24", 1, 0, 800, &SHIFT_EDGE);
}

/// Row 25: `verbose=1, cache_enabled=1` ⇒ shift **then** mask (order matters).
#[test]
fn cfg_25_apply_shift_then_mask() {
    let _g = lock();
    apply_case("row25", 1, 1, 800, &SHIFT_EDGE);
}

/// Row 26: all 256 byte-0 bit patterns.
#[test]
fn cfg_26_apply_all_256_flag_patterns() {
    let _g = lock();
    let mut rng = rng_for("cfg_26");
    for b0 in 0u8..=255 {
        for i in 0..6 {
            let value = rng.interesting_i32();
            let flags = Flags4([b0, 0, 0, 0]);
            diff_flags_with_env(
                &format!("row26/b0={b0:#04x}/#{i} value={value}"),
                &[],
                flags,
                move |api, p| unsafe { (api.apply_bit_operations)(value, p) },
            );
        }
    }
}

/// Row 27: garbage in bytes 1..3.
#[test]
fn cfg_27_apply_ignores_padding_garbage() {
    let _g = lock();
    let mut rng = rng_for("cfg_27");
    for i in 0..1200 {
        let value = rng.interesting_i32();
        let flags = rng.flags4();
        diff_flags_with_env(
            &format!("row27/#{i} flags={:02x?} value={value}", flags.0),
            &[],
            flags,
            move |api, p| unsafe { (api.apply_bit_operations)(value, p) },
        );
    }
}

// ===========================================================================
// envy — rows 28..44
// ===========================================================================

fn envy_diff(tag: &str, env: &[(&str, Option<&str>)], p: [i32; 4]) {
    diff_with_env(
        &format!("{tag} params={p:?}"),
        env,
        move |api| unsafe { (api.envy)(p[0], p[1], p[2], p[3]) },
    );
}

/// Row 28: pristine environment ⇒ non-optimize path, silent.
#[test]
fn cfg_28_envy_clean_environment() {
    let _g = lock();
    let mut rng = rng_for("cfg_28");
    for i in 0..1200 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        envy_diff(&format!("row28/#{i}"), &[], p);
    }
}

/// Row 29: `PROG_OPTIMIZE` set ⇒ `val1 + val2` path, silent.
#[test]
fn cfg_29_envy_optimize_path() {
    let _g = lock();
    let mut rng = rng_for("cfg_29");
    for i in 0..1200 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        let o = *rng.pick(&["", "1", "0", "on", "\t"]);
        envy_diff(
            &format!("row29/#{i} optimize={o:?}"),
            &[("PROG_OPTIMIZE", Some(o))],
            p,
        );
    }
}

/// Row 30: verbose only ⇒ the five verbose stdout lines.
#[test]
fn cfg_30_envy_verbose_only() {
    let _g = lock();
    let mut rng = rng_for("cfg_30");
    for i in 0..900 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        envy_diff(
            &format!("row30/#{i}"),
            &[("PROG_VERBOSE", Some("1"))],
            p,
        );
    }
}

/// Row 31: debug only ⇒ the four debug stdout lines.
#[test]
fn cfg_31_envy_debug_only() {
    let _g = lock();
    let mut rng = rng_for("cfg_31");
    for i in 0..900 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        envy_diff(&format!("row31/#{i}"), &[("PROG_DEBUG", Some("1"))], p);
    }
}

/// Row 32: verbose **and** debug ⇒ full interleaving in the exact C order.
#[test]
fn cfg_32_envy_verbose_and_debug() {
    let _g = lock();
    let mut rng = rng_for("cfg_32");
    for i in 0..900 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        envy_diff(
            &format!("row32/#{i}"),
            &[("PROG_VERBOSE", Some("1")), ("PROG_DEBUG", Some("1"))],
            p,
        );
    }
}

/// Row 33: all 8 combinations of the three boolean env flags.
#[test]
fn cfg_33_envy_all_eight_flag_combinations() {
    let _g = lock();
    let mut rng = rng_for("cfg_33");
    for mask in 0u32..8 {
        let env: Vec<(&str, Option<&str>)> = vec![
            ("PROG_VERBOSE", if mask & 1 != 0 { Some("1") } else { None }),
            ("PROG_DEBUG", if mask & 2 != 0 { Some("1") } else { None }),
            (
                "PROG_OPTIMIZE",
                if mask & 4 != 0 { Some("1") } else { None },
            ),
        ];
        for i in 0..300 {
            let p = [
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            ];
            envy_diff(&format!("row33/mask={mask}/#{i}"), &env, p);
        }
    }
}

/// Row 34: numeric / negative / huge `PROG_BASE_OFFSET` × `PROG_MULTIPLIER`.
#[test]
fn cfg_34_envy_numeric_env_pairs() {
    let _g = lock();
    let mut rng = rng_for("cfg_34");
    for i in 0..1500 {
        let bo = format!("{}", rng.interesting_i32());
        let mu = format!("{}", rng.interesting_i32());
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        let verbose = if rng.bool() { Some("1") } else { None };
        envy_diff(
            &format!("row34/#{i} bo={bo} mu={mu}"),
            &[
                ("PROG_BASE_OFFSET", Some(&bo)),
                ("PROG_MULTIPLIER", Some(&mu)),
                ("PROG_VERBOSE", verbose),
            ],
            p,
        );
    }
    // huge / unparseable numeric strings
    let weird = [
        "99999999999999999999",
        "-99999999999999999999",
        "2147483648",
        "-2147483649",
        "abc",
        "",
        "  12  ",
        "+7",
        "0100",
    ];
    for (i, bo) in weird.iter().enumerate() {
        for (j, mu) in weird.iter().enumerate() {
            let p = [
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            ];
            envy_diff(
                &format!("row34/weird#{i}.{j} bo={bo:?} mu={mu:?}"),
                &[
                    ("PROG_BASE_OFFSET", Some(bo)),
                    ("PROG_MULTIPLIER", Some(mu)),
                ],
                p,
            );
        }
    }
}

/// Row 35: `PROG_BASE_OFFSET` rejected by the `,`/`;` checks ⇒ default `0100`
/// plus a stderr warning; crossed with verbose on/off.
#[test]
fn cfg_35_envy_base_offset_rejected() {
    let _g = lock();
    let mut rng = rng_for("cfg_35");
    for i in 0..800 {
        let bo = match rng.below(3) {
            0 => random_comma_value(&mut rng),
            1 => random_semicolon_value(&mut rng),
            _ => {
                let mut s = random_clean_value(&mut rng, 8);
                let p1 = rng.below(s.len() as u64 + 1) as usize;
                s.insert(p1, ',');
                let p2 = rng.below(s.len() as u64 + 1) as usize;
                s.insert(p2, ';');
                s
            }
        };
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        let verbose = if rng.bool() { Some("1") } else { None };
        let debug = if rng.bool() { Some("1") } else { None };
        envy_diff(
            &format!("row35/#{i} bo={bo:?}"),
            &[
                ("PROG_BASE_OFFSET", Some(&bo)),
                ("PROG_VERBOSE", verbose),
                ("PROG_DEBUG", debug),
            ],
            p,
        );
    }
}

/// Row 36: `PROG_MULTIPLIER` rejected ⇒ default `012` = 10, crossed with
/// `param3` zero / non-zero (which decides whether the multiplier is used).
#[test]
fn cfg_36_envy_multiplier_rejected() {
    let _g = lock();
    let mut rng = rng_for("cfg_36");
    for i in 0..800 {
        let mu = if rng.bool() {
            random_comma_value(&mut rng)
        } else {
            random_semicolon_value(&mut rng)
        };
        let p3 = if rng.bool() { 0 } else { rng.interesting_i32() };
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            p3,
            rng.interesting_i32(),
        ];
        let verbose = if rng.bool() { Some("1") } else { None };
        envy_diff(
            &format!("row36/#{i} mu={mu:?}"),
            &[
                ("PROG_MULTIPLIER", Some(&mu)),
                ("PROG_VERBOSE", verbose),
            ],
            p,
        );
    }
}

/// Row 37: all four zero / non-zero combinations of `param3` and `param4`.
#[test]
fn cfg_37_envy_param3_param4_zero_combinations() {
    let _g = lock();
    let mut rng = rng_for("cfg_37");
    for p3zero in [true, false] {
        for p4zero in [true, false] {
            for i in 0..400 {
                let p = [
                    rng.interesting_i32(),
                    rng.interesting_i32(),
                    if p3zero { 0 } else { 1 + rng.below(1000) as i32 },
                    if p4zero { 0 } else { 1 + rng.below(1000) as i32 },
                ];
                let mask = rng.below(8);
                let env: Vec<(&str, Option<&str>)> = vec![
                    ("PROG_VERBOSE", if mask & 1 != 0 { Some("1") } else { None }),
                    ("PROG_DEBUG", if mask & 2 != 0 { Some("1") } else { None }),
                    (
                        "PROG_OPTIMIZE",
                        if mask & 4 != 0 { Some("1") } else { None },
                    ),
                ];
                envy_diff(
                    &format!("row37/p3zero={p3zero}/p4zero={p4zero}/#{i}"),
                    &env,
                    p,
                );
            }
        }
    }
}

/// Row 38: negative `param4` ⇒ arithmetic (sign-propagating) `>> 2`.
#[test]
fn cfg_38_envy_negative_param4_arithmetic_shift() {
    let _g = lock();
    let mut rng = rng_for("cfg_38");
    let p4s: [i32; 12] = [
        -1,
        -2,
        -3,
        -4,
        -5,
        -7,
        -8,
        -1000,
        i32::MIN,
        i32::MIN + 1,
        i32::MIN + 3,
        -0x4000_0000,
    ];
    for (i, p4) in p4s.iter().enumerate() {
        for j in 0..80 {
            let p = [
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                *p4,
            ];
            let verbose = if rng.bool() { Some("1") } else { None };
            envy_diff(
                &format!("row38/#{i}.{j} p4={p4}"),
                &[("PROG_VERBOSE", verbose)],
                p,
            );
        }
    }
    for i in 0..600 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            -(1 + rng.below(1_000_000) as i32),
        ];
        envy_diff(&format!("row38/rand#{i}"), &[], p);
    }
}

/// Row 39: the `result < 0` boundary approached from both sides, by sweeping
/// `param1` across the sign change of the pre-`base_offset` value.
#[test]
fn cfg_39_envy_rollback_boundary_sweep() {
    let _g = lock();
    // With a clean environment: log_level = 3, cache_enabled = 1, verbose = 0,
    // optimize = 0, multiplier = 10, base_offset = 64:
    //   result = ((p1*3 + p2/2) + p3*10 + p4>>2) | 0x0F, then + 64
    // Sweeping p1 around -(64+15)/3 walks straight over the sign change.
    for p1 in -80i32..=10 {
        for p2 in [-4i32, -1, 0, 1, 4] {
            for p3 in [-3i32, 0, 3] {
                for p4 in [-8i32, 0, 8] {
                    envy_diff("row39/sweep", &[], [p1, p2, p3, p4]);
                }
            }
        }
    }
    // and the same sweep with the optimize path (result = p1 + p2)
    for p1 in -200i32..=0 {
        envy_diff(
            "row39/sweep-optimize",
            &[("PROG_OPTIMIZE", Some("1"))],
            [p1, 0, 0, 0],
        );
    }
}

/// Row 40: roll-back taken × interesting `param1` × verbose on/off (the
/// `Restored state from backup` line).
#[test]
fn cfg_40_envy_rollback_values_and_output() {
    let _g = lock();
    let mut rng = rng_for("cfg_40");
    let p1s: [i32; 9] = [
        0,
        -1,
        1,
        -1000,
        1000,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        i32::MAX - 1,
    ];
    for (i, p1) in p1s.iter().enumerate() {
        for j in 0..120 {
            // Drive the result strongly negative via param3 * multiplier.
            let p = [
                *p1,
                rng.interesting_i32(),
                -(1 + rng.below(100_000) as i32),
                rng.interesting_i32(),
            ];
            let mask = rng.below(8);
            let env: Vec<(&str, Option<&str>)> = vec![
                ("PROG_VERBOSE", if mask & 1 != 0 { Some("1") } else { None }),
                ("PROG_DEBUG", if mask & 2 != 0 { Some("1") } else { None }),
                (
                    "PROG_OPTIMIZE",
                    if mask & 4 != 0 { Some("1") } else { None },
                ),
            ];
            envy_diff(&format!("row40/#{i}.{j} p1={p1}"), &env, p);
        }
    }
}

/// Row 41: every parameter at an `int` extreme × all 8 env-flag combinations —
/// makes each of the five overflow-capable expressions wrap.
#[test]
fn cfg_41_envy_extremes_times_flag_combinations() {
    let _g = lock();
    let ext: [i32; 8] = [
        0,
        1,
        -1,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
        0x4000_0000,
    ];
    for mask in 0u32..8 {
        let env: Vec<(&str, Option<&str>)> = vec![
            ("PROG_VERBOSE", if mask & 1 != 0 { Some("1") } else { None }),
            ("PROG_DEBUG", if mask & 2 != 0 { Some("1") } else { None }),
            (
                "PROG_OPTIMIZE",
                if mask & 4 != 0 { Some("1") } else { None },
            ),
        ];
        for a in ext {
            for b in ext {
                envy_diff(
                    &format!("row41/mask={mask} a={a} b={b}"),
                    &env,
                    [a, b, a, b],
                );
                envy_diff(
                    &format!("row41/mask={mask} swapped a={a} b={b}"),
                    &env,
                    [b, a, b, a],
                );
            }
        }
    }
}

/// Row 42: full randomized fuzz over all five environment variables and all
/// four parameters simultaneously.
#[test]
fn cfg_42_envy_full_random_fuzz() {
    let _g = lock();
    let mut rng = rng_for("cfg_42");
    for i in 0..4000 {
        // Each variable: absent, or a random value from a rich alphabet.
        let mut vals: Vec<Option<String>> = Vec::new();
        for _ in 0..5 {
            if rng.below(4) == 0 {
                vals.push(None);
            } else {
                vals.push(Some(match rng.below(4) {
                    0 => random_numeric_value(&mut rng),
                    1 => random_comma_value(&mut rng),
                    2 => random_semicolon_value(&mut rng),
                    _ => random_value(&mut rng, 14),
                }));
            }
        }
        let env: Vec<(&str, Option<&str>)> = PROG_ENV_VARS
            .iter()
            .zip(vals.iter())
            .map(|(n, v)| (*n, v.as_deref()))
            .collect();
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        envy_diff(&format!("row42/#{i} env={vals:?}"), &env, p);
    }
}

/// Row 43: `envy` holds no state between calls (`state`, `state_backup` and
/// `buffer` are automatics) — repeated calls under one environment must agree.
#[test]
fn cfg_43_envy_is_stateless_across_calls() {
    let _g = lock();
    let mut rng = rng_for("cfg_43");
    for i in 0..500 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        let q = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        let mask = rng.below(8);
        let env: Vec<(&str, Option<&str>)> = vec![
            ("PROG_VERBOSE", if mask & 1 != 0 { Some("1") } else { None }),
            ("PROG_DEBUG", if mask & 2 != 0 { Some("1") } else { None }),
            (
                "PROG_OPTIMIZE",
                if mask & 4 != 0 { Some("1") } else { None },
            ),
        ];
        // Three calls in one capture: the concatenated output and the last
        // return value must match between C and Rust.
        diff_with_env(&format!("row43/#{i}"), &env, move |api| unsafe {
            let a = (api.envy)(p[0], p[1], p[2], p[3]);
            let b = (api.envy)(q[0], q[1], q[2], q[3]);
            let c = (api.envy)(p[0], p[1], p[2], p[3]);
            assert_eq!(a, c, "row43/#{i}: {} envy not stateless", api.name);
            a ^ b ^ c
        });
    }
}

/// Row 44: composed-pipeline check. Rebuild `envy` out of the four low-level
/// exports in the C's exact order and require the hand-composed value to equal
/// `envy`'s own return value — for the C library, for the Rust library, and
/// across the two.
#[test]
fn cfg_44_envy_equals_hand_composed_low_level_pipeline() {
    let _g = lock();
    let (c, r) = both();
    let mut rng = rng_for("cfg_44");

    // Reimplements lib.c:115-187 using only the exported entry points.
    unsafe fn compose(api: &Api, p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
        let mut flags = Flags4([0, 0, 0, 0]);
        (api.init_config_from_env)(flags.as_mut_ptr());

        let bo_name = cstring("PROG_BASE_OFFSET");
        let mu_name = cstring("PROG_MULTIPLIER");
        let base_offset = (api.parse_env_numeric)(bo_name.as_ptr(), DEF_BASE_OFFSET);
        let multiplier = (api.parse_env_numeric)(mu_name.as_ptr(), DEF_MULTIPLIER);

        let mut result = (api.perform_operation)(p1, p2, flags.as_mut_ptr());
        if p3 != 0 {
            result = result.wrapping_add(p3.wrapping_mul(multiplier));
        }
        if p4 != 0 {
            result = result.wrapping_add(p4 >> 2);
        }
        result = (api.apply_bit_operations)(result, flags.as_mut_ptr());
        result = result.wrapping_add(base_offset);
        if result < 0 {
            // roll-back: state_backup.base_value == param1
            result = p1;
        }
        result
    }

    for i in 0..2500 {
        let p = [
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        ];
        let mask = rng.below(8);
        let bo = format!("{}", rng.interesting_i32());
        let mu = format!("{}", rng.interesting_i32());
        let use_env_nums = rng.bool();
        let env: Vec<(&str, Option<&str>)> = vec![
            ("PROG_VERBOSE", if mask & 1 != 0 { Some("1") } else { None }),
            ("PROG_DEBUG", if mask & 2 != 0 { Some("1") } else { None }),
            (
                "PROG_OPTIMIZE",
                if mask & 4 != 0 { Some("1") } else { None },
            ),
            (
                "PROG_BASE_OFFSET",
                if use_env_nums { Some(&bo) } else { None },
            ),
            (
                "PROG_MULTIPLIER",
                if use_env_nums { Some(&mu) } else { None },
            ),
        ];

        env_config(&env);
        let whole_c = capture(|| unsafe { (c.envy)(p[0], p[1], p[2], p[3]) }).ret;
        env_config(&env);
        let parts_c = capture(|| unsafe { compose(c, p[0], p[1], p[2], p[3]) }).ret;
        env_config(&env);
        let whole_r = capture(|| unsafe { (r.envy)(p[0], p[1], p[2], p[3]) }).ret;
        env_config(&env);
        let parts_r = capture(|| unsafe { compose(r, p[0], p[1], p[2], p[3]) }).ret;

        assert_eq!(
            whole_c, parts_c,
            "row44/#{i}: C envy {whole_c} != C hand-composed {parts_c} (params={p:?}, env={env:?})"
        );
        assert_eq!(
            whole_r, parts_r,
            "row44/#{i}: Rust envy {whole_r} != Rust hand-composed {parts_r} (params={p:?}, env={env:?})"
        );
        assert_eq!(
            parts_c, parts_r,
            "row44/#{i}: hand-composed pipeline diverged C={parts_c} Rust={parts_r} (params={p:?}, env={env:?})"
        );
    }
}
