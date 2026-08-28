//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects (C and Rust) through their exported
//! `gaussian_kernel` symbol and compares the entire scratch buffer, including
//! the bytes *before* `dest` and 16 `f32` of trailing guard padding, so a
//! divergent number of stores is caught as well as a divergent value.

mod common;

use common::*;
use std::ffi::c_int;

const SEED: u64 = 0x5EED_C0FF_EE00_0001;

// ---------------------------------------------------------------------------
// radius generators, one per `R_*` class of CONFIGS.md
// ---------------------------------------------------------------------------

fn r_typical(rng: &mut Rng) -> f32 {
    rng.range_f32(0.25, 8.0)
}
fn r_wide(rng: &mut Rng) -> f32 {
    rng.log_range_f32(1e2, 1e6)
}
fn r_narrow(rng: &mut Rng) -> f32 {
    rng.log_range_f32(1e-6, 1e-2)
}
fn r_neg(rng: &mut Rng) -> f32 {
    -r_typical(rng)
}

/// A radius for which some tap `r` lands exactly on the clamp threshold
/// `|r * (sigma/radius)| == CLAMP_X`, then jittered by a few ULPs.
fn r_threshold(rng: &mut Rng, size: c_int) -> f32 {
    let hsize = (size / 2).max(1);
    let r = rng.range_i32(1, hsize) as f32;
    let base = SIGMA * r / CLAMP_X;
    let steps = rng.range_i32(-3, 3);
    let mut v = base;
    for _ in 0..steps.abs() {
        v = if steps > 0 {
            f32::from_bits(v.to_bits() + 1)
        } else {
            f32::from_bits(v.to_bits() - 1)
        };
    }
    v
}

fn odd_small(rng: &mut Rng) -> c_int {
    3 + 2 * rng.range_i32(0, 6)
}
fn even_small(rng: &mut Rng) -> c_int {
    2 + 2 * rng.range_i32(0, 7)
}
fn odd_large(rng: &mut Rng) -> c_int {
    let n = rng.range_i32(31, 512);
    2 * n + 1
}
fn even_large(rng: &mut Rng) -> c_int {
    let n = rng.range_i32(32, 512);
    2 * n
}

const DRAWS: usize = 96;

// ---------------------------------------------------------------------------
// C1 .. C5 — the core valid shapes
// ---------------------------------------------------------------------------

#[test]
fn c01_size_one_typical_radius() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..DRAWS {
        let case = Case::new(1, r_typical(&mut rng));
        let out = assert_same(&case);
        // Coverage proof: a single tap always normalises to exactly 1.0f.
        assert_eq!(
            out[0], 0x3f80_0000,
            "size=1 must normalise to 1.0f, got 0x{:08x} for {:?}",
            out[0], case
        );
        // and nothing past index 0 may be touched
        assert!(out[1..].iter().all(|&w| w == SENTINEL));
    }
}

#[test]
fn c02_odd_small_typical_radius() {
    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..DRAWS {
        let size = odd_small(&mut rng);
        let out = assert_same(&Case::new(size, r_typical(&mut rng)));
        // exactly `size` stores: guard untouched
        assert!(
            out[size as usize..].iter().all(|&w| w == SENTINEL),
            "odd size={size} must store exactly {size} elements"
        );
    }
}

#[test]
fn c03_even_small_typical_radius_writes_one_past() {
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..DRAWS {
        let size = even_small(&mut rng);
        let out = assert_same(&Case::new(size, r_typical(&mut rng)));
        // The C stores size+1 elements: index `size` IS touched ...
        assert_ne!(
            out[size as usize], SENTINEL,
            "even size={size}: the C writes one element past `size`"
        );
        // ... and nothing beyond that.
        assert!(out[size as usize + 1..].iter().all(|&w| w == SENTINEL));
    }
}

#[test]
fn c04_odd_large_typical_radius() {
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..DRAWS {
        let size = odd_large(&mut rng);
        assert_same(&Case::new(size, r_typical(&mut rng)));
    }
}

#[test]
fn c05_even_large_typical_radius() {
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..DRAWS {
        let size = even_large(&mut rng);
        assert_same(&Case::new(size, r_typical(&mut rng)));
    }
}

// ---------------------------------------------------------------------------
// C6 .. C9 — clamp-branch coverage
// ---------------------------------------------------------------------------

#[test]
fn c06_wide_radius_clamp_never_taken() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..DRAWS {
        let size = if rng.next_u32() & 1 == 0 {
            odd_small(&mut rng)
        } else {
            even_small(&mut rng)
        };
        let out = assert_same(&Case::new(size, r_wide(&mut rng)));
        // every in-range element must be strictly positive (no clamping)
        for i in 0..size as usize {
            let v = f32::from_bits(out[i]);
            assert!(v > 0.0, "wide radius: out[{i}]={v} should be > 0");
        }
    }
}

#[test]
fn c07_narrow_radius_clamp_always_taken() {
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..DRAWS {
        let size = if rng.next_u32() & 1 == 0 {
            odd_large(&mut rng)
        } else {
            even_large(&mut rng)
        };
        let hsize = (size / 2) as usize;
        let out = assert_same(&Case::new(size, r_narrow(&mut rng)));
        // impulse at the centre, zeros elsewhere in range
        assert_eq!(
            out[hsize], 0x3f80_0000,
            "narrow radius: centre tap must normalise to 1.0f"
        );
        for i in 0..size as usize {
            if i != hsize {
                assert_eq!(out[i], 0, "narrow radius: out[{i}] must be +0.0f");
            }
        }
    }
}

#[test]
fn c08_threshold_radius() {
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..DRAWS {
        for &size in &[3, 4, 5, 8, 9, 16, 17, 64, 65] {
            let radius = r_threshold(&mut rng, size);
            assert_same(&Case::new(size, radius));
            assert_same(&Case::new(size, -radius));
        }
    }
}

#[test]
fn c09_negative_radius_mirror() {
    let mut rng = Rng::new(SEED ^ 9);
    for _ in 0..DRAWS {
        for &size in &[1, 2, 3, 4, 7, 8, 33, 64, 129] {
            assert_same(&Case::new(size, r_neg(&mut rng)));
        }
    }
}

// ---------------------------------------------------------------------------
// C10 .. C16 — special float classes for `radius`
// ---------------------------------------------------------------------------

fn sweep_radius(radius: f32, sizes: &[c_int]) {
    for &size in sizes {
        for fill in [Fill::Sentinel, Fill::Zero, Fill::Random(0xABCD_1234)] {
            for offset in [0usize, 1, 3] {
                assert_same(&Case::new(size, radius).fill(fill).offset(offset));
            }
        }
    }
}

const SPECIAL_SIZES: &[c_int] = &[0, 1, 2, 3, 4, 5, 8, 9, 16, 17, 31, 32, 63, 64, 65, 128, 129];

#[test]
fn c10_extreme_finite_radii() {
    for radius in [
        1.0f32,
        -1.0,
        f32::MAX,
        -f32::MAX,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        SIGMA,
        -SIGMA,
    ] {
        sweep_radius(radius, SPECIAL_SIZES);
    }
}

#[test]
fn c11_positive_infinity_radius() {
    let sizes = SPECIAL_SIZES;
    sweep_radius(f32::INFINITY, sizes);
    // coverage: rs == +0 ⇒ every tap is V0 before normalisation
    for &size in &[1i32, 2, 3, 4] {
        let out = assert_same(&Case::new(size, f32::INFINITY));
        let stores = (2 * (size / 2) + 1) as usize;
        // the OOB tail store (even size) is never scaled
        if size % 2 == 0 {
            assert_eq!(out[stores - 1], V0_BITS);
        }
        let expect = 1.0f32 / stores as f32;
        for i in 0..size as usize {
            assert_eq!(
                f32::from_bits(out[i]),
                expect,
                "size={size}: normalised tap {i}"
            );
        }
    }
}

#[test]
fn c12_negative_infinity_radius() {
    sweep_radius(f32::NEG_INFINITY, SPECIAL_SIZES);
}

#[test]
fn c13_positive_zero_radius_skips_normalisation() {
    sweep_radius(0.0, SPECIAL_SIZES);
    for &size in &[1i32, 2, 3, 4, 17] {
        let out = assert_same(&Case::new(size, 0.0));
        let stores = (2 * (size / 2) + 1) as usize;
        for i in 0..stores {
            assert_eq!(out[i], 0, "radius=+0: every store must be +0.0f");
        }
        assert!(out[stores..].iter().all(|&w| w == SENTINEL));
    }
}

#[test]
fn c14_negative_zero_radius() {
    sweep_radius(-0.0, SPECIAL_SIZES);
    for &size in &[1i32, 2, 3, 4, 17] {
        let out = assert_same(&Case::new(size, -0.0));
        let stores = (2 * (size / 2) + 1) as usize;
        for i in 0..stores {
            assert_eq!(out[i], 0, "radius=-0: every store must be +0.0f");
        }
    }
}

#[test]
fn c15_nan_radii() {
    for bits in [
        0x7fc0_0000u32,
        0xffc0_0000,
        0x7f80_0001,
        0xff80_0001,
        0x7fff_ffff,
        0xffff_ffff,
        0x7fc0_dead,
        0xffca_fe00,
    ] {
        let radius = f32::from_bits(bits);
        assert!(radius.is_nan());
        sweep_radius(radius, SPECIAL_SIZES);
        // a NaN radius must never leave a NaN in the buffer
        let out = assert_same(&Case::new(9, radius));
        for i in 0..9 {
            assert_eq!(out[i], 0, "NaN radius: out[{i}] must be +0.0f");
        }
    }
}

#[test]
fn c16_subnormal_radii() {
    for bits in [
        0x0000_0001u32,
        0x0000_0002,
        0x0000_ffff,
        0x0040_0000,
        0x007f_ffff,
        0x8000_0001,
        0x8040_0000,
        0x807f_ffff,
    ] {
        let radius = f32::from_bits(bits);
        assert!(radius != 0.0 && radius.is_subnormal());
        sweep_radius(radius, SPECIAL_SIZES);
    }
}

// ---------------------------------------------------------------------------
// C17 .. C21 — degenerate `size` shapes with every radius class
// ---------------------------------------------------------------------------

fn all_radius_classes(rng: &mut Rng) -> Vec<f32> {
    let mut v = special_radii();
    v.push(r_typical(rng));
    v.push(r_wide(rng));
    v.push(r_narrow(rng));
    v.push(r_neg(rng));
    for _ in 0..16 {
        v.push(rng.any_f32());
    }
    v
}

#[test]
fn c17_size_zero_unnormalised_single_store() {
    let mut rng = Rng::new(SEED ^ 17);
    for radius in all_radius_classes(&mut rng) {
        for fill in [Fill::Sentinel, Fill::Zero, Fill::Random(7)] {
            for offset in [0usize, 1, 3, 7] {
                let out = assert_same(&Case::new(0, radius).fill(fill).offset(offset));
                // exactly one store, at `dest[0]`
                if fill == Fill::Sentinel {
                    assert!(out[offset + 1..].iter().all(|&w| w == SENTINEL));
                }
            }
        }
    }
}

#[test]
fn c18_size_minus_one_unnormalised_single_store() {
    let mut rng = Rng::new(SEED ^ 18);
    for radius in all_radius_classes(&mut rng) {
        for fill in [Fill::Sentinel, Fill::Zero, Fill::Random(11)] {
            for offset in [0usize, 1, 3, 7] {
                let out = assert_same(&Case::new(-1, radius).fill(fill).offset(offset));
                if fill == Fill::Sentinel {
                    assert!(out[offset + 1..].iter().all(|&w| w == SENTINEL));
                }
            }
        }
    }
}

#[test]
fn c19_negative_size_no_stores() {
    let mut rng = Rng::new(SEED ^ 19);
    let radii = all_radius_classes(&mut rng);
    let mut sizes: Vec<c_int> = vec![-2, -3, -4, -5, -100, -101, -65536];
    for _ in 0..16 {
        sizes.push(rng.range_i32(-100_000, -2));
    }
    for size in sizes {
        for radius in &radii {
            let fill = Fill::Random(0xDEAD_BEEF);
            let out = assert_same(&Case::new(size, *radius).fill(fill));
            // buffer must be byte-identical to its initial contents
            let mut expect = Rng::new(0xDEAD_BEEF);
            for w in &out {
                assert_eq!(*w, expect.next_u32(), "size={size} must not store anything");
            }
        }
    }
}

#[test]
fn c20_int_min_size() {
    let mut rng = Rng::new(SEED ^ 20);
    for radius in all_radius_classes(&mut rng) {
        for size in [i32::MIN, i32::MIN + 1, i32::MIN + 2] {
            for fill in [Fill::Sentinel, Fill::Zero, Fill::Random(3)] {
                assert_same(&Case::new(size, radius).fill(fill));
            }
        }
    }
}

#[test]
fn c21_null_dest_when_no_stores() {
    let mut rng = Rng::new(SEED ^ 21);
    for radius in all_radius_classes(&mut rng) {
        for size in [-2, -3, -7, -12345, i32::MIN, i32::MIN + 1] {
            assert_same(&Case::new(size, radius).null_dest());
        }
    }
}

// ---------------------------------------------------------------------------
// C22 .. C24 — pointer shape, initial contents, repeated calls
// ---------------------------------------------------------------------------

#[test]
fn c22_offset_dest_pointer() {
    let mut rng = Rng::new(SEED ^ 22);
    for offset in [1usize, 2, 3, 5, 7, 9] {
        for &size in SPECIAL_SIZES {
            assert_same(&Case::new(size, r_typical(&mut rng)).offset(offset));
        }
        for size in [-1, -2, 0] {
            assert_same(&Case::new(size, r_typical(&mut rng)).offset(offset));
        }
    }
}

#[test]
fn c23_initial_buffer_contents() {
    let mut rng = Rng::new(SEED ^ 23);
    for fill in [
        Fill::Zero,
        Fill::Sentinel,
        Fill::Random(1),
        Fill::Random(2),
        Fill::Random(0xFFFF_FFFF),
    ] {
        for &size in SPECIAL_SIZES {
            assert_same(&Case::new(size, r_typical(&mut rng)).fill(fill));
        }
        for size in [-1, -2, -3, 0] {
            assert_same(&Case::new(size, r_typical(&mut rng)).fill(fill));
        }
    }
}

#[test]
fn c24_repeated_calls_share_no_state() {
    let mut rng = Rng::new(SEED ^ 24);
    for _ in 0..DRAWS {
        let seq = [
            Case::new(rng.range_i32(-4, 40), r_typical(&mut rng)).fill(Fill::Random(5)),
            Case::new(rng.range_i32(-4, 40), rng.any_f32()).fill(Fill::Random(5)),
            Case::new(rng.range_i32(-4, 40), r_narrow(&mut rng)).fill(Fill::Random(5)),
            Case::new(rng.range_i32(0, 40), 0.0).fill(Fill::Random(5)),
            Case::new(rng.range_i32(0, 40), f32::INFINITY).fill(Fill::Random(5)),
        ];
        assert_same_sequence(&seq);
    }
}

// ---------------------------------------------------------------------------
// C25 .. C28 — fuzz + exhaustive sweeps
// ---------------------------------------------------------------------------

#[test]
fn c25_fuzz_small() {
    let mut rng = Rng::new(SEED ^ 25);
    let mut n = 0u32;
    for i in 0..20_000u32 {
        let size = rng.range_i32(-8, 64);
        let radius = rng.any_f32();
        let fill = match i % 3 {
            0 => Fill::Sentinel,
            1 => Fill::Zero,
            _ => Fill::Random(i as u64),
        };
        let offset = (i % 5) as usize;
        let mut case = Case::new(size, radius).fill(fill).offset(offset);
        if size / 2 < 0 && i % 7 == 0 {
            case = case.null_dest();
        }
        assert_same(&case);
        n += 1;
    }
    assert_eq!(n, 20_000, "c25 must perform 20000 randomized draws");
    eprintln!("c25_fuzz_small: {n} randomized differential draws");
}

#[test]
fn c26_fuzz_large() {
    let mut rng = Rng::new(SEED ^ 26);
    let mut n = 0u32;
    for i in 0..2_000u32 {
        let size = rng.range_i32(1, 2048);
        let radius = rng.any_f32();
        let fill = if i % 2 == 0 {
            Fill::Sentinel
        } else {
            Fill::Random(i as u64)
        };
        assert_same(&Case::new(size, radius).fill(fill).offset((i % 3) as usize));
        n += 1;
    }
    assert_eq!(n, 2_000, "c26 must perform 2000 randomized draws");
    eprintln!("c26_fuzz_large: {n} randomized differential draws");
}

#[test]
fn c27_exhaustive_small_size_sweep() {
    let mut rng = Rng::new(SEED ^ 27);
    for size in -4..=40 {
        let mut radii = vec![
            r_typical(&mut rng),
            r_typical(&mut rng),
            r_wide(&mut rng),
            r_narrow(&mut rng),
            r_neg(&mut rng),
            r_threshold(&mut rng, size),
        ];
        radii.extend(special_radii());
        for _ in 0..8 {
            radii.push(rng.any_f32());
        }
        for radius in radii {
            assert_same(&Case::new(size, radius));
        }
    }
}

#[test]
fn c28_clamp_boundary_exact() {
    // radius values for which |r * (sigma/radius)| is exactly the clamp
    // threshold, plus the two neighbouring representable floats.
    for size in [1i32, 2, 3, 4, 5, 6, 7, 8, 9] {
        let hsize = (size / 2).max(1);
        for r in 1..=hsize {
            let base = SIGMA * r as f32 / CLAMP_X;
            for delta in -4i32..=4 {
                let bits = (base.to_bits() as i64 + delta as i64) as u32;
                let radius = f32::from_bits(bits);
                assert_same(&Case::new(size, radius));
                assert_same(&Case::new(size, -radius));
            }
        }
    }
    // and the threshold approached through `rs` directly
    for size in [3i32, 5, 9, 17, 33] {
        let hsize = size / 2;
        for r in 1..=hsize {
            let rs = CLAMP_X / r as f32;
            let radius = SIGMA / rs;
            for delta in -2i32..=2 {
                let radius = f32::from_bits((radius.to_bits() as i64 + delta as i64) as u32);
                assert_same(&Case::new(size, radius));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// representative-size smoke sweep across every class at once
// ---------------------------------------------------------------------------

#[test]
fn c29_representative_size_times_special_radius_matrix() {
    let mut rng = Rng::new(SEED ^ 29);
    let radii = all_radius_classes(&mut rng);
    for size in representative_sizes() {
        for radius in &radii {
            assert_same(&Case::new(size, *radius));
        }
    }
}

// ---------------------------------------------------------------------------
// Audit helper: with `-- --test-threads=1 --nocapture` this runs last (libtest
// orders tests by name) and reports how many differential comparisons the whole
// binary performed, so the advertised coverage is verifiable rather than
// asserted in prose.
// ---------------------------------------------------------------------------

#[test]
fn zz_report_comparison_count() {
    let n = comparisons();
    eprintln!("[{}] differential comparisons so far: {n}", module_path!());
    assert!(n > 0, "no differential comparison was performed at all");
}
