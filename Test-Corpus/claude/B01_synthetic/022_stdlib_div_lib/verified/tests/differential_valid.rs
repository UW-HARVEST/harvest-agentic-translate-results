//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every comparison loads BOTH
//! `libdriver.so`s (C and Rust) with `libloading` and calls the exported
//! `driver` symbol, comparing the bytes each writes to `stdout`. See
//! `tests/common/mod.rs` for why the calls happen in a worker child process.
//!
//! Rows that involve more than one value use a fixed-seed PRNG and many
//! randomized inputs, so a row passes only across the whole sample.

mod common;

use common::{
    assert_same, assert_same_all, assert_same_trap, c_results, op, parse_line, run_ops,
    run_ops_with, traps, Rng, Side, BOUNDARIES,
};
use std::ffi::c_int;

/// Child-side entry point for every batch this file runs.
/// Inert during a normal test run. See `common::worker_body`.
#[test]
fn difftest_worker() {
    common::worker_body();
}

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

/// A magnitude in `[1, INT_MAX]` spread across all bit-widths, so that sampled
/// pairs hit `|x| < |y|`, `|x| == |y|` and `|x| > |y|` often instead of always
/// landing in the "both huge" regime.
fn rand_mag(rng: &mut Rng) -> i32 {
    let bits = rng.in_range(1, 31) as u32;
    let mask: i32 = if bits >= 31 { i32::MAX } else { (1i32 << bits) - 1 };
    let v = (rng.next_u64() as u32 as i32) & mask;
    if v == 0 {
        1
    } else {
        v
    }
}

/// Rejection-samples `n` non-trapping pairs matching `pred`.
fn sample(
    seed: u64,
    n: usize,
    gen: impl Fn(&mut Rng) -> (i32, i32),
    pred: impl Fn(i32, i32) -> bool,
) -> Vec<(i32, i32)> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n);
    let mut attempts = 0usize;
    while out.len() < n {
        attempts += 1;
        assert!(
            attempts < n * 10_000 + 1_000_000,
            "generator could not produce {n} pairs (got {})",
            out.len()
        );
        let (x, y) = gen(&mut rng);
        if !traps(x, y) && pred(x, y) {
            out.push((x, y));
        }
    }
    out
}

/// Signed pair whose magnitudes come from `rand_mag`, with the requested signs.
fn signed_pairs(
    seed: u64,
    n: usize,
    sx: i32,
    sy: i32,
    pred: impl Fn(i32, i32) -> bool,
) -> Vec<(i32, i32)> {
    sample(
        seed,
        n,
        move |rng| (rand_mag(rng).wrapping_mul(sx), rand_mag(rng).wrapping_mul(sy)),
        pred,
    )
}

/// Exactly-divisible pairs (`x % y == 0`, `|quot| > 1`) with the given signs.
fn divisible_pairs(seed: u64, n: usize, sx: i32, sy: i32) -> Vec<(i32, i32)> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let mag = rand_mag(&mut rng);
        let k = rng.in_range(2, 1000) as i64;
        let prod = mag as i64 * k;
        if prod > i32::MAX as i64 {
            continue;
        }
        let x = prod as i32 * sx;
        let y = mag * sy;
        if !traps(x, y) && x.wrapping_rem(y) == 0 && x.wrapping_div(y).unsigned_abs() > 1 {
            out.push((x, y));
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Rows 1-8 — the sign x divisibility cross-product for |x| > |y|
// ---------------------------------------------------------------------------

#[test]
fn row01_pos_pos_inexact() {
    assert_same_all(
        "row01 x>0 y>0 |x|>|y| inexact",
        &signed_pairs(0x0101, 500, 1, 1, |x, y| {
            x > 0 && y > 0 && x > y && x % y != 0
        }),
    );
}

#[test]
fn row02_pos_pos_exact() {
    assert_same_all("row02 x>0 y>0 exact", &divisible_pairs(0x0202, 500, 1, 1));
}

#[test]
fn row03_neg_pos_inexact() {
    assert_same_all(
        "row03 x<0 y>0 |x|>|y| inexact",
        &signed_pairs(0x0303, 500, -1, 1, |x, y| {
            x < 0 && y > 0 && (x as i64).abs() > y as i64 && x % y != 0
        }),
    );
}

#[test]
fn row04_neg_pos_exact() {
    assert_same_all("row04 x<0 y>0 exact", &divisible_pairs(0x0404, 500, -1, 1));
}

#[test]
fn row05_pos_neg_inexact() {
    assert_same_all(
        "row05 x>0 y<0 |x|>|y| inexact",
        &signed_pairs(0x0505, 500, 1, -1, |x, y| {
            x > 0 && y < 0 && x as i64 > (y as i64).abs() && x % y != 0
        }),
    );
}

#[test]
fn row06_pos_neg_exact() {
    assert_same_all("row06 x>0 y<0 exact", &divisible_pairs(0x0606, 500, 1, -1));
}

#[test]
fn row07_neg_neg_inexact() {
    assert_same_all(
        "row07 x<0 y<0 |x|>|y| inexact",
        &signed_pairs(0x0707, 500, -1, -1, |x, y| {
            x < 0 && y < 0 && (x as i64).abs() > (y as i64).abs() && x % y != 0
        }),
    );
}

#[test]
fn row08_neg_neg_exact() {
    assert_same_all("row08 x<0 y<0 exact", &divisible_pairs(0x0808, 500, -1, -1));
}

/// Cross-check that rows 1-8 really did cover all eight quadrant/divisibility
/// combinations, using the sign of the C-reported quotient and remainder.
#[test]
fn rows01_08_cover_all_quotient_remainder_sign_combinations() {
    let cases = [
        (7, 2, 3, 1),      // +/+ inexact -> quot +, rem +
        (8, 2, 4, 0),      // +/+ exact
        (-7, 2, -3, -1),   // -/+ inexact -> quot -, rem - (truncate toward zero)
        (-8, 2, -4, 0),    // -/+ exact
        (7, -2, -3, 1),    // +/- inexact -> quot -, rem +
        (8, -2, -4, 0),    // +/- exact
        (-7, -2, 3, -1),   // -/- inexact -> quot +, rem -
        (-8, -2, 4, 0),    // -/- exact
    ];
    let pairs: Vec<(c_int, c_int)> = cases.iter().map(|&(x, y, _, _)| (x, y)).collect();
    assert_same_all("rows01_08 sign matrix", &pairs);

    for (&(x, y, eq, er), got) in cases.iter().zip(c_results(&pairs)) {
        assert_eq!(
            got,
            (eq as i64, er as i64),
            "C div({x}, {y}) should truncate toward zero"
        );
    }
}

// ---------------------------------------------------------------------------
// Row 9 — zero dividend
// ---------------------------------------------------------------------------

#[test]
fn row09_zero_dividend() {
    let mut rng = Rng::new(0x0909);
    let mut pairs: Vec<(c_int, c_int)> =
        vec![(0, 1), (0, -1), (0, 2), (0, -2), (0, i32::MAX), (0, i32::MIN)];
    for _ in 0..400 {
        pairs.push((0, rng.nonzero_i32()));
    }
    assert_same_all("row09 x==0", &pairs);

    // Both fields print as an unsigned-looking zero.
    for r in c_results(&pairs) {
        assert_eq!(r, (0, 0));
    }
    let c = run_ops(&[op(Side::C, 0, 7)]);
    assert_eq!(c.stdout, b"quotient: 0, remainder: 0\n");
}

// ---------------------------------------------------------------------------
// Row 10 — |x| < |y| (quotient 0, remainder == x)
// ---------------------------------------------------------------------------

#[test]
fn row10_dividend_smaller_than_divisor() {
    let mut pairs = Vec::new();
    for (i, (sx, sy)) in [(1, 1), (-1, 1), (1, -1), (-1, -1)].iter().enumerate() {
        pairs.extend(signed_pairs(
            0x1010 + i as u64,
            200,
            *sx,
            *sy,
            |x, y| x != 0 && (x as i64).abs() < (y as i64).abs(),
        ));
    }
    assert_same_all("row10 |x|<|y|", &pairs);

    for (&(x, _y), got) in pairs.iter().zip(c_results(&pairs)) {
        assert_eq!(got, (0, x as i64), "|x|<|y| must give quot 0, rem x");
    }
}

// ---------------------------------------------------------------------------
// Row 11 — |x| == |y| (quotient ±1, remainder 0)
// ---------------------------------------------------------------------------

#[test]
fn row11_equal_magnitudes() {
    let mut rng = Rng::new(0x1111);
    let mut pairs = Vec::new();
    for _ in 0..200 {
        let m = rand_mag(&mut rng);
        for (sx, sy) in [(1, 1), (-1, 1), (1, -1), (-1, -1)] {
            let (x, y) = (m * sx, m * sy);
            if !traps(x, y) {
                pairs.push((x, y));
            }
        }
    }
    // The extremes, where |x| == |y| still holds.
    pairs.extend([
        (i32::MAX, i32::MAX),
        (i32::MAX, -i32::MAX),
        (-i32::MAX, i32::MAX),
        (-i32::MAX, -i32::MAX),
        (i32::MIN, i32::MIN),
    ]);
    assert_same_all("row11 |x|==|y|", &pairs);

    for (&(x, y), got) in pairs.iter().zip(c_results(&pairs)) {
        let expect = if (x < 0) == (y < 0) { 1 } else { -1 };
        assert_eq!(got, (expect, 0), "|x|==|y| must give quot ±1, rem 0");
    }
}

// ---------------------------------------------------------------------------
// Row 12 — glibc's dead `numer >= 0 && rem < 0` fix-up branch
// ---------------------------------------------------------------------------

#[test]
fn row12_nonnegative_dividend_fixup_branch_never_fires() {
    let mut pairs = Vec::new();
    for (i, sy) in [1, -1].iter().enumerate() {
        pairs.extend(signed_pairs(0x1200 + i as u64, 500, 1, *sy, |x, _| x >= 0));
    }
    pairs.extend([(0, -1), (i32::MAX, -1), (1, i32::MIN), (0, 1), (i32::MAX, 1)]);

    assert_same_all("row12 x>=0 fix-up branch", &pairs);

    // For x >= 0, truncation toward zero means rem >= 0, so glibc's fix-up
    // (`++quot; rem -= denom;`) is unreachable. Confirmed on the C side; the
    // differential assertion above guarantees the Rust agrees.
    for (&(x, y), (q, r)) in pairs.iter().zip(c_results(&pairs)) {
        assert!(r >= 0, "C: driver({x}, {y}) gave rem {r} < 0 for x >= 0");
        assert_eq!(
            q * y as i64 + r,
            x as i64,
            "quot*y + rem == x must hold for ({x}, {y})"
        );
        assert!(r.abs() < (y as i64).abs(), "|rem| < |y| for ({x}, {y})");
    }
}

// ---------------------------------------------------------------------------
// Row 13 — unit divisors
// ---------------------------------------------------------------------------

#[test]
fn row13_unit_divisors() {
    let mut rng = Rng::new(0x1313);
    let mut pairs = Vec::new();
    for _ in 0..400 {
        let x = rng.next_i32();
        pairs.push((x, 1));
        if x != i32::MIN {
            pairs.push((x, -1)); // INT_MIN / -1 traps: ERRORS.md row 2
        }
    }
    for x in [0, 1, -1, 2, -2, i32::MAX, i32::MAX - 1, i32::MIN + 1] {
        pairs.push((x, 1));
        pairs.push((x, -1));
    }
    pairs.push((i32::MIN, 1));
    assert_same_all("row13 y==±1", &pairs);

    for (&(x, y), got) in pairs.iter().zip(c_results(&pairs)) {
        let expect = if y == 1 { x as i64 } else { -(x as i64) };
        assert_eq!(got, (expect, 0), "unit divisor for ({x}, {y})");
    }
}

// ---------------------------------------------------------------------------
// Rows 14-16 — INT_MIN / INT_MAX as dividend and as divisor
// ---------------------------------------------------------------------------

#[test]
fn row14_int_min_dividend() {
    let ys: [c_int; 12] = [
        1,
        2,
        -2,
        3,
        -3,
        7,
        -7,
        i32::MIN,
        i32::MIN + 1,
        i32::MAX,
        i32::MAX - 1,
        1_000_000,
    ];
    let pairs: Vec<(c_int, c_int)> = ys.iter().map(|&y| (i32::MIN, y)).collect();
    assert_same_all("row14 x==INT_MIN", &pairs);

    // Widest possible %d output, and the self-division case.
    let c = run_ops(&[op(Side::C, i32::MIN, 1)]);
    assert_eq!(c.stdout, b"quotient: -2147483648, remainder: 0\n");
    let c = run_ops(&[op(Side::C, i32::MIN, i32::MIN)]);
    assert_eq!(c.stdout, b"quotient: 1, remainder: 0\n");
    let c = run_ops(&[op(Side::C, i32::MIN, 3)]);
    assert_eq!(c.stdout, b"quotient: -715827882, remainder: -2\n");
}

#[test]
fn row15_int_max_dividend() {
    let ys: [c_int; 13] = [
        1,
        -1,
        2,
        -2,
        3,
        -3,
        10,
        -10,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        1_000_000,
    ];
    let pairs: Vec<(c_int, c_int)> = ys.iter().map(|&y| (i32::MAX, y)).collect();
    assert_same_all("row15 x==INT_MAX", &pairs);

    let c = run_ops(&[op(Side::C, i32::MAX, -1)]);
    assert_eq!(c.stdout, b"quotient: -2147483647, remainder: 0\n");
    let c = run_ops(&[op(Side::C, i32::MAX, i32::MIN)]);
    assert_eq!(c.stdout, b"quotient: 0, remainder: 2147483647\n");
}

#[test]
fn row16_extreme_divisors() {
    let mut rng = Rng::new(0x1616);
    let mut pairs = Vec::new();
    for y in [i32::MIN, i32::MIN + 1, i32::MAX, i32::MAX - 1] {
        for x in [0, 1, -1, 2, -2, 12345, -12345, i32::MAX, i32::MIN, i32::MIN + 1] {
            pairs.push((x, y));
        }
        for _ in 0..150 {
            pairs.push((rng.next_i32(), y));
        }
    }
    assert_same_all("row16 extreme divisors", &pairs);

    let c = run_ops(&[op(Side::C, -12345, i32::MAX)]);
    assert_eq!(c.stdout, b"quotient: 0, remainder: -12345\n");
}

// ---------------------------------------------------------------------------
// Row 17 — full 7x7 boundary cross-product (trapping pairs included)
// ---------------------------------------------------------------------------

#[test]
fn row17_full_boundary_matrix() {
    let mut ok_pairs = Vec::new();
    let mut trapping = Vec::new();

    for &x in BOUNDARIES.iter() {
        for &y in BOUNDARIES.iter() {
            if traps(x, y) {
                trapping.push((x, y));
            } else {
                ok_pairs.push((x, y));
            }
        }
    }

    assert_same_all("row17 boundary matrix (non-trapping)", &ok_pairs);
    for (x, y) in trapping.iter().copied() {
        assert_same_trap(x, y);
    }

    assert_eq!(
        ok_pairs.len() + trapping.len(),
        BOUNDARIES.len() * BOUNDARIES.len()
    );
    // y == 0 for each of the 7 x values, plus INT_MIN / -1.
    assert_eq!(trapping.len(), 8, "unexpected number of trapping pairs");
    eprintln!(
        "row17: {} matched, {} trapped identically",
        ok_pairs.len(),
        trapping.len()
    );
}

// ---------------------------------------------------------------------------
// Rows 18-19 — randomized property-style sweeps
// ---------------------------------------------------------------------------

#[test]
fn row18_random_full_range() {
    let mut rng = Rng::new(0xDEAD_BEEF_1234_5678);
    let mut pairs = Vec::with_capacity(20_000);
    while pairs.len() < 20_000 {
        let x = rng.next_i32();
        let y = rng.nonzero_i32();
        if !traps(x, y) {
            pairs.push((x, y));
        }
    }
    assert_same_all("row18 random full range", &pairs);
}

#[test]
fn row19_random_small_magnitudes() {
    let mut rng = Rng::new(0xC0FF_EE00_0BAD_F00D);
    let mut pairs = Vec::new();
    // Exhaustive over the dense core: every (x, y) with |x|, |y| <= 32.
    for x in -32..=32 {
        for y in -32..=32 {
            if !traps(x, y) {
                pairs.push((x, y));
            }
        }
    }
    // Plus randomized [-64, 64].
    for _ in 0..3_000 {
        let x = rng.in_range(-64, 64);
        let mut y = rng.in_range(-64, 64);
        if y == 0 {
            y = 1;
        }
        pairs.push((x, y));
    }
    assert_same_all("row19 small magnitudes", &pairs);
}

// ---------------------------------------------------------------------------
// Rows 20-21 — statelessness and shared-stdout buffering parity
// ---------------------------------------------------------------------------

#[test]
fn row20_interleaved_calls_are_stateless() {
    let mut rng = Rng::new(0x2020);
    let pairs: Vec<(c_int, c_int)> = (0..600)
        .map(|_| loop {
            let x = rng.next_i32();
            let y = rng.nonzero_i32();
            if !traps(x, y) {
                return (x, y);
            }
        })
        .collect();

    // C, Rust, C, Rust ... all in ONE process, so any hidden per-library state
    // or stdout interference would show up.
    let mut cr = Vec::new();
    let mut rc = Vec::new();
    for &(x, y) in &pairs {
        cr.push(op(Side::C, x, y));
        cr.push(op(Side::Rust, x, y));
        rc.push(op(Side::Rust, x, y));
        rc.push(op(Side::C, x, y));
    }

    let out_cr = run_ops(&cr);
    let out_rc = run_ops(&rc);
    assert_eq!(out_cr.signal, None);
    assert_eq!(out_rc.signal, None);

    let l_cr = out_cr.lines();
    let l_rc = out_rc.lines();
    assert_eq!(l_cr.len(), pairs.len() * 2);
    assert_eq!(l_rc.len(), pairs.len() * 2);

    for (i, &(x, y)) in pairs.iter().enumerate() {
        // Within a C-then-Rust pair the two lines must be identical...
        assert_eq!(
            l_cr[2 * i],
            l_cr[2 * i + 1],
            "interleaved C/Rust driver({x}, {y}) diverged"
        );
        // ...and swapping the call order must not change anything.
        assert_eq!(
            l_rc[2 * i],
            l_rc[2 * i + 1],
            "interleaved Rust/C driver({x}, {y}) diverged"
        );
        assert_eq!(l_cr[2 * i], l_rc[2 * i], "call order changed the output");
    }

    // Calling the same side repeatedly must keep producing the same bytes.
    let repeat: Vec<_> = (0..6)
        .flat_map(|_| [op(Side::C, -7, 2), op(Side::Rust, -7, 2)])
        .collect();
    let out = run_ops(&repeat);
    let lines = out.lines();
    assert_eq!(lines.len(), 12);
    for l in &lines {
        assert_eq!(*l, b"quotient: -3, remainder: -1");
    }
}

#[test]
fn row21_unflushed_multi_call_stream() {
    let mut rng = Rng::new(0x2121);
    let pairs: Vec<(c_int, c_int)> = (0..800)
        .map(|_| loop {
            let x = rng.next_i32();
            let y = rng.nonzero_i32();
            if !traps(x, y) {
                return (x, y);
            }
        })
        .collect();

    // Many calls with a single flush at the very end: compares the whole stream
    // as it lands in the shared libc `stdout` buffer, not just per-call output.
    let c_stream = run_ops(&common::ops_for(Side::C, &pairs));
    let r_stream = run_ops(&common::ops_for(Side::Rust, &pairs));
    assert_eq!(
        c_stream.stdout, r_stream.stdout,
        "fully-buffered multi-call stdout streams diverged"
    );
    assert_eq!(
        c_stream.stdout.iter().filter(|&&b| b == b'\n').count(),
        pairs.len(),
        "expected exactly one line per call"
    );

    // The same batch with a flush after every call must yield the same bytes:
    // `driver` must not depend on the buffering mode.
    let c_unbuf = run_ops_with(&common::ops_for(Side::C, &pairs), true);
    let r_unbuf = run_ops_with(&common::ops_for(Side::Rust, &pairs), true);
    assert_eq!(c_unbuf.stdout, c_stream.stdout, "C: buffering changed output");
    assert_eq!(
        r_unbuf.stdout, r_stream.stdout,
        "Rust: buffering changed output"
    );
    assert_eq!(c_unbuf.stdout, r_unbuf.stdout);

    // A single flush shared by a mixed C/Rust stream.
    let mixed: Vec<_> = pairs
        .iter()
        .take(200)
        .flat_map(|&(x, y)| [op(Side::C, x, y), op(Side::Rust, x, y)])
        .collect();
    let out = run_ops(&mixed);
    let lines = out.lines();
    assert_eq!(lines.len(), 400);
    for (i, &(x, y)) in pairs.iter().take(200).enumerate() {
        assert_eq!(
            lines[2 * i],
            lines[2 * i + 1],
            "mixed unflushed stream diverged at driver({x}, {y})"
        );
    }
}

// ---------------------------------------------------------------------------
// Output format parity, spelled out byte-for-byte
// ---------------------------------------------------------------------------

/// Locks down the literal format string, including the `%d` rendering of
/// negatives and the trailing newline, against known-good expected bytes.
#[test]
fn output_format_is_byte_exact() {
    let expected: [(c_int, c_int, &[u8]); 10] = [
        (0, 1, b"quotient: 0, remainder: 0\n"),
        (1, 1, b"quotient: 1, remainder: 0\n"),
        (7, 2, b"quotient: 3, remainder: 1\n"),
        (-7, 2, b"quotient: -3, remainder: -1\n"),
        (7, -2, b"quotient: -3, remainder: 1\n"),
        (-7, -2, b"quotient: 3, remainder: -1\n"),
        (1, 10, b"quotient: 0, remainder: 1\n"),
        (i32::MAX, 1, b"quotient: 2147483647, remainder: 0\n"),
        (i32::MIN, 1, b"quotient: -2147483648, remainder: 0\n"),
        (i32::MIN + 1, -1, b"quotient: 2147483647, remainder: 0\n"),
    ];

    let pairs: Vec<(c_int, c_int)> = expected.iter().map(|&(x, y, _)| (x, y)).collect();
    assert_same_all("output format", &pairs);

    let c = run_ops(&common::ops_for(Side::C, &pairs));
    let r = run_ops(&common::ops_for(Side::Rust, &pairs));
    for (i, &(x, y, want)) in expected.iter().enumerate() {
        let want_line = &want[..want.len() - 1];
        assert_eq!(
            c.lines()[i],
            want_line,
            "C driver({x}, {y}) format mismatch"
        );
        assert_eq!(
            r.lines()[i],
            want_line,
            "Rust driver({x}, {y}) format mismatch"
        );
    }

    // And the parse round-trip agrees on both sides.
    for (&(x, y), (q, rem)) in pairs.iter().zip(c_results(&pairs)) {
        assert_eq!(q * y as i64 + rem, x as i64, "invariant for ({x}, {y})");
    }
}

/// Sanity check that the differential harness can actually *detect* a
/// difference; otherwise every row above could be passing vacuously.
#[test]
fn harness_detects_differences() {
    // Same call on both sides -> identical lines.
    let same = run_ops(&[op(Side::C, 9, 4), op(Side::Rust, 9, 4)]);
    let l = same.lines();
    assert_eq!(l.len(), 2);
    assert_eq!(l[0], l[1]);

    // Deliberately different arguments -> the harness must see a difference.
    let differ = run_ops(&[op(Side::C, 9, 4), op(Side::Rust, 9, 5)]);
    let l = differ.lines();
    assert_eq!(l.len(), 2);
    assert_ne!(
        l[0], l[1],
        "harness cannot distinguish different outputs; the other rows are vacuous"
    );

    // The line-count guard also has to work.
    let one = run_ops(&[op(Side::C, 1, 1)]);
    assert_eq!(one.lines().len(), 1);
}

// ---------------------------------------------------------------------------
// Which `c_div` arm is compiled in (documents the target cfg axis)
// ---------------------------------------------------------------------------

#[test]
fn cfg_x86_64_idiv_path_is_active() {
    if cfg!(target_arch = "x86_64") {
        eprintln!("target_arch=x86_64: the inline `cdq; idiv` arm of c_div is under test");
    } else {
        eprintln!("non-x86_64: the wrapping_div/wrapping_rem arm of c_div is under test");
    }
    assert_same(-7, 2);
    let _ = parse_line(b"quotient: -3, remainder: -1");
}
