//! Phase C — error-path differential tests, one per `ERRORS.md` row.
//!
//! `driver` returns `void` and the C source contains no error return, assert,
//! or range check (verified by grep — see `ERRORS.md`). Its entire rejection
//! surface is the trap behaviour of `div(3)` on the two degenerate inputs, which
//! x86-64 delivers as `SIGFPE`.
//!
//! Each row runs `driver` in a forked child so the fault is *observable*, and
//! asserts the C and Rust children die from the SAME signal number (8 = SIGFPE),
//! not merely that "both failed somehow". The stdout each produced before dying
//! is compared too.
//!
//! This file is deliberately kept separate from `valid_paths.rs` so the forking
//! tests get their own process.

mod common;

use common::{load, outcome, Impls};

const IMIN: i32 = i32::MIN;
const IMAX: i32 = i32::MAX;
const SIGFPE: i32 = 8;

/// Every row of the ERRORS.md table: (row id, x, y, description).
const ROWS: &[(&str, i32, i32, &str)] = &[
    ("E1", 7, 0, "y == 0, x arbitrary"),
    ("E2", 0, 0, "y == 0, x == 0 (0/0)"),
    ("E3", IMIN, 0, "y == 0, x == INT_MIN"),
    ("E4", IMAX, 0, "y == 0, x == INT_MAX"),
    ("E5", IMIN, -1, "INT_MIN / -1 signed overflow"),
];

fn imps() -> Impls {
    load()
}

/// The core Phase C assertion for one row.
fn check_row(imp: &Impls, row: &str, x: i32, y: i32, what: &str) {
    let c = outcome(imp.c, "c_err", x, y);
    let r = outcome(imp.rust, "rust_err", x, y);

    // Same termination *mode* and same specific signal / exit code.
    assert_eq!(
        c.signal, r.signal,
        "[{row}] {what}: driver({x}, {y}) terminating signal differs \
         (C = {:?}, Rust = {:?})",
        c.signal, r.signal
    );
    assert_eq!(
        c.exit_code, r.exit_code,
        "[{row}] {what}: driver({x}, {y}) exit code differs \
         (C = {:?}, Rust = {:?})",
        c.exit_code, r.exit_code
    );
    // Same bytes emitted before termination.
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{row}] {what}: driver({x}, {y}) stdout differs \
         (C = {:?}, Rust = {:?})",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );

    // Pin the expected behaviour explicitly: the C ground truth faults with
    // SIGFPE and prints nothing. Asserting the concrete signal number stops a
    // Rust translation that "fails differently" (SIGILL, SIGABRT from a panic,
    // or a clean exit after printing a wrapped value) from passing.
    assert_eq!(
        c.signal,
        Some(SIGFPE),
        "[{row}] {what}: expected the C ground truth to raise SIGFPE, got {:?} \
         / exit {:?} / stdout {:?}",
        c.signal,
        c.exit_code,
        String::from_utf8_lossy(&c.stdout)
    );
    assert_eq!(
        r.signal,
        Some(SIGFPE),
        "[{row}] {what}: Rust must also raise SIGFPE, got {:?} / exit {:?} / stdout {:?}",
        r.signal,
        r.exit_code,
        String::from_utf8_lossy(&r.stdout)
    );
    assert!(
        c.stdout.is_empty() && r.stdout.is_empty(),
        "[{row}] {what}: expected no output before the fault"
    );
}

/// Phase C: all five ERRORS.md rows.
#[test]
fn phase_c_error_surface() {
    let imp = imps();
    for &(row, x, y, what) in ROWS {
        check_row(&imp, row, x, y, what);
    }
}

/// Divide-by-zero must fault for *every* numerator, not just the sampled ones.
/// This is the "y == 0 across the whole x domain" generalisation of E1..E4.
#[test]
fn divide_by_zero_across_numerator_domain() {
    let imp = imps();
    let mut rng = common::Rng::new(common::SEED ^ 0xE12);
    // Deterministic edges plus a randomized sample (fork is costly, so a
    // modest but non-trivial number of children).
    let mut xs = vec![0, 1, -1, 2, -2, 42, -42, IMAX, IMIN, IMAX - 1, IMIN + 1];
    for _ in 0..24 {
        xs.push(rng.next_i32());
    }
    for x in xs {
        check_row(&imp, "E1/E2/E3/E4", x, 0, "y == 0");
    }
}

/// Generic FFI boundary sweep, as required even where not in the table.
///
/// * null pointers / zero length / oversized length: N/A — the ABI is
///   `void driver(int, int)` with no pointer or length parameter. Asserted
///   structurally below by resolving the symbol at that exact signature.
/// * out-of-range enum values: N/A — no enum or flag parameter exists; every
///   32-bit pattern is a valid `int`. The nearest analogue is feeding argument
///   registers hostile bit patterns, done here.
/// * one step past a documented range: `INT_MIN`/`INT_MAX` and their neighbours.
#[test]
fn generic_ffi_boundaries() {
    let imp = imps();

    // Confirm the exported signature really is the 2-int one (a mismatched
    // arity/type export would show up as a divergence, not a link error, so
    // this is worth pinning).
    let _: common::DriverFn = imp.c;
    let _: common::DriverFn = imp.rust;

    // Hostile / extreme bit patterns in both argument registers. All of these
    // are VALID ints, so both sides must succeed identically (they are checked
    // via the fast in-process path). y == 0 and INT_MIN/-1 are excluded — they
    // are the fault rows above.
    let patterns: [i32; 14] = [
        IMIN,
        IMIN + 1,
        -1,
        0,
        1,
        IMAX,
        IMAX - 1,
        0x5555_5555u32 as i32,
        0xAAAA_AAAAu32 as i32,
        0x7FFF_0000u32 as i32,
        0x0000_FFFFu32 as i32,
        0xFFFF_0000u32 as i32,
        0x0001_0000,
        -0x0001_0000,
    ];
    for &x in &patterns {
        for &y in &patterns {
            if y == 0 || (x == IMIN && y == -1) {
                continue;
            }
            common::assert_same(&imp, "boundary", x, y);
        }
    }
}

/// Guard against a translation that "handles" the faults by swallowing them:
/// a Rust impl using `checked_div`/`wrapping_div` would print a line and exit 0
/// where the C dies. Asserted explicitly for the two classic mistranslations.
#[test]
fn faults_are_not_swallowed_by_rust() {
    let imp = imps();
    for &(x, y, note) in &[
        (IMIN, -1, "wrapping_div would yield INT_MIN and print"),
        (1, 0, "checked_div would yield None and print/skip"),
    ] {
        let r = outcome(imp.rust, "rust_swallow", x, y);
        assert_eq!(
            r.exit_code, None,
            "Rust driver({x}, {y}) exited normally with {:?} — {note}; \
             the C ground truth faults instead. stdout was {:?}",
            r.exit_code,
            String::from_utf8_lossy(&r.stdout)
        );
        assert_eq!(r.signal, Some(SIGFPE), "Rust driver({x}, {y}) — {note}");
        assert!(r.stdout.is_empty(), "Rust driver({x}, {y}) printed — {note}");
    }
}
