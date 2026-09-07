//! CONFIGS.md row 34 — exhaustive differential test over the ENTIRE input
//! domain: all 2^32 `f32` bit patterns, through both `.so` exports.
//!
//! `float2half`'s only input is its 32-bit argument, so this is a total proof
//! of equivalence for this ABI: it subsumes every other Phase B row and every
//! Phase C row. The work is split across worker threads; each thread loads its
//! own handles to both libraries (both functions are pure and stateless).
//!
//! Run with `cargo test --release` for the fast path. The sharded design keeps
//! the wall clock well under the 600 s budget.

mod harness;

use harness::Pair;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[test]
fn row34_exhaustive_all_2pow32_bit_patterns() {
    // Sanity-check the loader once on the main thread first.
    {
        let p = Pair::load();
        p.assert_bits(0, "exhaustive preflight");
        eprintln!("C   .so: {}", p.c_path.display());
        eprintln!("Rust.so: {}", p.rust_path.display());
    }

    let threads: u32 = std::thread::available_parallelism()
        .map(|n| n.get() as u32)
        .unwrap_or(4)
        .clamp(1, 32);

    // 2^32 patterns split into `threads` contiguous shards.
    const TOTAL: u64 = 1u64 << 32;
    let chunk = TOTAL / threads as u64;

    let mismatches = Arc::new(AtomicU64::new(0));
    let first_bad = Arc::new(AtomicU64::new(u64::MAX));
    let checked = Arc::new(AtomicU64::new(0));

    let mut handles = Vec::new();
    for t in 0..threads {
        let start = t as u64 * chunk;
        let end = if t == threads - 1 { TOTAL } else { start + chunk };
        let mismatches = Arc::clone(&mismatches);
        let first_bad = Arc::clone(&first_bad);
        let checked = Arc::clone(&checked);

        handles.push(std::thread::spawn(move || {
            let p = Pair::load();
            let c = p.c;
            let rs = p.rust;
            let mut local_bad = 0u64;
            let mut local_first = u64::MAX;

            for bits64 in start..end {
                let bits = bits64 as u32;
                let x = f32::from_bits(bits);
                // SAFETY: both symbols were type-checked as
                // `extern "C" fn(f32) -> u16` when the library was loaded.
                let (a, b) = unsafe { (c(x), rs(x)) };
                if a != b {
                    local_bad += 1;
                    if local_first == u64::MAX {
                        local_first = bits64;
                    }
                }
            }

            checked.fetch_add(end - start, Ordering::Relaxed);
            if local_bad > 0 {
                mismatches.fetch_add(local_bad, Ordering::Relaxed);
                first_bad.fetch_min(local_first, Ordering::Relaxed);
            }
        }));
    }

    for h in handles {
        h.join().expect("exhaustive worker thread panicked");
    }

    let n = checked.load(Ordering::Relaxed);
    assert_eq!(n, TOTAL, "did not cover the whole 2^32 domain (covered {n})");

    let bad = mismatches.load(Ordering::Relaxed);
    if bad != 0 {
        let fb = first_bad.load(Ordering::Relaxed) as u32;
        let p = Pair::load();
        let (c, r) = p.both_bits(fb);
        panic!(
            "EXHAUSTIVE DIVERGENCE: {bad} of {TOTAL} inputs differ. \
             First: bits 0x{fb:08X} (f32 {:e}) -> C 0x{c:04X} != Rust 0x{r:04X}",
            f32::from_bits(fb)
        );
    }

    eprintln!("exhaustive: all {TOTAL} f32 bit patterns agree ({threads} shards)");
}
