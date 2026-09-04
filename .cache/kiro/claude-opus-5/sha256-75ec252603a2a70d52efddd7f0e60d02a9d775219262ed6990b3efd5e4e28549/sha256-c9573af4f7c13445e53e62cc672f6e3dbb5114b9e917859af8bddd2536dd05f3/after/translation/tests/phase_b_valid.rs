//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Every row drives BOTH `.so`s through their exported symbols with many
//! randomized inputs (fixed seed) and compares the results byte-for-byte.

mod common;

use common::deflate::{self, Tok};
use common::png::{self, ColorType, PngSpec};
use common::*;

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

#[derive(Copy, Clone, Debug)]
enum Enc {
    Stored,
    Fixed,
    FixedLz,
    Dyn {
        depth: u8,
        hlit: usize,
        hdist: usize,
        rle: bool,
    },
    DynLz {
        depth: u8,
        rle: bool,
    },
    Flate2(u32),
}

fn encode(raw: &[u8], enc: Enc) -> Vec<u8> {
    match enc {
        Enc::Stored => {
            assert!(raw.len() <= 0xFFFF, "stored block needs len <= 65535");
            deflate::stored_block(raw, true)
        }
        Enc::Fixed => {
            let toks: Vec<Tok> = raw.iter().map(|&b| Tok::Lit(b)).collect();
            deflate::fixed_stream(&toks)
        }
        Enc::FixedLz => {
            let toks = deflate::lz77(raw, 32768, 32);
            deflate::fixed_stream(&toks)
        }
        Enc::Dyn {
            depth,
            hlit,
            hdist,
            rle,
        } => {
            let toks: Vec<Tok> = raw.iter().map(|&b| Tok::Lit(b)).collect();
            deflate::dynamic_stream(&toks, depth, hlit, hdist, rle)
        }
        Enc::DynLz { depth, rle } => {
            let toks = deflate::lz77(raw, 32768, 32);
            let (ll, dl) = deflate::full_tables(depth);
            let mut bw = deflate::BitWriter::new();
            deflate::dynamic_block(&mut bw, &ll, &dl, &toks, true, rle);
            bw.finish()
        }
        Enc::Flate2(level) => {
            use std::io::Write;
            let mut e = flate2::write::DeflateEncoder::new(
                Vec::new(),
                flate2::Compression::new(level),
            );
            e.write_all(raw).unwrap();
            e.finish().unwrap()
        }
    }
}

struct Case {
    ct: ColorType,
    w: usize,
    h: usize,
    filters: Vec<u8>,
    raw: Vec<u8>,
    spec: PngSpec,
    /// Compare against the independent reference model too.
    model: bool,
}

impl Case {
    fn new(rng: &mut Rng, ct: ColorType, w: usize, h: usize, filters: Vec<u8>, enc: Enc) -> Case {
        let bpp = ct.bpp();
        let raw = png::raw_scanlines(rng, w, h, bpp, &filters);
        let def = encode(&raw, enc);
        let spec = PngSpec::new(w as u32, h as u32, ct as u8, def, raw.clone());
        Case {
            ct,
            w,
            h,
            filters,
            raw,
            spec,
            model: true,
        }
    }

    /// Run the differential comparison. Returns `true` if both sides decoded.
    #[track_caller]
    fn check(&self, label: &str) -> bool {
        let file = self.spec.build();
        let (c, r) = call_load_png(&file);
        assert_same(label, &c, &r);
        if c.pix_null {
            return false;
        }
        if self.model {
            let bpp = self.ct.bpp();
            let mut u = self.raw.clone();
            assert!(
                png::model_unfilter(self.w, self.h, bpp, &mut u),
                "[{label}] model rejected filters {:?}",
                self.filters
            );
            let expect = png::model_pixels(
                self.w,
                self.h,
                bpp,
                self.ct as u8,
                &u,
                self.spec.plte.as_deref(),
                self.spec.trns.as_deref(),
            );
            assert_eq!(
                c.payload, expect,
                "[{label}] C output disagrees with the independent reference model"
            );
        }
        true
    }
}

fn rand_filters(rng: &mut Rng, h: usize) -> Vec<u8> {
    (0..h).map(|_| rng.below(5) as u8).collect()
}

// ===========================================================================
// Rows 1-2 — cp_inflate, stored blocks
// ===========================================================================

#[test]
fn row01_inflate_stored_random_len() {
    let mut rng = Rng::new(SEED ^ 0x01);
    // `cp_ptr` computes the source of the memcpy as
    // `(char*)(words + word_index) - count/8`, which only lands on the real
    // payload for some `in` alignments (with `in_shift == 2`, matching what
    // `load_png_mem` passes: `data + 2` off a 16-aligned malloc). For other
    // alignments the C copies from the wrong offset — that is its behaviour and
    // Rust must reproduce it, so those rows are compared differentially only.
    for _ in 0..200 {
        let n = rng.range(1, 64) as usize;
        let data = rng.bytes(n);
        let def = deflate::stored_block(&data, true);
        let (c, r) = call_inflate_cfg(&def, def.len() as i32, n as i32, 2, |_| {});
        assert_same(&format!("row01 stored len={n} shift=2"), &c, &r);
        assert_eq!(c.ret, 1, "C failed: {}", c.err_str());
        assert_eq!(c.payload, data, "aligned stored copy should be exact");
    }
    for n in [255usize, 256, 257, 1023, 1024, 4095, 65535] {
        let data = rng.bytes(n);
        let def = deflate::stored_block(&data, true);
        let (c, r) = call_inflate_cfg(&def, def.len() as i32, n as i32, 2, |_| {});
        assert_same(&format!("row01 stored len={n} shift=2"), &c, &r);
        assert_eq!(c.ret, 1, "C failed at len={n}: {}", c.err_str());
        assert_eq!(c.payload, data);
    }
    // All four `in` alignments, differential only.
    for shift in 0..4usize {
        for _ in 0..40 {
            let n = rng.range(1, 200) as usize;
            let data = rng.bytes(n);
            let def = deflate::stored_block(&data, true);
            let (c, r) = call_inflate_cfg(&def, def.len() as i32, n as i32, shift, |_| {});
            assert_same(&format!("row01 stored len={n} shift={shift}"), &c, &r);
        }
    }
}

#[test]
fn row02_inflate_stored_empty() {
    let def = deflate::stored_block(&[], true);
    for shift in 0..4usize {
        for out in [0i32, 1, 16] {
            let (c, r) = call_inflate_cfg(&def, def.len() as i32, out, shift, |_| {});
            assert_same(
                &format!("row02 stored LEN=0 out={out} shift={shift}"),
                &c,
                &r,
            );
        }
    }
}

// ===========================================================================
// Rows 3-7 — cp_inflate, fixed Huffman
// ===========================================================================

#[test]
fn row03_inflate_fixed_literals() {
    let mut rng = Rng::new(SEED ^ 0x03);
    // Every byte value, so both the 8-bit (0..143, 280..287) and 9-bit
    // (144..255) halves of the fixed literal tree are used.
    let all: Vec<u8> = (0..=255u8).collect();
    for chunk in [1usize, 2, 3, 143, 144, 145, 256, 300] {
        let data: Vec<u8> = (0..chunk).map(|i| all[i % 256]).collect();
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let def = deflate::fixed_stream(&toks);
        let (c, r) = call_inflate(&def, def.len() as i32, data.len() as i32);
        assert_same(&format!("row03 fixed literals n={chunk}"), &c, &r);
        assert_eq!(c.ret, 1, "C failed: {}", c.err_str());
        assert_eq!(c.payload, data);
    }
    for _ in 0..150 {
        let n = rng.range(1, 400) as usize;
        let data = rng.bytes(n);
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let def = deflate::fixed_stream(&toks);
        let (c, r) = call_inflate(&def, def.len() as i32, n as i32);
        assert_same(&format!("row03 fixed literals rand n={n}"), &c, &r);
        assert_eq!(c.ret, 1, "C failed: {}", c.err_str());
        assert_eq!(c.payload, data);
    }
}

#[test]
fn row04_inflate_fixed_distance_one_memset() {
    let mut rng = Rng::new(SEED ^ 0x04);
    for _ in 0..120 {
        let seed_byte = rng.u8();
        let run = rng.range(3, 258);
        let toks = vec![
            Tok::Lit(seed_byte),
            Tok::Match { len: run, dist: 1 },
            Tok::Lit(rng.u8()),
            Tok::Match { len: 3, dist: 1 },
        ];
        let expect = deflate::expand(&toks);
        let def = deflate::fixed_stream(&toks);
        let (c, r) = call_inflate(&def, def.len() as i32, expect.len() as i32);
        assert_same(&format!("row04 dist=1 run={run}"), &c, &r);
        assert_eq!(c.ret, 1, "C failed: {}", c.err_str());
        assert_eq!(c.payload, expect);
    }
}

#[test]
fn row05_inflate_fixed_overlapping_copy() {
    let mut rng = Rng::new(SEED ^ 0x05);
    for _ in 0..200 {
        let prefix_len = rng.range(1, 40) as usize;
        let prefix = rng.bytes(prefix_len);
        let dist = rng.range(1, prefix_len as u32);
        let len = rng.range(3, 258);
        let mut toks: Vec<Tok> = prefix.iter().map(|&b| Tok::Lit(b)).collect();
        toks.push(Tok::Match { len, dist });
        let expect = deflate::expand(&toks);
        let def = deflate::fixed_stream(&toks);
        let (c, r) = call_inflate(&def, def.len() as i32, expect.len() as i32);
        assert_same(&format!("row05 overlap dist={dist} len={len}"), &c, &r);
        assert_eq!(c.ret, 1, "C failed: {}", c.err_str());
        assert_eq!(c.payload, expect);
    }
}

#[test]
fn row06_inflate_all_length_codes() {
    // Every one of the 29 length codes, at the low, middle and high end of its
    // extra-bit range.
    let mut rng = Rng::new(SEED ^ 0x06);
    let history: Vec<u8> = (0..300u32).map(|i| (i * 7 % 251) as u8).collect();
    for lc in 0..29usize {
        let base = deflate::LEN_BASE[lc];
        let span = 1u32 << deflate::LEN_EXTRA[lc];
        let mut lens = vec![base];
        if span > 1 {
            lens.push(base + span / 2);
            lens.push((base + span - 1).min(258));
        }
        for len in lens {
            if len > 258 {
                continue;
            }
            let dist = rng.range(1, 300);
            let mut toks: Vec<Tok> = history.iter().map(|&b| Tok::Lit(b)).collect();
            toks.push(Tok::Match { len, dist });
            let expect = deflate::expand(&toks);
            let def = deflate::fixed_stream(&toks);
            let (c, r) = call_inflate(&def, def.len() as i32, expect.len() as i32);
            assert_same(&format!("row06 len code {lc} len={len} dist={dist}"), &c, &r);
            assert_eq!(c.ret, 1, "C failed lc={lc} len={len}: {}", c.err_str());
            assert_eq!(c.payload, expect);
        }
    }
}

#[test]
fn row07_inflate_all_distance_codes() {
    // Every one of the 30 distance codes, incl. dist == 32768 (needs a 32 KiB
    // history, so this also drives cp_peak_bits over many words).
    let history: Vec<u8> = (0..32768u32).map(|i| (i * 31 % 253) as u8).collect();
    for dc in 0..30usize {
        let base = deflate::DIST_BASE[dc];
        let span = 1u32 << deflate::DIST_EXTRA[dc];
        let mut dists = vec![base];
        if span > 1 {
            dists.push(base + span / 2);
            dists.push(base + span - 1);
        }
        for dist in dists {
            if dist > 32768 {
                continue;
            }
            let need = dist as usize;
            let hist = &history[..need];
            let mut toks: Vec<Tok> = hist.iter().map(|&b| Tok::Lit(b)).collect();
            toks.push(Tok::Match { len: 258, dist });
            let expect = deflate::expand(&toks);
            let def = deflate::fixed_stream(&toks);
            let (c, r) = call_inflate(&def, def.len() as i32, expect.len() as i32);
            assert_same(&format!("row07 dist code {dc} dist={dist}"), &c, &r);
            assert_eq!(c.ret, 1, "C failed dc={dc} dist={dist}: {}", c.err_str());
            assert_eq!(c.payload, expect);
        }
    }
}

// ===========================================================================
// Rows 8-10 — cp_inflate, dynamic Huffman
// ===========================================================================

#[test]
fn row08_inflate_dynamic_shallow() {
    // depth <= 9 ⇒ every code lands in cp_build's 512-entry `lookup` table.
    let mut rng = Rng::new(SEED ^ 0x08);
    for _ in 0..80 {
        let n = rng.range(1, 600) as usize;
        // A small alphabet keeps every code <= 9 bits.
        let data: Vec<u8> = (0..n).map(|_| rng.below(40) as u8).collect();
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let def = deflate::dynamic_stream(&toks, 9, 257, 2, true);
        let (c, r) = call_inflate(&def, def.len() as i32, n as i32);
        assert_same(&format!("row08 dynamic shallow n={n}"), &c, &r);
        assert_eq!(c.ret, 1, "C failed: {}", c.err_str());
        assert_eq!(c.payload, data);
    }
}

#[test]
fn row09_inflate_dynamic_deep() {
    // Skewed frequencies ⇒ codes longer than 9 bits ⇒ cp_decode's binary
    // search over `tree` is the only path (no `lookup` hit).
    let mut rng = Rng::new(SEED ^ 0x09);
    for trial in 0..40 {
        let mut data: Vec<u8> = Vec::new();
        // Exponentially decaying symbol popularity builds a deep tree.
        for sym in 0..14u32 {
            let reps = 1usize << (13 - sym);
            for _ in 0..reps {
                data.push(sym as u8);
            }
        }
        for _ in 0..rng.range(1, 50) {
            data.push(rng.below(14) as u8);
        }
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let (ll, dl) = deflate::tables_for(&toks, 15, 257, 2);
        let maxlen = *ll.iter().max().unwrap();
        assert!(maxlen > 9, "trial {trial}: tree not deep enough ({maxlen})");
        let mut bw = deflate::BitWriter::new();
        deflate::dynamic_block(&mut bw, &ll, &dl, &toks, true, true);
        let def = bw.finish();
        let (c, r) = call_inflate(&def, def.len() as i32, data.len() as i32);
        assert_same(&format!("row09 dynamic deep maxlen={maxlen}"), &c, &r);
        assert_eq!(c.ret, 1, "C failed: {}", c.err_str());
        assert_eq!(c.payload, data);
    }
}

#[test]
fn row10_inflate_dynamic_rle_code_lengths() {
    // Exercise code-length symbols 16 (repeat 3-6), 17 (3-10 zeros) and
    // 18 (11-138 zeros) in cp_dynamic's switch.
    let mut rng = Rng::new(SEED ^ 0x0A);
    for rle in [false, true] {
        for &(hlit, hdist) in &[
            (257usize, 1usize),
            (257, 2),
            (260, 4),
            (272, 16),
            (286, 30),
            (288, 32),
        ] {
            for _ in 0..12 {
                let n = rng.range(4, 400) as usize;
                let data: Vec<u8> = (0..n).map(|_| rng.below(64) as u8).collect();
                let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
                let def = deflate::dynamic_stream(&toks, 15, hlit, hdist, rle);
                let (c, r) = call_inflate(&def, def.len() as i32, n as i32);
                assert_same(
                    &format!("row10 rle={rle} hlit={hlit} hdist={hdist} n={n}"),
                    &c,
                    &r,
                );
                assert_eq!(c.ret, 1, "C failed: {}", c.err_str());
                assert_eq!(c.payload, data);
            }
        }
    }
}

// ===========================================================================
// Row 11 — multi-block streams
// ===========================================================================

#[test]
fn row11_inflate_multi_block() {
    let mut rng = Rng::new(SEED ^ 0x0B);
    // Fixed + dynamic blocks can be chained freely. A stored block can only be
    // the *last* block (cp_stored's `bits_left/8 <= LEN` check), so it is placed
    // last where present.
    for trial in 0..60 {
        let nblocks = rng.range(2, 5) as usize;
        let mut bw = deflate::BitWriter::new();
        let mut expect: Vec<u8> = Vec::new();
        let mut kinds = Vec::new();
        for b in 0..nblocks {
            let last = b + 1 == nblocks;
            let dn = rng.range(1, 120) as usize;
            let data = rng.bytes(dn);
            let toks: Vec<Tok> = data.iter().map(|&x| Tok::Lit(x)).collect();
            if rng.below(2) == 0 {
                kinds.push("fixed");
                deflate::fixed_block(&mut bw, &toks, last);
            } else {
                kinds.push("dynamic");
                let (ll, dl) = deflate::tables_for(&toks, 15, 288, 2);
                deflate::dynamic_block(&mut bw, &ll, &dl, &toks, last, true);
            }
            expect.extend_from_slice(&data);
        }
        let def = bw.finish();
        let (c, r) = call_inflate(&def, def.len() as i32, expect.len() as i32);
        assert_same(&format!("row11 trial={trial} kinds={kinds:?}"), &c, &r);
        assert_eq!(c.ret, 1, "C failed ({kinds:?}): {}", c.err_str());
        assert_eq!(c.payload, expect);
    }

    // ... plus a final stored block after compressed blocks.
    for trial in 0..30 {
        let mut bw = deflate::BitWriter::new();
        let hn = rng.range(1, 60) as usize;
        let head = rng.bytes(hn);
        let toks: Vec<Tok> = head.iter().map(|&x| Tok::Lit(x)).collect();
        deflate::fixed_block(&mut bw, &toks, false);
        let tn = rng.range(1, 60) as usize;
        let tail = rng.bytes(tn);
        // Stored block header must start byte-aligned from cp_stored's point of
        // view; the C aligns with `cp_read_bits(s, s->count & 7)`, which is what
        // is under test, so just append the raw header bits.
        bw.bits(1, 1);
        bw.bits(0, 2);
        bw.align();
        let mut def = bw.finish();
        let len = tail.len() as u16;
        def.extend_from_slice(&len.to_le_bytes());
        def.extend_from_slice(&(!len).to_le_bytes());
        def.extend_from_slice(&tail);
        let mut expect = head.clone();
        expect.extend_from_slice(&tail);
        let (c, r) = call_inflate(&def, def.len() as i32, expect.len() as i32);
        assert_same(&format!("row11 fixed+stored trial={trial}"), &c, &r);
    }
}

// ===========================================================================
// Rows 12-13 — input alignment / output sizing
// ===========================================================================

#[test]
fn row12_inflate_alignment_matrix() {
    let mut rng = Rng::new(SEED ^ 0x0C);
    // in_shift 0..3 gives first_bytes 0..3 (glibc malloc is 16-aligned);
    // padding the stream length covers last_bytes 0..3 / final_word_available.
    for in_shift in 0..4usize {
        for pad in 0..4usize {
            for _ in 0..20 {
                let n = rng.range(1, 80) as usize;
                let data = rng.bytes(n);
                let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
                let mut def = deflate::fixed_stream(&toks);
                for _ in 0..pad {
                    def.push(0);
                }
                let (c, r) = call_inflate_cfg(
                    &def,
                    def.len() as i32,
                    n as i32,
                    in_shift,
                    |_| {},
                );
                assert_same(
                    &format!("row12 shift={in_shift} pad={pad} n={n} in_bytes={}", def.len()),
                    &c,
                    &r,
                );
                assert_eq!(c.ret, 1, "C failed: {}", c.err_str());
                assert_eq!(c.payload, data);
            }
        }
    }
}

#[test]
fn row13_inflate_out_bytes_slack() {
    let mut rng = Rng::new(SEED ^ 0x0D);
    for _ in 0..120 {
        let n = rng.range(1, 100) as usize;
        let data = rng.bytes(n);
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let def = deflate::fixed_stream(&toks);
        for slack in [0usize, 1, 7, 64, 1000] {
            let ob = (n + slack) as i32;
            let (c, r) = call_inflate(&def, def.len() as i32, ob);
            assert_same(&format!("row13 n={n} slack={slack}"), &c, &r);
            assert_eq!(c.ret, 1, "C failed: {}", c.err_str());
            assert_eq!(&c.payload[..n], &data[..]);
            // Row 46: the untouched tail keeps the caller's 0xAA fill.
            assert!(c.payload[n..].iter().all(|&b| b == 0xAA));
        }
    }
}

// ===========================================================================
// Rows 14-22 — load_png_mem, colour types x filter types
// ===========================================================================

fn filter_sweep_case(ct: ColorType, fixed_filter: Option<u8>, tag: &str, seed_mix: u64) {
    let mut rng = Rng::new(SEED ^ seed_mix);
    for _ in 0..25 {
        let w = rng.range(1, 17) as usize;
        let h = rng.range(1, 17) as usize;
        let filters = match fixed_filter {
            Some(f) => vec![f; h],
            None => rand_filters(&mut rng, h),
        };
        let enc = if png::raw_size(w, h, ct.bpp()) <= 0xFFFF {
            Enc::Stored
        } else {
            Enc::Fixed
        };
        let mut case = Case::new(&mut rng, ct, w, h, filters.clone(), enc);
        if ct == ColorType::Indexed {
            case.spec.plte = Some(rng.bytes(256 * 3));
        }
        let label = format!("{tag} ct={} {w}x{h} filters={filters:?}", ct as u8);
        assert!(case.check(&label), "C failed to decode: {label}");
    }
}

#[test]
fn row14_grey_filter0() {
    filter_sweep_case(ColorType::Grey, Some(0), "row14", 0x14);
}
#[test]
fn row15_grey_filter1_sub() {
    filter_sweep_case(ColorType::Grey, Some(1), "row15", 0x15);
}
#[test]
fn row16_grey_filter2_up() {
    filter_sweep_case(ColorType::Grey, Some(2), "row16", 0x16);
}
#[test]
fn row17_grey_filter3_average() {
    filter_sweep_case(ColorType::Grey, Some(3), "row17", 0x17);
}
#[test]
fn row18_grey_filter4_paeth() {
    filter_sweep_case(ColorType::Grey, Some(4), "row18", 0x18);
}

#[test]
fn row19_grey_random_filters_and_first_row_paths() {
    filter_sweep_case(ColorType::Grey, None, "row19", 0x19);
    // Force each of the 5 reduced first-row paths explicitly.
    let mut rng = Rng::new(SEED ^ 0x191);
    for f0 in 0..5u8 {
        for h in [1usize, 2, 5] {
            let w = 7usize;
            let mut filters = rand_filters(&mut rng, h);
            filters[0] = f0;
            let case = Case::new(&mut rng, ColorType::Grey, w, h, filters.clone(), Enc::Stored);
            let label = format!("row19 first-row filter={f0} h={h}");
            assert!(case.check(&label), "C failed: {label}");
        }
    }
}

#[test]
fn row20_rgb_random_filters() {
    filter_sweep_case(ColorType::Rgb, None, "row20", 0x20);
    for f in 0..5u8 {
        filter_sweep_case(ColorType::Rgb, Some(f), "row20f", 0x200 + f as u64);
    }
}

#[test]
fn row21_greyalpha_random_filters() {
    filter_sweep_case(ColorType::GreyAlpha, None, "row21", 0x21);
    for f in 0..5u8 {
        filter_sweep_case(ColorType::GreyAlpha, Some(f), "row21f", 0x210 + f as u64);
    }
}

#[test]
fn row22_rgba_random_filters() {
    filter_sweep_case(ColorType::Rgba, None, "row22", 0x22);
    for f in 0..5u8 {
        filter_sweep_case(ColorType::Rgba, Some(f), "row22f", 0x220 + f as u64);
    }
}

// ===========================================================================
// Rows 23-26 — indexed colour / PLTE / tRNS
// ===========================================================================

#[test]
fn row23_indexed_no_trns() {
    let mut rng = Rng::new(SEED ^ 0x23);
    for _ in 0..30 {
        let w = rng.range(1, 13) as usize;
        let h = rng.range(1, 13) as usize;
        let filters = rand_filters(&mut rng, h);
        let mut case = Case::new(&mut rng, ColorType::Indexed, w, h, filters, Enc::Stored);
        case.spec.plte = Some(rng.bytes(256 * 3));
        assert!(case.check("row23 indexed no tRNS"), "C failed");
    }
}

#[test]
fn row24_indexed_full_trns() {
    let mut rng = Rng::new(SEED ^ 0x24);
    for _ in 0..30 {
        let w = rng.range(1, 13) as usize;
        let h = rng.range(1, 13) as usize;
        let filters = rand_filters(&mut rng, h);
        let mut case = Case::new(&mut rng, ColorType::Indexed, w, h, filters, Enc::Stored);
        case.spec.plte = Some(rng.bytes(256 * 3));
        case.spec.trns = Some(rng.bytes(256));
        assert!(case.check("row24 indexed full tRNS"), "C failed");
    }
}

#[test]
fn row25_indexed_short_trns() {
    // trns_len < 256 ⇒ both branches of cp_get_alpha_for_indexed_image.
    let mut rng = Rng::new(SEED ^ 0x25);
    for trns_len in [0usize, 1, 2, 17, 128, 255] {
        for _ in 0..8 {
            let w = rng.range(1, 13) as usize;
            let h = rng.range(1, 13) as usize;
            let filters = rand_filters(&mut rng, h);
            let mut case = Case::new(&mut rng, ColorType::Indexed, w, h, filters, Enc::Stored);
            case.spec.plte = Some(rng.bytes(256 * 3));
            case.spec.trns = Some(rng.bytes(trns_len));
            assert!(
                case.check(&format!("row25 indexed trns_len={trns_len}")),
                "C failed trns_len={trns_len}"
            );
        }
    }
}

#[test]
fn row26_indexed_short_plte() {
    // A PLTE shorter than the indices used makes the C read past the chunk.
    // No reference model here (the values come from whatever follows the chunk),
    // but C and Rust must agree byte-for-byte.
    let mut rng = Rng::new(SEED ^ 0x26);
    for plte_entries in [1usize, 2, 16, 100, 255] {
        for _ in 0..8 {
            let w = rng.range(1, 11) as usize;
            let h = rng.range(1, 11) as usize;
            let filters = vec![0u8; h];
            let mut case = Case::new(&mut rng, ColorType::Indexed, w, h, filters, Enc::Stored);
            case.spec.plte = Some(rng.bytes(plte_entries * 3));
            case.model = false;
            case.check(&format!("row26 indexed plte_entries={plte_entries}"));
        }
    }
}

// ===========================================================================
// Rows 27-28, 47 — image shapes
// ===========================================================================

#[test]
fn row27_one_by_one_every_color_type() {
    let mut rng = Rng::new(SEED ^ 0x27);
    for ct in ColorType::ALL {
        for enc in [Enc::Stored, Enc::Fixed, Enc::Dyn { depth: 15, hlit: 288, hdist: 2, rle: true }] {
            let mut case = Case::new(&mut rng, ct, 1, 1, vec![0], enc);
            if ct == ColorType::Indexed {
                case.spec.plte = Some(rng.bytes(256 * 3));
            }
            let label = format!("row27 1x1 ct={} enc={enc:?}", ct as u8);
            assert!(case.check(&label), "C failed: {label}");
        }
    }
}

#[test]
fn row28_tall_and_wide() {
    let mut rng = Rng::new(SEED ^ 0x28);
    for ct in ColorType::ALL {
        for (w, h) in [(1usize, 1usize), (1, 2), (1, 37), (37, 1), (2, 1), (1, 200), (200, 1)] {
            let filters = rand_filters(&mut rng, h);
            let mut case = Case::new(&mut rng, ct, w, h, filters, Enc::Stored);
            if ct == ColorType::Indexed {
                case.spec.plte = Some(rng.bytes(256 * 3));
            }
            let label = format!("row28 {w}x{h} ct={}", ct as u8);
            assert!(case.check(&label), "C failed: {label}");
        }
    }
}

#[test]
fn row47_out_offset_trick_every_bpp() {
    // (w+1)*h*bpp vs (w*bpp+1)*h differ by h*(bpp-1); `out` is placed so the
    // in-place expansion to RGBA works. Sweep bpp and shapes.
    let mut rng = Rng::new(SEED ^ 0x47);
    for ct in ColorType::ALL {
        for (w, h) in [(1usize, 1usize), (3, 3), (1, 64), (64, 1), (17, 9), (31, 31)] {
            let filters = rand_filters(&mut rng, h);
            let raw_sz = png::raw_size(w, h, ct.bpp());
            let enc = if raw_sz <= 0xFFFF { Enc::Stored } else { Enc::FixedLz };
            let mut case = Case::new(&mut rng, ct, w, h, filters, enc);
            if ct == ColorType::Indexed {
                case.spec.plte = Some(rng.bytes(256 * 3));
            }
            let label = format!("row47 {w}x{h} bpp={} ct={}", ct.bpp(), ct as u8);
            assert!(case.check(&label), "C failed: {label}");
        }
    }
}

// ===========================================================================
// Rows 29-32, 38 — chunk layout
// ===========================================================================

#[test]
fn row29_idat_two_chunks() {
    let mut rng = Rng::new(SEED ^ 0x29);
    for ct in ColorType::ALL {
        for _ in 0..6 {
            let w = rng.range(2, 12) as usize;
            let h = rng.range(2, 12) as usize;
            let filters = rand_filters(&mut rng, h);
            let mut case = Case::new(&mut rng, ct, w, h, filters, Enc::Stored);
            case.spec.idat_chunks = 2;
            if ct == ColorType::Indexed {
                case.spec.plte = Some(rng.bytes(256 * 3));
            }
            let label = format!("row29 2 IDATs ct={} {w}x{h}", ct as u8);
            assert!(case.check(&label), "C failed: {label}");
        }
    }
}

#[test]
fn row30_idat_many_chunks() {
    let mut rng = Rng::new(SEED ^ 0x30);
    for n in [3usize, 5, 8, 13] {
        for ct in ColorType::ALL {
            let w = rng.range(3, 14) as usize;
            let h = rng.range(3, 14) as usize;
            let filters = rand_filters(&mut rng, h);
            let mut case = Case::new(&mut rng, ct, w, h, filters, Enc::Stored);
            case.spec.idat_chunks = n;
            if ct == ColorType::Indexed {
                case.spec.plte = Some(rng.bytes(256 * 3));
            }
            let label = format!("row30 {n} IDATs ct={} {w}x{h}", ct as u8);
            assert!(case.check(&label), "C failed: {label}");
        }
    }
}

#[test]
fn row31_ancillary_chunks() {
    let mut rng = Rng::new(SEED ^ 0x31);
    for ct in ColorType::ALL {
        for _ in 0..6 {
            let w = rng.range(2, 10) as usize;
            let h = rng.range(2, 10) as usize;
            let filters = rand_filters(&mut rng, h);
            let mut case = Case::new(&mut rng, ct, w, h, filters, Enc::Stored);
            case.spec.pre = vec![
                png::chunk(b"gAMA", &45455u32.to_be_bytes()),
                png::chunk(b"cHRM", &rng.bytes(32)),
            ];
            case.spec.mid = vec![
                png::chunk(b"pHYs", &rng.bytes(9)),
                png::chunk(b"tEXt", b"Comment\0hello"),
                png::chunk(b"bKGD", &rng.bytes(6)),
            ];
            if ct == ColorType::Indexed {
                case.spec.plte = Some(rng.bytes(256 * 3));
            }
            let label = format!("row31 ancillary ct={} {w}x{h}", ct as u8);
            assert!(case.check(&label), "C failed: {label}");
        }
    }
}

#[test]
fn row32_plte_trns_order() {
    let mut rng = Rng::new(SEED ^ 0x32);
    for trns_first in [false, true] {
        for _ in 0..12 {
            let w = rng.range(2, 10) as usize;
            let h = rng.range(2, 10) as usize;
            let filters = rand_filters(&mut rng, h);
            let mut case =
                Case::new(&mut rng, ColorType::Indexed, w, h, filters, Enc::Stored);
            case.spec.plte = Some(rng.bytes(256 * 3));
            let tl = rng.range(0, 256) as usize;
            case.spec.trns = Some(rng.bytes(tl));
            case.spec.trns_first = trns_first;
            // With tRNS first, the C's two sequential cp_find calls make the
            // *second* search start after tRNS, so PLTE is still found but the
            // scan positions differ. Whatever the C does, Rust must match.
            case.model = !trns_first;
            let label = format!("row32 trns_first={trns_first} {w}x{h}");
            case.check(&label);
        }
    }
}

#[test]
fn row38_trailing_chunks() {
    let mut rng = Rng::new(SEED ^ 0x38);
    for ct in ColorType::ALL {
        for _ in 0..6 {
            let w = rng.range(2, 10) as usize;
            let h = rng.range(2, 10) as usize;
            let filters = rand_filters(&mut rng, h);
            let mut case = Case::new(&mut rng, ct, w, h, filters, Enc::Stored);
            case.spec.post = vec![
                png::chunk(b"tEXt", b"after\0idat"),
                png::chunk(b"zTXt", &rng.bytes(20)),
            ];
            if ct == ColorType::Indexed {
                case.spec.plte = Some(rng.bytes(256 * 3));
            }
            let label = format!("row38 trailing ct={} {w}x{h}", ct as u8);
            assert!(case.check(&label), "C failed: {label}");
        }
    }
}

// ===========================================================================
// Rows 33-34 — zlib header bytes
// ===========================================================================

#[test]
fn row33_zlib_cinfo_sweep() {
    let mut rng = Rng::new(SEED ^ 0x33);
    for cinfo in 0..8u8 {
        let w = 6usize;
        let h = 4usize;
        let filters = rand_filters(&mut rng, h);
        let mut case = Case::new(&mut rng, ColorType::Rgb, w, h, filters, Enc::Stored);
        case.spec.cmf = (cinfo << 4) | 0x08;
        let label = format!("row33 cinfo={cinfo}");
        assert!(case.check(&label), "C failed: {label}");
    }
}

#[test]
fn row34_zlib_flg_sweep() {
    let mut rng = Rng::new(SEED ^ 0x34);
    // Everything except bit 5 (FDICT, which the C rejects) is ignored.
    for flg in [0x00u8, 0x01, 0x1F, 0x40, 0x80, 0x9C, 0xDA, 0xFF & !0x20] {
        let w = 5usize;
        let h = 3usize;
        let filters = rand_filters(&mut rng, h);
        let mut case = Case::new(&mut rng, ColorType::Rgba, w, h, filters, Enc::Stored);
        case.spec.flg = flg;
        let label = format!("row34 flg=0x{flg:02x}");
        assert!(case.check(&label), "C failed: {label}");
    }
}

// ===========================================================================
// Rows 35-37, 39 — IDAT payload block types / bigger images
// ===========================================================================

#[test]
fn row35_png_stored_blocks() {
    let mut rng = Rng::new(SEED ^ 0x35);
    for ct in ColorType::ALL {
        for _ in 0..8 {
            let w = rng.range(1, 20) as usize;
            let h = rng.range(1, 20) as usize;
            let filters = rand_filters(&mut rng, h);
            let mut case = Case::new(&mut rng, ct, w, h, filters, Enc::Stored);
            if ct == ColorType::Indexed {
                case.spec.plte = Some(rng.bytes(256 * 3));
            }
            let label = format!("row35 stored ct={} {w}x{h}", ct as u8);
            assert!(case.check(&label), "C failed: {label}");
        }
    }
}

#[test]
fn row36_png_fixed_blocks() {
    let mut rng = Rng::new(SEED ^ 0x36);
    for ct in ColorType::ALL {
        for enc in [Enc::Fixed, Enc::FixedLz] {
            for _ in 0..6 {
                let w = rng.range(1, 20) as usize;
                let h = rng.range(1, 20) as usize;
                let filters = rand_filters(&mut rng, h);
                let mut case = Case::new(&mut rng, ct, w, h, filters, enc);
                if ct == ColorType::Indexed {
                    case.spec.plte = Some(rng.bytes(256 * 3));
                }
                let label = format!("row36 {enc:?} ct={} {w}x{h}", ct as u8);
                assert!(case.check(&label), "C failed: {label}");
            }
        }
    }
}

#[test]
fn row37_png_dynamic_blocks() {
    let mut rng = Rng::new(SEED ^ 0x37);
    for ct in ColorType::ALL {
        for enc in [
            Enc::Dyn { depth: 15, hlit: 288, hdist: 2, rle: true },
            Enc::Dyn { depth: 15, hlit: 257, hdist: 32, rle: false },
            Enc::DynLz { depth: 15, rle: true },
            Enc::DynLz { depth: 9, rle: true },
        ] {
            for _ in 0..5 {
                let w = rng.range(1, 24) as usize;
                let h = rng.range(1, 24) as usize;
                let filters = rand_filters(&mut rng, h);
                let mut case = Case::new(&mut rng, ct, w, h, filters, enc);
                if ct == ColorType::Indexed {
                    case.spec.plte = Some(rng.bytes(256 * 3));
                }
                let label = format!("row37 {enc:?} ct={} {w}x{h}", ct as u8);
                assert!(case.check(&label), "C failed: {label}");
            }
        }
    }
}

#[test]
fn row39_large_images() {
    let mut rng = Rng::new(SEED ^ 0x39);
    for ct in ColorType::ALL {
        for enc in [Enc::FixedLz, Enc::DynLz { depth: 15, rle: true }] {
            let (w, h) = (64usize, 64usize);
            let filters = rand_filters(&mut rng, h);
            let mut case = Case::new(&mut rng, ct, w, h, filters, enc);
            if ct == ColorType::Indexed {
                case.spec.plte = Some(rng.bytes(256 * 3));
            }
            let label = format!("row39 64x64 {enc:?} ct={}", ct as u8);
            assert!(case.check(&label), "C failed: {label}");
        }
    }
    // Also a compressible image (long runs ⇒ long matches and dist==1 memsets).
    for ct in ColorType::ALL {
        let (w, h) = (64usize, 40usize);
        let bpp = ct.bpp();
        let stride = w * bpp;
        let mut raw = Vec::new();
        let mut filters = Vec::new();
        for y in 0..h {
            let f = (y % 5) as u8;
            filters.push(f);
            raw.push(f);
            let b = rng.u8();
            for x in 0..stride {
                raw.push(if x % 7 == 0 { rng.u8() } else { b });
            }
        }
        let def = encode(&raw, Enc::FixedLz);
        let mut spec = PngSpec::new(w as u32, h as u32, ct as u8, def, raw.clone());
        if ct == ColorType::Indexed {
            spec.plte = Some(rng.bytes(256 * 3));
        }
        let case = Case {
            ct,
            w,
            h,
            filters,
            raw,
            spec,
            model: true,
        };
        let label = format!("row39 compressible ct={}", ct as u8);
        assert!(case.check(&label), "C failed: {label}");
    }
}

#[test]
fn row37b_png_flate2_streams() {
    // An independent compressor as a cross-check. miniz may emit stored blocks
    // that this C rejects; whatever happens, both libraries must agree.
    let mut rng = Rng::new(SEED ^ 0x3B);
    let mut decoded = 0;
    for ct in ColorType::ALL {
        for level in [1u32, 6, 9] {
            let w = rng.range(4, 30) as usize;
            let h = rng.range(4, 30) as usize;
            let filters = rand_filters(&mut rng, h);
            let mut case = Case::new(&mut rng, ct, w, h, filters, Enc::Flate2(level));
            if ct == ColorType::Indexed {
                case.spec.plte = Some(rng.bytes(256 * 3));
            }
            if case.check(&format!("row37b flate2 L{level} ct={}", ct as u8)) {
                decoded += 1;
            }
        }
    }
    assert!(decoded > 0, "no flate2 stream decoded — cross-check vacuous");
    eprintln!("row37b: {decoded}/15 flate2 streams decoded successfully");
}

// ===========================================================================
// Rows 40-43 — mutated exported tables
// ===========================================================================

#[test]
fn row40_mutated_cp_fixed_table() {
    let mut rng = Rng::new(SEED ^ 0x40);
    for trial in 0..12 {
        // A random-but-valid complete code over 288 lit/len and 32 dist symbols,
        // written into the exported cp_fixed_table of BOTH libraries.
        let lf: Vec<u32> = (0..288).map(|_| rng.range(1, 40)).collect();
        let df: Vec<u32> = (0..32).map(|_| rng.range(1, 40)).collect();
        let ll = deflate::huff_lengths(&lf, 15);
        let dl = deflate::huff_lengths(&df, 15);
        let mut table = [0u8; 320];
        table[..288].copy_from_slice(&ll);
        table[288..].copy_from_slice(&dl);

        let dn = rng.range(1, 200) as usize;
        let data = rng.bytes(dn);
        let mut toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        toks.push(Tok::Match { len: 5, dist: 3 });
        let expect = deflate::expand(&toks);

        // Encode with the same (mutated) code.
        let lcodes = deflate::canonical(&ll);
        let dcodes = deflate::canonical(&dl);
        let mut bw = deflate::BitWriter::new();
        bw.bits(1, 1);
        bw.bits(1, 2);
        for t in &toks {
            match *t {
                Tok::Lit(b) => bw.huff(lcodes[b as usize], ll[b as usize] as u32),
                Tok::Match { len, dist } => {
                    let lc = 257 + deflate::len_code(len);
                    bw.huff(lcodes[lc], ll[lc] as u32);
                    let le = deflate::LEN_EXTRA[lc - 257];
                    if le > 0 {
                        bw.bits(len - deflate::LEN_BASE[lc - 257], le);
                    }
                    let dc = deflate::dist_code(dist);
                    bw.huff(dcodes[dc], dl[dc] as u32);
                    let de = deflate::DIST_EXTRA[dc];
                    if de > 0 {
                        bw.bits(dist - deflate::DIST_BASE[dc], de);
                    }
                }
            }
        }
        bw.huff(lcodes[256], ll[256] as u32);
        let def = bw.finish();

        let (c, r) = call_inflate_cfg(&def, def.len() as i32, expect.len() as i32, 0, move |lib| {
            unsafe {
                std::ptr::copy_nonoverlapping(table.as_ptr(), lib.cp_fixed_table, 320);
            }
        });
        assert_same(&format!("row40 mutated cp_fixed_table trial={trial}"), &c, &r);
        assert_eq!(c.ret, 1, "C failed: {}", c.err_str());
        assert_eq!(c.payload, expect);
    }
}

#[test]
fn row41_mutated_cp_permutation_order() {
    let mut rng = Rng::new(SEED ^ 0x41);
    for trial in 0..12 {
        // A shuffled permutation of 0..18 (staying in range keeps the C's
        // `lenlens[perm[i]]` write inside its stack array).
        let mut order = [0usize; 19];
        for i in 0..19 {
            order[i] = i;
        }
        for i in (1..19).rev() {
            let j = rng.below(i as u32 + 1) as usize;
            order.swap(i, j);
        }
        let mut table = [0u8; 19];
        for i in 0..19 {
            table[i] = order[i] as u8;
        }

        let data: Vec<u8> = (0..rng.range(4, 300)).map(|_| rng.below(50) as u8).collect();
        let toks: Vec<Tok> = data.iter().map(|&b| Tok::Lit(b)).collect();
        let (ll, dl) = deflate::tables_for(&toks, 15, 288, 2);
        let mut bw = deflate::BitWriter::new();
        deflate::dynamic_block_with_order(&mut bw, &ll, &dl, &toks, true, true, &order);
        let def = bw.finish();

        let (c, r) = call_inflate_cfg(
            &def,
            def.len() as i32,
            data.len() as i32,
            0,
            move |lib| unsafe {
                std::ptr::copy_nonoverlapping(table.as_ptr(), lib.cp_permutation_order, 19);
            },
        );
        assert_same(
            &format!("row41 mutated cp_permutation_order trial={trial} order={order:?}"),
            &c,
            &r,
        );
        assert_eq!(c.ret, 1, "C failed: {}", c.err_str());
        assert_eq!(c.payload, data);
    }
}

#[test]
fn row42_mutated_length_tables() {
    let mut rng = Rng::new(SEED ^ 0x42);
    for trial in 0..20 {
        // Small extra-bit counts and bases keep the stream decodable; the
        // decoded lengths differ from RFC 1951, which is exactly the point.
        let mut extra = [0u8; 31];
        let mut base = [0u32; 31];
        for i in 0..31 {
            extra[i] = rng.below(4) as u8;
            base[i] = rng.range(3, 40);
        }
        let history: Vec<u8> = (0..200u32).map(|i| (i % 251) as u8).collect();
        let mut toks: Vec<Tok> = history.iter().map(|&b| Tok::Lit(b)).collect();
        for _ in 0..5 {
            toks.push(Tok::Match {
                len: rng.range(3, 100),
                dist: rng.range(1, 150),
            });
        }
        let def = deflate::fixed_stream(&toks);
        // Output size is unknown a priori (the mutated tables change it), so use
        // a generous buffer; C and Rust must agree on the whole thing.
        let (c, r) = call_inflate_cfg(&def, def.len() as i32, 4096, 0, move |lib| unsafe {
            std::ptr::copy_nonoverlapping(extra.as_ptr(), lib.cp_len_extra_bits, 31);
            std::ptr::copy_nonoverlapping(base.as_ptr(), lib.cp_len_base, 31);
        });
        assert_same(&format!("row42 mutated len tables trial={trial}"), &c, &r);
    }
}

#[test]
fn row43_mutated_distance_tables() {
    let mut rng = Rng::new(SEED ^ 0x43);
    for trial in 0..20 {
        let mut extra = [0u8; 32];
        let mut base = [0u32; 32];
        for i in 0..32 {
            extra[i] = rng.below(4) as u8;
            base[i] = rng.range(1, 60);
        }
        let history: Vec<u8> = (0..300u32).map(|i| (i * 3 % 251) as u8).collect();
        let mut toks: Vec<Tok> = history.iter().map(|&b| Tok::Lit(b)).collect();
        for _ in 0..5 {
            toks.push(Tok::Match {
                len: rng.range(3, 60),
                dist: rng.range(1, 250),
            });
        }
        let def = deflate::fixed_stream(&toks);
        let (c, r) = call_inflate_cfg(&def, def.len() as i32, 4096, 0, move |lib| unsafe {
            std::ptr::copy_nonoverlapping(extra.as_ptr(), lib.cp_dist_extra_bits, 32);
            std::ptr::copy_nonoverlapping(base.as_ptr(), lib.cp_dist_base, 32);
        });
        assert_same(&format!("row43 mutated dist tables trial={trial}"), &c, &r);
    }
}

// ===========================================================================
// Row 45 — cp_error_reason state carry-over
// ===========================================================================

#[test]
fn row45_error_reason_carry_over() {
    // A *successful* call must not clear a reason left by a previous failure.
    let mut rng = Rng::new(SEED ^ 0x45);
    let w = 4usize;
    let h = 3usize;
    let filters = vec![0u8; h];
    let raw = png::raw_scanlines(&mut rng, w, h, 1, &filters);
    let def = deflate::stored_block(&raw, true);
    let good = PngSpec::new(w as u32, h as u32, 0, def, raw.clone()).build();
    let mut bad = good.clone();
    bad[1] = b'X'; // break the signature

    let (c, r) = run_pair(move |lib, shm| unsafe {
        let b = (lib.load_png_mem)(bad.as_ptr(), bad.len() as i32);
        assert!(b.pix.is_null());
        let g = (lib.load_png_mem)(good.as_ptr(), good.len() as i32);
        (*shm).w = g.w;
        (*shm).h = g.h;
        (*shm).pix_null = g.pix.is_null() as i32;
        (*shm).ret = if g.pix.is_null() { 0 } else { 1 };
        if !g.pix.is_null() {
            set_payload(shm, g.pix, (g.w as usize) * (g.h as usize) * 4);
        }
    });
    assert_same("row45 error reason carry-over", &c, &r);
    assert_eq!(
        c.err_str(),
        "incorrect file signature (is this a png file?)",
        "the stale reason should survive the successful call"
    );
    assert!(!c.pix_null);
}

// ===========================================================================
// Row 48 — filter byte sweep (valid 0..4 and invalid 5..255 in one sweep)
// ===========================================================================

#[test]
fn row48_filter_byte_full_sweep() {
    let mut rng = Rng::new(SEED ^ 0x48);
    for ct in [ColorType::Grey, ColorType::Rgb, ColorType::Rgba] {
        for row in [0usize, 1] {
            for f in 0..=255u8 {
                let w = 3usize;
                let h = 3usize;
                let mut filters = vec![0u8; h];
                filters[row] = f;
                let mut case = Case::new(&mut rng, ct, w, h, filters, Enc::Stored);
                case.model = f <= 4;
                let label = format!("row48 ct={} row={row} filter={f}", ct as u8);
                let ok = case.check(&label);
                if f <= 4 {
                    assert!(ok, "valid filter {f} rejected: {label}");
                } else {
                    assert!(!ok, "invalid filter {f} accepted: {label}");
                }
            }
        }
    }
}
