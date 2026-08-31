//! Phase B rows 26-29 — differential tests of the `driver` executable
//! (`mdmain.c` vs `src/main.rs`), which composes the whole library surface.

mod common;

use common::*;
use std::process::Command;

fn run(path: &std::path::Path, args: &[String]) -> (String, String, Option<i32>) {
    let out = Command::new(path)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("spawn {}: {e}", path.display()));
    (
        String::from_utf8_lossy(&out.stdout).to_string(),
        String::from_utf8_lossy(&out.stderr).to_string(),
        out.status.code(),
    )
}

fn diff(args: &[String]) {
    let (cs, ce, cc) = run(&c_bin_path(), args);
    let (rs, re, rc) = run(&rust_bin_path(), args);
    assert_eq!(cs, rs, "driver {args:?} stdout mismatch [OP={OP} REPEAT={REPEAT}]");
    assert_eq!(
        ce.is_empty(),
        re.is_empty(),
        "driver {args:?} stderr presence mismatch"
    );
    assert_eq!(cc, rc, "driver {args:?} exit status mismatch");
}

/// Row 26 — small decimal arguments.
#[test]
fn cfg_26_main_small_args() {
    for a in -4..=4 {
        for b in -4..=4 {
            diff(&[a.to_string(), b.to_string()]);
        }
    }
}

/// Row 27 — randomized full-range arguments (exercises `summary=` wrapping).
#[test]
fn cfg_27_main_random_args() {
    let mut rng = Rng::new(SEED ^ 0xABCD);
    for _ in 0..128 {
        let a = rng.next_int();
        let b = rng.next_int();
        diff(&[a.to_string(), b.to_string()]);
    }
    for (a, b) in bounds_grid() {
        diff(&[a.to_string(), b.to_string()]);
    }
}

/// Row 28 — `atoi`-edge argument text (also covered by ERRORS rows 19/20; here as
/// part of the valid-path surface because the C accepts these without error).
#[test]
fn cfg_28_main_atoi_edges() {
    for (a, b) in [
        ("", ""),
        ("abc", "5"),
        ("12abc", "-3"),
        (" 7 ", "8"),
        ("+5", "-5"),
        ("0x10", "10"),
        ("007", "-007"),
        ("2147483648", "-2147483649"),
        ("99999999999999999999", "3"),
    ] {
        diff(&[a.to_string(), b.to_string()]);
    }
}

/// Row 29 — `argc` boundaries.
#[test]
fn cfg_29_main_argc_boundaries() {
    diff(&[]);
    diff(&["1".to_string()]);
    diff(&["1".to_string(), "2".to_string()]);
    diff(&["1".to_string(), "2".to_string(), "3".to_string()]);
    diff(&(1..=8).map(|i| i.to_string()).collect::<Vec<_>>());
}
