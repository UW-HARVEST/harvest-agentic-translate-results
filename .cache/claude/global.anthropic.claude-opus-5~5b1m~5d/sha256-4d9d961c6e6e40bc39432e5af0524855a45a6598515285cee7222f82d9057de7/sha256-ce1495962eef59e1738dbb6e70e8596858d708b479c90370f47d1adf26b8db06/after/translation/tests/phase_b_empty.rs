//! Phase B — valid-path differential tests, rows 1-9 and 25 of CONFIGS.md.
//!
//! These run against the C library exactly as CMake ships it, i.e. with
//! `node_count == 0` (because `initialize_test_data` is `static` and never
//! called). This binary NEVER touches the init hook, so the state stays
//! pristine for its whole lifetime.

mod common;

use common::*;
use std::ffi::c_int;

/// Widths 1..10 for positives and 1..11 for negatives, plus boundaries.
fn width_corpus() -> Vec<c_int> {
    let mut v = vec![0, i32::MIN, i32::MAX, i32::MIN + 1, i32::MAX - 1];
    for k in 0..10u32 {
        let base = 10i64.pow(k);
        for d in [-1i64, 0, 1, 5] {
            let x = base + d;
            if (i32::MIN as i64..=i32::MAX as i64).contains(&x) {
                v.push(x as c_int);
            }
            let y = -(base + d);
            if (i32::MIN as i64..=i32::MAX as i64).contains(&y) {
                v.push(y as c_int);
            }
        }
    }
    v.sort_unstable();
    v.dedup();
    v
}

// --- row 1 ----------------------------------------------------------------

#[test]
fn cfg_row1_mode1_empty_state() {
    let p = Pair::shipped();
    let mut rng = Rng::new(0x1111_0001);
    for _ in 0..20_000 {
        let n = rng.i32_interesting();
        let d = rng.i32_interesting();
        let f = rng.i32_interesting();
        p.assert_same_eq(0o1, n, d, f, ERR_MODE1_NOT_FOUND);
    }
}

// --- row 2 ----------------------------------------------------------------

#[test]
fn cfg_row2_mode2_empty_state() {
    let p = Pair::shipped();
    let mut rng = Rng::new(0x1111_0002);
    for _ in 0..20_000 {
        let n = rng.i32_interesting();
        let d = rng.i32_interesting();
        let f = rng.i32_interesting();
        p.assert_same_eq(0o2, n, d, f, ERR_MODE2_NOT_FOUND);
    }
}

// --- row 3 ----------------------------------------------------------------

#[test]
fn cfg_row3_mode3_width1() {
    let p = Pair::shipped();
    for n in 0..10 {
        for d in 0..10 {
            for f in 0..200 {
                p.assert_same_eq(0o3, n, d, f, expect_mode3(n, d, f));
            }
        }
    }
}

// --- row 4 ----------------------------------------------------------------

#[test]
fn cfg_row4_mode3_all_widths() {
    let p = Pair::shipped();
    let corpus = width_corpus();
    println!("width corpus size = {}", corpus.len());
    let mut rng = Rng::new(0x1111_0004);
    for &n in &corpus {
        for &d in &corpus {
            let f = rng.i32_any();
            p.assert_same_eq(0o3, n, d, f, expect_mode3(n, d, f));
        }
    }
    // Every attainable decimal width must actually have been visited.
    let mut widths: Vec<usize> = corpus.iter().map(|&v| decimal_width(v)).collect();
    widths.sort_unstable();
    widths.dedup();
    assert_eq!(widths, (1..=11).collect::<Vec<_>>(), "widths covered");
}

// --- row 5 ----------------------------------------------------------------

#[test]
fn cfg_row5_mode3_flag_mask() {
    let p = Pair::shipped();
    // All 128 residues of `flags & 0177`.
    for f in 0..128 {
        p.assert_same_eq(0o3, 12345, -678, f, expect_mode3(12345, -678, f));
    }
    // High bits (and the sign bit) must be ignored by the mask.
    let mut rng = Rng::new(0x1111_0005);
    for _ in 0..20_000 {
        let f = rng.i32_any();
        let n = rng.i32_range(-1000, 1000);
        let d = rng.i32_range(-1000, 1000);
        p.assert_same_eq(0o3, n, d, f, expect_mode3(n, d, f));
        // masking property: f and (f & 0177) must give the same answer
        let a = p.assert_same(0o3, n, d, f);
        let b = p.assert_same(0o3, n, d, f & 0o177);
        assert_eq!(a, b, "flags mask 0177 not honoured for f={f}");
    }
}

// --- row 6 ----------------------------------------------------------------

#[test]
fn cfg_row6_mode3_random_sweep() {
    let p = Pair::shipped();
    let mut rng = Rng::new(0x1111_0006);
    for _ in 0..100_000 {
        let n = rng.i32_interesting();
        let d = rng.i32_interesting();
        let f = rng.i32_interesting();
        p.assert_same_eq(0o3, n, d, f, expect_mode3(n, d, f));
    }
}

// --- row 7 ----------------------------------------------------------------

#[test]
fn cfg_row7_mode4_empty_state() {
    let p = Pair::shipped();
    let mut rng = Rng::new(0x1111_0007);
    for _ in 0..20_000 {
        let n = rng.i32_interesting();
        let d = rng.i32_interesting();
        let f = rng.i32_interesting();
        p.assert_same_eq(0o4, n, d, f, ERR_MODE4_NOT_FOUND);
    }
}

// --- row 8 ----------------------------------------------------------------

#[test]
fn cfg_row8_default_branch() {
    let p = Pair::shipped();
    for m in -8..=12 {
        if (1..=4).contains(&m) {
            continue;
        }
        for _ in 0..4 {
            p.assert_same_eq(m, 1, 2, 3, ERR_UNKNOWN_MODE);
        }
    }
    let mut rng = Rng::new(0x1111_0008);
    let mut checked = 0u32;
    while checked < 50_000 {
        let m = rng.i32_interesting();
        if (1..=4).contains(&m) {
            continue;
        }
        let n = rng.i32_interesting();
        let d = rng.i32_interesting();
        let f = rng.i32_interesting();
        p.assert_same_eq(m, n, d, f, ERR_UNKNOWN_MODE);
        checked += 1;
    }
}

// --- row 9 ----------------------------------------------------------------

#[test]
fn cfg_row9_full_random_property() {
    let p = Pair::shipped();
    let mut rng = Rng::new(0x1111_0009);
    for _ in 0..200_000 {
        // Bias the mode so every switch arm is hit often, but also allow any int.
        let m = match rng.next_u64() % 6 {
            0 => 0o1,
            1 => 0o2,
            2 => 0o3,
            3 => 0o4,
            4 => rng.i32_range(-6, 10),
            _ => rng.i32_interesting(),
        };
        let n = rng.i32_interesting();
        let d = rng.i32_interesting();
        let f = rng.i32_interesting();
        p.assert_same(m, n, d, f);
    }
}

// --- row 25 ---------------------------------------------------------------

#[test]
fn cfg_row25_mode_interleaving() {
    // No cross-call state leakage: interleaved modes must be identical to the
    // same calls made in isolation.
    let p = Pair::shipped();
    let mut rng = Rng::new(0x1111_0025);
    let mut script = Vec::new();
    for _ in 0..5_000 {
        let m = rng.i32_range(-1, 6);
        let n = rng.i32_interesting();
        let d = rng.i32_interesting();
        let f = rng.i32_interesting();
        script.push((m, n, d, f));
    }
    let first: Vec<c_int> = script.iter().map(|&(m, n, d, f)| p.assert_same(m, n, d, f)).collect();
    // Re-run in reverse order; results must be unchanged.
    for (idx, &(m, n, d, f)) in script.iter().enumerate().rev() {
        let again = p.assert_same(m, n, d, f);
        assert_eq!(
            again, first[idx],
            "call {idx} ({m},{n},{d},{f}) changed on replay: {} -> {again}",
            first[idx]
        );
    }
}
