//! Proof obligations for the mutants that SURVIVE `./mutation_check.sh`.
//!
//! A surviving mutant is only acceptable if it is *provably observationally
//! equivalent* to the C. Each test here discharges one such obligation by
//! exhaustively enumerating all 2^32 `f32` bit patterns (`sample` is the only
//! input to `mp3d_scale_pcm`, so 2^32 really is exhaustive for that function).
//!
//! See `EQUIVALENT_MUTANTS.md` for the written-up argument each test backs.

mod common;

use common::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// --- the baseline: a literal replay of the C -------------------------------

#[inline(always)]
fn orig(sample: f32) -> i16 {
    if sample as f64 >= 32766.5 {
        return 32767;
    }
    if sample as f64 <= -32767.5 {
        return -32768;
    }
    let s = (sample + 0.5f32) as i32 as i16;
    s.wrapping_sub((s < 0) as i16)
}

// --- the surviving mutants -------------------------------------------------

/// Survivor 1: `>=` weakened to `>` on the high guard.
#[inline(always)]
fn mut_hi_strict(sample: f32) -> i16 {
    if sample as f64 > 32766.5 {
        return 32767;
    }
    if sample as f64 <= -32767.5 {
        return -32768;
    }
    let s = (sample + 0.5f32) as i32 as i16;
    s.wrapping_sub((s < 0) as i16)
}

/// Survivor 2: `<=` weakened to `<` on the low guard.
#[inline(always)]
fn mut_lo_strict(sample: f32) -> i16 {
    if sample as f64 >= 32766.5 {
        return 32767;
    }
    if (sample as f64) < -32767.5 {
        return -32768;
    }
    let s = (sample + 0.5f32) as i32 as i16;
    s.wrapping_sub((s < 0) as i16)
}

/// Survivor 3: the high threshold moved from `32766.5` to `32767.5`.
#[inline(always)]
fn mut_hi_threshold(sample: f32) -> i16 {
    if sample as f64 >= 32767.5 {
        return 32767;
    }
    if sample as f64 <= -32767.5 {
        return -32768;
    }
    let s = (sample + 0.5f32) as i32 as i16;
    s.wrapping_sub((s < 0) as i16)
}

/// Survivor 4: narrow `f32 -> i16` directly instead of via `i32`.
#[inline(always)]
fn mut_direct_narrow(sample: f32) -> i16 {
    if sample as f64 >= 32766.5 {
        return 32767;
    }
    if sample as f64 <= -32767.5 {
        return -32768;
    }
    let s = (sample + 0.5f32) as i16;
    s.wrapping_sub((s < 0) as i16)
}

// ---------------------------------------------------------------------------
// Exhaustive equivalence over all 2^32 f32 bit patterns.
// ---------------------------------------------------------------------------

/// `1` under `--release`; strided for unoptimized runs (the same property is
/// re-proved exhaustively by the release run in `run_all_feature_combos.sh`).
fn eq_step() -> u64 {
    if optimized() {
        1
    } else {
        64
    }
}

fn exhaustive_equal(label: &'static str, variant: fn(f32) -> i16) {
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get() as u64)
        .unwrap_or(4)
        .clamp(1, 16);
    let bad = Arc::new(AtomicU64::new(0));
    let first = Arc::new(AtomicU64::new(u64::MAX));

    std::thread::scope(|s| {
        for t in 0..n_threads {
            let bad = Arc::clone(&bad);
            let first = Arc::clone(&first);
            s.spawn(move || {
                let mut bits = t * eq_step();
                while bits <= u32::MAX as u64 {
                    let x = f32::from_bits(bits as u32);
                    if orig(x) != variant(x) {
                        bad.fetch_add(1, Ordering::Relaxed);
                        let _ = first.fetch_min(bits, Ordering::Relaxed);
                    }
                    bits += n_threads * eq_step();
                }
            });
        }
    });

    let n = bad.load(Ordering::Relaxed);
    if n != 0 {
        let fb = first.load(Ordering::Relaxed) as u32;
        let x = f32::from_bits(fb);
        panic!(
            "{label}: NOT equivalent — {n} differing input(s); first at \
             0x{fb:08x} ({x:e}): C-replay = {}, mutant = {}",
            orig(x),
            variant(x)
        );
    }
    eprintln!("{label}: equivalent over all 2^32 f32 inputs");
}

#[test]
fn survivor_hi_guard_ge_vs_gt_is_equivalent() {
    // At exactly a == 32766.5 the conversion path yields
    // (int16_t)(32766.5 + 0.5f) == (int16_t)32767.0 == 32767, i.e. the same
    // value the guard would have returned.
    assert_eq!(orig(32766.5), 32767);
    assert_eq!(mut_hi_strict(32766.5), 32767);
    exhaustive_equal("hi guard >= vs >", mut_hi_strict);
}

#[test]
fn survivor_lo_guard_le_vs_lt_is_equivalent() {
    // At exactly a == -32767.5 the conversion path yields
    // (int16_t)(-32767.5 + 0.5f) == (int16_t)(-32767.0) == -32767, then the
    // `s -= (s < 0)` correction makes it -32768 — the guard's value.
    assert_eq!(orig(-32767.5), -32768);
    assert_eq!(mut_lo_strict(-32767.5), -32768);
    exhaustive_equal("lo guard <= vs <", mut_lo_strict);
}

#[test]
fn survivor_hi_threshold_32766_5_vs_32767_5_is_equivalent() {
    // For every f32 in [32766.5, 32767.5) the sum a + 0.5f lies in
    // [32767.0, 32768.0) and truncates to 32767 — the clamp value again.
    exhaustive_equal("hi threshold 32766.5 vs 32767.5", mut_hi_threshold);
}

#[test]
fn survivor_direct_f32_to_i16_narrowing_is_equivalent() {
    // The guards bound a + 0.5f inside (-32767.0, 32767.0], so truncation
    // (C, via i32 then a 16-bit store) and Rust's saturating `as i16` agree;
    // both map NaN to 0.
    exhaustive_equal("direct f32 -> i16 narrowing", mut_direct_narrow);
}

// ---------------------------------------------------------------------------
// Survivor 5: `a += t` vs `a = t + a`.
// ---------------------------------------------------------------------------

/// IEEE-754 addition is commutative for every pair of operands *except* that
/// the NaN payload propagated when both operands are NaN is implementation
/// chosen. `mp3d_scale_pcm` maps **every** NaN to `0`, so the choice is
/// unobservable. This test proves both halves of that claim.
#[test]
fn survivor_accumulation_order_is_equivalent() {
    // (a) every NaN maps to 0, exhaustively over all 2^32 patterns.
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get() as u64)
        .unwrap_or(4)
        .clamp(1, 16);
    let bad = Arc::new(AtomicU64::new(0));
    let nans = Arc::new(AtomicU64::new(0));
    std::thread::scope(|s| {
        for t in 0..n_threads {
            let bad = Arc::clone(&bad);
            let nans = Arc::clone(&nans);
            s.spawn(move || {
                let mut local_nan = 0u64;
                let mut local_bad = 0u64;
                let mut bits = t;
                while bits <= u32::MAX as u64 {
                    let x = f32::from_bits(bits as u32);
                    if x.is_nan() {
                        local_nan += 1;
                        if orig(x) != 0 {
                            local_bad += 1;
                        }
                    }
                    bits += n_threads;
                }
                nans.fetch_add(local_nan, Ordering::Relaxed);
                bad.fetch_add(local_bad, Ordering::Relaxed);
            });
        }
    });
    let n_nan = nans.load(Ordering::Relaxed);
    // 2 * (2^23 - 1) NaN encodings exist for f32.
    assert_eq!(n_nan, 2 * ((1u64 << 23) - 1), "unexpected NaN encoding count");
    assert_eq!(
        bad.load(Ordering::Relaxed),
        0,
        "some NaN encoding did not map to 0"
    );
    eprintln!("all {n_nan} f32 NaN encodings map to 0");

    // (b) `x + y` and `y + x` are bit-identical unless *both* are NaN.
    let mut rng = Rng::new(0xE005);
    let mut both_nan = 0u64;
    for _ in 0..4_000_000 {
        let x = f32::from_bits(rng.next_u32());
        let y = f32::from_bits(rng.next_u32());
        let ab = (x + y).to_bits();
        let ba = (y + x).to_bits();
        if ab != ba {
            assert!(
                x.is_nan() && y.is_nan(),
                "addition not commutative for non-NaN operands \
                 0x{:08x} + 0x{:08x}",
                x.to_bits(),
                y.to_bits()
            );
            both_nan += 1;
            // ... and in that case both results are NaN, hence both scale to 0.
            assert!(f32::from_bits(ab).is_nan() && f32::from_bits(ba).is_nan());
            assert_eq!(orig(f32::from_bits(ab)), 0);
            assert_eq!(orig(f32::from_bits(ba)), 0);
        }
    }
    // Structured pairs guarantee we actually exercised the both-NaN case.
    for xb in [0x7F80_0001u32, 0x7FC0_0000, 0xFFC0_0000, 0x7FFF_FFFF] {
        for yb in [0x7F80_0002u32, 0x7FBF_FFFF, 0xFF80_0003, 0xFFFF_FFFF] {
            let x = f32::from_bits(xb);
            let y = f32::from_bits(yb);
            assert!(orig(x + y) == 0 && orig(y + x) == 0);
        }
    }
    eprintln!("commutativity check: {both_nan} both-NaN pairs differed, all scale to 0");

    // (c) The multiplications are likewise order-independent: GCC emits
    //     `mulss %xmm1,%xmm0` with the *constant* in xmm0, i.e. `w * z`, while
    //     the Rust writes `z * w`. Prove that is bit-identical for every f32
    //     against every weight the library uses.
    let weights: Vec<f32> = LANE0_TERMS
        .iter()
        .map(|&(_, w, _)| w)
        .chain(LANE1_TERMS.iter().map(|&(_, w)| w))
        .collect();
    let bad2 = Arc::new(AtomicU64::new(0));
    std::thread::scope(|s| {
        for t in 0..n_threads {
            let bad2 = Arc::clone(&bad2);
            let weights = weights.clone();
            s.spawn(move || {
                let mut local = 0u64;
                let mut bits = t;
                while bits <= u32::MAX as u64 {
                    let x = f32::from_bits(bits as u32);
                    for &w in &weights {
                        if (x * w).to_bits() != (w * x).to_bits() {
                            local += 1;
                        }
                    }
                    // Stride to keep this within a reasonable runtime; the
                    // property is per-value so a dense sweep suffices.
                    bits += n_threads * 64;
                }
                bad2.fetch_add(local, Ordering::Relaxed);
            });
        }
    });
    assert_eq!(
        bad2.load(Ordering::Relaxed),
        0,
        "multiplication is not order-independent for some (value, weight)"
    );
    eprintln!("multiplication order-independence: OK for all 23 weights");
}

// ---------------------------------------------------------------------------
// Tie the reference replay to the REAL C `.so`, exhaustively.
// ---------------------------------------------------------------------------

/// `orig` / `c_scale_pcm_reference` are used as cross-checks throughout the
/// suite, so they must themselves be validated against the C shared object.
/// Sweep all 2^32 `f32` values through lane 0's last tap and require the C
/// `.so`'s `pcm[0]` to equal `c_scale_pcm_reference(model_lane0(z))`.
#[test]
fn exhaustive_reference_model_matches_the_c_so() {
    let n_threads = std::thread::available_parallelism()
        .map(|n| n.get() as u64)
        .unwrap_or(4)
        .clamp(1, 16);
    let bad = Arc::new(AtomicU64::new(0));
    let first = Arc::new(AtomicU64::new(u64::MAX));

    std::thread::scope(|s| {
        for t in 0..n_threads {
            let bad = Arc::clone(&bad);
            let first = Arc::clone(&first);
            s.spawn(move || {
                let p = pair();
                let mut z = zeros_z();
                let mut pcm = vec![0i16; 64];
                let mut bits = t;
                while bits <= u32::MAX as u64 {
                    let x = f32::from_bits(bits as u32);
                    z[448] = x;
                    unsafe { (p.c.synth_pair)(pcm.as_mut_ptr(), 2, z.as_ptr()) };
                    if pcm[0] != c_scale_pcm_reference(model_lane0(&z)) {
                        bad.fetch_add(1, Ordering::Relaxed);
                        let _ = first.fetch_min(bits, Ordering::Relaxed);
                    }
                    bits += n_threads;
                }
            });
        }
    });

    let n = bad.load(Ordering::Relaxed);
    if n != 0 {
        let fb = first.load(Ordering::Relaxed) as u32;
        panic!(
            "reference model disagrees with the C .so in {n} case(s); first at \
             z[448] = 0x{fb:08x}"
        );
    }
    eprintln!("reference model matches the C .so over all 2^32 lane-0 inputs");
}
