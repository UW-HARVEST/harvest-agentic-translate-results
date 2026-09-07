//! Residue-attribution control for the direct-`bad()` rows.
//!
//! `bad()` reads an uninitialized stack slot (CWE-457). In this harness the
//! residue that lands in that slot on a *direct* `bad()` call is a pointer into
//! the shared-library mmap region left behind by `dlopen`, so its value — and
//! the bytes it points at — are a function of the loaded object's own load
//! footprint (dependency list, `.dynsym` size, segment sizes), not of any
//! translated logic.
//!
//! These controls establish that mechanically, inside the *same* apparatus that
//! reports the C-vs-Rust divergence, so the divergence can be attributed rather
//! than guessed at. Each control compares two **C** builds that are
//! behaviourally identical by construction:
//!
//!   * `plain.so`  — `c_src/src/driver.c`, compiled as-is;
//!   * `fat.so`    — the same source plus 2000 unused exported functions and an
//!                   extra `DT_NEEDED` (`-lm`): identical behaviour, different
//!                   dynamic footprint.
//!
//! If `plain.so` vs `fat.so` diverges on exactly the rows where C vs Rust
//! diverges, then those rows measure the loader's residue, not the translation.
//!
//! The controls are skipped unless `DIFF_CTL_A`/`DIFF_CTL_B` are set (they are
//! built and set by `scripts/residue_control.sh`), so a plain `cargo test` does
//! not depend on an external toolchain.

mod common;

use common::{fixture_pair, Fixture};
use std::path::Path;

fn ctl_fixture(a: &str, b: &str) -> Fixture {
    fixture_pair(Path::new(a), Path::new(b))
}

fn controls() -> Option<(String, String)> {
    Some((
        std::env::var("DIFF_CTL_A").ok()?,
        std::env::var("DIFF_CTL_B").ok()?,
    ))
}

/// Rows that pass for C vs Rust: `bad()` reached only through `driver`, where
/// the residue is produced by the library's own frames and the PLT resolver.
/// These must also match between the two C controls.
const LIBRARY_RESIDUE_ROWS: &[&str] = &[
    "d:0", "d:1", "d:0,d:0", "d:1,d:0", "d:0,d:1", "d:1,d:1,d:0,d:0", "g", "g,g,g", "p:61", "pn",
];

/// Rows that fail for C vs Rust: `bad()` called directly, so the slot holds
/// `dlopen` residue.
const LOADER_RESIDUE_ROWS: &[&str] = &["b", "b,b", "g,b", "pn,b", "p:666f6f,b", "g,b,g,b,g,b"];

#[test]
fn control_library_residue_rows_match_between_c_builds() {
    let Some((a, b)) = controls() else {
        eprintln!("skipped: DIFF_CTL_A/DIFF_CTL_B not set");
        return;
    };
    let f = ctl_fixture(&a, &b);
    for ops in LIBRARY_RESIDUE_ROWS {
        for mode in ["lazy", "now"] {
            f.assert_same(&format!("CTL-lib[{ops}]"), mode, ops);
        }
    }
}

/// This test asserts the *control itself* diverges: two behaviourally identical
/// C builds disagree on the direct-`bad()` rows. That is the evidence that
/// those rows are not attributable to the Rust translation.
#[test]
fn control_loader_residue_rows_differ_between_two_c_builds() {
    let Some((a, b)) = controls() else {
        eprintln!("skipped: DIFF_CTL_A/DIFF_CTL_B not set");
        return;
    };
    let f = ctl_fixture(&a, &b);
    let mut diverged = Vec::new();
    for ops in LOADER_RESIDUE_ROWS {
        let x = f.c_output("now", ops);
        let y = f.rust_output("now", ops);
        if x != y {
            diverged.push(*ops);
        }
        println!("CTL-loader[{ops}] plain={x:02x?} fat={y:02x?}");
    }
    assert!(
        !diverged.is_empty(),
        "control produced no divergence; the attribution argument for the \
         direct-bad() rows would then be unsupported and they must be \
         investigated as translation defects instead"
    );
    println!(
        "{} of {} direct-bad() rows diverge between two behaviourally identical \
         C builds: {diverged:?}",
        diverged.len(),
        LOADER_RESIDUE_ROWS.len()
    );
}

// ---------------------------------------------------------------------------
// Slot-fidelity: footprint-independent verification of bad()'s stack slot
// ---------------------------------------------------------------------------

/// `bad()` must read back exactly the pointer the immediately preceding library
/// call left in the shared slot. Because the residue here is written by library
/// code (not by `dlopen`), this is independent of the object's load footprint
/// and *is* attributable to the translation: it pins `bad()`'s slot offset,
/// `printLine`'s parameter spill, and `good()`'s store all to the C's layout.
#[test]
fn slot_fidelity_bad_reads_previous_calls_slot() {
    let f = common::fixture();
    for mode in ["lazy", "now"] {
        // good(); bad();  -> "string" twice
        f.assert_same("SLOT/tg", mode, "tg");
        assert_eq!(
            f.c_output(mode, "tg"),
            b"string\nstring\n".to_vec(),
            "good() then bad() must print \"string\" twice: bad() reads the very \
             slot good() stored into"
        );
        assert_eq!(f.rust_output(mode, "tg"), b"string\nstring\n".to_vec());

        // printLine(s); bad();  -> s twice
        for (hex, want) in [
            ("666f6f", "foo\nfoo\n"),
            ("61", "a\na\n"),
            ("", "\n\n"),
            ("7a7a7a7a7a7a7a7a7a7a", "zzzzzzzzzz\nzzzzzzzzzz\n"),
        ] {
            let ops = format!("tp:{hex}");
            f.assert_same(&format!("SLOT/tp[{hex}]"), mode, &ops);
            assert_eq!(
                f.c_output(mode, &ops),
                want.as_bytes().to_vec(),
                "printLine then bad must echo the same string twice"
            );
            assert_eq!(f.rust_output(mode, &ops), want.as_bytes().to_vec());
        }

        // printLine(NULL); bad();  -> the NULL spill is what bad() reads back,
        // so the NULL guard rejects twice: zero bytes total.
        f.assert_same("SLOT/tn", mode, "tn");
        assert_eq!(
            f.c_output(mode, "tn"),
            Vec::<u8>::new(),
            "printLine(NULL) spills NULL into the slot; bad() reads it back and \
             the guard suppresses both lines"
        );
        assert_eq!(f.rust_output(mode, "tn"), Vec::<u8>::new());
    }
}
