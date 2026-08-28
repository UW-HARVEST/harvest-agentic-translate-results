//! Exhaustive differential sweeps.
//!
//! These go beyond the randomized `CONFIGS.md` rows: they enumerate **every one
//! of the 2^32 `f32` bit patterns** through a chosen tap and compare the C and
//! Rust `.so` outputs. That covers every normal, subnormal, zero, infinity and
//! NaN payload, and — because the products land densely around the clamp
//! thresholds — effectively every reachable accumulator value in the regions
//! `mp3d_scale_pcm` branches on.
//!
//! Work is split across threads; both `.so` entry points are pure functions of
//! their arguments (verified by `cfg_c15_repeated_calls_stateless`), so calling
//! them concurrently on disjoint buffers is sound.

mod common;

use common::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use common::optimized;

/// Unoptimized (`cargo test` without `--release`) runs are ~30x slower, so the
/// "fully exhaustive" sweeps are strided there to stay inside a sane runtime.
/// `--release` runs remain exhaustive (step == 1).
fn full_step() -> u32 {
    if optimized() {
        1
    } else {
        512
    }
}

fn coarse(step: u32) -> u32 {
    if optimized() {
        step
    } else {
        step.saturating_mul(32)
    }
}

fn threads() -> u32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
        .clamp(1, 16)
}

/// Sweeps every `u32` bit pattern into `z[tap]` (all other taps zero) and
/// asserts the two `.so`s agree on the full `pcm` buffer.
///
/// `step` allows a strided sweep; `step == 1` is fully exhaustive.
fn sweep_tap(label: &'static str, tap: usize, nch: i32, step: u32) {
    let n_threads = threads();
    let mismatches = Arc::new(AtomicU64::new(0));
    let first_bad = Arc::new(AtomicU64::new(u64::MAX));
    let checked = Arc::new(AtomicU64::new(0));

    std::thread::scope(|s| {
        for t in 0..n_threads {
            let mismatches = Arc::clone(&mismatches);
            let first_bad = Arc::clone(&first_bad);
            let checked = Arc::clone(&checked);
            s.spawn(move || {
                let p = pair();
                let mut z = zeros_z();
                let mut pcm_c = vec![0i16; 16 * 8 + 16];
                let mut pcm_r = vec![0i16; 16 * 8 + 16];
                let mut local = 0u64;

                // Thread `t` handles bit patterns t, t+n, t+2n, ... (scaled by step).
                let mut bits: u64 = (t as u64).wrapping_mul(step as u64);
                while bits <= u32::MAX as u64 {
                    z[tap] = f32::from_bits(bits as u32);
                    pcm_c.fill(0x5A5A_u16 as i16);
                    pcm_r.fill(0x5A5A_u16 as i16);
                    unsafe {
                        (p.c.synth_pair)(pcm_c.as_mut_ptr(), nch, z.as_ptr());
                        (p.rust.synth_pair)(pcm_r.as_mut_ptr(), nch, z.as_ptr());
                    }
                    if pcm_c != pcm_r {
                        mismatches.fetch_add(1, Ordering::Relaxed);
                        let _ = first_bad.fetch_min(bits, Ordering::Relaxed);
                    }
                    local += 1;
                    bits += (n_threads as u64) * (step as u64);
                }
                checked.fetch_add(local, Ordering::Relaxed);
            });
        }
    });

    let bad = mismatches.load(Ordering::Relaxed);
    let n = checked.load(Ordering::Relaxed);
    eprintln!("{label}: checked {n} bit patterns for z[{tap}] (nch={nch}, step={step})");
    assert!(n > 0, "{label}: swept nothing");
    if bad != 0 {
        let fb = first_bad.load(Ordering::Relaxed) as u32;
        panic!(
            "{label}: {bad} mismatch(es); first at z[{tap}] = 0x{fb:08x} ({:e})",
            f32::from_bits(fb)
        );
    }
}

/// Fully exhaustive: all 2^32 patterns through lane 0's dominant tap
/// (`z[7*64]`, weight `75038`), which is also the *last* accumulated term.
#[test]
fn exhaustive_all_f32_through_lane0_dominant_tap() {
    sweep_tap("exhaustive lane0 z[448]", 448, 2, full_step());
}

/// Fully exhaustive: all 2^32 patterns through lane 1's dominant tap
/// (`z[2 + 8*64]`, weight `64019`).
#[test]
fn exhaustive_all_f32_through_lane1_dominant_tap() {
    sweep_tap("exhaustive lane1 z[514]", 514, 2, full_step());
}

/// Strided sweeps through every remaining tap, so each of the 23 read indices
/// and each of the 16 distinct weights is probed across the whole `f32` domain.
#[test]
fn strided_all_f32_through_every_tap() {
    // 2^32 / 4096 ~= 1.05 M patterns per tap, 23 taps.
    const STEP: u32 = 4096;
    for &tap in all_taps().iter() {
        sweep_tap("strided", tap, 2, coarse(STEP));
    }
    // Odd strides hit different residue classes of the mantissa.
    for &tap in &[0usize, 448, 896, 2, 514, 898] {
        sweep_tap("strided-odd", tap, 1, coarse(4093));
    }
}

/// Exhaustive over one operand of each *difference* pair while the other
/// operand is pinned to a set of interesting values, so cancellation,
/// `inf - inf` and sign-of-zero behaviour are covered across the whole domain
/// of the swept operand.
#[test]
fn strided_difference_pairs_against_pinned_operands() {
    const DIFF_PAIRS: [(usize, usize); 4] = [(896, 0), (768, 128), (640, 256), (512, 384)];
    const STEP: u32 = 65_536; // 65_536 patterns per (pair, pinned) combination
    let step = coarse(STEP);
    let pinned: &[f32] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::MIN_POSITIVE,
        1e-40,
        f32::MAX,
        f32::MIN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        0.4366, // ~= 32767 / 75038, i.e. right at the clamp threshold
        -0.4366,
    ];

    let p = pair();
    let mut pcm_c = vec![0i16; 16 * 8 + 16];
    let mut pcm_r = vec![0i16; 16 * 8 + 16];
    let mut count = 0u64;

    for (hi, lo) in DIFF_PAIRS {
        for &pin in pinned {
            for which in 0..2 {
                let mut z = zeros_z();
                let (swept, fixed) = if which == 0 { (hi, lo) } else { (lo, hi) };
                z[fixed] = pin;
                let mut bits: u64 = 0;
                while bits <= u32::MAX as u64 {
                    z[swept] = f32::from_bits(bits as u32);
                    pcm_c.fill(0x5A5A_u16 as i16);
                    pcm_r.fill(0x5A5A_u16 as i16);
                    unsafe {
                        (p.c.synth_pair)(pcm_c.as_mut_ptr(), 2, z.as_ptr());
                        (p.rust.synth_pair)(pcm_r.as_mut_ptr(), 2, z.as_ptr());
                    }
                    assert_eq!(
                        pcm_c, pcm_r,
                        "difference pair ({hi},{lo}): z[{swept}]=0x{:08x} z[{fixed}]={pin:e}",
                        bits as u32
                    );
                    count += 1;
                    bits += step as u64;
                }
            }
        }
    }
    eprintln!("difference-pair sweep: {count} comparisons");
    assert!(count > 1_000_000 / coarse(1).max(1) as u64);
}

/// Exhaustive over the *sum* pairs, same idea.
#[test]
fn strided_sum_pairs_against_pinned_operands() {
    const SUM_PAIRS: [(usize, usize); 3] = [(64, 832), (192, 704), (320, 576)];
    const STEP: u32 = 65_536;
    let step = coarse(STEP);
    let pinned: &[f32] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NAN,
        f32::MAX,
        f32::MIN,
        153.83, // ~= 32767 / 213, right at the clamp threshold for this weight
        -153.83,
    ];

    let p = pair();
    let mut pcm_c = vec![0i16; 16 * 8 + 16];
    let mut pcm_r = vec![0i16; 16 * 8 + 16];
    let mut count = 0u64;

    for (a, b) in SUM_PAIRS {
        for &pin in pinned {
            let mut z = zeros_z();
            z[b] = pin;
            let mut bits: u64 = 0;
            while bits <= u32::MAX as u64 {
                z[a] = f32::from_bits(bits as u32);
                pcm_c.fill(0x5A5A_u16 as i16);
                pcm_r.fill(0x5A5A_u16 as i16);
                unsafe {
                    (p.c.synth_pair)(pcm_c.as_mut_ptr(), 2, z.as_ptr());
                    (p.rust.synth_pair)(pcm_r.as_mut_ptr(), 2, z.as_ptr());
                }
                assert_eq!(
                    pcm_c, pcm_r,
                    "sum pair ({a},{b}): z[{a}]=0x{:08x} z[{b}]={pin:e}",
                    bits as u32
                );
                count += 1;
                bits += step as u64;
            }
        }
    }
    eprintln!("sum-pair sweep: {count} comparisons");
    assert!(count > 500_000 / coarse(1).max(1) as u64);
}
