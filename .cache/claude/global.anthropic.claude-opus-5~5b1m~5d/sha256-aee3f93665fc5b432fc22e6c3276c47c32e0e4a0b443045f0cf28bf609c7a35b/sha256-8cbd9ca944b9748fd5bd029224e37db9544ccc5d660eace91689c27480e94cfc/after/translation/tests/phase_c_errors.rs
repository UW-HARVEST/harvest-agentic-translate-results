//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Each test asserts the SAME sentinel (`-1`) is returned by both libraries AND
//! that the 28-byte struct image is identical afterwards (so partial in-place
//! mutations performed before the rejection are compared too).

mod common;

use common::{diff_size_memory, diff_validate, Raw, Rng};

const ERR: std::ffi::c_int = -1;

// ---------------------------------------------------------------------------
// Row 1 — blocksize < 16
// ---------------------------------------------------------------------------
#[test]
fn err_01_blocksize_too_small() {
    for blocksize in 0u32..16 {
        let mut r = Raw::valid();
        r.set_blocksize(blocksize);
        assert_eq!(diff_validate(r).0, ERR, "blocksize {blocksize}");
    }
    // and the first accepted value
    let mut ok = Raw::valid();
    ok.set_blocksize(16);
    assert_eq!(diff_validate(ok).0, 0);
}

// ---------------------------------------------------------------------------
// Row 2 — blocksize > 65535
// ---------------------------------------------------------------------------
#[test]
fn err_02_blocksize_too_large() {
    for blocksize in [65536u32, 65537, 100_000, 1 << 20, u32::MAX - 1, u32::MAX] {
        let mut r = Raw::valid();
        r.set_blocksize(blocksize);
        assert_eq!(diff_validate(r).0, ERR, "blocksize {blocksize}");
    }
    let mut ok = Raw::valid();
    ok.set_blocksize(65535);
    assert_eq!(diff_validate(ok).0, 0);
}

// ---------------------------------------------------------------------------
// Row 3 — samplerate == 0
// ---------------------------------------------------------------------------
#[test]
fn err_03_samplerate_zero() {
    let mut r = Raw::valid();
    r.set_samplerate(0);
    assert_eq!(diff_validate(r).0, ERR);
    let mut ok = Raw::valid();
    ok.set_samplerate(1);
    assert_eq!(diff_validate(ok).0, 0);
}

// ---------------------------------------------------------------------------
// Row 4 — samplerate > 655350
// ---------------------------------------------------------------------------
#[test]
fn err_04_samplerate_too_large() {
    for sr in [655_351u32, 655_352, 1_000_000, u32::MAX] {
        let mut r = Raw::valid();
        r.set_samplerate(sr);
        assert_eq!(diff_validate(r).0, ERR, "samplerate {sr}");
    }
    let mut ok = Raw::valid();
    ok.set_samplerate(655_350);
    assert_eq!(diff_validate(ok).0, 0);
}

// ---------------------------------------------------------------------------
// Row 5 — channels == 0
// ---------------------------------------------------------------------------
#[test]
fn err_05_channels_zero() {
    let mut r = Raw::valid();
    r.set_channels(0);
    assert_eq!(diff_validate(r).0, ERR);
}

// ---------------------------------------------------------------------------
// Row 6 — channels > 8
// ---------------------------------------------------------------------------
#[test]
fn err_06_channels_too_large() {
    for ch in [9u32, 10, 255, 256, 1 << 16, u32::MAX] {
        let mut r = Raw::valid();
        r.set_channels(ch);
        assert_eq!(diff_validate(r).0, ERR, "channels {ch}");
    }
    let mut ok = Raw::valid();
    ok.set_channels(8);
    assert_eq!(diff_validate(ok).0, 0);
}

// ---------------------------------------------------------------------------
// Row 7 — bitdepth == 0
// ---------------------------------------------------------------------------
#[test]
fn err_07_bitdepth_zero() {
    let mut r = Raw::valid();
    r.set_bitdepth(0);
    assert_eq!(diff_validate(r).0, ERR);
}

// ---------------------------------------------------------------------------
// Row 8 — bitdepth > 32
// ---------------------------------------------------------------------------
#[test]
fn err_08_bitdepth_too_large() {
    for bd in [33u32, 34, 64, 255, 256, u32::MAX] {
        let mut r = Raw::valid();
        r.set_bitdepth(bd);
        assert_eq!(diff_validate(r).0, ERR, "bitdepth {bd}");
    }
    let mut ok = Raw::valid();
    ok.set_bitdepth(32);
    assert_eq!(diff_validate(ok).0, 0);
}

// ---------------------------------------------------------------------------
// Row 9 — max_rice_value > 30 (only reached when != 0)
// ---------------------------------------------------------------------------
#[test]
fn err_09_max_rice_value_too_large() {
    for mrv in 31u8..=255 {
        let mut r = Raw::valid();
        r.set_max_rice_value(mrv);
        let (rc, out) = diff_validate(r);
        assert_eq!(rc, ERR, "max_rice_value {mrv}");
        assert_eq!(out.max_rice_value(), mrv, "field is not modified before -1");
    }
    let mut ok = Raw::valid();
    ok.set_max_rice_value(30);
    assert_eq!(diff_validate(ok).0, 0);
}

// ---------------------------------------------------------------------------
// Row 10 — max_partition_order > 15
// ---------------------------------------------------------------------------
#[test]
fn err_10_max_partition_order_too_large() {
    for mpo in 16u8..=255 {
        let mut r = Raw::valid();
        r.set_max_partition_order(mpo);
        let (rc, out) = diff_validate(r);
        assert_eq!(rc, ERR, "max_partition_order {mpo}");
        // The rice default HAS already been written in-place before the return.
        assert_eq!(out.max_rice_value(), 14);
    }
    let mut ok = Raw::valid();
    ok.set_max_partition_order(15);
    assert_eq!(diff_validate(ok).0, 0);
}

// ---------------------------------------------------------------------------
// Row 11 — min_partition_order > max_partition_order
// ---------------------------------------------------------------------------
#[test]
fn err_11_min_gt_max_partition_order() {
    for max in 0u8..=15 {
        for min in (max as u16 + 1)..=255 {
            let mut r = Raw::valid();
            r.set_min_partition_order(min as u8)
                .set_max_partition_order(max);
            let (rc, out) = diff_validate(r);
            assert_eq!(rc, ERR, "min={min} max={max}");
            // partition_order must NOT have been written yet.
            assert_eq!(out.partition_order(), 0);
            assert_eq!(out.cur_blocksize(), 0);
        }
    }
    for max in 0u8..=15 {
        for min in 0..=max {
            let mut r = Raw::valid();
            r.set_min_partition_order(min).set_max_partition_order(max);
            assert_eq!(diff_validate(r).0, 0, "min={min} max={max} must pass");
        }
    }
}

// ---------------------------------------------------------------------------
// O1 — every check violated at once
// ---------------------------------------------------------------------------
#[test]
fn err_ordering_all_invalid() {
    let mut r = Raw::zeroed();
    r.set_blocksize(0)
        .set_samplerate(u32::MAX)
        .set_channels(u32::MAX)
        .set_bitdepth(u32::MAX)
        .set_channel_mode(0xFF)
        .set_max_rice_value(0xFF)
        .set_min_partition_order(0xFF)
        .set_max_partition_order(0x00)
        .set_partition_order(0xAA)
        .set_cur_blocksize(0xDEAD_BEEF);
    let (rc, out) = diff_validate(r);
    assert_eq!(rc, ERR);
    // The very first check (blocksize < 16) fires, so nothing is mutated.
    assert_eq!(out.0, r.0, "no mutation before the first failing check");
}

// ---------------------------------------------------------------------------
// O2 — partial mutation must be visible and identical
// ---------------------------------------------------------------------------
#[test]
fn err_partial_mutation_visible() {
    // channel_mode fix-up applies, THEN max_rice_value > 30 rejects.
    let mut r = Raw::valid();
    r.set_channels(1) // != 2  -> mode reset
        .set_channel_mode(3)
        .set_max_rice_value(31);
    let (rc, out) = diff_validate(r);
    assert_eq!(rc, ERR);
    assert_eq!(out.channel_mode(), 0, "fix-up happened before the rejection");
    assert_eq!(out.max_rice_value(), 31);

    // channel_mode fix-up + rice default applied, THEN max_partition_order > 15.
    let mut r2 = Raw::valid();
    r2.set_channels(8)
        .set_channel_mode(2)
        .set_bitdepth(24)
        .set_max_rice_value(0)
        .set_max_partition_order(200);
    let (rc2, out2) = diff_validate(r2);
    assert_eq!(rc2, ERR);
    assert_eq!(out2.channel_mode(), 0);
    assert_eq!(out2.max_rice_value(), 30, "rice default written before -1");
}

// ---------------------------------------------------------------------------
// G1 — out-of-range enum values across the FFI boundary
// ---------------------------------------------------------------------------
#[test]
fn err_g1_channel_mode_out_of_range() {
    // 4 == TFLAC_CHANNEL_MODE_COUNT (not a real mode) and everything above it.
    for mode in 4u8..=255 {
        for channels in [1u32, 2, 8] {
            for bitdepth in [16u32, 32] {
                let mut r = Raw::valid();
                r.set_channel_mode(mode)
                    .set_channels(channels)
                    .set_bitdepth(bitdepth);
                let (rc, out) = diff_validate(r);
                assert_eq!(rc, 0, "C accepts any non-zero mode");
                let expect = if channels != 2 || bitdepth == 32 { 0 } else { mode };
                assert_eq!(out.channel_mode(), expect, "mode={mode}");
            }
        }
    }
    // Also drive the enum-typed value as a full 32-bit int pattern written into
    // the byte at offset 16 plus garbage in the neighbouring bytes, proving no
    // wider read happens.
    let mut rng = Rng::new(0x0BAD_F00D_0BAD_F00D);
    for _ in 0..20_000 {
        let mut r = Raw::valid();
        r.set_channel_mode(rng.next_u8());
        r.0[17] = 0; // keep max_rice_value valid
        r.0[18] = 0;
        r.0[19] = rng.next_u8();
        r.0[20] = rng.next_u8();
        rng.fill(&mut r.0[21..24]);
        diff_validate(r);
    }
}

// ---------------------------------------------------------------------------
// G2 — exhaustive boundary combinations
// ---------------------------------------------------------------------------
#[test]
fn err_g2_boundaries_exhaustive() {
    let blocksizes = [0u32, 1, 15, 16, 17, 65534, 65535, 65536, 65537, u32::MAX];
    let samplerates = [0u32, 1, 2, 655_349, 655_350, 655_351, u32::MAX];
    let channels = [0u32, 1, 2, 3, 7, 8, 9, u32::MAX];
    let bitdepths = [0u32, 1, 15, 16, 17, 31, 32, 33, u32::MAX];
    let rices = [0u8, 1, 29, 30, 31, 255];
    let orders = [(0u8, 0u8), (0, 15), (15, 15), (15, 16), (16, 16), (1, 0), (255, 255)];

    for &bs in &blocksizes {
        for &sr in &samplerates {
            for &ch in &channels {
                for &bd in &bitdepths {
                    for &mrv in &rices {
                        for &(min, max) in &orders {
                            let mut r = Raw::zeroed();
                            r.set_blocksize(bs)
                                .set_samplerate(sr)
                                .set_channels(ch)
                                .set_bitdepth(bd)
                                .set_max_rice_value(mrv)
                                .set_min_partition_order(min)
                                .set_max_partition_order(max)
                                .set_channel_mode(if (bs ^ sr) & 1 == 0 { 0 } else { 3 });
                            diff_validate(r);
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// G3 — padding bytes preserved on the error paths too
// ---------------------------------------------------------------------------
#[test]
fn err_g3_padding_preserved() {
    let mut rng = Rng::new(0x7777_8888_9999_AAAA);
    for _ in 0..20_000 {
        let mut r = Raw::zeroed();
        rng.fill(&mut r.0);
        let pad = [r.0[21], r.0[22], r.0[23]];
        let (_, out) = diff_validate(r);
        assert_eq!([out.0[21], out.0[22], out.0[23]], pad);
    }
}

// ---------------------------------------------------------------------------
// G4 — tflac_size_memory has no error path; total over all u32
// ---------------------------------------------------------------------------
#[test]
fn err_g4_size_memory_total() {
    for b in [0u32, 1, u32::MAX, u32::MAX - 1, 1 << 30, 1 << 31] {
        diff_size_memory(b);
    }
    let mut rng = Rng::new(0x1111_2222_3333_4444);
    for _ in 0..100_000 {
        diff_size_memory(rng.next_u32());
    }
}

// ---------------------------------------------------------------------------
// G5 — null pointer: documented UB in C (unconditional deref), not executed.
// ---------------------------------------------------------------------------
#[test]
fn err_g5_null_pointer_documented() {
    // `flac_validate` in c_src/src/lib.c dereferences `t` on its very first
    // statement with no null check, so passing NULL is undefined behaviour and
    // segfaults in both implementations. Calling it would abort the test
    // process, so the row is satisfied by documentation, matching the C.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/src/lib.c"),
    )
    .unwrap();
    assert!(
        !src.contains("t == NULL") && !src.contains("!t"),
        "C gained a null check; the null-pointer row must become a real test"
    );
}
