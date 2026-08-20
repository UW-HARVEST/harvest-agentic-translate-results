//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH shared libraries
//! (the C one built by CMake and the Rust `cdylib`) with `libloading` and calls
//! the exported `pow43` symbol in each, comparing the returned `float`
//! bit-for-bit. The Rust implementation is never called directly, so the
//! `#[no_mangle] extern "C"` wrapper is part of what is under test.
//!
//! All randomness comes from a fixed-seed SplitMix64 PRNG, so a failure is
//! always reproducible.

mod common;

use std::ffi::c_int;

use common::*;

/// Samples per randomized row.
const N: usize = 2_000;

// ---------------------------------------------------------------------------
// Row 1 — branch A, negative-mirror half: x in [-16, -1] -> index 0..15
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row01_branch_a_negative_half() {
    // exhaustive over the whole (tiny) sub-domain ...
    for x in -16..=-1 {
        assert_eq!(c_branch(x), Branch::A);
        assert!((0..=15).contains(&c_table_index(x)));
        assert_bit_identical(x, "row01 exhaustive");
    }
    // ... plus randomized repetition in random order.
    let mut rng = Rng::new(0x01_5EED);
    for x in sample_where(&mut rng, -16, -1, N, |_| true) {
        assert_bit_identical(x, "row01 random");
    }
}

// ---------------------------------------------------------------------------
// Row 2 — branch A, the zero entry: x == 0 -> index 16
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row02_branch_a_zero() {
    assert_eq!(c_table_index(0), 16);
    assert_bit_identical(0, "row02");
    // repeated calls must keep returning the same bits
    for _ in 0..100 {
        assert_bit_identical(0, "row02 repeat");
    }
}

// ---------------------------------------------------------------------------
// Row 3 — branch A, small positives: x in [1, 15] -> index 17..31
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row03_branch_a_small_positive() {
    for x in 1..=15 {
        assert!((17..=31).contains(&c_table_index(x)));
        assert_bit_identical(x, "row03 exhaustive");
    }
    let mut rng = Rng::new(0x03_5EED);
    for x in sample_where(&mut rng, 1, 15, N, |_| true) {
        assert_bit_identical(x, "row03 random");
    }
}

// ---------------------------------------------------------------------------
// Row 4 — branch A, main positive half: x in [16, 128] -> index 32..144
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row04_branch_a_positive_half() {
    for x in 16..=128 {
        assert!((32..=144).contains(&c_table_index(x)));
        assert_bit_identical(x, "row04 exhaustive");
    }
    let mut rng = Rng::new(0x04_5EED);
    for x in sample_where(&mut rng, 16, 128, N, |_| true) {
        assert_bit_identical(x, "row04 random");
    }
}

// ---------------------------------------------------------------------------
// Row 5 — branch A boundaries
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row05_branch_a_boundaries() {
    let boundaries: [c_int; 10] = [-16, -15, -2, -1, 0, 1, 15, 16, 127, 128];
    for x in boundaries {
        assert_eq!(c_branch(x), Branch::A, "x={x} must take branch A");
        assert!(in_domain(x));
        assert_bit_identical(x, "row05");
    }
    // The two extremes must hit the first and last table entries.
    assert_eq!(c_table_index(-16), 0);
    assert_eq!(c_table_index(128), 144);
}

// ---------------------------------------------------------------------------
// Row 6 — branch B, sign == 0, frac > 0
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row06_branch_b_sign0_frac_pos() {
    let mut rng = Rng::new(0x06_5EED);
    let xs = sample_where(&mut rng, 129, 1023, N, |x| {
        c_sign(x) == 0 && c_frac_parts(x).0 > 0
    });
    for x in xs {
        assert_eq!(c_branch(x), Branch::B);
        assert_eq!(c_mult(x), 16);
        assert_eq!(c_sign(x), 0);
        assert!(c_frac_parts(x).0 > 0);
        assert!(in_domain(x));
        assert_bit_identical(x, "row06");
    }
}

// ---------------------------------------------------------------------------
// Row 7 — branch B, frac == 0 exactly (x % 8 == 0 so that 8x % 64 == 0)
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row07_branch_b_frac_zero() {
    let mut rng = Rng::new(0x07_5EED);
    let xs = sample_where(&mut rng, 129, 1023, N, |x| x % 8 == 0);
    for x in xs {
        let (num, den) = c_frac_parts(x);
        assert_eq!(num, 0, "x={x} should give frac == 0");
        assert!(den >= 1024);
        assert_eq!(c_sign(x), 0);
        assert_bit_identical(x, "row07");
    }
    // every multiple of 8 in the branch, exhaustively
    for x in (136..=1016).step_by(8) {
        assert_eq!(c_frac_parts(x).0, 0);
        assert_bit_identical(x, "row07 exhaustive");
    }
}

// ---------------------------------------------------------------------------
// Row 8 — branch B, sign == 64 -> frac < 0
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row08_branch_b_sign64_frac_neg() {
    let mut rng = Rng::new(0x08_5EED);
    let xs = sample_where(&mut rng, 129, 1023, N, |x| c_sign(x) == 64);
    for x in xs {
        assert_eq!(c_sign(x), 64);
        assert!(c_frac_parts(x).0 < 0, "x={x} must give a negative numerator");
        assert!(in_domain(x));
        assert_bit_identical(x, "row08");
    }
}

// ---------------------------------------------------------------------------
// Row 9 — branch B boundaries (both sign values at each end)
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row09_branch_b_boundaries() {
    let boundaries: [c_int; 11] = [129, 130, 131, 132, 135, 136, 1016, 1020, 1021, 1022, 1023];
    let mut saw_sign0 = false;
    let mut saw_sign64 = false;
    for x in boundaries {
        assert_eq!(c_branch(x), Branch::B);
        assert!(in_domain(x));
        match c_sign(x) {
            0 => saw_sign0 = true,
            64 => saw_sign64 = true,
            s => panic!("unexpected sign {s}"),
        }
        assert_bit_identical(x, "row09");
    }
    assert!(saw_sign0 && saw_sign64, "row must cover both sign values");
    // lowest / highest index reachable on branch B
    assert_eq!(c_table_index(129), 32);
    assert_eq!(c_table_index(1023), 144);
}

// ---------------------------------------------------------------------------
// Row 10 — branch C, sign == 0, frac > 0
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row10_branch_c_sign0_frac_pos() {
    let mut rng = Rng::new(0x10_5EED);
    let xs = sample_where(&mut rng, 1024, DOMAIN_MAX, N, |x| {
        c_sign(x) == 0 && (x & 63) != 0
    });
    for x in xs {
        assert_eq!(c_branch(x), Branch::C);
        assert_eq!(c_mult(x), 256);
        let (num, _) = c_frac_parts(x);
        assert!(num > 0);
        assert!(in_domain(x));
        assert_bit_identical(x, "row10");
    }
}

// ---------------------------------------------------------------------------
// Row 11 — branch C, frac == 0 exactly (x % 64 == 0)
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row11_branch_c_frac_zero() {
    for x in (1024..=DOMAIN_MAX).step_by(64) {
        assert_eq!(c_sign(x), 0);
        assert_eq!(c_frac_parts(x).0, 0);
        assert_bit_identical(x, "row11 exhaustive multiples of 64");
    }
    let mut rng = Rng::new(0x11_5EED);
    for x in sample_where(&mut rng, 1024, DOMAIN_MAX, N / 4, |x| x % 64 == 0) {
        assert_bit_identical(x, "row11 random");
    }
}

// ---------------------------------------------------------------------------
// Row 12 — branch C, sign == 64 -> frac < 0
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row12_branch_c_sign64_frac_neg() {
    let mut rng = Rng::new(0x12_5EED);
    let xs = sample_where(&mut rng, 1024, DOMAIN_MAX, N, |x| c_sign(x) == 64);
    for x in xs {
        assert_eq!(c_sign(x), 64);
        let (num, den) = c_frac_parts(x);
        assert!((-32..=-1).contains(&num), "x={x} num={num}");
        assert!(den >= 1024 + 64);
        assert!(in_domain(x));
        assert_bit_identical(x, "row12");
    }
}

// ---------------------------------------------------------------------------
// Row 13 — branch C, extreme table indices (32 and 144)
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row13_branch_c_index_extremes() {
    // lowest index reachable on branch C
    for x in 1024..=1055 {
        assert_eq!(c_table_index(x), 32, "x={x}");
        assert_bit_identical(x, "row13 index 32");
    }
    // highest defined index, reached both with sign == 64 (8128..8191) and
    // with sign == 0 (8192..8223)
    let mut saw64 = false;
    let mut saw0 = false;
    for x in 8128..=DOMAIN_MAX {
        if c_table_index(x) == 144 {
            match c_sign(x) {
                0 => saw0 = true,
                64 => saw64 = true,
                s => panic!("unexpected sign {s}"),
            }
            assert_bit_identical(x, "row13 index 144");
        }
    }
    assert!(saw0 && saw64, "index 144 must be reached with both sign values");
}

// ---------------------------------------------------------------------------
// Row 14 — branch C boundaries
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row14_branch_c_boundaries() {
    let boundaries: [c_int; 10] = [1024, 1025, 1055, 1056, 1087, 1088, 8191, 8192, 8222, 8223];
    for x in boundaries {
        assert_eq!(c_branch(x), Branch::C);
        assert!(in_domain(x), "x={x} idx={}", c_table_index(x));
        assert_bit_identical(x, "row14");
    }
    assert_eq!(c_table_index(DOMAIN_MAX), 144);
    // 8224 is the first input that leaves the table (see ERRORS.md row 7)
    assert_eq!(c_table_index(8224), 145);
}

// ---------------------------------------------------------------------------
// Row 15 — branch transitions / mult switch
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row15_branch_transitions() {
    // A -> B
    assert_eq!(c_branch(128), Branch::A);
    assert_eq!(c_branch(129), Branch::B);
    // B -> C, mult switches 16 -> 256
    assert_eq!(c_mult(1023), 16);
    assert_eq!(c_mult(1024), 256);
    for x in [126, 127, 128, 129, 130, 1021, 1022, 1023, 1024, 1025, 1026] {
        assert_bit_identical(x, "row15 transition");
    }
    // Both implementations must agree on the *whole* neighbourhood of each
    // transition, not just the two edge values.
    for x in 120..=140 {
        assert_bit_identical(x, "row15 A/B neighbourhood");
    }
    for x in 1010..=1040 {
        assert_bit_identical(x, "row15 B/C neighbourhood");
    }
}

// ---------------------------------------------------------------------------
// Row 16 — exhaustive sweep of the whole defined domain
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row16_exhaustive_domain_sweep() {
    let mut checked = 0usize;
    for x in DOMAIN_MIN..=DOMAIN_MAX {
        assert!(in_domain(x), "x={x} unexpectedly out of table range");
        assert_bit_identical(x, "row16 exhaustive");
        checked += 1;
    }
    assert_eq!(checked, 8240, "domain size changed");
}

// ---------------------------------------------------------------------------
// Row 17 — randomized whole-domain property test
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row17_randomized_whole_domain() {
    let mut rng = Rng::new(0x17_5EED_C0FFEE);
    let mut branch_hits = [0usize; 3];
    let mut sign_hits = [0usize; 2];
    for _ in 0..20_000 {
        let x = rng.range_i32(DOMAIN_MIN, DOMAIN_MAX);
        assert_bit_identical(x, "row17 random");
        branch_hits[match c_branch(x) {
            Branch::A => 0,
            Branch::B => 1,
            Branch::C => 2,
        }] += 1;
        if c_branch(x) != Branch::A {
            sign_hits[(c_sign(x) / 64) as usize] += 1;
        }
    }
    // the run must actually have covered all branches and both sign values
    assert!(branch_hits.iter().all(|&h| h > 0), "branch coverage {branch_hits:?}");
    assert!(sign_hits.iter().all(|&h| h > 0), "sign coverage {sign_hits:?}");
}

// ---------------------------------------------------------------------------
// Row 18 — call order / repetition independence (no hidden state)
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row18_call_order_and_repetition() {
    let i = impls();
    let mut rng = Rng::new(0x18_5EED);
    // random permutation of a slice of the domain
    let mut xs: Vec<c_int> = (DOMAIN_MIN..=DOMAIN_MAX).step_by(7).collect();
    for k in (1..xs.len()).rev() {
        let j = rng.range(0, k as i64) as usize;
        xs.swap(k, j);
    }

    // first pass: remember every result, calling C and Rust interleaved
    let mut first: Vec<(c_int, u32, u32)> = Vec::with_capacity(xs.len());
    for &x in &xs {
        let (c, r) = unsafe { ((i.c_pow43)(x), (i.rust_pow43)(x)) };
        assert_eq!(c.to_bits(), r.to_bits(), "row18 first pass x={x}");
        first.push((x, c.to_bits(), r.to_bits()));
    }

    // second pass: reverse order, all C calls first, then all Rust calls
    let c_second: Vec<u32> = xs
        .iter()
        .rev()
        .map(|&x| unsafe { (i.c_pow43)(x) }.to_bits())
        .collect();
    let r_second: Vec<u32> = xs
        .iter()
        .rev()
        .map(|&x| unsafe { (i.rust_pow43)(x) }.to_bits())
        .collect();
    for (k, (&x, (c, r))) in xs
        .iter()
        .rev()
        .zip(c_second.iter().copied().zip(r_second.iter().copied()))
        .enumerate()
    {
        assert_eq!(c, r, "row18 second pass x={x} (k={k})");
        let (_, c0, _) = first[xs.len() - 1 - k];
        assert_eq!(c, c0, "row18 result changed between passes for x={x}");
    }

    // third pass: same input many times in a row
    for &x in xs.iter().take(50) {
        for _ in 0..25 {
            assert_bit_identical(x, "row18 repeat");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 19 — concurrent calls from several threads
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row19_concurrent_calls() {
    let i = impls();
    let mut handles = Vec::new();
    for t in 0..8u64 {
        handles.push(std::thread::spawn(move || {
            let mut rng = Rng::new(0x19_5EED + t * 7919);
            for _ in 0..5_000 {
                let x = rng.range_i32(DOMAIN_MIN, DOMAIN_MAX);
                let (c, r) = unsafe { ((i.c_pow43)(x), (i.rust_pow43)(x)) };
                assert_eq!(
                    c.to_bits(),
                    r.to_bits(),
                    "row19 thread {t}: pow43({x}) C={c:?} Rust={r:?}"
                );
            }
        }));
    }
    for h in handles {
        h.join().expect("worker thread panicked");
    }
}

// ---------------------------------------------------------------------------
// Row 20 — fresh dlopen of both libraries
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row20_reload_libraries() {
    let (_c_lib, _r_lib, c_fn, r_fn) = load_fresh();
    let interesting: [c_int; 18] = [
        -16, -1, 0, 1, 15, 16, 127, 128, 129, 130, 1023, 1024, 1025, 4096, 8191, 8192, 8222, 8223,
    ];
    for x in interesting {
        let (c, r) = unsafe { (c_fn(x), r_fn(x)) };
        assert_eq!(
            c.to_bits(),
            r.to_bits(),
            "row20 (fresh handles) pow43({x}): C={c:?} Rust={r:?}"
        );
        // and identical to the values seen through the original handles
        let (c0, r0) = call_both(x);
        assert_eq!(c.to_bits(), c0.to_bits(), "row20 C changed after reload (x={x})");
        assert_eq!(r.to_bits(), r0.to_bits(), "row20 Rust changed after reload (x={x})");
    }
}

// ---------------------------------------------------------------------------
// Row 21 — result classification over the whole domain
// ---------------------------------------------------------------------------
#[test]
fn phase_b_row21_result_classification() {
    // sign of zero and finiteness, compared between the two implementations
    for x in DOMAIN_MIN..=DOMAIN_MAX {
        let (c, r) = call_both(x);
        assert_eq!(c.is_nan(), r.is_nan(), "NaN-ness differs at x={x}");
        assert_eq!(c.is_infinite(), r.is_infinite(), "infinity differs at x={x}");
        assert_eq!(c.is_sign_negative(), r.is_sign_negative(), "sign differs at x={x}");
        assert_eq!(c.to_bits(), r.to_bits(), "bits differ at x={x}");
        // the C never produces inf/NaN inside the defined domain
        // (see ERRORS.md row 11: the divisor is always >= 1024)
        assert!(c.is_finite(), "C produced a non-finite value at x={x}: {c:?}");
    }
    // +0.0, never -0.0, for the two inputs whose table entry is zero
    for x in [-16, 0] {
        let (c, r) = call_both(x);
        assert_eq!(c.to_bits(), 0x0000_0000, "C pow43({x}) should be +0.0");
        assert_eq!(r.to_bits(), 0x0000_0000, "Rust pow43({x}) should be +0.0");
    }
    // strictly increasing over the positive part of the table, identically in
    // both implementations (catches an off-by-one in the table or the index)
    let mut prev_c = f32::NEG_INFINITY;
    let mut prev_r = f32::NEG_INFINITY;
    for x in 1..=128 {
        let (c, r) = call_both(x);
        assert!(c > prev_c, "C not increasing at x={x}: {c} <= {prev_c}");
        assert!(r > prev_r, "Rust not increasing at x={x}: {r} <= {prev_r}");
        prev_c = c;
        prev_r = r;
    }
}
