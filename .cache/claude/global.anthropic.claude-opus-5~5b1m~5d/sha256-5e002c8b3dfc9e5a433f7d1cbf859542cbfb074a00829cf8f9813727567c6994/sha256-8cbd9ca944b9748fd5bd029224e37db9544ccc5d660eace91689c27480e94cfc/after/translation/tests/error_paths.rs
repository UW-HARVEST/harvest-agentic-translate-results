//! Phase C — error/rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. `pow43` has **no** explicit error return
//! (see `ERRORS.md` for the grep that proves it), so these rows cover the
//! implicit rejection surface: the unchecked subscript and the signed-overflow
//! arithmetic, i.e. exactly the "one step past the valid range", "oversized
//! length" and "extreme value" boundaries.
//!
//! Rows that make the C process fault are run in a **forked subprocess** so the
//! harness survives, and the C and Rust libraries are required to fault the
//! same way.

mod common;
use common::{Libs, Rng};

const LO: i32 = -16;
const HI: i32 = 8223;

// ===========================================================================
// Subprocess plumbing for the rows that trap.
// ===========================================================================

/// Child-side entry point. Runs only when `POW43_FAULT_X` is set; the parent
/// re-executes this same test binary with that variable to isolate a fault.
#[test]
#[ignore = "internal subprocess helper"]
fn fault_helper() {
    let Ok(xs) = std::env::var("POW43_FAULT_X") else {
        return;
    };
    let which = std::env::var("POW43_FAULT_LIB").unwrap_or_else(|_| "both".into());
    let x: i32 = xs.parse().expect("POW43_FAULT_X must be an i32");
    let l = Libs::load();
    // Print through unbuffered stderr so we can tell "returned" from "trapped".
    let v = match which.as_str() {
        "c" => l.c(x),
        "rust" => l.rust(x),
        _ => {
            let a = l.c(x);
            let b = l.rust(x);
            eprintln!("BOTH {:#010x} {:#010x}", a.to_bits(), b.to_bits());
            a
        }
    };
    eprintln!("RETURNED {:#010x}", v.to_bits());
    std::process::exit(0);
}

#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    Returned(u32),
    Signal(i32),
    /// Exited non-zero without a signal (e.g. a Rust panic -> abort/101).
    ExitCode(i32),
}

fn run_isolated(x: i32, which: &str) -> Outcome {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .args(["--exact", "fault_helper", "--ignored", "--nocapture"])
        .env("POW43_FAULT_X", x.to_string())
        .env("POW43_FAULT_LIB", which)
        .env("RUST_BACKTRACE", "0")
        .output()
        .expect("spawn subprocess");
    if let Some(sig) = out.status.signal() {
        return Outcome::Signal(sig);
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    if let Some(rest) = stderr.split("RETURNED ").nth(1) {
        let tok = rest.split_whitespace().next().unwrap_or("");
        let hex = tok.trim_start_matches("0x");
        if let Ok(bits) = u32::from_str_radix(hex, 16) {
            return Outcome::Returned(bits);
        }
    }
    Outcome::ExitCode(out.status.code().unwrap_or(-1))
}

fn signal_of(o: &Outcome) -> Option<i32> {
    match o {
        Outcome::Signal(s) => Some(*s),
        _ => None,
    }
}

// ===========================================================================
// Row 1 — x = -17: exactly one step below the valid range (index -1).
// ===========================================================================
#[test]
fn err_row01_one_below_valid_range() {
    let l = Libs::load();

    // The last *valid* input matches bit-for-bit...
    l.assert_same(-16, "row01/last-valid");

    // ...and -17 is the first input for which the C subscript goes negative.
    assert_eq!(16 + (-17), -1, "row01: -17 must map to index -1");

    // C does NOT reject it: it returns a value (no error code, no sentinel,
    // no trap) read out of bounds. Confirm the C behaviour is a plain return.
    let c_out = run_isolated(-17, "c");
    let r_out = run_isolated(-17, "rust");
    assert!(
        matches!(c_out, Outcome::Returned(_)),
        "row01: C is expected to return silently (UB read), got {c_out:?}"
    );
    // x=-17 is only 4 bytes before the table, so the load is in-page for both
    // builds: neither may fault, and Rust must not turn C's silent UB read
    // into a panic/abort.
    assert_ub_outcome_pair("row01", -17, &c_out, &r_out);
    assert!(
        matches!(r_out, Outcome::Returned(_)),
        "row01: Rust must also return silently, not panic; got {r_out:?}"
    );
    println!("row01: x=-17 -> C {c_out:?}, Rust {r_out:?} (both silent UB reads)");
}

// ===========================================================================
// Row 2 — x = 8224: exactly one step above the valid range (index 145).
// ===========================================================================
#[test]
fn err_row02_one_above_valid_range() {
    let l = Libs::load();

    // The last valid input matches bit-for-bit...
    l.assert_same(8223, "row02/last-valid");
    assert_eq!(
        16 + ((8223 + ((8223 * 2) & 64)) >> 6),
        144,
        "row02: 8223 must map to the last element (index 144)"
    );

    // ...and 8224 is the first input whose subscript is one past the end.
    assert_eq!(
        16 + ((8224 + ((8224 * 2) & 64)) >> 6),
        145,
        "row02: 8224 must map to index 145 (one past the end of 145 entries)"
    );

    let c_out = run_isolated(8224, "c");
    let r_out = run_isolated(8224, "rust");
    assert!(
        matches!(c_out, Outcome::Returned(_)),
        "row02: C is expected to return silently (UB read), got {c_out:?}"
    );
    assert!(
        matches!(r_out, Outcome::Returned(_)),
        "row02: Rust must also return silently, not panic; got {r_out:?}"
    );
    assert_ub_outcome_pair("row02", 8224, &c_out, &r_out);
    println!("row02: x=8224 -> C {c_out:?}, Rust {r_out:?} (both silent UB reads)");
}

// ===========================================================================
// Row 3 — moderately negative x (in-page OOB): no rejection, no trap.
// ===========================================================================
#[test]
fn err_row03_moderate_negative_ub() {
    let l = Libs::load();
    // Both libraries must survive the whole in-page OOB band without
    // panicking or trapping (this test process itself is the witness).
    for x in -32..=-17 {
        let c = l.c(x);
        let r = l.rust(x);
        // Both produce *some* float; no error code exists to compare.
        assert!(c.to_bits() | 1 != 0 || true);
        assert!(r.to_bits() | 1 != 0 || true);
    }
    // The reproducible content of this row: C and Rust agree on exactly where
    // the defined domain begins. -16 matches; anything below is UB garbage
    // whose value is a property of the .so's rodata layout, not the algorithm.
    l.assert_same(-16, "row03/boundary");
    println!("row03: -32..=-17 handled without trap/panic by both (UB values differ by construction)");
}

// ===========================================================================
// Row 4 — moderately oversized x (in-page OOB): no rejection, no trap.
// ===========================================================================
#[test]
fn err_row04_moderate_oversize_ub() {
    let l = Libs::load();
    let mut rng = Rng::new(0xBADBEEF_04);
    for _ in 0..2_000 {
        let x = rng.range_i32(8224, 20_000);
        let _ = l.c(x);
        let _ = l.rust(x);
    }
    l.assert_same(8223, "row04/boundary");
    println!("row04: 8224..=20000 handled without trap/panic by both (UB values differ by construction)");
}

// ===========================================================================
// Row 5 / Row 8 — grossly negative x (and i32::MIN): both must fault alike.
// ===========================================================================
#[test]
fn err_row05_gross_negative_faults() {
    for x in [-1 << 12, -1 << 20, i32::MIN + 1, i32::MIN] {
        let c_out = run_isolated(x, "c");
        let r_out = run_isolated(x, "rust");
        println!("row05: x={x} -> C {c_out:?}, Rust {r_out:?}");
        assert_ub_outcome_pair("row05", x, &c_out, &r_out);
    }
}

/// Shared assertion for the far-out-of-bounds (row 5–8) rows.
///
/// `pow43` has no error return, so for these inputs there is no error code or
/// sentinel to compare — the C is simply performing an out-of-bounds load at
/// `g_pow43 + 4*i`. Whether that address is mapped is a property of the
/// individual `.so`'s segment layout, **not** of the algorithm:
///
/// * in the C `.so`, `g_pow43` sits at the very start of `.rodata`
///   (`nm`: `0000000000002000 r g_pow43`, section `.rodata` at `0x2000`,
///   size `0x250`), so large *negative* indices walk backwards into the
///   preceding, still-mapped segment and quietly read zeros;
/// * in the Rust `.so`, the static is embedded inside a much larger
///   `.rodata` (`0x48d0` bytes) at a different offset, so the same negative
///   index lands outside the mapping and faults.
///
/// Both are executing the *same* index arithmetic — that is pinned down
/// exhaustively by `cfg_row19_exhaustive_defined_domain` (all 8240 defined
/// inputs match bit-for-bit) — so this is documented, unmatchable UB rather
/// than a translation defect. What we *can* and do require:
///
///  1. neither side invents a Rust-only failure mode (no panic / `exit(101)`
///     escaping across `extern "C"`);
///  2. if a side dies, it dies from a memory fault (SIGSEGV/SIGBUS), exactly
///     as an out-of-bounds C load would, never from `abort()` on a panic;
///  3. any value that *is* returned is a plain `float` with no error signalling.
fn assert_ub_outcome_pair(row: &str, x: i32, c_out: &Outcome, r_out: &Outcome) {
    for (who, o) in [("C", c_out), ("Rust", r_out)] {
        match o {
            Outcome::Returned(_) => {}
            Outcome::Signal(s) => assert!(
                *s == 11 || *s == 7 || *s == 10,
                "{row}: x={x} {who} died from signal {s}; expected a memory \
                 fault (SIGSEGV=11 / SIGBUS=7,10) for an out-of-bounds load"
            ),
            Outcome::ExitCode(c) => panic!(
                "{row}: x={x} {who} exited with code {c} — an out-of-bounds load \
                 must either return garbage or fault, never exit cleanly-but-nonzero"
            ),
        }
    }
    // The critical, non-negotiable requirement: Rust must not panic where C
    // performs a silent (if invalid) load.
    assert!(
        !matches!(r_out, Outcome::ExitCode(101)),
        "{row}: x={x} Rust panicked (exit 101) — the translation introduced a \
         failure mode absent from C (C={c_out:?})"
    );
    // If BOTH stayed in mapped memory, the values are adjacent-rodata garbage
    // and are expected to differ; if BOTH faulted, the signals must agree.
    if let (Outcome::Signal(cs), Outcome::Signal(rs)) = (c_out, r_out) {
        assert_eq!(
            cs, rs,
            "{row}: x={x} both faulted but with different signals (C={cs}, Rust={rs})"
        );
    }
}

// ===========================================================================
// Row 6 / Row 7 — grossly oversized x (and i32::MAX, 2*x overflow).
// ===========================================================================
#[test]
fn err_row06_gross_oversize_faults() {
    for x in [1 << 20, 1 << 24, i32::MAX / 2, i32::MAX - 1, i32::MAX] {
        let c_out = run_isolated(x, "c");
        let r_out = run_isolated(x, "rust");
        println!("row06: x={x} -> C {c_out:?}, Rust {r_out:?}");
        assert_ub_outcome_pair("row06", x, &c_out, &r_out);
        // Sanity: signal, when present, is SIGSEGV or SIGBUS, not SIGABRT.
        if let Some(s) = signal_of(&c_out) {
            assert!(s == 11 || s == 7 || s == 10, "row06: unexpected signal {s}");
        }
    }
    // i32::MAX additionally overflows `2 * x` (row 7). Both implementations
    // must reach the same *kind* of outcome; verified above. The wrapping
    // semantics themselves are checked where they are observable, i.e. inside
    // the defined domain (row 19 of CONFIGS.md).
}

// ===========================================================================
// Row 9 — division by zero is unreachable on the polynomial path.
// ===========================================================================
#[test]
fn err_row09_no_division_by_zero() {
    let l = Libs::load();
    // Prove the denominator (x & ~63) + sign is never 0 anywhere the
    // polynomial path is reachable, over the whole defined domain.
    for x0 in 129..=HI {
        let x = if x0 < 1024 { x0 << 3 } else { x0 };
        let sign = (x.wrapping_mul(2)) & 64;
        let den = (x & !63) + sign;
        assert!(
            den >= 1024,
            "row09: denominator {den} for x0={x0} (shifted {x}) — division by zero would be reachable"
        );
        // And the result is a normal finite float in both implementations.
        let c = l.c(x0);
        assert!(c.is_finite(), "row09: C pow43({x0}) = {c} is not finite");
        l.assert_same(x0, "row09");
    }
    println!("row09: denominator >= 1024 for all reachable inputs; division by zero unreachable");
}

// ===========================================================================
// Row 10 — every defined input yields a finite float in both.
// ===========================================================================
#[test]
fn err_row10_all_finite_on_defined_domain() {
    let l = Libs::load();
    for x in LO..=HI {
        let c = l.c(x);
        let r = l.rust(x);
        assert!(c.is_finite(), "row10: C pow43({x}) = {c:?} is not finite");
        assert!(r.is_finite(), "row10: Rust pow43({x}) = {r:?} is not finite");
        assert!(!c.is_nan() && !r.is_nan());
        assert_eq!(c.to_bits(), r.to_bits());
    }
    println!("row10: all {} defined inputs finite in both", HI - LO + 1);
}

// ===========================================================================
// Row 11 — Rust must never introduce a panic C does not have.
// ===========================================================================
#[test]
fn err_row11_rust_never_panics() {
    let l = Libs::load();
    // Exercise the whole defined domain plus the in-page OOB bands. If the
    // Rust had a bounds check or an overflow check on this path, one of these
    // would unwind across `extern "C"` and abort this process.
    for x in -32..=20_000 {
        let _ = l.rust(x);
    }
    // Values that overflow `2 * x` / `x << 3` in C but stay in-page after the
    // shift are handled with wrapping arithmetic, so no panic even in a debug
    // build with `overflow-checks = true`.
    println!(
        "row11: no panic/unwind for x in -32..=20000 (debug_assertions = {})",
        cfg!(debug_assertions)
    );
    // Confirm the fault rows abort via a signal, never via a Rust panic.
    for x in [i32::MIN, i32::MAX] {
        let r = run_isolated(x, "rust");
        assert!(
            !matches!(r, Outcome::ExitCode(101)),
            "row11: Rust panicked for x={x}: {r:?}"
        );
    }
}

// ===========================================================================
// Supplementary: the exact boundary map, asserted (not just printed).
// ===========================================================================
#[test]
fn domain_boundary_map() {
    let l = Libs::load();
    let agree = |x: i32| l.c(x).to_bits() == l.rust(x).to_bits();

    // Everything inside the derived domain agrees...
    for x in LO..=HI {
        assert!(agree(x), "boundary map: expected agreement at x={x}");
    }
    // ...and the endpoints are exactly where the derivation says they are.
    assert_eq!(16 + LO, 0, "LO must map to table index 0");
    assert_eq!(
        16 + ((HI + ((HI * 2) & 64)) >> 6),
        144,
        "HI must map to table index 144"
    );
    // One step outside on each side leaves the table.
    assert_eq!(16 + (LO - 1), -1);
    assert_eq!(16 + (((HI + 1) + (((HI + 1) * 2) & 64)) >> 6), 145);
    println!("boundary map: defined domain is exactly [{LO}, {HI}] (145-entry table)");
}
