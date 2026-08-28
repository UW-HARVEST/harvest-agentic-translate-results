//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`, each driving BOTH shared objects through
//! `libloading` with many randomized inputs (fixed seeds → reproducible).

mod common;

use common::*;

const SAMPLE_LEN: usize = 160; // update_md5 reads samples[0..136]

/* ================================================================== */
/* Rows 1–4: tflac_pack_u64le (lowest-level entry point)               */
/* ================================================================== */

#[test]
fn cfg01_pack_aligned() {
    let mut rng = Rng::new(0x0101);
    for i in 0..4096 {
        let arena = Arena::new(0x1000 + i as u64);
        let off = (rng.below(48) as usize) * 8; // 8-aligned
        diff_pack("aligned", &arena, off, rng.interesting_u64());
    }
}

#[test]
fn cfg02_pack_unaligned() {
    let mut rng = Rng::new(0x0202);
    for i in 0..2048 {
        let arena = Arena::new(0x2000 + i as u64);
        let off = (rng.below(48) as usize) * 8 + 1 + (rng.below(7) as usize);
        diff_pack("unaligned", &arena, off, rng.interesting_u64());
    }
}

#[test]
fn cfg03_pack_boundary_values() {
    let mut values: Vec<u64> = vec![
        0,
        1,
        u64::MAX,
        0x8000_0000_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
        0x0102_0304_0506_0708,
        0xFF00_FF00_FF00_FF00,
        0x00FF_00FF_00FF_00FF,
    ];
    for b in 0..64 {
        values.push(1u64 << b);
        values.push(!(1u64 << b));
    }
    for (i, v) in values.iter().enumerate() {
        for off in [0usize, 1, 7, 8, 15, 16, 63, 64, 71] {
            let arena = Arena::new(0x3000 + i as u64);
            diff_pack("boundary", &arena, off, *v);
        }
    }
}

#[test]
fn cfg04_pack_tail_offset() {
    // Largest offset the library itself ever uses: buffer[63] → stores 63..70,
    // i.e. arena bytes 79..86 when the buffer starts at 16.
    let mut rng = Rng::new(0x0404);
    for i in 0..1024 {
        let arena = Arena::new(0x4000 + i as u64);
        diff_pack("tail", &arena, OFF_BUFFER + 63, rng.interesting_u64());
        let arena2 = Arena::new(0x4800 + i as u64);
        diff_pack("tail-1", &arena2, OFF_BUFFER + 62, rng.interesting_u64());
    }
}

/* ================================================================== */
/* Rows 5–17: tflac_md5_addsample (mid-level entry point)               */
/* ================================================================== */

fn pattern_buffer(seed: u64) -> [u8; BUFFER_LEN] {
    let mut rng = Rng::new(seed);
    let mut b = [0u8; BUFFER_LEN];
    for (i, x) in b.iter_mut().enumerate() {
        *x = (i as u8) ^ (rng.next_u32() as u8);
    }
    b
}

#[test]
fn cfg05_addsample_pos0_bits64() {
    let mut rng = Rng::new(0x0505);
    for i in 0..2048 {
        let buf = pattern_buffer(0x5000 + i as u64);
        let arena = tflac_arena(0x5100 + i as u64, 0, rng.next_u64(), 0, 0, Some(&buf));
        diff_addsample("pos0/bits64", &arena, 64, rng.interesting_u64());
    }
}

#[test]
fn cfg06_addsample_exact_boundary() {
    // pos = 56, bits = 64 → pos becomes exactly 64 → pos %= 64 == 0 and the
    // spill loop body never runs.
    let mut rng = Rng::new(0x0606);
    for i in 0..1024 {
        let buf = pattern_buffer(0x6000 + i as u64);
        let arena = tflac_arena(0x6100 + i as u64, 56, rng.next_u64(), 0, 0, Some(&buf));
        diff_addsample("pos56 exact-boundary", &arena, 64, rng.interesting_u64());
    }
}

#[test]
fn cfg07_addsample_spill_partial() {
    let mut rng = Rng::new(0x0707);
    for pos in 57u32..=63 {
        for i in 0..256 {
            let buf = pattern_buffer(0x7000 + pos as u64 * 977 + i as u64);
            let arena = tflac_arena(0x7100 + i as u64, pos, rng.next_u64(), 0, 0, Some(&buf));
            diff_addsample(&format!("spill pos={pos}"), &arena, 64, rng.interesting_u64());
        }
    }
}

#[test]
fn cfg08_addsample_unaligned_pos() {
    let mut rng = Rng::new(0x0808);
    for pos in 1u32..=7 {
        for i in 0..256 {
            let buf = pattern_buffer(0x8000 + pos as u64 * 131 + i as u64);
            let arena = tflac_arena(0x8100 + i as u64, pos, rng.next_u64(), 0, 0, Some(&buf));
            diff_addsample(
                &format!("unaligned pos={pos}"),
                &arena,
                64,
                rng.interesting_u64(),
            );
        }
    }
}

#[test]
fn cfg09_addsample_pos63() {
    let mut rng = Rng::new(0x0909);
    for i in 0..1024 {
        let buf = pattern_buffer(0x9000 + i as u64);
        let arena = tflac_arena(0x9100 + i as u64, 63, rng.next_u64(), 0, 0, Some(&buf));
        diff_addsample("pos63", &arena, 64, rng.interesting_u64());
    }
}

#[test]
fn cfg10_addsample_bits0_sweep() {
    let mut rng = Rng::new(0x0a0a);
    for pos in 0u32..64 {
        for i in 0..16 {
            let buf = pattern_buffer(0xA000 + pos as u64 * 31 + i as u64);
            let arena = tflac_arena(0xA100 + i as u64, pos, rng.next_u64(), 0, 0, Some(&buf));
            diff_addsample(&format!("bits0 pos={pos}"), &arena, 0, rng.interesting_u64());
        }
    }
}

#[test]
fn cfg11_addsample_bit_width_sweep() {
    let mut rng = Rng::new(0x0b0b);
    for bits in [8u32, 16, 24, 32, 40, 48, 56, 64] {
        for pos in 0u32..64 {
            for i in 0..8 {
                let buf = pattern_buffer(0xB000 + bits as u64 * 7919 + pos as u64 * 31 + i as u64);
                let arena = tflac_arena(0xB100 + i as u64, pos, rng.next_u64(), 0, 0, Some(&buf));
                diff_addsample(
                    &format!("bits={bits} pos={pos}"),
                    &arena,
                    bits,
                    rng.interesting_u64(),
                );
            }
        }
    }
}

#[test]
fn cfg12_addsample_multi_block_cross() {
    let mut rng = Rng::new(0x0c0c);
    for bits in [512u32, 1024, 4096, 8 * 64, 8 * 65] {
        for pos in [0u32, 1, 7, 8, 56, 63] {
            for i in 0..64 {
                let buf = pattern_buffer(0xC000 + bits as u64 + pos as u64 * 13 + i as u64);
                let arena = tflac_arena(0xC100 + i as u64, pos, rng.next_u64(), 0, 0, Some(&buf));
                diff_addsample(
                    &format!("multi-block bits={bits} pos={pos}"),
                    &arena,
                    bits,
                    rng.interesting_u64(),
                );
            }
        }
    }
}

#[test]
fn cfg13_addsample_total_wrap() {
    let mut rng = Rng::new(0x0d0d);
    for total in [
        u64::MAX,
        u64::MAX - 1,
        u64::MAX - 63,
        u64::MAX - 64,
        0xFFFF_FFFF_FFFF_FF00,
    ] {
        for i in 0..256 {
            let buf = pattern_buffer(0xD000 + i as u64);
            let arena = tflac_arena(0xD100 + i as u64, rng.below(64), total, 0, 0, Some(&buf));
            diff_addsample(
                &format!("total-wrap total={total:#x}"),
                &arena,
                rng.next_u32() % 128,
                rng.interesting_u64(),
            );
        }
    }
}

#[test]
fn cfg14_addsample_spill_source_pattern() {
    // buffer[i] == i, so every spilled byte identifies its source index.
    let mut ident = [0u8; BUFFER_LEN];
    for (i, x) in ident.iter_mut().enumerate() {
        *x = i as u8;
    }
    for pos in 0u32..64 {
        for bits in [0u32, 8, 64, 72, 128] {
            let mut arena = Arena::zeroed();
            arena.set_pos(pos);
            arena.set_total(0);
            arena.set_buffer(&ident);
            diff_addsample(
                &format!("ident-pattern pos={pos} bits={bits}"),
                &arena,
                bits,
                0,
            );
            diff_addsample(
                &format!("ident-pattern pos={pos} bits={bits} val=max"),
                &arena,
                bits,
                u64::MAX,
            );
        }
    }
}

#[test]
fn cfg15_addsample_pos_ge_64() {
    let mut rng = Rng::new(0x0f0f);
    for pos in [64u32, 65, 71, 72, 127, 128, 129, 1000, 0xFFFF] {
        for i in 0..128 {
            let buf = pattern_buffer(0xF000 + pos as u64 + i as u64);
            let arena = tflac_arena(0xF100 + i as u64, pos, rng.next_u64(), 0, 0, Some(&buf));
            diff_addsample(
                &format!("pos>=64 pos={pos}"),
                &arena,
                64,
                rng.interesting_u64(),
            );
            diff_addsample(
                &format!("pos>=64 pos={pos} bits=8"),
                &arena,
                8,
                rng.interesting_u64(),
            );
        }
    }
}

#[test]
fn cfg16_addsample_val_boundaries() {
    let mut vals: Vec<u64> = vec![
        0,
        1,
        u64::MAX,
        0x8000_0000_0000_0000,
        0x00FF_00FF_00FF_00FF,
        0xFFFF_FFFF_0000_0000,
        0x0000_0000_FFFF_FFFF,
    ];
    for b in 0..64 {
        vals.push(1u64 << b);
    }
    for pos in [0u32, 7, 8, 55, 56, 63] {
        for (i, v) in vals.iter().enumerate() {
            let buf = pattern_buffer(0x1_0000 + i as u64 * 17 + pos as u64);
            let arena = tflac_arena(0x1_0100 + i as u64, pos, 12345, 0, 0, Some(&buf));
            diff_addsample(&format!("val-bound pos={pos} val={v:#x}"), &arena, 64, *v);
        }
    }
}

#[test]
fn cfg17_addsample_sequence_stateful() {
    // Drive the state machine the way a real consumer does: one struct, many
    // consecutive calls, compared after EVERY call so divergence is localised.
    let api = both();
    let mut rng = Rng::new(0x1717);
    let start = tflac_arena(0x1717_0001, 0, 0, 0, 0, Some(&pattern_buffer(0x1717_0002)));
    let mut ca = start.clone_arena();
    let mut ra = start.clone_arena();
    for step in 0..512 {
        let bits = match rng.below(6) {
            0 => 64,
            1 => 8 * (1 + rng.below(8)),
            2 => 0,
            3 => rng.below(200),
            4 => 64,
            _ => 8 * (1 + rng.below(20)),
        };
        let val = rng.interesting_u64();
        unsafe {
            (api.c.addsample)(ca.as_ptr(), bits, val);
            (api.rust.addsample)(ra.as_ptr(), bits, val);
        }
        assert_eq!(
            ca.bytes(),
            ra.bytes(),
            "stateful addsample divergence at step {step} (bits={bits} val={val:#x}); \
             C pos={} total={} / RS pos={} total={}",
            ca.pos(),
            ca.total(),
            ra.pos(),
            ra.total()
        );
    }
}

/* ================================================================== */
/* Rows 18–27: update_md5 (top-level entry point)                       */
/* ================================================================== */

#[test]
fn cfg18_update_typical() {
    let mut rng = Rng::new(0x1818);
    for i in 0..1024 {
        let buf = pattern_buffer(0x1800 + i as u64);
        let arena = tflac_arena(0x1900 + i as u64, 0, rng.next_u64(), 4096, 2, Some(&buf));
        let samples = random_samples(&mut rng, SAMPLE_LEN);
        diff_update("typical 4096x2", &arena, &samples);
    }
}

#[test]
fn cfg19_update_blocksize_channel_matrix() {
    let mut rng = Rng::new(0x1919);
    for channels in 1u32..=8 {
        for bs in [1u32, 16, 576, 4096, 65535] {
            for i in 0..24 {
                let buf = pattern_buffer(0x1A00 + channels as u64 * 101 + bs as u64 + i as u64);
                let arena = tflac_arena(
                    0x1B00 + i as u64,
                    rng.below(64),
                    rng.next_u64(),
                    bs,
                    channels,
                    Some(&buf),
                );
                let samples = random_samples(&mut rng, SAMPLE_LEN);
                diff_update(&format!("matrix bs={bs} ch={channels}"), &arena, &samples);
            }
        }
    }
}

#[test]
fn cfg20_update_product_exact_40() {
    let mut rng = Rng::new(0x2020);
    for (bs, ch) in [(8u32, 5u32), (40, 1), (20, 2), (10, 4), (5, 8), (1, 40)] {
        for i in 0..64 {
            let buf = pattern_buffer(0x2000 + bs as u64 * 7 + i as u64);
            let arena = tflac_arena(0x2100 + i as u64, rng.below(64), 0, bs, ch, Some(&buf));
            let samples = random_samples(&mut rng, SAMPLE_LEN);
            // b == 40 → five subtractions of 8 land exactly on 0.
            diff_update(&format!("exact40 {bs}x{ch}"), &arena, &samples);
        }
    }
}

#[test]
fn cfg21_update_product_underflow() {
    let mut rng = Rng::new(0x2121);
    for (bs, ch) in [
        (0u32, 0u32),
        (0, 7),
        (7, 0),
        (1, 1),
        (8, 1),
        (1, 8),
        (39, 1),
        (13, 3),
        (3, 13),
        (2, 4),
    ] {
        for i in 0..64 {
            let buf = pattern_buffer(0x2200 + bs as u64 * 13 + ch as u64 + i as u64);
            let arena = tflac_arena(0x2300 + i as u64, rng.below(64), rng.next_u64(), bs, ch, Some(&buf));
            let samples = random_samples(&mut rng, SAMPLE_LEN);
            diff_update(&format!("underflow {bs}x{ch}"), &arena, &samples);
        }
    }
}

#[test]
fn cfg22_update_product_overflow() {
    let mut rng = Rng::new(0x2222);
    for (bs, ch) in [
        (0x1000_0000u32, 0x11u32),
        (0xFFFF_FFFF, 3),
        (65537, 65537),
        (0x8000_0000, 2),
        (0x8000_0000, 3),
        (0xFFFF_FFFF, 0xFFFF_FFFF),
        (123_456_789, 987_654_321),
    ] {
        for i in 0..64 {
            let buf = pattern_buffer(0x2400 + i as u64);
            let arena = tflac_arena(0x2500 + i as u64, rng.below(64), rng.next_u64(), bs, ch, Some(&buf));
            let samples = random_samples(&mut rng, SAMPLE_LEN);
            diff_update(&format!("overflow {bs}x{ch}"), &arena, &samples);
        }
    }
}

#[test]
fn cfg23_update_sample_value_shapes() {
    let shapes: Vec<(&str, Box<dyn Fn(usize) -> i32>)> = vec![
        ("all-zero", Box::new(|_| 0)),
        ("all-minus-one", Box::new(|_| -1)),
        ("i32-min", Box::new(|_| i32::MIN)),
        ("i32-max", Box::new(|_| i32::MAX)),
        ("low-byte-00", Box::new(|i| ((i as i32) << 8) & !0xFF)),
        ("low-byte-ff", Box::new(|i| ((i as i32) << 8) | 0xFF)),
        ("ramp", Box::new(|i| i as i32)),
        ("neg-ramp", Box::new(|i| -(i as i32))),
        (
            "alt-sign",
            Box::new(|i| if i % 2 == 0 { i32::MIN + i as i32 } else { i32::MAX - i as i32 }),
        ),
        ("byte-ramp-shifted", Box::new(|i| (i as i32).wrapping_mul(0x0101_0101))),
    ];
    let mut rng = Rng::new(0x2323);
    for (name, f) in shapes {
        let samples: Vec<i32> = (0..SAMPLE_LEN).map(|i| f(i)).collect();
        for pos in [0u32, 1, 8, 56, 63, 64, 200] {
            let buf = pattern_buffer(0x2600 + pos as u64);
            let arena = tflac_arena(
                0x2700 + pos as u64,
                pos,
                rng.next_u64(),
                rng.next_u32(),
                rng.below(9),
                Some(&buf),
            );
            diff_update(&format!("shape={name} pos={pos}"), &arena, &samples);
        }
    }
}

#[test]
fn cfg24_update_stride_skip() {
    // Only samples[0..8], [32..40], [64..72], [96..104], [128..136] are read.
    // Fill the *skipped* ranges with loud sentinels; then randomize them and
    // require the result to be unchanged in BOTH libraries (and equal).
    let read_idx: Vec<usize> = (0..5).flat_map(|it| (0..8).map(move |k| it * 32 + k)).collect();
    let mut rng = Rng::new(0x2424);
    for trial in 0..256 {
        let mut samples = vec![0i32; SAMPLE_LEN];
        for &i in &read_idx {
            samples[i] = rng.interesting_i32();
        }
        // First run: sentinel 0x5A in every skipped slot.
        let mut a = samples.clone();
        for i in 0..SAMPLE_LEN {
            if !read_idx.contains(&i) {
                a[i] = 0x5A5A_5A5A;
            }
        }
        // Second run: random garbage in every skipped slot.
        let mut b = samples.clone();
        for i in 0..SAMPLE_LEN {
            if !read_idx.contains(&i) {
                b[i] = rng.next_i32();
            }
        }
        let buf = pattern_buffer(0x2800 + trial as u64);
        let arena = tflac_arena(
            0x2900 + trial as u64,
            rng.below(70),
            rng.next_u64(),
            rng.next_u32(),
            rng.below(9),
            Some(&buf),
        );
        diff_update("stride sentinel", &arena, &a);
        diff_update("stride garbage", &arena, &b);

        // Cross-check: the skipped slots really are ignored by BOTH libs.
        let api = both();
        let mut aa = arena.clone_arena();
        let mut bb = arena.clone_arena();
        let (r1, r2) = unsafe {
            (
                (api.c.update_md5)(aa.as_ptr(), a.as_ptr()),
                (api.rust.update_md5)(bb.as_ptr(), b.as_ptr()),
            )
        };
        assert_eq!(r1, r2, "stride: C(sentinel) vs RS(garbage) return differ");
        assert_eq!(
            aa.bytes(),
            bb.bytes(),
            "stride: skipped samples must not affect the result"
        );
    }
}

#[test]
fn cfg25_update_pos_sweep() {
    let mut rng = Rng::new(0x2525);
    for pos in 0u32..64 {
        for i in 0..16 {
            let buf = pattern_buffer(0x2A00 + pos as u64 * 61 + i as u64);
            let arena = tflac_arena(
                0x2B00 + i as u64,
                pos,
                rng.next_u64(),
                rng.next_u32(),
                1 + rng.below(8),
                Some(&buf),
            );
            let samples = random_samples(&mut rng, SAMPLE_LEN);
            diff_update(&format!("pos-sweep pos={pos}"), &arena, &samples);
        }
    }
}

#[test]
fn cfg26_update_pos_ge64_total_wrap() {
    let mut rng = Rng::new(0x2626);
    for pos in [64u32, 65, 71, 72, 100, 127, 128, 255, 1000] {
        for total in [u64::MAX, u64::MAX - 100, 0xFFFF_FFFF_FFFF_FF00] {
            for i in 0..16 {
                let buf = pattern_buffer(0x2C00 + pos as u64 + i as u64);
                let arena = tflac_arena(
                    0x2D00 + i as u64,
                    pos,
                    total,
                    rng.next_u32(),
                    rng.below(9),
                    Some(&buf),
                );
                let samples = random_samples(&mut rng, SAMPLE_LEN);
                diff_update(&format!("pos={pos} total={total:#x}"), &arena, &samples);
            }
        }
    }
}

#[test]
fn cfg27_update_sequence_stateful() {
    let api = both();
    let mut rng = Rng::new(0x2727);
    let mut start = tflac_arena(0x2727_0001, 0, 0, 4096, 2, Some(&pattern_buffer(0x2727_2)));
    start.set_cur_blocksize(4096);
    start.set_channels(2);
    let mut ca = start.clone_arena();
    let mut ra = start.clone_arena();
    for round in 0..128 {
        let samples = random_samples(&mut rng, SAMPLE_LEN);
        let (rc, rr) = unsafe {
            (
                (api.c.update_md5)(ca.as_ptr(), samples.as_ptr()),
                (api.rust.update_md5)(ra.as_ptr(), samples.as_ptr()),
            )
        };
        assert_eq!(rc, rr, "stateful update_md5 return divergence at round {round}");
        assert_eq!(
            ca.bytes(),
            ra.bytes(),
            "stateful update_md5 memory divergence at round {round}: \
             C pos={} total={} / RS pos={} total={}",
            ca.pos(),
            ca.total(),
            ra.pos(),
            ra.total()
        );
    }
}

/* ================================================================== */
/* Row 28: composed pipeline over all three entry points               */
/* ================================================================== */

#[test]
fn cfg28_mixed_pipeline() {
    let api = both();
    let mut rng = Rng::new(0x2828);
    let start = tflac_arena(0x2828_0001, 0, 0, 576, 2, Some(&pattern_buffer(0x2828_0002)));
    let mut ca = start.clone_arena();
    let mut ra = start.clone_arena();
    for step in 0..512 {
        match rng.below(3) {
            0 => {
                let off = rng.below((ARENA_BYTES - 8) as u32) as usize;
                let n = rng.interesting_u64();
                unsafe {
                    (api.c.pack_u64le)(ca.as_ptr().add(off), n);
                    (api.rust.pack_u64le)(ra.as_ptr().add(off), n);
                }
            }
            1 => {
                let bits = if rng.below(2) == 0 { 64 } else { rng.below(300) };
                let val = rng.interesting_u64();
                unsafe {
                    (api.c.addsample)(ca.as_ptr(), bits, val);
                    (api.rust.addsample)(ra.as_ptr(), bits, val);
                }
            }
            _ => {
                let samples = random_samples(&mut rng, SAMPLE_LEN);
                let (rc, rr) = unsafe {
                    (
                        (api.c.update_md5)(ca.as_ptr(), samples.as_ptr()),
                        (api.rust.update_md5)(ra.as_ptr(), samples.as_ptr()),
                    )
                };
                assert_eq!(rc, rr, "mixed pipeline: update_md5 return differs at step {step}");
            }
        }
        assert_eq!(
            ca.bytes(),
            ra.bytes(),
            "mixed pipeline divergence at step {step}"
        );
    }
}
