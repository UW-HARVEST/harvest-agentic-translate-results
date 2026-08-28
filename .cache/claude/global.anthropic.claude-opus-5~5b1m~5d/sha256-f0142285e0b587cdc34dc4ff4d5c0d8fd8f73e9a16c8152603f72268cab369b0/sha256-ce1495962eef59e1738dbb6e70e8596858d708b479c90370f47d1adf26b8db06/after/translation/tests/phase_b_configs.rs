//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test calls BOTH the C `.so` and the Rust `.so` through their exported
//! `ldexp_q2` symbol (loaded with `libloading`) and asserts the returned `f32`
//! is **bit-identical**. Inputs are randomized from a fixed-seed SplitMix64
//! PRNG so results are reproducible.

mod common;

use common::*;

/// Cases per randomized row.
const N: usize = 512;

// ---------------------------------------------------------------------------
// C1 — identity: exp_q2 == 0, e == 0, residue 0, k == 0, scale 2^30.
// frac[0] * 2^30 == 1.0f exactly, so the result must be `y` bit-for-bit.
// ---------------------------------------------------------------------------
#[test]
fn c1_identity_random_normals() {
    let mut rng = Rng::new(0x0000_0001);
    let cases = (0..N).map(|_| (rng.next_normal_f32(), EXP_IDENTITY));
    check_all("C1 identity (exp_q2=0) x random normals", cases);
}

// ---------------------------------------------------------------------------
// C2..C5 — positive e in (0,120), one trip, each residue class e & 3.
// Sweeps shift counts k = e>>2 across 1..29.
// ---------------------------------------------------------------------------
fn positive_residue_row(residue: i32, seed: u64, label: &str) {
    let mut rng = Rng::new(seed);
    // e values in (0,120) with the requested residue, i.e. 4*q + residue.
    let exps: Vec<i32> = (0..30)
        .map(|q| 4 * q + residue)
        .filter(|&e| e > 0 && e < 120)
        .collect();
    assert!(!exps.is_empty());
    let mut cases = Vec::new();
    for &e in &exps {
        for _ in 0..(N / exps.len() + 4) {
            cases.push((rng.next_normal_f32(), e));
        }
        // plus every special y at this exponent
        for y in special_ys() {
            cases.push((y, e));
        }
    }
    check_all(label, cases);
}

#[test]
fn c2_pos_residue0() {
    positive_residue_row(0, 0x0000_0002, "C2 positive e, residue 0");
}

#[test]
fn c3_pos_residue1() {
    positive_residue_row(1, 0x0000_0003, "C3 positive e, residue 1");
}

#[test]
fn c4_pos_residue2() {
    positive_residue_row(2, 0x0000_0004, "C4 positive e, residue 2");
}

#[test]
fn c5_pos_residue3() {
    positive_residue_row(3, 0x0000_0005, "C5 positive e, residue 3");
}

// ---------------------------------------------------------------------------
// C6 — exp_q2 == 120: clamp boundary hit exactly, k == 30 so scale == 1
// (the `(1<<30)>>30` corner), exactly one trip.
// ---------------------------------------------------------------------------
#[test]
fn c6_scale_one_k30() {
    let mut rng = Rng::new(0x0000_0006);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for _ in 0..N {
        cases.push((rng.next_normal_f32(), EXP_SCALE_ONE_POS));
        cases.push((rng.next_f32_bits(), EXP_SCALE_ONE_POS));
    }
    for y in special_ys() {
        cases.push((y, EXP_SCALE_ONE_POS));
    }
    check_all("C6 exp_q2=120 (clamp boundary, scale=1)", cases);
}

// ---------------------------------------------------------------------------
// C7 — exactly 2 trips (exp_q2 in 121..=240).
// ---------------------------------------------------------------------------
#[test]
fn c7_two_trips() {
    let mut rng = Rng::new(0x0000_0007);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for _ in 0..N {
        cases.push((rng.next_normal_f32(), rng.range_i32(121, 240)));
    }
    // every exp in the 2-trip window against a fixed set of specials
    for e in 121..=240 {
        for y in special_ys() {
            cases.push((y, e));
        }
    }
    check_all("C7 two trips (exp_q2 121..=240)", cases);
}

// ---------------------------------------------------------------------------
// C8 — exactly 3 trips (exp_q2 in 241..=360).
// ---------------------------------------------------------------------------
#[test]
fn c8_three_trips() {
    let mut rng = Rng::new(0x0000_0008);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for _ in 0..N {
        cases.push((rng.next_normal_f32(), rng.range_i32(241, 360)));
    }
    for e in 241..=360 {
        for y in special_ys() {
            cases.push((y, e));
        }
    }
    check_all("C8 three trips (exp_q2 241..=360)", cases);
}

// ---------------------------------------------------------------------------
// C9 — many trips (exp_q2 in 361..=12000): accumulated rounding of repeated
// 2^-30 products, including flush-to-zero part-way through the loop.
// ---------------------------------------------------------------------------
#[test]
fn c9_many_trips() {
    let mut rng = Rng::new(0x0000_0009);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for _ in 0..(N * 4) {
        cases.push((rng.next_normal_f32(), rng.range_i32(361, 12000)));
    }
    for &e in EXP_MULTITRIP {
        for y in special_ys() {
            cases.push((y, e));
        }
    }
    check_all("C9 many trips (exp_q2 361..=12000)", cases);
}

// ---------------------------------------------------------------------------
// C10 — maximum trip count. exp_q2 == INT_MAX runs the do/while 17,895,698
// times. Kept to a small number of y values because each call is ~130 ms
// across both implementations.
// ---------------------------------------------------------------------------
#[test]
fn c10_max_trips() {
    let big = [
        i32::MAX,
        i32::MAX - 1,
        2_147_483_640,
        17_895_698i64.saturating_mul(120).min(i32::MAX as i64) as i32,
    ];
    let ys = [
        1.0f32,
        -1.0f32,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::from_bits(0x7FC0_1234),
        0.0f32,
        -0.0f32,
        f32::MAX,
    ];
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for &e in &big {
        for &y in &ys {
            cases.push((y, e));
        }
    }
    check_all("C10 maximum trip count (INT_MAX neighbourhood)", cases);
}

// ---------------------------------------------------------------------------
// C11 — negative e, k == 31 => scale == 0 (annihilates y). exp_q2 -1..-4
// covers all four residues. This is the UB-masked-shift path.
// ---------------------------------------------------------------------------
#[test]
fn c11_neg_scale_zero_all_residues() {
    let mut rng = Rng::new(0x0000_000B);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for &e in EXP_SCALE_ZERO {
        for _ in 0..N {
            cases.push((rng.next_normal_f32(), e));
            cases.push((rng.next_f32_bits(), e));
        }
        for y in special_ys() {
            cases.push((y, e));
        }
    }
    check_all("C11 negative e, scale=0 (k=31), all residues", cases);
}

// ---------------------------------------------------------------------------
// C12 — negative e, k == 30 => scale == 1. exp_q2 -5..-8, all four residues.
// ---------------------------------------------------------------------------
#[test]
fn c12_neg_scale_one_all_residues() {
    let mut rng = Rng::new(0x0000_000C);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for &e in EXP_SCALE_ONE_NEG {
        for _ in 0..N {
            cases.push((rng.next_normal_f32(), e));
            cases.push((rng.next_f32_bits(), e));
        }
        for y in special_ys() {
            cases.push((y, e));
        }
    }
    check_all("C12 negative e, scale=1 (k=30), all residues", cases);
}

// ---------------------------------------------------------------------------
// C13 — negative e on the period-128 lattice (e % 128 == 0) => k == 0 =>
// scale 2^30 => identity even though exp_q2 is negative.
// ---------------------------------------------------------------------------
#[test]
fn c13_neg_identity_period128() {
    let mut rng = Rng::new(0x0000_000D);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for &e in EXP_NEG_IDENTITY {
        for _ in 0..N {
            cases.push((rng.next_normal_f32(), e));
            cases.push((rng.next_f32_bits(), e));
        }
        for y in special_ys() {
            cases.push((y, e));
        }
    }
    // the whole negative lattice within reach, mechanically generated
    for q in 1..=64 {
        let e = -128 * q;
        for y in special_ys() {
            cases.push((y, e));
        }
    }
    check_all("C13 negative e identity lattice (e % 128 == 0)", cases);
}

// ---------------------------------------------------------------------------
// C14 — negative e reaching every intermediate shift count k in 1..=29,
// across all four residues (exp_q2 in -124..=-9).
// ---------------------------------------------------------------------------
#[test]
fn c14_neg_all_shift_counts() {
    let mut rng = Rng::new(0x0000_000E);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for e in -124..=-9 {
        for _ in 0..8 {
            cases.push((rng.next_normal_f32(), e));
            cases.push((rng.next_f32_bits(), e));
        }
        for y in special_ys() {
            cases.push((y, e));
        }
    }
    check_all("C14 negative e, all shift counts k=1..29", cases);
}

// ---------------------------------------------------------------------------
// C15 — exhaustive sweep of exp_q2 in -1000..=1000 (>15 full 128-periods,
// crosses 0 and the 120 clamp) x random y.
// ---------------------------------------------------------------------------
#[test]
fn c15_exhaustive_small_exp_random_y() {
    let mut rng = Rng::new(0x0000_000F);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for e in -1000..=1000 {
        for _ in 0..4 {
            cases.push((rng.next_normal_f32(), e));
        }
        cases.push((rng.next_f32_bits(), e));
        cases.push((rng.next_subnormal_f32(), e));
    }
    check_all("C15 exhaustive exp_q2 -1000..=1000 x random y", cases);
}

// ---------------------------------------------------------------------------
// C16 — signed zeros against every scale regime.
// ---------------------------------------------------------------------------
#[test]
fn c16_signed_zeros_all_scales() {
    let mut cases: Vec<(f32, i32)> = Vec::new();
    let zeros = [0.0f32, -0.0f32];
    let mut exps: Vec<i32> = vec![EXP_IDENTITY, EXP_SCALE_ONE_POS];
    exps.extend_from_slice(EXP_SCALE_ZERO);
    exps.extend_from_slice(EXP_SCALE_ONE_NEG);
    exps.extend_from_slice(EXP_NEG_IDENTITY);
    exps.extend_from_slice(EXP_MULTITRIP);
    exps.extend(1..=120);
    exps.extend(-140..=-1);
    for &y in &zeros {
        for &e in &exps {
            cases.push((y, e));
        }
    }
    check_all("C16 signed zeros x all scale regimes", cases);
}

// ---------------------------------------------------------------------------
// C17 — infinities. Scale > 0 keeps inf; scale == 0 makes inf*0 => NaN
// (IEEE invalid operation) and the NaN must match bit-for-bit.
// ---------------------------------------------------------------------------
#[test]
fn c17_infinities_all_scales() {
    let mut cases: Vec<(f32, i32)> = Vec::new();
    let infs = [f32::INFINITY, f32::NEG_INFINITY];
    let mut exps: Vec<i32> = vec![EXP_IDENTITY, EXP_SCALE_ONE_POS];
    exps.extend_from_slice(EXP_SCALE_ZERO);
    exps.extend_from_slice(EXP_SCALE_ONE_NEG);
    exps.extend_from_slice(EXP_NEG_IDENTITY);
    exps.extend_from_slice(EXP_MULTITRIP);
    exps.extend(1..=120);
    exps.extend(-140..=-1);
    for &y in &infs {
        for &e in &exps {
            cases.push((y, e));
        }
    }
    check_all("C17 +/-inf x all scale regimes (incl. inf*0 -> NaN)", cases);
}

// ---------------------------------------------------------------------------
// C18 — quiet NaN payload preservation across the FFI boundary.
// ---------------------------------------------------------------------------
#[test]
fn c18_qnan_payloads_all_scales() {
    let mut cases: Vec<(f32, i32)> = Vec::new();
    let mut exps: Vec<i32> = vec![EXP_IDENTITY, EXP_SCALE_ONE_POS];
    exps.extend_from_slice(EXP_SCALE_ZERO);
    exps.extend_from_slice(EXP_SCALE_ONE_NEG);
    exps.extend_from_slice(EXP_NEG_IDENTITY);
    exps.extend_from_slice(EXP_MULTITRIP);
    exps.extend(1..=120);
    exps.extend(-140..=-1);
    for &bits in NAN_Y_BITS {
        for &e in &exps {
            cases.push((f32::from_bits(bits), e));
        }
    }
    check_all("C18 qNaN payloads x all scale regimes", cases);
}

// ---------------------------------------------------------------------------
// C19 — signalling NaN quieting across the FFI boundary.
// ---------------------------------------------------------------------------
#[test]
fn c19_snan_all_scales() {
    let mut cases: Vec<(f32, i32)> = Vec::new();
    let mut exps: Vec<i32> = vec![EXP_IDENTITY, EXP_SCALE_ONE_POS];
    exps.extend_from_slice(EXP_SCALE_ZERO);
    exps.extend_from_slice(EXP_SCALE_ONE_NEG);
    exps.extend_from_slice(EXP_NEG_IDENTITY);
    exps.extend_from_slice(EXP_MULTITRIP);
    exps.extend(1..=120);
    exps.extend(-140..=-1);
    for &bits in SNAN_Y_BITS {
        for &e in &exps {
            cases.push((f32::from_bits(bits), e));
        }
    }
    check_all("C19 sNaN x all scale regimes", cases);
}

// ---------------------------------------------------------------------------
// C20 — subnormal y against every scale regime (gradual underflow to zero).
// ---------------------------------------------------------------------------
#[test]
fn c20_subnormals_all_scales() {
    let mut rng = Rng::new(0x0000_0014);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    let fixed = [
        f32::from_bits(0x0000_0001),
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x007F_FFFF),
        f32::from_bits(0x807F_FFFF),
        f32::from_bits(0x0040_0000),
    ];
    let mut exps: Vec<i32> = vec![EXP_IDENTITY, EXP_SCALE_ONE_POS];
    exps.extend_from_slice(EXP_SCALE_ZERO);
    exps.extend_from_slice(EXP_SCALE_ONE_NEG);
    exps.extend_from_slice(EXP_NEG_IDENTITY);
    exps.extend_from_slice(EXP_MULTITRIP);
    exps.extend(1..=120);
    exps.extend(-140..=-1);
    for &y in &fixed {
        for &e in &exps {
            cases.push((y, e));
        }
    }
    for _ in 0..(N * 2) {
        cases.push((rng.next_subnormal_f32(), rng.range_i32(-1000, 1000)));
    }
    check_all("C20 subnormal y x all scale regimes", cases);
}

// ---------------------------------------------------------------------------
// C21 — extreme finite y (+/-FLT_MAX, +/-FLT_MIN) against every scale regime.
// ---------------------------------------------------------------------------
#[test]
fn c21_extreme_finite_all_scales() {
    let mut cases: Vec<(f32, i32)> = Vec::new();
    let fixed = [
        f32::MAX,
        f32::MIN,
        f32::MIN_POSITIVE,
        -f32::MIN_POSITIVE,
        f32::from_bits(0x7F7F_FFFF),
        f32::from_bits(0xFF7F_FFFF),
    ];
    let mut exps: Vec<i32> = vec![EXP_IDENTITY, EXP_SCALE_ONE_POS];
    exps.extend_from_slice(EXP_SCALE_ZERO);
    exps.extend_from_slice(EXP_SCALE_ONE_NEG);
    exps.extend_from_slice(EXP_NEG_IDENTITY);
    exps.extend_from_slice(EXP_MULTITRIP);
    exps.extend(1..=120);
    exps.extend(-140..=-1);
    for &y in &fixed {
        for &e in &exps {
            cases.push((y, e));
        }
    }
    check_all("C21 extreme finite y x all scale regimes", cases);
}

// ---------------------------------------------------------------------------
// C22 — broadest property test: uniformly random raw 32-bit patterns as y
// (hits every IEEE class incl. sNaN) x random exp_q2 in -4096..=4096.
// ---------------------------------------------------------------------------
#[test]
fn c22_random_bitpatterns() {
    let mut rng = Rng::new(0x0000_0016);
    let cases = (0..(N * 40)).map(|_| {
        let y = rng.next_f32_bits();
        let e = rng.range_i32(-4096, 4096);
        (y, e)
    });
    check_all("C22 random f32 bit patterns x exp_q2 -4096..=4096", cases);
}

// ---------------------------------------------------------------------------
// C23 — exp_q2 over the FULL int32 range, stratified so trip counts stay
// bounded: all negatives are single-trip and therefore free; positives are
// sampled from bounded windows plus a few genuinely huge values.
// ---------------------------------------------------------------------------
#[test]
fn c23_full_int32_stratified() {
    let mut rng = Rng::new(0x0000_0017);
    let mut cases: Vec<(f32, i32)> = Vec::new();

    // Stratum 1: any negative int32 (always exactly one trip -> cheap).
    for _ in 0..(N * 20) {
        let e = -(rng.next_u32() as i64 % (i32::MAX as i64 + 1)) as i32;
        cases.push((rng.next_f32_bits(), e));
    }
    // Stratum 2: i32::MIN exactly and its neighbourhood.
    for d in 0..64 {
        cases.push((rng.next_f32_bits(), i32::MIN + d));
        cases.push((rng.next_normal_f32(), i32::MIN + d));
    }
    // Stratum 3: the single-trip positive window [0, 120].
    for _ in 0..(N * 4) {
        cases.push((rng.next_f32_bits(), rng.range_i32(0, 120)));
    }
    // Stratum 4: bounded multi-trip positives (<= ~8334 trips).
    for _ in 0..(N * 4) {
        cases.push((rng.next_f32_bits(), rng.range_i32(121, 1_000_000)));
    }
    // Stratum 5: a few very large positives (expensive, so only a handful).
    for &e in &[
        1_000_000_001,
        1_500_000_000,
        2_000_000_000,
        i32::MAX - 119,
        i32::MAX - 120,
    ] {
        cases.push((rng.next_normal_f32(), e));
        cases.push((f32::INFINITY, e));
    }
    check_all("C23 full int32 exp_q2, stratified", cases);
}

// ---------------------------------------------------------------------------
// C24 — rounding-sensitive: residue != 0 (so frac is not an exact power of
// two) and mid-range k, with full 24-bit random mantissas in y. Catches any
// reassociation, double rounding, or fma contraction difference.
// ---------------------------------------------------------------------------
#[test]
fn c24_rounding_sensitive() {
    let mut rng = Rng::new(0x0000_0018);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    // e with residue 1..3 and k in 5..25 => inexact frac * inexact-ish scale.
    for residue in 1..=3i32 {
        for q in 5..=25i32 {
            let e = 4 * q + residue;
            for _ in 0..48 {
                cases.push((rng.next_midrange_f32(), e));
            }
        }
    }
    // negative counterparts (UB-shift path) with non-zero residues
    for e in -127..=-1 {
        if e % 4 != 0 {
            for _ in 0..16 {
                cases.push((rng.next_midrange_f32(), e));
            }
        }
    }
    check_all("C24 rounding-sensitive mantissas x inexact scales", cases);
}

// ---------------------------------------------------------------------------
// C25 — multi-trip walk across the underflow boundary: y is chosen near the
// smallest normals so the loop crosses normal -> subnormal -> zero *between*
// trips, which is sensitive to the per-trip order of operations.
// ---------------------------------------------------------------------------
#[test]
fn c25_multitrip_underflow_walk() {
    let mut rng = Rng::new(0x0000_0019);
    let mut cases: Vec<(f32, i32)> = Vec::new();
    // Exponents that make the total scaling land right around the f32
    // underflow threshold, in both single- and multi-trip regimes.
    for e in 100..=700 {
        for _ in 0..4 {
            // biased exponent 1..=40 => tiny normals and subnormals
            let r = rng.next_u64();
            let sign = ((r >> 63) as u32) << 31;
            let exp = (((r >> 32) as u32) % 40 + 1) << 23;
            let mant = (r as u32) & 0x007F_FFFF;
            cases.push((f32::from_bits(sign | exp | mant), e));
        }
    }
    // and the same near the overflow end (scales are all <= 1 so this must
    // never produce inf, but verify against C rather than assuming)
    for e in 100..=700 {
        for _ in 0..2 {
            let r = rng.next_u64();
            let sign = ((r >> 63) as u32) << 31;
            let exp = (((r >> 32) as u32) % 40 + 214) << 23;
            let mant = (r as u32) & 0x007F_FFFF;
            cases.push((f32::from_bits(sign | exp | mant), e));
        }
    }
    check_all("C25 multi-trip underflow/overflow boundary walk", cases);
}

// ---------------------------------------------------------------------------
// C26 — boundary lattice: clamp +/-1 and trip-count transitions, crossed with
// every special y value.
// ---------------------------------------------------------------------------
#[test]
fn c26_boundary_lattice_specials() {
    let lattice = [
        -5, -4, -3, -2, -1, 0, 1, 2, 3, 4, 5, 118, 119, 120, 121, 122, 238, 239, 240, 241, 242,
        359, 360, 361, 362,
    ];
    let mut cases: Vec<(f32, i32)> = Vec::new();
    for &e in &lattice {
        for y in special_ys() {
            cases.push((y, e));
        }
    }
    check_all("C26 boundary lattice x all special y", cases);
}

// ---------------------------------------------------------------------------
// C27 — int32 domain extremes: the INT_MIN and INT_MAX neighbourhoods,
// including the `exp_q2 -= e` corner where INT_MIN - INT_MIN == 0 (no signed
// overflow) and the maximum trip count.
// ---------------------------------------------------------------------------
#[test]
fn c27_int_extremes_neighbourhood() {
    let mut cases: Vec<(f32, i32)> = Vec::new();
    // INT_MIN side: single trip, cheap -> use every special y.
    for d in 0..=8 {
        for y in special_ys() {
            cases.push((y, i32::MIN + d));
        }
    }
    // INT_MAX side: ~17.9M trips per call -> only a few y values.
    let ys = [1.0f32, -1.0f32, 0.0f32, f32::INFINITY];
    for d in 0..=4 {
        for &y in &ys {
            cases.push((y, i32::MAX - d));
        }
    }
    check_all("C27 int32 extremes (INT_MIN / INT_MAX neighbourhoods)", cases);
}

// ---------------------------------------------------------------------------
// C28 — ABI / statelessness stress: interleave C and Rust calls in a tight
// loop and re-check previously-seen inputs, so any register clobbering or
// leaked state between calls would show up as a divergence.
// ---------------------------------------------------------------------------
#[test]
fn c28_interleaved_abi_stress() {
    let im = impls();
    let mut rng = Rng::new(0x0000_001C);
    let mut history: Vec<(f32, i32, u32)> = Vec::new();

    for i in 0..8000 {
        let y = if i % 3 == 0 {
            rng.next_f32_bits()
        } else {
            rng.next_normal_f32()
        };
        let e = rng.range_i32(-2000, 2000);

        // Alternate which implementation is invoked first.
        let (cv, rv) = if i % 2 == 0 {
            let a = im.c(y, e);
            let b = im.rust(y, e);
            (a, b)
        } else {
            let b = im.rust(y, e);
            let a = im.c(y, e);
            (a, b)
        };
        assert_eq!(
            cv.to_bits(),
            rv.to_bits(),
            "C28 divergence at i={i}: y=0x{:08x} exp_q2={e}: C=0x{:08x} Rust=0x{:08x}",
            y.to_bits(),
            cv.to_bits(),
            rv.to_bits()
        );
        history.push((y, e, cv.to_bits()));
    }

    // Replay every input in reverse: results must be identical (pure function,
    // no mutable `static` state in either implementation).
    for &(y, e, expected) in history.iter().rev() {
        assert_eq!(
            im.c(y, e).to_bits(),
            expected,
            "C replay differs for y=0x{:08x} exp_q2={e}",
            y.to_bits()
        );
        assert_eq!(
            im.rust(y, e).to_bits(),
            expected,
            "Rust replay differs for y=0x{:08x} exp_q2={e}",
            y.to_bits()
        );
    }
    eprintln!("C28: {} interleaved + replayed cases OK", history.len());
}

// ---------------------------------------------------------------------------
// Sanity: confirm both libraries really were loaded from distinct files.
// ---------------------------------------------------------------------------
#[test]
fn c00_harness_loads_two_distinct_shared_objects() {
    let im = impls();
    eprintln!("C   .so: {}", im.c_path.display());
    eprintln!("Rust.so: {}", im.rust_path.display());
    assert_ne!(
        im.c_path.canonicalize().unwrap(),
        im.rust_path.canonicalize().unwrap(),
        "harness must load two different shared objects"
    );
    // Both must actually respond.
    assert_eq!(im.c(1.0, 0).to_bits(), 0x3F80_0000);
    assert_eq!(im.rust(1.0, 0).to_bits(), 0x3F80_0000);
}
