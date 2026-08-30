//! Controls for the differential harness itself.
//!
//! Without these, an equality-only comparison could pass vacuously — for
//! example if stdout capture silently returned nothing, `bad()` (which is
//! expected to print nothing) would still "match".

mod common;

use std::ffi::CString;

use common::{FnPrintLine, FnVoid, Report, capture_stdout, impls, show, sym};

#[test]
fn capture_and_expected_payloads_are_correct() {
    let libs = impls();

    // Positive control: capture really observes what the library writes.
    for (tag, lib) in [("C", libs.c), ("Rust", libs.rust)] {
        let print_line: libloading::Symbol<FnPrintLine> = sym(lib, "printLine");
        let arg = CString::new("hello").unwrap();
        let out = capture_stdout(|| unsafe { print_line(arg.as_ptr()) });
        assert_eq!(
            out,
            b"hello\n",
            "{tag}: printLine(\"hello\") produced \"{}\"",
            show(&out)
        );

        let good: libloading::Symbol<FnVoid> = sym(lib, "good");
        let out = capture_stdout(|| unsafe { good() });
        assert_eq!(
            out,
            b"helperGood1 string\n",
            "{tag}: good() produced \"{}\"",
            show(&out)
        );

        // `helperBad` returns the address of an automatic array; gcc substitutes
        // a null pointer for the return value, so `printLine` sees NULL and
        // prints nothing. This is the C behaviour, defect included.
        let bad: libloading::Symbol<FnVoid> = sym(lib, "bad");
        let out = capture_stdout(|| unsafe { bad() });
        assert!(
            out.is_empty(),
            "{tag}: bad() was expected to print nothing but produced \"{}\"",
            show(&out)
        );
    }
}

#[test]
fn report_detects_a_mismatch() {
    // Negative control: the comparator must fail on differing bytes.
    let mut report = Report::new();
    report.check("synthetic", b"left\n", b"right\n");
    assert_eq!(report.checks(), 1);
    let outcome = std::panic::catch_unwind(move || report.finish());
    assert!(
        outcome.is_err(),
        "Report::finish accepted a known mismatch"
    );
}

#[test]
fn report_accepts_identical_bytes() {
    let mut report = Report::new();
    report.check("synthetic", b"same\n", b"same\n");
    report.finish();
}
