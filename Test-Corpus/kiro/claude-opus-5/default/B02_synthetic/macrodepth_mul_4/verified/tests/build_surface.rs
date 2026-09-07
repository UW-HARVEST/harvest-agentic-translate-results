//! Phase D — symbol parity and the build surface.
//!
//! CONFIGS.md rows 26-27, ERRORS.md row 12.

mod common;

use common::*;
use std::process::Command;

/// CONFIGS.md row 27 — the `nm -D` diff must be empty in both directions for the
/// C-defined symbols, and symbol classes must match.
#[test]
fn row27_symbol_parity() {
    let c = defined_dynamic_symbols(c_so_path());
    let r = defined_dynamic_symbols(rust_so_path());

    let expected: &[(&str, &str)] = &[
        ("op_add", "T"),
        ("op_sub", "T"),
        ("op_mul", "T"),
        ("helper_call", "T"),
        ("helper_ptr", "T"),
        ("use_generated", "T"),
        ("G_OP", "D"),
        ("G_OP_NAME", "D"),
    ];

    // The C .so must export exactly the surface SYMBOLS.md claims.
    assert_eq!(
        c.len(),
        expected.len(),
        "[{}] C .so exports {:?}, expected {expected:?}",
        tag(),
        c
    );
    for (name, class) in expected {
        assert!(
            c.contains(&(name.to_string(), class.to_string())),
            "[{}] C .so missing {name} ({class}); has {c:?}",
            tag()
        );
    }

    // And every one of them must be in the Rust .so with the same name and class.
    let mut missing = Vec::new();
    for sym in &c {
        if !r.contains(sym) {
            missing.push(sym.clone());
        }
    }
    assert!(
        missing.is_empty(),
        "[{}] symbols missing from the Rust .so: {missing:?}",
        tag()
    );
}

/// The Rust `.so` must have no unresolved non-libc symbol -- an untranslated
/// module would show up as one.
#[test]
fn row27_no_unresolved_symbols() {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(rust_so_path())
        .output()
        .expect("run nm");
    assert!(out.status.success());
    let leftovers: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let mut it = l.split_whitespace();
            let class = it.next()?;
            if class != "U" {
                return None; // weak ELF/glibc hooks
            }
            let name = it.next()?;
            let is_rust_runtime = name.starts_with("_ZN")
                || name.starts_with("_R")
                || name.starts_with("rust_")
                || name.contains("_Unwind")
                || name.starts_with("__rdl_")
                || name.starts_with("__rust_");
            let is_libc = name.contains("@GLIBC") || name.contains("@GCC") || name.starts_with("__");
            if is_rust_runtime || is_libc {
                None
            } else {
                Some(name.to_string())
            }
        })
        .collect();
    assert!(
        leftovers.is_empty(),
        "[{}] unresolved non-libc symbols in the Rust .so: {leftovers:?}",
        tag()
    );
}

/// ERRORS.md row 12 — build-time rejection. `-DREPEAT=8` and `-DOP=div` must
/// both fail to compile, which is what bounds the Rust feature set to
/// `add|sub|mul` x `0..=7`.
#[test]
fn row12_out_of_range_build_config() {
    let root = repo_root();
    let src = root.join("c_src/src/mdcore.c");
    let out = root.join("cbuild/harness/reject_probe.so");
    std::fs::create_dir_all(root.join("cbuild/harness")).unwrap();

    let try_build = |op: &str, repeat: &str| -> bool {
        Command::new("gcc")
            .args(["-O2", "-fPIC", "-std=c11", "-Werror=implicit-function-declaration"])
            .arg(format!("-DOP={op}"))
            .arg(format!("-DREPEAT={repeat}"))
            .arg("-shared")
            .arg("-o")
            .arg(&out)
            .arg(&src)
            .output()
            .expect("run gcc")
            .status
            .success()
    };

    // The 24 valid configurations all build.
    for op in ["add", "sub", "mul"] {
        for r in 0..=7 {
            assert!(
                try_build(op, &r.to_string()),
                "OP={op} REPEAT={r} should compile"
            );
        }
    }
    // REPEAT past REP7 -> CHOOSE_REP names an undefined REP8.
    for r in [8, 9, 12, 100] {
        assert!(
            !try_build("add", &r.to_string()),
            "REPEAT={r} should be rejected at compile time (no REP{r})"
        );
    }
    // OP outside {add,sub,mul} -> no INIT_<op>/STEP_<op>/op_<op>.
    for op in ["div", "mod", "xor", "foo"] {
        assert!(
            !try_build(op, "5"),
            "OP={op} should be rejected at compile time"
        );
    }
    let _ = std::fs::remove_file(&out);
}

/// CONFIGS.md row 26 — the Rust side accepts exactly the same 24 configurations.
/// Only the feature pair this binary was built with is asserted here (cheap);
/// `run_all.sh` covers all 24 by re-running the suite, and the loop in
/// `check_features.sh` runs `cargo check` for each.
#[test]
fn row26_this_build_config_is_valid() {
    assert!(matches!(OP, Op::Add | Op::Sub | Op::Mul));
    assert!((0..=7).contains(&REPEAT), "REPEAT={REPEAT} out of range");
    assert!(c_so_path().exists(), "C .so was not produced");
    assert!(rust_so_path().exists(), "Rust .so was not produced");
}
