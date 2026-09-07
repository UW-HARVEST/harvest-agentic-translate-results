//! Phase B — valid-path differential tests, one test per CONFIGS.md row.
//!
//! Both implementations are loaded from their `.so` via `libloading`; the Rust
//! function is only ever reached through its exported `read_side_info` symbol.

mod common;

use common::*;

const ITERS: usize = 512;

/// Drive one CONFIGS row: build `ITERS` randomized side-info bitstreams that
/// all match the requested configuration, and compare C vs Rust each time.
fn row<F>(label: &str, hdr: [u8; 4], iters: usize, mut shape: F)
where
    F: FnMut(&mut Rng, usize, &mut Granule),
{
    let mut rng = Rng::new();
    let n = gr_count_for(&hdr);
    for it in 0..iters {
        let mut grs = [Granule::default(); 4];
        for (i, g) in grs.iter_mut().enumerate() {
            *g = Granule::random(&mut rng);
            shape(&mut rng, i, g);
        }
        let mdb = rng.below(if (hdr[1] & 0x8) != 0 { 512 } else { 256 });
        let scfsi = rng.next_u32();
        let (buf, end) = build_side_info(&hdr, 0, 0, mdb, scfsi, &grs, 16);
        // Generous limit so nothing truncates: the valid path.
        let limit = end + rng.range(0, 64);
        assert_same(&format!("{label} it={it} gr_count={n}"), &buf, 0, limit, &hdr);
    }
}

fn long_block(_r: &mut Rng, _i: usize, g: &mut Granule) {
    g.window_switching = false;
}
fn bt(n: u32, mixed: u32) -> impl FnMut(&mut Rng, usize, &mut Granule) {
    move |_r: &mut Rng, _i: usize, g: &mut Granule| {
        g.window_switching = true;
        g.block_type = n;
        g.mixed_block_flag = mixed;
    }
}

// ---------------------------------------------------------------------------
// C1..C20 — mpeg1 x mono x block shape
// ---------------------------------------------------------------------------

// hdr helpers: chan 3 == mono, chan 0 == stereo. sr_bits/ext kept legal.
fn h(mpeg1: bool, chan: u32) -> [u8; 4] {
    make_hdr(mpeg1, 0, 1, chan, 0)
}

#[test]
fn c01_mpeg2_mono_long() {
    row("C1", h(false, 3), ITERS, long_block);
}
#[test]
fn c02_mpeg2_mono_bt1() {
    row("C2", h(false, 3), ITERS, bt(1, 0));
}
#[test]
fn c03_mpeg2_mono_bt2_short() {
    row("C3", h(false, 3), ITERS, bt(2, 0));
}
#[test]
fn c04_mpeg2_mono_bt2_mixed() {
    row("C4", h(false, 3), ITERS, bt(2, 1));
}
#[test]
fn c05_mpeg2_mono_bt3() {
    row("C5", h(false, 3), ITERS, bt(3, 0));
}
#[test]
fn c06_mpeg2_stereo_long() {
    row("C6", h(false, 0), ITERS, long_block);
}
#[test]
fn c07_mpeg2_stereo_bt1() {
    row("C7", h(false, 0), ITERS, bt(1, 0));
}
#[test]
fn c08_mpeg2_stereo_bt2_short() {
    row("C8", h(false, 0), ITERS, bt(2, 0));
}
#[test]
fn c09_mpeg2_stereo_bt2_mixed() {
    row("C9", h(false, 0), ITERS, bt(2, 1));
}
#[test]
fn c10_mpeg2_stereo_bt3() {
    row("C10", h(false, 0), ITERS, bt(3, 0));
}
#[test]
fn c11_mpeg1_mono_long() {
    row("C11", h(true, 3), ITERS, long_block);
}
#[test]
fn c12_mpeg1_mono_bt1() {
    row("C12", h(true, 3), ITERS, bt(1, 0));
}
#[test]
fn c13_mpeg1_mono_bt2_short() {
    row("C13", h(true, 3), ITERS, bt(2, 0));
}
#[test]
fn c14_mpeg1_mono_bt2_mixed() {
    row("C14", h(true, 3), ITERS, bt(2, 1));
}
#[test]
fn c15_mpeg1_mono_bt3() {
    row("C15", h(true, 3), ITERS, bt(3, 0));
}
#[test]
fn c16_mpeg1_stereo_long() {
    row("C16", h(true, 0), ITERS, long_block);
}
#[test]
fn c17_mpeg1_stereo_bt1() {
    row("C17", h(true, 0), ITERS, bt(1, 0));
}
#[test]
fn c18_mpeg1_stereo_bt2_short() {
    row("C18", h(true, 0), ITERS, bt(2, 0));
}
#[test]
fn c19_mpeg1_stereo_bt2_mixed() {
    row("C19", h(true, 0), ITERS, bt(2, 1));
}
#[test]
fn c20_mpeg1_stereo_bt3() {
    row("C20", h(true, 0), ITERS, bt(3, 0));
}

// ---------------------------------------------------------------------------
// C21..C25 — per-granule shape interactions
// ---------------------------------------------------------------------------

#[test]
fn c21_mpeg2_stereo_gr0_long_gr1_short() {
    row("C21", h(false, 0), ITERS, |_r, i, g| {
        if i == 0 {
            g.window_switching = false;
        } else {
            g.window_switching = true;
            g.block_type = 2;
            g.mixed_block_flag = 0;
        }
    });
}

#[test]
fn c22_mpeg2_stereo_gr0_mixed_gr1_long() {
    row("C22", h(false, 0), ITERS, |_r, i, g| {
        if i == 0 {
            g.window_switching = true;
            g.block_type = 2;
            g.mixed_block_flag = 1;
        } else {
            g.window_switching = false;
        }
    });
}

#[test]
fn c23_mpeg1_stereo_four_granules_fully_random() {
    // Granule::random already randomizes ws/block_type/mixed independently.
    row("C23", h(true, 0), 4 * ITERS, |_r, _i, _g| {});
}

#[test]
fn c24_mpeg1_mono_scfsi_mask_interaction() {
    row("C24", h(true, 3), ITERS, |_r, i, g| {
        g.window_switching = true;
        if i == 0 {
            g.block_type = 2;
            g.mixed_block_flag = 0;
        } else {
            g.block_type = 1;
        }
    });
}

#[test]
fn c25_mpeg1_stereo_scfsi_mask_on_third_granule() {
    row("C25", h(true, 0), ITERS, |_r, i, g| {
        g.window_switching = true;
        if i == 2 {
            g.block_type = 2;
            g.mixed_block_flag = 0;
        } else {
            g.block_type = 3;
        }
    });
}

// ---------------------------------------------------------------------------
// C26..C28 — the sr_idx axis, all 9 reachable values, x 3 table kinds
// ---------------------------------------------------------------------------

/// Every reachable `(mpeg1, ext, sr_bits)` triple, annotated with the resulting
/// `sr_idx`. `sr_idx = sr_bits + (mpeg1 + ext)*3`, minus 1 if non-zero.
fn sr_triples() -> Vec<(bool, u32, u32, i32)> {
    let mut v = Vec::new();
    for &mpeg1 in &[false, true] {
        for ext in 0..2u32 {
            for sr_bits in 0..4u32 {
                let hdr = make_hdr(mpeg1, ext, sr_bits, 3, 0);
                v.push((mpeg1, ext, sr_bits, sr_idx_for(&hdr)));
            }
        }
    }
    v
}

fn sr_row<F>(label: &str, mut shape: F)
where
    F: FnMut(&mut Rng, usize, &mut Granule) + Copy,
{
    let mut seen = std::collections::BTreeSet::new();
    for (mpeg1, ext, sr_bits, sr) in sr_triples() {
        seen.insert(sr);
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, ext, sr_bits, chan, 0);
            assert_eq!(sr_idx_for(&hdr), sr);
            row(
                &format!("{label} sr_idx={sr} mpeg1={mpeg1} ext={ext} sr_bits={sr_bits} chan={chan}"),
                hdr,
                96,
                &mut shape,
            );
        }
    }
    // Confirm the axis really was covered end to end.
    assert_eq!(
        seen.into_iter().collect::<Vec<_>>(),
        (0..=8).collect::<Vec<i32>>(),
        "sr_idx axis incomplete"
    );
}

#[test]
fn c26_sr_idx_all_long_table() {
    sr_row("C26", long_block);
}

#[test]
fn c27_sr_idx_all_short_table() {
    sr_row("C27", |_r: &mut Rng, _i: usize, g: &mut Granule| {
        g.window_switching = true;
        g.block_type = 2;
        g.mixed_block_flag = 0;
    });
}

#[test]
fn c28_sr_idx_all_mixed_table() {
    sr_row("C28", |_r: &mut Rng, _i: usize, g: &mut Granule| {
        g.window_switching = true;
        g.block_type = 2;
        g.mixed_block_flag = 1;
    });
}

/// The three table rows the C selects must be byte-identical for every
/// in-range `sr_idx` (0..=7) — asserted directly against the C tables
/// transcribed from `c_src/src/lib.c`.
#[test]
fn c26b_table_contents_match_c_source() {
    #[rustfmt::skip]
    const LONG: [[u8; 23]; 8] = [
        [6,6,6,6,6,6,8,10,12,14,16,20,24,28,32,38,46,52,60,68,58,54,0],
        [12,12,12,12,12,12,16,20,24,28,32,40,48,56,64,76,90,2,2,2,2,2,0],
        [6,6,6,6,6,6,8,10,12,14,16,20,24,28,32,38,46,52,60,68,58,54,0],
        [6,6,6,6,6,6,8,10,12,14,16,18,22,26,32,38,46,54,62,70,76,36,0],
        [6,6,6,6,6,6,8,10,12,14,16,20,24,28,32,38,46,52,60,68,58,54,0],
        [4,4,4,4,4,4,6,6,8,8,10,12,16,20,24,28,34,42,50,54,76,158,0],
        [4,4,4,4,4,4,6,6,6,8,10,12,16,18,22,28,34,40,46,54,54,192,0],
        [4,4,4,4,4,4,6,6,8,10,12,16,20,24,30,38,46,56,68,84,102,26,0],
    ];
    #[rustfmt::skip]
    const SHORT: [[u8; 40]; 8] = [
        [4,4,4,4,4,4,4,4,4,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,24,24,24,30,30,30,40,40,40,18,18,18,0],
        [8,8,8,8,8,8,8,8,8,12,12,12,16,16,16,20,20,20,24,24,24,28,28,28,36,36,36,2,2,2,2,2,2,2,2,2,26,26,26,0],
        [4,4,4,4,4,4,4,4,4,6,6,6,6,6,6,8,8,8,10,10,10,14,14,14,18,18,18,26,26,26,32,32,32,42,42,42,18,18,18,0],
        [4,4,4,4,4,4,4,4,4,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,24,24,24,32,32,32,44,44,44,12,12,12,0],
        [4,4,4,4,4,4,4,4,4,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,24,24,24,30,30,30,40,40,40,18,18,18,0],
        [4,4,4,4,4,4,4,4,4,4,4,4,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,22,22,22,30,30,30,56,56,56,0],
        [4,4,4,4,4,4,4,4,4,4,4,4,6,6,6,6,6,6,10,10,10,12,12,12,14,14,14,16,16,16,20,20,20,26,26,26,66,66,66,0],
        [4,4,4,4,4,4,4,4,4,4,4,4,6,6,6,8,8,8,12,12,12,16,16,16,20,20,20,26,26,26,34,34,34,42,42,42,12,12,12,0],
    ];
    // Rows 0..=7 of g_scf_mixed; C initialises 37 or 39 of the 40 entries and
    // zero-fills the rest.
    #[rustfmt::skip]
    const MIXED: [[u8; 40]; 8] = [
        [6,6,6,6,6,6,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,24,24,24,30,30,30,40,40,40,18,18,18,0,0,0,0],
        [12,12,12,4,4,4,8,8,8,12,12,12,16,16,16,20,20,20,24,24,24,28,28,28,36,36,36,2,2,2,2,2,2,2,2,2,26,26,26,0],
        [6,6,6,6,6,6,6,6,6,6,6,6,8,8,8,10,10,10,14,14,14,18,18,18,26,26,26,32,32,32,42,42,42,18,18,18,0,0,0,0],
        [6,6,6,6,6,6,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,24,24,24,32,32,32,44,44,44,12,12,12,0,0,0,0],
        [6,6,6,6,6,6,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,24,24,24,30,30,30,40,40,40,18,18,18,0,0,0,0],
        [4,4,4,4,4,4,6,6,4,4,4,6,6,6,8,8,8,10,10,10,12,12,12,14,14,14,18,18,18,22,22,22,30,30,30,56,56,56,0,0],
        [4,4,4,4,4,4,6,6,4,4,4,6,6,6,6,6,6,10,10,10,12,12,12,14,14,14,16,16,16,20,20,20,26,26,26,66,66,66,0,0],
        [4,4,4,4,4,4,6,6,4,4,4,6,6,6,8,8,8,12,12,12,16,16,16,20,20,20,26,26,26,34,34,34,42,42,42,12,12,12,0,0],
    ];

    for (mpeg1, ext, sr_bits, sr) in sr_triples() {
        if sr > 7 {
            continue; // out-of-range row, see ERRORS.md E8
        }
        let hdr = make_hdr(mpeg1, ext, sr_bits, 3, 0);
        let sri = sr as usize;
        for (kind, ws, b, m) in [
            ("long", false, 0u32, 0u32),
            ("short", true, 2, 0),
            ("mixed", true, 2, 1),
        ] {
            let mut grs = [Granule::default(); 4];
            for g in grs.iter_mut() {
                g.window_switching = ws;
                g.block_type = b;
                g.mixed_block_flag = m;
                g.big_values = 10;
            }
            let (buf, end) = build_side_info(&hdr, 0, 0, 0, 0, &grs, 16);
            let (c, r) = run_both(&buf, 0, end + 4096, &hdr);
            assert_eq!(c, r, "{kind} sr_idx={sri}");
            let expect: Vec<u8> = match kind {
                "long" => LONG[sri].to_vec(),
                "short" => SHORT[sri].to_vec(),
                _ => MIXED[sri].to_vec(),
            };
            let got = c.sfbtab_bytes[0].as_ref().expect("sfbtab written");
            assert_eq!(got, &expect, "C {kind} table row {sri} mismatch vs lib.c");
        }
    }
}

// ---------------------------------------------------------------------------
// C29 — channel-mode axis: all 4 raw values of hdr[3]>>6
// ---------------------------------------------------------------------------

#[test]
fn c29_all_channel_modes() {
    let mut rng = Rng::new();
    for chan in 0..4u32 {
        for &mpeg1 in &[false, true] {
            for _ in 0..64 {
                let noise = rng.next_u32() as u8;
                let hdr = make_hdr(mpeg1, rng.below(2), rng.below(4), chan, noise);
                let mut grs = [Granule::default(); 4];
                for g in grs.iter_mut() {
                    *g = Granule::random(&mut rng);
                }
                let mdb = rng.below(if mpeg1 { 512 } else { 256 });
                let (buf, end) =
                    build_side_info(&hdr, 0, 0, mdb, rng.next_u32(), &grs, 16);
                assert_same(
                    &format!("C29 chan={chan} mpeg1={mpeg1}"),
                    &buf,
                    0,
                    end + 32,
                    &hdr,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C30..C32, C38 — start bit position / phase (also covers get_bits phases)
// ---------------------------------------------------------------------------

fn pos_row(label: &str, positions: &[usize]) {
    let mut rng = Rng::new();
    for &start in positions {
        for &mpeg1 in &[false, true] {
            for chan in [0u32, 3u32] {
                let hdr = make_hdr(mpeg1, 0, 2, chan, 0);
                for _ in 0..48 {
                    let mut grs = [Granule::default(); 4];
                    for g in grs.iter_mut() {
                        *g = Granule::random(&mut rng);
                    }
                    let pad = rng.next_u32() as u8;
                    let mdb = rng.below(if mpeg1 { 512 } else { 256 });
                    let (buf, end) = build_side_info(
                        &hdr,
                        start,
                        pad,
                        mdb,
                        rng.next_u32(),
                        &grs,
                        16,
                    );
                    assert_same(
                        &format!("{label} start={start} mpeg1={mpeg1} chan={chan}"),
                        &buf,
                        start as i32,
                        end + rng.range(0, 32),
                        &hdr,
                    );
                }
            }
        }
    }
}

#[test]
fn c30_c38_all_eight_start_phases() {
    pos_row("C30/C38", &[0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn c31_byte_aligned_mid_buffer() {
    pos_row("C31", &[8, 64, 800]);
}

#[test]
fn c32_unaligned_mid_buffer() {
    pos_row("C32", &[11, 67, 803]);
}

// ---------------------------------------------------------------------------
// C33..C36 — limit shapes
// ---------------------------------------------------------------------------

#[test]
fn c33_limit_exactly_consumed() {
    let mut rng = Rng::new();
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 1, 1, chan, 0);
            for _ in 0..ITERS {
                let mut grs = [Granule::default(); 4];
                for g in grs.iter_mut() {
                    *g = Granule::random(&mut rng);
                }
                let (buf, end) =
                    build_side_info(&hdr, 0, 0, rng.below(256), rng.next_u32(), &grs, 16);
                assert_same("C33", &buf, 0, end, &hdr);
            }
        }
    }
}

#[test]
fn c34_limit_one_bit_short() {
    let mut rng = Rng::new();
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 1, 1, chan, 0);
            for _ in 0..ITERS {
                let mut grs = [Granule::default(); 4];
                for g in grs.iter_mut() {
                    *g = Granule::random(&mut rng);
                }
                let (buf, end) =
                    build_side_info(&hdr, 0, 0, rng.below(256), rng.next_u32(), &grs, 16);
                assert_same("C34", &buf, 0, end - 1, &hdr);
            }
        }
    }
}

#[test]
fn c35_limit_swept_over_every_truncation_point() {
    let mut rng = Rng::new();
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 3, chan, 0);
            for trial in 0..4 {
                let mut grs = [Granule::default(); 4];
                for g in grs.iter_mut() {
                    *g = Granule::random(&mut rng);
                }
                let (buf, end) =
                    build_side_info(&hdr, 0, 0, rng.below(256), rng.next_u32(), &grs, 16);
                for limit in 0..=end {
                    assert_same(
                        &format!("C35 trial={trial} mpeg1={mpeg1} chan={chan}"),
                        &buf,
                        0,
                        limit,
                        &hdr,
                    );
                }
            }
        }
    }
}

#[test]
fn c36_limit_intmax_and_max_main_data_begin() {
    let mut rng = Rng::new();
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 1, chan, 0);
            let maxmdb = if mpeg1 { 511 } else { 255 };
            for _ in 0..128 {
                let mut grs = [Granule::default(); 4];
                for g in grs.iter_mut() {
                    *g = Granule::random(&mut rng);
                }
                let (buf, _end) =
                    build_side_info(&hdr, 0, 0, maxmdb, rng.next_u32(), &grs, 16);
                assert_same("C36 intmax", &buf, 0, i32::MAX, &hdr);
                assert_same("C36 big", &buf, 0, i32::MAX - 7, &hdr);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C37 — main_data_begin x part_23_sum around the final-check boundary
// ---------------------------------------------------------------------------

#[test]
fn c37_part23_vs_main_data_begin_boundary() {
    let mut rng = Rng::new();
    for &mpeg1 in &[false, true] {
        for chan in [0u32, 3u32] {
            let hdr = make_hdr(mpeg1, 0, 2, chan, 0);
            let n = gr_count_for(&hdr);
            let maxmdb = if mpeg1 { 511u32 } else { 255u32 };
            for &mdb in &[0u32, maxmdb / 2, maxmdb] {
                for _ in 0..64 {
                    // First pass: find the consumed-bit count for this shape.
                    let mut grs = [Granule::default(); 4];
                    for g in grs.iter_mut() {
                        *g = Granule::random(&mut rng);
                        g.part_23_length = 0;
                    }
                    let (probe, end) =
                        build_side_info(&hdr, 0, 0, mdb, rng.next_u32(), &grs, 16);
                    let _ = probe;
                    // Choose a limit, then pick part_23_sum to hit the boundary:
                    // error iff part_23_sum + pos > limit + mdb*8, and after a
                    // successful parse pos == end.
                    let limit = end + rng.range(0, 200);
                    let budget = limit as i64 + (mdb as i64) * 8 - end as i64;
                    for delta in [-1i64, 0, 1] {
                        let target = budget + delta;
                        if target < 0 || target > 4095 * n as i64 {
                            continue;
                        }
                        // Distribute `target` over the granules' part_23_length.
                        let mut rem = target as u32;
                        for i in 0..n {
                            let take = rem.min(4095);
                            grs[i].part_23_length = take;
                            rem -= take;
                        }
                        if rem != 0 {
                            continue;
                        }
                        let (buf, e2) =
                            build_side_info(&hdr, 0, 0, mdb, rng.next_u32(), &grs, 16);
                        assert_eq!(e2, end, "bit length must not depend on part_23_length");
                        assert_same(
                            &format!("C37 mdb={mdb} delta={delta} mpeg1={mpeg1} chan={chan}"),
                            &buf,
                            0,
                            limit,
                            &hdr,
                        );
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// C39 — exhaustive raw header byte sweep
// ---------------------------------------------------------------------------

#[test]
fn c39_all_hdr1_x_hdr3_and_all_hdr2() {
    let mut rng = Rng::new();
    let mut buf = vec![0u8; 512];

    // Every hdr[1] x hdr[3] pair (65 536 combinations).
    for h1 in 0..=255u8 {
        for h3 in 0..=255u8 {
            let hdr = [rng.next_u32() as u8, h1, rng.next_u32() as u8, h3];
            rng.fill(&mut buf);
            let limit = rng.range(0, 600);
            assert_same("C39 h1xh3", &buf, 0, limit, &hdr);
        }
    }
    // Every hdr[2].
    for h2 in 0..=255u8 {
        for _ in 0..16 {
            let hdr = [rng.next_u32() as u8, rng.next_u32() as u8, h2, rng.next_u32() as u8];
            rng.fill(&mut buf);
            let limit = rng.range(0, 600);
            assert_same("C39 h2", &buf, 0, limit, &hdr);
        }
    }
}

// ---------------------------------------------------------------------------
// C40 — broad fuzz over raw random bitstreams
// ---------------------------------------------------------------------------

#[test]
fn c40_random_fuzz() {
    let mut rng = Rng::new();
    let mut buf = vec![0u8; 512];
    let iters = if std::env::var_os("QUICK").is_some() { 20_000 } else { 200_000 };
    for it in 0..iters {
        rng.fill(&mut buf);
        let hdr = [
            rng.next_u32() as u8,
            rng.next_u32() as u8,
            rng.next_u32() as u8,
            rng.next_u32() as u8,
        ];
        // Keep pos/limit inside the buffer so neither side reads out of bounds.
        let pos = rng.range(0, 64);
        let limit = match rng.below(4) {
            0 => pos,                       // nothing available
            1 => pos + rng.range(0, 8),     // truncates almost immediately
            2 => rng.range(0, 3200),        // arbitrary, may truncate mid-field
            _ => 3800,                      // plenty
        };
        assert_same(&format!("C40 it={it}"), &buf, pos, limit, &hdr);
    }
}
