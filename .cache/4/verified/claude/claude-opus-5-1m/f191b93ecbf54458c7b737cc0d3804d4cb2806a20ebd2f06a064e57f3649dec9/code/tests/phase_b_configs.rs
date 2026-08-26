//! Phase B — valid-path differential tests, one test per row of `CONFIGS.md`.
//!
//! Every test drives BOTH shared objects (C and Rust) through their exported
//! `read_side_info` symbol with byte-identical inputs and compares the return
//! value, the mutated `bs_t` and all 32 bytes of all six `L3_gr_info_t` slots.

mod common;

use common::*;

/// Randomized iterations per configuration row.
const ITERS: u32 = 300;

// ---------------------------------------------------------------------------
// Rows 1-4: gr_count / header layout, window switching off, sr_idx swept
// ---------------------------------------------------------------------------

fn row_gr_count(seed: u64, mpeg1: bool, mono: bool, expect_gr: usize, ctx: &str) {
    let mut rng = Rng::new(seed);
    let mut st = Stats::default();
    let sr_list: Vec<i32> = (0..=8).filter(|s| hdr_bits_for_sr_idx(mpeg1, *s).is_some()).collect();
    assert!(!sr_list.is_empty());
    for i in 0..ITERS {
        let want_sr = sr_list[(i as usize) % sr_list.len()];
        let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, want_sr).unwrap();
        let hdr = make_hdr(&mut rng, mpeg1, mono, srate, bit4);
        assert_eq!(gr_count(&hdr), expect_gr, "{ctx}: gr_count");
        assert_eq!(sr_idx(&hdr), want_sr, "{ctx}: sr_idx");
        let (case, _si) = build(&mut rng, hdr, |_r, si| set_window(si, 0));
        let out = diff(&case, &format!("{ctx} iter={i} sr_idx={want_sr}"));
        st.add(out.ret);
        // W == 0 => region_count[2] is always overwritten with 255
        for g in 0..expect_gr {
            assert_eq!(out.gr[g * GR_SIZE + O_REGION_COUNT + 2], 255, "{ctx}: rc[2]");
            assert_eq!(out.gr[g * GR_SIZE + O_BLOCK_TYPE], 0, "{ctx}: block_type");
        }
    }
    st.require_some_ok(ctx);
}

#[test]
fn cfg_01_mpeg2_mono_w0() {
    row_gr_count(0x1001, false, true, 1, "cfg_01");
}

#[test]
fn cfg_02_mpeg2_stereo_w0() {
    row_gr_count(0x1002, false, false, 2, "cfg_02");
}

#[test]
fn cfg_03_mpeg1_mono_w0() {
    row_gr_count(0x1003, true, true, 2, "cfg_03");
}

#[test]
fn cfg_04_mpeg1_stereo_w0() {
    row_gr_count(0x1004, true, false, 4, "cfg_04");
}

// ---------------------------------------------------------------------------
// Rows 5-12: window switching on, every block_type / mixed_block_flag combo
// ---------------------------------------------------------------------------

fn row_window(seed: u64, mpeg1: bool, block_type: u32, mixed: u32, ctx: &str) {
    let mut rng = Rng::new(seed);
    let mut st = Stats::default();
    let sr_list: Vec<i32> = (0..=8).filter(|s| hdr_bits_for_sr_idx(mpeg1, *s).is_some()).collect();
    for i in 0..ITERS {
        let want_sr = sr_list[(i as usize) % sr_list.len()];
        let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, want_sr).unwrap();
        let mono = i % 2 == 0;
        let hdr = make_hdr(&mut rng, mpeg1, mono, srate, bit4);
        let (case, _si) = build(&mut rng, hdr, |_r, si| {
            for g in si.gr.iter_mut() {
                g.window = 1;
                g.block_type = block_type;
                g.mixed = mixed;
            }
        });
        let out = diff(&case, &format!("{ctx} iter={i} sr_idx={want_sr} mono={mono}"));
        st.add(out.ret);
        let n = gr_count(&hdr);
        for g in 0..n {
            let b = &out.gr[g * GR_SIZE..(g + 1) * GR_SIZE];
            assert_eq!(b[O_BLOCK_TYPE] as u32, block_type, "{ctx}: block_type");
            assert_eq!(b[O_MIXED_BLOCK_FLAG] as u32, mixed, "{ctx}: mixed");
            // region_count[2] is NEVER written on the window-switching path
            assert_eq!(
                b[O_REGION_COUNT + 2],
                case.fill[g * GR_SIZE + O_REGION_COUNT + 2],
                "{ctx}: region_count[2] must keep the caller's value"
            );
            if block_type == 2 && mixed == 0 {
                assert_eq!(b[O_REGION_COUNT], 8);
                assert_eq!(b[O_N_LONG_SFB], 0);
                assert_eq!(b[O_N_SHORT_SFB], 39);
            } else if block_type == 2 {
                assert_eq!(b[O_REGION_COUNT], 7);
                assert_eq!(b[O_N_LONG_SFB], if mpeg1 { 8 } else { 6 });
                assert_eq!(b[O_N_SHORT_SFB], 30);
            } else {
                assert_eq!(b[O_REGION_COUNT], 7);
                assert_eq!(b[O_N_LONG_SFB], 22);
                assert_eq!(b[O_N_SHORT_SFB], 0);
            }
        }
    }
    st.require_some_ok(ctx);
}

#[test]
fn cfg_05_mpeg2_w1_bt1() {
    row_window(0x2005, false, 1, 0, "cfg_05");
}

#[test]
fn cfg_06_mpeg2_w1_bt3() {
    row_window(0x2006, false, 3, 1, "cfg_06");
}

#[test]
fn cfg_07_mpeg2_w1_bt2_short() {
    row_window(0x2007, false, 2, 0, "cfg_07");
}

#[test]
fn cfg_08_mpeg2_w1_bt2_mixed() {
    row_window(0x2008, false, 2, 1, "cfg_08");
}

#[test]
fn cfg_09_mpeg1_w1_bt1() {
    row_window(0x2009, true, 1, 1, "cfg_09");
}

#[test]
fn cfg_10_mpeg1_w1_bt3() {
    row_window(0x200a, true, 3, 0, "cfg_10");
}

#[test]
fn cfg_11_mpeg1_w1_bt2_short() {
    row_window(0x200b, true, 2, 0, "cfg_11");
}

#[test]
fn cfg_12_mpeg1_w1_bt2_mixed() {
    row_window(0x200c, true, 2, 1, "cfg_12");
}

// ---------------------------------------------------------------------------
// Row 13: gr_count == 2, exhaustive per-granule (block_type, mixed) cross product
// ---------------------------------------------------------------------------

#[test]
fn cfg_13_two_granules_all_block_type_combos() {
    let ctx = "cfg_13";
    let mut rng = Rng::new(0x3013);
    let mut st = Stats::default();
    for bt0 in [1u32, 2, 3] {
        for mx0 in [0u32, 1] {
            for bt1 in [1u32, 2, 3] {
                for mx1 in [0u32, 1] {
                    for rep in 0..8 {
                        let sr = (rep % 6) as i32;
                        let (srate, bit4) = hdr_bits_for_sr_idx(false, sr).unwrap();
                        let hdr = make_hdr(&mut rng, false, false, srate, bit4);
                        assert_eq!(gr_count(&hdr), 2);
                        let (case, _si) = build(&mut rng, hdr, |_r, si| {
                            si.gr[0].window = 1;
                            si.gr[0].block_type = bt0;
                            si.gr[0].mixed = mx0;
                            si.gr[1].window = 1;
                            si.gr[1].block_type = bt1;
                            si.gr[1].mixed = mx1;
                        });
                        let out = diff(
                            &case,
                            &format!("{ctx} bt=({bt0},{bt1}) mx=({mx0},{mx1}) rep={rep}"),
                        );
                        st.add(out.ret);
                    }
                }
            }
        }
    }
    st.require_some_ok(ctx);
}

// ---------------------------------------------------------------------------
// Row 14: gr_count == 4, random per-granule block types
// ---------------------------------------------------------------------------

#[test]
fn cfg_14_four_granules_random_block_types() {
    let ctx = "cfg_14";
    let mut rng = Rng::new(0x3014);
    let mut st = Stats::default();
    for i in 0..ITERS {
        let sr = 2 + (i % 7) as i32;
        let (srate, bit4) = hdr_bits_for_sr_idx(true, sr).unwrap();
        let hdr = make_hdr(&mut rng, true, false, srate, bit4);
        assert_eq!(gr_count(&hdr), 4);
        let (case, _si) = build(&mut rng, hdr, |r, si| {
            for g in si.gr.iter_mut() {
                g.window = 1;
                g.block_type = 1 + r.below(3);
                g.mixed = r.bits(1);
            }
        });
        st.add(diff(&case, &format!("{ctx} iter={i} sr={sr}")).ret);
    }
    st.require_some_ok(ctx);
}

// ---------------------------------------------------------------------------
// Row 15: heterogeneous window flags across granules
// ---------------------------------------------------------------------------

#[test]
fn cfg_15_heterogeneous_window_flags() {
    let ctx = "cfg_15";
    let mut rng = Rng::new(0x3015);
    let mut st = Stats::default();
    for i in 0..ITERS {
        let mpeg1 = true;
        let sr = 2 + (i % 7) as i32;
        let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
        let hdr = make_hdr(&mut rng, mpeg1, false, srate, bit4);
        let (case, _si) = build(&mut rng, hdr, |r, si| {
            for g in si.gr.iter_mut() {
                g.window = r.bits(1);
                g.block_type = 1 + r.below(3);
                g.mixed = r.bits(1);
            }
        });
        st.add(diff(&case, &format!("{ctx} iter={i}")).ret);
    }
    st.require_some_ok(ctx);
}

// ---------------------------------------------------------------------------
// Row 16: `scfsi &= 0x0F0F` masking triggered by granule 0, observed in later
// granules' `gr->scfsi`
// ---------------------------------------------------------------------------

#[test]
fn cfg_16_scfsi_masking_propagation() {
    let ctx = "cfg_16";
    let mut rng = Rng::new(0x3016);
    let mut st = Stats::default();
    for i in 0..ITERS {
        for mono in [false, true] {
            let sr = 2 + (i % 7) as i32;
            let (srate, bit4) = hdr_bits_for_sr_idx(true, sr).unwrap();
            let hdr = make_hdr(&mut rng, true, mono, srate, bit4);
            let (case, _si) = build(&mut rng, hdr, |r, si| {
                si.scfsi = r.bits(7 + gr_count(&hdr) as u32);
                si.gr[0].window = 1;
                si.gr[0].block_type = 2;
                si.gr[0].mixed = r.bits(1);
                for g in si.gr.iter_mut().skip(1) {
                    g.window = 0;
                }
            });
            st.add(diff(&case, &format!("{ctx} iter={i} mono={mono}")).ret);
        }
    }
    st.require_some_ok(ctx);
}

// ---------------------------------------------------------------------------
// Rows 17-19: the `scfsi` header field
// ---------------------------------------------------------------------------

fn row_scfsi(seed: u64, mpeg1: bool, mono: bool, ctx: &str) {
    let mut rng = Rng::new(seed);
    let mut st = Stats::default();
    let g = if mpeg1 {
        if mono { 2 } else { 4 }
    } else if mono {
        1
    } else {
        2
    };
    for i in 0..ITERS {
        let sr = if mpeg1 { 2 + (i % 7) as i32 } else { (i % 6) as i32 };
        let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
        let hdr = make_hdr(&mut rng, mpeg1, mono, srate, bit4);
        assert_eq!(gr_count(&hdr), g);
        let (case, _si) = build(&mut rng, hdr, |r, si| {
            si.scfsi = r.bits(7 + g as u32);
            for gg in si.gr.iter_mut() {
                gg.window = r.bits(1);
                gg.block_type = 1 + r.below(3);
            }
        });
        let out = diff(&case, &format!("{ctx} iter={i}"));
        if !mpeg1 {
            // The C never reads a scfsi field on this path, so `scfsi` stays 0
            // and every granule gets 0.
            for k in 0..g {
                assert_eq!(out.gr[k * GR_SIZE + O_SCFSI], 0, "{ctx}: scfsi must be 0");
            }
        }
        st.add(out.ret);
    }
    st.require_some_ok(ctx);
}

#[test]
fn cfg_17_scfsi_mpeg1_mono_9bit() {
    row_scfsi(0x3017, true, true, "cfg_17");
}

#[test]
fn cfg_18_scfsi_mpeg1_stereo_11bit() {
    row_scfsi(0x3018, true, false, "cfg_18");
}

#[test]
fn cfg_19_scfsi_absent_on_mpeg2() {
    row_scfsi(0x3019, false, true, "cfg_19a");
    row_scfsi(0x301a, false, false, "cfg_19b");
}

// ---------------------------------------------------------------------------
// Rows 20-21: preflag
// ---------------------------------------------------------------------------

#[test]
fn cfg_20_preflag_from_scalefac_compress_mpeg2() {
    let ctx = "cfg_20";
    let mut rng = Rng::new(0x3020);
    let mut st = Stats::default();
    // sweep the whole 9-bit field, hitting the 499/500 boundary
    for sfc in 0..512u32 {
        let sr = (sfc % 6) as i32;
        let (srate, bit4) = hdr_bits_for_sr_idx(false, sr).unwrap();
        let hdr = make_hdr(&mut rng, false, sfc % 2 == 0, srate, bit4);
        let (case, _si) = build(&mut rng, hdr, |_r, si| {
            for g in si.gr.iter_mut() {
                g.scalefac_compress = sfc;
            }
        });
        let out = diff(&case, &format!("{ctx} sfc={sfc}"));
        if out.ret >= 0 {
            for k in 0..gr_count(&hdr) {
                assert_eq!(
                    out.gr[k * GR_SIZE + O_PREFLAG] as u32,
                    (sfc >= 500) as u32,
                    "{ctx}: preflag for sfc={sfc}"
                );
            }
        }
        st.add(out.ret);
    }
    st.require_some_ok(ctx);
}

#[test]
fn cfg_21_preflag_explicit_bit_mpeg1() {
    let ctx = "cfg_21";
    let mut rng = Rng::new(0x3021);
    let mut st = Stats::default();
    for i in 0..ITERS {
        for pf in [0u32, 1] {
            let sr = 2 + (i % 7) as i32;
            let (srate, bit4) = hdr_bits_for_sr_idx(true, sr).unwrap();
            let hdr = make_hdr(&mut rng, true, i % 2 == 0, srate, bit4);
            let (case, _si) = build(&mut rng, hdr, |r, si| {
                for g in si.gr.iter_mut() {
                    g.preflag = pf;
                    g.scalefac_compress = r.bits(4);
                }
            });
            let out = diff(&case, &format!("{ctx} iter={i} pf={pf}"));
            if out.ret >= 0 {
                for k in 0..gr_count(&hdr) {
                    assert_eq!(out.gr[k * GR_SIZE + O_PREFLAG] as u32, pf, "{ctx}: preflag");
                }
            }
            st.add(out.ret);
        }
    }
    st.require_some_ok(ctx);
}

// ---------------------------------------------------------------------------
// Rows 22-24: sr_idx == 8 (out-of-range scalefactor-band table row)
// ---------------------------------------------------------------------------

fn row_sr_idx_8(seed: u64, window: u32, block_type: u32, mixed: u32, ctx: &str) {
    let mut rng = Rng::new(seed);
    let (srate, bit4) = hdr_bits_for_sr_idx(true, 8).unwrap();
    let mut st = Stats::default();
    for i in 0..ITERS {
        let hdr = make_hdr(&mut rng, true, i % 2 == 0, srate, bit4);
        assert_eq!(sr_idx(&hdr), 8, "{ctx}: sr_idx must be 8");
        let (case, _si) = build(&mut rng, hdr, |_r, si| {
            for g in si.gr.iter_mut() {
                g.window = window;
                g.block_type = block_type;
                g.mixed = mixed;
            }
        });
        st.add(diff(&case, &format!("{ctx} iter={i}")).ret);
    }
    st.require_some_ok(ctx);
}

#[test]
fn cfg_22_sr_idx_8_long_row() {
    row_sr_idx_8(0x3022, 0, 0, 0, "cfg_22");
}

#[test]
fn cfg_23_sr_idx_8_short_row() {
    row_sr_idx_8(0x3023, 1, 2, 0, "cfg_23");
}

#[test]
fn cfg_24_sr_idx_8_mixed_row() {
    row_sr_idx_8(0x3024, 1, 2, 1, "cfg_24");
}

// ---------------------------------------------------------------------------
// Rows 25-27: bs->pos shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg_25_all_start_alignments() {
    let ctx = "cfg_25";
    let mut rng = Rng::new(0x3025);
    let mut st = Stats::default();
    for align in 0..8i32 {
        for &(mpeg1, mono) in &[(false, true), (false, false), (true, true), (true, false)] {
            for rep in 0..12 {
                let sr = if mpeg1 { 2 + (rep % 7) } else { rep % 6 } as i32;
                let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
                let hdr = make_hdr(&mut rng, mpeg1, mono, srate, bit4);
                let mut si = SideInfo::random(&mut rng, &hdr);
                for g in si.gr.iter_mut() {
                    g.window = rng.bits(1);
                    g.block_type = 1 + rng.below(3);
                    g.mixed = rng.bits(1);
                }
                let mut case = Case::new(&mut rng, hdr);
                case.pos = align;
                case.put_side_info(&si);
                st.add(diff(&case, &format!("{ctx} align={align} rep={rep}")).ret);
            }
        }
    }
    st.require_some_ok(ctx);
}

#[test]
fn cfg_26_deep_byte_aligned_pos() {
    let ctx = "cfg_26";
    let mut rng = Rng::new(0x3026);
    let mut st = Stats::default();
    for i in 0..ITERS {
        let mpeg1 = i % 2 == 0;
        let sr = if mpeg1 { 2 + (i % 7) } else { i % 6 } as i32;
        let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
        let hdr = make_hdr(&mut rng, mpeg1, i % 3 == 0, srate, bit4);
        let mut si = SideInfo::random(&mut rng, &hdr);
        for g in si.gr.iter_mut() {
            g.window = rng.bits(1);
            g.block_type = 1 + rng.below(3);
        }
        let mut case = Case::new(&mut rng, hdr);
        case.pos = 8 * 64 + (i % 8) as i32;
        case.put_side_info(&si);
        st.add(diff(&case, &format!("{ctx} iter={i}")).ret);
    }
    st.require_some_ok(ctx);
}

#[test]
fn cfg_27_negative_pos() {
    let ctx = "cfg_27";
    let mut rng = Rng::new(0x3027);
    let mut st = Stats::default();
    for p in -64..0i32 {
        for rep in 0..4 {
            let mpeg1 = rep % 2 == 0;
            let sr = if mpeg1 { 2 + (rep % 7) } else { rep % 6 } as i32;
            let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
            let hdr = make_hdr(&mut rng, mpeg1, rep % 3 == 0, srate, bit4);
            let mut si = SideInfo::random(&mut rng, &hdr);
            for g in si.gr.iter_mut() {
                g.window = rng.bits(1);
                g.block_type = 1 + rng.below(3);
            }
            let mut case = Case::new(&mut rng, hdr);
            case.pos = p;
            case.put_side_info(&si);
            st.add(diff(&case, &format!("{ctx} pos={p} rep={rep}")).ret);
        }
    }
    st.require_some_ok(ctx);
}

// ---------------------------------------------------------------------------
// Rows 28-29: bs->limit exactly at / one bit below the last consumed bit
// ---------------------------------------------------------------------------

fn row_tight_limit(seed: u64, slack: i32, ctx: &str) -> Stats {
    let mut rng = Rng::new(seed);
    let mut st = Stats::default();
    for i in 0..ITERS {
        let mpeg1 = i % 2 == 0;
        let sr = if mpeg1 { 2 + (i % 7) } else { i % 6 } as i32;
        let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
        let hdr = make_hdr(&mut rng, mpeg1, i % 3 == 0, srate, bit4);
        let (mut case, _si) = build(&mut rng, hdr, |r, si| {
            for g in si.gr.iter_mut() {
                // part_23_length = 0 so that the final range check passes and the
                // row really exercises the *tight limit*, not the sum check.
                g.part_23_length = 0;
                g.window = r.bits(1);
                g.block_type = 1 + r.below(3);
                g.mixed = r.bits(1);
            }
        });
        let pos_end = probe_pos_end(&case);
        case.limit = pos_end + slack;
        st.add(diff(&case, &format!("{ctx} iter={i} limit={} ", case.limit)).ret);
    }
    st
}

#[test]
fn cfg_28_limit_exactly_at_end() {
    let st = row_tight_limit(0x3028, 0, "cfg_28");
    st.require_some_ok("cfg_28");
}

#[test]
fn cfg_29_limit_one_bit_short() {
    let st = row_tight_limit(0x3029, -1, "cfg_29");
    st.require_some_ok("cfg_29");
}

// ---------------------------------------------------------------------------
// Rows 30-32: part_23_length / big_values value shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg_30_part23_sum_boundary() {
    let ctx = "cfg_30";
    let mut rng = Rng::new(0x3030);
    let mut ok = 0;
    let mut err = 0;
    for i in 0..ITERS {
        for delta in [-1i32, 0, 1] {
            let mpeg1 = i % 2 == 0;
            let sr = if mpeg1 { 2 + (i % 7) } else { i % 6 } as i32;
            let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
            let hdr = make_hdr(&mut rng, mpeg1, i % 3 == 0, srate, bit4);
            let g = gr_count(&hdr);
            let (mut case, mut si) = build(&mut rng, hdr, |r, si| {
                // keep main_data_begin small so the target sum stays representable
                si.main_data_begin = r.below(64) << if is_mpeg1(&hdr) { 0 } else { g as u32 };
                for gg in si.gr.iter_mut() {
                    gg.part_23_length = 0;
                    gg.window = r.bits(1);
                    gg.block_type = 1 + r.below(3);
                }
            });
            let pos_end = probe_pos_end(&case);
            case.limit = 700 + rng.below(300) as i32;
            let mdb = main_data_begin_value(&hdr, &si);
            // want: part_23_sum + pos_end == limit + mdb*8 + delta
            let target = case.limit + mdb * 8 + delta - pos_end;
            if target < 0 || target > 4095 * g as i32 {
                continue;
            }
            let mut left = target;
            for gg in si.gr.iter_mut() {
                let take = left.min(4095);
                gg.part_23_length = take as u32;
                left -= take;
            }
            assert_eq!(left, 0);
            case.put_side_info(&si);
            let out = diff(&case, &format!("{ctx} iter={i} delta={delta}"));
            // delta <= 0 must succeed (check is `>`), delta == 1 must fail
            if delta <= 0 {
                assert!(out.ret >= 0, "{ctx}: delta={delta} should succeed, got {}", out.ret);
                ok += 1;
            } else {
                assert_eq!(out.ret, -1, "{ctx}: delta=1 should fail");
                err += 1;
            }
        }
    }
    assert!(ok > 0 && err > 0, "{ctx}: coverage ok={ok} err={err}");
}

#[test]
fn cfg_31_part23_length_max() {
    let ctx = "cfg_31";
    let mut rng = Rng::new(0x3031);
    let mut st = Stats::default();
    for i in 0..ITERS {
        let mpeg1 = i % 2 == 0;
        let sr = if mpeg1 { 2 + (i % 7) } else { i % 6 } as i32;
        let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
        let hdr = make_hdr(&mut rng, mpeg1, i % 3 == 0, srate, bit4);
        let (case, _si) = build(&mut rng, hdr, |r, si| {
            for g in si.gr.iter_mut() {
                g.part_23_length = 4095;
                g.window = r.bits(1);
                g.block_type = 1 + r.below(3);
            }
        });
        let out = diff(&case, &format!("{ctx} iter={i}"));
        for k in 0..gr_count(&hdr) {
            let mut b = [0u8; 2];
            b.copy_from_slice(&out.gr[k * GR_SIZE + O_PART_23_LENGTH..k * GR_SIZE + O_PART_23_LENGTH + 2]);
            assert_eq!(u16::from_ne_bytes(b), 4095);
        }
        st.add(out.ret);
    }
    st.require_some_ok(ctx);
}

#[test]
fn cfg_32_big_values_boundaries() {
    let ctx = "cfg_32";
    let mut rng = Rng::new(0x3032);
    let mut st = Stats::default();
    for bv in [0u32, 1, 144, 287, 288] {
        for i in 0..40 {
            let mpeg1 = i % 2 == 0;
            let sr = if mpeg1 { 2 + (i % 7) } else { i % 6 } as i32;
            let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
            let hdr = make_hdr(&mut rng, mpeg1, i % 3 == 0, srate, bit4);
            let (case, _si) = build(&mut rng, hdr, |r, si| {
                for g in si.gr.iter_mut() {
                    g.big_values = bv;
                    g.window = r.bits(1);
                    g.block_type = 1 + r.below(3);
                }
            });
            let out = diff(&case, &format!("{ctx} bv={bv} iter={i}"));
            assert!(out.ret >= 0, "{ctx}: bv={bv} must be accepted, got {}", out.ret);
            st.add(out.ret);
        }
    }
    st.require_some_ok(ctx);
}

// ---------------------------------------------------------------------------
// Rows 33-34: degenerate buffers
// ---------------------------------------------------------------------------

#[test]
fn cfg_33_all_zero_buffer() {
    let ctx = "cfg_33";
    let mut rng = Rng::new(0x3033);
    for i in 0..64 {
        let mut hdr = [0u8; 4];
        rng.fill(&mut hdr);
        let mut case = Case::new(&mut rng, hdr);
        case.buf.iter_mut().for_each(|b| *b = 0);
        case.pos = (i % 8) as i32;
        let out = diff(&case, &format!("{ctx} iter={i}"));
        assert_eq!(out.ret, 0, "{ctx}: all-zero side info => main_data_begin 0");
    }
}

#[test]
fn cfg_34_all_ones_buffer() {
    let ctx = "cfg_34";
    let mut rng = Rng::new(0x3034);
    // (a) raw 0xFF buffer: big_values decodes to 511 => rejected
    for i in 0..64 {
        let mut hdr = [0u8; 4];
        rng.fill(&mut hdr);
        let mut case = Case::new(&mut rng, hdr);
        case.buf.iter_mut().for_each(|b| *b = 0xFF);
        case.pos = (i % 8) as i32;
        let out = diff(&case, &format!("{ctx}a iter={i}"));
        assert_eq!(out.ret, -1, "{ctx}a: big_values=511 must be rejected");
    }
    // (b) every field at its maximum except big_values (<=288)
    for i in 0..ITERS {
        let mpeg1 = i % 2 == 0;
        let sr = if mpeg1 { 2 + (i % 7) } else { i % 6 } as i32;
        let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
        let hdr = make_hdr(&mut rng, mpeg1, i % 3 == 0, srate, bit4);
        let (case, _si) = build(&mut rng, hdr, |_r, si| {
            si.main_data_begin = u32::MAX;
            si.scfsi = u32::MAX;
            for g in si.gr.iter_mut() {
                *g = Gran {
                    part_23_length: 4095,
                    big_values: 288,
                    global_gain: 255,
                    scalefac_compress: 511,
                    window: 1,
                    block_type: 3,
                    mixed: 1,
                    tables10: 1023,
                    subblock_gain: [7, 7, 7],
                    tables15: 32767,
                    region_count0: 15,
                    region_count1: 7,
                    preflag: 1,
                    scalefac_scale: 1,
                    count1_table: 1,
                };
            }
        });
        diff(&case, &format!("{ctx}b iter={i}"));
    }
}

// ---------------------------------------------------------------------------
// Rows 35-37
// ---------------------------------------------------------------------------

#[test]
fn cfg_35_no_writes_past_gr_count() {
    let ctx = "cfg_35";
    let mut rng = Rng::new(0x3035);
    for i in 0..ITERS {
        let mpeg1 = i % 2 == 0;
        let mono = i % 3 == 0;
        let sr = if mpeg1 { 2 + (i % 7) } else { i % 6 } as i32;
        let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
        let hdr = make_hdr(&mut rng, mpeg1, mono, srate, bit4);
        let g = gr_count(&hdr);
        let (case, _si) = build(&mut rng, hdr, |r, si| {
            for gg in si.gr.iter_mut() {
                gg.window = r.bits(1);
                gg.block_type = 1 + r.below(3);
            }
        });
        let out = diff(&case, &format!("{ctx} iter={i}"));
        for k in g..N_GR {
            assert_eq!(
                &out.gr[k * GR_SIZE..(k + 1) * GR_SIZE],
                &case.fill[k * GR_SIZE..(k + 1) * GR_SIZE],
                "{ctx}: gr[{k}] must be untouched (gr_count={g})"
            );
        }
    }
}

#[test]
fn cfg_36_region_count2_preservation() {
    let ctx = "cfg_36";
    let mut rng = Rng::new(0x3036);
    for i in 0..ITERS {
        let mpeg1 = i % 2 == 0;
        let sr = if mpeg1 { 2 + (i % 7) } else { i % 6 } as i32;
        let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
        let hdr = make_hdr(&mut rng, mpeg1, i % 3 == 0, srate, bit4);
        let (case, si) = build(&mut rng, hdr, |r, si| {
            for gg in si.gr.iter_mut() {
                gg.window = r.bits(1);
                gg.block_type = 1 + r.below(3);
            }
        });
        let out = diff(&case, &format!("{ctx} iter={i}"));
        if out.ret >= 0 {
            for (k, gg) in si.gr.iter().enumerate() {
                let got = out.gr[k * GR_SIZE + O_REGION_COUNT + 2];
                if gg.window != 0 {
                    assert_eq!(got, case.fill[k * GR_SIZE + O_REGION_COUNT + 2], "{ctx}: W=1");
                } else {
                    assert_eq!(got, 255, "{ctx}: W=0");
                }
            }
        }
    }
}

#[test]
fn cfg_37_hdr0_and_dontcare_bits_ignored() {
    let ctx = "cfg_37";
    let mut rng = Rng::new(0x3037);
    for i in 0..ITERS {
        let mpeg1 = i % 2 == 0;
        let sr = if mpeg1 { 2 + (i % 7) } else { i % 6 } as i32;
        let (srate, bit4) = hdr_bits_for_sr_idx(mpeg1, sr).unwrap();
        let hdr = make_hdr(&mut rng, mpeg1, i % 3 == 0, srate, bit4);
        let (case, _si) = build(&mut rng, hdr, |r, si| {
            for gg in si.gr.iter_mut() {
                gg.window = r.bits(1);
                gg.block_type = 1 + r.below(3);
            }
        });
        let base = diff(&case, &format!("{ctx} iter={i} base"));
        // Flip every bit the C provably never reads and require identical output
        // from both libraries.
        let mut case2 = case.clone();
        case2.hdr[0] ^= 0xFF;
        case2.hdr[1] ^= 0xE7; // everything except bits 3 and 4
        case2.hdr[2] ^= 0xF3; // everything except bits 2 and 3
        let alt = diff(&case2, &format!("{ctx} iter={i} alt"));
        assert_eq!(base.ret, alt.ret, "{ctx}: unread hdr bits changed the result");
        assert_eq!(base.gr[8..], alt.gr[8..], "{ctx}: unread hdr bits changed gr");
    }
}

// ---------------------------------------------------------------------------
// Row 38: unconstrained random sweep
// ---------------------------------------------------------------------------

#[test]
fn cfg_38_random_hdr_sweep() {
    let ctx = "cfg_38";
    let mut rng = Rng::new(0x3038);
    let mut st = Stats::default();
    let max_limit = (8 * (BUF_LEN - BUF_MID - 8)) as i32;
    for i in 0..20000 {
        let mut hdr = [0u8; 4];
        rng.fill(&mut hdr);
        let mut case = Case::new(&mut rng, hdr);
        case.pos = rng.below(65) as i32 - 8;
        case.limit = match i % 4 {
            0 => AMPLE_LIMIT,
            1 => rng.below(400) as i32,
            2 => rng.below(max_limit as u32) as i32,
            _ => case.pos + rng.below(300) as i32,
        };
        // Half the iterations get a synthesised (well-formed) side info on top of
        // the random bytes, half stay fully random.
        if i % 2 == 0 {
            let mut si = SideInfo::random(&mut rng, &hdr);
            for gg in si.gr.iter_mut() {
                gg.window = rng.bits(1);
                gg.block_type = rng.below(4);
                gg.mixed = rng.bits(1);
                gg.big_values = rng.bits(9);
            }
            case.put_side_info(&si);
        }
        st.add(diff(&case, &format!("{ctx} iter={i}")).ret);
    }
    st.require_some_ok(ctx);
    st.require_some_err(ctx);
}
