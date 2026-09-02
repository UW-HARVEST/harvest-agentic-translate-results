//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Both the C `.so` and the Rust `.so` are
//! loaded with `libloading` and called through their exported `crc16` symbol;
//! results must match byte-for-byte. Randomized inputs use a fixed seed
//! (`harness::SEED`) so failures are reproducible.

mod harness;

use harness::{Impls, Rng, SEED};

// ---------------------------------------------------------------------------
// C1 — len == 0, non-empty buffer, random seeds. Pointer must be untouched.
// ---------------------------------------------------------------------------
#[test]
fn c1_len_zero_random_seeds() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED);
    let data = rng.bytes(64);
    for _ in 0..256 {
        let seed = rng.next_u16();
        let v = im.check(&data, 0, seed, "C1 len=0");
        assert_eq!(v, seed, "C1: len=0 must return the seed unchanged");
    }
    // Also the fixed extremes.
    for seed in [0x0000u16, 0x0001, 0x00FF, 0xFF00, 0xFFFF, 0x8000, 0x7FFF] {
        let v = im.check(&data, 0, seed, "C1 len=0 extreme seed");
        assert_eq!(v, seed);
    }
}

// ---------------------------------------------------------------------------
// C2 — len == 1: exhaustive over ALL 65536 seeds x ALL 256 byte values.
// Fully covers the tail loop and table[0] index range.
// ---------------------------------------------------------------------------
#[test]
fn c2_len_one_exhaustive_seed_and_byte() {
    let im = Impls::load();
    for b in 0u16..=255 {
        let data = [b as u8];
        for s in 0u32..=0xFFFF {
            let seed = s as u16;
            let cv = im.c_call(&data, 1, seed);
            let rv = im.rust_call(&data, 1, seed);
            assert_eq!(
                cv, rv,
                "C2 DIVERGENCE: byte=0x{b:02x} seed=0x{seed:04x} C=0x{cv:04x} Rust=0x{rv:04x}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C3 — len in 1..=7 (tail loop only, block loop never entered).
// ---------------------------------------------------------------------------
#[test]
fn c3_tail_only_lengths() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x03);
    for len in 1u32..=7 {
        for _ in 0..200 {
            let data = rng.bytes(len as usize);
            let seed = rng.next_u16();
            im.check(&data, len, seed, "C3 tail-only");
        }
    }
}

// ---------------------------------------------------------------------------
// C4 — len == 8 exactly: one block iteration, empty tail.
// ---------------------------------------------------------------------------
#[test]
fn c4_exactly_one_block() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x04);
    for _ in 0..2000 {
        let data = rng.bytes(8);
        let seed = rng.next_u16();
        im.check(&data, 8, seed, "C4 one block");
    }
}

// ---------------------------------------------------------------------------
// C5 — len == 8 with extreme data patterns x extreme seeds.
// ---------------------------------------------------------------------------
#[test]
fn c5_one_block_extremes() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x05);
    let patterns: Vec<Vec<u8>> = vec![
        vec![0x00; 8],
        vec![0xFF; 8],
        (0u8..8).collect(),
        (0u8..8).rev().collect(),
        vec![0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00],
        vec![0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF],
        vec![0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80],
        rng.bytes(8),
        rng.bytes(8),
    ];
    let seeds = [0x0000u16, 0xFFFF, 0xFF00, 0x00FF, 0x8000, 0x0001, 0xAB00, 0x00CD];
    for p in &patterns {
        for &seed in &seeds {
            im.check(p, 8, seed, "C5 block extremes");
        }
        for _ in 0..64 {
            let seed = rng.next_u16();
            im.check(p, 8, seed, "C5 block extremes random seed");
        }
    }
}

// ---------------------------------------------------------------------------
// C6 — len in 9..=15: one block plus each tail residue 1..=7.
// ---------------------------------------------------------------------------
#[test]
fn c6_one_block_plus_tail() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x06);
    for len in 9u32..=15 {
        for _ in 0..200 {
            let data = rng.bytes(len as usize);
            let seed = rng.next_u16();
            im.check(&data, len, seed, "C6 block+tail");
        }
    }
}

// ---------------------------------------------------------------------------
// C7 — exact multiples of 8 (multi-block, empty tail).
// ---------------------------------------------------------------------------
#[test]
fn c7_multiples_of_eight() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x07);
    for k in 2u32..=16 {
        let len = k * 8;
        for _ in 0..200 {
            let data = rng.bytes(len as usize);
            let seed = rng.next_u16();
            im.check(&data, len, seed, "C7 multiple of 8");
        }
    }
}

// ---------------------------------------------------------------------------
// C8 — 17..=127, non-multiples of 8 (multi-block + every tail residue).
// ---------------------------------------------------------------------------
#[test]
fn c8_multi_block_plus_tail() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x08);
    for len in 17u32..=127 {
        if len % 8 == 0 {
            continue;
        }
        for _ in 0..40 {
            let data = rng.bytes(len as usize);
            let seed = rng.next_u16();
            im.check(&data, len, seed, "C8 multi-block+tail");
        }
    }
}

// ---------------------------------------------------------------------------
// C9 — property-style: random len 0..=4096, random data, random seed.
// ---------------------------------------------------------------------------
#[test]
fn c9_property_random_lengths() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x09);
    for _ in 0..4000 {
        let len = rng.below(4097) as u32;
        let data = rng.bytes(len as usize);
        let seed = rng.next_u16();
        im.check(&data, len, seed, "C9 property random");
    }
}

// ---------------------------------------------------------------------------
// C10 — large buffers (many blocks; exercises the d += 8 pointer walk).
// ---------------------------------------------------------------------------
#[test]
fn c10_large_buffers() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x0A);
    for len in [65536u32, 65537, 65543, 100000, 100001, 1000003] {
        let data = rng.bytes(len as usize);
        for _ in 0..8 {
            let seed = rng.next_u16();
            im.check(&data, len, seed, "C10 large buffer");
        }
        // Extreme data patterns at scale too.
        let zeros = vec![0x00u8; len as usize];
        let ones = vec![0xFFu8; len as usize];
        im.check(&zeros, len, 0x0000, "C10 large zeros");
        im.check(&zeros, len, 0xFFFF, "C10 large zeros seed=ffff");
        im.check(&ones, len, 0x0000, "C10 large ones");
        im.check(&ones, len, 0xFFFF, "C10 large ones seed=ffff");
    }
}

// ---------------------------------------------------------------------------
// C11 — exhaustive seed sweep through the BLOCK loop (all 65536 seeds).
// Covers every table[7] / table[6] index via crc>>8 and crc&0xFF.
// ---------------------------------------------------------------------------
#[test]
fn c11_exhaustive_seeds_block_loop() {
    let im = Impls::load();
    let blocks: [[u8; 8]; 4] = [
        [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        [0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF],
        [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF],
        [0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE],
    ];
    for blk in &blocks {
        for s in 0u32..=0xFFFF {
            let seed = s as u16;
            let cv = im.c_call(blk, 8, seed);
            let rv = im.rust_call(blk, 8, seed);
            assert_eq!(
                cv, rv,
                "C11 DIVERGENCE: block={blk:02x?} seed=0x{seed:04x} C=0x{cv:04x} Rust=0x{rv:04x}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C12 — exhaustive seed sweep through the TAIL loop with multi-byte tails.
// ---------------------------------------------------------------------------
#[test]
fn c12_exhaustive_seeds_tail_loop() {
    let im = Impls::load();
    let data: [u8; 7] = [0x00, 0xFF, 0x80, 0x7F, 0x01, 0xFE, 0xAA];
    for len in [1u32, 3, 7] {
        for s in 0u32..=0xFFFF {
            let seed = s as u16;
            let cv = im.c_call(&data, len, seed);
            let rv = im.rust_call(&data, len, seed);
            assert_eq!(
                cv, rv,
                "C12 DIVERGENCE: len={len} seed=0x{seed:04x} C=0x{cv:04x} Rust=0x{rv:04x}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C13 — data-value axis: 0..=255 repeated, len swept 0..=300 so every byte
// value lands in every block position and every tail position.
// ---------------------------------------------------------------------------
#[test]
fn c13_all_byte_values_all_positions() {
    let im = Impls::load();
    let data: Vec<u8> = (0..512).map(|i| (i % 256) as u8).collect();
    let seeds = [0x0000u16, 0xFFFF, 0x1234, 0xABCD, 0x00FF, 0xFF00];
    for len in 0u32..=300 {
        for &seed in &seeds {
            im.check(&data, len, seed, "C13 all byte values");
        }
    }
    // Rotated starts, so a given value occupies a different d[k] slot.
    for rot in 1usize..8 {
        let rotated: Vec<u8> = (0..512).map(|i| ((i + rot) % 256) as u8).collect();
        for len in 0u32..=64 {
            for &seed in &seeds {
                im.check(&rotated, len, seed, "C13 rotated");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C14 — table coverage: force d[2]..d[7] to take all 256 values so tables
// [5],[4],[3],[2],[1],[0] are each indexed across their full range.
// ---------------------------------------------------------------------------
#[test]
fn c14_full_table_index_coverage() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x0E);
    // For each of the 6 value-indexed block slots (d[2]..d[7]) and each of the
    // 256 byte values, build a block where that slot holds the value.
    for slot in 2usize..8 {
        for v in 0u16..=255 {
            let mut blk = [0u8; 8];
            rng.fill(&mut blk);
            blk[slot] = v as u8;
            for &seed in &[0x0000u16, 0xFFFF, 0x5A5A] {
                im.check(&blk, 8, seed, "C14 table index coverage");
            }
        }
    }
    // And drive crc>>8 / crc&0xFF (tables [7]/[6]) across all 256 values each
    // by choosing seeds that, after the d[0]/d[1] xor, hit every index.
    for hi in 0u16..=255 {
        for lo in [0x00u16, 0x7F, 0x80, 0xFF] {
            let seed = (hi << 8) | lo;
            let blk = [0u8; 8];
            im.check(&blk, 8, seed, "C14 crc-derived index coverage");
        }
    }
}

// ---------------------------------------------------------------------------
// C15 — streaming: split a 1024-byte buffer at EVERY offset into two calls,
// feeding call 1's result in as call 2's seed. C-chained vs Rust-chained, and
// both vs the one-shot value.
// ---------------------------------------------------------------------------
#[test]
fn c15_streaming_every_split_point() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x0F);
    let data = rng.bytes(1024);
    let seed0 = 0x0000u16;

    let one_shot_c = im.c_call(&data, 1024, seed0);
    let one_shot_r = im.rust_call(&data, 1024, seed0);
    assert_eq!(one_shot_c, one_shot_r, "C15 one-shot mismatch");

    for split in 0usize..=1024 {
        let (a, b) = data.split_at(split);

        let mid_c = im.c_call(a, a.len() as u32, seed0);
        let mid_r = im.rust_call(a, a.len() as u32, seed0);
        assert_eq!(
            mid_c, mid_r,
            "C15 first-chunk mismatch at split={split}: C=0x{mid_c:04x} Rust=0x{mid_r:04x}"
        );

        let end_c = im.c_call(b, b.len() as u32, mid_c);
        let end_r = im.rust_call(b, b.len() as u32, mid_r);
        assert_eq!(
            end_c, end_r,
            "C15 second-chunk mismatch at split={split}: C=0x{end_c:04x} Rust=0x{end_r:04x}"
        );

        // Cross-check: the chained result of a split at a multiple of 8 must
        // equal the one-shot result (the block loop is resumable there).
        if split % 8 == 0 {
            assert_eq!(
                end_c, one_shot_c,
                "C15 chained != one-shot at 8-aligned split={split} (C itself)"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C16 — streaming with many random chunks.
// ---------------------------------------------------------------------------
#[test]
fn c16_streaming_random_chunks() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x10);
    for _ in 0..1000 {
        let total = rng.below(2048);
        let data = rng.bytes(total);
        let nchunks = 1 + rng.below(16);

        let mut cuts: Vec<usize> = (0..nchunks - 1).map(|_| rng.below(total + 1)).collect();
        cuts.push(0);
        cuts.push(total);
        cuts.sort_unstable();

        let seed = rng.next_u16();
        let mut cc = seed;
        let mut rc = seed;
        for w in cuts.windows(2) {
            let chunk = &data[w[0]..w[1]];
            cc = im.c_call(chunk, chunk.len() as u32, cc);
            rc = im.rust_call(chunk, chunk.len() as u32, rc);
            assert_eq!(
                cc, rc,
                "C16 chunk mismatch: total={total} chunk={}..{} C=0x{cc:04x} Rust=0x{rc:04x}",
                w[0], w[1]
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C17 — alignment independence: same payload read from offsets 0..=7.
// ---------------------------------------------------------------------------
#[test]
fn c17_alignment_offsets() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x11);
    let backing = rng.bytes(128);
    for off in 0usize..=7 {
        for len in 0u32..=56 {
            let slice = &backing[off..off + len as usize];
            for &seed in &[0x0000u16, 0xFFFF, 0x9E3Du16] {
                let cv = im.c_call(slice, len, seed);
                let rv = im.rust_call(slice, len, seed);
                assert_eq!(
                    cv, rv,
                    "C17 mismatch: off={off} len={len} seed=0x{seed:04x} C=0x{cv:04x} Rust=0x{rv:04x}"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C18 — a len==0 call chained into a non-zero call.
// ---------------------------------------------------------------------------
#[test]
fn c18_zero_length_composition() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x12);
    let data = rng.bytes(64);
    for _ in 0..500 {
        let seed = rng.next_u16();
        let len = rng.below(65) as u32;

        let c0 = im.c_call(&data, 0, seed);
        let r0 = im.rust_call(&data, 0, seed);
        assert_eq!(c0, r0);
        assert_eq!(c0, seed, "C18: len=0 must be the identity on the seed");

        let c1 = im.c_call(&data, len, c0);
        let r1 = im.rust_call(&data, len, r0);
        assert_eq!(c1, r1, "C18 chained mismatch");

        let direct_c = im.c_call(&data, len, seed);
        assert_eq!(c1, direct_c, "C18: zero-length prefix changed the result");
    }
}

// ---------------------------------------------------------------------------
// C19 — known-answer / anti-triviality. Guards against both sides being
// broken the same way (e.g. always returning the seed).
// ---------------------------------------------------------------------------
#[test]
fn c19_known_answer_and_anti_triviality() {
    let im = Impls::load();

    let check_str = b"123456789";
    let c = im.check(check_str, check_str.len() as u32, 0x0000, "C19 123456789");
    // Not a hardcoded expectation of the algorithm's identity, but the result
    // must at least not be the trivial passthrough / zero.
    assert_ne!(c, 0x0000, "C19: result must depend on the data");

    let payloads: Vec<Vec<u8>> = vec![
        b"123456789".to_vec(),
        b"The quick brown fox jumps over the lazy dog".to_vec(),
        b"a".to_vec(),
        b"abc".to_vec(),
        vec![0xFF; 1],
        vec![0xFF; 8],
        vec![0xFF; 17],
        vec![0x01; 8],
        vec![0x80; 9],
    ];
    let mut seen = std::collections::HashSet::new();
    for p in &payloads {
        let v = im.check(p, p.len() as u32, 0x0000, "C19 payload");
        seen.insert(v);
    }
    assert_eq!(
        seen.len(),
        payloads.len(),
        "C19: outputs are suspiciously non-distinct ({} unique of {}), \
         which would hide a stubbed implementation",
        seen.len(),
        payloads.len()
    );

    // The all-zero-data family legitimately maps to 0x0000 under a zero seed
    // for this CRC (no init/xorout), which is a property of the C algorithm,
    // not a stub. Assert that C and Rust agree on it rather than that it is
    // distinct.
    for n in [0u32, 1, 8, 9, 64] {
        let zeros = vec![0x00u8; n as usize];
        let v = im.check(&zeros, n, 0x0000, "C19 all-zero family");
        assert_eq!(v, 0x0000, "C19: zeros with a zero seed must stay 0x0000");
    }

    // Single-bit-flip sensitivity: flipping any bit must change the output.
    let base = vec![0x5Au8; 16];
    let base_v = im.check(&base, 16, 0x0000, "C19 base");
    for byte in 0..16usize {
        for bit in 0..8u8 {
            let mut m = base.clone();
            m[byte] ^= 1 << bit;
            let v = im.check(&m, 16, 0x0000, "C19 bitflip");
            assert_ne!(
                v, base_v,
                "C19: flipping byte {byte} bit {bit} did not change the CRC"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// C20 — adversarial byte patterns, len 0..=64.
// ---------------------------------------------------------------------------
#[test]
fn c20_adversarial_patterns() {
    let im = Impls::load();
    let mut rng = Rng::new(SEED ^ 0x14);
    let n = 96usize;
    let patterns: Vec<Vec<u8>> = vec![
        (0..n).map(|i| if i % 2 == 0 { 0x00 } else { 0xFF }).collect(),
        (0..n).map(|i| if i % 2 == 0 { 0xFF } else { 0x00 }).collect(),
        vec![0x80; n],
        vec![0x01; n],
        vec![0x7F; n],
        (0..n).map(|i| (i as u8).wrapping_mul(31)).collect(),
        (0..n).map(|i| 0x80u8 | (i as u8 & 0x0F)).collect(),
        (0..n).map(|i| if i % 8 == 0 { 0xFF } else { 0x00 }).collect(),
        (0..n).map(|i| if i % 8 == 7 { 0xFF } else { 0x00 }).collect(),
        rng.bytes(n),
    ];
    let seeds = [0x0000u16, 0xFFFF, 0x0001, 0x8000, 0x00FF, 0xFF00, 0xA5A5];
    for p in &patterns {
        for len in 0u32..=64 {
            for &seed in &seeds {
                im.check(p, len, seed, "C20 adversarial");
            }
        }
    }
}
