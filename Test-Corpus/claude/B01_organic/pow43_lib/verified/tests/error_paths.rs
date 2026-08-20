//! Phase C — error / rejection-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library has *no* explicit rejection
//! path (no assert, no error code, no null/length check — see `ERRORS.md`), so
//! the rejection surface consists of its two explicit range comparisons and the
//! inputs for which its unchecked table index leaves `g_pow43[0..=144]`.
//!
//! Inputs whose C behaviour is undefined (out-of-bounds read) are exercised
//! **out of process** (`common::child`) so that a fault is observed and
//! compared instead of killing the test run.

mod common;

use std::ffi::c_int;

use common::child::{self, Outcome, Which};
use common::*;

/// Child-process worker used by `common::child::call_isolated`.
///
/// `#[ignore]`d because it is not a test: the harness re-executes this binary
/// with `POW43_CHILD_IMPL`/`POW43_CHILD_X` set to perform one `pow43` call in
/// isolation. When those variables are absent (e.g. someone runs the suite with
/// `--ignored`) it is a no-op; a child that fails to perform its call is still
/// detected by the parent, which then sees no `RESULT=` marker.
#[test]
#[ignore]
fn child_worker() {
    if !child::run_if_child() {
        println!("not spawned as a child worker: nothing to do");
    }
}

// ---------------------------------------------------------------------------
// Row 1 — no validation path exists: every int is accepted, no sentinel
// ---------------------------------------------------------------------------
#[test]
fn err01_no_validation_path_exists() {
    // The C source has no rejection statement at all, so for every accepted
    // input both implementations must return the same ordinary float value —
    // there is no error code or sentinel that could differ.
    let mut rng = Rng::new(0xE0_01);
    for _ in 0..5_000 {
        let x = rng.range_i32(DOMAIN_MIN, DOMAIN_MAX);
        let (c, r) = call_both(x);
        assert_eq!(c.to_bits(), r.to_bits(), "pow43({x}) differs");
        // no NaN/inf is ever used as an error signal by this C code
        assert!(c.is_finite(), "C returned non-finite {c:?} for x={x}");
        assert!(r.is_finite(), "Rust returned non-finite {r:?} for x={x}");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — explicit range check #1: `if (x < 129)`
// ---------------------------------------------------------------------------
#[test]
fn err02_boundary_129() {
    // taken: direct table read of the LAST table entry
    let (c128, r128) = call_both(128);
    assert_eq!(c128.to_bits(), r128.to_bits());
    assert_eq!(
        c128.to_bits(),
        0x4421_4518,
        "C pow43(128) must be g_pow43[144] = 645.079578 (got {c128:?})"
    );
    // not taken: the scaled path with mult == 16
    let (c129, r129) = call_both(129);
    assert_eq!(c129.to_bits(), r129.to_bits());
    assert_eq!(c_branch(128), Branch::A);
    assert_eq!(c_branch(129), Branch::B);
    assert_eq!(c_mult(129), 16);
    // one more step on each side, to make sure the edge is where C puts it
    for x in [126, 127, 128, 129, 130, 131] {
        assert_bit_identical(x, "err02");
    }
}

// ---------------------------------------------------------------------------
// Row 3 — explicit range check #2: `if (x < 1024)`
// ---------------------------------------------------------------------------
#[test]
fn err03_boundary_1024() {
    for x in [1022, 1023, 1024, 1025] {
        assert_bit_identical(x, "err03");
    }
    assert_eq!(c_mult(1023), 16);
    assert_eq!(c_mult(1024), 256);
    // mult is observable: an input with frac == 0 must equal the plain table
    // entry scaled by mult (16 and 256 are exact powers of two, so the
    // multiplication introduces no rounding).
    let (c_1024, r_1024) = call_both(1024);
    let (c_16, r_16) = call_both(16); // same table index (32), frac == 0
    assert_eq!(c_1024, c_16 * 256.0, "C: pow43(1024) != pow43(16) * 256");
    assert_eq!(r_1024, r_16 * 256.0, "Rust: pow43(1024) != pow43(16) * 256");
    assert_eq!(c_1024.to_bits(), r_1024.to_bits());
    assert_eq!(c_16.to_bits(), r_16.to_bits());

    let (c_136, r_136) = call_both(136); // branch B, frac == 0, index 33
    let (c_17, r_17) = call_both(17);
    assert_eq!(c_136, c_17 * 16.0, "C: pow43(136) != pow43(17) * 16");
    assert_eq!(r_136, r_17 * 16.0, "Rust: pow43(136) != pow43(17) * 16");
}

// ---------------------------------------------------------------------------
// Row 4 — lowest input that stays inside the table: x == -16
// ---------------------------------------------------------------------------
#[test]
fn err04_lowest_defined_input() {
    assert_eq!(c_table_index(-16), 0);
    let (c, r) = call_both(-16);
    assert_eq!(c.to_bits(), r.to_bits());
    assert_eq!(c.to_bits(), 0x0000_0000, "g_pow43[0] is +0.0f");
    // and the whole defined negative tail
    for x in DOMAIN_MIN..0 {
        assert_bit_identical(x, "err04");
    }
}

// ---------------------------------------------------------------------------
// Row 5 — one step below the valid range: x == -17 (index -1) is UB
// ---------------------------------------------------------------------------
#[test]
fn err05_below_table_is_ub() {
    // the boundary is exactly here:
    assert!(in_domain(-16), "x=-16 must be the lowest defined input");
    assert!(!in_domain(-17), "x=-17 must be undefined (index -1)");
    assert_eq!(c_table_index(-17), -1);

    // both libraries are really called with this input, out of process
    let c = child::call_isolated(Which::C, -17);
    let r = child::call_isolated(Which::Rust, -17);
    // Neither implementation *rejects* the input (the C has no rejection
    // path): both return a value read from whatever precedes the table.
    assert!(
        matches!(c, Outcome::Value(_)),
        "C should still return a value for x=-17, got {c:?}"
    );
    assert_eq!(
        c.kind(),
        r.kind(),
        "termination class differs for the UB input x=-17: C={c:?} Rust={r:?}"
    );
    // The value itself is a property of the compiled image, not of the C
    // source (ERRORS.md "Why rows 5, 7, 8, 9 and 16 cannot assert value
    // equality"), so it is recorded rather than asserted.
    println!("x=-17 (UB, index -1): C={c:?} Rust={r:?}");
}

// ---------------------------------------------------------------------------
// Row 6 — highest input that stays inside the table: x == 8223
// ---------------------------------------------------------------------------
#[test]
fn err06_highest_defined_input() {
    assert_eq!(c_table_index(DOMAIN_MAX), 144);
    let (c, r) = call_both(DOMAIN_MAX);
    assert_eq!(c.to_bits(), r.to_bits());
    assert_eq!(
        c.to_bits(),
        0x4822_1588,
        "C pow43(8223) changed (got {c:?})"
    );
    for x in 8180..=DOMAIN_MAX {
        assert_bit_identical(x, "err06");
    }
}

// ---------------------------------------------------------------------------
// Row 7 — one step past the valid range: x == 8224 (index 145) is UB
// ---------------------------------------------------------------------------
#[test]
fn err07_above_table_is_ub() {
    assert!(in_domain(DOMAIN_MAX), "x=8223 must be the highest defined input");
    assert!(!in_domain(8224), "x=8224 must be undefined (index 145)");
    assert_eq!(c_table_index(8224), 145);

    let c = child::call_isolated(Which::C, 8224);
    let r = child::call_isolated(Which::Rust, 8224);
    assert!(
        matches!(c, Outcome::Value(_)),
        "C should still return a value for x=8224, got {c:?}"
    );
    assert_eq!(
        c.kind(),
        r.kind(),
        "termination class differs for the UB input x=8224: C={c:?} Rust={r:?}"
    );
    println!("x=8224 (UB, index 145): C={c:?} Rust={r:?}");
}

// ---------------------------------------------------------------------------
// Row 8 — x == INT_MAX: signed overflow in `2 * x` / `x + sign`, wild index
// ---------------------------------------------------------------------------
#[test]
fn err08_int_max_is_ub() {
    for x in [i32::MAX, i32::MAX - 1] {
        // the index the C computes (with wrapping, which is what the compiled
        // code does) is far outside the table
        let idx = c_table_index(x);
        assert!(idx < 0 || idx > 144, "x={x} idx={idx} should be out of range");
        let c = child::call_isolated(Which::C, x);
        let r = child::call_isolated(Which::Rust, x);
        assert_eq!(
            c.kind(),
            r.kind(),
            "termination class differs for x={x}: C={c:?} Rust={r:?}"
        );
        assert!(
            matches!(c, Outcome::Signal(_)),
            "expected the C build to fault for x={x}, got {c:?}"
        );
        println!("x={x} (UB, index {idx}): C={c:?} Rust={r:?}");
    }
}

// ---------------------------------------------------------------------------
// Row 9 — x == INT_MIN: first branch taken, wild negative index
// ---------------------------------------------------------------------------
#[test]
fn err09_int_min_is_ub() {
    for x in [i32::MIN, i32::MIN + 1] {
        assert_eq!(c_branch(x), Branch::A);
        let idx = c_table_index(x);
        assert!(idx < 0, "x={x} idx={idx} should be far negative");
        let c = child::call_isolated(Which::C, x);
        let r = child::call_isolated(Which::Rust, x);
        assert_eq!(
            c.kind(),
            r.kind(),
            "termination class differs for x={x}: C={c:?} Rust={r:?}"
        );
        assert!(
            matches!(c, Outcome::Signal(_)),
            "expected the C build to fault for x={x}, got {c:?}"
        );
        println!("x={x} (UB, index {idx}): C={c:?} Rust={r:?}");
    }
}

// ---------------------------------------------------------------------------
// Row 10 — x == 0 returns +0.0 (not -0.0)
// ---------------------------------------------------------------------------
#[test]
fn err10_zero_input_is_positive_zero() {
    let (c, r) = call_both(0);
    assert_eq!(c.to_bits(), 0x0000_0000, "C pow43(0) must be +0.0, got {c:?}");
    assert_eq!(r.to_bits(), 0x0000_0000, "Rust pow43(0) must be +0.0, got {r:?}");
    assert!(!c.is_sign_negative() && !r.is_sign_negative());
}

// ---------------------------------------------------------------------------
// Row 11 — the division can never see a zero denominator
// ---------------------------------------------------------------------------
#[test]
fn err11_denominator_never_zero() {
    // exhaustively for the defined domain
    for x in 129..=DOMAIN_MAX {
        let (_, den) = c_frac_parts(x);
        assert!(den >= 1024, "x={x} denominator {den} < 1024");
    }
    // and for a wide random sample of the *whole* positive int range that
    // reaches the divide (x >= 129), including the wrap corner where
    // `(x & ~63) + 64` overflows to INT_MIN
    let mut rng = Rng::new(0xE0_11);
    let mut samples: Vec<c_int> = (0..20_000)
        .map(|_| rng.range_i32(129, i32::MAX))
        .collect();
    samples.extend([129, 1023, 1024, i32::MAX - 64, i32::MAX - 1, i32::MAX, 2147483584]);
    for x in samples {
        let (_, den) = c_frac_parts(x);
        assert_ne!(den, 0, "denominator became 0 for x={x}");
    }
    // consequence: no in-domain input can produce inf or NaN in either library
    for x in DOMAIN_MIN..=DOMAIN_MAX {
        let (c, r) = call_both(x);
        assert!(c.is_finite() && r.is_finite(), "non-finite at x={x}: C={c:?} Rust={r:?}");
        assert_eq!(c.to_bits(), r.to_bits(), "x={x}");
    }
}

// ---------------------------------------------------------------------------
// Row 12 — the negative-frac branch (sign == 64)
// ---------------------------------------------------------------------------
#[test]
fn err12_negative_frac_branch() {
    let mut rng = Rng::new(0xE0_12);
    // branch C: sign == 64 <=> (x & 63) >= 32
    let xs_c = sample_where(&mut rng, 1024, DOMAIN_MAX, 1_000, |x| c_sign(x) == 64);
    for x in xs_c {
        let (num, den) = c_frac_parts(x);
        assert!(num < 0 && den > 0, "x={x} num={num} den={den}");
        let (c, r) = call_both(x);
        assert_eq!(c.to_bits(), r.to_bits(), "x={x}");
        // frac < 0 => polynomial < 1 => below the aligned table value
        let idx = c_table_index(x);
        let aligned = 64 * (idx - 16); // same index, frac == 0
        assert_eq!(c_table_index(aligned), idx);
        let (ca, ra) = call_both(aligned);
        assert!(c < ca, "C: pow43({x})={c} should be < pow43({aligned})={ca}");
        assert!(r < ra, "Rust: pow43({x})={r} should be < pow43({aligned})={ra}");
    }
    // branch B: sign == 64 <=> bit 2 of x set
    let xs_b = sample_where(&mut rng, 129, 1023, 1_000, |x| c_sign(x) == 64);
    for x in xs_b {
        let (num, _) = c_frac_parts(x);
        assert!(num < 0, "x={x} num={num}");
        assert_bit_identical(x, "err12 branch B");
    }
}

// ---------------------------------------------------------------------------
// Rows 13 & 14 — the API has no pointer / length / count arguments
// ---------------------------------------------------------------------------
#[test]
fn err13_api_has_no_pointer_or_length_args() {
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("c_src/include/lib.h"),
    )
    .expect("read c_src/include/lib.h");
    let decls: Vec<&str> = header
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with('#'))
        .collect();
    assert_eq!(
        decls,
        vec!["float pow43(int x);"],
        "the public header changed; re-derive ERRORS.md / CONFIGS.md"
    );
    // mechanical proof of the N/A rows: no pointer, no second parameter (hence
    // no length/size/count), no enum type in the public API.
    let d = decls[0];
    assert!(!d.contains('*'), "no pointer parameter exists: {d}");
    assert!(!d.contains(','), "there is exactly one parameter: {d}");
    assert!(!d.contains("enum"), "no enum parameter exists: {d}");
    assert!(!d.contains("size") && !d.contains("len"), "no length parameter: {d}");
}

// ---------------------------------------------------------------------------
// Row 15 — every 32-bit pattern is a legal input (no enum-style validation)
// ---------------------------------------------------------------------------
#[test]
fn err15_full_i32_domain_sampling() {
    let mut rng = Rng::new(0xE0_15);
    let mut in_dom = 0usize;
    let mut out_dom = 0usize;
    let mut isolated = 0usize;
    for _ in 0..3_000 {
        let x = rng.next_u32() as c_int;
        if in_domain(x) {
            in_dom += 1;
            assert_bit_identical(x, "err15 in-domain random i32");
        } else {
            out_dom += 1;
            // Compare a few of them out of process as well: both must be
            // *callable* with the value (no validation exists), and neither may
            // behave as if the value had been rejected before the call.
            if isolated < 8 {
                isolated += 1;
                let c = child::call_isolated(Which::C, x);
                let r = child::call_isolated(Which::Rust, x);
                assert_eq!(
                    c.kind(),
                    r.kind(),
                    "termination class differs for UB input x={x}: C={c:?} Rust={r:?}"
                );
            }
        }
    }
    assert!(out_dom > 0 && isolated > 0, "expected out-of-domain samples");
    println!("err15: random i32 samples: {in_dom} in domain, {out_dom} undefined ({isolated} run out-of-process)");
}

// ---------------------------------------------------------------------------
// Row 16 — the disagreement region is EXACTLY the out-of-table index region
// ---------------------------------------------------------------------------
#[test]
fn err16_ub_band_is_exactly_the_out_of_table_indices() {
    // 1. the defined domain is a contiguous range, and it is [-16, 8223]
    for x in -1_000..=20_000 {
        assert_eq!(
            in_domain(x),
            (DOMAIN_MIN..=DOMAIN_MAX).contains(&x),
            "domain boundary wrong at x={x} (index {})",
            c_table_index(x)
        );
    }

    // 2. scan a band that reaches past both ends of the table but stays inside
    //    the mapped images, and check that every disagreement is outside the
    //    defined domain (i.e. UB), and that nothing inside it disagrees.
    let mut diverging: Vec<c_int> = Vec::new();
    for x in -256..=8_500 {
        let (c, r) = call_both(x);
        if c.to_bits() != r.to_bits() {
            assert!(
                !in_domain(x),
                "in-domain input x={x} diverges: C={c:?} Rust={r:?}"
            );
            diverging.push(x);
        }
    }
    for &x in &diverging {
        let idx = c_table_index(x);
        assert!(
            idx < 0 || idx > 144,
            "x={x} was counted as UB but its index {idx} is inside the table"
        );
    }
    println!(
        "err16: {} inputs in [-256, 8500] read outside g_pow43 (all UB, values are image-dependent); \
         0 divergences inside the defined domain [{DOMAIN_MIN}, {DOMAIN_MAX}]",
        diverging.len()
    );
    // the near-UB band below the table is undefined for every one of its values
    for x in -64..=-17 {
        assert!(!in_domain(x));
    }
    // ... and so is the band just above it
    for x in 8_224..=8_320 {
        assert!(!in_domain(x));
    }
}
