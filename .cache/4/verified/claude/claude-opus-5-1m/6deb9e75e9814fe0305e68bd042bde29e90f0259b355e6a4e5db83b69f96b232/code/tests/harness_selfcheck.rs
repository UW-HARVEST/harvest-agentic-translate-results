// Harness self-check.
//
// The differential tests compare stdout captured around FFI calls. If that
// capture were broken, every comparison would compare "nothing" to "nothing"
// and the whole suite would pass vacuously. These tests exist to prove the
// machinery actually observes output and actually discriminates.

mod common;

use common::{assert_same, capture, from_fields, libs};

/// Both `.so`s must export the symbol under test. `libs()` resolves `driver`
/// by name from each object and panics if it is absent, so simply loading is
/// the check — plus we confirm the two pointers are genuinely different code.
#[test]
fn selfcheck_both_libraries_export_driver() {
    let l = libs();
    assert_ne!(
        l.c as usize, l.rust as usize,
        "the C and Rust `driver` resolved to the same address — the same library \
         was loaded twice, so the comparison would be meaningless"
    );
}

/// The capture must return real bytes, not an empty buffer.
#[test]
fn selfcheck_capture_is_not_vacuous() {
    let l = libs();
    let out = capture(l.c, &[1.0]);
    assert_eq!(
        out, b"3ff0000000000000 0x1p+0 1.0000\n",
        "captured bytes are not what the C library prints for 1.0 — the stdout \
         capture is broken"
    );
    let out_rust = capture(l.rust, &[1.0]);
    assert_eq!(out_rust, b"3ff0000000000000 0x1p+0 1.0000\n");
}

/// The comparison must be able to *fail*. Feeding different inputs to the same
/// library has to produce different bytes; if it does not, the harness cannot
/// detect a divergence either.
#[test]
fn selfcheck_comparison_discriminates() {
    let l = libs();
    for lib in [l.c, l.rust] {
        let a = capture(lib, &[1.0]);
        let b = capture(lib, &[2.0]);
        assert_ne!(
            a, b,
            "capture returned identical bytes for different inputs — the harness \
             is blind and every differential assertion is vacuous"
        );
    }
}

/// One line of output per call, exactly.
#[test]
fn selfcheck_one_line_per_call() {
    let l = libs();
    let out = capture(l.c, &[1.0, 2.0, 3.0]);
    assert_eq!(out.iter().filter(|&&b| b == b'\n').count(), 3);
    assert_eq!(out, b"3ff0000000000000 0x1p+0 1.0000\n\
                      4000000000000000 0x1p+1 2.0000\n\
                      4008000000000000 0x1.8p+1 3.0000\n");
}

/// Golden bytes for the representative classes, pinned from the C
/// implementation (the ground truth). Asserted against BOTH libraries so a
/// silent change on either side is caught, and so the expected output of each
/// IEEE-754 class is documented in the test suite itself.
#[test]
fn selfcheck_golden_output_pinned_from_c() {
    let cases: &[(f64, &str)] = &[
        (1.0, "3ff0000000000000 0x1p+0 1.0000"),
        (0.0, "0 0x0p+0 0.0000"),
        (-0.0, "8000000000000000 -0x0p+0 -0.0000"),
        (f64::INFINITY, "7ff0000000000000 inf inf"),
        (f64::NEG_INFINITY, "fff0000000000000 -inf -inf"),
        (f64::NAN, "7ff8000000000000 nan nan"),
        (
            f64::from_bits(0xFFF8_0000_0000_0000),
            "fff8000000000000 -nan -nan",
        ),
        // Signaling NaN: the union type-pun reproduces the payload verbatim,
        // while %a / %.4f collapse to `nan`.
        (
            from_fields(false, 0x7FF, 1),
            "7ff0000000000001 nan nan",
        ),
        // Smallest positive subnormal: %a uses a `0x0.` leading digit.
        (
            f64::from_bits(1),
            "1 0x0.0000000000001p-1022 0.0000",
        ),
        (3.14159, "400921f9f01b866e 0x1.921f9f01b866ep+1 3.1416"),
    ];

    let l = libs();
    for &(input, expected) in cases {
        let want = format!("{expected}\n").into_bytes();
        let c_out = capture(l.c, &[input]);
        assert_eq!(
            c_out,
            want,
            "C output changed for bits=0x{:016x}: got {:?}",
            input.to_bits(),
            String::from_utf8_lossy(&c_out)
        );
        let rust_out = capture(l.rust, &[input]);
        assert_eq!(
            rust_out,
            want,
            "Rust output differs from the pinned C golden for bits=0x{:016x}: got {:?}",
            input.to_bits(),
            String::from_utf8_lossy(&rust_out)
        );
    }
}

/// `f64::MAX` under `%.4f` expands past any small stack buffer; make sure the
/// capture handles a long single record.
#[test]
fn selfcheck_long_record_is_captured_whole() {
    let l = libs();
    let out = capture(l.c, &[f64::MAX]);
    assert!(
        out.len() > 320,
        "expected a >320-byte record for f64::MAX, got {}",
        out.len()
    );
    assert!(out.ends_with(b".0000\n"));
    assert_same("selfcheck f64::MAX", &[f64::MAX]);
}
