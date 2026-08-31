//! Runs EVERY corpus file found in `tests/corpus/` at run time.
//!
//! `b06_scripts.rs` binds each corpus with `include_str!`, which guarantees the
//! expected files exist at compile time. This test is the complement: it
//! discovers the directory at run time, so a corpus file that nobody wired into
//! `b06_scripts.rs` still gets exercised (and cannot silently rot).
mod common;
use common::*;
use std::ffi::c_int;

fn corpus_dir() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/corpus")
}

/// Same parsing rules as `b06_scripts.rs`.
fn parse(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty() && !l.trim_start().starts_with('#'))
        .map(|l| l.replace("\\n", "\n").replace("\\t", "\t"))
        .collect()
}

fn corpus_files() -> Vec<(String, String)> {
    let dir = corpus_dir();
    let mut out = Vec::new();
    let rd = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", dir.display()));
    for e in rd {
        let p = e.expect("dir entry").path();
        if p.extension().and_then(|s| s.to_str()) == Some("txt") {
            let name = p.file_stem().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&p)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", p.display()));
            out.push((name, text));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!out.is_empty(), "no corpus files found in {}", dir.display());
    out
}

#[test]
fn every_corpus_file_matches() {
    let files = corpus_files();
    let mut total = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for (name, text) in &files {
        let snippets = parse(text);
        assert!(
            snippets.len() >= 20,
            "corpus {name} has only {} snippets -- looks truncated",
            snippets.len()
        );
        let mut b = Batch::new();
        for flags in [0 as c_int, JS_STRICT] {
            for s in &snippets {
                b.script(flags, s);
            }
        }
        total += snippets.len() * 2;
        if !b.failures.is_empty() {
            failures.push(format!(
                "--- corpus {name}: {} of {} diverged ---\n{}",
                b.failures.len(),
                b.checked,
                b.failures.join("\n")
            ));
        } else {
            eprintln!("corpus {name}: {} cases matched", b.checked);
        }
    }
    assert!(
        failures.is_empty(),
        "{} corpus file(s) diverged:\n{}",
        failures.len(),
        failures.join("\n")
    );
    eprintln!("{} corpus files, {total} total cases, all matched", files.len());
}

/// Corpora whose name starts with `errors_` must genuinely error in the C build,
/// otherwise they are not testing an error path at all.
#[test]
fn error_corpora_really_error_in_c() {
    let c = Impl::c();
    let mut checked = 0usize;
    let mut bad: Vec<String> = Vec::new();
    let mut seen_error_corpus = false;
    for (name, text) in corpus_files() {
        if !name.starts_with("errors_") {
            continue;
        }
        seen_error_corpus = true;
        for s in parse(&text) {
            checked += 1;
            let a = c.eval_script(0, s.as_bytes());
            let b = c.eval_script(JS_STRICT, s.as_bytes());
            let errored = a.load_rc != 0 || a.call_rc != 0 || b.load_rc != 0 || b.call_rc != 0;
            if !errored {
                bad.push(format!("  [{name}] {s:?}\n      -> {}", a.pretty()));
            }
        }
    }
    assert!(seen_error_corpus, "no errors_*.txt corpora found");
    assert!(
        bad.is_empty(),
        "{} of {checked} \"error\" snippets do NOT error in the C build:\n{}",
        bad.len(),
        bad.join("\n")
    );
    eprintln!("all {checked} error-corpus snippets genuinely error in the C build");
}

/// Guard: every corpus file on disk is also wired into `b06_scripts.rs` by name,
/// so the compile-time-bound suite cannot drift from the directory.
#[test]
fn every_corpus_file_is_wired_into_b06() {
    let b06 = std::fs::read_to_string(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/b06_scripts.rs"),
    )
    .expect("read b06_scripts.rs");
    let mut missing = Vec::new();
    for (name, _) in corpus_files() {
        if !b06.contains(&format!("corpus/{name}.txt")) {
            missing.push(name);
        }
    }
    assert!(
        missing.is_empty(),
        "corpus files not referenced by b06_scripts.rs: {missing:?}"
    );
}
