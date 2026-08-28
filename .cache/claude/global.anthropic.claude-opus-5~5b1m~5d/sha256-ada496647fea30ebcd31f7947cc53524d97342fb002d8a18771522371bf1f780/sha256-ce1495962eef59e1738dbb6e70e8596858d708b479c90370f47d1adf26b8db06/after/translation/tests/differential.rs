//! Differential tests: C `.so` vs Rust `.so`, both loaded with `libloading`.
//!
//! * Phase A artifacts: `SYMBOLS.md`, `ERRORS.md`, `CONFIGS.md`
//! * Phase B (`cfg_c*`): one test per `CONFIGS.md` row, randomized inputs
//! * Phase C (`err_e*`, `bnd_b*`): one test per `ERRORS.md` row

mod common;
use common::*;

/// Iterations per randomized configuration row.
const ITERS: usize = 400;

// ===========================================================================
// Phase A — surface checks
// ===========================================================================

/// Both `.so`s export `read_side_info`; loading either would fail otherwise.
#[test]
fn sym_read_side_info_exported_by_both() {
    let h = harness();
    assert!(h.c_fn as usize != 0);
    assert!(h.r_fn as usize != 0);
    assert_ne!(
        h.c_fn as usize, h.r_fn as usize,
        "both handles resolved to the same address — the two libraries are not \
         actually distinct"
    );
}

/// The three scalefactor-band tables must be byte-identical **and** laid out
/// identically, because `sr_idx` reaches 8 and indexes one row past the end of
/// whichever table is selected (CONFIGS C15–C17). Compares the whole 832-byte
/// `.rodata` window anchored at `g_scf_long[0]` in each library.
#[test]
fn sym_layout_matches_c_rodata() {
    let h = harness();
    let n = RODATA_TABLE_BYTES as usize;
    let c = unsafe { std::slice::from_raw_parts(h.c_long_base, n) };
    let r = unsafe { std::slice::from_raw_parts(h.r_long_base, n) };
    if c != r {
        let first = (0..n).find(|&i| c[i] != r[i]).unwrap();
        panic!(
            "scalefactor table blob differs at offset {first}: C = {:#04x}, Rust = {:#04x}\n\
             (offsets: g_scf_long 0..184, pad 184..192, g_scf_short 192..512, \
             g_scf_mixed 512..832)\n  C[{first}..]    = {:02x?}\n  Rust[{first}..] = {:02x?}",
            c[first],
            r[first],
            &c[first..(first + 24).min(n)],
            &r[first..(first + 24).min(n)],
        );
    }
    // Spot-check the anchors the layout depends on.
    assert_eq!(&c[0..6], &[6, 6, 6, 6, 6, 6], "g_scf_long[0] start");
    assert_eq!(&c[184..192], &[0u8; 8], "8 pad bytes after g_scf_long");
    assert_eq!(&c[192..201], &[4u8; 9], "g_scf_short[0] start");
    assert_eq!(&c[512..521], &[6u8; 9], "g_scf_mixed[0] start");
}

// ===========================================================================
// Shared row driver
// ===========================================================================

/// Runs `iters` randomized cases for one fixed input shape. `mkgran` selects
/// the per-granule branch (window switching / block type / mixed flag) so a row
/// pins the *configuration* while every field *value* is randomized.
fn row<F>(label: &str, seed: u64, iters: usize, hs: HdrSpec, mut mkgran: F) -> Coverage
where
    F: FnMut(&mut Rng, usize) -> GranuleSpec,
{
    let h = harness();
    let mut rng = Rng::new(seed);
    let mut cov = Coverage::new();
    let gc = hs.gr_count() as usize;
    for it in 0..iters {
        let gs: Vec<GranuleSpec> = (0..gc).map(|i| mkgran(&mut rng, i)).collect();
        let sc = Scenario::new(hs, gs)
            .start_bit(rng.below(64) as usize)
            .scfsi(rng.bits(11));
        let mut case = sc.case(&mut rng);
        case.prefill = PREFILLS[it % PREFILLS.len()];
        let o = h.assert_same(&format!("{label} it={it}"), &case);
        assert_eq!(
            o.ret, sc.main_data_begin as i32,
            "{label} it={it}: expected the ample-budget parse to return \
             main_data_begin, got {}",
            o.ret
        );
        cov.observe(&o, gc);
    }
    cov
}

/// A granule on the non-window-switching (long block) path.
fn gran_long(rng: &mut Rng) -> GranuleSpec {
    let mut g = GranuleSpec::random(rng);
    g.ws = false;
    g
}

/// A granule on the window-switching path with a given block type / mixed flag.
fn gran_ws(rng: &mut Rng, block_type: u32, mixed: u32) -> GranuleSpec {
    let mut g = GranuleSpec::random(rng);
    g.ws = true;
    g.block_type = block_type;
    g.mixed = mixed;
    g
}

const MPEG2_MONO: HdrSpec = HdrSpec {
    mpeg1: false,
    hdr1_bit4: false,
    sr2: 0,
    mono: true,
};
const MPEG2_STEREO: HdrSpec = HdrSpec {
    mpeg1: false,
    hdr1_bit4: false,
    sr2: 1,
    mono: false,
};
const MPEG1_MONO: HdrSpec = HdrSpec {
    mpeg1: true,
    hdr1_bit4: false,
    sr2: 2,
    mono: true,
};
const MPEG1_STEREO: HdrSpec = HdrSpec {
    mpeg1: true,
    hdr1_bit4: true,
    sr2: 1,
    mono: false,
};

// ===========================================================================
// Phase B — CONFIGS.md rows
// ===========================================================================

/// C1: MPEG2 mono, `gr_count == 1`, long block.
#[test]
fn cfg_c1_mpeg2_mono_long() {
    assert_eq!(MPEG2_MONO.gr_count(), 1);
    let cov = row("C1", 0x0C01, ITERS, MPEG2_MONO, |r, _| gran_long(r));
    cov.require(&["bt=0 mixed=0 table=long"]);
}

/// C2: MPEG2 mono, window switching, `block_type == 1`.
#[test]
fn cfg_c2_mpeg2_mono_ws_bt1() {
    let cov = row("C2", 0x0C02, ITERS, MPEG2_MONO, |r, _| {
        let m = r.bits(1);
        gran_ws(r, 1, m)
    });
    cov.require(&["bt=1 mixed=0 table=long", "bt=1 mixed=1 table=long"]);
}

/// C3: MPEG2 mono, `block_type == 2`, `mixed_block_flag == 0` → `g_scf_short`.
#[test]
fn cfg_c3_mpeg2_mono_short() {
    let cov = row("C3", 0x0C03, ITERS, MPEG2_MONO, |r, _| gran_ws(r, 2, 0));
    cov.require(&["bt=2 mixed=0 table=short"]);
}

/// C4: MPEG2 mono, `block_type == 2`, `mixed_block_flag == 1` → `g_scf_mixed`,
/// `n_long_sfb == 6` (the MPEG2 value).
#[test]
fn cfg_c4_mpeg2_mono_mixed() {
    let cov = row("C4", 0x0C04, ITERS, MPEG2_MONO, |r, _| gran_ws(r, 2, 1));
    cov.require(&["bt=2 mixed=1 table=mixed"]);
}

/// C5: MPEG2 mono, window switching, `block_type == 3`.
#[test]
fn cfg_c5_mpeg2_mono_ws_bt3() {
    let cov = row("C5", 0x0C05, ITERS, MPEG2_MONO, |r, _| {
        let m = r.bits(1);
        gran_ws(r, 3, m)
    });
    cov.require(&["bt=3 mixed=0 table=long", "bt=3 mixed=1 table=long"]);
}

/// C6: MPEG2 stereo, `gr_count == 2`, both granules long.
#[test]
fn cfg_c6_mpeg2_stereo_long() {
    assert_eq!(MPEG2_STEREO.gr_count(), 2);
    row("C6", 0x0C06, ITERS, MPEG2_STEREO, |r, _| gran_long(r));
}

/// C7: MPEG2 stereo with a *different* branch in each granule.
#[test]
fn cfg_c7_mpeg2_stereo_mixed_shapes() {
    let cov = row("C7", 0x0C07, ITERS, MPEG2_STEREO, |r, i| {
        if i == 0 { gran_long(r) } else { gran_ws(r, 2, 0) }
    });
    cov.require(&["bt=0 mixed=0 table=long", "bt=2 mixed=0 table=short"]);
}

/// C8: MPEG1 mono → `gr_count == 2`, 9-bit `main_data_begin`, 9-bit `scfsi`.
#[test]
fn cfg_c8_mpeg1_mono_long() {
    assert_eq!(MPEG1_MONO.gr_count(), 2);
    row("C8", 0x0C08, ITERS, MPEG1_MONO, |r, _| gran_long(r));
}

/// C9: MPEG1 mixed block → `n_long_sfb == 8`, the MPEG1-only value.
#[test]
fn cfg_c9_mpeg1_mixed_n_long_sfb_8() {
    let h = harness();
    let cov = row("C9", 0x0C09, ITERS, MPEG1_MONO, |r, _| gran_ws(r, 2, 1));
    cov.require(&["bt=2 mixed=1 table=mixed"]);
    // Pin the MPEG1/MPEG2 difference explicitly: same granule shape, only the
    // mpeg1 bit changes, and n_long_sfb must go 8 -> 6.
    let mut rng = Rng::new(0xC09B);
    for (hs, want) in [(MPEG1_MONO, 8u8), (MPEG2_MONO, 6u8)] {
        let gs: Vec<GranuleSpec> = (0..hs.gr_count())
            .map(|_| gran_ws(&mut rng, 2, 1))
            .collect();
        let sc = Scenario::new(hs, gs);
        let case = sc.case(&mut rng);
        let o = h.assert_same("C9-pin", &case);
        assert_eq!(o.gr[0].n_long_sfb, want, "mpeg1={} n_long_sfb", hs.mpeg1);
        assert_eq!(o.gr[0].n_short_sfb, 30);
    }
}

/// C10: MPEG1 stereo → `gr_count == 4`, 11-bit `scfsi`.
#[test]
fn cfg_c10_mpeg1_stereo_four_granules() {
    assert_eq!(MPEG1_STEREO.gr_count(), 4);
    let h = harness();
    let cov = row("C10", 0x0C10, ITERS, MPEG1_STEREO, |r, _| gran_long(r));
    cov.require(&["bt=0 mixed=0 table=long"]);
    // All four granules written, the spare slots untouched.
    let mut rng = Rng::new(0xC10B);
    let gs: Vec<GranuleSpec> = (0..4).map(|_| gran_long(&mut rng)).collect();
    let sc = Scenario::new(MPEG1_STEREO, gs);
    let mut case = sc.case(&mut rng);
    case.prefill = 0xAA;
    let o = h.assert_same("C10-spare", &case);
    for i in 0..4 {
        assert!(matches!(o.gr[i].sfbtab, Sfbtab::Assigned { .. }), "granule {i}");
    }
    for i in 4..NGR {
        assert!(matches!(o.gr[i].sfbtab, Sfbtab::Untouched(_)), "spare slot {i}");
    }
}

/// C11: MPEG1 stereo, a different branch in each of the four granules.
#[test]
fn cfg_c11_mpeg1_stereo_per_granule_shapes() {
    let cov = row("C11", 0x0C11, ITERS, MPEG1_STEREO, |r, i| match i {
        0 => gran_long(r),
        1 => gran_ws(r, 1, 0),
        2 => gran_ws(r, 2, 0),
        _ => gran_ws(r, 2, 1),
    });
    cov.require(&[
        "bt=0 mixed=0 table=long",
        "bt=1 mixed=0 table=long",
        "bt=2 mixed=0 table=short",
        "bt=2 mixed=1 table=mixed",
    ]);
}

// --- C12..C17: the sr_idx sweep, including the out-of-range row 8 ----------

/// Drives every reachable header-bit combination against one table selection
/// and checks the resulting `sfbtab` lands on the expected row.
fn sr_idx_sweep(label: &str, seed: u64, ws: bool, block_type: u32, mixed: u32) -> Vec<i32> {
    let h = harness();
    let mut rng = Rng::new(seed);
    let mut seen = Vec::new();
    for (mpeg1, hdr1_bit4, sr2, sr_idx) in hdr_bit_combos() {
        for mono in [true, false] {
            let hs = HdrSpec { mpeg1, hdr1_bit4, sr2, mono };
            assert_eq!(hs.sr_idx(), sr_idx);
            for it in 0..40 {
                let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                    .map(|_| {
                        if ws {
                            gran_ws(&mut rng, block_type, mixed)
                        } else {
                            gran_long(&mut rng)
                        }
                    })
                    .collect();
                let sc = Scenario::new(hs, gs).start_bit(rng.below(16) as usize);
                let mut case = sc.case(&mut rng);
                case.prefill = PREFILLS[it % PREFILLS.len()];
                let o = h.assert_same(
                    &format!("{label} sr_idx={sr_idx} mono={mono} it={it}"),
                    &case,
                );
                let want = expected_sfbtab_offset(ws, block_type, mixed, sr_idx);
                match &o.gr[0].sfbtab {
                    Sfbtab::Assigned { offset, .. } => assert_eq!(
                        *offset, want,
                        "{label} sr_idx={sr_idx}: sfbtab offset from g_scf_long[0]"
                    ),
                    other => panic!("{label}: sfbtab not assigned: {other:?}"),
                }
            }
            if !seen.contains(&sr_idx) {
                seen.push(sr_idx);
            }
        }
    }
    seen.sort();
    seen
}

/// C12 + C15: `sr_idx = 0..=8` on the long table (`sr_idx == 8` is one row past
/// the end of `g_scf_long`, aliasing the pad bytes plus `g_scf_short[0]`).
#[test]
fn cfg_c12_c15_sr_idx_sweep_long() {
    let seen = sr_idx_sweep("C12/C15-long", 0x0C12, false, 0, 0);
    assert_eq!(seen, (0..=8).collect::<Vec<i32>>(), "every sr_idx reached");
}

/// C13 + C16: `sr_idx = 0..=8` on the short table (`sr_idx == 8` aliases
/// `g_scf_mixed[0]` exactly).
#[test]
fn cfg_c13_c16_sr_idx_sweep_short() {
    let seen = sr_idx_sweep("C13/C16-short", 0x0C13, true, 2, 0);
    assert_eq!(seen, (0..=8).collect::<Vec<i32>>());
}

/// C14 + C17: `sr_idx = 0..=8` on the mixed table. Row 8 leaves `.rodata`
/// entirely in C, so only the offset and the scalar fields are compared there
/// (see the note in `CONFIGS.md`).
#[test]
fn cfg_c14_c17_sr_idx_sweep_mixed() {
    let seen = sr_idx_sweep("C14/C17-mixed", 0x0C14, true, 2, 1);
    assert_eq!(seen, (0..=8).collect::<Vec<i32>>());
}

/// C18: every start alignment `s = bs->pos & 7` and every byte offset, on both
/// branches.
#[test]
fn cfg_c18_start_alignment_sweep() {
    let h = harness();
    let mut rng = Rng::new(0x0C18);
    for pos in 0..64usize {
        for hs in [MPEG2_MONO, MPEG1_STEREO] {
            for ws in [false, true] {
                let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                    .map(|_| {
                        if ws {
                            gran_ws(&mut rng, 2, 1)
                        } else {
                            gran_long(&mut rng)
                        }
                    })
                    .collect();
                let sc = Scenario::new(hs, gs).start_bit(pos);
                let mut case = sc.case(&mut rng);
                case.prefill = PREFILLS[pos % PREFILLS.len()];
                let o = h.assert_same(&format!("C18 pos={pos} ws={ws}"), &case);
                assert_eq!(o.pos as usize, pos + sc.bits(), "C18 pos={pos}: end position");
            }
        }
    }
}

/// C19 + C20: MPEG2 `preflag` comes from `scalefac_compress >= 500`, including
/// the 499/500 boundary. MPEG1 reads it from the bitstream instead.
#[test]
fn cfg_c19_c20_preflag_from_scalefac_compress() {
    let h = harness();
    let mut rng = Rng::new(0x0C19);
    // Exhaustive over the whole 9-bit field.
    for sfc in 0..512u32 {
        let mut g = gran_long(&mut rng);
        g.scalefac_compress = sfc;
        let sc = Scenario::new(MPEG2_MONO, vec![g]);
        let case = sc.case(&mut rng);
        let o = h.assert_same(&format!("C19/C20 sfc={sfc}"), &case);
        assert_eq!(o.gr[0].scalefac_compress, sfc as u16);
        assert_eq!(
            o.gr[0].preflag,
            (sfc >= 500) as u8,
            "C20 boundary: sfc={sfc} preflag"
        );
    }
    // MPEG1: 4-bit field, preflag is a separate bitstream bit.
    for pf in [0u32, 1] {
        for sfc in 0..16u32 {
            let gs: Vec<GranuleSpec> = (0..MPEG1_MONO.gr_count())
                .map(|_| {
                    let mut g = gran_long(&mut rng);
                    g.scalefac_compress = sfc;
                    g.preflag = pf;
                    g
                })
                .collect();
            let sc = Scenario::new(MPEG1_MONO, gs);
            let case = sc.case(&mut rng);
            let o = h.assert_same(&format!("C19 mpeg1 sfc={sfc} pf={pf}"), &case);
            assert_eq!(o.gr[0].preflag, pf as u8);
            assert_eq!(o.gr[0].scalefac_compress, sfc as u16);
        }
    }
}

/// C21: `big_values` over its whole valid range `0..=288`.
#[test]
fn cfg_c21_big_values_full_valid_range() {
    let h = harness();
    let mut rng = Rng::new(0x0C21);
    for bv in 0..=288u32 {
        for hs in [MPEG2_MONO, MPEG1_STEREO] {
            let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                .map(|_| {
                    let mut g = gran_long(&mut rng);
                    g.big_values = bv;
                    g
                })
                .collect();
            let sc = Scenario::new(hs, gs);
            let case = sc.case(&mut rng);
            let o = h.assert_same(&format!("C21 bv={bv}"), &case);
            assert_eq!(o.ret, 0, "C21 bv={bv} must not be rejected");
            assert_eq!(o.gr[0].big_values, bv as u16);
        }
    }
}

/// C22: MPEG1 mono shifts `scfsi` left by 4 **twice** per iteration (top and
/// bottom of the loop), so granule 1 sees bits 12..15 of `scfsi << 4`.
#[test]
fn cfg_c22_scfsi_mono_double_shift() {
    let h = harness();
    let mut rng = Rng::new(0x0C22);
    for scfsi in 0..512u32 {
        let gs: Vec<GranuleSpec> = (0..MPEG1_MONO.gr_count())
            .map(|_| gran_long(&mut rng))
            .collect();
        let sc = Scenario::new(MPEG1_MONO, gs).scfsi(scfsi);
        let case = sc.case(&mut rng);
        h.assert_same(&format!("C22 scfsi={scfsi}"), &case);
    }
}

/// C23: MPEG1 stereo distributes an 11-bit `scfsi` over four granules, with
/// `scfsi &= 0x0F0F` applied whenever a granule has `block_type == 2`.
#[test]
fn cfg_c23_scfsi_stereo_and_mask() {
    let h = harness();
    let mut rng = Rng::new(0x0C23);
    for scfsi in 0..2048u32 {
        // Alternate which granules take the masking branch.
        let pattern = scfsi & 0xF;
        let gs: Vec<GranuleSpec> = (0..4)
            .map(|i| {
                if pattern >> i & 1 == 1 {
                    gran_ws(&mut rng, 2, 0)
                } else {
                    gran_long(&mut rng)
                }
            })
            .collect();
        let sc = Scenario::new(MPEG1_STEREO, gs).scfsi(scfsi);
        let case = sc.case(&mut rng);
        h.assert_same(&format!("C23 scfsi={scfsi:#x}"), &case);
    }
}

/// C24: MPEG2 never reads `scfsi`, so `gr->scfsi` is `0` for every granule no
/// matter what the bitstream holds.
#[test]
fn cfg_c24_mpeg2_scfsi_always_zero() {
    let h = harness();
    let mut rng = Rng::new(0x0C24);
    for it in 0..ITERS {
        for hs in [MPEG2_MONO, MPEG2_STEREO] {
            let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                .map(|_| GranuleSpec::random(&mut rng))
                .collect();
            let sc = Scenario::new(hs, gs).scfsi(rng.bits(11));
            let case = sc.case(&mut rng);
            let o = h.assert_same(&format!("C24 it={it}"), &case);
            for g in o.gr.iter().take(hs.gr_count() as usize) {
                assert_eq!(g.scfsi, 0, "C24: MPEG2 must leave gr->scfsi at 0");
            }
        }
    }
}

/// C25 + C27: the `main_data_begin` reservoir check at C line 159. `limit`
/// exactly on the boundary succeeds; one bit less returns `-1`.
#[test]
fn cfg_c25_c27_main_data_begin_reservoir_boundary() {
    let h = harness();
    let mut rng = Rng::new(0x0C25);
    for hs in [MPEG2_MONO, MPEG2_STEREO, MPEG1_MONO, MPEG1_STEREO] {
        let max_mdb = if hs.mpeg1 { 511 } else { 255 };
        for mdb in (0..=max_mdb).step_by(7) {
            // Large part_23_length keeps the boundary above `end_pos`, so the
            // parse itself never overruns and only line 159 decides.
            let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                .map(|_| {
                    let mut g = gran_long(&mut rng);
                    g.part_23_length = 4000;
                    g
                })
                .collect();
            let sc = Scenario::new(hs, gs)
                .main_data_begin(mdb)
                .start_bit(rng.below(8) as usize);
            let boundary = sc.boundary_limit();
            assert!(boundary >= sc.end_pos(), "test setup: boundary too tight");

            let ok = sc.case_with_limit(&mut rng, boundary as i32);
            let o = h.assert_same(&format!("C25 mdb={mdb} at-boundary"), &ok);
            assert_eq!(o.ret, mdb as i32, "C25 mdb={mdb}: boundary must succeed");

            let bad = sc.case_with_limit(&mut rng, (boundary - 1) as i32);
            let o = h.assert_same(&format!("C27 mdb={mdb} below-boundary"), &bad);
            assert_eq!(o.ret, -1, "C27 mdb={mdb}: one bit short must return -1");
        }
    }
}

/// C26: `bs->limit` swept across every bit position inside the side info, so the
/// `get_bits` overrun happens at every distinct field in turn.
#[test]
fn cfg_c26_limit_swept_through_every_field() {
    let h = harness();
    let mut rng = Rng::new(0x0C26);
    for hs in [MPEG2_MONO, MPEG1_STEREO] {
        for ws in [false, true] {
            let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                .map(|_| {
                    let mut g = if ws {
                        gran_ws(&mut rng, 2, 1)
                    } else {
                        gran_long(&mut rng)
                    };
                    // part_23_length 0 puts the reservoir boundary exactly at the
                    // end of the side info, so the sweep straddles it. The
                    // part_23_sum interaction is C25/C27's job.
                    g.part_23_length = 0;
                    g
                })
                .collect();
            let sc = Scenario::new(hs, gs);
            let total = sc.bits();
            assert_eq!(sc.boundary_limit(), sc.end_pos());
            let mut saw_err = false;
            let mut saw_ok = false;
            for limit in 0..=(total as i64 + 8) {
                let mut case = sc.case_with_limit(&mut rng, limit as i32);
                case.prefill = PREFILLS[(limit as usize) % PREFILLS.len()];
                let o = h.assert_same(&format!("C26 ws={ws} limit={limit}"), &case);
                if o.ret < 0 {
                    saw_err = true;
                } else {
                    saw_ok = true;
                }
            }
            assert!(saw_err, "C26 ws={ws}: sweep never truncated");
            assert!(saw_ok, "C26 ws={ws}: sweep never succeeded");
        }
    }
}

/// C28: fully random headers, bitstreams, `pos` and `limit`.
#[test]
fn cfg_c28_full_random() {
    let h = harness();
    let mut rng = Rng::new(0x0C28);
    let mut cov = Coverage::new();
    let mut rets = (0usize, 0usize);
    for it in 0..20_000 {
        let hdr = [
            rng.bits(8) as u8,
            rng.bits(8) as u8,
            rng.bits(8) as u8,
            rng.bits(8) as u8,
        ];
        let hs = HdrSpec::from_bytes(&hdr);
        let gc = hs.gr_count() as usize;
        let gs: Vec<GranuleSpec> = (0..gc)
            .map(|_| {
                let mut g = GranuleSpec::random(&mut rng);
                // Occasionally reach into the rejected ranges too.
                if rng.below(8) == 0 {
                    g.big_values = 289 + rng.below(223);
                }
                if rng.below(8) == 0 {
                    g.block_type = 0;
                }
                g
            })
            .collect();
        let start_bit = rng.below(64) as usize;
        let sc = Scenario::new(hs, gs)
            .start_bit(start_bit)
            .main_data_begin(rng.bits(9))
            .scfsi(rng.bits(11));
        // Mix ample budgets with truncating ones.
        let limit = match rng.below(4) {
            0 => rng.below(sc.bits() as u32 + 16) as i64,
            1 => sc.boundary_limit() - 1,
            2 => sc.boundary_limit(),
            _ => sc.boundary_limit() + rng.below(4096) as i64,
        };
        let mut case = sc.case_with_limit(&mut rng, limit as i32);
        case.hdr = hdr;
        case.prefill = PREFILLS[it % PREFILLS.len()];
        let o = h.assert_same(&format!("C28 it={it}"), &case);
        if o.ret < 0 {
            rets.0 += 1;
        } else {
            rets.1 += 1;
        }
        cov.observe(&o, gc);
    }
    assert!(rets.0 > 100 && rets.1 > 100, "C28: unbalanced outcomes {rets:?}");
    cov.require(&[
        "bt=0 mixed=0 table=long",
        "bt=1 mixed=0 table=long",
        "bt=1 mixed=1 table=long",
        "bt=2 mixed=0 table=short",
        "bt=2 mixed=1 table=mixed",
        "bt=3 mixed=0 table=long",
        "bt=3 mixed=1 table=long",
    ]);
}

/// Sweeps `hdr[1]` over all 256 values crossed with every `hdr[2]` sample-rate
/// field and every `hdr[3]` channel-mode field. Shared by C29 and B9.
fn exhaustive_header_sweep(label: &str, seed: u64) {
    let h = harness();
    let mut rng = Rng::new(seed);
    let mut seen_sr = std::collections::BTreeSet::new();
    let mut seen_gc = std::collections::BTreeSet::new();
    for hdr1 in 0..256u32 {
        for sr2 in 0..4u32 {
            for h3top in 0..4u32 {
                let hdr = [
                    rng.bits(8) as u8,
                    hdr1 as u8,
                    ((sr2 << 2) | (rng.bits(8) & !0x0C)) as u8,
                    ((h3top << 6) | (rng.bits(8) & !0xC0)) as u8,
                ];
                let hs = HdrSpec::from_bytes(&hdr);
                seen_sr.insert(hs.sr_idx());
                seen_gc.insert(hs.gr_count());
                let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                    .map(|_| GranuleSpec::random(&mut rng))
                    .collect();
                let sc = Scenario::new(hs, gs).start_bit(rng.below(8) as usize);
                let mut case =
                    sc.case_with_limit(&mut rng, (sc.boundary_limit() + 8) as i32);
                case.hdr = hdr;
                h.assert_same(&format!("{label} hdr={hdr:02x?}"), &case);
            }
        }
    }
    assert_eq!(
        seen_sr.iter().copied().collect::<Vec<i32>>(),
        (0..=8).collect::<Vec<i32>>(),
        "{label}: every sr_idx 0..=8 must be reached"
    );
    assert_eq!(
        seen_gc.iter().copied().collect::<Vec<u32>>(),
        vec![1, 2, 4],
        "{label}: every gr_count must be reached"
    );
}

/// C29: exhaustive header-byte sweep — the header bytes are unvalidated `uint8_t`
/// so every value is a legal input, including reserved MPEG versions and the
/// invalid sample-rate index.
#[test]
fn cfg_c29_exhaustive_header_sweep() {
    exhaustive_header_sweep("C29", 0x0C29);
}

/// C30: on the window-switching path the C code never writes
/// `region_count[2]`, so the caller's byte must survive unchanged.
#[test]
fn cfg_c30_region_count2_retention() {
    let h = harness();
    let mut rng = Rng::new(0x0C30);
    for &prefill in &[0x00u8, 0x01, 0x7F, 0xAA, 0xFE, 0xFF] {
        for bt in 1..=3u32 {
            for mixed in 0..2u32 {
                let gs: Vec<GranuleSpec> = (0..MPEG1_STEREO.gr_count())
                    .map(|_| gran_ws(&mut rng, bt, mixed))
                    .collect();
                let sc = Scenario::new(MPEG1_STEREO, gs);
                let mut case = sc.case(&mut rng);
                case.prefill = prefill;
                let o = h.assert_same(
                    &format!("C30 prefill={prefill:#04x} bt={bt} mixed={mixed}"),
                    &case,
                );
                for g in o.gr.iter().take(4) {
                    assert_eq!(
                        g.region_count[2], prefill,
                        "C30: region_count[2] must retain the caller's byte"
                    );
                    let want0 = if bt == 2 && mixed == 0 { 8 } else { 7 };
                    assert_eq!(
                        g.region_count[0], want0,
                        "C30: region_count[0] is 8 only for block_type==2 && \
                         !mixed_block_flag"
                    );
                    assert_eq!(g.region_count[1], 255);
                }
            }
        }
    }
    // Contrast: the non-window-switching path always writes 255.
    let gs: Vec<GranuleSpec> = (0..MPEG1_STEREO.gr_count())
        .map(|_| gran_long(&mut rng))
        .collect();
    let sc = Scenario::new(MPEG1_STEREO, gs);
    let mut case = sc.case(&mut rng);
    case.prefill = 0xAA;
    let o = h.assert_same("C30 non-ws", &case);
    for g in o.gr.iter().take(4) {
        assert_eq!(g.region_count[2], 255);
    }
}

/// C31 / B10: `sr_idx -= (sr_idx != 0)` collapses raw sums 0 and 1 onto the same
/// table row, reached from two different header encodings.
#[test]
fn cfg_c31_sr_idx_decrement_quirk() {
    let h = harness();
    let mut rng = Rng::new(0x0C31);
    let a = HdrSpec { mpeg1: false, hdr1_bit4: false, sr2: 0, mono: true }; // raw 0
    let b = HdrSpec { mpeg1: false, hdr1_bit4: false, sr2: 1, mono: true }; // raw 1
    assert_eq!(a.sr_idx(), 0);
    assert_eq!(b.sr_idx(), 0);
    let mut offsets = Vec::new();
    for (name, hs) in [("raw0", a), ("raw1", b)] {
        let gs = vec![gran_long(&mut rng)];
        let sc = Scenario::new(hs, gs);
        let case = sc.case(&mut rng);
        let o = h.assert_same(&format!("C31 {name}"), &case);
        match &o.gr[0].sfbtab {
            Sfbtab::Assigned { offset, .. } => offsets.push(*offset),
            other => panic!("C31 {name}: {other:?}"),
        }
    }
    assert_eq!(offsets[0], 0, "raw sum 0 -> g_scf_long[0]");
    assert_eq!(offsets[1], 0, "raw sum 1 -> g_scf_long[0] as well");
    // And raw sum 2 really is a different row, proving the sweep is sensitive.
    let c = HdrSpec { mpeg1: false, hdr1_bit4: false, sr2: 2, mono: true };
    assert_eq!(c.sr_idx(), 1);
    let sc = Scenario::new(c, vec![gran_long(&mut rng)]);
    let case = sc.case(&mut rng);
    let o = h.assert_same("C31 raw2", &case);
    assert!(matches!(o.gr[0].sfbtab, Sfbtab::Assigned { offset: 23, .. }));
}

/// C32: the window-switching path reads only 10 `tables` bits and then shifts
/// left by 5, so `table_select[2]` is always 0; the other path reads 15 bits and
/// all three `table_select` entries are significant.
#[test]
fn cfg_c32_tables_width_difference() {
    let h = harness();
    let mut rng = Rng::new(0x0C32);
    // Window switching: 10 bits << 5.
    for t in 0..1024u32 {
        let gs: Vec<GranuleSpec> = (0..MPEG2_MONO.gr_count())
            .map(|_| {
                let mut g = gran_ws(&mut rng, 2, 0);
                g.tables = t;
                g
            })
            .collect();
        let sc = Scenario::new(MPEG2_MONO, gs);
        let case = sc.case(&mut rng);
        let o = h.assert_same(&format!("C32 ws tables={t}"), &case);
        let shifted = t << 5;
        assert_eq!(o.gr[0].table_select[0], (shifted >> 10) as u8);
        assert_eq!(o.gr[0].table_select[1], ((shifted >> 5) & 31) as u8);
        assert_eq!(o.gr[0].table_select[2], 0, "C32: always 0 on the ws path");
    }
    // Non-window-switching: a full 15 bits.
    let mut saw_nonzero_t2 = false;
    for t in (0..32768u32).step_by(31) {
        let gs: Vec<GranuleSpec> = (0..MPEG2_MONO.gr_count())
            .map(|_| {
                let mut g = gran_long(&mut rng);
                g.tables = t;
                g
            })
            .collect();
        let sc = Scenario::new(MPEG2_MONO, gs);
        let case = sc.case(&mut rng);
        let o = h.assert_same(&format!("C32 long tables={t}"), &case);
        assert_eq!(o.gr[0].table_select[0], (t >> 10) as u8);
        assert_eq!(o.gr[0].table_select[1], ((t >> 5) & 31) as u8);
        assert_eq!(o.gr[0].table_select[2], (t & 31) as u8);
        saw_nonzero_t2 |= o.gr[0].table_select[2] != 0;
    }
    assert!(saw_nonzero_t2, "C32: table_select[2] should vary on the long path");
}

// ===========================================================================
// Phase C — ERRORS.md rows
// ===========================================================================

/// E1: `get_bits` returns 0 **without reading the buffer** when the read would
/// cross `bs->limit`, and leaves `bs->pos` advanced anyway (the side effect is
/// not rolled back).
#[test]
fn err_e1_get_bits_overrun_truncates() {
    let h = harness();
    let mut rng = Rng::new(0x0E01);
    // Every field is all-ones in the bitstream, so a zero result can only come
    // from the truncating early return, never from the data.
    let all_ones = GranuleSpec {
        part_23_length: 0xFFF,
        big_values: 288,
        global_gain: 0xFF,
        scalefac_compress: 0x1FF,
        ws: false,
        block_type: 3,
        mixed: 1,
        tables: 0x7FFF,
        subblock_gain: [7; 3],
        region_count0: 0xF,
        region_count1: 7,
        preflag: 1,
        scalefac_scale: 1,
        count1_table: 1,
    };
    let sc = Scenario::new(MPEG2_MONO, vec![all_ones]);
    // Zero budget: the very first get_bits overruns.
    let case = sc.case_with_limit(&mut rng, 0);
    let o = h.assert_same("E1 limit=0", &case);
    assert_eq!(o.gr[0].part_23_length, 0, "E1: truncated field must read 0");
    assert_eq!(o.gr[0].global_gain, 0);
    assert!(
        o.pos > o.limit,
        "E1: pos ({}) must stay advanced past limit ({})",
        o.pos,
        o.limit
    );
    assert_eq!(o.pos, sc.bits() as i32, "E1: pos advanced by every get_bits n");

    // Truncate at each field boundary in turn and confirm parity throughout.
    for limit in 0..sc.bits() as i32 {
        let case = sc.case_with_limit(&mut rng, limit);
        let o = h.assert_same(&format!("E1 limit={limit}"), &case);
        assert!(o.pos > o.limit, "E1 limit={limit}: pos must exceed limit");
    }
}

/// E2: once `pos > limit`, every later `get_bits` also returns 0, so the whole
/// side info decodes to the all-zero shape.
#[test]
fn err_e2_all_zero_after_overrun() {
    let h = harness();
    let mut rng = Rng::new(0x0E02);
    let sc = Scenario::new(MPEG2_MONO, vec![gran_ws(&mut rng, 2, 1)]);
    let case = sc.case_with_limit(&mut rng, 0);
    let o = h.assert_same("E2", &case);
    let g = &o.gr[0];
    assert_eq!(g.part_23_length, 0);
    assert_eq!(g.big_values, 0);
    assert_eq!(g.global_gain, 0);
    assert_eq!(g.scalefac_compress, 0);
    // ws bit read as 0 => the non-window-switching branch is taken.
    assert_eq!(g.block_type, 0);
    assert_eq!(g.mixed_block_flag, 0);
    assert_eq!(g.region_count, [0, 0, 255]);
    assert_eq!(g.table_select, [0, 0, 0]);
    assert_eq!(g.n_long_sfb, 22);
    assert_eq!(g.n_short_sfb, 0);
    assert_eq!(g.preflag, 0, "scalefac_compress 0 is < 500");
    assert_eq!(g.scalefac_scale, 0);
    assert_eq!(g.count1_table, 0);
    assert_eq!(g.scfsi, 0);
    assert!(matches!(g.sfbtab, Sfbtab::Assigned { offset: 0, .. }));
    assert_eq!(o.ret, -1, "E2: line 159 rejects because pos > limit");
}

/// E3: `big_values > 288` rejects with `-1`, over the whole invalid range.
/// Fields written before the check keep their new values; later ones do not.
#[test]
fn err_e3_big_values_gt_288() {
    let h = harness();
    let mut rng = Rng::new(0x0E03);
    for bv in 289..=511u32 {
        for hs in [MPEG2_MONO, MPEG2_STEREO, MPEG1_MONO, MPEG1_STEREO] {
            let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                .map(|_| {
                    let mut g = gran_long(&mut rng);
                    g.big_values = bv;
                    g
                })
                .collect();
            let sc = Scenario::new(hs, gs);
            let mut case = sc.case(&mut rng);
            case.prefill = 0xAA;
            let o = h.assert_same(&format!("E3 bv={bv}"), &case);
            assert_eq!(o.ret, -1, "E3 bv={bv}: must be rejected");
            // C line 104 writes big_values, line 105 rejects: partial writes
            // stand, but sfbtab (line 111) was never reached.
            assert_eq!(o.gr[0].big_values, bv as u16);
            assert!(
                matches!(o.gr[0].sfbtab, Sfbtab::Untouched(_)),
                "E3: sfbtab is assigned after the check, so it must be untouched"
            );
            assert_eq!(o.gr[0].global_gain, 0xAA, "E3: written after the check");
        }
    }
}

/// E4: `big_values == 288` is the largest accepted value.
#[test]
fn err_e4_big_values_288_ok() {
    let h = harness();
    let mut rng = Rng::new(0x0E04);
    for hs in [MPEG2_MONO, MPEG1_STEREO] {
        let gs: Vec<GranuleSpec> = (0..hs.gr_count())
            .map(|_| {
                let mut g = gran_long(&mut rng);
                g.big_values = 288;
                g
            })
            .collect();
        let sc = Scenario::new(hs, gs);
        let case = sc.case(&mut rng);
        let o = h.assert_same("E4", &case);
        assert_eq!(o.ret, 0, "E4: 288 must be accepted");
        assert_eq!(o.gr[0].big_values, 288);
    }
}

/// E5: `big_values == 289` — one step past the range — is rejected.
#[test]
fn err_e5_big_values_289_err() {
    let h = harness();
    let mut rng = Rng::new(0x0E05);
    for hs in [MPEG2_MONO, MPEG1_STEREO] {
        let gs: Vec<GranuleSpec> = (0..hs.gr_count())
            .map(|_| {
                let mut g = gran_long(&mut rng);
                g.big_values = 289;
                g
            })
            .collect();
        let sc = Scenario::new(hs, gs);
        let case = sc.case(&mut rng);
        let o = h.assert_same("E5", &case);
        assert_eq!(o.ret, -1, "E5: 289 must be rejected");
    }
}

/// E6: window switching with a `block_type` field of 0 rejects with `-1`.
#[test]
fn err_e6_block_type_zero() {
    let h = harness();
    let mut rng = Rng::new(0x0E06);
    for hs in [MPEG2_MONO, MPEG2_STEREO, MPEG1_MONO, MPEG1_STEREO] {
        for it in 0..64 {
            let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                .map(|_| {
                    let m = rng.bits(1);
                    gran_ws(&mut rng, 0, m)
                })
                .collect();
            let sc = Scenario::new(hs, gs);
            let mut case = sc.case(&mut rng);
            case.prefill = PREFILLS[it % PREFILLS.len()];
            let o = h.assert_same(&format!("E6 it={it}"), &case);
            assert_eq!(o.ret, -1, "E6: block_type 0 must be rejected");
            assert_eq!(o.gr[0].block_type, 0);
            // sfbtab / n_long_sfb are set at C lines 111-113, before the check.
            assert!(matches!(o.gr[0].sfbtab, Sfbtab::Assigned { .. }));
            assert_eq!(o.gr[0].n_long_sfb, 22);
            assert_eq!(o.gr[0].n_short_sfb, 0);
            // mixed_block_flag / region_count come after it.
            assert_eq!(o.gr[0].mixed_block_flag, case.prefill);
        }
    }
}

/// E7: the `main_data_begin` reservoir check rejects when the granules claim
/// more main data than the budget allows.
#[test]
fn err_e7_main_data_overrun() {
    let h = harness();
    let mut rng = Rng::new(0x0E07);
    for hs in [MPEG2_MONO, MPEG2_STEREO, MPEG1_MONO, MPEG1_STEREO] {
        for excess in 1..=32i64 {
            let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                .map(|_| {
                    let mut g = gran_long(&mut rng);
                    g.part_23_length = 4000;
                    g
                })
                .collect();
            let sc = Scenario::new(hs, gs).main_data_begin(64);
            let limit = sc.boundary_limit() - excess;
            let case = sc.case_with_limit(&mut rng, limit as i32);
            let o = h.assert_same(&format!("E7 excess={excess}"), &case);
            assert_eq!(o.ret, -1, "E7 excess={excess}: must be rejected");
            // The check is last, so every granule is fully populated anyway.
            for g in o.gr.iter().take(hs.gr_count() as usize) {
                assert!(matches!(g.sfbtab, Sfbtab::Assigned { .. }));
            }
        }
    }
}

/// E8: the reservoir check is a strict `>`, so hitting the boundary exactly is
/// accepted.
#[test]
fn err_e8_main_data_exact_boundary() {
    let h = harness();
    let mut rng = Rng::new(0x0E08);
    for hs in [MPEG2_MONO, MPEG2_STEREO, MPEG1_MONO, MPEG1_STEREO] {
        for mdb in [0u32, 1, 17, 64, 200, 255] {
            let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                .map(|_| {
                    let mut g = gran_long(&mut rng);
                    g.part_23_length = 4000;
                    g
                })
                .collect();
            let sc = Scenario::new(hs, gs).main_data_begin(mdb);
            let case = sc.case_with_limit(&mut rng, sc.boundary_limit() as i32);
            let o = h.assert_same(&format!("E8 mdb={mdb}"), &case);
            assert_eq!(o.ret, mdb as i32, "E8 mdb={mdb}: boundary is accepted");
        }
    }
}

/// E9: `big_values > 288` on a *late* granule — earlier granules stay fully
/// written, later ones are never touched.
#[test]
fn err_e9_big_values_late_granule() {
    let h = harness();
    let mut rng = Rng::new(0x0E09);
    for bad in 0..4usize {
        let gs: Vec<GranuleSpec> = (0..4)
            .map(|i| {
                let mut g = gran_long(&mut rng);
                if i == bad {
                    g.big_values = 300;
                }
                g
            })
            .collect();
        let sc = Scenario::new(MPEG1_STEREO, gs);
        let mut case = sc.case(&mut rng);
        case.prefill = 0xAA;
        let o = h.assert_same(&format!("E9 bad_granule={bad}"), &case);
        assert_eq!(o.ret, -1);
        for i in 0..bad {
            assert!(
                matches!(o.gr[i].sfbtab, Sfbtab::Assigned { .. }),
                "E9: granule {i} before the failure must be complete"
            );
        }
        assert!(matches!(o.gr[bad].sfbtab, Sfbtab::Untouched(_)));
        assert_eq!(o.gr[bad].big_values, 300);
        for i in (bad + 1)..NGR {
            assert!(
                matches!(o.gr[i].sfbtab, Sfbtab::Untouched(_)),
                "E9: granule {i} after the failure must be untouched"
            );
            assert_eq!(o.gr[i].big_values, 0xAAAA);
        }
    }
}

/// E10: `block_type == 0` on a late granule.
#[test]
fn err_e10_block_type_zero_late_granule() {
    let h = harness();
    let mut rng = Rng::new(0x0E10);
    for bad in 0..4usize {
        let gs: Vec<GranuleSpec> = (0..4)
            .map(|i| {
                if i == bad {
                    gran_ws(&mut rng, 0, 0)
                } else {
                    gran_long(&mut rng)
                }
            })
            .collect();
        let sc = Scenario::new(MPEG1_STEREO, gs);
        let mut case = sc.case(&mut rng);
        case.prefill = 0x5A;
        let o = h.assert_same(&format!("E10 bad_granule={bad}"), &case);
        assert_eq!(o.ret, -1);
        assert_eq!(o.gr[bad].block_type, 0);
        assert!(matches!(o.gr[bad].sfbtab, Sfbtab::Assigned { .. }));
        for i in (bad + 1)..NGR {
            assert!(matches!(o.gr[i].sfbtab, Sfbtab::Untouched(_)));
        }
    }
}

// ===========================================================================
// Phase C — generic FFI boundary conditions (ERRORS.md B1..B11)
// ===========================================================================

/// Every header shape, driven at some pathological `(pos, limit)` pair.
fn boundary_all_shapes(label: &str, seed: u64, pos: i32, limit: i32) {
    let h = harness();
    let mut rng = Rng::new(seed);
    for hs in [MPEG2_MONO, MPEG2_STEREO, MPEG1_MONO, MPEG1_STEREO] {
        for ws in [false, true] {
            for it in 0..8 {
                let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                    .map(|_| {
                        if ws {
                            gran_ws(&mut rng, 2, 1)
                        } else {
                            gran_long(&mut rng)
                        }
                    })
                    .collect();
                let sc = Scenario::new(hs, gs);
                let mut case = sc.case_with_limit(&mut rng, limit);
                case.pos = pos;
                case.prefill = PREFILLS[it % PREFILLS.len()];
                h.assert_same(&format!("{label} ws={ws} it={it}"), &case);
            }
        }
    }
}

/// B1: `bs->limit == 0` — zero-length budget.
#[test]
fn bnd_b1_zero_limit() {
    boundary_all_shapes("B1", 0x0B01, 0, 0);
    // Pin the documented outcome for one shape.
    let h = harness();
    let mut rng = Rng::new(0x0B01B);
    let sc = Scenario::new(MPEG2_MONO, vec![gran_long(&mut rng)]);
    let case = sc.case_with_limit(&mut rng, 0);
    let o = h.assert_same("B1 pin", &case);
    assert_eq!(o.ret, -1);
    assert_eq!(o.gr[0].part_23_length, 0);
}

/// B2: negative `bs->limit`.
#[test]
fn bnd_b2_negative_limit() {
    for limit in [-1, -7, -8, -1000, i32::MIN + 1] {
        boundary_all_shapes(&format!("B2 limit={limit}"), 0x0B02, 0, limit);
    }
}

/// B3: `bs->pos > bs->limit` on entry.
#[test]
fn bnd_b3_pos_past_limit() {
    for (pos, limit) in [(1, 0), (100, 50), (64, 63), (4096, 8)] {
        boundary_all_shapes(&format!("B3 {pos}/{limit}"), 0x0B03, pos, limit);
    }
}

/// B4: `bs->pos == bs->limit` on entry — the first read is exactly one bit too
/// many.
#[test]
fn bnd_b4_pos_eq_limit() {
    for p in [0, 1, 7, 8, 63, 64, 512] {
        boundary_all_shapes(&format!("B4 pos=limit={p}"), 0x0B04, p, p);
    }
}

/// B5: negative `bs->pos`. `bs->buf + (pos >> 3)` addresses memory *before* the
/// buffer and the C code reads it; both libraries get the same pointer, so the
/// same bytes are read and the results must still agree.
#[test]
fn bnd_b5_negative_pos() {
    // Stay inside the harness guard region (GUARD bytes before `buf`).
    let max_back = (GUARD as i32 - 64) * 8;
    let mut positions: Vec<i32> = (1..=64).map(|i| -i).collect();
    positions.extend([-127, -128, -129, -255, -256, -1000, -max_back]);
    for pos in positions {
        boundary_all_shapes(&format!("B5 pos={pos}"), 0x0B05, pos, 1 << 14);
    }
}

/// B6: `bs->pos` near `INT_MAX`, so `bs->pos += n` overflows a signed int.
/// `bs->limit == INT_MIN` makes every `get_bits` take the early return, so the
/// wrap-around arithmetic is exercised with no dereference at all.
#[test]
fn bnd_b6_pos_int_max_overflow() {
    // A dereference would only happen if `pos` landed exactly on INT_MIN. The
    // first get_bits always consumes >= 9 bits and a whole parse consumes < 400,
    // so a wrap distance in 1..=8 or > 400 can never be hit.
    let candidates: Vec<i32> = (0..8)
        .map(|i| i32::MAX - i)
        .chain([i32::MAX - 500, i32::MAX - 1000, i32::MAX - 100_000])
        .collect();
    for pos in candidates {
        let dist = (i32::MIN as i64 - pos as i64).rem_euclid(1i64 << 32);
        assert!(
            (1..=8).contains(&dist) || dist > 400,
            "unsafe probe: pos={pos} would land on INT_MIN mid-parse (dist={dist})"
        );
        boundary_all_shapes(&format!("B6 pos={pos}"), 0x0B06, pos, i32::MIN);
    }
    // Pin the wrap: pos must come back negative and the call must reject.
    let h = harness();
    let mut rng = Rng::new(0x0B06B);
    let sc = Scenario::new(MPEG2_MONO, vec![gran_long(&mut rng)]);
    let mut case = sc.case_with_limit(&mut rng, i32::MIN);
    case.pos = i32::MAX;
    let o = h.assert_same("B6 pin", &case);
    assert_eq!(o.pos, i32::MAX.wrapping_add(sc.bits() as i32));
    assert!(o.pos < 0, "B6: pos must have wrapped negative");
    assert_eq!(o.ret, -1);
}

/// B7: `bs->limit == INT_MAX` (oversized budget). Nothing ever overruns, and
/// `limit + main_data_begin * 8` at C line 159 overflows for any non-zero
/// `main_data_begin`.
#[test]
fn bnd_b7_limit_int_max() {
    let h = harness();
    let mut rng = Rng::new(0x0B07);
    for hs in [MPEG2_MONO, MPEG2_STEREO, MPEG1_MONO, MPEG1_STEREO] {
        for mdb in [0u32, 1, 2, 255, 256, 511] {
            let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                .map(|_| gran_long(&mut rng))
                .collect();
            let sc = Scenario::new(hs, gs).main_data_begin(mdb);
            let case = sc.case_with_limit(&mut rng, i32::MAX);
            let o = h.assert_same(&format!("B7 mdb={mdb}"), &case);
            let effective_mdb = if hs.mpeg1 { mdb } else { mdb & 0xFF };
            if effective_mdb == 0 {
                assert_eq!(o.ret, 0, "B7: no overflow, budget is ample");
            } else {
                // INT_MAX + mdb*8 wraps negative, so the check fires.
                assert_eq!(o.ret, -1, "B7 mdb={mdb}: the wrapped comparison rejects");
            }
        }
    }
    for limit in [i32::MAX, i32::MAX - 1, i32::MAX - 7] {
        boundary_all_shapes(&format!("B7 limit={limit}"), 0x0B07B, 0, limit);
    }
}

/// B8: `sr_idx == 8` — one step past the last valid table row (`0..=7`). This is
/// the out-of-range "enum-like" input this API actually admits: `sr_idx` is
/// computed from unvalidated header bits and indexes a fixed 8-row table.
#[test]
fn bnd_b8_sr_idx_out_of_range() {
    let h = harness();
    let mut rng = Rng::new(0x0B08);
    // Only reachable combination: both hdr[1] version bits set and sr2 == 3.
    let hs8 = HdrSpec { mpeg1: true, hdr1_bit4: true, sr2: 3, mono: true };
    assert_eq!(hs8.sr_idx(), 8, "setup: this header must produce sr_idx 8");
    let hs7 = HdrSpec { mpeg1: true, hdr1_bit4: true, sr2: 2, mono: true };
    assert_eq!(hs7.sr_idx(), 7, "setup: the last in-range row");

    for (ws, bt, mixed, table) in [
        (false, 0u32, 0u32, "long"),
        (true, 1, 0, "long"),
        (true, 2, 0, "short"),
        (true, 2, 1, "mixed"),
        (true, 3, 1, "long"),
    ] {
        for (hs, sr_idx) in [(hs7, 7i32), (hs8, 8i32)] {
            for it in 0..32 {
                let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                    .map(|_| {
                        if ws {
                            gran_ws(&mut rng, bt, mixed)
                        } else {
                            gran_long(&mut rng)
                        }
                    })
                    .collect();
                let sc = Scenario::new(hs, gs);
                let mut case = sc.case(&mut rng);
                case.prefill = PREFILLS[it % PREFILLS.len()];
                let o = h.assert_same(
                    &format!("B8 table={table} sr_idx={sr_idx} it={it}"),
                    &case,
                );
                let want = expected_sfbtab_offset(ws, bt, mixed, sr_idx);
                match &o.gr[0].sfbtab {
                    Sfbtab::Assigned { offset, row } => {
                        assert_eq!(*offset, want, "B8 table={table} sr_idx={sr_idx}");
                        // g_scf_mixed[8] leaves .rodata in C; everything else
                        // still aliases inside the tables and is byte-compared.
                        if table == "mixed" && sr_idx == 8 {
                            assert!(row.is_none(), "B8: mixed[8] is past .rodata");
                        } else {
                            assert!(row.is_some(), "B8: {table}[{sr_idx}] must be compared");
                        }
                    }
                    other => panic!("B8: {other:?}"),
                }
            }
        }
    }
}

/// B9: exhaustive header-byte sweep (a second seed over `CONFIGS` C29) — the
/// header bytes are raw `uint8_t` with no validation, so every value is legal.
#[test]
fn bnd_b9_exhaustive_header_bytes() {
    exhaustive_header_sweep("B9", 0x0B09);
}

/// B9b: all 256 values of `hdr[2]` and `hdr[3]` for each distinct `hdr[1]`
/// version-bit pattern, so no unread bit of any header byte can change behaviour.
#[test]
fn bnd_b9b_full_hdr2_hdr3_sweep() {
    let h = harness();
    let mut rng = Rng::new(0x0B0B);
    for hdr1 in [0x00u8, 0x08, 0x10, 0x18, 0xFF, 0xE7] {
        for hdr2 in 0..256u32 {
            for hdr3 in 0..256u32 {
                let hdr = [rng.bits(8) as u8, hdr1, hdr2 as u8, hdr3 as u8];
                let hs = HdrSpec::from_bytes(&hdr);
                let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                    .map(|_| GranuleSpec::random(&mut rng))
                    .collect();
                let sc = Scenario::new(hs, gs);
                let mut case = sc.case_with_limit(&mut rng, (sc.boundary_limit() + 4) as i32);
                case.hdr = hdr;
                h.assert_same(&format!("B9b hdr={hdr:02x?}"), &case);
            }
        }
    }
}

/// B10: two different header encodings that both collapse onto the same table
/// row, via `sr_idx -= (sr_idx != 0)` and via the `+3` version-bit term.
#[test]
fn bnd_b10_sr_idx_aliasing_headers() {
    let h = harness();
    let mut rng = Rng::new(0x0B0A);
    // Raw sum 3 is reachable two ways; both must select row 2.
    let pairs = [
        (
            HdrSpec { mpeg1: false, hdr1_bit4: false, sr2: 3, mono: true },
            HdrSpec { mpeg1: false, hdr1_bit4: true, sr2: 0, mono: true },
            2i32,
        ),
        // ...and via the mpeg1 bit, which also contributes 3.
        (
            HdrSpec { mpeg1: true, hdr1_bit4: false, sr2: 1, mono: true },
            HdrSpec { mpeg1: false, hdr1_bit4: true, sr2: 1, mono: true },
            3i32,
        ),
    ];
    for (i, (a, b, want)) in pairs.iter().enumerate() {
        assert_eq!(a.sr_idx(), *want, "pair {i} lhs");
        assert_eq!(b.sr_idx(), *want, "pair {i} rhs");
        for hs in [*a, *b] {
            let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                .map(|_| gran_long(&mut rng))
                .collect();
            let sc = Scenario::new(hs, gs);
            let case = sc.case(&mut rng);
            let o = h.assert_same(&format!("B10 pair={i}"), &case);
            match &o.gr[0].sfbtab {
                Sfbtab::Assigned { offset, .. } => {
                    assert_eq!(*offset, 23 * (*want as isize), "B10 pair={i}")
                }
                other => panic!("B10: {other:?}"),
            }
        }
    }
}

/// B11: null pointers. The C source dereferences `bs`, `gr` and `hdr`
/// unconditionally — there is no null check anywhere — so a null argument is a
/// hard fault in *both* libraries rather than a distinguishable error code.
/// Rather than crashing the harness, this asserts mechanically that the C really
/// has no null handling (so parity holds by construction) and that the Rust
/// translation did not "helpfully" add any.
#[test]
fn bnd_b11_null_pointers_documented() {
    let c_src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/src/lib.c"),
    )
    .expect("read c_src/src/lib.c");
    // Null-check idioms, written so they cannot match the genuine
    // `if (!gr->block_type)` value test at C line 116.
    for pat in [
        "NULL", "nullptr", "!bs)", "!gr)", "!hdr)", "!bs ", "!gr ", "!hdr ",
        "bs ==", "gr ==", "hdr ==",
    ] {
        assert!(
            !c_src.contains(pat),
            "C source unexpectedly mentions {pat:?} — the null-pointer contract \
             changed and B11 must be re-derived"
        );
    }
    // The C code has exactly three `return -1` rejections and one `return 0`.
    assert_eq!(c_src.matches("return -1;").count(), 3, "ERRORS.md E3/E6/E7");
    assert_eq!(c_src.matches("return 0;").count(), 1, "ERRORS.md E1");

    let rust_src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .expect("read src/lib.rs");
    for pat in ["is_null()", ".is_none()", "unwrap_or"] {
        assert!(
            !rust_src.contains(pat),
            "Rust translation added null handling ({pat:?}) the C does not have"
        );
    }
    assert!(
        !rust_src.contains("unimplemented!") && !rust_src.contains("todo!"),
        "the translation must not contain stubs"
    );
}

/// B12: `hdr` aliasing the output array. The C code re-loads `hdr[1]` and
/// `hdr[3]` from inside the granule loop (the reference build carries no `-O`
/// flag, so every access is a fresh load), interleaved with its writes through
/// `gr`. Pointing `hdr` into the `gr` array therefore makes the *timing* of
/// those loads observable, and pins down that the translation does not hoist
/// them.
#[test]
fn bnd_b12_hdr_aliasing_gr_array() {
    let h = harness();
    let mut rng = Rng::new(0x0B0C);
    let mut differed_from_unaliased = 0usize;
    // Only offsets that land on the *scalar* fields are usable. Bytes 0..8 of
    // each 32-byte L3_gr_info_t hold the `sfbtab` pointer, whose value differs
    // between two separately-loaded shared objects by construction; aliasing
    // `hdr` onto it would feed each library a different header and compare
    // nothing meaningful. Scalar fields occupy bytes 8..32, so k+4 <= 32 gives
    // k in 8..=28 for granule 0 and 40..=60 for granule 1.
    let offsets: Vec<usize> = (8..=28).chain(40..=60).collect();
    for k in offsets {
        for hdr1 in [0x00u8, 0x08, 0x10, 0x18] {
            for it in 0..8 {
                let hdr = [rng.bits(8) as u8, hdr1, rng.bits(8) as u8, rng.bits(8) as u8];
                let hs = HdrSpec::from_bytes(&hdr);
                let gs: Vec<GranuleSpec> = (0..hs.gr_count())
                    .map(|_| GranuleSpec::random(&mut rng))
                    .collect();
                let sc = Scenario::new(hs, gs);
                let data = sc.data();
                let limit = (sc.boundary_limit() + 64) as i32;
                let prefill = PREFILLS[it % PREFILLS.len()];
                let aliased = h.assert_same_aliased(
                    &format!("B12 k={k} hdr1={hdr1:#04x} it={it}"),
                    hdr,
                    k,
                    &data,
                    0,
                    limit,
                    prefill,
                );
                // Also confirm the aliasing genuinely perturbs the result for at
                // least some offsets, i.e. the test is not vacuous.
                let mut plain = Case::new(hdr, data.clone()).limit(limit);
                plain.prefill = prefill;
                let unaliased = h.assert_same(&format!("B12 baseline k={k}"), &plain);
                if aliased.ret != unaliased.ret || aliased.gr[0] != unaliased.gr[0] {
                    differed_from_unaliased += 1;
                }
            }
        }
    }
    assert!(
        differed_from_unaliased > 0,
        "B12: aliasing never changed the outcome, so the test proves nothing"
    );
}
