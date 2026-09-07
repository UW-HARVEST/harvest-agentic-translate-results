//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every call goes through `libloading` into the C `.so` and the Rust `cdylib`;
//! no Rust function is ever called directly.

mod common;

use common::{diff_size_memory, diff_validate, Raw, Rng};

// ===========================================================================
// Row 1..4 — tflac_size_memory
// ===========================================================================

#[test]
fn cfg_01_size_memory_zero() {
    let v = diff_size_memory(0);
    // 15 + 5*((15+0) & 0xFFFFFFF0) == 15 + 5*0 == 15
    assert_eq!(v, 15);
}

#[test]
fn cfg_02_size_memory_small_exhaustive() {
    for b in 0u32..=65535 {
        diff_size_memory(b);
    }
}

#[test]
fn cfg_03_size_memory_wrapping() {
    let mut cases = vec![
        0,
        1,
        2,
        3,
        4,
        15,
        16,
        17,
        (1u32 << 30) - 1,
        1u32 << 30,
        (1u32 << 30) + 1,
        0x3FFF_FFFC,
        0x4000_0000,
        0x8000_0000,
        0xC000_0000,
        0xFFFF_FFF0,
        0xFFFF_FFF1,
        0xFFFF_FFFC,
        0xFFFF_FFFD,
        u32::MAX,
    ];
    // Every value near a multiple of 2^30 (where `*4` wraps) and near the
    // point where `+15` carries.
    for k in 0..64u32 {
        cases.push(u32::MAX - k);
        cases.push((1u32 << 30).wrapping_sub(k));
        cases.push((1u32 << 30).wrapping_add(k));
        cases.push(0x4000_0000u32.wrapping_mul(3).wrapping_add(k));
    }
    for b in cases {
        diff_size_memory(b);
    }
}

#[test]
fn cfg_04_size_memory_random() {
    let mut rng = Rng::new(0xC0FF_EE12_3456_789A);
    for _ in 0..200_000 {
        diff_size_memory(rng.next_u32());
    }
}

/// Also referenced from ERRORS.md row G4.
#[test]
fn size_memory_wrapping() {
    cfg_03_size_memory_wrapping();
}

// ===========================================================================
// Row 5..9 — max_rice_value defaulting
// ===========================================================================

#[test]
fn cfg_05_independent_minimal() {
    // channel_mode=0, max_rice_value=0, bitdepth<=16, max_partition_order=0,
    // min==max==0, odd blocksize => loop body never entered.
    for bitdepth in 1u32..=16 {
        for blocksize in [17u32, 4097, 65535] {
            let mut r = Raw::zeroed();
            r.set_blocksize(blocksize)
                .set_samplerate(44100)
                .set_channels(1)
                .set_bitdepth(bitdepth);
            let (rc, out) = diff_validate(r);
            assert_eq!(rc, 0);
            assert_eq!(out.max_rice_value(), 14, "bitdepth {bitdepth}");
            assert_eq!(out.partition_order(), 0);
            assert_eq!(out.cur_blocksize(), blocksize);
        }
    }
}

#[test]
fn cfg_06_rice_default_hi_bitdepth() {
    for bitdepth in 17u32..=32 {
        let mut r = Raw::valid();
        r.set_bitdepth(bitdepth).set_max_rice_value(0);
        let (rc, out) = diff_validate(r);
        assert_eq!(rc, 0);
        assert_eq!(out.max_rice_value(), 30, "bitdepth {bitdepth}");
    }
}

#[test]
fn cfg_07_rice_default_bitdepth_16() {
    // Rows 7 and 8: the 16/17 boundary of the rice default.
    let mut r16 = Raw::valid();
    r16.set_bitdepth(16).set_max_rice_value(0);
    assert_eq!(diff_validate(r16).1.max_rice_value(), 14);

    let mut r17 = Raw::valid();
    r17.set_bitdepth(17).set_max_rice_value(0);
    assert_eq!(diff_validate(r17).1.max_rice_value(), 30);
}

#[test]
fn cfg_09_rice_explicit_all_values() {
    for mrv in 1u8..=30 {
        for bitdepth in [1u32, 16, 17, 31, 32] {
            let mut r = Raw::valid();
            r.set_bitdepth(bitdepth).set_max_rice_value(mrv);
            let (rc, out) = diff_validate(r);
            assert_eq!(rc, 0);
            assert_eq!(out.max_rice_value(), mrv, "explicit value must be kept");
        }
    }
}

// ===========================================================================
// Row 10..13 — channel_mode fix-up
// ===========================================================================

#[test]
fn cfg_10_stereo_mode_preserved() {
    for mode in 1u8..=3 {
        for bitdepth in [1u32, 8, 16, 17, 31] {
            let mut r = Raw::valid();
            r.set_channels(2).set_bitdepth(bitdepth).set_channel_mode(mode);
            let (rc, out) = diff_validate(r);
            assert_eq!(rc, 0);
            assert_eq!(
                out.channel_mode(),
                mode,
                "mode {mode} must survive with channels=2, bitdepth={bitdepth}"
            );
        }
    }
}

#[test]
fn cfg_11_stereo_mode_reset_bitdepth32() {
    for mode in 1u8..=3 {
        let mut r = Raw::valid();
        r.set_channels(2).set_bitdepth(32).set_channel_mode(mode);
        let (rc, out) = diff_validate(r);
        assert_eq!(rc, 0);
        assert_eq!(out.channel_mode(), 0, "bitdepth 32 forces INDEPENDENT");
    }
}

#[test]
fn cfg_12_mode_reset_wrong_channels() {
    for mode in 1u8..=3 {
        for channels in [1u32, 3, 4, 5, 6, 7, 8] {
            for bitdepth in [1u32, 16, 31] {
                let mut r = Raw::valid();
                r.set_channels(channels)
                    .set_bitdepth(bitdepth)
                    .set_channel_mode(mode);
                let (rc, out) = diff_validate(r);
                assert_eq!(rc, 0);
                assert_eq!(
                    out.channel_mode(),
                    0,
                    "channels={channels} forces INDEPENDENT"
                );
            }
        }
    }
}

#[test]
fn cfg_13_mode_out_of_range_matrix() {
    // C enums accept any int; `channel_mode` is a uint8_t field so every one of
    // the 256 bit patterns is reachable. Only `!= 0` matters to the C.
    for mode in 0u8..=255 {
        for channels in [1u32, 2, 3, 8] {
            for bitdepth in [1u32, 16, 31, 32] {
                let mut r = Raw::valid();
                r.set_channels(channels)
                    .set_bitdepth(bitdepth)
                    .set_channel_mode(mode);
                let (rc, out) = diff_validate(r);
                assert_eq!(rc, 0);
                let expect = if mode != 0 && (channels != 2 || bitdepth == 32) {
                    0
                } else {
                    mode
                };
                assert_eq!(out.channel_mode(), expect);
            }
        }
    }
}

// ===========================================================================
// Row 14..20 — partition_order search loop
// ===========================================================================

/// Reference model of the C loop, used only to cross-check the shared answer
/// that C and Rust already agreed on.
fn expected_partition_order(blocksize: u32, min: u8, max: u8) -> u8 {
    let mut po = min;
    while blocksize % 1u32.wrapping_shl(po as u32 + 1) == 0 && po < max {
        po += 1;
    }
    po
}

#[test]
fn cfg_14_odd_blocksize_no_advance() {
    for blocksize in [17u32, 4097, 65535, 12345, 999] {
        for min in 0u8..=15 {
            let mut r = Raw::valid();
            r.set_blocksize(blocksize)
                .set_min_partition_order(min)
                .set_max_partition_order(15);
            let (rc, out) = diff_validate(r);
            assert_eq!(rc, 0);
            assert_eq!(out.partition_order(), min, "odd blocksize cannot advance");
            assert_eq!(out.cur_blocksize(), blocksize);
        }
    }
}

#[test]
fn cfg_15_v2_one() {
    // v2(4098) == 1: 4098 % 2 == 0 but 4098 % 4 != 0 -> exactly one increment.
    for blocksize in [4098u32, 18, 66, 65534] {
        let mut r = Raw::valid();
        r.set_blocksize(blocksize)
            .set_min_partition_order(0)
            .set_max_partition_order(15);
        let (rc, out) = diff_validate(r);
        assert_eq!(rc, 0);
        assert_eq!(out.partition_order(), 1, "blocksize {blocksize}");
    }
}

#[test]
fn cfg_16_loop_saturates_at_max() {
    // v2(32768) == 15 >= max+1 for every max <= 14, so the loop saturates.
    for max in 0u8..=14 {
        let mut r = Raw::valid();
        r.set_blocksize(32768)
            .set_min_partition_order(0)
            .set_max_partition_order(max);
        let (rc, out) = diff_validate(r);
        assert_eq!(rc, 0);
        assert_eq!(out.partition_order(), max, "must saturate at max={max}");
    }
}

#[test]
fn cfg_17_loop_stops_below_max() {
    // v2(4096) == 12 -> stops at 12 even though max is 15.
    let mut r = Raw::valid();
    r.set_blocksize(4096)
        .set_min_partition_order(0)
        .set_max_partition_order(15);
    let (rc, out) = diff_validate(r);
    assert_eq!(rc, 0);
    assert_eq!(out.partition_order(), 12);
}

#[test]
fn cfg_18_min_eq_max_all() {
    for order in 0u8..=15 {
        for blocksize in [32768u32, 65536 - 32768, 4096, 16] {
            let mut r = Raw::valid();
            r.set_blocksize(blocksize)
                .set_min_partition_order(order)
                .set_max_partition_order(order);
            let (rc, out) = diff_validate(r);
            assert_eq!(rc, 0);
            assert_eq!(out.partition_order(), order, "min==max pins the order");
        }
    }
}

#[test]
fn cfg_19_partition_order_matrix() {
    for blocksize in [16u32, 4096, 32768, 65535, 65534, 49152, 1024, 20] {
        for min in 0u8..=15 {
            for max in min..=15 {
                let mut r = Raw::valid();
                r.set_blocksize(blocksize)
                    .set_min_partition_order(min)
                    .set_max_partition_order(max);
                let (rc, out) = diff_validate(r);
                assert_eq!(rc, 0);
                assert_eq!(
                    out.partition_order(),
                    expected_partition_order(blocksize, min, max),
                    "blocksize={blocksize} min={min} max={max}"
                );
            }
        }
    }
}

#[test]
fn cfg_20_blocksize_boundaries() {
    for blocksize in [16u32, 17, 18, 32, 65533, 65534, 65535] {
        let mut r = Raw::valid();
        r.set_blocksize(blocksize).set_max_partition_order(15);
        let (rc, out) = diff_validate(r);
        assert_eq!(rc, 0);
        assert_eq!(out.cur_blocksize(), blocksize);
    }
}

// ===========================================================================
// Row 21..25
// ===========================================================================

#[test]
fn cfg_21_full_valid_cross_product() {
    for samplerate in [1u32, 8000, 44100, 655349, 655350] {
        for channels in 1u32..=8 {
            for bitdepth in 1u32..=32 {
                for mode in 0u8..=3 {
                    let mut r = Raw::valid();
                    r.set_samplerate(samplerate)
                        .set_channels(channels)
                        .set_bitdepth(bitdepth)
                        .set_channel_mode(mode);
                    let (rc, out) = diff_validate(r);
                    assert_eq!(rc, 0);
                    assert_eq!(out.samplerate(), samplerate);
                    assert_eq!(out.channels(), channels);
                    assert_eq!(out.bitdepth(), bitdepth);
                }
            }
        }
    }
}

#[test]
fn cfg_22_padding_untouched_valid() {
    let mut rng = Rng::new(0x1234_5678_9ABC_DEF0);
    for _ in 0..2_000 {
        let mut r = Raw::valid();
        // garbage in the tail padding (offsets 21..24)
        r.0[21] = rng.next_u8();
        r.0[22] = rng.next_u8();
        r.0[23] = rng.next_u8();
        let before = r.0[21..24].to_vec();
        let (rc, out) = diff_validate(r);
        assert_eq!(rc, 0);
        assert_eq!(&out.0[21..24], &before[..], "padding must round-trip");
    }
}

#[test]
fn cfg_23_random_fuzz_full_struct() {
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_0001);
    let mut ok = 0usize;
    for _ in 0..300_000 {
        let mut r = Raw::zeroed();
        rng.fill(&mut r.0);
        // Bias the u32 fields towards the valid ranges roughly half the time so
        // the success path is reached even in the "fully random" row.
        if rng.next_u8() & 1 == 0 {
            r.set_blocksize(rng.range_u32(1, 70_000))
                .set_samplerate(rng.range_u32(0, 700_000))
                .set_channels(rng.range_u32(0, 10))
                .set_bitdepth(rng.range_u32(0, 34));
        }
        let (rc, _) = diff_validate(r);
        if rc == 0 {
            ok += 1;
        }
    }
    assert!(ok > 0, "fuzz never reached the success path");
}

#[test]
fn cfg_24_random_fuzz_valid_only() {
    let mut rng = Rng::new(0xABCD_0F0F_1111_2222);
    for _ in 0..200_000 {
        let mut r = Raw::zeroed();
        rng.fill(&mut r.0);
        let max = rng.range_u32(0, 15) as u8;
        let min = rng.range_u32(0, max as u32) as u8;
        r.set_blocksize(rng.range_u32(16, 65535))
            .set_samplerate(rng.range_u32(1, 655_350))
            .set_channels(rng.range_u32(1, 8))
            .set_bitdepth(rng.range_u32(1, 32))
            .set_min_partition_order(min)
            .set_max_partition_order(max);
        // max_rice_value: 0 (auto) or a valid explicit value
        let mrv = if rng.next_u8() & 3 == 0 {
            0
        } else {
            rng.range_u32(1, 30) as u8
        };
        r.set_max_rice_value(mrv);

        let (rc, out) = diff_validate(r);
        assert_eq!(rc, 0, "should be valid: {r:?}");
        assert_eq!(out.cur_blocksize(), out.blocksize());
        assert_eq!(
            out.partition_order(),
            expected_partition_order(out.blocksize(), min, max)
        );
    }
}

#[test]
fn cfg_25_repeated_invocation() {
    let mut rng = Rng::new(0x5555_6666_7777_8888);
    for _ in 0..20_000 {
        let mut r = Raw::zeroed();
        rng.fill(&mut r.0);
        r.set_blocksize(rng.range_u32(14, 65_540))
            .set_samplerate(rng.range_u32(0, 655_360))
            .set_channels(rng.range_u32(0, 9))
            .set_bitdepth(rng.range_u32(0, 33));
        // First call: C and Rust must agree.
        let (_, out) = diff_validate(r);
        // Second call fed the (possibly mutated) output: they must still agree.
        diff_validate(out);
    }
}
