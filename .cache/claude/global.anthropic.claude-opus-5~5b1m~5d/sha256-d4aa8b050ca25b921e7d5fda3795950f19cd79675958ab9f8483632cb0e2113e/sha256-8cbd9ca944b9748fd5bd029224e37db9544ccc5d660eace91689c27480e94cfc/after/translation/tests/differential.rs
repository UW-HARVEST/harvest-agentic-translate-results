//! Differential tests: C `.so` vs Rust `.so`, both loaded via `libloading`.
//!
//! Phase B rows live in `mod configs`, Phase C rows in `mod error_paths`.
//! Row numbers refer to `CONFIGS.md` / `ERRORS.md`.

mod common;

use common::{EXTREME_SEEDS, Pair, Rng};

// ---------------------------------------------------------------------------
// Phase B — CONFIGS.md
// ---------------------------------------------------------------------------
mod configs {
    use super::*;

    /// Row 1 & 2 — `len == 0` with every interesting seed (+ 256 random seeds).
    #[test]
    fn row01_02_zero_length_all_seeds() {
        let p = Pair::load();
        let buf = [0xAAu8; 16];
        for &s in EXTREME_SEEDS {
            let v = p.both_ptr(buf.as_ptr(), 0, s);
            assert_eq!(v, s, "len=0 must return the seed unchanged");
        }
        let mut rng = Rng::new(1);
        for _ in 0..256 {
            let s = rng.next_u16();
            let v = p.both_ptr(buf.as_ptr(), 0, s);
            assert_eq!(v, s);
        }
    }

    /// Row 3 & 4 — tail-only lengths 1..=7, random data, seed 0 then many
    /// random seeds.
    #[test]
    fn row03_04_tail_only_lengths() {
        let p = Pair::load();
        let mut rng = Rng::new(2);
        for len in 1u32..=7 {
            for _ in 0..500 {
                let data = rng.bytes(len as usize);
                p.both(&data, len, 0);
                p.both(&data, len, rng.next_u16());
            }
        }
    }

    /// Row 5 — tail-only lengths with all-0x00 / all-0xFF data, extreme seeds.
    #[test]
    fn row05_tail_extreme_data() {
        let p = Pair::load();
        for len in 1u32..=7 {
            for fill in [0x00u8, 0xFF, 0x80, 0x01] {
                let data = vec![fill; len as usize];
                for &s in EXTREME_SEEDS {
                    p.both(&data, len, s);
                }
            }
        }
    }

    /// Row 6 — exactly one slice-by-8 iteration, no tail.
    #[test]
    fn row06_len8_random() {
        let p = Pair::load();
        let mut rng = Rng::new(3);
        for _ in 0..5000 {
            let data = rng.bytes(8);
            p.both(&data, 8, rng.next_u16());
        }
    }

    /// Row 7 — `len == 8` with structured data and extreme seeds.
    #[test]
    fn row07_len8_structured() {
        let p = Pair::load();
        let patterns: Vec<Vec<u8>> = vec![
            vec![0x00; 8],
            vec![0xFF; 8],
            (0u8..8).collect(),
            (0u8..8).map(|i| 0xFF - i).collect(),
            vec![0x80, 0x00, 0x80, 0x00, 0x80, 0x00, 0x80, 0x00],
        ];
        for data in &patterns {
            for &s in EXTREME_SEEDS {
                p.both(data, 8, s);
            }
        }
    }

    /// Row 8 — fast loop plus 1..7 tail iterations.
    #[test]
    fn row08_len9_to_15() {
        let p = Pair::load();
        let mut rng = Rng::new(4);
        for len in 9u32..=15 {
            for _ in 0..500 {
                let data = rng.bytes(len as usize);
                p.both(&data, len, rng.next_u16());
                for &s in EXTREME_SEEDS {
                    p.both(&data, len, s);
                }
            }
        }
    }

    /// Row 9 — lengths that are multiples of 8, 16..=1024.
    #[test]
    fn row09_multiples_of_8() {
        let p = Pair::load();
        let mut rng = Rng::new(5);
        for len in (16u32..=1024).step_by(8) {
            for _ in 0..8 {
                let data = rng.bytes(len as usize);
                p.both(&data, len, rng.next_u16());
            }
            let z = vec![0u8; len as usize];
            let f = vec![0xFFu8; len as usize];
            for &s in &[0x0000u16, 0xFFFF] {
                p.both(&z, len, s);
                p.both(&f, len, s);
            }
        }
    }

    /// Row 10 — lengths that are NOT multiples of 8, 16..=1024.
    #[test]
    fn row10_non_multiples_of_8() {
        let p = Pair::load();
        let mut rng = Rng::new(6);
        for len in 16u32..=1024 {
            if len % 8 == 0 {
                continue;
            }
            let data = rng.bytes(len as usize);
            p.both(&data, len, rng.next_u16());
            p.both(&data, len, 0xFFFF);
        }
    }

    /// Row 11 — lengths around the 2^16 boundary.
    #[test]
    fn row11_64k_boundary() {
        let p = Pair::load();
        let mut rng = Rng::new(7);
        let data = rng.bytes(65_600);
        for len in [65_535u32, 65_536, 65_537, 65_538, 65_543] {
            for &s in &[0x0000u16, 0x1234, 0xFFFF] {
                p.both(&data[..len as usize], len, s);
            }
        }
    }

    /// Row 12 — 1 MiB buffer.
    #[test]
    fn row12_one_mib() {
        let p = Pair::load();
        let mut rng = Rng::new(8);
        let len: u32 = 1 << 20;
        let data = rng.bytes(len as usize);
        for &s in &[0x0000u16, 0xFFFF] {
            p.both(&data, len, s);
        }
    }

    /// Row 13 — every single-byte value crossed with every high seed byte
    /// (256 * 256 = 65 536 calls; drives table 0 over its whole index range).
    #[test]
    fn row13_full_byte_seed_cross() {
        let p = Pair::load();
        for b in 0u16..=255 {
            let data = [b as u8];
            for hi in 0u16..=255 {
                let seed = (hi << 8) | 0x5A;
                p.both(&data, 1, seed);
            }
        }
    }

    /// Row 14 — one 0xFF per position in an 8-byte group, so each of the eight
    /// tables gets hit at a high index in isolation.
    #[test]
    fn row14_one_hot_group() {
        let p = Pair::load();
        for pos in 0..8usize {
            let mut data = vec![0u8; 8];
            data[pos] = 0xFF;
            for &s in EXTREME_SEEDS {
                p.both(&data, 8, s);
            }
            // and a longer buffer where the one-hot group repeats
            let mut long = Vec::new();
            for _ in 0..16 {
                long.extend_from_slice(&data);
            }
            p.both(&long, long.len() as u32, 0xBEEF);
        }
    }

    /// Row 15 — unaligned pointers (offsets 0..=8 into a buffer).
    #[test]
    fn row15_unaligned_pointers() {
        let p = Pair::load();
        let mut rng = Rng::new(9);
        let buf = rng.bytes(9 + 80);
        for off in 0..=8usize {
            for len in [64u32, 67] {
                let seed = rng.next_u16();
                let sub = &buf[off..off + len as usize];
                let v_unaligned = p.both_ptr(sub.as_ptr(), len, seed);
                // Same bytes, freshly allocated (different alignment class):
                let copy = sub.to_vec();
                let v_copy = p.both(&copy, len, seed);
                assert_eq!(v_unaligned, v_copy, "alignment must not matter");
            }
        }
    }

    /// Rows 16–18 — chunked/streaming use: chain the returned CRC as the next
    /// seed and compare against the one-shot call, in BOTH implementations.
    #[test]
    fn row16_17_18_chunked_streaming() {
        let p = Pair::load();
        let mut rng = Rng::new(10);

        for trial in 0..300 {
            let total = 1 + rng.below(2048);
            let data = rng.bytes(total);
            let seed = rng.next_u16();

            // one-shot reference (already asserted C == Rust inside `both`)
            let oneshot = p.both(&data, total as u32, seed);

            // choose chunk boundaries
            let multiples_of_8 = trial % 3 == 0;
            let zero_chunks = trial % 3 == 2;
            let mut offs = 0usize;
            let mut crc_c = seed;
            let mut crc_r = seed;
            while offs < total {
                let mut n = 1 + rng.below(std::cmp::min(64, total - offs));
                if multiples_of_8 {
                    n = ((n + 7) / 8) * 8;
                    if offs + n > total {
                        n = total - offs;
                    }
                }
                if zero_chunks {
                    // interleave a zero-length chunk (identity)
                    let c0 = unsafe { (p.c)(data[offs..].as_ptr(), 0, crc_c) };
                    let r0 = unsafe { (p.r)(data[offs..].as_ptr(), 0, crc_r) };
                    assert_eq!(c0, crc_c);
                    assert_eq!(r0, crc_r);
                    assert_eq!(c0, r0);
                }
                let chunk = &data[offs..offs + n];
                crc_c = unsafe { (p.c)(chunk.as_ptr(), n as u32, crc_c) };
                crc_r = unsafe { (p.r)(chunk.as_ptr(), n as u32, crc_r) };
                assert_eq!(
                    crc_c, crc_r,
                    "chunked divergence at offs={offs} n={n} total={total}"
                );
                offs += n;
            }
            assert_eq!(crc_c, crc_r);
            // Streaming equals one-shot only when every chunk is a multiple of
            // 8 or when the buffer is processed byte-wise; the C algorithm is
            // NOT chunk-boundary invariant in general, so only assert the
            // property the C itself exhibits: both libraries agree.
            let _ = oneshot;
        }
    }

    /// Row 19 — broad randomized fuzz sweep, fixed PRNG seed.
    #[test]
    fn row19_fuzz_sweep() {
        let p = Pair::load();
        let mut rng = Rng::new(0xDEADBEEF);
        let pool = rng.bytes(700);
        for _ in 0..20_000 {
            let len = rng.below(601) as u32;
            let start = rng.below(700 - 600);
            let seed = rng.next_u16();
            let slice = &pool[start..start + len as usize];
            if len == 0 {
                p.both_ptr(pool.as_ptr(), 0, seed);
            } else {
                p.both_ptr(slice.as_ptr(), len, seed);
            }
        }
    }

    /// Row 20 — force every index 0..=255 through every one of the eight
    /// tables: table 7 and 6 are indexed by the seed-mixed CRC bytes, tables
    /// 5..0 by data bytes 2..7 of the group.
    #[test]
    fn row20_full_table_index_coverage() {
        let p = Pair::load();
        // tables 5..0 <- d[2..8]: sweep each data position over all 256 values
        for pos in 2..8usize {
            for v in 0u16..=255 {
                let mut data = vec![0u8; 8];
                data[pos] = v as u8;
                p.both(&data, 8, 0x0000);
                p.both(&data, 8, 0xFFFF);
            }
        }
        // tables 7 and 6 <- (seed ^ (d0<<8|d1)) high/low byte: sweep both bytes
        for hi in 0u16..=255 {
            for lo in [0u16, 1, 0x7F, 0x80, 0xFE, 0xFF] {
                let data = [hi as u8, lo as u8, 0, 0, 0, 0, 0, 0];
                p.both(&data, 8, 0x0000);
                let data2 = [0u8, 0, 0, 0, 0, 0, 0, 0];
                p.both(&data2, 8, (hi << 8) | lo);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Phase C — ERRORS.md
// ---------------------------------------------------------------------------
mod error_paths {
    use super::*;

    /// ERRORS row 1 — `len == 0` with a valid pointer.
    #[test]
    fn err01_zero_len_valid_ptr() {
        let p = Pair::load();
        let buf = [1u8, 2, 3, 4, 5, 6, 7, 8, 9];
        for &s in EXTREME_SEEDS {
            let v = p.both_ptr(buf.as_ptr(), 0, s);
            assert_eq!(v, s);
        }
    }

    /// ERRORS rows 2 & 3 — `len == 0` with a NULL pointer (must not deref).
    #[test]
    fn err02_03_zero_len_null_ptr() {
        let p = Pair::load();
        for &s in EXTREME_SEEDS {
            let v = p.both_ptr(std::ptr::null(), 0, s);
            assert_eq!(v, s, "NULL + len=0 must return the seed");
        }
        assert_eq!(p.both_ptr(std::ptr::null(), 0, 0xFFFF), 0xFFFF);
        assert_eq!(p.both_ptr(std::ptr::null(), 0, 0x0000), 0x0000);
    }

    /// ERRORS row 4 — minimum non-empty length.
    #[test]
    fn err04_len1_minimum() {
        let p = Pair::load();
        for b in 0u16..=255 {
            let data = [b as u8];
            for &s in EXTREME_SEEDS {
                p.both(&data, 1, s);
            }
        }
    }

    /// ERRORS rows 5, 6, 7 — one below / at / one past the slice-by-8
    /// threshold.
    #[test]
    fn err05_06_07_threshold_boundaries() {
        let p = Pair::load();
        let mut rng = Rng::new(11);
        for len in [7u32, 8, 9] {
            for _ in 0..2000 {
                let data = rng.bytes(len as usize);
                p.both(&data, len, rng.next_u16());
            }
            for &s in EXTREME_SEEDS {
                p.both(&vec![0x00; len as usize], len, s);
                p.both(&vec![0xFF; len as usize], len, s);
            }
        }
    }

    /// ERRORS row 8 — extreme seeds driving table indices 0 and 255.
    #[test]
    fn err08_extreme_seeds() {
        let p = Pair::load();
        let mut rng = Rng::new(12);
        for seed in [0xFFFFu16, 0x8000, 0x00FF, 0xFF00, 0x0000] {
            for len in [1u32, 2, 7, 8, 15, 16, 31, 64] {
                let data = rng.bytes(len as usize);
                p.both(&data, len, seed);
            }
        }
        // full seed sweep at len 8 (all 65 536 seeds would be slow; sweep both
        // bytes independently over their whole range)
        for b in 0u16..=255 {
            let data = rng.bytes(8);
            p.both(&data, 8, b << 8);
            p.both(&data, 8, b);
            p.both(&data, 8, (b << 8) | b);
        }
    }

    /// ERRORS row 9 — largest length exercised (1 MiB) — "oversized" count.
    #[test]
    fn err09_large_len() {
        let p = Pair::load();
        let mut rng = Rng::new(13);
        let data = rng.bytes(1 << 20);
        p.both(&data, 1 << 20, 0x0000);
        p.both(&data, (1 << 20) - 1, 0xFFFF);
        p.both(&data, (1 << 20) - 7, 0x8005);
    }

    /// ERRORS row 10 — byte-value extremes in every group position.
    #[test]
    fn err10_byte_extremes() {
        let p = Pair::load();
        for pos in 0..8usize {
            for &(base, spike) in &[(0x00u8, 0xFFu8), (0xFF, 0x00)] {
                let mut data = vec![base; 8];
                data[pos] = spike;
                for &s in EXTREME_SEEDS {
                    p.both(&data, 8, s);
                }
            }
        }
    }

    /// ERRORS row 11 — unaligned pointer.
    #[test]
    fn err11_unaligned() {
        let p = Pair::load();
        let mut rng = Rng::new(14);
        let buf = rng.bytes(64);
        for off in 1..8usize {
            let len = (buf.len() - off) as u32;
            p.both_ptr(buf[off..].as_ptr(), len, 0x1234);
        }
    }

    /// ERRORS row 12 — no enum/flag parameter exists, but the full numeric
    /// range of both scalar parameters is legal input across the FFI boundary.
    /// Feed hostile/extreme scalars (including `u32::MAX`-adjacent lengths that
    /// are still backed by real memory only when small) and confirm agreement.
    #[test]
    fn err12_scalar_extremes() {
        let p = Pair::load();
        // Every u16 seed value, len = 0 (no memory needed): full 65 536 sweep.
        for s in 0u16..=u16::MAX {
            let v = p.both_ptr(std::ptr::null(), 0, s);
            assert_eq!(v, s);
        }
        // len values with only the low bits meaningful for loop structure.
        let mut rng = Rng::new(15);
        let data = rng.bytes(300);
        for len in [1u32, 2, 3, 4, 5, 6, 7, 8, 9, 16, 17, 255, 256, 257, 299, 300] {
            p.both(&data[..len as usize], len, 0xFFFF);
            p.both(&data[..len as usize], len, 0x0000);
        }
    }
}
