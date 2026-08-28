//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects through their exported C symbols and
//! compares the return value plus all 28 bytes of the `tflac` struct.

mod common;
use common::*;

// ===========================================================================
// tflac_size_memory (rows 1-6)
// ===========================================================================

#[test]
fn cfg_01_size_memory_zero() {
    assert_eq!(diff_size_memory(0), 15);
}

#[test]
fn cfg_02_size_memory_small_exhaustive() {
    for bs in 0u32..=4096 {
        diff_size_memory(bs);
    }
}

#[test]
fn cfg_03_size_memory_legal_range_random() {
    // Boundaries of the legal FLAC blocksize range, all residues mod 16, then
    // randomized values.
    for bs in [16u32, 17, 18, 19, 20, 31, 32, 33, 4095, 4096, 4097, 65534, 65535] {
        diff_size_memory(bs);
    }
    let mut rng = Rng::new(0x0303_0303_0303_0303);
    for _ in 0..20_000 {
        diff_size_memory(rng.range_u32(16, 65535));
    }
}

#[test]
fn cfg_04_size_memory_mul5_overflow() {
    // `blocksize * 4` still fits, but `5 * masked` wraps modulo 2^32.
    let lo = 0x0CCC_CCCDu32;
    let hi = 0x3FFF_FFFBu32;
    for bs in [lo, lo + 1, lo - 1, hi - 1, hi, 0x1999_9999, 0x2000_0000, 0x3333_3333] {
        diff_size_memory(bs);
    }
    let mut rng = Rng::new(0x0404_0404_0404_0404);
    for _ in 0..20_000 {
        let bs = rng.range_u32(lo, hi);
        let prod = 15u32.wrapping_add(bs.wrapping_mul(4));
        assert!(bs <= 0x3FFF_FFFB, "row precondition");
        let _ = prod;
        diff_size_memory(bs);
    }
}

#[test]
fn cfg_05_size_memory_mul4_overflow() {
    // `blocksize * 4` itself wraps around 2^32.
    for bs in [
        0x3FFF_FFFAu32,
        0x3FFF_FFFB,
        0x3FFF_FFFC,
        0x3FFF_FFFD,
        0x4000_0000,
        0x8000_0000,
        0xC000_0000,
        0xFFFF_FFF0,
        0xFFFF_FFFF,
    ] {
        diff_size_memory(bs);
    }
    let mut rng = Rng::new(0x0505_0505_0505_0505);
    for _ in 0..20_000 {
        diff_size_memory(rng.range_u32(0x4000_0000, 0xFFFF_FFFF));
    }
}

#[test]
fn cfg_06_size_memory_full_domain_sweep() {
    // Sweep the whole 2^32 domain with a large odd stride (~286k samples) so
    // every 16-residue class and both overflow regimes are hit repeatedly.
    let stride: u32 = 15_013;
    let mut bs: u32 = 0;
    loop {
        diff_size_memory(bs);
        match bs.checked_add(stride) {
            Some(n) => bs = n,
            None => break,
        }
    }
    diff_size_memory(u32::MAX);
    let mut rng = Rng::new(0x0606_0606_0606_0606);
    for _ in 0..50_000 {
        diff_size_memory(rng.next_u32());
    }
}

// ===========================================================================
// flac_validate — channel-mode decision matrix (rows 7-13)
// ===========================================================================

/// Helper: run a randomized sweep over the "free" fields while pinning the
/// axes a row is about, and additionally assert the observed C behaviour is the
/// documented one (so the row provably reaches the intended branch).
fn sweep<F: FnMut(&mut Rng) -> Fields>(seed: u64, iters: usize, mut make: F) -> Vec<Outcome> {
    let mut rng = Rng::new(seed);
    let mut out = Vec::new();
    for _ in 0..iters {
        let f = make(&mut rng);
        out.push(diff_validate(f));
    }
    out
}

#[test]
fn cfg_07_mode_independent_stereo() {
    for o in sweep(0x0707_0707, 3000, |rng| {
        let max_po = rng.range_u8(0, 15);
        Fields {
            blocksize: rng.range_u32(16, 65535),
            samplerate: rng.range_u32(1, 655350),
            channels: 2,
            bitdepth: rng.range_u32(1, 31),
            channel_mode: 0,
            max_rice_value: rng.range_u8(0, 30),
            min_partition_order: rng.range_u8(0, max_po),
            max_partition_order: max_po,
            partition_order: rng.next_u8(),
            padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
            cur_blocksize: rng.next_u32(),
        }
    }) {
        assert_eq!(o.ret, 0);
        assert_eq!(Fields::from_raw(o.out).channel_mode, 0);
    }
}

#[test]
fn cfg_08_mode_independent_nonstereo() {
    for ch in [1u32, 3, 4, 5, 6, 7, 8] {
        for o in sweep(0x0808_0808 + ch as u64, 400, |rng| {
            let max_po = rng.range_u8(0, 15);
            Fields {
                blocksize: rng.range_u32(16, 65535),
                samplerate: rng.range_u32(1, 655350),
                channels: ch,
                bitdepth: rng.range_u32(1, 32),
                channel_mode: 0,
                max_rice_value: rng.range_u8(0, 30),
                min_partition_order: rng.range_u8(0, max_po),
                max_partition_order: max_po,
                partition_order: rng.next_u8(),
                padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
                cur_blocksize: rng.next_u32(),
            }
        }) {
            assert_eq!(o.ret, 0);
            assert_eq!(Fields::from_raw(o.out).channel_mode, 0);
        }
    }
}

#[test]
fn cfg_09_mode_stereo_preserved() {
    for mode in [1u8, 2, 3] {
        for o in sweep(0x0909_0909 + mode as u64, 1500, |rng| {
            let max_po = rng.range_u8(0, 15);
            Fields {
                blocksize: rng.range_u32(16, 65535),
                samplerate: rng.range_u32(1, 655350),
                channels: 2,
                bitdepth: rng.range_u32(1, 31),
                channel_mode: mode,
                max_rice_value: rng.range_u8(0, 30),
                min_partition_order: rng.range_u8(0, max_po),
                max_partition_order: max_po,
                partition_order: rng.next_u8(),
                padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
                cur_blocksize: rng.next_u32(),
            }
        }) {
            assert_eq!(o.ret, 0);
            assert_eq!(
                Fields::from_raw(o.out).channel_mode, mode,
                "stereo + bitdepth<32 must preserve the mode"
            );
        }
    }
}

#[test]
fn cfg_10_mode_reset_by_bitdepth32() {
    for mode in [1u8, 2, 3] {
        for o in sweep(0x1010_1010 + mode as u64, 1500, |rng| {
            let max_po = rng.range_u8(0, 15);
            Fields {
                blocksize: rng.range_u32(16, 65535),
                samplerate: rng.range_u32(1, 655350),
                channels: 2,
                bitdepth: 32,
                channel_mode: mode,
                max_rice_value: rng.range_u8(0, 30),
                min_partition_order: rng.range_u8(0, max_po),
                max_partition_order: max_po,
                partition_order: rng.next_u8(),
                padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
                cur_blocksize: rng.next_u32(),
            }
        }) {
            assert_eq!(o.ret, 0);
            assert_eq!(Fields::from_raw(o.out).channel_mode, 0);
        }
    }
}

#[test]
fn cfg_11_mode_reset_by_channels() {
    for mode in [1u8, 2, 3] {
        for ch in [1u32, 3, 4, 5, 6, 7, 8] {
            for o in sweep(0x1111_1111 + mode as u64 * 31 + ch as u64, 200, |rng| {
                let max_po = rng.range_u8(0, 15);
                Fields {
                    blocksize: rng.range_u32(16, 65535),
                    samplerate: rng.range_u32(1, 655350),
                    channels: ch,
                    bitdepth: rng.range_u32(1, 32),
                    channel_mode: mode,
                    max_rice_value: rng.range_u8(0, 30),
                    min_partition_order: rng.range_u8(0, max_po),
                    max_partition_order: max_po,
                    partition_order: rng.next_u8(),
                    padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
                    cur_blocksize: rng.next_u32(),
                }
            }) {
                assert_eq!(o.ret, 0);
                assert_eq!(Fields::from_raw(o.out).channel_mode, 0);
            }
        }
    }
}

#[test]
fn cfg_12_mode_out_of_range_preserved() {
    // channel_mode 4..=255 has no valid TFLAC_CHANNEL_MODE variant; the C only
    // compares against TFLAC_CHANNEL_INDEPENDENT, so with stereo + bitdepth<32
    // the raw byte survives untouched.
    for mode in 4u8..=255 {
        for o in sweep(0x1212_1212 + mode as u64, 12, |rng| {
            let max_po = rng.range_u8(0, 15);
            Fields {
                blocksize: rng.range_u32(16, 65535),
                samplerate: rng.range_u32(1, 655350),
                channels: 2,
                bitdepth: rng.range_u32(1, 31),
                channel_mode: mode,
                max_rice_value: rng.range_u8(0, 30),
                min_partition_order: rng.range_u8(0, max_po),
                max_partition_order: max_po,
                partition_order: rng.next_u8(),
                padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
                cur_blocksize: rng.next_u32(),
            }
        }) {
            assert_eq!(o.ret, 0);
            assert_eq!(Fields::from_raw(o.out).channel_mode, mode);
        }
    }
}

#[test]
fn cfg_13_mode_out_of_range_reset() {
    for mode in 4u8..=255 {
        // channels != 2
        for o in sweep(0x1313_1313 + mode as u64, 6, |rng| {
            let max_po = rng.range_u8(0, 15);
            Fields {
                blocksize: rng.range_u32(16, 65535),
                samplerate: rng.range_u32(1, 655350),
                channels: rng.pick(&[1u32, 3, 4, 5, 6, 7, 8]),
                bitdepth: rng.range_u32(1, 32),
                channel_mode: mode,
                max_rice_value: rng.range_u8(0, 30),
                min_partition_order: rng.range_u8(0, max_po),
                max_partition_order: max_po,
                partition_order: rng.next_u8(),
                padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
                cur_blocksize: rng.next_u32(),
            }
        }) {
            assert_eq!(o.ret, 0);
            assert_eq!(Fields::from_raw(o.out).channel_mode, 0);
        }
        // channels == 2 but bitdepth == 32
        for o in sweep(0x1314_0000 + mode as u64, 6, |rng| {
            let max_po = rng.range_u8(0, 15);
            Fields {
                blocksize: rng.range_u32(16, 65535),
                samplerate: rng.range_u32(1, 655350),
                channels: 2,
                bitdepth: 32,
                channel_mode: mode,
                max_rice_value: rng.range_u8(0, 30),
                min_partition_order: rng.range_u8(0, max_po),
                max_partition_order: max_po,
                partition_order: rng.next_u8(),
                padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
                cur_blocksize: rng.next_u32(),
            }
        }) {
            assert_eq!(o.ret, 0);
            assert_eq!(Fields::from_raw(o.out).channel_mode, 0);
        }
    }
}

// ===========================================================================
// flac_validate — max_rice_value derivation (rows 14-17)
// ===========================================================================

#[test]
fn cfg_14_rice_auto_14() {
    for bd in 1u32..=16 {
        for o in sweep(0x1414_0000 + bd as u64, 150, |rng| {
            let max_po = rng.range_u8(0, 15);
            Fields {
                blocksize: rng.range_u32(16, 65535),
                samplerate: rng.range_u32(1, 655350),
                channels: rng.range_u32(1, 8),
                bitdepth: bd,
                channel_mode: rng.next_u8(),
                max_rice_value: 0,
                min_partition_order: rng.range_u8(0, max_po),
                max_partition_order: max_po,
                partition_order: rng.next_u8(),
                padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
                cur_blocksize: rng.next_u32(),
            }
        }) {
            assert_eq!(o.ret, 0);
            assert_eq!(Fields::from_raw(o.out).max_rice_value, 14);
        }
    }
}

#[test]
fn cfg_15_rice_auto_30() {
    for bd in 17u32..=32 {
        for o in sweep(0x1515_0000 + bd as u64, 150, |rng| {
            let max_po = rng.range_u8(0, 15);
            Fields {
                blocksize: rng.range_u32(16, 65535),
                samplerate: rng.range_u32(1, 655350),
                channels: rng.range_u32(1, 8),
                bitdepth: bd,
                channel_mode: rng.next_u8(),
                max_rice_value: 0,
                min_partition_order: rng.range_u8(0, max_po),
                max_partition_order: max_po,
                partition_order: rng.next_u8(),
                padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
                cur_blocksize: rng.next_u32(),
            }
        }) {
            assert_eq!(o.ret, 0);
            assert_eq!(Fields::from_raw(o.out).max_rice_value, 30);
        }
    }
}

#[test]
fn cfg_16_rice_explicit_preserved() {
    for mrv in 1u8..=30 {
        for o in sweep(0x1616_0000 + mrv as u64, 120, |rng| {
            let max_po = rng.range_u8(0, 15);
            Fields {
                blocksize: rng.range_u32(16, 65535),
                samplerate: rng.range_u32(1, 655350),
                channels: rng.range_u32(1, 8),
                bitdepth: rng.range_u32(1, 32),
                channel_mode: rng.next_u8(),
                max_rice_value: mrv,
                min_partition_order: rng.range_u8(0, max_po),
                max_partition_order: max_po,
                partition_order: rng.next_u8(),
                padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
                cur_blocksize: rng.next_u32(),
            }
        }) {
            assert_eq!(o.ret, 0);
            assert_eq!(Fields::from_raw(o.out).max_rice_value, mrv);
        }
    }
}

#[test]
fn cfg_17_rice_boundary_16_17() {
    for (bd, want) in [(16u32, 14u8), (17u32, 30u8)] {
        let o = diff_validate(Fields { bitdepth: bd, max_rice_value: 0, ..Default::default() });
        assert_eq!(o.ret, 0);
        assert_eq!(Fields::from_raw(o.out).max_rice_value, want);
    }
}

// ===========================================================================
// flac_validate — partition-order loop (rows 18-24)
// ===========================================================================

#[test]
fn cfg_18_partition_min_eq_max() {
    for po in 0u8..=15 {
        for o in sweep(0x1818_0000 + po as u64, 200, |rng| Fields {
            blocksize: rng.range_u32(16, 65535),
            samplerate: rng.range_u32(1, 655350),
            channels: rng.range_u32(1, 8),
            bitdepth: rng.range_u32(1, 32),
            channel_mode: rng.next_u8(),
            max_rice_value: rng.range_u8(0, 30),
            min_partition_order: po,
            max_partition_order: po,
            partition_order: rng.next_u8(),
            padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
            cur_blocksize: rng.next_u32(),
        }) {
            assert_eq!(o.ret, 0);
            assert_eq!(Fields::from_raw(o.out).partition_order, po);
        }
    }
}

#[test]
fn cfg_19_partition_pow2_blocksize() {
    let o = diff_validate(Fields {
        blocksize: 32768,
        min_partition_order: 0,
        max_partition_order: 15,
        ..Default::default()
    });
    assert_eq!(o.ret, 0);
    assert_eq!(Fields::from_raw(o.out).partition_order, 15);
    assert_eq!(Fields::from_raw(o.out).cur_blocksize, 32768);
}

#[test]
fn cfg_20_partition_odd_blocksize() {
    for bs in [17u32, 4097, 65535, 19, 12345] {
        let o = diff_validate(Fields {
            blocksize: bs,
            min_partition_order: 0,
            max_partition_order: 15,
            ..Default::default()
        });
        assert_eq!(o.ret, 0);
        assert_eq!(Fields::from_raw(o.out).partition_order, 0, "odd blocksize: no advance");
    }
}

#[test]
fn cfg_21_partition_stops_at_v2() {
    let o = diff_validate(Fields {
        blocksize: 4096,
        min_partition_order: 0,
        max_partition_order: 15,
        ..Default::default()
    });
    assert_eq!(o.ret, 0);
    assert_eq!(Fields::from_raw(o.out).partition_order, 12);
}

#[test]
fn cfg_22_partition_blocksize_16() {
    let o = diff_validate(Fields {
        blocksize: 16,
        min_partition_order: 0,
        max_partition_order: 15,
        ..Default::default()
    });
    assert_eq!(o.ret, 0);
    assert_eq!(Fields::from_raw(o.out).partition_order, 4);
}

#[test]
fn cfg_23_partition_full_cross() {
    let sizes = [16u32, 17, 24, 32, 48, 96, 4096, 32768, 49152, 65534, 65535];
    for &bs in &sizes {
        for max_po in 0u8..=15 {
            for min_po in 0u8..=max_po {
                let o = diff_validate(Fields {
                    blocksize: bs,
                    min_partition_order: min_po,
                    max_partition_order: max_po,
                    partition_order: 0xEE,
                    cur_blocksize: 0xDEAD_BEEF,
                    ..Default::default()
                });
                assert_eq!(o.ret, 0);
                let got = Fields::from_raw(o.out);
                assert_eq!(got.partition_order, expected_partition_order(bs, min_po, max_po));
                assert_eq!(got.cur_blocksize, bs);
            }
        }
    }
}

#[test]
fn cfg_24_partition_random() {
    let mut rng = Rng::new(0x2424_2424_2424_2424);
    for _ in 0..20_000 {
        let max_po = rng.range_u8(0, 15);
        let min_po = rng.range_u8(0, max_po);
        let bs = rng.range_u32(16, 65535);
        let o = diff_validate(Fields {
            blocksize: bs,
            samplerate: rng.range_u32(1, 655350),
            channels: rng.range_u32(1, 8),
            bitdepth: rng.range_u32(1, 32),
            channel_mode: rng.next_u8(),
            max_rice_value: rng.range_u8(0, 30),
            min_partition_order: min_po,
            max_partition_order: max_po,
            partition_order: rng.next_u8(),
            padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
            cur_blocksize: rng.next_u32(),
        });
        assert_eq!(o.ret, 0);
        assert_eq!(
            Fields::from_raw(o.out).partition_order,
            expected_partition_order(bs, min_po, max_po)
        );
    }
}

// ===========================================================================
// flac_validate — boundaries, garbage output fields, fuzz (rows 25-30)
// ===========================================================================

#[test]
fn cfg_25_valid_boundaries() {
    for bs in [16u32, 17, 65534, 65535] {
        for sr in [1u32, 2, 655349, 655350] {
            for ch in 1u32..=8 {
                for bd in 1u32..=32 {
                    let o = diff_validate(Fields {
                        blocksize: bs,
                        samplerate: sr,
                        channels: ch,
                        bitdepth: bd,
                        channel_mode: (bd % 5) as u8,
                        max_rice_value: (bd % 31) as u8,
                        min_partition_order: 0,
                        max_partition_order: 15,
                        partition_order: 0x7F,
                        padding: [1, 2, 3],
                        cur_blocksize: 0xA5A5_A5A5,
                    });
                    assert_eq!(o.ret, 0);
                }
            }
        }
    }
}

#[test]
fn cfg_26_output_fields_and_padding_garbage() {
    let mut rng = Rng::new(0x2626_2626_2626_2626);
    for _ in 0..5_000 {
        let mut f = rng.valid_fields();
        f.partition_order = 0xEE;
        f.cur_blocksize = 0xDEAD_BEEF;
        f.padding = [0xAA, 0xAA, 0xAA];
        let o = diff_validate(f);
        assert_eq!(o.ret, 0);
        let got = Fields::from_raw(o.out);
        assert_eq!(got.padding, [0xAA, 0xAA, 0xAA], "padding must not be touched");
        assert_eq!(got.cur_blocksize, f.blocksize);
    }
}

#[test]
fn cfg_27_random_valid_fuzz() {
    let mut rng = Rng::new(0x2727_2727_2727_2727);
    for _ in 0..20_000 {
        let f = rng.valid_fields();
        let o = diff_validate(f);
        assert_eq!(o.ret, 0, "randomized valid config must be accepted: {f:?}");
    }
}

#[test]
fn cfg_28_random_raw_fuzz() {
    // Completely random 28-byte struct images: valid and invalid mixed, every
    // channel_mode byte reachable, every field possibly out of range.
    let mut rng = Rng::new(0x2828_2828_2828_2828);
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    for _ in 0..50_000 {
        let o = diff_validate_raw(rng.raw());
        if o.ret == 0 {
            accepted += 1;
        } else {
            rejected += 1;
        }
    }
    // Random 32-bit fields are almost never in range, so add a biased mix that
    // reaches the accept path too.
    for _ in 0..50_000 {
        let mut f = rng.valid_fields();
        // Perturb one byte-sized field into its full range.
        match rng.next_u64() % 4 {
            0 => f.channel_mode = rng.next_u8(),
            1 => f.max_rice_value = rng.next_u8(),
            2 => f.min_partition_order = rng.next_u8(),
            _ => f.max_partition_order = rng.next_u8(),
        }
        let o = diff_validate(f);
        if o.ret == 0 {
            accepted += 1;
        } else {
            rejected += 1;
        }
    }
    assert!(accepted > 1000, "expected the accept path to be reached often, got {accepted}");
    assert!(rejected > 1000, "expected the reject path to be reached often, got {rejected}");
}

#[test]
fn cfg_29_repeated_calls() {
    let mut rng = Rng::new(0x2929_2929_2929_2929);
    for _ in 0..5_000 {
        let f = rng.valid_fields();
        let l = libs();
        let mut cbuf = f.to_raw();
        let mut rbuf = f.to_raw();
        for call in 0..3 {
            let cret = unsafe { (l.c.validate)(cbuf.0.as_mut_ptr()) };
            let rret = unsafe { (l.rust.validate)(rbuf.0.as_mut_ptr()) };
            assert_eq!(cret, rret, "call #{call} return mismatch for {f:?}");
            assert_eq!(
                cbuf, rbuf,
                "call #{call} struct mismatch for {f:?}: C={:?} Rust={:?}",
                Fields::from_raw(cbuf),
                Fields::from_raw(rbuf)
            );
        }
    }
}

#[test]
fn cfg_30_pipeline_validate_then_size() {
    // Composed pipeline: validate, then feed the resulting cur_blocksize into
    // tflac_size_memory (the way a real consumer sizes its buffers), using each
    // library's OWN output as the next input.
    let l = libs();
    let mut rng = Rng::new(0x3030_3030_3030_3030);
    for _ in 0..10_000 {
        let f = rng.valid_fields();
        let mut cbuf = f.to_raw();
        let mut rbuf = f.to_raw();
        let cret = unsafe { (l.c.validate)(cbuf.0.as_mut_ptr()) };
        let rret = unsafe { (l.rust.validate)(rbuf.0.as_mut_ptr()) };
        assert_eq!(cret, rret);
        assert_eq!(cbuf, rbuf);
        let cbs = Fields::from_raw(cbuf).cur_blocksize;
        let rbs = Fields::from_raw(rbuf).cur_blocksize;
        let csz = unsafe { (l.c.size_memory)(cbs) };
        let rsz = unsafe { (l.rust.size_memory)(rbs) };
        assert_eq!(csz, rsz, "pipeline size mismatch for {f:?}");
    }
}
