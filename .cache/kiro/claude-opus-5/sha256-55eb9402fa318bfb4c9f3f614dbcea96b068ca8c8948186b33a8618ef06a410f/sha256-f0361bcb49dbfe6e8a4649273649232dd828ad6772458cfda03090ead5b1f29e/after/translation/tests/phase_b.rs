//! Phase B — valid-path differential tests. One test per row of `CONFIGS.md`.
//!
//! Every call is made through the `.so` exports of BOTH libraries; every row is
//! driven with many randomized inputs from a fixed seed.

mod common;
use common::*;

const N: usize = 400; // randomized cases per row

// ===========================================================================
// Row 1 — pack_u64le, standalone aligned destination, randomized n
// ===========================================================================
#[test]
fn row01_pack_aligned_random() {
    let mut rng = Rng::new(0x0101_0101);
    for i in 0..N {
        let n = rng.next_u64();
        let st = StateBuilder::random(&mut rng).build();
        let sm = vec![0u8; SAMPLES_LEN];
        run_diff(&format!("row01/{i} n={n:#018x}"), &st, &sm, &|api, s, _| unsafe {
            (api.pack_u64le)(s, n);
        });
    }
}

// ===========================================================================
// Row 2 — pack_u64le, boundary n values
// ===========================================================================
#[test]
fn row02_pack_boundary_values() {
    let mut rng = Rng::new(0x0202_0202);
    let mut vals: Vec<u64> = vec![
        0,
        1,
        0xFF,
        0x100,
        0x00FF_00FF_00FF_00FF,
        0xFF00_FF00_FF00_FF00,
        u64::MAX,
        u64::MAX - 1,
        0x8000_0000_0000_0000,
        0x7FFF_FFFF_FFFF_FFFF,
    ];
    for bit in 0..64 {
        vals.push(1u64 << bit);
        vals.push(!(1u64 << bit));
    }
    for (i, n) in vals.iter().copied().enumerate() {
        let st = StateBuilder::random(&mut rng).build();
        let sm = vec![0u8; SAMPLES_LEN];
        run_diff(&format!("row02/{i} n={n:#018x}"), &st, &sm, &|api, s, _| unsafe {
            (api.pack_u64le)(s, n);
        });
    }
}

// ===========================================================================
// Row 3 — pack_u64le into an UNALIGNED destination
// ===========================================================================
#[test]
fn row03_pack_unaligned() {
    let mut rng = Rng::new(0x0303_0303);
    for i in 0..N {
        let off = 1 + (i % 7); // 1..7, never 8-aligned
        let n = rng.next_u64();
        let st = StateBuilder::random(&mut rng).build();
        let sm = vec![0u8; SAMPLES_LEN];
        run_diff(
            &format!("row03/{i} off={off} n={n:#018x}"),
            &st,
            &sm,
            &|api, s, _| unsafe { (api.pack_u64le)(s.add(off), n) },
        );
    }
}

// ===========================================================================
// Row 4 — pack_u64le at every offset 0..=64 inside tflac_md5.buffer
// ===========================================================================
#[test]
fn row04_pack_every_buffer_offset() {
    let mut rng = Rng::new(0x0404_0404);
    for rep in 0..8 {
        for off in 0..=64usize {
            let n = rng.next_u64();
            let st = StateBuilder::random(&mut rng).build();
            let sm = vec![0u8; SAMPLES_LEN];
            run_diff(
                &format!("row04/rep{rep}/buffer[{off}] n={n:#018x}"),
                &st,
                &sm,
                &|api, s, _| unsafe { (api.pack_u64le)(s.add(OFF_BUFFER + off), n) },
            );
        }
    }
}

// ===========================================================================
// Row 5 — addsample, pos = 0, bits = 64: carry-down NOT taken
// ===========================================================================
#[test]
fn row05_addsample_pos0_bits64() {
    let mut rng = Rng::new(0x0505_0505);
    for i in 0..N {
        let val = rng.next_u64();
        let total = rng.next_u64();
        let st = StateBuilder::random(&mut rng).pos(0).total(total).build();
        let sm = vec![0u8; SAMPLES_LEN];
        run_diff(&format!("row05/{i} val={val:#018x}"), &st, &sm, &|api, s, _| unsafe {
            (api.md5_addsample)(s, 64, val);
        });
    }
}

// ===========================================================================
// Row 6 — addsample, pos in 1..=55, bits = 64: write stays in first 64 bytes
// ===========================================================================
#[test]
fn row06_addsample_pos_1_55() {
    let mut rng = Rng::new(0x0606_0606);
    for pos in 1..=55u32 {
        for _ in 0..8 {
            let val = rng.next_u64();
            let st = StateBuilder::random(&mut rng).pos(pos).build();
            let sm = vec![0u8; SAMPLES_LEN];
            run_diff(
                &format!("row06/pos={pos} val={val:#018x}"),
                &st,
                &sm,
                &|api, s, _| unsafe { (api.md5_addsample)(s, 64, val) },
            );
        }
    }
}

// ===========================================================================
// Row 7 — addsample, pos in 56..=63, bits = 64: write spills into buffer[64..72]
//         AND the carry-down runs with bytes = 0..7 (in-bounds source)
// ===========================================================================
#[test]
fn row07_addsample_pos_56_63() {
    let mut rng = Rng::new(0x0707_0707);
    for pos in 56..=63u32 {
        for _ in 0..40 {
            let val = rng.next_u64();
            let st = StateBuilder::random(&mut rng).pos(pos).build();
            let sm = vec![0u8; SAMPLES_LEN];
            run_diff(
                &format!("row07/pos={pos} val={val:#018x}"),
                &st,
                &sm,
                &|api, s, _| unsafe { (api.md5_addsample)(s, 64, val) },
            );
        }
    }
}

// ===========================================================================
// Row 8 — addsample, bits = 512 (bytes = 64) with pos 0..=63: carry-down with
//         bytes up to 63, i.e. OUT-OF-BOUNDS source reads up to buffer[126]
// ===========================================================================
#[test]
fn row08_addsample_bits512_oob_carrydown() {
    let mut rng = Rng::new(0x0808_0808);
    for pos in 0..=63u32 {
        for _ in 0..8 {
            let val = rng.next_u64();
            // Fully randomized backing region so the OOB source bytes are
            // non-trivial but identical for both libraries.
            let st = StateBuilder::random(&mut rng).pos(pos).build();
            let sm = vec![0u8; SAMPLES_LEN];
            run_diff(
                &format!("row08/pos={pos} bits=512 val={val:#018x}"),
                &st,
                &sm,
                &|api, s, _| unsafe { (api.md5_addsample)(s, 512, val) },
            );
        }
    }
}

// ===========================================================================
// Row 9 — addsample, bits = 0
// ===========================================================================
#[test]
fn row09_addsample_bits0() {
    let mut rng = Rng::new(0x0909_0909);
    for pos in [0u32, 1, 32, 55, 56, 63, 64, 65, 100, 127, 1000] {
        for _ in 0..40 {
            let val = rng.next_u64();
            let total = rng.next_u64();
            let st = StateBuilder::random(&mut rng).pos(pos).total(total).build();
            let sm = vec![0u8; SAMPLES_LEN];
            run_diff(
                &format!("row09/pos={pos} bits=0 val={val:#018x}"),
                &st,
                &sm,
                &|api, s, _| unsafe { (api.md5_addsample)(s, 0, val) },
            );
        }
    }
}

// ===========================================================================
// Row 10 — addsample, bits NOT a multiple of 8 (truncating bits/8)
// ===========================================================================
#[test]
fn row10_addsample_bits_not_multiple_of_8() {
    let mut rng = Rng::new(0x0A0A_0A0A);
    let bitsv = [1u32, 2, 3, 4, 5, 6, 7, 9, 15, 63, 65, 127, 4095, 4097];
    let posv = [0u32, 1, 31, 55, 56, 63, 64, 65, 65535];
    for &bits in &bitsv {
        for &pos in &posv {
            for _ in 0..6 {
                let val = rng.next_u64();
                let total = rng.next_u64();
                let st = StateBuilder::random(&mut rng).pos(pos).total(total).build();
                let sm = vec![0u8; SAMPLES_LEN];
                run_diff(
                    &format!("row10/bits={bits} pos={pos} val={val:#018x}"),
                    &st,
                    &sm,
                    &|api, s, _| unsafe { (api.md5_addsample)(s, bits, val) },
                );
            }
        }
    }
}

// ===========================================================================
// Row 11 — addsample, pos >= 64 unreduced
// ===========================================================================
#[test]
fn row11_addsample_pos_unreduced() {
    let mut rng = Rng::new(0x0B0B_0B0B);
    let posv = [64u32, 65, 71, 72, 100, 127, 128, 1000, 0xFFFF, 0x7FFF_FFFF, u32::MAX, u32::MAX - 1];
    for &pos in &posv {
        for &bits in &[0u32, 8, 64, 512, 65] {
            for _ in 0..12 {
                let val = rng.next_u64();
                let total = rng.next_u64();
                let st = StateBuilder::random(&mut rng).pos(pos).total(total).build();
                let sm = vec![0u8; SAMPLES_LEN];
                run_diff(
                    &format!("row11/pos={pos} bits={bits} val={val:#018x}"),
                    &st,
                    &sm,
                    &|api, s, _| unsafe { (api.md5_addsample)(s, bits, val) },
                );
            }
        }
    }
}

// ===========================================================================
// Row 12 — addsample, pos + bits/8 overflows u32
// ===========================================================================
#[test]
fn row12_addsample_pos_add_overflow() {
    let mut rng = Rng::new(0x0C0C_0C0C);
    for i in 0..N {
        let pos = u32::MAX - rng.below(0x2000_0000);
        let bits = rng.pick(&[u32::MAX, u32::MAX - 1, 0xFFFF_FFF8, 64, 512, 0xE000_0000]);
        let val = rng.next_u64();
        let st = StateBuilder::random(&mut rng).pos(pos).build();
        let sm = vec![0u8; SAMPLES_LEN];
        run_diff(
            &format!("row12/{i} pos={pos} bits={bits}"),
            &st,
            &sm,
            &|api, s, _| unsafe { (api.md5_addsample)(s, bits, val) },
        );
    }
}

// ===========================================================================
// Row 13 — addsample, total near u64::MAX so total += bits wraps
// ===========================================================================
#[test]
fn row13_addsample_total_wrap() {
    let mut rng = Rng::new(0x0D0D_0D0D);
    for i in 0..N {
        let total = u64::MAX - (rng.next_u64() % 0x100);
        let bits = rng.next_u32();
        let val = rng.next_u64();
        let pos = rng.below(128);
        let st = StateBuilder::random(&mut rng).pos(pos).total(total).build();
        let sm = vec![0u8; SAMPLES_LEN];
        run_diff(
            &format!("row13/{i} total={total:#018x} bits={bits}"),
            &st,
            &sm,
            &|api, s, _| unsafe { (api.md5_addsample)(s, bits, val) },
        );
    }
}

// ===========================================================================
// Row 14 — addsample, 16 chained calls threading one context (composed ring)
// ===========================================================================
#[test]
fn row14_addsample_chained() {
    let mut rng = Rng::new(0x0E0E_0E0E);
    for i in 0..N {
        let mut script: Vec<(u32, u64)> = Vec::new();
        for _ in 0..16 {
            let bits = rng.pick(&[0u32, 1, 7, 8, 16, 24, 32, 64, 65, 128, 504, 512, 520, 4096]);
            script.push((bits, rng.next_u64()));
        }
        let pos = rng.below(200);
        let st = StateBuilder::random(&mut rng).pos(pos).build();
        let sm = vec![0u8; SAMPLES_LEN];
        run_diff(&format!("row14/{i} pos={pos}"), &st, &sm, &|api, s, _| unsafe {
            for &(bits, val) in &script {
                (api.md5_addsample)(s, bits, val);
            }
        });
    }
}

// ===========================================================================
// Row 15 — update_md5, product > 40
// ===========================================================================
#[test]
fn row15_update_product_gt_40() {
    let mut rng = Rng::new(0x0F0F_0F0F);
    for i in 0..N {
        let cb = rng.range(1, 4096);
        let ch = {
            // keep product in 41..=100000
            let mut ch = rng.range(1, 8);
            while (cb as u64) * (ch as u64) <= 40 {
                ch += 1;
            }
            ch
        };
        let st = StateBuilder::random(&mut rng)
            .pos(0)
            .total(0)
            .cur_blocksize(cb)
            .channels(ch)
            .build();
        let sm = random_samples(&mut rng);
        run_diff(
            &format!("row15/{i} cb={cb} ch={ch}"),
            &st,
            &sm,
            &|api, s, sam| unsafe { (api.update_md5)(s, sam) },
        );
    }
}

// ===========================================================================
// Row 16 — update_md5, product == 40 exactly (all factor pairs)
// ===========================================================================
#[test]
fn row16_update_product_eq_40() {
    let mut rng = Rng::new(0x1010_1010);
    for (cb, ch) in [(1u32, 40u32), (2, 20), (4, 10), (5, 8), (8, 5), (10, 4), (20, 2), (40, 1)] {
        for _ in 0..40 {
            let st = StateBuilder::random(&mut rng)
                .pos(0)
                .cur_blocksize(cb)
                .channels(ch)
                .build();
            let sm = random_samples(&mut rng);
            run_diff(
                &format!("row16/cb={cb} ch={ch}"),
                &st,
                &sm,
                &|api, s, sam| unsafe {
                    let r = (api.update_md5)(s, sam);
                    assert_eq!(r, 0, "product==40 must return 0, got {r} from {}", api.which);
                    r
                },
            );
        }
    }
}

// ===========================================================================
// Row 17 — update_md5, product < 40 ⇒ b underflows
// ===========================================================================
#[test]
fn row17_update_product_lt_40_underflow() {
    let mut rng = Rng::new(0x1111_1111);
    let mut pairs: Vec<(u32, u32)> = vec![(0, 0), (0, 7), (7, 0), (1, 1), (3, 13), (39, 1), (1, 39)];
    for p in 0..40u32 {
        pairs.push((p, 1));
        pairs.push((1, p));
    }
    for (cb, ch) in pairs {
        for _ in 0..10 {
            let st = StateBuilder::random(&mut rng)
                .pos(0)
                .cur_blocksize(cb)
                .channels(ch)
                .build();
            let sm = random_samples(&mut rng);
            let expect = cb.wrapping_mul(ch).wrapping_sub(40);
            run_diff(
                &format!("row17/cb={cb} ch={ch}"),
                &st,
                &sm,
                &|api, s, sam| unsafe {
                    let r = (api.update_md5)(s, sam);
                    assert_eq!(r, expect, "{} returned {r:#x}, want {expect:#x}", api.which);
                    r
                },
            );
        }
    }
}

// ===========================================================================
// Row 18 — update_md5, product overflows u32
// ===========================================================================
#[test]
fn row18_update_product_overflow() {
    let mut rng = Rng::new(0x1212_1212);
    let mut pairs: Vec<(u32, u32)> = vec![
        (0x10000, 0x10000),
        (u32::MAX, 3),
        (3, u32::MAX),
        (u32::MAX, u32::MAX),
        (0x8000_0000, 2),
        (0xFFFF, 0x1_0001),
    ];
    for _ in 0..80 {
        pairs.push((rng.next_u32() | 0x8000_0000, rng.next_u32() | 0x8000_0000));
    }
    for (cb, ch) in pairs {
        let st = StateBuilder::random(&mut rng)
            .pos(0)
            .cur_blocksize(cb)
            .channels(ch)
            .build();
        let sm = random_samples(&mut rng);
        run_diff(
            &format!("row18/cb={cb} ch={ch}"),
            &st,
            &sm,
            &|api, s, sam| unsafe { (api.update_md5)(s, sam) },
        );
    }
}

// ===========================================================================
// Row 19 — update_md5, extreme sample values (sign extension then mask)
// ===========================================================================
#[test]
fn row19_update_extreme_samples() {
    let mut rng = Rng::new(0x1313_1313);
    let fills: [i32; 6] = [0, -1, i32::MIN, i32::MAX, 0x0000_00FF, -256];
    for &f in &fills {
        for _ in 0..10 {
            let xs = vec![f; SAMPLES_ELEMS];
            let sm = samples_from_i32(&xs);
            let st = StateBuilder::random(&mut rng)
                .pos(0)
                .cur_blocksize(1024)
                .channels(2)
                .build();
            run_diff(
                &format!("row19/fill={f}"),
                &st,
                &sm,
                &|api, s, sam| unsafe { (api.update_md5)(s, sam) },
            );
        }
    }
    // Randomized mix of the extremes plus random values.
    for i in 0..100 {
        let xs: Vec<i32> = (0..SAMPLES_ELEMS)
            .map(|_| if rng.next_u32() & 1 == 0 { rng.pick(&fills) } else { rng.next_i32() })
            .collect();
        let sm = samples_from_i32(&xs);
        let st = StateBuilder::random(&mut rng)
            .pos(0)
            .cur_blocksize(rng.range(1, 1000))
            .channels(rng.range(1, 8))
            .build();
        run_diff(&format!("row19/mix{i}"), &st, &sm, &|api, s, sam| unsafe {
            (api.update_md5)(s, sam)
        });
    }
}

// ===========================================================================
// Row 20 — update_md5, low byte zero / masked-off bits must not matter
// ===========================================================================
#[test]
fn row20_update_masked_off_bits_irrelevant() {
    let mut rng = Rng::new(0x1414_1414);
    for i in 0..N {
        // Base: random low bytes only.
        let low: Vec<i32> = (0..SAMPLES_ELEMS).map(|_| (rng.next_u32() & 0xFF) as i32).collect();
        // Variant: same low bytes, randomized upper 24 bits.
        let hi: Vec<i32> = low
            .iter()
            .map(|&x| (x as u32 | (rng.next_u32() & !0xFF)) as i32)
            .collect();

        let st = StateBuilder::random(&mut rng)
            .pos(0)
            .cur_blocksize(rng.range(1, 4096))
            .channels(rng.range(1, 8))
            .build();

        let sm_low = samples_from_i32(&low);
        let sm_hi = samples_from_i32(&hi);

        // Both libraries must agree on each variant...
        run_diff(&format!("row20/{i}/low"), &st, &sm_low, &|api, s, sam| unsafe {
            (api.update_md5)(s, sam)
        });
        run_diff(&format!("row20/{i}/hi"), &st, &sm_hi, &|api, s, sam| unsafe {
            (api.update_md5)(s, sam)
        });

        // ...and the masked-off bits must not change the C result at all.
        let (c, _) = apis();
        let mut a = Region::from_bytes(&st);
        let mut sa = Region::from_bytes(&sm_low);
        let ra = unsafe { (c.update_md5)(a.as_mut_ptr(), sa.as_mut_ptr() as *const i32) };
        let mut b = Region::from_bytes(&st);
        let mut sb = Region::from_bytes(&sm_hi);
        let rb = unsafe { (c.update_md5)(b.as_mut_ptr(), sb.as_mut_ptr() as *const i32) };
        assert_eq!(ra, rb, "row20/{i}: masking invariant broken in C");
        assert_eq!(a.bytes(), b.bytes(), "row20/{i}: masking invariant broken in C state");
    }
}

// ===========================================================================
// Row 21 — update_md5, the +32-element stride: only the 5 read windows matter
// ===========================================================================
#[test]
fn row21_update_stride_windows() {
    let mut rng = Rng::new(0x1515_1515);
    // Element indices the C loop actually reads.
    let read: Vec<usize> = (0..5).flat_map(|k| k * 32..k * 32 + 8).collect();
    for i in 0..N {
        let base: Vec<i32> = (0..SAMPLES_ELEMS).map(|_| rng.next_i32()).collect();
        // Variant differs ONLY outside the read windows.
        let mut variant = base.clone();
        for (j, v) in variant.iter_mut().enumerate() {
            if !read.contains(&j) {
                *v = rng.next_i32();
            }
        }

        let st = StateBuilder::random(&mut rng)
            .pos(0)
            .cur_blocksize(rng.range(1, 4096))
            .channels(rng.range(1, 8))
            .build();
        let sm_base = samples_from_i32(&base);
        let sm_var = samples_from_i32(&variant);

        run_diff(&format!("row21/{i}/base"), &st, &sm_base, &|api, s, sam| unsafe {
            (api.update_md5)(s, sam)
        });
        run_diff(&format!("row21/{i}/variant"), &st, &sm_var, &|api, s, sam| unsafe {
            (api.update_md5)(s, sam)
        });

        // The stride invariant itself, measured on the C library and required of
        // the Rust one: changing only the skipped gaps changes nothing.
        for api in [&apis().0, &apis().1] {
            let mut a = Region::from_bytes(&st);
            let mut sa = Region::from_bytes(&sm_base);
            let ra = unsafe { (api.update_md5)(a.as_mut_ptr(), sa.as_mut_ptr() as *const i32) };
            let mut b = Region::from_bytes(&st);
            let mut sb = Region::from_bytes(&sm_var);
            let rb = unsafe { (api.update_md5)(b.as_mut_ptr(), sb.as_mut_ptr() as *const i32) };
            assert_eq!(ra, rb, "row21/{i}: {} stride window mismatch", api.which);
            assert_eq!(a.bytes(), b.bytes(), "row21/{i}: {} stride state mismatch", api.which);
        }
    }
}

// ===========================================================================
// Row 22 — update_md5 with md5_ctx.pos seeded 0..63 (carry-down inside loop)
// ===========================================================================
#[test]
fn row22_update_seeded_pos_0_63() {
    let mut rng = Rng::new(0x1616_1616);
    for pos in 0..=63u32 {
        for _ in 0..8 {
            let st = StateBuilder::random(&mut rng)
                .pos(pos)
                .cur_blocksize(rng.range(1, 4096))
                .channels(rng.range(1, 8))
                .build();
            let sm = random_samples(&mut rng);
            run_diff(
                &format!("row22/pos={pos}"),
                &st,
                &sm,
                &|api, s, sam| unsafe { (api.update_md5)(s, sam) },
            );
        }
    }
}

// ===========================================================================
// Row 23 — update_md5 with pos >= 64 and total near u64::MAX
// ===========================================================================
#[test]
fn row23_update_seeded_pos_ge_64_total_wrap() {
    let mut rng = Rng::new(0x1717_1717);
    for &pos in &[64u32, 65, 100, 127, 1000, 0xFFFF, 0xFFFF_FFF8, u32::MAX] {
        for _ in 0..40 {
            let total = u64::MAX - (rng.next_u64() % 0x200);
            let st = StateBuilder::random(&mut rng)
                .pos(pos)
                .total(total)
                .cur_blocksize(rng.next_u32())
                .channels(rng.range(1, 8))
                .build();
            let sm = random_samples(&mut rng);
            run_diff(
                &format!("row23/pos={pos} total={total:#018x}"),
                &st,
                &sm,
                &|api, s, sam| unsafe { (api.update_md5)(s, sam) },
            );
        }
    }
}

// ===========================================================================
// Row 24 — update_md5 called 8 times in a row, advancing samples by 136
// ===========================================================================
#[test]
fn row24_update_chained() {
    let mut rng = Rng::new(0x1818_1818);
    for i in 0..N {
        let st = StateBuilder::random(&mut rng)
            .pos(rng.below(128))
            .total(rng.next_u64())
            .cur_blocksize(rng.range(1, 100000))
            .channels(rng.range(1, 8))
            .build();
        let sm = random_samples(&mut rng);
        run_diff(&format!("row24/{i}"), &st, &sm, &|api, s, sam| unsafe {
            let mut rets = Vec::new();
            for k in 0..8usize {
                check_samples_window(k * 136);
                rets.push((api.update_md5)(s, sam.add(k * 136)));
            }
            rets
        });
    }
}

// ===========================================================================
// Row 25 — interleaved script across all three entry points, shared state
// ===========================================================================
#[test]
fn row25_interleaved_all_entry_points() {
    let mut rng = Rng::new(0x1919_1919);
    for i in 0..N {
        #[derive(Clone, Copy)]
        enum Op {
            Pack(usize, u64),
            Add(u32, u64),
            Update(usize),
        }
        let mut script = Vec::new();
        for _ in 0..64 {
            match rng.below(3) {
                0 => script.push(Op::Pack(rng.below(65) as usize, rng.next_u64())),
                1 => script.push(Op::Add(
                    rng.pick(&[0u32, 1, 7, 8, 63, 64, 65, 128, 512, 520, u32::MAX]),
                    rng.next_u64(),
                )),
                _ => script.push(Op::Update(rng.below(120) as usize)),
            }
        }
        let st = StateBuilder::random(&mut rng)
            .pos(rng.below(4096))
            .total(rng.next_u64())
            .cur_blocksize(rng.next_u32())
            .channels(rng.range(0, 8))
            .build();
        let sm = random_samples(&mut rng);
        run_diff(&format!("row25/{i}"), &st, &sm, &|api, s, sam| unsafe {
            let mut rets: Vec<u32> = Vec::new();
            for op in &script {
                match *op {
                    Op::Pack(off, n) => (api.pack_u64le)(s.add(OFF_BUFFER + off), n),
                    Op::Add(bits, val) => (api.md5_addsample)(s, bits, val),
                    Op::Update(k) => {
                        check_samples_window(k);
                        rets.push((api.update_md5)(s, sam.add(k)))
                    }
                }
            }
            rets
        });
    }
}
