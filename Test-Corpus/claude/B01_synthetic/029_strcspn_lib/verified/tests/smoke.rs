// Harness self-check: proves the two `.so`s are loaded independently, that
// stdout capture works, and that the C library agrees with the naive strcspn
// contract (validating the harness before any real comparison is trusted).

mod common;

use common::*;

#[test]
fn smoke_both_libraries_load_and_export_driver() {
    let h = Harness::new();
    // Harness::new() already asserts the two `driver` addresses differ.
    let cases = vec![Case::new(b"abcde", b"cd")];
    let c = h.capture_c(&cases.iter().map(|c| c.ptrs()).collect::<Vec<_>>());
    let r = h.capture_rs(&cases.iter().map(|c| c.ptrs()).collect::<Vec<_>>());
    assert_eq!(c, b"2\n", "C output for strcspn(\"abcde\",\"cd\")");
    assert_eq!(r, b"2\n", "Rust output for strcspn(\"abcde\",\"cd\")");
}

#[test]
fn smoke_harness_sanity_c_matches_naive_oracle() {
    let h = Harness::new();
    let mut rng = Rng::new(SEED ^ 0xA11CE);
    let alphabet = all_nonzero_bytes();
    let mut cases = Vec::new();
    for _ in 0..500 {
        let n1 = rng.range(0, 64);
        let n2 = rng.range(0, 16);
        let s1 = rng.string_from(n1, &alphabet);
        let s2 = rng.string_from(n2, &alphabet);
        cases.push(Case::raw(s1, s2));
    }
    h.assert_c_matches_oracle("harness sanity", &cases);
}

#[test]
fn smoke_fork_outcome_plumbing_reports_clean_exit() {
    let h = Harness::new();
    let s1 = b"hello\0";
    let s2 = b"lo\0";
    let out = h.assert_same_outcome(
        "clean call in child",
        s1.as_ptr() as *const std::ffi::c_char,
        s2.as_ptr() as *const std::ffi::c_char,
    );
    assert!(out.exited && out.exit_status == 0, "child should exit cleanly: {out:?}");
    // 'h','e' are not in {l,o}; 'l' is -> 2.
    assert_eq!(out.printed, b"2\n", "strcspn(\"hello\",\"lo\") == 2");
}
