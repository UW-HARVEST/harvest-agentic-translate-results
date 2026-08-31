//! Phase B (valid paths) and Phase C (error paths) differential tests driven by
//! line-oriented JS corpora in `tests/corpus/`.
//!
//! Each snippet is compiled and run through BOTH `.so`s via `js_ploadstring` +
//! `js_pcall`, and the full outcome (load rc, call rc, `js_type`, `js_typeof`,
//! stack depth, and the stringified result-or-error) is compared byte for byte.
//! Because the error value is stringified, the exact error *class and message*
//! must match, not merely "both failed".
mod common;
use common::*;
use std::ffi::c_int;

/// Parse a corpus file: one snippet per line; `#`-prefixed lines and blank
/// lines are comments. `\n` inside a line becomes a real newline so multi-line
/// snippets are expressible.
fn corpus(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.trim_end())
        .filter(|l| !l.is_empty() && !l.trim_start().starts_with('#'))
        .map(|l| l.replace("\\n", "\n").replace("\\t", "\t"))
        .collect()
}

fn run_corpus(name: &str, text: &str, min_expected: usize) {
    let snippets = corpus(text);
    assert!(
        snippets.len() >= min_expected,
        "corpus {name} looks truncated: {} snippets (expected >= {min_expected})",
        snippets.len()
    );
    let mut b = Batch::new();
    for flags in [0 as c_int, JS_STRICT] {
        for s in &snippets {
            b.script(flags, s);
        }
    }
    b.finish(&format!("corpus {name} ({} snippets x 2 modes)", snippets.len()));
}

// ---------------------------------------------------------------------------
// Phase B: valid paths
// ---------------------------------------------------------------------------

#[test]
fn corpus_language() {
    run_corpus("language", include_str!("corpus/language.txt"), 150);
}

#[test]
fn corpus_builtins_object_function() {
    run_corpus(
        "builtins_object_function",
        include_str!("corpus/builtins_object_function.txt"),
        100,
    );
}

#[test]
fn corpus_builtins_array() {
    run_corpus("builtins_array", include_str!("corpus/builtins_array.txt"), 100);
}

#[test]
fn corpus_builtins_string() {
    run_corpus("builtins_string", include_str!("corpus/builtins_string.txt"), 100);
}

#[test]
fn corpus_builtins_number_math() {
    run_corpus(
        "builtins_number_math",
        include_str!("corpus/builtins_number_math.txt"),
        100,
    );
}

#[test]
fn corpus_builtins_regexp() {
    run_corpus("builtins_regexp", include_str!("corpus/builtins_regexp.txt"), 80);
}

#[test]
fn corpus_builtins_json() {
    run_corpus("builtins_json", include_str!("corpus/builtins_json.txt"), 60);
}

#[test]
fn corpus_builtins_date() {
    run_corpus("builtins_date", include_str!("corpus/builtins_date.txt"), 80);
}

#[test]
fn corpus_strict_mode() {
    // CONFIGS section B: every distinct strict-mode-dependent branch. Each
    // snippet is run in BOTH modes, so the mode-dependent difference itself is
    // part of what gets compared.
    run_corpus("strict_mode", include_str!("corpus/strict_mode.txt"), 50);
}

// ---------------------------------------------------------------------------
// Phase C: error paths
// ---------------------------------------------------------------------------

#[test]
fn corpus_errors_lexer() {
    run_corpus("errors_lexer", include_str!("corpus/errors_lexer.txt"), 40);
}

#[test]
fn corpus_errors_parser_compiler() {
    run_corpus(
        "errors_parser_compiler",
        include_str!("corpus/errors_parser_compiler.txt"),
        50,
    );
}

#[test]
fn corpus_errors_runtime() {
    run_corpus("errors_runtime", include_str!("corpus/errors_runtime.txt"), 60);
}

#[test]
fn corpus_errors_builtins() {
    run_corpus("errors_builtins", include_str!("corpus/errors_builtins.txt"), 80);
}

/// Every error snippet must ACTUALLY produce an error in the C build (otherwise
/// the row is not really testing an error path). This guards the corpora against
/// bit-rot / wrong expectations.
#[test]
fn error_corpora_actually_error_in_c() {
    let c = Impl::c();
    let mut not_erroring = Vec::new();
    let mut total = 0;
    for (name, text) in [
        ("errors_lexer", include_str!("corpus/errors_lexer.txt")),
        ("errors_parser_compiler", include_str!("corpus/errors_parser_compiler.txt")),
        ("errors_runtime", include_str!("corpus/errors_runtime.txt")),
        ("errors_builtins", include_str!("corpus/errors_builtins.txt")),
    ] {
        for s in corpus(text) {
            total += 1;
            // An error snippet must fail in at least one of the two modes.
            let a = c.eval_script(0, s.as_bytes());
            let b = c.eval_script(JS_STRICT, s.as_bytes());
            let errored = a.load_rc != 0 || a.call_rc != 0 || b.load_rc != 0 || b.call_rc != 0;
            if !errored {
                not_erroring.push(format!("  [{name}] {s:?} -> {}", a.pretty()));
            }
        }
    }
    assert!(total > 200, "error corpora look too small: {total} snippets");
    assert!(
        not_erroring.is_empty(),
        "{} of {total} \"error\" snippets do not error in the C build \
         (fix the corpus, not the Rust):\n{}",
        not_erroring.len(),
        not_erroring.join("\n")
    );
    eprintln!("all {total} error snippets genuinely error in the C build");
}

