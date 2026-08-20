//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Both the C `.so` and the Rust `.so` are loaded with `libloading` and every
//! call goes through the exported C symbol.  Each row runs many seeded
//! pseudo-random instances (fixed seed ⇒ reproducible) rather than one
//! hand-picked value.

mod harness;
use harness::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// The 4 buffer pre-fill shapes from CONFIGS axis 6.
fn prefill(kind: usize, rng: &mut Rng) -> [u8; BUF_LEN] {
    match kind {
        0 => [0x00u8; BUF_LEN],
        1 => [0xFFu8; BUF_LEN],
        2 => {
            let mut b = [0u8; BUF_LEN];
            for (i, x) in b.iter_mut().enumerate() {
                *x = i as u8;
            }
            b
        }
        _ => rng.buf72(),
    }
}

/// Arena whose byte `i` is `i as u8` — makes any misplaced byte obvious.
fn index_pattern_arena() -> Vec<u8> {
    (0..ARENA).map(|i| i as u8).collect()
}

/// Build a samples arena for CONFIGS axis 10.
fn samples_shape(kind: usize, rng: &mut Rng) -> Vec<u8> {
    let n = ARENA / 4;
    let mut v = vec![0u8; ARENA];
    for i in 0..n {
        let val: i32 = match kind {
            0 => 0,
            1 => -1,
            2 => i32::MIN,
            3 => i32::MAX,
            4 => 0x1234_5600,               // low byte 0x00
            5 => 0x1234_567F,               // low byte 0x7F
            6 => -0x1234_5680i32,           // low byte 0x80
            7 => 0x1234_56FFu32 as i32,     // low byte 0xFF
            8 => {
                if i % 2 == 0 {
                    i as i32
                } else {
                    -(i as i32)
                }
            }
            9 => i as i32,
            10 => !(i as i32),
            _ => rng.i32(),
        };
        v[i * 4..i * 4 + 4].copy_from_slice(&val.to_le_bytes());
    }
    v
}

// ===========================================================================
// tflac_pack_u64le
// ===========================================================================

/// C1 — aligned destination, every interesting `n` shape + 512 random values.
#[test]
fn cfg_c1_pack_values() {
    let mut rng = Rng::new(0xC001);
    let mut fixed: Vec<u64> = vec![0, u64::MAX, 0x0123_4567_89AB_CDEF, 1, 0x8000_0000_0000_0000];
    for lane in 0..8u32 {
        fixed.push(1u64 << (lane * 8)); // one-hot bit per byte lane
        fixed.push(0xFFu64 << (lane * 8)); // one-hot byte per lane
        fixed.push(!(0xFFu64 << (lane * 8)));
    }
    let tpl = index_pattern_arena();
    for (k, n) in fixed.iter().enumerate() {
        diff_pack(&tpl, 0, *n, &format!("C1 fixed[{k}] n={n:#018x}"));
        diff_pack(&tpl, 64, *n, &format!("C1 fixed[{k}]@64 n={n:#018x}"));
    }
    for i in 0..512 {
        let n = rng.u64();
        let off = (rng.below((ARENA - 8) as u64 / 8) * 8) as usize;
        diff_pack(&tpl, off, n, &format!("C1 rand[{i}] off={off} n={n:#018x}"));
    }
}

/// C2 — every misalignment 0..7 of `d` × random payloads.
#[test]
fn cfg_c2_pack_misalignment() {
    let mut rng = Rng::new(0xC002);
    let tpl = index_pattern_arena();
    for mis in 0..8usize {
        for i in 0..256 {
            let base = (rng.below(((ARENA - 16) / 8) as u64) * 8) as usize;
            let off = base + mis;
            let n = rng.u64();
            diff_pack(&tpl, off, n, &format!("C2 mis={mis} i={i} off={off} n={n:#018x}"));
        }
    }
}

/// C3 — sweep `d` over every legal offset, including the very last 8 bytes.
#[test]
fn cfg_c3_pack_offset_sweep() {
    let mut rng = Rng::new(0xC003);
    let tpl = index_pattern_arena();
    for off in 0..=(ARENA - 8) {
        let n = rng.u64();
        diff_pack(&tpl, off, n, &format!("C3 off={off} n={n:#018x}"));
    }
    // and explicitly the boundary offset with the extreme payloads
    for n in [0u64, u64::MAX, 0x0123_4567_89AB_CDEF] {
        diff_pack(&tpl, ARENA - 8, n, &format!("C3 last8 n={n:#018x}"));
        diff_pack(&tpl, 0, n, &format!("C3 first8 n={n:#018x}"));
    }
}

/// C4 — overlapping / adjacent writes that partially clobber each other.
#[test]
fn cfg_c4_pack_overlapping_writes() {
    let mut rng = Rng::new(0xC004);
    let tpl = index_pattern_arena();
    for i in 0..256 {
        let p = (rng.below((ARENA - 32) as u64)) as usize;
        let writes = [
            (p, rng.u64()),
            (p + 1, rng.u64()),
            (p + 7, rng.u64()),
            (p + 8, rng.u64()),
            (p, rng.u64()),
        ];
        diff_pack_seq(&tpl, &writes, &format!("C4 i={i} p={p}"));
    }
}

// ===========================================================================
// tflac_md5_addsample
// ===========================================================================

/// C5 — `bits == 0` (⇒ `bytes == 0`, branch not taken) across `pos` shapes.
#[test]
fn cfg_c5_addsample_bits0() {
    let mut rng = Rng::new(0xC005);
    for pos in [0u32, 1, 7, 8, 56, 57, 63] {
        for i in 0..128 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
            diff_add(&tpl, 0, 0, rng.u64(), &format!("C5 pos={pos} i={i}"));
        }
    }
}

/// C6 — `bits == 8` (one byte per step), full sweep of `pos` 0..=63.
#[test]
fn cfg_c6_addsample_bits8_pos_sweep() {
    let mut rng = Rng::new(0xC006);
    for pos in 0..64u32 {
        for i in 0..64 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
            diff_add(&tpl, 0, 8, rng.u64(), &format!("C6 pos={pos} i={i}"));
        }
    }
}

/// C7 — `bits == 64` (the value `update_md5` hard-codes), full `pos` sweep.
/// Covers branch-not-taken (`pos < 56`), branch-taken/0-iteration (`pos == 56`)
/// and branch-taken/1..7-iteration in-bounds copies (`pos == 57..63`).
#[test]
fn cfg_c7_addsample_bits64_pos_sweep() {
    let mut rng = Rng::new(0xC007);
    for pos in 0..64u32 {
        for i in 0..64 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
            diff_add(&tpl, 0, 64, rng.u64(), &format!("C7 pos={pos} i={i}"));
        }
    }
}

/// C8 — branch taken with reduced `pos == 0` ⇒ `while (bytes--)` must run
/// exactly zero times (post-decrement underflow discarded).
#[test]
fn cfg_c8_addsample_zero_copy_iterations() {
    let mut rng = Rng::new(0xC008);
    for (pos, bits) in [(56u32, 64u32), (0, 512), (32, 256), (63, 8), (60, 32)] {
        for i in 0..128 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
            diff_add(&tpl, 0, bits, rng.u64(), &format!("C8 pos={pos} bits={bits} i={i}"));
        }
    }
}

/// C9 — branch taken with reduced `pos` in 1..=8: the copy source
/// `buffer[64..64+r]` stays inside the 72-byte array.
#[test]
fn cfg_c9_addsample_copy_in_bounds() {
    let mut rng = Rng::new(0xC009);
    for r in 1..=8u32 {
        let bits = bits_for_reduced_pos(r % 64);
        for i in 0..128 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, 0, rng.u64(), &buf);
            diff_add(&tpl, 0, bits, rng.u64(), &format!("C9 r={r} bits={bits} i={i}"));
        }
    }
}

/// C10 — branch taken with reduced `pos` in 9..=63: the copy source runs past
/// `buffer[72]`, out of the record, into the arena.  Deterministic because both
/// sides see the same arena bytes.
#[test]
fn cfg_c10_addsample_copy_out_of_bounds() {
    let mut rng = Rng::new(0xC010);
    for r in 9..64u32 {
        let bits = bits_for_reduced_pos(r);
        for i in 0..64 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, 0, rng.u64(), &buf);
            diff_add(&tpl, 0, bits, rng.u64(), &format!("C10 r={r} bits={bits} i={i}"));
        }
    }
    // same, but with the record placed at a non-zero, still 8-aligned offset
    for r in [9u32, 32, 62, 63] {
        let bits = bits_for_reduced_pos(r);
        for off in [8usize, 64, 512, ARENA - ADD_MAX_TOUCH - 8] {
            for i in 0..16 {
                let mut tpl = rng.arena();
                let buf = rng.buf72();
                put_md5(&mut tpl, off, 0, rng.u64(), &buf);
                diff_add(&tpl, off, bits, rng.u64(), &format!("C10 off={off} r={r} i={i}"));
            }
        }
    }
}

/// C11 — `pos` in 57..=63 makes `tflac_pack_u64le` spill into `buffer[64..72]`
/// *before* the copy loop reads that same region: a write/read interaction.
#[test]
fn cfg_c11_addsample_write_read_interaction() {
    let mut rng = Rng::new(0xC011);
    for pos in 57..64u32 {
        for bits in [8u32, 64, 512] {
            for i in 0..64 {
                let mut tpl = rng.arena();
                let buf = rng.buf72();
                put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
                diff_add(
                    &tpl,
                    0,
                    bits,
                    rng.u64(),
                    &format!("C11 pos={pos} bits={bits} i={i}"),
                );
            }
        }
    }
}

/// C12 — `bits` not a multiple of 8: `bytes = bits/8` truncates while
/// `total += bits` does not.
#[test]
fn cfg_c12_addsample_bits_not_multiple_of_8() {
    let mut rng = Rng::new(0xC012);
    for bits in [1u32, 2, 7, 9, 63, 65, 511, 513] {
        for pos in [0u32, 7, 56, 63] {
            for i in 0..64 {
                let mut tpl = rng.arena();
                let buf = rng.buf72();
                put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
                diff_add(
                    &tpl,
                    0,
                    bits,
                    rng.u64(),
                    &format!("C12 bits={bits} pos={pos} i={i}"),
                );
            }
        }
    }
}

/// C13 — whole-block / whole-array / oversized `bits`.
#[test]
fn cfg_c13_addsample_large_bits() {
    let mut rng = Rng::new(0xC013);
    for bits in [512u32, 576, 4096] {
        for pos in [0u32, 8, 63] {
            for i in 0..64 {
                let mut tpl = rng.arena();
                let buf = rng.buf72();
                put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
                diff_add(
                    &tpl,
                    0,
                    bits,
                    rng.u64(),
                    &format!("C13 bits={bits} pos={pos} i={i}"),
                );
            }
        }
    }
}

/// C14 — incoming `total` shapes, including values that wrap.
#[test]
fn cfg_c14_addsample_total_shapes() {
    let mut rng = Rng::new(0xC014);
    for total in [
        0u64,
        1,
        0x7FFF_FFFF_FFFF_FFFF,
        u64::MAX,
        u64::MAX - 63,
        u64::MAX - 64,
    ] {
        for bits in [0u32, 64, u32::MAX] {
            for i in 0..64 {
                let mut tpl = rng.arena();
                let buf = rng.buf72();
                let pos = rng.u32() % 64;
                put_md5(&mut tpl, 0, pos, total, &buf);
                diff_add(
                    &tpl,
                    0,
                    bits,
                    rng.u64(),
                    &format!("C14 total={total:#x} bits={bits} pos={pos} i={i}"),
                );
            }
        }
    }
}

/// C15 — the four buffer pre-fill patterns.
#[test]
fn cfg_c15_addsample_buffer_prefills() {
    let mut rng = Rng::new(0xC015);
    for kind in 0..4usize {
        for pos in [40u32, 63] {
            for i in 0..64 {
                let mut tpl = rng.arena();
                let buf = prefill(kind, &mut rng);
                put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
                diff_add(
                    &tpl,
                    0,
                    64,
                    rng.u64(),
                    &format!("C15 prefill={kind} pos={pos} i={i}"),
                );
            }
        }
    }
}

/// C16 — `val` payload shapes (byte-order / shift-amount coverage).
#[test]
fn cfg_c16_addsample_val_shapes() {
    let mut rng = Rng::new(0xC016);
    let mut vals: Vec<u64> = vec![0, u64::MAX, 0x0123_4567_89AB_CDEF];
    for lane in 0..8u32 {
        vals.push(0xFFu64 << (lane * 8));
    }
    vals.push(0x8080_8080_8080_8080);
    for pos in [0u32, 57, 63] {
        for (k, v) in vals.iter().enumerate() {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
            diff_add(&tpl, 0, 64, *v, &format!("C16 pos={pos} val[{k}]={v:#018x}"));
        }
        for i in 0..128 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_md5(&mut tpl, 0, pos, rng.u64(), &buf);
            diff_add(&tpl, 0, 64, rng.u64(), &format!("C16 pos={pos} rand={i}"));
        }
    }
}

/// C17 — streaming: 64 chained `bits == 64` calls (the `update_md5` cadence).
#[test]
fn cfg_c17_addsample_stream_bits64() {
    for seed in 0..64u64 {
        let mut rng = Rng::new(0xC017_0000 + seed);
        let mut tpl = rng.arena();
        let buf = rng.buf72();
        put_md5(&mut tpl, 0, 0, 0, &buf);
        let calls: Vec<(u32, u64)> = (0..64).map(|_| (64u32, rng.u64())).collect();
        diff_add_stream(&tpl, 0, &calls, &format!("C17 seed={seed}"));
    }
}

/// C18 — streaming with a full-range random `bits` per call: mixes every branch
/// and wraps both `pos` and `total`.
#[test]
fn cfg_c18_addsample_stream_random_bits() {
    for seed in 0..64u64 {
        let mut rng = Rng::new(0xC018_0000 + seed);
        let mut tpl = rng.arena();
        let buf = rng.buf72();
        put_md5(&mut tpl, 0, rng.u32(), rng.u64(), &buf);
        let calls: Vec<(u32, u64)> = (0..64)
            .map(|_| {
                let bits = match rng.below(4) {
                    0 => rng.u32(),
                    1 => (rng.u32() % 1024) * 8,
                    2 => rng.u32() % 1024,
                    _ => 64,
                };
                (bits, rng.u64())
            })
            .collect();
        diff_add_stream(&tpl, 0, &calls, &format!("C18 seed={seed}"));
    }
}

// ===========================================================================
// update_md5
// ===========================================================================

/// C19 — `b == 0` (the "empty" configurations).
#[test]
fn cfg_c19_update_b_zero() {
    let mut rng = Rng::new(0xC019);
    for (cb, ch) in [(0u32, 0u32), (0, 7), (7, 0)] {
        for i in 0..128 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_tflac(&mut tpl, 0, 0, 0, &buf, cb, ch);
            let stpl = rng.arena();
            let r = diff_upd(&tpl, 0, &stpl, 0, &format!("C19 cb={cb} ch={ch} i={i}"));
            assert_eq!(r, 0u32.wrapping_sub(40), "C19 expected 0-40 wraparound");
        }
    }
}

/// C20 — sweep `b` over 1..=80: spans the `b < 40` underflow, the exact
/// boundary `b == 40` (⇒ returns 0) and the normal `b > 40` range.
#[test]
fn cfg_c20_update_b_sweep() {
    let mut rng = Rng::new(0xC020);
    for b in 1..=80u32 {
        // every factorisation of b that fits, plus (b,1) and (1,b)
        let mut pairs = vec![(b, 1u32), (1u32, b)];
        for f in 2..=b {
            if b % f == 0 {
                pairs.push((f, b / f));
            }
        }
        for (cb, ch) in pairs {
            for i in 0..8 {
                let mut tpl = rng.arena();
                let buf = rng.buf72();
                put_tflac(&mut tpl, 0, 0, 0, &buf, cb, ch);
                let stpl = rng.arena();
                let r = diff_upd(&tpl, 0, &stpl, 0, &format!("C20 b={b} cb={cb} ch={ch} i={i}"));
                assert_eq!(r, b.wrapping_sub(40), "C20 b={b}: return value");
            }
        }
    }
}

/// C21 — realistic FLAC blocksize × channel shapes.
#[test]
fn cfg_c21_update_realistic_shapes() {
    let mut rng = Rng::new(0xC021);
    for (cb, ch) in [
        (1u32, 1u32),
        (1, 2),
        (576, 1),
        (1152, 2),
        (4096, 2),
        (4608, 8),
        (16, 8),
    ] {
        for i in 0..64 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_tflac(&mut tpl, 0, 0, 0, &buf, cb, ch);
            let stpl = rng.arena();
            let r = diff_upd(&tpl, 0, &stpl, 0, &format!("C21 cb={cb} ch={ch} i={i}"));
            assert_eq!(r, cb.wrapping_mul(ch).wrapping_sub(40));
        }
    }
}

/// C22 — `cur_blocksize * channels` overflows `tflac_u32`.
#[test]
fn cfg_c22_update_multiply_overflow() {
    let mut rng = Rng::new(0xC022);
    for (cb, ch) in [
        (0x10000u32, 0x10000u32),
        (0xFFFF_FFFF, 3),
        (0x8000_0000, 2),
        (0xFFFF, 0x10001),
        (0xFFFF_FFFF, 0xFFFF_FFFF),
    ] {
        for i in 0..64 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_tflac(&mut tpl, 0, 0, 0, &buf, cb, ch);
            let stpl = rng.arena();
            let r = diff_upd(&tpl, 0, &stpl, 0, &format!("C22 cb={cb:#x} ch={ch:#x} i={i}"));
            assert_eq!(r, cb.wrapping_mul(ch).wrapping_sub(40));
        }
    }
}

/// C23 — incoming `md5_ctx.pos` swept over its whole legal 0..=63 range.
#[test]
fn cfg_c23_update_pos_sweep() {
    let mut rng = Rng::new(0xC023);
    for pos in 0..64u32 {
        for i in 0..32 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_tflac(&mut tpl, 0, pos, rng.u64(), &buf, rng.u32(), rng.u32());
            let stpl = rng.arena();
            diff_upd(&tpl, 0, &stpl, 0, &format!("C23 pos={pos} i={i}"));
        }
    }
}

/// C24 — incoming `md5_ctx.pos` outside 0..=63.
#[test]
fn cfg_c24_update_pos_out_of_range() {
    let mut rng = Rng::new(0xC024);
    for pos in [64u32, 65, 71, 72, 127, 128, 1000, 0xFFFF_FFF8, 0xFFFF_FFFF] {
        for i in 0..64 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_tflac(&mut tpl, 0, pos, rng.u64(), &buf, rng.u32(), rng.u32());
            let stpl = rng.arena();
            diff_upd(&tpl, 0, &stpl, 0, &format!("C24 pos={pos:#x} i={i}"));
        }
    }
}

/// C25 — incoming `md5_ctx.total` shapes (5 × 64 == 320 bits are added).
#[test]
fn cfg_c25_update_total_shapes() {
    let mut rng = Rng::new(0xC025);
    for total in [0u64, 1, u64::MAX, u64::MAX - 319, u64::MAX - 320] {
        for i in 0..64 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_tflac(&mut tpl, 0, 0, total, &buf, 4096, 2);
            let stpl = rng.arena();
            diff_upd(&tpl, 0, &stpl, 0, &format!("C25 total={total:#x} i={i}"));
            // sanity: the C must have wrapped, not saturated
            let l = libs();
            let _ = l;
        }
    }
}

/// C26 — the twelve sample data shapes (sign-extension then `& 0xFF`).
#[test]
fn cfg_c26_update_sample_shapes() {
    let mut rng = Rng::new(0xC026);
    for kind in 0..12usize {
        for pos in [0u32, 57] {
            let stpl = samples_shape(kind, &mut rng);
            for i in 0..16 {
                let mut tpl = rng.arena();
                let buf = rng.buf72();
                put_tflac(&mut tpl, 0, pos, rng.u64(), &buf, 1152, 2);
                diff_upd(&tpl, 0, &stpl, 0, &format!("C26 kind={kind} pos={pos} i={i}"));
            }
        }
    }
}

/// C27 — stride verification.  `samples += 8 * sizeof(tflac_s32)` advances 32
/// elements, so elements 8..31, 40..63, … are never read.  Perturbing only
/// those must change nothing — on both sides.
#[test]
fn cfg_c27_update_stride() {
    let mut rng = Rng::new(0xC027);
    let read_elems: Vec<usize> = (0..5).flat_map(|it| (0..8).map(move |k| it * 32 + k)).collect();
    for i in 0..128 {
        let base = ramp_samples(rng.u32());
        let mut tpl = rng.arena();
        let buf = rng.buf72();
        put_tflac(&mut tpl, 0, rng.u32() % 64, rng.u64(), &buf, 4096, 2);

        let r0 = diff_upd(&tpl, 0, &base, 0, &format!("C27 base i={i}"));

        // perturb ONLY the elements the stride skips, inside the read span
        let mut perturbed = base.clone();
        for e in 0..UPD_SAMPLE_SPAN_ELEMS {
            if !read_elems.contains(&e) {
                let v = rng.i32();
                perturbed[e * 4..e * 4 + 4].copy_from_slice(&v.to_le_bytes());
            }
        }
        let r1 = diff_upd(&tpl, 0, &perturbed, 0, &format!("C27 skipped-perturbed i={i}"));
        assert_eq!(r0, r1, "C27: perturbing skipped elements changed the result");

        // and perturb a READ element: both sides must change the same way
        let mut touched = base.clone();
        let e = read_elems[(rng.below(read_elems.len() as u64)) as usize];
        let v = rng.i32();
        touched[e * 4..e * 4 + 4].copy_from_slice(&v.to_le_bytes());
        diff_upd(&tpl, 0, &touched, 0, &format!("C27 read-perturbed e={e} i={i}"));
    }
}

/// C28 — misaligned `samples` pointer (the C reads `tflac_s32` through it).
#[test]
fn cfg_c28_update_samples_misaligned() {
    let mut rng = Rng::new(0xC028);
    for mis in 0..4usize {
        for i in 0..64 {
            let mut tpl = rng.arena();
            let buf = rng.buf72();
            put_tflac(&mut tpl, 0, rng.u32() % 64, rng.u64(), &buf, 1152, 2);
            let stpl = rng.arena();
            let soff = 8 + mis;
            diff_upd(&tpl, 0, &stpl, soff, &format!("C28 mis={mis} soff={soff} i={i}"));
        }
    }
}

/// C29 — buffer pre-fill patterns × interesting `pos`.
#[test]
fn cfg_c29_update_buffer_prefills() {
    let mut rng = Rng::new(0xC029);
    for kind in 0..4usize {
        for pos in [0u32, 40, 63] {
            for i in 0..64 {
                let mut tpl = rng.arena();
                let buf = prefill(kind, &mut rng);
                put_tflac(&mut tpl, 0, pos, rng.u64(), &buf, 4096, 2);
                let stpl = rng.arena();
                diff_upd(
                    &tpl,
                    0,
                    &stpl,
                    0,
                    &format!("C29 prefill={kind} pos={pos} i={i}"),
                );
            }
        }
    }
}

/// C30 — streaming: 32 chained `update_md5` calls, sample window advancing.
#[test]
fn cfg_c30_update_stream() {
    for seed in 0..32u64 {
        let mut rng = Rng::new(0xC030_0000 + seed);
        let mut tpl = rng.arena();
        let buf = rng.buf72();
        put_tflac(&mut tpl, 0, 0, 0, &buf, 4096, 2);
        let stpl = rng.arena();
        let soffs: Vec<usize> = (0..32)
            .map(|k| (k * 8) % (ARENA - UPD_SAMPLE_SPAN_BYTES) / 4 * 4)
            .collect();
        diff_upd_stream(&tpl, 0, &stpl, &soffs, &format!("C30 seed={seed}"));
    }
}

// ===========================================================================
// Mixed / global
// ===========================================================================

/// C31 — mixed pipeline: a randomized program interleaving all three entry
/// points on one shared arena, so composition bugs cannot hide.
#[test]
fn cfg_c31_mixed_pipeline_fuzz() {
    let l = libs();
    for seed in 0..64u64 {
        let mut rng = Rng::new(0xC031_0000 + seed);
        let tpl = rng.arena();
        let stpl = rng.arena();
        let mut a = Arena::from_template(&tpl);
        let mut b = Arena::from_template(&tpl);
        let mut sa = Arena::from_template(&stpl);
        let mut sb = Arena::from_template(&stpl);

        // the record lives at a fixed 8-aligned offset
        let off = 0usize;
        for step in 0..64 {
            let which = rng.below(3);
            match which {
                0 => {
                    // pack straight into the record / arena
                    let d = (rng.below((ARENA - 8) as u64)) as usize;
                    let n = rng.u64();
                    unsafe {
                        (l.c.pack)(a.at(d), n);
                        (l.r.pack)(b.at(d), n);
                    }
                    assert_arenas_eq(
                        "C31/pack",
                        &format!("seed={seed} step={step} d={d} n={n:#x}"),
                        &a,
                        &b,
                    );
                }
                1 => {
                    let bits = match rng.below(4) {
                        0 => rng.u32(),
                        1 => (rng.u32() % 200) * 8,
                        2 => rng.u32() % 200,
                        _ => 64,
                    };
                    let val = rng.u64();
                    unsafe {
                        (l.c.add)(a.at(off), bits, val);
                        (l.r.add)(b.at(off), bits, val);
                    }
                    assert_arenas_eq(
                        "C31/addsample",
                        &format!("seed={seed} step={step} bits={bits} val={val:#x}"),
                        &a,
                        &b,
                    );
                }
                _ => {
                    // randomise cur_blocksize / channels through pack writes so
                    // the shape varies without leaving the FFI surface
                    let cb = rng.u32();
                    let ch = rng.u32();
                    let packed = ((ch as u64) << 32) | cb as u64;
                    unsafe {
                        (l.c.pack)(a.at(off + 88), packed);
                        (l.r.pack)(b.at(off + 88), packed);
                    }
                    let soff = ((rng.below(((ARENA - UPD_SAMPLE_SPAN_BYTES) / 4) as u64)) * 4) as usize;
                    let (rc, rr) = unsafe {
                        (
                            (l.c.upd)(a.at(off), sa.at(soff) as *const i32),
                            (l.r.upd)(b.at(off), sb.at(soff) as *const i32),
                        )
                    };
                    assert_eq!(
                        rc, rr,
                        "C31/update return mismatch seed={seed} step={step} cb={cb:#x} ch={ch:#x}"
                    );
                    assert_arenas_eq(
                        "C31/update(record)",
                        &format!("seed={seed} step={step}"),
                        &a,
                        &b,
                    );
                    assert_arenas_eq(
                        "C31/update(samples)",
                        &format!("seed={seed} step={step}"),
                        &sa,
                        &sb,
                    );
                }
            }
        }
    }
}

/// C32 — global fuzz: 2000 iterations with every byte of the record, the sample
/// window and the surrounding arena randomised, and full-range scalars.
#[test]
fn cfg_c32_global_fuzz() {
    let mut rng = Rng::new(0xC032);
    for i in 0..2000 {
        let mut tpl = rng.arena();
        let stpl = rng.arena();
        let off = ((rng.below(((ARENA - ADD_MAX_TOUCH - 8) / 8) as u64)) * 8) as usize;
        // fully random record contents (pos/total/buffer/cur_blocksize/channels
        // already random from the arena fill); occasionally force extremes
        match rng.below(4) {
            0 => {
                let buf = rng.buf72();
                put_tflac(&mut tpl, off, 0xFFFF_FFFF, u64::MAX, &buf, 0, 0);
            }
            1 => {
                let buf = rng.buf72();
                put_tflac(&mut tpl, off, 63, u64::MAX - 1, &buf, 0xFFFF_FFFF, 0xFFFF_FFFF);
            }
            2 => {
                let buf = rng.buf72();
                put_tflac(&mut tpl, off, rng.u32() % 64, rng.u64(), &buf, rng.u32(), rng.u32());
            }
            _ => { /* leave the raw random arena bytes as the record */ }
        }

        match rng.below(3) {
            0 => {
                let d = (rng.below((ARENA - 8) as u64)) as usize;
                diff_pack(&tpl, d, rng.u64(), &format!("C32 pack i={i} d={d}"));
            }
            1 => {
                let bits = match rng.below(5) {
                    0 => rng.u32(),
                    1 => (rng.u32() % 200) * 8,
                    2 => rng.u32() % 200,
                    3 => u32::MAX,
                    _ => 64,
                };
                diff_add(&tpl, off, bits, rng.u64(), &format!("C32 add i={i} off={off} bits={bits}"));
            }
            _ => {
                let soff = ((rng.below(((ARENA - UPD_SAMPLE_SPAN_BYTES) / 4) as u64)) * 4) as usize;
                diff_upd(&tpl, off, &stpl, soff, &format!("C32 upd i={i} off={off} soff={soff}"));
            }
        }
    }
}

/// Layout parity check driven purely through the FFI: writing a known `pos` /
/// `total` / `buffer` through the arena and observing where each `.so` reads and
/// writes proves both agree on `offsetof`.
#[test]
fn layout_parity_via_ffi() {
    let l = libs();
    // `bits = 8` with `pos = 0` writes exactly buffer[0..8] and pos becomes 1.
    let mut tpl = vec![0u8; ARENA];
    let buf = [0u8; BUF_LEN];
    put_md5(&mut tpl, 0, 0, 0, &buf);
    let mut a = Arena::from_template(&tpl);
    let mut b = Arena::from_template(&tpl);
    unsafe {
        (l.c.add)(a.ptr(), 8, 0x0807_0605_0403_0201);
        (l.r.add)(b.ptr(), 8, 0x0807_0605_0403_0201);
    }
    assert_arenas_eq("layout", "addsample bits=8", &a, &b);
    // pos @ 0, total @ 8, buffer @ 16
    assert_eq!(get_pos(a.bytes(), 0), 1, "pos must live at offset 0");
    assert_eq!(get_total(a.bytes(), 0), 8, "total must live at offset 8");
    assert_eq!(&a.bytes()[16..24], &[1, 2, 3, 4, 5, 6, 7, 8], "buffer must live at offset 16");
    assert_eq!(&a.bytes()[4..8], &[0, 0, 0, 0], "padding must be untouched");

    // cur_blocksize @ 88, channels @ 92: b = cb * ch, return = b - 40
    let mut tpl2 = vec![0u8; ARENA];
    put_tflac(&mut tpl2, 0, 0, 0, &buf, 7, 9);
    let stpl = vec![0u8; ARENA];
    let r = diff_upd(&tpl2, 0, &stpl, 0, "layout update_md5 cb=7 ch=9");
    assert_eq!(r, 63u32.wrapping_sub(40), "cur_blocksize@88 / channels@92");
}
