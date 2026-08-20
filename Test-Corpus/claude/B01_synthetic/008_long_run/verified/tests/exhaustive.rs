//! Phase B, exhaustive variant (CONFIGS.md row 29).
//!
//! `perform_expensive_operations()` applies a fixed map `f: i32 -> i32` (100
//! iterations of the inner loop body) to each element independently, so a single
//! call over the 262 144-slot `array` checks 262 144 distinct inputs. 2³² / 262 144
//! = 16 384 calls therefore cover **every possible `int` value exactly once**.
//!
//! That is ~40 minutes of compute per implementation, so the test is `#[ignore]`d
//! and shardable:
//!
//! ```sh
//! cargo build --release
//! for s in $(seq 0 15); do
//!   SHARD=$s SHARDS=16 cargo test --release --test exhaustive -- --ignored --nocapture &
//! done; wait
//! ```
//!
//! `IMPLS` (default `c-O2`) selects which C build to compare against.

mod common;

use common::{rust_impl, Impl, ARRAY_SIZE};
use std::path::PathBuf;

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

#[test]
#[ignore]
fn exhaustive_domain_sweep() {
    let shards = env_usize("SHARDS", 1).max(1);
    let shard = env_usize("SHARD", 0);
    assert!(shard < shards, "SHARD must be < SHARDS");

    let want = std::env::var("IMPLS").unwrap_or_else(|_| "c-O2".to_string());
    let mut c_impls: Vec<Impl> = Vec::new();
    for (name, path) in [
        ("c-O0", env!("C_DRIVER_SO_O0")),
        ("c-O2", env!("C_DRIVER_SO_O2")),
    ] {
        if want.split(',').any(|w| w == name) {
            c_impls.push(Impl::load(name, PathBuf::from(path)));
        }
    }
    assert!(!c_impls.is_empty(), "IMPLS={want} selected no C build");
    let rust = rust_impl();

    let chunks = (1u64 << 32) / ARRAY_SIZE as u64; // 16384
    let mut input = vec![0i32; ARRAY_SIZE];
    let mut done = 0u64;
    let t0 = std::time::Instant::now();

    for chunk in (0..chunks).filter(|c| (c % shards as u64) == shard as u64) {
        let base = (chunk * ARRAY_SIZE as u64) as u32;
        for (i, slot) in input.iter_mut().enumerate() {
            *slot = base.wrapping_add(i as u32) as i32;
        }

        rust.set_array(&input);
        rust.perform();
        let rust_out = rust.get_array();

        for c in &c_impls {
            c.set_array(&input);
            c.perform();
            let c_out = c.get_array();
            common::assert_arrays_eq(
                &format!("exhaustive chunk {chunk} (values {base}..)"),
                &c.name,
                &c_out,
                &rust_out,
                &input,
            );
        }

        done += 1;
        if done % 64 == 0 {
            eprintln!(
                "shard {shard}/{shards}: {done} chunks ({} values) in {:?}",
                done * ARRAY_SIZE as u64,
                t0.elapsed()
            );
        }
    }

    eprintln!(
        "shard {shard}/{shards} DONE: {done} chunks = {} values verified in {:?}",
        done * ARRAY_SIZE as u64,
        t0.elapsed()
    );
    let expected_min = chunks / shards as u64;
    assert!(
        done == expected_min || done == expected_min + 1,
        "unexpected chunk count {done} (expected ~{expected_min})"
    );
}
