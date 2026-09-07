//! Phase C — error-path differential tests. One test per ERRORS.md row.
//! Every case asserts the C's *specific* error identity (return sentinel plus
//! the exact `cp_error_reason` string, read from each `.so`'s own exported
//! global) and then asserts the Rust `.so` produces the same identity.

mod common;

use common::deflate::*;
use common::*;
use std::os::raw::{c_int, c_void};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

/// `cp_inflate` must return 0 with exactly `err` on the C side, then Rust must
/// agree (return value, output bytes and error string).
fn inflate_err(label: &str, stream: &[u8], out_bytes: c_int, out_cap: usize, err: &str) {
    let r = c_inflate(stream, stream.len() as c_int, out_bytes, out_cap.max(1));
    assert_eq!(r.ret, 0, "[{label}] C unexpectedly succeeded");
    assert_eq!(
        r.error.as_deref(),
        Some(err),
        "[{label}] C hit a different error branch than intended"
    );
    diff_inflate_full(
        label,
        stream,
        stream.len() as c_int,
        0,
        out_bytes,
        out_cap.max(1),
    );
}

/// `load_png_mem` must return `pix == NULL` with exactly `err` on the C side,
/// then Rust must agree on `w`, `h`, null-ness and the message.
fn png_err(label: &str, png: &[u8], err: &str) {
    png_err_len(label, png, png.len() as c_int, err)
}

fn png_err_len(label: &str, png: &[u8], length: c_int, err: &str) {
    let r = c_png(png, length);
    assert!(
        r.null,
        "[{label}] C unexpectedly succeeded (w={} h={})",
        r.w, r.h
    );
    assert_eq!(
        r.error.as_deref(),
        Some(err),
        "[{label}] C hit a different error branch than intended"
    );
    diff_png_len(label, png, length);
}

fn mk_png(chunks: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
    let mut v = PNG_SIG.to_vec();
    for (k, d) in chunks {
        v.extend_from_slice(&chunk(k, d));
    }
    v
}

/// A minimal but otherwise valid 2x2 RGBA PNG, used as the base for mutation.
fn base_rgba(w: usize, h: usize) -> PngSpec {
    let data: Vec<u8> = (0..w * h * 4).map(|i| (i * 13 + 5) as u8).collect();
    let mut s = PngSpec::new(w as u32, h as u32, 6);
    s.raw = encode_scanlines(w, h, 4, &data, &vec![0u8; h]);
    s.deflate = DeflateMode::FixedLz;
    s
}

const E_SIG: &str = "incorrect file signature (is this a png file?)";
const E_IHDR: &str = "unable to find IHDR chunk";
const E_DEPTH: &str = "only bit-depth of 8 is supported";
const E_CT: &str = "unknown color type";
const E_W: &str = "invalid IHDR chunk found, image width was less than 1";
const E_H: &str = "invalid IHDR chunk found, image height was less than 1";
const E_BIG: &str = "image too large";
const E_COMP: &str = "only standard compression DEFLATE is supported";
const E_FILT: &str = "only standard adaptive filtering is supported";
const E_IL: &str = "interlacing is not supported";
const E_ZLIB: &str = "corrupt zlib structure in DEFLATE stream";
const E_CM: &str = "only zlib compression method (RFC 1950) is supported";
const E_WIN: &str = "innapropriate window size detected";
const E_DICT: &str = "preset dictionary is present and not supported";
const E_DEFL: &str = "DEFLATE algorithm failed";
const E_FBYTE: &str = "invalid filter byte found";
const E_PLTE: &str = "color type of indexed requires a PLTE chunk";

// ===========================================================================
// A. cp_inflate / DEFLATE layer
// ===========================================================================

#[test]
fn a1_stored_len_nlen_not_complements() {
    // bfinal=1, btype=0, then LEN=3 / NLEN=0 (not complements)
    let mut s = vec![0x01u8];
    s.extend_from_slice(&3u16.to_le_bytes());
    s.extend_from_slice(&0u16.to_le_bytes());
    s.extend_from_slice(&[0xAA, 0xBB, 0xCC]);
    inflate_err(
        "A1 LEN/NLEN not complements",
        &s,
        16,
        16,
        "Failed to find LEN and NLEN as complements within stored (uncompressed) stream.",
    );
}

#[test]
fn a2_stored_block_extends_beyond_input() {
    // valid complements, but more input remains than LEN says
    for (len, extra) in [(0usize, 10usize), (1, 40), (4, 9), (16, 100)] {
        let mut s = vec![0x01u8];
        s.extend_from_slice(&(len as u16).to_le_bytes());
        s.extend_from_slice(&(!(len as u16)).to_le_bytes());
        s.extend(std::iter::repeat(0x5Au8).take(len + extra));
        inflate_err(
            &format!("A2 stored len={len} extra={extra}"),
            &s,
            512,
            512,
            "Stored block extends beyond end of input stream.",
        );
    }
}

#[test]
fn a3_literal_overflows_out_buffer() {
    let toks: Vec<Tok> = (0..8u8).map(Tok::Lit).collect();
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, &toks, true);
    let s = bw.finish();
    for out_bytes in [0i32, 1, 3, 7] {
        inflate_err(
            &format!("A3 literal overflow out_bytes={out_bytes}"),
            &s,
            out_bytes,
            out_bytes.max(1) as usize,
            "Attempted to overwrite out buffer while outputting a symbol.",
        );
    }
    // E8: negative out_bytes makes out_end < out, so even the first literal fails
    inflate_err(
        "E8 negative out_bytes",
        &s,
        -1,
        8,
        "Attempted to overwrite out buffer while outputting a symbol.",
    );
}

#[test]
fn a4_backwards_distance_before_out_buffer() {
    for dist in [1u32, 2, 5, 300] {
        // a match as the very first token: out - dist < begin
        let toks = vec![Tok::Match(3, dist)];
        let mut bw = BitWriter::new();
        write_fixed_block(&mut bw, &toks, true);
        let s = bw.finish();
        inflate_err(
            &format!("A4 dist={dist} at start"),
            &s,
            64,
            64,
            "Attempted to write before out buffer (invalid backwards distance).",
        );
    }
    // and a distance that reaches back past `begin` after some output
    let mut toks: Vec<Tok> = (0..4u8).map(Tok::Lit).collect();
    toks.push(Tok::Match(3, 5));
    let mut bw = BitWriter::new();
    write_fixed_block(&mut bw, &toks, true);
    let s = bw.finish();
    inflate_err(
        "A4 dist just past begin",
        &s,
        64,
        64,
        "Attempted to write before out buffer (invalid backwards distance).",
    );
}

#[test]
fn a5_string_overflows_out_buffer() {
    for (nlit, len, dist, out_bytes) in [
        (3usize, 10u32, 3u32, 5i32),
        (4, 258, 1, 10),
        (8, 20, 8, 9),
        (3, 3, 3, 3),
    ] {
        let mut toks: Vec<Tok> = (0..nlit).map(|i| Tok::Lit(i as u8)).collect();
        toks.push(Tok::Match(len, dist));
        let mut bw = BitWriter::new();
        write_fixed_block(&mut bw, &toks, true);
        let s = bw.finish();
        inflate_err(
            &format!("A5 nlit={nlit} len={len} dist={dist} out={out_bytes}"),
            &s,
            out_bytes,
            out_bytes as usize,
            "Attempted to overwrite out buffer while outputting a string.",
        );
    }
}

#[test]
fn a6_unknown_block_type() {
    // bfinal=1, btype=3 -> bits 1,1,1
    for extra in [1usize, 2, 3, 4, 8] {
        let mut s = vec![0x07u8];
        s.extend(std::iter::repeat(0u8).take(extra));
        inflate_err(
            &format!("A6 btype=3 len={}", s.len()),
            &s,
            64,
            64,
            "Detected unknown block type within input stream.",
        );
    }
}

#[test]
fn a6b_non_final_unknown_block_type() {
    // bfinal=0, btype=3 -> bits 0,1,1 = 0x06
    let s = vec![0x06u8, 0, 0, 0];
    inflate_err(
        "A6 non-final btype=3",
        &s,
        64,
        64,
        "Detected unknown block type within input stream.",
    );
}

#[test]
fn e10_null_out_with_empty_stored_block() {
    // out = NULL, out_bytes = 0, single empty stored block: both must succeed
    let mut s = vec![0x01u8];
    s.extend_from_slice(&0u16.to_le_bytes());
    s.extend_from_slice(&0xFFFFu16.to_le_bytes());
    let c = inflate_null_out(true, &s, s.len() as c_int, 0);
    let r = inflate_null_out(false, &s, s.len() as c_int, 0);
    assert_eq!(c.0, 1, "E10 expected C success, got {c:?}");
    assert_eq!(c, r, "E10 out=NULL: C={c:?} Rust={r:?}");
}

// E9 (in_bytes past/short of the real buffer) and D21b (single-byte corruption
// sweep) can make the C abort on a live assert() or segfault, so they live in
// the subprocess harness: see tests/isolated.rs.

// ===========================================================================
// B. cp_unfilter
// ===========================================================================

/// Builds a PNG whose scanline filter bytes are exactly `filters` (no forward
/// filtering, so invalid filter bytes can be injected).
fn png_with_raw_filters(w: usize, h: usize, ct: u8, filters: &[u8]) -> Vec<u8> {
    let bpp = bpp_for(ct);
    let stride = w * bpp;
    let mut raw = vec![0u8; (stride + 1) * h];
    for y in 0..h {
        raw[y * (stride + 1)] = filters[y];
        for x in 0..stride {
            raw[y * (stride + 1) + 1 + x] = ((y * stride + x) * 7) as u8;
        }
    }
    let mut s = PngSpec::new(w as u32, h as u32, ct);
    s.raw = raw;
    s.deflate = DeflateMode::FixedLz;
    if ct == 3 {
        s.plte = Some(vec![0x40u8; 768]);
    }
    s.build()
}

#[test]
fn b1_row0_filter_byte_out_of_range() {
    for f in [5u8, 6, 127, 128, 255] {
        for ct in [0u8, 2, 3, 4, 6] {
            let png = png_with_raw_filters(4, 3, ct, &[f, 0, 0]);
            png_err(&format!("B1 row0 filter={f} ct={ct}"), &png, E_FBYTE);
        }
    }
}

#[test]
fn b2_later_row_filter_byte_out_of_range() {
    for f in [5u8, 9, 200, 255] {
        for y in 1..4usize {
            let mut filters = vec![0u8; 4];
            filters[y] = f;
            let png = png_with_raw_filters(3, 4, 6, &filters);
            png_err(&format!("B2 row{y} filter={f}"), &png, E_FBYTE);
        }
    }
}

#[test]
fn e7_filter_byte_one_past_valid() {
    // exactly one step past the documented range, on row 0 and on a later row
    let png = png_with_raw_filters(2, 2, 0, &[5, 0]);
    png_err("E7 row0 filter=5", &png, E_FBYTE);
    let png = png_with_raw_filters(2, 2, 0, &[0, 5]);
    png_err("E7 row1 filter=5", &png, E_FBYTE);
    // filter == 4 is the last valid one and must succeed
    let png = png_with_raw_filters(2, 2, 0, &[4, 4]);
    diff_png("E7 filter=4 accepted", &png);
}

// ===========================================================================
// C. chunk walkers (via the public API)
// ===========================================================================

#[test]
fn c1_chunk_name_mismatch() {
    // first chunk after the signature is not IHDR -> cp_chunk returns NULL
    let png = mk_png(&[(b"IDAT", vec![0u8; 20]), (b"IEND", vec![])]);
    png_err("C1 first chunk not IHDR", &png, E_IHDR);
    let png = mk_png(&[(b"ihdr", ihdr_data(2, 2, 8, 6, 0, 0, 0))]);
    png_err("C1 lowercase ihdr", &png, E_IHDR);
}

#[test]
fn c2_chunk_len_below_minlen() {
    // IHDR requires minlen 13; declare 12 (and keep the file self-consistent)
    for declared in [0u32, 1, 12] {
        let mut v = PNG_SIG.to_vec();
        let body = ihdr_data(2, 2, 8, 6, 0, 0, 0);
        v.extend_from_slice(&declared.to_be_bytes());
        v.extend_from_slice(b"IHDR");
        v.extend_from_slice(&body[..declared as usize]);
        v.extend_from_slice(&[0, 0, 0, 0]);
        png_err(&format!("C2 IHDR len={declared}"), &v, E_IHDR);
    }
}

#[test]
fn c3_chunk_extends_past_end() {
    // declared IHDR length 13 but the buffer stops early
    let full = mk_png(&[(b"IHDR", ihdr_data(2, 2, 8, 6, 0, 0, 0))]);
    for cut in 1..=12usize {
        let truncated = &full[..full.len() - cut];
        png_err(&format!("C3 truncated by {cut}"), truncated, E_IHDR);
    }
    // an over-declared length also runs past the end
    let mut v = PNG_SIG.to_vec();
    v.extend_from_slice(&1000u32.to_be_bytes());
    v.extend_from_slice(b"IHDR");
    v.extend_from_slice(&ihdr_data(2, 2, 8, 6, 0, 0, 0));
    v.extend_from_slice(&[0, 0, 0, 0]);
    png_err("C3 over-declared IHDR len", &v, E_IHDR);
}

#[test]
fn c4_find_finds_nothing() {
    // valid IHDR, no IDAT anywhere -> cp_find returns NULL, datalen stays 0
    let png = mk_png(&[
        (b"IHDR", ihdr_data(2, 2, 8, 6, 0, 0, 0)),
        (b"IEND", vec![]),
    ]);
    png_err("C4 no IDAT chunk", &png, E_ZLIB);
    let png = mk_png(&[(b"IHDR", ihdr_data(2, 2, 8, 6, 0, 0, 0))]);
    png_err("C4 nothing after IHDR", &png, E_ZLIB);
}

// ===========================================================================
// D. load_png_mem
// ===========================================================================

#[test]
fn d1_bad_signature() {
    png_err("D1 all zeros", &[0u8; 64], E_SIG);
    png_err("D1 jpeg magic", &[0xFF, 0xD8, 0xFF, 0xE0, 0, 0, 0, 0, 1, 2, 3], E_SIG);
    // every single-byte corruption of the signature
    for i in 0..8usize {
        let mut v = PNG_SIG.to_vec();
        v[i] ^= 0xFF;
        v.extend_from_slice(&chunk(b"IHDR", &ihdr_data(2, 2, 8, 6, 0, 0, 0)));
        png_err(&format!("D1 sig byte {i} corrupted"), &v, E_SIG);
    }
    // the signature is compared with memcmp *before* any length check
    png_err_len("E1 length 0", &[0u8; 64], 0, E_SIG);
    png_err_len("E1 length 1", &[0u8; 64], 1, E_SIG);
}

#[test]
fn d2_missing_ihdr() {
    png_err("D2 signature only", &PNG_SIG, E_IHDR);
    png_err_len("E4 length 8", &mk_png(&[(b"IHDR", ihdr_data(2, 2, 8, 6, 0, 0, 0))]), 8, E_IHDR);
    // negative length: png.end < png.p
    let full = mk_png(&[(b"IHDR", ihdr_data(2, 2, 8, 6, 0, 0, 0))]);
    png_err_len("E2 negative length", &full, -1, E_IHDR);
    png_err_len("E2 length i32::MIN", &full, i32::MIN, E_IHDR);
}

#[test]
fn d3_unsupported_bit_depth() {
    for bd in [0u8, 1, 2, 4, 7, 9, 16, 32, 255] {
        let png = mk_png(&[
            (b"IHDR", ihdr_data(2, 2, bd, 6, 0, 0, 0)),
            (b"IEND", vec![]),
        ]);
        png_err(&format!("D3/E6 bit_depth={bd}"), &png, E_DEPTH);
    }
}

#[test]
fn d4_unknown_color_type() {
    // 1, 5, 7 are one step off the valid values; 8..255 have no variant at all
    for ct in [1u8, 5, 7, 8, 9, 16, 100, 127, 128, 200, 254, 255] {
        let png = mk_png(&[
            (b"IHDR", ihdr_data(2, 2, 8, ct, 0, 0, 0)),
            (b"IEND", vec![]),
        ]);
        png_err(&format!("D4/E5 color_type={ct}"), &png, E_CT);
    }
    // all valid ones must get past this check
    for ct in [0u8, 2, 3, 4, 6] {
        let png = mk_png(&[
            (b"IHDR", ihdr_data(2, 2, 8, ct, 0, 0, 0)),
            (b"IEND", vec![]),
        ]);
        let r = c_png(&png, png.len() as c_int);
        assert!(r.null);
        assert_ne!(r.error.as_deref(), Some(E_CT), "ct={ct}");
        diff_png(&format!("D4 valid ct={ct}"), &png);
    }
}

#[test]
fn d5_width_less_than_one() {
    // w = make32(ihdr) + 1, so raw 0xFFFFFFFF gives w == 0
    for raw_w in [0xFFFFFFFFu32, 0x80000000, 0x80000001, 0xFFFFFFFE, 0xC0000000] {
        let mut d = ihdr_data(0, 4, 8, 6, 0, 0, 0);
        d[0..4].copy_from_slice(&raw_w.to_be_bytes());
        let png = mk_png(&[(b"IHDR", d), (b"IEND", vec![])]);
        png_err(&format!("D5 raw_w={raw_w:#x}"), &png, E_W);
    }
    // raw width 0 is *accepted* (w becomes 1): the C uses w = width + 1
    let png = mk_png(&[
        (b"IHDR", ihdr_data(0, 1, 8, 6, 0, 0, 0)),
        (b"IEND", vec![]),
    ]);
    png_err("D5 raw_w=0 passes width check", &png, E_ZLIB);
}

#[test]
fn d6_height_less_than_one() {
    for raw_h in [0u32, 0x80000000, 0xFFFFFFFF, 0x80000001] {
        let mut d = ihdr_data(4, 0, 8, 6, 0, 0, 0);
        d[4..8].copy_from_slice(&raw_h.to_be_bytes());
        let png = mk_png(&[(b"IHDR", d), (b"IEND", vec![])]);
        png_err(&format!("D6 raw_h={raw_h:#x}"), &png, E_H);
    }
}

#[test]
fn d7_image_too_large() {
    // (int64)w * h * 4 must stay below INT_MAX
    for (raw_w, raw_h) in [
        (0xFFFFu32, 0x8000u32),
        (0x7FFFFFFEu32, 2u32),
        (0x10000, 0x10000),
        (0x20000000, 8),
        (0x7FFFFFFE, 0x7FFFFFFE),
    ] {
        let mut d = ihdr_data(0, 0, 8, 6, 0, 0, 0);
        d[0..4].copy_from_slice(&raw_w.to_be_bytes());
        d[4..8].copy_from_slice(&raw_h.to_be_bytes());
        let png = mk_png(&[(b"IHDR", d), (b"IEND", vec![])]);
        png_err(&format!("D7 {raw_w:#x}x{raw_h:#x}"), &png, E_BIG);
    }
}

/// D8 (`malloc` failure), D16 and D17 (`cp_out_size < 1`) are unreachable by
/// construction: D7 already guarantees `1 <= (int64)w*h*4 < INT_MAX`, and
/// `cp_out_size(img,bpp) == w*h*bpp` with `bpp <= 4`, so the product always
/// fits in an `int` and is at least 1, and `malloc` of a sub-2 GiB block does
/// not fail here. This test sweeps the whole boundary region to confirm the
/// neighbouring branch (D7) fires instead — in BOTH implementations — rather
/// than D8/D16/D17 ever being reached.
#[test]
fn d8_d16_d17_boundary_sweep() {
    let mut checked = 0usize;
    for h in [1u32, 2, 4, 8, 255, 4096, 65536] {
        // the largest raw width that still satisfies (w+1)*h*4 < INT_MAX
        let limit = (i32::MAX as u64) / 4 / (h as u64);
        for delta in [-2i64, -1, 0, 1, 2] {
            let cand = limit as i64 + delta;
            if cand < 1 || cand > u32::MAX as i64 {
                continue;
            }
            let raw_w = (cand - 1) as u32;
            let mut d = ihdr_data(0, 0, 8, 6, 0, 0, 0);
            d[0..4].copy_from_slice(&raw_w.to_be_bytes());
            d[4..8].copy_from_slice(&h.to_be_bytes());
            let png = mk_png(&[(b"IHDR", d), (b"IEND", vec![])]);
            let r0 = c_png(&png, png.len() as c_int);
            assert!(r0.null);
            let msg = r0.error.clone().unwrap();
            // never D8/D16/D17
            assert_ne!(msg, "unable to allocate raw image space", "D8 reached at {raw_w}x{h}");
            assert_ne!(msg, "invalid image size found", "D16/D17 reached at {raw_w}x{h}");
            assert!(
                msg == E_BIG || msg == E_ZLIB,
                "unexpected branch at {raw_w}x{h}: {msg}"
            );
            diff_png(&format!("D8/16/17 sweep {raw_w}x{h}"), &png);
            checked += 1;
        }
    }
    assert!(checked >= 25, "sweep covered too few points: {checked}");
}

#[test]
fn d9_bad_compression_method() {
    for c in [1u8, 2, 8, 255] {
        let png = mk_png(&[
            (b"IHDR", ihdr_data(2, 2, 8, 6, c, 0, 0)),
            (b"IEND", vec![]),
        ]);
        png_err(&format!("D9 compression={c}"), &png, E_COMP);
    }
}

#[test]
fn d10_bad_filter_method() {
    for f in [1u8, 2, 64, 255] {
        let png = mk_png(&[
            (b"IHDR", ihdr_data(2, 2, 8, 6, 0, f, 0)),
            (b"IEND", vec![]),
        ]);
        png_err(&format!("D10 filter_method={f}"), &png, E_FILT);
    }
}

#[test]
fn d11_interlace_unsupported() {
    for il in [1u8, 2, 7, 255] {
        let png = mk_png(&[
            (b"IHDR", ihdr_data(2, 2, 8, 6, 0, 0, il)),
            (b"IEND", vec![]),
        ]);
        png_err(&format!("D11 interlace={il}"), &png, E_IL);
    }
}

#[test]
fn d12_corrupt_zlib_structure() {
    // no IDAT at all
    let png = mk_png(&[
        (b"IHDR", ihdr_data(2, 2, 8, 6, 0, 0, 0)),
        (b"IEND", vec![]),
    ]);
    png_err("D12 no IDAT", &png, E_ZLIB);
    // total IDAT payload < 6 bytes, in one chunk and split across several
    for n in 0..6usize {
        let png = mk_png(&[
            (b"IHDR", ihdr_data(2, 2, 8, 6, 0, 0, 0)),
            (b"IDAT", vec![0x78u8; n]),
            (b"IEND", vec![]),
        ]);
        png_err(&format!("D12 idat len={n}"), &png, E_ZLIB);
    }
    let png = mk_png(&[
        (b"IHDR", ihdr_data(2, 2, 8, 6, 0, 0, 0)),
        (b"IDAT", vec![0x78]),
        (b"IDAT", vec![0x9C]),
        (b"IDAT", vec![0x00]),
        (b"IEND", vec![]),
    ]);
    png_err("D12 split idat total=3", &png, E_ZLIB);
}

#[test]
fn d13_bad_zlib_compression_method() {
    let mut base = base_rgba(2, 2);
    for cm in 0..16u8 {
        if cm == 8 {
            continue;
        }
        base.cmf = 0x70 | cm; // keep CINFO <= 7 so D14 doesn't fire first
        let png = base.build();
        png_err(&format!("D13 CM={cm}"), &png, E_CM);
    }
}

#[test]
fn d14_window_size_too_large() {
    let mut base = base_rgba(2, 2);
    for cinfo in 8..16u8 {
        base.cmf = (cinfo << 4) | 0x08;
        let png = base.build();
        png_err(&format!("D14 CINFO={cinfo}"), &png, E_WIN);
    }
}

#[test]
fn d15_preset_dictionary() {
    let mut base = base_rgba(2, 2);
    for flg in [0x20u8, 0x21, 0x3F, 0xA0, 0xFF] {
        assert_ne!(flg & 0x20, 0);
        base.flg = flg;
        let png = base.build();
        png_err(&format!("D15 FLG={flg:#02x}"), &png, E_DICT);
    }
}

#[test]
fn d18_deflate_failure_is_reported() {
    // Each distinct inner DEFLATE failure surfaces as the same outer message.
    let inner: Vec<Vec<u8>> = vec![
        // btype = 3
        vec![0x07, 0, 0, 0, 0, 0],
        // stored block with mismatched LEN/NLEN
        {
            let mut v = vec![0x01u8];
            v.extend_from_slice(&3u16.to_le_bytes());
            v.extend_from_slice(&0u16.to_le_bytes());
            v.extend_from_slice(&[1, 2, 3]);
            v
        },
        // fixed block whose literals overflow the (small) out buffer
        deflate_fixed_literals(&vec![0xAAu8; 400]),
        // a match at the very start (invalid backwards distance)
        {
            let mut bw = BitWriter::new();
            write_fixed_block(&mut bw, &[Tok::Match(200, 5)], true);
            bw.finish()
        },
    ];
    for (i, d) in inner.iter().enumerate() {
        let mut s = base_rgba(2, 2);
        s.raw_zlib = Some(zlib(d));
        let png = s.build();
        png_err(&format!("D18 inner case {i}"), &png, E_DEFL);
    }
}

#[test]
fn d20_indexed_without_plte() {
    let w = 4usize;
    let h = 3usize;
    let data: Vec<u8> = (0..w * h).map(|i| (i * 5) as u8).collect();
    let mut s = PngSpec::new(w as u32, h as u32, 3);
    s.raw = encode_scanlines(w, h, 1, &data, &vec![0u8; h]);
    s.deflate = DeflateMode::FixedLz;
    // no PLTE
    let png = s.build();
    png_err("D20 indexed without PLTE", &png, E_PLTE);
    // a tRNS without PLTE is also not enough
    let mut s2 = s;
    s2.trns = Some(vec![0x80u8; 16]);
    let png = s2.build();
    png_err("D20 indexed with tRNS but no PLTE", &png, E_PLTE);
}

#[test]
fn d21_oversized_and_undersized_lengths() {
    // E3: png_length larger than the real payload. Both libraries read from the
    // same padded allocation, so their results must still agree exactly.
    let mut s = base_rgba(3, 3);
    s.iend = true;
    let png = s.build();
    for delta in [1i32, 2, 3, 4, 16, 64, 1024] {
        diff_png_len(&format!("E3 length+{delta}"), &png, png.len() as c_int + delta);
    }
    // truncation at every offset must agree too
    for cut in 1..png.len().min(80) {
        diff_png_len(
            &format!("D21 length-{cut}"),
            &png,
            (png.len() - cut) as c_int,
        );
    }
}

// d21b lives in tests/isolated.rs (some corruptions crash the C).
