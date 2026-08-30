//! Phase B, rows 21-52 of `CONFIGS.md`: the `load_png_mem` wrapper.

mod harness;

use harness::make::*;
use harness::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Style {
    FixedLit,
    Dynamic,
    Stored,
    MultiFixed,
    MultiMixed,
}

const STYLES: [Style; 5] = [
    Style::FixedLit,
    Style::Dynamic,
    Style::Stored,
    Style::MultiFixed,
    Style::MultiMixed,
];

/// Encodes `raw` as a DEFLATE stream in the requested shape.
fn deflate_of(rng: &mut Rng, raw: &[u8], style: Style) -> Vec<u8> {
    let codes = Codes::fixed();
    let mut bw = BitW::new();
    match style {
        Style::FixedLit => {
            let toks: Vec<Tok> = raw.iter().map(|b| Tok::Lit(*b)).collect();
            block_fixed(&mut bw, true, &toks, &codes);
        }
        Style::Dynamic => {
            let toks: Vec<Tok> = raw.iter().map(|b| Tok::Lit(*b)).collect();
            let (lit, dst) = random_codes_for(rng, &toks);
            let enc = if rng.bool() {
                ClEncoding::Literal
            } else {
                ClEncoding::RunLength
            };
            block_dynamic(
                &mut bw, true, &toks, &lit, &dst, &PERMUTATION_ORDER, enc, rng,
            );
        }
        Style::Stored => {
            block_stored(&mut bw, true, raw);
        }
        Style::MultiFixed | Style::MultiMixed => {
            // split into 2..4 blocks
            let parts = (rng.range(2, 4) as usize).min(raw.len().max(1));
            let per = (raw.len() + parts - 1) / parts.max(1);
            let mut off = 0usize;
            let mut first = true;
            while off < raw.len() || first {
                let end = (off + per).min(raw.len());
                let last = end >= raw.len();
                let toks: Vec<Tok> = raw[off..end].iter().map(|b| Tok::Lit(*b)).collect();
                if style == Style::MultiFixed || rng.bool() {
                    block_fixed(&mut bw, last, &toks, &codes);
                } else {
                    let (lit, dst) = random_codes_for(rng, &toks);
                    block_dynamic(
                        &mut bw,
                        last,
                        &toks,
                        &lit,
                        &dst,
                        &PERMUTATION_ORDER,
                        ClEncoding::RunLength,
                        rng,
                    );
                }
                off = end;
                first = false;
            }
        }
    }
    bw.finish()
}

fn spec_for(
    rng: &mut Rng,
    w: usize,
    h: usize,
    ct: u8,
    filters: &[u8],
    style: Style,
) -> (PngSpec, Vec<u8>) {
    let bpp = bpp_of(ct);
    let raw = raw_scanlines(rng, w, h, bpp, filters);
    let d = deflate_of(rng, &raw, style);
    (PngSpec::new(w as u32, h as u32, ct, d), raw)
}

/// Rows 21-24, 29-30, 31-35, 44-47, 49: every colour type x every filter type x
/// several sizes x every DEFLATE shape.
#[test]
fn rows_21_35_44_49_colour_filter_size_matrix() {
    let pair = load_pair();
    let mut rng = Rng::new(0x21);
    let mut cases = Vec::new();

    for &ct in &[0u8, 2, 4, 6] {
        for &(w, h) in &[
            (1usize, 1usize),
            (1, 5),
            (5, 1),
            (2, 2),
            (3, 4),
            (7, 3),
            (16, 2),
            (33, 3),
            (2, 17),
        ] {
            // row 0 filter x rows>=1 filter, all combinations
            for f0 in 0..5u8 {
                for f1 in 0..5u8 {
                    let mut filters = vec![f0];
                    for _ in 1..h {
                        filters.push(f1);
                    }
                    let style = STYLES[(w + h + f0 as usize + f1 as usize) % STYLES.len()];
                    let (spec, _) = spec_for(&mut rng, w, h, ct, &filters, style);
                    cases.push(Case::png(
                        format!("ct={ct} {w}x{h} f0={f0} f1={f1} {style:?}"),
                        spec.build(),
                    ));
                }
            }
            // randomized per-row filters
            for _ in 0..3 {
                let filters: Vec<u8> = (0..h.max(1)).map(|_| rng.below(5) as u8).collect();
                let style = STYLES[rng.below(STYLES.len() as u32) as usize];
                let (spec, _) = spec_for(&mut rng, w, h, ct, &filters, style);
                cases.push(Case::png(
                    format!("ct={ct} {w}x{h} rndfilters {style:?}"),
                    spec.build(),
                ));
            }
        }
    }
    assert_same(&pair, &cases);
}

/// Rows 25-28: indexed (colour type 3) with PLTE and tRNS variants.
#[test]
fn rows_25_28_indexed() {
    let pair = load_pair();
    let mut rng = Rng::new(0x25);
    let mut cases = Vec::new();

    for &(w, h) in &[(1usize, 1usize), (4, 3), (9, 5), (1, 8), (16, 2)] {
        for &ncolors in &[1usize, 2, 5, 16, 255, 256] {
            let plte: Vec<u8> = rng.bytes(ncolors * 3);
            for trns_len in [None, Some(0usize), Some(1), Some(ncolors / 2), Some(ncolors)] {
                for f in 0..5u8 {
                    let filters: Vec<u8> = vec![f; h];
                    let (mut spec, _) = spec_for(&mut rng, w, h, 3, &filters, Style::FixedLit);
                    spec.plte = Some(plte.clone());
                    spec.trns = trns_len.map(|n| rng.bytes(n));
                    cases.push(Case::png(
                        format!("indexed {w}x{h} ncolors={ncolors} trns={trns_len:?} f={f}"),
                        spec.build(),
                    ));
                }
            }
        }
    }
    // row 28: indices deliberately past the end of a short palette
    for &ncolors in &[1usize, 2, 3, 7] {
        let plte: Vec<u8> = rng.bytes(ncolors * 3);
        let w = 8usize;
        let h = 3usize;
        // raw data is random bytes, so most indices are out of range
        let raw = raw_scanlines(&mut rng, w, h, 1, &[0]);
        let d = deflate_of(&mut rng, &raw, Style::FixedLit);
        let mut spec = PngSpec::new(w as u32, h as u32, 3, d);
        spec.plte = Some(plte);
        spec.trns = Some(rng.bytes(4));
        cases.push(Case::png(
            format!("indexed oob indices ncolors={ncolors}"),
            spec.build(),
        ));
    }
    assert_same(&pair, &cases);
}

/// Rows 36-42: chunk-level shapes -- split/empty IDATs, ancillary chunks,
/// PLTE/tRNS on non-indexed images, tRNS before PLTE, oversized IHDR.
#[test]
fn rows_36_42_chunk_layout() {
    let pair = load_pair();
    let mut rng = Rng::new(0x36);
    let mut cases = Vec::new();

    for &ct in &[0u8, 2, 3, 4, 6] {
        let bpp = bpp_of(ct);
        let (w, h) = (6usize, 4usize);
        let filters = vec![0u8, 1, 2, 3];
        for parts in 1..6usize {
            for empty in [false, true] {
                let raw = raw_scanlines(&mut rng, w, h, bpp, &filters);
                let d = deflate_of(&mut rng, &raw, Style::FixedLit);
                let mut spec = PngSpec::new(w as u32, h as u32, ct, d);
                spec.idat_parts = parts;
                spec.empty_idat = empty;
                if ct == 3 {
                    spec.plte = Some(rng.bytes(256 * 3));
                }
                cases.push(Case::png(
                    format!("ct={ct} idat parts={parts} empty={empty}"),
                    spec.build(),
                ));
            }
        }
        // ancillary chunks in every position
        for pos in 0..3usize {
            let raw = raw_scanlines(&mut rng, w, h, bpp, &filters);
            let d = deflate_of(&mut rng, &raw, Style::FixedLit);
            let mut spec = PngSpec::new(w as u32, h as u32, ct, d);
            if ct == 3 {
                spec.plte = Some(rng.bytes(48));
            }
            let anc = ancillary(&mut rng);
            match pos {
                0 => spec.before_plte = anc,
                1 => spec.before_idat = anc,
                _ => spec.after_idat = anc,
            }
            cases.push(Case::png(
                format!("ct={ct} ancillary pos={pos}"),
                spec.build(),
            ));
        }
        // PLTE / tRNS present on non-indexed images, and tRNS before PLTE
        for (with_plte, with_trns, trns_first) in [
            (true, false, false),
            (false, true, false),
            (true, true, false),
            (true, true, true),
        ] {
            let raw = raw_scanlines(&mut rng, w, h, bpp, &filters);
            let d = deflate_of(&mut rng, &raw, Style::FixedLit);
            let mut spec = PngSpec::new(w as u32, h as u32, ct, d);
            if with_plte || ct == 3 {
                spec.plte = Some(rng.bytes(96));
            }
            if with_trns {
                spec.trns = Some(rng.bytes(5));
            }
            spec.trns_first = trns_first;
            cases.push(Case::png(
                format!("ct={ct} plte={with_plte} trns={with_trns} trns_first={trns_first}"),
                spec.build(),
            ));
        }
        // row 42: IHDR longer than 13 bytes
        for extra in 1..5usize {
            let raw = raw_scanlines(&mut rng, w, h, bpp, &filters);
            let d = deflate_of(&mut rng, &raw, Style::FixedLit);
            let mut spec = PngSpec::new(w as u32, h as u32, ct, d);
            spec.ihdr_extra = extra;
            if ct == 3 {
                spec.plte = Some(rng.bytes(48));
            }
            cases.push(Case::png(
                format!("ct={ct} ihdr+{extra}"),
                spec.build(),
            ));
        }
    }
    assert_same(&pair, &cases);
}

/// Row 43: every accepted zlib header byte pair.
#[test]
fn row_43_zlib_headers() {
    let pair = load_pair();
    let mut rng = Rng::new(0x43);
    let mut cases = Vec::new();
    let (w, h) = (4usize, 3usize);
    for cinfo in 0..8u8 {
        for &flg in &[0x00u8, 0x01, 0x1F, 0x40, 0x80, 0xC0, 0xDF] {
            let raw = raw_scanlines(&mut rng, w, h, 4, &[0]);
            let d = deflate_of(&mut rng, &raw, Style::FixedLit);
            let mut spec = PngSpec::new(w as u32, h as u32, 6, d);
            spec.cmf = 0x08 | (cinfo << 4);
            spec.flg = flg;
            cases.push(Case::png(
                format!("zlib cmf={:#02x} flg={flg:#02x}", spec.cmf),
                spec.build(),
            ));
        }
    }
    assert_same(&pair, &cases);
}

/// Row 48: the DEFLATE stream ends well before `out_end` (which is
/// `img.pix + (w+1)*h*4`, i.e. past the end of the pixel buffer), and streams
/// that write extra bytes into the slack between the raw block and `out_end`.
#[test]
fn row_48_output_slack() {
    let pair = load_pair();
    let mut rng = Rng::new(0x48);
    let mut cases = Vec::new();
    for &ct in &[0u8, 2, 4, 6] {
        let bpp = bpp_of(ct);
        for &(w, h) in &[(3usize, 3usize), (8, 2), (1, 9)] {
            let filters = vec![0u8; h];
            let raw = raw_scanlines(&mut rng, w, h, bpp, &filters);
            // exactly the required bytes
            let d = deflate_of(&mut rng, &raw, Style::FixedLit);
            cases.push(Case::png(
                format!("exact ct={ct} {w}x{h}"),
                PngSpec::new(w as u32, h as u32, ct, d).build(),
            ));
            // `out_end` is `img.pix + (w+1)*h*4`, which is `h*(bpp-1)` bytes
            // *past* the end of the `img.pix` allocation once the `out` offset
            // is accounted for, so writing more than `h*(bpp-1)` extra bytes is
            // a heap overflow (undefined in the C, and whether glibc notices
            // depends on the heap layout, not on the translation). Stay inside.
            let slack = h * (bpp - 1);
            for extra in 1..=slack {
                let mut raw2 = raw.clone();
                raw2.extend(rng.bytes(extra));
                let d = deflate_of(&mut rng, &raw2, Style::FixedLit);
                cases.push(Case::png(
                    format!("slack+{extra} ct={ct} {w}x{h}"),
                    PngSpec::new(w as u32, h as u32, ct, d).build(),
                ));
            }
        }
    }
    assert_same(&pair, &cases);
}

/// Rows 50, 52: repeated / interleaved calls carry no hidden state, and
/// `cp_error_reason` keeps its previous value across a successful call.
#[test]
fn rows_50_52_no_hidden_state() {
    let pair = load_pair();
    let mut rng = Rng::new(0x50);
    let (w, h) = (5usize, 4usize);
    let raw_a = raw_scanlines(&mut rng, w, h, 4, &[0, 1, 2, 3]);
    let good = PngSpec::new(w as u32, h as u32, 6, deflate_of(&mut rng, &raw_a, Style::FixedLit))
        .build();
    let mut bad = good.clone();
    bad[0] = 0;

    let mut cases = Vec::new();
    for i in 0..12 {
        cases.push(Case::png(
            format!("alt {i}"),
            if i % 3 == 2 { bad.clone() } else { good.clone() },
        ));
    }
    let out = run_same(&pair, &cases);
    // every "good" call must succeed and leave cp_error_reason untouched
    for (i, o) in out.iter().enumerate() {
        match o {
            Outcome::Ret(v) => {
                if i % 3 == 2 {
                    assert_eq!(v[8], 0, "case {i} should fail");
                } else {
                    assert_eq!(v[8], 1, "case {i} should succeed");
                    assert!(
                        String::from_utf8_lossy(v).ends_with("<null>"),
                        "case {i}: cp_error_reason was written on success"
                    );
                }
            }
            o => panic!("case {i}: {o:?}"),
        }
    }
    // and repeats are identical
    assert_eq!(out[0], out[3]);
    assert_eq!(out[2], out[5]);
}

/// Row 51: table mutations applied through the PNG wrapper.
#[test]
fn row_51_table_mutations_via_png() {
    let pair = load_pair();
    let mut rng = Rng::new(0x51);
    let mut cases = Vec::new();
    let (w, h) = (6usize, 3usize);

    for i in 0..200 {
        let raw = raw_scanlines(&mut rng, w, h, 4, &[0, 1, 4]);
        let style = if i % 2 == 0 {
            Style::FixedLit
        } else {
            Style::Dynamic
        };
        let d = deflate_of(&mut rng, &raw, style);
        let spec = PngSpec::new(w as u32, h as u32, 6, d);
        let table = Table::ALL[(i % 6) as usize];
        let mut off = rng.below(table.byte_len() as u32) as usize;
        if matches!(table, Table::LenBase | Table::DistBase) {
            off &= !3; // see phase_b_inflate: a negative `length` is untestable
        }
        let val = match table {
            Table::LenExtraBits | Table::DistExtraBits => rng.below(6) as u8,
            Table::FixedTable => rng.below(16) as u8,
            Table::PermutationOrder => rng.below(19) as u8,
            _ => rng.byte(),
        };
        cases.push(
            Case::png(format!("png mutate {table:?}[{off}]={val}"), spec.build())
                .with_mutations(vec![Mutation { table, off, val }]),
        );
    }
    assert_same(&pair, &cases);
}
