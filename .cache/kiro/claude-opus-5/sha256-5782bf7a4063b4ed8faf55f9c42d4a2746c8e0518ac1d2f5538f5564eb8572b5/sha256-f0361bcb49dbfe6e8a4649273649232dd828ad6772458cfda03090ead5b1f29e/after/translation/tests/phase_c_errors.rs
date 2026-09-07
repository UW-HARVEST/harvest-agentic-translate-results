//! Phase C — error-path differential tests, GATED on `ERRORS.md`.
//!
//! One test per row of `ERRORS.md`, plus the generic C-API boundaries: null
//! pointers, zero and oversized lengths, values one step past a valid range,
//! and out-of-range "enum" values crossing the FFI boundary.
//!
//! Every function in this library returns `void`, so "same error" means: the
//! same *rejection behaviour* — byte-identical stdout (0 bytes when the NULL
//! guard rejects, N bytes when it does not) **and** the same completion status
//! (both children exit 0, i.e. neither crashes). The child asserts a successful
//! exit, so a segfault or abort on either side fails the test.

mod common;

use common::{fixture, Rng, SEED};

// ---------------------------------------------------------------------------
// Row 1 — printLine(NULL): the library's only real rejection
// ---------------------------------------------------------------------------

#[test]
fn err01_printline_null_rejects_lazy() {
    let f = fixture();
    // The NULL guard must suppress all output: exactly 0 bytes.
    f.assert_same("E1/lazy", "lazy", "pn");
    f.assert_same("E1/now", "now", "pn");
    // Repeated, and interleaved with accepted input, so the guard is exercised
    // both cold and warm.
    f.assert_same("E1/repeat", "lazy", "pn,pn,pn");
    f.assert_same("E1/mixed", "lazy", "pn,p:61,pn,p:62,pn");
}

/// The NULL rejection must produce *zero* bytes, not an empty line. Asserted
/// against the C directly so the row's expected result is pinned, not just
/// mirrored.
#[test]
fn err01_printline_null_emits_nothing() {
    let f = fixture();
    f.assert_same("E1/absolute", "lazy", "pn");
    assert_eq!(
        f.c_output("lazy", "pn"),
        Vec::<u8>::new(),
        "printLine(NULL) must write nothing (driver.c:30 NULL guard)"
    );
    assert_eq!(f.rust_output("lazy", "pn"), Vec::<u8>::new());
}

// ---------------------------------------------------------------------------
// Row 2 — bad() reaching the NULL guard via residue
// ---------------------------------------------------------------------------

#[test]
fn err02_bad_null_residue() {
    let f = fixture();

    // The decisive case: prime the slot with NULL from the immediately
    // preceding library call, so `bad()` provably reaches the NULL guard. Both
    // sides must reject and emit nothing at all.
    for mode in ["lazy", "now"] {
        f.assert_same("E2/primed-null", mode, "tn");
        assert_eq!(
            f.c_output(mode, "tn"),
            Vec::<u8>::new(),
            "printLine(NULL) leaves NULL in the slot; bad() reads it back and the \
             guard at driver.c:30 must suppress output on both calls"
        );
        assert_eq!(f.rust_output(mode, "tn"), Vec::<u8>::new());

        // Repeated, and mixed with a non-NULL priming, so the guard is
        // exercised both ways in one process.
        f.assert_same("E2/primed-null-rep", mode, "tn,tn,tn");
        f.assert_same("E2/primed-mixed", mode, "tn,tg,tn,tp:61,tn");
    }

    // Unprimed variants: the residue is loader-determined (see
    // phase_b_residue_control.rs), so only the residue lines may differ. The
    // branch *count* must still agree, which is what pins C and Rust to the
    // same NULL-vs-non-NULL decision.
    for (i, (ops, n_bad)) in [("pn,b", 1), ("b", 1), ("d:0,b", 1), ("pn,pn,b", 1), ("g,pn,b", 1)]
        .iter()
        .enumerate()
    {
        f.assert_same_except_loader_residue(&format!("E2/unprimed[{i}]"), "lazy", ops, *n_bad);
        f.assert_same_except_loader_residue(&format!("E2/unprimed[{i}]"), "now", ops, *n_bad);
    }
}

// ---------------------------------------------------------------------------
// Rows 3, 4, 6 — zero-length / NUL-first inputs (accepted, unlike NULL)
// ---------------------------------------------------------------------------

#[test]
fn err03_printline_empty_string_emits_newline() {
    let f = fixture();
    f.assert_same("E3/lazy", "lazy", "p:");
    f.assert_same("E3/now", "now", "p:");
    // Distinct from row 1: one byte, not zero.
    assert_eq!(f.c_output("lazy", "p:"), b"\n".to_vec());
    assert_eq!(f.rust_output("lazy", "p:"), b"\n".to_vec());
}

#[test]
fn err04_printline_lone_nul_byte() {
    // A 1-byte buffer holding only '\0' — same as row 3.
    let f = fixture();
    f.assert_same("E4", "lazy", "p:");
    f.assert_same("E4/now", "now", "p:");
}

#[test]
fn err06_printline_nul_first_with_trailing_bytes() {
    let f = fixture();
    // "\0abc": puts must stop at the first NUL -> a single newline.
    f.assert_same("E6", "lazy", "p:00616263");
    f.assert_same("E6/now", "now", "p:00616263");
    assert_eq!(f.c_output("lazy", "p:00616263"), b"\n".to_vec());
    assert_eq!(f.rust_output("lazy", "p:00616263"), b"\n".to_vec());
}

// ---------------------------------------------------------------------------
// Row 5 — oversized input: no length check exists
// ---------------------------------------------------------------------------

#[test]
fn err05_printline_oversized_no_truncation() {
    let f = fixture();
    for n in [4096usize, 65536, 1048576] {
        let ops = format!("pr:78:{n}");
        f.assert_same(&format!("E5[{n}]"), "lazy", &ops);
        // No rejection and no truncation: n bytes plus the newline puts adds.
        assert_eq!(f.c_output("lazy", &ops).len(), n + 1);
    }
    f.assert_same("E5/now", "now", "pr:78:1048576");
}

// ---------------------------------------------------------------------------
// Rows 7, 8, 9 — driver's int domain: no value is rejected
// ---------------------------------------------------------------------------

#[test]
fn err07_driver_zero_takes_bad_branch() {
    let f = fixture();
    f.assert_same("E7/lazy", "lazy", "d:0");
    f.assert_same("E7/now", "now", "d:0");
}

#[test]
fn err08_driver_out_of_domain_values_are_not_rejected() {
    let f = fixture();
    // One step past / far outside any 0..=1 domain. `if (useGood)` accepts all
    // of these and prints "string\n"; the Rust must not range-check.
    for v in ["-1", "2", "3", "999999", "-999999", "-2147483648", "2147483647"] {
        let ops = format!("d:{v}");
        f.assert_same(&format!("E8[{v}]"), "lazy", &ops);
        assert_eq!(
            f.c_output("lazy", &ops),
            b"string\n".to_vec(),
            "driver({v}) must take the good() branch"
        );
        assert_eq!(f.rust_output("lazy", &ops), b"string\n".to_vec());
    }
}

#[test]
fn err09_driver_out_of_range_enum_like_ints_across_ffi() {
    let f = fixture();
    // Bit patterns with no meaningful "variant", including the sign-bit-only
    // value and every single-bit pattern: a C enum/int parameter accepts any
    // int, so these are real inputs.
    let mut ops: Vec<String> = Vec::new();
    for bit in 0..32u32 {
        ops.push(format!("d:{}", (1u32 << bit) as i32));
    }
    f.assert_same("E9/bits", "lazy", &ops.join(","));
    f.assert_same("E9/bits-now", "now", &ops.join(","));

    // Randomized full-width ints, including zero by chance.
    let mut rng = Rng::new(SEED ^ 0xDEAD);
    let mut rops: Vec<String> = Vec::new();
    for _ in 0..64 {
        rops.push(format!("d:{}", rng.next_u64() as u32 as i32));
    }
    f.assert_same("E9/random", "lazy", &rops.join(","));
    f.assert_same("E9/random-now", "now", &rops.join(","));
}

// ---------------------------------------------------------------------------
// Generic boundaries beyond the table
// ---------------------------------------------------------------------------

#[test]
fn generic_null_then_every_entry_point() {
    let f = fixture();
    // A NULL-rejecting call before each other entry point, so the guard's
    // side effects on the stack are observed by all of them.
    for ops in ["pn,g", "pn,d:0", "pn,d:1", "pn,tn", "pn,tg"] {
        f.assert_same("G/null-prefix", "lazy", ops);
        f.assert_same("G/null-prefix", "now", ops);
    }
    // Same shapes with an unprimed bad(): only its residue line may differ.
    for (ops, n) in [("pn,b", 1), ("pn,pn,b,g,d:0", 1)] {
        f.assert_same_except_loader_residue("G/null-prefix-unprimed", "lazy", ops, n);
        f.assert_same_except_loader_residue("G/null-prefix-unprimed", "now", ops, n);
    }
}

#[test]
fn generic_high_bytes_and_non_utf8() {
    let f = fixture();
    // Non-ASCII / invalid-UTF-8 byte strings: `puts` is byte-oriented, so these
    // must pass through untouched. A Rust translation that assumed UTF-8 would
    // diverge or panic here.
    for ops in [
        "p:ff",
        "p:fffefdfc",
        "p:c3",            // truncated UTF-8 lead byte
        "p:c328",          // invalid 2-byte sequence
        "p:eda080",        // encoded surrogate half
        "p:f4908080",      // above U+10FFFF
        "p:80808080",      // lone continuation bytes
    ] {
        f.assert_same("G/high-bytes", "lazy", ops);
        f.assert_same("G/high-bytes", "now", ops);
    }

    // All 255 non-NUL bytes individually, and one long buffer containing every
    // one of them.
    let all: String = (1u16..=255)
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join("");
    f.assert_same("G/all-bytes", "lazy", &format!("p:{all}"));
}

#[test]
fn generic_zero_and_oversized_lengths_together() {
    let f = fixture();
    f.assert_same(
        "G/len-mix",
        "lazy",
        "p:,pr:41:1,pr:41:2,pr:41:1023,pr:41:1024,pr:41:1025,pr:41:65535,pr:41:65536,p:",
    );
}

#[test]
fn generic_no_crash_on_any_row() {
    // `assert_same` already asserts both children exited 0; this row makes the
    // "neither side crashes" requirement explicit over a long mixed program
    // that touches every entry point and every branch.
    let f = fixture();
    // Fully comparable form (bad() always primed).
    f.assert_same(
        "G/no-crash",
        "lazy",
        "pn,p:,p:61,pr:78:4096,g,tg,d:0,d:1,d:-2147483648,d:2147483647,tn,g,pn,tg",
    );
    // And with three unprimed bad() calls, where only those lines may differ.
    f.assert_same_except_loader_residue(
        "G/no-crash-unprimed",
        "lazy",
        "pn,p:,p:61,pr:78:4096,g,b,d:0,d:1,d:-2147483648,d:2147483647,b,g,pn,b",
        3,
    );
}
