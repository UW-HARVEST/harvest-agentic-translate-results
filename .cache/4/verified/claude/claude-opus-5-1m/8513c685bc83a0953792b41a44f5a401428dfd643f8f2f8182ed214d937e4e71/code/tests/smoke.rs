// Harness self-test: proves the loader, fork isolation and stdout capture all
// actually work, so that a "pass" in the real phases is meaningful.
mod common;
use common::*;

#[test]
fn both_libraries_export_the_symbols() {
    let p = pair();
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rust.name, "Rust");
    println!("C  : {}", c_so_path().display());
    println!("Rust: {}", rust_so_path().display());
}

#[test]
fn harness_captures_real_output_and_state_is_pristine_per_fork() {
    // A single run(0) from pristine state must print the documented 4 lines.
    // Doing it twice in separate scenarios must give the SAME transcript, which
    // proves each fork starts from the pristine `the_house`.
    assert_same("smoke_run0_a", &[Op::Run(0)]);
    assert_same("smoke_run0_b", &[Op::Run(0)]);
}

#[test]
fn harness_detects_a_real_difference() {
    // Guard against a vacuous harness: if capture were broken, these two
    // different op sequences would compare "equal" against each other. We verify
    // the transcripts of different inputs really do differ.
    let a = transcript(&[Op::Run(0)]);
    let b = transcript(&[Op::Run(7)]);
    assert_ne!(
        a, b,
        "harness is vacuous: run(0) and run(7) produced identical transcripts"
    );
    assert!(
        a.contains("The house has 2 floors, 5 bedrooms, and 2.5 bathrooms"),
        "unexpected pristine transcript:\n{}",
        a
    );
}

/// Capture just the C library's transcript, for harness introspection.
fn transcript(ops: &[Op]) -> String {
    let s = capture_c(ops);
    println!("--- transcript ---\n{}", s);
    s
}

fn capture_c(ops: &[Op]) -> String {
    common::capture_one(&pair().c, ops)
}
