//! Exhaustive sweeps. These are expensive, so they are opt-in:
//!
//! ```text
//! EXHAUSTIVE=1 cargo test --release --test exhaustive -- --nocapture
//! ```
//!
//! Two things are established here:
//!
//! 1. `equivalent_mutants_are_provably_equivalent` — the three mutations that
//!    survived `mutation_check.sh` (`>=` -> `>`, `<=` -> `<`, and doing the
//!    `+ .5` in `double`) produce **identical output for every one of the 2^32
//!    `f32` inputs**, so they are equivalent mutants rather than blind spots in
//!    the test suite.
//! 2. `exhaustive_tap_sweep_through_both_so` — a real C-vs-Rust differential
//!    sweep across a large, evenly spaced slice of the whole `f32` tap space.

mod common;

use common::*;

fn enabled() -> bool {
    std::env::var("EXHAUSTIVE").map(|v| v != "0").unwrap_or(false)
}

/// `mp3d_scale_pcm` with the clip comparison weakened to a strict `>`.
fn scale_pcm_strict_gt(sample: f32) -> i16 {
    if f64::from(sample) > 32766.5 {
        return 32767;
    }
    if f64::from(sample) <= -32767.5 {
        return -32768;
    }
    let s = (sample + 0.5f32) as i32 as i16;
    s.wrapping_sub(i16::from(s < 0))
}

/// ... and with the lower clip weakened to a strict `<`.
fn scale_pcm_strict_lt(sample: f32) -> i16 {
    if f64::from(sample) >= 32766.5 {
        return 32767;
    }
    if f64::from(sample) < -32767.5 {
        return -32768;
    }
    let s = (sample + 0.5f32) as i32 as i16;
    s.wrapping_sub(i16::from(s < 0))
}

/// ... and with the `+ .5f` performed in `double` (double rounding).
fn scale_pcm_f64_add(sample: f32) -> i16 {
    if f64::from(sample) >= 32766.5 {
        return 32767;
    }
    if f64::from(sample) <= -32767.5 {
        return -32768;
    }
    let s = ((f64::from(sample) + 0.5) as f32) as i32 as i16;
    s.wrapping_sub(i16::from(s < 0))
}

#[test]
fn equivalent_mutants_are_provably_equivalent() {
    if !enabled() {
        eprintln!("skipping: set EXHAUSTIVE=1 to run the 2^32 sweep");
        return;
    }
    let mut diff_gt = 0u64;
    let mut diff_lt = 0u64;
    let mut diff_f64 = 0u64;
    let mut first: Vec<String> = Vec::new();
    for bits in 0u32..=u32::MAX {
        let v = f32::from_bits(bits);
        let base = expected_scale_pcm(v);
        if scale_pcm_strict_gt(v) != base {
            diff_gt += 1;
            if first.len() < 8 {
                first.push(format!("strict-> {bits:08x} ({v})"));
            }
        }
        if scale_pcm_strict_lt(v) != base {
            diff_lt += 1;
            if first.len() < 8 {
                first.push(format!("strict-< {bits:08x} ({v})"));
            }
        }
        if scale_pcm_f64_add(v) != base {
            diff_f64 += 1;
            if first.len() < 8 {
                first.push(format!("f64-add {bits:08x} ({v})"));
            }
        }
        if bits == u32::MAX {
            break;
        }
    }
    println!(
        "exhaustive 2^32 f32 sweep: >= vs > differs {diff_gt} times, \
         <= vs < differs {diff_lt} times, f32-add vs f64-add differs {diff_f64} times"
    );
    assert_eq!(
        (diff_gt, diff_lt, diff_f64),
        (0, 0, 0),
        "these mutants are NOT equivalent, so mutation_check.sh found a real \
         coverage gap; examples: {first:?}"
    );
}

#[test]
fn exhaustive_tap_sweep_through_both_so() {
    if !enabled() {
        eprintln!("skipping: set EXHAUSTIVE=1 to run the wide tap sweep");
        return;
    }
    let h = Harness::load();
    let prefill = vec![0i16; 4096];
    // Stride chosen coprime with 2^32 so the walk visits every exponent and a
    // dense, evenly spread set of mantissas: ~4.3M calls per tap position.
    const STRIDE: u32 = 1009;
    let mut z = z_zeros();
    let mut n = 0u64;
    for tap in [0usize, 7, 14] {
        let mut bits: u32 = 0;
        loop {
            let v = f32::from_bits(bits);
            z[tap * 64] = v;
            z[2 + tap * 64] = v;
            let (c, r) = h.call_both(&z, 2, &prefill, 2048);
            assert_eq!(
                c, r,
                "exhaustive sweep divergence: tap {tap} v bits {bits:08x} ({v})"
            );
            n += 1;
            let (next, wrapped) = bits.overflowing_add(STRIDE);
            if wrapped {
                break;
            }
            bits = next;
        }
        z[tap * 64] = 0.0;
        z[2 + tap * 64] = 0.0;
    }
    println!("exhaustive tap sweep: {n} differential calls, 0 divergences");
}
