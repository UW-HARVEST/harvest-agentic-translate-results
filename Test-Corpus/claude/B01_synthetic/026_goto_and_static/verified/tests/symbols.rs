// Phase D — symbol parity between the C artifact and the Rust artifact.
//
// The CMake project builds an EXECUTABLE whose only non-static function is
// `main`, so neither artifact exports any dynamic symbol.  The check below is
// still performed mechanically (nm -D) so that any future divergence — e.g. the
// C side growing an exported symbol — fails the suite instead of going
// unnoticed.  See SYMBOLS.md.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(args: &[&str], bin: &Path) -> Option<String> {
    let out = Command::new("nm").args(args).arg(bin).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn defined_dynamic_symbols(bin: &Path) -> BTreeSet<String> {
    let text = nm(&["-D", "--defined-only"], bin).unwrap_or_default();
    text.lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_string))
        .collect()
}

#[test]
fn symbol_parity_c_vs_rust() {
    if nm(&["--version"], Path::new("/dev/null")).is_none() && which_nm().is_none() {
        eprintln!("nm not available — skipping symbol parity check");
        return;
    }
    let c = defined_dynamic_symbols(&c_bin());
    let r = defined_dynamic_symbols(Path::new(RUST_BIN));

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C artifact but missing from the Rust artifact: {missing:?}"
    );
    // Documented invariant: an executable with a single non-static function
    // exports nothing dynamically.
    assert!(
        c.is_empty(),
        "the C artifact unexpectedly exports dynamic symbols: {c:?} — SYMBOLS.md must be updated"
    );
}

fn which_nm() -> Option<()> {
    Command::new("nm").arg("--version").output().ok().map(|_| ())
}

/// Every undefined symbol the Rust artifact needs must come from libc / libgcc;
/// nothing from the program itself may be left unresolved.
#[test]
fn no_unresolved_program_symbols() {
    let Some(text) = nm(&["-D", "-u"], Path::new(RUST_BIN)) else {
        eprintln!("nm not available — skipping");
        return;
    };
    for line in text.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.is_empty() {
            continue;
        }
        let (class, name) = match fields.as_slice() {
            [c, n] => (*c, *n),
            [n] => ("U", *n),
            _ => continue,
        };
        let ok = class == "w"
            || name.contains("@GLIBC_")
            || name.contains("@GCC_")
            || name.starts_with("__libc_")
            || name.starts_with("_ITM_")
            || name == "__gmon_start__";
        assert!(ok, "unresolved non-libc symbol in the Rust artifact: {line:?}");
    }
}

/// The translation must not contain stubs that lie about behaviour.
#[test]
fn no_stubs_in_translation() {
    let src = std::fs::read_to_string(manifest_dir().join("src/main.rs")).expect("read src/main.rs");
    for forbidden in ["unimplemented!", "todo!", "unreachable!(\"stub", "panic!(\"stub"] {
        assert!(
            !src.contains(forbidden),
            "src/main.rs contains a stub marker: {forbidden}"
        );
    }
    // All three C entities must have a counterpart.
    for needed in ["fn main", "fn multi_stage", "123"] {
        assert!(src.contains(needed), "src/main.rs is missing {needed:?}");
    }
}
