//! Meta-tests: prove the differential harness itself works (both `.so`s really
//! load, symbols really resolve, and `diff` really fails on a divergence).

mod common;
use common::*;

#[test]
fn both_libraries_load_and_are_distinct() {
    let (c, r) = apis();
    assert_eq!(c.name, "C");
    assert_eq!(r.name, "RUST");
    // Same symbol resolved from two different objects must be two different
    // addresses — otherwise we would be testing one library against itself.
    let ac = c.compile as usize;
    let ar = r.compile as usize;
    assert_ne!(
        ac, ar,
        "C and Rust pcre2_compile_8 resolved to the same address; \
         the harness is loading one library twice"
    );
    eprintln!("C   .so = {:?}", c_so_path());
    eprintln!("RUST.so = {:?}", rust_so_path());
}

#[test]
fn every_c_symbol_is_exported_by_rust() {
    // Phase A / D gate, enforced as a test: nm -D on both objects.
    let syms = |p: &std::path::Path| -> std::collections::BTreeSet<String> {
        let out = std::process::Command::new("nm")
            .args(["-D", "--defined-only", p.to_str().unwrap()])
            .output()
            .expect("nm");
        assert!(out.status.success());
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(|l| l.split_whitespace().nth(2).map(|s| s.to_string()))
            .collect()
    };
    let c = syms(&c_so_path());
    let r = syms(&rust_so_path());
    let missing: Vec<&String> = c.difference(&r).collect();
    assert!(
        missing.is_empty(),
        "{} symbol(s) exported by the C .so are missing from the Rust .so: {:?}",
        missing.len(),
        missing
    );
    assert!(c.len() >= 140, "implausibly few C symbols: {}", c.len());
}

#[test]
fn no_undefined_non_libc_symbols_in_rust_so() {
    let out = std::process::Command::new("nm")
        .args([
            "-D",
            "--undefined-only",
            rust_so_path().to_str().unwrap(),
        ])
        .output()
        .expect("nm");
    assert!(out.status.success());
    let bad: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().nth(1))
        .filter(|s| s.starts_with("pcre2_") || s.starts_with("_pcre2_"))
        .map(|s| s.to_string())
        .collect();
    assert!(bad.is_empty(), "undefined pcre2 symbols in Rust .so: {bad:?}");
}

/// The harness must actually detect divergence; if `diff` were a no-op every
/// other test in the suite would be vacuous.
#[test]
fn diff_detects_divergence() {
    let r = std::panic::catch_unwind(|| {
        diff("intentional", |api| {
            let mut l = Log::new();
            // Deliberately produce different logs for the two libraries.
            l.tag(api.name);
            l
        });
    });
    assert!(r.is_err(), "diff() failed to detect an injected divergence");
}

/// And it must not report a divergence when there is none.
#[test]
fn diff_accepts_agreement() {
    diff("agreement", |_api| {
        let mut l = Log::new();
        l.tag("same").u(42);
        l
    });
}
