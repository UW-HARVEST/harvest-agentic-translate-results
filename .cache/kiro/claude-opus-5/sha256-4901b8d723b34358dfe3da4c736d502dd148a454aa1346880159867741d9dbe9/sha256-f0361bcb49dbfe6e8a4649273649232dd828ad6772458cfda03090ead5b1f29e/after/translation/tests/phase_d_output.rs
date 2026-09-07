//! `CONFIGS.md` row 53 — byte-for-byte comparison of everything each `.so`
//! writes to fd 1 and fd 2, compared **per call** so that ordering and
//! interleaving are checked, not just the aggregate.
//!
//! The project builds no executable (`c_src/CMakeLists.txt` declares only
//! `add_library(... SHARED ...)`, and the crate has no `[[bin]]` / `src/main.rs`),
//! so there is no driver stdout to compare; the library's own printf output is
//! the equivalent surface and is compared here.

mod common;

use common::*;
use std::ffi::CString;

#[track_caller]
fn diff_call_output(ctx: &str, cfg: &[(&str, Option<&str>)], p: (i32, i32, i32, i32)) {
    let b = both();
    let _g = env_guard();
    apply_env(cfg);
    let (cv, ccap) = capture(|| unsafe { (b.c.envy)(p.0, p.1, p.2, p.3) });
    let (rv, rcap) = capture(|| unsafe { (b.rs.envy)(p.0, p.1, p.2, p.3) });
    clear_prog_env();

    assert_eq!(
        cv, rv,
        "DIVERGENCE [{ctx}] envy{p:?} env {cfg:?}: C = {cv}, Rust = {rv}"
    );
    assert_eq!(
        ccap.stdout,
        rcap.stdout,
        "DIVERGENCE [{ctx}] stdout for envy{p:?} env {cfg:?}:\n  C    = {}\n  Rust = {}",
        show(&ccap.stdout),
        show(&rcap.stdout)
    );
    assert_eq!(
        ccap.stderr,
        rcap.stderr,
        "DIVERGENCE [{ctx}] stderr for envy{p:?} env {cfg:?}:\n  C    = {}\n  Rust = {}",
        show(&ccap.stderr),
        show(&rcap.stderr)
    );
}

fn printing_configs() -> Vec<Vec<(&'static str, Option<&'static str>)>> {
    let mut v = Vec::new();
    for verbose in [None, Some("1")] {
        for debug in [None, Some("1")] {
            for optimize in [None, Some("1")] {
                for (bo, mu) in [
                    (None, None),
                    (Some("100"), Some("7")),
                    (Some("-5000"), Some("-3")),
                    (Some("1,2"), Some("3;4")),
                    (Some("abc"), Some("")),
                    (Some("2147483647"), Some("2147483648")),
                ] {
                    v.push(vec![
                        ("PROG_VERBOSE", verbose),
                        ("PROG_DEBUG", debug),
                        ("PROG_OPTIMIZE", optimize),
                        ("PROG_BASE_OFFSET", bo),
                        ("PROG_MULTIPLIER", mu),
                    ]);
                }
            }
        }
    }
    v
}

#[test]
fn row53_per_call_output_bytes_match() {
    let mut r = Rng::with_seed(SEED ^ 53);
    let mut params: Vec<(i32, i32, i32, i32)> = vec![
        (0, 0, 0, 0),
        (1, 1, 1, 1),
        (-1, -1, -1, -1),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, 0, 0, 0),
        (-7, -9, 0, 0),
        (100, 200, 300, 400),
        (-100, -200, -300, -400),
    ];
    for _ in 0..12 {
        params.push((
            r.next_i32_mixed(),
            r.next_i32_mixed(),
            r.next_i32_mixed(),
            r.next_i32_mixed(),
        ));
    }

    for cfg in printing_configs() {
        for &p in &params {
            diff_call_output("row53", &cfg, p);
        }
    }
}

/// The stderr warnings of `parse_env_numeric` (rows 7-9 of `CONFIGS.md`)
/// compared per call, including the `%s` name substitution.
#[test]
fn row53_parse_warning_bytes_match() {
    let b = both();
    let _g = env_guard();
    let names = [
        "PROG_BASE_OFFSET",
        "PROG_MULTIPLIER",
        "A",
        "WITH_UNDERSCORES_AND_A_QUITE_LONG_NAME_0123456789",
    ];
    let values = ["1,2", ",", "1;2", ";", "1,2;3", "1;2,3", ",;", ";,"];
    for n in names {
        for v in values {
            clear_prog_env();
            set_env(n, v);
            let cn = CString::new(n).unwrap();
            let (cv, ccap) = capture(|| unsafe { (b.c.parse_env_numeric)(cn.as_ptr(), 31337) });
            let (rv, rcap) = capture(|| unsafe { (b.rs.parse_env_numeric)(cn.as_ptr(), 31337) });
            unset_env(n);
            assert_eq!(cv, rv, "value diverged for {n}={v:?}");
            assert_eq!(
                ccap.stderr,
                rcap.stderr,
                "stderr diverged for {n}={v:?}:\n  C    = {}\n  Rust = {}",
                show(&ccap.stderr),
                show(&rcap.stderr)
            );
            assert!(
                !ccap.stderr.is_empty(),
                "expected a warning from C for {n}={v:?}"
            );
            assert!(ccap.stdout.is_empty() && rcap.stdout.is_empty());
        }
    }
    clear_prog_env();
}

/// `perform_operation`'s two debug lines, compared per call for every log level
/// and both optimize settings — this is where the octal `%o` formatting of
/// `operation_mode = 0755` is checked.
#[test]
fn row53_perform_debug_output_bytes_match() {
    let b = both();
    let _g = env_guard();
    let mut r = Rng::with_seed(SEED ^ 0x53);
    for ll in 0..8u32 {
        for opt in 0..2u32 {
            let w = flag_word(0, 1, opt, 1, ll, 0);
            let mut vals: Vec<(i32, i32)> = EXTREMES
                .iter()
                .flat_map(|&a| EXTREMES.iter().map(move |&b2| (a, b2)))
                .collect();
            for _ in 0..8 {
                vals.push((r.next_i32_mixed(), r.next_i32_mixed()));
            }
            for (v1, v2) in vals {
                let mut fc = w;
                let mut fr = w;
                let (cv, ccap) = capture(|| unsafe { (b.c.perform_operation)(v1, v2, &mut fc) });
                let (rv, rcap) = capture(|| unsafe { (b.rs.perform_operation)(v1, v2, &mut fr) });
                assert_eq!(
                    cv, rv,
                    "perform_operation({v1}, {v2}, 0x{w:08x}): C = {cv}, Rust = {rv}"
                );
                assert_eq!(
                    ccap.stdout,
                    rcap.stdout,
                    "debug stdout diverged for perform_operation({v1}, {v2}, 0x{w:08x}):\n  C    = {}\n  Rust = {}",
                    show(&ccap.stdout),
                    show(&rcap.stdout)
                );
                assert!(!ccap.stdout.is_empty(), "expected debug output from C");
            }
        }
    }
}

/// `init_config_from_env` and `apply_bit_operations` must print *nothing*, in
/// both libraries.
#[test]
fn row53_silent_functions_stay_silent() {
    let b = both();
    let _g = env_guard();
    for cfg in [
        &[][..],
        &[("PROG_VERBOSE", Some("1")), ("PROG_DEBUG", Some("1"))][..],
    ] {
        apply_env(cfg);
        let mut fc: FlagsWord = 0xFFFF_FFFF;
        let mut fr: FlagsWord = 0xFFFF_FFFF;
        let (_, cc) = capture(|| unsafe { (b.c.init_config_from_env)(&mut fc) });
        let (_, rc) = capture(|| unsafe { (b.rs.init_config_from_env)(&mut fr) });
        assert_eq!(fc, fr);
        assert!(cc.stdout.is_empty() && cc.stderr.is_empty(), "C init printed: {}", show(&cc.stdout));
        assert!(rc.stdout.is_empty() && rc.stderr.is_empty(), "Rust init printed: {}", show(&rc.stdout));

        for w in [0u32, 0xFF, 0xFFFF_FFFF, 0x0A] {
            let mut a = w;
            let mut c2 = w;
            let (cv, cc) = capture(|| unsafe { (b.c.apply_bit_operations)(-12345, &mut a) });
            let (rv, rc) = capture(|| unsafe { (b.rs.apply_bit_operations)(-12345, &mut c2) });
            assert_eq!(cv, rv);
            assert!(cc.stdout.is_empty() && cc.stderr.is_empty());
            assert!(rc.stdout.is_empty() && rc.stderr.is_empty());
        }
    }
    clear_prog_env();
}
