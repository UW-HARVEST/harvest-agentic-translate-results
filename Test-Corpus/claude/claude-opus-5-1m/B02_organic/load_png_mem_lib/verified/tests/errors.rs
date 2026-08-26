//! Phase C — one differential test per row of `ERRORS.md`.
//!
//! Every test constructs the exact invalid input, calls **both** `.so`s and
//! asserts the same sentinel *and* the same `cp_error_reason` string (contents,
//! not pointer).  Rows whose C behaviour is `abort()` (the C library is built
//! without `NDEBUG`, so every `assert()` is live) are driven through
//! `fork()`/`waitpid()` so the termination status can be compared too; for
//! those the C side's `__assert_fail` message is checked as well, which proves
//! that the intended assertion — and not some other one — fired.

mod common;

use common::*;

const COLOUR_TYPES: [u8; 5] = [0, 2, 3, 4, 6];

fn expect_err(png: &[u8], reason: &str, label: &str) {
    let r = diff_png(png, label);
    assert!(!r.ok, "[{label}] expected rejection, got a decoded image");
    assert_eq!(r.err.as_deref(), Some(reason), "[{label}] wrong reason");
}

fn expect_ok(png: &[u8], label: &str) {
    let r = diff_png(png, label);
    assert!(r.ok, "[{label}] expected success, err={:?}", r.err);
}

// ===========================================================================
// A. load_png_mem rejections — ERRORS.md rows 1-24
// ===========================================================================

/// Row 1 — bad 8-byte signature.
#[test]
fn row_1_bad_signature() {
    let good = Spec::new(4, 4, 6).build();
    for i in 0..8usize {
        for delta in [1u8, 0x80, 0xFF] {
            let mut png = good.clone();
            png[i] ^= delta;
            expect_err(
                &png,
                "incorrect file signature (is this a png file?)",
                &format!("sig byte {i} ^ {delta:#04X}"),
            );
        }
    }
    // completely unrelated data
    for buf in [
        vec![0u8; 64],
        vec![0xFFu8; 64],
        b"GIF89a\0\0not a png at all........".to_vec(),
        b"\x89PNG\r\n\x1a".to_vec(), // 7 correct bytes, then padding
    ] {
        expect_err(
            &buf,
            "incorrect file signature (is this a png file?)",
            "non-png",
        );
    }
}

/// Rows 2-4 — the three ways `cp_chunk(&png, "IHDR", 13)` returns NULL.
#[test]
fn row_2_4_ihdr_chunk() {
    let ihdr_data = ihdr(4, 4, 8, 6, 0, 0, 0);
    let z = Spec::new(4, 4, 6).zlib_stream();

    // row 2: the chunk name is not "IHDR"
    for name in [b"iHDR", b"IHDr", b"XHDR", b"\0\0\0\0", b"IDAT", b"IHD\0"] {
        let png = build_png(
            &PNG_SIG,
            &[
                Chunk::new(name, ihdr_data.clone()),
                Chunk::new(b"IDAT", z.clone()),
                Chunk::new(b"IEND", vec![]),
            ],
        );
        expect_err(&png, "unable to find IHDR chunk", &format!("name={name:?}"));
    }

    // row 3: the declared length is < 13 (minlen)
    for len in 0..13u32 {
        let mut c = Chunk::new(b"IHDR", ihdr_data.clone());
        c.len_override = Some(len);
        let png = build_png(
            &PNG_SIG,
            &[c, Chunk::new(b"IDAT", z.clone()), Chunk::new(b"IEND", vec![])],
        );
        expect_err(&png, "unable to find IHDR chunk", &format!("ihdr len={len}"));
    }
    // 13 is accepted (boundary, one step inside the valid range)
    expect_ok(&Spec::new(4, 4, 6).build(), "ihdr len=13");

    // row 4: png.p + len + 12 > png.end -- the IHDR chunk does not fit inside
    // png_length.  A complete signature + IHDR needs 8 + 25 = 33 bytes.
    let good = Spec::new(4, 4, 6).build();
    for len in 0..33i32 {
        let r = diff_png_len(&good, len, &format!("png_length={len}"));
        assert!(!r.ok);
        assert_eq!(
            r.err.as_deref(),
            Some("unable to find IHDR chunk"),
            "png_length={len}"
        );
    }
    // 33 is exactly enough for the IHDR chunk (and then no IDAT is in range)
    let r = diff_png_len(&good, 33, "png_length=33");
    assert_eq!(
        r.err.as_deref(),
        Some("corrupt zlib structure in DEFLATE stream")
    );
    // a declared IHDR length that runs past the end of the file
    for extra in [1u32, 2, 100, 0xFFFF, 0x7FFF_FFFF, 0xFFFF_FFFF] {
        let mut c = Chunk::new(b"IHDR", ihdr(4, 4, 8, 6, 0, 0, 0));
        c.len_override = Some(13u32.wrapping_add(extra));
        let png = build_png(
            &PNG_SIG,
            &[c, Chunk::new(b"IDAT", z.clone()), Chunk::new(b"IEND", vec![])],
        );
        let r = diff_png_abort(&png, png.len() as i32, &format!("ihdr len=13+{extra}"));
        // may abort or be rejected -- either way both libraries must agree
        if let Outcome::Exited(_, _) = r.outcome {
            let rr = diff_png(&png, &format!("ihdr len=13+{extra} (in process)"));
            assert!(!rr.ok);
        }
    }
}

/// Row 5 — bit depth != 8 (the *whole* byte range, not just legal PNG depths).
#[test]
fn row_5_bit_depth() {
    for bd in 0..=255u8 {
        if bd == 8 {
            continue;
        }
        let mut s = Spec::new(4, 4, 6);
        s.bit_depth = bd;
        expect_err(
            &s.build(),
            "only bit-depth of 8 is supported",
            &format!("bit_depth={bd}"),
        );
    }
    expect_ok(&Spec::new(4, 4, 6).build(), "bit_depth=8");
}

/// Row 6 / row 50 — colour type outside `{0,2,3,4,6}`: every out-of-range
/// "enum" value that can cross the FFI boundary.
#[test]
fn row_6_colour_type() {
    for ct in 0..=255u8 {
        if COLOUR_TYPES.contains(&ct) {
            continue;
        }
        let mut s = Spec::new(4, 4, 6);
        s.ihdr_ct = Some(ct); // declared colour type only; payload stays valid
        expect_err(&s.build(), "unknown color type", &format!("ct={ct}"));
    }
    for ct in COLOUR_TYPES {
        expect_ok(&Spec::new(4, 4, ct).build(), &format!("ct={ct} ok"));
    }
}

/// Row 7 — `w = cp_make32(ihdr) + 1 < 1`.
#[test]
fn row_7_width_less_than_one() {
    for declared in [
        0xFFFF_FFFFu32, // w == 0
        0x7FFF_FFFF,    // w == INT_MIN
        0x8000_0000,    // w negative
        0xABCD_EF01,    // w negative
        0xFFFF_FFFE,    // w == -1
        0xC000_0000,
    ] {
        let mut s = Spec::new(4, 4, 6);
        s.ihdr_w = Some(declared);
        expect_err(
            &s.build(),
            "invalid IHDR chunk found, image width was less than 1",
            &format!("declared w={declared:#010X}"),
        );
    }
    // Declared width 0 gives `w == 1` and therefore `img.w == w - 1 == 0`:
    // a zero-pixel-wide image, which the C code accepts.  The scanline stream
    // is then h rows of just a filter byte.
    for h in [1u32, 2, 4, 9] {
        let s = Spec::new(0, h, 6);
        let r = diff_png(&s.build(), &format!("declared w=0 h={h}"));
        assert!(r.ok, "declared width 0 must be accepted: {:?}", r.err);
        assert_eq!((r.w, r.h), (0, h as i32));
        assert!(r.pixels.is_empty(), "a 0px wide image has no pixels");
    }
}

/// Row 8 — `h = cp_make32(ihdr + 4) < 1`.
#[test]
fn row_8_height_less_than_one() {
    for declared in [0u32, 0x8000_0000, 0xFFFF_FFFF, 0x9234_5678, 0xFFFF_FFFE] {
        let mut s = Spec::new(4, 4, 6);
        s.ihdr_h = Some(declared);
        expect_err(
            &s.build(),
            "invalid IHDR chunk found, image height was less than 1",
            &format!("declared h={declared:#010X}"),
        );
    }
    // 1 is the boundary that is accepted
    expect_ok(&Spec::new(4, 1, 6).build(), "h=1");
}

/// Rows 9 & 10 — `(int64_t)w * h * sizeof(cp_pixel_t) < INT_MAX`, and the
/// `malloc` failure right behind it.
#[test]
fn row_9_10_image_too_large() {
    // w*h*4 >= INT_MAX  (INT_MAX = 2147483647, so w*h >= 536870912 = 2^29)
    for (dw, dh) in [
        (0xFFFFu32, 8192u32),      // w=65536, h=8192  -> 2^31
        (0x1FFF_FFFF, 1),          // w=2^29, h=1      -> 2^31
        (0x7FFF_FFFE, 1),          // w=INT_MAX, h=1
        (0xFFFE, 0xFFFF),          // 65535 * 65535
        (1, 0x7FFF_FFFF),          // h=INT_MAX
        (0x1FFF_FFFF, 0x1FFF_FFFF),
    ] {
        let mut s = Spec::new(4, 4, 6);
        s.ihdr_w = Some(dw);
        s.ihdr_h = Some(dh);
        expect_err(
            &s.build(),
            "image too large",
            &format!("w={dw:#X} h={dh:#X}"),
        );
    }
    // exact boundary: w*h*4 == 2147483644 < INT_MAX is *accepted*, and then a
    // ~2 GiB malloc is attempted.  Whatever malloc does, both libraries must
    // agree (this is the only way to reach row 10 at all).
    let mut s = Spec::new(4, 4, 6);
    s.ihdr_w = Some(536_870_910); // w = 536870911, w*4 = 2147483644
    s.ihdr_h = Some(1);
    s.n_idat = 0; // bail out immediately after the allocation
    let r = diff_png(&s.build(), "INT_MAX boundary");
    assert!(!r.ok);
    assert!(
        matches!(
            r.err.as_deref(),
            Some("corrupt zlib structure in DEFLATE stream")
                | Some("unable to allocate raw image space")
        ),
        "unexpected reason at the INT_MAX boundary: {:?}",
        r.err
    );
    // one step over the boundary
    let mut s2 = s.clone();
    s2.ihdr_w = Some(536_870_911); // w = 536870912, w*4 = 2^31 >= INT_MAX
    expect_err(&s2.build(), "image too large", "INT_MAX boundary + 1");
}

/// Rows 11-13 — compression / filter / interlace method bytes, including the
/// legal-in-PNG-but-unsupported interlace value 1.
#[test]
fn row_11_13_ihdr_method_bytes() {
    for v in 1..=255u8 {
        let mut s = Spec::new(4, 4, 6);
        s.comp = v;
        let r = diff_png(&s.build(), &format!("comp={v}"));
        assert_eq!(
            r.err.as_deref(),
            Some("only standard compression DEFLATE is supported")
        );
        // w/h are already filled in when this branch is taken
        assert_eq!((r.w, r.h), (4, 4), "w/h must survive the error path");
        assert!(!r.ok);

        let mut s = Spec::new(4, 4, 6);
        s.filt = v;
        let r = diff_png(&s.build(), &format!("filt={v}"));
        assert_eq!(
            r.err.as_deref(),
            Some("only standard adaptive filtering is supported")
        );
        assert_eq!((r.w, r.h), (4, 4));

        let mut s = Spec::new(4, 4, 6);
        s.inter = v;
        let r = diff_png(&s.build(), &format!("inter={v}"));
        assert_eq!(r.err.as_deref(), Some("interlacing is not supported"));
        assert_eq!((r.w, r.h), (4, 4));
    }
}

/// Rows 14-15 — `!(data && datalen >= 6)`.
#[test]
fn row_14_15_short_or_missing_idat() {
    // row 14: no IDAT chunk at all -> datalen == 0, malloc(0) != NULL
    for ct in COLOUR_TYPES {
        let mut s = Spec::new(4, 4, ct);
        s.n_idat = 0;
        expect_err(
            &s.build(),
            "corrupt zlib structure in DEFLATE stream",
            &format!("no idat ct={ct}"),
        );
    }
    // row 15: IDAT present but the total payload is 0..5 bytes
    for n in 0..6usize {
        for split in 1..=3usize {
            let mut s = Spec::new(4, 4, 6);
            s.raw_zlib = Some((0..n).map(|i| 0x78u8.wrapping_add(i as u8)).collect());
            s.n_idat = split;
            expect_err(
                &s.build(),
                "corrupt zlib structure in DEFLATE stream",
                &format!("datalen={n} split={split}"),
            );
        }
    }
    // 6 is the boundary that gets *past* this check -- and then
    // `cp_inflate(data + 2, datalen - 6, ...)` is called with `in_bytes == 0`,
    // which trips `assert(s->bits_left > 0)`.  So `datalen == 6` aborts.
    let mut s = Spec::new(4, 4, 6);
    s.raw_zlib = Some(vec![0x78, 0x9C, 0, 0, 0, 0]);
    let png = s.build();
    let r = diff_png_abort(&png, png.len() as i32, "datalen=6");
    assert!(r.aborted(), "datalen == 6 must abort, got {:?}", r.outcome);
    assert_eq!(r.assertion().as_deref(), Some("s->bits_left > 0"));
    // 7 is the first length that actually reaches the DEFLATE decoder
    let mut s = Spec::new(4, 4, 6);
    s.raw_zlib = Some(vec![0x78, 0x9C, 0x07, 0, 0, 0, 0]);
    expect_err(&s.build(), "DEFLATE algorithm failed", "datalen=7 btype=3");
}

/// Row 16 — zlib CM != 8.
#[test]
fn row_16_zlib_method() {
    for cmf in 0..=255u8 {
        if cmf & 0x0F == 0x08 {
            continue;
        }
        let mut s = Spec::new(4, 4, 6);
        s.cmf = cmf;
        expect_err(
            &s.build(),
            "only zlib compression method (RFC 1950) is supported",
            &format!("cmf={cmf:#04X}"),
        );
    }
}

/// Row 17 — zlib CINFO > 7.
#[test]
fn row_17_zlib_window() {
    for cinfo in 8..=15u8 {
        let mut s = Spec::new(4, 4, 6);
        s.cmf = (cinfo << 4) | 0x08;
        expect_err(
            &s.build(),
            "innapropriate window size detected",
            &format!("cinfo={cinfo}"),
        );
    }
    // 7 is the boundary that is accepted
    let mut s = Spec::new(4, 4, 6);
    s.cmf = 0x78;
    expect_ok(&s.build(), "cinfo=7");
}

/// Row 18 — zlib FDICT set.
#[test]
fn row_18_zlib_preset_dictionary() {
    for flg in 0..=255u8 {
        let mut s = Spec::new(4, 4, 6);
        s.flg = flg;
        if flg & 0x20 != 0 {
            expect_err(
                &s.build(),
                "preset dictionary is present and not supported",
                &format!("flg={flg:#04X}"),
            );
        } else {
            expect_ok(&s.build(), &format!("flg={flg:#04X}"));
        }
    }
}

/// Rows 19-20 — `cp_out_size(&img, 4) < 1` / `cp_out_size(&img, bpp) < 1`.
///
/// These are **unreachable**: rows 7/8/9 have already forced `w >= 1`,
/// `h >= 1` and `(int64_t)w*h*4 < INT_MAX`, so `(img.w+1)*img.h*bpp` is in
/// `[1, INT_MAX)` for every `bpp` in `{1,2,3,4}`.  The test therefore sweeps
/// the whole reachable `w`/`h` boundary set and asserts that the branch is
/// never taken by either implementation.
#[test]
fn row_19_20_out_size_unreachable() {
    for ct in COLOUR_TYPES {
        for dw in [0u32, 1, 2, 3] {
            for dh in [1u32, 2, 3] {
                let w = dw + 1;
                let mut s = Spec::new(w, dh, ct);
                s.filters = vec![0, 1, 2, 3, 4];
                let r = diff_png(&s.build(), &format!("outsize ct={ct} {w}x{dh}"));
                assert_ne!(
                    r.err.as_deref(),
                    Some("invalid image size found"),
                    "the out-size branch must be unreachable (ct={ct} {w}x{dh})"
                );
            }
        }
    }
}

/// Row 21 — `cp_inflate` fails inside `load_png_mem`; the inner reason is
/// overwritten by `"DEFLATE algorithm failed"`.
#[test]
fn row_21_deflate_failed() {
    // BTYPE = 3 (0b11) with BFINAL = 1 -> byte 0b111 = 0x07
    let mut s = Spec::new(4, 4, 6);
    s.raw_zlib = Some(zlib_wrap(&[0x07, 0, 0, 0], 0x78, 0x9C, 0));
    expect_err(&s.build(), "DEFLATE algorithm failed", "btype=3 inside png");

    // a stored block whose LEN/NLEN are not complements
    let mut bw = BitWriter::new();
    write_stored_block(&mut bw, &[1, 2, 3, 4, 5, 6, 7, 8], true, Some(0x1234));
    let mut s = Spec::new(4, 4, 6);
    s.raw_zlib = Some(zlib_wrap(&bw.finish(), 0x78, 0x9C, 0));
    expect_err(&s.build(), "DEFLATE algorithm failed", "bad NLEN inside png");

    // output longer than the image buffer
    let mut s = Spec::new(2, 2, 6);
    let big = vec![0x41u8; 4096];
    s.raw_zlib = Some(zlib_wrap(&deflate_literals_fixed(&big), 0x78, 0x9C, 0));
    expect_err(&s.build(), "DEFLATE algorithm failed", "overlong stream");
}

/// Rows 22-23 — `cp_unfilter` rejects a filter byte > 4, separately for row 0
/// (the special-cased first row) and for rows `y >= 1`.
#[test]
fn row_22_23_bad_filter_byte() {
    for f in 5..=255u8 {
        for ct in COLOUR_TYPES {
            // row 22: only row 0 is bad
            let mut s = Spec::new(3, 3, ct);
            s.filters = vec![f, 0, 0];
            expect_err(
                &s.build(),
                "invalid filter byte found",
                &format!("row0 filter={f} ct={ct}"),
            );
            // row 23: row 0 fine, row 1 bad
            let mut s = Spec::new(3, 3, ct);
            s.filters = vec![0, f, 0];
            expect_err(
                &s.build(),
                "invalid filter byte found",
                &format!("row1 filter={f} ct={ct}"),
            );
            // and the last row
            let mut s = Spec::new(3, 3, ct);
            s.filters = vec![0, 0, f];
            expect_err(
                &s.build(),
                "invalid filter byte found",
                &format!("row2 filter={f} ct={ct}"),
            );
        }
        if f > 12 {
            continue; // the sweep above is expensive; sample the rest
        }
    }
    // 4 is the boundary that is accepted
    for ct in COLOUR_TYPES {
        let mut s = Spec::new(3, 3, ct);
        s.filters = vec![4];
        expect_ok(&s.build(), &format!("filter=4 ct={ct}"));
    }
}

/// Row 24 — colour type 3 without a PLTE chunk.
#[test]
fn row_24_indexed_without_plte() {
    let mut s = Spec::new(4, 4, 3);
    s.plte = None;
    expect_err(
        &s.build(),
        "color type of indexed requires a PLTE chunk",
        "indexed, no PLTE",
    );
    // a PLTE that arrives only *after* the IDATs is not found either
    let mut s = Spec::new(4, 4, 3);
    s.order = Order::PlteAfterIdat;
    let r = diff_png(&s.build(), "indexed, PLTE after IDAT");
    assert!(!r.ok);
    // a zero-length PLTE *is* found (minlen == 0), so it is accepted here
    let mut s = Spec::new(4, 4, 3);
    s.plte = Some(vec![]);
    let r = diff_png(&s.build(), "indexed, empty PLTE");
    assert_ne!(
        r.err.as_deref(),
        Some("color type of indexed requires a PLTE chunk"),
        "cp_find(\"PLTE\", 0) accepts a zero-length chunk"
    );
}

// ===========================================================================
// B. cp_inflate rejections — ERRORS.md rows 25-32
// ===========================================================================

fn expect_inflate_err(deflate: &[u8], out_bytes: i32, reason: &str, label: &str) {
    for align in 0..4usize {
        let r = diff_inflate_full(
            deflate,
            deflate.len() as i32,
            align,
            out_bytes,
            64,
            &format!("{label} align={align}"),
        );
        assert_eq!(r.rc, 0, "[{label}] expected failure");
        assert_eq!(r.err.as_deref(), Some(reason), "[{label}] wrong reason");
    }
}

/// Row 25 — stored block with `LEN != (uint16_t)~NLEN`.
#[test]
fn row_25_stored_len_nlen_mismatch() {
    for len in [0usize, 1, 3, 8, 64] {
        let data: Vec<u8> = (0..len).map(|i| i as u8).collect();
        for bad in [0x0000u16, 0xFFFF, 0x1234, 0x5555] {
            if bad == !(len as u16) {
                continue;
            }
            let mut bw = BitWriter::new();
            write_stored_block(&mut bw, &data, true, Some(bad));
            expect_inflate_err(
                &bw.finish(),
                4096,
                "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
                &format!("LEN={len} NLEN={bad:#06X}"),
            );
        }
    }
}

/// Row 26 — `!(s->bits_left / 8 <= LEN)`: more input remains than the stored
/// block declares.  This fires for *every* multi-stored-block stream.
#[test]
fn row_26_stored_extends_beyond_input() {
    // two stored blocks
    for (a, b) in [(1usize, 1usize), (4, 4), (16, 3), (100, 100)] {
        let mut bw = BitWriter::new();
        let da: Vec<u8> = (0..a).map(|i| i as u8).collect();
        let db: Vec<u8> = (0..b).map(|i| (i * 3) as u8).collect();
        write_stored_block(&mut bw, &da, false, None);
        write_stored_block(&mut bw, &db, true, None);
        expect_inflate_err(
            &bw.finish(),
            4096,
            "Stored block extends beyond end of input stream.",
            &format!("two stored blocks {a}+{b}"),
        );
    }
    // one stored block plus trailing bytes
    for extra in 1..=8usize {
        let data: Vec<u8> = (0..20).collect();
        let mut d = deflate_stored(&data);
        d.extend(std::iter::repeat(0xAAu8).take(extra));
        expect_inflate_err(
            &d,
            4096,
            "Stored block extends beyond end of input stream.",
            &format!("stored + {extra} trailing bytes"),
        );
    }
}

/// Row 27 / row 31 — a literal that does not fit into `out_bytes`.
#[test]
fn row_27_31_literal_overflows_output() {
    for n in 1..=8usize {
        let data: Vec<u8> = (0..n).map(|i| 0x40 + i as u8).collect();
        let d = deflate_literals_fixed(&data);
        for out_bytes in 0..n as i32 {
            expect_inflate_err(
                &d,
                out_bytes,
                "Attempted to overwrite out buffer while outputting a symbol.",
                &format!("n={n} out_bytes={out_bytes}"),
            );
        }
        // exactly enough is fine
        let r = diff_inflate(&d, 0, n as i32, "exact fit");
        assert_eq!(r.rc, 1, "{:?}", r.err);
    }
    // row 31: out_bytes == 0
    expect_inflate_err(
        &deflate_literals_fixed(&[0x5A]),
        0,
        "Attempted to overwrite out buffer while outputting a symbol.",
        "out_bytes=0",
    );
}

/// Row 28 — a back-reference that points before the start of `out`.
#[test]
fn row_28_backwards_distance_before_begin() {
    for (nlit, dist) in [(0u32, 1u32), (1, 2), (1, 5), (3, 4), (3, 300), (8, 32768)] {
        let mut bw = BitWriter::new();
        let mut toks: Vec<Tok> = (0..nlit).map(|i| Tok::Lit(0x60 + i as u8)).collect();
        toks.push(Tok::Match { len: 3, dist });
        write_fixed_block(&mut bw, &toks, true);
        expect_inflate_err(
            &bw.finish(),
            4096,
            "Attempted to write before out buffer (invalid backwards distance).",
            &format!("nlit={nlit} dist={dist}"),
        );
    }
}

/// Row 29 — a length/distance pair whose expansion does not fit `out_bytes`.
#[test]
fn row_29_string_overflows_output() {
    for len in [3u32, 4, 17, 258] {
        let mut bw = BitWriter::new();
        let toks = vec![
            Tok::Lit(1),
            Tok::Lit(2),
            Tok::Lit(3),
            Tok::Lit(4),
            Tok::Match { len, dist: 2 },
        ];
        write_fixed_block(&mut bw, &toks, true);
        let d = bw.finish();
        // 4 literals fit, the string does not
        for out_bytes in 4..(4 + len as i32) {
            expect_inflate_err(
                &d,
                out_bytes,
                "Attempted to overwrite out buffer while outputting a string.",
                &format!("len={len} out_bytes={out_bytes}"),
            );
        }
        let r = diff_inflate(&d, 0, 4 + len as i32, "string exact fit");
        assert_eq!(r.rc, 1, "{:?}", r.err);
    }
}

/// Row 30 — `BTYPE == 3`.
#[test]
fn row_30_unknown_block_type() {
    // BFINAL=1, BTYPE=11 -> low three bits 0b111
    for tail in [0u8, 0xFF, 0x5A] {
        let d = vec![0x07u8, tail, tail, tail, tail, tail, tail, tail];
        expect_inflate_err(
            &d,
            4096,
            "Detected unknown block type within input stream.",
            &format!("btype=3 tail={tail:#04X}"),
        );
    }
    // BTYPE=3 in a *later* block
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, &[Tok::Lit(9), Tok::Lit(8)], false);
    bw.bits(1, 1);
    bw.bits(3, 2);
    bw.raw_pad(8);
    expect_inflate_err(
        &bw.finish(),
        4096,
        "Detected unknown block type within input stream.",
        "btype=3 in second block",
    );
}

// ===========================================================================
// C. live assert()s — ERRORS.md rows 32-42 (SIGABRT, compared via fork())
// ===========================================================================

fn expect_abort(deflate: &[u8], in_bytes: i32, align: usize, out_bytes: i32, expr: &str, label: &str) {
    let r = diff_inflate_abort(deflate, in_bytes, align, out_bytes, label);
    assert!(
        r.aborted(),
        "[{label}] expected SIGABRT from the C library, got {:?} (stderr {:?})",
        r.outcome,
        r.stderr
    );
    assert_eq!(
        r.assertion().as_deref(),
        Some(expr),
        "[{label}] a different assertion fired: {:?}",
        r.stderr
    );
}

/// Row 33 — `cp_ptr`: `assert(!(s->bits_left & 7))`.
///
/// See `tests/discover.rs::cp_ptr_assert_stream` for the derivation: a refill
/// from `s->final_word` adds `bits_left` (rather than 32) to `count`, after
/// which `bits_left` at `cp_ptr` is `-c0 (mod 8)`, where `c0` is `count` at the
/// refill.  `c0 = 9`, `last_bytes = 3`, `word_count = 2` is feasible.
#[test]
fn row_33_cp_ptr_alignment_assert() {
    let mut bw = BitWriter::new();
    bw.bits(0, 1); // BFINAL = 0
    bw.bits(1, 2); // BTYPE  = 01 (fixed)
    bw.code(0x30, 8); // literal 0   (8 bits)
    bw.code(0x31, 8); // literal 1   (8 bits)
    for _ in 0..4 {
        bw.code(0x190 + (200 - 144), 9); // literal 200 (9 bits)
    }
    bw.code(0x00, 7); // end of block -> the final_word refill happens here
    bw.bits(1, 1); // BFINAL = 1
    bw.bits(0, 2); // BTYPE  = 00 (stored)
    bw.bits(0xFFFF, 16); // LEN
    bw.bits(0, 7); // 7 real NLEN bits; the top 9 are phantom zeros
    let d = bw.finish();
    assert_eq!(d.len(), 11);
    expect_abort(&d, 11, 0, 4096, "!(s->bits_left & 7)", "row 33");
    // the same bytes at other alignments run out of input earlier
    for align in 1..4usize {
        expect_abort(&d, 11, align, 4096, "s->bits_left > 0", &format!("row 33 align={align}"));
    }
}

/// Row 34 — `cp_peak_bits`: `assert(s->word_index <= s->word_count)`.
///
/// Unreachable: the enclosing `if (s->word_index < s->word_count)` guarantees
/// `word_index + 1 <= word_count` after the increment.  The check is present in
/// the Rust translation for completeness; this test documents the reasoning and
/// verifies that no input in the (large) randomised corpus ever trips it.
#[test]
fn row_34_peak_bits_assert_unreachable() {
    let mut rng = Rng::new(0x3401);
    for _ in 0..40 {
        let n = rng.range(1, 40) as usize;
        let data = rng.bytes(n);
        // well-formed streams of every flavour, plus truncations of them
        let mut streams = vec![
            deflate_literals_fixed(&data),
            deflate_stored(&data),
            deflate_flate2(&data, 6),
        ];
        let base = streams[0].clone();
        for cut in [1usize, 2, base.len() / 2] {
            if cut < base.len() {
                streams.push(base[..base.len() - cut].to_vec());
            }
        }
        for d in streams {
            for align in 0..4usize {
                let r = diff_inflate_abort(&d, d.len() as i32, align, 256, "row 34 sweep");
                if let Some(a) = r.assertion() {
                    assert_ne!(
                        a, "s->word_index <= s->word_count",
                        "this assertion was believed unreachable"
                    );
                }
            }
        }
    }
}

/// Row 35 — `cp_consume_bits`: `assert(s->count >= num_bits_to_read)`.
#[test]
fn row_35_consume_bits_assert() {
    // discovered by tests/discover.rs: a stored block header that needs 32 bits
    // for LEN/NLEN while only 8 are buffered
    expect_abort(&[0x23], 1, 0, 64, "s->count >= num_bits_to_read", "row 35 [23]");
    expect_abort(&[0x01, 0x00], 2, 0, 64, "s->count >= num_bits_to_read", "row 35 [01 00]");
}

/// Row 36 — `cp_read_bits`: `assert(s->bits_left > 0)`.
#[test]
fn row_36_bits_left_assert() {
    // no input at all
    for align in 0..4usize {
        expect_abort(&[], 0, align, 64, "s->bits_left > 0", &format!("row 36 in=0 a={align}"));
    }
    // a truncated fixed block: BFINAL=0 so another header read is attempted
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, &[Tok::Lit(1), Tok::Lit(2)], false);
    let d = bw.finish();
    expect_abort(&d, d.len() as i32, 0, 64, "s->bits_left > 0", "row 36 missing final block");
}

/// Row 37 — `cp_read_bits`: `assert(!cp_would_overflow(s, num_bits_to_read))`.
#[test]
fn row_37_would_overflow_assert() {
    // first_bytes = 1 (align 3), in_bytes = 2: after BFINAL/BTYPE/align the
    // buffer is empty but 8 real bits remain, so the 16-bit LEN read overflows
    expect_abort(
        &[0x00, 0x25],
        2,
        3,
        64,
        "!cp_would_overflow(s, num_bits_to_read)",
        "row 37 [00 25] align=3",
    );
    expect_abort(
        &[0x01, 0x00],
        2,
        3,
        64,
        "!cp_would_overflow(s, num_bits_to_read)",
        "row 37 [01 00] align=3",
    );
}

/// Row 38 — `cp_read_bits`: `assert(num_bits_to_read <= 32)`.  Only reachable
/// by writing into the public writable extra-bit tables (which a real consumer
/// can do — they are exported, non-const globals).
#[test]
fn row_38_num_bits_range_assert() {
    let p = pair();
    let toks = vec![Tok::Lit(b'q'), Tok::Match { len: 3, dist: 1 }];
    let d = deflate_fixed(&toks);
    let (mut buf, off) = aligned_input(&d, 0);
    let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut std::ffi::c_void;
    for (which, poke) in [
        ("len", 33u8),
        ("len", 64),
        ("len", 255),
        ("dist", 33),
        ("dist", 200),
    ] {
        let go = |im: &'static Impl| {
            let t = if which == "len" {
                im.len_extra_bits
            } else {
                im.dist_extra_bits
            };
            unsafe { *t = poke };
            let r = call_inflate(im, ptr, d.len() as i32, 4096, 4096 + 64);
            r.rc.to_le_bytes().to_vec()
        };
        // the poke happens inside the fork, so the parent's tables stay pristine
        let a = run_forked_capture(|| go(&p.c));
        let b = run_forked_capture(|| go(&p.rust));
        assert_eq!(a.outcome, b.outcome, "row 38 {which}={poke}: {:?}", a.stderr);
        assert!(a.aborted(), "row 38 {which}={poke}: {:?}", a.outcome);
        assert_eq!(a.assertion().as_deref(), Some("num_bits_to_read <= 32"));
    }
    // the parent's tables must be untouched by the forked pokes
    let a = unsafe { std::slice::from_raw_parts(p.c.len_extra_bits, 31) };
    let b = unsafe { std::slice::from_raw_parts(p.rust.len_extra_bits, 31) };
    assert_eq!(a, b);
    assert_eq!(a[0], 0);
}

/// Row 39 — `assert(num_bits_to_read >= 0)`.
///
/// Unreachable: every argument is either a literal (`1`, `2`, `3`, `4`, `5`,
/// `7`, `16`), a `uint8_t` table entry (`0..=255`), or `s->count & 7`, and
/// `s->count` can never become negative because `cp_consume_bits` asserts
/// `count >= num` first.  Documented rather than tested; the Rust translation
/// carries the identical check.
#[test]
fn row_39_negative_num_bits_unreachable() {
    // `s->count & 7` is the only non-constant, non-table argument.  Sweep the
    // stored-block path (the only caller of `cp_read_bits(s, s->count & 7)`)
    // and prove the assertion never fires.
    let mut rng = Rng::new(0x3901);
    for _ in 0..120 {
        let n = rng.range(0, 40) as usize;
        let data = rng.bytes(n);
        let mut d = deflate_stored(&data);
        let extra = rng.below(4) as usize;
        d.extend(rng.bytes(extra));
        for align in 0..4usize {
            let r = diff_inflate_abort(&d, d.len() as i32, align, 4096, "row 39 sweep");
            if let Some(a) = r.assertion() {
                assert_ne!(a, "num_bits_to_read >= 0");
            }
        }
    }
}

/// Row 40 — `assert(s->count <= 64)`.
///
/// Unreachable: `count` only grows in `cp_peak_bits`, and only while
/// `count < num_bits_to_read <= 32` (`num_bits_to_read` is at most 16 for every
/// call that can refill).  The `words[]` branch adds 32 (so `count < 48`); the
/// `final_word` branch adds `bits_left = 8*last_bytes + count <= 24 + 15`, so
/// `count <= 54`.  Documented; swept below.
#[test]
fn row_40_count_bound_unreachable() {
    let mut rng = Rng::new(0x4001);
    for _ in 0..120 {
        let n = rng.range(1, 24) as usize;
        let d = rng.bytes(n);
        for align in 0..4usize {
            for out_bytes in [0i32, 1, 4096] {
                let r = diff_inflate_abort(&d, n as i32, align, out_bytes, "row 40 sweep");
                if let Some(a) = r.assertion() {
                    assert_ne!(a, "s->count <= 64");
                }
            }
        }
    }
}

/// Row 41 — `cp_build`: `assert(len < 16)`.  A code length >= 16 can only come
/// from the public writable `cp_fixed_table`.
#[test]
fn row_41_code_length_assert() {
    let p = pair();
    let d = deflate_fixed(&[Tok::Lit(b'x')]);
    let (mut buf, off) = aligned_input(&d, 0);
    let ptr = unsafe { buf.as_mut_ptr().add(off) } as *mut std::ffi::c_void;
    for poke in [16u8, 17, 20, 31, 40, 47] {
        let go = |im: &'static Impl| {
            unsafe { *im.fixed_table = poke };
            let r = call_inflate(im, ptr, d.len() as i32, 64, 128);
            r.rc.to_le_bytes().to_vec()
        };
        let a = run_forked_capture(|| go(&p.c));
        let b = run_forked_capture(|| go(&p.rust));
        assert_eq!(a.outcome, b.outcome, "row 41 poke={poke}: {:?}", a.stderr);
        assert!(a.aborted(), "row 41 poke={poke}: {:?}", a.outcome);
        assert_eq!(a.assertion().as_deref(), Some("len < 16"));
    }
    // parent's table untouched
    let a = unsafe { std::slice::from_raw_parts(p.c.fixed_table, 320) };
    let b = unsafe { std::slice::from_raw_parts(p.rust.fixed_table, 320) };
    assert_eq!(a, b);
    assert_eq!(a[0], 8);
}

/// Row 42 — `cp_decode`: `assert((search >> len) == (key >> len))` — the peeked
/// bits match no code in the tree.
#[test]
fn row_42_decode_assert() {
    expect_abort(
        &[0x1C, 0x41, 0x66, 0x8B, 0xB0],
        5,
        0,
        64,
        "(search >> len) == (key >> len)",
        "row 42 discovered case",
    );
    // A dynamic block whose *code-length* alphabet is empty: all HCLEN entries
    // are 0, so `cp_build` returns 0 and `cp_decode(s, s->len, 0)` reads
    // `tree[-1]` == `s->dst[31]` == 0 (the state is calloc'ed).  `key & 0xF` is
    // then 0, so `len == 32`; gcc masks the shift to 0 and the assertion
    // degenerates to `search == key == 0`, which can never hold because
    // `search` always has its low 16 bits set.
    let mut bw = BitWriter::new();
    bw.bits(1, 1); // BFINAL
    bw.bits(2, 2); // BTYPE  = 10 (dynamic)
    bw.bits(0, 5); // HLIT   = 0 -> nlit = 257
    bw.bits(0, 5); // HDIST  = 0 -> ndst = 1
    bw.bits(0, 4); // HCLEN  = 0 -> nlen = 4
    for _ in 0..4 {
        bw.bits(0, 3); // all four transmitted code lengths are 0
    }
    bw.raw_pad(35); // pad the stream out to 8 bytes
    let d = bw.finish();
    assert_eq!(d.len(), 8);
    expect_abort(
        &d,
        8,
        0,
        4096,
        "(search >> len) == (key >> len)",
        "row 42 empty code-length alphabet",
    );
}

/// Row 32 — `cp_inflate` with an input too small to even read the block header.
#[test]
fn row_32_input_too_small() {
    for align in 0..4usize {
        expect_abort(&[], 0, align, 64, "s->bits_left > 0", &format!("in=0 a={align}"));
    }
}

// ===========================================================================
// D. generic FFI boundary conditions — ERRORS.md rows 43-52
// ===========================================================================

/// Row 43 — `png_length == 0` (and other tiny lengths) on an otherwise valid
/// PNG.  The signature `memcmp` reads 8 bytes regardless of `png_length`, so a
/// valid file with `png_length == 0` gets past the signature and then fails in
/// `cp_chunk` (`png->p + 25 <= png->end` is false).
#[test]
fn row_43_zero_png_length() {
    let good = Spec::new(4, 4, 6).build();
    for len in 0..33i32 {
        let r = diff_png_len(&good, len, &format!("png_length={len}"));
        assert!(!r.ok, "png_length={len} must not decode");
        assert_eq!(r.err.as_deref(), Some("unable to find IHDR chunk"));
    }
    // 33 bytes = signature + IHDR: the IHDR is found, but no IDAT is in range
    let r = diff_png_len(&good, 33, "png_length=33");
    assert!(!r.ok);
    assert_eq!(
        r.err.as_deref(),
        Some("corrupt zlib structure in DEFLATE stream")
    );
    // every length from 34 up to the real size: both must agree (some of these
    // truncate the IDAT stream, which the C code may abort on, so compare
    // termination status too)
    for len in 34..good.len() as i32 {
        let r = diff_png_abort(&good, len, &format!("png_length={len}"));
        let _ = r;
    }
    // an all-padding buffer really does fail the signature check
    let empty: Vec<u8> = Vec::new();
    expect_err(
        &empty,
        "incorrect file signature (is this a png file?)",
        "empty buffer",
    );
}

/// Row 44 — negative `png_length` (`png.end < png.p`).
#[test]
fn row_44_negative_png_length() {
    let good = Spec::new(4, 4, 6).build();
    for len in [-1i32, -2, -8, -1024, i32::MIN, i32::MIN + 1] {
        let r = diff_png_len(&good, len, &format!("png_length={len}"));
        assert!(!r.ok);
        assert_eq!(r.err.as_deref(), Some("unable to find IHDR chunk"));
    }
    // ...and with a bad signature the length is never even looked at
    let mut bad = good.clone();
    bad[0] ^= 1;
    for len in [-1i32, 0, i32::MIN, i32::MAX] {
        let r = diff_png_len(&bad, len, &format!("bad sig png_length={len}"));
        assert_eq!(
            r.err.as_deref(),
            Some("incorrect file signature (is this a png file?)")
        );
    }
}

/// Row 45 — `png_length` far larger than the real buffer, with a bad signature
/// (so nothing beyond the first 8 bytes is touched).
#[test]
fn row_45_oversized_png_length() {
    let mut bad = Spec::new(4, 4, 6).build();
    bad[3] ^= 0xFF;
    for len in [i32::MAX, i32::MAX - 1, 1 << 20, 1 << 28] {
        let r = diff_png_len(&bad, len, &format!("oversized {len}"));
        assert_eq!(
            r.err.as_deref(),
            Some("incorrect file signature (is this a png file?)")
        );
    }
}

/// Row 46 — a valid PNG with trailing garbage and an oversized `png_length`.
#[test]
fn row_46_trailing_garbage() {
    let mut rng = Rng::new(0x4601);
    for ct in COLOUR_TYPES {
        let mut s = Spec::new(5, 5, ct);
        s.filters = vec![0, 1, 2, 3, 4];
        s.payload = rng.bytes(5 * 5 * bpp_of(ct) + 3);
        let base = s.build();
        for extra in [0usize, 1, 3, 12, 64, 1000] {
            let mut png = base.clone();
            png.extend(rng.bytes(extra));
            let r = diff_png(&png, &format!("trailing {extra} ct={ct}"));
            assert!(r.ok, "err={:?}", r.err);
            assert_eq!(r.pixels, reference_rgba(&s).unwrap());
        }
    }
}

/// Row 47 — `cp_inflate` with `out_bytes == 0` and with a negative `out_bytes`
/// (`out_end < out`).
#[test]
fn row_47_out_bytes_boundaries() {
    let d = deflate_literals_fixed(&[1, 2, 3, 4]);
    for out_bytes in [0i32, -1, -8, -4096] {
        for align in 0..4usize {
            let r = diff_inflate_full(
                &d,
                d.len() as i32,
                align,
                out_bytes,
                64,
                &format!("out_bytes={out_bytes} a={align}"),
            );
            assert_eq!(r.rc, 0);
            assert_eq!(
                r.err.as_deref(),
                Some("Attempted to overwrite out buffer while outputting a symbol.")
            );
        }
    }
    // an empty stored block emits nothing, so out_bytes == 0 succeeds
    let d0 = deflate_stored(&[]);
    let r = diff_inflate_full(&d0, d0.len() as i32, 3, 0, 64, "empty stored, out=0");
    assert_eq!(r.rc, 1, "{:?}", r.err);
}

/// Row 48 — negative `in_bytes` (`bits_left < 0`).
#[test]
fn row_48_negative_in_bytes() {
    let d = deflate_literals_fixed(&[1, 2, 3, 4]);
    for in_bytes in [-1i32, -4, -1024] {
        for align in 0..4usize {
            expect_abort(
                &d,
                in_bytes,
                align,
                64,
                "s->bits_left > 0",
                &format!("in_bytes={in_bytes} a={align}"),
            );
        }
    }
    // `s->bits_left = in_bytes * 8` overflows for very large magnitudes, so a
    // different assertion fires -- both libraries must still agree, and the
    // *same* assertion must be the one reported.
    // For large magnitudes `s->bits_left = in_bytes * 8` overflows and
    // `s->final_word` is filled from `in[in_bytes - last_bytes + i]`, i.e. from
    // far *before* the buffer -- so the process may die with SIGSEGV instead of
    // SIGABRT.  Both libraries must agree on which (that is what
    // `diff_inflate_abort` asserts); the exact signal is memory-layout
    // dependent and is therefore not pinned here.
    for in_bytes in [i32::MIN, i32::MIN + 8, i32::MIN + 1, -0x2000_0000, -100_000] {
        for align in 0..4usize {
            let r = diff_inflate_abort(
                &d,
                in_bytes,
                align,
                64,
                &format!("in_bytes={in_bytes} a={align}"),
            );
            assert!(
                matches!(r.outcome, Outcome::Signaled(_)),
                "in_bytes={in_bytes}: expected a fatal signal, got {:?}",
                r.outcome
            );
        }
    }
}

/// Row 49 — every input alignment x input-tail residue (also covered
/// exhaustively by `tests/inflate.rs` rows 1-9).
#[test]
fn row_49_alignment_matrix() {
    let mut rng = Rng::new(0x4901);
    let mut seen = [[0u32; 4]; 4];
    for _ in 0..200 {
        let n = rng.range(1, 60) as usize;
        let data = rng.bytes(n);
        let d = deflate_literals_fixed(&data);
        for align in 0..4usize {
            let first_bytes = (4 - align) % 4;
            if d.len() < first_bytes {
                continue;
            }
            seen[align][(d.len() - first_bytes) % 4] += 1;
            let r = diff_inflate(&d, align, n as i32, "row 49");
            assert_eq!(r.rc, 1, "{:?}", r.err);
            assert_eq!(&r.out[..n], &data[..]);
        }
    }
    for a in 0..4 {
        for t in 0..4 {
            assert!(seen[a][t] > 0, "align={a} tail={t} not covered");
        }
    }
}

/// Row 50 — out-of-range "enum" values crossing the FFI boundary: one step
/// outside every documented IHDR range.
#[test]
fn row_50_out_of_range_enums() {
    // bit depth: legal PNG depths 1,2,4,8,16 -- only 8 is supported
    for bd in [0u8, 1, 2, 3, 4, 5, 7, 9, 15, 16, 17, 32, 128, 255] {
        if bd == 8 {
            continue;
        }
        let mut s = Spec::new(4, 4, 6);
        s.bit_depth = bd;
        expect_err(&s.build(), "only bit-depth of 8 is supported", &format!("bd={bd}"));
    }
    // colour type: one step outside each valid value
    for ct in [1u8, 5, 7, 8, 9, 127, 128, 255] {
        let mut s = Spec::new(4, 4, 6);
        s.ihdr_ct = Some(ct);
        expect_err(&s.build(), "unknown color type", &format!("ct={ct}"));
    }
    // compression / filter / interlace: 1 (and 2 for interlace, the legal
    // Adam7 value) are all rejected
    for v in [1u8, 2, 3, 255] {
        let mut s = Spec::new(4, 4, 6);
        s.comp = v;
        expect_err(
            &s.build(),
            "only standard compression DEFLATE is supported",
            &format!("comp={v}"),
        );
        let mut s = Spec::new(4, 4, 6);
        s.filt = v;
        expect_err(
            &s.build(),
            "only standard adaptive filtering is supported",
            &format!("filt={v}"),
        );
        let mut s = Spec::new(4, 4, 6);
        s.inter = v;
        expect_err(&s.build(), "interlacing is not supported", &format!("inter={v}"));
    }
    // filter byte: 5 is one step past the valid 0..4 range
    for f in [5u8, 6, 128, 255] {
        let mut s = Spec::new(4, 4, 6);
        s.filters = vec![f];
        expect_err(&s.build(), "invalid filter byte found", &format!("filter={f}"));
    }
    // DEFLATE block type: 3 is one step past 0..2
    expect_inflate_err(
        &[0x07u8, 0, 0, 0, 0, 0, 0, 0],
        64,
        "Detected unknown block type within input stream.",
        "btype=3",
    );
}

/// Row 51 — `cp_get_alpha_for_indexed_image` boundaries: `trns == NULL`,
/// `index >= trns_len`, `trns_len == 0` and `trns_len > 256`.
#[test]
fn row_51_trns_boundaries() {
    let mut rng = Rng::new(0x5101);
    for &tl in &[0usize, 1, 128, 255, 256, 257, 1024] {
        let mut s = Spec::new(16, 16, 3);
        // indices spanning the whole 0..=255 range
        s.payload = (0..256u32).map(|i| i as u8).collect();
        s.plte = Some(rng.bytes(256 * 3));
        s.trns = Some(rng.bytes(tl));
        let r = diff_png(&s.build(), &format!("trns_len={tl}"));
        assert!(r.ok, "err={:?}", r.err);
        assert_eq!(r.pixels, reference_rgba(&s).unwrap());
        // every index >= trns_len must have alpha 255
        for (i, px) in r.pixels.chunks(4).enumerate() {
            let idx = (i % 256) as usize;
            if idx >= tl {
                assert_eq!(px[3], 255, "trns_len={tl} index={idx}");
            }
        }
    }
    // no tRNS at all -> alpha 255 everywhere
    let mut s = Spec::new(16, 16, 3);
    s.payload = (0..256u32).map(|i| i as u8).collect();
    s.plte = Some(rng.bytes(256 * 3));
    s.trns = None;
    let r = diff_png(&s.build(), "no trns");
    assert!(r.ok);
    assert!(r.pixels.chunks(4).all(|p| p[3] == 255));
}

/// Row 52 — a PLTE shorter than the largest index used (`plte[c*3]` reads past
/// the chunk).  Both libraries read the identical padded buffer, so the result
/// must still be identical.
#[test]
fn row_52_short_plte() {
    let mut rng = Rng::new(0x5201);
    for &pl in &[0usize, 1, 2, 3, 4, 5, 6, 30, 300, 765, 766, 767, 768] {
        let mut s = Spec::new(16, 16, 3);
        s.payload = (0..256u32).map(|i| i as u8).collect();
        s.plte = Some(rng.bytes(pl));
        s.trns = None;
        let r = diff_png(&s.build(), &format!("plte_len={pl}"));
        assert!(r.ok, "err={:?}", r.err);
        if pl >= 768 {
            assert_eq!(r.pixels, reference_rgba(&s).unwrap());
        }
    }
}
