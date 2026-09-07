//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every call crosses the FFI boundary of a `.so` loaded with `libloading`;
//! neither library is linked directly.

mod common;

use common::*;
use std::ffi::{c_char, CString};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Drive `envy` in both libraries over a parameter list under one environment
/// configuration, comparing return values *and* everything written to
/// stdout/stderr byte-for-byte.
fn diff_envy(ctx: &str, cfg: &[(&str, Option<&str>)], params: &[(i32, i32, i32, i32)]) {
    let b = both();
    let _g = env_guard();
    apply_env(cfg);

    let run = |api: &Api| {
        capture(|| {
            params
                .iter()
                .map(|&(p1, p2, p3, p4)| unsafe { (api.envy)(p1, p2, p3, p4) })
                .collect::<Vec<i32>>()
        })
    };

    let (c_res, c_cap) = run(&b.c);
    let (r_res, r_cap) = run(&b.rs);
    clear_prog_env();

    for (i, (&cv, &rv)) in c_res.iter().zip(r_res.iter()).enumerate() {
        if cv != rv {
            let (p1, p2, p3, p4) = params[i];
            panic!(
                "DIVERGENCE [{ctx}] envy({p1}, {p2}, {p3}, {p4}) with env {cfg:?}: \
                 C = {cv} (0x{cv:08x}), Rust = {rv} (0x{rv:08x})"
            );
        }
    }
    assert_eq!(
        c_cap.stdout,
        r_cap.stdout,
        "DIVERGENCE [{ctx}] stdout with env {cfg:?}:\n  C    = {}\n  Rust = {}",
        show(&c_cap.stdout),
        show(&r_cap.stdout)
    );
    assert_eq!(
        c_cap.stderr,
        r_cap.stderr,
        "DIVERGENCE [{ctx}] stderr with env {cfg:?}:\n  C    = {}\n  Rust = {}",
        show(&c_cap.stderr),
        show(&r_cap.stderr)
    );
}

/// Drive `perform_operation` in both libraries over (flags, val1, val2) triples.
fn diff_perform(ctx: &str, cases: &[(FlagsWord, i32, i32)]) {
    let b = both();
    let _g = env_guard();
    let run = |api: &Api| {
        capture(|| {
            cases
                .iter()
                .map(|&(fw, v1, v2)| {
                    let mut f: FlagsWord = fw;
                    unsafe { (api.perform_operation)(v1, v2, &mut f) }
                })
                .collect::<Vec<i32>>()
        })
    };
    let (c_res, c_cap) = run(&b.c);
    let (r_res, r_cap) = run(&b.rs);
    for (i, (&cv, &rv)) in c_res.iter().zip(r_res.iter()).enumerate() {
        if cv != rv {
            let (fw, v1, v2) = cases[i];
            panic!(
                "DIVERGENCE [{ctx}] perform_operation({v1}, {v2}, flags=0x{fw:08x}): \
                 C = {cv} (0x{cv:08x}), Rust = {rv} (0x{rv:08x})"
            );
        }
    }
    assert_eq!(
        c_cap.stdout,
        r_cap.stdout,
        "DIVERGENCE [{ctx}] perform_operation stdout:\n  C    = {}\n  Rust = {}",
        show(&c_cap.stdout),
        show(&r_cap.stdout)
    );
    assert_eq!(c_cap.stderr, r_cap.stderr, "DIVERGENCE [{ctx}] perform_operation stderr");
}

/// Drive `apply_bit_operations` in both libraries over (flags, value) pairs.
fn diff_apply(ctx: &str, cases: &[(FlagsWord, i32)]) {
    let b = both();
    for &(fw, v) in cases {
        let mut fc: FlagsWord = fw;
        let mut fr: FlagsWord = fw;
        let cv = unsafe { (b.c.apply_bit_operations)(v, &mut fc) };
        let rv = unsafe { (b.rs.apply_bit_operations)(v, &mut fr) };
        if cv != rv {
            panic!(
                "DIVERGENCE [{ctx}] apply_bit_operations({v}, flags=0x{fw:08x}): \
                 C = {cv} (0x{cv:08x}), Rust = {rv} (0x{rv:08x})"
            );
        }
        assert_eq!(
            fc, fr,
            "DIVERGENCE [{ctx}] apply_bit_operations mutated the flags word differently: \
             C = 0x{fc:08x}, Rust = 0x{fr:08x} (input 0x{fw:08x})"
        );
    }
}

/// Drive `parse_env_numeric` in both libraries; compares return value and the
/// stderr warning bytes.
fn diff_parse(ctx: &str, name: &str, value: Option<&str>, default_val: i32) {
    let b = both();
    let _g = env_guard();
    match value {
        Some(v) => set_env(name, v),
        None => unset_env(name),
    }
    let cname = CString::new(name).unwrap();
    let ptr: *const c_char = cname.as_ptr();

    let (cv, ccap) = capture(|| unsafe { (b.c.parse_env_numeric)(ptr, default_val) });
    let (rv, rcap) = capture(|| unsafe { (b.rs.parse_env_numeric)(ptr, default_val) });
    unset_env(name);

    assert_eq!(
        cv, rv,
        "DIVERGENCE [{ctx}] parse_env_numeric({name}={value:?}, default={default_val}): \
         C = {cv}, Rust = {rv}"
    );
    assert_eq!(
        ccap.stderr,
        rcap.stderr,
        "DIVERGENCE [{ctx}] parse_env_numeric stderr for {name}={value:?}:\n  C    = {}\n  Rust = {}",
        show(&ccap.stderr),
        show(&rcap.stderr)
    );
    assert_eq!(
        ccap.stdout, rcap.stdout,
        "DIVERGENCE [{ctx}] parse_env_numeric wrote to stdout differently"
    );
}

/// Drive `init_config_from_env` in both libraries starting from the same
/// pre-existing garbage in the caller's storage word.
fn diff_init(ctx: &str, cfg: &[(&str, Option<&str>)], seeds: &[FlagsWord]) {
    let b = both();
    let _g = env_guard();
    apply_env(cfg);
    for &seed in seeds {
        let mut fc: FlagsWord = seed;
        let mut fr: FlagsWord = seed;
        let (_, ccap) = capture(|| unsafe { (b.c.init_config_from_env)(&mut fc) });
        let (_, rcap) = capture(|| unsafe { (b.rs.init_config_from_env)(&mut fr) });
        assert_eq!(
            fc, fr,
            "DIVERGENCE [{ctx}] init_config_from_env with env {cfg:?}, seed 0x{seed:08x}: \
             C wrote 0x{fc:08x}, Rust wrote 0x{fr:08x}"
        );
        assert_eq!(ccap.stdout, rcap.stdout, "DIVERGENCE [{ctx}] init stdout");
        assert_eq!(ccap.stderr, rcap.stderr, "DIVERGENCE [{ctx}] init stderr");
    }
    clear_prog_env();
}

fn rand_params(n: usize, seed: u64) -> Vec<(i32, i32, i32, i32)> {
    let mut r = Rng::with_seed(seed);
    (0..n)
        .map(|_| {
            (
                r.next_i32_mixed(),
                r.next_i32_mixed(),
                r.next_i32_mixed(),
                r.next_i32_mixed(),
            )
        })
        .collect()
}

const NAME: &str = "PROG_BASE_OFFSET";

// ---------------------------------------------------------------------------
// Rows 1-9 — parse_env_numeric
// ---------------------------------------------------------------------------

#[test]
fn row01_parse_absent_returns_default() {
    let mut r = Rng::with_seed(SEED ^ 1);
    for _ in 0..500 {
        diff_parse("row01", NAME, None, r.next_i32());
    }
    for d in EXTREMES {
        diff_parse("row01/extremes", NAME, None, d);
    }
}

#[test]
fn row02_parse_plain_decimal() {
    let mut r = Rng::with_seed(SEED ^ 2);
    for _ in 0..500 {
        let v = (r.next_u32() >> 1) as i32; // 0..=INT_MAX
        diff_parse("row02", NAME, Some(&v.to_string()), r.next_i32());
    }
}

#[test]
fn row03_parse_negative_decimal() {
    let mut r = Rng::with_seed(SEED ^ 3);
    for _ in 0..500 {
        let v = -((r.next_u32() >> 1) as i32);
        diff_parse("row03", NAME, Some(&v.to_string()), r.next_i32());
    }
}

#[test]
fn row04_parse_sign_whitespace_leading_zeros() {
    let mut r = Rng::with_seed(SEED ^ 4);
    let shapes: [&str; 14] = [
        "+42", "  007", "\t-9", "\n13", " +0", "-0", "0000000000", "007", "  +  7", "+", "-",
        "  ", "\u{b}5", "\u{c}6",
    ];
    for s in shapes {
        for _ in 0..20 {
            diff_parse("row04", NAME, Some(s), r.next_i32());
        }
    }
    // Randomized: random amount of leading whitespace/zeros around a number.
    for _ in 0..300 {
        let v = r.next_i32() / 4;
        let ws = ["", " ", "  ", "\t", "\n", " \t "][(r.below(6)) as usize];
        let zeros = "0".repeat(r.below(4) as usize);
        let sign = if v < 0 { "-" } else { ["", "+"][(r.below(2)) as usize] };
        let s = format!("{ws}{sign}{zeros}{}", v.unsigned_abs());
        diff_parse("row04/rand", NAME, Some(&s), r.next_i32());
    }
}

#[test]
fn row05_parse_trailing_junk() {
    let mut r = Rng::with_seed(SEED ^ 5);
    for _ in 0..400 {
        let v = r.next_i32() / 2;
        let junk = ["abc", "x", " 9", ".5", "e10", "%", "_", "\t", "?!"][(r.below(9)) as usize];
        diff_parse("row05", NAME, Some(&format!("{v}{junk}")), r.next_i32());
    }
}

#[test]
fn row06_parse_int_bounds_and_overflow() {
    let mut r = Rng::with_seed(SEED ^ 6);
    let fixed: [&str; 14] = [
        "2147483647",
        "2147483648",
        "2147483649",
        "-2147483648",
        "-2147483649",
        "-2147483647",
        "4294967295",
        "4294967296",
        "9223372036854775807",
        "9223372036854775808",
        "-9223372036854775808",
        "99999999999999",
        "-99999999999999",
        "00000000002147483647",
    ];
    for s in fixed {
        for _ in 0..10 {
            diff_parse("row06", NAME, Some(s), r.next_i32());
        }
    }
    for _ in 0..300 {
        // Random long digit strings, guaranteed to overflow int.
        let mut s = String::new();
        if r.below(2) == 0 {
            s.push('-');
        }
        let n = 11 + r.below(10) as usize;
        for _ in 0..n {
            s.push((b'0' + (r.below(10) as u8)) as char);
        }
        diff_parse("row06/rand", NAME, Some(&s), r.next_i32());
    }
}

#[test]
fn row07_parse_comma_rejection() {
    let mut r = Rng::with_seed(SEED ^ 7);
    for _ in 0..400 {
        let a = r.next_i32() / 2;
        let b = r.next_i32() / 2;
        let s = match r.below(4) {
            0 => format!("{a},{b}"),
            1 => format!(",{a}"),
            2 => format!("{a},"),
            _ => format!(","),
        };
        diff_parse("row07", NAME, Some(&s), r.next_i32());
    }
}

#[test]
fn row08_parse_semicolon_rejection() {
    let mut r = Rng::with_seed(SEED ^ 8);
    for _ in 0..400 {
        let a = r.next_i32() / 2;
        let b = r.next_i32() / 2;
        let s = match r.below(4) {
            0 => format!("{a};{b}"),
            1 => format!(";{a}"),
            2 => format!("{a};"),
            _ => format!(";"),
        };
        diff_parse("row08", NAME, Some(&s), r.next_i32());
    }
}

#[test]
fn row09_parse_comma_and_semicolon_both_orders() {
    let mut r = Rng::with_seed(SEED ^ 9);
    for _ in 0..300 {
        let a = r.next_i32() / 2;
        let s = match r.below(4) {
            0 => format!("{a},;"),
            1 => format!("{a};,"),
            2 => format!(",{a};"),
            _ => format!(";{a},"),
        };
        diff_parse("row09", NAME, Some(&s), r.next_i32());
    }
}

// ---------------------------------------------------------------------------
// Rows 10-12 — init_config_from_env
// ---------------------------------------------------------------------------

const VERBOSE_STATES: [Option<&str>; 3] = [None, Some("1"), Some("0")];
const DEBUG_STATES: [Option<&str>; 3] = [None, Some("1"), Some("0")];
const OPTIMIZE_STATES: [Option<&str>; 2] = [None, Some("1")];

#[test]
fn row10_init_full_env_cross_product() {
    let seeds: [FlagsWord; 6] = [0, 0xFFFF_FFFF, 0xDEAD_BEEF, 0x0000_00FF, 0xFFFF_FF00, 0x1234_5678];
    for v in VERBOSE_STATES {
        for d in DEBUG_STATES {
            for o in OPTIMIZE_STATES {
                diff_init(
                    "row10",
                    &[("PROG_VERBOSE", v), ("PROG_DEBUG", d), ("PROG_OPTIMIZE", o)],
                    &seeds,
                );
            }
        }
    }
}

#[test]
fn row11_init_preexisting_garbage_word() {
    let mut r = Rng::with_seed(SEED ^ 11);
    let seeds: Vec<FlagsWord> = (0..200).map(|_| r.next_u32()).collect();
    diff_init("row11/unset", &[], &seeds);
    diff_init(
        "row11/all-set",
        &[
            ("PROG_VERBOSE", Some("1")),
            ("PROG_DEBUG", Some("1")),
            ("PROG_OPTIMIZE", Some("1")),
        ],
        &seeds,
    );
}

#[test]
fn row12_init_empty_and_embedded_one() {
    let seeds: [FlagsWord; 3] = [0, 0xFFFF_FFFF, 0xA5A5_A5A5];
    let values: [Option<&str>; 8] = [
        None,
        Some(""),
        Some("0"),
        Some("1"),
        Some("x1x"),
        Some("no"),
        Some("11"),
        Some("truth1"),
    ];
    for v in values {
        for d in values {
            for o in values {
                diff_init(
                    "row12",
                    &[("PROG_VERBOSE", v), ("PROG_DEBUG", d), ("PROG_OPTIMIZE", o)],
                    &seeds,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 13-25 — perform_operation
// ---------------------------------------------------------------------------

fn perform_cases(fw: FlagsWord, n: usize, seed: u64) -> Vec<(FlagsWord, i32, i32)> {
    let mut r = Rng::with_seed(seed);
    let mut v: Vec<(FlagsWord, i32, i32)> = (0..n)
        .map(|_| (fw, r.next_i32_mixed(), r.next_i32_mixed()))
        .collect();
    for a in EXTREMES {
        for b in EXTREMES {
            v.push((fw, a, b));
        }
    }
    v
}

#[test]
fn row13_perform_optimize_nodebug() {
    let fw = flag_word(0, 0, 1, 1, 3, 0);
    diff_perform("row13", &perform_cases(fw, 3000, SEED ^ 13));
}

#[test]
fn row14_perform_optimize_debug() {
    let fw = flag_word(0, 1, 1, 1, 3, 0);
    diff_perform("row14", &perform_cases(fw, 300, SEED ^ 14));
}

#[test]
fn rows15_to_22_perform_each_log_level_nodebug() {
    for ll in 0..8u32 {
        let fw = flag_word(0, 0, 0, 1, ll, 0);
        diff_perform(
            &format!("row{}/log_level={ll}", 15 + ll),
            &perform_cases(fw, 2000, SEED ^ (0x1500 + ll as u64)),
        );
    }
}

#[test]
fn row23_perform_nooptimize_debug_random_log_level() {
    let mut r = Rng::with_seed(SEED ^ 23);
    let mut cases = Vec::new();
    for _ in 0..600 {
        let ll = r.below(8) as u32;
        let fw = flag_word(0, 1, 0, 1, ll, (r.below(2)) as u32);
        cases.push((fw, r.next_i32_mixed(), r.next_i32_mixed()));
    }
    diff_perform("row23", &cases);
}

#[test]
fn row24_perform_boundary_vector_all_log_levels() {
    let mut cases = Vec::new();
    for opt in 0..2u32 {
        for ll in 0..8u32 {
            let fw = flag_word(0, 0, opt, 1, ll, 0);
            for a in EXTREMES {
                for b in EXTREMES {
                    cases.push((fw, a, b));
                }
            }
        }
    }
    diff_perform("row24", &cases);
}

#[test]
fn row25_perform_all_256_flag_bytes() {
    let mut r = Rng::with_seed(SEED ^ 25);
    let mut cases = Vec::new();
    for byte in 0..256u32 {
        // Garbage in the 24 padding bits must not change anything.
        for pad in [0u32, 0xFFFF_FF00, r.next_u32() & 0xFFFF_FF00] {
            let fw = byte | pad;
            for _ in 0..8 {
                cases.push((fw, r.next_i32_mixed(), r.next_i32_mixed()));
            }
            for a in [i32::MIN, -1, 0, 1, i32::MAX] {
                for b in [i32::MIN, -1, 0, 1, i32::MAX] {
                    cases.push((fw, a, b));
                }
            }
        }
    }
    diff_perform("row25", &cases);
}

// ---------------------------------------------------------------------------
// Rows 26-30 — apply_bit_operations
// ---------------------------------------------------------------------------

fn apply_cases(fw: FlagsWord, n: usize, seed: u64) -> Vec<(FlagsWord, i32)> {
    let mut r = Rng::with_seed(seed);
    let mut v: Vec<(FlagsWord, i32)> = (0..n).map(|_| (fw, r.next_i32_mixed())).collect();
    for a in EXTREMES {
        v.push((fw, a));
    }
    for a in [0x4000_0000, 0x3FFF_FFFF, -0x4000_0000, -0x4000_0001, 0x7FFF_FFF0] {
        v.push((fw, a));
    }
    v
}

#[test]
fn row26_apply_identity() {
    diff_apply("row26", &apply_cases(flag_word(0, 0, 0, 0, 3, 0), 3000, SEED ^ 26));
}

#[test]
fn row27_apply_cache_only() {
    diff_apply("row27", &apply_cases(flag_word(0, 0, 0, 1, 3, 0), 3000, SEED ^ 27));
}

#[test]
fn row28_apply_shift_only() {
    diff_apply("row28", &apply_cases(flag_word(1, 0, 0, 0, 3, 0), 3000, SEED ^ 28));
}

#[test]
fn row29_apply_shift_then_or() {
    diff_apply("row29", &apply_cases(flag_word(1, 0, 0, 1, 3, 0), 3000, SEED ^ 29));
}

#[test]
fn row30_apply_all_256_flag_bytes() {
    let mut r = Rng::with_seed(SEED ^ 30);
    let mut cases = Vec::new();
    for byte in 0..256u32 {
        for pad in [0u32, 0xFFFF_FF00, r.next_u32() & 0xFFFF_FF00] {
            let fw = byte | pad;
            for _ in 0..16 {
                cases.push((fw, r.next_i32_mixed()));
            }
            for a in EXTREMES {
                cases.push((fw, a));
            }
        }
    }
    diff_apply("row30", &cases);
}

// ---------------------------------------------------------------------------
// Rows 31-51 — envy
// ---------------------------------------------------------------------------

#[test]
fn row31_envy_default_env() {
    diff_envy("row31", &[], &rand_params(4000, SEED ^ 31));
}

#[test]
fn row32_envy_optimize_only() {
    diff_envy(
        "row32",
        &[("PROG_OPTIMIZE", Some("1"))],
        &rand_params(4000, SEED ^ 32),
    );
}

#[test]
fn row33_envy_verbose_only() {
    diff_envy(
        "row33",
        &[("PROG_VERBOSE", Some("1"))],
        &rand_params(1500, SEED ^ 33),
    );
}

#[test]
fn row34_envy_debug_only() {
    diff_envy(
        "row34",
        &[("PROG_DEBUG", Some("1"))],
        &rand_params(1500, SEED ^ 34),
    );
}

#[test]
fn row35_envy_all_flags_on() {
    diff_envy(
        "row35",
        &[
            ("PROG_VERBOSE", Some("1")),
            ("PROG_DEBUG", Some("1")),
            ("PROG_OPTIMIZE", Some("1")),
        ],
        &rand_params(1500, SEED ^ 35),
    );
}

#[test]
fn row36_envy_flag_cross_product() {
    let params = rand_params(400, SEED ^ 36);
    for v in VERBOSE_STATES {
        for d in DEBUG_STATES {
            for o in [None, Some("1"), Some("0")] {
                diff_envy(
                    "row36",
                    &[("PROG_VERBOSE", v), ("PROG_DEBUG", d), ("PROG_OPTIMIZE", o)],
                    &params,
                );
            }
        }
    }
}

#[test]
fn row37_envy_base_offset_values() {
    let mut r = Rng::with_seed(SEED ^ 37);
    let params = rand_params(200, SEED ^ 0x37);
    for _ in 0..40 {
        let v = r.next_i32_mixed();
        diff_envy(
            "row37",
            &[("PROG_BASE_OFFSET", Some(&v.to_string()))],
            &params,
        );
    }
    for v in EXTREMES {
        diff_envy(
            "row37/extremes",
            &[("PROG_BASE_OFFSET", Some(&v.to_string()))],
            &params,
        );
    }
}

#[test]
fn row38_envy_multiplier_values() {
    let mut r = Rng::with_seed(SEED ^ 38);
    let params = rand_params(200, SEED ^ 0x38);
    for _ in 0..40 {
        let v = r.next_i32_mixed();
        diff_envy(
            "row38",
            &[("PROG_MULTIPLIER", Some(&v.to_string()))],
            &params,
        );
    }
    for v in EXTREMES {
        diff_envy(
            "row38/extremes",
            &[("PROG_MULTIPLIER", Some(&v.to_string()))],
            &params,
        );
    }
}

#[test]
fn row39_envy_both_numeric_vars_times_flags() {
    let mut r = Rng::with_seed(SEED ^ 39);
    let params = rand_params(150, SEED ^ 0x39);
    for _ in 0..24 {
        let bo = r.next_i32_mixed().to_string();
        let mu = r.next_i32_mixed().to_string();
        for (v, o) in [(None, None), (Some("1"), None), (None, Some("1")), (Some("1"), Some("1"))] {
            diff_envy(
                "row39",
                &[
                    ("PROG_BASE_OFFSET", Some(bo.as_str())),
                    ("PROG_MULTIPLIER", Some(mu.as_str())),
                    ("PROG_VERBOSE", v),
                    ("PROG_OPTIMIZE", o),
                ],
                &params,
            );
        }
    }
}

#[test]
fn row40_envy_rejected_numeric_vars_use_octal_defaults() {
    let params = rand_params(400, SEED ^ 40);
    let bad: [&str; 6] = ["1,2", "3;4", ",", ";", "5,;6", "7;,8"];
    for b in bad {
        diff_envy(
            "row40/base",
            &[("PROG_BASE_OFFSET", Some(b))],
            &params,
        );
        diff_envy("row40/mult", &[("PROG_MULTIPLIER", Some(b))], &params);
        diff_envy(
            "row40/both",
            &[("PROG_BASE_OFFSET", Some(b)), ("PROG_MULTIPLIER", Some(b))],
            &params,
        );
    }
}

#[test]
fn row41_envy_non_numeric_vars_yield_zero() {
    let params = rand_params(400, SEED ^ 41);
    let junk: [&str; 7] = ["", "abc", "+", "-", "0x10", " ", "\t\n"];
    for j in junk {
        diff_envy("row41/base", &[("PROG_BASE_OFFSET", Some(j))], &params);
        diff_envy("row41/mult", &[("PROG_MULTIPLIER", Some(j))], &params);
        diff_envy(
            "row41/both",
            &[("PROG_BASE_OFFSET", Some(j)), ("PROG_MULTIPLIER", Some(j))],
            &params,
        );
    }
}

#[test]
fn row42_envy_param3_zero() {
    let mut r = Rng::with_seed(SEED ^ 42);
    let params: Vec<_> = (0..3000)
        .map(|_| {
            let mut p4 = r.next_i32_mixed();
            if p4 == 0 {
                p4 = 1;
            }
            (r.next_i32_mixed(), r.next_i32_mixed(), 0, p4)
        })
        .collect();
    diff_envy("row42", &[], &params);
    diff_envy("row42/opt", &[("PROG_OPTIMIZE", Some("1"))], &params);
}

#[test]
fn row43_envy_param4_zero() {
    let mut r = Rng::with_seed(SEED ^ 43);
    let params: Vec<_> = (0..3000)
        .map(|_| {
            let mut p3 = r.next_i32_mixed();
            if p3 == 0 {
                p3 = 1;
            }
            (r.next_i32_mixed(), r.next_i32_mixed(), p3, 0)
        })
        .collect();
    diff_envy("row43", &[], &params);
    diff_envy("row43/opt", &[("PROG_OPTIMIZE", Some("1"))], &params);
}

#[test]
fn row44_envy_param3_and_param4_zero() {
    let mut r = Rng::with_seed(SEED ^ 44);
    let params: Vec<_> = (0..3000)
        .map(|_| (r.next_i32_mixed(), r.next_i32_mixed(), 0, 0))
        .collect();
    diff_envy("row44", &[], &params);
    diff_envy("row44/opt", &[("PROG_OPTIMIZE", Some("1"))], &params);
}

#[test]
fn row45_envy_negative_param4_arithmetic_shift() {
    let mut r = Rng::with_seed(SEED ^ 45);
    let mut params: Vec<(i32, i32, i32, i32)> = (0..3000)
        .map(|_| {
            let p4 = -(r.next_i32_mixed().unsigned_abs() as i64) as i32;
            let p4 = if p4 == 0 { -1 } else { p4 };
            (r.next_i32_mixed(), r.next_i32_mixed(), r.next_i32_mixed(), p4)
        })
        .collect();
    // The values where >>2 and /4 disagree.
    for p4 in [-1i32, -2, -3, -4, -5, -7, i32::MIN, i32::MIN + 1, i32::MIN + 3] {
        params.push((1, 1, 1, p4));
        params.push((0, 0, 0, p4));
    }
    diff_envy("row45", &[], &params);
}

/// Build parameter sets that are *guaranteed* to reach both sides of the
/// `result < 0` test, and assert that both sides were actually reached.
#[test]
fn row46_envy_negative_result_recovery_path() {
    let mut r = Rng::with_seed(SEED ^ 46);
    let b = both();

    // Explore, then confirm coverage of both branches.
    let params: Vec<(i32, i32, i32, i32)> = (0..8000)
        .map(|_| {
            (
                r.next_i32_mixed(),
                r.next_i32_mixed(),
                r.next_i32_mixed(),
                r.next_i32_mixed(),
            )
        })
        .collect();

    // Count how many take the recovery path under the default env: recovery
    // means the return value equals param1 while the pre-recovery value was
    // negative. We detect it structurally: the non-recovery result always has
    // its low 4 bits set (`| 0x0F`) plus base_offset 64, so results equal to
    // param1 with param1 having clear low bits are recoveries.
    {
        let _g = env_guard();
        clear_prog_env();
        let (outcome, _cap) = capture(|| {
            let mut recovered = 0usize;
            let mut bad: Option<(i32, i32, i32, i32, i32, i32)> = None;
            for &(p1, p2, p3, p4) in &params {
                let cv = unsafe { (b.c.envy)(p1, p2, p3, p4) };
                let rv = unsafe { (b.rs.envy)(p1, p2, p3, p4) };
                if cv != rv {
                    bad = Some((p1, p2, p3, p4, cv, rv));
                    break;
                }
                if cv == p1 && (p1 & 0x0F) != 0x0F {
                    recovered += 1;
                }
            }
            (recovered, bad)
        });
        let (recovered, bad) = outcome;
        if let Some((p1, p2, p3, p4, cv, rv)) = bad {
            panic!("DIVERGENCE [row46] envy({p1}, {p2}, {p3}, {p4}): C = {cv}, Rust = {rv}");
        }
        assert!(
            recovered > 100,
            "row46 did not exercise the recovery path enough ({recovered} hits)"
        );
    }

    // Hand-built cases pinned on the boundary, plus verbose on/off.
    let mut pinned: Vec<(i32, i32, i32, i32)> = Vec::new();
    for p1 in [i32::MIN, i32::MIN + 1, -1000, -100, -65, -64, -63, -1, 0, 1, 63, 64, 65] {
        for p2 in [i32::MIN, -200, -2, -1, 0, 1, 2, 200, i32::MAX] {
            pinned.push((p1, p2, 0, 0));
            pinned.push((p1, p2, 1, 1));
            pinned.push((p1, p2, -1, -1));
        }
    }
    diff_envy("row46/pinned", &[], &pinned);
    diff_envy("row46/pinned+verbose", &[("PROG_VERBOSE", Some("1"))], &pinned);
    diff_envy("row46/pinned+opt", &[("PROG_OPTIMIZE", Some("1"))], &pinned);
    diff_envy(
        "row46/pinned+verbose+opt",
        &[("PROG_VERBOSE", Some("1")), ("PROG_OPTIMIZE", Some("1"))],
        &pinned,
    );
}

#[test]
fn row47_envy_result_exactly_zero_and_minus_one() {
    // With base_offset chosen so the value entering the `result < 0` test lands
    // exactly on 0 / -1 / +1 for a wide range of computed prefixes.
    let mut params: Vec<(i32, i32, i32, i32)> = Vec::new();
    for p1 in -40..40 {
        for p2 in -8..8 {
            params.push((p1, p2, 0, 0));
        }
    }
    for off in [-79i32, -80, -81, -15, -16, -17, 0, -1, 1] {
        let s = off.to_string();
        diff_envy("row47", &[("PROG_BASE_OFFSET", Some(&s))], &params);
        diff_envy(
            "row47/opt",
            &[("PROG_BASE_OFFSET", Some(&s)), ("PROG_OPTIMIZE", Some("1"))],
            &params,
        );
        diff_envy(
            "row47/verbose",
            &[("PROG_BASE_OFFSET", Some(&s)), ("PROG_VERBOSE", Some("1"))],
            &params,
        );
    }
}

fn extreme_quads() -> Vec<(i32, i32, i32, i32)> {
    let mut v = Vec::with_capacity(EXTREMES.len().pow(4));
    for &a in &EXTREMES {
        for &b in &EXTREMES {
            for &c in &EXTREMES {
                for &d in &EXTREMES {
                    v.push((a, b, c, d));
                }
            }
        }
    }
    v
}

#[test]
fn row48_envy_extreme_quads_default() {
    diff_envy("row48", &[], &extreme_quads());
}

#[test]
fn row49_envy_extreme_quads_optimize() {
    diff_envy("row49", &[("PROG_OPTIMIZE", Some("1"))], &extreme_quads());
}

#[test]
fn row50_envy_extreme_quads_verbose() {
    diff_envy("row50", &[("PROG_VERBOSE", Some("1"))], &extreme_quads());
}

#[test]
fn row51_envy_large_random_sweep() {
    // 100k iterations per configuration, non-printing configurations so the
    // sweep stays fast; printing configurations are covered by rows 33-36/53.
    let params = rand_params(100_000, SEED ^ 51);
    diff_envy("row51/default", &[], &params);
    diff_envy("row51/opt", &[("PROG_OPTIMIZE", Some("1"))], &params);
    diff_envy(
        "row51/vars",
        &[
            ("PROG_BASE_OFFSET", Some("-12345")),
            ("PROG_MULTIPLIER", Some("777")),
        ],
        &params,
    );
    diff_envy(
        "row51/vars+opt",
        &[
            ("PROG_BASE_OFFSET", Some("2147483647")),
            ("PROG_MULTIPLIER", Some("-2147483648")),
            ("PROG_OPTIMIZE", Some("1")),
        ],
        &params,
    );
}

// ---------------------------------------------------------------------------
// Row 52 — the composed pipeline, driven through the low-level entry points
// ---------------------------------------------------------------------------

/// Re-implement `envy`'s composition using only the four low-level exports of
/// one library, and check it against that library's own `envy`. Doing this for
/// both libraries proves the low-level exports compose the same way, which a
/// per-function test cannot see.
fn compose(api: &Api, p1: i32, p2: i32, p3: i32, p4: i32) -> i32 {
    let mut flags: FlagsWord = 0;
    unsafe { (api.init_config_from_env)(&mut flags) };
    let base_name = CString::new("PROG_BASE_OFFSET").unwrap();
    let mult_name = CString::new("PROG_MULTIPLIER").unwrap();
    let base_offset = unsafe { (api.parse_env_numeric)(base_name.as_ptr(), 0o100) };
    let multiplier = unsafe { (api.parse_env_numeric)(mult_name.as_ptr(), 0o12) };

    let mut result = unsafe { (api.perform_operation)(p1, p2, &mut flags) };
    if p3 != 0 {
        result = result.wrapping_add(p3.wrapping_mul(multiplier));
    }
    if p4 != 0 {
        result = result.wrapping_add(p4 >> 2);
    }
    result = unsafe { (api.apply_bit_operations)(result, &mut flags) };
    result = result.wrapping_add(base_offset);
    if result < 0 {
        result = p1;
    }
    result
}

#[test]
fn row52_composed_pipeline_low_level() {
    let b = both();
    let mut r = Rng::with_seed(SEED ^ 52);
    let mut params = rand_params(4000, SEED ^ 0x52);
    params.extend(extreme_quads());

    let configs: [&[(&str, Option<&str>)]; 6] = [
        &[],
        &[("PROG_OPTIMIZE", Some("1"))],
        &[("PROG_VERBOSE", Some("1"))],
        &[("PROG_VERBOSE", Some("1")), ("PROG_OPTIMIZE", Some("1"))],
        &[("PROG_BASE_OFFSET", Some("-999")), ("PROG_MULTIPLIER", Some("31"))],
        &[("PROG_BASE_OFFSET", Some("1,2")), ("PROG_MULTIPLIER", Some("x")), ("PROG_OPTIMIZE", Some("1"))],
    ];

    for cfg in configs {
        let _g = env_guard();
        apply_env(cfg);
        let (bad, _cap) = capture(|| {
            for &(p1, p2, p3, p4) in &params {
                let c_envy = unsafe { (b.c.envy)(p1, p2, p3, p4) };
                let r_envy = unsafe { (b.rs.envy)(p1, p2, p3, p4) };
                let c_comp = compose(&b.c, p1, p2, p3, p4);
                let r_comp = compose(&b.rs, p1, p2, p3, p4);
                if !(c_envy == r_envy && c_envy == c_comp && r_envy == r_comp) {
                    return Some((p1, p2, p3, p4, c_envy, r_envy, c_comp, r_comp));
                }
            }
            None
        });
        clear_prog_env();
        if let Some((p1, p2, p3, p4, ce, re, cc, rc)) = bad {
            panic!(
                "DIVERGENCE [row52] env {cfg:?} params ({p1}, {p2}, {p3}, {p4}): \
                 C envy = {ce}, Rust envy = {re}, C composed = {cc}, Rust composed = {rc}"
            );
        }
        let _ = r.next_u64();
    }
}
