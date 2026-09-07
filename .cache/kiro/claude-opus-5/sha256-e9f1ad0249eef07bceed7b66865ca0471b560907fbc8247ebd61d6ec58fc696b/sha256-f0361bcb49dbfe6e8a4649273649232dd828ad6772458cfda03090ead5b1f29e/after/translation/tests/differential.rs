//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both libraries are loaded with `libloading` and called only through their
//! `read_side_info` dynamic export.

mod harness;
use harness::*;

// ---------------------------------------------------------------------------
// Row driver
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum PerGr {
    /// same window-switch / block-type / mixed choice for every granule
    Fixed { w: bool, bt: u32, mb: bool },
    /// independently random per granule (the composed pipeline)
    Random,
}

#[derive(Clone, Copy, PartialEq)]
enum LimitMode {
    /// limit covers the whole buffer
    Ample,
    /// limit is exactly the last bit of side info
    Exact,
    /// limit is one bit short of the last side-info bit
    ExactMinusOne,
    /// limit picked at random anywhere in/around the side info
    Random,
}

#[derive(Clone, Copy)]
struct Spec {
    mpeg1: bool,
    mono: bool,
    /// None -> random reachable value
    sr: Option<i32>,
    per_gr: PerGr,
    /// None -> random 0..8
    align: Option<usize>,
    /// leading slack bytes; controls how large `pos >> 3` gets
    lead: usize,
    limit: LimitMode,
    sfc: Option<u32>,
    mdb: Option<u32>,
    p23: Option<u32>,
    bv: Option<u32>,
}

impl Spec {
    fn new(mpeg1: bool, mono: bool) -> Spec {
        Spec {
            mpeg1,
            mono,
            sr: None,
            per_gr: PerGr::Random,
            align: None,
            lead: PAD,
            limit: LimitMode::Ample,
            sfc: None,
            mdb: None,
            p23: None,
            bv: None,
        }
    }
    fn fixed(mut self, w: bool, bt: u32, mb: bool) -> Spec {
        self.per_gr = PerGr::Fixed { w, bt, mb };
        self
    }
    fn sr(mut self, sr: i32) -> Spec {
        self.sr = Some(sr);
        self
    }
    fn align(mut self, a: usize) -> Spec {
        self.align = Some(a);
        self
    }
    fn lead(mut self, l: usize) -> Spec {
        self.lead = l;
        self
    }
    fn limit(mut self, l: LimitMode) -> Spec {
        self.limit = l;
        self
    }
    fn sfc(mut self, v: u32) -> Spec {
        self.sfc = Some(v);
        self
    }
    fn mdb(mut self, v: u32) -> Spec {
        self.mdb = Some(v);
        self
    }
    fn p23(mut self, v: u32) -> Spec {
        self.p23 = Some(v);
        self
    }
    fn bv(mut self, v: u32) -> Spec {
        self.bv = Some(v);
        self
    }
}

/// Run `n` randomised iterations of one `CONFIGS.md` row.
fn run_row(row: &str, spec: Spec, n: usize, seed: u64) {
    let p = Pair::load();
    let mut rng = Rng::new(seed);
    let mut used_sr = std::collections::BTreeSet::new();
    for it in 0..n {
        let sr = match spec.sr {
            Some(s) => s,
            None => {
                let opts = reachable_sr(spec.mpeg1);
                opts[rng.below(opts.len() as u32) as usize]
            }
        };
        let hdr = match make_hdr(spec.mpeg1, spec.mono, sr, &mut rng) {
            Some(h) => h,
            None => panic!("{row}: sr_idx {sr} unreachable with mpeg1={}", spec.mpeg1),
        };
        used_sr.insert(sr);
        let gr_count = hdr_gr_count(&hdr) as usize;

        let mut choices = [GrChoice::random(&mut rng); MAX_GR];
        for g in 0..gr_count {
            let mut c = GrChoice::random(&mut rng);
            if let PerGr::Fixed { w, bt, mb } = spec.per_gr {
                c.w = w;
                c.bt = bt;
                c.mb = mb;
            }
            if let Some(v) = spec.sfc {
                c.scalefac_compress = Some(v);
            }
            if let Some(v) = spec.p23 {
                c.part_23_length = v;
            }
            if let Some(v) = spec.bv {
                c.big_values = v;
            }
            choices[g] = c;
        }

        let mdb = spec.mdb.unwrap_or_else(|| {
            if spec.mpeg1 {
                rng.below(512)
            } else {
                rng.below(256)
            }
        });
        let scfsi_raw = rng.next_u32();
        let w = build_side_info(&hdr, mdb, scfsi_raw, &choices, &mut rng);
        let align = spec.align.unwrap_or_else(|| rng.below(8) as usize);
        let packed = pack_with_pad(&w, spec.lead, align, &mut rng);
        let total_bits = (packed.buf.len() * 8) as i32;
        let end = packed.pos + packed.nbits as i32;
        let limit = match spec.limit {
            LimitMode::Ample => total_bits,
            LimitMode::Exact => end,
            LimitMode::ExactMinusOne => end - 1,
            LimitMode::Random => {
                // anywhere from before the start to the end of the buffer
                packed.pos - 8 + rng.below((packed.nbits + 24) as u32) as i32
            }
        };
        let limit = limit.min(total_bits);

        diff(
            &p,
            &Case {
                label: format!("{row} iter={it} sr_idx={sr} align={align}"),
                buf: &packed.buf,
                pos: packed.pos,
                limit,
                hdr,
                compare_sfb_contents: sr <= 7,
            },
        );
    }
    eprintln!("{row}: {n} iterations OK (sr_idx values covered: {used_sr:?})");
}

// ---------------------------------------------------------------------------
// Group 1 — granule count x long-block path (W = 0)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c1_mpeg2_mono_long() {
    run_row("C1", Spec::new(false, true).fixed(false, 0, false), 256, 1);
}
#[test]
fn cfg_c2_mpeg2_stereo_long() {
    run_row("C2", Spec::new(false, false).fixed(false, 0, false), 256, 2);
}
#[test]
fn cfg_c3_mpeg1_mono_long() {
    run_row("C3", Spec::new(true, true).fixed(false, 0, false), 256, 3);
}
#[test]
fn cfg_c4_mpeg1_stereo_long() {
    run_row("C4", Spec::new(true, false).fixed(false, 0, false), 256, 4);
}

// ---------------------------------------------------------------------------
// Group 2 — window switched, block_type = 1
// ---------------------------------------------------------------------------

#[test]
fn cfg_c5_mpeg2_mono_bt1() {
    run_row("C5", Spec::new(false, true).fixed(true, 1, false), 256, 5);
}
#[test]
fn cfg_c6_mpeg2_stereo_bt1() {
    run_row("C6", Spec::new(false, false).fixed(true, 1, false), 256, 6);
}
#[test]
fn cfg_c7_mpeg1_mono_bt1() {
    run_row("C7", Spec::new(true, true).fixed(true, 1, false), 256, 7);
}
#[test]
fn cfg_c8_mpeg1_stereo_bt1() {
    run_row("C8", Spec::new(true, false).fixed(true, 1, false), 256, 8);
}

// ---------------------------------------------------------------------------
// Group 3 — window switched, block_type = 3
// ---------------------------------------------------------------------------

#[test]
fn cfg_c9_mpeg2_mono_bt3() {
    run_row("C9", Spec::new(false, true).fixed(true, 3, true), 256, 9);
}
#[test]
fn cfg_c10_mpeg2_stereo_bt3() {
    run_row("C10", Spec::new(false, false).fixed(true, 3, false), 256, 10);
}
#[test]
fn cfg_c11_mpeg1_mono_bt3() {
    run_row("C11", Spec::new(true, true).fixed(true, 3, true), 256, 11);
}
#[test]
fn cfg_c12_mpeg1_stereo_bt3() {
    run_row("C12", Spec::new(true, false).fixed(true, 3, false), 256, 12);
}

// ---------------------------------------------------------------------------
// Group 4 — block_type = 2, mixed_block_flag = 0 (g_scf_short)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c13_mpeg2_mono_bt2_short() {
    run_row("C13", Spec::new(false, true).fixed(true, 2, false), 256, 13);
}
#[test]
fn cfg_c14_mpeg2_stereo_bt2_short() {
    run_row("C14", Spec::new(false, false).fixed(true, 2, false), 256, 14);
}
#[test]
fn cfg_c15_mpeg1_mono_bt2_short() {
    run_row("C15", Spec::new(true, true).fixed(true, 2, false), 256, 15);
}
#[test]
fn cfg_c16_mpeg1_stereo_bt2_short() {
    run_row("C16", Spec::new(true, false).fixed(true, 2, false), 256, 16);
}

// ---------------------------------------------------------------------------
// Group 5 — block_type = 2, mixed_block_flag = 1 (g_scf_mixed)
// ---------------------------------------------------------------------------

#[test]
fn cfg_c17_mpeg2_mono_bt2_mixed() {
    run_row("C17", Spec::new(false, true).fixed(true, 2, true), 256, 17);
}
#[test]
fn cfg_c18_mpeg2_stereo_bt2_mixed() {
    run_row("C18", Spec::new(false, false).fixed(true, 2, true), 256, 18);
}
#[test]
fn cfg_c19_mpeg1_mono_bt2_mixed() {
    run_row("C19", Spec::new(true, true).fixed(true, 2, true), 256, 19);
}
#[test]
fn cfg_c20_mpeg1_stereo_bt2_mixed() {
    run_row("C20", Spec::new(true, false).fixed(true, 2, true), 256, 20);
}

// ---------------------------------------------------------------------------
// Group 6 — heterogeneous granules
// ---------------------------------------------------------------------------

#[test]
fn cfg_c21_mpeg2_stereo_heterogeneous() {
    run_row("C21", Spec::new(false, false).limit(LimitMode::Random), 2048, 21);
}
#[test]
fn cfg_c22_mpeg1_stereo_heterogeneous() {
    run_row("C22", Spec::new(true, false).limit(LimitMode::Random), 2048, 22);
}
#[test]
fn cfg_c23_mpeg1_mono_heterogeneous() {
    run_row("C23", Spec::new(true, true).limit(LimitMode::Random), 2048, 23);
}

// ---------------------------------------------------------------------------
// Group 7 — sr_idx sweep, all three tables per index
// ---------------------------------------------------------------------------

fn sr_row(row: &str, sr: i32, seed: u64) {
    // sr_idx 0..5 only reachable with MPEG2, 6..8 only with MPEG1
    let mpeg1 = sr >= 6;
    for (i, (w, bt, mb)) in [
        (false, 0u32, false),
        (true, 1, false),
        (true, 3, false),
        (true, 2, false),
        (true, 2, true),
        (true, 1, true),
    ]
    .into_iter()
    .enumerate()
    {
        for mono in [false, true] {
            run_row(
                &format!("{row}/sr={sr}/w={w},bt={bt},mb={mb},mono={mono}"),
                Spec::new(mpeg1, mono).sr(sr).fixed(w, bt, mb),
                32,
                seed * 100 + i as u64 * 2 + mono as u64,
            );
        }
    }
}

#[test]
fn cfg_c24_sr0() {
    sr_row("C24", 0, 24);
}
#[test]
fn cfg_c25_sr1() {
    sr_row("C25", 1, 25);
}
#[test]
fn cfg_c26_sr2() {
    sr_row("C26", 2, 26);
}
#[test]
fn cfg_c27_sr3() {
    sr_row("C27", 3, 27);
}
#[test]
fn cfg_c28_sr4() {
    sr_row("C28", 4, 28);
}
#[test]
fn cfg_c29_sr5() {
    sr_row("C29", 5, 29);
}
#[test]
fn cfg_c30_sr6() {
    sr_row("C30", 6, 30);
}
#[test]
fn cfg_c31_sr7() {
    sr_row("C31", 7, 31);
}
#[test]
fn cfg_c32_sr8_out_of_range() {
    // sr_idx == 8 indexes one row past the end of every table (ERRORS.md E5).
    // The pointed-to bytes are not comparable across libraries, but every other
    // observable must still match and neither side may panic or clamp.
    sr_row("C32", 8, 32);
}

// ---------------------------------------------------------------------------
// Group 8 — initial bit alignment
// ---------------------------------------------------------------------------

fn align_row(row: &str, a: usize, seed: u64) {
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        run_row(
            &format!("{row}/align={a}/mpeg1={mpeg1},mono={mono}"),
            Spec::new(mpeg1, mono).align(a).limit(LimitMode::Random),
            64,
            seed * 10 + mpeg1 as u64 * 2 + mono as u64,
        );
    }
}

#[test]
fn cfg_c33_align0() {
    align_row("C33", 0, 33);
}
#[test]
fn cfg_c34_align1() {
    align_row("C34", 1, 34);
}
#[test]
fn cfg_c35_align2() {
    align_row("C35", 2, 35);
}
#[test]
fn cfg_c36_align3() {
    align_row("C36", 3, 36);
}
#[test]
fn cfg_c37_align4() {
    align_row("C37", 4, 37);
}
#[test]
fn cfg_c38_align5() {
    align_row("C38", 5, 38);
}
#[test]
fn cfg_c39_align6() {
    align_row("C39", 6, 39);
}
#[test]
fn cfg_c40_align7() {
    align_row("C40", 7, 40);
}

#[test]
fn cfg_c41_large_pos() {
    // pos >> 3 is large: the payload sits ~1 MiB into the buffer.
    for (mpeg1, mono) in [(false, false), (true, false)] {
        run_row(
            "C41/large-pos",
            Spec::new(mpeg1, mono).lead(1 << 20).limit(LimitMode::Random),
            128,
            41,
        );
    }
}

// ---------------------------------------------------------------------------
// Group 9 — limit shapes
// ---------------------------------------------------------------------------

#[test]
fn cfg_c42_limit_ample() {
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        run_row(
            "C42/ample",
            Spec::new(mpeg1, mono).limit(LimitMode::Ample),
            128,
            42,
        );
    }
}

#[test]
fn cfg_c43_limit_exact() {
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        run_row(
            "C43/exact",
            Spec::new(mpeg1, mono).limit(LimitMode::Exact),
            128,
            43,
        );
    }
}

#[test]
fn cfg_c44_limit_one_short() {
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        run_row(
            "C44/exact-1",
            Spec::new(mpeg1, mono).limit(LimitMode::ExactMinusOne),
            128,
            44,
        );
    }
}

/// C45: sweep the limit over EVERY bit position from before the start of the
/// side info to just past its end, for each of the four `M`x`CH` shapes and
/// each block-type path. Every single truncation point is compared.
#[test]
fn cfg_c45_limit_sweep_every_bit() {
    let p = Pair::load();
    let mut rng = Rng::new(45);
    let mut cases = 0usize;
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        for (w, bt, mb) in [
            (false, 0u32, false),
            (true, 1, false),
            (true, 2, false),
            (true, 2, true),
            (true, 3, false),
        ] {
            let opts = reachable_sr(mpeg1);
            let sr = opts[rng.below(opts.len() as u32) as usize];
            let hdr = make_hdr(mpeg1, mono, sr, &mut rng).unwrap();
            let gr_count = hdr_gr_count(&hdr) as usize;
            let mut choices = [GrChoice::random(&mut rng); MAX_GR];
            for g in 0..gr_count {
                let mut c = GrChoice::random(&mut rng);
                c.w = w;
                c.bt = bt;
                c.mb = mb;
                choices[g] = c;
            }
            let mdb = if mpeg1 { rng.below(512) } else { rng.below(256) };
            let bw = build_side_info(&hdr, mdb, rng.next_u32(), &choices, &mut rng);
            let packed = pack_with_pad(&bw, PAD, 3, &mut rng);
            let total_bits = (packed.buf.len() * 8) as i32;
            for k in -8..=(packed.nbits as i32 + 8) {
                let limit = (packed.pos + k).min(total_bits);
                diff(
                    &p,
                    &Case {
                        label: format!(
                            "C45 mpeg1={mpeg1} mono={mono} w={w} bt={bt} mb={mb} limit_off={k}"
                        ),
                        buf: &packed.buf,
                        pos: packed.pos,
                        limit,
                        hdr,
                        compare_sfb_contents: sr <= 7,
                    },
                );
                cases += 1;
            }
        }
    }
    eprintln!("C45: {cases} limit positions OK");
}

// ---------------------------------------------------------------------------
// Group 10 — preflag / scalefac_compress
// ---------------------------------------------------------------------------

#[test]
fn cfg_c46_mpeg2_sfc_below_500() {
    for v in [0u32, 1, 100, 250, 498] {
        run_row(
            &format!("C46/sfc={v}"),
            Spec::new(false, false).sfc(v),
            64,
            46 * 1000 + v as u64,
        );
    }
}

#[test]
fn cfg_c47_mpeg2_sfc_at_or_above_500() {
    for v in [500u32, 501, 505, 510, 511] {
        run_row(
            &format!("C47/sfc={v}"),
            Spec::new(false, false).sfc(v),
            64,
            47 * 1000 + v as u64,
        );
    }
}

#[test]
fn cfg_c48_mpeg2_sfc_threshold_boundary() {
    for v in [499u32, 500] {
        for mono in [false, true] {
            run_row(
                &format!("C48/sfc={v}/mono={mono}"),
                Spec::new(false, mono).sfc(v),
                128,
                48 * 1000 + v as u64 * 2 + mono as u64,
            );
        }
    }
}

#[test]
fn cfg_c49_mpeg1_preflag_from_bitstream() {
    // MPEG1: scalefac_compress is only 4 bits and preflag comes from the
    // bitstream, so `>= 500` can never fire.
    for v in [0u32, 15] {
        for mono in [false, true] {
            run_row(
                &format!("C49/sfc={v}/mono={mono}"),
                Spec::new(true, mono).sfc(v),
                128,
                49 * 1000 + v as u64 * 2 + mono as u64,
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Group 11 — value extremes
// ---------------------------------------------------------------------------

#[test]
fn cfg_c50_all_zero_fields() {
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        run_row(
            "C50/all-zero",
            Spec::new(mpeg1, mono).p23(0).bv(0).mdb(0).sfc(0),
            32,
            50,
        );
    }
}

#[test]
fn cfg_c51_all_max_fields() {
    let p = Pair::load();
    let mut rng = Rng::new(51);
    // an all-ones buffer: every field reads its maximum value
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        for sr in reachable_sr(mpeg1) {
            let hdr = make_hdr(mpeg1, mono, *sr, &mut rng).unwrap();
            let buf = vec![0xFFu8; 512];
            for align in 0..8 {
                let pos = 64 * 8 + align;
                diff(
                    &p,
                    &Case {
                        label: format!("C51 all-ones mpeg1={mpeg1} mono={mono} sr={sr} align={align}"),
                        buf: &buf,
                        pos: pos as i32,
                        limit: (buf.len() * 8) as i32,
                        hdr,
                        compare_sfb_contents: *sr <= 7,
                    },
                );
            }
        }
    }
    eprintln!("C51: all-ones buffer OK");
}

#[test]
fn cfg_c52_big_values_at_288() {
    for (mpeg1, mono) in [(false, false), (true, false), (true, true)] {
        run_row(
            "C52/bv=288",
            Spec::new(mpeg1, mono).bv(288).limit(LimitMode::Ample),
            128,
            52,
        );
    }
}

#[test]
fn cfg_c53_main_data_begin_extremes() {
    // exercise the final `part_23_sum + pos > limit + main_data_begin*8`
    // comparison from both sides
    for (mpeg1, mono) in [(false, false), (false, true), (true, false), (true, true)] {
        let mdb_max = if mpeg1 { 511 } else { 255 };
        for mdb in [0u32, 1, mdb_max / 2, mdb_max] {
            for p23 in [0u32, 1, 2047, 4095] {
                run_row(
                    &format!("C53/mdb={mdb}/p23={p23}/mpeg1={mpeg1}/mono={mono}"),
                    Spec::new(mpeg1, mono).mdb(mdb).p23(p23),
                    8,
                    53 * 10_000 + mdb as u64 * 10 + p23 as u64,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Group 12 — unconstrained fuzz
// ---------------------------------------------------------------------------

#[test]
fn cfg_c54_unconstrained_fuzz() {
    let p = Pair::load();
    let mut rng = Rng::new(54);
    let n: usize = std::env::var("FUZZ_N")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(200_000);
    // coverage counters over the axes CONFIGS.md enumerates
    let mut cov: std::collections::BTreeMap<(bool, bool, i32, u8, u8, u8), u32> =
        Default::default();
    let mut rets: std::collections::BTreeMap<i32, u32> = Default::default();
    let buf_len = 512usize;
    for it in 0..n {
        let mut buf = vec![0u8; buf_len];
        rng.fill(&mut buf);
        let mut hdr = [0u8; 4];
        rng.fill(&mut hdr);
        // pos anywhere in the first half, limit anywhere at all (including
        // before pos) but never past the buffer
        let pos = rng.below((buf_len as u32 / 2) * 8) as i32;
        let limit = match rng.below(4) {
            0 => (buf_len * 8) as i32,
            1 => pos,
            2 => pos + rng.below(200) as i32,
            _ => rng.below((buf_len as u32) * 8) as i32,
        };
        let sr = hdr_sr_idx(&hdr);
        let case = Case {
            label: format!("C54 iter={it}"),
            buf: &buf,
            pos,
            limit,
            hdr,
            compare_sfb_contents: false, // cheap path; contents covered by C24..C31
        };
        diff(&p, &case);

        // record coverage from the C side's observable behaviour
        let c = unsafe {
            let mut bs = bs_t { buf: buf.as_ptr(), pos, limit };
            let mut gr = [std::mem::zeroed::<L3_gr_info_t>(); MAX_GR];
            let ret = (p.c)(&mut bs, gr.as_mut_ptr(), hdr.as_ptr());
            (ret, gr)
        };
        *rets.entry(c.0.min(1)).or_default() += 1;
        let g = &c.1[0];
        *cov.entry((
            hdr_is_mpeg1(&hdr),
            hdr_is_mono(&hdr),
            sr,
            (g.block_type != 0 || g.n_short_sfb == 39) as u8,
            g.block_type,
            g.mixed_block_flag & 1,
        ))
        .or_default() += 1;
    }
    eprintln!("C54: {n} random cases OK");
    eprintln!("C54: {} distinct (mpeg1,mono,sr,w,bt,mb) tuples covered", cov.len());
    eprintln!("C54: return-value classes: {rets:?}");
    // every mpeg1 x mono x reachable-sr combination must have been hit
    for mpeg1 in [false, true] {
        for mono in [false, true] {
            for sr in reachable_sr(mpeg1) {
                assert!(
                    cov.keys().any(|k| k.0 == mpeg1 && k.1 == mono && k.2 == *sr),
                    "C54 never covered mpeg1={mpeg1} mono={mono} sr_idx={sr}"
                );
            }
        }
    }
    // both success and failure returns must be represented
    assert!(rets.len() >= 2, "C54 only ever saw one return class: {rets:?}");
}
