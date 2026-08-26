//! Phase B (extended) — exhaustive sweeps over the full 2^24 colour domain.
//!
//! The complete input domain of `contrast_ratio` is 256^6 = 2.8e14 pairs, which
//! is not enumerable. But the function factors through `cbLuminance`, which has a
//! 2^24 domain, and *that* is enumerable. Holding one argument fixed and sweeping
//! the other over all 16 777 216 colours therefore exercises **100 % of the
//! reachable inputs of the inner luminance routine**, in both argument positions.
//!
//! These tests are `#[ignore]`d so the default `cargo test` stays fast. Run with:
//!
//! ```text
//! cargo test --release -- --ignored --nocapture
//! ```

mod common;

use common::*;

/// Sweep every one of the 2^24 colours in argument position A against a fixed B.
fn sweep_all_colors_as_a(p: &Pair, b: Rgb, label: &str) {
    let mut mismatches = 0usize;
    for packed in 0u32..(1 << 24) {
        let a = Rgb::new(packed as u8, (packed >> 8) as u8, (packed >> 16) as u8);
        let cv = unsafe { (p.c.contrast_ratio)(a, b) };
        let rv = unsafe { (p.rust.contrast_ratio)(a, b) };
        if cv.to_bits() != rv.to_bits() {
            if mismatches < 10 {
                eprintln!(
                    "MISMATCH [{label}] A={{{},{},{}}} B={{{},{},{}}} C=0x{:08X} Rust=0x{:08X}",
                    a.r,
                    a.g,
                    a.b,
                    b.r,
                    b.g,
                    b.b,
                    cv.to_bits(),
                    rv.to_bits()
                );
            }
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0, "[{label}] {mismatches} mismatches over all 2^24 colours");
    eprintln!("[{label}] 16777216/16777216 colours bit-identical");
}

/// Same, with the swept colour in argument position B.
fn sweep_all_colors_as_b(p: &Pair, a: Rgb, label: &str) {
    let mut mismatches = 0usize;
    for packed in 0u32..(1 << 24) {
        let b = Rgb::new(packed as u8, (packed >> 8) as u8, (packed >> 16) as u8);
        let cv = unsafe { (p.c.contrast_ratio)(a, b) };
        let rv = unsafe { (p.rust.contrast_ratio)(a, b) };
        if cv.to_bits() != rv.to_bits() {
            if mismatches < 10 {
                eprintln!(
                    "MISMATCH [{label}] A={{{},{},{}}} B={{{},{},{}}} C=0x{:08X} Rust=0x{:08X}",
                    a.r,
                    a.g,
                    a.b,
                    b.r,
                    b.g,
                    b.b,
                    cv.to_bits(),
                    rv.to_bits()
                );
            }
            mismatches += 1;
        }
    }
    assert_eq!(mismatches, 0, "[{label}] {mismatches} mismatches over all 2^24 colours");
    eprintln!("[{label}] 16777216/16777216 colours bit-identical");
}

#[test]
#[ignore = "exhaustive: 2^24 colours, run with --ignored"]
fn x1_all_colors_vs_white_as_a() {
    sweep_all_colors_as_a(pair(), WHITE, "A sweep vs WHITE");
}

#[test]
#[ignore = "exhaustive: 2^24 colours, run with --ignored"]
fn x2_all_colors_vs_mid_as_a() {
    // A mid-grey partner: about half the sweep takes the swap branch and half
    // does not, so both sides of the `High < Low` test are exercised across the
    // whole colour domain.
    sweep_all_colors_as_a(pair(), MID, "A sweep vs MID(127)");
}

#[test]
#[ignore = "exhaustive: 2^24 colours, run with --ignored"]
fn x3_all_colors_vs_dark_as_a() {
    // A partner on the *linear* branch in every channel, and a very small
    // denominator, so the sweep also covers the un-guarded near-zero division
    // across the whole colour domain.
    sweep_all_colors_as_a(pair(), Rgb::new(1, 2, 3), "A sweep vs {1,2,3}");
}

#[test]
#[ignore = "exhaustive: 2^24 colours, run with --ignored"]
fn x4_all_colors_vs_white_as_b() {
    sweep_all_colors_as_b(pair(), WHITE, "B sweep vs WHITE");
}

#[test]
#[ignore = "exhaustive: 2^24 colours, run with --ignored"]
fn x5_all_colors_vs_black_as_b() {
    // Every colour against black: the entire +inf / NaN degenerate surface,
    // exhaustively.
    sweep_all_colors_as_b(pair(), BLACK, "B sweep vs BLACK");
}

#[test]
#[ignore = "exhaustive: 2^24 colours, run with --ignored"]
fn x6_all_colors_vs_boundary_as_b() {
    // Partner straddling the 0.04045 branch boundary in every channel.
    sweep_all_colors_as_b(pair(), Rgb::new(10, 11, 10), "B sweep vs {10,11,10}");
}

/// A very large randomized sweep over the *full* 6-byte space, well beyond what
/// the default suite runs, to probe pairs that the fixed-partner sweeps cannot
/// reach.
#[test]
#[ignore = "exhaustive: 40M random pairs, run with --ignored"]
fn x7_huge_random_pair_sweep() {
    let p = pair();
    let mut rng = Rng::new(0x5EED_0000_0000_0001);
    let n: usize = std::env::var("DIFF_HUGE_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(40_000_000);
    let mut nan = 0usize;
    let mut inf = 0usize;
    let mut finite = 0usize;
    for i in 0..n {
        let a = rng.color();
        let b = rng.color();
        let cv = unsafe { (p.c.contrast_ratio)(a, b) };
        let rv = unsafe { (p.rust.contrast_ratio)(a, b) };
        assert_eq!(
            cv.to_bits(),
            rv.to_bits(),
            "mismatch at iteration {i}: A={a:?} B={b:?} C=0x{:08X} Rust=0x{:08X}",
            cv.to_bits(),
            rv.to_bits()
        );
        if cv.is_nan() {
            nan += 1;
        } else if cv.is_infinite() {
            inf += 1;
        } else {
            finite += 1;
        }
    }
    eprintln!("[x7] {n} random pairs bit-identical (finite={finite} inf={inf} nan={nan})");
}

/// Exhaustive over all pairs drawn from a dense stratified subset of the colour
/// cube: every 16th value in each channel (16^3 = 4096 colours) crossed with
/// itself = 16 777 216 pairs. Unlike the fixed-partner sweeps this varies *both*
/// arguments simultaneously.
#[test]
#[ignore = "exhaustive: 4096x4096 pairs, run with --ignored"]
fn x8_stratified_pair_cross_product() {
    let p = pair();
    // Include both ends and both sides of the 0.04045 boundary in the strata.
    let vals: Vec<u8> = {
        let mut v: Vec<u8> = (0..16).map(|i| (i * 17) as u8).collect(); // 0,17,...,255
        v.extend_from_slice(&[1, 10, 11, 12, 127, 128, 254]);
        v.sort_unstable();
        v.dedup();
        v
    };
    let mut colors: Vec<Rgb> = Vec::with_capacity(vals.len().pow(3));
    for &r in &vals {
        for &g in &vals {
            for &b in &vals {
                colors.push(Rgb::new(r, g, b));
            }
        }
    }
    eprintln!("[x8] {} colours -> {} pairs", colors.len(), colors.len() * colors.len());
    for &a in &colors {
        for &b in &colors {
            let cv = unsafe { (p.c.contrast_ratio)(a, b) };
            let rv = unsafe { (p.rust.contrast_ratio)(a, b) };
            assert_eq!(
                cv.to_bits(),
                rv.to_bits(),
                "mismatch: A={a:?} B={b:?} C=0x{:08X} Rust=0x{:08X}",
                cv.to_bits(),
                rv.to_bits()
            );
        }
    }
    eprintln!("[x8] all pairs bit-identical");
}
