//! `CONFIGS.md` row 34 / `ERRORS.md` row 11 — exhaustive differential sweep over
//! **all 2^32 possible `float` bit patterns**.
//!
//! `float2half` takes a single 32-bit-wide argument and is branch-free, so the
//! entire input domain is enumerable. Doing so replaces every sampling argument
//! with a proof: if C and Rust agree on all 4,294,967,296 inputs, they are the
//! same function.
//!
//! Work is split across all available cores. In a `debug` build the full sweep is
//! too slow for the test budget, so a coprime-stride sweep is used there instead
//! (still tens of millions of inputs, hitting every exponent class); the full
//! sweep is what `cargo test --release` runs.

mod common;

use common::Pair;
use std::sync::atomic::{AtomicU64, Ordering};

type F = unsafe extern "C" fn(f32) -> u16;

#[test]
fn exhaustive_all_2_pow_32_bit_patterns() {
    let pair = Pair::load();
    let c: F = pair.c;
    let rust: F = pair.rust;

    let threads: u64 = std::thread::available_parallelism()
        .map(|n| n.get() as u64)
        .unwrap_or(4)
        .min(32);

    let total: u64 = 1u64 << 32;
    let full = !cfg!(debug_assertions);

    // In debug, walk the domain with a stride coprime to 2^32 so the sample is
    // spread uniformly over every exponent/mantissa combination.
    const DEBUG_STRIDE: u64 = 101;

    let mismatches = AtomicU64::new(0);
    let first_bad = AtomicU64::new(u64::MAX);
    let checked = AtomicU64::new(0);

    std::thread::scope(|scope| {
        for t in 0..threads {
            let mismatches = &mismatches;
            let first_bad = &first_bad;
            let checked = &checked;
            scope.spawn(move || {
                let chunk = total / threads;
                let start = t * chunk;
                let end = if t + 1 == threads { total } else { start + chunk };

                let mut local = 0u64;
                let mut i = if full {
                    start
                } else {
                    // First multiple of DEBUG_STRIDE at or after `start`.
                    start + (DEBUG_STRIDE - start % DEBUG_STRIDE) % DEBUG_STRIDE
                };
                let step = if full { 1 } else { DEBUG_STRIDE };

                while i < end {
                    let bits = i as u32;
                    let x = f32::from_bits(bits);
                    // SAFETY: scalar-in / scalar-out FFI, no shared state.
                    let (a, b) = unsafe { (c(x), rust(x)) };
                    if a != b {
                        mismatches.fetch_add(1, Ordering::Relaxed);
                        first_bad.fetch_min(i, Ordering::Relaxed);
                    }
                    local += 1;
                    i += step;
                }
                checked.fetch_add(local, Ordering::Relaxed);
            });
        }
    });

    let n = checked.load(Ordering::Relaxed);
    let bad = mismatches.load(Ordering::Relaxed);
    println!(
        "exhaustive sweep: {} inputs checked across {} threads ({} build)",
        n,
        threads,
        if full { "release/full" } else { "debug/strided" }
    );

    if bad != 0 {
        let fb = first_bad.load(Ordering::Relaxed) as u32;
        let x = f32::from_bits(fb);
        panic!(
            "{bad} mismatches over {n} inputs; first at bits {fb:#010x} (f32 {x:e}): \
             C {:#06x} vs Rust {:#06x}",
            pair.c_of_bits(fb),
            pair.rust_of_bits(fb)
        );
    }

    if full {
        assert_eq!(n, total, "the full sweep must cover every bit pattern");
    } else {
        assert!(n > 40_000_000, "strided sweep covered too few inputs: {n}");
    }
}
