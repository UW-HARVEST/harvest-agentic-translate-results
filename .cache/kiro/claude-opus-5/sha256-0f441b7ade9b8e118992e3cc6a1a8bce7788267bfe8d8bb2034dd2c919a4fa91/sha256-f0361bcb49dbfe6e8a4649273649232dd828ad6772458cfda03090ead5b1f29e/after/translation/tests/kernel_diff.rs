//! Phase B — valid-path differential tests for the lowest-level entry points:
//! the exported `array` object and `perform_expensive_operations()`, driven
//! directly and under composition.  Covers `CONFIGS.md` rows 1-19 and 34.
//!
//! Every row loads BOTH `.so`s through `libloading`, writes the same input into
//! each library's own exported `array`, runs the same number of
//! `perform_expensive_operations()` calls, and compares all 262144 output
//! elements byte-for-byte.

mod common;

use common::{assert_arrays_eq, load_pair, Impl, Rng, ARRAY_SIZE};
use std::ffi::c_int;

/// Run one row: write `input` into both libraries, call
/// `perform_expensive_operations()` `k` times in each, compare the arrays.
fn diff_row(ctx: &str, c: &Impl, r: &Impl, input: &[c_int], k: usize) {
    c.set_array(input);
    r.set_array(input);
    c.peo_times(k);
    r.peo_times(k);
    assert_arrays_eq(&format!("{ctx} (k={k}, n={})", k * 100), c.array(), r.array());
    // The XOR reduction `long_exec` performs must also agree.
    assert_eq!(c.xor(), r.xor(), "{ctx}: XOR reduction differs");
}

fn filled(v: c_int) -> Vec<c_int> {
    vec![v; ARRAY_SIZE]
}

// --------------------------------------------------------------------------
// Row 1 -- the exported `array` object itself.
// --------------------------------------------------------------------------

#[test]
fn row01_exported_array_object_is_fully_addressable_in_both() {
    let p = load_pair();
    let (c, r) = p.split();
    let mut rng = Rng::new(0x1111_2222_3333_4444);
    let mut input = vec![0i32; ARRAY_SIZE];
    rng.fill_full_range(&mut input);
    // Pin the boundary slots explicitly.
    input[0] = i32::MIN;
    input[1] = i32::MAX;
    input[ARRAY_SIZE - 2] = -1;
    input[ARRAY_SIZE - 1] = 0x7EED_BEEF;

    c.set_array(&input);
    r.set_array(&input);
    assert_arrays_eq("row01 read-back", c.array(), r.array());
    assert_eq!(c.array(), &input[..], "row01: C array read-back differs");
    assert_eq!(r.array(), &input[..], "row01: Rust array read-back differs");
    // Distinct objects: writing one must not disturb the other.
    c.array_mut()[12345] = 42;
    assert_eq!(r.array()[12345], input[12345]);
}

// --------------------------------------------------------------------------
// Rows 2-9 -- degenerate / boundary input shapes, single call (n = 100).
// --------------------------------------------------------------------------

#[test]
fn rows02_09_degenerate_and_boundary_shapes() {
    let p = load_pair();
    let (c, r) = p.split();

    // Row 2: pristine .bss shape (all zeros).
    diff_row("row02 all-zero", c, r, &filled(0), 1);
    // Row 3: all INT_MIN.
    diff_row("row03 all-INT_MIN", c, r, &filled(i32::MIN), 1);
    // Row 4: all INT_MAX.
    diff_row("row04 all-INT_MAX", c, r, &filled(i32::MAX), 1);
    // Row 5: all -1.
    diff_row("row05 all-minus-one", c, r, &filled(-1), 1);

    // Row 6: descending negatives -1, -2, ... (truncating `/`, sign-of-dividend `%`).
    let neg: Vec<c_int> = (0..ARRAY_SIZE).map(|i| -1 - (i as c_int)).collect();
    diff_row("row06 descending-negative", c, r, &neg, 1);

    // Row 7: ascending small positives.
    let pos: Vec<c_int> = (0..ARRAY_SIZE).map(|i| i as c_int).collect();
    diff_row("row07 ascending-positive", c, r, &pos, 1);

    // Row 8: multiples of 7 (and hence x % 7 == 0) plus 7-neighbourhoods.
    let sevens: Vec<c_int> = (0..ARRAY_SIZE)
        .map(|i| {
            let base = (i as c_int / 3).wrapping_mul(7);
            match i % 3 {
                0 => base,
                1 => base.wrapping_add(1),
                _ => base.wrapping_neg(),
            }
        })
        .collect();
    diff_row("row08 multiples-of-7", c, r, &sevens, 1);

    // Row 9: exhaustive boundary bands around INT_MIN, INT_MAX and 0.
    let mut band: Vec<c_int> = Vec::with_capacity(ARRAY_SIZE);
    let k = 40_000i64;
    for d in 0..k {
        band.push((i32::MIN as i64 + d) as c_int);
    }
    for d in 0..k {
        band.push((i32::MAX as i64 - d) as c_int);
    }
    for d in -(k as i64)..(k as i64) {
        band.push(d as c_int);
    }
    while band.len() < ARRAY_SIZE {
        band.push(0);
    }
    band.truncate(ARRAY_SIZE);
    diff_row("row09 boundary-bands", c, r, &band, 1);
}

// --------------------------------------------------------------------------
// Row 10 -- randomized full-range arrays, several independent batches.
// --------------------------------------------------------------------------

#[test]
fn row10_randomized_full_range_batches() {
    let p = load_pair();
    let (c, r) = p.split();
    let mut input = vec![0i32; ARRAY_SIZE];
    for batch in 0..8u64 {
        let mut rng = Rng::new(0xDEAD_BEEF_0000_0000 ^ batch);
        rng.fill_full_range(&mut input);
        diff_row(&format!("row10 random-full batch {batch}"), c, r, &input, 1);
    }
}

// --------------------------------------------------------------------------
// Row 11 -- the `rand()`-shaped sub-domain (bit 31 always clear).
// --------------------------------------------------------------------------

#[test]
fn row11_rand_like_domain() {
    let p = load_pair();
    let (c, r) = p.split();
    let mut input = vec![0i32; ARRAY_SIZE];
    for batch in 0..4u64 {
        let mut rng = Rng::new(0x5EED_0000_0000_0001 ^ batch);
        rng.fill_rand_like(&mut input);
        for &v in input.iter().take(64) {
            assert!(v >= 0, "rand-like domain must be non-negative");
        }
        diff_row(&format!("row11 rand-like batch {batch}"), c, r, &input, 1);
    }
}

// --------------------------------------------------------------------------
// Rows 12-18 -- composition: k calls in a row, sweeping n across the Rust
// translation's strategy threshold (LEARN_MIN_N = 8192).
// --------------------------------------------------------------------------

#[test]
fn rows13_18_composition_sweep() {
    let p = load_pair();
    let (c, r) = p.split();
    let mut rng = Rng::new(0xC0FF_EE00_1234_5678);
    let mut input = vec![0i32; ARRAY_SIZE];
    rng.fill_full_range(&mut input);

    // Row 13: n = 200, Row 14: n = 700, Row 15: n = 5000,
    // Row 16: n = 8100 (last below 8192), Row 17: n = 8200 (first at/above).
    for (row, k) in [(13usize, 2usize), (14, 7), (15, 50), (16, 81), (17, 82)] {
        diff_row(&format!("row{row} composition"), c, r, &input, k);
    }

    // Row 18: n = 20000, deep in the accelerated regime.
    diff_row("row18 composition", c, r, &input, 200);
}

// --------------------------------------------------------------------------
// Row 12 -- values that are already on a kernel cycle.  Discovered from the C
// `.so` itself: after 8200 kernel applications an orbit has overwhelmingly
// entered its cycle, so the resulting values are cycle members.
// --------------------------------------------------------------------------

#[test]
fn row12_cycle_member_inputs() {
    let p = load_pair();
    let (c, r) = p.split();
    let mut rng = Rng::new(0xABCD_0123_4567_89EF);
    let mut input = vec![0i32; ARRAY_SIZE];
    rng.fill_full_range(&mut input);

    // Drive the C library 82 times (n = 8200) to land on cycles.
    c.set_array(&input);
    c.peo_times(82);
    let deep: Vec<c_int> = c.array().to_vec();

    // Sanity: `deep` should contain very few distinct values if orbits coalesce.
    diff_row("row12 cycle-members k=1", c, r, &deep, 1);
    diff_row("row12 cycle-members k=3", c, r, &deep, 3);
    diff_row("row12 cycle-members k=17", c, r, &deep, 17);
}

// --------------------------------------------------------------------------
// Row 19 -- degenerate shapes under composition.
// --------------------------------------------------------------------------

#[test]
fn row19_degenerate_shapes_under_composition() {
    let p = load_pair();
    let (c, r) = p.split();
    for v in [0, i32::MIN, i32::MAX, -1, 1, 7, -7, 6, -6] {
        diff_row(&format!("row19 all-{v} k=2"), c, r, &filled(v), 2);
    }
}

// --------------------------------------------------------------------------
// Row 34 -- `perform_expensive_operations()` on the pristine .bss array, i.e.
// before `long_exec` has ever run in this library instance.
// --------------------------------------------------------------------------

#[test]
fn row34_pristine_bss_first_call() {
    let p = load_pair();
    let (c, r) = p.split();
    // Freshly dlopen'd: both `array` objects must read as all zeros.
    assert!(c.array().iter().all(|&v| v == 0), "C .bss not zeroed");
    assert!(r.array().iter().all(|&v| v == 0), "Rust .bss not zeroed");
    c.peo();
    r.peo();
    assert_arrays_eq("row34 pristine-bss", c.array(), r.array());
    assert_eq!(c.xor(), r.xor());
}
