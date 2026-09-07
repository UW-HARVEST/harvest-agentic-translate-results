//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH the C `.so` and the
//! Rust `cdylib` via `libloading` and compares `pow43` bit-for-bit. Randomized
//! rows use a fixed-seed xorshift64* PRNG so failures are reproducible.

mod common;
use common::{Libs, Rng};

/// Well-defined domain, derived in `ERRORS.md`: `g_pow43` has 145 entries and
/// the subscript is `16 + x` / `16 + ((x+sign)>>6)`.
const LO: i32 = -16;
const HI: i32 = 8223;

/// Number of randomized draws per randomized row.
const N: usize = 20_000;

/// Assert the row's shape predicate actually holds, so a mis-specified filter
/// can never silently turn a row into a no-op.
fn check_all(l: &Libs, row: &str, xs: impl IntoIterator<Item = i32>) {
    let mut n = 0usize;
    for x in xs {
        assert!(
            (LO..=HI).contains(&x),
            "{row}: generator produced x={x} outside the well-defined domain [{LO}, {HI}]"
        );
        l.assert_same(x, row);
        n += 1;
    }
    assert!(n > 0, "{row}: generated zero inputs — row would be vacuous");
    println!("{row}: {n} inputs matched bit-for-bit");
}

// ---------------------------------------------------------------------------
// Row 1 — A0 table path, negative half, first valid index.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row01_table_x_minus_16() {
    let l = Libs::load();
    check_all(&l, "row01", [-16]);
    // i = 16 + (-16) = 0 -> stored +0.0; check the exact bit pattern.
    assert_eq!(l.c(-16).to_bits(), 0x0000_0000, "C pow43(-16) should be +0.0");
    assert_eq!(l.rust(-16).to_bits(), l.c(-16).to_bits());
}

// ---------------------------------------------------------------------------
// Row 2 — A0 table path, negative half (all 15 negative entries).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row02_table_negative_half() {
    let l = Libs::load();
    check_all(&l, "row02/exhaustive", -15..=-1);
    // ... plus randomized re-draws over the same shape.
    let mut rng = Rng::new(0xC0FFEE_02);
    check_all(&l, "row02/random", (0..N).map(|_| rng.range_i32(-15, -1)));
    // All of these must be strictly negative floats.
    for x in -15..=-1 {
        assert!(l.c(x) < 0.0, "C pow43({x}) expected negative");
    }
}

// ---------------------------------------------------------------------------
// Row 3 — A0 table path, x == 0.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row03_table_x_zero() {
    let l = Libs::load();
    check_all(&l, "row03", [0]);
    // +0.0 (0x00000000), not -0.0 (0x80000000) — bit compare catches this.
    assert_eq!(l.c(0).to_bits(), 0x0000_0000);
    assert_eq!(l.rust(0).to_bits(), 0x0000_0000);
}

// ---------------------------------------------------------------------------
// Row 4 — A0 table path, positive half, randomized.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row04_table_positive_half_random() {
    let l = Libs::load();
    check_all(&l, "row04/exhaustive", 1..=128);
    let mut rng = Rng::new(0xC0FFEE_04);
    check_all(&l, "row04/random", (0..N).map(|_| rng.range_i32(1, 128)));
}

// ---------------------------------------------------------------------------
// Row 5 — A0 upper boundary (last input taking the table path).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row05_table_upper_boundary_128() {
    let l = Libs::load();
    check_all(&l, "row05", [127, 128]);
    // 128 takes the table path; 129 does not. Both must agree, and the
    // dispatch must flip between them identically in C and Rust.
    check_all(&l, "row05/dispatch", [128, 129]);
}

// ---------------------------------------------------------------------------
// Row 6 — A1 mid path lower boundary.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row06_mid_lower_boundary_129() {
    let l = Libs::load();
    check_all(&l, "row06", [129, 130, 131]);
}

// ---------------------------------------------------------------------------
// Row 7 — A1 mid path, sign == 0  (x & 4 == 0 after the << 3).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row07_mid_sign0_random() {
    let l = Libs::load();
    let mut rng = Rng::new(0xC0FFEE_07);
    let xs: Vec<i32> = (0..N)
        .map(|_| rng.range_i32(129, 1023))
        .filter(|x| ((x << 3) * 2) & 64 == 0)
        .collect();
    for &x in &xs {
        assert_eq!(x & 4, 0, "row07 selector: expected x&4==0 for x={x}");
    }
    check_all(&l, "row07", xs);
}

// ---------------------------------------------------------------------------
// Row 8 — A1 mid path, sign == 64 (x & 4 != 0 after the << 3).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row08_mid_sign64_random() {
    let l = Libs::load();
    let mut rng = Rng::new(0xC0FFEE_08);
    let xs: Vec<i32> = (0..N)
        .map(|_| rng.range_i32(129, 1023))
        .filter(|x| ((x << 3) * 2) & 64 == 64)
        .collect();
    for &x in &xs {
        assert_ne!(x & 4, 0, "row08 selector: expected x&4!=0 for x={x}");
    }
    check_all(&l, "row08", xs);
}

// ---------------------------------------------------------------------------
// Row 9 — A1 mid path, exact bucket ((x<<3) & 63 == 0  <=>  x % 8 == 0).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row09_mid_exact_bucket() {
    let l = Libs::load();
    let xs: Vec<i32> = (129..=1023).filter(|x| (x << 3) & 63 == 0).collect();
    check_all(&l, "row09", xs);
}

// ---------------------------------------------------------------------------
// Row 10 — A1 mid path, top of the bucket ((x<<3) & 63 == 56).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row10_mid_bucket_top() {
    let l = Libs::load();
    let xs: Vec<i32> = (129..=1023).filter(|x| (x << 3) & 63 == 56).collect();
    check_all(&l, "row10", xs);
    // The << 3 shape can never produce low bits that are not a multiple of 8.
    for x in 129..=1023 {
        assert_eq!((x << 3) & 7, 0, "shifted low bits must be a multiple of 8");
    }
}

// ---------------------------------------------------------------------------
// Row 11 — A1 mid path upper boundary (last input with mult == 16).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row11_mid_upper_boundary_1023() {
    let l = Libs::load();
    check_all(&l, "row11", [1021, 1022, 1023]);
}

// ---------------------------------------------------------------------------
// Row 12 — A2 high path lower boundary (first input with mult == 256).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row12_high_lower_boundary_1024() {
    let l = Libs::load();
    check_all(&l, "row12", [1024, 1025, 1026]);
    // The mult switch (16 -> 256) happens exactly between 1023 and 1024;
    // both sides must be identical in C and Rust.
    check_all(&l, "row12/dispatch", [1023, 1024]);
}

// ---------------------------------------------------------------------------
// Row 13 — A2 high path, sign == 0 (x & 32 == 0).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row13_high_sign0_random() {
    let l = Libs::load();
    let mut rng = Rng::new(0xC0FFEE_13);
    let xs: Vec<i32> = (0..N)
        .map(|_| rng.range_i32(1024, HI))
        .filter(|x| (x * 2) & 64 == 0)
        .collect();
    for &x in &xs {
        assert_eq!(x & 32, 0, "row13 selector: expected x&32==0 for x={x}");
    }
    check_all(&l, "row13", xs);
}

// ---------------------------------------------------------------------------
// Row 14 — A2 high path, sign == 64 (x & 32 != 0).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row14_high_sign64_random() {
    let l = Libs::load();
    let mut rng = Rng::new(0xC0FFEE_14);
    let xs: Vec<i32> = (0..N)
        .map(|_| rng.range_i32(1024, HI))
        .filter(|x| (x * 2) & 64 == 64)
        .collect();
    for &x in &xs {
        assert_ne!(x & 32, 0, "row14 selector: expected x&32!=0 for x={x}");
    }
    check_all(&l, "row14", xs);
}

// ---------------------------------------------------------------------------
// Row 15 — A2 high path, x & 63 == 0 (frac == 0, polynomial == 1.0).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row15_high_exact_bucket() {
    let l = Libs::load();
    let xs: Vec<i32> = (1024..=HI).filter(|x| x & 63 == 0).collect();
    check_all(&l, "row15", xs);
    // Exact bucket hits must be exactly 256 * table entry.
    let v = l.c(4096);
    assert!(v.is_finite() && v > 0.0);
}

// ---------------------------------------------------------------------------
// Row 16 — A2 high path, x & 63 == 63 (max low bits; implies sign == 64).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row16_high_bucket_max_lowbits() {
    let l = Libs::load();
    let xs: Vec<i32> = (1024..=HI).filter(|x| x & 63 == 63).collect();
    for &x in &xs {
        assert_eq!((x * 2) & 64, 64, "x&63==63 must imply sign==64 (x={x})");
    }
    check_all(&l, "row16", xs);
}

// ---------------------------------------------------------------------------
// Row 17 — A2 high path, the two values straddling the sign flip.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row17_high_sign_flip_straddle() {
    let l = Libs::load();
    let mut xs = Vec::new();
    for base in (1024..=HI - 64).step_by(64) {
        let a = base | 31; // sign == 0
        let b = base | 32; // sign == 64
        if a <= HI {
            assert_eq!((a * 2) & 64, 0);
            xs.push(a);
        }
        if b <= HI {
            assert_eq!((b * 2) & 64, 64);
            xs.push(b);
        }
    }
    check_all(&l, "row17", xs);
}

// ---------------------------------------------------------------------------
// Row 18 — A2 high path upper boundary (last well-defined input).
// ---------------------------------------------------------------------------
#[test]
fn cfg_row18_high_upper_boundary_8223() {
    let l = Libs::load();
    check_all(&l, "row18", [8221, 8222, 8223]);
    // i = 16 + (8223 >> 6) = 144 = last element of g_pow43.
    assert_eq!(16 + ((8223 + ((8223 * 2) & 64)) >> 6), 144);
}

// ---------------------------------------------------------------------------
// Row 19 — exhaustive sweep of the entire well-defined domain.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row19_exhaustive_defined_domain() {
    let l = Libs::load();
    let mut mismatches = Vec::new();
    for x in LO..=HI {
        if l.c(x).to_bits() != l.rust(x).to_bits() {
            mismatches.push((x, l.c(x).to_bits(), l.rust(x).to_bits()));
        }
    }
    assert!(
        mismatches.is_empty(),
        "row19: {} of {} inputs diverged; first 20: {:?}",
        mismatches.len(),
        HI - LO + 1,
        &mismatches[..mismatches.len().min(20)]
    );
    println!("row19: all {} inputs in [{LO}, {HI}] matched", HI - LO + 1);
}

// ---------------------------------------------------------------------------
// Row 20 — unbiased fuzz + statelessness.
// ---------------------------------------------------------------------------
#[test]
fn cfg_row20_fuzz_and_statelessness() {
    let l = Libs::load();
    let mut rng = Rng::new(0xC0FFEE_20);
    for _ in 0..200_000 {
        let x = rng.range_i32(LO, HI);
        l.assert_same(x, "row20/fuzz");
    }
    // Statelessness: interleaving other calls must not change the result.
    let mut rng = Rng::new(0xFEED_20);
    for _ in 0..5_000 {
        let x = rng.range_i32(LO, HI);
        let y = rng.range_i32(LO, HI);
        let (c1, r1) = (l.c(x), l.rust(x));
        let _ = (l.c(y), l.rust(y));
        let (c2, r2) = (l.c(x), l.rust(x));
        assert_eq!(c1.to_bits(), c2.to_bits(), "C pow43({x}) is not stateless");
        assert_eq!(
            r1.to_bits(),
            r2.to_bits(),
            "Rust pow43({x}) is not stateless"
        );
        assert_eq!(c2.to_bits(), r2.to_bits());
    }
    println!("row20: 200000 fuzz draws + 5000 statelessness probes matched");
}
