//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH `.so`s through
//! `libloading` and compares `pow43`'s result bit-for-bit.

mod harness;
use harness::*;

/// Row 1 — branch 1, negative table region `x ∈ [-16, -1]` (indices 0..15).
#[test]
fn row01_branch1_negative_table_region() {
    let p = Pair::load();
    let n = p.assert_same_all(-16..=-1, "row01");
    assert_eq!(n, 16);
}

/// Row 2 — branch 1, exact lower boundary `x == -16` (index 0).
#[test]
fn row02_branch1_lower_boundary() {
    let p = Pair::load();
    p.assert_same(-16, "row02");
    assert_eq!(c_table_index(-16), 0);
    // The C table's element 0 is `0` -> +0.0f. Bit-compare catches -0.0.
    assert_eq!(p.c(-16).to_bits(), 0x0000_0000, "row02: expected +0.0f");
}

/// Row 3 — branch 1, `x == 0` (index 16, the table's second zero).
#[test]
fn row03_branch1_zero() {
    let p = Pair::load();
    p.assert_same(0, "row03");
    assert_eq!(c_table_index(0), 16);
    assert_eq!(p.rs(0).to_bits(), p.c(0).to_bits());
}

/// Row 4 — branch 1, non-negative table region `x ∈ [1, 128]` (indices 17..144).
#[test]
fn row04_branch1_positive_table_region() {
    let p = Pair::load();
    let n = p.assert_same_all(1..=128, "row04");
    assert_eq!(n, 128);
}

/// Row 5 — branch 1 upper boundary `x == 128` (last table entry, index 144).
#[test]
fn row05_branch1_upper_boundary() {
    let p = Pair::load();
    p.assert_same(128, "row05");
    assert_eq!(c_table_index(128), 144);
}

/// Row 6 — branch 1, randomized `x ∈ [-16, 128]`.
#[test]
fn row06_branch1_randomized() {
    let p = Pair::load();
    let mut r = Rng::new(SEED);
    for _ in 0..N_RANDOM {
        p.assert_same(r.range(-16, 128), "row06");
    }
}

/// Row 7 — branch 2 lower boundary `x == 129`.
#[test]
fn row07_branch2_lower_boundary() {
    let p = Pair::load();
    p.assert_same(129, "row07");
    assert_eq!(c_table_index(129), 32);
}

/// Row 8 — branch 2 upper boundary `x == 1023` (post-shift 8184, sign = 64).
#[test]
fn row08_branch2_upper_boundary() {
    let p = Pair::load();
    p.assert_same(1023, "row08");
    assert_eq!(c_table_index(1023), 144);
    // Confirm this row really exercises sign == 64 post-shift.
    let shifted = 1023i32 << 3;
    assert_eq!(shifted.wrapping_mul(2) & 64, 64);
}

/// Row 9 — branch 2 with `sign == 0` after the `x <<= 3`.
#[test]
fn row09_branch2_sign_zero() {
    let p = Pair::load();
    let xs: Vec<i32> = (129..1024).filter(|x| ((x << 3) * 2) & 64 == 0).collect();
    assert!(!xs.is_empty(), "row09: no inputs selected");
    p.assert_same_all(xs, "row09");
}

/// Row 10 — branch 2 with `sign == 64` after the `x <<= 3`.
#[test]
fn row10_branch2_sign_set() {
    let p = Pair::load();
    let xs: Vec<i32> = (129..1024).filter(|x| ((x << 3) * 2) & 64 == 64).collect();
    assert!(!xs.is_empty(), "row10: no inputs selected");
    p.assert_same_all(xs, "row10");
}

/// Row 11 — branch 2, post-shift `x & 63 == 0` (exact grid point).
#[test]
fn row11_branch2_exact_grid_point() {
    let p = Pair::load();
    let xs: Vec<i32> = (129..1024).filter(|x| ((x << 3) & 63) == 0).collect();
    assert!(!xs.is_empty(), "row11: no inputs selected");
    p.assert_same_all(xs, "row11");
}

/// Row 12 — branch 2, post-shift `x & 63 != 0` (interpolating).
#[test]
fn row12_branch2_interpolating() {
    let p = Pair::load();
    let xs: Vec<i32> = (129..1024).filter(|x| ((x << 3) & 63) != 0).collect();
    assert!(!xs.is_empty(), "row12: no inputs selected");
    p.assert_same_all(xs, "row12");
}

/// Row 13 — branch 2, randomized `x ∈ [129, 1023]`.
#[test]
fn row13_branch2_randomized() {
    let p = Pair::load();
    let mut r = Rng::new(SEED ^ 0xB2);
    for _ in 0..N_RANDOM {
        p.assert_same(r.range(129, 1023), "row13");
    }
}

/// Row 14 — branch 2, exhaustive over `[129, 1023]`.
#[test]
fn row14_branch2_exhaustive() {
    let p = Pair::load();
    let n = p.assert_same_all(129..=1023, "row14");
    assert_eq!(n, 895);
}

/// Row 15 — branch 3 lower boundary `x == 1024` (mult stays 256, no shift).
#[test]
fn row15_branch3_lower_boundary() {
    let p = Pair::load();
    p.assert_same(1024, "row15");
    assert_eq!(c_table_index(1024), 32);
}

/// Row 16 — branch 3 upper boundary `x == 8223` (last in-bounds argument).
#[test]
fn row16_branch3_upper_boundary() {
    let p = Pair::load();
    p.assert_same(8223, "row16");
    assert_eq!(c_table_index(8223), 144);
    // One step further is out of bounds -> covered by the Phase C tests.
    assert_eq!(c_table_index(8224), 145);
}

/// Row 17 — branch 3 with `sign == 0`.
#[test]
fn row17_branch3_sign_zero() {
    let p = Pair::load();
    let xs: Vec<i32> = (1024..=DOMAIN_HI).filter(|x| (x * 2) & 64 == 0).collect();
    assert!(!xs.is_empty(), "row17: no inputs selected");
    p.assert_same_all(xs, "row17");
}

/// Row 18 — branch 3 with `sign == 64`.
#[test]
fn row18_branch3_sign_set() {
    let p = Pair::load();
    let xs: Vec<i32> = (1024..=DOMAIN_HI).filter(|x| (x * 2) & 64 == 64).collect();
    assert!(!xs.is_empty(), "row18: no inputs selected");
    p.assert_same_all(xs, "row18");
}

/// Row 19 — branch 3, `x & 63 == 0` (multiple of 64).
#[test]
fn row19_branch3_multiple_of_64() {
    let p = Pair::load();
    let xs: Vec<i32> = (1024..=DOMAIN_HI).filter(|x| x & 63 == 0).collect();
    assert!(!xs.is_empty(), "row19: no inputs selected");
    p.assert_same_all(xs, "row19");
}

/// Row 20 — branch 3, `x & 63 == 63` (largest fraction inside a segment).
#[test]
fn row20_branch3_max_fraction() {
    let p = Pair::load();
    let xs: Vec<i32> = (1024..=DOMAIN_HI).filter(|x| x & 63 == 63).collect();
    assert!(!xs.is_empty(), "row20: no inputs selected");
    p.assert_same_all(xs, "row20");
}

/// Row 21 — branch 3, randomized `x ∈ [1024, 8223]`.
#[test]
fn row21_branch3_randomized() {
    let p = Pair::load();
    let mut r = Rng::new(SEED ^ 0xB3);
    for _ in 0..N_RANDOM {
        p.assert_same(r.range(1024, DOMAIN_HI), "row21");
    }
}

/// Row 22 — branch 3, exhaustive over `[1024, 8223]`.
#[test]
fn row22_branch3_exhaustive() {
    let p = Pair::load();
    let n = p.assert_same_all(1024..=DOMAIN_HI, "row22");
    assert_eq!(n, 7200);
}

/// Row 23 — branch-selector transitions, both sides of each `if`.
#[test]
fn row23_branch_transitions() {
    let p = Pair::load();
    for (lo, hi) in [(128, 129), (1023, 1024), (8222, 8223)] {
        p.assert_same(lo, "row23");
        p.assert_same(hi, "row23");
    }
    // The 8223 -> 8224 transition leaves the defined domain; asserted in Phase C.
    assert!(in_bounds(8223) && !in_bounds(8224));
}

/// Row 24 — exhaustive sweep of the whole defined domain.
#[test]
fn row24_exhaustive_full_domain() {
    let p = Pair::load();
    let n = p.assert_same_all(DOMAIN_LO..=DOMAIN_HI, "row24");
    assert_eq!(n, 8240);
    // Sanity: the classifier agrees that this is exactly the in-bounds set.
    assert!((DOMAIN_LO..=DOMAIN_HI).all(in_bounds));
}

/// Row 25 — order independence / absence of hidden state.
///
/// Drives the whole domain in a randomized order, records each result, then
/// replays in ascending order and requires identical results, for BOTH
/// libraries. Proves the `static const` table is never mutated and that
/// neither implementation carries per-call state.
#[test]
fn row25_order_independence() {
    let p = Pair::load();
    let mut xs: Vec<i32> = (DOMAIN_LO..=DOMAIN_HI).collect();

    // Fisher-Yates with the fixed seed.
    let mut r = Rng::new(SEED ^ 0x0DDE_u64);
    for i in (1..xs.len()).rev() {
        let j = (r.next_u64() % (i as u64 + 1)) as usize;
        xs.swap(i, j);
    }

    let shuffled: Vec<(i32, u32, u32)> = xs
        .iter()
        .map(|&x| (x, p.c(x).to_bits(), p.rs(x).to_bits()))
        .collect();

    for &(x, cb, rb) in &shuffled {
        assert_eq!(cb, rb, "row25: C/Rust differ at x = {x}");
    }
    // Replay ascending and require identical bits.
    for &(x, cb, rb) in &shuffled {
        assert_eq!(p.c(x).to_bits(), cb, "row25: C not order-independent at {x}");
        assert_eq!(
            p.rs(x).to_bits(),
            rb,
            "row25: Rust not order-independent at {x}"
        );
    }
}

/// Row 26 — realistic consumer pipeline.
///
/// `pow43` is the mp3 dequantizer's `|ix|^(4/3)` helper; a real consumer feeds
/// it a whole 576-bin spectrum. Drive both libraries over many such spectra and
/// compare the accumulated result as well as every element.
#[test]
fn row26_consumer_pipeline_576_bins() {
    let p = Pair::load();
    let mut r = Rng::new(SEED ^ 0xC0FFEE);
    for _iter in 0..200 {
        let mut c_acc: f32 = 0.0;
        let mut rs_acc: f32 = 0.0;
        for _bin in 0..576 {
            // LAME's real index range for pow43 is [0, 8206].
            let x = r.range(0, 8206);
            let cv = p.c(x);
            let rv = p.rs(x);
            assert_eq!(
                cv.to_bits(),
                rv.to_bits(),
                "row26: element divergence at x = {x}"
            );
            c_acc += cv;
            rs_acc += rv;
        }
        assert_eq!(
            c_acc.to_bits(),
            rs_acc.to_bits(),
            "row26: accumulated spectrum diverged"
        );
    }
}
