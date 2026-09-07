//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test calls BOTH the C `.so` and the Rust `.so` through `libloading`
//! and compares the returned `tflac_u16` byte-for-byte. Randomized inputs use a
//! fixed seed so failures reproduce.

mod common;
use common::{assert_same, assert_same_slice, Rng};

/// CONFIGS row 1 — `len == 0`, non-null `d`, random seeds.
#[test]
fn row01_len_zero_nonnull() {
    let mut rng = Rng::new(0x1001);
    let buf = [0xAAu8; 32];
    for i in 0..2000 {
        let crc = rng.u16();
        let out = assert_same(&format!("row01 #{i}"), buf.as_ptr(), 0, crc);
        // C returns the seed untouched when both loops fall through.
        assert_eq!(out, crc, "row01 #{i}: len==0 must return the seed");
    }
}

/// CONFIGS row 2 — `len == 0`, `d == NULL`: the pointer must never be read.
#[test]
fn row02_len_zero_null_ptr() {
    let mut rng = Rng::new(0x1002);
    for i in 0..2000 {
        let crc = rng.u16();
        let out = assert_same(&format!("row02 #{i}"), std::ptr::null(), 0, crc);
        assert_eq!(out, crc, "row02 #{i}: NULL/len==0 must return the seed");
    }
}

/// CONFIGS row 3 — tail-only path: every `len` in `1..=7`.
#[test]
fn row03_tail_only_lengths_1_to_7() {
    let mut rng = Rng::new(0x1003);
    for len in 1u32..=7 {
        for i in 0..3000 {
            let mut buf = vec![0u8; len as usize];
            rng.fill(&mut buf);
            let crc = rng.u16();
            assert_same_slice(&format!("row03 len={len} #{i}"), &buf, crc);
        }
    }
}

/// CONFIGS row 4 — `len == 8` exactly: one bulk iteration, no tail.
#[test]
fn row04_len_exactly_eight() {
    let mut rng = Rng::new(0x1004);
    for i in 0..20_000 {
        let mut buf = [0u8; 8];
        rng.fill(&mut buf);
        let crc = rng.u16();
        assert_same_slice(&format!("row04 #{i}"), &buf, crc);
    }
}

/// CONFIGS row 5 — bulk only, `len` a multiple of 8 in `16..=512`.
#[test]
fn row05_bulk_only_multiples_of_eight() {
    let mut rng = Rng::new(0x1005);
    for i in 0..5000 {
        let blocks = 2 + rng.below(63) as usize; // 2..=64 blocks -> 16..=512 bytes
        let mut buf = vec![0u8; blocks * 8];
        rng.fill(&mut buf);
        let crc = rng.u16();
        assert_same_slice(&format!("row05 #{i} len={}", buf.len()), &buf, crc);
    }
}

/// CONFIGS row 6 — bulk **then** tail: `len > 8` with `len % 8` in `1..=7`.
#[test]
fn row06_bulk_then_tail() {
    let mut rng = Rng::new(0x1006);
    for rem in 1usize..=7 {
        for i in 0..3000 {
            let blocks = 1 + rng.below(32) as usize;
            let mut buf = vec![0u8; blocks * 8 + rem];
            rng.fill(&mut buf);
            let crc = rng.u16();
            assert_same_slice(&format!("row06 rem={rem} #{i} len={}", buf.len()), &buf, crc);
        }
    }
}

/// CONFIGS row 7 — sweep every `len` in `0..=64` (both thresholds, all residues).
#[test]
fn row07_length_sweep_0_to_64() {
    let mut rng = Rng::new(0x1007);
    for len in 0usize..=64 {
        for i in 0..500 {
            let mut buf = vec![0u8; len.max(1)];
            rng.fill(&mut buf);
            let crc = rng.u16();
            assert_same(&format!("row07 len={len} #{i}"), buf.as_ptr(), len as u32, crc);
        }
    }
}

/// CONFIGS row 8 — seed `0x0000`.
#[test]
fn row08_seed_zero() {
    let mut rng = Rng::new(0x1008);
    for i in 0..10_000 {
        let len = rng.below(300) as usize;
        let mut buf = vec![0u8; len.max(1)];
        rng.fill(&mut buf);
        assert_same(&format!("row08 #{i} len={len}"), buf.as_ptr(), len as u32, 0x0000);
    }
}

/// CONFIGS row 9 — seed `0xFFFF` (drives both split indices to table slot 255).
#[test]
fn row09_seed_max() {
    let mut rng = Rng::new(0x1009);
    for i in 0..10_000 {
        let len = rng.below(300) as usize;
        let mut buf = vec![0u8; len.max(1)];
        rng.fill(&mut buf);
        assert_same(&format!("row09 #{i} len={len}"), buf.as_ptr(), len as u32, 0xFFFF);
    }
}

/// CONFIGS row 10 — every one of the 65536 `u16` seeds.
#[test]
fn row10_full_seed_domain() {
    let mut rng = Rng::new(0x100A);
    let mut buf = [0u8; 11]; // 1 bulk block + 3 tail bytes
    rng.fill(&mut buf);
    for seed in 0u32..=0xFFFF {
        assert_same_slice(&format!("row10 seed=0x{seed:04x}"), &buf, seed as u16);
    }
}

/// CONFIGS row 11 — all-zero data (table index 0 in every lane), lengths 0..=64.
#[test]
fn row11_all_zero_bytes() {
    let mut rng = Rng::new(0x100B);
    let buf = [0x00u8; 64];
    for len in 0u32..=64 {
        for i in 0..200 {
            let crc = rng.u16();
            assert_same(&format!("row11 len={len} #{i}"), buf.as_ptr(), len, crc);
        }
    }
}

/// CONFIGS row 12 — all-`0xFF` data (table index 255 in every lane), lengths 0..=64.
#[test]
fn row12_all_ff_bytes() {
    let mut rng = Rng::new(0x100C);
    let buf = [0xFFu8; 64];
    for len in 0u32..=64 {
        for i in 0..200 {
            let crc = rng.u16();
            assert_same(&format!("row12 len={len} #{i}"), buf.as_ptr(), len, crc);
        }
    }
}

/// CONFIGS row 13 — counting pattern so every byte value hits every bulk lane;
/// all 8 * 256 table slots are read.
#[test]
fn row13_all_table_slots() {
    let mut rng = Rng::new(0x100D);

    // 256 bytes: value == index, so lane k sees {v : v % 8 == k}.
    let ramp: Vec<u8> = (0..256u32).map(|v| v as u8).collect();
    for i in 0..500 {
        let crc = rng.u16();
        assert_same_slice(&format!("row13 ramp256 #{i}"), &ramp, crc);
    }

    // 2048 bytes with a rotating offset so each lane sees ALL 256 values.
    let big: Vec<u8> = (0..2048u32).map(|v| ((v / 8) + (v % 8) * 32) as u8).collect();
    for i in 0..500 {
        let crc = rng.u16();
        assert_same_slice(&format!("row13 big2048 #{i}"), &big, crc);
    }

    // Explicit per-lane saturation: for each lane, place every value 0..=255
    // at that lane position across 256 blocks.
    for lane in 0usize..8 {
        let mut buf = vec![0u8; 256 * 8];
        for v in 0..256usize {
            buf[v * 8 + lane] = v as u8;
        }
        for i in 0..50 {
            let crc = rng.u16();
            assert_same_slice(&format!("row13 lane={lane} #{i}"), &buf, crc);
        }
    }
}

/// CONFIGS row 14 — unaligned start pointer (offsets 1..=7 into the buffer).
#[test]
fn row14_unaligned_pointer() {
    let mut rng = Rng::new(0x100E);
    let mut backing = vec![0u8; 1024];
    rng.fill(&mut backing);
    for off in 1usize..=7 {
        for i in 0..2000 {
            let len = rng.below(512) as usize;
            let crc = rng.u16();
            let p = unsafe { backing.as_ptr().add(off) };
            assert_same(&format!("row14 off={off} len={len} #{i}"), p, len as u32, crc);
        }
    }
}

/// CONFIGS row 15 — 1 MiB buffer, exact multiple of 8 (bulk loop only).
#[test]
fn row15_one_mib_bulk() {
    let mut rng = Rng::new(0x100F);
    let mut buf = vec![0u8; 1024 * 1024];
    rng.fill(&mut buf);
    for i in 0..8 {
        let crc = rng.u16();
        assert_same_slice(&format!("row15 #{i}"), &buf, crc);
    }
}

/// CONFIGS row 16 — 1 MiB + r bytes for r in 1..=7 (many bulk plus tail).
#[test]
fn row16_one_mib_plus_tail() {
    let mut rng = Rng::new(0x1010);
    let mut buf = vec![0u8; 1024 * 1024 + 7];
    rng.fill(&mut buf);
    for r in 1u32..=7 {
        let len = 1024 * 1024 + r;
        for i in 0..4 {
            let crc = rng.u16();
            assert_same(&format!("row16 r={r} #{i}"), buf.as_ptr(), len, crc);
        }
    }
}

/// CONFIGS row 17 — streaming: random chunking chained through the seed must
/// equal the one-shot result, on both libs and across libs.
#[test]
fn row17_streaming_random_chunks() {
    let p = common::pair();
    let mut rng = Rng::new(0x1011);
    for i in 0..2000 {
        let len = rng.below(1024) as usize;
        let mut buf = vec![0u8; len.max(1)];
        rng.fill(&mut buf);
        let seed = rng.u16();

        let one_shot = assert_same(&format!("row17 oneshot #{i}"), buf.as_ptr(), len as u32, seed);

        let mut pos = 0usize;
        let mut c_acc = seed;
        let mut r_acc = seed;
        while pos < len {
            let chunk = 1 + rng.below((len - pos) as u64) as usize;
            unsafe {
                let q = buf.as_ptr().add(pos);
                c_acc = (p.c.crc16)(q, chunk as u32, c_acc);
                r_acc = (p.rs.crc16)(q, chunk as u32, r_acc);
            }
            assert_eq!(
                c_acc, r_acc,
                "row17 #{i}: chunked divergence at pos={pos} chunk={chunk} \
                 (C=0x{c_acc:04x} RUST=0x{r_acc:04x})"
            );
            pos += chunk;
        }
        assert_eq!(c_acc, one_shot, "row17 #{i}: C chunked != C one-shot");
        assert_eq!(r_acc, one_shot, "row17 #{i}: RUST chunked != one-shot");
    }
}

/// CONFIGS row 18 — split at every offset `0..=len` (crosses bulk boundaries
/// mid-stream at every possible phase).
#[test]
fn row18_streaming_every_split_point() {
    let p = common::pair();
    let mut rng = Rng::new(0x1012);
    for trial in 0..30 {
        let len = 1 + rng.below(200) as usize;
        let mut buf = vec![0u8; len];
        rng.fill(&mut buf);
        let seed = rng.u16();
        let one_shot = assert_same(&format!("row18 t{trial} oneshot"), buf.as_ptr(), len as u32, seed);

        for split in 0..=len {
            let (c, r) = unsafe {
                let head = buf.as_ptr();
                let tail = buf.as_ptr().add(split);
                let c1 = (p.c.crc16)(head, split as u32, seed);
                let r1 = (p.rs.crc16)(head, split as u32, seed);
                assert_eq!(c1, r1, "row18 t{trial} split={split}: head divergence");
                (
                    (p.c.crc16)(tail, (len - split) as u32, c1),
                    (p.rs.crc16)(tail, (len - split) as u32, r1),
                )
            };
            assert_eq!(c, r, "row18 t{trial} split={split}: C=0x{c:04x} RUST=0x{r:04x}");
            assert_eq!(c, one_shot, "row18 t{trial} split={split}: split != one-shot");
        }
    }
}

/// CONFIGS row 19 — broad property fuzz over (len, bytes, seed).
#[test]
fn row19_property_fuzz() {
    let mut rng = Rng::new(0x1013);
    for i in 0..20_000 {
        let len = rng.below(1025) as usize;
        let mut buf = vec![0u8; len.max(1)];
        rng.fill(&mut buf);
        let crc = rng.u16();
        assert_same(&format!("row19 #{i} len={len}"), buf.as_ptr(), len as u32, crc);
    }
}

/// CONFIGS row 20 — the crate has no `[features]`, so there is exactly one
/// configuration; this test simply records that it is the one under test and
/// re-runs a representative differential sweep in it.
#[test]
fn row20_single_feature_configuration() {
    let mut rng = Rng::new(0x1014);
    for len in 0u32..=40 {
        for i in 0..100 {
            let mut buf = vec![0u8; (len as usize).max(1)];
            rng.fill(&mut buf);
            let crc = rng.u16();
            assert_same(&format!("row20 len={len} #{i}"), buf.as_ptr(), len, crc);
        }
    }
}
