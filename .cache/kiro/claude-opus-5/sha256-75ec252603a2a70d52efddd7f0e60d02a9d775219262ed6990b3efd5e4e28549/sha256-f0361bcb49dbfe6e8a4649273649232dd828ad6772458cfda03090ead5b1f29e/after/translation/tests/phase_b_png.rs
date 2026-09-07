//! Phase B — valid-path differential tests for `load_png_mem`.
//! Covers CONFIGS.md rows 22..49.

mod common;

use common::deflate::*;
use common::*;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::os::raw::c_int;

/// The pixels a correct consumer should see, computed independently of both
/// implementations so the differential tests are not vacuous.
fn expected_pixels(
    w: usize,
    h: usize,
    color_type: u8,
    data: &[u8],
    plte: Option<&[u8]>,
    trns: Option<&[u8]>,
) -> Vec<CpPixel> {
    let bpp = bpp_for(color_type);
    let mut out = Vec::with_capacity(w * h);
    for y in 0..h {
        for x in 0..w {
            let s = &data[(y * w + x) * bpp..];
            let p = match color_type {
                0 => CpPixel { r: s[0], g: s[0], b: s[0], a: 0xFF },
                2 => CpPixel { r: s[0], g: s[1], b: s[2], a: 0xFF },
                4 => CpPixel { r: s[0], g: s[0], b: s[0], a: s[1] },
                6 => CpPixel { r: s[0], g: s[1], b: s[2], a: s[3] },
                3 => {
                    let c = s[0] as usize;
                    let pl = plte.unwrap();
                    let a = match trns {
                        Some(t) if c < t.len() => t[c],
                        _ => 255,
                    };
                    CpPixel { r: pl[c * 3], g: pl[c * 3 + 1], b: pl[c * 3 + 2], a }
                }
                _ => unreachable!(),
            };
            out.push(p);
        }
    }
    out
}

/// Asserts (a) the C decodes the generated PNG to `expect`, and (b) the Rust
/// `.so` agrees with the C `.so` byte for byte.
fn ok_png(label: &str, png: &[u8], w: usize, h: usize, expect: &[CpPixel]) {
    let r = c_png(png, png.len() as c_int);
    assert!(
        !r.null,
        "[{label}] C rejected our generated PNG: {:?}",
        r.error
    );
    assert_eq!((r.w, r.h), (w as c_int, h as c_int), "[{label}] dims");
    if r.pixels != expect {
        let i = r.pixels.iter().zip(expect).position(|(a, b)| a != b).unwrap();
        panic!(
            "[{label}] C decoded wrong at pixel {i} (x={},y={}): got {:?} want {:?}",
            i % w,
            i / w,
            r.pixels[i],
            expect[i]
        );
    }
    diff_png(label, png);
}

fn build(
    w: usize,
    h: usize,
    color_type: u8,
    data: &[u8],
    filters: &[u8],
    mode: DeflateMode,
) -> PngSpec {
    let bpp = bpp_for(color_type);
    let mut spec = PngSpec::new(w as u32, h as u32, color_type);
    spec.raw = encode_scanlines(w, h, bpp, data, filters);
    spec.deflate = mode;
    spec
}

fn rnd_data(rng: &mut StdRng, n: usize) -> Vec<u8> {
    (0..n).map(|_| rng.gen()).collect()
}

const ALL_MODES: &[DeflateMode] = &[
    DeflateMode::Stored,
    DeflateMode::FixedLiterals,
    DeflateMode::FixedLz,
    DeflateMode::DynamicLiteralCl,
    DeflateMode::DynamicRle,
    DeflateMode::DynamicLz,
];

// --- rows 22..26: every colour type ---------------------------------------

#[test]
fn row22_greyscale_1x1_filter0() {
    let spec = build(1, 1, 0, &[0x7F], &[0], DeflateMode::FixedLiterals);
    let png = spec.build();
    let exp = expected_pixels(1, 1, 0, &[0x7F], None, None);
    ok_png("row22 grey 1x1", &png, 1, 1, &exp);
}

#[test]
fn row23_greyscale_mixed_filters() {
    let mut rng = StdRng::seed_from_u64(23);
    for (w, h) in [(1usize, 1usize), (5, 5), (7, 3), (16, 9), (33, 2)] {
        let data = rnd_data(&mut rng, w * h);
        let filters: Vec<u8> = (0..h).map(|y| (y % 5) as u8).collect();
        let exp = expected_pixels(w, h, 0, &data, None, None);
        for &m in ALL_MODES {
            let png = build(w, h, 0, &data, &filters, m).build();
            ok_png(&format!("row23 grey {w}x{h} {m:?}"), &png, w, h, &exp);
        }
    }
}

#[test]
fn row24_rgb_mixed_filters() {
    let mut rng = StdRng::seed_from_u64(24);
    for (w, h) in [(1usize, 1usize), (4, 4), (5, 7), (13, 3)] {
        let data = rnd_data(&mut rng, w * h * 3);
        let filters: Vec<u8> = (0..h).map(|y| ((y * 3) % 5) as u8).collect();
        let exp = expected_pixels(w, h, 2, &data, None, None);
        for &m in ALL_MODES {
            let png = build(w, h, 2, &data, &filters, m).build();
            ok_png(&format!("row24 rgb {w}x{h} {m:?}"), &png, w, h, &exp);
        }
    }
}

#[test]
fn row25_grey_alpha_mixed_filters() {
    let mut rng = StdRng::seed_from_u64(25);
    for (w, h) in [(1usize, 1usize), (3, 6), (8, 8), (17, 2)] {
        let data = rnd_data(&mut rng, w * h * 2);
        let filters: Vec<u8> = (0..h).map(|y| ((y * 2) % 5) as u8).collect();
        let exp = expected_pixels(w, h, 4, &data, None, None);
        for &m in ALL_MODES {
            let png = build(w, h, 4, &data, &filters, m).build();
            ok_png(&format!("row25 grey+a {w}x{h} {m:?}"), &png, w, h, &exp);
        }
    }
}

#[test]
fn row26_rgba_mixed_filters() {
    let mut rng = StdRng::seed_from_u64(26);
    for (w, h) in [(1usize, 1usize), (2, 9), (6, 6), (11, 4)] {
        let data = rnd_data(&mut rng, w * h * 4);
        let filters: Vec<u8> = (0..h).map(|y| ((y * 4) % 5) as u8).collect();
        let exp = expected_pixels(w, h, 6, &data, None, None);
        for &m in ALL_MODES {
            let png = build(w, h, 6, &data, &filters, m).build();
            ok_png(&format!("row26 rgba {w}x{h} {m:?}"), &png, w, h, &exp);
        }
    }
}

// --- rows 27..30: indexed / palette ---------------------------------------

fn palette(n: usize, seed: u64) -> Vec<u8> {
    let mut rng = StdRng::seed_from_u64(seed);
    (0..n * 3).map(|_| rng.gen()).collect()
}

#[test]
fn row27_indexed_no_trns() {
    let (w, h) = (9usize, 5usize);
    let mut rng = StdRng::seed_from_u64(27);
    let data: Vec<u8> = (0..w * h).map(|_| rng.gen_range(0..16u8)).collect();
    let plte = palette(256, 271);
    let filters: Vec<u8> = (0..h).map(|y| (y % 5) as u8).collect();
    let exp = expected_pixels(w, h, 3, &data, Some(&plte), None);
    for &m in ALL_MODES {
        let mut spec = build(w, h, 3, &data, &filters, m);
        spec.plte = Some(plte.clone());
        let png = spec.build();
        ok_png(&format!("row27 indexed {m:?}"), &png, w, h, &exp);
    }
}

#[test]
fn row28_indexed_trns_covers_all_indices() {
    let (w, h) = (8usize, 4usize);
    let mut rng = StdRng::seed_from_u64(28);
    let data: Vec<u8> = (0..w * h).map(|_| rng.gen_range(0..32u8)).collect();
    let plte = palette(256, 281);
    let trns: Vec<u8> = (0..256u32).map(|i| (i * 7) as u8).collect();
    let filters: Vec<u8> = (0..h).map(|y| (y % 5) as u8).collect();
    let exp = expected_pixels(w, h, 3, &data, Some(&plte), Some(&trns));
    for &m in ALL_MODES {
        let mut spec = build(w, h, 3, &data, &filters, m);
        spec.plte = Some(plte.clone());
        spec.trns = Some(trns.clone());
        let png = spec.build();
        ok_png(&format!("row28 indexed+trns {m:?}"), &png, w, h, &exp);
    }
}

#[test]
fn row29_indexed_trns_shorter_than_max_index() {
    // indices >= trns_len must fall back to alpha 255
    let (w, h) = (10usize, 3usize);
    let mut rng = StdRng::seed_from_u64(29);
    let data: Vec<u8> = (0..w * h).map(|_| rng.gen_range(0..40u8)).collect();
    let plte = palette(256, 291);
    for trns_len in [0usize, 1, 5, 20, 39, 40, 41] {
        let trns: Vec<u8> = (0..trns_len).map(|i| (i * 11 + 1) as u8).collect();
        let filters: Vec<u8> = (0..h).map(|y| (y % 5) as u8).collect();
        let exp = expected_pixels(w, h, 3, &data, Some(&plte), Some(&trns));
        let mut spec = build(w, h, 3, &data, &filters, DeflateMode::FixedLz);
        spec.plte = Some(plte.clone());
        spec.trns = Some(trns.clone());
        let png = spec.build();
        ok_png(&format!("row29 trns_len={trns_len}"), &png, w, h, &exp);
    }
}

#[test]
fn row30_indexed_short_palette() {
    // The C reads plte[c*3..c*3+2] with no bound on c: a short PLTE makes it
    // read the bytes that follow the chunk. Both libraries get an identical
    // buffer, so they must agree on whatever that is. Only the differential
    // check applies here (no independent expectation).
    let (w, h) = (12usize, 4usize);
    let mut rng = StdRng::seed_from_u64(30);
    let data: Vec<u8> = (0..w * h).map(|_| rng.gen()).collect();
    for pl_entries in [1usize, 2, 8, 64, 128, 255] {
        let plte = palette(pl_entries, 301);
        let filters: Vec<u8> = (0..h).map(|y| (y % 5) as u8).collect();
        let mut spec = build(w, h, 3, &data, &filters, DeflateMode::FixedLz);
        spec.plte = Some(plte);
        let png = spec.build();
        // C must still accept it
        assert!(
            !c_png(&png, png.len() as c_int).null,
            "C rejected short-PLTE image"
        );
        diff_png(&format!("row30 short plte n={pl_entries}"), &png);
    }
}

// --- rows 31..36: every filter, row 0 and rows >= 1 -----------------------

#[test]
fn rows31_34_row0_filters() {
    let mut rng = StdRng::seed_from_u64(3134);
    for f in 0..5u8 {
        for ct in [0u8, 2, 3, 4, 6] {
            let bpp = bpp_for(ct);
            for w in [1usize, 2, 3, 5, 9] {
                let h = 1; // row 0 only
                let data = rnd_data(&mut rng, w * h * bpp);
                let plte = palette(256, 314);
                let exp = expected_pixels(
                    w,
                    h,
                    ct,
                    &data,
                    if ct == 3 { Some(&plte) } else { None },
                    None,
                );
                let mut spec = build(w, h, ct, &data, &[f], DeflateMode::FixedLz);
                if ct == 3 {
                    spec.plte = Some(plte.clone());
                }
                let png = spec.build();
                ok_png(&format!("row31-34 f{f} ct{ct} w{w}"), &png, w, h, &exp);
            }
        }
    }
}

#[test]
fn rows35_36_later_row_filters() {
    let mut rng = StdRng::seed_from_u64(3536);
    for f in 0..5u8 {
        for ct in [0u8, 2, 4, 6] {
            let bpp = bpp_for(ct);
            for (w, h) in [(1usize, 2usize), (3, 4), (5, 8), (9, 3)] {
                let data = rnd_data(&mut rng, w * h * bpp);
                // row 0 = filter 0, all later rows = f
                let mut filters = vec![f; h];
                filters[0] = 0;
                let exp = expected_pixels(w, h, ct, &data, None, None);
                let png = build(w, h, ct, &data, &filters, DeflateMode::FixedLz).build();
                ok_png(&format!("row35-36 f{f} ct{ct} {w}x{h}"), &png, w, h, &exp);
            }
        }
    }
}

#[test]
fn row36_paeth_all_predictor_branches() {
    // Drive paeth so that each of the three outcomes (a, b, c) is selected.
    let (w, h) = (4usize, 4usize);
    let ct = 2u8; // bpp 3, so x-bpp neighbours exist
    let mut cases: Vec<Vec<u8>> = Vec::new();
    // hand-picked patterns that make p-a, p-b, p-c minimal in turn
    cases.push(vec![0u8; w * h * 3]);
    cases.push((0..w * h * 3).map(|i| (i * 17) as u8).collect());
    cases.push((0..w * h * 3).map(|i| if i % 3 == 0 { 255 } else { 0 }).collect());
    cases.push((0..w * h * 3).map(|i| (255 - (i % 256)) as u8).collect());
    let mut rng = StdRng::seed_from_u64(360);
    for _ in 0..40 {
        cases.push((0..w * h * 3).map(|_| rng.gen_range(0..4u8)).collect());
    }
    for (i, data) in cases.iter().enumerate() {
        let filters = vec![4u8; h];
        let exp = expected_pixels(w, h, ct, data, None, None);
        let png = build(w, h, ct, data, &filters, DeflateMode::FixedLz).build();
        ok_png(&format!("row36 paeth case={i}"), &png, w, h, &exp);
    }
}

// --- rows 37..40: shapes ---------------------------------------------------

#[test]
fn rows37_40_shapes() {
    let mut rng = StdRng::seed_from_u64(3740);
    let shapes = [
        (1usize, 1usize),
        (1, 2),
        (1, 17),
        (2, 1),
        (17, 1),
        (3, 3),
        (5, 1),
        (7, 7),
        (4, 4),
        (13, 11),
        (33, 1),
        (1, 33),
    ];
    for (w, h) in shapes {
        for ct in [0u8, 2, 4, 6] {
            let bpp = bpp_for(ct);
            let data = rnd_data(&mut rng, w * h * bpp);
            let filters: Vec<u8> = (0..h).map(|y| (y % 5) as u8).collect();
            let exp = expected_pixels(w, h, ct, &data, None, None);
            let png = build(w, h, ct, &data, &filters, DeflateMode::FixedLz).build();
            ok_png(&format!("rows37-40 {w}x{h} ct{ct} (w*bpp={})", w * bpp), &png, w, h, &exp);
        }
    }
}

// --- rows 41..46: container variations ------------------------------------

#[test]
fn row41_idat_split_across_chunks() {
    let (w, h) = (11usize, 7usize);
    let mut rng = StdRng::seed_from_u64(41);
    let data = rnd_data(&mut rng, w * h * 4);
    let filters: Vec<u8> = (0..h).map(|y| (y % 5) as u8).collect();
    let exp = expected_pixels(w, h, 6, &data, None, None);
    for parts in 1..=6usize {
        let mut spec = build(w, h, 6, &data, &filters, DeflateMode::FixedLz);
        spec.idat_parts = parts;
        let png = spec.build();
        ok_png(&format!("row41 idat_parts={parts}"), &png, w, h, &exp);
    }
}

#[test]
fn row42_ancillary_chunk_before_idat() {
    let (w, h) = (6usize, 3usize);
    let mut rng = StdRng::seed_from_u64(42);
    let data = rnd_data(&mut rng, w * h * 3);
    let filters: Vec<u8> = (0..h).map(|y| (y % 5) as u8).collect();
    let exp = expected_pixels(w, h, 2, &data, None, None);
    let mut spec = build(w, h, 2, &data, &filters, DeflateMode::FixedLz);
    spec.pre_idat_chunks = vec![
        (*b"gAMA", vec![0, 1, 0x86, 0xA0]),
        (*b"cHRM", vec![0u8; 32]),
        (*b"sRGB", vec![0]),
    ];
    let png = spec.build();
    ok_png("row42 ancillary before idat", &png, w, h, &exp);
}

#[test]
fn row43_zlib_cinfo_sweep() {
    let (w, h) = (5usize, 4usize);
    let mut rng = StdRng::seed_from_u64(43);
    let data = rnd_data(&mut rng, w * h);
    let filters = vec![0u8; h];
    let exp = expected_pixels(w, h, 0, &data, None, None);
    for cinfo in 0..=7u8 {
        let mut spec = build(w, h, 0, &data, &filters, DeflateMode::FixedLz);
        spec.cmf = (cinfo << 4) | 0x08;
        let png = spec.build();
        ok_png(&format!("row43 cinfo={cinfo}"), &png, w, h, &exp);
    }
}

#[test]
fn row44_zlib_flg_ignored_bits() {
    let (w, h) = (5usize, 4usize);
    let mut rng = StdRng::seed_from_u64(44);
    let data = rnd_data(&mut rng, w * h * 4);
    let filters = vec![0u8; h];
    let exp = expected_pixels(w, h, 6, &data, None, None);
    for flg in [0x00u8, 0x01, 0x1F, 0x40, 0x80, 0x9C, 0xDA, 0xDF] {
        // bit 0x20 (FDICT) is rejected; every other bit must be ignored
        assert_eq!(flg & 0x20, 0);
        let mut spec = build(w, h, 6, &data, &filters, DeflateMode::FixedLz);
        spec.flg = flg;
        let png = spec.build();
        ok_png(&format!("row44 flg={flg:#02x}"), &png, w, h, &exp);
    }
}

#[test]
fn row45_all_deflate_block_types() {
    let (w, h) = (7usize, 5usize);
    let mut rng = StdRng::seed_from_u64(45);
    for ct in [0u8, 2, 4, 6] {
        let bpp = bpp_for(ct);
        let data = rnd_data(&mut rng, w * h * bpp);
        let filters: Vec<u8> = (0..h).map(|y| (y % 5) as u8).collect();
        let exp = expected_pixels(w, h, ct, &data, None, None);
        for &m in ALL_MODES {
            let png = build(w, h, ct, &data, &filters, m).build();
            ok_png(&format!("row45 ct{ct} {m:?}"), &png, w, h, &exp);
        }
    }
}

#[test]
fn row46_iend_present_or_absent() {
    let (w, h) = (4usize, 4usize);
    let mut rng = StdRng::seed_from_u64(46);
    let data = rnd_data(&mut rng, w * h * 4);
    let filters = vec![0u8; h];
    let exp = expected_pixels(w, h, 6, &data, None, None);
    for iend in [true, false] {
        let mut spec = build(w, h, 6, &data, &filters, DeflateMode::FixedLz);
        spec.iend = iend;
        let png = spec.build();
        ok_png(&format!("row46 iend={iend}"), &png, w, h, &exp);
    }
}

// --- rows 47..49: randomized and large -----------------------------------

#[test]
fn row47_randomized_images() {
    let mut rng = StdRng::seed_from_u64(0x5A17_9E9E_D00D);
    let cts = [0u8, 2, 3, 4, 6];
    for case in 0..420 {
        let w = rng.gen_range(1..25usize);
        let h = rng.gen_range(1..25usize);
        let ct = cts[rng.gen_range(0..cts.len())];
        let bpp = bpp_for(ct);
        let data: Vec<u8> = (0..w * h * bpp)
            .map(|_| if case % 3 == 0 { rng.gen_range(0..8u8) } else { rng.gen() })
            .collect();
        let filters: Vec<u8> = (0..h).map(|_| rng.gen_range(0..5u8)).collect();
        let plte = palette(256, 470 + case as u64);
        let mode = ALL_MODES[rng.gen_range(0..ALL_MODES.len())];
        let mut spec = build(w, h, ct, &data, &filters, mode);
        spec.idat_parts = rng.gen_range(1..4);
        if ct == 3 {
            spec.plte = Some(plte.clone());
        }
        let png = spec.build();
        let exp = expected_pixels(w, h, ct, &data, if ct == 3 { Some(&plte) } else { None }, None);
        ok_png(
            &format!("row47 case={case} {w}x{h} ct{ct} {mode:?}"),
            &png,
            w,
            h,
            &exp,
        );
    }
}

#[test]
fn row48_randomized_indexed_with_palette_and_trns() {
    let mut rng = StdRng::seed_from_u64(0x1DEA_4811);
    for case in 0..200 {
        let w = rng.gen_range(1..20usize);
        let h = rng.gen_range(1..20usize);
        let pl_entries = rng.gen_range(1..=256usize);
        let plte = palette(pl_entries, 4800 + case as u64);
        let trns_len = rng.gen_range(0..=pl_entries);
        let trns: Vec<u8> = (0..trns_len).map(|_| rng.gen()).collect();
        // keep indices inside the palette so an independent expectation exists
        let data: Vec<u8> = (0..w * h)
            .map(|_| rng.gen_range(0..pl_entries) as u8)
            .collect();
        let filters: Vec<u8> = (0..h).map(|_| rng.gen_range(0..5u8)).collect();
        // filtering can move indices outside the palette, so only filter 0 rows
        // are used when the palette is short
        let filters = if pl_entries < 256 { vec![0u8; h] } else { filters };
        let mode = ALL_MODES[rng.gen_range(0..ALL_MODES.len())];
        let mut spec = build(w, h, 3, &data, &filters, mode);
        spec.plte = Some(plte.clone());
        if rng.gen_bool(0.7) {
            spec.trns = Some(trns.clone());
        }
        let png = spec.build();
        let exp = expected_pixels(
            w,
            h,
            3,
            &data,
            Some(&plte),
            spec.trns.as_deref(),
        );
        ok_png(
            &format!("row48 case={case} {w}x{h} plte={pl_entries} trns={trns_len}"),
            &png,
            w,
            h,
            &exp,
        );
    }
}

#[test]
fn row49_large_image() {
    let (w, h) = (256usize, 256usize);
    let mut rng = StdRng::seed_from_u64(49);
    // low-entropy content so the LZ77 encoder produces long matches
    let mut data = vec![0u8; w * h * 4];
    for i in 0..data.len() {
        data[i] = if i % 97 == 0 { rng.gen() } else { (i / 4 % 251) as u8 };
    }
    let filters: Vec<u8> = (0..h).map(|y| (y % 5) as u8).collect();
    let exp = expected_pixels(w, h, 6, &data, None, None);
    for &m in &[DeflateMode::FixedLz, DeflateMode::DynamicLz, DeflateMode::FixedLiterals] {
        let mut spec = build(w, h, 6, &data, &filters, m);
        spec.idat_parts = 3;
        let png = spec.build();
        ok_png(&format!("row49 large {m:?}"), &png, w, h, &exp);
    }
    // a wide, single-row image and a tall, single-column one
    for (w2, h2) in [(4096usize, 1usize), (1usize, 4096usize)] {
        let d = rnd_data(&mut rng, w2 * h2 * 3);
        let f: Vec<u8> = (0..h2).map(|y| (y % 5) as u8).collect();
        let e = expected_pixels(w2, h2, 2, &d, None, None);
        let png = build(w2, h2, 2, &d, &f, DeflateMode::FixedLz).build();
        ok_png(&format!("row49 {w2}x{h2}"), &png, w2, h2, &e);
    }
}
