//! Phase C — error-path differential tests, one test per row of `ERRORS.md`.
//!
//! Every test constructs the exact rejecting input, calls BOTH shared objects
//! through their exported `read_side_info` symbol and asserts that they return
//! the *same* sentinel (`-1`) / the same truncated state, not merely that both
//! "failed somehow".

mod common;

use common::*;
use std::ffi::c_int;

const ITERS: u32 = 200;

fn hdr_for(rng: &mut Rng, mpeg1: bool, mono: bool, i: u32) -> [u8; 4] {
    let sr = if mpeg1 { 2 + (i % 7) } else { i % 6 } as i32;
    let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
    make_hdr(rng, mpeg1, mono, srate, bit4)
}

// ---------------------------------------------------------------------------
// Row 1: the very first get_bits read overruns bs->limit
// ---------------------------------------------------------------------------

#[test]
fn err_01_get_bits_truncates_first_read() {
    let ctx = "err_01";
    let mut rng = Rng::new(0xc001);
    for i in 0..ITERS {
        let mpeg1 = i % 2 == 0;
        let hdr = hdr_for(&mut rng, mpeg1, i % 3 == 0, i);
        let (mut case, _si) = build(&mut rng, hdr, |_r, _si| {});
        // first read is 9 bits (MPEG1) or 8+gr_count bits (MPEG2)
        let first = if mpeg1 { 9 } else { 8 + gr_count(&hdr) };
        case.limit = rng.below(first as u32) as c_int; // 0 .. first-1
        let out = diff(&case, &format!("{ctx} iter={i} limit={}", case.limit));
        assert_eq!(out.ret, -1, "{ctx}: truncated stream must be rejected");
        // every decoded field must be zero
        let g = gr_count(&hdr);
        for k in 0..g {
            assert_eq!(gr_u16(&out.gr, k, O_PART_23_LENGTH), 0);
            assert_eq!(gr_u16(&out.gr, k, O_BIG_VALUES), 0);
            assert_eq!(out.gr[k * GR_SIZE + O_GLOBAL_GAIN], 0);
        }
        // pos is advanced even though nothing was read
        assert!(out.bs.pos > case.limit, "{ctx}: pos must still advance");
    }
}

// ---------------------------------------------------------------------------
// Row 2: truncation at every possible cut point
// ---------------------------------------------------------------------------

#[test]
fn err_02_get_bits_truncates_midstream() {
    let ctx = "err_02";
    let mut rng = Rng::new(0xc002);
    for &(mpeg1, mono) in &[(false, true), (false, false), (true, true), (true, false)] {
        for rep in 0..3u32 {
            let hdr = hdr_for(&mut rng, mpeg1, mono, rep);
            let (mut case, _si) = build(&mut rng, hdr, |r, si| {
                for g in si.gr.iter_mut() {
                    g.window = r.bits(1);
                    g.block_type = 1 + r.below(3);
                    g.mixed = r.bits(1);
                }
            });
            let pos_end = probe_pos_end(&case);
            for cut in 0..=pos_end {
                case.limit = cut;
                let out = diff(&case, &format!("{ctx} mpeg1={mpeg1} mono={mono} cut={cut}"));
                if cut < pos_end {
                    // something was truncated; the C always ends up rejecting
                    // because part_23_sum + pos > limit (+ mdb*8) in these cases
                    // OR because block_type decoded as 0. Either way both libs
                    // must agree, which `diff` already asserted. Just record.
                    let _ = out.ret;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 3: negative bs->limit
// ---------------------------------------------------------------------------

#[test]
fn err_03_negative_limit() {
    let ctx = "err_03";
    let mut rng = Rng::new(0xc003);
    for &limit in &[-1i32, -2, -1000, i32::MIN + 1, i32::MIN] {
        for i in 0..40u32 {
            let mpeg1 = i % 2 == 0;
            let hdr = hdr_for(&mut rng, mpeg1, i % 3 == 0, i);
            let (mut case, _si) = build(&mut rng, hdr, |_r, _si| {});
            case.limit = limit;
            let out = diff(&case, &format!("{ctx} limit={limit} iter={i}"));
            assert_eq!(out.ret, -1, "{ctx}: negative limit must be rejected");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 4: zero readable bits (pos == limit)
// ---------------------------------------------------------------------------

#[test]
fn err_04_zero_readable_bits() {
    let ctx = "err_04";
    let mut rng = Rng::new(0xc004);
    for p in [0i32, 1, 7, 8, 17, 64, -1, -9] {
        for i in 0..20u32 {
            let mpeg1 = i % 2 == 0;
            let hdr = hdr_for(&mut rng, mpeg1, i % 3 == 0, i);
            let (mut case, _si) = build(&mut rng, hdr, |_r, _si| {});
            case.pos = p;
            case.limit = p;
            let out = diff(&case, &format!("{ctx} pos=limit={p} iter={i}"));
            assert_eq!(out.ret, -1, "{ctx}: zero readable bits must be rejected");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 5: pos + n == limit is NOT an error (`>` and not `>=`)
// ---------------------------------------------------------------------------

#[test]
fn err_05_limit_boundary_exact() {
    let ctx = "err_05";
    let mut rng = Rng::new(0xc005);
    for i in 0..ITERS {
        let mpeg1 = i % 2 == 0;
        let hdr = hdr_for(&mut rng, mpeg1, i % 3 == 0, i);
        // `main_data_begin` >= 64 so that `limit + mdb*8` stays above `pos_end`
        // and the *final* range check passes: then the return value is exactly
        // `main_data_begin`, which proves the first read was NOT truncated even
        // though `pos + n == limit` (a truncated read would have yielded 0 and
        // hence -1).
        let mdb_want = 64 + rng.below(192);
        let (mut case, si) = build(&mut rng, hdr, |_r, si| {
            si.main_data_begin = if mpeg1 {
                mdb_want
            } else {
                mdb_want << gr_count(&hdr) as u32
            };
        });
        let mdb = main_data_begin_value(&hdr, &si);
        assert_eq!(mdb as u32, mdb_want);
        // exactly enough bits for the first field, not one more
        let first = if mpeg1 { 9 } else { 8 + gr_count(&hdr) } as c_int;
        case.limit = first;
        let out = diff(&case, &format!("{ctx} iter={i}"));
        assert_eq!(
            out.ret, mdb,
            "{ctx}: pos+n == limit must NOT truncate (the check is `>` not `>=`)"
        );
        assert!(out.bs.pos > case.limit, "{ctx}: later reads did truncate");
    }
    // And the positive control: limit large enough for the whole stream minus
    // nothing at all -> success (already covered by cfg_28, repeated here so the
    // ERRORS.md row has its own passing assertion).
    let mut rng = Rng::new(0xc105);
    for i in 0..40u32 {
        let mpeg1 = i % 2 == 0;
        let hdr = hdr_for(&mut rng, mpeg1, i % 3 == 0, i);
        let (mut case, _si) = build(&mut rng, hdr, |_r, si| {
            for g in si.gr.iter_mut() {
                g.part_23_length = 0;
                g.window = 0;
            }
        });
        let pos_end = probe_pos_end(&case);
        case.limit = pos_end;
        let out = diff(&case, &format!("{ctx} exact iter={i}"));
        assert!(out.ret >= 0, "{ctx}: limit == pos_end must succeed, got {}", out.ret);
    }
}

// ---------------------------------------------------------------------------
// Row 6: bs->pos integer overflow
// ---------------------------------------------------------------------------

#[test]
fn err_06_pos_int_overflow() {
    let ctx = "err_06";
    let mut rng = Rng::new(0xc006);
    for i in 0..40u32 {
        let mpeg1 = i % 2 == 0;
        let hdr = hdr_for(&mut rng, mpeg1, i % 3 == 0, i);
        let (mut case, _si) = build(&mut rng, hdr, |_r, _si| {});
        case.pos = i32::MAX;
        case.limit = i32::MIN;
        let out = diff(&case, &format!("{ctx} iter={i}"));
        assert_eq!(out.ret, -1, "{ctx}");
        assert!(out.bs.pos < 0, "{ctx}: pos must have wrapped, got {}", out.bs.pos);
    }
    // A few more wrap-around shapes, all chosen so that no dereference happens.
    for &(pos, limit) in &[
        (i32::MAX, i32::MIN),
        (i32::MAX - 1, i32::MIN),
        (i32::MAX, i32::MIN + 1),
        (i32::MIN, i32::MIN),
    ] {
        for i in 0..8u32 {
            let hdr = hdr_for(&mut rng, i % 2 == 0, i % 3 == 0, i);
            let (mut case, _si) = build(&mut rng, hdr, |_r, _si| {});
            case.pos = pos;
            case.limit = limit;
            diff(&case, &format!("{ctx} pos={pos} limit={limit}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 7-9: big_values range check
// ---------------------------------------------------------------------------

#[test]
fn err_07_big_values_gt_288_granule0() {
    let ctx = "err_07";
    let mut rng = Rng::new(0xc007);
    for bv in 289..=511u32 {
        let mpeg1 = bv % 2 == 0;
        let hdr = hdr_for(&mut rng, mpeg1, bv % 3 == 0, bv);
        let (case, _si) = build(&mut rng, hdr, |r, si| {
            si.gr[0].big_values = bv;
            si.gr[0].window = r.bits(1);
            si.gr[0].block_type = 1 + r.below(3);
        });
        let out = diff(&case, &format!("{ctx} bv={bv}"));
        assert_eq!(out.ret, -1, "{ctx}: big_values={bv} must be rejected");
        assert_eq!(gr_u16(&out.gr, 0, O_BIG_VALUES) as u32, bv);
        // nothing after big_values may have been written
        assert_eq!(
            out.gr[O_GLOBAL_GAIN],
            case.fill[O_GLOBAL_GAIN],
            "{ctx}: global_gain must be untouched"
        );
        assert_eq!(
            &out.gr[GR_SIZE..],
            &case.fill[GR_SIZE..],
            "{ctx}: later granules must be untouched"
        );
    }
}

#[test]
fn err_08_big_values_gt_288_late_granule() {
    let ctx = "err_08";
    let mut rng = Rng::new(0xc008);
    for &(mpeg1, mono) in &[(false, false), (true, true), (true, false)] {
        let probe = hdr_for(&mut rng, mpeg1, mono, 0);
        let g = gr_count(&probe);
        for k in 1..g {
            for rep in 0..30u32 {
                let hdr = hdr_for(&mut rng, mpeg1, mono, rep);
                let bv = 289 + rng.below(223);
                let (case, _si) = build(&mut rng, hdr, |r, si| {
                    for gg in si.gr.iter_mut() {
                        gg.window = r.bits(1);
                        gg.block_type = 1 + r.below(3);
                    }
                    si.gr[k].big_values = bv;
                });
                let out = diff(&case, &format!("{ctx} k={k} bv={bv} rep={rep}"));
                assert_eq!(out.ret, -1, "{ctx}: granule {k} big_values={bv}");
                assert_eq!(gr_u16(&out.gr, k, O_BIG_VALUES) as u32, bv);
                for later in (k + 1)..N_GR {
                    assert_eq!(
                        &out.gr[later * GR_SIZE..(later + 1) * GR_SIZE],
                        &case.fill[later * GR_SIZE..(later + 1) * GR_SIZE],
                        "{ctx}: gr[{later}] must be untouched"
                    );
                }
            }
        }
    }
}

#[test]
fn err_09_big_values_288_ok() {
    let ctx = "err_09";
    let mut rng = Rng::new(0xc009);
    for i in 0..ITERS {
        let mpeg1 = i % 2 == 0;
        let hdr = hdr_for(&mut rng, mpeg1, i % 3 == 0, i);
        let (case, _si) = build(&mut rng, hdr, |r, si| {
            for gg in si.gr.iter_mut() {
                gg.big_values = 288;
                gg.window = r.bits(1);
                gg.block_type = 1 + r.below(3);
            }
        });
        let out = diff(&case, &format!("{ctx} iter={i}"));
        assert!(out.ret >= 0, "{ctx}: big_values == 288 must be accepted");
    }
}

// ---------------------------------------------------------------------------
// Rows 10-11: block_type == 0 on the window-switching path
// ---------------------------------------------------------------------------

#[test]
fn err_10_block_type_zero() {
    let ctx = "err_10";
    let mut rng = Rng::new(0xc00a);
    for &(mpeg1, mono) in &[(false, true), (false, false), (true, true), (true, false)] {
        let probe = hdr_for(&mut rng, mpeg1, mono, 0);
        let g = gr_count(&probe);
        for k in 0..g {
            for rep in 0..30u32 {
                let hdr = hdr_for(&mut rng, mpeg1, mono, rep);
                let (case, _si) = build(&mut rng, hdr, |r, si| {
                    for gg in si.gr.iter_mut() {
                        gg.window = 1;
                        gg.block_type = 1 + r.below(3);
                        gg.mixed = r.bits(1);
                    }
                    si.gr[k].block_type = 0;
                });
                let out = diff(&case, &format!("{ctx} k={k} rep={rep}"));
                assert_eq!(out.ret, -1, "{ctx}: block_type 0 must be rejected");
                let b = &out.gr[k * GR_SIZE..(k + 1) * GR_SIZE];
                assert_eq!(b[O_BLOCK_TYPE], 0);
                assert_eq!(b[O_N_LONG_SFB], 22, "{ctx}: long table defaults kept");
                assert_eq!(b[O_N_SHORT_SFB], 0);
                assert_eq!(
                    b[O_MIXED_BLOCK_FLAG],
                    case.fill[k * GR_SIZE + O_MIXED_BLOCK_FLAG],
                    "{ctx}: mixed_block_flag must be untouched"
                );
                assert_eq!(
                    &b[O_TABLE_SELECT..O_TABLE_SELECT + 3],
                    &case.fill[k * GR_SIZE + O_TABLE_SELECT..k * GR_SIZE + O_TABLE_SELECT + 3],
                    "{ctx}: table_select must be untouched"
                );
                for later in (k + 1)..N_GR {
                    assert_eq!(
                        &out.gr[later * GR_SIZE..(later + 1) * GR_SIZE],
                        &case.fill[later * GR_SIZE..(later + 1) * GR_SIZE],
                        "{ctx}: gr[{later}] untouched"
                    );
                }
            }
        }
    }
}

#[test]
fn err_11_block_type_zero_by_truncation() {
    let ctx = "err_11";
    let mut rng = Rng::new(0xc00b);
    for &(mpeg1, mono) in &[(false, true), (false, false), (true, true), (true, false)] {
        let probe = hdr_for(&mut rng, mpeg1, mono, 0);
        let g = gr_count(&probe);
        for k in 0..g {
            for rep in 0..20u32 {
                let hdr = hdr_for(&mut rng, mpeg1, mono, rep);
                let (mut case, _si) = build(&mut rng, hdr, |r, si| {
                    for gg in si.gr.iter_mut() {
                        gg.window = 1;
                        gg.block_type = 1 + r.below(3);
                        gg.mixed = r.bits(1);
                        gg.part_23_length = 0;
                    }
                });
                // cut the limit exactly before granule k's block_type field
                let off = block_type_bit_offset_all_w1(&hdr, k) as c_int;
                case.limit = case.pos + off;
                let out = diff(&case, &format!("{ctx} k={k} rep={rep} off={off}"));
                assert_eq!(out.ret, -1, "{ctx}");
                assert_eq!(
                    out.gr[k * GR_SIZE + O_BLOCK_TYPE],
                    0,
                    "{ctx}: truncated block_type must decode as 0"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 12-14: the final part_23_sum range check
// ---------------------------------------------------------------------------

#[test]
fn err_12_part23_sum_overrun() {
    let ctx = "err_12";
    let mut rng = Rng::new(0xc00c);
    for i in 0..ITERS {
        let mpeg1 = i % 2 == 0;
        let hdr = hdr_for(&mut rng, mpeg1, i % 3 == 0, i);
        let (mut case, _si) = build(&mut rng, hdr, |r, si| {
            si.main_data_begin = 0;
            for gg in si.gr.iter_mut() {
                gg.part_23_length = 4095;
                gg.window = r.bits(1);
                gg.block_type = 1 + r.below(3);
            }
        });
        case.limit = 500 + rng.below(1000) as c_int;
        let out = diff(&case, &format!("{ctx} iter={i}"));
        assert_eq!(out.ret, -1, "{ctx}: part_23_sum overrun must be rejected");
        // every granule was fully decoded before the rejection
        for k in 0..gr_count(&hdr) {
            assert_eq!(gr_u16(&out.gr, k, O_PART_23_LENGTH), 4095);
        }
    }
}

#[test]
fn err_13_part23_sum_exact_boundary() {
    let ctx = "err_13";
    let mut rng = Rng::new(0xc00d);
    let mut hits = 0;
    for i in 0..ITERS {
        let mpeg1 = i % 2 == 0;
        let hdr = hdr_for(&mut rng, mpeg1, i % 3 == 0, i);
        let g = gr_count(&hdr);
        let (mut case, mut si) = build(&mut rng, hdr, |r, si| {
            si.main_data_begin = 0;
            for gg in si.gr.iter_mut() {
                gg.part_23_length = 0;
                gg.window = r.bits(1);
                gg.block_type = 1 + r.below(3);
            }
        });
        let pos_end = probe_pos_end(&case);
        case.limit = 600 + rng.below(200) as c_int;
        // part_23_sum + pos_end == limit + 0  =>  exactly on the boundary
        let target = case.limit - pos_end;
        if target < 0 || target > 4095 * g as i32 {
            continue;
        }
        let mut left = target;
        for gg in si.gr.iter_mut() {
            let take = left.min(4095);
            gg.part_23_length = take as u32;
            left -= take;
        }
        case.put_side_info(&si);
        let out = diff(&case, &format!("{ctx} iter={i}"));
        assert!(
            out.ret >= 0,
            "{ctx}: exact boundary must be accepted (check is `>`), got {}",
            out.ret
        );
        hits += 1;
    }
    assert!(hits > 20, "{ctx}: only {hits} usable iterations");
}

#[test]
fn err_14_limit_plus_mdb_overflow() {
    let ctx = "err_14";
    let mut rng = Rng::new(0xc00e);
    for i in 0..80u32 {
        let hdr = hdr_for(&mut rng, true, i % 3 == 0, i);
        let (mut case, si) = build(&mut rng, hdr, |_r, si| {
            si.main_data_begin = 511; // 9 bits, MPEG1 => mdb == 511
            for gg in si.gr.iter_mut() {
                gg.part_23_length = 0;
                gg.window = 0;
            }
        });
        assert_eq!(main_data_begin_value(&hdr, &si), 511);
        // limit + 511*8 overflows int => wraps negative => rejected
        case.limit = i32::MAX;
        let out = diff(&case, &format!("{ctx} overflow iter={i}"));
        assert_eq!(out.ret, -1, "{ctx}: signed overflow must reject");
        // control: one less than the overflow point => accepted
        case.limit = i32::MAX - 511 * 8;
        let out = diff(&case, &format!("{ctx} control iter={i}"));
        assert_eq!(out.ret, 511, "{ctx}: no overflow => success");
    }
}

// ---------------------------------------------------------------------------
// Row 15: negative bs->pos
// ---------------------------------------------------------------------------

#[test]
fn err_15_negative_pos() {
    let ctx = "err_15";
    let mut rng = Rng::new(0xc00f);
    for p in -(8 * 100)..0i32 {
        if p % 7 != 0 {
            continue;
        }
        for rep in 0..3u32 {
            let mpeg1 = rep % 2 == 0;
            let hdr = hdr_for(&mut rng, mpeg1, rep % 3 == 0, rep);
            let (mut case, _si) = build(&mut rng, hdr, |r, si| {
                for gg in si.gr.iter_mut() {
                    gg.window = r.bits(1);
                    gg.block_type = 1 + r.below(3);
                }
            });
            case.pos = p;
            case.limit = match rep % 3 {
                0 => AMPLE_LIMIT,
                1 => p + 40,
                _ => -1,
            };
            case.put_side_info(&_si);
            diff(&case, &format!("{ctx} pos={p} rep={rep}"));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 16: sr_idx == 8 out-of-bounds table rows
// ---------------------------------------------------------------------------

#[test]
fn err_16_sr_idx_8_oob_rows() {
    let ctx = "err_16";
    let mut rng = Rng::new(0xc010);
    let (srate, bit4) = hdr_bits_for_sr_idx(true, 8).unwrap();

    // The exact bytes the C's out-of-bounds reads alias onto, derived from the
    // declaration order of the three function-local static tables:
    //   &g_scf_long[8]  == 8 pad bytes + g_scf_short[0][0..15]
    //   &g_scf_short[8] == g_scf_mixed[0]
    let long8: Vec<u8> = {
        let mut v = vec![0u8; 8];
        v.extend_from_slice(&[4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 8]);
        v
    };
    let short8: Vec<u8> = vec![
        6, 6, 6, 6, 6, 6, 6, 6, 6, 8, 8, 8, 10, 10, 10, 12, 12, 12, 14, 14, 14, 18, 18, 18, 24, 24,
        24, 30, 30, 30, 40, 40, 40, 18, 18, 18, 0, 0, 0, 0,
    ];

    for i in 0..60u32 {
        // (a) long table row 8
        let hdr = make_hdr(&mut rng, true, i % 2 == 0, srate, bit4);
        assert_eq!(sr_idx(&hdr), 8);
        let (case, _si) = build(&mut rng, hdr, |_r, si| set_window(si, 0));
        let out = diff(&case, &format!("{ctx}a iter={i}"));
        let got = unsafe { std::slice::from_raw_parts(out.ptrs[0] as *const u8, 23) };
        assert_eq!(got, &long8[..], "{ctx}a: &g_scf_long[8] aliasing");

        // (b) short table row 8 == g_scf_mixed[0]
        let (case, _si) = build(&mut rng, hdr, |_r, si| {
            for gg in si.gr.iter_mut() {
                gg.window = 1;
                gg.block_type = 2;
                gg.mixed = 0;
            }
        });
        let out = diff(&case, &format!("{ctx}b iter={i}"));
        let got = unsafe { std::slice::from_raw_parts(out.ptrs[0] as *const u8, 40) };
        assert_eq!(got, &short8[..], "{ctx}b: &g_scf_short[8] aliasing");

        // (c) mixed table row 8: past the end of .rodata. Only the byte contents
        // are unreproducible; every other output must still match, and the
        // pointer must sit exactly 320 bytes after `&g_scf_short[8]`
        // (== `&g_scf_mixed[0]`) in *both* libraries. Granule 0 selects
        // `&g_scf_mixed[8]` and granule 1 selects `&g_scf_short[8]`, so
        // `compare()`'s cross-granule pointer-delta check proves the offset.
        let hdr2 = make_hdr(&mut rng, true, false, srate, bit4);
        assert_eq!(sr_idx(&hdr2), 8);
        assert!(gr_count(&hdr2) >= 2);
        let (case, _si) = build(&mut rng, hdr2, |_r, si| {
            for gg in si.gr.iter_mut() {
                gg.window = 1;
                gg.block_type = 2;
                gg.mixed = 0;
            }
            si.gr[0].mixed = 1;
        });
        let out = diff(&case, &format!("{ctx}c iter={i}"));
        assert_eq!(
            out.ptrs[0].wrapping_sub(out.ptrs[1]),
            320,
            "{ctx}c: &g_scf_mixed[8] must be 320 bytes past &g_scf_short[8]"
        );
        let got = unsafe { std::slice::from_raw_parts(out.ptrs[1] as *const u8, 40) };
        assert_eq!(got, &short8[..], "{ctx}c: &g_scf_short[8] aliasing");
    }
}

// ---------------------------------------------------------------------------
// Rows 17-20: null pointers (no checks in the C at all => must fault the same)
// ---------------------------------------------------------------------------

mod nullptr {
    use super::*;
    use std::os::unix::process::ExitStatusExt;
    use std::process::Command;

    fn child_body(which: &str, which_null: &str) -> ! {
        let l = libs();
        let lib = if which == "c" { &l.c } else { &l.rust };
        let buf = vec![0u8; BUF_LEN];
        let fill = vec![0u8; N_GR * GR_SIZE];
        let mut gr = GrArray::new(&fill);
        // MPEG1 + mono, sr_idx == 2, ample limit: guarantees the C reaches the
        // first `gr->...` store and the first `bs->buf` load.
        let hdr = [0x00u8, 0x08, 0x00, 0xC0];
        let mut bs = BsT {
            buf: unsafe { buf.as_ptr().add(BUF_MID) },
            pos: 0,
            limit: AMPLE_LIMIT,
        };
        let ret = unsafe {
            match which_null {
                "bs" => lib.read_side_info(std::ptr::null_mut(), gr.as_mut_ptr(), hdr.as_ptr()),
                "gr" => lib.read_side_info(&mut bs, std::ptr::null_mut(), hdr.as_ptr()),
                "hdr" => lib.read_side_info(&mut bs, gr.as_mut_ptr(), std::ptr::null()),
                "buf" => {
                    bs.buf = std::ptr::null();
                    lib.read_side_info(&mut bs, gr.as_mut_ptr(), hdr.as_ptr())
                }
                other => panic!("unknown null case {other}"),
            }
        };
        println!("{which}/{which_null} returned {ret}");
        // Distinct exit codes so the parent can tell "returned -1" from
        // "returned something else" from "crashed".
        std::process::exit(if ret == -1 { 20 } else { 21 });
    }

    /// Runs `test_name` in two child processes (one per library) and asserts the
    /// two processes terminate in exactly the same way.
    pub fn expect_identical_termination(which_null: &str, test_name: &str) {
        if let Ok(which) = std::env::var("HARVEST_NULL_CHILD") {
            child_body(&which, which_null);
        }
        let exe = std::env::current_exe().expect("current_exe");
        let mut results = Vec::new();
        for which in ["c", "rust"] {
            let out = Command::new(&exe)
                .arg("--exact")
                .arg(test_name)
                .arg("--nocapture")
                .arg("--test-threads=1")
                .env("HARVEST_NULL_CHILD", which)
                .output()
                .expect("spawn child");
            results.push((which, out.status.code(), out.status.signal()));
        }
        let (_, c_code, c_sig) = results[0];
        let (_, r_code, r_sig) = results[1];
        assert_eq!(
            (c_code, c_sig),
            (r_code, r_sig),
            "{test_name} ({which_null}): C terminated with code={c_code:?} signal={c_sig:?} \
             but Rust with code={r_code:?} signal={r_sig:?}"
        );
        // Sanity: it must be a real termination difference detector, i.e. the C
        // really did crash (SIGSEGV = 11) or really did return.
        assert!(
            c_sig == Some(11) || c_code == Some(20) || c_code == Some(21),
            "{test_name}: unexpected C termination code={c_code:?} signal={c_sig:?}"
        );
    }
}

#[test]
fn err_17_null_bs() {
    nullptr::expect_identical_termination("bs", "err_17_null_bs");
}

#[test]
fn err_18_null_gr() {
    nullptr::expect_identical_termination("gr", "err_18_null_gr");
}

#[test]
fn err_19_null_hdr() {
    nullptr::expect_identical_termination("hdr", "err_19_null_hdr");
}

#[test]
fn err_20_null_bs_buf() {
    nullptr::expect_identical_termination("buf", "err_20_null_bs_buf");
}

// ---------------------------------------------------------------------------
// Row 21: bs->buf == NULL but every read truncates => no fault, -1
// ---------------------------------------------------------------------------

#[test]
fn err_21_null_buf_but_truncated() {
    let ctx = "err_21";
    let mut rng = Rng::new(0xc015);
    for i in 0..80u32 {
        let mpeg1 = i % 2 == 0;
        let hdr = hdr_for(&mut rng, mpeg1, i % 3 == 0, i);
        let (mut case, _si) = build(&mut rng, hdr, |_r, _si| {});
        case.null_buf = true;
        case.pos = 0;
        case.limit = -1; // every get_bits returns before dereferencing `p`
        let out = diff(&case, &format!("{ctx} iter={i}"));
        assert_eq!(out.ret, -1, "{ctx}");
    }
}

// ---------------------------------------------------------------------------
// Row 23: unaligned `bs_t *` / `L3_gr_info_t *` crossing the FFI boundary
// ---------------------------------------------------------------------------

#[test]
fn err_23_unaligned_pointers() {
    let ctx = "err_23";
    let mut rng = Rng::new(0xc017);
    for i in 0..80u32 {
        let mpeg1 = i % 2 == 0;
        let hdr = hdr_for(&mut rng, mpeg1, i % 3 == 0, i);
        let (case, _si) = build(&mut rng, hdr, |r, si| {
            for gg in si.gr.iter_mut() {
                gg.window = r.bits(1);
                gg.block_type = 1 + r.below(3);
            }
        });
        let skew = 1 + (i % 7) as usize; // deliberately never 0 mod 8
        let mut results = Vec::new();
        for lib in [&libs().c, &libs().rust] {
            let buf = case.buf.clone();
            // Unaligned storage for both out-parameters.
            let mut raw_bs = vec![0u8; std::mem::size_of::<BsT>() + 8];
            let mut raw_gr = vec![0u8; N_GR * GR_SIZE + 8];
            raw_gr[skew..skew + N_GR * GR_SIZE].copy_from_slice(&case.fill);
            let bs_ptr = unsafe { raw_bs.as_mut_ptr().add(skew) } as *mut BsT;
            let gr_ptr = unsafe { raw_gr.as_mut_ptr().add(skew) };
            unsafe {
                bs_ptr.write_unaligned(BsT {
                    buf: buf.as_ptr().add(BUF_MID),
                    pos: case.pos,
                    limit: case.limit,
                });
            }
            let ret = unsafe { lib.read_side_info(bs_ptr, gr_ptr, case.hdr.as_ptr()) };
            let bs_after = unsafe { bs_ptr.read_unaligned() };
            let gr_after = raw_gr[skew..skew + N_GR * GR_SIZE].to_vec();
            results.push((ret, bs_after.pos, bs_after.limit, gr_after));
        }
        assert_eq!(results[0].0, results[1].0, "{ctx}: ret (skew={skew})");
        assert_eq!(results[0].1, results[1].1, "{ctx}: pos (skew={skew})");
        assert_eq!(results[0].2, results[1].2, "{ctx}: limit (skew={skew})");
        for k in 0..N_GR {
            let a = &results[0].3[k * GR_SIZE + 8..(k + 1) * GR_SIZE];
            let b = &results[1].3[k * GR_SIZE + 8..(k + 1) * GR_SIZE];
            assert_eq!(a, b, "{ctx}: gr[{k}] bytes 8..32 (skew={skew})");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 22: every value of the 2-bit block_type field
// ---------------------------------------------------------------------------

#[test]
fn err_22_block_type_all_values() {
    let ctx = "err_22";
    let mut rng = Rng::new(0xc016);
    for bt in 0..4u32 {
        for mixed in 0..2u32 {
            for i in 0..40u32 {
                let mpeg1 = i % 2 == 0;
                let hdr = hdr_for(&mut rng, mpeg1, i % 3 == 0, i);
                let (case, _si) = build(&mut rng, hdr, |_r, si| {
                    for gg in si.gr.iter_mut() {
                        gg.window = 1;
                        gg.block_type = bt;
                        gg.mixed = mixed;
                    }
                });
                let out = diff(&case, &format!("{ctx} bt={bt} mixed={mixed} iter={i}"));
                if bt == 0 {
                    assert_eq!(out.ret, -1, "{ctx}: block_type 0 => -1");
                } else {
                    assert!(out.ret >= 0, "{ctx}: block_type {bt} => success");
                }
            }
        }
    }
}
