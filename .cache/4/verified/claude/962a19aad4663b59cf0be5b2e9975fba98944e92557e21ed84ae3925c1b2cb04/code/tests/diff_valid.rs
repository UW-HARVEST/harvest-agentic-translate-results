//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` through `libloading`
//! and compares the `int` return value **and** all 28 bytes of `struct tflac`
//! (padding included) after the call.

mod common;

use common::*;

// ---------------------------------------------------------------------------
// Generators
// ---------------------------------------------------------------------------

/// Blocksizes biased towards shapes the `partition_order` loop distinguishes.
fn rand_blocksize(rng: &mut Rng) -> u32 {
    match rng.next_u64() % 4 {
        0 => rng.range_u32(16, 65535),
        1 => 1u32 << rng.range_u32(4, 15), // exact power of two
        2 => {
            // 2^v * odd, kept inside 16..=65535
            let v = rng.range_u32(0, 12);
            let base = 1u32 << v;
            let max_odd_idx = ((65535 / base).max(1)) / 2;
            let odd = 2 * rng.range_u32(0, max_odd_idx) + 1;
            let val = base.saturating_mul(odd);
            if (16..=65535).contains(&val) { val } else { 4096 }
        }
        _ => rng.pick(&[
            16, 17, 18, 32, 576, 1152, 4096, 4608, 32768, 49152, 65520, 65534, 65535,
        ]),
    }
}

/// A struct that satisfies every validation check, with poisoned output fields.
fn rand_valid(rng: &mut Rng) -> Fields {
    let max_po = rng.range_u8(0, 15);
    let min_po = rng.range_u8(0, max_po);
    Fields {
        blocksize: rand_blocksize(rng),
        samplerate: rng.range_u32(1, 655_350),
        channels: rng.range_u32(1, 8),
        bitdepth: rng.range_u32(1, 32),
        channel_mode: rng.next_u8(),
        max_rice_value: rng.range_u8(0, 30),
        min_partition_order: min_po,
        max_partition_order: max_po,
        partition_order: rng.next_u8(),
        pad: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
        cur_blocksize: rng.next_u32(),
    }
}

/// A random bitdepth from the bucket the C code distinguishes.
fn bitdepth_le16(rng: &mut Rng) -> u32 {
    rng.range_u32(1, 16)
}
fn bitdepth_17_31(rng: &mut Rng) -> u32 {
    rng.range_u32(17, 31)
}

/// A random `channels` value that is not 2.
fn channels_not2(rng: &mut Rng) -> u32 {
    let v = rng.range_u32(1, 7);
    if v >= 2 { v + 1 } else { v }
}

const ITERS: usize = 20_000;

// ---------------------------------------------------------------------------
// Rows 1..11 — channel_mode x channels x bitdepth
// ---------------------------------------------------------------------------

#[test]
fn cfg_row01_indep_2ch_bitdepth_le16() {
    let mut rng = Rng::new(0x0101);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.channel_mode = 0;
        f.channels = 2;
        f.bitdepth = bitdepth_le16(&mut rng);
        check_validate_ret("cfg#1", f, 0);
    }
}

#[test]
fn cfg_row02_indep_2ch_bitdepth_17_31() {
    let mut rng = Rng::new(0x0102);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.channel_mode = 0;
        f.channels = 2;
        f.bitdepth = bitdepth_17_31(&mut rng);
        check_validate_ret("cfg#2", f, 0);
    }
}

#[test]
fn cfg_row03_indep_2ch_bitdepth_32() {
    let mut rng = Rng::new(0x0103);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.channel_mode = 0;
        f.channels = 2;
        f.bitdepth = 32;
        check_validate_ret("cfg#3", f, 0);
    }
}

#[test]
fn cfg_row04_indep_not2ch_all_bitdepth_buckets() {
    let mut rng = Rng::new(0x0104);
    for i in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.channel_mode = 0;
        f.channels = channels_not2(&mut rng);
        f.bitdepth = match i % 3 {
            0 => bitdepth_le16(&mut rng),
            1 => bitdepth_17_31(&mut rng),
            _ => 32,
        };
        check_validate_ret("cfg#4", f, 0);
    }
}

#[test]
fn cfg_row05_mode1to3_2ch_bitdepth_le16_mode_preserved() {
    let mut rng = Rng::new(0x0105);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.channel_mode = rng.range_u8(1, 3);
        f.channels = 2;
        f.bitdepth = bitdepth_le16(&mut rng);
        check_validate_ret("cfg#5", f, 0);
        // C keeps the mode in this configuration.
        let (_, out) = pair().c.validate(f);
        assert_eq!(out.channel_mode, f.channel_mode, "cfg#5 premise: {f:?}");
        assert_eq!(
            out.max_rice_value,
            if f.max_rice_value == 0 { 14 } else { f.max_rice_value },
            "cfg#5 rice premise: {f:?}"
        );
    }
}

#[test]
fn cfg_row06_mode1to3_2ch_bitdepth_17_31_mode_preserved() {
    let mut rng = Rng::new(0x0106);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.channel_mode = rng.range_u8(1, 3);
        f.channels = 2;
        f.bitdepth = bitdepth_17_31(&mut rng);
        check_validate_ret("cfg#6", f, 0);
        let (_, out) = pair().c.validate(f);
        assert_eq!(out.channel_mode, f.channel_mode, "cfg#6 premise: {f:?}");
        assert_eq!(
            out.max_rice_value,
            if f.max_rice_value == 0 { 30 } else { f.max_rice_value },
            "cfg#6 rice premise: {f:?}"
        );
    }
}

#[test]
fn cfg_row07_mode1to3_2ch_bitdepth32_mode_forced_indep() {
    let mut rng = Rng::new(0x0107);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.channel_mode = rng.range_u8(1, 3);
        f.channels = 2;
        f.bitdepth = 32;
        check_validate_ret("cfg#7", f, 0);
        let (_, out) = pair().c.validate(f);
        assert_eq!(out.channel_mode, 0, "cfg#7 premise: {f:?}");
    }
}

#[test]
fn cfg_row08_mode1to3_not2ch_bitdepth_not32_mode_forced_indep() {
    let mut rng = Rng::new(0x0108);
    for i in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.channel_mode = rng.range_u8(1, 3);
        f.channels = channels_not2(&mut rng);
        f.bitdepth = if i % 2 == 0 {
            bitdepth_le16(&mut rng)
        } else {
            bitdepth_17_31(&mut rng)
        };
        check_validate_ret("cfg#8", f, 0);
        let (_, out) = pair().c.validate(f);
        assert_eq!(out.channel_mode, 0, "cfg#8 premise: {f:?}");
    }
}

#[test]
fn cfg_row09_mode1to3_not2ch_bitdepth32_both_arms() {
    let mut rng = Rng::new(0x0109);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.channel_mode = rng.range_u8(1, 3);
        f.channels = channels_not2(&mut rng);
        f.bitdepth = 32;
        check_validate_ret("cfg#9", f, 0);
        let (_, out) = pair().c.validate(f);
        assert_eq!(out.channel_mode, 0, "cfg#9 premise: {f:?}");
    }
}

#[test]
fn cfg_row10_mode_out_of_enum_range_kept() {
    let mut rng = Rng::new(0x010A);
    // Every out-of-enum-range value, exhaustively, plus randomized rest.
    for mode in 4u8..=255 {
        let mut f = rand_valid(&mut rng);
        f.channel_mode = mode;
        f.channels = 2;
        f.bitdepth = rng.range_u32(1, 31); // != 32
        check_validate_ret("cfg#10", f, 0);
        let (_, out) = pair().c.validate(f);
        assert_eq!(out.channel_mode, mode, "cfg#10 premise: {f:?}");
    }
}

#[test]
fn cfg_row11_mode_out_of_enum_range_forced_indep() {
    let mut rng = Rng::new(0x010B);
    for mode in 4u8..=255 {
        // channels != 2
        let mut f = rand_valid(&mut rng);
        f.channel_mode = mode;
        f.channels = channels_not2(&mut rng);
        f.bitdepth = rng.range_u32(1, 32);
        check_validate_ret("cfg#11a", f, 0);
        assert_eq!(pair().c.validate(f).1.channel_mode, 0, "cfg#11a: {f:?}");

        // bitdepth == 32
        let mut g = rand_valid(&mut rng);
        g.channel_mode = mode;
        g.channels = 2;
        g.bitdepth = 32;
        check_validate_ret("cfg#11b", g, 0);
        assert_eq!(pair().c.validate(g).1.channel_mode, 0, "cfg#11b: {g:?}");
    }
}

// ---------------------------------------------------------------------------
// Rows 12..17 — max_rice_value axis
// ---------------------------------------------------------------------------

#[test]
fn cfg_row12to15_rice_autofill_bitdepth_boundaries() {
    let mut rng = Rng::new(0x0112);
    for (row, bd, want) in [
        ("cfg#12", 1u32, 14u8),
        ("cfg#13", 16, 14),
        ("cfg#14", 17, 30),
        ("cfg#15", 32, 30),
    ] {
        for _ in 0..iters(ITERS) {
            let mut f = rand_valid(&mut rng);
            f.max_rice_value = 0;
            f.bitdepth = bd;
            check_validate_ret(row, f, 0);
            assert_eq!(pair().c.validate(f).1.max_rice_value, want, "{row}: {f:?}");
        }
    }
}

#[test]
fn cfg_row16_rice_1_to_30_kept_verbatim() {
    let mut rng = Rng::new(0x0116);
    for rice in 1u8..=30 {
        for _ in 0..64 {
            let mut f = rand_valid(&mut rng);
            f.max_rice_value = rice;
            f.bitdepth = rng.range_u32(1, 32);
            check_validate_ret("cfg#16", f, 0);
            assert_eq!(pair().c.validate(f).1.max_rice_value, rice, "cfg#16: {f:?}");
        }
    }
}

#[test]
fn cfg_row17_rice_exactly_30() {
    let mut rng = Rng::new(0x0117);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.max_rice_value = 30;
        check_validate_ret("cfg#17", f, 0);
    }
}

// ---------------------------------------------------------------------------
// Rows 18..29 — partition_order loop
// ---------------------------------------------------------------------------

#[test]
fn cfg_row18_min_eq_max_eq_zero() {
    let mut rng = Rng::new(0x0118);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.min_partition_order = 0;
        f.max_partition_order = 0;
        check_validate_ret("cfg#18", f, 0);
        assert_eq!(pair().c.validate(f).1.partition_order, 0, "cfg#18: {f:?}");
    }
}

#[test]
fn cfg_row19_min_eq_max_eq_k_for_all_k() {
    let mut rng = Rng::new(0x0119);
    for k in 0u8..=15 {
        for _ in 0..128 {
            let mut f = rand_valid(&mut rng);
            f.min_partition_order = k;
            f.max_partition_order = k;
            check_validate_ret("cfg#19", f, 0);
            assert_eq!(pair().c.validate(f).1.partition_order, k, "cfg#19: {f:?}");
        }
    }
}

#[test]
fn cfg_row20_odd_blocksize_loop_never_advances() {
    let mut rng = Rng::new(0x0120);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.blocksize = 2 * rng.range_u32(8, 32767) + 1; // odd, 17..=65535
        f.min_partition_order = 0;
        f.max_partition_order = 15;
        check_validate_ret("cfg#20", f, 0);
        assert_eq!(pair().c.validate(f).1.partition_order, 0, "cfg#20: {f:?}");
    }
}

#[test]
fn cfg_row21_blocksize_2powv_times_odd() {
    let mut rng = Rng::new(0x0121);
    for v in 1u32..=15 {
        let base = 1u32 << v;
        let mut odd = 1u32;
        while base.checked_mul(odd).is_some_and(|x| x <= 65535) {
            let bs = base * odd;
            if bs >= 16 {
                for _ in 0..8 {
                    let mut f = rand_valid(&mut rng);
                    f.blocksize = bs;
                    f.min_partition_order = 0;
                    f.max_partition_order = 15;
                    check_validate_ret("cfg#21", f, 0);
                    // v2(bs) == v here, so the loop must stop exactly at v.
                    assert_eq!(
                        pair().c.validate(f).1.partition_order,
                        v as u8,
                        "cfg#21 premise for blocksize={bs}: {f:?}"
                    );
                }
            }
            odd += 2;
        }
    }
}

#[test]
fn cfg_row22_blocksize_32768_extreme_shift() {
    let mut rng = Rng::new(0x0122);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.blocksize = 32768;
        f.min_partition_order = 0;
        f.max_partition_order = 15;
        check_validate_ret("cfg#22", f, 0);
        assert_eq!(pair().c.validate(f).1.partition_order, 15, "cfg#22: {f:?}");
    }
}

#[test]
fn cfg_row23_loop_clamped_by_max() {
    let mut rng = Rng::new(0x0123);
    for max_po in 0u8..=14 {
        for _ in 0..64 {
            let mut f = rand_valid(&mut rng);
            f.blocksize = 32768; // v2 == 15, so max always clamps
            f.min_partition_order = 0;
            f.max_partition_order = max_po;
            check_validate_ret("cfg#23", f, 0);
            assert_eq!(
                pair().c.validate(f).1.partition_order,
                max_po,
                "cfg#23: {f:?}"
            );
        }
    }
}

#[test]
fn cfg_row24_min_beyond_divisible_run() {
    let mut rng = Rng::new(0x0124);
    for min_po in 1u8..=15 {
        for max_po in min_po..=15 {
            let mut f = rand_valid(&mut rng);
            f.blocksize = 17; // odd => loop cannot advance from min
            f.min_partition_order = min_po;
            f.max_partition_order = max_po;
            check_validate_ret("cfg#24", f, 0);
            assert_eq!(
                pair().c.validate(f).1.partition_order,
                min_po,
                "cfg#24: {f:?}"
            );
        }
    }
}

#[test]
fn cfg_row25_min_inside_divisible_run() {
    let mut rng = Rng::new(0x0125);
    for min_po in 0u8..=12 {
        for _ in 0..32 {
            let mut f = rand_valid(&mut rng);
            f.blocksize = 4096; // v2 == 12
            f.min_partition_order = min_po;
            f.max_partition_order = 15;
            check_validate_ret("cfg#25", f, 0);
            assert_eq!(pair().c.validate(f).1.partition_order, 12, "cfg#25: {f:?}");
        }
    }
}

#[test]
fn cfg_row26_min_eq_max_eq_15_extreme_shift_first_test() {
    let mut rng = Rng::new(0x0126);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.min_partition_order = 15;
        f.max_partition_order = 15;
        check_validate_ret("cfg#26", f, 0);
        assert_eq!(pair().c.validate(f).1.partition_order, 15, "cfg#26: {f:?}");
    }
}

#[test]
fn cfg_row27_blocksize_lower_boundary_16() {
    let mut rng = Rng::new(0x0127);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.blocksize = 16;
        f.min_partition_order = 0;
        f.max_partition_order = 15;
        check_validate_ret("cfg#27", f, 0);
    }
}

#[test]
fn cfg_row28_blocksize_upper_boundary_65535() {
    let mut rng = Rng::new(0x0128);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.blocksize = 65535;
        f.min_partition_order = 0;
        f.max_partition_order = 15;
        check_validate_ret("cfg#28", f, 0);
    }
}

#[test]
fn cfg_row29_all_min_max_pairs_x_blocksize_shapes() {
    let mut rng = Rng::new(0x0129);
    let mut shapes: Vec<u32> = vec![16, 17, 18, 20, 24, 31, 32, 33, 65535, 65534, 65532, 65528];
    for k in 1u32..=16 {
        if let Some(v) = 65536u32.checked_sub(1u32 << k) {
            if (16..=65535).contains(&v) {
                shapes.push(v);
            }
        }
        if (16..=65535).contains(&(1u32 << k)) {
            shapes.push(1u32 << k);
        }
    }
    for bs in shapes {
        for max_po in 0u8..=15 {
            for min_po in 0..=max_po {
                let mut f = poisoned(&mut rng, Fields::valid_base());
                f.blocksize = bs;
                f.min_partition_order = min_po;
                f.max_partition_order = max_po;
                f.channel_mode = rng.next_u8();
                f.max_rice_value = rng.range_u8(0, 30);
                f.channels = rng.range_u32(1, 8);
                f.bitdepth = rng.range_u32(1, 32);
                check_validate_ret("cfg#29", f, 0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 30..34 — remaining axes
// ---------------------------------------------------------------------------

#[test]
fn cfg_row30_samplerate_valid_boundaries() {
    let mut rng = Rng::new(0x0130);
    for sr in [1u32, 2, 8000, 44100, 48000, 192_000, 655_349, 655_350] {
        for _ in 0..256 {
            let mut f = rand_valid(&mut rng);
            f.samplerate = sr;
            check_validate_ret("cfg#30", f, 0);
        }
    }
}

#[test]
fn cfg_row31_output_fields_and_padding_poisoned() {
    let mut rng = Rng::new(0x0131);
    for _ in 0..iters(ITERS) {
        let mut f = rand_valid(&mut rng);
        f.partition_order = 0xAA;
        f.cur_blocksize = 0xDEAD_BEEF;
        f.pad = [0x5A, 0xA5, 0x3C];
        check_validate_ret("cfg#31", f, 0);
        let (_, out) = pair().c.validate(f);
        assert_eq!(out.cur_blocksize, f.blocksize, "cfg#31 premise: {f:?}");
        assert_eq!(out.pad, f.pad, "cfg#31 padding must be untouched: {f:?}");
    }
}

#[test]
fn cfg_row32_full_random_valid_structs() {
    let mut rng = Rng::new(0x0132);
    for _ in 0..iters(2_000_000) {
        let f = rand_valid(&mut rng);
        check_validate_ret("cfg#32", f, 0);
    }
}

#[test]
fn cfg_row33_full_random_arbitrary_bytes() {
    let mut rng = Rng::new(0x0133);
    // Pure random 28-byte images (mostly rejected — exercises every check).
    for _ in 0..iters(1_000_000) {
        let mut b = [0u8; TFLAC_SIZE];
        rng.fill(&mut b);
        check_validate_raw("cfg#33a", Raw(b));
    }
    // Hybrid: a valid struct with one field replaced by a fully random value,
    // so the interesting (deep) code paths are reached far more often.
    for _ in 0..iters(1_000_000) {
        let mut f = rand_valid(&mut rng);
        match rng.next_u64() % 9 {
            0 => f.blocksize = rng.next_u32(),
            1 => f.samplerate = rng.next_u32(),
            2 => f.channels = rng.next_u32(),
            3 => f.bitdepth = rng.next_u32(),
            4 => f.channel_mode = rng.next_u8(),
            5 => f.max_rice_value = rng.next_u8(),
            6 => f.min_partition_order = rng.next_u8(),
            7 => f.max_partition_order = rng.next_u8(),
            _ => {
                f.min_partition_order = rng.next_u8();
                f.max_partition_order = rng.next_u8();
            }
        }
        check_validate("cfg#33b", f);
    }
}

#[test]
fn cfg_row34_repeated_calls_feed_forward() {
    let mut rng = Rng::new(0x0134);
    let p = pair();
    for _ in 0..iters(100_000) {
        let mut cf = rand_valid(&mut rng);
        let mut rfs: Vec<Fields> = p.rust.iter().map(|_| cf).collect();
        for round in 0..4 {
            let (cret, cout) = p.c.validate(cf);
            for (i, r) in p.rust.iter().enumerate() {
                let (rret, rout) = r.validate(rfs[i]);
                assert_eq!(
                    (cret, cout.to_raw()),
                    (rret, rout.to_raw()),
                    "cfg#34 round {round} mismatch ({} vs {})",
                    p.c.name,
                    r.name
                );
                rfs[i] = rout;
            }
            cf = cout;
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 35..40 — tflac_size_memory
// ---------------------------------------------------------------------------

#[test]
fn cfg_row35_size_memory_zero() {
    check_size_memory("cfg#35", 0);
}

#[test]
fn cfg_row36_size_memory_exhaustive_1_to_4096() {
    for bs in 1u32..=4096 {
        check_size_memory("cfg#36", bs);
    }
}

#[test]
fn cfg_row37_size_memory_exhaustive_flac_range() {
    for bs in 16u32..=65535 {
        check_size_memory("cfg#37", bs);
    }
}

#[test]
fn cfg_row38_size_memory_just_below_wrap() {
    for bs in [0x3FFF_FFFCu32, 0x3FFF_FFFD, 0x3FFF_FFFE, 0x3FFF_FFFF] {
        check_size_memory("cfg#38", bs);
    }
}

#[test]
fn cfg_row39_size_memory_at_and_after_wrap() {
    for bs in [
        0x4000_0000u32,
        0x4000_0001,
        0x4000_0002,
        0x4000_0003,
        0x4000_0004,
        0x7FFF_FFFF,
        0x8000_0000,
        0xBFFF_FFFF,
        0xC000_0000,
        0xFFFF_FFFC,
        0xFFFF_FFFD,
        0xFFFF_FFFE,
        0xFFFF_FFFF,
    ] {
        check_size_memory("cfg#39", bs);
    }
}

#[test]
fn cfg_row40_size_memory_random_full_u32() {
    let mut rng = Rng::new(0x0140);
    for _ in 0..iters(2_000_000) {
        check_size_memory("cfg#40", rng.next_u32());
    }
    // Also sweep every residue class mod 16 near each 2^k boundary.
    for k in 0u32..32 {
        let base = 1u32 << k;
        for d in 0..32u32 {
            check_size_memory("cfg#40", base.wrapping_add(d));
            check_size_memory("cfg#40", base.wrapping_sub(d));
        }
    }
}

// ---------------------------------------------------------------------------
// Exhaustive sweeps — the discrete input space of this library is small enough
// to enumerate outright for the axes that actually drive branches.
// ---------------------------------------------------------------------------

/// CONFIGS rows 18..29, EXHAUSTIVELY: every in-range `blocksize` crossed with
/// every legal `(min_partition_order, max_partition_order)` pair.
/// 65 520 blocksizes x 136 pairs = 8 910 720 configurations.
#[test]
fn cfg_exhaustive_blocksize_x_partition_orders() {
    let p = pair();
    let mut f = Fields::valid_base();
    f.samplerate = 44100;
    f.channels = 2;
    f.bitdepth = 16;
    f.channel_mode = 1;
    f.max_rice_value = 7;
    for bs in 16u32..=65535 {
        f.blocksize = bs;
        for max_po in 0u8..=15 {
            f.max_partition_order = max_po;
            for min_po in 0..=max_po {
                f.min_partition_order = min_po;
                let (cret, cout) = p.c.validate(f);
                for r in &p.rust {
                    let (rret, rout) = r.validate(f);
                    assert_eq!(
                        (cret, cout.to_raw()),
                        (rret, rout.to_raw()),
                        "exhaustive po mismatch ({} vs {}) for {f:?}",
                        p.c.name,
                        r.name
                    );
                }
            }
        }
    }
}

/// CONFIGS rows 1..17, EXHAUSTIVELY: every `channel_mode` (all 256 `u8`
/// values, in-enum and out-of-enum) x every `channels` x every `bitdepth`
/// x every legal `max_rice_value`.
/// 256 x 8 x 32 x 31 = 2 031 616 configurations.
#[test]
fn cfg_exhaustive_mode_x_channels_x_bitdepth_x_rice() {
    let p = pair();
    let mut f = Fields::valid_base();
    f.blocksize = 4096;
    f.samplerate = 44100;
    f.min_partition_order = 2;
    f.max_partition_order = 9;
    for mode in 0u8..=255 {
        f.channel_mode = mode;
        for ch in 1u32..=8 {
            f.channels = ch;
            for bd in 1u32..=32 {
                f.bitdepth = bd;
                for rice in 0u8..=30 {
                    f.max_rice_value = rice;
                    let (cret, cout) = p.c.validate(f);
                    assert_eq!(cret, 0, "premise: should be valid: {f:?}");
                    for r in &p.rust {
                        let (rret, rout) = r.validate(f);
                        assert_eq!(
                            (cret, cout.to_raw()),
                            (rret, rout.to_raw()),
                            "exhaustive mode/ch/bd/rice mismatch ({} vs {}) for {f:?}",
                            p.c.name,
                            r.name
                        );
                    }
                }
            }
        }
    }
}

/// `tflac_size_memory`, EXHAUSTIVELY over a dense low window plus a dense
/// window around every power of two (the masking/wrapping boundaries).
#[test]
fn cfg_exhaustive_size_memory_dense_windows() {
    let p = pair();
    let check = |bs: u32| {
        let cv = p.c.size_memory(bs);
        for r in &p.rust {
            assert_eq!(
                cv,
                r.size_memory(bs),
                "tflac_size_memory({bs}) mismatch vs {}",
                r.name
            );
        }
    };
    for bs in 0u32..(1 << 21) {
        check(bs);
    }
    for k in 0u32..32 {
        let base = 1u32 << k;
        for d in 0..4096u32 {
            check(base.wrapping_add(d));
            check(base.wrapping_sub(d));
        }
    }
    for d in 0..4096u32 {
        check(u32::MAX - d);
    }
}
