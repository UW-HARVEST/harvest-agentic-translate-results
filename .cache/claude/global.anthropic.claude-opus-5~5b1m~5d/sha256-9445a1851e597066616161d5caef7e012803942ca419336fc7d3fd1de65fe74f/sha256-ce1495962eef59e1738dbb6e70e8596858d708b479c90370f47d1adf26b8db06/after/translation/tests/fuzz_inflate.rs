//! Randomized differential fuzzing of `cp_inflate`.
//!
//! `fuzz_same` only compares inputs on which the C library is self-consistent
//! across two runs, which filters out the C's layout-dependent undefined
//! behaviour (chiefly `cp_stored`'s `memcpy`, which ignores `out_end`).

mod fuzzcommon;
mod harness;

use fuzzcommon::*;
use harness::*;

/// Rounds of `250` cases each. Override with `FUZZ_ROUNDS=<n>` for a longer run.
fn rounds() -> u32 {
    std::env::var("FUZZ_ROUNDS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2)
}

/// Mutated / random DEFLATE streams through `cp_inflate`.
#[test]
fn fuzz_cp_inflate() {
    let pair = load_pair();
    let mut rng = Rng::new(0xF00D);
    let base = corpus(&mut rng);
    let mut total_compared = 0usize;
    let mut total_dropped = 0usize;

    for round in 0..rounds() {
        let mut cases = Vec::new();
        for i in 0..250 {
            let b = &base[rng.below(base.len() as u32) as usize];
            let m = mutate(&mut rng, b);
            let out_bytes = match rng.below(4) {
                0 => 0,
                1 => rng.range(1, 32) as i32,
                2 => rng.range(1, 400) as i32,
                _ => 4096,
            };
            cases.push(fuzz_inflate(
                format!("r{round} #{i}"),
                m,
                rng.below(4) as usize,
                out_bytes,
            ));
        }
        let rep = fuzz_same(&pair, &cases);
        eprintln!("  round {round}: compared {} dropped {}", rep.compared, rep.dropped);
        total_compared += rep.compared;
        total_dropped += rep.dropped;
    }
    eprintln!("fuzz_cp_inflate: compared {total_compared}, dropped {total_dropped}");
    assert!(total_compared > 125, "too few deterministic cases");
}

