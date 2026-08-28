//! Mechanical Phase B / Phase C gate.
//!
//! Parses the row IDs out of `CONFIGS.md` and `ERRORS.md` and requires that a
//! correspondingly named test function exists in `tests/configs.rs` /
//! `tests/errors.rs`. This makes it impossible to "check off" a row in the
//! tables without an actual differential test backing it, and impossible to add
//! a row later without noticing that it is untested.

use std::collections::BTreeSet;
use std::path::Path;

fn read(rel: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
}

/// Row IDs from a markdown table: lines like `| C12 | ... |` or `| E3 | ... |`.
fn table_rows(md: &str, prefix: char) -> BTreeSet<u32> {
    md.lines()
        .filter_map(|l| {
            let l = l.trim();
            let first = l.strip_prefix('|')?.trim();
            let id = first.split('|').next()?.trim();
            let rest = id.strip_prefix(prefix)?;
            rest.parse::<u32>().ok()
        })
        .collect()
}

/// Test function names in a test file: `fn c12_...` / `fn e3_...`.
fn test_fns(src: &str, prefix: char) -> BTreeSet<u32> {
    src.lines()
        .filter_map(|l| {
            let l = l.trim();
            let name = l.strip_prefix("fn ")?;
            let rest = name.strip_prefix(prefix)?;
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            if digits.is_empty() || !rest[digits.len()..].starts_with('_') {
                return None;
            }
            digits.parse::<u32>().ok()
        })
        .collect()
}

#[test]
fn phase_b_every_configs_row_has_a_test() {
    let rows = table_rows(&read("CONFIGS.md"), 'C');
    let tests = test_fns(&read("tests/configs.rs"), 'c');
    assert!(!rows.is_empty(), "no rows parsed from CONFIGS.md");
    let untested: Vec<_> = rows.difference(&tests).collect();
    assert!(
        untested.is_empty(),
        "CONFIGS.md rows with NO differential test: {untested:?}\n\
         (rows found: {}, tests found: {})",
        rows.len(),
        tests.len()
    );
    let orphan: Vec<_> = tests.difference(&rows).collect();
    assert!(orphan.is_empty(), "tests with no CONFIGS.md row: {orphan:?}");
    eprintln!("Phase B gate: {} CONFIGS.md rows, all have tests", rows.len());
}

#[test]
fn phase_c_every_errors_row_has_a_test() {
    let rows = table_rows(&read("ERRORS.md"), 'E');
    let tests = test_fns(&read("tests/errors.rs"), 'e');
    assert!(!rows.is_empty(), "no rows parsed from ERRORS.md");
    let untested: Vec<_> = rows.difference(&tests).collect();
    assert!(
        untested.is_empty(),
        "ERRORS.md rows with NO error-path differential test: {untested:?}\n\
         (rows found: {}, tests found: {})",
        rows.len(),
        tests.len()
    );
    let orphan: Vec<_> = tests.difference(&rows).collect();
    assert!(orphan.is_empty(), "tests with no ERRORS.md row: {orphan:?}");
    eprintln!("Phase C gate: {} ERRORS.md rows, all have tests", rows.len());
}

/// Guard against the failure mode that silently invalidated an earlier version
/// of this suite: `cargo test` does not build `cdylib` artifacts, so a stale
/// `.so` could be compared instead of the current source.
#[test]
fn harness_guards_against_stale_shared_objects() {
    let src = read("tests/common/mod.rs");
    assert!(
        src.contains("build_rust_cdylib"),
        "the harness must build the cdylib itself"
    );
    assert!(
        src.contains("STALE Rust .so") && src.contains("STALE C .so"),
        "the harness must assert both shared objects are newer than their sources"
    );
}
