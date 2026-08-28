//! Phase B rows C65 / C66 — exhaustive sweeps over the library's ENTIRE
//! reachable input domain for one operand.
//!
//! `contrast_ratio` only ever feeds `cbLuminance` the 256 values `n/255.f`, so
//! sweeping all 2^24 colors in one operand position covers every `pow` argument
//! and every branch decision the library can ever make. This is strictly
//! stronger than sampling.
//!
//! Set `EXHAUSTIVE_STRIDE=<n>` to sub-sample (default 1 = full sweep).

mod common;

use common::{diff_one, Rgb};

fn stride() -> u32 {
    std::env::var("EXHAUSTIVE_STRIDE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|&s| s >= 1)
        .unwrap_or(1)
}

/// Sweep all 2^24 colors in operand position `pos` against `partner`.
fn sweep(label: &str, partner: Rgb, a_side: bool) {
    let (c, r) = common::load_pair();
    let step = stride();
    let start = std::time::Instant::now();

    let mut checked = 0u64;
    let mut failures: Vec<String> = Vec::new();
    let mut non_finite = 0u64;

    let mut code = 0u32;
    while code < (1 << 24) {
        let col = Rgb::new((code >> 16) as u8, (code >> 8) as u8, code as u8);
        let (a, b) = if a_side { (col, partner) } else { (partner, col) };
        match diff_one(&c, &r, a, b) {
            Ok(v) => {
                if !v.is_finite() {
                    non_finite += 1;
                }
            }
            Err(m) => {
                if failures.len() < 20 {
                    failures.push(m);
                }
            }
        }
        checked += 1;
        code += step;
    }

    let secs = start.elapsed().as_secs_f64();
    println!(
        "{label}: {checked} colors swept in {secs:.1}s (stride {step}), {non_finite} non-finite"
    );
    assert!(
        failures.is_empty(),
        "{label}: {} divergences (first {} shown):\n{}",
        failures.len(),
        failures.len(),
        failures.join("\n")
    );
    assert!(checked > 0);
}

/// C65 — every one of the 16,777,216 colors as operand A, against white.
#[test]
fn exhaustive_all_colors_vs_white() {
    sweep("C65 all colors (A) vs WHITE", Rgb::WHITE, true);
}

/// C66 — every one of the 16,777,216 colors as operand B, against black. This
/// also drives the `Low == 0` degenerate divisor for every possible `High`.
#[test]
fn exhaustive_all_colors_vs_black() {
    sweep("C66 all colors (B) vs BLACK", Rgb::BLACK, false);
}
