//! Phase B — the exhaustive rows of `CONFIGS.md` (C20 and C22).
//!
//! `hdr_compare` reads exactly five bytes: `h2[0..3]`, `h1[1]` and `h1[2]` (`h1[0]` is never
//! touched). `h2[0]` only ever enters through `h2[0] == 0xff`, so pinning it to `0xFF` and
//! enumerating the other four bytes covers the entire *reachable* behaviour of the function;
//! the `h2[0] != 0xFF` half is covered exhaustively by `c21`/`e1`/`e6`.

mod common;

use common::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Sweeps `h2[1] ∈ b1_values`, and all 256 values each of `h2[2]`, `h1[1]`, `h1[2]`,
/// with `h2[0]` pinned to `b0`.
fn sweep_shard(b0: u8, b1_values: &[u8]) -> u64 {
    let l = libs();
    let mut h1 = [0u8; 3];
    let mut h2 = [b0, 0, 0];
    let mut matches = 0u64;
    for &b1 in b1_values {
        h2[1] = b1;
        for c2 in 0..=255u8 {
            h2[2] = c2;
            for a1 in 0..=255u8 {
                h1[1] = a1;
                for a2 in 0..=255u8 {
                    h1[2] = a2;
                    let a = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
                    let b = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
                    if a != b {
                        panic!(
                            "DIVERGENCE h1=[--,{a1:02X},{a2:02X}] \
                             h2=[{b0:02X},{b1:02X},{c2:02X}]: C = {a}, Rust = {b}"
                        );
                    }
                    if a != 0 && a != 1 {
                        panic!("non-boolean result {a} for h1[1]={a1:02X} h1[2]={a2:02X}");
                    }
                    let m = model(&h1, &h2);
                    if a != m {
                        panic!(
                            "model mismatch h1=[--,{a1:02X},{a2:02X}] \
                             h2=[{b0:02X},{b1:02X},{c2:02X}]: so = {a}, model = {m}"
                        );
                    }
                    matches += a as u64;
                }
            }
        }
    }
    matches
}

fn parallel_sweep(b0: u8, b1_values: Vec<u8>) -> u64 {
    let nthreads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
        .min(b1_values.len().max(1))
        .min(16);
    let total = Arc::new(AtomicU64::new(0));
    let values = Arc::new(b1_values);
    let mut handles = Vec::new();
    for t in 0..nthreads {
        let values = Arc::clone(&values);
        let total = Arc::clone(&total);
        handles.push(std::thread::spawn(move || {
            let shard: Vec<u8> = values
                .iter()
                .enumerate()
                .filter(|(i, _)| i % nthreads == t)
                .map(|(_, &v)| v)
                .collect();
            let m = sweep_shard(b0, &shard);
            total.fetch_add(m, Ordering::Relaxed);
        }));
    }
    for h in handles {
        h.join().expect("sweep shard panicked (see the divergence above)");
    }
    total.load(Ordering::Relaxed)
}

/// C20 — full cross-product over the *valid* `h2` configuration space
/// (14 valid `h2[1]` x 180 valid `h2[2]` x 256 `h1[1]` x 256 `h1[2]` = 165 M cases).
#[test]
fn c20_valid_h2_full_cross_product() {
    let l = libs();
    // Only 14 valid byte-1 values exist: always sweep all of them.
    let vb1: Vec<u8> = valid_byte1();
    let vb2: Vec<u8> = valid_byte2().into_iter().step_by(stride()).collect();
    let mut matches = 0u64;
    let mut h1 = [0u8; 3];
    let mut h2 = [0xFFu8, 0, 0];
    for &b1 in vb1.iter() {
        h2[1] = b1;
        for &c2 in vb2.iter() {
            h2[2] = c2;
            for a1 in 0..=255u8 {
                h1[1] = a1;
                for a2 in 0..=255u8 {
                    h1[2] = a2;
                    let a = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
                    let b = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
                    if a != b {
                        panic!(
                            "DIVERGENCE h1=[--,{a1:02X},{a2:02X}] h2=[FF,{b1:02X},{c2:02X}]: \
                             C = {a}, Rust = {b}"
                        );
                    }
                    let m = model(&h1, &h2);
                    if a != m {
                        panic!("model mismatch h2=[FF,{b1:02X},{c2:02X}]: so = {a}, model = {m}");
                    }
                    matches += a as u64;
                }
            }
        }
    }
    // Sanity: the valid space must produce a non-trivial number of matches.
    assert!(matches > 0, "expected some matches in the valid cross-product");
    println!(
        "c20: {} x {} x 256 x 256 = {} cases, {matches} matches",
        vb1.len(),
        vb2.len(),
        vb1.len() as u64 * vb2.len() as u64 * 65536,
    );
    if full_size() {
        assert_eq!(matches, 283_584, "match count over the valid cross-product changed");
    }
}

/// C22 — complete sweep of the reachable input space: `h2[0] = 0xFF` and all 2^32
/// combinations of `h2[1]`, `h2[2]`, `h1[1]`, `h1[2]`.
///
/// This is the whole behaviour of the function, exhaustively. Runs by default; set
/// `HDR_FULL_SWEEP=0` to skip it.
#[test]
fn c22_full_2p32_sweep() {
    if std::env::var("HDR_FULL_SWEEP").map(|v| v == "0").unwrap_or(false) {
        println!("c22: skipped (HDR_FULL_SWEEP=0)");
        return;
    }
    let start = std::time::Instant::now();
    let b1s = byte_range();
    let matches = parallel_sweep(0xFF, b1s.clone());
    println!(
        "c22: {} x 256 x 256 x 256 = {} cases in {:?}, {matches} matches",
        b1s.len(),
        b1s.len() as u64 * 16_777_216,
        start.elapsed()
    );
    assert!(matches > 0);
    // The valid cross-product (c20) must account for every match in the whole space.
    if full_size() {
        assert_eq!(
            matches, 283_584,
            "match count for the h2[0]=0xFF space changed; re-derive CONFIGS.md"
        );
    }
}

/// C34 — the `h2[0] != 0xFF` half of the space, exhaustively for representative sync bytes:
/// every one of the 2^32 `(h2[1], h2[2], h1[1], h1[2])` combinations must be rejected.
#[test]
fn c34_non_sync_byte0_full_sweeps() {
    for b0 in [0x00u8, 0x01, 0x7F, 0xFE] {
        let matches = parallel_sweep(b0, byte_range());
        assert_eq!(
            matches, 0,
            "h2[0] = {b0:#04X} is not the sync byte, nothing may match"
        );
    }
}

/// C35 — all 256 `h2[0]` values against the full 2^16 `(h2[1], h2[2])` space and a battery
/// of `h1` patterns. Cheap, and it touches every possible sync-byte value.
#[test]
fn c35_all_byte0_values_vs_h1_battery() {
    let l = libs();
    let mut h2 = [0u8; 3];
    for b0 in byte_range() {
        h2[0] = b0;
        for b1 in 0..=255u8 {
            h2[1] = b1;
            for c2 in 0..=255u8 {
                h2[2] = c2;
                for h1 in H1_BATTERY.iter() {
                    let a = unsafe { (l.c)(h1.as_ptr(), h2.as_ptr()) };
                    let b = unsafe { (l.rs)(h1.as_ptr(), h2.as_ptr()) };
                    if a != b {
                        panic!("DIVERGENCE h1={h1:02X?} h2={h2:02X?}: C = {a}, Rust = {b}");
                    }
                    if b0 != 0xFF && a != 0 {
                        panic!("h2[0]={b0:#04X} must never match, got {a} for {h1:02X?}");
                    }
                }
            }
        }
    }
}

/// C36 — the complete 2^40 sweep of every byte the C reads (`h2[0..3]`, `h1[1]`, `h1[2]`).
/// This is total exhaustive equivalence. Opt-in because it takes ~25 min:
/// `HDR_SWEEP_2P40=1 cargo test --release --test exhaustive c36 -- --nocapture`
#[test]
fn c36_complete_2p40_sweep() {
    if std::env::var("HDR_SWEEP_2P40").map(|v| v != "1").unwrap_or(true) {
        println!("c36: skipped (set HDR_SWEEP_2P40=1 to run the complete 2^40 sweep)");
        return;
    }
    let start = std::time::Instant::now();
    let mut total = 0u64;
    for b0 in 0..=255u8 {
        let m = parallel_sweep(b0, byte_range());
        if b0 != 0xFF {
            assert_eq!(m, 0, "h2[0] = {b0:#04X} must never match");
        } else if full_size() {
            assert_eq!(m, 283_584);
        }
        total += m;
        println!("c36: h2[0] = {b0:#04X} done ({m} matches, {:?} elapsed)", start.elapsed());
    }
    println!("c36: 2^40 = 1,099,511,627,776 cases in {:?}, {total} matches", start.elapsed());
    if full_size() {
        assert_eq!(total, 283_584);
    }
}
