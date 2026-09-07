//! Phase C — error-path differential tests, one test per ERRORS.md row.
//!
//! Each test constructs the exact invalid input/condition and asserts that the
//! C and the Rust `.so` return the *same* sentinel (`-1` / `0` / the same
//! `main_data_begin`), not merely that "both failed".

mod common;

use common::*;

fn mono_hdr(mpeg1: bool) -> [u8; 4] {
    make_hdr(mpeg1, 0, 1, 3, 0)
}

/// Run both and additionally assert the C returned `expect`, so the test pins
/// the actual sentinel value rather than only "C == Rust".
fn expect_ret(label: &str, buf: &[u8], pos: i32, limit: i32, hdr: &[u8; 4], expect: i32) {
    assert_same(label, buf, pos, limit, hdr);
    let (c, r) = run_both(buf, pos, limit, hdr);
    assert_eq!(c.ret, expect, "[{label}] C returned {} not {expect}", c.ret);
    assert_eq!(r.ret, expect, "[{label}] Rust returned {} not {expect}", r.ret);
}

// ---------------------------------------------------------------------------
// E1 / E2 — get_bits limit behaviour (exercised through read_side_info)
// ---------------------------------------------------------------------------

/// E1: once `pos + n > limit`, `get_bits` returns 0 *and leaves pos advanced*,
/// so every later call also fails. With `limit == pos` on entry, the very first
/// `get_bits` fails and every field must come out of all-zero bits.
#[test]
fn err_e1_get_bits_past_limit_returns_zero_and_advances() {
    let buf = vec![0xFFu8; 512];
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            for pos in 0..16i32 {
                // limit == pos: nothing at all is available.
                assert_same("E1 limit==pos", &buf, pos, pos, &hdr);
                // limit < pos: already overrun on entry.
                assert_same("E1 limit<pos", &buf, pos, pos - 1, &hdr);
                assert_same("E1 limit<<pos", &buf, pos + 100, pos, &hdr);
            }
        }
    }
    // The bit position must still have advanced past the limit in both.
    let hdr = mono_hdr(false);
    let (c, r) = run_both(&buf, 0, 0, &hdr);
    assert_eq!(c, r);
    assert!(c.pos > 0, "pos must be advanced even though every read failed");
}

/// E2: `pos + n == limit` is *not* an error (the test is `>`, not `>=`).
/// A limit of exactly the consumed bit count must succeed.
#[test]
fn err_e2_get_bits_exact_limit_is_not_an_error() {
    let mut rng = Rng::new();
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            for _ in 0..256 {
                let mut grs = [Granule::default(); 4];
                for g in grs.iter_mut() {
                    *g = Granule::random(&mut rng);
                    g.part_23_length = 0; // keep the final check happy
                }
                let (buf, end) = build_side_info(&hdr, 0, 0, 0, rng.next_u32(), &grs, 16);
                // exact limit -> success; one less -> truncation
                expect_ret("E2 exact", &buf, 0, end, &hdr, 0);
                assert_same("E2 one-short", &buf, 0, end - 1, &hdr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E3 — big_values > 288
// ---------------------------------------------------------------------------

#[test]
fn err_e3_big_values_gt_288() {
    let mut rng = Rng::new();
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            let n = gr_count_for(&hdr);
            // Trip the check in each granule position in turn.
            for bad_gr in 0..n {
                for bv in [289u32, 290, 400, 511] {
                    for _ in 0..24 {
                        let mut grs = [Granule::default(); 4];
                        for g in grs.iter_mut() {
                            *g = Granule::random(&mut rng);
                        }
                        grs[bad_gr].big_values = bv;
                        let (buf, end) =
                            build_side_info(&hdr, 0, 0, rng.below(256), rng.next_u32(), &grs, 16);
                        expect_ret(
                            &format!("E3 bad_gr={bad_gr} bv={bv}"),
                            &buf,
                            0,
                            end + 64,
                            &hdr,
                            -1,
                        );
                    }
                }
            }
        }
    }
}

/// One step past the range, and the last valid value, must differ.
#[test]
fn err_generic_one_past_big_values() {
    for &mpeg1 in &[false, true] {
        let hdr = mono_hdr(mpeg1);
        for (bv, want_err) in [(287u32, false), (288, false), (289, true), (290, true)] {
            let mut grs = [Granule::default(); 4];
            for g in grs.iter_mut() {
                g.big_values = bv;
                g.window_switching = true;
                g.block_type = 1;
            }
            let (buf, end) = build_side_info(&hdr, 0, 0, 0, 0, &grs, 16);
            let expect = if want_err { -1 } else { 0 };
            expect_ret(&format!("one_past bv={bv}"), &buf, 0, end + 64, &hdr, expect);
        }
    }
}

// ---------------------------------------------------------------------------
// E4 — block_type == 0 with window switching set
// ---------------------------------------------------------------------------

#[test]
fn err_e4_block_type_zero() {
    let mut rng = Rng::new();
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            let n = gr_count_for(&hdr);
            for bad_gr in 0..n {
                for _ in 0..48 {
                    let mut grs = [Granule::default(); 4];
                    for g in grs.iter_mut() {
                        *g = Granule::random(&mut rng);
                        g.big_values = rng.below(289);
                    }
                    grs[bad_gr].window_switching = true;
                    grs[bad_gr].block_type = 0;
                    let (buf, end) =
                        build_side_info(&hdr, 0, 0, rng.below(256), rng.next_u32(), &grs, 16);
                    expect_ret(&format!("E4 bad_gr={bad_gr}"), &buf, 0, end + 64, &hdr, -1);
                }
            }
        }
    }
}

/// All four raw 2-bit `block_type` values — an "out of range enum" sweep:
/// 0 is rejected, 1/2/3 accepted, and 2 additionally switches tables.
#[test]
fn err_generic_all_block_type_values() {
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            for b in 0..4u32 {
                for m in 0..2u32 {
                    let mut grs = [Granule::default(); 4];
                    for g in grs.iter_mut() {
                        g.window_switching = true;
                        g.block_type = b;
                        g.mixed_block_flag = m;
                    }
                    let (buf, end) = build_side_info(&hdr, 0, 0, 0, 0xFFFF_FFFF, &grs, 16);
                    let expect = if b == 0 { -1 } else { 0 };
                    expect_ret(
                        &format!("block_type={b} mixed={m} mpeg1={mpeg1} chan={chan}"),
                        &buf,
                        0,
                        end + 64,
                        &hdr,
                        expect,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E5 / E6 — final part_23_sum overflow check and its exact boundary
// ---------------------------------------------------------------------------

#[test]
fn err_e5_part23_overflow() {
    let mut rng = Rng::new();
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            let n = gr_count_for(&hdr);
            for _ in 0..128 {
                let mut grs = [Granule::default(); 4];
                for g in grs.iter_mut() {
                    *g = Granule::random(&mut rng);
                    g.big_values = rng.below(289);
                    g.part_23_length = 4095; // maximal -> guaranteed overflow
                }
                let (buf, end) = build_side_info(&hdr, 0, 0, 0, rng.next_u32(), &grs, 16);
                // main_data_begin == 0 and part_23_sum == 4095*n >> available
                assert!(4095 * n as i32 + end > end);
                expect_ret("E5", &buf, 0, end, &hdr, -1);
            }
        }
    }
}

/// E6: `part_23_sum + pos == limit + mdb*8` exactly must NOT be an error.
/// Constructed by solving for the limit given a fixed part_23_sum.
#[test]
fn err_e6_part23_exact_boundary() {
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            let n = gr_count_for(&hdr) as i32;
            for &mdb in &[0u32, 7, 100, if mpeg1 { 511 } else { 255 }] {
                for &per in &[0u32, 1, 37, 1000, 4095] {
                    let mut grs = [Granule::default(); 4];
                    for g in grs.iter_mut() {
                        g.part_23_length = per;
                        g.window_switching = true;
                        g.block_type = 3;
                        g.big_values = 100;
                    }
                    let (buf, end) = build_side_info(&hdr, 0, 0, mdb, 0, &grs, 16);
                    let sum = per as i32 * n;
                    // want: sum + end == limit + mdb*8  ->  limit = sum + end - mdb*8
                    let limit = sum + end - mdb as i32 * 8;
                    if limit < end {
                        // would truncate the parse; not the boundary we want
                        continue;
                    }
                    expect_ret(
                        &format!("E6 exact mdb={mdb} per={per}"),
                        &buf,
                        0,
                        limit,
                        &hdr,
                        mdb as i32,
                    );
                    // one bit tighter -> must now be an error
                    if limit - 1 >= end {
                        expect_ret(
                            &format!("E6 one-tighter mdb={mdb} per={per}"),
                            &buf,
                            0,
                            limit - 1,
                            &hdr,
                            -1,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E7 — success returns main_data_begin
// ---------------------------------------------------------------------------

#[test]
fn err_e7_success_returns_main_data_begin() {
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            let maxmdb = if mpeg1 { 511u32 } else { 255u32 };
            for mdb in 0..=maxmdb {
                let mut grs = [Granule::default(); 4];
                for g in grs.iter_mut() {
                    g.window_switching = false;
                    g.big_values = 288;
                    g.part_23_length = 0;
                }
                let (buf, end) = build_side_info(&hdr, 0, 0, mdb, 0, &grs, 16);
                expect_ret(
                    &format!("E7 mdb={mdb} mpeg1={mpeg1} chan={chan}"),
                    &buf,
                    0,
                    end + 8,
                    &hdr,
                    mdb as i32,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E8 — sr_idx == 8 selects a row one PAST the end of the table
// ---------------------------------------------------------------------------

#[test]
fn err_e8_sr_idx_8_out_of_range_row() {
    // sr_idx == 8 requires mpeg1 (hdr[1]&8), ext bit (hdr[1]&0x10) and
    // sr_bits == 3: 3 + (1+1)*3 = 9, minus 1 => 8.
    let hdr = make_hdr(true, 1, 3, 3, 0);
    assert_eq!(sr_idx_for(&hdr), 8);

    let mut rng = Rng::new();
    // Exactly one of the three tables is last in the C `.rodata`; its row 8
    // runs off the section end and is unreproducible (ERRORS.md N5). The other
    // two rows alias the next table inside the same section and ARE compared
    // byte-for-byte here.
    let skip = unreproducible_row8_kind();
    let kinds: [(&str, bool, u32, u32); 3] = [
        ("long", false, 0, 0),
        ("short", true, 2, 0),
        ("mixed", true, 2, 1),
    ];
    for (label, ws, b, m) in kinds {
        for _ in 0..128 {
            let mut grs = [Granule::default(); 4];
            for g in grs.iter_mut() {
                *g = Granule::random(&mut rng);
                g.window_switching = ws;
                g.block_type = b;
                g.mixed_block_flag = m;
            }
            let (buf, end) = build_side_info(&hdr, 0, 0, 0, rng.next_u32(), &grs, 16);
            if label == skip {
                // Return value, bs->pos and all 32 struct bytes are still
                // compared; only the sfbtab dereference is skipped.
                assert_same(&format!("E8 {label} row 8 (deref skipped)"), &buf, 0, end + 4096, &hdr);
            } else {
                let (c, r) = run_both(&buf, 0, end + 4096, &hdr);
                assert_eq!(c, r, "E8 {label} row 8");
                assert!(c.sfbtab_bytes[0].is_some(), "sfbtab must have been written");
            }
        }
    }
    // Nothing but (mpeg1, ext, sr_bits=3) yields sr_idx == 8.
    for (mpeg1, ext, srb) in [(false, 0u32, 3u32), (false, 1, 3), (true, 0, 3), (true, 1, 2)] {
        let h = make_hdr(mpeg1, ext, srb, 3, 0);
        assert!(sr_idx_for(&h) < 8);
    }
}

// ---------------------------------------------------------------------------
// E9 — zero, oversized and negative limits
// ---------------------------------------------------------------------------

#[test]
fn err_e9_zero_and_negative_limit() {
    let buf = vec![0x5Au8; 512];
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            for &limit in &[0i32, -1, -2, -1000, i32::MIN, i32::MIN + 1] {
                expect_ret(
                    &format!("E9 limit={limit} mpeg1={mpeg1} chan={chan}"),
                    &buf,
                    0,
                    limit,
                    &hdr,
                    // Every get_bits fails => main_data_begin == 0 and
                    // part_23_sum == 0, so the final check is 0 + pos > limit.
                    -1,
                );
            }
        }
    }
}

#[test]
fn err_generic_extreme_limits() {
    let buf = vec![0xC3u8; 4096];
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            for &limit in &[0i32, 1, 7, 8, 9, 4095, 4096, i32::MAX - 1, i32::MAX] {
                assert_same(
                    &format!("extreme limit={limit} mpeg1={mpeg1} chan={chan}"),
                    &buf,
                    0,
                    limit,
                    &hdr,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E10 — negative bs->pos
// ---------------------------------------------------------------------------

#[test]
fn err_e10_negative_pos() {
    let mut rng = Rng::new();
    let mut backing = vec![0u8; 1024];
    rng.fill(&mut backing);
    // Present the middle of the allocation as the buffer so that the negative
    // `pos` (which makes C compute `buf + (pos>>3)`, an arithmetic shift, i.e.
    // a pointer *before* buf) still lands inside our own allocation and both
    // implementations therefore read the same bytes.
    let view = &backing[256..];

    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            for &pos in &[-1i32, -7, -8, -9, -100, -1024] {
                // (a) limit even lower: every get_bits short-circuits, no read.
                expect_ret(
                    &format!("E10 no-read pos={pos}"),
                    view,
                    pos,
                    pos - 1000,
                    &hdr,
                    -1,
                );
                // (b) limit high: C really reads before `buf`.
                assert_same(&format!("E10 read-before pos={pos}"), view, pos, 4000, &hdr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E11 — unaligned start position masks the first byte with 255 >> s
// ---------------------------------------------------------------------------

#[test]
fn err_e11_unaligned_start_pos() {
    let mut rng = Rng::new();
    let mut buf = vec![0u8; 512];
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            for pos in 0..64i32 {
                for _ in 0..8 {
                    rng.fill(&mut buf);
                    assert_same(
                        &format!("E11 pos={pos} mpeg1={mpeg1} chan={chan}"),
                        &buf,
                        pos,
                        pos + rng.range(0, 400),
                        &hdr,
                    );
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// E12 — scalefac_compress >= 500 sets preflag (non-MPEG1 only)
// ---------------------------------------------------------------------------

#[test]
fn err_e12_scalefac_compress_500_sets_preflag() {
    // Non-MPEG1: 9-bit field, so 500..511 is reachable and sets preflag = 1.
    let hdr = mono_hdr(false);
    for sfc in 0..512u32 {
        let mut grs = [Granule::default(); 4];
        for g in grs.iter_mut() {
            g.scalefac_compress = sfc;
            g.window_switching = false;
            g.big_values = 1;
        }
        let (buf, end) = build_side_info(&hdr, 0, 0, 0, 0, &grs, 16);
        assert_same(&format!("E12 sfc={sfc}"), &buf, 0, end + 8, &hdr);
        let (c, _r) = run_both(&buf, 0, end + 8, &hdr);
        // preflag lives at offset 28 of L3_gr_info_t
        let want = if sfc >= 500 { 1u8 } else { 0 };
        assert_eq!(c.gr_bytes[0][28], want, "C preflag for sfc={sfc}");
    }
    // MPEG1: the field is only 4 bits so it can never reach 500; preflag is a
    // real bitstream bit instead.
    let hdr1 = mono_hdr(true);
    for sfc in 0..16u32 {
        for pf in 0..2u32 {
            let mut grs = [Granule::default(); 4];
            for g in grs.iter_mut() {
                g.scalefac_compress = sfc;
                g.preflag_bit = pf;
                g.window_switching = false;
                g.big_values = 1;
            }
            let (buf, end) = build_side_info(&hdr1, 0, 0, 0, 0, &grs, 16);
            assert_same(&format!("E12 mpeg1 sfc={sfc} pf={pf}"), &buf, 0, end + 8, &hdr1);
            let (c, _r) = run_both(&buf, 0, end + 8, &hdr1);
            assert_eq!(c.gr_bytes[0][28], pf as u8, "C preflag bit");
        }
    }
}

/// One step past the 500 threshold.
#[test]
fn err_generic_one_past_scalefac_compress() {
    let hdr = mono_hdr(false);
    for (sfc, want) in [(498u32, 0u8), (499, 0), (500, 1), (501, 1), (511, 1)] {
        let mut grs = [Granule::default(); 4];
        for g in grs.iter_mut() {
            g.scalefac_compress = sfc;
        }
        let (buf, end) = build_side_info(&hdr, 0, 0, 0, 0, &grs, 16);
        assert_same(&format!("one_past sfc={sfc}"), &buf, 0, end + 8, &hdr);
        let (c, r) = run_both(&buf, 0, end + 8, &hdr);
        assert_eq!(c.gr_bytes[0][28], want);
        assert_eq!(r.gr_bytes[0][28], want);
    }
}

// ---------------------------------------------------------------------------
// E13 — block_type == 2 masks scfsi with 0x0F0F
// ---------------------------------------------------------------------------

#[test]
fn err_e13_block_type_2_masks_scfsi() {
    // MPEG1 reads a real scfsi field; walk every value of it for both
    // gr_counts, with and without a block_type==2 granule.
    for chan in [0u32, 3u32] {
        let hdr = make_hdr(true, 0, 1, chan, 0);
        let n = gr_count_for(&hdr);
        let scfsi_bits = 7 + n; // 9 or 11
        for raw in 0..(1u32 << scfsi_bits) {
            for mask_at in [usize::MAX, 0, n - 1] {
                let mut grs = [Granule::default(); 4];
                for (i, g) in grs.iter_mut().enumerate() {
                    g.window_switching = true;
                    g.block_type = if i == mask_at { 2 } else { 1 };
                    g.mixed_block_flag = 0;
                }
                let (buf, end) = build_side_info(&hdr, 0, 0, 0, raw, &grs, 16);
                assert_same(
                    &format!("E13 raw={raw} mask_at={mask_at} chan={chan}"),
                    &buf,
                    0,
                    end + 8,
                    &hdr,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Generic: exhaustive raw header bytes ("out of range enum" values via FFI)
// ---------------------------------------------------------------------------

#[test]
fn err_generic_exhaustive_header_bytes() {
    let mut rng = Rng::new();
    let mut buf = vec![0u8; 512];
    // All 256 values of each header byte, individually, with the others fixed
    // and then randomized. hdr[0] is never read by the C, which this also pins.
    for idx in 0..4usize {
        for v in 0..=255u8 {
            for rep in 0..8 {
                let mut hdr = [0u8; 4];
                if rep > 0 {
                    for b in hdr.iter_mut() {
                        *b = rng.next_u32() as u8;
                    }
                }
                hdr[idx] = v;
                rng.fill(&mut buf);
                let limit = rng.range(0, 600);
                assert_same(
                    &format!("hdr[{idx}]={v} rep={rep}"),
                    &buf,
                    rng.range(0, 16),
                    limit,
                    &hdr,
                );
            }
        }
    }
    // hdr[0] genuinely unused: flipping it must not change anything.
    let mut grs = [Granule::default(); 4];
    for g in grs.iter_mut() {
        *g = Granule::random(&mut rng);
    }
    let base = make_hdr(true, 1, 2, 0, 0);
    let (b, end) = build_side_info(&base, 0, 0, 0, 0, &grs, 16);
    let (ref0, _) = run_both(&b, 0, end + 64, &base);
    for v in 0..=255u8 {
        let mut h = base;
        h[0] = v;
        let (c, r) = run_both(&b, 0, end + 64, &h);
        assert_eq!(c, r, "hdr[0]={v}");
        assert_eq!(c, ref0, "hdr[0] must not affect the result (v={v})");
    }
}

// ---------------------------------------------------------------------------
// Struct/ABI sanity (both sides agree on the layout the C header specifies)
// ---------------------------------------------------------------------------

#[test]
fn struct_layout_matches() {
    assert_eq!(core::mem::size_of::<GrInfo>(), 32);
    assert_eq!(core::mem::align_of::<GrInfo>(), 8);
    assert_eq!(core::mem::size_of::<BsT>(), 16);
    assert_eq!(core::mem::align_of::<BsT>(), 8);
    // Offsets, spelled out (must match SYMBOLS.md).
    let g = GrInfo {
        sfbtab: core::ptr::null(),
        part_23_length: 0,
        big_values: 0,
        scalefac_compress: 0,
        global_gain: 0,
        block_type: 0,
        mixed_block_flag: 0,
        n_long_sfb: 0,
        n_short_sfb: 0,
        table_select: [0; 3],
        region_count: [0; 3],
        subblock_gain: [0; 3],
        preflag: 0,
        scalefac_scale: 0,
        count1_table: 0,
        scfsi: 0,
    };
    let base = &g as *const GrInfo as usize;
    let off = |p: *const u8| p as usize - base;
    assert_eq!(off(&g.part_23_length as *const _ as *const u8), 8);
    assert_eq!(off(&g.big_values as *const _ as *const u8), 10);
    assert_eq!(off(&g.scalefac_compress as *const _ as *const u8), 12);
    assert_eq!(off(&g.global_gain), 14);
    assert_eq!(off(&g.block_type), 15);
    assert_eq!(off(&g.mixed_block_flag), 16);
    assert_eq!(off(&g.n_long_sfb), 17);
    assert_eq!(off(&g.n_short_sfb), 18);
    assert_eq!(off(g.table_select.as_ptr()), 19);
    assert_eq!(off(g.region_count.as_ptr()), 22);
    assert_eq!(off(g.subblock_gain.as_ptr()), 25);
    assert_eq!(off(&g.preflag), 28);
    assert_eq!(off(&g.scalefac_scale), 29);
    assert_eq!(off(&g.count1_table), 30);
    assert_eq!(off(&g.scfsi), 31);
}

/// Fields the C leaves untouched must be left untouched by the Rust too:
/// `region_count[2]` and `subblock_gain[*]` on the short path, and
/// `subblock_gain[*]` / `region_count[2]` in general.
#[test]
fn err_generic_untouched_fields_stay_untouched() {
    for &mpeg1 in &[false, true] {
        let hdr = mono_hdr(mpeg1);
        // window switching -> region_count[2] never written
        let mut grs = [Granule::default(); 4];
        for g in grs.iter_mut() {
            g.window_switching = true;
            g.block_type = 1;
        }
        let (buf, end) = build_side_info(&hdr, 0, 0, 0, 0, &grs, 16);
        let (c, r) = run_both(&buf, 0, end + 8, &hdr);
        assert_eq!(c, r);
        assert_eq!(c.gr_bytes[0][24], GR_FILL, "region_count[2] must be untouched");

        // long path -> subblock_gain never written
        for g in grs.iter_mut() {
            g.window_switching = false;
        }
        let (buf, end) = build_side_info(&hdr, 0, 0, 0, 0, &grs, 16);
        let (c, r) = run_both(&buf, 0, end + 8, &hdr);
        assert_eq!(c, r);
        assert_eq!(&c.gr_bytes[0][25..28], &[GR_FILL; 3], "subblock_gain untouched");
    }
}
