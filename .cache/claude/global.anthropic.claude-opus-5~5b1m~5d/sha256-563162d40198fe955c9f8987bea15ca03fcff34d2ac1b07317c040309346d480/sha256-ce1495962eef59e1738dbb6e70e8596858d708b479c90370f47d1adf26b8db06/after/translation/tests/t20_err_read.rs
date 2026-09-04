//! Phase C — read-path rejections.
//!
//! Mechanism: the SIMPLIFIED read API (`png_image_begin_read_from_memory` +
//! `png_image_finish_read`) wraps the whole low-level read pipeline in
//! `png_safe_execute`, so a `png_error` is turned into a return value of 0 plus
//! `png_image.warning_or_error` and a `png_image.message` string.  That makes it
//! possible to compare the EXACT error code and the EXACT message text of the
//! two libraries in-process, for every crafted invalid stream, without needing
//! `setjmp` from Rust.
//!
//! Each case therefore asserts equality of:
//!   * the return value of `begin_read` and of `finish_read`
//!   * `warning_or_error` (0 = clean, 1 = warning, 2 = error, 3 = both)
//!   * the message string, byte for byte
//!   * every field of `png_image` except the `opaque` allocation pointer
//!   * the decoded bytes, when the read succeeded
mod common;

use common::api::{apis, Api};
use common::pngbuild as pb;
use common::*;
use std::ffi::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// simplified-API driver
// ---------------------------------------------------------------------------

fn fmt_channels(format: png_uint_32) -> usize {
    if format & PNG_FORMAT_FLAG_COLORMAP != 0 {
        return 1;
    }
    let base = if format & PNG_FORMAT_FLAG_COLOR != 0 {
        3
    } else {
        1
    };
    base + if format & PNG_FORMAT_FLAG_ALPHA != 0 { 1 } else { 0 }
}

fn fmt_pixel_bytes(format: png_uint_32) -> usize {
    if format & PNG_FORMAT_FLAG_COLORMAP != 0 {
        1
    } else {
        fmt_channels(format) * if format & PNG_FORMAT_FLAG_LINEAR != 0 { 2 } else { 1 }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SimpleOut {
    pub begin: c_int,
    pub after_begin: (u32, u32, u32, u32, u32, u32, u32, String),
    pub finish: Option<c_int>,
    pub after_finish: Option<(u32, u32, u32, u32, u32, u32, u32, String)>,
    pub bytes: Vec<u8>,
    pub colormap: Vec<u8>,
}

/// Drive one library.  `format`: `None` keeps whatever `begin_read` chose.
unsafe fn simple_read(a: &Api, png: &[u8], format: Option<png_uint_32>, flags: u32) -> SimpleOut {
    let mut img = png_image::default();
    let begin = (a.png_image_begin_read_from_memory)(
        &mut img,
        png.as_ptr() as *const c_void,
        png.len(),
    );
    let after_begin = img.cmp_tuple();
    if begin == 0 {
        (a.png_image_free)(&mut img);
        return SimpleOut {
            begin,
            after_begin,
            finish: None,
            after_finish: None,
            bytes: Vec::new(),
            colormap: Vec::new(),
        };
    }
    if let Some(f) = format {
        img.format = f;
    }
    img.flags |= flags;
    // Allocate for the WORST case (4 channels x 2 bytes per channel) plus a
    // wide margin, because `format` may be any integer the caller passes across
    // the FFI boundary and libpng sizes its output from it: an undersized buffer
    // would be a heap overflow in the *test*, not a divergence.
    let total = (img.width as u64)
        .saturating_mul(img.height as u64)
        .saturating_mul(8);
    if total > 64 * 1024 * 1024 {
        (a.png_image_free)(&mut img);
        return SimpleOut {
            begin,
            after_begin,
            finish: None,
            after_finish: None,
            bytes: Vec::new(),
            colormap: Vec::new(),
        };
    }
    let mut buf = vec![0u8; total as usize + 8192];
    // PNG_IMAGE_COLORMAP_SIZE worst case: 256 entries x 4 channels x 2 bytes.
    let mut cmap = vec![0u8; 256 * 4 * 2 + 8192];
    let want_cmap = img.format & PNG_FORMAT_FLAG_COLORMAP != 0;
    let finish = (a.png_image_finish_read)(
        &mut img,
        std::ptr::null(),
        buf.as_mut_ptr() as *mut c_void,
        0,
        if want_cmap {
            cmap.as_mut_ptr() as *mut c_void
        } else {
            std::ptr::null_mut()
        },
    );
    let after_finish = img.cmp_tuple();
    (a.png_image_free)(&mut img);
    SimpleOut {
        begin,
        after_begin,
        finish: Some(finish),
        after_finish: Some(after_finish),
        bytes: if finish != 0 { buf } else { Vec::new() },
        colormap: if finish != 0 && want_cmap {
            cmap
        } else {
            Vec::new()
        },
    }
}

#[track_caller]
fn diff_simple(png: &[u8], what: &str) {
    diff_simple_fmt(png, None, 0, what)
}

#[track_caller]
fn diff_simple_fmt(png: &[u8], format: Option<png_uint_32>, flags: u32, what: &str) {
    let b = apis();
    let c = unsafe { simple_read(&b.c, png, format, flags) };
    let r = unsafe { simple_read(&b.rs, png, format, flags) };
    eq_dbg(&format!("{what}: begin_read ret"), c.begin, r.begin);
    eq_dbg(
        &format!("{what}: png_image after begin_read"),
        c.after_begin.clone(),
        r.after_begin.clone(),
    );
    eq_dbg(&format!("{what}: finish_read ret"), c.finish, r.finish);
    eq_dbg(
        &format!("{what}: png_image after finish_read"),
        c.after_finish.clone(),
        r.after_finish.clone(),
    );
    eq_bytes(&format!("{what}: decoded bytes"), &c.bytes, &r.bytes);
    eq_bytes(&format!("{what}: colormap"), &c.colormap, &r.colormap);
}

// ---------------------------------------------------------------------------
// stream builders
// ---------------------------------------------------------------------------

fn valid_rgb8(seed: u64, w: u32, h: u32) -> Vec<u8> {
    pb::make_png(seed, w, h, 8, 2, 0)
}

/// A stream with the given IHDR payload, a plausible IDAT and an IEND.
fn with_ihdr(ihdr: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&pb::PNG_SIG);
    pb::push_chunk(&mut out, b"IHDR", ihdr);
    pb::push_chunk(&mut out, b"IDAT", &pb::zlib_store(&[0u8; 4]));
    pb::push_chunk(&mut out, b"IEND", &[]);
    out
}

/// A valid 4x2 8-bit RGB stream with `extra` inserted before IDAT.
fn with_pre_chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut spec = pb::PngSpec::new(4, 2, 8, 2, 0);
    spec.pre_idat = vec![(*name, data.to_vec())];
    let mut rng = Rng::new(7);
    spec.raw = pb::raw_rows_none(4, 2, 8, 2, &mut |_y, rb| {
        (0..rb).map(|_| rng.next_u8()).collect()
    });
    spec.build()
}

/// Same, but the chunk goes after IDAT.
fn with_post_chunk(name: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut spec = pb::PngSpec::new(4, 2, 8, 2, 0);
    spec.post_idat = vec![(*name, data.to_vec())];
    let mut rng = Rng::new(7);
    spec.raw = pb::raw_rows_none(4, 2, 8, 2, &mut |_y, rb| {
        (0..rb).map(|_| rng.next_u8()).collect()
    });
    spec.build()
}

/// Palette stream with a caller-supplied PLTE payload.
fn with_plte(bd: u8, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&pb::PNG_SIG);
    pb::push_chunk(&mut out, b"IHDR", &pb::ihdr_data(4, 2, bd, 3, 0, 0, 0));
    pb::push_chunk(&mut out, b"PLTE", data);
    let rb = pb::rowbytes(bd, 3, 4);
    let raw: Vec<u8> = (0..2).flat_map(|_| {
        let mut v = vec![0u8];
        v.extend(std::iter::repeat(0u8).take(rb));
        v
    }).collect();
    pb::push_chunk(&mut out, b"IDAT", &pb::zlib_store(&raw));
    pb::push_chunk(&mut out, b"IEND", &[]);
    out
}

// ---------------------------------------------------------------------------
// signature
// ---------------------------------------------------------------------------

#[test]
fn signature_rejections() {
    // ERRORS.md: "Not a PNG file" / "bad signature" / zero-length input
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    cases.push(("empty".into(), Vec::new()));
    for n in 1..8usize {
        cases.push((format!("sig-prefix-{n}"), pb::PNG_SIG[..n].to_vec()));
    }
    cases.push(("sig-only".into(), pb::PNG_SIG.to_vec()));
    for i in 0..8usize {
        let mut s = valid_rgb8(1, 4, 2);
        s[i] ^= 0xff;
        cases.push((format!("sig-byte{i}-flipped"), s));
    }
    // every single-byte value in each signature position
    let mut rng = Rng::new(0x2001);
    for _ in 0..400 {
        let mut s = valid_rgb8(1, 4, 2);
        let i = rng.below(8) as usize;
        s[i] = rng.next_u8();
        cases.push(("sig-random".into(), s));
    }
    // JPEG / GIF / other magic
    for magic in [
        &b"\xff\xd8\xff\xe0\x00\x10JF"[..],
        &b"GIF89a\x00\x00"[..],
        &b"BM\x00\x00\x00\x00\x00\x00"[..],
        &b"\x89PNG\x0d\x0a\x1a\x0b"[..],
        &b"\x8aPNG\x0d\x0a\x1a\x0a"[..],
    ] {
        let mut s = magic.to_vec();
        s.extend_from_slice(&valid_rgb8(1, 4, 2)[8..]);
        cases.push(("other-magic".into(), s));
    }
    for (name, s) in &cases {
        diff_simple(s, &format!("signature/{name} len={}", s.len()));
    }
}

// ---------------------------------------------------------------------------
// IHDR
// ---------------------------------------------------------------------------

#[test]
fn ihdr_rejections() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    // wrong IHDR length: every length from 0 to 20 except 13
    for n in 0..=20usize {
        let mut d = pb::ihdr_data(4, 2, 8, 2, 0, 0, 0);
        d.resize(n, 0);
        cases.push((format!("ihdr-len-{n}"), with_ihdr(&d)));
    }
    // width / height zero and enormous
    for &(w, h) in &[
        (0u32, 2u32),
        (4, 0),
        (0, 0),
        (0x8000_0000, 2),
        (2, 0x8000_0000),
        (0xffff_ffff, 0xffff_ffff),
        (0x7fff_ffff, 1),
        (1, 0x7fff_ffff),
        (1_000_001, 1),
        (1, 1_000_001),
        (1_000_000, 1),
    ] {
        cases.push((
            format!("ihdr-dim-{w}x{h}"),
            with_ihdr(&pb::ihdr_data(w, h, 8, 2, 0, 0, 0)),
        ));
    }
    // every bit depth 0..=32 x every colour type 0..=8
    for bd in 0u8..=32 {
        for ct in 0u8..=8 {
            cases.push((
                format!("ihdr-bd{bd}-ct{ct}"),
                with_ihdr(&pb::ihdr_data(4, 2, bd, ct, 0, 0, 0)),
            ));
        }
    }
    for ct in [9u8, 16, 64, 128, 255] {
        cases.push((
            format!("ihdr-ct{ct}"),
            with_ihdr(&pb::ihdr_data(4, 2, 8, ct, 0, 0, 0)),
        ));
    }
    // compression / filter / interlace methods
    for cm in [1u8, 2, 8, 255] {
        cases.push((
            format!("ihdr-cm{cm}"),
            with_ihdr(&pb::ihdr_data(4, 2, 8, 2, cm, 0, 0)),
        ));
    }
    for fm in [1u8, 2, 64, 255] {
        cases.push((
            format!("ihdr-fm{fm}"),
            with_ihdr(&pb::ihdr_data(4, 2, 8, 2, 0, fm, 0)),
        ));
    }
    for il in [2u8, 3, 255] {
        cases.push((
            format!("ihdr-il{il}"),
            with_ihdr(&pb::ihdr_data(4, 2, 8, 2, 0, 0, il)),
        ));
    }
    // first chunk not IHDR
    {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"gAMA", &45455u32.to_be_bytes());
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 4]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("no-ihdr-first".into(), s));
    }
    // IDAT before IHDR
    {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 4]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("idat-without-ihdr".into(), s));
    }
    // duplicate IHDR
    {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 4]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("duplicate-ihdr".into(), s));
    }
    // truncated after IHDR
    {
        let full = valid_rgb8(3, 4, 2);
        for n in [8usize, 12, 16, 20, 25, 29, 32, 33, 40] {
            if n <= full.len() {
                cases.push((format!("truncated-{n}"), full[..n].to_vec()));
            }
        }
    }
    // bad IHDR CRC
    {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk_bad_crc(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 4]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("ihdr-bad-crc".into(), s));
    }

    for (name, s) in &cases {
        diff_simple(s, &format!("ihdr/{name}"));
    }
}

// ---------------------------------------------------------------------------
// chunk framing
// ---------------------------------------------------------------------------

#[test]
fn chunk_framing_rejections() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    // declared length with the top bit set / > PNG_UINT_31_MAX
    for len in [0x8000_0000u32, 0xffff_ffff, 0x7fff_ffff, 0x0080_0001, 8_000_001] {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk_raw(&mut s, len, b"prVt", &[1, 2, 3], 0);
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 4]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push((format!("chunk-len-{len:#x}"), s));
    }
    // invalid chunk names (reserved bit / non-alpha bytes)
    for name in [
        b"prvt", b"PRVT", b"pr0t", b"pr t", b"\x00\x00\x00\x00", b"IHD\x00",
        b"iend", b"idat", b"plte", b"\xff\xff\xff\xff",
    ] {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, name, &[1, 2]);
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 4]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push((
            format!("chunk-name-{:02x?}", name),
            s,
        ));
    }
    // unhandled critical chunk
    for name in [b"cRIT", b"XXXX", b"Abcd"] {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, name, &[1, 2]);
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 4]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push((format!("critical-{}", String::from_utf8_lossy(name)), s));
    }
    // bad CRC on an ancillary and on a critical chunk
    {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk_bad_crc(&mut s, b"gAMA", &45455u32.to_be_bytes());
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 4]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("ancillary-bad-crc".into(), s));
    }
    {
        let mut spec = pb::PngSpec::new(4, 2, 8, 2, 0);
        let mut rng = Rng::new(9);
        spec.raw = pb::raw_rows_none(4, 2, 8, 2, &mut |_y, rb| {
            (0..rb).map(|_| rng.next_u8()).collect()
        });
        let good = spec.build();
        // corrupt the IDAT CRC in place: locate the IDAT chunk
        let mut s = good.clone();
        // find "IDAT"
        if let Some(pos) = s.windows(4).position(|w| w == b"IDAT") {
            let len = u32::from_be_bytes([s[pos - 4], s[pos - 3], s[pos - 2], s[pos - 1]]) as usize;
            let crc_off = pos + 4 + len;
            s[crc_off] ^= 0xff;
            cases.push(("idat-bad-crc".into(), s));
        }
    }
    // no IEND / IEND with a payload / IEND before IDAT / data after IEND
    {
        let full = valid_rgb8(4, 4, 2);
        cases.push(("no-iend".into(), full[..full.len() - 12].to_vec()));
        let mut s = full[..full.len() - 12].to_vec();
        pb::push_chunk(&mut s, b"IEND", &[1, 2, 3]);
        cases.push(("iend-with-payload".into(), s));
        let mut s = full.clone();
        s.extend_from_slice(b"garbage-after-iend");
        cases.push(("data-after-iend".into(), s));
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("iend-without-idat".into(), s));
    }
    // truncated in the middle of a chunk header / payload / CRC
    {
        let full = valid_rgb8(5, 8, 4);
        for n in (8..full.len()).step_by(3) {
            cases.push((format!("trunc-{n}"), full[..n].to_vec()));
        }
    }

    for (name, s) in &cases {
        diff_simple(s, &format!("framing/{name}"));
    }
}

// ---------------------------------------------------------------------------
// IDAT / zlib
// ---------------------------------------------------------------------------

#[test]
fn idat_rejections() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    // no IDAT at all
    {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("missing-idat".into(), s));
    }
    // empty IDAT
    {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"IDAT", &[]);
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("empty-idat".into(), s));
    }
    // IDAT with garbage / truncated zlib / wrong CMF / bad adler
    let raw = {
        let mut rng = Rng::new(11);
        pb::raw_rows_none(4, 2, 8, 2, &mut |_y, rb| {
            (0..rb).map(|_| rng.next_u8()).collect()
        })
    };
    let z = pb::zlib_store(&raw);
    let variants: Vec<(String, Vec<u8>)> = vec![
        ("garbage".into(), vec![0xde, 0xad, 0xbe, 0xef]),
        ("truncated-1".into(), z[..1].to_vec()),
        ("truncated-2".into(), z[..2].to_vec()),
        ("truncated-half".into(), z[..z.len() / 2].to_vec()),
        ("no-adler".into(), z[..z.len() - 4].to_vec()),
        ("bad-adler".into(), {
            let mut v = z.clone();
            let n = v.len();
            v[n - 1] ^= 0xff;
            v
        }),
        ("bad-cmf".into(), {
            let mut v = z.clone();
            v[0] = 0x99;
            v
        }),
        ("bad-flg".into(), {
            let mut v = z.clone();
            v[1] = 0x00;
            v
        }),
        ("extra-trailing".into(), {
            let mut v = z.clone();
            v.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
            v
        }),
        ("too-little-data".into(), pb::zlib_store(&raw[..raw.len() / 2])),
        ("too-much-data".into(), {
            let mut r2 = raw.clone();
            r2.extend_from_slice(&raw);
            pb::zlib_store(&r2)
        }),
    ];
    for (n, zz) in &variants {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"IDAT", zz);
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push((format!("zlib-{n}"), s));
    }
    // invalid filter byte on a scan line (0..=255)
    for f in [1u8, 2, 3, 4, 5, 6, 7, 128, 255] {
        let mut r = raw.clone();
        r[0] = f;
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&r));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push((format!("filter-byte-{f}"), s));
    }
    // IDAT split by a non-IDAT chunk (=> "Too many IDATs found")
    {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"IDAT", &z[..z.len() / 2]);
        pb::push_chunk(&mut s, b"gAMA", &45455u32.to_be_bytes());
        pb::push_chunk(&mut s, b"IDAT", &z[z.len() / 2..]);
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("idat-interrupted".into(), s));
    }
    // an IDAT after IEND
    {
        let mut s = valid_rgb8(6, 4, 2);
        pb::push_chunk(&mut s, b"IDAT", &z);
        cases.push(("idat-after-iend".into(), s));
    }

    for (name, s) in &cases {
        diff_simple(s, &format!("idat/{name}"));
    }
}

// ---------------------------------------------------------------------------
// PLTE / tRNS / hIST (palette family)
// ---------------------------------------------------------------------------

#[test]
fn palette_family_rejections() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    // PLTE lengths 0..=32 plus the boundaries
    for n in 0..=32usize {
        cases.push((format!("plte-len{n}"), with_plte(8, &vec![0x40u8; n])));
    }
    for n in [255usize, 256, 767, 768, 769, 770, 900, 1000] {
        cases.push((format!("plte-len{n}"), with_plte(8, &vec![0x40u8; n])));
    }
    // too many entries for the bit depth
    for bd in [1u8, 2, 4] {
        let maxn = 1usize << bd;
        for n in [maxn, maxn + 1, 256] {
            cases.push((
                format!("plte-bd{bd}-n{n}"),
                with_plte(bd, &vec![0x40u8; n * 3]),
            ));
        }
    }
    // PLTE missing for colour type 3
    {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 3, 0, 0, 0));
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 10]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("plte-missing".into(), s));
    }
    // PLTE present for grey / grey-alpha (illegal) and for RGB/RGBA (legal
    // suggested palette)
    for ct in [0u8, 2, 4, 6] {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, ct, 0, 0, 0));
        pb::push_chunk(&mut s, b"PLTE", &vec![0x40u8; 9]);
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 40]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push((format!("plte-on-ct{ct}"), s));
    }
    // duplicate PLTE / PLTE after IDAT
    {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 3, 0, 0, 0));
        pb::push_chunk(&mut s, b"PLTE", &vec![0x40u8; 9]);
        pb::push_chunk(&mut s, b"PLTE", &vec![0x50u8; 9]);
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 10]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("plte-duplicate".into(), s));
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 3, 0, 0, 0));
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 10]));
        pb::push_chunk(&mut s, b"PLTE", &vec![0x40u8; 9]);
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("plte-after-idat".into(), s));
    }

    // tRNS: wrong length for every colour type
    for ct in [0u8, 2, 3, 4, 6] {
        for n in 0..=10usize {
            let mut spec = pb::PngSpec::new(4, 2, 8, ct, 0);
            if ct == 3 {
                spec.palette = vec![0x40u8; 9];
            }
            spec.trns = Some(vec![0x80u8; n]);
            let rb = pb::rowbytes(8, ct, 4);
            spec.raw = (0..2)
                .flat_map(|_| {
                    let mut v = vec![0u8];
                    v.extend(std::iter::repeat(0x11u8).take(rb));
                    v
                })
                .collect();
            cases.push((format!("trns-ct{ct}-len{n}"), spec.build()));
        }
    }
    // tRNS with more entries than the palette / before PLTE / duplicated
    {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 3, 0, 0, 0));
        pb::push_chunk(&mut s, b"tRNS", &[0x80, 0x81]);
        pb::push_chunk(&mut s, b"PLTE", &vec![0x40u8; 9]);
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 10]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("trns-before-plte".into(), s));
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 3, 0, 0, 0));
        pb::push_chunk(&mut s, b"PLTE", &vec![0x40u8; 9]);
        pb::push_chunk(&mut s, b"tRNS", &[0x80, 0x81, 0x82, 0x83, 0x84]);
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 10]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("trns-too-many".into(), s));
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 3, 0, 0, 0));
        pb::push_chunk(&mut s, b"PLTE", &vec![0x40u8; 9]);
        pb::push_chunk(&mut s, b"tRNS", &[0x80]);
        pb::push_chunk(&mut s, b"tRNS", &[0x81]);
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 10]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("trns-duplicate".into(), s));
    }

    // hIST: wrong length, no PLTE, after IDAT, duplicate
    {
        for n in [0usize, 1, 2, 3, 5, 6, 7, 8, 512] {
            let mut s = Vec::new();
            s.extend_from_slice(&pb::PNG_SIG);
            pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 3, 0, 0, 0));
            pb::push_chunk(&mut s, b"PLTE", &vec![0x40u8; 9]);
            pb::push_chunk(&mut s, b"hIST", &vec![0u8; n]);
            pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 10]));
            pb::push_chunk(&mut s, b"IEND", &[]);
            cases.push((format!("hist-len{n}"), s));
        }
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"hIST", &[0, 1, 0, 2]);
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 40]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        cases.push(("hist-no-plte".into(), s));
    }

    for (name, s) in &cases {
        diff_simple(s, &format!("palette/{name}"));
    }
}

// ---------------------------------------------------------------------------
// every other chunk: wrong length, wrong position, duplicated
// ---------------------------------------------------------------------------

/// (name, a payload that is VALID, and a list of payload lengths to try)
const CHUNKS: &[(&[u8; 4], &[u8])] = &[
    (b"gAMA", &[0, 0, 0xb1, 0x8f]),
    (b"sBIT", &[8, 8, 8]),
    (b"cHRM", &[0u8; 32]),
    (b"sRGB", &[0]),
    (b"bKGD", &[0, 1, 0, 2, 0, 3]),
    (b"pHYs", &[0, 0, 1, 0x2c, 0, 0, 1, 0x2c, 1]),
    (b"oFFs", &[0, 0, 0, 1, 0, 0, 0, 2, 1]),
    (b"tIME", &[0x07, 0xe8, 1, 1, 0, 0, 0]),
    (b"tEXt", b"K\0v"),
    (b"zTXt", b"K\0\0"),
    (b"iTXt", b"K\0\0\0e\0k\0v"),
    (b"sCAL", b"\x011.0\x002.0"),
    (b"pCAL", b"p\0\0\0\0\0\0\0\0\xff\0\0u\0"),
    (b"sPLT", b"n\0\x08"),
    (b"iCCP", b"n\0\0"),
    (b"eXIf", b"II\x2a\0\x08\0\0\0"),
    (b"cICP", &[9, 16, 0, 1]),
    (b"cLLI", &[0, 0, 0, 1, 0, 0, 0, 2]),
    (b"mDCV", &[0u8; 24]),
    (b"prVt", &[1, 2, 3]),
];

#[test]
fn ancillary_chunk_length_rejections() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, good) in CHUNKS {
        // every length from 0 up to len(good)+4, and a couple of large ones
        let mut lens: Vec<usize> = (0..=(good.len() + 4)).collect();
        lens.extend_from_slice(&[64, 100, 1000]);
        for n in lens {
            let mut d = good.to_vec();
            d.resize(n, 0x5a);
            cases.push((
                format!("{}-len{n}", String::from_utf8_lossy(*name)),
                with_pre_chunk(name, &d),
            ));
        }
    }
    for (name, s) in &cases {
        diff_simple(s, &format!("anclen/{name}"));
    }
}

#[test]
fn ancillary_chunk_position_rejections() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, good) in CHUNKS {
        let n = String::from_utf8_lossy(*name).into_owned();
        // after IDAT
        cases.push((format!("{n}-after-idat"), with_post_chunk(name, good)));
        // duplicated before IDAT
        {
            let mut spec = pb::PngSpec::new(4, 2, 8, 2, 0);
            spec.pre_idat = vec![(**name, good.to_vec()), (**name, good.to_vec())];
            let mut rng = Rng::new(13);
            spec.raw = pb::raw_rows_none(4, 2, 8, 2, &mut |_y, rb| {
                (0..rb).map(|_| rng.next_u8()).collect()
            });
            cases.push((format!("{n}-duplicate"), spec.build()));
        }
        // duplicated after IDAT
        {
            let mut spec = pb::PngSpec::new(4, 2, 8, 2, 0);
            spec.post_idat = vec![(**name, good.to_vec()), (**name, good.to_vec())];
            let mut rng = Rng::new(13);
            spec.raw = pb::raw_rows_none(4, 2, 8, 2, &mut |_y, rb| {
                (0..rb).map(|_| rng.next_u8()).collect()
            });
            cases.push((format!("{n}-duplicate-after"), spec.build()));
        }
        // before IHDR
        {
            let mut s = Vec::new();
            s.extend_from_slice(&pb::PNG_SIG);
            pb::push_chunk(&mut s, name, good);
            pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
            pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 30]));
            pb::push_chunk(&mut s, b"IEND", &[]);
            cases.push((format!("{n}-before-ihdr"), s));
        }
        // in the middle of the IDAT stream
        {
            let mut rng = Rng::new(17);
            let raw = pb::raw_rows_none(4, 2, 8, 2, &mut |_y, rb| {
                (0..rb).map(|_| rng.next_u8()).collect()
            });
            let z = pb::zlib_store(&raw);
            let mut s = Vec::new();
            s.extend_from_slice(&pb::PNG_SIG);
            pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
            pb::push_chunk(&mut s, b"IDAT", &z[..z.len() / 2]);
            pb::push_chunk(&mut s, name, good);
            pb::push_chunk(&mut s, b"IDAT", &z[z.len() / 2..]);
            pb::push_chunk(&mut s, b"IEND", &[]);
            cases.push((format!("{n}-inside-idat"), s));
        }
        // bad CRC
        {
            let mut s = Vec::new();
            s.extend_from_slice(&pb::PNG_SIG);
            pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
            pb::push_chunk_bad_crc(&mut s, name, good);
            pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 30]));
            pb::push_chunk(&mut s, b"IEND", &[]);
            cases.push((format!("{n}-bad-crc"), s));
        }
    }
    for (name, s) in &cases {
        diff_simple(s, &format!("ancpos/{name}"));
    }
}

// ---------------------------------------------------------------------------
// chunk-specific content rejections
// ---------------------------------------------------------------------------

#[test]
fn chunk_content_rejections() {
    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    // gAMA == 0 and gAMA huge
    for g in [0u32, 1, 0x7fff_ffff, 0x8000_0000, 0xffff_ffff] {
        cases.push((
            format!("gAMA-{g:#x}"),
            with_pre_chunk(b"gAMA", &g.to_be_bytes()),
        ));
    }
    // sRGB intent out of range
    for i in 0u8..=8 {
        cases.push((format!("sRGB-{i}"), with_pre_chunk(b"sRGB", &[i])));
    }
    for i in [16u8, 128, 255] {
        cases.push((format!("sRGB-{i}"), with_pre_chunk(b"sRGB", &[i])));
    }
    // sBIT values 0 and > bit depth, for every colour type
    for ct in [0u8, 2, 3, 4, 6] {
        let n = match ct {
            0 => 1,
            2 | 3 => 3,
            4 => 2,
            _ => 4,
        };
        for v in [0u8, 1, 8, 9, 16, 17, 255] {
            let mut spec = pb::PngSpec::new(4, 2, 8, ct, 0);
            if ct == 3 {
                spec.palette = vec![0x40u8; 9];
            }
            spec.pre_idat = vec![(*b"sBIT", vec![v; n])];
            let rb = pb::rowbytes(8, ct, 4);
            spec.raw = (0..2)
                .flat_map(|_| {
                    let mut r = vec![0u8];
                    r.extend(std::iter::repeat(0u8).take(rb));
                    r
                })
                .collect();
            cases.push((format!("sBIT-ct{ct}-v{v}"), spec.build()));
        }
    }
    // tEXt: no separator, empty keyword, leading/trailing space, control chars,
    // keyword longer than 79 bytes, several NULs
    for (n, d) in [
        ("no-nul", b"KeyNoNul".to_vec()),
        ("empty-key", b"\0value".to_vec()),
        ("leading-space", b" Key\0v".to_vec()),
        ("trailing-space", b"Key \0v".to_vec()),
        ("double-space", b"Ke  y\0v".to_vec()),
        ("control-char", b"Ke\x01y\0v".to_vec()),
        ("nonlatin", b"Ke\x7fy\0v".to_vec()),
        ("long-key", {
            let mut v = vec![b'K'; 200];
            v.push(0);
            v.extend_from_slice(b"v");
            v
        }),
        ("two-nuls", b"Key\0va\0lue".to_vec()),
        ("only-nul", b"\0".to_vec()),
        ("empty", b"".to_vec()),
    ] {
        cases.push((format!("tEXt-{n}"), with_pre_chunk(b"tEXt", &d)));
    }
    // zTXt: bad compression method, missing method byte, corrupt stream
    for (n, d) in [
        ("no-method", b"Key\0".to_vec()),
        ("method-1", {
            let mut v = b"Key\0".to_vec();
            v.push(1);
            v.extend_from_slice(&pb::zlib_store(b"x"));
            v
        }),
        ("method-255", {
            let mut v = b"Key\0".to_vec();
            v.push(255);
            v.extend_from_slice(&pb::zlib_store(b"x"));
            v
        }),
        ("corrupt-stream", {
            let mut v = b"Key\0".to_vec();
            v.push(0);
            v.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
            v
        }),
        ("truncated-stream", {
            let mut v = b"Key\0".to_vec();
            v.push(0);
            let z = pb::zlib_store(b"hello world");
            v.extend_from_slice(&z[..z.len() / 2]);
            v
        }),
        ("no-nul", b"KeyNoNul\0".to_vec()),
    ] {
        cases.push((format!("zTXt-{n}"), with_pre_chunk(b"zTXt", &d)));
    }
    // iTXt: bad compression flag, bad method, missing language / translated key
    for (n, d) in [
        ("flag-2", b"K\0\x02\0e\0k\0v".to_vec()),
        ("flag-255", b"K\0\xff\0e\0k\0v".to_vec()),
        ("method-1", b"K\0\0\x01e\0k\0v".to_vec()),
        ("no-lang-nul", b"K\0\0\0en".to_vec()),
        ("no-transkey-nul", b"K\0\0\0e\0kk".to_vec()),
        ("compressed-corrupt", {
            let mut v = b"K\0\x01\0e\0k\0".to_vec();
            v.extend_from_slice(&[0xde, 0xad]);
            v
        }),
        ("empty-key", b"\0\0\0e\0k\0v".to_vec()),
    ] {
        cases.push((format!("iTXt-{n}"), with_pre_chunk(b"iTXt", &d)));
    }
    // iCCP: bad compression method, empty/long name, missing NUL, bad profile
    for (n, d) in [
        ("method-1", {
            let mut v = b"n\0".to_vec();
            v.push(1);
            v.extend_from_slice(&pb::zlib_store(&[0u8; 132]));
            v
        }),
        ("no-nul", b"nameonly".to_vec()),
        ("empty-name", {
            let mut v = b"\0".to_vec();
            v.push(0);
            v.extend_from_slice(&pb::zlib_store(&[0u8; 132]));
            v
        }),
        ("long-name", {
            let mut v = vec![b'n'; 200];
            v.push(0);
            v.push(0);
            v.extend_from_slice(&pb::zlib_store(&[0u8; 132]));
            v
        }),
        ("corrupt-zlib", {
            let mut v = b"n\0".to_vec();
            v.push(0);
            v.extend_from_slice(&[0xde, 0xad, 0xbe, 0xef]);
            v
        }),
        ("profile-too-short", {
            let mut v = b"n\0".to_vec();
            v.push(0);
            v.extend_from_slice(&pb::zlib_store(&[0u8; 8]));
            v
        }),
        ("profile-zeroed", {
            let mut v = b"n\0".to_vec();
            v.push(0);
            v.extend_from_slice(&pb::zlib_store(&[0u8; 132]));
            v
        }),
        ("profile-bad-signature", {
            let mut prof = vec![0u8; 132];
            let len = prof.len() as u32;
            prof[0..4].copy_from_slice(&len.to_be_bytes());
            prof[36..40].copy_from_slice(b"XXXX");
            let mut v = b"n\0".to_vec();
            v.push(0);
            v.extend_from_slice(&pb::zlib_store(&prof));
            v
        }),
        ("profile-length-mismatch", {
            let mut prof = vec![0u8; 132];
            prof[0..4].copy_from_slice(&999u32.to_be_bytes());
            prof[36..40].copy_from_slice(b"acsp");
            let mut v = b"n\0".to_vec();
            v.push(0);
            v.extend_from_slice(&pb::zlib_store(&prof));
            v
        }),
    ] {
        cases.push((format!("iCCP-{n}"), with_pre_chunk(b"iCCP", &d)));
    }
    // sPLT: bad sample depth, entry count not a multiple, missing NUL
    for (n, d) in [
        ("depth-0", b"n\0\x00".to_vec()),
        ("depth-1", b"n\0\x01".to_vec()),
        ("depth-4", b"n\0\x04".to_vec()),
        ("depth-32", b"n\0\x20".to_vec()),
        ("depth-255", b"n\0\xff".to_vec()),
        ("no-nul", b"nnnn".to_vec()),
        ("ragged-8", b"n\0\x08\x01\x02\x03".to_vec()),
        ("ragged-16", b"n\0\x10\x01\x02\x03".to_vec()),
        ("empty-name", b"\0\x08".to_vec()),
        ("long-name", {
            let mut v = vec![b'n'; 200];
            v.push(0);
            v.push(8);
            v
        }),
    ] {
        cases.push((format!("sPLT-{n}"), with_pre_chunk(b"sPLT", &d)));
    }
    // pCAL: bad equation type, wrong number of parameters, missing fields
    for (n, d) in [
        ("eq-4", {
            let mut v = b"p\0".to_vec();
            v.extend_from_slice(&0i32.to_be_bytes());
            v.extend_from_slice(&255i32.to_be_bytes());
            v.push(4);
            v.push(0);
            v.extend_from_slice(b"u\0");
            v
        }),
        ("eq-255", {
            let mut v = b"p\0".to_vec();
            v.extend_from_slice(&0i32.to_be_bytes());
            v.extend_from_slice(&255i32.to_be_bytes());
            v.push(255);
            v.push(0);
            v.extend_from_slice(b"u\0");
            v
        }),
        ("nparams-mismatch", {
            let mut v = b"p\0".to_vec();
            v.extend_from_slice(&0i32.to_be_bytes());
            v.extend_from_slice(&255i32.to_be_bytes());
            v.push(0);
            v.push(3);
            v.extend_from_slice(b"u\0");
            v.extend_from_slice(b"1.0\0");
            v
        }),
        ("x0-eq-x1", {
            let mut v = b"p\0".to_vec();
            v.extend_from_slice(&5i32.to_be_bytes());
            v.extend_from_slice(&5i32.to_be_bytes());
            v.push(0);
            v.push(0);
            v.extend_from_slice(b"u\0");
            v
        }),
        ("no-purpose-nul", b"purpose-with-no-nul".to_vec()),
        ("bad-param", {
            let mut v = b"p\0".to_vec();
            v.extend_from_slice(&0i32.to_be_bytes());
            v.extend_from_slice(&255i32.to_be_bytes());
            v.push(0);
            v.push(1);
            v.extend_from_slice(b"u\0");
            v.extend_from_slice(b"not-a-number");
            v
        }),
    ] {
        cases.push((format!("pCAL-{n}"), with_pre_chunk(b"pCAL", &d)));
    }
    // sCAL: bad unit, non-numeric / negative / zero values, missing NUL
    for (n, d) in [
        ("unit-0", b"\x001.0\x002.0".to_vec()),
        ("unit-3", b"\x031.0\x002.0".to_vec()),
        ("unit-255", b"\xff1.0\x002.0".to_vec()),
        ("no-nul", b"\x011.02.0".to_vec()),
        ("width-bad", b"\x01abc\x002.0".to_vec()),
        ("height-bad", b"\x011.0\x00xyz".to_vec()),
        ("width-zero", b"\x010\x002.0".to_vec()),
        ("width-negative", b"\x01-1.0\x002.0".to_vec()),
        ("height-zero", b"\x011.0\x000".to_vec()),
        ("empty-width", b"\x01\x002.0".to_vec()),
        ("empty-height", b"\x011.0\x00".to_vec()),
    ] {
        cases.push((format!("sCAL-{n}"), with_pre_chunk(b"sCAL", &d)));
    }
    // tIME with out-of-range fields
    for (n, d) in [
        ("month-0", vec![0x07, 0xe8, 0, 1, 0, 0, 0]),
        ("month-13", vec![0x07, 0xe8, 13, 1, 0, 0, 0]),
        ("day-0", vec![0x07, 0xe8, 1, 0, 0, 0, 0]),
        ("day-32", vec![0x07, 0xe8, 1, 32, 0, 0, 0]),
        ("hour-24", vec![0x07, 0xe8, 1, 1, 24, 0, 0]),
        ("minute-60", vec![0x07, 0xe8, 1, 1, 0, 60, 0]),
        ("second-61", vec![0x07, 0xe8, 1, 1, 0, 0, 61]),
        ("all-255", vec![0xff, 0xff, 255, 255, 255, 255, 255]),
    ] {
        cases.push((format!("tIME-{n}"), with_pre_chunk(b"tIME", &d)));
    }
    // cICP with a non-zero matrix coefficient (rejected) and other values
    for mc in [0u8, 1, 2, 255] {
        cases.push((
            format!("cICP-mc{mc}"),
            with_pre_chunk(b"cICP", &[9, 16, mc, 1]),
        ));
    }
    for vf in [0u8, 1, 2, 255] {
        cases.push((
            format!("cICP-vf{vf}"),
            with_pre_chunk(b"cICP", &[9, 16, 0, vf]),
        ));
    }
    // pHYs / oFFs unit types out of range
    for u in [0u8, 1, 2, 255] {
        let mut d = Vec::new();
        d.extend_from_slice(&300u32.to_be_bytes());
        d.extend_from_slice(&400u32.to_be_bytes());
        d.push(u);
        cases.push((format!("pHYs-unit{u}"), with_pre_chunk(b"pHYs", &d)));
        let mut d = Vec::new();
        d.extend_from_slice(&1i32.to_be_bytes());
        d.extend_from_slice(&2i32.to_be_bytes());
        d.push(u);
        cases.push((format!("oFFs-unit{u}"), with_pre_chunk(b"oFFs", &d)));
    }
    // eXIf with a bad byte order marker
    for (n, d) in [
        ("mm", b"MM\x00\x2a\x00\x00\x00\x08".to_vec()),
        ("bad-order", b"XX\x2a\x00\x08\x00\x00\x00".to_vec()),
        ("bad-magic", b"II\x2b\x00\x08\x00\x00\x00".to_vec()),
        ("short", b"II".to_vec()),
        ("one-byte", b"I".to_vec()),
    ] {
        cases.push((format!("eXIf-{n}"), with_pre_chunk(b"eXIf", &d)));
    }
    // bKGD out of range for the bit depth
    for ct in [0u8, 3, 2] {
        for v in [0u16, 1, 255, 256, 0xffff] {
            let mut spec = pb::PngSpec::new(4, 2, 8, ct, 0);
            if ct == 3 {
                spec.palette = vec![0x40u8; 9];
                spec.pre_idat = vec![(*b"bKGD", vec![(v & 0xff) as u8])];
            } else if ct == 0 {
                spec.pre_idat = vec![(*b"bKGD", v.to_be_bytes().to_vec())];
            } else {
                let mut d = Vec::new();
                for _ in 0..3 {
                    d.extend_from_slice(&v.to_be_bytes());
                }
                spec.pre_idat = vec![(*b"bKGD", d)];
            }
            let rb = pb::rowbytes(8, ct, 4);
            spec.raw = (0..2)
                .flat_map(|_| {
                    let mut r = vec![0u8];
                    r.extend(std::iter::repeat(0u8).take(rb));
                    r
                })
                .collect();
            cases.push((format!("bKGD-ct{ct}-v{v}"), spec.build()));
        }
    }

    for (name, s) in &cases {
        diff_simple(s, &format!("content/{name}"));
    }
}

// ---------------------------------------------------------------------------
// mutation fuzzing: everything the enumeration above may have missed
// ---------------------------------------------------------------------------

#[test]
fn mutation_fuzz() {
    // Single-byte, multi-byte, truncation and insertion mutations of a set of
    // valid streams that between them cover all colour types and interlacing.
    let mut bases: Vec<Vec<u8>> = Vec::new();
    for &(bd, ct) in &[(1u8, 0u8), (8, 0), (16, 0), (8, 2), (16, 2), (4, 3), (8, 3), (8, 4), (8, 6)] {
        for &il in &[0u8, 1] {
            bases.push(pb::make_png(0x999 + bd as u64 * 8 + ct as u64 + il as u64, 9, 5, bd, ct, il));
        }
    }
    // one stream with every ancillary chunk present
    {
        let mut spec = pb::PngSpec::new(9, 5, 8, 2, 0);
        spec.pre_idat = CHUNKS
            .iter()
            .filter(|(n, _)| *n != b"iCCP")
            .map(|(n, d)| (**n, d.to_vec()))
            .collect();
        let mut rng = Rng::new(0x4242);
        spec.raw = pb::raw_rows_none(9, 5, 8, 2, &mut |_y, rb| {
            (0..rb).map(|_| rng.next_u8()).collect()
        });
        bases.push(spec.build());
    }

    let mut rng = Rng::new(0xfeed_face);
    let mut n = 0usize;
    for base in &bases {
        for _ in 0..2000 {
            let mut s = base.clone();
            match rng.below(6) {
                0 => {
                    // flip one byte
                    let i = rng.below(s.len() as u32) as usize;
                    s[i] = rng.next_u8();
                }
                1 => {
                    // flip up to 4 bytes
                    for _ in 0..rng.range(2, 4) {
                        let i = rng.below(s.len() as u32) as usize;
                        s[i] = rng.next_u8();
                    }
                }
                2 => {
                    // truncate
                    let i = rng.below(s.len() as u32) as usize;
                    s.truncate(i);
                }
                3 => {
                    // insert random bytes
                    let i = rng.below(s.len() as u32) as usize;
                    let k = rng.range(1, 8) as usize;
                    let ins: Vec<u8> = (0..k).map(|_| rng.next_u8()).collect();
                    s.splice(i..i, ins);
                }
                4 => {
                    // delete a run
                    let i = rng.below(s.len() as u32) as usize;
                    let k = (rng.range(1, 8) as usize).min(s.len() - i);
                    s.drain(i..i + k);
                }
                _ => {
                    // zero a run
                    let i = rng.below(s.len() as u32) as usize;
                    let k = (rng.range(1, 16) as usize).min(s.len() - i);
                    for b in &mut s[i..i + k] {
                        *b = 0;
                    }
                }
            }
            n += 1;
            diff_simple(&s, &format!("fuzz#{n}"));
        }
    }
}

#[test]
fn format_and_flag_rejections() {
    // out-of-range / nonsensical `png_image.format` and `flags` values passed
    // across the FFI boundary
    let png = valid_rgb8(0x77, 5, 3);
    let mut fmts: Vec<png_uint_32> = (0u32..0x80).collect();
    fmts.extend_from_slice(&[0x80, 0xff, 0x100, 0xffff_ffff, 0x8000_0000]);
    for f in fmts {
        diff_simple_fmt(&png, Some(f), 0, &format!("format {f:#x}"));
    }
    for fl in [0u32, 1, 2, 4, 7, 8, 0xff, 0xffff_ffff] {
        diff_simple_fmt(&png, None, fl, &format!("flags {fl:#x}"));
    }
}

#[test]
fn begin_read_argument_rejections() {
    // NULL / zero-size memory, wrong version, non-NULL opaque
    let b = apis();
    let png = valid_rgb8(0x88, 4, 2);

    let run = |a: &Api, version: u32, ptr_null: bool, size: usize| unsafe {
        let mut img = png_image::default();
        img.version = version;
        let r = (a.png_image_begin_read_from_memory)(
            &mut img,
            if ptr_null {
                std::ptr::null()
            } else {
                png.as_ptr() as *const c_void
            },
            size,
        );
        let t = img.cmp_tuple();
        (a.png_image_free)(&mut img);
        (r, t)
    };

    for version in [0u32, 1, 2, 0xffff_ffff] {
        for ptr_null in [false, true] {
            for size in [0usize, 1, 8, png.len()] {
                if ptr_null && size != 0 {
                    // a NULL pointer with a non-zero size would be UB in the C
                    // memcpy; libpng checks `memory == NULL` first, so keep only
                    // size == 0 plus the pure-NULL case below.
                }
                let c = run(&b.c, version, ptr_null, size);
                let r = run(&b.rs, version, ptr_null, size);
                eq_dbg(
                    &format!("begin_read version={version} null={ptr_null} size={size}"),
                    c,
                    r,
                );
            }
        }
    }
    // NOTE: a non-NULL but bogus `opaque` is NOT a testable input.
    // `png_image_read_init` (pngread.c:1130) only allocates when
    // `image->opaque == NULL`, and the caller then dereferences
    // `image->opaque->memory` (pngread.c:1448) unconditionally -- so a garbage
    // pointer is dereferenced by the C. The C has no check to compare against.

    // `image == NULL`
    unsafe {
        let cr = (b.c.png_image_begin_read_from_memory)(
            std::ptr::null_mut(),
            png.as_ptr() as *const c_void,
            png.len(),
        );
        let rr = (b.rs.png_image_begin_read_from_memory)(
            std::ptr::null_mut(),
            png.as_ptr() as *const c_void,
            png.len(),
        );
        eq_dbg("begin_read image=NULL", cr, rr);
        // png_image_free(NULL) must be a no-op in both
        (b.c.png_image_free)(std::ptr::null_mut());
        (b.rs.png_image_free)(std::ptr::null_mut());
    }
}

#[test]
fn finish_read_argument_rejections() {
    let b = apis();
    let png = valid_rgb8(0x99, 5, 3);

    // buffer NULL, colormap NULL when required, row_stride too small,
    // row_stride zero with a colormapped format, negative strides
    let run = |a: &Api,
               format: png_uint_32,
               buf_null: bool,
               cmap_null: bool,
               stride: i32|
     -> (c_int, Option<c_int>, (u32, u32, u32, u32, u32, u32, u32, String)) {
        unsafe {
            let mut img = png_image::default();
            let begin = (a.png_image_begin_read_from_memory)(
                &mut img,
                png.as_ptr() as *const c_void,
                png.len(),
            );
            if begin == 0 {
                let t = img.cmp_tuple();
                (a.png_image_free)(&mut img);
                return (begin, None, t);
            }
            img.format = format;
            let need = (img.height as usize) * (img.width as usize) * 8;
            let mut buf = vec![0u8; need.max(1) + 8192];
            let mut cmap = vec![0u8; 256 * 4 * 2 + 8192];
            let fin = (a.png_image_finish_read)(
                &mut img,
                std::ptr::null(),
                if buf_null {
                    std::ptr::null_mut()
                } else {
                    buf.as_mut_ptr() as *mut c_void
                },
                stride,
                if cmap_null {
                    std::ptr::null_mut()
                } else {
                    cmap.as_mut_ptr() as *mut c_void
                },
            );
            let t = img.cmp_tuple();
            (a.png_image_free)(&mut img);
            (begin, Some(fin), t)
        }
    };

    for format in [
        PNG_FORMAT_RGB,
        PNG_FORMAT_RGBA,
        PNG_FORMAT_GRAY,
        PNG_FORMAT_LINEAR_RGB,
        PNG_FORMAT_RGB_COLORMAP,
        PNG_FORMAT_RGBA_COLORMAP,
    ] {
        for buf_null in [false, true] {
            for cmap_null in [false, true] {
                for stride in [0i32, 1, 2, 3, 15, -1, -3, -15, i32::MAX, i32::MIN, i32::MIN + 1] {
                    let c = run(&b.c, format, buf_null, cmap_null, stride);
                    let r = run(&b.rs, format, buf_null, cmap_null, stride);
                    eq_dbg(
                        &format!(
                            "finish_read fmt={format:#x} buf_null={buf_null} cmap_null={cmap_null} stride={stride}"
                        ),
                        c,
                        r,
                    );
                }
            }
        }
    }

    // finish_read on a struct that was never begun, and twice in a row
    unsafe {
        for (label, a) in [("C", &b.c), ("RUST", &b.rs)] {
            let _ = label;
            let _ = a;
        }
        let mk = |a: &Api| -> (c_int, (u32, u32, u32, u32, u32, u32, u32, String)) {
            let mut img = png_image::default();
            img.width = 4;
            img.height = 2;
            img.format = PNG_FORMAT_RGB;
            let mut buf = vec![0u8; 4 * 2 * 3];
            let r = (a.png_image_finish_read)(
                &mut img,
                std::ptr::null(),
                buf.as_mut_ptr() as *mut c_void,
                0,
                std::ptr::null_mut(),
            );
            let t = img.cmp_tuple();
            (a.png_image_free)(&mut img);
            (r, t)
        };
        eq_dbg("finish_read without begin", mk(&b.c), mk(&b.rs));

        let twice = |a: &Api| -> Vec<String> {
            let mut img = png_image::default();
            let mut v = Vec::new();
            let bg = (a.png_image_begin_read_from_memory)(
                &mut img,
                png.as_ptr() as *const c_void,
                png.len(),
            );
            v.push(format!("begin:{bg}"));
            img.format = PNG_FORMAT_RGB;
            let need = img.width as usize * img.height as usize * 3;
            let mut buf = vec![0u8; need + 16];
            let f1 = (a.png_image_finish_read)(
                &mut img,
                std::ptr::null(),
                buf.as_mut_ptr() as *mut c_void,
                0,
                std::ptr::null_mut(),
            );
            v.push(format!("finish1:{f1}:{:?}", img.cmp_tuple()));
            let f2 = (a.png_image_finish_read)(
                &mut img,
                std::ptr::null(),
                buf.as_mut_ptr() as *mut c_void,
                0,
                std::ptr::null_mut(),
            );
            v.push(format!("finish2:{f2}:{:?}", img.cmp_tuple()));
            (a.png_image_free)(&mut img);
            (a.png_image_free)(&mut img);
            v.push(format!("after-free:{:?}", img.cmp_tuple()));
            v
        };
        eq_dbg("finish_read twice", twice(&b.c), twice(&b.rs));
    }
}

// ---------------------------------------------------------------------------
// self-check: prove the harness actually observes distinct error messages
// (a differential test that compares "nothing" would pass vacuously)
// ---------------------------------------------------------------------------

#[test]
fn harness_self_check() {
    let b = apis();

    // a valid stream must succeed cleanly in both
    let good = valid_rgb8(0x1234, 6, 4);
    let c = unsafe { simple_read(&b.c, &good, None, 0) };
    let r = unsafe { simple_read(&b.rs, &good, None, 0) };
    assert_eq!(c.begin, 1, "valid stream must be accepted by C");
    assert_eq!(c.finish, Some(1), "valid stream must decode in C");
    assert_eq!(c.after_finish.as_ref().unwrap().6, 0, "no warning expected");
    assert_eq!(c.after_finish.as_ref().unwrap().7, "", "no message expected");
    assert_eq!(c.begin, r.begin);
    assert_eq!(c.finish, r.finish);
    assert_eq!(c.after_finish, r.after_finish);
    assert!(!c.bytes.is_empty());

    // and a representative set of invalid streams must produce a NON-EMPTY,
    // identical message plus warning_or_error >= 2 (error) in both libraries.
    let mut probes: Vec<(&str, Vec<u8>)> = Vec::new();
    probes.push(("bad signature", {
        let mut s = good.clone();
        s[1] = b'X';
        s
    }));
    probes.push(("zero width", with_ihdr(&pb::ihdr_data(0, 2, 8, 2, 0, 0, 0))));
    probes.push(("bad bit depth", with_ihdr(&pb::ihdr_data(4, 2, 3, 0, 0, 0, 0))));
    probes.push((
        "bad colour type",
        with_ihdr(&pb::ihdr_data(4, 2, 8, 1, 0, 0, 0)),
    ));
    probes.push((
        "bad interlace",
        with_ihdr(&pb::ihdr_data(4, 2, 8, 2, 0, 0, 7)),
    ));
    probes.push(("missing PLTE", {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 3, 0, 0, 0));
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 10]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        s
    }));
    probes.push(("critical chunk", {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"cRIT", &[1]);
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 30]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        s
    }));
    probes.push(("truncated", good[..good.len() / 2].to_vec()));
    probes.push(("bad IHDR crc", {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk_bad_crc(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"IDAT", &pb::zlib_store(&[0u8; 30]));
        pb::push_chunk(&mut s, b"IEND", &[]);
        s
    }));
    probes.push(("corrupt zlib", {
        let mut s = Vec::new();
        s.extend_from_slice(&pb::PNG_SIG);
        pb::push_chunk(&mut s, b"IHDR", &pb::ihdr_data(4, 2, 8, 2, 0, 0, 0));
        pb::push_chunk(&mut s, b"IDAT", &[0xde, 0xad, 0xbe, 0xef]);
        pb::push_chunk(&mut s, b"IEND", &[]);
        s
    }));

    let mut seen = std::collections::BTreeSet::new();
    for (label, s) in &probes {
        let c = unsafe { simple_read(&b.c, s, None, 0) };
        let r = unsafe { simple_read(&b.rs, s, None, 0) };
        let (code, msg) = if c.begin == 0 {
            (c.after_begin.6, c.after_begin.7.clone())
        } else {
            let f = c.after_finish.clone().unwrap();
            (f.6, f.7)
        };
        assert!(
            !msg.is_empty(),
            "probe {label:?} produced no message in C -- the harness would be \
             comparing nothing"
        );
        // >= 1 means "at least a warning was recorded"; some rejections are
        // benign on read (the chunk is discarded and a warning is issued).
        assert!(
            code >= PNG_IMAGE_WARNING,
            "probe {label:?} was not reported at all by C (code {code})"
        );
        eq_dbg(&format!("self-check {label} begin"), c.begin, r.begin);
        eq_dbg(&format!("self-check {label} finish"), c.finish, r.finish);
        eq_dbg(
            &format!("self-check {label} after_begin"),
            c.after_begin.clone(),
            r.after_begin.clone(),
        );
        eq_dbg(
            &format!("self-check {label} after_finish"),
            c.after_finish.clone(),
            r.after_finish.clone(),
        );
        seen.insert(msg);
    }
    // the probes must exercise several DIFFERENT messages, not one generic one
    assert!(
        seen.len() >= 6,
        "expected many distinct error messages, saw {}: {seen:?}",
        seen.len()
    );
    eprintln!("distinct error messages observed: {seen:#?}");
}

// ---------------------------------------------------------------------------
// structured fuzzing: mutate the CHUNK FRAMING rather than random bytes, so the
// length / name / CRC validation paths are hit far more often than a blind
// byte-flipper reaches them
// ---------------------------------------------------------------------------

/// Split a PNG into (signature, [(len, name, data, crc)]).
fn parse_chunks(png: &[u8]) -> Option<(Vec<u8>, Vec<(u32, [u8; 4], Vec<u8>, u32)>)> {
    if png.len() < 8 {
        return None;
    }
    let sig = png[..8].to_vec();
    let mut i = 8usize;
    let mut out = Vec::new();
    while i + 12 <= png.len() {
        let len = u32::from_be_bytes([png[i], png[i + 1], png[i + 2], png[i + 3]]);
        let name = [png[i + 4], png[i + 5], png[i + 6], png[i + 7]];
        let dstart = i + 8;
        let dend = dstart.checked_add(len as usize)?;
        if dend + 4 > png.len() {
            return None;
        }
        let data = png[dstart..dend].to_vec();
        let crc = u32::from_be_bytes([png[dend], png[dend + 1], png[dend + 2], png[dend + 3]]);
        out.push((len, name, data, crc));
        i = dend + 4;
    }
    Some((sig, out))
}

fn rebuild(sig: &[u8], chunks: &[(u32, [u8; 4], Vec<u8>, u32)]) -> Vec<u8> {
    let mut out = sig.to_vec();
    for (len, name, data, crc) in chunks {
        out.extend_from_slice(&len.to_be_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        out.extend_from_slice(&crc.to_be_bytes());
    }
    out
}

#[test]
fn structured_chunk_fuzz() {
    let mut bases: Vec<Vec<u8>> = Vec::new();
    for &(bd, ct) in &[(1u8, 0u8), (8, 0), (16, 0), (8, 2), (16, 2), (4, 3), (8, 3), (8, 4), (8, 6)] {
        for &il in &[0u8, 1] {
            bases.push(pb::make_png(0x777 + bd as u64 * 8 + ct as u64 + il as u64, 7, 4, bd, ct, il));
        }
    }
    // one with every ancillary chunk so there is plenty of framing to mutate
    {
        let mut spec = pb::PngSpec::new(7, 4, 8, 2, 0);
        spec.pre_idat = CHUNKS.iter().map(|(n, d)| (**n, d.to_vec())).collect();
        spec.idat_chunks = 3;
        let mut rng = Rng::new(0x1357);
        spec.raw = pb::raw_rows_none(7, 4, 8, 2, &mut |_y, rb| {
            (0..rb).map(|_| rng.next_u8()).collect()
        });
        bases.push(spec.build());
    }

    let mut rng = Rng::new(0x0bad_c0de);
    let mut n = 0usize;
    for base in &bases {
        let Some((sig, chunks)) = parse_chunks(base) else {
            continue;
        };
        for _ in 0..1200 {
            let mut cs = chunks.clone();
            if cs.is_empty() {
                break;
            }
            let k = rng.below(cs.len() as u32) as usize;
            match rng.below(10) {
                0 => cs[k].0 = rng.interesting_u32(),          // declared length
                1 => cs[k].3 = rng.next_u32(),                 // CRC
                2 => {
                    // chunk name byte
                    let b = rng.below(4) as usize;
                    cs[k].1[b] = rng.next_u8();
                }
                3 => {
                    // truncate the payload (length field left alone)
                    let l = cs[k].2.len();
                    if l > 0 {
                        cs[k].2.truncate(rng.below(l as u32) as usize);
                    }
                }
                4 => {
                    // extend the payload
                    let extra = rng.range(1, 8) as usize;
                    for _ in 0..extra {
                        cs[k].2.push(rng.next_u8());
                    }
                }
                5 => {
                    // flip a payload byte
                    if !cs[k].2.is_empty() {
                        let i = rng.below(cs[k].2.len() as u32) as usize;
                        cs[k].2[i] = rng.next_u8();
                    }
                }
                6 => {
                    // duplicate the chunk
                    let c = cs[k].clone();
                    cs.insert(k, c);
                }
                7 => {
                    // delete the chunk
                    cs.remove(k);
                }
                8 => {
                    // move the chunk to the end (after IEND)
                    let c = cs.remove(k);
                    cs.push(c);
                }
                _ => {
                    // swap two chunks
                    let j = rng.below(cs.len() as u32) as usize;
                    cs.swap(k, j);
                }
            }
            n += 1;
            let s = rebuild(&sig, &cs);
            diff_simple(&s, &format!("structfuzz#{n}"));
        }
    }
    assert!(n > 10000, "expected a large number of structured cases, got {n}");
}
