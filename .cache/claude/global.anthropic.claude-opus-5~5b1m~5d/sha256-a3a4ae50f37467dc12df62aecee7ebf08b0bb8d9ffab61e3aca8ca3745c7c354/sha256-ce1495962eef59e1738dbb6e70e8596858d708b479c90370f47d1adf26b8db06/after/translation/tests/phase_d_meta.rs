//! Phase D — symbol parity plus harness self-validation.
//!
//! The point of this file is to make the other two test files *non-vacuous*.
//! A differential suite passes trivially if it accidentally loads the same
//! library twice, or if the "outputs" it compares are just the untouched
//! inputs. These tests fail loudly in either case.

mod common;

use common::*;
use std::process::Command;

// ---------------------------------------------------------------------------
// Symbol parity
// ---------------------------------------------------------------------------

fn defined_dynamic_symbols(so: &std::path::Path) -> Vec<String> {
    let out = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(so)
        .output()
        .unwrap_or_else(|e| panic!("cannot run `nm` on {}: {e}", so.display()));
    assert!(
        out.status.success(),
        "nm failed on {}: {}",
        so.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    let mut v: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn meta_symbol_parity_c_subset_of_rust() {
    let (c, rust) = both();
    let c_syms = defined_dynamic_symbols(&c.path);
    let rust_syms = defined_dynamic_symbols(&rust.path);

    assert!(
        !c_syms.is_empty(),
        "nm reported no symbols for the C .so — the parity check would be vacuous"
    );

    let missing: Vec<&String> = c_syms.iter().filter(|s| !rust_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "symbols exported by the C .so but MISSING from the Rust .so: {missing:?}\n\
         C:    {c_syms:?}\n\
         Rust: {rust_syms:?}"
    );

    // The C exports exactly one function, so a Rust .so exporting far more
    // would mean the three `static` helpers leaked into the ABI.
    assert!(
        c_syms.iter().any(|s| s == "colourblind"),
        "expected `colourblind` in the C .so, got {c_syms:?}"
    );
    assert!(
        rust_syms.iter().any(|s| s == "colourblind"),
        "expected `colourblind` in the Rust .so, got {rust_syms:?}"
    );
    for leaked in ["Protanopia", "Deuteranopia", "Tritanopia", "protanopia", "deuteranopia", "tritanopia"] {
        assert!(
            !rust_syms.iter().any(|s| s == leaked),
            "`{leaked}` is `static` in the C but exported by the Rust .so"
        );
    }
}

#[test]
fn meta_rust_so_has_no_undefined_non_libc_symbols() {
    let (_, rust) = both();
    let out = Command::new("nm")
        .args(["-D", "-u"])
        .arg(&rust.path)
        .output()
        .expect("run nm -D -u");
    assert!(out.status.success(), "nm -D -u failed");
    let bad: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| l.split_whitespace().last().map(str::to_owned))
        .filter(|s| {
            !s.starts_with("__")
                && !s.starts_with("_ITM_")
                && !s.starts_with("_Unwind_")
                && !s.contains("@GLIBC")
                && !s.contains("@@GLIBC")
        })
        .collect();
    assert!(
        bad.is_empty(),
        "Rust .so imports non-libc symbols (untranslated dependency?): {bad:?}"
    );
}

// ---------------------------------------------------------------------------
// Harness self-validation — guards against a vacuously-passing suite.
// ---------------------------------------------------------------------------

#[test]
fn meta_two_distinct_libraries_are_loaded() {
    let (c, rust) = both();
    assert_ne!(
        c.path.canonicalize().unwrap(),
        rust.path.canonicalize().unwrap(),
        "the harness loaded the SAME file twice — every differential test would pass vacuously"
    );
    assert!(
        c.path.to_string_lossy().contains("c_src"),
        "C .so is not under c_src/: {}",
        c.path.display()
    );
    assert!(
        rust.path.to_string_lossy().contains("target"),
        "Rust .so is not under target/: {}",
        rust.path.display()
    );
    // And the two files really are different builds.
    let cb = std::fs::read(&c.path).unwrap();
    let rb = std::fs::read(&rust.path).unwrap();
    assert_ne!(cb, rb, "the two .so files have identical contents");
}

#[test]
fn meta_valid_impairments_actually_transform_data() {
    let (c, rust) = both();
    // For each valid impairment there must exist an input the library CHANGES,
    // otherwise "outputs match" would just mean "neither library did anything".
    for &imp in &VALID {
        let input = [0.9f32, 0.2, 0.4];
        let mut a = input;
        let mut b = input;
        c.call(imp, &mut a);
        rust.call(imp, &mut b);
        assert_ne!(
            bits(&a),
            bits(&input),
            "C library did not transform anything for {} — Phase B would be vacuous",
            impairment_name(imp)
        );
        assert_ne!(
            bits(&b),
            bits(&input),
            "Rust library did not transform anything for {}",
            impairment_name(imp)
        );
        assert_eq!(bits(&a), bits(&b), "C and Rust disagree for {}", impairment_name(imp));
    }
}

#[test]
fn meta_the_three_impairments_are_distinguishable() {
    // If the Rust dispatched every impairment to the same helper, Phase B would
    // still pass row-by-row only if the C did too. Prove they are distinct so
    // that the per-impairment rows really are three different code paths.
    let (c, rust) = both();
    let input = [0.9f32, 0.2, 0.4];
    let mut results = Vec::new();
    for &imp in &VALID {
        let mut a = input;
        c.call(imp, &mut a);
        let mut b = input;
        rust.call(imp, &mut b);
        assert_eq!(bits(&a), bits(&b));
        results.push(bits(&a));
    }
    assert_ne!(results[0], results[1], "protanopia == deuteranopia output");
    assert_ne!(results[1], results[2], "deuteranopia == tritanopia output");
    assert_ne!(results[0], results[2], "protanopia == tritanopia output");
}

#[test]
fn meta_comparison_is_bit_exact_not_approximate() {
    // A tolerance-based comparison would hide sign-of-zero and NaN-payload
    // divergence, the two classes this translation is most at risk of. Prove the
    // helper the suite uses rejects a one-ULP difference.
    let x = 1.0f32;
    let y = f32::from_bits(x.to_bits() + 1);
    assert_ne!(bits(&[x, 0.0, 0.0]), bits(&[y, 0.0, 0.0]));
    assert_ne!(bits(&[0.0, 0.0, 0.0]), bits(&[-0.0, 0.0, 0.0]), "signed zero");
    assert_ne!(
        bits(&[f32::from_bits(0x7FC0_0001), 0.0, 0.0]),
        bits(&[f32::from_bits(0x7FC0_0002), 0.0, 0.0]),
        "NaN payload"
    );
    assert_ne!(
        bits(&[f32::from_bits(0x7FC0_0001), 0.0, 0.0]),
        bits(&[f32::from_bits(0xFFC0_0001), 0.0, 0.0]),
        "NaN sign"
    );
}

#[test]
fn meta_out_of_range_impairment_is_observably_different_from_valid() {
    // Confirms the ERRORS.md sentinel ("nothing written") is distinguishable
    // from the valid path, so the Phase C rows are not just re-testing a no-op
    // against another no-op.
    let (c, rust) = both();
    let input = [0.9f32, 0.2, 0.4];
    for lib in [&c, &rust] {
        let mut valid = input;
        lib.call(CB_PROTANOPIA, &mut valid);
        let mut invalid = input;
        lib.call(3, &mut invalid);
        assert_ne!(
            bits(&valid),
            bits(&invalid),
            "{}: out-of-range impairment produced the same result as cbProtanopia",
            lib.which
        );
        assert_eq!(
            bits(&invalid),
            bits(&input),
            "{}: out-of-range impairment did not leave the buffer untouched",
            lib.which
        );
    }
}

// ---------------------------------------------------------------------------
// A tiny known-answer table, captured from the C .so, so a future regression in
// BOTH libraries at once (e.g. someone "fixing" a coefficient in the C) would
// still be caught here rather than silently agreed upon.
// ---------------------------------------------------------------------------

#[test]
fn meta_known_answer_vectors() {
    let (c, rust) = both();
    // (impairment, input bits, expected output bits) — recorded from the C .so
    // built by c_src/CMakeLists.txt.
    const VECTORS: &[(u32, [u32; 3], [u32; 3])] = &[
        // 0.5, 0.25, 0.125 — an ordinary in-gamut colour.
        (CB_PROTANOPIA, [0x3F000000, 0x3E800000, 0x3E000000], [0x3E95D4D0, 0x3E95D4D0, 0x3DFDAFEE]),
        (CB_DEUTERANOPIA, [0x3F000000, 0x3E800000, 0x3E000000], [0x3EAA5312, 0x3EAA5312, 0x3DF1BCF0]),
        (CB_TRITANOPIA, [0x3F000000, 0x3E800000, 0x3E000000], [0x3F0413A7, 0x3E6FDC42, 0x3E6FDC42]),
        // Basis vectors: each output is exactly one coefficient, so these pin
        // the f32 rounding of the literals themselves.
        (CB_PROTANOPIA, [0x3F800000, 0x00000000, 0x00000000], [0x3E2EA67E, 0x3E2EA67E, 0xBB94048D]),
        (CB_DEUTERANOPIA, [0x00000000, 0x3F800000, 0x00000000], [0x3F2B59DD, 0x3F2B59DD, 0x3CE430F9]),
        (CB_TRITANOPIA, [0x00000000, 0x00000000, 0x3F800000], [0xBE0274D9, 0x3E011DEC, 0x3E011DEC]),
        // All-negative-zero: red keeps the sign, green and blue do NOT
        // (`(-0) + (-0)` is `-0`, but `(-0) - (-0)` and `(-0) + (+0)` are `+0`).
        (CB_PROTANOPIA, [0x80000000, 0x80000000, 0x80000000], [0x80000000, 0x00000000, 0x00000000]),
        // INF + (-INF): x86 substitutes the *negative* default QNaN, 0xFFC00000.
        (CB_TRITANOPIA, [0x7F800000, 0xFF800000, 0x00000000], [0xFFC00000, 0xFF800000, 0xFFC00000]),
        // An incoming qNaN payload must survive unmodified through all three.
        (CB_DEUTERANOPIA, [0x7FC00001, 0x3F800000, 0x40000000], [0x7FC00001, 0x7FC00001, 0x7FC00001]),
    ];
    for &(imp, inb, expected) in VECTORS {
        let input = [
            f32::from_bits(inb[0]),
            f32::from_bits(inb[1]),
            f32::from_bits(inb[2]),
        ];
        let mut a = input;
        c.call(imp, &mut a);
        let mut b = input;
        rust.call(imp, &mut b);
        assert_eq!(
            bits(&a),
            expected,
            "C drifted from the recorded vector for {} on {inb:08x?}",
            impairment_name(imp)
        );
        assert_eq!(
            bits(&b),
            expected,
            "Rust drifted from the recorded vector for {} on {inb:08x?}",
            impairment_name(imp)
        );
    }
}
