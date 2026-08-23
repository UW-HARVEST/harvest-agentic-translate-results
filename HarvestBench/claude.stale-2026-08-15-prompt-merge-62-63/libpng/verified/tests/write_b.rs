//! Write-pipeline differential tests, part B (CONFIGS.md rows W13..W24).
//!
//! Every test drives both the C reference `libpng.so` and the translated Rust
//! `liblibpng.so` through the identical call sequence and compares the produced
//! PNG byte stream (`Trace::out`) together with the whole event trace, byte for
//! byte.
//!
//! All pixel/chunk data is produced by a deterministic PRNG seeded with a
//! literal and is generated *outside* the `diff` closure, so both libraries see
//! byte-identical input.  No pointer value is ever logged - only null-ness,
//! sizes and contents.
mod support;

use std::ffi::{c_char, c_int, c_void, CString};
use support::core::*;
use support::*;

// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

/// The 15 legal (colour_type, bit_depth) combinations.
const ALL_COMBOS: &[(c_int, c_int)] = &[
    (PNG_COLOR_TYPE_GRAY, 1),
    (PNG_COLOR_TYPE_GRAY, 2),
    (PNG_COLOR_TYPE_GRAY, 4),
    (PNG_COLOR_TYPE_GRAY, 8),
    (PNG_COLOR_TYPE_GRAY, 16),
    (PNG_COLOR_TYPE_RGB, 8),
    (PNG_COLOR_TYPE_RGB, 16),
    (PNG_COLOR_TYPE_PALETTE, 1),
    (PNG_COLOR_TYPE_PALETTE, 2),
    (PNG_COLOR_TYPE_PALETTE, 4),
    (PNG_COLOR_TYPE_PALETTE, 8),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
    (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    (PNG_COLOR_TYPE_RGB_ALPHA, 16),
];

fn channels(ct: c_int) -> u32 {
    pngbuild::channels(ct as u8)
}

/// `PNG_ROWBYTES(pixel_depth, width)`
fn rowbytes_pd(pixel_depth: u32, w: u32) -> usize {
    ((pixel_depth as u64 * w as u64 + 7) / 8) as usize
}

fn rb(ct: c_int, bd: c_int, w: u32) -> usize {
    pngbuild::rowbytes(ct as u8, bd as u8, w)
}

/// Store a (possibly sub-byte) palette index, MSB-first as PNG requires.
fn set_index(row: &mut [u8], x: usize, bd: c_int, idx: u8) {
    if bd == 8 {
        row[x] = idx;
        return;
    }
    let per = (8 / bd) as usize;
    let byte = x / per;
    let shift = 8 - bd as usize * (x % per + 1);
    let mask = ((1u16 << bd) - 1) as u8;
    row[byte] = (row[byte] & !(mask << shift)) | ((idx & mask) << shift);
}

/// `h` rows of pseudo-random content for the given image shape.  Palette rows
/// only contain indices `< npal`.  Each row has 8 bytes of slack.
fn make_rows(rng: &mut Rng, ct: c_int, bd: c_int, w: u32, h: u32, npal: u32) -> Vec<Vec<u8>> {
    let n = rb(ct, bd, w);
    (0..h)
        .map(|_| {
            let mut row = vec![0u8; n + 8];
            if ct == PNG_COLOR_TYPE_PALETTE {
                for x in 0..w as usize {
                    let idx = rng.below(npal) as u8;
                    set_index(&mut row, x, bd, idx);
                }
            } else {
                for i in 0..n {
                    row[i] = rng.byte();
                }
            }
            row
        })
        .collect()
}

/// Rows of a fixed byte length (used where a write transform changes the number
/// of user-supplied channels or the user bit depth).
fn make_flat_rows(rng: &mut Rng, bytes: usize, h: u32) -> Vec<Vec<u8>> {
    (0..h)
        .map(|_| {
            let mut row = vec![0u8; bytes + 8];
            for i in 0..bytes {
                row[i] = rng.byte();
            }
            row
        })
        .collect()
}

/// A compressible 8-bit RGB image: linear gradients plus a little noise.
fn gradient_rows(rng: &mut Rng, w: u32, h: u32) -> Vec<Vec<u8>> {
    let n = 3 * w as usize;
    (0..h)
        .map(|y| {
            let mut row = vec![0u8; n + 8];
            for x in 0..w as usize {
                let base = (x * 9 + y as usize * 5) as u8;
                row[3 * x] = base.wrapping_add(rng.byte() & 0x0f);
                row[3 * x + 1] = base.wrapping_mul(2).wrapping_add(rng.byte() & 0x07);
                row[3 * x + 2] = base.wrapping_add(0x40) ^ (rng.byte() & 0x03);
            }
            row
        })
        .collect()
}

/// An incompressible 8-bit RGB image (pure noise): forces the deflate output to
/// be at least as large as the input, so small compression buffers split the
/// IDAT into many chunks.
fn noise_rows(rng: &mut Rng, w: u32, h: u32) -> Vec<Vec<u8>> {
    make_flat_rows(rng, 3 * w as usize, h)
}

/// 16-bit rows with real structure so the filter heuristic makes choices.
fn rows16(rng: &mut Rng, ct: c_int, w: u32, h: u32) -> Vec<Vec<u8>> {
    let ch = channels(ct) as usize;
    let n = 2 * ch * w as usize;
    (0..h)
        .map(|y| {
            let mut row = vec![0u8; n + 8];
            for x in 0..w as usize {
                for c in 0..ch {
                    let base = (x as u32 * 761 + y * 4099 + c as u32 * 137) & 0xffff;
                    let v = if rng.below(8) == 0 {
                        (base ^ (rng.next_u32() & 0x0fff)) as u16
                    } else {
                        base as u16
                    };
                    let o = 2 * (ch * x + c);
                    row[o] = (v >> 8) as u8;
                    row[o + 1] = v as u8;
                }
            }
            row
        })
        .collect()
}

fn ptr_vec(rows: &mut [Vec<u8>]) -> Vec<*mut u8> {
    rows.iter_mut().map(|r| r.as_mut_ptr()).collect()
}

/// CRC over every row buffer: libpng must not modify the caller's rows.
fn rowsum(rows: &[Vec<u8>]) -> u32 {
    let mut all = Vec::new();
    for r in rows {
        all.extend_from_slice(r);
    }
    pngbuild::crc32(&all)
}

/// The tail of a trace, for assertion messages.
fn tail(t: &Trace) -> String {
    let n = t.lines.len();
    let head = t.lines[..std::cmp::min(4, n)].join(" | ");
    format!("{head} ... {}", t.lines[n.saturating_sub(6)..].join(" | "))
}

/// Sanity guard so a configuration can never silently do nothing.
fn checked(t: Trace) -> Trace {
    assert_eq!(
        t.rc,
        0,
        "unexpected longjmp out of the write driver: {}",
        tail(&t)
    );
    assert!(
        t.out.starts_with(&pngbuild::SIG) && t.out.len() > 8,
        "no PNG datastream produced (out.len={}) {}",
        t.out.len(),
        tail(&t)
    );
    t
}

/// `with_write` plus the sanity guard.
fn wwrite(lib: &Lib, body: &mut dyn FnMut(&Core, Png, Info)) -> Trace {
    checked(with_write(lib, body))
}

/// `with_write` requiring only that the driver ran to completion (used where
/// the produced stream legitimately does not start with a full signature).
fn wwrite_nosig(lib: &Lib, body: &mut dyn FnMut(&Core, Png, Info)) -> Trace {
    let t = with_write(lib, body);
    assert_eq!(
        t.rc,
        0,
        "unexpected longjmp out of the write driver: {}",
        tail(&t)
    );
    assert!(t.out.len() > 8, "no bytes produced");
    t
}

/// `with_write` tolerating a longjmp: used where libpng legitimately raises a
/// fatal error for some of the configurations under test (the trace, including
/// `rc` and the truncated datastream, must still match).
fn wsoft(lib: &Lib, body: &mut dyn FnMut(&Core, Png, Info)) -> Trace {
    let t = with_write(lib, body);
    assert!(
        t.out.starts_with(&pngbuild::SIG) && t.out.len() > 8,
        "no PNG datastream produced (out.len={}) {}",
        t.out.len(),
        tail(&t)
    );
    t
}

/// Number of trace lines starting with `p`.
fn count_prefix(t: &Trace, p: &str) -> usize {
    t.lines.iter().filter(|l| l.starts_with(p)).count()
}

/// Number of chunks called `name` in a produced datastream.
fn count_chunk(out: &[u8], name: &[u8; 4]) -> usize {
    pngbuild::split(out)
        .iter()
        .filter(|c| &c.name == name)
        .count()
}

/// Number of bytes handed to the write callback so far.  Logging this between
/// calls pins down the *ordering* of the produced bytes relative to the other
/// trace events (FLUSH, row callbacks, ...).
fn outlen() -> usize {
    with_session(|s| s.out.len())
}

/// The chunk-name sequence and per-chunk length of a produced datastream.
fn chunk_summary(out: &[u8]) -> String {
    let cs = pngbuild::split(out);
    let mut s = String::new();
    for c in &cs {
        s.push_str(&format!(
            "{}:{} ",
            String::from_utf8_lossy(&c.name),
            c.data.len()
        ));
    }
    format!("nchunks={} [{}]", cs.len(), s.trim_end())
}

/// `with_write` + guard, with the chunk summary of the produced datastream
/// appended to the trace (makes a divergence in the chunk layout readable).
fn wwrite_c(lib: &Lib, body: &mut dyn FnMut(&Core, Png, Info)) -> Trace {
    let mut t = wwrite(lib, body);
    let s = chunk_summary(&t.out);
    t.lines.push(s);
    t
}

/// Same as `wwrite_c` but without the signature-prefix requirement.
fn wwrite_c_nosig(lib: &Lib, body: &mut dyn FnMut(&Core, Png, Info)) -> Trace {
    let mut t = wwrite_nosig(lib, body);
    let s = chunk_summary(&t.out);
    t.lines.push(s);
    t
}

/// Same, but tolerating a longjmp out of the driver (used where libpng is
/// expected to raise a fatal error for some configurations).
fn write_c(lib: &Lib, body: &mut dyn FnMut(&Core, Png, Info)) -> Trace {
    let mut t = with_write(lib, body);
    let s = chunk_summary(&t.out);
    t.lines.push(s);
    t
}

unsafe fn log_hdr(c: &Core, png: Png, info: Info) {
    log(format!(
        "rowbytes={} channels={} cbuf={}",
        (c.get_rowbytes)(png, info),
        (c.get_channels)(png, info),
        (c.get_compression_buffer_size)(png)
    ));
}

/// A 256-entry random palette; only the first `3*npal` bytes are ever used.
fn full_palette(rng: &mut Rng) -> Vec<u8> {
    rng.bytes(3 * 256)
}

/// Log the tIME and text contents of an info struct.  `log_all_info` cannot be
/// used on an info struct that never saw `png_set_IHDR`, because `png_get_IHDR`
/// re-runs `png_check_IHDR` and png_errors on the zeroed header.
unsafe fn log_time_and_text(c: &Core, png: Png, info: Info) {
    let mut tp: *mut u8 = std::ptr::null_mut();
    let r = (c.get_tIME)(png, info, &mut tp);
    log(format!("tIME rc={r}"));
    if r != 0 && !tp.is_null() {
        let v = *(tp as *const PngTime);
        log(format!("tIME v={v:?}"));
    }
    let mut tptr: *mut c_void = std::ptr::null_mut();
    let mut ntext: c_int = 0;
    let n = (c.get_text)(png, info, &mut tptr, &mut ntext);
    log(format!("text n={n} num={ntext}"));
    if n > 0 && !tptr.is_null() {
        let arr = std::slice::from_raw_parts(tptr as *const PngText, n as usize);
        for (i, t) in arr.iter().enumerate() {
            log(format!(
                "text[{i}] comp={} key={} text={} tlen={} ilen={} lang={} langkey={}",
                t.compression,
                cstr(t.key),
                cstr(t.text),
                t.text_length,
                t.itxt_length,
                cstr(t.lang),
                cstr(t.lang_key)
            ));
        }
    }
}

unsafe fn maybe_plte(c: &Core, png: Png, info: Info, ct: c_int, pal: &[u8], npal: u32) {
    if ct == PNG_COLOR_TYPE_PALETTE {
        (c.set_PLTE)(png, info, pal.as_ptr(), npal as c_int);
    }
}

// ---------------------------------------------------------------------------
// W13 — png_set_compression_method + png_set_compression_buffer_size
// ---------------------------------------------------------------------------
//
// `png_set_compression_buffer_size` (pngset.c):
//   * size 0 or > PNG_UINT_31_MAX  -> png_error   (Phase C)
//   * size < 6                     -> warning "Compression buffer size cannot
//                                     be reduced below 6" and the size is left
//                                     unchanged
//   * zowner != 0 (deflate claimed for IDAT, i.e. after the first row)
//                                  -> warning "Compression buffer size cannot
//                                     be changed because it is in use"
//   * otherwise the buffer list is freed and zbuffer_size is updated; setting
//     the size after png_write_info but before the first row still takes
//     effect because the IDAT deflate stream is only claimed by the first
//     png_compress_IDAT call.
//
// `png_set_compression_method`: only 8 is legal (other values warn -> Phase C).

/// When the buffer size is applied.
#[derive(Copy, Clone)]
enum When {
    BeforeInfo,
    AfterInfo,
    AfterFirstRow,
}

#[test]
fn w13_compression_buffer_size() {
    let mut rng = Rng::new(0x2313);
    // Two 8-bit RGB images whose raw IDAT input is far larger than most of the
    // buffer sizes below: noise (incompressible, ~12 KB of deflate output) and
    // a gradient (compressible).
    let images: Vec<(&str, u32, u32, Vec<Vec<u8>>)> = vec![
        ("noise", 64, 64, noise_rows(&mut rng, 64, 64)),
        ("grad", 40, 48, gradient_rows(&mut rng, 40, 48)),
        ("noise2", 24, 90, noise_rows(&mut rng, 24, 90)),
    ];
    let sizes: [usize; 7] = [1, 2, 3, 100, 1024, 8192, 65536];
    for (name, w, h, rows) in &images {
        for &size in &sizes {
            for (wi, when) in [When::BeforeInfo, When::AfterInfo, When::AfterFirstRow]
                .iter()
                .enumerate()
            {
                let when = *when;
                let label = format!("W13 img={name} size={size} when={wi}");
                diff(&label, |lib| {
                    let t = wwrite_c(lib, &mut |c, png, info| unsafe {
                        (c.set_IHDR)(
                            png,
                            info,
                            *w,
                            *h,
                            8,
                            PNG_COLOR_TYPE_RGB,
                            PNG_INTERLACE_NONE,
                            0,
                            0,
                        );
                        // Only method 8 is legal for PNG.
                        (c.set_compression_method)(png, 8);
                        log(format!("cbuf.initial={}", (c.get_compression_buffer_size)(png)));
                        if matches!(when, When::BeforeInfo) {
                            (c.set_compression_buffer_size)(png, size);
                            log(format!("cbuf.set={}", (c.get_compression_buffer_size)(png)));
                        }
                        log_hdr(c, png, info);
                        (c.write_info)(png, info);
                        if matches!(when, When::AfterInfo) {
                            (c.set_compression_buffer_size)(png, size);
                            log(format!("cbuf.set={}", (c.get_compression_buffer_size)(png)));
                        }
                        for (i, r) in rows.iter().enumerate() {
                            (c.write_row)(png, r.as_ptr());
                            if i == 0 && matches!(when, When::AfterFirstRow) {
                                (c.set_compression_buffer_size)(png, size);
                                log(format!(
                                    "cbuf.set={}",
                                    (c.get_compression_buffer_size)(png)
                                ));
                            }
                        }
                        (c.write_end)(png, info);
                        log(format!("cbuf.final={}", (c.get_compression_buffer_size)(png)));
                        log(format!("rowsum={:08x}", rowsum(rows)));
                    });
                    // The configuration must really have been exercised: both
                    // libraries have to emit the same clamp/in-use warning and
                    // the same number of IDAT chunks.
                    let warns = count_prefix(&t, "WARNING(");
                    let expect_warn = matches!(when, When::AfterFirstRow) || size < 6;
                    assert_eq!(
                        warns,
                        expect_warn as usize,
                        "[{}] {label}: warnings={warns}",
                        lib.tag
                    );
                    let idats = count_chunk(&t.out, b"IDAT");
                    assert!(idats >= 1, "[{}] {label}: idats={idats}", lib.tag);
                    // A 100-byte compression buffer must split every one of
                    // these images into many IDAT chunks.
                    if size == 100 && !matches!(when, When::AfterFirstRow) {
                        assert!(idats >= 3, "[{}] {label}: idats={idats}", lib.tag);
                    }
                    t
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W14 — png_set_flush + png_write_flush
// ---------------------------------------------------------------------------
//
// `png_write_filtered_row` increments `flush_rows` and calls
// `png_write_flush` once `flush_rows >= flush_dist` (with flush_dist > 0).
// `png_write_flush` is a no-op once `row_number >= num_rows`.
// The harness `cb_flush` logs `FLUSH`, and `out.len` is logged after every row,
// so the flush ordering relative to the produced bytes is compared.

#[test]
fn w14_flush() {
    let mut rng = Rng::new(0x2414);
    let w = 14u32;
    let h = 12u32;
    let images: Vec<(&str, Vec<Vec<u8>>)> = vec![
        ("grad", gradient_rows(&mut rng, w, h)),
        ("noise", noise_rows(&mut rng, w, h)),
    ];
    for (name, rows) in &images {
        for &n in &[0i32, 1, 2, 3, 7] {
            // mode 0: automatic flushing only
            // mode 1: + explicit png_write_flush every 3rd row
            // mode 2: + explicit png_write_flush before the first and after the
            //           last row (the latter is the no-op path)
            // mode 3: png_set_flush called again (to 0) half way through
            for mode in 0..4 {
                for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                    let label = format!("W14 img={name} n={n} mode={mode} il={il}");
                    // Two png_write_flush calls with no row data in between make
                    // zlib return Z_BUF_ERROR, which libpng turns into a fatal
                    // png_error ("buffer error").  That happens for the explicit
                    // flush modes on interlaced images (most passes skip most
                    // rows), so a longjmp is tolerated there; the purely
                    // automatic modes must always run to completion.
                    let soft = mode == 1 || mode == 2;
                    diff(&label, |lib| {
                        let mut body = |c: &Core, png: Png, info: Info| unsafe {
                            (c.set_IHDR)(png, info, w, h, 8, PNG_COLOR_TYPE_RGB, il, 0, 0);
                            (c.set_flush)(png, n);
                            log_hdr(c, png, info);
                            (c.write_info)(png, info);
                            let passes = if il == PNG_INTERLACE_ADAM7 {
                                let p = (c.set_interlace_handling)(png);
                                log(format!("passes={p}"));
                                p
                            } else {
                                1
                            };
                            if mode == 2 {
                                (c.write_flush)(png);
                                log(format!("pre out={}", outlen()));
                            }
                            for pass in 0..passes {
                                for (i, r) in rows.iter().enumerate() {
                                    (c.write_row)(png, r.as_ptr());
                                    log(format!("row {pass}/{i} out={}", outlen()));
                                    if mode == 1 && i % 3 == 2 {
                                        (c.write_flush)(png);
                                        log(format!("xflush {pass}/{i} out={}", outlen()));
                                    }
                                    if mode == 3 && i == 5 {
                                        (c.set_flush)(png, 0);
                                        log("flush off".to_string());
                                    }
                                }
                            }
                            if mode == 2 {
                                (c.write_flush)(png);
                                log(format!("post out={}", outlen()));
                            }
                            (c.write_end)(png, info);
                            log(format!("end out={}", outlen()));
                            log(format!("rowsum={:08x}", rowsum(rows)));
                        };
                        let t = if soft {
                            wsoft(lib, &mut body)
                        } else {
                            wwrite(lib, &mut body)
                        };
                        // The flush callback must actually have run whenever
                        // flushing is enabled or requested explicitly.
                        let flushes = count_prefix(&t, "FLUSH");
                        if mode == 1 || (n > 0 && mode != 3) {
                            assert!(flushes > 0, "[{}] {label}: no FLUSH", lib.tag);
                        }
                        // With flushing off and no explicit call there must be no
                        // flush at all.  (mode 2 is excluded: after the last
                        // Adam7 pass png_write_finish_row leaves row_number at 0
                        // while num_rows is the image height, so the trailing
                        // png_write_flush is *not* a no-op and re-claims the
                        // deflate stream - a real libpng quirk that both
                        // libraries must reproduce.)
                        if n == 0 && mode != 1 && mode != 2 {
                            assert_eq!(flushes, 0, "[{}] {label}: FLUSH={flushes}", lib.tag);
                        }
                        t
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W15 — png_write_sig / png_write_chunk / png_write_chunk_start+data+end
// ---------------------------------------------------------------------------

/// A legal private, ancillary chunk name: lower-case first byte (ancillary),
/// lower-case second byte (private), upper-case third byte (reserved bit must
/// be 0), either case for the fourth (safe-to-copy) byte.
fn private_name(rng: &mut Rng) -> [u8; 4] {
    let lo = |r: &mut Rng| b'a' + r.below(26) as u8;
    let up = |r: &mut Rng| b'A' + r.below(26) as u8;
    let b0 = lo(rng);
    let b1 = lo(rng);
    let b2 = up(rng);
    let b3 = if rng.next_u32() & 1 != 0 {
        lo(rng)
    } else {
        up(rng)
    };
    [b0, b1, b2, b3]
}

#[test]
fn w15_manual_chunks() {
    let mut rng = Rng::new(0x2515);
    let w = 9u32;
    let h = 5u32;
    for &plen in &[0usize, 1, 13, 1000] {
        // sig_mode 0: no explicit png_write_sig (png_write_info writes it)
        // sig_mode 1: explicit png_write_sig + png_set_sig_bytes(8) so the
        //             internal call writes nothing
        // sig_mode 2: explicit png_write_sig, then png_write_info writes a
        //             second signature (legal calls, deterministic bytes)
        // sig_mode 3: png_set_sig_bytes(3) then png_write_sig writes 5 bytes
        for sig_mode in 0..4 {
            for rep in 0..2 {
                // 6 chunk names + payloads, generated outside the closure.
                let names: Vec<[u8; 4]> = (0..6).map(|_| private_name(&mut rng)).collect();
                let payload: Vec<u8> = rng.bytes(plen);
                let rows = make_rows(&mut rng, PNG_COLOR_TYPE_RGB, 8, w, h, 1);
                // Split points for the chunk_start/data*/end variant.
                let splits: Vec<usize> = if plen == 0 {
                    vec![0]
                } else {
                    let a = 1 + rng.below(plen as u32) as usize;
                    let b = a + rng.below((plen - a + 1) as u32) as usize;
                    vec![a, b, plen]
                };
                let label = format!("W15 plen={plen} sig={sig_mode} rep={rep}");
                diff(&label, |lib| {
                    let set_sig_bytes: unsafe extern "C" fn(Png, c_int) =
                        lib.f("png_set_sig_bytes");
                    let t = wwrite_c_nosig(lib, &mut |c, png, info| unsafe {
                        (c.set_IHDR)(
                            png,
                            info,
                            w,
                            h,
                            8,
                            PNG_COLOR_TYPE_RGB,
                            PNG_INTERLACE_NONE,
                            0,
                            0,
                        );
                        match sig_mode {
                            1 => {
                                (c.write_sig)(png);
                                log(format!("sig out={}", outlen()));
                                set_sig_bytes(png, 8);
                            }
                            2 => {
                                (c.write_sig)(png);
                                log(format!("sig out={}", outlen()));
                            }
                            3 => {
                                set_sig_bytes(png, 3);
                                (c.write_sig)(png);
                                log(format!("sig out={}", outlen()));
                                set_sig_bytes(png, 8);
                            }
                            _ => {}
                        }
                        (c.write_info)(png, info);
                        log(format!("info out={}", outlen()));

                        // (a) one-shot chunk with the whole payload
                        (c.write_chunk)(
                            png,
                            names[0].as_ptr(),
                            if payload.is_empty() {
                                std::ptr::null()
                            } else {
                                payload.as_ptr()
                            },
                            payload.len(),
                        );
                        // (b) one-shot chunk with an explicit NULL payload
                        (c.write_chunk)(png, names[1].as_ptr(), std::ptr::null(), 0);
                        // (c) start + several data calls + end
                        (c.write_chunk_start)(png, names[2].as_ptr(), payload.len() as u32);
                        let mut from = 0usize;
                        for &to in &splits {
                            let n = to - from;
                            (c.write_chunk_data)(
                                png,
                                if n == 0 {
                                    std::ptr::null()
                                } else {
                                    payload[from..].as_ptr()
                                },
                                n,
                            );
                            from = to;
                        }
                        (c.write_chunk_end)(png);
                        // (d) zero-length chunk written the long way
                        (c.write_chunk_start)(png, names[3].as_ptr(), 0);
                        (c.write_chunk_data)(png, payload.as_ptr(), 0);
                        (c.write_chunk_end)(png);
                        log(format!("mid out={}", outlen()));

                        for r in &rows {
                            (c.write_row)(png, r.as_ptr());
                        }
                        log(format!("rows out={}", outlen()));

                        // Chunks between the last IDAT and IEND.
                        (c.write_chunk)(
                            png,
                            names[4].as_ptr(),
                            if payload.is_empty() {
                                std::ptr::null()
                            } else {
                                payload.as_ptr()
                            },
                            payload.len(),
                        );
                        (c.write_chunk_start)(png, names[5].as_ptr(), payload.len() as u32);
                        (c.write_chunk_data)(png, payload.as_ptr(), payload.len());
                        (c.write_chunk_end)(png);

                        (c.write_end)(png, info);
                        log(format!("rowsum={:08x}", rowsum(&rows)));
                    });
                    // Every manually written chunk must be present.  For
                    // sig_mode 2/3 the stream is not a well-formed single-
                    // signature PNG, so the chunk walk does not apply.
                    if sig_mode <= 1 {
                        for nm in &names {
                            assert_eq!(
                                count_chunk(&t.out, nm),
                                1,
                                "[{}] {label}: chunk {} missing",
                                lib.tag,
                                String::from_utf8_lossy(nm)
                            );
                        }
                    }
                    // The signature must appear exactly as many times as it was
                    // written (sig_mode 3 only writes its last five bytes).
                    let sigs = t.out.windows(8).filter(|w| *w == pngbuild::SIG).count();
                    let want = match sig_mode {
                        2 => 2,
                        3 => 0,
                        _ => 1,
                    };
                    assert_eq!(sigs, want, "[{}] {label}: signatures={sigs}", lib.tag);
                    t
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W16 — png_write_info_before_PLTE + png_write_info
// ---------------------------------------------------------------------------

#[test]
fn w16_write_info_before_plte() {
    let mut rng = Rng::new(0x2616);
    let key_a = CString::new("Title").unwrap();
    let txt_a = CString::new("W16 header text").unwrap();
    let key_b = CString::new("Author").unwrap();
    let txt_b = CString::new("differential harness").unwrap();

    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_PALETTE, 1),
        (PNG_COLOR_TYPE_PALETTE, 2),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    ] {
        let npal = 1u32 << bd;
        let w = 7u32;
        let h = 4u32;
        for variant in 0..4 {
            for rep in 0..2 {
                let pal = full_palette(&mut rng);
                let rows = make_rows(&mut rng, ct, bd, w, h, npal);
                let trns: Vec<u8> = rng.bytes(npal as usize);
                let hist: Vec<u16> = (0..npal).map(|_| rng.next_u32() as u16).collect();
                let maxb = if ct == PNG_COLOR_TYPE_PALETTE { 8 } else { bd };
                let sb = std::cmp::max(1, maxb - 1) as u8;
                let sbit = PngColor8 {
                    red: sb,
                    green: sb,
                    blue: sb,
                    gray: sb,
                    alpha: sb,
                };
                let bkgd = PngColor16 {
                    index: rng.below(npal) as u8,
                    red: rng.next_u32() as u16 & 0xff,
                    green: rng.next_u32() as u16 & 0xff,
                    blue: rng.next_u32() as u16 & 0xff,
                    gray: rng.next_u32() as u16 & 0xff,
                };
                let extra = rng.bytes(9);
                let extra_name = private_name(&mut rng);
                let texts = [
                    PngText {
                        compression: PNG_TEXT_COMPRESSION_NONE,
                        key: key_a.as_ptr() as *mut c_char,
                        text: txt_a.as_ptr() as *mut c_char,
                        ..Default::default()
                    },
                    PngText {
                        compression: PNG_TEXT_COMPRESSION_NONE,
                        key: key_b.as_ptr() as *mut c_char,
                        text: txt_b.as_ptr() as *mut c_char,
                        ..Default::default()
                    },
                ];
                let label = format!("W16 ct={ct} bd={bd} variant={variant} rep={rep}");
                diff(&label, |lib| {
                    let t = wwrite_c(lib, &mut |c, png, info| unsafe {
                        (c.set_IHDR)(png, info, w, h, bd, ct, PNG_INTERLACE_NONE, 0, 0);
                        if ct == PNG_COLOR_TYPE_PALETTE {
                            (c.set_PLTE)(png, info, pal.as_ptr(), npal as c_int);
                            (c.set_tRNS)(
                                png,
                                info,
                                trns.as_ptr(),
                                npal as c_int,
                                std::ptr::null(),
                            );
                            (c.set_hIST)(png, info, hist.as_ptr());
                        }
                        (c.set_gAMA_fixed)(png, info, 45455);
                        (c.set_sBIT)(png, info, &sbit as *const PngColor8 as *const u8);
                        (c.set_bKGD)(png, info, &bkgd as *const PngColor16 as *const u8);
                        (c.set_pHYs)(png, info, 3000, 2500, PNG_RESOLUTION_METER);
                        (c.set_text)(png, info, texts.as_ptr() as *const c_void, 2);
                        log_hdr(c, png, info);

                        match variant {
                            0 => {
                                // png_write_info calls it internally
                                (c.write_info)(png, info);
                            }
                            1 => {
                                (c.write_info_before_PLTE)(png, info);
                                log(format!("before_PLTE out={}", outlen()));
                                (c.write_info)(png, info);
                            }
                            2 => {
                                // the second call is a no-op (mode flag)
                                (c.write_info_before_PLTE)(png, info);
                                log(format!("before_PLTE out={}", outlen()));
                                (c.write_info_before_PLTE)(png, info);
                                log(format!("before_PLTE2 out={}", outlen()));
                                (c.write_info)(png, info);
                            }
                            _ => {
                                // an application chunk between the header and
                                // the PLTE
                                (c.write_info_before_PLTE)(png, info);
                                log(format!("before_PLTE out={}", outlen()));
                                (c.write_chunk)(
                                    png,
                                    extra_name.as_ptr(),
                                    extra.as_ptr(),
                                    extra.len(),
                                );
                                (c.write_info)(png, info);
                            }
                        }
                        log(format!("info out={}", outlen()));
                        log_all_info(c, png, info);
                        for r in &rows {
                            (c.write_row)(png, r.as_ptr());
                        }
                        (c.write_end)(png, info);
                        log(format!("rowsum={:08x}", rowsum(&rows)));
                    });
                    // Every ancillary chunk that was set must have been written
                    // exactly once, whichever split of the header write is used.
                    for nm in [b"gAMA", b"sBIT", b"bKGD", b"pHYs"] {
                        assert_eq!(
                            count_chunk(&t.out, nm),
                            1,
                            "[{}] {label}: {} missing",
                            lib.tag,
                            String::from_utf8_lossy(nm)
                        );
                    }
                    assert_eq!(
                        count_chunk(&t.out, b"tEXt"),
                        2,
                        "[{}] {label}: tEXt count",
                        lib.tag
                    );
                    let pal_chunks = ct == PNG_COLOR_TYPE_PALETTE;
                    for nm in [b"PLTE", b"tRNS", b"hIST"] {
                        assert_eq!(
                            count_chunk(&t.out, nm),
                            pal_chunks as usize,
                            "[{}] {label}: {} count",
                            lib.tag,
                            String::from_utf8_lossy(nm)
                        );
                    }
                    if variant == 3 {
                        assert_eq!(
                            count_chunk(&t.out, &extra_name),
                            1,
                            "[{}] {label}: app chunk missing",
                            lib.tag
                        );
                    }
                    t
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W17 — the write transforms, each alone and in random combinations
// ---------------------------------------------------------------------------
//
// All of these must be applied *after* png_write_info: png_write_IHDR is what
// sets png_ptr->bit_depth / color_type / usr_bit_depth / usr_channels, and the
// setters validate against (and modify) those fields.  This is exactly the
// order png_write_png uses.
//
// Only png_set_filler and png_set_shift can fail:
//   * png_set_filler on write accepts RGB (usr_channels = 4) and GRAY with
//     bit depth >= 8 (usr_channels = 2); anything else is a fatal app error
//     (PNG_FLAG_APP_ERRORS_WARN is not set for write structs) -> Phase C.
//   * png_set_shift needs 1 <= true_bits[channel] <= bit_depth.

const T_BGR: u32 = 1 << 0;
const T_SWAP: u32 = 1 << 1;
const T_PACKSWAP: u32 = 1 << 2;
const T_INVERT_MONO: u32 = 1 << 3;
const T_INVERT_ALPHA: u32 = 1 << 4;
const T_SWAP_ALPHA: u32 = 1 << 5;
const T_FILLER_BEFORE: u32 = 1 << 6;
const T_FILLER_AFTER: u32 = 1 << 7;
const T_SHIFT: u32 = 1 << 8;
const T_PACKING: u32 = 1 << 9;

const W17_NAMES: &[(&str, u32)] = &[
    ("bgr", T_BGR),
    ("swap", T_SWAP),
    ("packswap", T_PACKSWAP),
    ("invert_mono", T_INVERT_MONO),
    ("invert_alpha", T_INVERT_ALPHA),
    ("swap_alpha", T_SWAP_ALPHA),
    ("filler_before", T_FILLER_BEFORE),
    ("filler_after", T_FILLER_AFTER),
    ("shift", T_SHIFT),
    ("packing", T_PACKING),
];

/// `png_set_filler` is only accepted for these (colour type, bit depth) pairs.
fn filler_ok(ct: c_int, bd: c_int) -> bool {
    (ct == PNG_COLOR_TYPE_RGB && (bd == 8 || bd == 16))
        || (ct == PNG_COLOR_TYPE_GRAY && (bd == 8 || bd == 16))
}

fn w17_legal(bit: u32, ct: c_int, bd: c_int) -> bool {
    match bit {
        T_FILLER_BEFORE | T_FILLER_AFTER => filler_ok(ct, bd),
        // Everything else is accepted for every colour type / bit depth; the
        // corresponding png_do_* is simply a no-op where it does not apply.
        _ => true,
    }
}

/// The user-supplied pixel depth once `mask` has been applied.
fn w17_usr_pixel_depth(mask: u32, ct: c_int, bd: c_int) -> u32 {
    let usr_bd = if (mask & T_PACKING) != 0 && bd < 8 {
        8
    } else {
        bd as u32
    };
    let usr_ch = if (mask & (T_FILLER_BEFORE | T_FILLER_AFTER)) != 0 {
        if ct == PNG_COLOR_TYPE_RGB {
            4
        } else {
            2
        }
    } else {
        channels(ct)
    };
    usr_bd * usr_ch
}

#[allow(clippy::too_many_arguments)]
fn w17_run(label: &str, mask: u32, ct: c_int, bd: c_int, w: u32, h: u32, il: c_int, rng: &mut Rng) {
    let npal = 1u32 << bd;
    let pal = full_palette(rng);
    let upd = w17_usr_pixel_depth(mask, ct, bd);
    let n = rowbytes_pd(upd, w);
    let rows: Vec<Vec<u8>> = (0..h)
        .map(|_| {
            let mut row = vec![0u8; n + 8];
            if ct == PNG_COLOR_TYPE_PALETTE {
                if (mask & T_PACKING) != 0 && bd < 8 {
                    // one index per byte
                    for x in 0..w as usize {
                        row[x] = rng.below(npal) as u8;
                    }
                } else {
                    for x in 0..w as usize {
                        set_index(&mut row, x, bd, rng.below(npal) as u8);
                    }
                }
            } else {
                for i in 0..n {
                    row[i] = rng.byte();
                }
            }
            row
        })
        .collect();
    // 1 <= shift <= bit_depth for every channel that is checked.
    let s1 = std::cmp::max(1, bd - 2) as u8;
    let shift = PngColor8 {
        red: s1,
        green: s1,
        blue: s1,
        gray: s1,
        alpha: s1,
    };
    // png_write_sBIT validates against usr_bit_depth, which at png_write_info
    // time is still the image bit depth.
    let maxb = if ct == PNG_COLOR_TYPE_PALETTE { 8 } else { bd };
    let sb = std::cmp::max(1, maxb - 1) as u8;
    let sbit = PngColor8 {
        red: sb,
        green: sb,
        blue: sb,
        gray: sb,
        alpha: sb,
    };
    let filler = 0x7fu32;
    diff(label, |lib| {
        wwrite(lib, &mut |c, png, info| unsafe {
            (c.set_IHDR)(png, info, w, h, bd, ct, il, 0, 0);
            maybe_plte(c, png, info, ct, &pal, npal);
            if (mask & T_SHIFT) != 0 {
                (c.set_sBIT)(png, info, &sbit as *const PngColor8 as *const u8);
            }
            log_hdr(c, png, info);
            (c.write_info)(png, info);

            // Same order as png_write_png applies them.
            if (mask & T_INVERT_MONO) != 0 {
                (c.set_invert_mono)(png);
            }
            if (mask & T_SHIFT) != 0 {
                (c.set_shift)(png, &shift as *const PngColor8 as *const u8);
            }
            if (mask & T_PACKING) != 0 {
                (c.set_packing)(png);
            }
            if (mask & T_SWAP_ALPHA) != 0 {
                (c.set_swap_alpha)(png);
            }
            if (mask & T_FILLER_AFTER) != 0 {
                (c.set_filler)(png, filler, PNG_FILLER_AFTER);
            } else if (mask & T_FILLER_BEFORE) != 0 {
                (c.set_filler)(png, filler, PNG_FILLER_BEFORE);
            }
            if (mask & T_BGR) != 0 {
                (c.set_bgr)(png);
            }
            if (mask & T_SWAP) != 0 {
                (c.set_swap)(png);
            }
            if (mask & T_PACKSWAP) != 0 {
                (c.set_packswap)(png);
            }
            if (mask & T_INVERT_ALPHA) != 0 {
                (c.set_invert_alpha)(png);
            }

            let passes = if il == PNG_INTERLACE_ADAM7 {
                let p = (c.set_interlace_handling)(png);
                log(format!("passes={p}"));
                p
            } else {
                1
            };
            for _pass in 0..passes {
                for r in &rows {
                    (c.write_row)(png, r.as_ptr());
                }
            }
            (c.write_end)(png, info);
            log(format!("rowsum={:08x}", rowsum(&rows)));
        })
    });
}

#[test]
fn w17_write_transforms() {
    let mut rng = Rng::new(0x2717);

    // (a) each transform alone, on every colour type / bit depth the C accepts.
    for &(name, bit) in W17_NAMES {
        for &(ct, bd) in ALL_COMBOS {
            if !w17_legal(bit, ct, bd) {
                continue;
            }
            for &(w, h) in &[(7u32, 5u32), (17, 3)] {
                let label = format!("W17 {name} ct={ct} bd={bd} w={w} h={h}");
                w17_run(&label, bit, ct, bd, w, h, PNG_INTERLACE_NONE, &mut rng);
            }
        }
    }

    // (b) eight seeded random combinations of the compatible transforms per
    //     colour type / bit depth, interlace 0 and 1.
    for &(ct, bd) in ALL_COMBOS {
        let legal: Vec<u32> = W17_NAMES
            .iter()
            .map(|(_, b)| *b)
            .filter(|b| w17_legal(*b, ct, bd))
            .collect();
        for k in 0..8 {
            let mut mask = 0u32;
            for &b in &legal {
                if rng.next_u32() & 1 != 0 {
                    mask |= b;
                }
            }
            // BEFORE and AFTER together is meaningless (the last wins in the C
            // and only one flag exists); keep just AFTER.
            if mask & T_FILLER_AFTER != 0 {
                mask &= !T_FILLER_BEFORE;
            }
            let il = if rng.next_u32() & 1 != 0 {
                PNG_INTERLACE_ADAM7
            } else {
                PNG_INTERLACE_NONE
            };
            let label = format!("W17 rand{k} ct={ct} bd={bd} il={il} mask={mask:#05x}");
            w17_run(&label, mask, ct, bd, 9, 6, il, &mut rng);
        }
    }
}

// ---------------------------------------------------------------------------
// W18 — png_set_write_user_transform_fn + png_set_user_transform_info
// ---------------------------------------------------------------------------

/// Write user transform: logs the row_info fields and the row bytes, then
/// deterministically rewrites the row (XOR each byte with its index).  It must
/// not change `row_info`, otherwise png_write_row's pixel-depth consistency
/// check fires.
unsafe extern "C" fn cb_user_transform(_png: Png, row_info: *mut PngRowInfo, data: *mut u8) {
    if row_info.is_null() || data.is_null() {
        log("UT(null)".to_string());
        return;
    }
    let ri = *row_info;
    log(format!(
        "UT w={} rb={} ct={} bd={} ch={} pd={}",
        ri.width, ri.rowbytes, ri.color_type, ri.bit_depth, ri.channels, ri.pixel_depth
    ));
    let s = std::slice::from_raw_parts(data, ri.rowbytes);
    log(format!("UT in={}", hex(s)));
    let d = std::slice::from_raw_parts_mut(data, ri.rowbytes);
    for (i, b) in d.iter_mut().enumerate() {
        *b ^= (i & 0xff) as u8;
    }
    log(format!("UT out={}", hex(d)));
}

#[test]
fn w18_write_user_transform() {
    let mut rng = Rng::new(0x2818);
    // The user-transform pointer target; its *contents* are logged (never the
    // address), so the round trip through png_set/get_user_transform_info is
    // verified.
    let tag: [u8; 8] = *b"UTPTR!\0\0";
    let combos: &[(c_int, c_int)] = &[
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 4),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_GRAY, 16),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB, 16),
        (PNG_COLOR_TYPE_PALETTE, 2),
        (PNG_COLOR_TYPE_PALETTE, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ];
    for &(ct, bd) in combos {
        let npal = 1u32 << bd;
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            // info variant 0: no png_set_user_transform_info at all
            //              1: matching depth/channels
            //              2: deliberately different (legal: on the write side
            //                 user_transform_depth/channels are recorded but
            //                 never used)
            //              3: NULL pointer, zero depth/channels
            for iv in 0..4 {
                let w = 7u32;
                let h = 4u32;
                let pal = full_palette(&mut rng);
                let rows = make_rows(&mut rng, ct, bd, w, h, npal);
                let (depth, chans) = match iv {
                    1 => (bd, channels(ct) as c_int),
                    2 => (16, 4),
                    _ => (0, 0),
                };
                let label = format!("W18 ct={ct} bd={bd} il={il} iv={iv}");
                diff(&label, |lib| {
                    let t = wwrite(lib, &mut |c, png, info| unsafe {
                        (c.set_IHDR)(png, info, w, h, bd, ct, il, 0, 0);
                        maybe_plte(c, png, info, ct, &pal, npal);
                        log_hdr(c, png, info);
                        (c.write_info)(png, info);
                        (c.set_write_user_transform_fn)(png, cb_user_transform as Cb);
                        if iv != 0 {
                            let p = if iv == 3 {
                                std::ptr::null_mut()
                            } else {
                                tag.as_ptr() as *mut c_void
                            };
                            (c.set_user_transform_info)(png, p, depth, chans);
                        }
                        let got = (c.get_user_transform_ptr)(png);
                        log(format!("utptr null={}", got.is_null() as u8));
                        if !got.is_null() {
                            log(format!(
                                "utptr data={}",
                                hex(std::slice::from_raw_parts(got as *const u8, 8))
                            ));
                        }
                        let passes = if il == PNG_INTERLACE_ADAM7 {
                            let p = (c.set_interlace_handling)(png);
                            log(format!("passes={p}"));
                            p
                        } else {
                            1
                        };
                        for _pass in 0..passes {
                            for r in &rows {
                                (c.write_row)(png, r.as_ptr());
                            }
                        }
                        (c.write_end)(png, info);
                        log(format!("rowsum={:08x}", rowsum(&rows)));
                    });
                    // The transform callback must have run for every row that
                    // actually carries data.
                    let nut = count_prefix(&t, "UT w=");
                    let want = if il == PNG_INTERLACE_ADAM7 {
                        (0..7).map(|p| pngbuild::pass_height(h, p) as usize).sum()
                    } else {
                        h as usize
                    };
                    assert_eq!(nut, want, "[{}] {label}: UT calls={nut}", lib.tag);
                    assert_eq!(
                        count_prefix(&t, "UT(null)"),
                        0,
                        "[{}] {label}: UT got NULL",
                        lib.tag
                    );
                    t
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W19 — png_set_write_status_fn
// ---------------------------------------------------------------------------

unsafe extern "C" fn cb_write_status(_png: Png, row: u32, pass: c_int) {
    log(format!("STATUS row={row} pass={pass}"));
}

#[test]
fn w19_write_status_fn() {
    let mut rng = Rng::new(0x2919);
    let combos: &[(c_int, c_int)] = &[
        (PNG_COLOR_TYPE_GRAY, 1),
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_PALETTE, 4),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ];
    for &(ct, bd) in combos {
        let npal = 1u32 << bd;
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for &(w, h) in &[(9u32, 6u32), (3, 2)] {
                // api 0 = write_row, 1 = write_rows, 2 = write_image,
                // 3 = write_png
                for api in 0..4 {
                    let pal = full_palette(&mut rng);
                    let mut rows = make_rows(&mut rng, ct, bd, w, h, npal);
                    let mut ptrs = ptr_vec(&mut rows);
                    let label = format!("W19 ct={ct} bd={bd} il={il} w={w} h={h} api={api}");
                    diff(&label, |lib| {
                        let t = wwrite(lib, &mut |c, png, info| unsafe {
                            (c.set_IHDR)(png, info, w, h, bd, ct, il, 0, 0);
                            maybe_plte(c, png, info, ct, &pal, npal);
                            (c.set_write_status_fn)(png, cb_write_status as Cb);
                            log_hdr(c, png, info);
                            if api == 3 {
                                (c.set_rows)(png, info, ptrs.as_mut_ptr());
                                (c.write_png)(
                                    png,
                                    info,
                                    PNG_TRANSFORM_IDENTITY,
                                    std::ptr::null_mut(),
                                );
                                log(format!("rowsum={:08x}", rowsum(&rows)));
                                return;
                            }
                            (c.write_info)(png, info);
                            match api {
                                0 => {
                                    let passes = if il == PNG_INTERLACE_ADAM7 {
                                        let p = (c.set_interlace_handling)(png);
                                        log(format!("passes={p}"));
                                        p
                                    } else {
                                        1
                                    };
                                    for _pass in 0..passes {
                                        for r in &rows {
                                            (c.write_row)(png, r.as_ptr());
                                        }
                                    }
                                }
                                1 => {
                                    let passes = if il == PNG_INTERLACE_ADAM7 {
                                        let p = (c.set_interlace_handling)(png);
                                        log(format!("passes={p}"));
                                        p
                                    } else {
                                        1
                                    };
                                    let base = ptrs.as_mut_ptr();
                                    for _pass in 0..passes {
                                        (c.write_rows)(png, base, h);
                                    }
                                }
                                _ => {
                                    (c.write_image)(png, ptrs.as_mut_ptr());
                                }
                            }
                            (c.write_end)(png, info);
                            log(format!("rowsum={:08x}", rowsum(&rows)));
                        });
                        // The status callback must fire exactly once per row
                        // that actually carries data.
                        let nst = count_prefix(&t, "STATUS ");
                        let want: usize = if il == PNG_INTERLACE_ADAM7 {
                            (0..7)
                                .map(|p| {
                                    if pngbuild::pass_width(w, p) == 0 {
                                        0
                                    } else {
                                        pngbuild::pass_height(h, p) as usize
                                    }
                                })
                                .sum()
                        } else {
                            h as usize
                        };
                        assert_eq!(nst, want, "[{}] {label}: STATUS={nst}", lib.tag);
                        t
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W20 — png_set_text on the write side + png_set_text_compression_*
// ---------------------------------------------------------------------------

/// The text entries, built once so both libraries see identical `png_text`
/// records.  The `CString`s must outlive the `diff` closure.
struct TextBank {
    _keys: Vec<CString>,
    _bodies: Vec<CString>,
    _langs: Vec<CString>,
    texts: Vec<PngText>,
}

fn text_bank() -> TextBank {
    // >= 500 bytes and highly repetitive so zlib really compresses.
    let long1 = "libpng differential harness: repetitive filler text. ".repeat(12);
    let long2 = "AAAAABBBBBCCCCCDDDDD0123456789".repeat(20);
    let keys: Vec<CString> = [
        "Title",
        "Author",
        "Description",
        "Copyright",
        "Comment",
        "Software",
        "Warning",
    ]
    .iter()
    .map(|s| CString::new(*s).unwrap())
    .collect();
    let bodies: Vec<CString> = [
        "short uncompressed tEXt value".to_string(),
        long1.clone(),
        "iTXt, not compressed".to_string(),
        long2.clone(),
        long1,
        "".to_string(),
        long2,
    ]
    .iter()
    .map(|s| CString::new(s.as_str()).unwrap())
    .collect();
    let langs: Vec<CString> = ["en", "de-DE", "en-GB", "fr"]
        .iter()
        .map(|s| CString::new(*s).unwrap())
        .collect();
    let lk: Vec<CString> = ["Titel", "Autor", "Beschreibung"]
        .iter()
        .map(|s| CString::new(*s).unwrap())
        .collect();

    let k = |i: usize| keys[i].as_ptr() as *mut c_char;
    let b = |i: usize| bodies[i].as_ptr() as *mut c_char;

    let texts = vec![
        // tEXt, uncompressed
        PngText {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: k(0),
            text: b(0),
            ..Default::default()
        },
        // zTXt, long repetitive text
        PngText {
            compression: PNG_TEXT_COMPRESSION_zTXt,
            key: k(1),
            text: b(1),
            ..Default::default()
        },
        // iTXt, uncompressed, with lang + lang_key
        PngText {
            compression: PNG_ITXT_COMPRESSION_NONE,
            key: k(2),
            text: b(2),
            lang: langs[0].as_ptr() as *mut c_char,
            lang_key: lk[0].as_ptr() as *mut c_char,
            ..Default::default()
        },
        // iTXt, compressed, with lang + lang_key
        PngText {
            compression: PNG_ITXT_COMPRESSION_zTXt,
            key: k(3),
            text: b(3),
            lang: langs[1].as_ptr() as *mut c_char,
            lang_key: lk[1].as_ptr() as *mut c_char,
            ..Default::default()
        },
        // iTXt, compressed, without lang / lang_key (NULL -> "")
        PngText {
            compression: PNG_ITXT_COMPRESSION_zTXt,
            key: k(4),
            text: b(4),
            ..Default::default()
        },
        // zTXt with an empty body (png_set_text downgrades it to NONE)
        PngText {
            compression: PNG_TEXT_COMPRESSION_zTXt,
            key: k(5),
            text: b(5),
            ..Default::default()
        },
        // iTXt, uncompressed, lang given but lang_key NULL
        PngText {
            compression: PNG_ITXT_COMPRESSION_NONE,
            key: k(6),
            text: b(6),
            lang: langs[2].as_ptr() as *mut c_char,
            ..Default::default()
        },
    ];
    TextBank {
        _keys: keys,
        _bodies: bodies,
        _langs: {
            let mut v = langs;
            v.extend(lk);
            v
        },
        texts,
    }
}

#[test]
fn w20_text_compression() {
    let mut rng = Rng::new(0x3020);
    let bank = text_bank();
    let w = 8u32;
    let h = 4u32;
    let rows = make_rows(&mut rng, PNG_COLOR_TYPE_RGB, 8, w, h, 1);
    let ntext = bank.texts.len() as c_int;

    // (a) level x strategy
    let mut cases: Vec<(c_int, c_int, c_int, c_int)> = Vec::new();
    for &level in &[0, 1, 6, 9] {
        for strategy in 0..=4 {
            cases.push((level, strategy, 15, 8));
        }
    }
    // (b) window bits x mem level
    for &wb in &[9, 12, 15] {
        for &ml in &[1, 8] {
            cases.push((6, 0, wb, ml));
        }
    }
    for (level, strategy, wb, ml) in cases {
        let label = format!("W20 level={level} strategy={strategy} wbits={wb} memlevel={ml}");
        let texts = &bank.texts;
        diff(&label, |lib| {
            let t = wwrite_c(lib, &mut |c, png, info| unsafe {
                (c.set_IHDR)(png, info, w, h, 8, PNG_COLOR_TYPE_RGB, PNG_INTERLACE_NONE, 0, 0);
                (c.set_text_compression_level)(png, level);
                (c.set_text_compression_strategy)(png, strategy);
                (c.set_text_compression_window_bits)(png, wb);
                (c.set_text_compression_mem_level)(png, ml);
                (c.set_text)(png, info, texts.as_ptr() as *const c_void, ntext);
                log_hdr(c, png, info);
                (c.write_info)(png, info);
                // png_write_info rewrites the compression field of every entry
                // it wrote (to the *_WR values); make that visible.
                log_all_info(c, png, info);
                for r in &rows {
                    (c.write_row)(png, r.as_ptr());
                }
                (c.write_end)(png, info);
                log(format!("rowsum={:08x}", rowsum(&rows)));
            });
            // All three text chunk types must really have been emitted.
            assert_eq!(
                count_chunk(&t.out, b"tEXt"),
                2,
                "[{}] {label}: tEXt count",
                lib.tag
            );
            assert_eq!(
                count_chunk(&t.out, b"zTXt"),
                1,
                "[{}] {label}: zTXt count",
                lib.tag
            );
            assert_eq!(
                count_chunk(&t.out, b"iTXt"),
                4,
                "[{}] {label}: iTXt count",
                lib.tag
            );
            t
        });
    }
}

// ---------------------------------------------------------------------------
// W21 — png_write_end with a separate end-info struct, and with NULL
// ---------------------------------------------------------------------------

#[test]
fn w21_write_end_info() {
    let mut rng = Rng::new(0x3121);
    let key_a = CString::new("Comment").unwrap();
    let key_b = CString::new("Disclaimer").unwrap();
    let body_a = CString::new("trailer text after IDAT".to_string()).unwrap();
    let long = "trailing compressed comment, repeated many times. ".repeat(12);
    let body_b = CString::new(long).unwrap();
    let w = 8u32;
    let h = 4u32;

    for &hdr_time in &[false, true] {
        for end_mode in 0..3 {
            // 0 = png_write_end(png, NULL)
            // 1 = end_info with tIME only
            // 2 = end_info with tIME + tEXt + zTXt
            for rep in 0..2 {
                let rows = make_rows(&mut rng, PNG_COLOR_TYPE_RGB, 8, w, h, 1);
                let head_time = PngTime {
                    year: 1998,
                    month: 4,
                    day: 15,
                    hour: 6,
                    minute: 7,
                    second: 8,
                };
                let end_time = PngTime {
                    year: 2024,
                    month: 12,
                    day: 31,
                    hour: 23,
                    minute: 59,
                    second: 58,
                };
                let etexts = [
                    PngText {
                        compression: PNG_TEXT_COMPRESSION_NONE,
                        key: key_a.as_ptr() as *mut c_char,
                        text: body_a.as_ptr() as *mut c_char,
                        ..Default::default()
                    },
                    PngText {
                        compression: PNG_TEXT_COMPRESSION_zTXt,
                        key: key_b.as_ptr() as *mut c_char,
                        text: body_b.as_ptr() as *mut c_char,
                        ..Default::default()
                    },
                ];
                let label = format!("W21 hdr_time={hdr_time} end_mode={end_mode} rep={rep}");
                diff(&label, |lib| {
                    let t = wwrite_c(lib, &mut |c, png, info| unsafe {
                        (c.set_IHDR)(
                            png,
                            info,
                            w,
                            h,
                            8,
                            PNG_COLOR_TYPE_RGB,
                            PNG_INTERLACE_NONE,
                            0,
                            0,
                        );
                        if hdr_time {
                            (c.set_tIME)(png, info, &head_time as *const PngTime as *const u8);
                        }
                        log_hdr(c, png, info);
                        (c.write_info)(png, info);
                        for r in &rows {
                            (c.write_row)(png, r.as_ptr());
                        }
                        log(format!("idat out={}", outlen()));
                        if end_mode == 0 {
                            (c.write_end)(png, std::ptr::null_mut());
                        } else {
                            let end_info = (c.create_info)(png);
                            log(format!(
                                "end_info={}",
                                if end_info.is_null() { 0 } else { 1 }
                            ));
                            if end_info.is_null() {
                                return;
                            }
                            (c.set_tIME)(
                                png,
                                end_info,
                                &end_time as *const PngTime as *const u8,
                            );
                            if end_mode == 2 {
                                (c.set_text)(
                                    png,
                                    end_info,
                                    etexts.as_ptr() as *const c_void,
                                    2,
                                );
                            }
                            (c.write_end)(png, end_info);
                            log_time_and_text(c, png, end_info);
                            let mut ei = end_info;
                            (c.destroy_info)(png, &mut ei);
                            log("end_info destroyed".to_string());
                        }
                        log(format!("rowsum={:08x}", rowsum(&rows)));
                    });
                    // tIME is written once: from the header info if it was set
                    // there (png_write_end then skips it), otherwise from the
                    // end info.
                    let want_time = (hdr_time || end_mode > 0) as usize;
                    assert_eq!(
                        count_chunk(&t.out, b"tIME"),
                        want_time,
                        "[{}] {label}: tIME count",
                        lib.tag
                    );
                    let want_text = (end_mode == 2) as usize;
                    assert_eq!(
                        count_chunk(&t.out, b"tEXt"),
                        want_text,
                        "[{}] {label}: tEXt count",
                        lib.tag
                    );
                    assert_eq!(
                        count_chunk(&t.out, b"zTXt"),
                        want_text,
                        "[{}] {label}: zTXt count",
                        lib.tag
                    );
                    // Any trailer chunk has to come after the last IDAT.
                    let cs = pngbuild::split(&t.out);
                    let last_idat = cs.iter().rposition(|c| &c.name == b"IDAT").unwrap();
                    for (i, c) in cs.iter().enumerate() {
                        if &c.name == b"tIME" || &c.name == b"tEXt" || &c.name == b"zTXt" {
                            let after = i > last_idat;
                            assert_eq!(
                                after,
                                !(hdr_time && &c.name == b"tIME"),
                                "[{}] {label}: {} placement",
                                lib.tag,
                                String::from_utf8_lossy(&c.name)
                            );
                        }
                    }
                    t
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W22 — 16-bit + png_set_swap + interlace + every filter
// ---------------------------------------------------------------------------

#[test]
fn w22_16bit_swap_filters() {
    let mut rng = Rng::new(0x3222);
    let filters: &[(&str, c_int)] = &[
        ("NONE", PNG_FILTER_NONE),
        ("SUB", PNG_FILTER_SUB),
        ("UP", PNG_FILTER_UP),
        ("AVG", PNG_FILTER_AVG),
        ("PAETH", PNG_FILTER_PAETH),
        ("ALL", PNG_ALL_FILTERS),
    ];
    let w = 13u32;
    let h = 12u32;
    for &ct in &[
        PNG_COLOR_TYPE_GRAY,
        PNG_COLOR_TYPE_RGB,
        PNG_COLOR_TYPE_GRAY_ALPHA,
        PNG_COLOR_TYPE_RGB_ALPHA,
    ] {
        for &(fname, f) in filters {
            for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                for &swap in &[false, true] {
                    let rows = rows16(&mut rng, ct, w, h);
                    let label = format!("W22 ct={ct} f={fname} il={il} swap={swap}");
                    diff(&label, |lib| {
                        wwrite(lib, &mut |c, png, info| unsafe {
                            (c.set_IHDR)(png, info, w, h, 16, ct, il, 0, 0);
                            (c.set_filter)(png, PNG_FILTER_TYPE_BASE, f);
                            log_hdr(c, png, info);
                            (c.write_info)(png, info);
                            if swap {
                                // must come after png_write_IHDR set bit_depth
                                (c.set_swap)(png);
                            }
                            let passes = if il == PNG_INTERLACE_ADAM7 {
                                let p = (c.set_interlace_handling)(png);
                                log(format!("passes={p}"));
                                p
                            } else {
                                1
                            };
                            for _pass in 0..passes {
                                for r in &rows {
                                    (c.write_row)(png, r.as_ptr());
                                }
                            }
                            (c.write_end)(png, info);
                            log(format!("rowsum={:08x}", rowsum(&rows)));
                        })
                    });
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// W23 — png_create_write_struct_2 with the user memory callbacks
// ---------------------------------------------------------------------------
//
// `trace_alloc` is only switched on *after* the png_struct, the jmp_buf and the
// png_info have been allocated: those three sizes are internal to each
// implementation.  Every allocation libpng makes afterwards (row_buf, try_row,
// tst_row, prev_row, the zlib buffers via png_zalloc, ...) must match in size
// and order.

#[test]
fn w23_create_write_struct_2() {
    let mut rng = Rng::new(0x3323);
    let err_tag: [u8; 8] = *b"ERRPTR\0\0";
    let mem_tag: [u8; 8] = *b"MEMPTR\0\0";
    for &(ct, bd) in &[
        (PNG_COLOR_TYPE_GRAY, 8),
        (PNG_COLOR_TYPE_RGB, 8),
        (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
        (PNG_COLOR_TYPE_RGB_ALPHA, 16),
    ] {
        for &il in &[PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let w = 6u32;
            let h = 5u32;
            let rows = make_rows(&mut rng, ct, bd, w, h, 1);
            let label = format!("W23 ct={ct} bd={bd} il={il}");
            diff(&label, |lib| {
                session_reset(Vec::new());
                let c = Core::new(lib);
                let rc = protected(|| unsafe {
                    let png = (c.create_write_2)(
                        VER_STRING.as_ptr() as *const c_char,
                        err_tag.as_ptr() as *mut c_void,
                        cb_error as Cb,
                        cb_warning as Cb,
                        mem_tag.as_ptr() as *mut c_void,
                        cb_malloc as Cb,
                        cb_free as Cb,
                    );
                    log(format!("create2={}", if png.is_null() { 0 } else { 1 }));
                    if png.is_null() {
                        return;
                    }
                    (c.set_longjmp)(png, shim().longjmp_ptr, shim().jmp_buf_size);
                    // Only the sizes from here on are implementation-independent.
                    with_session(|s| {
                        s.trace_alloc = true;
                        s.malloc_count = 0;
                    });
                    let info = (c.create_info)(png);
                    log(format!("create_info={}", if info.is_null() { 0 } else { 1 }));
                    let ep = (c.get_error_ptr)(png);
                    let mp = (c.get_mem_ptr)(png);
                    log(format!(
                        "error_ptr null={} mem_ptr null={}",
                        ep.is_null() as u8,
                        mp.is_null() as u8
                    ));
                    if !ep.is_null() {
                        log(format!(
                            "error_ptr data={}",
                            hex(std::slice::from_raw_parts(ep as *const u8, 8))
                        ));
                    }
                    if !mp.is_null() {
                        log(format!(
                            "mem_ptr data={}",
                            hex(std::slice::from_raw_parts(mp as *const u8, 8))
                        ));
                    }
                    (c.set_write_fn)(png, std::ptr::null_mut(), cb_write as Cb, cb_flush as Cb);
                    (c.set_IHDR)(png, info, w, h, bd, ct, il, 0, 0);
                    log(format!(
                        "rowbytes={} channels={}",
                        (c.get_rowbytes)(png, info),
                        (c.get_channels)(png, info)
                    ));
                    (c.write_info)(png, info);
                    let passes = if il == PNG_INTERLACE_ADAM7 {
                        let p = (c.set_interlace_handling)(png);
                        log(format!("passes={p}"));
                        p
                    } else {
                        1
                    };
                    for _pass in 0..passes {
                        for r in &rows {
                            (c.write_row)(png, r.as_ptr());
                        }
                    }
                    (c.write_end)(png, info);
                    let mut p = png;
                    let mut i = info;
                    (c.destroy_write)(&mut p, &mut i);
                    log("destroyed".to_string());
                    log(format!("live_allocs={}", with_session(|s| s.live_allocs)));
                });
                let t = Trace {
                    lines: take_log(),
                    out: take_out(),
                    rc,
                };
                // The user memory callbacks must really have been used.
                let mallocs = count_prefix(&t, "MALLOC(");
                let frees = count_prefix(&t, "FREE(");
                assert!(
                    mallocs >= 3,
                    "[{}] {label}: traced mallocs={mallocs}",
                    lib.tag
                );
                assert!(frees >= 3, "[{}] {label}: traced frees={frees}", lib.tag);
                assert_eq!(t.rc, 0, "[{}] {label}: rc={}", lib.tag, t.rc);
                t
            });
        }
    }
}

// ---------------------------------------------------------------------------
// W24 — png_set_check_for_invalid_index on the write side
// ---------------------------------------------------------------------------
//
// `png_set_check_for_invalid_index(png, allowed)` sets num_palette_max to 0
// when allowed > 0 and to -1 otherwise.  png_write_row runs
// png_do_check_palette_indexes whenever num_palette_max >= 0 (which is also the
// default state of a freshly created write struct), and png_write_end reports
//   "Wrote palette index exceeding num_palette"
// through png_benign_error.  PNG_BENIGN_WRITE_ERRORS_SUPPORTED is *not*
// configured, so on the write side that is a fatal png_error: the write driver
// longjmps out and png_destroy_write_struct is never reached.  Both libraries
// get the same input, so the whole trace (including rc and the truncated
// datastream) must still match.
//
// Note png_do_check_palette_indexes only runs at all when
// num_palette < (1 << bit_depth), hence npal is always short below.

#[test]
fn w24_check_for_invalid_index() {
    let mut rng = Rng::new(0x3424);
    for &bd in &[1, 2, 4, 8] {
        let full = 1u32 << bd;
        // A short palette so png_do_check_palette_indexes actually runs.
        let npal = std::cmp::max(1, full / 2);
        for &allowed in &[-1i32, 0, 1] {
            // valid = 0: every index < num_palette
            // valid = 1: some indices >= num_palette
            // valid = 2: the very first row already exceeds it
            for valid in 0..3 {
                for &w in &[5u32, 16] {
                    for rep in 0..2 {
                        let h = 4u32;
                        let pal = full_palette(&mut rng);
                        let n = rb(PNG_COLOR_TYPE_PALETTE, bd, w);
                        let rows: Vec<Vec<u8>> = (0..h)
                            .map(|y| {
                                let mut row = vec![0u8; n + 8];
                                let over = match valid {
                                    0 => false,
                                    1 => y == h - 1,
                                    _ => true,
                                };
                                for x in 0..w as usize {
                                    let idx = if over && x % 3 == 0 {
                                        npal as u8 + rng.below(full - npal) as u8
                                    } else {
                                        rng.below(npal) as u8
                                    };
                                    set_index(&mut row, x, bd, idx);
                                }
                                row
                            })
                            .collect();
                        let label = format!(
                            "W24 bd={bd} npal={npal} allowed={allowed} valid={valid} w={w} rep={rep}"
                        );
                        diff(&label, |lib| {
                            // A fatal png_error is expected for some of these
                            // configurations, so the rc==0 guard is not used.
                            let t = write_c(lib, &mut |c, png, info| unsafe {
                                (c.set_IHDR)(
                                    png,
                                    info,
                                    w,
                                    h,
                                    bd,
                                    PNG_COLOR_TYPE_PALETTE,
                                    PNG_INTERLACE_NONE,
                                    0,
                                    0,
                                );
                                (c.set_PLTE)(png, info, pal.as_ptr(), npal as c_int);
                                (c.set_check_for_invalid_index)(png, allowed);
                                log(format!(
                                    "palette_max.init={}",
                                    (c.get_palette_max)(png, info)
                                ));
                                log_hdr(c, png, info);
                                (c.write_info)(png, info);
                                for (i, r) in rows.iter().enumerate() {
                                    (c.write_row)(png, r.as_ptr());
                                    log(format!(
                                        "row {i} palette_max={} out={}",
                                        (c.get_palette_max)(png, info),
                                        outlen()
                                    ));
                                }
                                (c.write_end)(png, info);
                                log(format!(
                                    "palette_max.end={}",
                                    (c.get_palette_max)(png, info)
                                ));
                                log(format!("rowsum={:08x}", rowsum(&rows)));
                            });
                            // Checking is only enabled for allowed > 0; when it
                            // is enabled and a row used an out-of-range index,
                            // png_write_end must raise the benign-error-turned-
                            // fatal report.
                            let errs = count_prefix(&t, "ERROR(");
                            let want = (allowed > 0 && valid != 0) as usize;
                            assert_eq!(
                                errs,
                                want,
                                "[{}] {label}: errors={errs} rc={}",
                                lib.tag,
                                t.rc
                            );
                            assert_eq!(
                                (t.rc != 0) as usize,
                                want,
                                "[{}] {label}: rc={}",
                                lib.tag,
                                t.rc
                            );
                            t
                        });
                    }
                }
            }
        }
    }
}
