//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH shared objects via `libloading` and compares the
//! *entire* scratch buffer (live regions + guard bands) bit-for-bit after each
//! call, over many seeded-random inputs.

mod common;

use common::*;

// --- C1 .. C4 : disjoint, `sum` a positive normal -------------------------

#[test]
fn cfg_c1_disjoint_uniform() {
    run_row("C1", 2000, Alias::Disjoint, Pop::Uniform, None);
}

#[test]
fn cfg_c2_disjoint_wide_exponent() {
    run_row("C2", 2000, Alias::Disjoint, Pop::WideExp, None);
}

#[test]
fn cfg_c3_disjoint_powers_of_two() {
    run_row("C3", 1000, Alias::Disjoint, Pop::PowersOfTwo, None);
}

/// `sum` is contrived to be **exactly** `1.0f`: the first `m` elements (with
/// `m` the largest power of four `<= n`) are `±1/sqrt(m)` — an exact power of
/// two — and the rest are `±0.0`. Then `scale == 1.0f` and `dest` must equal
/// `src` bit-for-bit, including the sign of any `-0.0`.
#[test]
fn cfg_c4_disjoint_sum_exactly_one() {
    run_row_with(
        "C4",
        500,
        Alias::Disjoint,
        None,
        |rng, dst| {
            let n = dst.len();
            // largest power of four <= n
            let mut m = 1usize;
            while m * 4 <= n {
                m *= 4;
            }
            let v = 1.0f32 / (m as f32).sqrt(); // exact: m is a power of four
            for (i, slot) in dst.iter_mut().enumerate() {
                *slot = if i < m {
                    rng.sign() * v
                } else if rng.next_u64() & 1 == 0 {
                    0.0
                } else {
                    -0.0
                };
            }
        },
        "sum==1.0 exactly",
    );
}

// --- C5 .. C8 : aliasing variants on the writing path ---------------------

#[test]
fn cfg_c5_inplace_normal() {
    run_row("C5", 2000, Alias::InPlace, Pop::WideExp, None);
}

#[test]
fn cfg_c6_overlap_forward_1() {
    run_row("C6", 2000, Alias::Delta(1), Pop::WideExp, None);
}

#[test]
fn cfg_c7_overlap_backward_1() {
    run_row("C7", 2000, Alias::Delta(-1), Pop::WideExp, None);
}

#[test]
fn cfg_c8_overlap_half() {
    run_row("C8", 2000, Alias::HalfOverlap, Pop::WideExp, None);
}

// --- C9 / C10 : degenerate sizes ------------------------------------------

#[test]
fn cfg_c9_size_one() {
    run_row("C9-uniform", 400, Alias::Disjoint, Pop::WideExp, Some(&[1]));
    run_row("C9-denormal", 400, Alias::Disjoint, Pop::Denormal, Some(&[1]));
    run_row("C9-bits", 400, Alias::Disjoint, Pop::RandomBits, Some(&[1]));
}

#[test]
fn cfg_c10_size_zero() {
    // memset of zero bytes: nothing at all may be written, and the sentinel
    // painted over the whole `dest` region must survive identically.
    run_row("C10-disjoint", 100, Alias::Disjoint, Pop::Uniform, Some(&[0]));
    run_row("C10-inplace", 100, Alias::InPlace, Pop::Uniform, Some(&[0]));
}

// --- C11 .. C14 : the `sum == +0.0` (zero-fill / no-write) path ------------

#[test]
fn cfg_c11_all_zeros_disjoint() {
    run_row("C11", 1000, Alias::Disjoint, Pop::Zeros, None);
}

#[test]
fn cfg_c12_all_zeros_inplace() {
    run_row("C12", 1000, Alias::InPlace, Pop::Zeros, None);
}

#[test]
fn cfg_c13_denormal_underflow_disjoint() {
    run_row("C13-denorm", 500, Alias::Disjoint, Pop::Denormal, None);
    run_row("C13-tiny", 500, Alias::Disjoint, Pop::TinyUnderflow, None);
}

#[test]
fn cfg_c14_denormal_underflow_inplace() {
    run_row("C14-denorm", 500, Alias::InPlace, Pop::Denormal, None);
    run_row("C14-tiny", 500, Alias::InPlace, Pop::TinyUnderflow, None);
}

// --- C15 / C16 : denormal `sum`, mixed magnitudes -------------------------

#[test]
fn cfg_c15_denormal_sum() {
    run_row("C15", 500, Alias::Disjoint, Pop::DenormalSum, None);
    run_row("C15-small", 500, Alias::Disjoint, Pop::DenormalSum, Some(&[1, 2, 3, 4]));
}

#[test]
fn cfg_c16_mixed_denormal_normal() {
    run_row("C16", 2000, Alias::Disjoint, Pop::MixedDenormalNormal, None);
}

// --- C17 .. C19 : `sum == +inf` -------------------------------------------

#[test]
fn cfg_c17_sum_overflow_inf() {
    run_row("C17", 1000, Alias::Disjoint, Pop::NearMax, None);
}

#[test]
fn cfg_c18_sum_overflow_inf_inplace() {
    run_row("C18", 1000, Alias::InPlace, Pop::NearMax, None);
}

#[test]
fn cfg_c19_inf_elements_disjoint() {
    run_row("C19", 1000, Alias::Disjoint, Pop::InfSprinkle, None);
    run_row("C19-inplace", 500, Alias::InPlace, Pop::InfSprinkle, None);
}

// --- C20 .. C22 : NaN, and the zero-fill path under aliasing ---------------

#[test]
fn cfg_c20_nan_elements_disjoint() {
    run_row("C20", 1000, Alias::Disjoint, Pop::NanSprinkle, None);
}

#[test]
fn cfg_c21_nan_elements_inplace() {
    run_row("C21", 1000, Alias::InPlace, Pop::NanSprinkle, None);
}

/// `sum == +0.0` while `dest != src` *and* the two regions overlap: the
/// `memset` scribbles over part of `src`. Both implementations must clear the
/// same byte range.
#[test]
fn cfg_c22_overlap_zero_fill() {
    run_row("C22-zeros-f", 500, Alias::Delta(1), Pop::Zeros, None);
    run_row("C22-zeros-b", 500, Alias::Delta(-1), Pop::Zeros, None);
    run_row("C22-denorm-f", 500, Alias::Delta(2), Pop::Denormal, None);
    run_row("C22-nan-h", 500, Alias::HalfOverlap, Pop::NanSprinkle, None);
}

// --- C23 .. C25 : unconstrained fuzz over every float class ---------------

#[test]
fn cfg_c23_disjoint_random_bits() {
    run_row("C23", 4000, Alias::Disjoint, Pop::RandomBits, None);
}

#[test]
fn cfg_c24_inplace_random_bits() {
    run_row("C24", 4000, Alias::InPlace, Pop::RandomBits, None);
}

#[test]
fn cfg_c25_overlap_random_bits() {
    run_row("C25", 4000, Alias::RandomDelta(-4, 4), Pop::RandomBits, None);
}

// --- C26 : long accumulation chain ---------------------------------------

#[test]
fn cfg_c26_large_size() {
    run_row("C26-uniform", 10, Alias::Disjoint, Pop::Uniform, Some(&[65536]));
    run_row("C26-wide", 10, Alias::Disjoint, Pop::WideExp, Some(&[65536]));
    run_row("C26-bits", 10, Alias::Disjoint, Pop::RandomBits, Some(&[65536]));
    run_row("C26-inplace", 10, Alias::InPlace, Pop::WideExp, Some(&[65536]));
}

// --- sanity: the two libraries really are two distinct .so files ----------

#[test]
fn harness_loads_two_distinct_shared_objects() {
    let l = libs();
    assert_ne!(l.c_path, l.r_path);
    assert!(l.c_path.to_string_lossy().contains("c_src/build"), "{:?}", l.c_path);
    assert!(l.r_path.to_string_lossy().ends_with("libnormalize_lib.so"), "{:?}", l.r_path);
    // both symbols resolve
    let _ = l.c();
    let _ = l.rust();
}
