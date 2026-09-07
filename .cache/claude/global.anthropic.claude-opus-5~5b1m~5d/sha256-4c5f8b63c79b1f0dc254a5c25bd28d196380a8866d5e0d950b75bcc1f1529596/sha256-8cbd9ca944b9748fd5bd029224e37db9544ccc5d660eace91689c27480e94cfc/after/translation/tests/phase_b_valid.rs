//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`, each driven with many randomized inputs
//! from a fixed-seed PRNG. Both implementations are loaded from their `.so`
//! files via `libloading`; nothing calls the Rust crate directly.

mod common;
use common::*;

const PATTERNS: [&[u8]; 4] = [&[0x00], &[0xAA], &[0x55], &[0xDE, 0xAD, 0xBE, 0xEF]];

/* ================================================================== */
/* Rows 1-4: tflac_pack_u64le                                          */
/* ================================================================== */

#[test]
fn row01_pack_offset0_boundary_and_single_bits() {
    for pat in PATTERNS {
        let buf = PackBuf::new(pat);
        diff_pack("row01/zero", buf, 0, 0);
        diff_pack("row01/max", buf, 0, u64::MAX);
        for bit in 0..64 {
            diff_pack("row01/bit", buf, 0, 1u64 << bit);
            diff_pack("row01/nbit", buf, 0, !(1u64 << bit));
        }
    }
}

#[test]
fn row02_pack_offset_sweep_random() {
    let mut rng = Rng::new(0x1111_2222_3333_4444);
    for offset in 0..=64usize {
        for pat in PATTERNS {
            let buf = PackBuf::new(pat);
            for _ in 0..64 {
                diff_pack("row02", buf, offset, rng.next_u64());
            }
        }
    }
}

#[test]
fn row03_pack_at_buffer_end_offset64() {
    // offset 64 is the largest offset the library itself can generate
    // (pos % 64 == 63 would be 63; update_md5's 8-byte store at 64 exactly
    // fills the 72-byte buffer). Whole 104-byte image incl. guard compared.
    let mut rng = Rng::new(0xDEAD_BEEF_CAFE_1234);
    for pat in PATTERNS {
        let buf = PackBuf::new(pat);
        for _ in 0..2000 {
            diff_pack("row03", buf, 64, rng.next_u64());
        }
        diff_pack("row03/0", buf, 64, 0);
        diff_pack("row03/max", buf, 64, u64::MAX);
    }
}

#[test]
fn row04_pack_repeated_overlapping_writes() {
    let p = pair();
    let mut rng = Rng::new(0x0BAD_F00D_0000_0001);
    for _ in 0..500 {
        let base = PackBuf::new(&[0x33]);
        let mut cb = base;
        let mut rb = base;
        // 12 overlapping stores at random in-bounds offsets, applied in the
        // same order to both impls; state carried forward across calls.
        for _ in 0..12 {
            let off = rng.below(65) as usize;
            let n = rng.next_u64();
            unsafe {
                (p.c.pack_u64le)(cb.as_mut_ptr().add(off), n);
                (p.rs.pack_u64le)(rb.as_mut_ptr().add(off), n);
            }
            assert_eq!(cb, rb, "row04 divergence after off={off} n={n:#018x}");
        }
    }
}

/* ================================================================== */
/* Rows 5-14: tflac_md5_addsample                                      */
/* ================================================================== */

#[test]
fn row05_addsample_pos0_bits64_no_folddown() {
    let mut rng = Rng::new(0x5555_0000_0000_0005);
    let zero = [0u8; BUF_LEN];
    for _ in 0..2000 {
        let out = diff_addsample("row05", Md5Ctx::new(0, 0, &zero), 64, rng.next_u64());
        // sanity: the C leaves pos == 8 here (no fold-down)
        assert_eq!(out.pos(), 8);
        assert_eq!(out.total(), 64);
    }
}

#[test]
fn row06_addsample_pos_sweep_0_to_63() {
    let mut rng = Rng::new(0x6666_0000_0000_0006);
    for pos in 0..64u32 {
        for _ in 0..200 {
            let buf = rng.fill_buf();
            let total = rng.next_u64();
            diff_addsample("row06", Md5Ctx::new(pos, total, &buf), 64, rng.next_u64());
        }
    }
}

#[test]
fn row07_addsample_pos56_exact_boundary_zero_length_copy() {
    // pos 56 + 8 == 64 -> fold-down taken, pos %= 64 == 0, `while (bytes--)`
    // runs zero times (the unsigned post-decrement wrap is unobservable).
    let mut rng = Rng::new(0x7777_0000_0000_0007);
    for _ in 0..3000 {
        let buf = rng.fill_buf();
        let out = diff_addsample("row07", Md5Ctx::new(56, rng.next_u64(), &buf), 64, rng.next_u64());
        assert_eq!(out.pos(), 0, "row07: expected fold-down to pos 0");
    }
}

#[test]
fn row08_addsample_pos57_to_63_real_tail_copy() {
    let mut rng = Rng::new(0x8888_0000_0000_0008);
    for pos in 57..64u32 {
        for _ in 0..500 {
            let buf = rng.fill_buf();
            let out =
                diff_addsample("row08", Md5Ctx::new(pos, rng.next_u64(), &buf), 64, rng.next_u64());
            assert_eq!(out.pos(), (pos + 8) % 64);
        }
    }
}

#[test]
fn row09_addsample_pos_out_of_documented_range() {
    let mut rng = Rng::new(0x9999_0000_0000_0009);
    let odd_pos = [64u32, 65, 66, 71, 72, 127, 128, 191, 255, 4096, 0xFFFF_FF00, 0xFFFF_FFFF];
    for &pos in &odd_pos {
        for &bits in &[0u32, 8, 16, 32, 64] {
            for _ in 0..200 {
                let buf = rng.fill_buf();
                diff_addsample(
                    "row09",
                    Md5Ctx::new(pos, rng.next_u64(), &buf),
                    bits,
                    rng.next_u64(),
                );
            }
        }
    }
}

#[test]
fn row10_addsample_bits_including_non_multiples_of_8() {
    let mut rng = Rng::new(0xAAAA_0000_0000_000A);
    let bits_set = [0u32, 1, 2, 3, 7, 8, 9, 15, 16, 17, 23, 24, 31, 32, 39, 40, 47, 48, 55, 56, 63,
        64, 65, 71, 72, 127, 128, 255, 256, 511, 512];
    for &bits in &bits_set {
        for _ in 0..300 {
            let buf = rng.fill_buf();
            let pos = rng.below(200);
            diff_addsample("row10", Md5Ctx::new(pos, rng.next_u64(), &buf), bits, rng.next_u64());
        }
    }
}

#[test]
fn row11_addsample_oversized_bits() {
    let mut rng = Rng::new(0xBBBB_0000_0000_000B);
    let bits_set =
        [0xFFFF_FFFFu32, 0xFFFF_FFF8, 0xFFFF_FF00, 0x8000_0000, 0x7FFF_FFFF, 0x0000_0200, u32::MAX - 1];
    for &bits in &bits_set {
        for &pos in &[0u32, 1, 56, 63, 64, 0xFFFF_FFFF] {
            for _ in 0..200 {
                let buf = rng.fill_buf();
                diff_addsample(
                    "row11",
                    Md5Ctx::new(pos, rng.next_u64(), &buf),
                    bits,
                    rng.next_u64(),
                );
            }
        }
    }
}

#[test]
fn row12_addsample_total_wraparound() {
    let mut rng = Rng::new(0xCCCC_0000_0000_000C);
    let totals = [0u64, 1, 63, 64, u64::MAX, u64::MAX - 1, u64::MAX - 63, u64::MAX - 64,
        0xFFFF_FFFF_FFFF_FF00, 0x8000_0000_0000_0000];
    for &total in &totals {
        for &bits in &[0u32, 1, 64, 0xFFFF_FFFF] {
            for _ in 0..200 {
                let buf = rng.fill_buf();
                let pos = rng.below(128);
                diff_addsample("row12", Md5Ctx::new(pos, total, &buf), bits, rng.next_u64());
            }
        }
    }
}

#[test]
fn row13_addsample_buffer_content_variants() {
    let mut rng = Rng::new(0xDDDD_0000_0000_000D);
    let mut variants: Vec<[u8; BUF_LEN]> = vec![
        [0x00; BUF_LEN],
        [0xFF; BUF_LEN],
        [0xAA; BUF_LEN],
        [0x55; BUF_LEN],
    ];
    // an ascending ramp makes an off-by-one in the fold-down copy obvious
    let mut ramp = [0u8; BUF_LEN];
    for (i, b) in ramp.iter_mut().enumerate() {
        *b = i as u8;
    }
    variants.push(ramp);
    for _ in 0..8 {
        variants.push(rng.fill_buf());
    }

    for buf in &variants {
        for pos in 56..64u32 {
            for _ in 0..100 {
                diff_addsample("row13", Md5Ctx::new(pos, rng.next_u64(), buf), 64, rng.next_u64());
            }
        }
        for _ in 0..300 {
            let pos = rng.below(300);
            let bits = rng.next_u32() % 200;
            diff_addsample("row13/rand", Md5Ctx::new(pos, rng.next_u64(), buf), bits, rng.next_u64());
        }
    }
}

#[test]
fn row14_addsample_long_sequence_shared_state() {
    // Composed pipeline: 500 randomized calls per run, state carried forward,
    // compared after EVERY call so the first divergence is pinpointed.
    let p = pair();
    let mut rng = Rng::new(0xEEEE_0000_0000_000E);
    for run in 0..20 {
        let buf = rng.fill_buf();
        let start = Md5Ctx::new(rng.below(70), rng.next_u64(), &buf);
        let mut cm = start;
        let mut rm = start;
        for step in 0..500 {
            let bits = match rng.below(4) {
                0 => 64,
                1 => 8 * (1 + rng.below(8)),
                2 => rng.next_u32() % 128,
                _ => rng.next_u32(),
            };
            let val = rng.next_u64();
            unsafe {
                (p.c.md5_addsample)(cm.as_mut_ptr(), bits, val);
                (p.rs.md5_addsample)(rm.as_mut_ptr(), bits, val);
            }
            assert_eq!(
                cm, rm,
                "row14 divergence run={run} step={step} bits={bits} val={val:#018x}\n \
                 C: {cm:?}\nRS: {rm:?}"
            );
        }
    }
}

/* ================================================================== */
/* Rows 15-27: update_md5                                              */
/* ================================================================== */

const N: usize = UPDATE_MD5_MIN_SAMPLES; // 136

#[test]
fn row15_update_minimal_config() {
    let zero = [0u8; BUF_LEN];
    let samples = vec![0i32; N];
    let (ret, _) = diff_update("row15", TflacCtx::new(0, 0, &zero, 1, 1), &samples);
    // 1*1 - 5*8 = 1 - 40 -> wraps
    assert_eq!(ret, 1u32.wrapping_sub(40));
}

#[test]
fn row16_update_product_gt_40_random_samples() {
    let mut rng = Rng::new(0x1616_0000_0000_0010);
    let combos: [(u32, u32); 8] =
        [(4096, 2), (1152, 8), (64, 1), (41, 1), (48, 1), (1, 41), (256, 4), (4608, 2)];
    for &(cbs, ch) in &combos {
        for _ in 0..300 {
            let buf = rng.fill_buf();
            let samples = rng.samples(N);
            diff_update("row16", TflacCtx::new(rng.below(70), rng.next_u64(), &buf, cbs, ch), &samples);
        }
    }
}

#[test]
fn row17_update_product_exactly_40() {
    let mut rng = Rng::new(0x1717_0000_0000_0011);
    for &(cbs, ch) in &[(40u32, 1u32), (1, 40), (8, 5), (5, 8), (20, 2), (2, 20), (4, 10), (10, 4)] {
        for _ in 0..200 {
            let buf = rng.fill_buf();
            let samples = rng.samples(N);
            let (ret, _) =
                diff_update("row17", TflacCtx::new(rng.below(70), rng.next_u64(), &buf, cbs, ch), &samples);
            assert_eq!(ret, 0, "row17: {cbs}*{ch} should return 0");
        }
    }
}

#[test]
fn row18_update_product_lt_40_underflow() {
    let mut rng = Rng::new(0x1818_0000_0000_0012);
    for prod in 1..40u32 {
        // factorise a couple of ways to also vary the two fields
        for &(cbs, ch) in &[(prod, 1u32), (1u32, prod)] {
            for _ in 0..40 {
                let buf = rng.fill_buf();
                let samples = rng.samples(N);
                let (ret, _) = diff_update(
                    "row18",
                    TflacCtx::new(rng.below(70), rng.next_u64(), &buf, cbs, ch),
                    &samples,
                );
                assert_eq!(ret, prod.wrapping_sub(40));
            }
        }
    }
}

#[test]
fn row19_update_zero_blocksize_or_channels() {
    let mut rng = Rng::new(0x1919_0000_0000_0013);
    let combos: [(u32, u32); 7] =
        [(0, 0), (0, 1), (1, 0), (0, 4096), (4096, 0), (0, u32::MAX), (u32::MAX, 0)];
    for &(cbs, ch) in &combos {
        for _ in 0..200 {
            let buf = rng.fill_buf();
            let samples = rng.samples(N);
            let (ret, _) = diff_update(
                "row19",
                TflacCtx::new(rng.below(70), rng.next_u64(), &buf, cbs, ch),
                &samples,
            );
            assert_eq!(ret, 0xFFFF_FFD8, "row19: {cbs}*{ch}");
        }
    }
}

#[test]
fn row20_update_product_overflows_u32() {
    let mut rng = Rng::new(0x2020_0000_0000_0014);
    let combos: [(u32, u32); 8] = [
        (0x1_0000, 0x1_0000),
        (u32::MAX, u32::MAX),
        (0xFFFF, 0x1_0001),
        (0x8000_0000, 2),
        (2, 0x8000_0000),
        (0xFFFF_FFFF, 3),
        (0x1234_5678, 0x9ABC_DEF0u32),
        (0x10000, 0x10001),
    ];
    for &(cbs, ch) in &combos {
        for _ in 0..200 {
            let buf = rng.fill_buf();
            let samples = rng.samples(N);
            let (ret, _) = diff_update(
                "row20",
                TflacCtx::new(rng.below(70), rng.next_u64(), &buf, cbs, ch),
                &samples,
            );
            assert_eq!(ret, cbs.wrapping_mul(ch).wrapping_sub(40));
        }
    }
}

#[test]
fn row21_update_extreme_sample_values() {
    let mut rng = Rng::new(0x2121_0000_0000_0015);
    let fixed: Vec<Vec<i32>> = vec![
        vec![0i32; N],
        vec![-1i32; N],
        vec![i32::MIN; N],
        vec![i32::MAX; N],
        (0..N).map(|i| if i % 2 == 0 { i32::MIN } else { i32::MAX }).collect(),
        (0..N).map(|i| -(i as i32)).collect(),
        (0..N).map(|i| (i as i32) - 68).collect(),
        (0..N).map(|i| if i % 3 == 0 { -256 } else { 255 }).collect(),
        (0..N).map(|i| ((i as i32) << 8) | 0x7F).collect(),
    ];
    for samples in &fixed {
        for _ in 0..100 {
            let buf = rng.fill_buf();
            diff_update(
                "row21",
                TflacCtx::new(rng.below(70), rng.next_u64(), &buf, rng.next_u32(), rng.next_u32()),
                samples,
            );
        }
    }
}

#[test]
fn row22_update_fully_random_samples() {
    for seed in 0..24u64 {
        let mut rng = Rng::new(0x2222_0000_0000_0000 ^ (seed << 17) ^ seed);
        for _ in 0..200 {
            let buf = rng.fill_buf();
            let samples = rng.samples(N);
            diff_update(
                "row22",
                TflacCtx::new(rng.next_u32(), rng.next_u64(), &buf, rng.next_u32(), rng.next_u32()),
                &samples,
            );
        }
    }
}

#[test]
fn row23_update_initial_pos_variants() {
    let mut rng = Rng::new(0x2323_0000_0000_0017);
    let positions = [0u32, 1, 7, 8, 16, 24, 32, 40, 48, 55, 56, 57, 60, 63, 64, 65, 71, 72, 127,
        128, 191, 255, 4096, 0xFFFF_FF00, 0xFFFF_FFFF];
    for &pos in &positions {
        for _ in 0..200 {
            let buf = rng.fill_buf();
            let samples = rng.samples(N);
            diff_update(
                "row23",
                TflacCtx::new(pos, rng.next_u64(), &buf, 4096, 2),
                &samples,
            );
        }
    }
}

#[test]
fn row24_update_initial_total_variants() {
    let mut rng = Rng::new(0x2424_0000_0000_0018);
    let totals = [0u64, 1, 319, 320, u64::MAX, u64::MAX - 1, u64::MAX - 100, u64::MAX - 320,
        0x8000_0000_0000_0000, 0xFFFF_FFFF_FFFF_FF00];
    for &total in &totals {
        for _ in 0..200 {
            let buf = rng.fill_buf();
            let samples = rng.samples(N);
            diff_update("row24", TflacCtx::new(rng.below(70), total, &buf, 4096, 2), &samples);
        }
    }
}

#[test]
fn row25_update_initial_buffer_variants() {
    let mut rng = Rng::new(0x2525_0000_0000_0019);
    let mut ramp = [0u8; BUF_LEN];
    for (i, b) in ramp.iter_mut().enumerate() {
        *b = (0xF0 ^ i) as u8;
    }
    let mut variants: Vec<[u8; BUF_LEN]> =
        vec![[0x00; BUF_LEN], [0xFF; BUF_LEN], [0xAA; BUF_LEN], [0x55; BUF_LEN], ramp];
    for _ in 0..8 {
        variants.push(rng.fill_buf());
    }
    for buf in &variants {
        for pos in [0u32, 56, 57, 60, 63, 64] {
            for _ in 0..80 {
                let samples = rng.samples(N);
                diff_update("row25", TflacCtx::new(pos, rng.next_u64(), buf, 4096, 2), &samples);
            }
        }
    }
}

#[test]
fn row26_update_exact_136_element_buffer_and_guard() {
    // (a) Exactly 136 elements: proves nothing past index 135 is read.
    // (b) 512 elements where everything *except* the 40 indices the C reads
    //     ({i*32+k | i<5, k<8}) is replaced by junk: the result must be
    //     identical to the 136-element case, pinning the 32-element stride.
    let mut rng = Rng::new(0x2626_0000_0000_001A);
    for _ in 0..400 {
        let buf = rng.fill_buf();
        let pos = rng.below(70);
        let total = rng.next_u64();
        let (cbs, ch) = (rng.next_u32(), rng.next_u32());
        let start = TflacCtx::new(pos, total, &buf, cbs, ch);

        let exact = rng.samples(N);
        let (r1, s1) = diff_update("row26/exact", start, &exact);

        let mut padded = exact.clone();
        padded.resize(512, 0);
        for (i, v) in padded.iter_mut().enumerate() {
            let read_by_c = i < N && (i % 32) < 8;
            if !read_by_c {
                *v = rng.next_i32(); // junk in every never-read slot
            }
        }
        let (r2, s2) = diff_update("row26/padded", start, &padded);
        assert_eq!(r1, r2, "row26: unread slots affected the return value");
        assert_eq!(s1, s2, "row26: unread slots affected the context");
    }
}

#[test]
fn row27_update_repeated_calls_advancing_window() {
    let p = pair();
    let mut rng = Rng::new(0x2727_0000_0000_001B);
    for run in 0..20 {
        let buf = rng.fill_buf();
        let start = TflacCtx::new(rng.below(70), rng.next_u64(), &buf, 4096, 2);
        let mut ct = start;
        let mut rt = start;
        let pool = rng.samples(N + 60 * 16);
        for step in 0..60usize {
            let window = &pool[step * 16..step * 16 + N];
            let (cr, rr) = unsafe {
                (
                    (p.c.update_md5)(ct.as_mut_ptr(), window.as_ptr()),
                    (p.rs.update_md5)(rt.as_mut_ptr(), window.as_ptr()),
                )
            };
            assert_eq!(cr, rr, "row27 return divergence run={run} step={step}");
            assert_eq!(ct, rt, "row27 state divergence run={run} step={step}\n C: {ct:?}\nRS: {rt:?}");
        }
    }
}

/* ================================================================== */
/* Row 28: mixed interleaved sequence over one shared context          */
/* ================================================================== */

#[test]
fn row28_mixed_interleaved_pipeline() {
    let p = pair();
    let mut rng = Rng::new(0x2828_0000_0000_001C);
    for run in 0..25 {
        let buf = rng.fill_buf();
        let start = TflacCtx::new(rng.below(70), rng.next_u64(), &buf, rng.next_u32(), rng.next_u32());
        let mut ct = start;
        let mut rt = start;
        let pool = rng.samples(N + 512);

        for step in 0..300usize {
            match rng.below(3) {
                // tflac_pack_u64le directly into the context's md5 buffer
                0 => {
                    let off = 16 + rng.below(65) as usize; // buffer starts at +16
                    let n = rng.next_u64();
                    unsafe {
                        (p.c.pack_u64le)(ct.as_mut_ptr().add(off), n);
                        (p.rs.pack_u64le)(rt.as_mut_ptr().add(off), n);
                    }
                    assert_eq!(ct, rt, "row28 pack divergence run={run} step={step} off={off}");
                }
                // tflac_md5_addsample directly on the embedded md5 context
                1 => {
                    let bits = if rng.below(2) == 0 { 64 } else { rng.next_u32() % 300 };
                    let val = rng.next_u64();
                    unsafe {
                        (p.c.md5_addsample)(ct.as_mut_ptr(), bits, val);
                        (p.rs.md5_addsample)(rt.as_mut_ptr(), bits, val);
                    }
                    assert_eq!(ct, rt, "row28 addsample divergence run={run} step={step} bits={bits}");
                }
                // the composed high-level entry point
                _ => {
                    let off = rng.below(512) as usize;
                    let w = &pool[off..off + N];
                    let (cr, rr) = unsafe {
                        (
                            (p.c.update_md5)(ct.as_mut_ptr(), w.as_ptr()),
                            (p.rs.update_md5)(rt.as_mut_ptr(), w.as_ptr()),
                        )
                    };
                    assert_eq!(cr, rr, "row28 update ret divergence run={run} step={step}");
                    assert_eq!(ct, rt, "row28 update state divergence run={run} step={step}");
                }
            }
        }
    }
}

/* ================================================================== */
/* ABI / layout parity (documented in SYMBOLS.md)                      */
/* ================================================================== */

#[test]
fn abi_layout_matches_c() {
    assert_eq!(MD5_SIZE, 88);
    assert_eq!(TFLAC_SIZE, 96);
    assert_eq!(BUF_LEN, 72);
    // If the Rust struct layout disagreed with the C one, the byte-image
    // comparisons above would fail; this test documents the verified numbers
    // reported by the C compiler (see SYMBOLS.md).
    let mut m = Md5Ctx::new(0x1234_5678, 0x0102_0304_0506_0708, &[0x99; BUF_LEN]);
    assert_eq!(m.pos(), 0x1234_5678);
    assert_eq!(m.total(), 0x0102_0304_0506_0708);
    assert_eq!(m.as_mut_ptr() as usize % 8, 0);
    let t = TflacCtx::new(1, 2, &[0; BUF_LEN], 4096, 2);
    assert_eq!(t.cur_blocksize(), 4096);
    assert_eq!(t.channels(), 2);
}

#[test]
fn both_libraries_export_all_three_symbols() {
    let p = pair();
    // Loading already resolved all three symbols in both libs; report paths so
    // a stale build is obvious in the test log.
    eprintln!("C  .so: {}", p.c.path.display());
    eprintln!("RS .so: {}", p.rs.path.display());
    assert_eq!(p.c.name, "C");
    assert_eq!(p.rs.name, "Rust");
}
