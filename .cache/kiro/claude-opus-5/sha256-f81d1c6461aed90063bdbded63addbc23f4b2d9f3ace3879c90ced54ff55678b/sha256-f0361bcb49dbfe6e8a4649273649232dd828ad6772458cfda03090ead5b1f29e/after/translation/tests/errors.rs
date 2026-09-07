//! Phase C — error / rejection-path differential tests.
//!
//! One test per row group of `ERRORS.md`. `sieve` has no failure channel
//! (`void` return, no `errno`, no sentinel, no assert), so "the same
//! error/rejection" is asserted as "the same observable byte stream **and** the
//! same termination behaviour" — the only observables this API exposes.

mod harness;

use harness::*;

fn c_src_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("c_src")
}

/// Drop `//` comment lines so the licence prose cannot create false hits.
fn code_only(s: &str) -> String {
    s.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ------------------------------------------------------------------ R1..R4
/// R1–R4: the four generic C-API rejection classes (null pointer, zero length,
/// oversized length, out-of-range enum) are *structurally absent* from this
/// library — there is no parameter of the right kind to make them
/// representable. Proved mechanically from the C header and source so the claim
/// cannot rot, rather than asserted in prose.
#[test]
fn r1_r4_structurally_absent_rejections() {
    let header = std::fs::read_to_string(c_src_dir().join("include/sieve.h")).expect("sieve.h");
    let source = std::fs::read_to_string(c_src_dir().join("src/sieve.c")).expect("sieve.c");
    let h = code_only(&header);
    let c = code_only(&source);

    // R1: no pointer parameter -> a null pointer is not a representable input.
    assert!(!h.contains('*'), "public API gained a pointer parameter:\n{h}");
    // R2/R3: no length/size/count parameter -> zero and oversized length are
    // not representable inputs.
    for kw in ["size", "len", "count", "num"] {
        assert!(
            !h.to_lowercase().contains(kw),
            "public API gained a length-like parameter ({kw}):\n{h}"
        );
    }
    // R4: no enum parameter -> there is no invalid variant to smuggle across
    // the FFI boundary. The one parameter is `int`, and *every* one of its 2^32
    // bit patterns is accepted (covered by E1–E12), so there is no rejected
    // sub-range either.
    assert!(!h.contains("enum"), "public API gained an enum parameter:\n{h}");
    assert!(
        h.contains("void sieve(int start);"),
        "unexpected public API surface:\n{h}"
    );

    // The implementation genuinely has no rejection path.
    for forbidden in ["return", "assert", "errno", "exit(", "abort("] {
        assert!(
            !c.contains(forbidden),
            "C implementation gained a `{forbidden}` — ERRORS.md needs a new row"
        );
    }
    // Exactly one conditional in the whole implementation: `val % 10 == 9`.
    assert_eq!(
        c.matches("if").count(),
        1,
        "C implementation gained another conditional — ERRORS.md/CONFIGS.md must be redone"
    );
    assert!(c.contains("val % 10 == 9"), "the sole conditional changed:\n{c}");
    // No conditional compilation, so there is only one configuration to verify.
    for src in [&h, &c] {
        assert!(
            !src.contains("#ifdef") && !src.contains("#if "),
            "C gained conditional compilation — CONFIGS.md must grow feature rows"
        );
    }
}

// ------------------------------------------------------------------ E1..E6
/// E1–E6: the reachable degenerate / boundary starts whose runs terminate.
/// Each asserts C and Rust agree byte-for-byte AND that the exact bytes derived
/// from the C source are produced, so a shared mistake is still caught.
#[test]
fn e1_e6_boundary_bytes() {
    let cases: Vec<(&str, i32, String)> = vec![
        // E1: smallest non-negative value satisfying `val % 10 == 9`.
        ("E1", 9, "9\n".to_string()),
        // E2: zero — residue 0, longest non-negative run.
        ("E2", 0, "0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n".to_string()),
        // E3: negative, crosses the sign boundary.
        ("E3", -1, "-1\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n".to_string()),
        // E4: the trap case — `-9 % 10 == -9` in C, not 9.
        ("E4", -9, (-9..=9).map(|v| format!("{v}\n")).collect()),
        // E5: further "ends in 9" negatives.
        ("E5", -19, (-19..=9).map(|v| format!("{v}\n")).collect()),
        ("E5", -99, (-99..=9).map(|v| format!("{v}\n")).collect()),
        ("E5", -109, (-109..=9).map(|v| format!("{v}\n")).collect()),
        // E6: largest int satisfying `val % 10 == 9`; terminates in one line
        // with no overflow.
        ("E6", 2_147_483_639, "2147483639\n".to_string()),
    ];

    for (row, val, expected) in cases {
        let c = c_out(val);
        let r = r_out(val);
        assert_eq!(
            c,
            expected.as_bytes(),
            "[{row}] C output for sieve({val}) is not what the C source prescribes"
        );
        assert_eq!(r, c, "[{row}] Rust diverged from C for sieve({val})");
    }

    // E6/E7 premise: 2147483639 ends in 9, its successor does not.
    assert_eq!(2_147_483_639i32 % 10, 9);
    assert_ne!(2_147_483_640i32 % 10, 9);
}

// ----------------------------------------------------------------- E7..E11
/// E7–E11: the signed-overflow region and the extreme magnitudes. These emit
/// 23–45 GB and need 2.1e9–4.3e9 `printf` calls, so both sides run in child
/// processes and a bounded stdout prefix is compared. Neither side may
/// terminate early or trap.
#[test]
fn e7_e10_overflow_region_prefix() {
    const LIMIT: usize = 256 * 1024;

    // Premise for E7–E9: from any start in [INT_MAX-7, INT_MAX] the loop cannot
    // reach a value ending in 9 without signed overflow.
    for s in 2_147_483_640i64..=2_147_483_647 {
        assert!(
            (s..=2_147_483_647).all(|v| v % 10 != 9),
            "overflow-region premise wrong for {s}"
        );
    }

    for (row, arg) in [
        ("E7", "2147483640"),
        ("E8", "2147483646"),
        ("E9", "2147483647"),
        ("E10", "-2147483648"),
        ("E11", "-2147483647"),
    ] {
        assert_same_prefix(arg, LIMIT, row);
    }

    // Confirm we observed the real run (correct first line) and that neither
    // side stopped: an implementation that silently returned would otherwise
    // "match" by both producing nothing.
    for (arg, first_line) in [
        ("2147483640", "2147483640\n"),
        ("2147483646", "2147483646\n"),
        ("2147483647", "2147483647\n"),
        ("-2147483648", "-2147483648\n"),
        ("-2147483647", "-2147483647\n"),
    ] {
        for side in [Side::C, Side::Rust] {
            let p = prefix(side, arg, 8192);
            assert!(
                String::from_utf8_lossy(&p.bytes).starts_with(first_line),
                "{side:?}: sieve({arg}) did not begin with {first_line:?}"
            );
            assert!(
                !p.terminated,
                "{side:?}: sieve({arg}) terminated inside the budget — it should still be running"
            );
        }
    }
}

/// E7–E9 detail: the `INT_MAX -> INT_MIN` wrap itself must be reproduced
/// identically. Starting at `INT_MAX` the second line must be `INT_MIN`.
#[test]
fn e7_e9_wrap_point_bytes() {
    for side in [Side::C, Side::Rust] {
        let p = prefix(side, "2147483647", 4096);
        let text = String::from_utf8_lossy(&p.bytes);
        let mut lines = text.lines();
        assert_eq!(lines.next(), Some("2147483647"), "{side:?}: first line");
        assert_eq!(
            lines.next(),
            Some("-2147483648"),
            "{side:?}: `val++` at INT_MAX must wrap to INT_MIN, as the compiled C does"
        );
        assert_eq!(lines.next(), Some("-2147483647"), "{side:?}: third line");
    }
    // And the two sides agree on that prefix.
    assert_same_prefix("2147483647", 4096, "E9-wrap");
}

// --------------------------------------------------------------------- E12
/// E12: full-width bit patterns handed across the FFI boundary as `c_int`.
/// `0x80000000` must be seen as `INT_MIN`, `0xFFFFFFFF` as `-1`, `0x7FFFFFFF`
/// as `INT_MAX` — identically on both sides, with no validation path.
#[test]
fn e12_bit_pattern_reinterpretation() {
    // 0xFFFFFFFF == -1 terminates quickly.
    let c = run(&[step_raw(Side::C, "0xffffffff")]);
    let r = run(&[step_raw(Side::Rust, "0xffffffff")]);
    assert_eq!(c, r, "[E12] 0xFFFFFFFF diverged");
    assert_eq!(c, c_out(-1), "[E12] C treated 0xFFFFFFFF differently from -1");
    assert_eq!(r, r_out(-1), "[E12] Rust treated 0xFFFFFFFF differently from -1");

    // 0x80000000 (INT_MIN) and 0x7FFFFFFF (INT_MAX) are the huge classes.
    for (hex, dec) in [("0x80000000", "-2147483648"), ("0x7fffffff", "2147483647")] {
        assert_same_prefix(hex, 64 * 1024, "E12");
        for side in [Side::C, Side::Rust] {
            let via_hex = prefix(side, hex, 4096);
            let via_dec = prefix(side, dec, 4096);
            assert_eq!(
                via_hex.bytes, via_dec.bytes,
                "[E12] {side:?} treated {hex} differently from {dec}"
            );
            assert_eq!(via_hex.terminated, via_dec.terminated);
        }
    }
}

// -------------------------------------------------- generic FFI boundaries
/// Generic boundaries required by Phase C beyond the derived table: one step
/// either side of every interesting point in the `int` domain whose run
/// terminates within budget.
#[test]
fn generic_one_step_past_boundaries() {
    let mut vals: Vec<i32> = Vec::new();
    for anchor in [
        0i64,
        9,
        10,
        -1,
        -9,
        -10,
        99,
        100,
        -99,
        -100,
        i32::MAX as i64 - 9,
        i32::MAX as i64 - 8,
    ] {
        for d in -1i64..=1 {
            let v = anchor + d;
            // Exclude the classes that emit gigabytes (E7–E11 cover those).
            if v < 2_147_483_640 && v > -100_000 {
                vals.push(v as i32);
            }
        }
    }
    vals.sort_unstable();
    vals.dedup();
    assert!(vals.len() >= 20, "boundary set unexpectedly small: {vals:?}");
    assert_same_batch(&vals, "generic-boundaries");
}

/// The `INT_MIN`-adjacent boundaries, which emit ~23 GB each and so are
/// prefix-compared rather than run to completion.
#[test]
fn generic_int_min_adjacent_boundaries() {
    for arg in [
        "-2147483648", // INT_MIN
        "-2147483647", // INT_MIN + 1
        "-2147483646",
        "-2147483640",
        "-2147483639", // ends in 9, but negative -> remainder -9, no early break
    ] {
        assert_same_prefix(arg, 32 * 1024, "generic-INT_MIN");
    }
}

// ------------------------------------------------------------ symbol parity
/// Symbol parity is itself part of the error surface: a caller doing
/// `dlsym(handle, "sieve")` must succeed against both libraries, neither side
/// may be missing a symbol, and neither may export an extra one.
#[test]
fn symbol_parity_dlsym() {
    assert_distinct_libraries();
    let p = paths();
    let c_syms = exported_symbols(&p.c_so);
    let r_syms = exported_symbols(&p.r_so);
    assert_eq!(c_syms, vec!["sieve".to_string()], "unexpected C export set");
    let missing: Vec<_> = c_syms.iter().filter(|s| !r_syms.contains(s)).collect();
    assert!(
        missing.is_empty(),
        "Rust .so is missing symbols exported by the C .so: {missing:?}"
    );
    assert_eq!(
        c_syms, r_syms,
        "export sets differ (C: {c_syms:?}, Rust: {r_syms:?})"
    );
}
