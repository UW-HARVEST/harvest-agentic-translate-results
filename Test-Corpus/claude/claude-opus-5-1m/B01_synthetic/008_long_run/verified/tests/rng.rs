//! Phase B — differential tests for the `srand()`/`rand()` behaviour the port
//! reimplements in `src/rng.rs` (CONFIGS.md rows 11–14).
//!
//! Ground truth is the *real* glibc `srand`/`rand` in this process — exactly the
//! functions the C `.so` imports (`nm -D` shows `U srand@GLIBC_2.2.5`,
//! `U rand@GLIBC_2.2.5`). The Rust side is reached only through the exported
//! `harness_srand` / `harness_rand` symbols of `libdriver.so`, which drive the
//! same `rng::GlibcRand` the translated program uses.

mod common;

use common::{rust_impl, Impl, Rng, ARRAY_SIZE};

/// glibc reference sequence for `seed`.
fn glibc_sequence(seed: u32, draws: usize) -> Vec<i32> {
    unsafe {
        libc::srand(seed);
        (0..draws).map(|_| libc::rand()).collect()
    }
}

fn rust_sequence(rust: &Impl, seed: u32, draws: usize) -> Vec<i32> {
    rust.harness_srand(seed);
    (0..draws).map(|_| rust.harness_rand()).collect()
}

fn assert_sequence_matches(rust: &Impl, seed: u32, draws: usize) {
    let expected = glibc_sequence(seed, draws);
    let got = rust_sequence(rust, seed, draws);
    if expected == got {
        return;
    }
    let i = (0..draws).find(|&i| expected[i] != got[i]).unwrap();
    panic!(
        "seed {seed}: rand() stream diverges at draw {i}: glibc={} rust={} \
         (previous draw: glibc={:?} rust={:?})",
        expected[i],
        got[i],
        i.checked_sub(1).map(|j| expected[j]),
        i.checked_sub(1).map(|j| got[j]),
    );
}

/// CONFIGS.md row 11 — the seeds where glibc's seeding code branches:
/// `0` (remapped to 1), and seeds `>= 2^31` which make glibc's
/// `int32_t word = seed` negative during Schrage seeding.
#[test]
fn edge_seeds() {
    let _g = common::glibc_guard();
    let rust = rust_impl();
    let seeds: [u32; 18] = [
        0,
        1,
        2,
        3,
        7,
        42,
        127_773,
        127_774,
        2_147_483_646,
        2_147_483_647, // 2^31 - 1
        2_147_483_648, // 2^31      (int32_t word goes negative here)
        2_147_483_649,
        3_000_000_000,
        4_294_967_294,
        4_294_967_295, // UINT_MAX
        16_807,
        2_836,
        12_345,
    ];
    for seed in seeds {
        assert_sequence_matches(&rust, seed, 400);
    }
}

/// CONFIGS.md row 12 — 256 random seeds over the whole `u32` domain.
#[test]
fn random_seeds() {
    let _g = common::glibc_guard();
    let rust = rust_impl();
    let mut rng = Rng::new(0x5EED_6000);
    for _ in 0..4096 {
        let seed = rng.next_u32();
        assert_sequence_matches(&rust, seed, 400);
    }

    // Systematic: every seed 0..=2047 and every neighbourhood of a power of two
    // (the seeding recurrence is value-dependent, so small seeds matter).
    for seed in 0..=2047u32 {
        assert_sequence_matches(&rust, seed, 64);
    }
    for bit in 0..32u32 {
        let p = 1u32 << bit;
        for d in [u32::MAX, 0, 1] {
            assert_sequence_matches(&rust, p.wrapping_add(d), 64);
        }
    }
}

/// CONFIGS.md row 13 — a full `ARRAY_SIZE` sequence: precisely the program's
/// seeding loop (`for (i...) array[i] = rand();`).
#[test]
fn full_array_fill() {
    let _g = common::glibc_guard();
    let rust = rust_impl();
    for seed in [0u32, 1, 2, 42, 2_147_483_647, 2_147_483_648, 4_294_967_295] {
        assert_sequence_matches(&rust, seed, ARRAY_SIZE);
    }
}

/// CONFIGS.md row 14 — re-seeding mid-stream, and two seeds interleaved.
#[test]
fn reseeding() {
    let _g = common::glibc_guard();
    let rust = rust_impl();

    // Re-seed after a partial draw: the new stream must not depend on the old.
    for (a, b) in [(1u32, 2u32), (0, 1), (4_294_967_295, 0), (99, 99)] {
        let _ = rust_sequence(&rust, a, 37);
        assert_sequence_matches(&rust, b, 200);
    }

    // Interleave: seed A, take 10, seed B, take 10, seed A again, take 10 — the
    // third block must equal the first.
    let first = rust_sequence(&rust, 123, 10);
    let _ = rust_sequence(&rust, 456, 10);
    let third = rust_sequence(&rust, 123, 10);
    assert_eq!(first, third, "re-seeding is not deterministic");
    assert_eq!(first, glibc_sequence(123, 10), "seed 123 stream mismatch");

    // seed 0 is remapped to 1 by glibc's srandom_r; the port must do the same.
    assert_eq!(
        rust_sequence(&rust, 0, 64),
        rust_sequence(&rust, 1, 64),
        "seed 0 must behave like seed 1 (glibc remaps it)"
    );
    assert_eq!(glibc_sequence(0, 64), glibc_sequence(1, 64));
}
