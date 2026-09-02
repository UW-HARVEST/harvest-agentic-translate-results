//! Phase B — valid-path differential tests.
//!
//! One `#[test]` per row of `CONFIGS.md`. Every test drives BOTH shared objects
//! through their exported `update_frame_header` symbol and compares the full
//! 24-byte struct image. Each row uses many randomized inputs from a fixed
//! seed, so results are reproducible.

mod common;

use common::*;

/// Iterations per randomized row. Kept high enough to sweep the other axes,
/// low enough that the whole suite finishes in well under a second.
const N: usize = 4000;

// --- Row 1: each enumerated cur_blocksize -----------------------------------

#[test]
fn cfg_01_blocksize_enumerated() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0001);
    for &bs in BLOCKSIZES.iter() {
        for _ in 0..N {
            let mut t = rand_other_axes(&mut rng);
            t.cur_blocksize = bs;
            p.check(t);
        }
    }
}

// --- Row 2: cur_blocksize default && <= 256 ---------------------------------

#[test]
fn cfg_02_blocksize_default_small() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0002);
    let mut checked = 0usize;
    // Exhaustive over 0..=256, plus randomized repeats of the other axes.
    for bs in 0u32..=256 {
        if BLOCKSIZES.contains(&bs) {
            continue;
        }
        for _ in 0..8 {
            let mut t = rand_other_axes(&mut rng);
            t.cur_blocksize = bs;
            p.check(t);
            checked += 1;
        }
    }
    assert!(checked > 1000, "row 2 barely exercised: {checked}");
}

// --- Row 3: cur_blocksize default && > 256 ----------------------------------

#[test]
fn cfg_03_blocksize_default_large() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0003);
    for _ in 0..N * 4 {
        let mut t = rand_other_axes(&mut rng);
        let bs = loop {
            let c = if rng.next_u64() % 2 == 0 {
                rng.range_u32(257, 40_000)
            } else {
                rng.range_u32(257, u32::MAX)
            };
            if !BLOCKSIZES.contains(&c) {
                break c;
            }
        };
        t.cur_blocksize = bs;
        p.check(t);
    }
}

// --- Row 4: each enumerated samplerate --------------------------------------

#[test]
fn cfg_04_samplerate_enumerated() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0004);
    for &sr in SAMPLERATES.iter() {
        for _ in 0..N {
            let mut t = rand_other_axes(&mut rng);
            t.samplerate = sr;
            p.check(t);
        }
    }
}

// --- Rows 5..10: the six samplerate default sub-branches --------------------

fn sr_class(sr: u32) -> u8 {
    if SAMPLERATES.contains(&sr) {
        return 0; // enumerated, not a default sub-branch
    }
    if sr % 1000 == 0 {
        if sr / 1000 < 256 {
            1 // B-d1
        } else {
            2 // B-d2
        }
    } else if sr < 65536 {
        3 // B-d3
    } else if sr % 10 == 0 {
        if sr / 10 < 65536 {
            4 // B-d4
        } else {
            5 // B-d5
        }
    } else {
        6 // B-d6
    }
}

fn run_sr_class(seed: u64, want: u8, gen: &mut dyn FnMut(&mut Rng) -> u32) {
    let p = Pair::load();
    let mut rng = Rng::new(seed);
    let mut hits = 0usize;
    let mut tries = 0usize;
    while hits < N && tries < N * 200 {
        tries += 1;
        let sr = gen(&mut rng);
        if sr_class(sr) != want {
            continue;
        }
        let mut t = rand_other_axes(&mut rng);
        t.samplerate = sr;
        p.check(t);
        hits += 1;
    }
    assert!(
        hits >= N,
        "samplerate class {want}: only produced {hits} inputs in {tries} tries"
    );
}

#[test]
fn cfg_05_samplerate_d1_khz_in_range() {
    // %1000 == 0 && /1000 < 256
    run_sr_class(0x0000_0005, 1, &mut |r| r.range_u32(0, 255) * 1000);
}

#[test]
fn cfg_06_samplerate_d2_khz_out_of_range() {
    // %1000 == 0 && /1000 >= 256
    run_sr_class(0x0000_0006, 2, &mut |r| {
        r.range_u32(256, 4_000_000).wrapping_mul(1000)
    });
}

#[test]
fn cfg_07_samplerate_d3_sub65536() {
    // %1000 != 0 && < 65536
    run_sr_class(0x0000_0007, 3, &mut |r| r.range_u32(0, 65535));
}

#[test]
fn cfg_08_samplerate_d4_dahz_in_range() {
    // %1000 != 0 && >= 65536 && %10 == 0 && /10 < 65536
    run_sr_class(0x0000_0008, 4, &mut |r| r.range_u32(6554, 65535) * 10);
}

#[test]
fn cfg_09_samplerate_d5_dahz_out_of_range() {
    // %1000 != 0 && >= 65536 && %10 == 0 && /10 >= 65536
    run_sr_class(0x0000_0009, 5, &mut |r| {
        r.range_u32(65536, 400_000_000).wrapping_mul(10)
    });
}

#[test]
fn cfg_10_samplerate_d6_unrepresentable() {
    // %1000 != 0 && >= 65536 && %10 != 0
    run_sr_class(0x0000_000A, 6, &mut |r| r.range_u32(65536, u32::MAX));
}

// --- Rows 11..14: independent mode, channels sub-axis -----------------------

fn run_independent(seed: u64, gen: &mut dyn FnMut(&mut Rng) -> u32) {
    let p = Pair::load();
    let mut rng = Rng::new(seed);
    for _ in 0..N * 2 {
        let mut t = rand_other_axes(&mut rng);
        // Any channel_mode whose %4 == 0, not just literal 0.
        t.channel_mode = rng.range_u32(0, 63) as u8 * 4;
        t.channels = gen(&mut rng);
        p.check(t);
    }
}

#[test]
fn cfg_11_independent_channels_1_to_8() {
    run_independent(0x0000_000B, &mut |r| r.range_u32(1, 8));
}

#[test]
fn cfg_12_independent_channels_zero_underflow() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_000C);
    for _ in 0..N * 2 {
        let mut t = rand_other_axes(&mut rng);
        t.channel_mode = rng.range_u32(0, 63) as u8 * 4;
        t.channels = 0;
        p.check(t);
        // Document the observable: (0-1)<<4 saturates the header.
        let h = p.c_header(t);
        assert_eq!(
            h & 0xFFFF_FFF0,
            0xFFFF_FFF0,
            "expected channels==0 underflow spill, got 0x{h:08X}"
        );
    }
}

#[test]
fn cfg_13_independent_channels_9_to_255() {
    run_independent(0x0000_000D, &mut |r| r.range_u32(9, 255));
}

#[test]
fn cfg_14_independent_channels_full_u32() {
    run_independent(0x0000_000E, &mut |r| match r.next_u64() % 3 {
        0 => r.next_u32(),
        1 => u32::MAX - r.range_u32(0, 4),
        _ => r.range_u32(0, 0xFFFF_FFF),
    });
}

// --- Rows 15..17: the three joint stereo modes ------------------------------

fn run_mode(seed: u64, residue: u8) {
    let p = Pair::load();
    let mut rng = Rng::new(seed);
    for _ in 0..N * 2 {
        let mut t = rand_other_axes(&mut rng);
        // Sweep every byte with this residue class mod 4.
        t.channel_mode = rng.range_u32(0, 63) as u8 * 4 + residue;
        assert_eq!(t.channel_mode % 4, residue);
        t.channels = random_channels(&mut rng);
        p.check(t);
    }
}

#[test]
fn cfg_15_mode_left_side() {
    run_mode(0x0000_000F, 1);
}

#[test]
fn cfg_16_mode_side_right() {
    run_mode(0x0000_0010, 2);
}

#[test]
fn cfg_17_mode_mid_side() {
    run_mode(0x0000_0011, 3);
}

// --- Row 18: every possible channel_mode byte -------------------------------

#[test]
fn cfg_18_channel_mode_all_256_bytes() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0012);
    for m in 0u16..=255 {
        for _ in 0..24 {
            let mut t = rand_other_axes(&mut rng);
            t.channel_mode = m as u8;
            p.check(t);
        }
    }
}

// --- Rows 19..20: bit depth -------------------------------------------------

#[test]
fn cfg_19_bitdepth_enumerated() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0013);
    for &bd in BITDEPTHS.iter() {
        for _ in 0..N {
            let mut t = rand_other_axes(&mut rng);
            t.bitdepth = bd;
            p.check(t);
        }
    }
}

#[test]
fn cfg_20_bitdepth_default() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0014);
    // Exhaustive over the interesting low range...
    for bd in 0u32..=64 {
        if BITDEPTHS.contains(&bd) {
            continue;
        }
        for _ in 0..16 {
            let mut t = rand_other_axes(&mut rng);
            t.bitdepth = bd;
            p.check(t);
        }
    }
    // ...plus wide random values and the u32 ceiling.
    for _ in 0..N {
        let mut t = rand_other_axes(&mut rng);
        let bd = loop {
            let c = rng.next_u32();
            if !BITDEPTHS.contains(&c) {
                break c;
            }
        };
        t.bitdepth = bd;
        p.check(t);
    }
    for bd in [u32::MAX, u32::MAX - 1, 0] {
        let mut t = rand_other_axes(&mut rng);
        t.bitdepth = bd;
        p.check(t);
    }
}

// --- Row 21: exhaustive realistic cross product -----------------------------

#[test]
fn cfg_21_full_realistic_cross_product() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0015);
    let mut count = 0usize;
    for &bs in BLOCKSIZES.iter() {
        for &sr in SAMPLERATES.iter() {
            for mode in 0u8..4 {
                for ch in 1u32..=8 {
                    for &bd in BITDEPTHS.iter() {
                        let t = Tflac {
                            samplerate: sr,
                            channels: ch,
                            bitdepth: bd,
                            channel_mode: mode,
                            frame_header: rng.next_u32(),
                            cur_blocksize: bs,
                        };
                        p.check(t);
                        count += 1;
                    }
                }
            }
        }
    }
    assert_eq!(count, 13 * 11 * 4 * 8 * 6);
}

// --- Row 22: unconstrained fuzz over all six fields -------------------------

#[test]
fn cfg_22_unconstrained_fuzz() {
    let p = Pair::load();
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_0022);
    for _ in 0..200_000 {
        let t = Tflac {
            samplerate: rng.next_u32(),
            channels: rng.next_u32(),
            bitdepth: rng.next_u32(),
            channel_mode: rng.next_u8(),
            frame_header: rng.next_u32(),
            cur_blocksize: rng.next_u32(),
        };
        p.check(t);
    }
}

// --- Row 23: idempotence / does not read incoming frame_header --------------

#[test]
fn cfg_23_idempotent_and_ignores_prior_header() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0017);
    for _ in 0..N * 4 {
        let base = rand_other_axes(&mut rng);

        // Same input, two different pre-existing frame_header values: the
        // result must be identical (the C assigns, never OR-accumulates).
        let mut a = base;
        a.frame_header = 0x0000_0000;
        let mut b = base;
        b.frame_header = 0xFFFF_FFFF;

        let (ca, ra) = p.run(a);
        let (cb, rb) = p.run(b);
        assert_eq!(ca.frame_header, cb.frame_header, "C is header-dependent?");
        assert_eq!(ra.frame_header, rb.frame_header, "Rust ORs into old header");
        assert_eq!(ca, ra);
        assert_eq!(cb, rb);

        // Calling twice in a row must be a no-op the second time, on both.
        let mut cc = base;
        let mut rr = base;
        unsafe {
            (p.c)(&mut cc);
            let after_one = cc;
            (p.c)(&mut cc);
            assert_eq!(after_one, cc, "C not idempotent");

            (p.rust)(&mut rr);
            let after_one_r = rr;
            (p.rust)(&mut rr);
            assert_eq!(after_one_r, rr, "Rust not idempotent");
        }
        assert_eq!(cc, rr);
    }
}

// --- Row 24: boundary sweep around every constant ---------------------------

#[test]
fn cfg_24_boundary_sweep() {
    let p = Pair::load();
    let mut rng = Rng::new(0x0000_0018);

    // Every value 0..=1024 on each axis in turn.
    for v in 0u32..=1024 {
        for axis in 0..4 {
            let mut t = rand_other_axes(&mut rng);
            match axis {
                0 => t.cur_blocksize = v,
                1 => t.samplerate = v,
                2 => t.channels = v,
                _ => t.bitdepth = v,
            }
            p.check(t);
        }
    }

    // n-1, n, n+1 around every constant the C switches on, plus the nested-if
    // thresholds, applied to every axis (a constant for one axis is still a
    // valid probe for another).
    let mut probes: Vec<u32> = Vec::new();
    for &c in BLOCKSIZES
        .iter()
        .chain(SAMPLERATES.iter())
        .chain(BITDEPTHS.iter())
    {
        probes.extend_from_slice(&[c.wrapping_sub(1), c, c + 1]);
    }
    for &c in &[
        0u32, 1, 255, 256, 257, 65535, 65536, 65537, 255_000, 256_000, 655_350, 655_360,
        u32::MAX - 1,
        u32::MAX,
    ] {
        probes.extend_from_slice(&[c.wrapping_sub(1), c, c.wrapping_add(1)]);
    }
    probes.sort_unstable();
    probes.dedup();

    for &v in &probes {
        for axis in 0..4 {
            for _ in 0..4 {
                let mut t = rand_other_axes(&mut rng);
                match axis {
                    0 => t.cur_blocksize = v,
                    1 => t.samplerate = v,
                    2 => t.channels = v,
                    _ => t.bitdepth = v,
                }
                p.check(t);
            }
        }
    }

    // Cross product of the probe set against itself on the two axes whose
    // fields can collide (channels spill vs. blocksize/bitdepth nibbles).
    for &a in probes.iter() {
        for &b in probes.iter().step_by(7) {
            let mut t = rand_other_axes(&mut rng);
            t.channels = a;
            t.channel_mode = 0;
            t.cur_blocksize = b;
            p.check(t);
        }
    }
}
