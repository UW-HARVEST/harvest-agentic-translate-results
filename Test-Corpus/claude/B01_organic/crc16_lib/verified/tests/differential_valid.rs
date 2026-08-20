//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test drives BOTH the C `.so` and the
//! Rust `.so` through their exported `crc16` symbol and asserts byte-identical
//! (`u16`-identical) results across many randomized inputs from a fixed seed.

mod common;

use common::{libs, Rng, SEED};

/// Seeds that probe the boundaries of every place `crc` is used as an index or
/// shifted: `crc >> 8`, `crc & 0xFF`, `crc << 8`.
const BOUNDARY_SEEDS: [u16; 8] = [
    0x0000, 0xFFFF, 0x00FF, 0xFF00, 0x0001, 0x8000, 0x1234, 0xABCD,
];

// ---------------------------------------------------------------- C1
#[test]
fn c1_len_zero_all_seeds() {
    let l = libs();
    let buf = [0xAAu8; 16];
    for &s in &BOUNDARY_SEEDS {
        let got = l.assert_same(&buf[..0], s, "C1 boundary seed");
        // The C never touches either loop, so the seed passes straight through.
        assert_eq!(got, s, "C1: len=0 must return the seed unchanged");
    }
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..64 {
        let s = rng.next_u16();
        let got = l.assert_same(&buf[..0], s, "C1 random seed");
        assert_eq!(got, s);
    }
}

// ---------------------------------------------------------------- C2
#[test]
fn c2_tail_only_lengths_1_to_7() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 2);
    for len in 1..=7usize {
        for _ in 0..256 {
            let data = rng.bytes(len);
            let seed = rng.next_u16();
            l.assert_same(&data, seed, &format!("C2 tail-only len={len}"));
        }
        for &s in &BOUNDARY_SEEDS {
            let data = rng.bytes(len);
            l.assert_same(&data, s, &format!("C2 tail-only len={len} boundary seed"));
        }
    }
}

// ---------------------------------------------------------------- C3
#[test]
fn c3_exactly_one_block() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..512 {
        let data = rng.bytes(8);
        let seed = rng.next_u16();
        l.assert_same(&data, seed, "C3 exactly one block (len=8)");
    }
    for &s in &BOUNDARY_SEEDS {
        for _ in 0..64 {
            let data = rng.bytes(8);
            l.assert_same(&data, s, "C3 len=8 boundary seed");
        }
    }
}

// ---------------------------------------------------------------- C4
#[test]
fn c4_one_block_plus_tail() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 4);
    for len in 9..=15usize {
        for _ in 0..256 {
            let data = rng.bytes(len);
            let seed = rng.next_u16();
            l.assert_same(&data, seed, &format!("C4 one block + tail len={len}"));
        }
        for &s in &BOUNDARY_SEEDS {
            let data = rng.bytes(len);
            l.assert_same(&data, s, &format!("C4 len={len} boundary seed"));
        }
    }
}

// ---------------------------------------------------------------- C5
#[test]
fn c5_multi_block_no_tail() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 5);
    for len in [16usize, 24, 32, 64, 128, 256] {
        for _ in 0..256 {
            let data = rng.bytes(len);
            let seed = rng.next_u16();
            l.assert_same(&data, seed, &format!("C5 multi-block len={len}"));
        }
        for &s in &BOUNDARY_SEEDS {
            let data = rng.bytes(len);
            l.assert_same(&data, s, &format!("C5 len={len} boundary seed"));
        }
    }
}

// ---------------------------------------------------------------- C6
#[test]
fn c6_multi_block_plus_tail() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 6);
    // Every residue mod 8, across several block counts.
    for len in 17..=71usize {
        for _ in 0..64 {
            let data = rng.bytes(len);
            let seed = rng.next_u16();
            l.assert_same(&data, seed, &format!("C6 multi-block + tail len={len}"));
        }
    }
    for len in 17..=71usize {
        for &s in &BOUNDARY_SEEDS {
            let data = rng.bytes(len);
            l.assert_same(&data, s, &format!("C6 len={len} boundary seed"));
        }
    }
}

// ---------------------------------------------------------------- C7
#[test]
fn c7_all_zero_bytes() {
    let l = libs();
    let buf = [0x00u8; 72];
    for len in 0..=72usize {
        for &s in &[0x0000u16, 0xFFFF, 0x1234, 0x00FF, 0xFF00] {
            l.assert_same(&buf[..len], s, &format!("C7 all-0x00 len={len}"));
        }
    }
}

// ---------------------------------------------------------------- C8
#[test]
fn c8_all_ff_bytes() {
    let l = libs();
    let buf = [0xFFu8; 72];
    for len in 0..=72usize {
        for &s in &[0x0000u16, 0xFFFF, 0x1234, 0x00FF, 0xFF00] {
            l.assert_same(&buf[..len], s, &format!("C8 all-0xFF len={len}"));
        }
    }
}

// ---------------------------------------------------------------- C9
#[test]
fn c9_full_byte_ramp_all_lengths() {
    let l = libs();
    // 0,1,2,...,255 — guarantees every one of the 256 index slots of every one
    // of the 8 tables is exercised at some offset.
    let ramp: Vec<u8> = (0..=255u8).collect();
    for len in 0..=256usize {
        for &s in &[0x0000u16, 0xFFFF] {
            l.assert_same(&ramp[..len], s, &format!("C9 ramp len={len}"));
        }
    }
    // Rotations, so each byte value also lands in every lane of the 8-byte block.
    for rot in 1..8usize {
        let mut r = ramp.clone();
        r.rotate_left(rot);
        for &s in &BOUNDARY_SEEDS {
            l.assert_same(&r, s, &format!("C9 ramp rot={rot}"));
        }
    }
}

// ---------------------------------------------------------------- C10
#[test]
fn c10_exhaustive_seed_sweep() {
    let l = libs();
    let eight = [0x12u8, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0];
    let three = [0xDEu8, 0xAD, 0xBE];
    let eleven = [0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B];
    for seed in 0..=u16::MAX {
        l.assert_same(&eight, seed, "C10 exhaustive seed / len=8");
        l.assert_same(&three, seed, "C10 exhaustive seed / len=3");
        l.assert_same(&eleven, seed, "C10 exhaustive seed / len=11");
        l.assert_same(&[], seed, "C10 exhaustive seed / len=0");
    }
}

// ---------------------------------------------------------------- C11
#[test]
fn c11_exhaustive_single_byte() {
    let l = libs();
    for b in 0..=255u8 {
        for &s in &[0x0000u16, 0xFFFF, 0xABCD, 0x00FF, 0xFF00, 0x0100] {
            l.assert_same(&[b], s, &format!("C11 single byte 0x{b:02x}"));
        }
    }
    // All 65536 seeds against the two extreme byte values.
    for seed in 0..=u16::MAX {
        l.assert_same(&[0x00], seed, "C11 byte 0x00, all seeds");
        l.assert_same(&[0xFF], seed, "C11 byte 0xFF, all seeds");
    }
}

// ---------------------------------------------------------------- C12
#[test]
fn c12_chained_aligned_splits() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..500 {
        let blocks = 1 + rng.below(9); // 1..=9 blocks of 8 bytes
        let data = rng.bytes(blocks * 8);
        let seed = rng.next_u16();

        // Split at a random 8-aligned point.
        let cut = 8 * rng.below(blocks + 1);
        let (a, b) = data.split_at(cut);
        let (c_chained, r_chained) = l.chained(&[a, b], seed);
        assert_eq!(
            c_chained, r_chained,
            "C12 divergence: len={} cut={cut} seed=0x{seed:04x}",
            data.len()
        );

        // And the one-shot must agree with the chained result on both libs.
        let one_shot = l.assert_same(&data, seed, "C12 one-shot");
        assert_eq!(
            one_shot, c_chained,
            "C12: chaining at an 8-aligned split must equal the one-shot CRC"
        );
    }
}

// ---------------------------------------------------------------- C13
#[test]
fn c13_chained_unaligned_splits() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 13);
    for _ in 0..1000 {
        let len = rng.below(200);
        let data = rng.bytes(len);
        let seed = rng.next_u16();

        // 1..=5 chunks split at arbitrary (mostly unaligned) points.
        let nchunks = 1 + rng.below(5);
        let mut cuts: Vec<usize> = (0..nchunks - 1).map(|_| rng.below(len + 1)).collect();
        cuts.sort_unstable();
        let mut chunks: Vec<&[u8]> = Vec::new();
        let mut prev = 0usize;
        for &c in &cuts {
            chunks.push(&data[prev..c]);
            prev = c;
        }
        chunks.push(&data[prev..]);

        let (c_res, r_res) = l.chained(&chunks, seed);
        assert_eq!(
            c_res, r_res,
            "C13 divergence: len={len} cuts={cuts:?} seed=0x{seed:04x}"
        );

        let one_shot = l.assert_same(&data, seed, "C13 one-shot");
        assert_eq!(
            one_shot, c_res,
            "C13: chaining at arbitrary splits must equal the one-shot CRC \
             (len={len} cuts={cuts:?} seed=0x{seed:04x})"
        );
    }
}

// ---------------------------------------------------------------- C14
#[test]
fn c14_byte_at_a_time_chaining() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..300 {
        let len = rng.below(80);
        let data = rng.bytes(len);
        let seed = rng.next_u16();

        let mut c = seed;
        let mut r = seed;
        for &b in &data {
            c = l.c(&[b], c);
            r = l.rust(&[b], r);
            assert_eq!(
                c, r,
                "C14 divergence mid-stream: seed=0x{seed:04x} byte=0x{b:02x}"
            );
        }
        let one_shot = l.assert_same(&data, seed, "C14 one-shot");
        assert_eq!(
            one_shot, c,
            "C14: byte-at-a-time chaining must equal the one-shot CRC (len={len})"
        );
    }
}

// ---------------------------------------------------------------- C15
#[test]
fn c15_block_vs_tail_paths_agree() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 15);
    // len=8k goes exclusively through the slice-by-8 body; feeding the same
    // bytes one at a time goes exclusively through the tail loop. Both
    // libraries must produce the same value *and* the two paths must agree.
    for blocks in 1..=6usize {
        for _ in 0..200 {
            let data = rng.bytes(blocks * 8);
            let seed = rng.next_u16();

            let block_c = l.c(&data, seed);
            let block_r = l.rust(&data, seed);
            assert_eq!(block_c, block_r, "C15 block-path divergence");

            let mut tail_c = seed;
            let mut tail_r = seed;
            for &b in &data {
                tail_c = l.c(&[b], tail_c);
                tail_r = l.rust(&[b], tail_r);
            }
            assert_eq!(tail_c, tail_r, "C15 tail-path divergence");
            assert_eq!(
                block_c, tail_c,
                "C15: slice-by-8 body and tail loop must compute the same CRC \
                 (blocks={blocks} seed=0x{seed:04x})"
            );
        }
    }
}

// ---------------------------------------------------------------- C16
#[test]
fn c16_unaligned_buffer_offsets() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 16);
    let backing = rng.bytes(512);
    for off in [0usize, 1, 2, 3, 5, 7, 9, 15] {
        for len in 0..=24usize {
            for &s in &BOUNDARY_SEEDS {
                let slice = &backing[off..off + len];
                l.assert_same(slice, s, &format!("C16 offset={off} len={len}"));
            }
        }
    }
    // Larger unaligned spans too.
    for off in [1usize, 3, 7] {
        for &s in &[0x0000u16, 0xFFFF] {
            let slice = &backing[off..off + 300];
            l.assert_same(slice, s, &format!("C16 offset={off} len=300"));
        }
    }
}

// ---------------------------------------------------------------- C17
#[test]
fn c17_large_buffers() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 17);
    for len in [1024usize, 64 * 1024, 1024 * 1024] {
        let data = rng.bytes(len);
        for &s in &[0x0000u16, 0xFFFF, 0x1234] {
            l.assert_same(&data, s, &format!("C17 large len={len}"));
        }
        // Also a couple of odd lengths near the block boundary of a big buffer.
        for extra in [1usize, 7] {
            l.assert_same(&data[..len - extra], 0xBEEF, &format!("C17 len={}", len - extra));
        }
    }
}

// ---------------------------------------------------------------- C18
#[test]
fn c18_randomized_fuzz() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 18);
    for i in 0..20_000 {
        let len = rng.below(1025);
        let data = rng.bytes(len);
        let seed = rng.next_u16();
        l.assert_same(&data, seed, &format!("C18 fuzz iter={i}"));
    }
}

// ---------------------------------------------------------------- C19
#[test]
fn c19_len_shorter_than_buffer() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 19);
    for _ in 0..500 {
        let total = 1 + rng.below(200);
        let mut data = rng.bytes(total);
        let use_len = rng.below(total + 1);
        let seed = rng.next_u16();

        let before_c = l.c(&data[..use_len], seed);
        let before_r = l.rust(&data[..use_len], seed);
        assert_eq!(before_c, before_r, "C19 divergence");

        // Scribble over the bytes past `use_len`; neither library may notice.
        for b in &mut data[use_len..] {
            *b = !*b;
        }
        let after_c = l.c(&data[..use_len], seed);
        let after_r = l.rust(&data[..use_len], seed);
        assert_eq!(after_c, after_r, "C19 divergence after scribble");
        assert_eq!(before_c, after_c, "C19: bytes past `len` must be ignored (C)");
        assert_eq!(
            before_r, after_r,
            "C19: bytes past `len` must be ignored (Rust)"
        );
    }
}

// ---------------------------------------------------------------- C20
#[test]
fn c20_structured_patterns() {
    let l = libs();
    let mk = |f: &dyn Fn(usize) -> u8| -> Vec<u8> { (0..64).map(f).collect() };
    let patterns: Vec<(&str, Vec<u8>)> = vec![
        ("00FF", mk(&|i| if i % 2 == 0 { 0x00 } else { 0xFF })),
        ("FF00", mk(&|i| if i % 2 == 0 { 0xFF } else { 0x00 })),
        ("AA55", mk(&|i| if i % 2 == 0 { 0xAA } else { 0x55 })),
        ("incr", mk(&|i| i as u8)),
        ("decr", mk(&|i| (255 - i) as u8)),
        ("hi-bit", mk(&|i| 0x80 | (i as u8 & 0x0F))),
        (
            "ascii",
            b"The quick brown fox jumps over the lazy dog, 0123456789!!!!!!!!".to_vec(),
        ),
    ];
    for (name, pat) in &patterns {
        for len in 0..=pat.len() {
            for &s in &BOUNDARY_SEEDS {
                l.assert_same(&pat[..len], s, &format!("C20 {name} len={len}"));
            }
        }
    }
    // The classic CRC check vector, plus its documented C value.
    let check = b"123456789";
    let v = l.assert_same(check, 0x0000, "C20 check vector");
    assert_eq!(v, 0xFEE8, "C20: C reference value for \"123456789\" seed 0");
}
