//! Phase C error-path cases. Each case hand-builds a MALFORMED PNG byte stream,
//! feeds it to the read API, and records how libpng reports the defect. The
//! harness error callback prints `ERROR <message>` and exits 90; the warning
//! callback prints `WARN  <message>` and continues. The driver compares the
//! whole transcript AND the exit status between the C and Rust `.so`, so "both
//! rejected it the same way with the same message" is the literal assertion.
//!
//! STRUCTURAL RULE: a `png_error` never returns — the worker process ends. A
//! case that must probe several independent fatal conditions runs one condition
//! per process via `support::variant()` / `pick()`. See each case's comment for
//! its required `variants` count (mirrored in cases.rs).

use crate::api::Api;
use crate::p;
use crate::support::*;
use crate::types::*;
use std::os::raw::c_char;

pub fn run(api: &Api, case: &str, seed: u64) -> bool {
    let mut rng = Rng::new(seed);
    match case {
        "err/sig_bad" => sig_bad(api, &mut rng),
        "err/chunk_header" => chunk_header(api, &mut rng),
        "err/missing_ihdr" => missing_ihdr(api, &mut rng),
        "err/out_of_place" => out_of_place(api, &mut rng),
        "err/duplicate" => duplicate(api, &mut rng),
        "err/len_too_short" => len_too_short(api, &mut rng),
        "err/len_too_long" => len_too_long(api, &mut rng),
        "err/missing_plte" => missing_plte(api, &mut rng),
        "err/too_many_idat" => too_many_idat(api, &mut rng),
        "err/critical_unknown" => critical_unknown(api, &mut rng),
        "err/ihdr_length" => ihdr_length(api, &mut rng),
        "err/ihdr_fields" => ihdr_fields(api, &mut rng),
        "err/plte" => plte(api, &mut rng),
        "err/iend" => iend(api, &mut rng),
        "err/gama" => gama(api, &mut rng),
        "err/sbit" => sbit(api, &mut rng),
        "err/chrm" => chrm(api, &mut rng),
        "err/srgb" => srgb(api, &mut rng),
        "err/iccp_frame" => iccp_frame(api, &mut rng),
        "err/iccp_profile" => iccp_profile(api, &mut rng),
        "err/splt" => splt(api, &mut rng),
        "err/trns" => trns(api, &mut rng),
        "err/bkgd" => bkgd(api, &mut rng),
        "err/exif" => exif(api, &mut rng),
        "err/hist" => hist(api, &mut rng),
        "err/pcal" => pcal(api, &mut rng),
        "err/scal" => scal(api, &mut rng),
        "err/ztxt" => ztxt(api, &mut rng),
        "err/itxt" => itxt(api, &mut rng),
        "err/zstream_reuse" => zstream_reuse(api, &mut rng),
        "err/row_before_idat" => row_before_idat(api, &mut rng),
        "err/bad_filter_value" => bad_filter_value(api, &mut rng),
        "err/not_enough_data" => not_enough_data(api, &mut rng),
        "err/too_much_data" => too_much_data(api, &mut rng),
        "err/idat_corrupt" => idat_corrupt(api, &mut rng),
        "err/row_too_big" => row_too_big(api, &mut rng),
        "err/duplicate_update" => duplicate_update(api, &mut rng),
        "err/read_png_limits" => read_png_limits(api, &mut rng),
        "err/int32_overflow" => int32_overflow(api, &mut rng),
        "err/row_null" => row_null(api, &mut rng),
        "err/truncated_stream" => truncated_stream(api, &mut rng),
        "err/uint31_range" => uint31_range(api, &mut rng),
        "err/process_data_skip" => process_data_skip(api, &mut rng),
        "err/get_eXIf_deprecated" => get_exif_deprecated(api, &mut rng),
        _ => return false,
    }
    true
}

/* ================================================================== */
/* shared helpers                                                     */
/* ================================================================== */

/// Feed a hand-built stream to the sequential reader through `png_read_info`,
/// then (if it survives) read every row and end. Any `png_error` fires the
/// error callback and terminates the process before returning here.
unsafe fn read_stream(api: &Api, bytes: Vec<u8>) {
    let mut rd = Reader::new(api, bytes);
    (api.png_read_info)(rd.png, rd.info);
    // Surviving to here means no fatal error during header/ancillary parsing.
    let w = (api.png_get_image_width)(rd.png, rd.info);
    let h = (api.png_get_image_height)(rd.png, rd.info);
    p!("survived read_info {}x{}", w, h);
    (api.png_read_update_info)(rd.png, rd.info);
    let rb = (api.png_get_rowbytes)(rd.png, rd.info);
    for _ in 0..h {
        let mut row = vec![0u8; rb + 8];
        (api.png_read_row)(rd.png, row.as_mut_ptr(), null());
    }
    (api.png_read_end)(rd.png, rd.info);
    emit("survived read_end");
    rd.destroy();
}

/// Drive an already-constructed reader (so benign-error state can be set first)
/// through the full sequential read: info, update, rows, end.
unsafe fn drive(api: &Api, rd: &Reader) {
    (api.png_read_info)(rd.png, rd.info);
    let w = (api.png_get_image_width)(rd.png, rd.info);
    let h = (api.png_get_image_height)(rd.png, rd.info);
    p!("survived read_info {}x{}", w, h);
    (api.png_read_update_info)(rd.png, rd.info);
    let rb = (api.png_get_rowbytes)(rd.png, rd.info);
    for _ in 0..h {
        let mut row = vec![0u8; rb + 8];
        (api.png_read_row)(rd.png, row.as_mut_ptr(), null());
    }
    (api.png_read_end)(rd.png, rd.info);
    emit("survived read_end");
}

/// Feed a stream to the progressive reader in one shot. Info/row/end callbacks
/// are installed so the transcript records progress; any png_error terminates.
unsafe fn feed_progressive(api: &Api, data: &[u8]) {
    let png = (api.png_create_read_struct)(
        PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
        std::ptr::null_mut(),
        Some(err_fn),
        Some(warn_fn),
    );
    let info = (api.png_create_info_struct)(png);
    (api.png_set_progressive_read_fn)(
        png,
        std::ptr::null_mut(),
        Some(prog_info_fn),
        Some(prog_row_fn),
        Some(prog_end_fn),
    );
    (api.png_process_data)(png, info, data.as_ptr() as *mut u8, data.len());
    let mut pp = png;
    let mut ip = info;
    (api.png_destroy_read_struct)(&mut pp, &mut ip, null());
}

/// A minimal valid non-interlaced IHDR (8x1 RGB8) for building streams that
/// need a good header before the defect under test.
fn good_ihdr() -> Vec<u8> {
    chunk(b"IHDR", &ihdr_data(8, 1, 8, 2, 0, 0, 0))
}

/// Prepend the signature.
fn start() -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&PNG_SIG);
    v
}

/* ================================================================== */
/* 2. signature and chunk-stream framing                              */
/* ================================================================== */

/// C37/C38 (sequential) and C159/C160 (progressive) — a corrupt signature.
/// variants: 4  (2 defects × sequential/progressive).
fn sig_bad(api: &Api, rng: &mut Rng) {
    // Build an otherwise valid stream, then damage the signature.
    let mut data = build_png(rng, 8, 1, 8, 2, &[], &[]);
    let (which, prog) = {
        let v = variant() % 4;
        (v % 2, v / 2)
    };
    match which {
        // Differs within the first 4 bytes: "Not a PNG file".
        0 => data[1] ^= 0xff,
        // Matches "\x89PNG" but the trailing CR/LF/^Z bytes are mangled:
        // "PNG file corrupted by ASCII conversion".
        _ => data[4] ^= 0xff,
    }
    p!("case sig which={} prog={}", which, prog);
    unsafe {
        if prog == 0 {
            read_stream(api, data);
        } else {
            feed_progressive(api, &data);
            emit("survived progressive");
        }
    }
}

/// C39/C40 (sequential) and C158 (bad-length via uint31) — a corrupt chunk
/// header after a valid signature.
/// variants: 3.
fn chunk_header(api: &Api, rng: &mut Rng) {
    let defects: &[&str] = &["len_high_bit", "type_nonalpha", "len_over_uint31"];
    let d = pick(defects);
    p!("case {}", d);
    let mut v = start();
    match d {
        // Length high byte >= 0x80 -> "bad header (invalid length)".
        "len_high_bit" => {
            v.extend_from_slice(&0x8000_0000u32.to_be_bytes());
            v.extend_from_slice(b"IHDR");
            v.extend_from_slice(&[0u8; 13]);
            v.extend_from_slice(&0u32.to_be_bytes());
        }
        // Chunk type contains a non-alphabetic byte -> "bad header (invalid type)".
        "type_nonalpha" => {
            let mut bad = *b"IHDR";
            bad[2] = 0x01;
            v.extend_from_slice(&chunk_raw_len(&bad, 13, &ihdr_data(8, 1, 8, 2, 0, 0, 0)));
        }
        // Length just over PNG_UINT_31_MAX.
        _ => {
            v.extend_from_slice(&0x7fff_ffffu32.wrapping_add(1).to_be_bytes());
            v.extend_from_slice(b"IHDR");
            v.extend_from_slice(&[0u8; 13]);
            v.extend_from_slice(&0u32.to_be_bytes());
        }
    }
    let _ = rng;
    unsafe { read_stream(api, v); }
}

/// C43/C49 (sequential) and C162 (progressive) — first chunk / IDAT with no
/// IHDR.
/// variants: 3.
fn missing_ihdr(api: &Api, rng: &mut Rng) {
    let kind = variant() % 3;
    p!("case missing_ihdr kind={}", kind);
    let mut v = start();
    match kind {
        // Some ancillary chunk appears before IHDR (sequential): "missing IHDR".
        0 => {
            v.extend_from_slice(&chunk(b"gAMA", &45455u32.to_be_bytes()));
            v.extend_from_slice(&good_ihdr());
            v.extend_from_slice(&chunk(b"IEND", &[]));
            unsafe { read_stream(api, v); }
        }
        // IDAT with no IHDR at all (sequential): "Missing IHDR before IDAT".
        1 => {
            let raw = raw_rows(rng, 8, 1, 8, 2);
            v.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
            v.extend_from_slice(&chunk(b"IEND", &[]));
            unsafe { read_stream(api, v); }
        }
        // Same, but through the progressive reader (C162).
        _ => {
            let raw = raw_rows(rng, 8, 1, 8, 2);
            v.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
            v.extend_from_slice(&chunk(b"IEND", &[]));
            unsafe {
                feed_progressive(api, &v);
                emit("survived progressive");
            }
        }
    }
}

/// C44/C75/C77/C113/C116 — a chunk appearing out of its legal position. Several
/// of these are benign (ancillary) so also drive the benign on/off toggle.
/// variants: 8  (5 conditions; the benign ones repeat with benign errors on).
fn out_of_place(api: &Api, rng: &mut Rng) {
    // Conditions: (name, builder). Some are critical-fatal (PLTE after IDAT),
    // some benign. We probe them one per process.
    let conds = 5usize;
    let v = variant();
    let cond = (v as usize) % conds;
    let benign_on = (v as usize) >= conds;
    p!("case out_of_place cond={} benign_on={}", cond, benign_on);
    let raw = raw_rows(rng, 8, 1, 8, 2);
    let idat = chunk(b"IDAT", &zlib_store(&raw));
    let mut s = start();
    match cond {
        // C44: gAMA after IDAT (ancillary, benign "out of place").
        0 => {
            s.extend_from_slice(&good_ihdr());
            s.extend_from_slice(&idat);
            s.extend_from_slice(&chunk(b"gAMA", &45455u32.to_be_bytes()));
            s.extend_from_slice(&chunk(b"IEND", &[]));
        }
        // C75: PLTE after IDAT on a palette image (critical, fatal).
        1 => {
            s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 4, 3, 0, 0, 0)));
            let pal: Vec<u8> = (0..3 * 4).map(|_| rng.u8()).collect();
            let praw = raw_rows(rng, 8, 1, 4, 3);
            s.extend_from_slice(&chunk(b"PLTE", &pal));
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&praw)));
            s.extend_from_slice(&chunk(b"PLTE", &pal));
            s.extend_from_slice(&chunk(b"IEND", &[]));
        }
        // C77: non-palette image where tRNS already seen, then PLTE.
        2 => {
            s.extend_from_slice(&good_ihdr());
            s.extend_from_slice(&chunk(b"tRNS", &[0, 0, 0, 0, 0, 0]));
            let pal: Vec<u8> = (0..3 * 4).map(|_| rng.u8()).collect();
            s.extend_from_slice(&chunk(b"PLTE", &pal));
            s.extend_from_slice(&idat);
            s.extend_from_slice(&chunk(b"IEND", &[]));
        }
        // C113: palette image, tRNS before PLTE.
        3 => {
            s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 4, 3, 0, 0, 0)));
            s.extend_from_slice(&chunk(b"tRNS", &[0, 1, 2, 3]));
            let pal: Vec<u8> = (0..3 * 4).map(|_| rng.u8()).collect();
            let praw = raw_rows(rng, 8, 1, 4, 3);
            s.extend_from_slice(&chunk(b"PLTE", &pal));
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&praw)));
            s.extend_from_slice(&chunk(b"IEND", &[]));
        }
        // C116: palette image, bKGD before PLTE.
        _ => {
            s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 4, 3, 0, 0, 0)));
            s.extend_from_slice(&chunk(b"bKGD", &[1]));
            let pal: Vec<u8> = (0..3 * 4).map(|_| rng.u8()).collect();
            let praw = raw_rows(rng, 8, 1, 4, 3);
            s.extend_from_slice(&chunk(b"PLTE", &pal));
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&praw)));
            s.extend_from_slice(&chunk(b"IEND", &[]));
        }
    }
    unsafe {
        let mut rd = Reader::new(api, s);
        if benign_on {
            (api.png_set_benign_errors)(rd.png, 1);
        }
        drive(api, &rd);
        rd.destroy();
    }
}

/// C45/C74 — a non-repeatable chunk appears twice.
/// variants: 4  (2 conditions × benign on/off).
fn duplicate(api: &Api, rng: &mut Rng) {
    let conds = 2usize;
    let v = variant();
    let cond = (v as usize) % conds;
    let benign_on = (v as usize) >= conds;
    p!("case duplicate cond={} benign_on={}", cond, benign_on);
    let raw = raw_rows(rng, 8, 1, 8, 2);
    let idat = chunk(b"IDAT", &zlib_store(&raw));
    let mut s = start();
    s.extend_from_slice(&good_ihdr());
    match cond {
        // C45: two gAMA chunks.
        0 => {
            s.extend_from_slice(&chunk(b"gAMA", &45455u32.to_be_bytes()));
            s.extend_from_slice(&chunk(b"gAMA", &45455u32.to_be_bytes()));
        }
        // C74: two PLTE chunks (on a non-palette image these are benign).
        _ => {
            let pal: Vec<u8> = (0..3 * 4).map(|_| rng.u8()).collect();
            s.extend_from_slice(&chunk(b"PLTE", &pal));
            s.extend_from_slice(&chunk(b"PLTE", &pal));
        }
    }
    s.extend_from_slice(&idat);
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        let mut rd = Reader::new(api, s);
        if benign_on {
            (api.png_set_benign_errors)(rd.png, 1);
        }
        drive(api, &rd);
        rd.destroy();
    }
}

/// C46 — a chunk shorter than its min_length (gAMA length 3). Benign.
/// variants: 2  (benign on/off).
fn len_too_short(api: &Api, rng: &mut Rng) {
    let benign_on = variant() >= 1;
    p!("case len_too_short benign_on={}", benign_on);
    let mut s = start();
    s.extend_from_slice(&good_ihdr());
    s.extend_from_slice(&chunk(b"gAMA", &[0, 0, 0])); // 3 bytes, needs 4
    let raw = raw_rows(rng, 8, 1, 8, 2);
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        let mut rd = Reader::new(api, s);
        if benign_on {
            (api.png_set_benign_errors)(rd.png, 1);
        }
        drive(api, &rd);
        rd.destroy();
    }
}

/// C47 — a chunk longer than its max_length (gAMA length 5). Benign.
/// variants: 2  (benign on/off).
fn len_too_long(api: &Api, rng: &mut Rng) {
    let benign_on = variant() >= 1;
    p!("case len_too_long benign_on={}", benign_on);
    let mut s = start();
    s.extend_from_slice(&good_ihdr());
    s.extend_from_slice(&chunk(b"gAMA", &[0, 0, 0, 0, 0])); // 5 bytes, needs 4
    let raw = raw_rows(rng, 8, 1, 8, 2);
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        let mut rd = Reader::new(api, s);
        if benign_on {
            (api.png_set_benign_errors)(rd.png, 1);
        }
        drive(api, &rd);
        rd.destroy();
    }
}

/// C50 (sequential) and C163 (progressive) — palette image reaching IDAT with
/// no PLTE.
/// variants: 2.
fn missing_plte(api: &Api, rng: &mut Rng) {
    let prog = variant() % 2;
    p!("case missing_plte prog={}", prog);
    let mut s = start();
    s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 4, 3, 0, 0, 0)));
    let raw = raw_rows(rng, 8, 1, 4, 3);
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        if prog == 0 {
            read_stream(api, s);
        } else {
            feed_progressive(api, &s);
            emit("survived progressive");
        }
    }
}

/// C51/C57/C58/C164 — too many IDATs (benign), and C149 (extra IDAT bytes after
/// Z_STREAM_END, benign).
/// variants: 10  (5 conditions × benign on/off).
fn too_many_idat(api: &Api, rng: &mut Rng) {
    let conds = 5usize;
    let v = variant();
    let cond = (v as usize) % conds;
    let benign_on = (v as usize) >= conds;
    p!("case too_many_idat cond={} benign_on={}", cond, benign_on);
    let raw = raw_rows(rng, 8, 1, 8, 2);
    let z = zlib_store(&raw);
    let mut s = start();
    s.extend_from_slice(&good_ihdr());
    match cond {
        // C51: an IDAT run, then after AFTER_IDAT a fresh IDAT.
        0 => {
            s.extend_from_slice(&chunk(b"IDAT", &z));
            s.extend_from_slice(&chunk(b"IEND", &[]));
            // splice a second IDAT before IEND
            s = {
                let mut t = start();
                t.extend_from_slice(&good_ihdr());
                t.extend_from_slice(&chunk(b"IDAT", &z));
                t.extend_from_slice(&chunk(b"tEXt", b"k\0v"));
                t.extend_from_slice(&chunk(b"IDAT", &z));
                t.extend_from_slice(&chunk(b"IEND", &[]));
                t
            };
        }
        // C57: extra IDAT bytes after the zlib stream ended (".Too many IDATs").
        1 => {
            let mut z2 = z.clone();
            z2.extend_from_slice(&[0u8; 4]);
            s.extend_from_slice(&chunk(b"IDAT", &z2));
            s.extend_from_slice(&chunk(b"IEND", &[]));
        }
        // C58: non-IDAT chunk between IDATs, then more IDAT ("..Too many IDATs").
        2 => {
            let half = z.len() / 2;
            s.extend_from_slice(&chunk(b"IDAT", &z[..half]));
            s.extend_from_slice(&chunk(b"tEXt", b"k\0v"));
            s.extend_from_slice(&chunk(b"IDAT", &z[half..]));
            s.extend_from_slice(&chunk(b"IEND", &[]));
        }
        // C164: same as C51 but through the progressive reader.
        3 => {
            let mut t = start();
            t.extend_from_slice(&good_ihdr());
            t.extend_from_slice(&chunk(b"IDAT", &z));
            t.extend_from_slice(&chunk(b"tEXt", b"k\0v"));
            t.extend_from_slice(&chunk(b"IDAT", &z));
            t.extend_from_slice(&chunk(b"IEND", &[]));
            s = t;
        }
        // C149: leftover IDAT bytes after Z_STREAM_END ("Extra compressed data").
        _ => {
            let mut z2 = z.clone();
            z2.extend_from_slice(b"trailing-bytes");
            s.extend_from_slice(&chunk(b"IDAT", &z2));
            s.extend_from_slice(&chunk(b"IEND", &[]));
        }
    }
    unsafe {
        if cond == 3 {
            // progressive path
            feed_progressive(api, &s);
            emit("survived progressive");
        } else {
            let mut rd = Reader::new(api, s);
            if benign_on {
                (api.png_set_benign_errors)(rd.png, 1);
            }
            drive(api, &rd);
            rd.destroy();
        }
    }
}

/// C52 — a critical unknown chunk (uppercase first letter), unhandled.
/// variants: 1.
fn critical_unknown(api: &Api, rng: &mut Rng) {
    p!("case critical_unknown");
    let mut s = start();
    s.extend_from_slice(&good_ihdr());
    s.extend_from_slice(&chunk(b"cRIT", b"payload"));
    let raw = raw_rows(rng, 8, 1, 8, 2);
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe { read_stream(api, s); }
}

/* ================================================================== */
/* 3. IHDR validation                                                 */
/* ================================================================== */

/// C59 (sequential) and C161 (progressive) — IHDR chunk length != 13.
/// variants: 4  (short/long × sequential/progressive).
fn ihdr_length(api: &Api, rng: &mut Rng) {
    let v = variant() % 4;
    let long = v % 2 == 1;
    let prog = v / 2;
    p!("case ihdr_length long={} prog={}", long, prog);
    let data = if long {
        // 14-byte IHDR.
        let mut d = ihdr_data(8, 1, 8, 2, 0, 0, 0);
        d.push(0);
        chunk(b"IHDR", &d)
    } else {
        // 12-byte IHDR.
        let d = &ihdr_data(8, 1, 8, 2, 0, 0, 0)[..12];
        chunk(b"IHDR", d)
    };
    let mut s = start();
    s.extend_from_slice(&data);
    let raw = raw_rows(rng, 8, 1, 8, 2);
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        if prog == 0 {
            read_stream(api, s);
        } else {
            feed_progressive(api, &s);
            emit("survived progressive");
        }
    }
}

/// C60-C72 and C314 — one malformed IHDR field per process. Each fires a
/// warning then "Invalid IHDR data" (fatal), except C72 (MNG filter) which is
/// its own warning.
/// variants: 12.
fn ihdr_fields(api: &Api, rng: &mut Rng) {
    // (name, w, h, depth, color, comp, filt, il)
    let bad: &[(&str, u32, u32, u8, u8, u8, u8, u8)] = &[
        ("w=0", 0, 1, 8, 2, 0, 0, 0),                    // C60
        ("w>2^31", 0x8000_0000, 1, 8, 2, 0, 0, 0),       // C61
        ("h=0", 8, 0, 8, 2, 0, 0, 0),                    // C63
        ("h>2^31", 8, 0x8000_0000, 8, 2, 0, 0, 0),       // C64
        ("bad_depth", 8, 1, 3, 2, 0, 0, 0),              // C66
        ("bad_color", 8, 1, 8, 1, 0, 0, 0),              // C67
        ("pal16", 8, 1, 16, 3, 0, 0, 0),                 // C68
        ("bad_il", 8, 1, 8, 2, 0, 0, 2),                 // C69
        ("bad_comp", 8, 1, 8, 2, 1, 0, 0),               // C70
        ("bad_filt", 8, 1, 8, 2, 0, 1, 0),               // C71
        ("mng_filt64", 8, 1, 8, 2, 0, 64, 0),            // C72
        ("color5", 8, 1, 8, 5, 0, 0, 0),                 // C314 (color_type 5)
    ];
    let (name, w, h, d, c, comp, filt, il) = pick(bad);
    p!("case ihdr_fields {}", name);
    let mut s = start();
    s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(w, h, d, c, comp, filt, il)));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    let _ = rng;
    unsafe {
        let mut rd = Reader::new(api, s);
        (api.png_read_info)(rd.png, rd.info);
        emit("no error");
        rd.destroy();
    }
}

/* ================================================================== */
/* 4. per-chunk content validation                                    */
/* ================================================================== */

/// Build a stream: SIG + IHDR(color) + [PLTE if palette] + the supplied chunk
/// bytes (placed before IDAT) + valid IDAT + IEND. Then drive it with benign
/// errors either off or on depending on the variant split.
///
/// `conds` is the number of distinct malformed-chunk conditions; the variant
/// index selects the condition (low half) and the benign flag (high half), so
/// the case needs `variants: 2 * conds`.
unsafe fn run_ancillary_conditions(
    api: &Api,
    rng: &mut Rng,
    color: u8,
    depth: u8,
    build_chunk: impl Fn(usize, &mut Rng) -> Vec<u8>,
    conds: usize,
    tag: &str,
) {
    let v = variant() as usize;
    let cond = v % conds;
    let benign_on = v >= conds;
    p!("case {} cond={} benign_on={}", tag, cond, benign_on);
    let mut s = start();
    s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, depth, color, 0, 0, 0)));
    if color == 3 {
        let n = 1usize << depth.min(8);
        let pal: Vec<u8> = (0..3 * n).map(|_| rng.u8()).collect();
        s.extend_from_slice(&chunk(b"PLTE", &pal));
    }
    s.extend_from_slice(&build_chunk(cond, rng));
    let raw = raw_rows(rng, 8, 1, depth, color);
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    let mut rd = Reader::new(api, s);
    if benign_on {
        (api.png_set_benign_errors)(rd.png, 1);
    }
    drive(api, &rd);
    rd.destroy();
}

/// C73 (bad PLTE length) and C76 (PLTE in a grayscale image).
/// variants: 4.
fn plte(api: &Api, rng: &mut Rng) {
    let v = variant() as usize;
    let cond = v % 2;
    let benign_on = v >= 2;
    p!("case plte cond={} benign_on={}", cond, benign_on);
    let mut s = start();
    match cond {
        // C73: palette image with a PLTE length not a multiple of 3.
        0 => {
            s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 4, 3, 0, 0, 0)));
            s.extend_from_slice(&chunk(b"PLTE", &[1, 2, 3, 4])); // 4 bytes
            let raw = raw_rows(rng, 8, 1, 4, 3);
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
        }
        // C76: PLTE present in a grayscale image (benign, ignored).
        _ => {
            s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 8, 0, 0, 0, 0)));
            let pal: Vec<u8> = (0..3 * 4).map(|_| rng.u8()).collect();
            s.extend_from_slice(&chunk(b"PLTE", &pal));
            let raw = raw_rows(rng, 8, 1, 8, 0);
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
        }
    }
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        let mut rd = Reader::new(api, s);
        if benign_on {
            (api.png_set_benign_errors)(rd.png, 1);
        }
        drive(api, &rd);
        rd.destroy();
    }
}

/// C78 — IEND with a non-zero length. Benign.
/// variants: 2.
fn iend(api: &Api, rng: &mut Rng) {
    let benign_on = variant() >= 1;
    p!("case iend benign_on={}", benign_on);
    let mut s = start();
    s.extend_from_slice(&good_ihdr());
    let raw = raw_rows(rng, 8, 1, 8, 2);
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    s.extend_from_slice(&chunk(b"IEND", &[1, 2, 3, 4]));
    unsafe {
        let mut rd = Reader::new(api, s);
        if benign_on {
            (api.png_set_benign_errors)(rd.png, 1);
        }
        drive(api, &rd);
        rd.destroy();
    }
}

/// C79 — gAMA value > PNG_UINT_31_MAX. Benign.
/// variants: 2.
fn gama(api: &Api, rng: &mut Rng) {
    unsafe {
        run_ancillary_conditions(api, rng, 2, 8,
            |_c, _r| chunk(b"gAMA", &0x8000_0000u32.to_be_bytes()),
            1, "gama");
    }
}

/// C80 (bad sBIT length) and C81 (invalid sBIT sample). Benign.
/// variants: 4.
fn sbit(api: &Api, rng: &mut Rng) {
    unsafe {
        run_ancillary_conditions(api, rng, 2, 8,
            |c, _r| match c {
                // RGB needs 3 sBIT bytes; supply 2.
                0 => chunk(b"sBIT", &[8, 8]),
                // Sample byte 0 (invalid) / > sample depth.
                _ => chunk(b"sBIT", &[0, 9, 8]),
            },
            2, "sbit");
    }
}

/// C82 — a cHRM coordinate that overflows the checked int32 (0x80000000).
/// Benign.
/// variants: 2.
fn chrm(api: &Api, rng: &mut Rng) {
    unsafe {
        run_ancillary_conditions(api, rng, 2, 8,
            |_c, _r| {
                let mut d = Vec::new();
                for i in 0..8u32 {
                    if i == 0 {
                        d.extend_from_slice(&0x8000_0000u32.to_be_bytes());
                    } else {
                        d.extend_from_slice(&10000u32.to_be_bytes());
                    }
                }
                chunk(b"cHRM", &d)
            },
            1, "chrm");
    }
}

/// C83 — sRGB rendering intent byte > 3. Benign.
/// variants: 2.
fn srgb(api: &Api, rng: &mut Rng) {
    unsafe {
        run_ancillary_conditions(api, rng, 2, 8,
            |_c, _r| chunk(b"sRGB", &[4]),
            1, "srgb");
    }
}

/* ---- iCCP ---- */

/// Build a syntactically well-formed ICC profile of `len` bytes with the given
/// header fields, so that only the field under test is wrong. Layout per
/// ERRORS.md C89-C106:
///   0..3   = length (big-endian)
///   8      = version major
///   12..15 = profile class (4 bytes)
///   16..19 = colour space (4 bytes)
///   20..23 = PCS
///   36..39 = 'acsp'
///   64..67 = rendering intent (big-endian u32)
///   68..79 = PCS illuminant (D50 = 0x0000f6d6 0x00010000 0x0000d32d)
///   128..131 = tag count (big-endian)
fn icc_profile(
    len: usize,
    class: &[u8; 4],
    color_space: &[u8; 4],
    pcs: &[u8; 4],
    acsp_ok: bool,
    intent: u32,
    d50: bool,
    tag_count: u32,
) -> Vec<u8> {
    let mut p = vec![0u8; len.max(132)];
    let n = p.len();
    p[0..4].copy_from_slice(&(n as u32).to_be_bytes());
    p[8] = 2; // version major
    p[12..16].copy_from_slice(class);
    p[16..20].copy_from_slice(color_space);
    p[20..24].copy_from_slice(pcs);
    if acsp_ok {
        p[36..40].copy_from_slice(b"acsp");
    } else {
        p[36..40].copy_from_slice(b"xxxx");
    }
    p[64..68].copy_from_slice(&intent.to_be_bytes());
    if d50 {
        p[68..72].copy_from_slice(&0x0000_f6d6u32.to_be_bytes());
        p[72..76].copy_from_slice(&0x0001_0000u32.to_be_bytes());
        p[76..80].copy_from_slice(&0x0000_d32du32.to_be_bytes());
    }
    p[128..132].copy_from_slice(&tag_count.to_be_bytes());
    p
}

/// A default valid RGB profile (used as the base for single-field mutations).
fn icc_ok() -> Vec<u8> {
    icc_profile(132, b"scnr", b"RGB ", b"XYZ ", true, 0, true, 0)
}

/// Assemble an iCCP chunk: keyword\0 method deflate(profile). `method` and the
/// keyword can be corrupted by the caller.
fn iccp_chunk(keyword: &[u8], method: u8, deflate: &[u8]) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(keyword);
    d.push(0);
    d.push(method);
    d.extend_from_slice(deflate);
    chunk(b"iCCP", &d)
}

/// C84-C88 — iCCP frame-level defects. Benign.
/// variants: 10  (5 conditions × benign on/off).
fn iccp_frame(api: &Api, rng: &mut Rng) {
    unsafe {
        run_ancillary_conditions(api, rng, 2, 8,
            |c, _r| {
                let prof = icc_ok();
                match c {
                    // C84: length < 11 (LZ77Min) — total chunk too short.
                    0 => chunk(b"iCCP", &[b'x', 0, 0]),
                    // C85: compression method byte != 0.
                    1 => iccp_chunk(b"icc", 1, &zlib_store(&prof)),
                    // C86: keyword length > 79.
                    2 => {
                        let key = vec![b'k'; 90];
                        iccp_chunk(&key, 0, &zlib_store(&prof))
                    }
                    // C87: trailing bytes after the deflate stream ends.
                    3 => {
                        let mut z = zlib_store(&prof);
                        z.extend_from_slice(b"extra");
                        iccp_chunk(b"icc", 0, &z)
                    }
                    // C88: deflate payload is not a valid zlib stream.
                    _ => iccp_chunk(b"icc", 0, b"\x00\x00not-zlib"),
                }
            },
            5, "iccp_frame");
    }
}

/// C89-C106 — iCCP profile-header defects (post-inflate). Benign or warning.
/// variants: 36  (18 conditions × benign on/off).
fn iccp_profile(api: &Api, rng: &mut Rng) {
    unsafe {
        // color 2 (RGB) so a GRAY profile is rejected; some rows use grayscale.
        let v = variant() as usize;
        let conds = 18usize;
        let cond = v % conds;
        let benign_on = v >= conds;
        p!("case iccp_profile cond={} benign_on={}", cond, benign_on);
        // Most conditions test on an RGB image; C97 needs a grayscale image.
        let color: u8 = if cond == 8 { 0 } else { 2 };
        // Build the (possibly malformed) profile.
        let prof: Vec<u8> = match cond {
            0 => icc_profile(100, b"scnr", b"RGB ", b"XYZ ", true, 0, true, 0), // C89 too short (<132)
            1 => {
                // C90: length field != actual length.
                let mut p = icc_ok();
                p[0..4].copy_from_slice(&999u32.to_be_bytes());
                p
            }
            2 => {
                // C91: version > 3 with length not a multiple of 4.
                let mut p = icc_profile(134, b"scnr", b"RGB ", b"XYZ ", true, 0, true, 0);
                p[8] = 4;
                p
            }
            3 => {
                // C92: tag count too large.
                icc_profile(132, b"scnr", b"RGB ", b"XYZ ", true, 0, true, 0x2000_0000)
            }
            4 => icc_profile(132, b"scnr", b"RGB ", b"XYZ ", true, 0xffff, true, 0), // C93 intent >= 0xffff
            5 => icc_profile(132, b"scnr", b"RGB ", b"XYZ ", true, 5, true, 0),      // C94 intent in [4,0xfffe] (warn)
            6 => icc_profile(132, b"scnr", b"RGB ", b"XYZ ", false, 0, true, 0),     // C95 signature != acsp
            7 => icc_profile(132, b"scnr", b"RGB ", b"XYZ ", true, 0, false, 0),     // C96 PCS illuminant != D50 (warn)
            8 => icc_profile(132, b"scnr", b"RGB ", b"XYZ ", true, 0, true, 0),      // C97 RGB space on grayscale PNG
            9 => icc_profile(132, b"scnr", b"GRAY", b"XYZ ", true, 0, true, 0),      // C98 GRAY space on RGB PNG
            10 => icc_profile(132, b"scnr", b"CMYK", b"XYZ ", true, 0, true, 0),     // C99 invalid colour space
            11 => icc_profile(132, b"abst", b"RGB ", b"XYZ ", true, 0, true, 0),     // C100 Abstract class
            12 => icc_profile(132, b"link", b"RGB ", b"XYZ ", true, 0, true, 0),     // C101 DeviceLink class
            13 => icc_profile(132, b"nmcl", b"RGB ", b"XYZ ", true, 0, true, 0),     // C102 NamedColor (warn)
            14 => icc_profile(132, b"zzzz", b"RGB ", b"XYZ ", true, 0, true, 0),     // C103 unrecognized class (warn)
            15 => icc_profile(132, b"scnr", b"RGB ", b"zzz ", true, 0, true, 0),     // C104 bad PCS encoding
            16 => {
                // C105: a tag whose start is outside the profile.
                let mut p = icc_profile(144, b"scnr", b"RGB ", b"XYZ ", true, 0, true, 1);
                // tag table starts at byte 132: signature(4) offset(4) size(4).
                p[132..136].copy_from_slice(b"desc");
                p[136..140].copy_from_slice(&9_000_000u32.to_be_bytes()); // offset outside
                p[140..144].copy_from_slice(&10u32.to_be_bytes());
                p
            }
            _ => {
                // C106: tag start not a multiple of 4 (warn).
                let mut p = icc_profile(144, b"scnr", b"RGB ", b"XYZ ", true, 0, true, 1);
                p[132..136].copy_from_slice(b"desc");
                p[136..140].copy_from_slice(&133u32.to_be_bytes()); // offset 133, not %4
                p[140..144].copy_from_slice(&4u32.to_be_bytes());
                p
            }
        };
        let mut s = start();
        s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 8, color, 0, 0, 0)));
        s.extend_from_slice(&iccp_chunk(b"icc", 0, &zlib_store(&prof)));
        let raw = raw_rows(rng, 8, 1, 8, color);
        s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
        s.extend_from_slice(&chunk(b"IEND", &[]));
        let mut rd = Reader::new(api, s);
        if benign_on {
            (api.png_set_benign_errors)(rd.png, 1);
        }
        drive(api, &rd);
        rd.destroy();
    }
}

/* ---- sPLT / tRNS / bKGD / eXIf / hIST / pCAL / sCAL / zTXt / iTXt ---- */

/// C107 (malformed sPLT) and C108 (bad sPLT length). These are png_warning
/// (non-fatal); the read continues. No benign toggle needed.
/// variants: 2.
fn splt(api: &Api, rng: &mut Rng) {
    let cond = variant() as usize % 2;
    p!("case splt cond={}", cond);
    let mut s = start();
    s.extend_from_slice(&good_ihdr());
    match cond {
        // C107: name not NUL-terminated within the chunk (length >= 2 but no NUL).
        0 => s.extend_from_slice(&chunk(b"sPLT", &[b'n', b'a', b'm', b'e'])),
        // C108: data length not a multiple of the entry size. Name "p\0",
        // depth 8 => entry size 6; supply 5 trailing bytes.
        _ => {
            let mut d = b"p\0".to_vec();
            d.push(8); // sample depth
            d.extend_from_slice(&[1, 2, 3, 4, 5]);
            s.extend_from_slice(&chunk(b"sPLT", &d));
        }
    }
    let raw = raw_rows(rng, 8, 1, 8, 2);
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe { read_stream(api, s); }
}

/// C111/C112/C114/C115 — malformed tRNS. Benign.
/// variants: 8.
fn trns(api: &Api, rng: &mut Rng) {
    let v = variant() as usize;
    let conds = 4usize;
    let cond = v % conds;
    let benign_on = v >= conds;
    p!("case trns cond={} benign_on={}", cond, benign_on);
    let mut s = start();
    match cond {
        // C111: grayscale image with tRNS length != 2.
        0 => {
            s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 8, 0, 0, 0, 0)));
            s.extend_from_slice(&chunk(b"tRNS", &[0, 0, 0]));
            let raw = raw_rows(rng, 8, 1, 8, 0);
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
        }
        // C112: RGB image with tRNS length != 6.
        1 => {
            s.extend_from_slice(&good_ihdr());
            s.extend_from_slice(&chunk(b"tRNS", &[0, 0, 0, 0]));
            let raw = raw_rows(rng, 8, 1, 8, 2);
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
        }
        // C114: palette image, tRNS length > num_palette.
        2 => {
            s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 4, 3, 0, 0, 0)));
            let pal: Vec<u8> = (0..3 * 4).map(|_| rng.u8()).collect();
            s.extend_from_slice(&chunk(b"PLTE", &pal));
            // 4 palette entries but 8 tRNS values.
            s.extend_from_slice(&chunk(b"tRNS", &[0u8; 8]));
            let raw = raw_rows(rng, 8, 1, 4, 3);
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
        }
        // C115: color type already has an alpha channel (RGBA).
        _ => {
            s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 8, 6, 0, 0, 0)));
            s.extend_from_slice(&chunk(b"tRNS", &[0, 0, 0, 0, 0, 0]));
            let raw = raw_rows(rng, 8, 1, 8, 6);
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
        }
    }
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        let mut rd = Reader::new(api, s);
        if benign_on {
            (api.png_set_benign_errors)(rd.png, 1);
        }
        drive(api, &rd);
        rd.destroy();
    }
}

/// C117-C120 — malformed bKGD. Benign.
/// variants: 8.
fn bkgd(api: &Api, rng: &mut Rng) {
    let v = variant() as usize;
    let conds = 4usize;
    let cond = v % conds;
    let benign_on = v >= conds;
    p!("case bkgd cond={} benign_on={}", cond, benign_on);
    let mut s = start();
    match cond {
        // C117: bKGD length != truelen for the color type (RGB needs 6).
        0 => {
            s.extend_from_slice(&good_ihdr());
            s.extend_from_slice(&chunk(b"bKGD", &[0, 0]));
            let raw = raw_rows(rng, 8, 1, 8, 2);
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
        }
        // C118: palette image, bKGD index >= num_palette.
        1 => {
            s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 4, 3, 0, 0, 0)));
            let pal: Vec<u8> = (0..3 * 4).map(|_| rng.u8()).collect();
            s.extend_from_slice(&chunk(b"PLTE", &pal));
            s.extend_from_slice(&chunk(b"bKGD", &[200]));
            let raw = raw_rows(rng, 8, 1, 4, 3);
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
        }
        // C119: grayscale bit_depth<=8 with high byte nonzero / gray too big.
        2 => {
            s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 8, 0, 0, 0, 0)));
            // bKGD is 2 bytes (16-bit sample); high byte nonzero at depth 8.
            s.extend_from_slice(&chunk(b"bKGD", &[0x01, 0x00]));
            let raw = raw_rows(rng, 8, 1, 8, 0);
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
        }
        // C120: RGB bit_depth<=8 with a high byte nonzero.
        _ => {
            s.extend_from_slice(&good_ihdr());
            s.extend_from_slice(&chunk(b"bKGD", &[0x01, 0x00, 0x01, 0x00, 0x01, 0x00]));
            let raw = raw_rows(rng, 8, 1, 8, 2);
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
        }
    }
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        let mut rd = Reader::new(api, s);
        if benign_on {
            (api.png_set_benign_errors)(rd.png, 1);
        }
        drive(api, &rd);
        rd.destroy();
    }
}

/// C121 — eXIf TIFF header neither II*\0 nor MM\0*. Benign.
/// variants: 2.
fn exif(api: &Api, rng: &mut Rng) {
    unsafe {
        run_ancillary_conditions(api, rng, 2, 8,
            |_c, _r| chunk(b"eXIf", &[0x00, 0x00, 0x00, 0x00]),
            1, "exif");
    }
}

/// C122 — hIST length != 2*num_palette. Benign.
/// variants: 2.
fn hist(api: &Api, rng: &mut Rng) {
    let benign_on = variant() >= 1;
    p!("case hist benign_on={}", benign_on);
    let mut s = start();
    s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 1, 4, 3, 0, 0, 0)));
    let pal: Vec<u8> = (0..3 * 4).map(|_| rng.u8()).collect();
    s.extend_from_slice(&chunk(b"PLTE", &pal));
    // 4 palette entries => hIST must be 8 bytes; supply 6.
    s.extend_from_slice(&chunk(b"hIST", &[0u8; 6]));
    let raw = raw_rows(rng, 8, 1, 4, 3);
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        let mut rd = Reader::new(api, s);
        if benign_on {
            (api.png_set_benign_errors)(rd.png, 1);
        }
        drive(api, &rd);
        rd.destroy();
    }
}

/// C123-C126 — malformed pCAL. Benign.
/// variants: 8.
fn pcal(api: &Api, rng: &mut Rng) {
    unsafe {
        run_ancillary_conditions(api, rng, 2, 8,
            |c, _r| {
                match c {
                    // C123: purpose string not NUL-terminated in length-12.
                    0 => chunk(b"pCAL", &[b'p'; 20]),
                    // C124: nparams wrong for the equation type.
                    1 => {
                        // purpose\0 x0(4) x1(4) type(1) nparams(1) unit\0
                        let mut d = b"p\0".to_vec();
                        d.extend_from_slice(&0i32.to_be_bytes());
                        d.extend_from_slice(&100i32.to_be_bytes());
                        d.push(0); // equation type 0 (linear) -> expects 2 params
                        d.push(9); // nparams = 9 (wrong)
                        d.extend_from_slice(b"u\0");
                        chunk(b"pCAL", &d)
                    }
                    // C125: equation type >= 4.
                    2 => {
                        let mut d = b"p\0".to_vec();
                        d.extend_from_slice(&0i32.to_be_bytes());
                        d.extend_from_slice(&100i32.to_be_bytes());
                        d.push(7); // invalid equation type
                        d.push(0);
                        d.extend_from_slice(b"u\0");
                        chunk(b"pCAL", &d)
                    }
                    // C126: a parameter string runs past the chunk end.
                    _ => {
                        let mut d = b"p\0".to_vec();
                        d.extend_from_slice(&0i32.to_be_bytes());
                        d.extend_from_slice(&100i32.to_be_bytes());
                        d.push(0); // linear, 2 params
                        d.push(2);
                        d.extend_from_slice(b"u\0");
                        d.extend_from_slice(b"1.0\0"); // only one param, second missing
                        chunk(b"pCAL", &d)
                    }
                }
            },
            4, "pcal");
    }
}

/// C127-C131 — malformed sCAL. Benign.
/// variants: 10.
fn scal(api: &Api, rng: &mut Rng) {
    unsafe {
        run_ancillary_conditions(api, rng, 2, 8,
            |c, _r| {
                match c {
                    // C127: unit byte neither 1 nor 2.
                    0 => chunk(b"sCAL", &[3, b'1', b'\0', b'1']),
                    // C128: width not a valid fp string.
                    1 => {
                        let mut d = vec![1u8];
                        d.extend_from_slice(b"abc\0");
                        d.extend_from_slice(b"1");
                        chunk(b"sCAL", &d)
                    }
                    // C129: width parses but is not positive.
                    2 => {
                        let mut d = vec![1u8];
                        d.extend_from_slice(b"0\0");
                        d.extend_from_slice(b"1");
                        chunk(b"sCAL", &d)
                    }
                    // C130: height not a valid fp string / trailing bytes.
                    3 => {
                        let mut d = vec![1u8];
                        d.extend_from_slice(b"1\0");
                        d.extend_from_slice(b"xyz");
                        chunk(b"sCAL", &d)
                    }
                    // C131: height parses but is not positive.
                    _ => {
                        let mut d = vec![1u8];
                        d.extend_from_slice(b"1\0");
                        d.extend_from_slice(b"0");
                        chunk(b"sCAL", &d)
                    }
                }
            },
            5, "scal");
    }
}

/// C134-C137 — malformed zTXt. Benign.
/// variants: 8.
fn ztxt(api: &Api, rng: &mut Rng) {
    unsafe {
        run_ancillary_conditions(api, rng, 2, 8,
            |c, _r| {
                match c {
                    // C134: keyword length 0.
                    0 => chunk(b"zTXt", &[0, 0]),
                    // C135: keyword_length + 3 > length (truncated).
                    1 => {
                        let mut d = b"key".to_vec();
                        // no NUL, no method, no data -> truncated
                        d.truncate(3);
                        chunk(b"zTXt", &d)
                    }
                    // C136: compression method byte != 0.
                    2 => {
                        let mut d = b"key\0".to_vec();
                        d.push(1); // bad compression method
                        d.extend_from_slice(&zlib_store(b"hello"));
                        chunk(b"zTXt", &d)
                    }
                    // C137: deflate payload invalid.
                    _ => {
                        let mut d = b"key\0".to_vec();
                        d.push(0);
                        d.extend_from_slice(b"\x00\x00not-zlib");
                        chunk(b"zTXt", &d)
                    }
                }
            },
            4, "ztxt");
    }
}

/// C138-C142 — malformed iTXt. Benign.
/// variants: 10.
fn itxt(api: &Api, rng: &mut Rng) {
    unsafe {
        run_ancillary_conditions(api, rng, 2, 8,
            |c, _r| {
                // iTXt layout: keyword\0 comp_flag comp_method lang\0 transkey\0 text
                match c {
                    // C138: keyword length 0.
                    0 => chunk(b"iTXt", &[0, 0, 0, 0, 0]),
                    // C139: prefix_length + 5 > length (truncated).
                    1 => chunk(b"iTXt", b"key"),
                    // C140: compression flag not 0/1.
                    2 => {
                        let mut d = b"key\0".to_vec();
                        d.push(5); // bad compression flag
                        d.push(0);
                        d.extend_from_slice(b"\0\0text");
                        chunk(b"iTXt", &d)
                    }
                    // C141: compressed, prefix_length >= length.
                    3 => {
                        let mut d = b"key\0".to_vec();
                        d.push(1); // compressed
                        d.push(0);
                        d.extend_from_slice(b"en\0"); // lang
                        // truncate before transkey/text
                        chunk(b"iTXt", &d)
                    }
                    // C142: compressed payload invalid.
                    _ => {
                        let mut d = b"key\0".to_vec();
                        d.push(1); // compressed
                        d.push(0);
                        d.extend_from_slice(b"en\0"); // lang
                        d.extend_from_slice(b"tk\0"); // transkey
                        d.extend_from_slice(b"\x00\x00not-zlib");
                        chunk(b"iTXt", &d)
                    }
                }
            },
            5, "itxt");
    }
}

/* ================================================================== */
/* 5. row / IDAT decode path and misc                                 */
/* ================================================================== */

/// C143 — a second chunk needing the zstream while it is still owned.
/// W:"<chunk> using zstream" (non-fatal warning). Build two compressed text
/// chunks (zTXt) so the inflate claim path is exercised twice.
/// variants: 1.
fn zstream_reuse(api: &Api, rng: &mut Rng) {
    p!("case zstream_reuse");
    let mut s = start();
    s.extend_from_slice(&good_ihdr());
    // An iCCP holds the zstream, then a zTXt tries to claim it too. We put two
    // compressed chunks back to back before IDAT.
    let prof = icc_ok();
    s.extend_from_slice(&iccp_chunk(b"icc", 0, &zlib_store(&prof)));
    let mut zt = b"key\0".to_vec();
    zt.push(0);
    zt.extend_from_slice(&zlib_store(b"some text"));
    s.extend_from_slice(&chunk(b"zTXt", &zt));
    let raw = raw_rows(rng, 8, 1, 8, 2);
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe { read_stream(api, s); }
}

/// C144 — png_read_row called before any IDAT was seen.
/// E:"Invalid attempt to read row data". variants: 1.
fn row_before_idat(api: &Api, rng: &mut Rng) {
    p!("case row_before_idat");
    // Stream with a valid header but NO IDAT; then attempt to read a row.
    let mut s = start();
    s.extend_from_slice(&good_ihdr());
    s.extend_from_slice(&chunk(b"IEND", &[]));
    let _ = rng;
    unsafe {
        let mut rd = Reader::new(api, s);
        (api.png_read_info)(rd.png, rd.info);
        emit("survived read_info");
        // Do NOT call read_update_info via IDAT; force a row read directly.
        let rb = (api.png_get_rowbytes)(rd.png, rd.info);
        let mut row = vec![0u8; rb + 8];
        (api.png_read_row)(rd.png, row.as_mut_ptr(), null());
        emit("survived read_row");
        rd.destroy();
    }
}

/// C145 (sequential) / C170 (progressive) — a row filter byte >= 5.
/// E:"bad adaptive filter value". variants: 2.
fn bad_filter_value(api: &Api, rng: &mut Rng) {
    let prog = variant() % 2;
    p!("case bad_filter_value prog={}", prog);
    // Build raw rows but set the filter byte of the first row to 99.
    let (w, h) = (8u32, 2u32);
    let rb = rowbytes_of(w, 8, 2);
    let mut raw = Vec::new();
    for r in 0..h {
        raw.push(if r == 0 { 99u8 } else { 0u8 }); // invalid filter on row 0
        raw.extend_from_slice(&rng.image_bytes(rb));
    }
    let mut s = start();
    s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(w, h, 8, 2, 0, 0, 0)));
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        if prog == 0 {
            read_stream(api, s);
        } else {
            feed_progressive(api, &s);
            emit("survived progressive");
        }
    }
}

/// C146/C147 (sequential) and C165/C168 (progressive) — the compressed stream
/// ends before all rows are produced.
/// variants: 4.
fn not_enough_data(api: &Api, rng: &mut Rng) {
    let v = variant() % 4;
    let non_idat = v % 2 == 1; // C147/C165: a non-IDAT chunk interrupts
    let prog = v / 2;
    p!("case not_enough_data non_idat={} prog={}", non_idat, prog);
    // Declare a tall image but only supply enough compressed data for 1 row.
    let (w, h) = (8u32, 8u32);
    let rb = rowbytes_of(w, 8, 2);
    let mut short_raw = Vec::new();
    short_raw.push(0u8);
    short_raw.extend_from_slice(&rng.image_bytes(rb)); // only 1 row
    let z = zlib_store(&short_raw);
    let mut s = start();
    s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(w, h, 8, 2, 0, 0, 0)));
    if non_idat {
        // half the compressed data, a non-IDAT chunk, then no more IDAT.
        let half = z.len() / 2;
        s.extend_from_slice(&chunk(b"IDAT", &z[..half]));
        s.extend_from_slice(&chunk(b"tEXt", b"k\0v"));
    } else {
        s.extend_from_slice(&chunk(b"IDAT", &z));
    }
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        if prog == 0 {
            read_stream(api, s);
        } else {
            feed_progressive(api, &s);
            emit("survived progressive");
        }
    }
}

/// C148 (sequential) / C169 (progressive) — more inflated bytes than the image
/// needs. Benign (sequential) / warning (progressive).
/// variants: 4  (sequential benign on/off, plus progressive).
fn too_much_data(api: &Api, rng: &mut Rng) {
    let v = variant() as usize;
    // v 0/1: sequential benign off/on ; v 2: progressive ; v 3: progressive dup
    let prog = v >= 2;
    let benign_on = v == 1;
    p!("case too_much_data prog={} benign_on={}", prog, benign_on);
    // Declare a 1-row image but supply data for 3 rows.
    let (w, h) = (8u32, 1u32);
    let rb = rowbytes_of(w, 8, 2);
    let mut big_raw = Vec::new();
    for _ in 0..3 {
        big_raw.push(0u8);
        big_raw.extend_from_slice(&rng.image_bytes(rb));
    }
    let z = zlib_store(&big_raw);
    let mut s = start();
    s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(w, h, 8, 2, 0, 0, 0)));
    s.extend_from_slice(&chunk(b"IDAT", &z));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        if prog {
            feed_progressive(api, &s);
            emit("survived progressive");
        } else {
            let mut rd = Reader::new(api, s);
            if benign_on {
                (api.png_set_benign_errors)(rd.png, 1);
            }
            drive(api, &rd);
            rd.destroy();
        }
    }
}

/// C150/C151 (sequential) and C166/C167 (progressive) — corrupt deflate data
/// or a wrong Adler-32 in IDAT.
/// variants: 4.
fn idat_corrupt(api: &Api, rng: &mut Rng) {
    let v = variant() % 4;
    let adler = v % 2 == 1; // corrupt Adler (C151/C166) vs deflate body (C150/C167)
    let prog = v / 2;
    p!("case idat_corrupt adler={} prog={}", adler, prog);
    let (w, h) = (8u32, 2u32);
    let raw = raw_rows(rng, w, h, 8, 2);
    let mut z = zlib_store(&raw);
    if adler {
        // Corrupt the trailing Adler-32.
        let n = z.len();
        z[n - 1] ^= 0xff;
    } else {
        // Corrupt a deflate body byte (after the 2-byte zlib header).
        if z.len() > 4 {
            z[3] ^= 0xff;
        }
    }
    let mut s = start();
    s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(w, h, 8, 2, 0, 0, 0)));
    s.extend_from_slice(&chunk(b"IDAT", &z));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        if prog == 0 {
            read_stream(api, s);
        } else {
            feed_progressive(api, &s);
            emit("survived progressive");
        }
    }
}

/// C152 — rowbytes > 65535. E:"This image requires a row greater than 64KB".
/// Raise the user limits so the width check does not fire first, declare a
/// large WIDTH with HEIGHT=1 and NO real image data (the error fires in
/// png_read_start_row before any row is decoded).
/// variants: 1.
fn row_too_big(api: &Api, rng: &mut Rng) {
    p!("case row_too_big");
    // width 70000, 8-bit RGBA => rowbytes = 70000*4 = 280000 > 65535.
    let mut s = start();
    s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(70000, 1, 8, 6, 0, 0, 0)));
    // A single empty IDAT is enough to reach start_row; no real data needed.
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&[])));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    let _ = rng;
    unsafe {
        let mut rd = Reader::new(api, s);
        (api.png_set_user_limits)(rd.png, 0x7fff_ffff, 0x7fff_ffff);
        (api.png_read_info)(rd.png, rd.info);
        emit("survived read_info");
        (api.png_read_update_info)(rd.png, rd.info);
        emit("survived read_update_info");
        // Attempt the first row; the row buffer allocation / start_row check
        // fires here. Size the buffer safely (never read OOB).
        let rb = (api.png_get_rowbytes)(rd.png, rd.info);
        let mut row = vec![0u8; rb + 8];
        (api.png_read_row)(rd.png, row.as_mut_ptr(), null());
        emit("survived read_row");
        rd.destroy();
    }
}

/// C153/C154 — png_read_update_info / png_start_read_image called twice.
/// A:"...: duplicate call" (fatal, PNG_APP_ERRORS_WARN unset).
/// variants: 2.
fn duplicate_update(api: &Api, rng: &mut Rng) {
    let which = variant() % 2;
    p!("case duplicate_update which={}", which);
    let data = build_png(rng, 8, 1, 8, 2, &[], &[]);
    unsafe {
        let mut rd = Reader::new(api, data);
        (api.png_read_info)(rd.png, rd.info);
        emit("survived read_info");
        if which == 0 {
            // C153: update_info twice.
            (api.png_read_update_info)(rd.png, rd.info);
            emit("first update_info");
            (api.png_read_update_info)(rd.png, rd.info);
            emit("second update_info");
        } else {
            // C154: start_read_image after update_info.
            (api.png_read_update_info)(rd.png, rd.info);
            emit("update_info");
            (api.png_start_read_image)(rd.png);
            emit("start_read_image");
        }
        rd.destroy();
    }
}

/// C155/C156 — png_read_png with an image too tall to process, and the
/// interlace-handling advisory on png_read_image.
/// C155: E:"Image is too high to process with png_read_png()".
/// C156: W:"Interlace handling should be turned on when using png_read_image".
/// variants: 2.
fn read_png_limits(api: &Api, rng: &mut Rng) {
    let which = variant() % 2;
    p!("case read_png_limits which={}", which);
    unsafe {
        if which == 0 {
            // C155: declare an enormous height. Raise user limits so the height
            // check inside png_read_png (not the IHDR user limit) is what fires.
            let mut s = start();
            s.extend_from_slice(&chunk(b"IHDR", &ihdr_data(1, 0x7fff_ffff, 8, 0, 0, 0, 0)));
            s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&[])));
            s.extend_from_slice(&chunk(b"IEND", &[]));
            let mut rd = Reader::new(api, s);
            (api.png_set_user_limits)(rd.png, 0x7fff_ffff, 0x7fff_ffff);
            (api.png_read_png)(rd.png, rd.info, PNG_TRANSFORM_IDENTITY, null());
            emit("survived read_png");
            rd.destroy();
        } else {
            // C156: interlaced image read with png_read_image without
            // png_set_interlace_handling -> advisory warning.
            let mut wr = Writer::new(api);
            (api.png_set_IHDR)(wr.png, wr.info, 8, 4, 8, 2,
                PNG_INTERLACE_ADAM7, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
            (api.png_write_info)(wr.png, wr.info);
            let rb = rowbytes_of(8, 8, 2);
            let mut rows: Vec<Vec<u8>> = (0..4).map(|_| rng.image_bytes(rb)).collect();
            for _ in 0..7 {
                for r in rows.iter_mut() {
                    (api.png_write_row)(wr.png, r.as_ptr());
                }
            }
            (api.png_write_end)(wr.png, wr.info);
            let data = wbuf().clone();
            wr.destroy();
            let mut rd = Reader::new(api, data);
            (api.png_read_info)(rd.png, rd.info);
            emit("survived read_info");
            (api.png_read_update_info)(rd.png, rd.info);
            let h = (api.png_get_image_height)(rd.png, rd.info);
            let rb = (api.png_get_rowbytes)(rd.png, rd.info);
            let mut storage: Vec<Vec<u8>> = (0..h as usize).map(|_| vec![0u8; rb + 8]).collect();
            let mut ptrs: Vec<*mut u8> = storage.iter_mut().map(|v| v.as_mut_ptr()).collect();
            (api.png_read_image)(rd.png, ptrs.as_mut_ptr());
            (api.png_read_end)(rd.png, rd.info);
            emit("survived read_image");
            rd.destroy();
        }
    }
}

/// C157 — png_get_int_32 on 0x80000000 (two's-complement negate overflow) via
/// an oFFs chunk. -> the accessor returns 0 for that field. This is a value
/// check with no callback; drive an oFFs chunk with x_offset = 0x80000000 and
/// read it back.
/// variants: 1.
fn int32_overflow(api: &Api, rng: &mut Rng) {
    p!("case int32_overflow");
    let mut s = start();
    s.extend_from_slice(&good_ihdr());
    // oFFs: x(4) y(4) unit(1). Set x to 0x80000000.
    let mut d = Vec::new();
    d.extend_from_slice(&0x8000_0000u32.to_be_bytes());
    d.extend_from_slice(&0i32.to_be_bytes());
    d.push(0);
    s.extend_from_slice(&chunk(b"oFFs", &d));
    let raw = raw_rows(rng, 8, 1, 8, 2);
    s.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
    s.extend_from_slice(&chunk(b"IEND", &[]));
    unsafe {
        let mut rd = Reader::new(api, s);
        (api.png_read_info)(rd.png, rd.info);
        let xo = (api.png_get_x_offset_pixels)(rd.png, rd.info);
        p!("x_offset_pixels={}", xo);
        (api.png_read_update_info)(rd.png, rd.info);
        let rb = (api.png_get_rowbytes)(rd.png, rd.info);
        let mut row = vec![0u8; rb + 8];
        (api.png_read_row)(rd.png, row.as_mut_ptr(), null());
        (api.png_read_end)(rd.png, rd.info);
        emit("survived read_end");
        rd.destroy();
    }
}

/// C325/C326/C327 — row / display_row NULL handling.
/// C325: png_read_row with both row and display_row NULL (row consumed).
/// C326: png_read_rows with NULL row pointers.
/// C327: mixed. All must be non-crashing and identical.
/// variants: 3.
fn row_null(api: &Api, rng: &mut Rng) {
    let which = variant() % 3;
    p!("case row_null which={}", which);
    let (w, h) = (8u32, 3u32);
    let data = build_png(rng, w, h, 8, 2, &[], &[]);
    unsafe {
        let mut rd = Reader::new(api, data);
        (api.png_read_info)(rd.png, rd.info);
        (api.png_read_update_info)(rd.png, rd.info);
        match which {
            // C325: png_read_row(png, NULL, NULL) for every row.
            0 => {
                for _ in 0..h {
                    (api.png_read_row)(rd.png, null(), null());
                }
                emit("read_row null done");
            }
            // C326: png_read_rows with NULL row pointer array (display only).
            1 => {
                let rb = (api.png_get_rowbytes)(rd.png, rd.info);
                for _ in 0..h {
                    let mut disp = vec![0u8; rb + 8];
                    let mut dp = disp.as_mut_ptr();
                    (api.png_read_rows)(rd.png, null::<*mut u8>() as *mut *mut u8, &mut dp, 1);
                }
                emit("read_rows null-row done");
            }
            // C327: png_read_rows with both arrays NULL.
            _ => {
                for _ in 0..h {
                    (api.png_read_rows)(
                        rd.png,
                        null::<*mut u8>() as *mut *mut u8,
                        null::<*mut u8>() as *mut *mut u8,
                        1,
                    );
                }
                emit("read_rows both-null done");
            }
        }
        (api.png_read_end)(rd.png, rd.info);
        emit("survived read_end");
        rd.destroy();
    }
}

/// C328/C329 — a 0-byte stream and a mid-chunk-truncated stream. The harness
/// read_fn reports EOF as a fatal error; both libraries must agree.
/// variants: 2.
fn truncated_stream(api: &Api, rng: &mut Rng) {
    let which = variant() % 2;
    p!("case truncated_stream which={}", which);
    unsafe {
        if which == 0 {
            // C328: completely empty stream.
            read_stream(api, Vec::new());
        } else {
            // C329: truncated mid-chunk (signature + partial IHDR).
            let full = build_png(rng, 8, 1, 8, 2, &[], &[]);
            let cut = full.len().min(8 + 8); // sig + chunk length/type only
            read_stream(api, full[..cut].to_vec());
        }
    }
}

/// C158 — png_get_uint_31 on a value > PNG_UINT_31_MAX in a length field.
/// E:"PNG unsigned integer out of range". This is reached by a chunk whose
/// dimension field exceeds the 31-bit limit; use an sCAL/oFFs-style path via a
/// crafted IHDR width read through png_get_uint_31 internally. The most direct
/// FFI trigger is a chunk length header with the high bit set (same as
/// chunk_header len_over_uint31), so build that.
/// variants: 1.
fn uint31_range(api: &Api, rng: &mut Rng) {
    p!("case uint31_range");
    let mut v = start();
    // A chunk whose declared length exceeds PNG_UINT_31_MAX.
    v.extend_from_slice(&0x8000_0001u32.to_be_bytes());
    v.extend_from_slice(b"IHDR");
    v.extend_from_slice(&[0u8; 13]);
    v.extend_from_slice(&0u32.to_be_bytes());
    let _ = rng;
    unsafe { read_stream(api, v); }
}

/// C36 — png_process_data_skip: W:"...not implemented...", -> 0. Non-fatal.
/// variants: 1.
fn process_data_skip(api: &Api, rng: &mut Rng) {
    p!("case process_data_skip");
    let _ = rng;
    unsafe {
        let png = (api.png_create_read_struct)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            Some(err_fn),
            Some(warn_fn),
        );
        let r = (api.png_process_data_skip)(png);
        p!("process_data_skip -> {}", r);
        let mut pp = png;
        (api.png_destroy_read_struct)(&mut pp, null(), null());
    }
}

/// C27 — png_get_eXIf (deprecated): W:"...use png_get_eXIf_1", -> 0.
/// variants: 1.
fn get_exif_deprecated(api: &Api, rng: &mut Rng) {
    p!("case get_eXIf_deprecated");
    let _ = rng;
    unsafe {
        let png = (api.png_create_read_struct)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            Some(err_fn),
            Some(warn_fn),
        );
        let info = (api.png_create_info_struct)(png);
        let mut exif: *mut u8 = std::ptr::null_mut();
        let r = (api.png_get_eXIf)(png, info, &mut exif);
        p!("get_eXIf -> {} null={}", r, exif.is_null());
        let mut pp = png;
        let mut ip = info;
        (api.png_destroy_read_struct)(&mut pp, &mut ip, null());
    }
}
