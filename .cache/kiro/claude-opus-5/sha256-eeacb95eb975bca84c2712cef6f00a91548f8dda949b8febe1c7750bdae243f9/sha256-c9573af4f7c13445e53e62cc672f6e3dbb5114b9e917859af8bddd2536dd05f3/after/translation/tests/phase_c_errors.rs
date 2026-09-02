//! Phase C — error-path differential tests, one test per `ERRORS.md` row.
//!
//! Each test asserts the SAME error sentinel (`-1`) *and* the same partial
//! mutation of `struct tflac` (all 28 bytes), because several C error branches
//! fire only after the function has already written `channel_mode` /
//! `max_rice_value`.

mod common;
use common::*;

/// A struct that is valid except for the axis under test.
fn base(rng: &mut Rng) -> Tflac {
    rng.valid_struct()
}

// --- Row 1: blocksize < 16 --------------------------------------------------

#[test]
fn e01_blocksize_too_small() {
    let mut rng = Rng::new(0xE001);
    for bs in 0..16u32 {
        for _ in 0..64 {
            let mut t = base(&mut rng);
            t.set_u32(OFF_BLOCKSIZE, bs);
            let (rc, out) = diff_validate("E01", &t);
            assert_eq!(rc, -1, "blocksize={bs} must be rejected");
            assert_eq!(out.0, t.0, "rejected before any mutation");
        }
    }
}

// --- Row 2: blocksize > 65535 ----------------------------------------------

#[test]
fn e02_blocksize_too_large() {
    let mut rng = Rng::new(0xE002);
    let mut probes = vec![65536u32, 65537, 0x1_0000, 0x7FFF_FFFF, 0x8000_0000, u32::MAX];
    for _ in 0..512 {
        probes.push(rng.range_u32(65536, u32::MAX));
    }
    for bs in probes {
        let mut t = base(&mut rng);
        t.set_u32(OFF_BLOCKSIZE, bs);
        let (rc, out) = diff_validate("E02", &t);
        assert_eq!(rc, -1, "blocksize={bs} must be rejected");
        assert_eq!(out.0, t.0, "rejected before any mutation");
    }
    // 65535 is the last accepted value — proves the boundary is exact
    let mut t = base(&mut rng);
    t.set_u32(OFF_BLOCKSIZE, 65535);
    assert_eq!(diff_validate("E02-boundary", &t).0, 0);
}

// --- Row 3: samplerate == 0 ------------------------------------------------

#[test]
fn e03_samplerate_zero() {
    let mut rng = Rng::new(0xE003);
    for _ in 0..512 {
        let mut t = base(&mut rng);
        t.set_u32(OFF_SAMPLERATE, 0);
        let (rc, out) = diff_validate("E03", &t);
        assert_eq!(rc, -1);
        assert_eq!(out.0, t.0);
    }
}

// --- Row 4: samplerate > 655350 -------------------------------------------

#[test]
fn e04_samplerate_too_large() {
    let mut rng = Rng::new(0xE004);
    let mut probes = vec![655351u32, 655352, 1_000_000, 0x8000_0000, u32::MAX];
    for _ in 0..512 {
        probes.push(rng.range_u32(655351, u32::MAX));
    }
    for sr in probes {
        let mut t = base(&mut rng);
        t.set_u32(OFF_SAMPLERATE, sr);
        let (rc, out) = diff_validate("E04", &t);
        assert_eq!(rc, -1, "samplerate={sr} must be rejected");
        assert_eq!(out.0, t.0);
    }
    let mut t = base(&mut rng);
    t.set_u32(OFF_SAMPLERATE, 655350);
    assert_eq!(diff_validate("E04-boundary", &t).0, 0);
}

// --- Row 5: channels == 0 -------------------------------------------------

#[test]
fn e05_channels_zero() {
    let mut rng = Rng::new(0xE005);
    for _ in 0..512 {
        let mut t = base(&mut rng);
        t.set_u32(OFF_CHANNELS, 0);
        let (rc, out) = diff_validate("E05", &t);
        assert_eq!(rc, -1);
        assert_eq!(out.0, t.0);
    }
}

// --- Row 6: channels > 8 -------------------------------------------------

#[test]
fn e06_channels_too_many() {
    let mut rng = Rng::new(0xE006);
    let mut probes = vec![9u32, 10, 16, 255, 256, 0x8000_0000, u32::MAX];
    for _ in 0..512 {
        probes.push(rng.range_u32(9, u32::MAX));
    }
    for ch in probes {
        let mut t = base(&mut rng);
        t.set_u32(OFF_CHANNELS, ch);
        let (rc, out) = diff_validate("E06", &t);
        assert_eq!(rc, -1, "channels={ch} must be rejected");
        assert_eq!(out.0, t.0);
    }
    let mut t = base(&mut rng);
    t.set_u32(OFF_CHANNELS, 8);
    assert_eq!(diff_validate("E06-boundary", &t).0, 0);
}

// --- Row 7: bitdepth == 0 -------------------------------------------------

#[test]
fn e07_bitdepth_zero() {
    let mut rng = Rng::new(0xE007);
    for _ in 0..512 {
        let mut t = base(&mut rng);
        t.set_u32(OFF_BITDEPTH, 0);
        let (rc, out) = diff_validate("E07", &t);
        assert_eq!(rc, -1);
        assert_eq!(out.0, t.0);
    }
}

// --- Row 8: bitdepth > 32 -------------------------------------------------

#[test]
fn e08_bitdepth_too_large() {
    let mut rng = Rng::new(0xE008);
    let mut probes = vec![33u32, 34, 64, 255, 256, 0x8000_0000, u32::MAX];
    for _ in 0..512 {
        probes.push(rng.range_u32(33, u32::MAX));
    }
    for bd in probes {
        let mut t = base(&mut rng);
        t.set_u32(OFF_BITDEPTH, bd);
        let (rc, out) = diff_validate("E08", &t);
        assert_eq!(rc, -1, "bitdepth={bd} must be rejected");
        assert_eq!(out.0, t.0);
    }
    let mut t = base(&mut rng);
    t.set_u32(OFF_BITDEPTH, 32);
    assert_eq!(diff_validate("E08-boundary", &t).0, 0);
}

// --- Row 9: max_rice_value > 30 -------------------------------------------

#[test]
fn e09_max_rice_value_too_large() {
    let mut rng = Rng::new(0xE009);
    for mrv in 31..=255u8 {
        for _ in 0..8 {
            let mut t = base(&mut rng);
            t.set_u8(OFF_MAX_RICE, mrv);
            let (rc, out) = diff_validate("E09", &t);
            assert_eq!(rc, -1, "max_rice_value={mrv} must be rejected");
            // max_rice_value itself is NOT overwritten on this path
            assert_eq!(out.max_rice_value(), mrv);
            // ...but channel_mode may already have been forced to 0
            assert_eq!(out.u32_at(OFF_CUR_BLOCKSIZE), t.u32_at(OFF_CUR_BLOCKSIZE));
            assert_eq!(out.partition_order(), t.partition_order());
        }
    }
    // 30 is the last accepted value
    let mut t = base(&mut rng);
    t.set_u8(OFF_MAX_RICE, 30);
    assert_eq!(diff_validate("E09-boundary", &t).0, 0);
}

// --- Row 10: max_partition_order > 15 -------------------------------------

#[test]
fn e10_max_partition_order_too_large() {
    let mut rng = Rng::new(0xE00A);
    for max_po in 16..=255u8 {
        for _ in 0..8 {
            let mut t = base(&mut rng);
            t.set_u8(OFF_MAX_PO, max_po).set_u8(OFF_MAX_RICE, 0);
            let (rc, out) = diff_validate("E10", &t);
            assert_eq!(rc, -1, "max_partition_order={max_po} must be rejected");
            // rice auto-fill already happened before this rejection
            let bd = t.u32_at(OFF_BITDEPTH);
            assert_eq!(out.max_rice_value(), if bd <= 16 { 14 } else { 30 });
            assert_eq!(out.max_rice_value() != 0, true);
            assert_eq!(out.u8_at(OFF_MAX_PO), max_po);
        }
    }
    let mut t = base(&mut rng);
    t.set_u8(OFF_MAX_PO, 15).set_u8(OFF_MIN_PO, 0);
    assert_eq!(diff_validate("E10-boundary", &t).0, 0);
}

// --- Row 11: min_partition_order > max_partition_order --------------------

#[test]
fn e11_min_greater_than_max_partition_order() {
    let mut rng = Rng::new(0xE00B);
    for max_po in 0..=15u8 {
        for min_po in (max_po + 1)..=255u8 {
            let mut t = base(&mut rng);
            t.set_u8(OFF_MIN_PO, min_po).set_u8(OFF_MAX_PO, max_po);
            let pre_po = t.partition_order();
            let pre_cbs = t.cur_blocksize();
            let (rc, out) = diff_validate("E11", &t);
            assert_eq!(rc, -1, "min={min_po} > max={max_po} must be rejected");
            assert_eq!(out.partition_order(), pre_po, "partition_order not written");
            assert_eq!(out.cur_blocksize(), pre_cbs, "cur_blocksize not written");
        }
    }
    // min == max is accepted
    for po in 0..=15u8 {
        let mut t = base(&mut rng);
        t.set_u8(OFF_MIN_PO, po).set_u8(OFF_MAX_PO, po);
        assert_eq!(diff_validate("E11-boundary", &t).0, 0);
    }
}

// --- Row 14: earliest check wins, nothing mutated -------------------------

#[test]
fn e14_all_invalid_earliest_check_wins() {
    let mut t = Tflac::poisoned();
    t.set_u32(OFF_BLOCKSIZE, 0)
        .set_u32(OFF_SAMPLERATE, 0)
        .set_u32(OFF_CHANNELS, 0)
        .set_u32(OFF_BITDEPTH, 0)
        .set_u8(OFF_CHANNEL_MODE, 255)
        .set_u8(OFF_MAX_RICE, 255)
        .set_u8(OFF_MIN_PO, 255)
        .set_u8(OFF_MAX_PO, 255);
    let (rc, out) = diff_validate("E14", &t);
    assert_eq!(rc, -1);
    assert_eq!(out.0, t.0, "blocksize check fires first; nothing is mutated");
    assert_eq!(out.channel_mode(), 255);
    assert_eq!(out.max_rice_value(), 255);
}

// --- Row 15: rice check precedes partition-order check --------------------

#[test]
fn e15_rice_check_precedes_partition_order() {
    let mut rng = Rng::new(0xE00F);
    for _ in 0..256 {
        let mut t = base(&mut rng);
        t.set_u8(OFF_MAX_RICE, 255).set_u8(OFF_MAX_PO, 255).set_u8(OFF_MIN_PO, 255);
        let (rc, out) = diff_validate("E15", &t);
        assert_eq!(rc, -1);
        assert_eq!(out.max_rice_value(), 255, "not auto-filled: it was nonzero");
        assert_eq!(out.u8_at(OFF_MAX_PO), 255, "max_po untouched, rice rejected first");
    }
}

// --- Row 16: auto-fill happens before the partition-order rejection -------

#[test]
fn e16_autofill_before_partition_order_rejection() {
    let mut rng = Rng::new(0xE010);
    for bd in [1u32, 16, 17, 32] {
        for _ in 0..64 {
            let mut t = base(&mut rng);
            t.set_u32(OFF_BITDEPTH, bd).set_u8(OFF_MAX_RICE, 0).set_u8(OFF_MAX_PO, 255);
            let (rc, out) = diff_validate("E16", &t);
            assert_eq!(rc, -1);
            assert_eq!(
                out.max_rice_value(),
                if bd <= 16 { 14 } else { 30 },
                "rice auto-fill is observable on the max_po error path"
            );
        }
    }
}

// --- Row 17: out-of-range enum across the FFI boundary -------------------

#[test]
fn e17_out_of_range_channel_mode_enum() {
    let mut rng = Rng::new(0xE011);
    // Every possible byte value for the enum field, in both predicate branches
    // and on an error path, exhaustively.
    for mode in 0..=255u8 {
        // kept branch (channels==2, bitdepth!=32)
        let mut t = base(&mut rng);
        t.set_u32(OFF_CHANNELS, 2)
            .set_u32(OFF_BITDEPTH, rng.range_u32(1, 31))
            .set_u8(OFF_CHANNEL_MODE, mode);
        let (rc, out) = diff_validate("E17-kept", &t);
        assert_eq!(rc, 0, "out-of-range enum is not an error in this C");
        assert_eq!(out.channel_mode(), mode);

        // reset branch
        let mut t = base(&mut rng);
        t.set_u32(OFF_CHANNELS, rng.range_u32(3, 8)).set_u8(OFF_CHANNEL_MODE, mode);
        let (rc, out) = diff_validate("E17-reset", &t);
        assert_eq!(rc, 0);
        assert_eq!(out.channel_mode(), 0);

        // out-of-range enum combined with an early rejection: mode survives
        let mut t = base(&mut rng);
        t.set_u32(OFF_BLOCKSIZE, 0).set_u8(OFF_CHANNEL_MODE, mode);
        let (rc, out) = diff_validate("E17-early-reject", &t);
        assert_eq!(rc, -1);
        assert_eq!(out.channel_mode(), mode);

        // out-of-range enum combined with a late rejection: mode may be reset
        let mut t = base(&mut rng);
        t.set_u32(OFF_CHANNELS, rng.range_u32(3, 8))
            .set_u8(OFF_CHANNEL_MODE, mode)
            .set_u8(OFF_MAX_PO, 200);
        let (rc, _) = diff_validate("E17-late-reject", &t);
        assert_eq!(rc, -1);
    }
}

// --- Row 13 companion: tflac_size_memory has no rejection path -----------

#[test]
fn e13_size_memory_never_rejects() {
    // Not an error row, but asserted here too: no input value is special-cased.
    let mut rng = Rng::new(0xE013);
    for b in [0u32, 1, 15, 16, u32::MAX, u32::MAX - 1, 0x8000_0000] {
        diff_size_memory("E13", b);
    }
    for _ in 0..10_000 {
        diff_size_memory("E13", rng.next_u32());
    }
}

// --- Generic FFI boundary sweeps (beyond the table) ---------------------

#[test]
fn generic_one_past_every_documented_range() {
    let mut rng = Rng::new(0xE0FF);
    // (offset, is_u32, last_valid, first_invalid_low, first_invalid_high)
    let cases: &[(usize, bool, u32, Option<u32>, Option<u32>)] = &[
        (OFF_BLOCKSIZE, true, 65535, Some(15), Some(65536)),
        (OFF_SAMPLERATE, true, 655350, Some(0), Some(655351)),
        (OFF_CHANNELS, true, 8, Some(0), Some(9)),
        (OFF_BITDEPTH, true, 32, Some(0), Some(33)),
        (OFF_MAX_RICE, false, 30, None, Some(31)),
        (OFF_MAX_PO, false, 15, None, Some(16)),
    ];
    for &(off, is_u32, last_valid, lo_bad, hi_bad) in cases {
        for _ in 0..64 {
            let mut ok = base(&mut rng);
            ok.set_u8(OFF_MIN_PO, 0);
            if is_u32 { ok.set_u32(off, last_valid); } else { ok.set_u8(off, last_valid as u8); }
            assert_eq!(
                diff_validate("GEN-last-valid", &ok).0, 0,
                "offset {off} value {last_valid} should be accepted"
            );
            for bad in [lo_bad, hi_bad].into_iter().flatten() {
                let mut t = base(&mut rng);
                t.set_u8(OFF_MIN_PO, 0);
                if is_u32 { t.set_u32(off, bad); } else { t.set_u8(off, bad as u8); }
                assert_eq!(
                    diff_validate("GEN-one-past", &t).0, -1,
                    "offset {off} value {bad} should be rejected"
                );
            }
        }
    }
}

#[test]
fn generic_all_zero_and_all_ones_structs() {
    let zero = Tflac([0x00; 28]);
    let (rc, out) = diff_validate("GEN-zero", &zero);
    assert_eq!(rc, -1);
    assert_eq!(out.0, zero.0);

    let ones = Tflac([0xFF; 28]);
    let (rc, out) = diff_validate("GEN-ones", &ones);
    assert_eq!(rc, -1);
    assert_eq!(out.0, ones.0);
}
