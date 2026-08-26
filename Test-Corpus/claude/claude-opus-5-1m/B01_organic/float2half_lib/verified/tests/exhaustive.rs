//! ERRORS.md row G11 / CONFIGS.md rows 1..86 — exhaustive differential proof.
//!
//! `float2half` has a 32-bit input domain and no state, so equivalence can be
//! decided *completely*: compare C and Rust over all 2^32 bit patterns. This
//! leaves no reachable input, code path, table entry or configuration row
//! untested.
//!
//! In a release build the full 2^32 sweep runs (parallelised across cores). In
//! an unoptimised `cargo test` build it strides the domain so the test stays
//! fast; run `cargo test --release` for the complete sweep.

mod common;

use common::libs;
use std::sync::atomic::{AtomicU64, Ordering};

#[test]
fn exhaustive_all_2_pow_32_inputs() {
    let l = libs();
    // Copy the raw pointers out: they are `Copy + Send`, so each worker calls
    // straight through the `.so` exports with no synchronisation.
    let c_fn = l.c_float2half;
    let r_fn = l.rust_float2half;

    // Full sweep when optimised; strided otherwise to keep the test bounded.
    let stride: u64 = if cfg!(debug_assertions) { 512 } else { 1 };

    let threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(32) as u64;

    const DOMAIN: u64 = 1u64 << 32;
    let chunk = DOMAIN.div_ceil(threads);

    let checked = AtomicU64::new(0);
    let first_bad = AtomicU64::new(u64::MAX);

    std::thread::scope(|scope| {
        for t in 0..threads {
            let checked = &checked;
            let first_bad = &first_bad;
            scope.spawn(move || {
                let start = t * chunk;
                let end = ((t + 1) * chunk).min(DOMAIN);
                let mut n = 0u64;
                let mut bad = u64::MAX;
                let mut i = start;
                // Align each worker's start onto the stride grid.
                if stride > 1 && i % stride != 0 {
                    i += stride - (i % stride);
                }
                while i < end {
                    let bits = i as u32;
                    let x = f32::from_bits(bits);
                    let c = unsafe { c_fn(x) };
                    let r = unsafe { r_fn(x) };
                    if c != r && bad == u64::MAX {
                        bad = i;
                    }
                    n += 1;
                    i += stride;
                }
                checked.fetch_add(n, Ordering::Relaxed);
                if bad != u64::MAX {
                    first_bad.fetch_min(bad, Ordering::Relaxed);
                }
            });
        }
    });

    let bad = first_bad.load(Ordering::Relaxed);
    if bad != u64::MAX {
        let bits = bad as u32;
        let x = f32::from_bits(bits);
        let c = unsafe { c_fn(x) };
        let r = unsafe { r_fn(x) };
        panic!(
            "EXHAUSTIVE MISMATCH at bits=0x{bits:08X} (j={}, mant=0x{:06X}): \
             C=0x{c:04X} Rust=0x{r:04X}",
            (bits >> 23) & 0x1ff,
            bits & 0x007f_ffff
        );
    }

    let n = checked.load(Ordering::Relaxed);
    let expected = if stride == 1 {
        DOMAIN
    } else {
        DOMAIN / stride
    };
    assert_eq!(n, expected, "exhaustive sweep did not cover the whole domain");
    eprintln!(
        "exhaustive: {n} inputs agreed bit-for-bit (stride={stride}, threads={threads}){}",
        if stride == 1 {
            " -- COMPLETE 2^32 DOMAIN"
        } else {
            " -- strided (debug); run `cargo test --release` for the full sweep"
        }
    );
}
