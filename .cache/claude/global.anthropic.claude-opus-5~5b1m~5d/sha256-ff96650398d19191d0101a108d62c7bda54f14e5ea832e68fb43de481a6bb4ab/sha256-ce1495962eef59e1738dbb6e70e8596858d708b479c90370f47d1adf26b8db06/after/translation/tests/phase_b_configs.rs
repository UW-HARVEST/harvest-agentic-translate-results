//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every test loads BOTH shared objects with `libloading` and compares the full
//! 24-byte `tflac` record (plus guard bytes) after the call.

mod common;

use common::*;

/// Helper: run `n` randomized inputs, overriding fields via `pin`.
#[track_caller]
fn randomized(n: usize, seed_salt: u64, mut pin: impl FnMut(&mut Rng, &mut Input)) -> usize {
    let d = Diff::load();
    let pools = Pools::new();
    let mut rng = Rng::new(SEED ^ seed_salt);
    let mut count = 0usize;
    for _ in 0..n {
        let mut input = random_input(&mut rng, &pools);
        pin(&mut rng, &mut input);
        d.check(&input);
        count += 1;
    }
    assert!(count > 0);
    count
}

// ===========================================================================
// Rows 1-4 — axis BS (cur_blocksize)
// ===========================================================================

/// Row 1 — BS1..BS13: each of the 13 exact `case` blocksizes.
#[test]
fn cfg_01_blocksize_exact_cases() {
    let per = iters(4000);
    for (i, &bs) in BLOCKSIZE_CASES.iter().enumerate() {
        randomized(per, 0x0100 + i as u64, |_, inp| inp.cur_blocksize = bs);
    }
}

/// Row 2 — BS14: default arm with `cur_blocksize <= 256`.
#[test]
fn cfg_02_blocksize_default_le_256() {
    randomized(iters(40_000), 0x0200, |rng, inp| {
        // 0..=256 minus the two case labels that live in that range (192, 256).
        loop {
            let bs = rng.range_u32(0, 256);
            if !BLOCKSIZE_CASES.contains(&bs) {
                inp.cur_blocksize = bs;
                return;
            }
        }
    });
    // Plus the explicit endpoints of the range.
    let d = Diff::load();
    for bs in [0u32, 1, 2, 191, 193, 255] {
        d.check(&Input { cur_blocksize: bs, ..Default::default() });
    }
}

/// Row 3 — BS15: default arm with `cur_blocksize > 256`.
#[test]
fn cfg_03_blocksize_default_gt_256() {
    randomized(iters(40_000), 0x0300, |rng, inp| loop {
        let bs = rng.range_u32(257, u32::MAX);
        if !BLOCKSIZE_CASES.contains(&bs) {
            inp.cur_blocksize = bs;
            return;
        }
    });
    let d = Diff::load();
    for bs in [257u32, 258, 65535, 65536, u32::MAX - 1, u32::MAX] {
        d.check(&Input { cur_blocksize: bs, ..Default::default() });
    }
}

/// Row 4 — exhaustive sweep `cur_blocksize = 0..=70_000` under 3 field profiles.
#[test]
fn cfg_04_blocksize_exhaustive_sweep() {
    let d = Diff::load();
    let profiles = [
        Input::new(44100, 2, 16, 0, 0),
        Input::new(65537, 0, 33, 3, 0),
        Input { channel_mode: 250, ..Input::new(255000, 19, 0, 0, 0) },
    ];
    for base in profiles {
        for bs in 0..=70_000u32 {
            d.check(&Input { cur_blocksize: bs, ..base });
        }
    }
}

// ===========================================================================
// Rows 5-13 — axis SR (samplerate)
// ===========================================================================

/// Row 5 — SR1..SR11: each of the 11 exact `case` samplerates.
#[test]
fn cfg_05_samplerate_exact_cases() {
    let per = iters(4000);
    for (i, &sr) in SAMPLERATE_CASES.iter().enumerate() {
        randomized(per, 0x0500 + i as u64, |_, inp| inp.samplerate = sr);
    }
}

/// Row 6 — SR12: `%1000 == 0` and `/1000 < 256` → nibble `0xC`.
#[test]
fn cfg_06_samplerate_khz_in_range() {
    randomized(iters(30_000), 0x0600, |rng, inp| loop {
        let sr = rng.range_u32(0, 255) * 1000;
        if !SAMPLERATE_CASES.contains(&sr) {
            inp.samplerate = sr;
            return;
        }
    });
    let d = Diff::load();
    for k in 0..256u32 {
        let sr = k * 1000;
        if SAMPLERATE_CASES.contains(&sr) {
            continue;
        }
        assert_eq!(
            d.check(&Input { samplerate: sr, ..Default::default() }) >> 8 & 0xF,
            0xC,
            "SR12 nibble for {sr}"
        );
    }
}

/// Row 7 — SR13: `%1000 == 0` and `/1000 >= 256` → no samplerate bits.
#[test]
fn cfg_07_samplerate_khz_out_of_range() {
    randomized(iters(30_000), 0x0700, |rng, inp| loop {
        let sr = rng.range_u32(256, u32::MAX / 1000) * 1000;
        if !SAMPLERATE_CASES.contains(&sr) {
            inp.samplerate = sr;
            return;
        }
    });
}

/// Row 8 — SR14: `%1000 != 0` and `< 65536` → nibble `0xD`.
#[test]
fn cfg_08_samplerate_sub_65536() {
    randomized(iters(30_000), 0x0800, |rng, inp| loop {
        let sr = rng.range_u32(0, 65535);
        if sr % 1000 != 0 && !SAMPLERATE_CASES.contains(&sr) {
            inp.samplerate = sr;
            return;
        }
    });
    // Exhaustive over the whole band, with the other fields fixed.
    let d = Diff::load();
    for sr in 0..65536u32 {
        d.check(&Input { samplerate: sr, ..Default::default() });
    }
}

/// Row 9 — SR15: `%1000 != 0`, `>= 65536`, `%10 == 0`, `/10 < 65536` → `0xE`.
#[test]
fn cfg_09_samplerate_decahertz_in_range() {
    randomized(iters(30_000), 0x0900, |rng, inp| loop {
        let sr = rng.range_u32(6554, 65535) * 10; // 65540 ..= 655350
        if sr >= 65536 && sr % 1000 != 0 && !SAMPLERATE_CASES.contains(&sr) {
            inp.samplerate = sr;
            return;
        }
    });
}

/// Row 10 — SR16: `%1000 != 0`, `>= 65536`, `%10 == 0`, `/10 >= 65536` → none.
#[test]
fn cfg_10_samplerate_decahertz_out_of_range() {
    randomized(iters(30_000), 0x0A00, |rng, inp| loop {
        let sr = rng.range_u32(65536, u32::MAX / 10) * 10;
        if sr % 1000 != 0 && !SAMPLERATE_CASES.contains(&sr) {
            inp.samplerate = sr;
            return;
        }
    });
}

/// Row 11 — SR17: `%1000 != 0`, `>= 65536`, `%10 != 0` → none.
#[test]
fn cfg_11_samplerate_unrepresentable() {
    randomized(iters(30_000), 0x0B00, |rng, inp| loop {
        let sr = rng.range_u32(65536, u32::MAX);
        if sr % 1000 != 0 && sr % 10 != 0 && !SAMPLERATE_CASES.contains(&sr) {
            inp.samplerate = sr;
            return;
        }
    });
}

/// Row 12 — exhaustive sweep `samplerate = 0..=200_000` under 3 field profiles.
#[test]
fn cfg_12_samplerate_exhaustive_sweep() {
    let d = Diff::load();
    let profiles = [
        Input::new(0, 2, 16, 0, 4096),
        Input::new(0, 0, 0, 1, 193),
        Input { channel_mode: 7, frame_header: u32::MAX, ..Input::new(0, 17, 32, 0, 32768) },
    ];
    for base in profiles {
        for sr in 0..=200_000u32 {
            d.check(&Input { samplerate: sr, ..base });
        }
    }
}

/// Row 13 — exhaustive sweep across the SR15 ↔ SR16 crossover at 655_360.
#[test]
fn cfg_13_samplerate_decahertz_crossover_sweep() {
    let d = Diff::load();
    for sr in 650_000..=660_000u32 {
        d.check(&Input { samplerate: sr, ..Default::default() });
        d.check(&Input { samplerate: sr, channels: 0, channel_mode: 0, ..Default::default() });
    }
}

// ===========================================================================
// Rows 14-19 — axes CM (channel_mode) and CH (channels)
// ===========================================================================

/// Row 14 — CM0 × legal channel counts 1..=8.
#[test]
fn cfg_14_mode0_legal_channels() {
    for ch in 1..=8u32 {
        randomized(iters(6000), 0x0E00 + ch as u64, |rng, inp| {
            inp.channels = ch;
            // any channel_mode whose %4 == 0
            inp.channel_mode = (rng.next_u8() & 0x3F) * 4;
        });
    }
}

/// Row 15 — CM0 × `channels == 0` (unsigned underflow).
#[test]
fn cfg_15_mode0_zero_channels() {
    randomized(iters(30_000), 0x0F00, |rng, inp| {
        inp.channels = 0;
        inp.channel_mode = (rng.next_u8() & 0x3F) * 4;
    });
}

/// Row 16 — CM0 × `channels = 9..=16` (still inside bits 4..7).
#[test]
fn cfg_16_mode0_channels_9_to_16() {
    randomized(iters(30_000), 0x1000, |rng, inp| {
        inp.channels = rng.range_u32(9, 16);
        inp.channel_mode = (rng.next_u8() & 0x3F) * 4;
    });
}

/// Row 17 — CM0 × `channels >= 17` incl. the `<< 4` truncation boundary.
#[test]
fn cfg_17_mode0_channels_overflow() {
    randomized(iters(30_000), 0x1100, |rng, inp| {
        inp.channels = rng.range_u32(17, u32::MAX);
        inp.channel_mode = (rng.next_u8() & 0x3F) * 4;
    });
    let d = Diff::load();
    for ch in [
        17u32,
        18,
        255,
        256,
        257,
        4095,
        4096,
        0x0FFF_FFFE,
        0x0FFF_FFFF,
        0x1000_0000,
        0x1000_0001,
        0x2000_0000,
        0x8000_0000,
        0xFFFF_FFFE,
        u32::MAX,
    ] {
        for mode in [0u8, 4, 8, 252] {
            d.check(&Input { channels: ch, channel_mode: mode, ..Default::default() });
        }
    }
}

/// Row 18 — CM1/CM2/CM3: `channels` must be ignored entirely.
#[test]
fn cfg_18_modes_1_2_3_ignore_channels() {
    for m in 1..=3u8 {
        randomized(iters(20_000), 0x1200 + m as u64, |rng, inp| {
            inp.channel_mode = (rng.next_u8() & 0x3F) * 4 + m;
        });
    }
    // The channel nibble must be identical regardless of `channels`.
    let d = Diff::load();
    for m in 1..=3u8 {
        let base = d.check(&Input { channel_mode: m, channels: 1, ..Default::default() });
        for ch in channels_pool() {
            let got = d.check(&Input { channel_mode: m, channels: ch, ..Default::default() });
            assert_eq!(got, base, "mode {m} must ignore channels={ch}");
        }
    }
}

/// Row 19 — every one of the 256 `channel_mode` byte values.
#[test]
fn cfg_19_channel_mode_all_256_values() {
    for m in 0..=255u8 {
        randomized(iters(400), 0x1300 + m as u64, |_, inp| inp.channel_mode = m);
    }
}

// ===========================================================================
// Rows 20-21 — axis BD (bitdepth)
// ===========================================================================

/// Row 20 — BD1..BD6: each of the 6 exact `case` bitdepths.
#[test]
fn cfg_20_bitdepth_exact_cases() {
    let per = iters(6000);
    for (i, &bd) in BITDEPTH_CASES.iter().enumerate() {
        randomized(per, 0x1400 + i as u64, |_, inp| inp.bitdepth = bd);
    }
}

/// Row 21 — BD7: `bitdepth` outside the 6 cases.
#[test]
fn cfg_21_bitdepth_default() {
    let d = Diff::load();
    for bd in 0..=64u32 {
        for base in [Input::default(), Input::new(65537, 0, 0, 2, 1), Input::new(0, 17, 0, 0, 999)] {
            d.check(&Input { bitdepth: bd, ..base });
        }
    }
    randomized(iters(30_000), 0x1500, |rng, inp| loop {
        let bd = rng.range_u32(65, u32::MAX);
        if !BITDEPTH_CASES.contains(&bd) {
            inp.bitdepth = bd;
            return;
        }
    });
}

// ===========================================================================
// Row 22 — axis FH (incoming frame_header must be fully overwritten)
// ===========================================================================

#[test]
fn cfg_22_incoming_frame_header_ignored() {
    let d = Diff::load();
    let pools = Pools::new();
    let mut rng = Rng::new(SEED ^ 0x1600);
    for _ in 0..iters(30_000) {
        let mut inp = random_input(&mut rng, &pools);
        inp.frame_header = 0;
        let with_zero = d.check(&inp);
        for seed_fh in [u32::MAX, 0x5555_5555, 0xAAAA_AAAA, rng.next_u32()] {
            let got = d.check(&Input { frame_header: seed_fh, ..inp });
            assert_eq!(
                got, with_zero,
                "incoming frame_header 0x{seed_fh:08X} leaked into the result for {inp:?}"
            );
        }
    }
}

// ===========================================================================
// Row 23 — realistic full cross-product ("well-formed encoder" configs)
// ===========================================================================

#[test]
fn cfg_23_realistic_cross_product() {
    let d = Diff::load();
    let mut n = 0usize;
    for &bs in BLOCKSIZE_CASES.iter() {
        for &sr in SAMPLERATE_CASES.iter() {
            for mode in 0..4u8 {
                for ch in 1..=8u32 {
                    for &bd in BITDEPTH_CASES.iter() {
                        d.check(&Input::new(sr, ch, bd, mode, bs));
                        n += 1;
                    }
                }
            }
        }
    }
    assert_eq!(n, 13 * 11 * 4 * 8 * 6);
}

// ===========================================================================
// Rows 24-25 — cross-field interactions
// ===========================================================================

/// Row 24 — nibble overflow from `channels` colliding with every SR class.
#[test]
fn cfg_24_channel_overflow_vs_samplerate() {
    let d = Diff::load();
    // One representative per SR class SR1..SR17.
    let sr_reps: Vec<u32> = SAMPLERATE_CASES
        .iter()
        .copied()
        .chain([0u32, 1000, 255000, 256000, 512000, 11025, 65535, 65536, 88200, 655350, 655360, 65537, u32::MAX])
        .collect();
    for sr in sr_reps {
        for ch in [0u32, 1, 8, 16, 17, 18, 33, 255, 4096, 0x1000_0001, u32::MAX] {
            for &bs in BLOCKSIZE_CASES.iter() {
                d.check(&Input::new(sr, ch, 16, 0, bs));
                d.check(&Input::new(sr, ch, 0, 4, bs));
            }
        }
    }
}

/// Row 25 — `channels == 0` (`0xFFFF_FFF0`) against every BS and BD class.
#[test]
fn cfg_25_channel_underflow_vs_blocksize_bitdepth() {
    let d = Diff::load();
    let bs_reps: Vec<u32> =
        BLOCKSIZE_CASES.iter().copied().chain([0u32, 1, 255, 256, 257, u32::MAX]).collect();
    let bd_reps: Vec<u32> = BITDEPTH_CASES.iter().copied().chain([0u32, 1, 33, u32::MAX]).collect();
    for &bs in bs_reps.iter() {
        for &bd in bd_reps.iter() {
            for &sr in [44100u32, 0, 65537, 655360].iter() {
                let fh = d.check(&Input::new(sr, 0, bd, 0, bs));
                // The C ORs 0xFFFFFFF0 in, so bits 4..31 are all set.
                assert_eq!(fh & 0xFFFF_FFF0, 0xFFFF_FFF0, "underflow saturation for bs={bs}");
            }
        }
    }
}

// ===========================================================================
// Rows 26-27 — fuzzing
// ===========================================================================

/// Row 26 — unconstrained fuzz over the full input domain.
#[test]
fn cfg_26_unconstrained_fuzz() {
    let d = Diff::load();
    let mut rng = Rng::new(SEED ^ 0x1A00);
    let n = iters(2_000_000);
    for _ in 0..n {
        d.check(&Input {
            samplerate: rng.next_u32(),
            channels: rng.next_u32(),
            bitdepth: rng.next_u32(),
            channel_mode: rng.next_u8(),
            frame_header: rng.next_u32(),
            cur_blocksize: rng.next_u32(),
            padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
        });
    }
}

/// Row 27 — structure-aware fuzz drawing from the per-axis interesting pools.
#[test]
fn cfg_27_structure_aware_fuzz() {
    let d = Diff::load();
    let pools = Pools::new();
    let mut rng = Rng::new(SEED ^ 0x1B00);
    let n = iters(2_000_000);
    for _ in 0..n {
        d.check(&Input {
            samplerate: rng.pick(&pools.samplerate),
            channels: rng.pick(&pools.channels),
            bitdepth: rng.pick(&pools.bitdepth),
            channel_mode: rng.next_u8(),
            frame_header: rng.pick(&pools.samplerate),
            cur_blocksize: rng.pick(&pools.blocksize),
            padding: [rng.next_u8(), rng.next_u8(), rng.next_u8()],
        });
    }
}

// ===========================================================================
// Row 28 — repeated / in-place invocation and odd alignment
// ===========================================================================

#[test]
fn cfg_28_repeated_and_misaligned_invocation() {
    let d = Diff::load();
    let pools = Pools::new();
    let mut rng = Rng::new(SEED ^ 0x1C00);

    // (a) Calling twice in a row on the same record must be idempotent and
    //     identical between the two libraries.
    for _ in 0..iters(20_000) {
        let inp = random_input(&mut rng, &pools);
        let mut cb = Buf::new(&inp);
        let mut rb = Buf::new(&inp);
        unsafe {
            (d.c)(cb.ptr());
            (d.c)(cb.ptr());
            (d.rust)(rb.ptr());
            (d.rust)(rb.ptr());
        }
        assert_eq!(cb.bytes(), rb.bytes(), "second call diverged for {inp:?}");
        // and it equals the single-call result
        assert_eq!(cb.bytes(), d.run_c(&inp).bytes(), "C not idempotent for {inp:?}");
    }

    // (b) The record placed at a 4-byte-aligned-but-not-8-byte-aligned address.
    #[repr(align(16))]
    struct Odd([u8; 64]);
    for _ in 0..iters(20_000) {
        let inp = random_input(&mut rng, &pools);
        let mk = |f: UpdateFrameHeaderFn| -> [u8; 64] {
            let mut o = Odd([0x5Au8; 64]);
            let src = Buf::new(&inp);
            o.0[4..4 + TFLAC_SIZE].copy_from_slice(&src.bytes()[GUARD..GUARD + TFLAC_SIZE]);
            unsafe { f(o.0.as_mut_ptr().add(4)) };
            o.0
        };
        assert_eq!(mk(d.c), mk(d.rust), "misaligned record diverged for {inp:?}");
    }
}

// ===========================================================================
// Rows 29-32 — exhaustive per-axis sweeps
// ===========================================================================

/// Row 29 — every `samplerate` in `0..=4_300_000`.
#[test]
fn cfg_29_samplerate_exhaustive_wide() {
    let d = Diff::load();
    let profiles = [Input::new(0, 2, 16, 0, 4096), Input::new(0, 0, 33, 0, 1)];
    for base in profiles {
        for sr in 0..=4_300_000u32 {
            d.check(&Input { samplerate: sr, ..base });
        }
    }
}

/// Row 30 — every `cur_blocksize` in `0..=1_048_576`.
#[test]
fn cfg_30_blocksize_exhaustive_wide() {
    let d = Diff::load();
    let profiles = [Input::new(44100, 2, 16, 0, 0), Input::new(655_360, 17, 0, 3, 0)];
    for base in profiles {
        for bs in 0..=1_048_576u32 {
            d.check(&Input { cur_blocksize: bs, ..base });
        }
    }
}

/// Row 31 — every `channels` in `0..=1_048_576` under mode 0, plus the two
/// `<< 4` truncation neighbourhoods.
#[test]
fn cfg_31_channels_exhaustive_wide() {
    let d = Diff::load();
    let base = Input::new(8000, 0, 16, 0, 4096);
    for ch in 0..=1_048_576u32 {
        d.check(&Input { channels: ch, ..base });
    }
    for ch in 0x0FFF_F000u32..=0x1000_1000 {
        d.check(&Input { channels: ch, ..base });
    }
    for ch in (u32::MAX - 4096)..=u32::MAX {
        d.check(&Input { channels: ch, ..base });
    }
}

/// Row 32 — every `bitdepth` in `0..=65_536`.
#[test]
fn cfg_32_bitdepth_exhaustive_wide() {
    let d = Diff::load();
    let profiles = [Input::new(44100, 2, 0, 0, 4096), Input::new(65_536, 0, 0, 2, 257)];
    for base in profiles {
        for bd in 0..=65_536u32 {
            d.check(&Input { bitdepth: bd, ..base });
        }
    }
}

// ===========================================================================
// Row 33 — complete cross-product of class representatives on all five axes
// ===========================================================================

#[test]
fn cfg_33_complete_class_cross_product() {
    let d = Diff::load();

    // One representative per class of every axis (see CONFIGS.md).
    let bs: Vec<u32> = BLOCKSIZE_CASES
        .iter()
        .copied()
        .chain([0u32, 1, 255, 256, 257, 65_535, u32::MAX])
        .collect();
    let sr: Vec<u32> = SAMPLERATE_CASES
        .iter()
        .copied()
        .chain([
            0u32, 1_000, 255_000, 256_000, 11_025, 65_535, 65_536, 65_537, 88_200, 655_350,
            655_360, u32::MAX,
        ])
        .collect();
    let ch: [u32; 13] = [
        0,
        1,
        2,
        7,
        8,
        9,
        16,
        17,
        255,
        0x0FFF_FFFF,
        0x1000_0000,
        0x1000_0001,
        u32::MAX,
    ];
    let cm: [u8; 9] = [0, 1, 2, 3, 4, 5, 6, 7, 255];
    let bd: Vec<u32> = BITDEPTH_CASES.iter().copied().chain([0u32, 1, 33, u32::MAX]).collect();

    assert_eq!((bs.len(), sr.len(), ch.len(), cm.len(), bd.len()), (20, 23, 13, 9, 10));

    let mut n = 0usize;
    for &b in bs.iter() {
        for &s in sr.iter() {
            for &c in ch.iter() {
                for &m in cm.iter() {
                    for &t in bd.iter() {
                        d.check(&Input::new(s, c, t, m, b));
                        n += 1;
                    }
                }
            }
        }
    }
    assert_eq!(n, 20 * 23 * 13 * 9 * 10, "cross-product size");
}
