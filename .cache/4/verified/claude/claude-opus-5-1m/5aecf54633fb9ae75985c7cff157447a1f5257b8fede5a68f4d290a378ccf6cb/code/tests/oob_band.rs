//! The one region that cannot be asserted exactly: the far out-of-bounds write.
//!
//! For large `data`, whether `buffer[data] = 1` faults depends on how far the top
//! of the `[stack]` mapping happens to be from `bad()`'s frame, and stack ASLR
//! re-rolls that on every execution. The C binary itself is therefore
//! nondeterministic there — measured over 40 runs per index, index 1500 crashed
//! 25/40 times and index 2200 37/40. No implementation can be byte-identical
//! inside that band.
//!
//! What *can* be pinned down, and is pinned down here:
//!
//!   * below the band both implementations never crash,
//!   * above it both always crash,
//!   * and across the band their crash rates track each other, which is what
//!     would break if the emulation in `imp.rs` were removed or badly calibrated.
//!
//! Without the third check a regression that deleted `far_write_is_fatal`
//! entirely would still pass every other test in the suite, because the strict
//! rows deliberately stay out of the band.

mod common;
use common::exe;

fn crash_rate(exe_path: &std::path::Path, k: i64, n: usize) -> usize {
    (0..n)
        .filter(|_| {
            let stdin = format!("0\n{k}\n").into_bytes();
            exe::run(exe_path, &stdin).status.is_err()
        })
        .count()
}

#[test]
fn below_the_band_neither_implementation_crashes() {
    common::ensure_built();
    let limit = common::deterministic_benign_limit();
    for k in [28, limit / 2, limit] {
        let c = crash_rate(&common::c_exe(), k, 15);
        let r = crash_rate(&common::rust_exe(), k, 15);
        assert_eq!(c, 0, "C crashed {c}/15 times at index {k}, below the band");
        assert_eq!(r, 0, "Rust crashed {r}/15 times at index {k}, below the band");
    }
}

#[test]
fn above_the_band_both_implementations_always_crash() {
    common::ensure_built();
    for k in [200_000i64, 1_000_000, 100_000_000, i32::MAX as i64] {
        let c = crash_rate(&common::c_exe(), k, 12);
        let r = crash_rate(&common::rust_exe(), k, 12);
        assert_eq!(c, 12, "C crashed only {c}/12 times at index {k}");
        assert_eq!(r, 12, "Rust crashed only {r}/12 times at index {k}");
    }
}

/// The crash-rate curves must track each other across the transition. Compared in
/// aggregate over a ladder spanning the band, because per-index rates are far too
/// noisy at any sample count a test can afford.
#[test]
fn across_the_band_the_crash_rates_track_each_other() {
    common::ensure_built();
    const N: usize = 12;
    let ladder: [i64; 9] = [1_500, 2_500, 3_500, 4_500, 6_000, 8_000, 12_000, 20_000, 40_000];

    let mut c_total = 0usize;
    let mut r_total = 0usize;
    let mut per_index = Vec::new();
    for k in ladder {
        let c = crash_rate(&common::c_exe(), k, N);
        let r = crash_rate(&common::rust_exe(), k, N);
        per_index.push((k, c, r));
        c_total += c;
        r_total += r;
    }
    let samples = ladder.len() * N;

    // The ladder deliberately straddles the boundary, so C must land strictly
    // inside the interval -- otherwise the ladder has drifted out of the band and
    // the comparison below would be vacuous.
    assert!(
        c_total > 0 && c_total < samples,
        "the ladder no longer straddles C's transition ({c_total}/{samples} crashed); \
         per-index: {per_index:?}"
    );

    let diff = c_total.abs_diff(r_total);
    let tolerance = samples / 3;
    assert!(
        diff <= tolerance,
        "crash-rate curves diverged: C {c_total}/{samples} vs Rust {r_total}/{samples} \
         (difference {diff} > tolerance {tolerance}); per-index (k, C, Rust): {per_index:?}"
    );
}
