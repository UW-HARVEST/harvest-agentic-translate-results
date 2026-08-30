//! Phase C: one differential test per row of `ERRORS.md`.
//!
//! The reference C library is built with live `assert()`s, so some rejections
//! are `SIGABRT` rather than an error return. Both kinds are compared.

mod harness;

use harness::make::*;
use harness::*;
use std::ffi::c_int;

fn payload_ends_with(o: &Outcome, s: &str) -> bool {
    match o {
        Outcome::Ret(v) => v.ends_with(s.as_bytes()),
        _ => false,
    }
}

fn check(o: &Outcome, s: &str, what: &str) {
    assert!(
        payload_ends_with(o, s),
        "{what}: expected error {s:?}, got {o:?}"
    );
}

fn check_abort(o: &Outcome, what: &str) {
    assert_eq!(*o, Outcome::Signal(6), "{what}: expected SIGABRT");
}

/// A valid RGBA PNG plus its raw scanline block, used as the base for the
/// structural error cases.
fn good(rng: &mut Rng, w: usize, h: usize, ct: u8) -> (Vec<u8>, Vec<u8>, PngSpec) {
    let bpp = bpp_of(ct);
    let raw = raw_scanlines(rng, w, h, bpp, &[0]);
    let d = deflate_literals(&raw);
    let mut spec = PngSpec::new(w as u32, h as u32, ct, d);
    if ct == 3 {
        spec.plte = Some(rng.bytes(768));
    }
    (spec.build(), raw, spec)
}

// ---------------------------------------------------------------------------
// Rows 1-6: cp_inflate's graceful rejections
// ---------------------------------------------------------------------------

#[test]
fn rows_01_06_inflate_graceful() {
    let pair = load_pair();
    let mut rng = Rng::new(0xE1);
    let codes = Codes::fixed();
    let mut cases = Vec::new();
    let mut names: Vec<&'static str> = Vec::new();

    // Row 1: LEN and NLEN are not complements.
    for bad in [0u32, 1, 0xFFFF, 0x1234] {
        let payload = rng.bytes(4);
        let mut bw = BitW::new();
        bw.bits(1, 1);
        bw.bits(0, 2);
        bw.align();
        bw.bits(payload.len() as u32 & 0xFFFF, 16);
        bw.bits(bad, 16);
        assert_eq!(bw.nbits % 8, 0);
        bw.buf.extend_from_slice(&payload);
        bw.nbits += payload.len() * 8;
        cases.push(Case::inflate(
            format!("row1 bad NLEN {bad:#x}"),
            bw.finish(),
            0,
            64,
        ));
        names.push("row1");
    }

    // Row 2: LEN complements NLEN but the stored block is not last / too short.
    for len in [0u32, 1, 2, 3] {
        let payload = rng.bytes(16);
        let mut bw = BitW::new();
        bw.bits(1, 1);
        bw.bits(0, 2);
        bw.align();
        bw.bits(len, 16);
        bw.bits(!len & 0xFFFF, 16);
        bw.buf.extend_from_slice(&payload);
        bw.nbits += payload.len() * 8;
        cases.push(Case::inflate(format!("row2 LEN={len}"), bw.finish(), 0, 64));
        names.push("row2");
    }

    // Row 3: a literal that does not fit in the output buffer.
    for out in [0i32, 1, 3] {
        let toks: Vec<Tok> = (0..8u8).map(Tok::Lit).collect();
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        cases.push(Case::inflate(
            format!("row3 out={out}"),
            bw.finish(),
            0,
            out,
        ));
        names.push("row3");
    }
    // and a negative out_bytes (G7)
    {
        let toks: Vec<Tok> = (0..4u8).map(Tok::Lit).collect();
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        cases.push(Case::inflate("G7 out=-1", bw.finish(), 0, -1));
        names.push("row3");
    }

    // Row 4: a back-reference pointing before the start of the output.
    for (nlit, dist) in [(1usize, 2u32), (1, 5), (3, 9), (0, 1)] {
        let mut bw = BitW::new();
        bw.bits(1, 1);
        bw.bits(1, 2);
        for i in 0..nlit {
            bw.huff(codes.lit_codes[i], codes.lit_lens[i]);
        }
        let (ls, lx, lb) = len_code(3);
        bw.huff(codes.lit_codes[ls], codes.lit_lens[ls]);
        bw.bits(lx, lb as usize);
        let (ds, dx, db) = dist_code(dist);
        bw.huff(codes.dst_codes[ds], codes.dst_lens[ds]);
        bw.bits(dx, db as usize);
        bw.huff(codes.lit_codes[256], codes.lit_lens[256]);
        cases.push(Case::inflate(
            format!("row4 nlit={nlit} dist={dist}"),
            bw.finish(),
            0,
            64,
        ));
        names.push("row4");
    }

    // Row 5: a back-reference whose length overruns the output buffer.
    for (nlit, len, out) in [(5usize, 10u32, 6i32), (5, 258, 100), (1, 3, 2)] {
        let mut bw = BitW::new();
        bw.bits(1, 1);
        bw.bits(1, 2);
        for i in 0..nlit {
            bw.huff(codes.lit_codes[i], codes.lit_lens[i]);
        }
        let (ls, lx, lb) = len_code(len);
        bw.huff(codes.lit_codes[ls], codes.lit_lens[ls]);
        bw.bits(lx, lb as usize);
        let (ds, dx, db) = dist_code(1);
        bw.huff(codes.dst_codes[ds], codes.dst_lens[ds]);
        bw.bits(dx, db as usize);
        bw.huff(codes.lit_codes[256], codes.lit_lens[256]);
        cases.push(Case::inflate(
            format!("row5 nlit={nlit} len={len} out={out}"),
            bw.finish(),
            0,
            out,
        ));
        names.push("row5");
    }

    // Row 6: btype == 3.
    for align in 0..4usize {
        let mut bw = BitW::new();
        bw.bits(1, 1);
        bw.bits(3, 2);
        bw.bits(0, 40);
        cases.push(Case::inflate(
            format!("row6 btype=3 align={align}"),
            bw.finish(),
            align,
            64,
        ));
        names.push("row6");
    }
    // btype == 3 as a non-final block too
    {
        let mut bw = BitW::new();
        bw.bits(0, 1);
        bw.bits(3, 2);
        bw.bits(0, 40);
        cases.push(Case::inflate("row6 btype=3 nonfinal", bw.finish(), 0, 64));
        names.push("row6");
    }

    let out = run_same(&pair, &cases);
    for (i, o) in out.iter().enumerate() {
        let expect = match names[i] {
            "row1" => "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
            "row2" => "Stored block extends beyond end of input stream.",
            "row3" => "Attempted to overwrite out buffer while outputting a symbol.",
            "row4" => "Attempted to write before out buffer (invalid backwards distance).",
            "row5" => "Attempted to overwrite out buffer while outputting a string.",
            "row6" => "Detected unknown block type within input stream.",
            _ => unreachable!(),
        };
        check(o, expect, &cases[i].label);
        // and the return code must be 0
        if let Outcome::Ret(v) = o {
            assert_eq!(
                i32::from_le_bytes(v[0..4].try_into().unwrap()),
                0,
                "{}: cp_inflate must return 0",
                cases[i].label
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 7-21, 27: load_png_mem's structural rejections
// ---------------------------------------------------------------------------

#[test]
fn rows_07_21_27_png_structural() {
    let pair = load_pair();
    let mut rng = Rng::new(0xE2);
    let mut cases: Vec<Case> = Vec::new();
    let mut expect: Vec<&'static str> = Vec::new();

    let (base, raw, spec) = good(&mut rng, 4, 3, 6);

    // Row 7: bad signature (each of the 8 bytes wrong, plus a short buffer)
    for i in 0..8usize {
        let mut b = base.clone();
        b[i] ^= 0xFF;
        cases.push(Case::png(format!("row7 sig byte {i}"), b));
        expect.push("incorrect file signature (is this a png file?)");
    }
    // G3: the *data* is truncated inside the signature, so `memcmp` reads into
    // the (deterministically filled) padding past the buffer.
    for n in 1..8usize {
        let mut b = base.clone();
        b.truncate(n);
        cases.push(Case::png_len(
            format!("G3 data truncated to {n}"),
            b,
            n as c_int,
        ));
        expect.push("incorrect file signature (is this a png file?)");
    }
    // A short `png_length` with an intact buffer still passes the signature
    // check (the C reads 8 bytes regardless of `png_length`) and then fails to
    // find IHDR because `p + len + 12 > end`.
    for n in 0..8i32 {
        cases.push(Case::png_len(
            format!("G1 len={n} intact buffer"),
            base.clone(),
            n,
        ));
        expect.push("unable to find IHDR chunk");
    }

    // Row 8: IHDR not found.
    {
        // wrong chunk type
        let mut b = base.clone();
        b[12] = b'i';
        cases.push(Case::png("row8 iHDR", b));
        expect.push("unable to find IHDR chunk");
        // declared length < 13
        for l in [0u32, 1, 12] {
            let mut b = base.clone();
            b[8..12].copy_from_slice(&l.to_be_bytes());
            cases.push(Case::png(format!("row8 ihdr len={l}"), b));
            expect.push("unable to find IHDR chunk");
        }
        // declared length that runs past the end of the buffer
        for l in [1000u32, 0x0100_0000, 0x7FFF_FFF0] {
            let mut b = base.clone();
            b[8..12].copy_from_slice(&l.to_be_bytes());
            cases.push(Case::png(format!("row8 ihdr len={l:#x}"), b));
            expect.push("unable to find IHDR chunk");
        }
    }
    // G1/G2: zero and negative png_length
    cases.push(Case::png_len("G1 len=0", base.clone(), 0));
    expect.push("unable to find IHDR chunk");
    for n in [-1i32, -100, i32::MIN] {
        cases.push(Case::png_len(format!("G2 len={n}"), base.clone(), n));
        expect.push("unable to find IHDR chunk");
    }

    // Row 9: bit depth != 8 (IHDR data starts at byte 16, depth at 16+8 = 24)
    for bd in [0u8, 1, 2, 4, 7, 9, 16, 255] {
        let mut s = spec.clone();
        s.bit_depth = bd;
        cases.push(Case::png(format!("row9 depth={bd}"), s.build()));
        expect.push("only bit-depth of 8 is supported");
    }

    // Row 10 / G8: colour types with no `switch` case
    for ct in [1u8, 5, 7, 8, 9, 127, 128, 254, 255] {
        let mut s = spec.clone();
        s.color_type = ct;
        cases.push(Case::png(format!("row10 ct={ct}"), s.build()));
        expect.push("unknown color type");
    }

    // Row 11: width such that `cp_make32(ihdr) + 1 < 1` as an int
    for w in [0xFFFF_FFFFu32, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFF0] {
        let mut s = spec.clone();
        s.w = w;
        cases.push(Case::png(format!("row11 w={w:#x}"), s.build()));
        expect.push(if w == 0xFFFF_FFFF || w == 0x7FFF_FFFF {
            "invalid IHDR chunk found, image width was less than 1"
        } else {
            // 0x80000000 + 1 and 0xFFFFFFF0 + 1 are still negative as ints
            "invalid IHDR chunk found, image width was less than 1"
        });
    }

    // Row 12: height < 1
    for h in [0u32, 0x8000_0000, 0xFFFF_FFFF, 0x9000_0000] {
        let mut s = spec.clone();
        s.h = h;
        cases.push(Case::png(format!("row12 h={h:#x}"), s.build()));
        expect.push("invalid IHDR chunk found, image height was less than 1");
    }

    // Row 13: w * h * 4 >= INT_MAX
    for (w, h) in [
        (65535u32, 8192u32),
        (0x7FFF_FFFEu32, 1u32),
        (1u32, 0x7FFF_FFFFu32),
        (100000u32, 100000u32),
    ] {
        let mut s = spec.clone();
        s.w = w;
        s.h = h;
        cases.push(Case::png(format!("row13 {w}x{h}"), s.build()));
        expect.push("image too large");
    }

    // Rows 15-17: compression / filter / interlace method bytes
    for v in [1u8, 2, 255] {
        let mut s = spec.clone();
        s.comp = v;
        cases.push(Case::png(format!("row15 comp={v}"), s.build()));
        expect.push("only standard compression DEFLATE is supported");
        let mut s = spec.clone();
        s.filt = v;
        cases.push(Case::png(format!("row16 filt={v}"), s.build()));
        expect.push("only standard adaptive filtering is supported");
        let mut s = spec.clone();
        s.inter = v;
        cases.push(Case::png(format!("row17 inter={v}"), s.build()));
        expect.push("interlacing is not supported");
    }

    // Row 18: no IDAT at all, or an IDAT payload shorter than 6 bytes
    {
        let hdr = ihdr(4, 3, 8, 6, 0, 0, 0);
        let b = png_from_chunks(&[chunk(b"IHDR", &hdr), chunk(b"IEND", &[])]);
        cases.push(Case::png("row18 no IDAT", b));
        expect.push("corrupt zlib structure in DEFLATE stream");
        for n in 0..6usize {
            let b = png_from_chunks(&[
                chunk(b"IHDR", &hdr),
                chunk(b"IDAT", &rng.bytes(n)),
                chunk(b"IEND", &[]),
            ]);
            cases.push(Case::png(format!("row18 idat {n} bytes"), b));
            expect.push("corrupt zlib structure in DEFLATE stream");
        }
        // split into several chunks that still total < 6
        let b = png_from_chunks(&[
            chunk(b"IHDR", &hdr),
            chunk(b"IDAT", &[1, 2]),
            chunk(b"IDAT", &[3]),
            chunk(b"IDAT", &[4, 5]),
            chunk(b"IEND", &[]),
        ]);
        cases.push(Case::png("row18 split idat 5 bytes", b));
        expect.push("corrupt zlib structure in DEFLATE stream");
    }

    // Row 19: zlib compression method != 8
    for cm in [0u8, 1, 7, 9, 0x0F] {
        let mut s = spec.clone();
        s.cmf = (s.cmf & 0xF0) | cm;
        cases.push(Case::png(format!("row19 cm={cm}"), s.build()));
        expect.push("only zlib compression method (RFC 1950) is supported");
    }

    // Row 20: window size (CINFO) > 7
    for cinfo in 8..16u8 {
        let mut s = spec.clone();
        s.cmf = 0x08 | (cinfo << 4);
        cases.push(Case::png(format!("row20 cinfo={cinfo}"), s.build()));
        expect.push("innapropriate window size detected");
    }

    // Row 21: FDICT set
    for flg in [0x20u8, 0x21, 0x3F, 0xE0, 0xFF] {
        let mut s = spec.clone();
        s.flg = flg;
        cases.push(Case::png(format!("row21 flg={flg:#02x}"), s.build()));
        expect.push("preset dictionary is present and not supported");
    }

    // Row 27: colour type 3 without a PLTE chunk
    {
        let raw3 = raw_scanlines(&mut rng, 4, 3, 1, &[0]);
        let s = PngSpec::new(4, 3, 3, deflate_literals(&raw3));
        cases.push(Case::png("row27 indexed without PLTE", s.build()));
        expect.push("color type of indexed requires a PLTE chunk");
    }
    let _ = raw;

    let out = run_same(&pair, &cases);
    for (i, o) in out.iter().enumerate() {
        check(o, expect[i], &cases[i].label);
        if let Outcome::Ret(v) = o {
            assert_eq!(v[8], 0, "{}: pix must be NULL", cases[i].label);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 24-26: failures detected after inflating
// ---------------------------------------------------------------------------

#[test]
fn rows_24_26_deflate_and_filter_failures() {
    let pair = load_pair();
    let mut rng = Rng::new(0xE3);
    let mut cases: Vec<Case> = Vec::new();
    let mut expect: Vec<&'static str> = Vec::new();

    // Row 24: cp_inflate fails -> "DEFLATE algorithm failed" (the inflate
    // reason is overwritten).
    for &(w, h, ct) in &[(4usize, 3usize, 6u8), (2, 2, 0), (5, 1, 2), (3, 3, 4)] {
        let bpp = bpp_of(ct);
        // Running out of output space only stays inside the `img.pix`
        // allocation for bpp == 4 (`out_end` is `(w+1)*h*(4-bpp)` bytes past the
        // end of the buffer otherwise, so overflowing it corrupts the heap --
        // undefined in the C and layout-dependent, hence not compared).
        if bpp == 4 {
            let n = (w + 1) * h * 4 + 32;
            let data: Vec<u8> = (0..n).map(|i| (i & 0xFF) as u8).collect();
            let s = PngSpec::new(w as u32, h as u32, ct, deflate_literals(&data));
            cases.push(Case::png(
                format!("row24 out of space ct={ct} {w}x{h}"),
                s.build(),
            ));
            expect.push("DEFLATE algorithm failed");
        }
        // btype == 3
        let mut bw = BitW::new();
        bw.bits(1, 1);
        bw.bits(3, 2);
        bw.bits(0, 40);
        let s = PngSpec::new(w as u32, h as u32, ct, bw.finish());
        cases.push(Case::png(format!("row24 btype3 ct={ct}"), s.build()));
        expect.push("DEFLATE algorithm failed");
        // an invalid backwards distance
        let codes = Codes::fixed();
        let mut bw = BitW::new();
        bw.bits(1, 1);
        bw.bits(1, 2);
        bw.huff(codes.lit_codes[7], codes.lit_lens[7]);
        let (ls, lx, lb) = len_code(3);
        bw.huff(codes.lit_codes[ls], codes.lit_lens[ls]);
        bw.bits(lx, lb as usize);
        let (ds, dx, db) = dist_code(9);
        bw.huff(codes.dst_codes[ds], codes.dst_lens[ds]);
        bw.bits(dx, db as usize);
        bw.huff(codes.lit_codes[256], codes.lit_lens[256]);
        let s = PngSpec::new(w as u32, h as u32, ct, bw.finish());
        cases.push(Case::png(format!("row24 bad distance ct={ct}"), s.build()));
        expect.push("DEFLATE algorithm failed");
        // a stored block whose LEN/NLEN are not complements
        let mut bw = BitW::new();
        bw.bits(1, 1);
        bw.bits(0, 2);
        bw.align();
        bw.bits(4, 16);
        bw.bits(0, 16);
        bw.buf.extend_from_slice(&[1, 2, 3, 4]);
        bw.nbits += 32;
        let s = PngSpec::new(w as u32, h as u32, ct, bw.finish());
        cases.push(Case::png(format!("row24 stored bad NLEN ct={ct}"), s.build()));
        expect.push("DEFLATE algorithm failed");
    }

    // Rows 25 and 26: an invalid filter byte on row 0 and on a later row.
    for &ct in &[0u8, 2, 4, 6] {
        let bpp = bpp_of(ct);
        for &f in &[5u8, 6, 127, 128, 255] {
            // row 0
            let mut raw = raw_scanlines(&mut rng, 4, 3, bpp, &[0]);
            raw[0] = f;
            let s = PngSpec::new(4, 3, ct, deflate_literals(&raw));
            cases.push(Case::png(format!("row25 ct={ct} f={f}"), s.build()));
            expect.push("invalid filter byte found");
            // row 2
            let mut raw = raw_scanlines(&mut rng, 4, 3, bpp, &[0]);
            raw[2 * (1 + 4 * bpp)] = f;
            let s = PngSpec::new(4, 3, ct, deflate_literals(&raw));
            cases.push(Case::png(format!("row26 ct={ct} f={f} row2"), s.build()));
            expect.push("invalid filter byte found");
        }
    }

    let out = run_same(&pair, &cases);
    for (i, o) in out.iter().enumerate() {
        check(o, expect[i], &cases[i].label);
    }
}

// ---------------------------------------------------------------------------
// Rows A3, A4, A6, A9, A10: the reachable `assert()` failures
// ---------------------------------------------------------------------------

#[test]
fn rows_a3_a10_aborts() {
    let pair = load_pair();
    let mut rng = Rng::new(0xE4);
    let codes = Codes::fixed();
    let mut cases: Vec<Case> = Vec::new();

    // A6: bits_left <= 0 on the very first read.
    for align in 0..4usize {
        cases.push(Case::inflate(
            format!("A6 in_bytes=0 align={align}"),
            Vec::new(),
            align,
            64,
        ));
    }
    // A6/A8: negative in_bytes
    for n in [-1i32, -3, -8, -100] {
        let mut c = Case::inflate("A6 negative", rng.bytes(16), 0, 64);
        if let Call::Inflate {
            ref mut in_bytes, ..
        } = c.call
        {
            *in_bytes = n;
        }
        c.label = format!("A6 in_bytes={n}");
        cases.push(c);
    }
    // A6 through the PNG wrapper: an IDAT payload of exactly 6 bytes leaves
    // cp_inflate with in_bytes == 0.
    {
        let hdr = ihdr(4, 3, 8, 6, 0, 0, 0);
        let b = png_from_chunks(&[
            chunk(b"IHDR", &hdr),
            chunk(b"IDAT", &[0x78, 0x01, 0, 0, 0, 0]),
            chunk(b"IEND", &[]),
        ]);
        cases.push(Case::png("A6 idat exactly 6 bytes", b));
    }

    // A3: the stream is cut in the middle of a symbol.
    for cut in 1..6usize {
        let toks: Vec<Tok> = (0..30u8).map(Tok::Lit).collect();
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        let mut d = bw.finish();
        let keep = d.len().saturating_sub(cut);
        d.truncate(keep.max(1));
        cases.push(Case::inflate(format!("A3 cut {cut}"), d, 0, 64));
    }
    // A3: a stored block header that is cut short
    for n in [1usize, 2, 3, 4] {
        let mut bw = BitW::new();
        bw.bits(1, 1);
        bw.bits(0, 2);
        bw.align();
        bw.bits(0, 16);
        let mut d = bw.finish();
        d.truncate(n);
        cases.push(Case::inflate(format!("A3 stored cut {n}"), d, 0, 64));
    }

    // A4: `cp_read_bits(s, n)` with n > 32, reached by retuning an extra-bits
    // table above 32.
    for (table, val) in [
        (Table::LenExtraBits, 33u8),
        (Table::LenExtraBits, 64),
        (Table::LenExtraBits, 255),
        (Table::DistExtraBits, 33),
        (Table::DistExtraBits, 200),
    ] {
        let mut toks: Vec<Tok> = (0..8u8).map(Tok::Lit).collect();
        toks.push(Tok::Match(3, 1));
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        // symbol for length 3 is 257 -> index 0; distance 1 -> index 0
        cases.push(
            Case::inflate(format!("A4 {table:?}[0]={val}"), bw.finish(), 0, 128)
                .with_mutations(vec![Mutation {
                    table,
                    off: 0,
                    val,
                }]),
        );
    }

    // A9: `cp_build` sees a code length >= 16, reached by retuning
    // `cp_fixed_table` (btype == 1) ...
    for off in [0usize, 100, 287, 288, 319] {
        for val in [16u8, 17, 100, 255] {
            let toks: Vec<Tok> = (0..4u8).map(Tok::Lit).collect();
            let mut bw = BitW::new();
            block_fixed(&mut bw, true, &toks, &codes);
            cases.push(
                Case::inflate(
                    format!("A9 fixed_table[{off}]={val}"),
                    bw.finish(),
                    0,
                    64,
                )
                .with_mutations(vec![Mutation {
                    table: Table::FixedTable,
                    off,
                    val,
                }]),
            );
        }
    }
    // ... and by a dynamic block that transmits a code-length symbol >= 19.
    for sym in [19u8, 20, 25, 30] {
        // a 2-symbol code-length alphabet: {0, sym}, both length 1
        let mut lenlens = [0u8; 19];
        lenlens[0] = 1;
        if (sym as usize) < 19 {
            lenlens[sym as usize] = 1;
        }
        // symbols >= 19 cannot be transmitted, so instead give a *literal*
        // alphabet entry a code length of `sym` directly: possible because the
        // code-length alphabet carries values 0..=15 only, so we use a
        // 3-bit-limited value and then mutate the fixed table instead. Here we
        // drive it via nlen == 19 with a code length value of 15 which is legal,
        // and rely on the mutation-based A9 above; this case checks that a
        // *legal* 15 does not abort.
        let _ = lenlens;
        let toks: Vec<Tok> = (0..4u8).map(Tok::Lit).collect();
        let (lit, dst) = random_codes_for(&mut rng, &toks);
        let mut bw = BitW::new();
        block_dynamic(
            &mut bw,
            true,
            &toks,
            &lit,
            &dst,
            &PERMUTATION_ORDER,
            ClEncoding::Literal,
            &mut rng,
        );
        cases.push(Case::inflate(
            format!("A9 control legal dynamic {sym}"),
            bw.finish(),
            0,
            64,
        ));
    }

    // A10: an empty / incomplete code-length tree makes cp_decode read
    // `tree[-1]` and the assert fire.
    {
        // nlen == 4 with all four code lengths zero -> s->nlen == 0
        let mut bw = BitW::new();
        bw.bits(1, 1);
        bw.bits(2, 2);
        bw.bits(0, 5); // nlit = 257
        bw.bits(0, 5); // ndst = 1
        bw.bits(0, 4); // nlen = 4
        for _ in 0..4 {
            bw.bits(0, 3);
        }
        bw.bits(0, 64);
        cases.push(Case::inflate("A10 empty cl tree", bw.finish(), 0, 64));
    }
    for nlen in 4..20usize {
        // one code-length symbol with length 1 -> incomplete code
        let mut bw = BitW::new();
        bw.bits(1, 1);
        bw.bits(2, 2);
        bw.bits(0, 5);
        bw.bits(0, 5);
        bw.bits((nlen - 4) as u32, 4);
        for i in 0..nlen {
            bw.bits(if i == 0 { 1 } else { 0 }, 3);
        }
        bw.bits(0xFFFF_FFFF, 32);
        bw.bits(0xFFFF_FFFF, 32);
        cases.push(Case::inflate(
            format!("A10 incomplete cl tree nlen={nlen}"),
            bw.finish(),
            0,
            64,
        ));
    }

    let out = run_same(&pair, &cases);
    // count how many actually aborted, to prove the rows are exercised
    let aborts = out.iter().filter(|o| **o == Outcome::Signal(6)).count();
    assert!(
        aborts >= 30,
        "expected the abort rows to actually abort, got {aborts} of {}: {:?}",
        out.len(),
        &out[..out.len().min(8)]
    );
    // the A6 rows specifically
    for i in 0..4 {
        check_abort(&out[i], &cases[i].label);
    }
}

// ---------------------------------------------------------------------------
// Generic FFI boundary cases (G1-G10) and the unreachable rows
// ---------------------------------------------------------------------------

#[test]
fn generic_boundaries() {
    let pair = load_pair();
    let mut rng = Rng::new(0xE5);
    let codes = Codes::fixed();
    let mut cases: Vec<Case> = Vec::new();

    // G6: out_bytes == 0 and out_bytes == 1
    for out in [0i32, 1, 2] {
        let toks: Vec<Tok> = (0..4u8).map(Tok::Lit).collect();
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        cases.push(Case::inflate(format!("G6 out={out}"), bw.finish(), 0, out));
    }
    // G6 with an immediate end-of-block symbol and out_bytes == 0 (succeeds)
    {
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &[], &codes);
        cases.push(Case::inflate("G6 empty block out=0", bw.finish(), 0, 0));
    }
    // out_bytes far larger than needed
    {
        let toks: Vec<Tok> = (0..4u8).map(Tok::Lit).collect();
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        cases.push(Case::inflate("out huge", bw.finish(), 0, 4096));
    }

    // Row 22/23 ("invalid image size found") are unreachable: check 13 already
    // bounds w*h*4 < INT_MAX and both w and h are >= 1, so cp_out_size can never
    // be < 1. The closest reachable inputs are the 1x1 image and the largest
    // image that passes check 13.
    {
        let raw = raw_scanlines(&mut rng, 1, 1, 4, &[0]);
        let s = PngSpec::new(1, 1, 6, deflate_literals(&raw));
        cases.push(Case::png("row22/23 boundary 1x1", s.build()));
    }
    for (w, h) in [(2047u32, 4096u32), (4095u32, 2048u32)] {
        // (w+1)*h*4 comfortably under INT_MAX -> passes check 13 and the
        // malloc, then inflate fails immediately. (Sizes are kept modest: the
        // arithmetic that makes rows 22/23 unreachable does not depend on how
        // close to INT_MAX we get, and a 2 GiB allocation would just be slow.)
        let mut bw = BitW::new();
        bw.bits(1, 1);
        bw.bits(3, 2);
        bw.bits(0, 40);
        let s = PngSpec::new(w, h, 6, bw.finish());
        cases.push(Case::png(format!("row22/23 boundary {w}x{h}"), s.build()).compare_pixels(0));
    }

    // Row 14 ("unable to allocate raw image space") is unreachable because
    // pix_bytes < INT_MAX always; the largest allocation that check 13 permits
    // is exercised above.

    // A1/A2/A5 are unreachable:
    //  * A1: `bits_left ≡ count (mod 8)` is invariant and cp_stored discards
    //    exactly `count & 7` bits, so `bits_left & 7 == 0` at cp_ptr.
    //  * A2: guarded by the `word_index < word_count` test above it.
    //  * A5: every call site passes a literal or a `uint8_t` table entry.
    // Cover the closest reachable inputs: stored blocks at every bit offset
    // that a preceding block can leave behind, and every extra-bits value
    // 0..=32 (the boundary of A4).
    for pre in 0..8usize {
        let mut bw = BitW::new();
        // `pre` filler literals shift the stored block's start bit
        let toks: Vec<Tok> = (0..pre as u8).map(Tok::Lit).collect();
        if pre > 0 {
            block_fixed(&mut bw, false, &toks, &codes);
        }
        let payload = rng.bytes(9);
        block_stored(&mut bw, true, &payload);
        cases.push(Case::inflate(
            format!("A1 boundary pre={pre}"),
            bw.finish(),
            pre % 4,
            64,
        ));
    }
    for val in [0u8, 1, 15, 31, 32] {
        let mut toks: Vec<Tok> = (0..40u8).map(Tok::Lit).collect();
        toks.push(Tok::Match(3, 1));
        let mut bw = BitW::new();
        block_fixed(&mut bw, true, &toks, &codes);
        cases.push(
            Case::inflate(format!("A5/A4 boundary extra={val}"), bw.finish(), 0, 4096)
                .with_mutations(vec![Mutation {
                    table: Table::LenExtraBits,
                    off: 0,
                    val,
                }]),
        );
        cases.push(
            Case::inflate(
                format!("A5/A4 boundary dist extra={val}"),
                {
                    let mut bw = BitW::new();
                    block_fixed(&mut bw, true, &toks, &codes);
                    bw.finish()
                },
                0,
                4096,
            )
            .with_mutations(vec![Mutation {
                table: Table::DistExtraBits,
                off: 0,
                val,
            }]),
        );
    }

    assert_same(&pair, &cases);
}

/// `load_png_mem(NULL, n)` -- the C dereferences the pointer in `memcmp`, so
/// both libraries must die the same way.
#[test]
fn null_pointer() {
    let pair = load_pair();
    let cases: Vec<Case> = [0i32, 1, 8, 100, -1]
        .iter()
        .map(|n| Case {
            label: format!("load_png_mem(NULL, {n})"),
            mutations: Vec::new(),
            digest: false,
            call: Call::LoadPngNull { len: *n },
        })
        .collect();
    let out = run_same(&pair, &cases);
    for (i, o) in out.iter().enumerate() {
        assert_eq!(
            *o,
            Outcome::Signal(11),
            "{}: expected SIGSEGV, got {o:?}",
            cases[i].label
        );
    }
}
