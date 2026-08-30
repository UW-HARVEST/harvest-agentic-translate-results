//! Phase B addendum — the `bad()` overflow region `data >= 12`.
//!
//! `bad()` is the CWE-121 defect: `buffer[data] = 1` with only a lower-bound
//! check. In the reference build `buffer` lives at `-0x30(%rbp)`, so:
//!
//! ```text
//!   data  0..=9   ->  buffer itself                     (in bounds)
//!   data  10      ->  -0x08(%rbp), frame padding        (benign)
//!   data  11      ->  -0x04(%rbp), the loop counter `i` (benign, `i` is re-init'd)
//!   data  12..=13 ->  +0x00(%rbp), the SAVED rbp        (caller's frame pointer)
//!   data  14..=15 ->  +0x08(%rbp), the RETURN ADDRESS   (fatal on `ret`)
//!   data  >=16    ->  the caller's frame, and upward
//! ```
//!
//! From `data >= 12` the write lands outside `bad`'s own frame, in memory owned
//! by the *caller*. Whether that is fatal is a property of the entire call
//! chain's stack layout, so it is different for the two builds — in both
//! directions. These tests therefore do not compare exit status there; they
//! assert the strongest property that is actually well defined, and that a
//! faithful translation must satisfy:
//!
//! > **Whatever `bad()` prints is byte-identical in both builds.**
//!
//! To observe that, `stdout` is made unbuffered before the call, otherwise the
//! ten lines a doomed call already wrote are lost in the block-buffered pipe.
//!
//! See `UB.md` for the measured crash maps.

mod common;

use common::{run_both_unbuffered, Op};

/// The overflow slots that clobber the saved `rbp` and the return address.
/// `bad()` still runs its loop to completion and prints ten zeros before the
/// damage takes effect, in *both* builds.
#[test]
fn ub_01_bad_saved_rbp_and_return_address_slots() {
    for data in 12..=15 {
        let (c, r) = run_both_unbuffered(&[Op::Bad(data)]);
        assert_eq!(
            c.stdout, r.stdout,
            "bad({data}): what the library printed must match byte for byte\n\
             C   : {:?}\nRust: {:?}",
            c.text(),
            r.text()
        );
        assert_eq!(
            c.lines(),
            vec![b"0" as &[u8]; 10],
            "bad({data}) must print ten zeros: the write misses `buffer` entirely"
        );
    }
}

/// A broad sweep of the overflow region. `bad()`'s own output must be identical
/// at every index; only survival may differ.
#[test]
fn ub_02_bad_overflow_sweep_output_is_identical() {
    let mut checked = 0;
    let mut c_deaths = Vec::new();
    let mut r_deaths = Vec::new();

    for data in 12..=400 {
        let (c, r) = run_both_unbuffered(&[Op::Bad(data)]);
        assert_eq!(
            c.stdout,
            r.stdout,
            "bad({data}): printed output diverged\nC   : {:?}\nRust: {:?}",
            c.text(),
            r.text()
        );
        assert_eq!(
            c.lines(),
            vec![b"0" as &[u8]; 10],
            "bad({data}): the ten dumped values must all be zero"
        );
        if c.crashed() {
            c_deaths.push(data);
        }
        if r.crashed() {
            r_deaths.push(data);
        }
        checked += 1;
    }

    assert_eq!(checked, 389);
    eprintln!("ub_02: C died at {c_deaths:?}");
    eprintln!("ub_02: Rust died at {r_deaths:?}");
    // Both builds are expected to be destroyed at *some* indices; that is the
    // vulnerability being demonstrated. What must not happen is a difference in
    // what the library printed, which is asserted above.
    assert!(
        !c_deaths.is_empty(),
        "the C reference is supposed to be corruptible here; if it never dies, \
         the frame-layout assumption in UB.md no longer holds and this test \
         needs re-deriving from a fresh disassembly of bad()"
    );
}

/// The same region reached through the composed `driver` pipeline. Here the
/// crash can truncate `driver`'s own trailing output (`Finished bad()`), so the
/// comparison is limited to the prefix both processes lived long enough to
/// emit — and that prefix must agree.
#[test]
fn ub_03_driver_overflow_prefix_is_identical() {
    for data in 12..=200 {
        let (c, r) = run_both_unbuffered(&[Op::Driver(7, data)]);
        let n = c.stdout.len().min(r.stdout.len());
        assert_eq!(
            &c.stdout[..n],
            &r.stdout[..n],
            "driver(7, {data}): common output prefix diverged\nC   : {:?}\nRust: {:?}",
            c.text(),
            r.text()
        );
        // Whichever side survived must have produced the full pipeline output.
        for (name, o) in [("C", &c), ("Rust", &r)] {
            if !o.crashed() {
                let lines = o.lines();
                // 1 banner + 10 (goodG2B) + 10 (goodB2G) + 1 + 1 + 10 (bad) + 1
                assert_eq!(
                    lines.len(),
                    34,
                    "{name} survived driver(7, {data}) but printed {} lines, not 34",
                    lines.len()
                );
                assert_eq!(lines[0], b"Calling good()...");
                assert_eq!(*lines.last().unwrap(), b"Finished bad()");
            }
        }
    }
}

/// Huge indices walk off the end of the stack into unmapped memory. Both builds
/// must die, and neither may print anything different from the other before it
/// does. (`bad()` writes *before* it prints, so nothing is printed at all here.)
#[test]
fn ub_04_bad_far_out_of_range_kills_both() {
    for data in [100_000, 1_000_000, 100_000_000, i32::MAX] {
        let (c, r) = run_both_unbuffered(&[Op::Bad(data)]);
        assert!(
            c.crashed(),
            "C survived bad({data})?! stdout={:?}",
            c.text()
        );
        assert!(
            r.crashed(),
            "Rust survived bad({data}) — the translation absorbed a write that \
             the C reference cannot: stdout={:?}",
            r.text()
        );
        assert_eq!(
            c.stdout,
            r.stdout,
            "bad({data}): output before death diverged"
        );
        assert!(
            c.stdout.is_empty(),
            "bad({data}) writes before it prints, so nothing should be emitted"
        );
    }
}
