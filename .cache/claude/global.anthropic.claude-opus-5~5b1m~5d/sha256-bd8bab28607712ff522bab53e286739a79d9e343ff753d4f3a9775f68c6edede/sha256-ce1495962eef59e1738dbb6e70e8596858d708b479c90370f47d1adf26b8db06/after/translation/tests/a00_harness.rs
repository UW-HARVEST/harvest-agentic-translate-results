//! Sanity checks for the differential harness itself: both `.so`s load, export
//! the full symbol set, and agree on a trivial script.
mod common;
use common::*;
use std::ffi::c_int;

#[test]
fn both_libraries_load() {
    let l = libs();
    let _ = &l.c;
    let _ = &l.rust;
}

/// Every symbol listed in SYMBOLS.md (i.e. every symbol the C `.so` exports)
/// must be resolvable in BOTH libraries via `dlsym`.
#[test]
fn all_c_symbols_resolvable_in_both() {
    let list = include_str!("symbols.txt");
    let mut n = 0;
    let mut missing = Vec::new();
    for line in list.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        n += 1;
        let l = libs();
        if unsafe { l.c.get::<*const std::ffi::c_void>(s.as_bytes()) }.is_err() {
            missing.push(format!("C missing {s}"));
        }
        if unsafe { l.rust.get::<*const std::ffi::c_void>(s.as_bytes()) }.is_err() {
            missing.push(format!("Rust missing {s}"));
        }
    }
    assert!(n >= 200, "symbol list looks truncated: only {n} entries");
    assert!(missing.is_empty(), "unresolvable symbols:\n{}", missing.join("\n"));
    eprintln!("all {n} exported symbols resolvable in both .so files");
}

#[test]
fn trivial_script_agrees() {
    assert_script_eq(0, "1 + 1");
    assert_script_eq(0, "'a' + 'b'");
    assert_script_eq(0, "[1,2,3].join('-')");
    assert_script_eq(JS_STRICT, "1 + 1");
}

#[test]
fn newstate_and_freestate_roundtrip() {
    for flags in [0 as c_int, JS_STRICT, 2, 0x7fff_ffff, -1] {
        let (c, r) = Impl::both();
        let jc = c.newstate(flags);
        let jr = r.newstate(flags);
        assert_eq!(c.gettop(jc), r.gettop(jr), "gettop after newstate(flags={flags})");
        c.freestate(jc);
        r.freestate(jr);
    }
}

/// Completeness gate: every symbol the C `.so` exports must be called BY NAME
/// somewhere in the test suite, not merely resolved with `dlsym`. This is what
/// makes "symbol parity" mean "differentially tested" rather than just "present".
#[test]
fn every_exported_symbol_is_exercised_by_name() {
    let list = include_str!("symbols.txt");
    let tests_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let mut blob = String::new();
    fn walk(dir: &std::path::Path, out: &mut String) {
        for e in std::fs::read_dir(dir).expect("read tests dir") {
            let p = e.expect("entry").path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push_str(&std::fs::read_to_string(&p).expect("read test file"));
                out.push('\n');
            }
        }
    }
    walk(&tests_dir, &mut blob);

    let mut missing = Vec::new();
    let mut total = 0;
    for line in list.lines() {
        let s = line.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        total += 1;
        if !blob.contains(&format!("\"{s}\"")) {
            missing.push(s.to_string());
        }
    }
    assert!(total >= 200, "symbol list looks truncated: {total}");
    assert!(
        missing.is_empty(),
        "{} of {total} exported symbols are never called by name in the tests:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );
    eprintln!("all {total} exported symbols are called by name in the differential tests");
}

/// The `symbols.txt` fixture must still match what the C `.so` actually exports.
#[test]
fn symbols_fixture_matches_the_c_so() {
    let list: std::collections::BTreeSet<String> = include_str!("symbols.txt")
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();
    // Every listed symbol must resolve in the C .so ...
    let l = libs();
    for s in &list {
        assert!(
            unsafe { l.c.get::<*const std::ffi::c_void>(s.as_bytes()) }.is_ok(),
            "symbols.txt lists `{s}` but the C .so does not export it"
        );
        assert!(
            unsafe { l.rust.get::<*const std::ffi::c_void>(s.as_bytes()) }.is_ok(),
            "symbols.txt lists `{s}` but the Rust .so does not export it"
        );
    }
    assert_eq!(list.len(), 237, "expected 237 exported symbols, found {}", list.len());
}
