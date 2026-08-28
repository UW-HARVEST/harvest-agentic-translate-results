//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every row calls **both** the C `.so` and the
//! Rust `.so` through their exported `crc16` symbol and compares byte-for-byte,
//! across many randomized inputs from a fixed seed.

mod common;

use common::{assert_same, pair, Rng, SEED_EXTREMES};

/// Row 1 — `len == 0`, valid pointer, **exhaustive over all 65536 seeds**.
/// Wide loop 0x, tail loop 0x: the identity path.
#[test]
fn cfg01_len_zero_all_seeds() {
    let buf = [0xAAu8; 8]; // valid, readable, never read
    for seed in 0..=u16::MAX {
        let got = assert_same(&buf[..0], seed, &format!("len=0 seed=0x{seed:04x}"));
        // The C code cannot touch `d` here, so the seed must pass through.
        assert_eq!(got, seed, "len=0 must return the seed unchanged");
    }
}

/// Row 2 — tail-loop-only lengths `1..=7` (the wide loop body never runs).
#[test]
fn cfg02_tail_only_lengths() {
    let mut rng = Rng::fixed(2);
    for len in 1usize..=7 {
        for trial in 0..2000 {
            let data = rng.bytes(len);
            let seed = rng.next_u16();
            assert_same(&data, seed, &format!("tail-only len={len} trial={trial}"));
        }
        // plus every boundary seed
        for &seed in &SEED_EXTREMES {
            let data = rng.bytes(len);
            assert_same(&data, seed, &format!("tail-only len={len} seed=0x{seed:04x}"));
        }
    }
}

/// Row 3 — `len == 8` exactly: wide loop runs once, tail runs zero times.
#[test]
fn cfg03_exactly_one_wide_block() {
    let mut rng = Rng::fixed(3);
    for trial in 0..5000 {
        let data = rng.bytes(8);
        let seed = rng.next_u16();
        assert_same(&data, seed, &format!("len=8 trial={trial}"));
    }
    for &seed in &SEED_EXTREMES {
        for pat in [0x00u8, 0xFF, 0x80, 0x01, 0x7F] {
            assert_same(&[pat; 8], seed, &format!("len=8 pat=0x{pat:02x} seed=0x{seed:04x}"));
        }
    }
}

/// Row 4 — `len ∈ 9..=15`: one wide block then a 1..7-byte tail. Exercises the
/// wide->tail handoff, where the running `crc` crosses between the two loops.
#[test]
fn cfg04_one_wide_block_plus_tail() {
    let mut rng = Rng::fixed(4);
    for len in 9usize..=15 {
        for trial in 0..2000 {
            let data = rng.bytes(len);
            let seed = rng.next_u16();
            assert_same(&data, seed, &format!("wide+tail len={len} trial={trial}"));
        }
        for &seed in &SEED_EXTREMES {
            let data = rng.bytes(len);
            assert_same(&data, seed, &format!("wide+tail len={len} seed=0x{seed:04x}"));
        }
    }
}

/// Row 5 — `len == 16`: two wide rounds, no tail. The second round consumes the
/// `crc` produced by the first, so this is the minimal state-carry test.
#[test]
fn cfg05_two_wide_blocks_no_tail() {
    let mut rng = Rng::fixed(5);
    for trial in 0..5000 {
        let data = rng.bytes(16);
        let seed = rng.next_u16();
        assert_same(&data, seed, &format!("len=16 trial={trial}"));
    }
    for &seed in &SEED_EXTREMES {
        assert_same(&[0x00; 16], seed, "len=16 zeros");
        assert_same(&[0xFF; 16], seed, "len=16 ones");
    }
}

/// Row 6 — many wide rounds, no tail (`len` a multiple of 8).
#[test]
fn cfg06_many_wide_blocks_no_tail() {
    let mut rng = Rng::fixed(6);
    for &len in &[24usize, 32, 64, 128, 256, 512, 1024, 4096] {
        for trial in 0..200 {
            let data = rng.bytes(len);
            let seed = rng.next_u16();
            assert_same(&data, seed, &format!("wide-only len={len} trial={trial}"));
        }
        for &seed in &SEED_EXTREMES {
            let data = rng.bytes(len);
            assert_same(&data, seed, &format!("wide-only len={len} seed=0x{seed:04x}"));
        }
    }
}

/// Row 7 — dense sweep of **every** length `0..=520`, covering all 8 residues at
/// 65 different wide-iteration counts (the whole A x B x C grid).
#[test]
fn cfg07_dense_length_sweep_0_to_520() {
    let mut rng = Rng::fixed(7);
    for len in 0usize..=520 {
        for trial in 0..12 {
            let data = rng.bytes(len);
            let seed = rng.next_u16();
            assert_same(&data, seed, &format!("sweep len={len} trial={trial}"));
        }
        // fixed content patterns at each length, so failures are reproducible
        let inc: Vec<u8> = (0..len).map(|i| i as u8).collect();
        assert_same(&inc, 0x0000, &format!("sweep len={len} inc seed=0"));
        assert_same(&inc, 0xFFFF, &format!("sweep len={len} inc seed=ffff"));
    }
}

/// Row 8 — **exhaustive per lane**: each of the 8 bytes of a wide block indexes a
/// *different* table (`d[0]`,`d[1]` fold into the seed; `d[2]`->T5, `d[3]`->T4,
/// `d[4]`->T3, `d[5]`->T2, `d[6]`->T1, `d[7]`->T0). Varying one lane at a time
/// over all 256 values pins every table to its lane, including index 255.
#[test]
fn cfg08_every_byte_value_in_every_lane() {
    let base: [u8; 8] = [0x5A, 0xA5, 0x3C, 0xC3, 0x0F, 0xF0, 0x69, 0x96];
    for lane in 0..8usize {
        for v in 0..=255u8 {
            let mut d = base;
            d[lane] = v;
            for &seed in &[0x0000u16, 0xFFFF, 0x1234, 0xABCD, 0x00FF, 0xFF00] {
                assert_same(&d, seed, &format!("lane={lane} v=0x{v:02x} seed=0x{seed:04x}"));
            }
        }
    }
    // Also the all-lanes-equal diagonal over all 256 values.
    for v in 0..=255u8 {
        assert_same(&[v; 8], 0x0000, &format!("all-lanes v=0x{v:02x}"));
        assert_same(&[v; 8], 0xFFFF, &format!("all-lanes v=0x{v:02x} seed=ffff"));
    }
}

/// Row 9 — **exhaustive tail table**: `len == 1` over every byte value and every
/// value of `crc >> 8`, pinning all 256 entries of `tables[0]` reachable from the
/// tail loop, and the truncating `crc << 8`.
#[test]
fn cfg09_tail_table_exhaustive() {
    for byte in 0..=255u8 {
        for hi in 0..=255u16 {
            // sweep the high byte (the table index source) and vary the low byte
            let seed = (hi << 8) | (hi ^ 0x5A);
            assert_same(&[byte], seed, &format!("tail byte=0x{byte:02x} seed=0x{seed:04x}"));
        }
    }
}

/// Row 10 — seed extremes across every length class.
#[test]
fn cfg10_seed_extremes_across_lengths() {
    let mut rng = Rng::fixed(10);
    for &seed in &SEED_EXTREMES {
        for len in 0usize..=24 {
            for trial in 0..40 {
                let data = rng.bytes(len);
                assert_same(&data, seed, &format!("seed=0x{seed:04x} len={len} t={trial}"));
            }
        }
    }
    // Every possible seed, at one length per residue class.
    for len in [0usize, 1, 7, 8, 9, 15, 16, 17] {
        let data: Vec<u8> = (0..len).map(|i| (i * 37 + 11) as u8).collect();
        for seed in 0..=u16::MAX {
            assert_same(&data, seed, &format!("all-seeds len={len} seed=0x{seed:04x}"));
        }
    }
}

/// Row 11 — degenerate / structured content patterns.
#[test]
fn cfg11_degenerate_content_patterns() {
    for len in 0usize..=64 {
        let patterns: Vec<(String, Vec<u8>)> = vec![
            ("zeros".into(), vec![0x00; len]),
            ("ones".into(), vec![0xFF; len]),
            ("inc".into(), (0..len).map(|i| i as u8).collect()),
            ("dec".into(), (0..len).map(|i| (255 - (i & 0xFF)) as u8).collect()),
            ("alt".into(), (0..len).map(|i| if i % 2 == 0 { 0x00 } else { 0xFF }).collect()),
            ("0x80".into(), vec![0x80; len]),
            ("0x01".into(), vec![0x01; len]),
        ];
        for (name, data) in &patterns {
            for &seed in &SEED_EXTREMES {
                assert_same(data, seed, &format!("pat={name} len={len} seed=0x{seed:04x}"));
            }
        }
        // single bit set at every bit position in the buffer
        for bit in 0..(len * 8) {
            let mut data = vec![0u8; len];
            data[bit / 8] = 1u8 << (bit % 8);
            assert_same(&data, 0x0000, &format!("single-bit {bit} len={len}"));
            assert_same(&data, 0xFFFF, &format!("single-bit {bit} len={len} seed=ffff"));
        }
    }
}

/// Row 12 — broad property-style randomized sweep.
#[test]
fn cfg12_property_random_full_range() {
    let mut rng = Rng::fixed(12);
    for trial in 0..20_000 {
        let len = rng.below(1025);
        let data = rng.bytes(len);
        let seed = rng.next_u16();
        assert_same(&data, seed, &format!("random trial={trial} len={len}"));
    }
}

/// Row 13 — composed pipeline: stream a message **one byte at a time**, chaining
/// the result in as the next seed (512 consecutive tail-only calls). Compares the
/// C chain against the Rust chain, and both against the one-shot value.
#[test]
fn cfg13_stream_one_byte_at_a_time() {
    let p = pair();
    let mut rng = Rng::fixed(13);
    for trial in 0..20 {
        let msg = rng.bytes(512);
        let seed = rng.next_u16();

        let mut c_acc = seed;
        let mut r_acc = seed;
        for (i, b) in msg.iter().enumerate() {
            c_acc = p.c.crc16(&[*b], c_acc);
            r_acc = p.rust.crc16(&[*b], r_acc);
            assert_eq!(c_acc, r_acc, "stream-by-1 diverged at byte {i} (trial {trial})");
        }
        let one_shot = assert_same(&msg, seed, &format!("stream-by-1 one-shot trial={trial}"));
        assert_eq!(c_acc, one_shot, "byte-at-a-time chain != one-shot (C is ground truth)");
    }
}

/// Row 14 — composed pipeline in 8-byte chunks (all wide, no tail, chained).
#[test]
fn cfg14_stream_eight_byte_chunks() {
    let p = pair();
    let mut rng = Rng::fixed(14);
    for trial in 0..50 {
        let msg = rng.bytes(8 * 64);
        let seed = rng.next_u16();
        let mut c_acc = seed;
        let mut r_acc = seed;
        for (i, chunk) in msg.chunks(8).enumerate() {
            c_acc = p.c.crc16(chunk, c_acc);
            r_acc = p.rust.crc16(chunk, r_acc);
            assert_eq!(c_acc, r_acc, "stream-by-8 diverged at chunk {i} (trial {trial})");
        }
        let one_shot = assert_same(&msg, seed, &format!("stream-by-8 one-shot trial={trial}"));
        assert_eq!(c_acc, one_shot, "8-byte chain != one-shot");
    }
}

/// Row 15 — composed pipeline split at **every** offset: `crc16(tail,
/// crc16(head, seed))` must equal the one-shot CRC for every split point, which
/// forces every combination of (wide count, tail residue) on both sides.
#[test]
fn cfg15_stream_split_at_every_offset() {
    let p = pair();
    let mut rng = Rng::fixed(15);
    for &n in &[0usize, 1, 7, 8, 9, 16, 17, 23, 24, 31, 33, 64, 65, 127, 128, 129, 200] {
        let msg = rng.bytes(n);
        let seed = rng.next_u16();
        let one_shot = assert_same(&msg, seed, &format!("split n={n} one-shot"));
        for k in 0..=n {
            let c_mid = p.c.crc16(&msg[..k], seed);
            let r_mid = p.rust.crc16(&msg[..k], seed);
            assert_eq!(c_mid, r_mid, "split n={n} k={k}: head diverged");

            let c_end = p.c.crc16(&msg[k..], c_mid);
            let r_end = p.rust.crc16(&msg[k..], r_mid);
            assert_eq!(c_end, r_end, "split n={n} k={k}: tail diverged");
            assert_eq!(c_end, one_shot, "split n={n} k={k}: chain != one-shot (C)");
        }
    }
}

/// Row 16 — composed pipeline with random N-way chunking.
#[test]
fn cfg16_stream_random_chunk_sequences() {
    let p = pair();
    let mut rng = Rng::fixed(16);
    for trial in 0..2000 {
        let n = rng.below(300);
        let msg = rng.bytes(n);
        let seed = rng.next_u16();
        let one_shot = assert_same(&msg, seed, &format!("chunked trial={trial} n={n}"));

        let mut c_acc = seed;
        let mut r_acc = seed;
        let mut off = 0usize;
        let mut step = 0;
        while off < n {
            let remaining = n - off;
            let take = 1 + rng.below(remaining.min(20));
            let chunk = &msg[off..off + take];
            c_acc = p.c.crc16(chunk, c_acc);
            r_acc = p.rust.crc16(chunk, r_acc);
            assert_eq!(
                c_acc, r_acc,
                "chunked trial={trial} step={step} off={off} take={take} diverged"
            );
            off += take;
            step += 1;
        }
        assert_eq!(c_acc, one_shot, "chunked trial={trial}: chain != one-shot");
    }
}

/// Row 17 — unaligned start offsets. The C code reads `d` bytewise and makes no
/// alignment assumption; the Rust code builds a slice from the raw pointer. Read
/// the *same logical bytes* starting at offsets 0..=7 of an over-allocated
/// buffer to catch any slice-origin or alignment assumption.
#[test]
fn cfg17_unaligned_start_offsets() {
    let p = pair();
    let mut rng = Rng::fixed(17);
    let mut backing = vec![0u8; 8 + 64 + 8];
    for off in 0..=7usize {
        for len in 0usize..=64 {
            for trial in 0..8 {
                rng.fill(&mut backing);
                let sub = &backing[off..off + len];
                let seed = rng.next_u16();
                let ctx = format!("unaligned off={off} len={len} t={trial}");
                let v = assert_same(sub, seed, &ctx);

                // Same bytes copied to a fresh, differently-aligned allocation
                // must give the same answer.
                let copy = sub.to_vec();
                let c2 = p.c.crc16(&copy, seed);
                let r2 = p.rust.crc16(&copy, seed);
                assert_eq!(c2, v, "C depends on buffer address? {ctx}");
                assert_eq!(r2, v, "Rust depends on buffer address? {ctx}");
            }
        }
    }
}

/// Row 18 — large buffers with `len` beyond the `u16` range, exercising the
/// wrapper's `len as usize` widening (a `len as u16` narrowing bug would show up
/// here as a wrong CRC).
#[test]
fn cfg18_large_buffers_beyond_u16() {
    let mut rng = Rng::fixed(18);
    let mut buf = vec![0u8; 0x1_0000 + 64];
    rng.fill(&mut buf);

    for &len in &[
        0xFFFEusize,
        0xFFFF,
        0x1_0000,
        0x1_0001,
        0x1_0007,
        0x1_0008,
        0x1_0009,
        0x1_000F,
        0x1_0010,
        0x1_0040,
    ] {
        for &seed in &[0x0000u16, 0xFFFF, 0x1234, 0xBEEF] {
            assert_same(&buf[..len], seed, &format!("large len={len} seed=0x{seed:04x}"));
        }
    }
}
