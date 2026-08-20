//! Ties the Phase A artifacts to the Phase B/C tests mechanically.
//!
//! Fails if `CONFIGS.md` / `ERRORS.md` document a row that no test claims, or if
//! a test claims a row that is not documented. This is what stops a row from
//! being quietly dropped when the docs or the tests change.

mod common;
use common::*;

use std::collections::BTreeSet;

fn read(name: &str) -> String {
    let p = manifest_dir().join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()))
}

/// Row ids from the leading `| Cn |` / `| En |` cell of a markdown table row.
fn table_row_ids(md: &str, prefix: char) -> BTreeSet<String> {
    md.lines()
        .filter_map(|line| {
            let t = line.trim();
            let rest = t.strip_prefix('|')?;
            let first = rest.split('|').next()?.trim();
            let mut ch = first.chars();
            if ch.next()? != prefix {
                return None;
            }
            let digits: String = ch.collect();
            (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit()))
                .then(|| first.to_string())
        })
        .collect()
}

#[test]
fn configs_md_rows_match_the_tests() {
    let md = read("CONFIGS.md");
    let doc = table_row_ids(&md, 'C');
    let tests: BTreeSet<String> = CONFIG_ROWS.iter().map(|s| s.to_string()).collect();
    println!("CONFIGS.md documents {} rows", doc.len());
    assert!(!doc.is_empty(), "no rows parsed out of CONFIGS.md");
    assert_eq!(
        doc, tests,
        "CONFIGS.md / valid_paths.rs row mismatch\n  documented but untested: {:?}\n  tested but undocumented: {:?}",
        doc.difference(&tests).collect::<Vec<_>>(),
        tests.difference(&doc).collect::<Vec<_>>()
    );
}

#[test]
fn errors_md_rows_match_the_tests() {
    let md = read("ERRORS.md");
    // The generic-boundary appendix uses `G1..Gn`, which is intentionally a
    // cross-reference table, not a row set; only `En` rows are the surface.
    let doc = table_row_ids(&md, 'E');
    let mut tests: BTreeSet<String> = ERROR_ROWS_NONFATAL.iter().map(|s| s.to_string()).collect();
    tests.extend(ERROR_ROWS_FATAL.iter().map(|s| s.to_string()));
    println!(
        "ERRORS.md documents {} rows ({} non-fatal + {} fatal claimed by tests)",
        doc.len(),
        ERROR_ROWS_NONFATAL.len(),
        ERROR_ROWS_FATAL.len()
    );
    assert!(!doc.is_empty(), "no rows parsed out of ERRORS.md");
    assert_eq!(
        doc, tests,
        "ERRORS.md / error_paths.rs+crash_parity.rs row mismatch\n  documented but untested: {:?}\n  tested but undocumented: {:?}",
        doc.difference(&tests).collect::<Vec<_>>(),
        tests.difference(&doc).collect::<Vec<_>>()
    );
    // No row may be claimed by both binaries.
    let n: BTreeSet<&&str> = ERROR_ROWS_NONFATAL.iter().collect();
    let f: BTreeSet<&&str> = ERROR_ROWS_FATAL.iter().collect();
    assert!(
        n.intersection(&f).next().is_none(),
        "a row is claimed by both error_paths.rs and crash_parity.rs"
    );
}

#[test]
fn symbols_md_lists_every_c_symbol() {
    let md = read("SYMBOLS.md");
    for s in EXPECTED_SYMBOLS {
        assert!(
            md.contains(&format!("`{s}`")),
            "SYMBOLS.md does not mention symbol `{s}`"
        );
    }
    println!("SYMBOLS.md mentions all {} symbols", EXPECTED_SYMBOLS.len());
}

#[test]
fn configs_and_errors_rows_are_contiguous() {
    // A gap would mean a row was deleted from the docs without renumbering,
    // i.e. the surface is no longer mechanically complete.
    for (name, prefix) in [("CONFIGS.md", 'C'), ("ERRORS.md", 'E')] {
        let md = read(name);
        let mut nums: Vec<u32> = table_row_ids(&md, prefix)
            .iter()
            .map(|s| s[1..].parse().unwrap())
            .collect();
        nums.sort_unstable();
        assert_eq!(nums.first().copied(), Some(1), "{name} must start at {prefix}1");
        for (i, n) in nums.iter().enumerate() {
            assert_eq!(*n, i as u32 + 1, "{name} has a gap before {prefix}{n}");
        }
        println!("{name}: rows {prefix}1..={prefix}{}", nums.len());
    }
}
