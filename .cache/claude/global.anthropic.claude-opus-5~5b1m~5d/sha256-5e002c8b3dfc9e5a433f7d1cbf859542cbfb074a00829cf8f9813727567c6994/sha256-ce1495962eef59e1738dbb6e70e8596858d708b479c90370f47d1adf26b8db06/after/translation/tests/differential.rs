//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads **both** shared objects
//! with `libloading` and compares the raw IEEE-754 bits returned through the
//! FFI boundary; the Rust implementation is never called directly.

mod support;

use support::{DOMAIN_HI, DOMAIN_LO, Rng, assert_same, assert_same_all, decode, libs};

// ---------------------------------------------------------------------------
// Row 1-6 — A1 = table-only path (`x < 129`), no arithmetic at all
// ---------------------------------------------------------------------------

/// Row 1: `x = -16`, the lowest in-bounds subscript (`g_pow43[0]`).
#[test]
fn row01_lower_domain_edge_minus16() {
    assert_eq!(decode(-16).idx, 0);
    assert_same(-16);
}

/// Row 2: `x ∈ -15..=-1` exhaustively — the 16 negative leading table entries.
#[test]
fn row02_negative_leading_entries() {
    let n = assert_same_all(-15..=-1);
    assert_eq!(n, 15);
    for x in -15..=-1 {
        assert_eq!(decode(x).idx, 16 + x);
    }
}

/// Row 3: `x = 0` — `g_pow43[16]`, the *second* `+0.0` entry.
#[test]
fn row03_zero_maps_to_index_16() {
    assert_eq!(decode(0).idx, 16);
    assert_same(0);
    // Both `+0.0` entries must come back as `+0.0`, not `-0.0`.
    let l = libs();
    for x in [-16, 0] {
        assert_eq!(unsafe { (l.c)(x) }.to_bits(), 0x0000_0000);
        assert_eq!(unsafe { (l.rust)(x) }.to_bits(), 0x0000_0000);
    }
}

/// Row 4: `x ∈ 1..=128` exhaustively — the `x^(4/3)` entries reached directly.
#[test]
fn row04_direct_table_entries_1_to_128() {
    let n = assert_same_all(1..=128);
    assert_eq!(n, 128);
}

/// Row 5: the A1 dispatch flip — `x = 128` (table-only) vs `x = 129` (computed).
#[test]
fn row05_a1_dispatch_flip_128_129() {
    assert!(!decode(128).computed);
    assert!(decode(129).computed);
    assert_same(128);
    assert_same(129);
}

/// Row 6: randomized over the whole table-only path.
#[test]
fn row06_random_table_only_path() {
    let mut rng = Rng::new(0x5EED_0001);
    let xs: Vec<i32> = (0..4096).map(|_| rng.range(-16, 128)).collect();
    assert_same_all(xs);
}

// ---------------------------------------------------------------------------
// Rows 7-12 — A1 = computed, A2 = `mult 16` (`129 <= x <= 1023`, `x <<= 3`)
// ---------------------------------------------------------------------------

/// Row 7: `mult = 16`, `sign = 0`, `frac > 0`.
#[test]
fn row07_mult16_sign0_frac_positive() {
    let mut rng = Rng::new(0x5EED_0007);
    let xs = rng.take_where(4096, 129, 1023, |x| x & 4 == 0 && x % 8 != 0);
    for &x in &xs {
        let d = decode(x);
        assert_eq!((d.mult, d.sign), (16, 0));
        assert!(f32::from_bits(d.frac_bits) > 0.0);
    }
    assert_same_all(xs);
}

/// Row 8: `mult = 16`, `frac == 0` exactly (`x % 8 == 0` ⇒ `(x<<3) & 63 == 0`),
/// so `poly` is exactly `1.0f` with no rounding.
#[test]
fn row08_mult16_frac_exactly_zero() {
    let xs: Vec<i32> = (129..=1023).filter(|x| x % 8 == 0).collect();
    // multiples of 8 in 129..=1023: 136, 144, ..., 1016
    assert_eq!(xs.len(), 111);
    for &x in &xs {
        let d = decode(x);
        assert_eq!((d.mult, d.sign), (16, 0));
        assert_eq!(d.frac_bits, 0.0f32.to_bits());
        assert_eq!(d.poly_bits, 1.0f32.to_bits());
    }
    assert_same_all(xs);
}

/// Row 9: `mult = 16`, `sign = 64`, `frac < 0`.
#[test]
fn row09_mult16_sign64_frac_negative() {
    let mut rng = Rng::new(0x5EED_0009);
    let xs = rng.take_where(4096, 129, 1023, |x| x & 4 != 0);
    for &x in &xs {
        let d = decode(x);
        assert_eq!((d.mult, d.sign), (16, 64));
        assert!(f32::from_bits(d.frac_bits) < 0.0);
    }
    assert_same_all(xs);
}

/// Row 10: the A2 scale flip, lower side — first and last `mult = 16` inputs.
#[test]
fn row10_mult16_edges_129_and_1023() {
    assert_eq!(decode(129).mult, 16);
    assert_eq!(decode(1023).mult, 16);
    assert_same(129);
    assert_same(1023);
}

/// Row 11: the whole `mult = 16` path exhaustively.
#[test]
fn row11_mult16_exhaustive() {
    let n = assert_same_all(129..=1023);
    assert_eq!(n, 895);
}

/// Row 12: every A3 (`sign`) transition on the `mult = 16` path.
#[test]
fn row12_mult16_sign_transitions() {
    let mut rng = Rng::new(0x5EED_0012);
    let xs = rng.take_where(2048, 129, 1023, |x| matches!(x & 7, 0 | 3 | 4 | 7));
    assert_same_all(xs);
    // The deciding bit on this path is `x & 4` (bit 2), because `x` was shifted
    // left by 3 before `sign = 2*x & 64` looked at bit 5.
    for x in 129..=1023 {
        let expected = if x & 4 != 0 { 64 } else { 0 };
        assert_eq!(decode(x).sign, expected, "sign mismatch at x={x}");
    }
}

// ---------------------------------------------------------------------------
// Rows 13-21 — A1 = computed, A2 = `mult 256` (`x >= 1024`)
// ---------------------------------------------------------------------------

/// Row 13: `mult = 256`, `sign = 0`, `frac > 0` (`x & 63 ∈ 1..=31`).
#[test]
fn row13_mult256_sign0_frac_positive() {
    let mut rng = Rng::new(0x5EED_0013);
    let xs = rng.take_where(8192, 1024, DOMAIN_HI, |x| (1..=31).contains(&(x & 63)));
    for &x in &xs {
        let d = decode(x);
        assert_eq!((d.mult, d.sign), (256, 0));
        assert!(f32::from_bits(d.frac_bits) > 0.0);
    }
    assert_same_all(xs);
}

/// Row 14: `mult = 256`, `frac == 0` exactly (`x & 63 == 0`).
#[test]
fn row14_mult256_frac_exactly_zero() {
    let xs: Vec<i32> = (1024..=DOMAIN_HI).filter(|x| x & 63 == 0).collect();
    assert_eq!(xs.len(), 113);
    for &x in &xs {
        let d = decode(x);
        assert_eq!((d.mult, d.sign), (256, 0));
        assert_eq!(d.frac_bits, 0.0f32.to_bits());
        assert_eq!(d.poly_bits, 1.0f32.to_bits());
    }
    assert_same_all(xs);
}

/// Row 15: `mult = 256`, `sign = 64`, `frac < 0` (`x & 63 ∈ 32..=63`).
#[test]
fn row15_mult256_sign64_frac_negative() {
    let mut rng = Rng::new(0x5EED_0015);
    let xs = rng.take_where(8192, 1024, DOMAIN_HI, |x| (32..=63).contains(&(x & 63)));
    for &x in &xs {
        let d = decode(x);
        assert_eq!((d.mult, d.sign), (256, 64));
        assert!(f32::from_bits(d.frac_bits) < 0.0);
    }
    assert_same_all(xs);
}

/// Row 16: `x & 63 == 31` — largest positive numerator before the sign flip.
#[test]
fn row16_mult256_block_offset_31() {
    let xs: Vec<i32> = (1024..=DOMAIN_HI).filter(|x| x & 63 == 31).collect();
    assert!(!xs.is_empty());
    assert_same_all(xs);
}

/// Row 17: `x & 63 == 32` — the sign flip (numerator `-32`, denominator `+64`).
#[test]
fn row17_mult256_block_offset_32_sign_flip() {
    let xs: Vec<i32> = (1024..=DOMAIN_HI).filter(|x| x & 63 == 32).collect();
    assert!(!xs.is_empty());
    for &x in &xs {
        assert_eq!(decode(x).sign, 64);
    }
    assert_same_all(xs);
}

/// Row 18: `x & 63 == 63` — block top, numerator `-1`.
#[test]
fn row18_mult256_block_offset_63() {
    let xs: Vec<i32> = (1024..=DOMAIN_HI).filter(|x| x & 63 == 63).collect();
    assert!(!xs.is_empty());
    assert_same_all(xs);
}

/// Row 19: the A2 scale flip, upper side.
#[test]
fn row19_mult256_edge_1024() {
    assert_eq!(decode(1023).mult, 16);
    assert_eq!(decode(1024).mult, 256);
    assert_same(1024);
}

/// Row 20: the last in-bounds index block (`idx == 144`), including the upper
/// domain edge `x = 8223`.
#[test]
fn row20_last_index_block_8192_to_8223() {
    for x in 8192..=8223 {
        assert_eq!(decode(x).idx, 144, "idx should be 144 at x={x}");
    }
    let n = assert_same_all(8192..=8223);
    assert_eq!(n, 32);
    // One step further leaves the table (covered by ERRORS.md rows 5/6).
    assert_eq!(decode(8224).idx, 145);
}

/// Row 21: the whole `mult = 256` in-domain path exhaustively.
#[test]
fn row21_mult256_exhaustive() {
    let n = assert_same_all(1024..=DOMAIN_HI);
    assert_eq!(n, 7200);
}

// ---------------------------------------------------------------------------
// Rows 22-26 — cross-cutting
// ---------------------------------------------------------------------------

/// Row 22: the entire defined domain, exhaustively — every `x ∈ -16..=8223`.
#[test]
fn row22_whole_defined_domain_exhaustive() {
    let n = assert_same_all(DOMAIN_LO..=DOMAIN_HI);
    assert_eq!(n, 8240);
}

/// Row 23: statelessness — replay the domain shuffled and reversed and check
/// each result still matches the ascending sweep in both objects.
#[test]
fn row23_stateless_across_call_orders() {
    let l = libs();
    let ascending: Vec<(u32, u32)> = (DOMAIN_LO..=DOMAIN_HI)
        .map(|x| unsafe { ((l.c)(x).to_bits(), (l.rust)(x).to_bits()) })
        .collect();

    let idx_of = |x: i32| (x - DOMAIN_LO) as usize;

    for x in (DOMAIN_LO..=DOMAIN_HI).rev() {
        let got = unsafe { ((l.c)(x).to_bits(), (l.rust)(x).to_bits()) };
        assert_eq!(got, ascending[idx_of(x)], "reverse replay differs at x={x}");
    }

    let mut order: Vec<i32> = (DOMAIN_LO..=DOMAIN_HI).collect();
    let mut rng = Rng::new(0x5EED_0023);
    for i in (1..order.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        order.swap(i, j);
    }
    for &x in &order {
        let got = unsafe { ((l.c)(x).to_bits(), (l.rust)(x).to_bits()) };
        assert_eq!(got, ascending[idx_of(x)], "shuffled replay differs at x={x}");
    }
}

/// Row 24: shape invariants that must hold identically in both objects — all
/// results finite (proves the `frac` denominator is never zero) and `+0.0`
/// never becomes `-0.0`.
#[test]
fn row24_all_results_finite_in_both() {
    let l = libs();
    for x in DOMAIN_LO..=DOMAIN_HI {
        let c = unsafe { (l.c)(x) };
        let r = unsafe { (l.rust)(x) };
        assert!(c.is_finite(), "C produced non-finite at x={x}: {c:?}");
        assert!(r.is_finite(), "Rust produced non-finite at x={x}: {r:?}");
        assert_eq!(c.to_bits(), r.to_bits());
        if c == 0.0 {
            assert_eq!(c.to_bits(), 0x0000_0000, "C returned -0.0 at x={x}");
            assert_eq!(r.to_bits(), 0x0000_0000, "Rust returned -0.0 at x={x}");
        }
    }
}

/// Row 25: interleaved C/Rust/C/Rust calls through the two live handles.
#[test]
fn row25_interleaved_calls() {
    let l = libs();
    let mut rng = Rng::new(0x5EED_0025);
    for _ in 0..8192 {
        let x = rng.range(DOMAIN_LO, DOMAIN_HI);
        let c1 = unsafe { (l.c)(x) }.to_bits();
        let r1 = unsafe { (l.rust)(x) }.to_bits();
        let c2 = unsafe { (l.c)(x) }.to_bits();
        let r2 = unsafe { (l.rust)(x) }.to_bits();
        assert_eq!(c1, r1, "x={x}");
        assert_eq!(c1, c2, "C not deterministic at x={x}");
        assert_eq!(r1, r2, "Rust not deterministic at x={x}");
    }
}

/// Row 26 lives in `errors.rs` (`row26_full_i32_table_relative_sweep`) because
/// it needs the table-relative oracle that the out-of-bounds rows share.
#[test]
fn row26_see_errors_rs() {
    // Cheap in-domain slice of the same sweep, so this file also fails fast if
    // the full-range oracle regresses.
    let mut rng = Rng::new(0x5EED_0026);
    let xs: Vec<i32> = (0..4096).map(|_| rng.range(DOMAIN_LO, DOMAIN_HI)).collect();
    assert_same_all(xs);
}
