//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row group.
//!
//! Both implementations are loaded as shared objects with `libloading`; the Rust
//! code is exercised only through its exported `read_side_info` symbol.

mod common;

use common::*;

const SEED: u64 = 0x5EED_0001;

fn rng_for(row: &str) -> Pcg32 {
    // FNV-1a of the row id, mixed into the master seed: reproducible per row.
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in row.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    Pcg32::new(SEED ^ h)
}

/// Fixed EXT/MONO/sr_idx, fixed block config for every granule.
fn row_fixed(row: &str, ext: bool, mono: bool, sr: i32, blocks: Vec<Blk>, pre: PreMode, iters: usize) {
    let mut rng = rng_for(row);
    for _ in 0..iters {
        let hdr = hdr_for(ext, mono, sr, &mut rng);
        let opts = BuildOpts {
            blocks: blocks.clone(),
            pre,
            ..Default::default()
        };
        check(row, hdr, &opts, &mut rng);
    }
}

/// Sweep sr_idx over `srs`, random block per granule.
fn row_sr_sweep(row: &str, ext: bool, mono: bool, srs: &[i32], iters_per: usize) {
    let mut rng = rng_for(row);
    for &sr in srs {
        for _ in 0..iters_per {
            let hdr = hdr_for(ext, mono, sr, &mut rng);
            let n = gr_count_of(&hdr) as usize;
            let blocks: Vec<Blk> = (0..n).map(|_| Blk::rand(&mut rng)).collect();
            let opts = BuildOpts {
                blocks,
                ..Default::default()
            };
            check(row, hdr, &opts, &mut rng);
        }
    }
}

/// Sweep the start bit alignment 0..7 (and a few whole-byte offsets on top).
fn row_align_sweep(row: &str, ext: bool, mono: bool, srs: &[i32], iters_per: usize) {
    let mut rng = rng_for(row);
    for byte_off in [0usize, 1, 5, 17] {
        for s in 0..8usize {
            for _ in 0..iters_per {
                let sr = srs[rng.below(srs.len() as u32) as usize];
                let hdr = hdr_for(ext, mono, sr, &mut rng);
                let n = gr_count_of(&hdr) as usize;
                let blocks: Vec<Blk> = (0..n).map(|_| Blk::rand(&mut rng)).collect();
                let opts = BuildOpts {
                    blocks,
                    start_bit: byte_off * 8 + s,
                    ..Default::default()
                };
                check(row, hdr, &opts, &mut rng);
            }
        }
    }
}

/// Everything random within the given EXT/MONO group.
fn row_random(row: &str, ext: bool, mono: bool, srs: &[i32], iters: usize) {
    let mut rng = rng_for(row);
    for _ in 0..iters {
        let sr = srs[rng.below(srs.len() as u32) as usize];
        let hdr = hdr_for(ext, mono, sr, &mut rng);
        let n = gr_count_of(&hdr) as usize;
        let blocks: Vec<Blk> = (0..n).map(|_| Blk::rand(&mut rng)).collect();
        let pre = [PreMode::Any, PreMode::On, PreMode::Off][rng.below(3) as usize];
        let limit = match rng.below(4) {
            0 => LimitMode::Ample,
            1 => LimitMode::ExactBoundary,
            2 => LimitMode::Literal(rng.range_i32(0, 400)),
            _ => LimitMode::Literal(rng.range_i32(-8, 4096)),
        };
        let opts = BuildOpts {
            blocks,
            pre,
            limit,
            start_bit: rng.below(64) as usize,
            ..Default::default()
        };
        check(row, hdr, &opts, &mut rng);
    }
}

const EXT0_SRS: [i32; 6] = [0, 1, 2, 3, 4, 5];
const EXT1_SRS: [i32; 7] = [2, 3, 4, 5, 6, 7, 8];

// ---------------------------------------------------------------------------
// Group A — EXT=0, stereo (gr_count = 2)
// ---------------------------------------------------------------------------

#[test]
fn a1_ext0_stereo_long() {
    row_fixed("A1", false, false, 0, vec![Blk::L], PreMode::Any, 256);
}
#[test]
fn a2_ext0_stereo_bt1() {
    row_fixed("A2", false, false, 0, vec![Blk::S1], PreMode::Any, 256);
}
#[test]
fn a3_ext0_stereo_bt2_short() {
    row_fixed("A3", false, false, 0, vec![Blk::S2M0], PreMode::Any, 256);
}
#[test]
fn a4_ext0_stereo_bt2_mixed() {
    row_fixed("A4", false, false, 0, vec![Blk::S2M1], PreMode::Any, 256);
}
#[test]
fn a5_ext0_stereo_bt3() {
    row_fixed("A5", false, false, 0, vec![Blk::S3], PreMode::Any, 256);
}
#[test]
fn a6_ext0_stereo_seq_l_short() {
    row_fixed("A6", false, false, 0, vec![Blk::L, Blk::S2M0], PreMode::Any, 256);
}
#[test]
fn a7_ext0_stereo_seq_mixed_l() {
    row_fixed("A7", false, false, 0, vec![Blk::S2M1, Blk::L], PreMode::Any, 256);
}
#[test]
fn a8_ext0_stereo_sr_sweep() {
    row_sr_sweep("A8", false, false, &EXT0_SRS, 128);
}
#[test]
fn a9_ext0_stereo_align_sweep() {
    row_align_sweep("A9", false, false, &EXT0_SRS, 8);
}
#[test]
fn a10_ext0_stereo_preflag_on() {
    row_fixed("A10", false, false, 3, vec![Blk::L], PreMode::On, 256);
}
#[test]
fn a11_ext0_stereo_preflag_off() {
    row_fixed("A11", false, false, 3, vec![Blk::S2M1], PreMode::Off, 256);
}
#[test]
fn a12_ext0_stereo_random() {
    row_random("A12", false, false, &EXT0_SRS, 1024);
}

// ---------------------------------------------------------------------------
// Group B — EXT=0, mono (gr_count = 1)
// ---------------------------------------------------------------------------

#[test]
fn b1_ext0_mono_long() {
    row_fixed("B1", false, true, 0, vec![Blk::L], PreMode::Any, 256);
}
#[test]
fn b2_ext0_mono_bt1() {
    row_fixed("B2", false, true, 0, vec![Blk::S1], PreMode::Any, 256);
}
#[test]
fn b3_ext0_mono_bt2_short() {
    row_fixed("B3", false, true, 0, vec![Blk::S2M0], PreMode::Any, 256);
}
#[test]
fn b4_ext0_mono_bt2_mixed() {
    row_fixed("B4", false, true, 0, vec![Blk::S2M1], PreMode::Any, 256);
}
#[test]
fn b5_ext0_mono_bt3() {
    row_fixed("B5", false, true, 0, vec![Blk::S3], PreMode::Any, 256);
}
#[test]
fn b6_ext0_mono_seq_l_short() {
    row_fixed("B6", false, true, 0, vec![Blk::L, Blk::S2M0], PreMode::Any, 256);
}
#[test]
fn b7_ext0_mono_seq_mixed_l() {
    row_fixed("B7", false, true, 0, vec![Blk::S2M1, Blk::L], PreMode::Any, 256);
}
#[test]
fn b8_ext0_mono_sr_sweep() {
    row_sr_sweep("B8", false, true, &EXT0_SRS, 128);
}
#[test]
fn b9_ext0_mono_align_sweep() {
    row_align_sweep("B9", false, true, &EXT0_SRS, 8);
}
#[test]
fn b10_ext0_mono_preflag_on() {
    row_fixed("B10", false, true, 4, vec![Blk::L], PreMode::On, 256);
}
#[test]
fn b11_ext0_mono_preflag_off() {
    row_fixed("B11", false, true, 4, vec![Blk::S2M1], PreMode::Off, 256);
}
#[test]
fn b12_ext0_mono_random() {
    row_random("B12", false, true, &EXT0_SRS, 1024);
}

// ---------------------------------------------------------------------------
// Group C — EXT=8, stereo (gr_count = 4)
// ---------------------------------------------------------------------------

#[test]
fn c1_ext1_stereo_long() {
    row_fixed("C1", true, false, 2, vec![Blk::L], PreMode::Any, 256);
}
#[test]
fn c2_ext1_stereo_bt1() {
    row_fixed("C2", true, false, 2, vec![Blk::S1], PreMode::Any, 256);
}
#[test]
fn c3_ext1_stereo_bt2_short() {
    row_fixed("C3", true, false, 2, vec![Blk::S2M0], PreMode::Any, 256);
}
#[test]
fn c4_ext1_stereo_bt2_mixed() {
    row_fixed("C4", true, false, 2, vec![Blk::S2M1], PreMode::Any, 256);
}
#[test]
fn c5_ext1_stereo_bt3() {
    row_fixed("C5", true, false, 2, vec![Blk::S3], PreMode::Any, 256);
}
#[test]
fn c6_ext1_stereo_four_distinct_granules() {
    row_fixed(
        "C6",
        true,
        false,
        7,
        vec![Blk::L, Blk::S2M0, Blk::S2M1, Blk::S3],
        PreMode::Any,
        256,
    );
}
#[test]
fn c7_ext1_stereo_seq_mixed_l() {
    row_fixed("C7", true, false, 7, vec![Blk::S2M1, Blk::L], PreMode::Any, 256);
}
#[test]
fn c8_ext1_stereo_sr_sweep() {
    row_sr_sweep("C8", true, false, &EXT1_SRS, 128);
}
#[test]
fn c9_ext1_stereo_align_sweep() {
    row_align_sweep("C9", true, false, &EXT1_SRS, 8);
}
#[test]
fn c10_ext1_stereo_preflag_on() {
    row_fixed("C10", true, false, 5, vec![Blk::L], PreMode::On, 256);
}
#[test]
fn c11_ext1_stereo_preflag_off() {
    row_fixed("C11", true, false, 5, vec![Blk::S2M1], PreMode::Off, 256);
}
#[test]
fn c12_ext1_stereo_random() {
    row_random("C12", true, false, &EXT1_SRS, 1024);
}

// ---------------------------------------------------------------------------
// Group D — EXT=8, mono (gr_count = 2)
// ---------------------------------------------------------------------------

#[test]
fn d1_ext1_mono_long() {
    row_fixed("D1", true, true, 2, vec![Blk::L], PreMode::Any, 256);
}
#[test]
fn d2_ext1_mono_bt1() {
    row_fixed("D2", true, true, 2, vec![Blk::S1], PreMode::Any, 256);
}
#[test]
fn d3_ext1_mono_bt2_short() {
    row_fixed("D3", true, true, 2, vec![Blk::S2M0], PreMode::Any, 256);
}
#[test]
fn d4_ext1_mono_bt2_mixed() {
    row_fixed("D4", true, true, 2, vec![Blk::S2M1], PreMode::Any, 256);
}
#[test]
fn d5_ext1_mono_bt3() {
    row_fixed("D5", true, true, 2, vec![Blk::S3], PreMode::Any, 256);
}
#[test]
fn d6_ext1_mono_sr8_seq_l_short() {
    row_fixed("D6", true, true, 8, vec![Blk::L, Blk::S2M0], PreMode::Any, 256);
}
#[test]
fn d7_ext1_mono_sr8_seq_mixed_bt1() {
    row_fixed("D7", true, true, 8, vec![Blk::S2M1, Blk::S1], PreMode::Any, 256);
}
#[test]
fn d8_ext1_mono_sr_sweep() {
    row_sr_sweep("D8", true, true, &EXT1_SRS, 128);
}
#[test]
fn d9_ext1_mono_align_sweep() {
    row_align_sweep("D9", true, true, &EXT1_SRS, 8);
}
#[test]
fn d10_ext1_mono_preflag_on() {
    row_fixed("D10", true, true, 6, vec![Blk::S2M0], PreMode::On, 256);
}
#[test]
fn d11_ext1_mono_preflag_off() {
    row_fixed("D11", true, true, 6, vec![Blk::L], PreMode::Off, 256);
}
#[test]
fn d12_ext1_mono_random() {
    row_random("D12", true, true, &EXT1_SRS, 1024);
}

// ---------------------------------------------------------------------------
// Group X — cross-cutting shapes
// ---------------------------------------------------------------------------

/// X1: sr_idx == 8 is out of range for the `[8][...]` tables. The C computes
/// `&table[8][0]` anyway; the Rust must land at exactly the same offset.
#[test]
fn x1_sr_idx_8_out_of_range_offset() {
    let mut rng = rng_for("X1");
    for mono in [false, true] {
        for blocks in [
            vec![Blk::L],
            vec![Blk::S1],
            vec![Blk::S2M0],
            vec![Blk::S2M1],
            vec![Blk::S3],
        ] {
            for _ in 0..64 {
                let hdr = hdr_for(true, mono, 8, &mut rng);
                assert_eq!(sr_idx_of(&hdr), 8);
                let opts = BuildOpts {
                    blocks: blocks.clone(),
                    ..Default::default()
                };
                let input = build(hdr, &opts, &mut rng);
                compare("X1", &input);
                // Independently confirm the offset really is one row past the end.
                let (c, r) = impls();
                let (cg, _, _) = unsafe { run(c, &input) };
                let (rg, _, _) = unsafe { run(r, &input) };
                let t = match blocks[0] {
                    Blk::S2M0 => T_SHORT,
                    Blk::S2M1 => T_MIXED,
                    _ => T_LONG,
                };
                let coff = (cg[0].sfbtab as usize).wrapping_sub(c.table_bases[t]);
                let roff = (rg[0].sfbtab as usize).wrapping_sub(r.table_bases[t]);
                assert_eq!(coff, 8 * ROW_SIZE[t], "X1: C offset {coff} != 8*{}", ROW_SIZE[t]);
                assert_eq!(roff, coff, "X1: Rust offset {roff} != C offset {coff}");
            }
        }
    }
}

/// X2: reservoir exactly exhausted (`>` not `>=`, so this is accepted).
#[test]
fn x2_reservoir_exact_boundary() {
    let mut rng = rng_for("X2");
    for ext in [false, true] {
        for mono in [false, true] {
            let srs: &[i32] = if ext { &EXT1_SRS } else { &EXT0_SRS };
            for _ in 0..256 {
                let sr = srs[rng.below(srs.len() as u32) as usize];
                let hdr = hdr_for(ext, mono, sr, &mut rng);
                let n = gr_count_of(&hdr) as usize;
                let blocks: Vec<Blk> = (0..n).map(|_| Blk::rand(&mut rng)).collect();
                let opts = BuildOpts {
                    blocks,
                    main_data_begin: Some(0),
                    part_23_length: Some(1 + rng.below(4095)),
                    limit: LimitMode::ExactBoundary,
                    ..Default::default()
                };
                let input = build(hdr, &opts, &mut rng);
                compare("X2", &input);
                // The boundary case must be ACCEPTED (returns main_data_begin = 0).
                let (c, _) = impls();
                let (_, _, rv) = unsafe { run(c, &input) };
                assert_eq!(rv, 0, "X2: exact boundary should be accepted: {}", input.desc);
            }
        }
    }
}

/// X3: large `main_data_begin` makes the final check pass with a small limit.
#[test]
fn x3_large_main_data_begin() {
    let mut rng = rng_for("X3");
    for ext in [false, true] {
        for mono in [false, true] {
            let srs: &[i32] = if ext { &EXT1_SRS } else { &EXT0_SRS };
            let mdb_max = if ext { 511 } else { 255 };
            for _ in 0..256 {
                let sr = srs[rng.below(srs.len() as u32) as usize];
                let hdr = hdr_for(ext, mono, sr, &mut rng);
                let n = gr_count_of(&hdr) as usize;
                let blocks: Vec<Blk> = (0..n).map(|_| Blk::rand(&mut rng)).collect();
                let opts = BuildOpts {
                    blocks,
                    main_data_begin: Some(mdb_max - rng.below(4)),
                    limit: LimitMode::Literal(rng.range_i32(60, 300)),
                    ..Default::default()
                };
                check("X3", hdr, &opts, &mut rng);
            }
        }
    }
}

/// X4: `hdr[0]` is never read by the C, so varying it must change nothing.
#[test]
fn x4_hdr0_ignored() {
    let mut rng = rng_for("X4");
    for _ in 0..32 {
        let base_hdr = hdr_for(rng.bool(), rng.bool(), 2, &mut rng);
        let n = gr_count_of(&base_hdr) as usize;
        let blocks: Vec<Blk> = (0..n).map(|_| Blk::rand(&mut rng)).collect();
        let opts = BuildOpts {
            blocks,
            ..Default::default()
        };
        let mut proto = build(base_hdr, &opts, &mut rng);
        let (c, _) = impls();
        let (_, _, rv0) = unsafe { run(c, &proto) };
        for h0 in 0..=255u8 {
            proto.hdr[0] = h0;
            compare("X4", &proto);
            let (_, _, rv) = unsafe { run(c, &proto) };
            assert_eq!(rv, rv0, "X4: hdr[0]={h0} changed the C result");
        }
    }
}

/// X5: full-space fuzz over every bit the C reads from `hdr`, plus random
/// buffers, positions and limits. The bitstream is pure noise here, so the
/// granule fields, error paths and reservoir check are all hit at random.
#[test]
fn x5_full_space_fuzz() {
    let mut rng = rng_for("X5");
    let (c, r) = impls();
    for i in 0..20_000 {
        let mut hdr = [0u8; 4];
        for b in hdr.iter_mut() {
            *b = rng.next_u32() as u8;
        }
        let mut buf = vec![0u8; BUF_BYTES];
        for b in buf.iter_mut() {
            *b = rng.next_u32() as u8;
        }
        let pos = rng.range_i32(0, 63);
        let limit = match rng.below(5) {
            0 => rng.range_i32(-4, 40),
            1 => rng.range_i32(0, 200),
            2 => rng.range_i32(150, 400),
            3 => rng.range_i32(0, 5000),
            _ => 4096,
        };
        let input = Input {
            hdr,
            buf,
            pos,
            limit,
            desc: format!("X5 iter {i}: hdr={hdr:02x?} pos={pos} limit={limit}"),
        };
        compare("X5", &input);
        let _ = (c, r);
    }
}

/// X6: misaligned start plus a limit that cuts the decode off mid-stream, so
/// `get_bits` starts returning 0 part-way through.
#[test]
fn x6_partial_decode_truncated_reservoir() {
    let mut rng = rng_for("X6");
    for ext in [false, true] {
        for mono in [false, true] {
            let srs: &[i32] = if ext { &EXT1_SRS } else { &EXT0_SRS };
            for start in 0..16usize {
                for cut in 0..64i32 {
                    let sr = srs[rng.below(srs.len() as u32) as usize];
                    let hdr = hdr_for(ext, mono, sr, &mut rng);
                    let n = gr_count_of(&hdr) as usize;
                    let blocks: Vec<Blk> = (0..n).map(|_| Blk::rand(&mut rng)).collect();
                    let opts = BuildOpts {
                        blocks,
                        start_bit: start,
                        limit: LimitMode::Literal(start as i32 + cut),
                        ..Default::default()
                    };
                    check("X6", hdr, &opts, &mut rng);
                }
            }
        }
    }
}

/// X7: extreme `scfsi` field patterns across EXT/MONO — drives the
/// `scfsi <<= 4`, `scfsi &= 0x0F0F` and `(scfsi >> 12) & 15` interaction.
#[test]
fn x7_scfsi_patterns() {
    let mut rng = rng_for("X7");
    for ext in [false, true] {
        for mono in [false, true] {
            let srs: &[i32] = if ext { &EXT1_SRS } else { &EXT0_SRS };
            for pat in [
                0x0000u32, 0xFFFF, 0xAAAA, 0x5555, 0x0F0F, 0xF0F0, 0x00FF, 0xFF00, 0x1248, 0x8421,
            ] {
                for blocks in [
                    vec![Blk::L],
                    vec![Blk::S2M0],
                    vec![Blk::S2M1],
                    vec![Blk::S1, Blk::S2M0],
                    vec![Blk::L, Blk::S2M1],
                ] {
                    for _ in 0..8 {
                        let sr = srs[rng.below(srs.len() as u32) as usize];
                        let hdr = hdr_for(ext, mono, sr, &mut rng);
                        let opts = BuildOpts {
                            blocks: blocks.clone(),
                            scfsi: Some(pat),
                            ..Default::default()
                        };
                        check("X7", hdr, &opts, &mut rng);
                    }
                }
            }
        }
    }
}

/// X8: exactly `gr_count` granules are written; slots `gr_count..8` must stay
/// at the pre-fill sentinel in BOTH implementations.
#[test]
fn x8_granule_count_boundary() {
    let mut rng = rng_for("X8");
    let (c, r) = impls();
    for (ext, mono, want) in [
        (false, true, 1usize),
        (false, false, 2),
        (true, true, 2),
        (true, false, 4),
    ] {
        let srs: &[i32] = if ext { &EXT1_SRS } else { &EXT0_SRS };
        for _ in 0..128 {
            let sr = srs[rng.below(srs.len() as u32) as usize];
            let hdr = hdr_for(ext, mono, sr, &mut rng);
            assert_eq!(gr_count_of(&hdr) as usize, want);
            let blocks: Vec<Blk> = (0..want).map(|_| Blk::rand(&mut rng)).collect();
            let opts = BuildOpts {
                blocks,
                ..Default::default()
            };
            let input = build(hdr, &opts, &mut rng);
            compare("X8", &input);
            for (im, name) in [(c, "C"), (r, "Rust")] {
                let (grs, _, _) = unsafe { run(im, &input) };
                for g in want..GR_SLOTS {
                    assert!(
                        grs[g].sfbtab.is_null()
                            && grs[g].part_23_length == 0xA5A5
                            && grs[g].scfsi == 0xA5,
                        "X8 [{name}]: granule slot {g} was written even though gr_count={want}"
                    );
                }
                // And the granules that SHOULD be written are.
                for g in 0..want {
                    assert!(
                        !grs[g].sfbtab.is_null(),
                        "X8 [{name}]: granule slot {g} not written (gr_count={want})"
                    );
                }
            }
        }
    }
}

/// X9: exhaustive scalefactor-table content check.
///
/// For every table (`g_scf_long`, `g_scf_short`, `g_scf_mixed`) and every
/// in-range row index 0..=7, force the configuration that selects it and compare
/// the whole pointed-to row byte-for-byte between the two shared objects. This
/// pins down all 8*23 + 2*8*40 = 824 table literals (including the implicit
/// zero-fill of the short C initialiser lists in `g_scf_mixed`) rather than
/// relying on the random sweeps to happen to hit every row.
#[test]
fn x9_exhaustive_table_rows() {
    let mut rng = rng_for("X9");
    let (c, r) = impls();
    let mut covered = [[false; 8]; 3];

    for sr in 0..8i32 {
        // sr_idx 0..1 are only reachable with EXT=0, 6..8 only with EXT=8.
        let exts: Vec<bool> = match sr {
            0 | 1 => vec![false],
            2..=5 => vec![false, true],
            _ => vec![true],
        };
        for ext in exts {
            for (t, blk) in [(T_LONG, Blk::L), (T_SHORT, Blk::S2M0), (T_MIXED, Blk::S2M1)] {
                for mono in [false, true] {
                    let hdr = hdr_for(ext, mono, sr, &mut rng);
                    assert_eq!(sr_idx_of(&hdr), sr);
                    let opts = BuildOpts {
                        blocks: vec![blk],
                        ..Default::default()
                    };
                    let input = build(hdr, &opts, &mut rng);
                    compare("X9", &input);

                    let (cg, _, _) = unsafe { run(c, &input) };
                    let (rg, _, _) = unsafe { run(r, &input) };
                    let cp = cg[0].sfbtab as usize;
                    let rp = rg[0].sfbtab as usize;
                    assert_eq!(
                        cp.wrapping_sub(c.table_bases[t]),
                        sr as usize * ROW_SIZE[t],
                        "X9: C selected the wrong {} row for sr_idx={sr}",
                        TABLE_NAME[t]
                    );
                    assert_eq!(
                        rp.wrapping_sub(r.table_bases[t]),
                        sr as usize * ROW_SIZE[t],
                        "X9: Rust selected the wrong {} row for sr_idx={sr}",
                        TABLE_NAME[t]
                    );
                    let n = ROW_SIZE[t];
                    let crow = unsafe { std::slice::from_raw_parts(cp as *const u8, n) };
                    let rrow = unsafe { std::slice::from_raw_parts(rp as *const u8, n) };
                    assert_eq!(
                        crow, rrow,
                        "X9: {}[{sr}] differs\n  C   = {crow:?}\n  Rust= {rrow:?}",
                        TABLE_NAME[t]
                    );
                    covered[t][sr as usize] = true;
                }
            }
        }
    }

    for t in 0..3 {
        for sr in 0..8 {
            assert!(covered[t][sr], "X9: {}[{sr}] was never checked", TABLE_NAME[t]);
        }
    }
}
