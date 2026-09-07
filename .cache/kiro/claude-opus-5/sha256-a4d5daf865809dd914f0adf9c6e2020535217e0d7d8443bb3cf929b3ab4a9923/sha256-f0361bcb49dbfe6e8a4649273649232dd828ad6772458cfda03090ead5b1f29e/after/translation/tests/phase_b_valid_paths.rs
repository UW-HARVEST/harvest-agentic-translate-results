//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads BOTH the C `.so` and the
//! Rust `.so` via `libloading` and compares all 24 struct bytes after the call.
//! Each row pins its target axis and randomizes all other axes (fixed seed), so
//! each row is a cross-product sample, not one hand-picked value.

mod common;
use common::*;

/// Per-row randomized iteration count.
const N: usize = 4000;

/// Row 0 — full-axis random sweep over BS×SR×CM×CH×BD×IN×PAD.
#[test]
fn cfg_row_00_full_axis_random_sweep() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED);
    for i in 0..200_000 {
        libs.assert_same(rand_input(&mut rng), &format!("row0 full sweep iter {i}"));
    }
}

/// Row 1 — the 13 exact block-size `case` labels.
#[test]
fn cfg_row_01_blocksize_exact_cases() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 1);
    for &bs in BLOCKSIZE_CASES.iter() {
        for i in 0..N {
            let mut inp = rand_input(&mut rng);
            inp.cur_blocksize = bs;
            libs.assert_same(inp, &format!("row1 blocksize={bs} iter {i}"));
        }
    }
}

/// Row 2 — block-size `default` branch, `<= 256`.
#[test]
fn cfg_row_02_blocksize_default_le_256() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 2);
    // Exhaustive over 0..=256 (excluding the two exact cases), each with random
    // other axes, plus extra randomized repeats.
    for bs in 0u32..=256 {
        if is_blocksize_case(bs) {
            continue;
        }
        let mut inp = rand_input(&mut rng);
        inp.cur_blocksize = bs;
        libs.assert_same(inp, &format!("row2 exhaustive blocksize={bs}"));
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.cur_blocksize = rand_blocksize_default_le256(&mut rng);
        libs.assert_same(inp, &format!("row2 random iter {i}"));
    }
}

/// Row 3 — block-size `default` branch, `> 256`.
#[test]
fn cfg_row_03_blocksize_default_gt_256() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.cur_blocksize = rand_blocksize_default_gt256(&mut rng);
        libs.assert_same(inp, &format!("row3 random iter {i}"));
    }
}

/// Row 4 — the 11 exact sample-rate `case` labels.
#[test]
fn cfg_row_04_samplerate_exact_cases() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 4);
    for &sr in SAMPLERATE_CASES.iter() {
        for i in 0..N {
            let mut inp = rand_input(&mut rng);
            inp.samplerate = sr;
            libs.assert_same(inp, &format!("row4 samplerate={sr} iter {i}"));
        }
    }
}

/// Row 5 — SR default, `%1000==0 && /1000 < 256` → `0x0C << 8`.
#[test]
fn cfg_row_05_samplerate_kilo_ok() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 5);
    // Exhaustive over every multiple of 1000 below 256000.
    for k in 0u32..256 {
        let sr = k * 1000;
        if is_samplerate_case(sr) {
            continue;
        }
        let mut inp = rand_input(&mut rng);
        inp.samplerate = sr;
        libs.assert_same(inp, &format!("row5 exhaustive samplerate={sr}"));
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = rand_sr_kilo_ok(&mut rng);
        libs.assert_same(inp, &format!("row5 random iter {i}"));
    }
}

/// Row 6 — SR default, `%1000==0 && /1000 >= 256` → no SR bits.
#[test]
fn cfg_row_06_samplerate_kilo_overflow() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 6);
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = rand_sr_kilo_overflow(&mut rng);
        libs.assert_same(inp, &format!("row6 random iter {i}"));
    }
    // Boundary: 256000 is the first rejected multiple of 1000.
    for &sr in &[256000u32, 255999, 256001, 1_000_000, 4_294_967_000] {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = sr;
        libs.assert_same(inp, &format!("row6 boundary samplerate={sr}"));
    }
}

/// Row 7 — SR default, `%1000!=0 && < 65536` → `0x0D << 8`.
#[test]
fn cfg_row_07_samplerate_lt_65536() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 7);
    // Exhaustive over the whole sub-65536 space.
    for sr in 0u32..65536 {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = sr;
        libs.assert_same(inp, &format!("row7 exhaustive samplerate={sr}"));
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = rand_sr_lt_65536(&mut rng);
        libs.assert_same(inp, &format!("row7 random iter {i}"));
    }
}

/// Row 8 — SR default, `%1000!=0 && >=65536 && %10==0 && /10 < 65536` → `0x0E << 8`.
#[test]
fn cfg_row_08_samplerate_deca_ok() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 8);
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = rand_sr_deca_ok(&mut rng);
        libs.assert_same(inp, &format!("row8 random iter {i}"));
    }
    for &sr in &[65540u32, 65530, 655350, 655340, 100010, 123450] {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = sr;
        libs.assert_same(inp, &format!("row8 boundary samplerate={sr}"));
    }
}

/// Row 9 — SR default, `%1000!=0 && >=65536 && %10==0 && /10 >= 65536` → no SR bits.
#[test]
fn cfg_row_09_samplerate_deca_overflow() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 9);
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = rand_sr_deca_overflow(&mut rng);
        libs.assert_same(inp, &format!("row9 random iter {i}"));
    }
    for &sr in &[655370u32, 655360, 655350, 4_294_967_290] {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = sr;
        libs.assert_same(inp, &format!("row9 boundary samplerate={sr}"));
    }
}

/// Row 10 — SR default, `%1000!=0 && >=65536 && %10!=0` → no SR bits.
#[test]
fn cfg_row_10_samplerate_no_representation() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 10);
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = rand_sr_none(&mut rng);
        libs.assert_same(inp, &format!("row10 random iter {i}"));
    }
    for &sr in &[65536u32, 65537, 65539, u32::MAX, u32::MAX - 1] {
        let mut inp = rand_input(&mut rng);
        inp.samplerate = sr;
        libs.assert_same(inp, &format!("row10 boundary samplerate={sr}"));
    }
}

/// Row 11 — INDEPENDENT mode with `channels == 1` and `2..=8`.
#[test]
fn cfg_row_11_independent_typical_channels() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 11);
    for ch in 1u32..=8 {
        for i in 0..N {
            let mut inp = rand_input(&mut rng);
            // any channel_mode ≡ 0 (mod 4)
            inp.channel_mode = (rng.range_u32(0, 63) * 4) as u8;
            inp.channels = ch;
            libs.assert_same(inp, &format!("row11 channels={ch} iter {i}"));
        }
    }
}

/// Row 12 — INDEPENDENT mode with `channels == 0` (unsigned underflow).
#[test]
fn cfg_row_12_independent_channels_zero() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 12);
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.channel_mode = (rng.range_u32(0, 63) * 4) as u8;
        inp.channels = 0;
        libs.assert_same(inp, &format!("row12 channels=0 iter {i}"));
    }
}

/// Row 13 — INDEPENDENT mode with `channels` in `9..=16` (fills the nibble).
#[test]
fn cfg_row_13_independent_channels_9_to_16() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 13);
    for ch in 9u32..=16 {
        for i in 0..N {
            let mut inp = rand_input(&mut rng);
            inp.channel_mode = (rng.range_u32(0, 63) * 4) as u8;
            inp.channels = ch;
            libs.assert_same(inp, &format!("row13 channels={ch} iter {i}"));
        }
    }
}

/// Row 14 — INDEPENDENT mode, `channels > 16`: bits bleed into other fields.
#[test]
fn cfg_row_14_independent_channels_bleed() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..N * 4 {
        let mut inp = rand_input(&mut rng);
        inp.channel_mode = (rng.range_u32(0, 63) * 4) as u8;
        inp.channels = rng.range_u32(17, 0xEFFF_FFFF);
        libs.assert_same(inp, &format!("row14 random iter {i}"));
    }
    for &ch in &[17u32, 18, 4096, 0x1000_0000, 0x0FFF_FFFF, 0xEFFF_FFFF] {
        let mut inp = rand_input(&mut rng);
        inp.channel_mode = 0;
        inp.channels = ch;
        libs.assert_same(inp, &format!("row14 boundary channels={ch}"));
    }
}

/// Row 15 — INDEPENDENT mode, `channels >= 0xF000_0000`: `<< 4` truncates.
#[test]
fn cfg_row_15_independent_channels_shift_truncates() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 15);
    for i in 0..N * 4 {
        let mut inp = rand_input(&mut rng);
        inp.channel_mode = (rng.range_u32(0, 63) * 4) as u8;
        inp.channels = rng.range_u32(0xF000_0000, u32::MAX);
        libs.assert_same(inp, &format!("row15 random iter {i}"));
    }
    for &ch in &[0xF000_0000u32, 0xF000_0001, u32::MAX - 1, u32::MAX] {
        let mut inp = rand_input(&mut rng);
        inp.channel_mode = 0;
        inp.channels = ch;
        libs.assert_same(inp, &format!("row15 boundary channels={ch}"));
    }
}

/// Row 16 — LEFT_SIDE (`channel_mode % 4 == 1`); `channels` must be ignored.
#[test]
fn cfg_row_16_mode_left_side() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 16);
    for i in 0..N * 4 {
        let mut inp = rand_input(&mut rng);
        inp.channel_mode = (rng.range_u32(0, 63) * 4 + 1) as u8;
        libs.assert_same(inp, &format!("row16 random iter {i}"));
    }
}

/// Row 17 — SIDE_RIGHT (`channel_mode % 4 == 2`).
#[test]
fn cfg_row_17_mode_side_right() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..N * 4 {
        let mut inp = rand_input(&mut rng);
        inp.channel_mode = (rng.range_u32(0, 63) * 4 + 2) as u8;
        libs.assert_same(inp, &format!("row17 random iter {i}"));
    }
}

/// Row 18 — MID_SIDE (`channel_mode % 4 == 3`).
#[test]
fn cfg_row_18_mode_mid_side() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 18);
    for i in 0..N * 4 {
        let mut inp = rand_input(&mut rng);
        inp.channel_mode = (rng.range_u32(0, 63) * 4 + 3) as u8;
        libs.assert_same(inp, &format!("row18 random iter {i}"));
    }
}

/// Row 19 — exhaustive `channel_mode` sweep `0..=255` (covers value 4 ==
/// `TFLAC_CHANNEL_MODE_COUNT` and every other out-of-enum byte).
#[test]
fn cfg_row_19_channel_mode_exhaustive_byte_sweep() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 19);
    for m in 0u16..=255 {
        for i in 0..64 {
            let mut inp = rand_input(&mut rng);
            inp.channel_mode = m as u8;
            libs.assert_same(inp, &format!("row19 channel_mode={m} iter {i}"));
        }
    }
}

/// Row 20 — the 6 exact bit-depth `case` labels.
#[test]
fn cfg_row_20_bitdepth_exact_cases() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 20);
    for &bd in BITDEPTH_CASES.iter() {
        for i in 0..N {
            let mut inp = rand_input(&mut rng);
            inp.bitdepth = bd;
            libs.assert_same(inp, &format!("row20 bitdepth={bd} iter {i}"));
        }
    }
}

/// Row 21 — bit-depth `default` branch.
#[test]
fn cfg_row_21_bitdepth_default() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 21);
    // Exhaustive over 0..=64 excluding the 6 cases.
    for bd in 0u32..=64 {
        if is_bitdepth_case(bd) {
            continue;
        }
        let mut inp = rand_input(&mut rng);
        inp.bitdepth = bd;
        libs.assert_same(inp, &format!("row21 exhaustive bitdepth={bd}"));
    }
    for i in 0..N {
        let mut inp = rand_input(&mut rng);
        inp.bitdepth = rand_bitdepth_default(&mut rng);
        libs.assert_same(inp, &format!("row21 random iter {i}"));
    }
}

/// Row 22 — pre-dirtied `frame_header` must be overwritten (`=`), not ORed.
#[test]
fn cfg_row_22_frame_header_pre_dirtied() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 22);
    for i in 0..N * 4 {
        let mut inp = rand_input(&mut rng);
        inp.frame_header_initial = match rng.next_u64() % 4 {
            0 => u32::MAX,
            1 => 0x5555_5555,
            2 => 1,
            _ => rng.next_u32(),
        };
        libs.assert_same(inp, &format!("row22 random iter {i}"));
    }
}

/// Row 23 — struct tail padding (bytes 13..16) must be left untouched.
#[test]
fn cfg_row_23_tail_padding_preserved() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 23);
    for i in 0..N * 4 {
        let mut inp = rand_input(&mut rng);
        inp.padding = if i % 3 == 0 { 0xAA } else { rng.next_u8() };
        // `assert_same` already compares all 24 bytes, padding included.
        libs.assert_same(inp, &format!("row23 random iter {i}"));
    }
    // And explicitly check both libs leave every non-frame_header byte alone.
    for i in 0..N {
        let inp = rand_input(&mut rng);
        let before = inp.to_raw();
        let mut c = before;
        let mut r = before;
        libs.call_c(&mut c);
        libs.call_rust(&mut r);
        for off in 0..TFLAC_SIZE {
            if (OFF_FRAME_HEADER..OFF_FRAME_HEADER + 4).contains(&off) {
                continue;
            }
            assert_eq!(
                c.0[off], before.0[off],
                "row23 iter {i}: C modified byte {off} (unexpected; C is ground truth)"
            );
            assert_eq!(
                r.0[off], before.0[off],
                "row23 iter {i}: Rust modified byte {off} but C did not"
            );
        }
    }
}

/// Row 24 — calling twice in a row on the same struct must match.
#[test]
fn cfg_row_24_repeated_invocation() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..N * 4 {
        let inp = rand_input(&mut rng);
        let mut c = inp.to_raw();
        let mut r = inp.to_raw();
        libs.call_c(&mut c);
        libs.call_rust(&mut r);
        assert_eq!(c, r, "row24 iter {i}: first call diverged\nC={c:?}\nR={r:?}");
        libs.call_c(&mut c);
        libs.call_rust(&mut r);
        assert_eq!(c, r, "row24 iter {i}: second call diverged\nC={c:?}\nR={r:?}");
    }
}

/// Row 25 — cross-field interaction: valid BS × valid SR × INDEPENDENT with an
/// over-large channel count corrupting the otherwise well-formed header.
#[test]
fn cfg_row_25_interaction_valid_header_corrupted_by_channels() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 25);
    for &bs in BLOCKSIZE_CASES.iter() {
        for &sr in SAMPLERATE_CASES.iter() {
            for _ in 0..8 {
                let mut inp = Input::default();
                inp.cur_blocksize = bs;
                inp.samplerate = sr;
                inp.bitdepth = *rng.pick(&BITDEPTH_CASES);
                inp.channel_mode = (rng.range_u32(0, 63) * 4) as u8;
                inp.channels = rng.range_u32(17, u32::MAX);
                inp.frame_header_initial = rng.next_u32();
                inp.padding = rng.next_u8();
                libs.assert_same(
                    inp,
                    &format!("row25 bs={bs} sr={sr} ch={}", inp.channels),
                );
            }
        }
    }
}

/// Row 26 — simultaneous boundary values on every axis.
#[test]
fn cfg_row_26_boundaries_on_every_axis() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 26);

    // Build boundary sets: for each exact case c -> {c-1, c, c+1}, plus extremes.
    let mut bs_set: Vec<u32> = vec![0, 1, 255, 256, 257, u32::MAX - 1, u32::MAX];
    for &c in BLOCKSIZE_CASES.iter() {
        bs_set.extend_from_slice(&[c.wrapping_sub(1), c, c.wrapping_add(1)]);
    }
    let mut sr_set: Vec<u32> = vec![
        0,
        1,
        65535,
        65536,
        65537,
        255_000,
        256_000,
        655_350,
        655_360,
        655_370,
        u32::MAX - 1,
        u32::MAX,
    ];
    for &c in SAMPLERATE_CASES.iter() {
        sr_set.extend_from_slice(&[c.wrapping_sub(1), c, c.wrapping_add(1)]);
    }
    let mut bd_set: Vec<u32> = vec![0, 1, u32::MAX - 1, u32::MAX];
    for &c in BITDEPTH_CASES.iter() {
        bd_set.extend_from_slice(&[c.wrapping_sub(1), c, c.wrapping_add(1)]);
    }
    let ch_set: Vec<u32> = vec![
        0,
        1,
        2,
        8,
        9,
        15,
        16,
        17,
        0x0FFF_FFFF,
        0x1000_0000,
        0xF000_0000,
        u32::MAX - 1,
        u32::MAX,
    ];
    let cm_set: [u8; 8] = [0, 1, 2, 3, 4, 5, 254, 255];

    // Exhaustive over the boundary cross-product of the two widest axes, with
    // the rest drawn from their boundary sets.
    for &bs in bs_set.iter() {
        for &sr in sr_set.iter() {
            let inp = Input {
                cur_blocksize: bs,
                samplerate: sr,
                bitdepth: *rng.pick(&bd_set),
                channels: *rng.pick(&ch_set),
                channel_mode: *rng.pick(&cm_set),
                frame_header_initial: if rng.bool() { 0 } else { rng.next_u32() },
                padding: rng.next_u8(),
            };
            libs.assert_same(inp, &format!("row26 bs={bs} sr={sr}"));
        }
    }
    // Plus a randomized pass over the full boundary cross-product.
    for i in 0..50_000 {
        let inp = Input {
            cur_blocksize: *rng.pick(&bs_set),
            samplerate: *rng.pick(&sr_set),
            bitdepth: *rng.pick(&bd_set),
            channels: *rng.pick(&ch_set),
            channel_mode: *rng.pick(&cm_set),
            frame_header_initial: if rng.bool() { 0 } else { rng.next_u32() },
            padding: rng.next_u8(),
        };
        libs.assert_same(inp, &format!("row26 random iter {i}"));
    }
}

/// Row 27 — batch/array shape: 1, 2 and 1024 contiguous `tflac` structs.
///
/// Catches struct-size / stride ABI mismatches that a single-element test
/// cannot: the C and Rust libs each walk the same flat byte buffer with a
/// 24-byte stride, and the whole buffer is compared afterwards.
#[test]
fn cfg_row_27_contiguous_array_stride() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 27);

    for &count in &[1usize, 2, 1024] {
        for round in 0..16 {
            // Allocate as u32 so the buffer is guaranteed 4-byte aligned, which
            // is `tflac`'s alignment requirement.
            let words = count * (TFLAC_SIZE / 4);
            let mut c_words = vec![0u32; words];
            {
                let c_buf: &mut [u8] = unsafe {
                    std::slice::from_raw_parts_mut(
                        c_words.as_mut_ptr() as *mut u8,
                        count * TFLAC_SIZE,
                    )
                };
                for i in 0..count {
                    let raw = rand_input(&mut rng).to_raw();
                    c_buf[i * TFLAC_SIZE..(i + 1) * TFLAC_SIZE].copy_from_slice(&raw.0);
                }
            }
            let mut r_words = c_words.clone();

            for i in 0..count {
                unsafe {
                    let cp = (c_words.as_mut_ptr() as *mut u8).add(i * TFLAC_SIZE);
                    let rp = (r_words.as_mut_ptr() as *mut u8).add(i * TFLAC_SIZE);
                    libs.call_c_raw(cp);
                    libs.call_rust_raw(rp);
                }
            }
            assert_eq!(
                c_words, r_words,
                "row27 count={count} round={round}: contiguous buffer diverged at word {:?}",
                (0..c_words.len()).find(|&i| c_words[i] != r_words[i])
            );
        }
    }
}

/// Heavy soak: 10 million randomized inputs across the full axis cross-product.
/// Run explicitly with `cargo test --release -- --ignored soak`.
#[test]
#[ignore = "long-running soak; run explicitly"]
fn soak_10m_random() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x5_0A_4B);
    for i in 0..10_000_000u64 {
        libs.assert_same(rand_input(&mut rng), &format!("soak iter {i}"));
    }
}
