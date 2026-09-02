//! SYMBOLS.md / Phase D — exported-symbol parity, enforced as a test so it
//! cannot silently rot.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::process::Command;

/// `nm -D --defined-only <so>` → set of symbol names.
fn defined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(
        out.status.success(),
        "nm failed on {so:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(2))
        .map(|s| s.to_string())
        .collect()
}

fn undefined_symbols(so: &std::path::Path) -> BTreeSet<String> {
    let out = Command::new("nm")
        .args(["-D", "--undefined-only"])
        .arg(so)
        .output()
        .expect("run nm");
    assert!(out.status.success());
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last())
        .map(|s| s.split('@').next().unwrap().to_string())
        .collect()
}

/// Every symbol the C `.so` exports must be exported by the Rust `.so` under
/// the exact same name. The diff MUST be empty.
#[test]
fn symbol_parity_is_exact() {
    let c = defined_symbols(&c_so_path());
    let r = defined_symbols(&rust_so_path());

    // Sanity: the C really does export the nine functions we expect, so a
    // silently-empty `nm` cannot make this test pass.
    for expected in [
        "driver",
        "Init_FileQueue",
        "Read_FileMon",
        "GetAlertData",
        "FreeAlertData",
        "merror",
        "os_calloc",
        "os_realloc",
        "os_strdup",
    ] {
        assert!(
            c.contains(expected),
            "the C .so does not export {expected} — is c_src/build stale?"
        );
    }
    assert_eq!(c.len(), 9, "unexpected C export set: {c:?}");

    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is MISSING {} C symbol(s): {missing:?}\n\
         Add the #[no_mangle] wrapper, or translate the missing C module.",
        missing.len()
    );
}

/// The Rust `.so` must not reference any *project* symbol it does not define.
/// `RTLD_NOW` resolves every relocation eagerly, so a successful open proves
/// there are no unresolved references at all.
#[test]
fn no_unresolved_symbols() {
    use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};
    for so in [c_so_path(), rust_so_path()] {
        let lib = unsafe { Library::open(Some(&so), RTLD_NOW | RTLD_LOCAL) };
        assert!(
            lib.is_ok(),
            "dlopen({so:?}, RTLD_NOW) failed — unresolved symbols: {:?}",
            lib.err()
        );
    }

    // Beyond that: every undefined symbol in the Rust .so must also be a libc
    // (or compiler-runtime) symbol, i.e. none of the project's own names.
    let c_defined = defined_symbols(&c_so_path());
    let r_undefined = undefined_symbols(&rust_so_path());
    let leaked: Vec<&String> = r_undefined.intersection(&c_defined).collect();
    assert!(
        leaked.is_empty(),
        "the Rust .so imports project symbols instead of defining them: {leaked:?}"
    );
}

/// The three `SYMBOLS.md` / `ERRORS.md` / `CONFIGS.md` artifacts must exist and
/// be non-trivial, and every table row must be checked off.
#[test]
fn artifacts_exist_and_are_complete() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for (name, min_lines) in [("SYMBOLS.md", 40), ("ERRORS.md", 40), ("CONFIGS.md", 60)] {
        let p = root.join(name);
        let text = std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{name}: {e}"));
        let lines = text.lines().count();
        assert!(
            lines >= min_lines,
            "{name} has only {lines} lines (expected >= {min_lines})"
        );
        // No unchecked boxes may remain.
        assert!(
            !text.contains("[ ]"),
            "{name} still has unchecked rows: {:?}",
            text.lines()
                .filter(|l| l.contains("[ ]"))
                .collect::<Vec<_>>()
        );
    }
}
