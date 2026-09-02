// Phase D -- symbol parity between the C `.so` and the Rust `.so`.
//
// Runs `nm -D` on both objects and asserts the C export set is a subset of the
// Rust export set, with exact names. Also asserts the Rust object has no
// unresolved non-libc imports.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

fn nm(path: &Path, extra: &str) -> BTreeSet<String> {
    let out = Command::new("nm")
        .arg("-D")
        .arg(extra)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("failed to run nm on {}: {e}", path.display()));
    assert!(
        out.status.success(),
        "nm -D {extra} {} failed: {}",
        path.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(|s| s.to_string()))
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn sym_01_every_c_export_is_exported_by_rust() {
    let c = c_so_path();
    let r = rust_so_path();
    assert!(c.exists(), "missing {}", c.display());
    assert!(r.exists(), "missing {}", r.display());

    let c_defined = nm(&c, "--defined-only");
    let r_defined = nm(&r, "--defined-only");

    let missing: Vec<&String> = c_defined.difference(&r_defined).collect();
    assert!(
        missing.is_empty(),
        "the Rust .so is missing {} symbol(s) exported by the C .so: {missing:?}\n\
         C exports:    {c_defined:?}\n\
         Rust exports: {r_defined:?}",
        missing.len()
    );

    // Guard against the table in SYMBOLS.md silently going stale.
    assert!(
        c_defined.contains("driver") && c_defined.contains("printLine"),
        "unexpected C export set (SYMBOLS.md needs regenerating): {c_defined:?}"
    );
    assert_eq!(
        c_defined.len(),
        2,
        "the C .so now exports {} symbols; regenerate SYMBOLS.md and re-derive \
         ERRORS.md / CONFIGS.md: {c_defined:?}",
        c_defined.len()
    );
}

#[test]
fn sym_02_rust_has_no_unresolved_non_libc_imports() {
    let r = rust_so_path();
    assert!(r.exists(), "missing {}", r.display());

    let undefined = nm(&r, "--undefined-only");
    let unresolved: Vec<&String> = undefined
        .iter()
        .filter(|s| {
            // Versioned libc / libgcc imports, plus the linker's own optional
            // weak hooks, are all satisfied by the platform.
            !s.contains("@GLIBC")
                && !s.contains("@GCC")
                && !s.starts_with("_ITM_")
                && s.as_str() != "__gmon_start__"
        })
        .collect();

    assert!(
        unresolved.is_empty(),
        "Rust .so has unresolved non-libc imports: {unresolved:?}"
    );
}

#[test]
fn sym_03_both_libraries_load_and_resolve_through_dlopen() {
    // The strongest form of the parity check: actually dlopen both objects and
    // dlsym every C-exported name. `load_pair` panics if either lookup fails.
    let pair = load_pair();
    assert_eq!(pair.c.name, "C");
    assert_eq!(pair.rust.name, "Rust");

    // And a smoke call through each resolved pointer, so an exported-but-broken
    // symbol cannot pass this test.
    assert_same_and_output(&pair, &[Call::Driver(3)], b"AAA\n", "sym_03 smoke driver");
    assert_same_and_output(
        &pair,
        &[Call::print_line(b"ok")],
        b"ok\n",
        "sym_03 smoke printLine",
    );
}

#[test]
fn sym_04_artifacts_exist() {
    // The Phase A artifacts are part of the deliverable; fail loudly if a
    // future edit deletes one.
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for f in ["SYMBOLS.md", "ERRORS.md", "CONFIGS.md"] {
        let p = root.join(f);
        assert!(p.exists(), "missing Phase A artifact: {}", p.display());
        let len = std::fs::metadata(&p).unwrap().len();
        assert!(len > 512, "{f} looks truncated ({len} bytes)");
    }
}
