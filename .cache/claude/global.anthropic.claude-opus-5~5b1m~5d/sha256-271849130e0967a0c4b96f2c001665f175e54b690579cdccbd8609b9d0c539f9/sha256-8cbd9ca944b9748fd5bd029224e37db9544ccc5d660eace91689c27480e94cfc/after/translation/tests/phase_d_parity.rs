// Phase D — symbol parity + harness self-check (negative control).

mod common;

use common::*;
use std::process::Command;

/// Every symbol the C `.so` exports must also be exported by the Rust `.so`,
/// with the exact same name. Enforced from inside the test suite so it cannot
/// silently rot.
#[test]
fn d_01_symbol_parity_nm() {
    fn defined_syms(path: &std::path::Path) -> Vec<String> {
        let out = Command::new("nm")
            .args(["-D", "--defined-only", path.to_str().unwrap()])
            .output()
            .expect("failed to run nm");
        assert!(out.status.success(), "nm failed on {path:?}");
        let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
            .collect();
        v.sort();
        v.dedup();
        v
    }

    let root = {
        let mut p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.pop();
        p
    };
    let c_so = std::fs::read_dir(root.join("c_src").join("build"))
        .expect("build the C library first")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().map(|x| x == "so").unwrap_or(false))
        .expect("no C .so found");
    let r_so = std::env::var("RUST_SO_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| root.join("translation/target/release/libfallcalc_lib.so"));

    let c_syms = defined_syms(&c_so);
    let r_syms = defined_syms(&r_so);

    let missing: Vec<&String> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing {} C symbol(s): {missing:?}",
        missing.len()
    );

    // Sanity: the six documented functions really are in the C export list.
    for expected in [
        "fallcalc",
        "safe_double_to_int",
        "process_array_reverse",
        "switch_fallthrough_calculator",
        "allocate_and_compute",
        "foreach_sum",
    ] {
        assert!(
            c_syms.iter().any(|s| s == expected),
            "C .so unexpectedly lacks {expected}"
        );
        assert!(
            r_syms.iter().any(|s| s == expected),
            "Rust .so lacks {expected}"
        );
    }
}

/// Negative control: prove the differential harness would actually FAIL on a
/// divergence, instead of silently passing. We feed `eq` a deliberately wrong
/// value and require it to panic.
#[test]
fn d_02_harness_detects_divergence() {
    let p = pair();
    let c = unsafe { (p.c.fallcalc)(1, 2, 3, 4) };
    let wrong = c.wrapping_add(1);
    let caught = std::panic::catch_unwind(move || eq("negative-control", c, wrong)).is_err();
    assert!(
        caught,
        "harness FAILED to detect an injected divergence -- assertions are not effective"
    );
    // And the real comparison still holds.
    let r = unsafe { (p.rust.fallcalc)(1, 2, 3, 4) };
    eq("control-positive", c, r);
}

/// Sanity check that both libraries are really distinct objects (i.e. we are
/// not accidentally comparing the C library against itself).
#[test]
fn d_03_two_distinct_libraries() {
    let p = pair();
    let cf = p.c.fallcalc as usize;
    let rf = p.rust.fallcalc as usize;
    assert_ne!(
        cf, rf,
        "C and Rust `fallcalc` resolved to the SAME address -- the same .so was loaded twice"
    );
}

/// Cross-check that the Rust `.so` under test is not an empty/stub build: each
/// symbol must produce at least two different outputs over varied inputs.
#[test]
fn d_04_rust_symbols_are_not_stubs() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 0xD4);
    let mut seen_fc = std::collections::HashSet::new();
    let mut seen_sw = std::collections::HashSet::new();
    let mut seen_sd = std::collections::HashSet::new();
    let mut seen_ac = std::collections::HashSet::new();
    let mut seen_fs = std::collections::HashSet::new();
    let mut seen_pr = std::collections::HashSet::new();
    for _ in 0..1_000 {
        let (a, b, c3, d) = (rng.next_i32(), rng.next_i32(), rng.next_i32(), rng.next_i32());
        seen_fc.insert(unsafe { (p.rust.fallcalc)(a, b, c3, d) });
        seen_sw.insert(unsafe { (p.rust.switch_fallthrough_calculator)(a, rng.range_i32(0, 4)) });
        seen_sd.insert(unsafe { (p.rust.safe_double_to_int)(rng.range_f64(-1e6, 1e6)) });
        seen_ac.insert(unsafe { (p.rust.allocate_and_compute)(rng.range_i32(1, 20), 1.5) });
        let mut buf: Vec<i32> = (0..4).map(|_| rng.next_i32()).collect();
        seen_fs.insert(unsafe { (p.rust.foreach_sum)(buf.as_mut_ptr(), 4) });
        seen_pr.insert(unsafe { (p.rust.process_array_reverse)(buf.as_mut_ptr().add(3), 4) });
    }
    for (name, set) in [
        ("fallcalc", &seen_fc),
        ("switch_fallthrough_calculator", &seen_sw),
        ("safe_double_to_int", &seen_sd),
        ("allocate_and_compute", &seen_ac),
        ("foreach_sum", &seen_fs),
        ("process_array_reverse", &seen_pr),
    ] {
        assert!(
            set.len() > 1,
            "Rust {name} looks like a stub: only {:?} ever returned",
            set
        );
    }
}
