// Phase B — valid-path differential tests for `envy`, the public entry point
// declared in `c_src/include/lib.h`. CONFIGS.md rows 37..53.
mod common;

use common::*;
use std::ffi::{c_int, CString};

#[derive(Clone, Copy, Debug)]
struct Env {
    verbose: Option<&'static str>,
    debug: Option<&'static str>,
    optimize: Option<&'static str>,
    base_offset: Option<&'static str>,
    multiplier: Option<&'static str>,
}

const DEFAULT_ENV: Env = Env {
    verbose: None,
    debug: None,
    optimize: None,
    base_offset: None,
    multiplier: None,
};

impl Env {
    fn apply(&self) {
        apply_env("PROG_VERBOSE", self.verbose);
        apply_env("PROG_DEBUG", self.debug);
        apply_env("PROG_OPTIMIZE", self.optimize);
        apply_env("PROG_BASE_OFFSET", self.base_offset);
        apply_env("PROG_MULTIPLIER", self.multiplier);
    }
}

/// The 8 verbose × debug × optimize combinations reachable from the environment.
fn flag_envs() -> Vec<Env> {
    let mut v = Vec::new();
    for verbose in [None, Some("1")] {
        for debug in [None, Some("1")] {
            for optimize in [None, Some("1")] {
                v.push(Env { verbose, debug, optimize, ..DEFAULT_ENV });
            }
        }
    }
    v
}

fn check_envy(ctx: &str, env: &Env, p1: c_int, p2: c_int, p3: c_int, p4: c_int) {
    let (p, _g) = pair();
    env.apply();
    let c = call(|| unsafe { (p.c.envy)(p1, p2, p3, p4) });
    let r = call(|| unsafe { (p.rs.envy)(p1, p2, p3, p4) });
    clear_prog_env();
    assert_same(&format!("{ctx} env={env:?} params=({p1},{p2},{p3},{p4})"), &c, &r);
}

/// Randomized sweep + full boundary sweep for one environment configuration.
fn sweep_envy(ctx: &str, env: &Env, seed: u64, n: usize) {
    let mut rng = Rng::new(seed);
    for _ in 0..n {
        check_envy(
            ctx,
            env,
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
            rng.interesting_i32(),
        );
    }
}

// ---------------------------------------------------------------------------
// rows 37..42 — flag combinations and the parameter guards
// ---------------------------------------------------------------------------

#[test]
fn row_37_all_flag_env_combinations_random_params() {
    for (i, env) in flag_envs().iter().enumerate() {
        sweep_envy("row37", env, SEED ^ (0x3700 + i as u64), 60);
    }
}

#[test]
fn row_38_param3_zero_skips_multiplier_term() {
    let mut rng = Rng::new(SEED ^ 38);
    for env in flag_envs() {
        for _ in 0..25 {
            check_envy(
                "row38",
                &env,
                rng.interesting_i32(),
                rng.interesting_i32(),
                0,
                rng.interesting_i32(),
            );
        }
    }
}

#[test]
fn row_39_param4_zero_skips_shift_term() {
    let mut rng = Rng::new(SEED ^ 39);
    for env in flag_envs() {
        for _ in 0..25 {
            check_envy(
                "row39",
                &env,
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                0,
            );
        }
    }
}

#[test]
fn row_40_both_guards_skipped() {
    let mut rng = Rng::new(SEED ^ 40);
    for env in flag_envs() {
        for _ in 0..25 {
            check_envy("row40", &env, rng.interesting_i32(), rng.interesting_i32(), 0, 0);
        }
    }
    for env in flag_envs() {
        check_envy("row40/zeros", &env, 0, 0, 0, 0);
    }
}

#[test]
fn row_41_param4_shift_boundaries() {
    for env in flag_envs() {
        for p4 in [1, 2, 3, 4, -1, -2, -3, -4, -5, 7, -7, i32::MIN, i32::MAX] {
            for p1 in [0, 1, -1, 1000] {
                check_envy("row41", &env, p1, 0, 0, p4);
            }
        }
    }
}

#[test]
fn row_42_negative_result_restore_branch() {
    // Inputs chosen so the accumulated result is < 0 before the final compare,
    // with the three interesting signs of `param1` (the restored base value).
    let mut rng = Rng::new(SEED ^ 42);
    for env in flag_envs() {
        for p1 in [5000, 0, -5000, 1, -1, i32::MAX, i32::MIN] {
            for _ in 0..8 {
                let big_neg = -(1 << 20) - (rng.below(1 << 20) as i32);
                check_envy("row42", &env, p1, big_neg, big_neg, big_neg);
            }
            check_envy("row42/fixed", &env, p1, -1000000, -1000000, -1000000);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 43..50 — PROG_BASE_OFFSET / PROG_MULTIPLIER configurations
// ---------------------------------------------------------------------------

/// Same as `check_envy` but with runtime-built (owned) env values.
fn check_envy_owned(
    ctx: &str,
    flags: &Env,
    base: Option<&str>,
    mult: Option<&str>,
    p1: c_int,
    p2: c_int,
    p3: c_int,
    p4: c_int,
) {
    let (p, _g) = pair();
    apply_env("PROG_VERBOSE", flags.verbose);
    apply_env("PROG_DEBUG", flags.debug);
    apply_env("PROG_OPTIMIZE", flags.optimize);
    apply_env("PROG_BASE_OFFSET", base);
    apply_env("PROG_MULTIPLIER", mult);
    let c = call(|| unsafe { (p.c.envy)(p1, p2, p3, p4) });
    let r = call(|| unsafe { (p.rs.envy)(p1, p2, p3, p4) });
    clear_prog_env();
    assert_same(
        &format!(
            "{ctx} flags={flags:?} base={base:?} mult={mult:?} params=({p1},{p2},{p3},{p4})"
        ),
        &c,
        &r,
    );
}

#[test]
fn row_43_custom_base_offset() {
    let mut rng = Rng::new(SEED ^ 43);
    for env in flag_envs() {
        for base in ["0", "1", "-1", "64", "100", "-100", "2147483647", "-2147483648"] {
            check_envy_owned(
                "row43/fixed",
                &env,
                Some(base),
                None,
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
        }
        for _ in 0..15 {
            let base = rng.interesting_i32().to_string();
            check_envy_owned(
                "row43/random",
                &env,
                Some(&base),
                None,
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
        }
    }
}

#[test]
fn row_44_custom_multiplier() {
    let mut rng = Rng::new(SEED ^ 44);
    for env in flag_envs() {
        for mult in ["0", "1", "-1", "10", "-10", "2147483647", "-2147483648", "3"] {
            check_envy_owned(
                "row44/fixed",
                &env,
                None,
                Some(mult),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
        }
        for _ in 0..15 {
            let mult = rng.interesting_i32().to_string();
            check_envy_owned(
                "row44/random",
                &env,
                None,
                Some(&mult),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
        }
    }
}

#[test]
fn row_45_both_customized() {
    let mut rng = Rng::new(SEED ^ 45);
    for env in flag_envs() {
        for _ in 0..25 {
            let base = rng.interesting_i32().to_string();
            let mult = rng.interesting_i32().to_string();
            check_envy_owned(
                "row45",
                &env,
                Some(&base),
                Some(&mult),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
        }
    }
}

#[test]
fn row_46_base_offset_rejected_by_comma() {
    let mut rng = Rng::new(SEED ^ 46);
    for env in flag_envs() {
        for base in ["1,2", ",", "100,", ",100", "a,b"] {
            check_envy_owned(
                "row46",
                &env,
                Some(base),
                None,
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
        }
    }
}

#[test]
fn row_47_multiplier_rejected_by_semicolon() {
    let mut rng = Rng::new(SEED ^ 47);
    for env in flag_envs() {
        for mult in ["1;2", ";", "10;", ";10", "a;b"] {
            check_envy_owned(
                "row47",
                &env,
                None,
                Some(mult),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
        }
    }
}

#[test]
fn row_48_both_rejected_warning_order() {
    let mut rng = Rng::new(SEED ^ 48);
    for env in flag_envs() {
        for (base, mult) in [
            ("1,2", "3;4"),
            ("1;2", "3,4"),
            (",", ";"),
            (";", ","),
            ("1,2;3", "4;5,6"),
        ] {
            check_envy_owned(
                "row48",
                &env,
                Some(base),
                Some(mult),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
        }
    }
}

#[test]
fn row_49_non_numeric_offset_and_multiplier() {
    let mut rng = Rng::new(SEED ^ 49);
    for env in flag_envs() {
        for (base, mult) in [
            ("abc", "xyz"),
            ("", ""),
            (" ", "\t"),
            ("0x10", "0b11"),
            ("+", "-"),
            ("12abc", "34xyz"),
        ] {
            // multiplier 0 makes the param3 term vanish even for param3 != 0
            check_envy_owned(
                "row49",
                &env,
                Some(base),
                Some(mult),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32() | 1,
                rng.interesting_i32(),
            );
        }
    }
}

#[test]
fn row_50_overflowing_offset_and_multiplier() {
    let long = "9".repeat(64);
    let mut rng = Rng::new(SEED ^ 50);
    for env in flag_envs() {
        for (base, mult) in [
            ("99999999999999", "99999999999999"),
            ("-99999999999999", "-99999999999999"),
            ("2147483648", "2147483648"),
            ("-2147483649", "-2147483649"),
            ("4294967296", "4294967297"),
        ] {
            check_envy_owned(
                "row50",
                &env,
                Some(base),
                Some(mult),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
        }
        check_envy_owned("row50/long", &env, Some(&long), Some(&long), 1, 2, 3, 4);
    }
}

// ---------------------------------------------------------------------------
// rows 51..53 — exhaustive boundary tuples and pipeline composition
// ---------------------------------------------------------------------------

#[test]
fn row_51_all_boundary_tuples_default_env() {
    for p1 in BOUNDARIES {
        for p2 in BOUNDARIES {
            for p3 in BOUNDARIES {
                for p4 in BOUNDARIES {
                    check_envy("row51", &DEFAULT_ENV, p1, p2, p3, p4);
                }
            }
        }
    }
}

#[test]
fn row_52_all_boundary_tuples_verbose_and_optimize() {
    let envs = [
        Env { verbose: Some("1"), ..DEFAULT_ENV },
        Env { optimize: Some("1"), ..DEFAULT_ENV },
        Env { verbose: Some("1"), debug: Some("1"), optimize: Some("1"), ..DEFAULT_ENV },
    ];
    for env in envs {
        for p1 in BOUNDARIES {
            for p2 in BOUNDARIES {
                for p3 in BOUNDARIES {
                    for p4 in BOUNDARIES {
                        check_envy("row52", &env, p1, p2, p3, p4);
                    }
                }
            }
        }
    }
}

/// Re-implements `envy`'s body by composing the library's OWN low-level exports,
/// so the composed pipeline is compared as well as the one-shot wrapper.
fn pipeline(imp: &Impl, p1: c_int, p2: c_int, p3: c_int, p4: c_int) -> c_int {
    let base_name = CString::new("PROG_BASE_OFFSET").unwrap();
    let mult_name = CString::new("PROG_MULTIPLIER").unwrap();
    let mut flags = Flags([0; 4]);
    unsafe {
        (imp.init_config_from_env)(flags.as_ptr());
        let base_offset = (imp.parse_env_numeric)(base_name.as_ptr(), 0o100);
        let multiplier = (imp.parse_env_numeric)(mult_name.as_ptr(), 0o12);
        let mut result = (imp.perform_operation)(p1, p2, flags.as_ptr());
        if p3 != 0 {
            result = result.wrapping_add(p3.wrapping_mul(multiplier));
        }
        if p4 != 0 {
            result = result.wrapping_add(p4 >> 2);
        }
        result = (imp.apply_bit_operations)(result, flags.as_ptr());
        result = result.wrapping_add(base_offset);
        if result < 0 {
            result = p1; // state restored from the backup ⇒ base_value == param1
        }
        result
    }
}

#[test]
fn row_53_pipeline_composition_matches_envy() {
    let (p, _g) = pair();
    let mut rng = Rng::new(SEED ^ 53);
    let mut envs = flag_envs();
    envs.push(Env { base_offset: Some("-5"), multiplier: Some("7"), ..DEFAULT_ENV });
    envs.push(Env { base_offset: Some("1,2"), multiplier: Some("3;4"), ..DEFAULT_ENV });
    envs.push(Env { verbose: Some("1"), base_offset: Some("0"), multiplier: Some("0"), ..DEFAULT_ENV });
    for env in envs {
        for _ in 0..40 {
            let (a, b, c3, d) = (
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
                rng.interesting_i32(),
            );
            env.apply();
            let (chain_c, _) = capture(|| pipeline(&p.c, a, b, c3, d));
            let (chain_r, _) = capture(|| pipeline(&p.rs, a, b, c3, d));
            let (envy_c, _) = capture(|| unsafe { (p.c.envy)(a, b, c3, d) });
            let (envy_r, _) = capture(|| unsafe { (p.rs.envy)(a, b, c3, d) });
            clear_prog_env();
            let ctx = format!("row53 env={env:?} params=({a},{b},{c3},{d})");
            assert_eq!(chain_c, chain_r, "{ctx}: composed pipelines diverge");
            assert_eq!(chain_c, envy_c, "{ctx}: C pipeline != C envy");
            assert_eq!(chain_r, envy_r, "{ctx}: Rust pipeline != Rust envy");
        }
    }
}
