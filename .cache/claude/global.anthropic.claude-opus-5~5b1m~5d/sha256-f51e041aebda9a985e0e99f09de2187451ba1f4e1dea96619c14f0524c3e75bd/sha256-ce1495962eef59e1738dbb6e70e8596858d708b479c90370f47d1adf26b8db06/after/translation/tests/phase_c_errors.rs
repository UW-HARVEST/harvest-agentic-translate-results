//! Phase C — error/rejection-path differential tests, one test per row of
//! `ERRORS.md`.
//!
//! `gaussian_kernel` returns `void` and validates nothing, so its entire
//! rejection surface is *implicit*: for the input classes below one of the two
//! branches degenerates and the function silently stores nothing, stores fewer
//! or more elements than `size` implies, or skips normalisation. Each test
//! therefore asserts (a) the C and the Rust `.so` produce byte-identical
//! buffers — the equivalent of "same error code" for a `void` API — and
//! (b) the exact degenerate outcome documented in `ERRORS.md`.

mod common;

use common::*;
use std::ffi::c_int;

const SEED: u64 = 0x5EED_C0FF_EE00_0002;

/// Radii for which `rs = sigma / radius` is finite and non-NaN, so the `r == 0`
/// tap is guaranteed to be `V0`.
fn finite_rs_radii() -> Vec<f32> {
    vec![
        1.0,
        -1.0,
        0.5,
        -0.5,
        2.0,
        1.6,
        3.7,
        100.0,
        1e6,
        1e-3,
        1e-6,
        f32::MAX,
        -f32::MAX,
        f32::MIN_POSITIVE,
        f32::INFINITY,
        f32::NEG_INFINITY,
    ]
}

/// Radii that make `rs` non-finite (`±inf` or NaN) ⇒ every tap clamps to `+0`.
fn nonfinite_rs_radii() -> Vec<f32> {
    let mut v = vec![
        0.0f32,
        -0.0,
        f32::NAN,
        -f32::NAN,
        f32::from_bits(0x7f80_0001), // sNaN
        f32::from_bits(0xff80_0001),
        f32::from_bits(0x7fff_ffff),
        f32::from_bits(0x0000_0001), // subnormal: sigma/x overflows to +inf
        f32::from_bits(0x0000_0002),
        f32::from_bits(0x8000_0001), // -> -inf
        f32::from_bits(0x0000_0100),
    ];
    // every subnormal below 1.6/FLT_MAX makes the division overflow
    v.retain(|r| {
        let rs = SIGMA / *r;
        !rs.is_finite()
    });
    v
}

// ---------------------------------------------------------------------------
// E1 / E2 — size 0 and -1: one store, normalisation loop runs zero times
// ---------------------------------------------------------------------------

#[test]
fn e01_size_zero_single_unnormalised_store() {
    for radius in finite_rs_radii() {
        for fill in [Fill::Sentinel, Fill::Zero, Fill::Random(1)] {
            for offset in [0usize, 1, 3, 7] {
                let case = Case::new(0, radius).fill(fill).offset(offset);
                let out = assert_same(&case);
                assert_eq!(
                    out[offset], V0_BITS,
                    "size=0 radius={radius}: dest[0] must be the raw, \
                     UNNORMALISED V0 (0x{V0_BITS:08x})"
                );
                if fill == Fill::Sentinel {
                    // exactly one store
                    assert!(out[..offset].iter().all(|&w| w == SENTINEL));
                    assert!(out[offset + 1..].iter().all(|&w| w == SENTINEL));
                }
            }
        }
    }
}

#[test]
fn e02_size_minus_one_single_unnormalised_store() {
    for radius in finite_rs_radii() {
        for fill in [Fill::Sentinel, Fill::Zero, Fill::Random(2)] {
            for offset in [0usize, 1, 3, 7] {
                let case = Case::new(-1, radius).fill(fill).offset(offset);
                let out = assert_same(&case);
                assert_eq!(
                    out[offset], V0_BITS,
                    "size=-1 (hsize = -1/2 = 0 by C truncation) must store \
                     exactly one raw V0"
                );
                if fill == Fill::Sentinel {
                    assert!(out[..offset].iter().all(|&w| w == SENTINEL));
                    assert!(out[offset + 1..].iter().all(|&w| w == SENTINEL));
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E3 / E4 / E5 — negative sizes: zero stores
// ---------------------------------------------------------------------------

fn assert_untouched(size: c_int, radius: f32) {
    for offset in [0usize, 1, 7] {
        let out = assert_same(&Case::new(size, radius).fill(Fill::Sentinel).offset(offset));
        assert!(
            out.iter().all(|&w| w == SENTINEL),
            "size={size} radius={radius}: the C never stores, buffer must be pristine"
        );
        // and with random contents: still pristine
        let out = assert_same(&Case::new(size, radius).fill(Fill::Random(0x1234_5678)));
        let mut rng = Rng::new(0x1234_5678);
        for w in &out {
            assert_eq!(*w, rng.next_u32(), "size={size}: buffer must be untouched");
        }
    }
}

#[test]
fn e03_size_minus_two_no_stores() {
    for radius in finite_rs_radii().into_iter().chain(nonfinite_rs_radii()) {
        assert_untouched(-2, radius);
    }
}

#[test]
fn e04_all_negative_sizes_no_stores() {
    let mut rng = Rng::new(SEED ^ 4);
    let mut sizes: Vec<c_int> = vec![-2, -3, -4, -5, -6, -100, -101, -12_345, -1_000_000];
    for _ in 0..12 {
        sizes.push(rng.range_i32(-2_000_000_000, -2));
    }
    for size in sizes {
        for radius in [1.0f32, 0.0, f32::NAN, f32::INFINITY, rng.any_f32()] {
            assert_untouched(size, radius);
        }
    }
}

#[test]
fn e05_int_min_size_no_stores_no_overflow() {
    for size in [i32::MIN, i32::MIN + 1, i32::MIN + 2, i32::MIN + 3] {
        for radius in [1.0f32, -1.0, 0.0, -0.0, f32::NAN, f32::INFINITY, 1e-45] {
            assert_untouched(size, radius);
        }
    }
}

// ---------------------------------------------------------------------------
// E6 — NULL dest, the only configuration in which the C never dereferences
// ---------------------------------------------------------------------------

#[test]
fn e06_null_dest_with_negative_size() {
    let mut rng = Rng::new(SEED ^ 6);
    let mut radii = finite_rs_radii();
    radii.extend(nonfinite_rs_radii());
    for _ in 0..8 {
        radii.push(rng.any_f32());
    }
    for size in [-2, -3, -4, -99, -100_000, i32::MIN, i32::MIN + 1] {
        for radius in &radii {
            // Must return without touching memory in BOTH implementations.
            assert_same(&Case::new(size, *radius).null_dest());
        }
    }
}

// ---------------------------------------------------------------------------
// E7 — even size: one store past the caller's `size`
// ---------------------------------------------------------------------------

#[test]
fn e07_even_size_writes_one_element_past() {
    let mut rng = Rng::new(SEED ^ 7);
    for size in (2..=64).step_by(2) {
        // radius = +inf ⇒ every raw tap is exactly V0, which makes the
        // "the last store is never normalised" bug directly observable.
        let out = assert_same(&Case::new(size, f32::INFINITY));
        assert_eq!(
            out[size as usize], V0_BITS,
            "size={size}: dest[size] holds the RAW, unnormalised tap"
        );
        assert_ne!(
            out[0], V0_BITS,
            "size={size}: dest[0..size] are normalised, so they differ from V0"
        );
        assert!(
            out[size as usize + 1..].iter().all(|&w| w == SENTINEL),
            "size={size}: exactly one element past `size` is written"
        );

        // ... and for arbitrary radii the store count is still size+1
        for _ in 0..8 {
            let radius = rng.range_f32(0.3, 6.0);
            let out = assert_same(&Case::new(size, radius));
            assert_ne!(out[size as usize], SENTINEL);
            assert!(out[size as usize + 1..].iter().all(|&w| w == SENTINEL));
        }
    }
}

// ---------------------------------------------------------------------------
// E8 .. E11 — radius classes that make `rs` non-finite ⇒ sum == 0 ⇒ no
//             normalisation, and every stored element is +0.0f
// ---------------------------------------------------------------------------

fn assert_all_plus_zero(radius: f32, label: &str) {
    for size in [0i32, 1, 2, 3, 4, 5, 8, 9, 16, 17, 33, 64, 65, 128, 129] {
        for offset in [0usize, 1, 3] {
            let out = assert_same(&Case::new(size, radius).offset(offset));
            let stores = (2 * (size / 2) + 1) as usize;
            for i in 0..stores {
                assert_eq!(
                    out[offset + i],
                    0u32,
                    "{label}: size={size} store {i} must be +0.0f (bit pattern \
                     0x00000000), got 0x{:08x}",
                    out[offset + i]
                );
            }
            assert!(
                out[offset + stores..].iter().all(|&w| w == SENTINEL),
                "{label}: size={size} must store exactly {stores} elements"
            );
        }
    }
}

#[test]
fn e08_radius_positive_zero() {
    assert_all_plus_zero(0.0, "radius=+0.0 (rs=+inf)");
}

#[test]
fn e09_radius_negative_zero() {
    assert_all_plus_zero(-0.0, "radius=-0.0 (rs=-inf)");
}

#[test]
fn e10_radius_nan_every_payload() {
    for bits in [
        0x7fc0_0000u32,
        0xffc0_0000,
        0x7f80_0001,
        0xff80_0001,
        0x7fbf_ffff,
        0x7fff_ffff,
        0xffff_ffff,
        0x7fc0_dead,
        0xffca_fe00,
        0x7fea_1234,
    ] {
        let radius = f32::from_bits(bits);
        assert!(radius.is_nan());
        assert_all_plus_zero(radius, "radius=NaN");
    }
}

#[test]
fn e11_radius_subnormal_division_overflows() {
    for radius in nonfinite_rs_radii() {
        if radius == 0.0 || radius.is_nan() {
            continue;
        }
        assert!(
            !(SIGMA / radius).is_finite(),
            "radius={radius:e} should make sigma/radius overflow"
        );
        assert_all_plus_zero(radius, "radius=subnormal (rs overflows)");
    }
    // the largest subnormal for which the division still overflows, and the
    // first normal for which it does not — the exact boundary
    let boundary = SIGMA / f32::MAX; // smallest radius with finite rs
    for delta in -4i32..=4 {
        let radius = f32::from_bits((boundary.to_bits() as i64 + delta as i64) as u32);
        for size in [0i32, 1, 2, 3, 9, 17] {
            assert_same(&Case::new(size, radius));
            assert_same(&Case::new(size, -radius));
        }
    }
}

// ---------------------------------------------------------------------------
// E12 / E13 — ±inf radius ⇒ rs == ±0 ⇒ every tap is V0, normalisation runs
// ---------------------------------------------------------------------------

fn assert_flat_kernel(radius: f32, label: &str) {
    for size in [0i32, 1, 2, 3, 4, 5, 8, 9, 16, 17, 64, 65] {
        let out = assert_same(&Case::new(size, radius));
        let stores = (2 * (size / 2) + 1) as usize;
        // all in-range elements are equal to each other
        for i in 1..(size.max(0) as usize) {
            assert_eq!(
                out[i], out[0],
                "{label}: size={size} kernel must be flat at index {i}"
            );
        }
        if size <= 0 {
            // no normalisation at all
            assert_eq!(out[0], V0_BITS, "{label}: size={size} must be raw V0");
        } else if size % 2 == 0 {
            // last store is the unnormalised one
            assert_eq!(
                out[stores - 1], V0_BITS,
                "{label}: size={size} tail store must be raw V0"
            );
        }
        assert!(out[stores..].iter().all(|&w| w == SENTINEL));
    }
}

#[test]
fn e12_radius_negative_infinity() {
    assert_flat_kernel(f32::NEG_INFINITY, "radius=-inf (rs=-0.0)");
}

#[test]
fn e13_radius_positive_infinity() {
    assert_flat_kernel(f32::INFINITY, "radius=+inf (rs=+0.0)");
}

// ---------------------------------------------------------------------------
// E14 / E15 — the two extreme clamp regimes
// ---------------------------------------------------------------------------

#[test]
fn e14_clamp_never_taken() {
    let mut rng = Rng::new(SEED ^ 14);
    for size in [1i32, 2, 3, 4, 8, 9, 16, 17] {
        for _ in 0..32 {
            // |hsize * rs| < 2.4 for every tap
            let radius = rng.log_range_f32(1e3, 1e7);
            let out = assert_same(&Case::new(size, radius));
            for i in 0..size as usize {
                assert!(
                    f32::from_bits(out[i]) > 0.0,
                    "size={size} radius={radius:e}: no tap may be clamped"
                );
            }
        }
    }
}

#[test]
fn e15_clamp_taken_for_every_off_centre_tap() {
    let mut rng = Rng::new(SEED ^ 15);
    for size in [3i32, 4, 5, 8, 9, 17, 32, 33] {
        let hsize = (size / 2) as usize;
        for _ in 0..32 {
            // rs finite but huge ⇒ every r != 0 clamps, sum == V0 > 0
            let radius = rng.log_range_f32(1e-6, 1e-2);
            let out = assert_same(&Case::new(size, radius));
            assert_eq!(
                out[hsize], 0x3f80_0000,
                "size={size} radius={radius:e}: centre tap must be exactly 1.0f"
            );
            for i in 0..size as usize {
                if i != hsize {
                    assert_eq!(out[i], 0, "size={size}: out[{i}] must be +0.0f");
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E16 / E17 — the size==1 / size==2 boundary
// ---------------------------------------------------------------------------

#[test]
fn e16_size_one_normalises_to_exactly_one() {
    for radius in finite_rs_radii() {
        let out = assert_same(&Case::new(1, radius));
        assert_eq!(
            out[0], 0x3f80_0000,
            "size=1 radius={radius:e}: must be exactly 1.0f"
        );
        assert!(out[1..].iter().all(|&w| w == SENTINEL));
    }
}

#[test]
fn e17_size_two_off_by_one_boundary() {
    for radius in finite_rs_radii() {
        let out = assert_same(&Case::new(2, radius));
        // three stores for a two-element request
        assert_ne!(out[2], SENTINEL, "size=2 stores dest[0..=2]");
        assert!(out[3..].iter().all(|&w| w == SENTINEL));
        // taps 0 and 1 are symmetric (r = -1 and r = 0) — NOT equal in general,
        // but both normalised; tap 2 (r = +1) is raw and equals tap 0's raw
        // value, so with radius = +/-inf they are all V0 before scaling.
        assert!(f32::from_bits(out[0]) >= 0.0);
        assert!(f32::from_bits(out[1]) > 0.0);
    }
    let out = assert_same(&Case::new(2, f32::INFINITY));
    assert_eq!(out[2], V0_BITS);
    assert_eq!(out[0], out[1]);
}

// ---------------------------------------------------------------------------
// E18 — full-domain fuzz of the `radius` bit pattern crossed with the
//        degenerate `size` values (the "out-of-range enum value" analogue:
//        every one of the 2^32 float patterns is a legal C argument)
// ---------------------------------------------------------------------------

#[test]
fn e18_fuzz_radius_bit_patterns_times_degenerate_sizes() {
    let mut rng = Rng::new(SEED ^ 18);
    let degenerate: &[c_int] = &[i32::MIN, -100_000, -3, -2, -1, 0, 1, 2, 3];
    let mut n = 0u32;
    for i in 0..30_000u32 {
        let size = degenerate[(i as usize) % degenerate.len()];
        let radius = rng.any_f32();
        let fill = match i % 3 {
            0 => Fill::Sentinel,
            1 => Fill::Zero,
            _ => Fill::Random(i as u64),
        };
        let mut case = Case::new(size, radius).fill(fill).offset((i % 4) as usize);
        if size / 2 < 0 && i % 5 == 0 {
            case = case.null_dest();
        }
        let out = assert_same(&case);
        // invariant: no store may ever be a NaN or an infinity
        if size / 2 >= 0 && !case.null_dest {
            let stores = (2 * (size / 2) + 1) as usize;
            let off = case.offset;
            for j in 0..stores {
                let v = f32::from_bits(out[off + j]);
                assert!(
                    v.is_finite(),
                    "size={size} radius bits 0x{:08x}: stored non-finite {v}",
                    radius.to_bits()
                );
            }
        }
        n += 1;
    }
    assert_eq!(n, 30_000, "e18 must perform 30000 randomized draws");
    eprintln!("e18_fuzz_radius_bit_patterns: {n} randomized differential draws");
}

// ---------------------------------------------------------------------------
// Generic C-API boundaries required by Phase C beyond the table
// ---------------------------------------------------------------------------

#[test]
fn e19_oversized_lengths() {
    // "oversized" length: far beyond anything a caller would sensibly use, but
    // still allocatable. (INT_MAX itself would need an 8 GiB buffer and is not
    // testable; there is no additional branch there — the loop bound is the
    // only thing that changes.)
    for size in [65_535i32, 65_536, 262_143, 262_144, 1_048_575, 1_048_576] {
        for radius in [3.0f32, 0.0, f32::NAN, f32::INFINITY, 1e-30] {
            assert_same(&Case::new(size, radius));
        }
    }
}

#[test]
fn e20_one_step_past_every_documented_size_boundary() {
    // Every place where `size / 2` changes behaviour, plus one step either side.
    for size in [-4, -3, -2, -1, 0, 1, 2, 3, 4, 5] {
        for radius in finite_rs_radii().into_iter().chain(nonfinite_rs_radii()) {
            for fill in [Fill::Sentinel, Fill::Zero, Fill::Random(9)] {
                for offset in [0usize, 1, 2, 3] {
                    assert_same(&Case::new(size, radius).fill(fill).offset(offset));
                }
            }
        }
    }
}

#[test]
fn e21_zero_length_with_every_radius_class() {
    let mut rng = Rng::new(SEED ^ 21);
    let mut radii = finite_rs_radii();
    radii.extend(nonfinite_rs_radii());
    for _ in 0..64 {
        radii.push(rng.any_f32());
    }
    for radius in radii {
        for fill in [Fill::Sentinel, Fill::Zero, Fill::Random(13)] {
            assert_same(&Case::new(0, radius).fill(fill));
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
