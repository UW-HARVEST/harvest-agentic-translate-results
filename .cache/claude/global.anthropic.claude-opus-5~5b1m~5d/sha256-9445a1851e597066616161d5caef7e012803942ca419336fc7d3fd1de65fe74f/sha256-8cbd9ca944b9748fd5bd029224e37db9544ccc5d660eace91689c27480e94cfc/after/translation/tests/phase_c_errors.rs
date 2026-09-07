//! Phase C -- error-path differential tests, one test per `ERRORS.md` row.
//!
//! Rows that cannot abort the assert-enabled reference build are compared
//! in-process (and their exact `cp_error_reason` is pinned).  Rows that can trip
//! one of the C's `assert()`s, plus every generic boundary (NULL pointers,
//! negative lengths, out-of-range enum values) and the fuzz corpora, go through
//! `examples/diffrunner`, which executes each case in a forked child so an abort
//! or a segfault is reported as a signal instead of killing the test.

mod common;

use common::*;

fn both() -> (Lib, Lib) {
    (Lib::open(&c_lib()), Lib::open(&rust_lib()))
}

/// Calls both libraries and asserts (a) they agree completely and (b) the C's
/// `cp_error_reason` is exactly `expect`.
#[track_caller]
fn expect_png_err(c: &Lib, r: &Lib, label: &str, data: &[u8], expect: &str) {
    let a = call_load(c, data);
    let b = call_load(r, data);
    assert_eq!(a, b, "divergence [{label}]");
    assert_eq!(a.err, expect, "wrong C error for [{label}]");
    assert!(a.pix.is_none(), "[{label}] unexpectedly succeeded");
}

#[track_caller]
fn expect_inflate_err(
    c: &Lib,
    r: &Lib,
    label: &str,
    stream: &[u8],
    out_bytes: i32,
    expect: &str,
) {
    for align in 0..4 {
        let a = call_inflate(c, stream, align, out_bytes);
        let b = call_inflate(r, stream, align, out_bytes);
        assert_eq!(a, b, "divergence [{label}] align={align}");
        assert_eq!(a.ret, 0, "[{label}] unexpectedly succeeded");
        assert_eq!(a.err, expect, "wrong C error for [{label}]");
    }
}

fn base_png(rng: &mut Rng, w: u32, h: u32, ct: u8) -> PngSpec {
    let raw = raw_rows(rng, w, h, bpp_for(ct), &[0, 1, 2, 3, 4]);
    PngSpec::new(w, h, ct, raw)
}

/// sig + IHDR(dw, dh, ct) + IEND, i.e. no IDAT at all.  Useful for the
/// huge-dimension rows: the size check / `malloc` runs, and the very next check
/// (`datalen >= 6`) fails, so nothing touches the multi-gigabyte allocation.
fn png_header_only(dw: u32, dh: u32, ct: u8) -> Vec<u8> {
    let mut v = SIG.to_vec();
    v.extend_from_slice(&chunk(b"IHDR", &ihdr_body(dw, dh, 8, ct)));
    v.extend_from_slice(&chunk(b"IEND", &[]));
    v
}


// ===========================================================================
// rows 1..20 -- load_png_mem
// ===========================================================================

#[test]
fn row01_bad_signature() {
    let (c, r) = both();
    let mut rng = Rng::new(101);
    let good = base_png(&mut rng, 5, 5, 2).build();
    const MSG: &str = "incorrect file signature (is this a png file?)";
    // every single-byte corruption of the signature
    for i in 0..8usize {
        for delta in [1u8, 0x80, 0xff] {
            let mut v = good.clone();
            v[i] = v[i].wrapping_add(delta);
            expect_png_err(&c, &r, &format!("sig byte {i} +{delta}"), &v, MSG);
        }
    }
    // random garbage
    for _ in 0..50 {
        let v = rng.bytes(64);
        let a = call_load(&c, &v);
        let b = call_load(&r, &v);
        assert_eq!(a, b);
    }
    // exactly 8 valid signature bytes and nothing else is *not* this error
    let mut only_sig = SIG.to_vec();
    only_sig.extend([0u8; 32]);
    let a = call_load(&c, &only_sig);
    let b = call_load(&r, &only_sig);
    assert_eq!(a, b);
    assert_eq!(a.err, "unable to find IHDR chunk");
}

#[test]
fn row02_missing_ihdr() {
    let (c, r) = both();
    let mut rng = Rng::new(102);
    const MSG: &str = "unable to find IHDR chunk";
    // (a) wrong chunk type
    {
        let mut v = SIG.to_vec();
        v.extend_from_slice(&chunk(b"IHDX", &ihdr_body(4, 4, 8, 2)));
        expect_png_err(&c, &r, "wrong ihdr type", &v, MSG);
    }
    // (b) declared length < 13
    for n in 0..13u32 {
        let mut v = SIG.to_vec();
        let body = ihdr_body(4, 4, 8, 2);
        v.extend_from_slice(&chunk_declared_len(b"IHDR", n, &body));
        v.extend([0u8; 64]);
        expect_png_err(&c, &r, &format!("ihdr len {n}"), &v, MSG);
    }
    // (c) truncated: p + len + 12 > end
    {
        let good = base_png(&mut rng, 4, 4, 2).build();
        for cut in 9..(8 + 12 + 13) {
            let v = good[..cut.min(good.len())].to_vec();
            expect_png_err(&c, &r, &format!("truncated at {cut}"), &v, MSG);
        }
    }
    // (d) a huge declared IHDR length makes `int offset = len + 12` negative, so
    //     cp_chunk walks *backwards* and the following cp_find runs off into
    //     unmapped memory -- see `boundaries_b1_b13_via_forked_runner`, which
    //     replays those cases in a forked child.
}

#[test]
fn row03_bit_depth_not_8() {
    // out-of-range "enum" across FFI: every byte value
    let (c, r) = both();
    let mut rng = Rng::new(103);
    for bd in 0..=255u8 {
        if bd == 8 {
            continue;
        }
        let mut spec = base_png(&mut rng, 5, 3, 2);
        spec.bit_depth = bd;
        expect_png_err(
            &c,
            &r,
            &format!("bit depth {bd}"),
            &spec.build(),
            "only bit-depth of 8 is supported",
        );
    }
}

#[test]
fn row04_unknown_color_type() {
    let (c, r) = both();
    let mut rng = Rng::new(104);
    for ct in 0..=255u8 {
        if matches!(ct, 0 | 2 | 3 | 4 | 6) {
            continue;
        }
        let mut spec = base_png(&mut rng, 5, 3, 2);
        spec.color_type = ct;
        expect_png_err(
            &c,
            &r,
            &format!("colour type {ct}"),
            &spec.build(),
            "unknown color type",
        );
    }
}

#[test]
fn row05_width_less_than_1() {
    let (c, r) = both();
    let mut rng = Rng::new(105);
    const MSG: &str = "invalid IHDR chunk found, image width was less than 1";
    for raw_w in [
        0xffff_ffffu32,
        0x8000_0000,
        0x8000_0001,
        0xffff_fffe,
        0xc000_0000,
    ] {
        let mut spec = base_png(&mut rng, 5, 3, 2);
        spec.width = raw_w;
        expect_png_err(&c, &r, &format!("width {raw_w:#x}"), &spec.build(), MSG);
    }
    // raw width 0 gives w == 1, which is *accepted* (the C's +1 quirk)
    let mut spec = base_png(&mut rng, 0, 3, 2);
    spec.width = 0;
    let a = call_load(&c, &spec.build());
    let b = call_load(&r, &spec.build());
    assert_eq!(a, b);
    assert_ne!(a.err, MSG);
}

#[test]
fn row06_height_less_than_1() {
    let (c, r) = both();
    let mut rng = Rng::new(106);
    const MSG: &str = "invalid IHDR chunk found, image height was less than 1";
    for raw_h in [0u32, 0x8000_0000, 0xffff_ffff, 0x8000_0001, 0xf000_0000] {
        let mut spec = base_png(&mut rng, 5, 3, 2);
        spec.height = raw_h;
        expect_png_err(&c, &r, &format!("height {raw_h:#x}"), &spec.build(), MSG);
    }
}

#[test]
fn row07_image_too_large() {
    let (c, r) = both();
    const MSG: &str = "image too large";
    // (int64_t)w * h * 4 >= INT_MAX, with w = declared_width + 1 and w, h >= 1
    for (dw, dh) in [
        (0xffffu32, 0x2000u32),
        (1, 0x7fff_ffff),
        (0xffff, 0xffff),
        (0x1fff_ffff, 1),
        (0x1ff_ffff, 0x10),
        (0x1fff_ffff, 0x7fff_ffff),
        (0x7fff_fffe, 2),
    ] {
        expect_png_err(
            &c,
            &r,
            &format!("size {dw:#x}x{dh:#x}"),
            &png_header_only(dw, dh, 2),
            MSG,
        );
    }
    // a declared width of 0x7fffffff makes w == INT_MIN, so the *width* check
    // fires first -- the C's check order is observable.
    expect_png_err(
        &c,
        &r,
        "width 0x7fffffff",
        &png_header_only(0x7fff_ffff, 1, 2),
        "invalid IHDR chunk found, image width was less than 1",
    );
    // just below the limit: w*h*4 == 0x7ffffffc -> the size check passes, so the
    // failure has to come from a later check
    let png = png_header_only(0x1fff_fffe, 1, 2);
    let a = call_load(&c, &png);
    let b = call_load(&r, &png);
    assert_eq!(a, b);
    assert_eq!(a.err, "corrupt zlib structure in DEFLATE stream");
    assert_eq!((a.w, a.h), (0x1fff_fffe, 1));
}

#[test]
fn row08_allocation_attempted_near_the_limit() {
    // The malloc-failure branch.  Pick the largest sizes the row-7 check accepts
    // so that `malloc` is really attempted with ~2 GiB, and give the file no IDAT
    // so that nothing touches the allocation.  Whether malloc succeeds or not,
    // both libraries must report the same thing.
    let (c, r) = both();
    for (dw, dh) in [
        (0x1fff_fffeu32, 1u32),
        (0xfffe, 0x1fff),
        (0xffff, 0x1fff),
        (0x7ffe, 0x3fff),
        (0x1fff_fffd, 1),
    ] {
        let png = png_header_only(dw, dh, 2);
        let a = call_load(&c, &png);
        let b = call_load(&r, &png);
        assert_eq!(a, b, "near-limit {dw:#x}x{dh:#x}");
        assert!(a.pix.is_none());
        assert!(
            a.err == "corrupt zlib structure in DEFLATE stream"
                || a.err == "unable to allocate raw image space",
            "unexpected reason {}",
            a.err
        );
    }
}

#[test]
fn row09_10_11_ihdr_method_bytes() {
    let (c, r) = both();
    let mut rng = Rng::new(109);
    for v in 1..=255u8 {
        let mut s = base_png(&mut rng, 4, 3, 2);
        s.compression = v;
        expect_png_err(
            &c,
            &r,
            &format!("compression {v}"),
            &s.build(),
            "only standard compression DEFLATE is supported",
        );
        let mut s = base_png(&mut rng, 4, 3, 2);
        s.filter_method = v;
        expect_png_err(
            &c,
            &r,
            &format!("filter method {v}"),
            &s.build(),
            "only standard adaptive filtering is supported",
        );
        let mut s = base_png(&mut rng, 4, 3, 2);
        s.interlace = v;
        expect_png_err(
            &c,
            &r,
            &format!("interlace {v}"),
            &s.build(),
            "interlacing is not supported",
        );
    }
}

/// Builds a PNG whose IDAT chunks carry exactly `bytes` (may be empty).
fn png_with_idat_bytes(w: u32, h: u32, ct: u8, bytes: &[u8], chunks: usize) -> Vec<u8> {
    let mut v = SIG.to_vec();
    v.extend_from_slice(&chunk(b"IHDR", &ihdr_body(w, h, 8, ct)));
    if chunks == 0 {
        v.extend_from_slice(&chunk(b"IEND", &[]));
        return v;
    }
    let per = (bytes.len() + chunks - 1) / chunks.max(1);
    let mut off = 0usize;
    for _ in 0..chunks {
        let n = per.min(bytes.len() - off);
        v.extend_from_slice(&chunk(b"IDAT", &bytes[off..off + n]));
        off += n;
    }
    v.extend_from_slice(&chunk(b"IEND", &[]));
    v
}

#[test]
fn row12_corrupt_zlib_structure() {
    let (c, r) = both();
    const MSG: &str = "corrupt zlib structure in DEFLATE stream";
    // (a) no IDAT at all
    expect_png_err(&c, &r, "no idat", &png_with_idat_bytes(4, 3, 2, &[], 0), MSG);
    // (b) IDAT totalling 0..5 bytes, in 1..3 chunks
    for n in 0..6usize {
        for chunks in 1..=3usize {
            let bytes: Vec<u8> = (0..n).map(|i| 0x78u8.wrapping_add(i as u8)).collect();
            expect_png_err(
                &c,
                &r,
                &format!("idat {n} bytes in {chunks}"),
                &png_with_idat_bytes(4, 3, 2, &bytes, chunks),
                MSG,
            );
        }
    }
    // (c) a single empty IDAT
    expect_png_err(
        &c,
        &r,
        "one empty idat",
        &png_with_idat_bytes(4, 3, 2, &[], 1),
        MSG,
    );
}

#[test]
fn row13_zlib_cm_not_8() {
    let (c, r) = both();
    let mut rng = Rng::new(113);
    const MSG: &str = "only zlib compression method (RFC 1950) is supported";
    for cmf in 0..=255u8 {
        if cmf & 0x0f == 0x08 {
            continue;
        }
        let mut spec = base_png(&mut rng, 4, 3, 2);
        spec.cmf = cmf;
        expect_png_err(&c, &r, &format!("cmf {cmf:#02x}"), &spec.build(), MSG);
    }
}

#[test]
fn row14_zlib_window_too_large() {
    let (c, r) = both();
    let mut rng = Rng::new(114);
    const MSG: &str = "innapropriate window size detected";
    for cinfo in 8..16u8 {
        let mut spec = base_png(&mut rng, 4, 3, 2);
        spec.cmf = (cinfo << 4) | 0x08;
        expect_png_err(&c, &r, &format!("cinfo {cinfo}"), &spec.build(), MSG);
    }
}

#[test]
fn row15_zlib_preset_dictionary() {
    let (c, r) = both();
    let mut rng = Rng::new(115);
    const MSG: &str = "preset dictionary is present and not supported";
    for flg in 0..=255u8 {
        if flg & 0x20 == 0 {
            continue;
        }
        let mut spec = base_png(&mut rng, 4, 3, 2);
        spec.flg = flg;
        expect_png_err(&c, &r, &format!("flg {flg:#02x}"), &spec.build(), MSG);
    }
}

#[test]
fn row16_17_invalid_image_size_is_unreachable() {
    // `cp_out_size(&img, bpp) = (img.w + 1) * img.h * bpp` with img.w = w - 1,
    // so it equals w * h * bpp.  Row 7 guarantees w*h*4 < INT_MAX and w, h >= 1,
    // hence 1 <= w*h*bpp <= w*h*4 < INT_MAX for every bpp in 1..=4: neither
    // "invalid image size found" branch is reachable.  Sweep the extreme
    // accepted shapes for every colour type and check the message never appears.
    let (c, r) = both();
    let mut rng = Rng::new(116);
    for ct in [0u8, 2, 3, 4, 6] {
        for (dw, dh) in [
            (0u32, 1u32),
            (0, 0x7fff_fffe),
            (0x1fff_fffe, 1),
            (0xfffe, 0x1fff),
            (0x3fff_fffe, 1),
            (1, 0x3fff_fffe),
            (0x1fff_fffd, 1),
        ] {
            let png = png_header_only(dw, dh, ct);
            let a = call_load(&c, &png);
            let b = call_load(&r, &png);
            assert_eq!(a, b, "ct{ct} {dw:#x}x{dh:#x}");
            assert_ne!(a.err, "invalid image size found", "ct{ct} {dw:#x}x{dh:#x}");
        }
        // and the smallest possible real images, which also decode successfully
        for &(w, h) in &[(1u32, 1u32), (1, 2), (2, 1)] {
            let raw = raw_rows(&mut rng, w, h, bpp_for(ct), &[0]);
            let mut spec = PngSpec::new(w, h, ct, raw);
            if ct == 3 {
                spec.plte = Some(rng.bytes(768));
            }
            let png = spec.build();
            let a = call_load(&c, &png);
            let b = call_load(&r, &png);
            assert_eq!(a, b, "ct{ct} {w}x{h}");
            assert_ne!(a.err, "invalid image size found");
            assert!(a.pix.is_some(), "ct{ct} {w}x{h}: {}", a.err);
        }
    }
}

#[test]
fn row18_deflate_algorithm_failed() {
    let (c, r) = both();
    let mut rng = Rng::new(118);
    const MSG: &str = "DEFLATE algorithm failed";
    // btype 3 inside the PNG's zlib payload
    {
        let mut spec = base_png(&mut rng, 4, 3, 2);
        spec.deflate = Some(vec![0x07, 0, 0, 0]);
        expect_png_err(&c, &r, "btype 3", &spec.build(), MSG);
    }
    // stored block with mismatching NLEN
    {
        let mut spec = base_png(&mut rng, 4, 3, 2);
        let mut bw = BitWriter::new();
        bw.bits(1, 1);
        bw.bits(0, 2);
        bw.align();
        bw.raw(&4u16.to_le_bytes());
        bw.raw(&0u16.to_le_bytes());
        bw.raw(&[1, 2, 3, 4]);
        spec.deflate = Some(bw.finish());
        expect_png_err(&c, &r, "stored nlen", &spec.build(), MSG);
    }
    // fixed block whose output exceeds the image buffer
    {
        let mut spec = base_png(&mut rng, 1, 1, 2);
        let payload = rng.bytes(4096);
        let items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b)).collect();
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &items, true);
        spec.deflate = Some(bw.finish());
        expect_png_err(&c, &r, "output overflow", &spec.build(), MSG);
    }
    // back-reference pointing before the start of the buffer
    {
        let mut spec = base_png(&mut rng, 4, 3, 2);
        let items = vec![Item::Lit(1), Item::Mat(3, 100)];
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &items, true);
        spec.deflate = Some(bw.finish());
        expect_png_err(&c, &r, "bad distance", &spec.build(), MSG);
    }
}

#[test]
fn row19_27_28_invalid_filter_byte() {
    let (c, r) = both();
    let mut rng = Rng::new(119);
    const MSG: &str = "invalid filter byte found";
    // row 0 (cp_unfilter's first switch) and row 1 (the second switch)
    for badrow in [0usize, 1] {
        for f in 5..=255u8 {
            let (w, h) = (4u32, 3u32);
            let bpp = 3usize;
            let stride = 1 + w as usize * bpp;
            let mut raw = raw_rows(&mut rng, w, h, bpp, &[0]);
            raw[badrow * stride] = f;
            let spec = PngSpec::new(w, h, 2, raw);
            expect_png_err(
                &c,
                &r,
                &format!("filter {f} on row {badrow}"),
                &spec.build(),
                MSG,
            );
        }
    }
    // ... and on the *last* row
    for f in [5u8, 200, 255] {
        let (w, h) = (6u32, 5u32);
        let bpp = 1usize;
        let stride = 1 + w as usize * bpp;
        let mut raw = raw_rows(&mut rng, w, h, bpp, &[0]);
        raw[(h as usize - 1) * stride] = f;
        expect_png_err(
            &c,
            &r,
            &format!("filter {f} last row"),
            &PngSpec::new(w, h, 0, raw).build(),
            MSG,
        );
    }
}

#[test]
fn row20_indexed_without_plte() {
    let (c, r) = both();
    let mut rng = Rng::new(120);
    const MSG: &str = "color type of indexed requires a PLTE chunk";
    for &(w, h) in &[(1u32, 1u32), (5, 5), (16, 3)] {
        let raw = raw_rows(&mut rng, w, h, 1, &[0, 1, 2, 3, 4]);
        let mut spec = PngSpec::new(w, h, 3, raw);
        expect_png_err(&c, &r, &format!("ct3 no plte {w}x{h}"), &spec.build(), MSG);
        // a tRNS chunk alone is still not a PLTE
        spec.trns = Some(rng.bytes(8));
        expect_png_err(&c, &r, &format!("ct3 trns only {w}x{h}"), &spec.build(), MSG);
        // a chunk whose type is *almost* PLTE
        let mut spec2 = PngSpec::new(w, h, 3, raw_rows(&mut rng, w, h, 1, &[0]));
        spec2.pre_chunks = vec![chunk(b"PLTe", &rng.bytes(768))];
        expect_png_err(&c, &r, &format!("ct3 PLTe {w}x{h}"), &spec2.build(), MSG);
    }
}

// ===========================================================================
// rows 21..26 -- cp_inflate error returns
// ===========================================================================

#[test]
fn row21_stored_len_nlen_mismatch() {
    let (c, r) = both();
    let mut rng = Rng::new(121);
    const MSG: &str =
        "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.";
    for len in [0u16, 1, 4, 300] {
        for wrong in [0u16, 1, 0xffff, 0x1234] {
            if wrong == !len {
                continue;
            }
            let data = rng.bytes(len as usize);
            let mut bw = BitWriter::new();
            bw.bits(1, 1);
            bw.bits(0, 2);
            bw.align();
            bw.raw(&len.to_le_bytes());
            bw.raw(&wrong.to_le_bytes());
            bw.raw(&data);
            expect_inflate_err(
                &c,
                &r,
                &format!("stored len={len} nlen={wrong:#x}"),
                &bw.finish(),
                (len as i32).max(1) + 16,
                MSG,
            );
        }
    }
}

#[test]
fn row22_stored_block_not_last() {
    let (c, r) = both();
    let mut rng = Rng::new(122);
    const MSG: &str = "Stored block extends beyond end of input stream.";
    for len in [0u16, 1, 2, 17] {
        for extra in [1usize, 4, 64] {
            let data = rng.bytes(len as usize);
            let mut bw = BitWriter::new();
            bw.bits(1, 1);
            bw.bits(0, 2);
            bw.align();
            bw.raw(&len.to_le_bytes());
            bw.raw(&(!len).to_le_bytes());
            bw.raw(&data);
            let mut stream = bw.finish();
            stream.extend(rng.bytes(extra));
            expect_inflate_err(
                &c,
                &r,
                &format!("stored len={len} +{extra}"),
                &stream,
                (len as i32) + 128,
                MSG,
            );
        }
    }
}

#[test]
fn row23_out_buffer_full_for_symbol() {
    let (c, r) = both();
    let mut rng = Rng::new(123);
    const MSG: &str = "Attempted to overwrite out buffer while outputting a symbol.";
    for n in [1usize, 2, 5, 100] {
        let payload = rng.bytes(n);
        let items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b)).collect();
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &items, true);
        let stream = bw.finish();
        // out_bytes = 0 -> the very first literal overflows
        expect_inflate_err(&c, &r, &format!("outbytes 0, n={n}"), &stream, 0, MSG);
        // and one byte short
        if n > 1 {
            expect_inflate_err(
                &c,
                &r,
                &format!("outbytes {}, n={n}", n - 1),
                &stream,
                n as i32 - 1,
                MSG,
            );
        }
    }
}

#[test]
fn row24_backwards_distance_before_begin() {
    let (c, r) = both();
    const MSG: &str = "Attempted to write before out buffer (invalid backwards distance).";
    for (pre, dist) in [(1usize, 2u32), (1, 5), (3, 4), (10, 11), (1, 32768)] {
        let items: Vec<Item> = (0..pre)
            .map(|i| Item::Lit(i as u8))
            .chain(std::iter::once(Item::Mat(3, dist)))
            .collect();
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &items, true);
        expect_inflate_err(
            &c,
            &r,
            &format!("pre={pre} dist={dist}"),
            &bw.finish(),
            4096,
            MSG,
        );
    }
}

#[test]
fn row25_out_buffer_full_for_string() {
    let (c, r) = both();
    const MSG: &str = "Attempted to overwrite out buffer while outputting a string.";
    for (len, out) in [(258u32, 10i32), (3, 3), (100, 50), (258, 258)] {
        let items = vec![Item::Lit(0x41), Item::Mat(len, 1)];
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &items, true);
        expect_inflate_err(
            &c,
            &r,
            &format!("len={len} out={out}"),
            &bw.finish(),
            out,
            MSG,
        );
    }
}

#[test]
fn row26_unknown_block_type() {
    let (c, r) = both();
    const MSG: &str = "Detected unknown block type within input stream.";
    // bfinal x btype=3, and btype 3 as a later block
    for first in [0x07u8, 0x06] {
        let mut stream = vec![first];
        stream.extend([0u8; 8]);
        expect_inflate_err(&c, &r, &format!("btype3 {first:#02x}"), &stream, 64, MSG);
    }
    // a non-final fixed block followed by a btype-3 block
    {
        let items: Vec<Item> = (0..4u8).map(Item::Lit).collect();
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &items, false);
        bw.bits(1, 1);
        bw.bits(3, 2);
        bw.align();
        bw.raw(&[0; 8]);
        expect_inflate_err(&c, &r, "fixed then btype3", &bw.finish(), 64, MSG);
    }
}

// ===========================================================================
// rows 29..32 -- silent-acceptance branches
// ===========================================================================

#[test]
fn row29_30_indexed_alpha_defaults() {
    let (c, r) = both();
    let mut rng = Rng::new(129);
    // trns == NULL -> alpha 255 for every pixel; trns_len shorter than an index
    // -> 255 for those indices.  Compare full pixel buffers.
    for trns_len in [None, Some(0usize), Some(1), Some(64), Some(256)] {
        for &(w, h) in &[(1u32, 1u32), (8, 4), (17, 3)] {
            let raw = raw_rows(&mut rng, w, h, 1, &[0]);
            let mut spec = PngSpec::new(w, h, 3, raw);
            spec.plte = Some(rng.bytes(768));
            spec.trns = trns_len.map(|n| rng.bytes(n));
            let png = spec.build();
            let a = call_load(&c, &png);
            let b = call_load(&r, &png);
            assert_eq!(a, b, "trns={trns_len:?} {w}x{h}");
            assert!(a.pix.is_some());
            if trns_len.is_none() {
                // every alpha byte must be 255
                assert!(a.pix.unwrap().chunks(4).all(|p| p[3] == 255));
            }
        }
    }
}

#[test]
fn row31_32_chunk_walk_termination() {
    let mut rng = Rng::new(131);
    // cp_chunk returning NULL because the *next* chunk runs past the end:
    // truncate the file at every offset.  Truncation can exhaust the bit reader
    // and trip `assert(s->count >= num_bits_to_read)` in the reference build, so
    // this sweep runs through the forked runner.
    let mut cp = Corpus::new();
    let raw = raw_rows(&mut rng, 6, 4, 3, &[0, 1]);
    let mut spec = PngSpec::new(6, 4, 2, raw);
    spec.idat_split = vec![4, 4, 4];
    let png = spec.build();
    for cut in 0..=png.len() {
        cp.png(&format!("truncated at {cut}"), &png[..cut]);
    }
    // the same for an indexed image with PLTE + tRNS (more chunks to walk)
    let raw = raw_rows(&mut rng, 6, 4, 1, &[0, 1, 2, 3, 4]);
    let mut spec = PngSpec::new(6, 4, 3, raw);
    spec.plte = Some(rng.bytes(768));
    spec.trns = Some(rng.bytes(40));
    spec.idat_split = vec![7, 7];
    let png = spec.build();
    for cut in 0..=png.len() {
        cp.png(&format!("ct3 truncated at {cut}"), &png[..cut]);
    }
    assert_corpus_matches("truncations", &cp);

    // cp_find walking to the end without a match: PLTE/tRNS never present
    let (c, r) = both();
    let raw = raw_rows(&mut rng, 6, 4, 3, &[0]);
    let png = PngSpec::new(6, 4, 2, raw).build();
    let a = call_load(&c, &png);
    let b = call_load(&r, &png);
    assert_eq!(a, b);
    assert!(a.pix.is_some());
}

// ===========================================================================
// generic boundaries + assert-reachable cases: run through diffrunner
// ===========================================================================

#[test]
fn boundaries_b1_b13_via_forked_runner() {
    let mut rng = Rng::new(200);
    let mut cp = Corpus::new();

    let raw = raw_rows(&mut rng, 6, 4, 3, &[0, 1, 2, 3, 4]);
    let good = PngSpec::new(6, 4, 2, raw.clone()).build();

    // B1: NULL png pointer, every interesting length
    for n in [0i32, 1, 7, 8, 100, -1, i32::MIN, i32::MAX] {
        cp.png_null(&format!("B1 null len={n}"), n);
    }
    // B2/B3/B4: real buffer, bogus lengths
    for n in [
        0i32,
        1,
        2,
        3,
        4,
        5,
        6,
        7,
        8,
        9,
        16,
        20,
        -1,
        -8,
        i32::MIN,
        i32::MAX,
        good.len() as i32,
        good.len() as i32 + 1,
        good.len() as i32 * 4,
    ] {
        cp.png_len(&format!("B2/3/4 len={n}"), &good, n);
    }
    // B5: NULL input to cp_inflate
    for out in [0i32, 1, 64] {
        for inl in [0i32, 1, 8, -1, i32::MAX] {
            cp.inflate_null_in(&format!("B5 out={out} in={inl}"), out, inl);
        }
    }
    // B6: NULL output
    let lit = {
        let items: Vec<Item> = raw.iter().map(|&b| Item::Lit(b)).collect();
        let mut bw = BitWriter::new();
        emit_fixed(&mut bw, &items, true);
        bw.finish()
    };
    for out in [0i32, 1, -1, i32::MAX] {
        cp.inflate_null_out(&format!("B6 out={out}"), &lit, out);
    }
    // B7: negative sizes
    for inl in [-1i32, -8, i32::MIN] {
        for out in [0i32, 64, -1] {
            for align in 0..4 {
                cp.inflate_len(
                    &format!("B7 in={inl} out={out} a={align}"),
                    &lit,
                    align,
                    out,
                    inl,
                );
            }
        }
    }
    // B8: zero-size output buffer
    for align in 0..4 {
        cp.inflate(&format!("B8 out=0 a={align}"), &lit, align, 0);
    }
    // B4': a declared IHDR length that sign-extends negative (cp_chunk walks
    // backwards) or runs past the end of the buffer
    for n in [
        0u32,
        1,
        12,
        13,
        14,
        0x7fff_fff8,
        0x7fff_ffff,
        0x8000_0000,
        0xffff_fff4,
        0xffff_ffff,
        0x0000_1000,
    ] {
        let mut v = SIG.to_vec();
        v.extend_from_slice(&chunk_declared_len(b"IHDR", n, &ihdr_body(6, 4, 8, 2)));
        v.extend([0u8; 64]);
        cp.png(&format!("B4' ihdr declared len {n:#x}"), &v);
    }
    // ... and the same for a PLTE / tRNS / IDAT chunk length
    for kind in [b"PLTE", b"tRNS", b"IDAT"] {
        for n in [0x7fff_fff8u32, 0x8000_0000, 0xffff_ffff, 0x0010_0000] {
            let mut v = SIG.to_vec();
            v.extend_from_slice(&chunk(b"IHDR", &ihdr_body(6, 4, 8, 3)));
            v.extend_from_slice(&chunk_declared_len(kind, n, &[0u8; 8]));
            v.extend_from_slice(&chunk(b"IEND", &[]));
            cp.png(
                &format!("B4' {} declared len {n:#x}", String::from_utf8_lossy(kind)),
                &v,
            );
        }
    }
    // B9/B10/B13: out-of-range enum bytes in IHDR (all 256 values each)
    for v in 0..=255u8 {
        for which in 0..5 {
            let mut spec = PngSpec::new(6, 4, 2, raw.clone());
            match which {
                0 => spec.color_type = v,
                1 => spec.bit_depth = v,
                2 => spec.compression = v,
                3 => spec.filter_method = v,
                _ => spec.interlace = v,
            }
            cp.png(&format!("B9/10/13 field{which}={v}"), &spec.build());
        }
    }
    // B11: every filter byte value on row 0 and row 1
    for v in 0..=255u8 {
        for badrow in [0usize, 1] {
            let bpp = 3usize;
            let stride = 1 + 6 * bpp;
            let mut r2 = raw.clone();
            r2[badrow * stride] = v;
            cp.png(
                &format!("B11 filter{v} row{badrow}"),
                &PngSpec::new(6, 4, 2, r2).build(),
            );
        }
    }
    // B12: every btype, as the first and as a second block
    for bt in 0..4u32 {
        for bf in 0..2u32 {
            let mut bw = BitWriter::new();
            bw.bits(bf, 1);
            bw.bits(bt, 2);
            bw.align();
            bw.raw(&[0u8; 16]);
            for align in 0..4 {
                cp.inflate(
                    &format!("B12 bfinal{bf} btype{bt} a{align}"),
                    &bw.bytes.clone(),
                    align,
                    64,
                );
            }
        }
    }

    assert_corpus_matches("boundaries", &cp);
}

#[test]
fn row48_out_of_range_table_reads() {
    // A corrupt Huffman tree makes cp_decode return symbols far past the end of
    // cp_len_* / cp_dist_*, so the C reads the neighbouring table in `.data`.
    let mut rng = Rng::new(300);
    let mut cp = Corpus::new();

    // (a) dynamic block with an all-zero literal/length alphabet -> nlit == 0 ->
    //     cp_decode reads tree[-1] (the tail of s->lookup)
    // (b) all-zero distance alphabet -> ndst == 0 -> tree[-1] is s->lit[287]
    // (c) trees that are valid but where the *emitted* bits select symbols
    //     286/287 (index 29/30 of the length tables) and beyond
    for variant in 0..3 {
        let mut lit = vec![0u8; 288];
        let mut dist = vec![0u8; 32];
        match variant {
            0 => {}
            1 => {
                lit[256] = 1;
                lit[257] = 1;
            }
            _ => {
                for i in 0..288 {
                    lit[i] = 9;
                }
                for i in 0..32 {
                    dist[i] = 5;
                }
            }
        }
        let cl = cl_lens_uniform5();
        let mut bw = BitWriter::new();
        bw.bits(1, 1);
        bw.bits(2, 2);
        bw.bits(288 - 257, 5);
        bw.bits(31, 5);
        bw.bits(19 - 4, 4);
        for i in 0..19 {
            bw.bits(cl[PERM[i]] as u32, 3);
        }
        let cl_codes = canonical(&cl);
        let mut all = lit.clone();
        all.extend_from_slice(&dist);
        for (sym, extra) in encode_cl(&all, ClStrategy::Rle) {
            bw.huff(cl_codes[sym as usize], cl[sym as usize] as u32);
            bw.bits(extra, cl_extra_bits(sym));
        }
        // then random payload bits
        let tail = rng.bytes(48);
        bw.align();
        bw.raw(&tail);
        let stream = bw.finish();
        for align in 0..4 {
            for out in [1i32, 64, 4096] {
                cp.inflate(
                    &format!("oob-tables v{variant} a{align} out{out}"),
                    &stream,
                    align,
                    out,
                );
            }
        }
    }

    // (c') fixed-Huffman streams that decode literal/length symbols 286 and 287
    //      (which index cp_len_extra_bits[29]/[30] and cp_len_base[29]/[30])
    for sym in [286usize, 287] {
        let lens = fixed_lit_lengths();
        let codes = canonical(&lens);
        for dsym in [0usize, 29, 30, 31] {
            let dlens = vec![5u8; 32];
            let dcodes = canonical(&dlens);
            let mut bw = BitWriter::new();
            bw.bits(1, 1);
            bw.bits(1, 2);
            bw.huff(codes[b'x' as usize], lens[b'x' as usize] as u32);
            bw.huff(codes[sym], lens[sym] as u32);
            bw.huff(dcodes[dsym], dlens[dsym] as u32);
            bw.bits(0, 16);
            bw.huff(codes[256], lens[256] as u32);
            bw.align();
            bw.raw(&[0u8; 8]);
            let stream = bw.finish();
            for align in 0..4 {
                cp.inflate(
                    &format!("litsym{sym} distsym{dsym} a{align}"),
                    &stream,
                    align,
                    4096,
                );
            }
        }
    }

    assert_corpus_matches("oob_tables", &cp);
}

#[test]
fn fuzz_mutated_pngs() {
    let mut rng = Rng::new(0xC0FFEE);
    let mut cp = Corpus::new();

    // a small seed corpus of valid PNGs across every configuration axis
    let mut seeds: Vec<Vec<u8>> = Vec::new();
    for ct in [0u8, 2, 3, 4, 6] {
        for &(w, h) in &[(1u32, 1u32), (5, 4), (16, 3)] {
            let raw = raw_rows(&mut rng, w, h, bpp_for(ct), &[0, 1, 2, 3, 4]);
            let mut spec = PngSpec::new(w, h, ct, raw.clone());
            if ct == 3 {
                spec.plte = Some(rng.bytes(768));
                spec.trns = Some(rng.bytes(60));
            }
            seeds.push(spec.build());
            let mut spec2 = PngSpec::new(w, h, ct, raw.clone());
            let items: Vec<Item> = raw.iter().map(|&b| Item::Lit(b)).collect();
            let mut bw = BitWriter::new();
            emit_fixed(&mut bw, &items, true);
            spec2.deflate = Some(bw.finish());
            if ct == 3 {
                spec2.plte = Some(rng.bytes(768));
            }
            seeds.push(spec2.build());
        }
    }

    for i in 0..1200 {
        let seed = &seeds[rng.below(seeds.len() as u32) as usize];
        let mut v = seed.clone();
        match i % 4 {
            0 => {
                // single byte flip
                let pos = rng.below(v.len() as u32) as usize;
                v[pos] ^= 1 << rng.below(8);
            }
            1 => {
                // a few random byte overwrites
                for _ in 0..rng.range(1, 6) {
                    let pos = rng.below(v.len() as u32) as usize;
                    v[pos] = rng.u8();
                }
            }
            2 => {
                // truncation
                let n = rng.below(v.len() as u32) as usize;
                v.truncate(n);
            }
            _ => {
                // splice two seeds
                let other = &seeds[rng.below(seeds.len() as u32) as usize];
                let cut = rng.below(v.len() as u32) as usize;
                v.truncate(cut);
                v.extend_from_slice(other);
            }
        }
        cp.png(&format!("fuzz-png {i}"), &v);
    }
    assert_corpus_matches("fuzz_png", &cp);
}

#[test]
fn fuzz_random_deflate_streams() {
    let mut rng = Rng::new(0xBADF00D);
    let mut cp = Corpus::new();
    for i in 0..500 {
        let n = rng.range(1, 80) as usize;
        let data: Vec<u8> = if i % 3 == 0 {
            rng.bytes(n)
        } else if i % 3 == 1 {
            // start from a real fixed-Huffman stream and corrupt it
            let payload = rng.bytes(n);
            let items: Vec<Item> = payload.iter().map(|&b| Item::Lit(b)).collect();
            let mut bw = BitWriter::new();
            emit_fixed(&mut bw, &items, true);
            let mut v = bw.finish();
            for _ in 0..rng.range(1, 4) {
                let p = rng.below(v.len() as u32) as usize;
                v[p] ^= 1 << rng.below(8);
            }
            v
        } else {
            // a plausible dynamic-block header followed by garbage
            let mut bw = BitWriter::new();
            bw.bits(1, 1);
            bw.bits(2, 2);
            bw.bits(rng.below(32), 5);
            bw.bits(rng.below(32), 5);
            bw.bits(rng.below(16), 4);
            for _ in 0..19 {
                bw.bits(rng.below(8), 3);
            }
            let mut v = bw.bytes.clone();
            v.extend(rng.bytes(n));
            v
        };
        for align in 0..4 {
            cp.inflate(&format!("fuzz-inf {i} a{align}"), &data, align, 512);
        }
    }
    assert_corpus_matches("fuzz_inflate", &cp);
}
