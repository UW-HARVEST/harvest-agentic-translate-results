//! Phase B -- valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both libraries are loaded with `dlopen`/`dlsym`; nothing is linked directly.

mod common;

use common::*;
use flate2::Compression;
use flate2::write::DeflateEncoder;
use std::io::Write;

fn both() -> (Lib, Lib) {
    (Lib::open(&c_lib()), Lib::open(&rust_lib()))
}

/// Compares one PNG buffer through both libraries.
#[track_caller]
fn cmp_png(c: &Lib, r: &Lib, label: &str, data: &[u8]) {
    let a = call_load(c, data);
    let b = call_load(r, data);
    if a != b {
        panic!(
            "PNG divergence [{label}]\n  C    = w={} h={} pix={:?} err={}\n  RUST = w={} h={} pix={:?} err={}\n  first diff: {:?}",
            a.w,
            a.h,
            a.pix.as_ref().map(|v| v.len()),
            a.err,
            b.w,
            b.h,
            b.pix.as_ref().map(|v| v.len()),
            b.err,
            first_diff(&a, &b)
        );
    }
}

fn first_diff(a: &PngOut, b: &PngOut) -> Option<(usize, u8, u8)> {
    match (&a.pix, &b.pix) {
        (Some(x), Some(y)) => x
            .iter()
            .zip(y.iter())
            .enumerate()
            .find(|(_, (p, q))| p != q)
            .map(|(i, (p, q))| (i, *p, *q)),
        _ => None,
    }
}

#[track_caller]
fn cmp_inflate(c: &Lib, r: &Lib, label: &str, data: &[u8], align: usize, out_bytes: i32) {
    let a = call_inflate(c, data, align, out_bytes);
    let b = call_inflate(r, data, align, out_bytes);
    assert!(
        a == b,
        "cp_inflate divergence [{label}] align={align} out={out_bytes}\n  C    ret={} err={}\n  RUST ret={} err={}\n  first byte diff at {:?}",
        a.ret,
        a.err,
        b.ret,
        b.err,
        a.out
            .iter()
            .zip(b.out.iter())
            .position(|(x, y)| x != y)
    );
}

const SHAPES: &[(u32, u32)] = &[(1, 1), (1, 7), (7, 1), (5, 5), (16, 9), (63, 2), (3, 40)];

fn deflate_fixed_literals(raw: &[u8]) -> Vec<u8> {
    let mut bw = BitWriter::new();
    let items: Vec<Item> = raw.iter().map(|&b| Item::Lit(b)).collect();
    emit_fixed(&mut bw, &items, true);
    bw.finish()
}

fn deflate_real(raw: &[u8], level: u32) -> Vec<u8> {
    let mut e = DeflateEncoder::new(Vec::new(), Compression::new(level));
    e.write_all(raw).unwrap();
    e.finish().unwrap()
}

/// Builds a colour-type-`ct` PNG with the given per-row filter bytes.
fn png_ct(rng: &mut Rng, ct: u8, w: u32, h: u32, filters: &[u8], deflate: Option<Vec<u8>>) -> Vec<u8> {
    let bpp = bpp_for(ct);
    let raw = raw_rows(rng, w, h, bpp, filters);
    let mut spec = PngSpec::new(w, h, ct, raw);
    spec.deflate = deflate;
    spec.build()
}

fn run_ct_filter_row(seed: u64, ct: u8, filters: &[u8], label: &str) {
    let (c, r) = both();
    let mut rng = Rng::new(seed);
    for &(w, h) in SHAPES {
        for iter in 0..6 {
            let use_fixed = iter % 2 == 1;
            let bpp = bpp_for(ct);
            let raw = raw_rows(&mut rng, w, h, bpp, filters);
            let mut spec = PngSpec::new(w, h, ct, raw.clone());
            if use_fixed {
                spec.deflate = Some(deflate_fixed_literals(&raw));
            }
            if ct == 3 {
                spec.plte = Some(rng.bytes(256 * 3));
            }
            let png = spec.build();
            cmp_png(&c, &r, &format!("{label} {w}x{h} iter{iter}"), &png);
        }
    }
}

// ---------------------------------------------------------------------------
// rows 1..15 -- colour type x filter x shape
// ---------------------------------------------------------------------------

#[test]
fn row01_ct0_filter0() {
    run_ct_filter_row(1, 0, &[0], "ct0 f0");
}
#[test]
fn row02_ct0_filter1() {
    run_ct_filter_row(2, 0, &[1], "ct0 f1");
}
#[test]
fn row03_ct0_filter2() {
    run_ct_filter_row(3, 0, &[2], "ct0 f2");
}
#[test]
fn row04_ct0_filter3() {
    run_ct_filter_row(4, 0, &[3], "ct0 f3");
}
#[test]
fn row05_ct0_filter4() {
    run_ct_filter_row(5, 0, &[4], "ct0 f4");
}
#[test]
fn row06_ct0_filter_mixed() {
    let mut rng = Rng::new(6);
    let (c, r) = both();
    for &(w, h) in SHAPES {
        for iter in 0..8 {
            let filters: Vec<u8> = (0..h.max(1)).map(|_| rng.below(5) as u8).collect();
            let raw = raw_rows(&mut rng, w, h, 1, &filters);
            let png = PngSpec::new(w, h, 0, raw).build();
            cmp_png(&c, &r, &format!("ct0 mixed {w}x{h} i{iter}"), &png);
        }
    }
}

#[test]
fn row07_ct2_all_filters() {
    for f in 0..5u8 {
        run_ct_filter_row(70 + f as u64, 2, &[f], &format!("ct2 f{f}"));
    }
    run_ct_filter_row(79, 2, &[0, 1, 2, 3, 4], "ct2 mixed");
}

#[test]
fn row08_ct4_all_filters() {
    for f in 0..5u8 {
        run_ct_filter_row(80 + f as u64, 4, &[f], &format!("ct4 f{f}"));
    }
    run_ct_filter_row(89, 4, &[0, 4, 1, 3, 2], "ct4 mixed");
}

#[test]
fn row09_ct6_all_filters() {
    for f in 0..5u8 {
        run_ct_filter_row(90 + f as u64, 6, &[f], &format!("ct6 f{f}"));
    }
    run_ct_filter_row(99, 6, &[4, 3, 2, 1, 0], "ct6 mixed");
}

#[test]
fn row10_ct3_full_palette() {
    for f in 0..5u8 {
        run_ct_filter_row(100 + f as u64, 3, &[f], &format!("ct3 f{f}"));
    }
    run_ct_filter_row(109, 3, &[0, 1, 2, 3, 4], "ct3 mixed");
}

#[test]
fn row11_ct3_short_palette() {
    // cp_depalette indexes plte[c*3 ..] with no bound check, so a short PLTE
    // makes the C read past the chunk -- into the following (deterministic)
    // bytes of the PNG buffer the caller supplied.
    let (c, r) = both();
    let mut rng = Rng::new(11);
    for entries in [1usize, 2, 17, 63] {
        for &(w, h) in SHAPES {
            for _ in 0..4 {
                let raw = raw_rows(&mut rng, w, h, 1, &[0, 1, 2, 3, 4]);
                let mut spec = PngSpec::new(w, h, 3, raw);
                spec.plte = Some(rng.bytes(entries * 3));
                let png = spec.build();
                // `cp_depalette` can read up to plte[255*3 + 2], i.e. 767 bytes
                // past a short PLTE.  Pad the *buffer* (not the declared
                // png_length) so those reads stay inside our own deterministic
                // allocation instead of hitting unrelated heap bytes, which
                // differ between two runs of the same library.
                let real = png.len();
                let mut buf = png.clone();
                buf.extend((0..2048).map(|i| (i % 251) as u8));
                let a = call_load_len(&c, &buf, real as i32);
                let b = call_load_len(&r, &buf, real as i32);
                assert_eq!(a, b, "ct3 plte{entries} {w}x{h}");
            }
        }
    }
}

#[test]
fn row12_ct3_full_trns() {
    let (c, r) = both();
    let mut rng = Rng::new(12);
    for &(w, h) in SHAPES {
        for _ in 0..6 {
            let raw = raw_rows(&mut rng, w, h, 1, &[0, 1, 2, 3, 4]);
            let mut spec = PngSpec::new(w, h, 3, raw);
            spec.plte = Some(rng.bytes(256 * 3));
            spec.trns = Some(rng.bytes(256));
            cmp_png(&c, &r, &format!("ct3 trns256 {w}x{h}"), &spec.build());
        }
    }
}

#[test]
fn row13_ct3_short_trns() {
    let (c, r) = both();
    let mut rng = Rng::new(13);
    for tl in [0usize, 1, 5, 128, 255] {
        for &(w, h) in SHAPES {
            for _ in 0..3 {
                let raw = raw_rows(&mut rng, w, h, 1, &[0, 2, 4]);
                let mut spec = PngSpec::new(w, h, 3, raw);
                spec.plte = Some(rng.bytes(256 * 3));
                spec.trns = Some(rng.bytes(tl));
                cmp_png(&c, &r, &format!("ct3 trns{tl} {w}x{h}"), &spec.build());
            }
        }
    }
}

#[test]
fn row14_non_indexed_with_palette_chunks() {
    let (c, r) = both();
    let mut rng = Rng::new(14);
    for ct in [0u8, 2, 4, 6] {
        for &(w, h) in SHAPES {
            for variant in 0..3 {
                let raw = raw_rows(&mut rng, w, h, bpp_for(ct), &[0, 1, 2, 3, 4]);
                let mut spec = PngSpec::new(w, h, ct, raw);
                if variant != 1 {
                    spec.plte = Some(rng.bytes(256 * 3));
                }
                if variant != 0 {
                    spec.trns = Some(rng.bytes(6));
                }
                cmp_png(&c, &r, &format!("ct{ct} v{variant} {w}x{h}"), &spec.build());
            }
        }
    }
}

#[test]
fn row15_ct3_trns_without_plte() {
    let (c, r) = both();
    let mut rng = Rng::new(15);
    for &(w, h) in SHAPES {
        let raw = raw_rows(&mut rng, w, h, 1, &[0]);
        let mut spec = PngSpec::new(w, h, 3, raw);
        spec.trns = Some(rng.bytes(16));
        cmp_png(&c, &r, &format!("ct3 no-plte {w}x{h}"), &spec.build());
    }
}

// ---------------------------------------------------------------------------
// rows 16..24 -- chunk-stream shapes
// ---------------------------------------------------------------------------

#[test]
fn row16_single_idat() {
    run_ct_filter_row(16, 2, &[0, 1, 2, 3, 4], "single idat");
}

#[test]
fn row17_split_idats() {
    let (c, r) = both();
    let mut rng = Rng::new(17);
    for split in [vec![1usize], vec![2, 3], vec![1, 1, 1], vec![5, 7, 2, 4]] {
        for &(w, h) in SHAPES {
            for ct in [0u8, 2, 6] {
                let raw = raw_rows(&mut rng, w, h, bpp_for(ct), &[0, 1, 4]);
                let mut spec = PngSpec::new(w, h, ct, raw);
                spec.idat_split = split.clone();
                cmp_png(
                    &c,
                    &r,
                    &format!("split{split:?} ct{ct} {w}x{h}"),
                    &spec.build(),
                );
            }
        }
    }
}

#[test]
fn row18_idats_separated_by_ancillary() {
    let (c, r) = both();
    let mut rng = Rng::new(18);
    for &(w, h) in SHAPES {
        let raw = raw_rows(&mut rng, w, h, 3, &[0, 1]);
        let mut spec = PngSpec::new(w, h, 2, raw);
        spec.idat_split = vec![4, 4];
        spec.idat_separator = Some(chunk(b"tEXt", b"k\0v"));
        cmp_png(&c, &r, &format!("idat-sep {w}x{h}"), &spec.build());
    }
}

#[test]
fn row19_leading_empty_idat() {
    let (c, r) = both();
    let mut rng = Rng::new(19);
    for &(w, h) in SHAPES {
        for ct in [0u8, 3, 6] {
            let raw = raw_rows(&mut rng, w, h, bpp_for(ct), &[0, 2]);
            let mut spec = PngSpec::new(w, h, ct, raw);
            spec.empty_idat_first = true;
            if ct == 3 {
                spec.plte = Some(rng.bytes(768));
            }
            cmp_png(&c, &r, &format!("empty-idat ct{ct} {w}x{h}"), &spec.build());
        }
    }
}

#[test]
fn row20_ancillary_chunks_before_data() {
    let (c, r) = both();
    let mut rng = Rng::new(20);
    for &(w, h) in SHAPES {
        let raw = raw_rows(&mut rng, w, h, 4, &[0, 1, 2, 3, 4]);
        let mut spec = PngSpec::new(w, h, 6, raw);
        spec.pre_chunks = vec![
            chunk(b"gAMA", &45455u32.to_be_bytes()),
            chunk(b"tEXt", b"Comment\0hello"),
            chunk(b"pHYs", &[0, 0, 11, 18, 0, 0, 11, 18, 1]),
        ];
        cmp_png(&c, &r, &format!("ancillary {w}x{h}"), &spec.build());
    }
}

#[test]
fn row21_trns_before_plte() {
    // the file order is tRNS then PLTE; cp_find("PLTE") has to scan past tRNS
    // and the rewind logic then has to find tRNS again.
    let (c, r) = both();
    let mut rng = Rng::new(21);
    for &(w, h) in SHAPES {
        let bpp = 1;
        let raw = raw_rows(&mut rng, w, h, bpp, &[0, 3]);
        let mut spec = PngSpec::new(w, h, 3, raw);
        // emit tRNS as a pre-chunk so it precedes PLTE
        spec.pre_chunks = vec![chunk(b"tRNS", &rng.bytes(200))];
        spec.plte = Some(rng.bytes(768));
        cmp_png(&c, &r, &format!("trns-first {w}x{h}"), &spec.build());
    }
}

#[test]
fn row22_chunks_and_junk_after_idat() {
    let (c, r) = both();
    let mut rng = Rng::new(22);
    for &(w, h) in SHAPES {
        for trail in [vec![], vec![0u8; 3], rng.bytes(37)] {
            let raw = raw_rows(&mut rng, w, h, 2, &[0, 1, 4]);
            let mut spec = PngSpec::new(w, h, 4, raw);
            spec.trailing = trail.clone();
            cmp_png(
                &c,
                &r,
                &format!("trailing{} {w}x{h}", trail.len()),
                &spec.build(),
            );
        }
    }
}

#[test]
fn row23_length_longer_than_buffer_content() {
    // png_length that covers trailing padding bytes present in the buffer
    let (c, r) = both();
    let mut rng = Rng::new(23);
    for &(w, h) in SHAPES {
        let raw = raw_rows(&mut rng, w, h, 3, &[0, 1]);
        let mut spec = PngSpec::new(w, h, 2, raw);
        spec.trailing = vec![0u8; 64];
        let png = spec.build();
        let real = png.len() - 64;
        for extra in [0usize, 1, 12, 64] {
            let a = call_load_len(&c, &png, (real + extra) as i32);
            let b = call_load_len(&r, &png, (real + extra) as i32);
            assert_eq!(a, b, "len={} {w}x{h}", real + extra);
        }
    }
}

#[test]
fn row24_ihdr_longer_than_13() {
    let (c, r) = both();
    let mut rng = Rng::new(24);
    for n in [13u32, 14, 20, 40] {
        for &(w, h) in SHAPES {
            let raw = raw_rows(&mut rng, w, h, 1, &[0, 1]);
            let mut spec = PngSpec::new(w, h, 0, raw);
            spec.ihdr_declared_len = Some(n);
            cmp_png(&c, &r, &format!("ihdr-len{n} {w}x{h}"), &spec.build());
        }
    }
}

// ---------------------------------------------------------------------------
// rows 25..31 -- DEFLATE / zlib configuration inside a PNG
// ---------------------------------------------------------------------------

#[test]
fn row25_stored_block() {
    // PngSpec's default encoder is a single final stored block
    run_ct_filter_row(25, 2, &[0, 1, 2, 3, 4], "stored");
}

#[test]
fn row26_fixed_literals() {
    let (c, r) = both();
    let mut rng = Rng::new(26);
    for ct in [0u8, 2, 4, 6] {
        for &(w, h) in SHAPES {
            let raw = raw_rows(&mut rng, w, h, bpp_for(ct), &[0, 1, 2, 3, 4]);
            let mut spec = PngSpec::new(w, h, ct, raw.clone());
            spec.deflate = Some(deflate_fixed_literals(&raw));
            cmp_png(&c, &r, &format!("fixed-lit ct{ct} {w}x{h}"), &spec.build());
        }
    }
}

#[test]
fn row27_fixed_backref_dist1() {
    let (c, r) = both();
    let mut rng = Rng::new(27);
    for &(w, h) in SHAPES {
        let bpp = 3usize;
        let stride = 1 + w as usize * bpp;
        // build `raw` as: first byte, then a run -- encoded as lit + match(dist 1)
        let total = stride * h as usize;
        let mut raw = Vec::with_capacity(total);
        let mut items = Vec::new();
        let first = rng.u8();
        raw.push(first);
        items.push(Item::Lit(first));
        let mut left = total - 1;
        while left > 0 {
            let n = (rng.range(3, 258) as usize).min(left);
            if n < 3 {
                for _ in 0..n {
                    raw.push(*raw.last().unwrap());
                    items.push(Item::Lit(*raw.last().unwrap()));
                }
            } else {
                let v = *raw.last().unwrap();
                for _ in 0..n {
                    raw.push(v);
                }
                items.push(Item::Mat(n as u32, 1));
            }
            left -= n;
        }
        // force the row filter bytes to legal values
        for y in 0..h as usize {
            raw[y * stride] = (y % 5) as u8;
        }
        let mut bw = BitWriter::new();
        // rebuild items to match the patched raw exactly: just emit literals for
        // the patched positions by re-encoding the whole buffer with dist-1 runs
        let items2 = dist1_encode(&raw);
        emit_fixed(&mut bw, &items2, true);
        let _ = items;
        let mut spec = PngSpec::new(w, h, 2, raw);
        spec.deflate = Some(bw.finish());
        cmp_png(&c, &r, &format!("dist1 {w}x{h}"), &spec.build());
    }
}

/// Encodes `data` as literals plus `dist == 1` matches (the `memset` fast path).
fn dist1_encode(data: &[u8]) -> Vec<Item> {
    let mut items = Vec::new();
    let mut i = 0usize;
    while i < data.len() {
        let mut run = 1usize;
        while i + run < data.len() && data[i + run] == data[i] {
            run += 1;
        }
        items.push(Item::Lit(data[i]));
        let mut left = run - 1;
        i += run;
        while left >= 3 {
            let n = left.min(258);
            if n < 3 {
                break;
            }
            items.push(Item::Mat(n as u32, 1));
            left -= n;
        }
        for _ in 0..left {
            items.push(Item::Lit(data[i - 1]));
        }
    }
    items
}

/// Encodes `data` with matches of distance > 1 wherever a repeat is available.
fn lz_encode(data: &[u8], rng: &mut Rng) -> Vec<Item> {
    let mut items = Vec::new();
    let mut i = 0usize;
    while i < data.len() {
        let mut best: Option<(u32, u32)> = None;
        if i >= 2 {
            let maxd = i.min(4096);
            for _ in 0..8 {
                let d = rng.range(2, maxd as u32) as usize;
                if d > i {
                    continue;
                }
                let mut l = 0usize;
                while l < 258 && i + l < data.len() && data[i + l] == data[i - d + l] {
                    l += 1;
                }
                if l >= 3 && best.map(|(bl, _)| (l as u32) > bl).unwrap_or(true) {
                    best = Some((l as u32, d as u32));
                }
            }
        }
        match best {
            Some((l, d)) => {
                items.push(Item::Mat(l, d));
                i += l as usize;
            }
            None => {
                items.push(Item::Lit(data[i]));
                i += 1;
            }
        }
    }
    items
}

#[test]
fn row28_fixed_backref_dist_gt1() {
    let (c, r) = both();
    let mut rng = Rng::new(28);
    for &(w, h) in SHAPES {
        for ct in [0u8, 2, 6] {
            let bpp = bpp_for(ct);
            let stride = 1 + w as usize * bpp;
            // repetitive data so that matches exist
            let mut raw = Vec::new();
            let pat = rng.bytes(7);
            for y in 0..h as usize {
                raw.push((y % 5) as u8);
                for x in 0..(stride - 1) {
                    raw.push(pat[x % pat.len()]);
                }
            }
            let items = lz_encode(&raw, &mut rng);
            let mut bw = BitWriter::new();
            emit_fixed(&mut bw, &items, true);
            let mut spec = PngSpec::new(w, h, ct, raw);
            spec.deflate = Some(bw.finish());
            cmp_png(&c, &r, &format!("lz ct{ct} {w}x{h}"), &spec.build());
        }
    }
}

#[test]
fn row29_real_dynamic_deflate() {
    let (c, r) = both();
    let mut rng = Rng::new(29);
    for level in 1..=9u32 {
        for &(w, h) in SHAPES {
            for ct in [0u8, 2, 4, 6] {
                let bpp = bpp_for(ct);
                let mut raw = raw_rows(&mut rng, w, h, bpp, &[0, 1, 2, 3, 4]);
                // make it compressible half the time
                if level % 2 == 0 {
                    let stride = 1 + w as usize * bpp;
                    for y in 0..h as usize {
                        for x in 1..stride {
                            raw[y * stride + x] = (x % 3) as u8;
                        }
                    }
                }
                let mut spec = PngSpec::new(w, h, ct, raw.clone());
                spec.deflate = Some(deflate_real(&raw, level));
                cmp_png(
                    &c,
                    &r,
                    &format!("real-deflate L{level} ct{ct} {w}x{h}"),
                    &spec.build(),
                );
            }
        }
    }
}

#[test]
fn row30_multiple_blocks() {
    let (c, r) = both();
    let mut rng = Rng::new(30);
    for &(w, h) in SHAPES {
        let bpp = 3usize;
        let stride = 1 + w as usize * bpp;
        let raw = raw_rows(&mut rng, w, h, bpp, &[0, 1, 2, 3, 4]);
        let total = raw.len();
        // split into 2..4 chunks; every non-final block is fixed or dynamic
        for nblocks in 2..=4usize {
            if total < nblocks {
                continue;
            }
            let per = total / nblocks;
            let mut bw = BitWriter::new();
            for b in 0..nblocks {
                let lo = b * per;
                let hi = if b + 1 == nblocks { total } else { (b + 1) * per };
                let part = &raw[lo..hi];
                let items: Vec<Item> = part.iter().map(|&x| Item::Lit(x)).collect();
                let is_final = b + 1 == nblocks;
                if b % 2 == 0 {
                    emit_fixed(&mut bw, &items, is_final);
                } else {
                    let (lit, dist) = uniform_trees(&items, 288, 1);
                    emit_dynamic(
                        &mut bw,
                        &items,
                        &lit,
                        &dist,
                        &cl_lens_uniform5(),
                        19,
                        ClStrategy::Rle,
                        is_final,
                    );
                }
            }
            let mut spec = PngSpec::new(w, h, 2, raw.clone());
            spec.deflate = Some(bw.finish());
            let _ = stride;
            cmp_png(&c, &r, &format!("multi{nblocks} {w}x{h}"), &spec.build());
        }
        // ... and a final *stored* block preceded by nothing (stored must be last
        // and the only block, per the C's bits_left/8 <= LEN check)
        let mut bw = BitWriter::new();
        emit_stored(&mut bw, &raw, true);
        let mut spec = PngSpec::new(w, h, 2, raw.clone());
        spec.deflate = Some(bw.finish());
        cmp_png(&c, &r, &format!("stored-only {w}x{h}"), &spec.build());
    }
}

#[test]
fn row31_zlib_header_variants() {
    let (c, r) = both();
    let mut rng = Rng::new(31);
    for cinfo in 0..8u8 {
        for flevel in 0..4u8 {
            for fcheck in [0u8, 5, 31] {
                let (w, h) = (9u32, 4u32);
                let raw = raw_rows(&mut rng, w, h, 1, &[0, 1, 2, 3, 4]);
                let mut spec = PngSpec::new(w, h, 0, raw);
                spec.cmf = (cinfo << 4) | 0x08;
                spec.flg = (flevel << 6) | fcheck;
                cmp_png(
                    &c,
                    &r,
                    &format!("zlib cinfo{cinfo} flevel{flevel} fcheck{fcheck}"),
                    &spec.build(),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 32..42 -- cp_inflate driven directly
// ---------------------------------------------------------------------------

#[test]
fn row32_inflate_stored() {
    let (c, r) = both();
    let mut rng = Rng::new(32);
    for len in [0usize, 1, 7, 64, 1024, 65535] {
        let data = rng.bytes(len);
        let mut bw = BitWriter::new();
        emit_stored(&mut bw, &data, true);
        let stream = bw.finish();
        for align in 0..4 {
            for extra in [0i32, 1, 64] {
                cmp_inflate(
                    &c,
                    &r,
                    &format!("stored{len}"),
                    &stream,
                    align,
                    (len as i32 + extra).max(1),
                );
            }
        }
    }
}

#[test]
fn row33_inflate_fixed_literals_alignments() {
    let (c, r) = both();
    let mut rng = Rng::new(33);
    // vary the stream length so that (in_bytes - first_bytes) % 4 hits 0..3
    for n in 1..=24usize {
        let data = rng.bytes(n);
        let stream = deflate_fixed_literals(&data);
        for align in 0..4 {
            for out in [n as i32, n as i32 + 3, n as i32 + 64] {
                cmp_inflate(&c, &r, &format!("fixed-lit{n}"), &stream, align, out);
            }
        }
    }
    // longer payloads too
    for n in [100usize, 255, 1000, 5000] {
        let data = rng.bytes(n);
        let stream = deflate_fixed_literals(&data);
        for align in 0..4 {
            cmp_inflate(&c, &r, &format!("fixed-lit{n}"), &stream, align, n as i32);
        }
    }
}

#[test]
fn row34_inflate_all_length_symbols() {
    let (c, r) = both();
    let mut rng = Rng::new(34);
    for sym in 257..=285usize {
        let base = LEN_BASE[sym - 257];
        let extra = LEN_EXTRA[sym - 257];
        for e in [0u32, (1 << extra) - 1] {
            let len = base + e;
            if len > 258 {
                continue;
            }
            let seed = rng.u8();
            let mut items = vec![Item::Lit(seed)];
            items.push(Item::Mat(len, 1));
            let mut bw = BitWriter::new();
            emit_fixed(&mut bw, &items, true);
            let stream = bw.finish();
            let expect = 1 + len as i32;
            for align in 0..4 {
                cmp_inflate(
                    &c,
                    &r,
                    &format!("len-sym{sym} e{e}"),
                    &stream,
                    align,
                    expect,
                );
            }
        }
    }
}

#[test]
fn row35_inflate_all_distance_symbols() {
    let (c, r) = both();
    let mut rng = Rng::new(35);
    for ds in 0..30usize {
        let base = DIST_BASE[ds];
        let extra = DIST_EXTRA[ds];
        for e in [0u32, (1 << extra) - 1] {
            let dist = base + e;
            // need at least `dist` bytes of history
            let pre = dist as usize;
            let prefix = rng.bytes(pre);
            let mlen = 3u32;
            let mut items: Vec<Item> = prefix.iter().map(|&b| Item::Lit(b)).collect();
            items.push(Item::Mat(mlen, dist));
            let mut bw = BitWriter::new();
            emit_fixed(&mut bw, &items, true);
            let stream = bw.finish();
            let expect = pre as i32 + mlen as i32;
            for align in 0..4 {
                cmp_inflate(
                    &c,
                    &r,
                    &format!("dist-sym{ds} e{e}"),
                    &stream,
                    align,
                    expect,
                );
            }
        }
    }
}

#[test]
fn row36_inflate_dynamic_hlit_hdist_hclen() {
    let (c, r) = both();
    let mut rng = Rng::new(36);
    // hclen: only symbols in PERM[..hclen] may be used.  With hclen >= 5 the
    // set contains {16,17,18,0,8}, which is enough for a uniform 8-bit tree.
    for hclen in 5..=19usize {
        for hlit in [257usize, 258, 288] {
            for hdist in [1usize, 2, 32] {
                let payload = rng.bytes(40);
                let mut items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b)).collect();
                let has_match = hlit > 257;
                if has_match {
                    items.push(Item::Mat(3, 1));
                }
                let mut lit = vec![0u8; hlit];
                let mut dist = vec![0u8; hdist];
                lit[256] = 8;
                for &b in &payload {
                    lit[b as usize] = 8;
                }
                if has_match {
                    lit[257] = 8; // length symbol for len 3
                }
                dist[0] = 8;
                let mut cl = [0u8; 19];
                for &s in &PERM[..hclen] {
                    cl[s] = 5;
                }
                // symbols 0 and 8 must be codable
                assert!(cl[0] != 0 && cl[8] != 0);
                let mut bw = BitWriter::new();
                emit_dynamic(
                    &mut bw,
                    &items,
                    &lit,
                    &dist,
                    &cl,
                    hclen,
                    ClStrategy::Rle,
                    true,
                );
                let stream = bw.finish();
                let expect = payload.len() as i32 + if has_match { 3 } else { 0 };
                for align in 0..4 {
                    cmp_inflate(
                        &c,
                        &r,
                        &format!("dyn hclen{hclen} hlit{hlit} hdist{hdist}"),
                        &stream,
                        align,
                        expect,
                    );
                }
            }
        }
    }
}

fn dynamic_with_strategy(rng: &mut Rng, strat: ClStrategy, hlit: usize) -> (Vec<u8>, i32) {
    let payload = rng.bytes(60);
    let mut items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b)).collect();
    let mut lit = vec![0u8; hlit];
    lit[256] = 8;
    for &b in &payload {
        lit[b as usize] = 8;
    }
    let dist = vec![8u8; 1];
    items.push(Item::Mat(3, 1));
    lit[257] = 8;
    let mut bw = BitWriter::new();
    emit_dynamic(
        &mut bw,
        &items,
        &lit,
        &dist,
        &cl_lens_uniform5(),
        19,
        strat,
        true,
    );
    (bw.finish(), payload.len() as i32 + 3)
}

#[test]
fn row37_dynamic_cl_symbol_16() {
    let (c, r) = both();
    let mut rng = Rng::new(37);
    for _ in 0..12 {
        let (stream, expect) = dynamic_with_strategy(&mut rng, ClStrategy::RepeatsOnly, 288);
        for align in 0..4 {
            cmp_inflate(&c, &r, "dyn cl16", &stream, align, expect);
        }
    }
}

#[test]
fn row38_dynamic_cl_symbol_17() {
    let (c, r) = both();
    let mut rng = Rng::new(38);
    // short zero runs: build lit lens with runs of 3..10 zeros
    for _ in 0..12 {
        let (stream, expect) = dynamic_with_strategy(&mut rng, ClStrategy::ZeroRunsOnly, 288);
        for align in 0..4 {
            cmp_inflate(&c, &r, "dyn cl17/18", &stream, align, expect);
        }
    }
}

#[test]
fn row39_dynamic_cl_symbol_18_long_zero_runs() {
    let (c, r) = both();
    let mut rng = Rng::new(39);
    // a tree where only a handful of literals are used -> 200+ zero run -> 18
    for _ in 0..12 {
        let payload: Vec<u8> = (0..40).map(|_| rng.below(4) as u8).collect();
        let mut items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b)).collect();
        items.push(Item::Mat(258, 1));
        let mut lit = vec![0u8; 288];
        lit[256] = 8;
        for &b in &payload {
            lit[b as usize] = 8;
        }
        lit[285] = 8; // length symbol for 258
        let dist = vec![8u8; 1];
        let mut bw = BitWriter::new();
        emit_dynamic(
            &mut bw,
            &items,
            &lit,
            &dist,
            &cl_lens_uniform5(),
            19,
            ClStrategy::Rle,
            true,
        );
        let stream = bw.finish();
        let expect = payload.len() as i32 + 258;
        for align in 0..4 {
            cmp_inflate(&c, &r, "dyn cl18", &stream, align, expect);
        }
    }
}

#[test]
fn row40_inflate_block_permutations() {
    let (c, r) = both();
    let mut rng = Rng::new(40);
    // non-final blocks may be fixed(1) or dynamic(2); stored(0) must be last and
    // alone, so it is covered by row32.
    for a in [1u8, 2] {
        for b in [1u8, 2] {
            for cc in [1u8, 2] {
                let p1 = rng.bytes(11);
                let p2 = rng.bytes(23);
                let p3 = rng.bytes(7);
                let mut bw = BitWriter::new();
                for (i, (t, p)) in [(a, &p1), (b, &p2), (cc, &p3)].iter().enumerate() {
                    let items: Vec<Item> = p.iter().map(|&x| Item::Lit(x)).collect();
                    let is_final = i == 2;
                    if *t == 1 {
                        emit_fixed(&mut bw, &items, is_final);
                    } else {
                        let (lit, dist) = uniform_trees(&items, 288, 1);
                        emit_dynamic(
                            &mut bw,
                            &items,
                            &lit,
                            &dist,
                            &cl_lens_uniform5(),
                            19,
                            ClStrategy::Rle,
                            is_final,
                        );
                    }
                }
                let stream = bw.finish();
                let expect = (p1.len() + p2.len() + p3.len()) as i32;
                for align in 0..4 {
                    cmp_inflate(
                        &c,
                        &r,
                        &format!("blocks {a}{b}{cc}"),
                        &stream,
                        align,
                        expect,
                    );
                }
            }
        }
    }
}

#[test]
fn row41_inflate_out_sizes() {
    let (c, r) = both();
    let mut rng = Rng::new(41);
    for _ in 0..20 {
        let n = rng.range(1, 300) as usize;
        let data = rng.bytes(n);
        let stream = deflate_fixed_literals(&data);
        for out in [n as i32, n as i32 + 1, n as i32 * 3 + 17, 100_000] {
            for align in 0..4 {
                cmp_inflate(&c, &r, &format!("outsize {n}->{out}"), &stream, align, out);
            }
        }
    }
}

#[test]
fn row42_inflate_random_valid_streams() {
    let (c, r) = both();
    let mut rng = Rng::new(42);
    for iter in 0..200 {
        let n = rng.range(1, 2000) as usize;
        let data: Vec<u8> = if iter % 3 == 0 {
            rng.bytes(n)
        } else if iter % 3 == 1 {
            let pn = rng.range(1, 9) as usize;
            let pat = rng.bytes(pn);
            (0..n).map(|i| pat[i % pat.len()]).collect()
        } else {
            (0..n).map(|_| rng.below(3) as u8).collect()
        };
        let level = rng.range(0, 9);
        let stream = deflate_real(&data, level);
        for align in 0..4 {
            for out in [data.len() as i32, data.len() as i32 + 40] {
                cmp_inflate(
                    &c,
                    &r,
                    &format!("real{iter} n{n} L{level}"),
                    &stream,
                    align,
                    out,
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// rows 43..48 -- the exported mutable tables as runtime options
// ---------------------------------------------------------------------------

/// Applies the same byte patch to both libraries' copy of a table, runs the
/// closure, then restores.
fn with_patched<F: FnMut(&Lib, &Lib)>(
    c: &Lib,
    r: &Lib,
    pick: fn(&Lib) -> *mut u8,
    off: usize,
    bytes: &[u8],
    mut f: F,
) {
    unsafe {
        let pc = pick(c).add(off);
        let pr = pick(r).add(off);
        let save_c = std::slice::from_raw_parts(pc, bytes.len()).to_vec();
        let save_r = std::slice::from_raw_parts(pr, bytes.len()).to_vec();
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pc, bytes.len());
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), pr, bytes.len());
        f(c, r);
        std::ptr::copy_nonoverlapping(save_c.as_ptr(), pc, bytes.len());
        std::ptr::copy_nonoverlapping(save_r.as_ptr(), pr, bytes.len());
    }
}

#[test]
fn row43_mutated_fixed_table() {
    let (c, r) = both();
    let mut rng = Rng::new(43);
    // A different but still valid fixed-Huffman assignment: swap the 7-bit and
    // 8-bit groups' boundary.  Build the stream with the *same* lengths so the
    // decode must follow the mutated table.
    let mut lens = fixed_lit_lengths();
    lens[0..144].fill(9);
    lens[144..256].fill(8);
    let mut newtable = vec![0u8; 320];
    newtable[..288].copy_from_slice(&lens);
    newtable[288..].fill(5);
    let data = rng.bytes(64);
    let items: Vec<Item> = data.iter().map(|&b| Item::Lit(b)).collect();
    let mut bw = BitWriter::new();
    bw.bits(1, 1);
    bw.bits(1, 2);
    let codes = canonical(&lens);
    for it in &items {
        if let Item::Lit(b) = it {
            bw.huff(codes[*b as usize], lens[*b as usize] as u32);
        }
    }
    bw.huff(codes[256], lens[256] as u32);
    let stream = bw.finish();
    with_patched(
        &c,
        &r,
        |l| l.fixed_table,
        0,
        &newtable,
        |c, r| {
            for align in 0..4 {
                cmp_inflate(c, r, "mutated fixed_table", &stream, align, data.len() as i32);
            }
        },
    );
    // and confirm the un-mutated table still decodes the unmodified stream
    let stream2 = deflate_fixed_literals(&data);
    cmp_inflate(&c, &r, "restored fixed_table", &stream2, 0, data.len() as i32);
}

#[test]
fn row44_mutated_len_tables() {
    let (c, r) = both();
    let mut rng = Rng::new(44);
    let data = rng.bytes(8);
    let mut items: Vec<Item> = data.iter().map(|&b| Item::Lit(b)).collect();
    items.push(Item::Mat(3, 1));
    let mut bw = BitWriter::new();
    emit_fixed(&mut bw, &items, true);
    let stream = bw.finish();
    // cp_len_base[0]: 3 -> 5, so the same stream produces a longer run
    for newbase in [3u32, 4, 5, 9] {
        unsafe {
            let pc = c.len_base;
            let pr = r.len_base;
            let sc = *pc;
            *pc = newbase;
            *pr = newbase;
            for align in 0..4 {
                cmp_inflate(
                    &c,
                    &r,
                    &format!("len_base[0]={newbase}"),
                    &stream,
                    align,
                    64,
                );
            }
            *pc = sc;
            *pr = sc;
        }
    }
    // cp_len_extra_bits[0]: 0 -> 1 changes how many bits are consumed
    with_patched(&c, &r, |l| l.len_extra_bits, 0, &[1u8], |c, r| {
        for align in 0..4 {
            cmp_inflate(c, r, "len_extra_bits[0]=1", &stream, align, 64);
        }
    });
}

#[test]
fn row45_mutated_dist_tables() {
    let (c, r) = both();
    let mut rng = Rng::new(45);
    let data = rng.bytes(16);
    let mut items: Vec<Item> = data.iter().map(|&b| Item::Lit(b)).collect();
    items.push(Item::Mat(4, 3));
    let mut bw = BitWriter::new();
    emit_fixed(&mut bw, &items, true);
    let stream = bw.finish();
    for nb in [1u32, 2, 3, 8] {
        unsafe {
            let pc = c.dist_base.add(2);
            let pr = r.dist_base.add(2);
            let sc = *pc;
            *pc = nb;
            *pr = nb;
            for align in 0..4 {
                cmp_inflate(&c, &r, &format!("dist_base[2]={nb}"), &stream, align, 64);
            }
            *pc = sc;
            *pr = sc;
        }
    }
    with_patched(&c, &r, |l| l.dist_extra_bits, 2, &[1u8], |c, r| {
        for align in 0..4 {
            cmp_inflate(c, r, "dist_extra_bits[2]=1", &stream, align, 64);
        }
    });
}

#[test]
fn row46_mutated_permutation_order() {
    let (c, r) = both();
    let mut rng = Rng::new(46);
    // rotate the permutation order and build a dynamic block that matches it
    let mut perm = PERM;
    perm.rotate_left(3);
    let newbytes: Vec<u8> = perm.iter().map(|&x| x as u8).collect();
    let payload = rng.bytes(30);
    let mut items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b)).collect();
    items.push(Item::Mat(3, 1));
    let mut lit = vec![0u8; 288];
    lit[256] = 8;
    lit[257] = 8;
    for &b in &payload {
        lit[b as usize] = 8;
    }
    let dist = vec![8u8; 1];
    let cl = cl_lens_uniform5();
    // emit with the rotated permutation
    let mut bw = BitWriter::new();
    bw.bits(1, 1);
    bw.bits(2, 2);
    bw.bits(288 - 257, 5);
    bw.bits(0, 5);
    bw.bits(19 - 4, 4);
    for i in 0..19 {
        bw.bits(cl[perm[i]] as u32, 3);
    }
    let cl_codes = canonical(&cl);
    let mut all = lit.clone();
    all.extend_from_slice(&dist);
    for (sym, extra) in encode_cl(&all, ClStrategy::Rle) {
        bw.huff(cl_codes[sym as usize], cl[sym as usize] as u32);
        bw.bits(extra, cl_extra_bits(sym));
    }
    let lit_codes = canonical(&lit);
    let dist_codes = canonical(&dist);
    for it in &items {
        match *it {
            Item::Lit(b) => bw.huff(lit_codes[b as usize], lit[b as usize] as u32),
            Item::Mat(l, d) => {
                let (ls, lx) = len_symbol(l);
                bw.huff(lit_codes[ls], lit[ls] as u32);
                bw.bits(lx, LEN_EXTRA[ls - 257]);
                let (ds, dx) = dist_symbol(d);
                bw.huff(dist_codes[ds], dist[ds] as u32);
                bw.bits(dx, DIST_EXTRA[ds]);
            }
        }
    }
    bw.huff(lit_codes[256], lit[256] as u32);
    let stream = bw.finish();
    with_patched(
        &c,
        &r,
        |l| l.permutation_order,
        0,
        &newbytes,
        |c, r| {
            for align in 0..4 {
                cmp_inflate(
                    c,
                    r,
                    "mutated permutation_order",
                    &stream,
                    align,
                    payload.len() as i32 + 3,
                );
            }
        },
    );
}

#[test]
fn row47_error_reason_is_never_cleared_on_success() {
    let (c, r) = both();
    let mut rng = Rng::new(47);
    let raw = raw_rows(&mut rng, 5, 5, 3, &[0, 1, 2, 3, 4]);
    let good = PngSpec::new(5, 5, 2, raw).build();
    unsafe {
        // 1. NULL beforehand -> still NULL after a successful call
        *c.err = std::ptr::null();
        *r.err = std::ptr::null();
        let a = (c.load)(good.as_ptr(), good.len() as i32);
        let b = (r.load)(good.as_ptr(), good.len() as i32);
        assert!(!a.pix.is_null() && !b.pix.is_null());
        libc::free(a.pix as *mut libc::c_void);
        libc::free(b.pix as *mut libc::c_void);
        assert_eq!(c.reason(), "(null)");
        assert_eq!(r.reason(), "(null)");

        // 2. a failing call sets it; a following successful call leaves it set
        let bad = vec![0u8; 8];
        let a = (c.load)(bad.as_ptr(), 8);
        let b = (r.load)(bad.as_ptr(), 8);
        assert!(a.pix.is_null() && b.pix.is_null());
        assert_eq!(c.reason(), r.reason());
        let before = c.reason();
        let a = (c.load)(good.as_ptr(), good.len() as i32);
        let b = (r.load)(good.as_ptr(), good.len() as i32);
        assert!(!a.pix.is_null() && !b.pix.is_null());
        libc::free(a.pix as *mut libc::c_void);
        libc::free(b.pix as *mut libc::c_void);
        assert_eq!(c.reason(), before);
        assert_eq!(r.reason(), before);
    }
}

// row 48 (out-of-range table reads driven by a corrupt Huffman tree) lives in
// phase_c_errors.rs, because those streams are malformed and may abort the
// assert-enabled reference build.

// ---------------------------------------------------------------------------
// rows 49..50 -- repeat / interleaving
// ---------------------------------------------------------------------------

#[test]
fn row49_repeat_and_recover() {
    let (c, r) = both();
    let mut rng = Rng::new(49);
    let raw = raw_rows(&mut rng, 11, 6, 4, &[0, 1, 2, 3, 4]);
    let good = PngSpec::new(11, 6, 6, raw).build();
    let bad = {
        let mut v = good.clone();
        v[8 + 8 + 8] = 4; // bit depth -> 4
        v
    };
    for _ in 0..5 {
        cmp_png(&c, &r, "repeat good", &good);
        cmp_png(&c, &r, "repeat bad", &bad);
        cmp_png(&c, &r, "repeat good again", &good);
    }
}

#[test]
fn row50_interleaved_entry_points() {
    let (c, r) = both();
    let mut rng = Rng::new(50);
    for _ in 0..30 {
        let (w, h) = (rng.range(1, 20), rng.range(1, 12));
        let ct = [0u8, 2, 4, 6][rng.below(4) as usize];
        let raw = raw_rows(&mut rng, w, h, bpp_for(ct), &[0, 1, 2, 3, 4]);
        let png = PngSpec::new(w, h, ct, raw).build();
        cmp_png(&c, &r, "interleave png", &png);
        let pn = rng.range(1, 200) as usize;
        let payload = rng.bytes(pn);
        let stream = deflate_fixed_literals(&payload);
        cmp_inflate(
            &c,
            &r,
            "interleave inflate",
            &stream,
            rng.below(4) as usize,
            payload.len() as i32,
        );
    }
}
