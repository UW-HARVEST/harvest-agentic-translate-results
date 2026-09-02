//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` via `libloading` and
//! compares the `int` return value plus all 28 bytes of `struct tflac`.

mod common;
use common::*;

// ===========================================================================
// tflac_size_memory — rows S1..S9
// ===========================================================================

#[test]
fn s1_size_memory_zero() {
    diff_size_memory("S1", 0);
}

#[test]
fn s2_size_memory_sub_mask_granularity() {
    for b in 1..=15u32 {
        diff_size_memory("S2", b);
    }
}

#[test]
fn s3_size_memory_multiple_of_four_no_wrap() {
    let mut rng = Rng::new(0x5311_0003);
    for _ in 0..ITERS {
        let b = rng.range_u32(0, 0x0CCC_CCCC) & !3u32;
        diff_size_memory("S3", b);
    }
    for b in [4u32, 8, 16, 64, 1024, 4096, 65536, 1 << 20] {
        diff_size_memory("S3", b);
    }
}

#[test]
fn s4_size_memory_nonzero_residues_no_wrap() {
    let mut rng = Rng::new(0x5311_0004);
    for r in 1..=3u32 {
        for _ in 0..ITERS {
            let b = (rng.range_u32(0, 0x0CCC_CCCC) & !3u32) | r;
            diff_size_memory("S4", b);
        }
    }
}

#[test]
fn s5_size_memory_flac_legal_range() {
    let mut rng = Rng::new(0x5311_0005);
    for _ in 0..ITERS {
        diff_size_memory("S5", rng.range_u32(16, 65535));
    }
    // exhaustive over the whole legal blocksize range as well — it is cheap
    for b in 16..=65535u32 {
        diff_size_memory("S5", b);
    }
}

#[test]
fn s6_size_memory_multiply_by_five_wraps() {
    // masked > 0x33333333 makes `5U * masked` wrap, while 4U*b still fits.
    let mut rng = Rng::new(0x5311_0006);
    for _ in 0..ITERS {
        let b = rng.range_u32(0x0CCC_CCCD, 0x3FFF_FFFF);
        let got = diff_size_memory("S6", b);
        // sanity: confirm the row really is in the wrapping regime
        let masked = 15u32.wrapping_add(b.wrapping_mul(4)) & 0xFFFF_FFF0;
        assert!(masked as u64 * 5 > u32::MAX as u64, "S6 row not wrapping for b={b}");
        let _ = got;
    }
}

#[test]
fn s7_size_memory_multiply_by_four_wraps() {
    let mut rng = Rng::new(0x5311_0007);
    for _ in 0..ITERS {
        let b = rng.range_u32(0x4000_0000, u32::MAX);
        diff_size_memory("S7", b);
        assert!(b as u64 * 4 > u32::MAX as u64, "S7 row not wrapping for b={b}");
    }
}

#[test]
fn s8_size_memory_boundary_sweep() {
    let mut probes: Vec<u32> = Vec::new();
    for centre in [
        0x3FFF_FFFFu32,
        0x0CCC_CCCD,
        0x7FFF_FFFF,
        0x8000_0000,
        0xFFFF_FFFF,
        0x0000_0010,
        0x0001_0000,
        0x3333_3333,
    ] {
        for d in -2i64..=2 {
            let v = centre as i64 + d;
            if (0..=u32::MAX as i64).contains(&v) {
                probes.push(v as u32);
            }
        }
    }
    for b in probes {
        diff_size_memory("S8", b);
    }
}

#[test]
fn s9_size_memory_full_domain_random() {
    let mut rng = Rng::new(0x5311_0009);
    for _ in 0..(ITERS * 25) {
        diff_size_memory("S9", rng.next_u32());
    }
}

// ===========================================================================
// flac_validate — rows V1..V27
// ===========================================================================

#[test]
fn v1_all_defaults_random_sizes() {
    let mut rng = Rng::new(0x0001);
    for _ in 0..ITERS {
        let mut t = Tflac::poisoned();
        t.set_u32(OFF_BLOCKSIZE, rng.range_u32(16, 65535))
            .set_u32(OFF_SAMPLERATE, rng.range_u32(1, 655350))
            .set_u32(OFF_CHANNELS, rng.range_u32(1, 8))
            .set_u32(OFF_BITDEPTH, rng.range_u32(1, 32))
            .set_u8(OFF_CHANNEL_MODE, 0)
            .set_u8(OFF_MAX_RICE, 0)
            .set_u8(OFF_MIN_PO, 0)
            .set_u8(OFF_MAX_PO, 0);
        let (rc, out) = diff_validate("V1", &t);
        assert_eq!(rc, 0, "V1 must be accepted: {}", t.describe());
        assert_eq!(out.partition_order(), 0, "max_po=0 pins partition_order to 0");
        assert_eq!(out.cur_blocksize(), t.blocksize());
    }
}

#[test]
fn v2_auto_rice_low_bitdepth() {
    let mut rng = Rng::new(0x0002);
    for _ in 0..ITERS {
        let max_po = rng.range_u8(0, 15);
        let mut t = Tflac::poisoned();
        t.set_u32(OFF_BLOCKSIZE, rng.range_u32(16, 65535))
            .set_u32(OFF_SAMPLERATE, rng.range_u32(1, 655350))
            .set_u32(OFF_CHANNELS, rng.range_u32(1, 8))
            .set_u32(OFF_BITDEPTH, rng.range_u32(1, 16))
            .set_u8(OFF_CHANNEL_MODE, 0)
            .set_u8(OFF_MAX_RICE, 0)
            .set_u8(OFF_MAX_PO, max_po)
            .set_u8(OFF_MIN_PO, rng.range_u8(0, max_po));
        let (rc, out) = diff_validate("V2", &t);
        assert_eq!(rc, 0);
        assert_eq!(out.max_rice_value(), 14, "bitdepth<=16 auto-fills 14");
    }
}

#[test]
fn v3_auto_rice_high_bitdepth() {
    let mut rng = Rng::new(0x0003);
    for _ in 0..ITERS {
        let max_po = rng.range_u8(0, 15);
        let mut t = Tflac::poisoned();
        t.set_u32(OFF_BLOCKSIZE, rng.range_u32(16, 65535))
            .set_u32(OFF_SAMPLERATE, rng.range_u32(1, 655350))
            .set_u32(OFF_CHANNELS, rng.range_u32(1, 8))
            .set_u32(OFF_BITDEPTH, rng.range_u32(17, 32))
            .set_u8(OFF_CHANNEL_MODE, 0)
            .set_u8(OFF_MAX_RICE, 0)
            .set_u8(OFF_MAX_PO, max_po)
            .set_u8(OFF_MIN_PO, rng.range_u8(0, max_po));
        let (rc, out) = diff_validate("V3", &t);
        assert_eq!(rc, 0);
        assert_eq!(out.max_rice_value(), 30, "bitdepth>16 auto-fills 30");
    }
}

#[test]
fn v4_explicit_rice_values() {
    let mut rng = Rng::new(0x0004);
    for _ in 0..ITERS {
        let mut t = rng.valid_struct();
        t.set_u8(OFF_MAX_RICE, rng.range_u8(1, 30));
        let want = t.max_rice_value();
        let (rc, out) = diff_validate("V4", &t);
        assert_eq!(rc, 0);
        assert_eq!(out.max_rice_value(), want, "explicit rice value must be kept verbatim");
    }
}

#[test]
fn v5_rice_value_upper_boundary() {
    let mut rng = Rng::new(0x0005);
    for _ in 0..ITERS {
        let mut t = rng.valid_struct();
        t.set_u8(OFF_MAX_RICE, 30);
        let (rc, out) = diff_validate("V5", &t);
        assert_eq!(rc, 0, "max_rice_value=30 is accepted");
        assert_eq!(out.max_rice_value(), 30);
    }
}

#[test]
fn v6_rice_value_lower_nonzero_boundary() {
    let mut rng = Rng::new(0x0006);
    for _ in 0..ITERS {
        let mut t = rng.valid_struct();
        t.set_u8(OFF_MAX_RICE, 1);
        let (rc, out) = diff_validate("V6", &t);
        assert_eq!(rc, 0);
        assert_eq!(out.max_rice_value(), 1);
    }
}

#[test]
fn v7_stereo_mode_kept() {
    let mut rng = Rng::new(0x0007);
    for mode in 1..=3u8 {
        for _ in 0..ITERS {
            let mut t = rng.valid_struct();
            t.set_u32(OFF_CHANNELS, 2)
                .set_u32(OFF_BITDEPTH, rng.range_u32(1, 31))
                .set_u8(OFF_CHANNEL_MODE, mode);
            let (rc, out) = diff_validate("V7", &t);
            assert_eq!(rc, 0);
            assert_eq!(out.channel_mode(), mode, "channels==2 && bitdepth!=32 keeps the mode");
        }
    }
}

#[test]
fn v8_stereo_mode_reset_by_bitdepth_32() {
    let mut rng = Rng::new(0x0008);
    for mode in 1..=3u8 {
        for _ in 0..ITERS {
            let mut t = rng.valid_struct();
            t.set_u32(OFF_CHANNELS, 2)
                .set_u32(OFF_BITDEPTH, 32)
                .set_u8(OFF_CHANNEL_MODE, mode);
            let (rc, out) = diff_validate("V8", &t);
            assert_eq!(rc, 0);
            assert_eq!(out.channel_mode(), 0, "bitdepth==32 forces INDEPENDENT");
        }
    }
}

#[test]
fn v9_mode_reset_by_channel_count() {
    let mut rng = Rng::new(0x0009);
    for mode in 1..=3u8 {
        for ch in [1u32, 3, 4, 5, 6, 7, 8] {
            for _ in 0..(ITERS / 4) {
                let mut t = rng.valid_struct();
                t.set_u32(OFF_CHANNELS, ch).set_u8(OFF_CHANNEL_MODE, mode);
                let (rc, out) = diff_validate("V9", &t);
                assert_eq!(rc, 0);
                assert_eq!(out.channel_mode(), 0, "channels!=2 forces INDEPENDENT");
            }
        }
    }
}

#[test]
fn v10_mode_equal_to_mode_count() {
    let mut rng = Rng::new(0x000A);
    for _ in 0..ITERS {
        // kept branch
        let mut t = rng.valid_struct();
        t.set_u32(OFF_CHANNELS, 2)
            .set_u32(OFF_BITDEPTH, rng.range_u32(1, 31))
            .set_u8(OFF_CHANNEL_MODE, 4);
        let (rc, out) = diff_validate("V10-kept", &t);
        assert_eq!(rc, 0);
        assert_eq!(out.channel_mode(), 4, "C only compares against 0, so 4 survives");

        // reset branch
        let mut t = rng.valid_struct();
        t.set_u32(OFF_CHANNELS, rng.range_u32(3, 8)).set_u8(OFF_CHANNEL_MODE, 4);
        let (rc, out) = diff_validate("V10-reset", &t);
        assert_eq!(rc, 0);
        assert_eq!(out.channel_mode(), 0);
    }
}

#[test]
fn v11_out_of_range_enum_mode() {
    let mut rng = Rng::new(0x000B);
    // exhaustive over every out-of-range channel_mode byte, both branches
    for mode in 5..=255u8 {
        let mut t = rng.valid_struct();
        t.set_u32(OFF_CHANNELS, 2)
            .set_u32(OFF_BITDEPTH, rng.range_u32(1, 31))
            .set_u8(OFF_CHANNEL_MODE, mode);
        let (rc, out) = diff_validate("V11-kept", &t);
        assert_eq!(rc, 0);
        assert_eq!(out.channel_mode(), mode, "any nonzero mode survives the kept branch");

        let mut t = rng.valid_struct();
        t.set_u32(OFF_CHANNELS, 2).set_u32(OFF_BITDEPTH, 32).set_u8(OFF_CHANNEL_MODE, mode);
        let (rc, out) = diff_validate("V11-reset-bd32", &t);
        assert_eq!(rc, 0);
        assert_eq!(out.channel_mode(), 0);

        let mut t = rng.valid_struct();
        t.set_u32(OFF_CHANNELS, rng.range_u32(3, 8)).set_u8(OFF_CHANNEL_MODE, mode);
        let (rc, out) = diff_validate("V11-reset-ch", &t);
        assert_eq!(rc, 0);
        assert_eq!(out.channel_mode(), 0);
    }
}

#[test]
fn v12_min_equals_max_partition_order() {
    let mut rng = Rng::new(0x000C);
    for po in 0..=15u8 {
        for _ in 0..(ITERS / 4) {
            let mut t = rng.valid_struct();
            t.set_u8(OFF_MIN_PO, po).set_u8(OFF_MAX_PO, po);
            let (rc, out) = diff_validate("V12", &t);
            assert_eq!(rc, 0);
            assert_eq!(out.partition_order(), po, "loop cannot advance when min==max");
        }
    }
}

#[test]
fn v13_odd_blocksize_loop_cannot_advance() {
    let mut rng = Rng::new(0x000D);
    for _ in 0..ITERS {
        let max_po = rng.range_u8(1, 15);
        let min_po = rng.range_u8(0, max_po - 1);
        let mut t = rng.valid_struct();
        t.set_u32(OFF_BLOCKSIZE, rng.range_u32(8, 32767) * 2 + 1)
            .set_u8(OFF_MIN_PO, min_po)
            .set_u8(OFF_MAX_PO, max_po);
        let (rc, out) = diff_validate("V13", &t);
        assert_eq!(rc, 0);
        assert_eq!(out.partition_order(), min_po, "odd blocksize: no doubling divides it");
    }
}

#[test]
fn v14_maximal_loop_run() {
    // blocksize = 32768 = 2^15, min=0, max=15 -> loop advances all the way and
    // evaluates `1 << 16` at partition_order == 15.
    let mut t = Tflac::valid();
    t.set_u32(OFF_BLOCKSIZE, 32768).set_u8(OFF_MIN_PO, 0).set_u8(OFF_MAX_PO, 15);
    let (rc, out) = diff_validate("V14", &t);
    assert_eq!(rc, 0);
    assert_eq!(out.partition_order(), 15);
    assert_eq!(out.cur_blocksize(), 32768);
}

#[test]
fn v15_power_of_two_blocksizes() {
    let mut rng = Rng::new(0x000F);
    for k in 4..=15u32 {
        for _ in 0..(ITERS / 8) {
            let mut t = rng.valid_struct();
            t.set_u32(OFF_BLOCKSIZE, 1u32 << k).set_u8(OFF_MIN_PO, 0).set_u8(OFF_MAX_PO, 15);
            let (rc, out) = diff_validate("V15", &t);
            assert_eq!(rc, 0);
            assert_eq!(out.partition_order() as u32, k.min(15));
        }
    }
}

#[test]
fn v16_two_adic_valuation_cross_product() {
    let mut rng = Rng::new(0x0010);
    for _ in 0..(ITERS * 4) {
        // build blocksize = 2^k * odd, staying inside 16..=65535
        let k = rng.range_u32(0, 15);
        let pow = 1u32 << k;
        if pow > 65535 {
            continue;
        }
        let max_odd = (65535 / pow).max(1);
        let odd = (rng.range_u32(0, max_odd) | 1).min(if max_odd % 2 == 1 { max_odd } else { max_odd - 1 }).max(1);
        let bs = pow.saturating_mul(odd);
        if bs < 16 || bs > 65535 {
            continue;
        }
        let max_po = rng.range_u8(0, 15);
        let min_po = rng.range_u8(0, max_po);
        let mut t = rng.valid_struct();
        t.set_u32(OFF_BLOCKSIZE, bs).set_u8(OFF_MIN_PO, min_po).set_u8(OFF_MAX_PO, max_po);
        let (rc, out) = diff_validate("V16", &t);
        assert_eq!(rc, 0);
        // independent model of the C loop
        let mut expect = min_po;
        while bs % (1u32 << (expect as u32 + 1)) == 0 && expect < max_po {
            expect += 1;
        }
        assert_eq!(out.partition_order(), expect, "bs={bs} min={min_po} max={max_po}");
    }
}

#[test]
fn v17_full_order_cross_product() {
    let mut rng = Rng::new(0x0011);
    for max_po in 0..=15u8 {
        for min_po in 0..=max_po {
            for _ in 0..16 {
                let mut t = rng.valid_struct();
                t.set_u8(OFF_MIN_PO, min_po).set_u8(OFF_MAX_PO, max_po);
                let (rc, _) = diff_validate("V17", &t);
                assert_eq!(rc, 0);
            }
        }
    }
}

#[test]
fn v18_max_partition_order_upper_boundary() {
    let mut rng = Rng::new(0x0012);
    for min_po in 0..=15u8 {
        for _ in 0..(ITERS / 8) {
            let mut t = rng.valid_struct();
            t.set_u8(OFF_MIN_PO, min_po).set_u8(OFF_MAX_PO, 15);
            let (rc, _) = diff_validate("V18", &t);
            assert_eq!(rc, 0);
        }
    }
}

#[test]
fn v19_blocksize_lower_boundary() {
    let mut rng = Rng::new(0x0013);
    for max_po in 0..=15u8 {
        for min_po in 0..=max_po {
            let mut t = rng.valid_struct();
            t.set_u32(OFF_BLOCKSIZE, 16).set_u8(OFF_MIN_PO, min_po).set_u8(OFF_MAX_PO, max_po);
            let (rc, out) = diff_validate("V19", &t);
            assert_eq!(rc, 0, "blocksize=16 is the accepted lower boundary");
            assert_eq!(out.cur_blocksize(), 16);
        }
    }
}

#[test]
fn v20_blocksize_upper_boundary() {
    let mut rng = Rng::new(0x0014);
    for max_po in 0..=15u8 {
        for min_po in 0..=max_po {
            let mut t = rng.valid_struct();
            t.set_u32(OFF_BLOCKSIZE, 65535).set_u8(OFF_MIN_PO, min_po).set_u8(OFF_MAX_PO, max_po);
            let (rc, out) = diff_validate("V20", &t);
            assert_eq!(rc, 0, "blocksize=65535 is the accepted upper boundary");
            assert_eq!(out.partition_order(), min_po, "65535 is odd");
            assert_eq!(out.cur_blocksize(), 65535);
        }
    }
}

#[test]
fn v21_samplerate_boundaries() {
    let mut rng = Rng::new(0x0015);
    for sr in [1u32, 2, 8000, 44100, 48000, 655349, 655350] {
        for _ in 0..(ITERS / 8) {
            let mut t = rng.valid_struct();
            t.set_u32(OFF_SAMPLERATE, sr);
            let (rc, _) = diff_validate("V21", &t);
            assert_eq!(rc, 0, "samplerate {sr} must be accepted");
        }
    }
}

#[test]
fn v22_channels_x_bitdepth_exhaustive() {
    let mut rng = Rng::new(0x0016);
    for ch in 1..=8u32 {
        for bd in 1..=32u32 {
            for _ in 0..8 {
                let mut t = rng.valid_struct();
                t.set_u32(OFF_CHANNELS, ch).set_u32(OFF_BITDEPTH, bd);
                let (rc, out) = diff_validate("V22", &t);
                assert_eq!(rc, 0, "ch={ch} bd={bd} must be accepted");
                if t.max_rice_value() == 0 {
                    assert_eq!(out.max_rice_value(), if bd <= 16 { 14 } else { 30 });
                }
            }
        }
    }
}

#[test]
fn v23_auto_rice_split_boundary() {
    let mut rng = Rng::new(0x0017);
    for bd in [1u32, 15, 16, 17, 18, 31, 32] {
        for _ in 0..(ITERS / 8) {
            let mut t = rng.valid_struct();
            t.set_u32(OFF_BITDEPTH, bd).set_u8(OFF_MAX_RICE, 0);
            let (rc, out) = diff_validate("V23", &t);
            assert_eq!(rc, 0);
            assert_eq!(out.max_rice_value(), if bd <= 16 { 14 } else { 30 });
        }
    }
}

#[test]
fn v24_pre_dirtied_output_fields() {
    let mut rng = Rng::new(0x0018);
    for _ in 0..ITERS {
        let mut t = rng.valid_struct();
        // garbage in the fields flac_validate is supposed to overwrite,
        // plus garbage in the padding bytes
        t.set_u8(OFF_PARTITION_ORDER, rng.next_u8());
        t.set_u32(OFF_CUR_BLOCKSIZE, rng.next_u32());
        t.0[21] = rng.next_u8();
        t.0[22] = rng.next_u8();
        t.0[23] = rng.next_u8();
        let pad = [t.0[21], t.0[22], t.0[23]];
        let (rc, out) = diff_validate("V24", &t);
        assert_eq!(rc, 0);
        assert_eq!(out.cur_blocksize(), t.blocksize());
        assert_eq!([out.0[21], out.0[22], out.0[23]], pad, "padding must not be written");
    }
}

#[test]
fn v25_repeated_invocation() {
    let mut rng = Rng::new(0x0019);
    for _ in 0..ITERS {
        let t0 = rng.valid_struct();
        let (_, after1) = diff_validate("V25-pass1", &t0);
        let (_, after2) = diff_validate("V25-pass2", &after1);
        let (_, after3) = diff_validate("V25-pass3", &after2);
        assert_eq!(after2.0, after3.0, "validate must be idempotent from pass 2 on");
    }
}

#[test]
fn v26_fully_unconstrained_random_struct() {
    let mut rng = Rng::new(0x001A);
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    for _ in 0..(ITERS * 25) {
        let mut t = Tflac::poisoned();
        for b in t.0.iter_mut() {
            *b = rng.next_u8();
        }
        let (rc, _) = diff_validate("V26-bytes", &t);
        if rc == 0 { accepted += 1 } else { rejected += 1 }
    }
    // biased generator so the accept path is also hit often
    for _ in 0..(ITERS * 10) {
        let mut t = Tflac::poisoned();
        t.set_u32(OFF_BLOCKSIZE, rng.range_u32(0, 70000))
            .set_u32(OFF_SAMPLERATE, rng.range_u32(0, 700000))
            .set_u32(OFF_CHANNELS, rng.range_u32(0, 12))
            .set_u32(OFF_BITDEPTH, rng.range_u32(0, 40))
            .set_u8(OFF_CHANNEL_MODE, rng.next_u8())
            .set_u8(OFF_MAX_RICE, rng.range_u8(0, 40))
            .set_u8(OFF_MIN_PO, rng.range_u8(0, 20))
            .set_u8(OFF_MAX_PO, rng.range_u8(0, 20));
        let (rc, _) = diff_validate("V26-biased", &t);
        if rc == 0 { accepted += 1 } else { rejected += 1 }
    }
    assert!(accepted > 100, "generator never accepted (got {accepted})");
    assert!(rejected > 100, "generator never rejected (got {rejected})");
}

#[test]
fn v27_composed_pipeline() {
    // Drive the two entry points as a real consumer would: validate, then size
    // the memory for the resulting cur_blocksize. Compare the C pair against
    // the Rust pair so a divergence anywhere in the chain shows up.
    let p = pair();
    let mut rng = Rng::new(0x001B);
    for _ in 0..(ITERS * 2) {
        let mut t = Tflac::poisoned();
        t.set_u32(OFF_BLOCKSIZE, rng.range_u32(0, 70000))
            .set_u32(OFF_SAMPLERATE, rng.range_u32(0, 700000))
            .set_u32(OFF_CHANNELS, rng.range_u32(0, 10))
            .set_u32(OFF_BITDEPTH, rng.range_u32(0, 34))
            .set_u8(OFF_CHANNEL_MODE, rng.next_u8())
            .set_u8(OFF_MAX_RICE, rng.range_u8(0, 32))
            .set_u8(OFF_MIN_PO, rng.range_u8(0, 17))
            .set_u8(OFF_MAX_PO, rng.range_u8(0, 17));

        let mut tc = t;
        let mut tr = t;
        let rc = p.c.flac_validate(&mut tc);
        let rr = p.rs.flac_validate(&mut tr);
        assert_eq!(rc, rr, "V27 return diverged for {}", t.describe());
        assert_eq!(tc.0, tr.0, "V27 struct diverged for {}", t.describe());

        let sc = p.c.tflac_size_memory(tc.cur_blocksize());
        let sr = p.rs.tflac_size_memory(tr.cur_blocksize());
        assert_eq!(sc, sr, "V27 size_memory diverged after validate of {}", t.describe());
    }
}
