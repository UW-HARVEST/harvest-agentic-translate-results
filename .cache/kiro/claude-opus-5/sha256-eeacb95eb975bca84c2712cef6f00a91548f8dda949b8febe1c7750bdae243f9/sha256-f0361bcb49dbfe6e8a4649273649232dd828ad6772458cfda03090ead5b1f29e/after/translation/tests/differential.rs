//! Phase B — valid-path differential tests. One test per row of `CONFIGS.md`.
//!
//! Every call goes through `dlopen` + the exported C symbol for BOTH the C
//! shared object and the Rust cdylib.

mod harness;

use harness::{Rng, Tflac, load_pair};

/// Base seed; each row derives its own stream so rows stay independent.
const SEED: u64 = 0xC0FFEE_1234_5678;

// ---------------------------------------------------------------------------
// tflac_size_memory
// ---------------------------------------------------------------------------

/// CONFIGS row 1 — exhaustive over the non-wrapping range 0..=65535.
#[test]
fn cfg_01_size_memory_exhaustive_small() {
    let p = load_pair();
    for b in 0u32..=65535 {
        p.cmp_size_memory(b);
    }
}

/// CONFIGS row 2 — masking / 16-byte alignment boundaries of `(15 + 4b) & !0xF`.
#[test]
fn cfg_02_size_memory_mask_boundaries() {
    let p = load_pair();
    // Small hand-picked values plus every neighbourhood where the mask ticks.
    for b in [0u32, 1, 2, 3, 4, 5, 15, 16, 17, 31, 32, 33] {
        p.cmp_size_memory(b);
    }
    // (15 + 4b) crosses a multiple of 16 every 4 steps of b; sweep neighbours.
    for k in 0u32..4096 {
        let centre = 4 * k;
        for d in 0..8u32 {
            p.cmp_size_memory(centre.wrapping_add(d).wrapping_sub(4));
        }
    }
}

/// CONFIGS row 3 — wrapping arithmetic: `blocksize * 4` overflows for
/// `blocksize >= 2^30`, and the `5 *` multiply wraps too.
#[test]
fn cfg_03_size_memory_wrapping() {
    let p = load_pair();

    // Exact overflow thresholds and u32 extremes.
    for b in [
        0x3FFF_FFFEu32,
        0x3FFF_FFFF,
        0x4000_0000,
        0x4000_0001,
        0x7FFF_FFFF,
        0x8000_0000,
        0x8000_0001,
        0xBFFF_FFFF,
        0xC000_0000,
        0xFFFF_FFF0,
        0xFFFF_FFFB,
        0xFFFF_FFFC,
        0xFFFF_FFFD,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
    ] {
        p.cmp_size_memory(b);
    }

    // Dense sweeps around each power-of-two boundary.
    for shift in 0..32u32 {
        let base = 1u32 << shift;
        for d in 0..64u32 {
            p.cmp_size_memory(base.wrapping_add(d).wrapping_sub(32));
        }
    }

    // Randomised full-range u32.
    let mut rng = Rng::new(SEED ^ 3);
    for _ in 0..300_000 {
        p.cmp_size_memory(rng.next_u32());
    }
}

// ---------------------------------------------------------------------------
// flac_validate — helpers
// ---------------------------------------------------------------------------

/// A struct whose non-configured fields are random garbage (including the
/// output fields `partition_order` / `cur_blocksize` and the tail padding), but
/// whose validated fields are all in range.
fn valid_random(rng: &mut Rng) -> Tflac {
    let mut t = Tflac::zeroed();
    rng.fill(&mut t.0); // garbage everywhere, incl. padding bytes 21..=23
    let bs = rng.range_u32(16, 65535);
    t.set_blocksize(bs)
        .set_samplerate(rng.range_u32(1, 655350))
        .set_channels(rng.range_u32(1, 8))
        .set_bitdepth(rng.range_u32(1, 32));
    // max_rice_value must be 0 or 1..=30
    let mrv = rng.range_u8(0, 30);
    t.set_max_rice_value(mrv);
    let maxpo = rng.range_u8(0, 15);
    let minpo = rng.range_u8(0, maxpo);
    t.set_max_partition_order(maxpo).set_min_partition_order(minpo);
    t
}

/// `odd * 2^k`, clamped into the valid blocksize range 16..=65535.
fn blocksize_with_valuation(rng: &mut Rng, k: u32) -> u32 {
    let pow = 1u32 << k;
    if pow > 65535 {
        return 32768; // valuation 15, the largest representable
    }
    let max_odd = 65535 / pow;
    let mut odd = rng.range_u32(1, max_odd.max(1)) | 1;
    while odd * pow < 16 {
        odd += 2;
    }
    let v = odd.wrapping_mul(pow);
    if v < 16 || v > 65535 { pow.max(16) } else { v }
}

// ---------------------------------------------------------------------------
// flac_validate — rows
// ---------------------------------------------------------------------------

/// CONFIGS row 4 — full-surface fuzz: all 28 bytes random.
#[test]
fn cfg_04_validate_full_random_bytes() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..400_000 {
        let mut t = Tflac::zeroed();
        rng.fill(&mut t.0);
        p.cmp_validate(&t);
    }
    // Biased fuzz: keep u32 fields small so more calls reach deep code paths.
    for _ in 0..400_000 {
        let mut t = Tflac::zeroed();
        rng.fill(&mut t.0);
        t.set_blocksize(rng.range_u32(0, 70000))
            .set_samplerate(rng.range_u32(0, 700000))
            .set_channels(rng.range_u32(0, 10))
            .set_bitdepth(rng.range_u32(0, 34));
        p.cmp_validate(&t);
    }
}

/// CONFIGS row 5 — independent channel mode, random valid channels/bitdepth.
#[test]
fn cfg_05_channel_mode_independent() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 5);
    for _ in 0..100_000 {
        let mut t = valid_random(&mut rng);
        t.set_channel_mode(0);
        p.cmp_validate(&t);
    }
    // Deterministic sweep over every (channels, bitdepth) pair.
    for ch in 1..=8u32 {
        for bd in 1..=32u32 {
            let mut t = Tflac::valid();
            t.set_channel_mode(0).set_channels(ch).set_bitdepth(bd);
            p.cmp_validate(&t);
        }
    }
}

/// CONFIGS row 6 — valid non-independent mode preserved (channels==2, bd<32).
#[test]
fn cfg_06_side_stereo_preserved() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..100_000 {
        let mut t = valid_random(&mut rng);
        t.set_channel_mode(rng.range_u8(1, 3))
            .set_channels(2)
            .set_bitdepth(rng.range_u32(1, 31));
        p.cmp_validate(&t);
    }
    for mode in 1..=3u8 {
        for bd in 1..=31u32 {
            let mut t = Tflac::valid();
            t.set_channel_mode(mode).set_channels(2).set_bitdepth(bd);
            p.cmp_validate(&t);
        }
    }
}

/// CONFIGS row 7 — bitdepth==32 forces independent even with channels==2.
#[test]
fn cfg_07_bitdepth32_forces_independent() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 7);
    for _ in 0..50_000 {
        let mut t = valid_random(&mut rng);
        t.set_channel_mode(rng.range_u8(1, 3))
            .set_channels(2)
            .set_bitdepth(32);
        p.cmp_validate(&t);
    }
    for mode in 1..=3u8 {
        let mut t = Tflac::valid();
        t.set_channel_mode(mode).set_channels(2).set_bitdepth(32);
        p.cmp_validate(&t);
    }
}

/// CONFIGS row 8 — channels != 2 forces independent.
#[test]
fn cfg_08_non_stereo_forces_independent() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 8);
    for _ in 0..100_000 {
        let mut t = valid_random(&mut rng);
        let mut ch = rng.range_u32(1, 8);
        if ch == 2 {
            ch = 1;
        }
        t.set_channel_mode(rng.range_u8(1, 3)).set_channels(ch);
        p.cmp_validate(&t);
    }
    for mode in 1..=3u8 {
        for ch in [1u32, 3, 4, 5, 6, 7, 8] {
            for bd in [1u32, 16, 17, 31, 32] {
                let mut t = Tflac::valid();
                t.set_channel_mode(mode).set_channels(ch).set_bitdepth(bd);
                p.cmp_validate(&t);
            }
        }
    }
}

/// CONFIGS row 9 — out-of-enum `channel_mode` (4..=255) with channels==2 and
/// bitdepth<32: the C keeps the bogus value verbatim.
#[test]
fn cfg_09_out_of_range_enum_preserved() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 9);
    for mode in 4..=255u8 {
        for bd in [1u32, 8, 16, 17, 24, 31] {
            let mut t = Tflac::valid();
            t.set_channel_mode(mode).set_channels(2).set_bitdepth(bd);
            p.cmp_validate(&t);
        }
    }
    for _ in 0..100_000 {
        let mut t = valid_random(&mut rng);
        t.set_channel_mode(rng.range_u8(4, 255))
            .set_channels(2)
            .set_bitdepth(rng.range_u32(1, 31));
        p.cmp_validate(&t);
    }
}

/// CONFIGS row 10 — out-of-enum `channel_mode` that still gets normalised to 0.
#[test]
fn cfg_10_out_of_range_enum_forced_independent() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 10);
    for mode in 4..=255u8 {
        // channels != 2
        let mut t = Tflac::valid();
        t.set_channel_mode(mode).set_channels(5).set_bitdepth(24);
        p.cmp_validate(&t);
        // channels == 2 but bitdepth == 32
        let mut t = Tflac::valid();
        t.set_channel_mode(mode).set_channels(2).set_bitdepth(32);
        p.cmp_validate(&t);
    }
    for _ in 0..100_000 {
        let mut t = valid_random(&mut rng);
        t.set_channel_mode(rng.range_u8(4, 255));
        if rng.next_u8() & 1 == 0 {
            let mut ch = rng.range_u32(1, 8);
            if ch == 2 {
                ch = 3;
            }
            t.set_channels(ch);
        } else {
            t.set_channels(2).set_bitdepth(32);
        }
        p.cmp_validate(&t);
    }
}

/// CONFIGS row 11 — max_rice_value defaulting for bitdepth <= 16 (→ 14).
#[test]
fn cfg_11_rice_default_low_bitdepth() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 11);
    for bd in 1..=16u32 {
        let mut t = Tflac::valid();
        t.set_bitdepth(bd).set_max_rice_value(0);
        p.cmp_validate(&t);
    }
    for _ in 0..50_000 {
        let mut t = valid_random(&mut rng);
        t.set_bitdepth(rng.range_u32(1, 16)).set_max_rice_value(0);
        p.cmp_validate(&t);
    }
}

/// CONFIGS row 12 — max_rice_value defaulting for bitdepth >= 17 (→ 30).
#[test]
fn cfg_12_rice_default_high_bitdepth() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 12);
    for bd in 17..=32u32 {
        let mut t = Tflac::valid();
        t.set_bitdepth(bd).set_max_rice_value(0);
        p.cmp_validate(&t);
    }
    for _ in 0..50_000 {
        let mut t = valid_random(&mut rng);
        t.set_bitdepth(rng.range_u32(17, 32)).set_max_rice_value(0);
        p.cmp_validate(&t);
    }
}

/// CONFIGS row 13 — explicit max_rice_value 1..=30 is kept.
#[test]
fn cfg_13_rice_explicit_kept() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 13);
    for mrv in 1..=30u8 {
        for bd in [1u32, 16, 17, 32] {
            let mut t = Tflac::valid();
            t.set_max_rice_value(mrv).set_bitdepth(bd);
            p.cmp_validate(&t);
        }
    }
    for _ in 0..50_000 {
        let mut t = valid_random(&mut rng);
        t.set_max_rice_value(rng.range_u8(1, 30));
        p.cmp_validate(&t);
    }
}

/// CONFIGS row 14 — min == max partition order: loop body never runs.
#[test]
fn cfg_14_partition_order_min_eq_max() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 14);
    for po in 0..=15u8 {
        for bs in [16u32, 17, 4096, 32768, 65535, 65534] {
            let mut t = Tflac::valid();
            t.set_min_partition_order(po)
                .set_max_partition_order(po)
                .set_blocksize(bs);
            p.cmp_validate(&t);
        }
    }
    for _ in 0..50_000 {
        let mut t = valid_random(&mut rng);
        let po = rng.range_u8(0, 15);
        t.set_min_partition_order(po).set_max_partition_order(po);
        p.cmp_validate(&t);
    }
}

/// CONFIGS row 15 — odd blocksize: loop exits before the first increment.
#[test]
fn cfg_15_partition_order_odd_blocksize() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..50_000 {
        let mut t = valid_random(&mut rng);
        t.set_blocksize(rng.range_u32(16, 65535) | 1)
            .set_min_partition_order(0)
            .set_max_partition_order(15);
        p.cmp_validate(&t);
    }
    for bs in [17u32, 19, 4097, 65535] {
        let mut t = Tflac::valid();
        t.set_blocksize(bs)
            .set_min_partition_order(0)
            .set_max_partition_order(15);
        p.cmp_validate(&t);
    }
}

/// CONFIGS row 16 — blocksize 32768 with max order 15: the loop drives the
/// shift all the way to `1 << 16`.
#[test]
fn cfg_16_partition_order_climbs_to_max() {
    let p = load_pair();
    for minpo in 0..=15u8 {
        let mut t = Tflac::valid();
        t.set_blocksize(32768)
            .set_min_partition_order(minpo)
            .set_max_partition_order(15);
        p.cmp_validate(&t);
    }
    // Same but with every max order, so the clamp point varies.
    for maxpo in 0..=15u8 {
        let mut t = Tflac::valid();
        t.set_blocksize(32768)
            .set_min_partition_order(0)
            .set_max_partition_order(maxpo);
        p.cmp_validate(&t);
    }
}

/// CONFIGS row 17 — blocksize = odd * 2^k for every k: loop stops at min(k,max).
#[test]
fn cfg_17_partition_order_every_valuation() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 17);
    for k in 0..=15u32 {
        for _ in 0..2_000 {
            let bs = blocksize_with_valuation(&mut rng, k);
            for maxpo in 0..=15u8 {
                let mut t = Tflac::valid();
                t.set_blocksize(bs)
                    .set_min_partition_order(0)
                    .set_max_partition_order(maxpo);
                p.cmp_validate(&t);
            }
        }
    }
}

/// CONFIGS row 18 — random min <= max with random blocksize.
#[test]
fn cfg_18_partition_order_random() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..300_000 {
        let t = valid_random(&mut rng);
        p.cmp_validate(&t);
    }
}

/// CONFIGS row 19 — boundary-value cross product.
#[test]
fn cfg_19_boundary_cross_product() {
    let p = load_pair();
    for bs in [16u32, 17, 18, 32, 65534, 65535] {
        for sr in [1u32, 2, 44100, 655349, 655350] {
            for ch in 1..=8u32 {
                for bd in [1u32, 15, 16, 17, 31, 32] {
                    for mode in [0u8, 1, 2, 3, 4, 255] {
                        for mrv in [0u8, 1, 14, 30] {
                            let mut t = Tflac::valid();
                            t.set_blocksize(bs)
                                .set_samplerate(sr)
                                .set_channels(ch)
                                .set_bitdepth(bd)
                                .set_channel_mode(mode)
                                .set_max_rice_value(mrv)
                                .set_min_partition_order(0)
                                .set_max_partition_order(15);
                            p.cmp_validate(&t);
                        }
                    }
                }
            }
        }
    }
}

/// CONFIGS row 20 — output fields pre-seeded with garbage on both success and
/// reject paths: confirms identical overwrite / non-overwrite semantics.
#[test]
fn cfg_20_output_fields_preseeded() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 20);
    for _ in 0..200_000 {
        let mut t = valid_random(&mut rng);
        t.set_partition_order(rng.next_u8())
            .set_cur_blocksize(rng.next_u32());
        p.cmp_validate(&t);
    }
    // Reject paths with garbage outputs — the C must leave them untouched.
    for _ in 0..200_000 {
        let mut t = valid_random(&mut rng);
        t.set_partition_order(rng.next_u8())
            .set_cur_blocksize(rng.next_u32());
        match rng.next_u8() % 8 {
            0 => {
                t.set_blocksize(rng.range_u32(0, 15));
            }
            1 => {
                t.set_blocksize(rng.range_u32(65536, u32::MAX));
            }
            2 => {
                t.set_samplerate(0);
            }
            3 => {
                t.set_samplerate(rng.range_u32(655351, u32::MAX));
            }
            4 => {
                t.set_channels(rng.range_u32(9, u32::MAX));
            }
            5 => {
                t.set_bitdepth(rng.range_u32(33, u32::MAX));
            }
            6 => {
                t.set_max_rice_value(rng.range_u8(31, 255));
            }
            _ => {
                t.set_max_partition_order(rng.range_u8(16, 255));
            }
        }
        p.cmp_validate(&t);
    }
}

/// CONFIGS row 21 — calling validate twice: the second pass sees the already
/// normalised state (non-zero max_rice_value, forced channel_mode).
#[test]
fn cfg_21_validate_twice() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 21);
    for _ in 0..200_000 {
        let mut t = Tflac::zeroed();
        rng.fill(&mut t.0);
        t.set_blocksize(rng.range_u32(0, 70000))
            .set_samplerate(rng.range_u32(0, 700000))
            .set_channels(rng.range_u32(0, 10))
            .set_bitdepth(rng.range_u32(0, 34));

        let (rc_c1, out_c1) = p.c.validate(&t);
        let (rc_rs1, out_rs1) = p.rs.validate(&t);
        assert_eq!(rc_c1, rc_rs1, "pass 1 rc mismatch on {t:?}");
        assert_eq!(out_c1.0, out_rs1.0, "pass 1 struct mismatch on {t:?}");

        let (rc_c2, out_c2) = p.c.validate(&out_c1);
        let (rc_rs2, out_rs2) = p.rs.validate(&out_rs1);
        assert_eq!(rc_c2, rc_rs2, "pass 2 rc mismatch on {out_c1:?}");
        assert_eq!(out_c2.0, out_rs2.0, "pass 2 struct mismatch on {out_c1:?}");
    }
}

/// CONFIGS row 22 — composed pipeline: validate, then size the memory from the
/// resulting `cur_blocksize`.
#[test]
fn cfg_22_composed_pipeline() {
    let p = load_pair();
    let mut rng = Rng::new(SEED ^ 22);
    for _ in 0..200_000 {
        let t = valid_random(&mut rng);
        let (rc_c, out_c) = p.c.validate(&t);
        let (rc_rs, out_rs) = p.rs.validate(&t);
        assert_eq!(rc_c, rc_rs);
        assert_eq!(out_c.0, out_rs.0, "input {t:?}");
        assert_eq!(rc_c, 0, "valid_random should always validate: {t:?}");

        let sz_c = p.c.size_memory(out_c.cur_blocksize());
        let sz_rs = p.rs.size_memory(out_rs.cur_blocksize());
        assert_eq!(
            sz_c, sz_rs,
            "pipeline size_memory mismatch for cur_blocksize={}",
            out_c.cur_blocksize()
        );
    }
}
