//! Phase B — the *high level* entry points `png_read_png` / `png_write_png`.
//!
//! Covers CONFIGS.md rows
//!   * C-131 — `highlevel::read_png`
//!   * C-132 — `highlevel::write_png`
//!   * C-133 — `highlevel::round_trip`
//!
//! `png_read_png` (`c_src/src/pngread.c:868`) and `png_write_png`
//! (`c_src/src/pngwrite.c:1407`) are thin wrappers that translate a
//! `PNG_TRANSFORM_*` bit set into `png_set_*` calls and then drive a complete
//! read / write through `info_ptr->row_pointers`.  Everything they can be
//! observed doing is compared here: the `Guard`, every warning, the whole of
//! `png_get_rows()` byte for byte, all the `png_get_*` state afterwards and (for
//! the write side) the produced file.
//!
//! ## The uninitialised-tail trap
//!
//! `png_combine_row` (`c_src/src/pngrutil.c:3227`) *preserves* the padding bits
//! of the last byte of a row from the destination buffer:
//!
//! ```c
//!    end_mask = (pixel_depth * row_width) & 7;
//!    if (end_mask != 0) { end_ptr = dp + PNG_ROWBYTES(..) - 1; end_byte = *end_ptr; ... }
//!    ...
//!    if (end_ptr != NULL)
//!       *end_ptr = (png_byte)((end_byte & end_mask) | (*end_ptr & ~end_mask));
//! ```
//!
//! When `png_read_png` allocates the rows itself it uses `png_malloc`, which
//! does **not** zero, so for sub-byte pixel depths whose row is not a whole
//! number of bytes those padding bits are uninitialised heap and must not be
//! compared.  Two arrangements are therefore exercised:
//!
//! * `Rows::Own` — the test installs its own zeroed rows with `png_set_rows`
//!   before the call (`info_ptr->free_me` has no `PNG_FREE_ROWS`, so
//!   `png_free_data(.., PNG_FREE_ROWS, 0)` inside `png_read_png` leaves them
//!   alone).  Fully deterministic, every byte is compared.
//! * `Rows::Lib` — `png_read_png` mallocs the rows.  The final byte of each row
//!   is blanked before comparison when the row has padding bits.
#![allow(non_snake_case)]

mod common;

use common::*;
use core::ffi::{c_int, c_void};
use core::ptr;

/* ------------------------------------------------------------------ */
/* the transform flags                                                 */
/* ------------------------------------------------------------------ */

/// Every `PNG_TRANSFORM_*` bit defined in `c_src/include/png.h:850..871`.
const ALL_FLAGS: [(&str, c_int); 16] = [
    ("STRIP_16", PNG_TRANSFORM_STRIP_16),
    ("STRIP_ALPHA", PNG_TRANSFORM_STRIP_ALPHA),
    ("PACKING", PNG_TRANSFORM_PACKING),
    ("PACKSWAP", PNG_TRANSFORM_PACKSWAP),
    ("EXPAND", PNG_TRANSFORM_EXPAND),
    ("INVERT_MONO", PNG_TRANSFORM_INVERT_MONO),
    ("SHIFT", PNG_TRANSFORM_SHIFT),
    ("BGR", PNG_TRANSFORM_BGR),
    ("SWAP_ALPHA", PNG_TRANSFORM_SWAP_ALPHA),
    ("SWAP_ENDIAN", PNG_TRANSFORM_SWAP_ENDIAN),
    ("INVERT_ALPHA", PNG_TRANSFORM_INVERT_ALPHA),
    ("STRIP_FILLER_BEFORE", PNG_TRANSFORM_STRIP_FILLER_BEFORE),
    ("STRIP_FILLER_AFTER", PNG_TRANSFORM_STRIP_FILLER_AFTER),
    ("GRAY_TO_RGB", PNG_TRANSFORM_GRAY_TO_RGB),
    ("EXPAND_16", PNG_TRANSFORM_EXPAND_16),
    ("SCALE_16", PNG_TRANSFORM_SCALE_16),
];

/// The flags `png_read_png` actually acts on (`pngread.c:892..1027`).
const READ_FLAGS: [c_int; 14] = [
    PNG_TRANSFORM_SCALE_16,
    PNG_TRANSFORM_STRIP_16,
    PNG_TRANSFORM_STRIP_ALPHA,
    PNG_TRANSFORM_PACKING,
    PNG_TRANSFORM_PACKSWAP,
    PNG_TRANSFORM_EXPAND,
    PNG_TRANSFORM_INVERT_MONO,
    PNG_TRANSFORM_SHIFT,
    PNG_TRANSFORM_BGR,
    PNG_TRANSFORM_SWAP_ALPHA,
    PNG_TRANSFORM_SWAP_ENDIAN,
    PNG_TRANSFORM_INVERT_ALPHA,
    PNG_TRANSFORM_GRAY_TO_RGB,
    PNG_TRANSFORM_EXPAND_16,
];

/// The flags `png_write_png` acts on (`pngwrite.c:1427..1516`).
const WRITE_FLAGS: [c_int; 10] = [
    PNG_TRANSFORM_INVERT_MONO,
    PNG_TRANSFORM_SHIFT,
    PNG_TRANSFORM_PACKING,
    PNG_TRANSFORM_SWAP_ALPHA,
    PNG_TRANSFORM_STRIP_FILLER_BEFORE,
    PNG_TRANSFORM_STRIP_FILLER_AFTER,
    PNG_TRANSFORM_BGR,
    PNG_TRANSFORM_SWAP_ENDIAN,
    PNG_TRANSFORM_PACKSWAP,
    PNG_TRANSFORM_INVERT_ALPHA,
];

/// Write-only flags — `png_read_png` has no code for them at all ("We don't
/// handle adding filler bytes", `pngread.c:1029`), so they must be ignored.
const WRITE_ONLY_FLAGS: [c_int; 3] = [
    PNG_TRANSFORM_STRIP_FILLER_BEFORE,
    PNG_TRANSFORM_STRIP_FILLER_AFTER,
    PNG_TRANSFORM_STRIP_FILLER_BEFORE | PNG_TRANSFORM_STRIP_FILLER_AFTER,
];

/// Read-only flags — `png_write_png` has no code for them, so they must be
/// ignored on write.
const READ_ONLY_FLAGS: [c_int; 6] = [
    PNG_TRANSFORM_STRIP_16,
    PNG_TRANSFORM_STRIP_ALPHA,
    PNG_TRANSFORM_EXPAND,
    PNG_TRANSFORM_GRAY_TO_RGB,
    PNG_TRANSFORM_EXPAND_16,
    PNG_TRANSFORM_SCALE_16,
];

fn tname(t: c_int) -> String {
    if t == 0 {
        return "IDENTITY".to_string();
    }
    let mut v: Vec<&str> = Vec::new();
    let mut known: c_int = 0;
    for (n, f) in ALL_FLAGS {
        known |= f;
        if t & f != 0 {
            v.push(n);
        }
    }
    if t & !known != 0 {
        v.push("<unknown>");
    }
    v.join("|")
}

/// A random set of `2..=6` of the given flags.
fn random_combo(rng: &mut Rng, flags: &[c_int]) -> c_int {
    let k = rng.range(2, 7) as usize;
    let mut t = 0;
    for _ in 0..k {
        t |= flags[rng.below(flags.len())];
    }
    t
}

/* ------------------------------------------------------------------ */
/* the optional chunks that make the transforms bite                   */
/* ------------------------------------------------------------------ */

/// `png_read_png` only honours `PNG_TRANSFORM_SHIFT` when the file carries
/// sBIT (`pngread.c:973`) and `PNG_TRANSFORM_EXPAND` only produces an alpha
/// channel when it carries tRNS, so half the source images get both.
#[derive(Clone, Debug)]
struct Rich {
    sbit: png_color_8,
    trns: Trns,
}

#[derive(Clone, Debug)]
enum Trns {
    None,
    /// tRNS for a palette image: one alpha byte per entry.
    Palette(Vec<u8>),
    /// tRNS for gray / RGB.
    Color(png_color_16),
}

fn rich_for(rng: &mut Rng, ct: c_int, bd: c_int, npal: usize) -> Rich {
    // sBIT must satisfy png_write_sBIT (pngwutil.c:1250/1266/1278): 0 < v <= depth
    // (<= 8 for a palette).  Deliberately smaller than the depth so SHIFT shifts.
    let color_max = if ct == PNG_COLOR_TYPE_PALETTE { 8 } else { bd };
    let c = ((color_max / 2).max(1)) as u8;
    let g = ((bd / 2).max(1)) as u8;
    let sbit = png_color_8 {
        red: c,
        green: c,
        blue: c,
        gray: g,
        alpha: g,
    };
    let sample_max: u32 = if bd >= 16 { 0xffff } else { (1u32 << bd) - 1 };
    let trns = match ct {
        // png_write_tRNS requires 0 < num_trans <= num_palette.
        PNG_COLOR_TYPE_PALETTE => {
            let n = 1 + rng.below(npal.max(1));
            Trns::Palette((0..n).map(|_| rng.u8()).collect())
        }
        PNG_COLOR_TYPE_GRAY => Trns::Color(png_color_16 {
            index: 0,
            red: 0,
            green: 0,
            blue: 0,
            gray: (rng.u32() % (sample_max + 1)) as u16,
        }),
        PNG_COLOR_TYPE_RGB => Trns::Color(png_color_16 {
            index: 0,
            red: (rng.u32() % (sample_max + 1)) as u16,
            green: (rng.u32() % (sample_max + 1)) as u16,
            blue: (rng.u32() % (sample_max + 1)) as u16,
            gray: 0,
        }),
        // tRNS is illegal with an alpha channel ("Can't write tRNS with an
        // alpha channel", pngwutil.c).
        _ => Trns::None,
    };
    Rich { sbit, trns }
}

unsafe fn install_rich(api: &Api, png: *mut PngStruct, info: *mut PngInfo, r: &Rich) {
    (api.png_set_sBIT)(png, info, &r.sbit);
    match &r.trns {
        Trns::None => {}
        Trns::Palette(v) => {
            (api.png_set_tRNS)(png, info, v.as_ptr(), v.len() as c_int, ptr::null())
        }
        Trns::Color(c) => (api.png_set_tRNS)(png, info, ptr::null(), 1, c),
    }
}

/* ------------------------------------------------------------------ */
/* recording                                                           */
/* ------------------------------------------------------------------ */

/// Everything `png_get_*` reports that `log_info` does not already cover.
unsafe fn log_more(api: &Api, png: *mut PngStruct, info: *mut PngInfo, tag: &str) {
    let mut pal: *mut png_color = ptr::null_mut();
    let mut npal: c_int = -1;
    let r = (api.png_get_PLTE)(png, info, &mut pal, &mut npal);
    log(format!("{}: PLTE r={} n={}", tag, r, npal));
    if r != 0 && !pal.is_null() && npal > 0 {
        log(format!(
            "{}: palette={:?}",
            tag,
            core::slice::from_raw_parts(pal, npal as usize)
        ));
    }

    let mut sb: *mut png_color_8 = ptr::null_mut();
    let r = (api.png_get_sBIT)(png, info, &mut sb);
    log(format!(
        "{}: sBIT r={} {:?}",
        tag,
        r,
        if sb.is_null() { None } else { Some(*sb) }
    ));

    let mut ta: *mut u8 = ptr::null_mut();
    let mut nt: c_int = -1;
    let mut tc: *mut png_color_16 = ptr::null_mut();
    let r = (api.png_get_tRNS)(png, info, &mut ta, &mut nt, &mut tc);
    log(format!(
        "{}: tRNS r={} n={} color={:?}",
        tag,
        r,
        nt,
        if tc.is_null() { None } else { Some(*tc) }
    ));
    if r != 0 && !ta.is_null() && nt > 0 {
        log(format!(
            "{}: trans_alpha={:02x?}",
            tag,
            core::slice::from_raw_parts(ta, nt as usize)
        ));
    }

    let mut bg: *mut png_color_16 = ptr::null_mut();
    let r = (api.png_get_bKGD)(png, info, &mut bg);
    log(format!(
        "{}: bKGD r={} {:?}",
        tag,
        r,
        if bg.is_null() { None } else { Some(*bg) }
    ));

    log(format!(
        "{}: w={} h={} bd={} ct={} il={} rows_null={}",
        tag,
        (api.png_get_image_width)(png, info),
        (api.png_get_image_height)(png, info),
        (api.png_get_bit_depth)(png, info),
        (api.png_get_color_type)(png, info),
        (api.png_get_interlace_type)(png, info),
        (api.png_get_rows)(png, info).is_null(),
    ));
}

/// Who owns `info_ptr->row_pointers` for a `png_read_png` call.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Rows {
    /// `png_set_rows` with the test's own zeroed buffers.
    Own,
    /// `png_read_png` allocates them with `png_malloc` (not zeroed).
    Lib,
}

/// Read back everything `png_get_rows` points at, appending it to `out`.
///
/// With `Rows::Lib` the last byte of a row whose pixel data does not fill a
/// whole number of bytes is blanked: `png_combine_row` copies those padding
/// bits straight out of the uninitialised `png_malloc`ed destination.
unsafe fn record_rows(
    api: &Api,
    png: *mut PngStruct,
    info: *mut PngInfo,
    who: Rows,
    out: &mut Vec<u8>,
) {
    let rp = (api.png_get_rows)(png, info);
    if rp.is_null() {
        log("rows: <null>".to_string());
        return;
    }
    let h = (api.png_get_image_height)(png, info) as usize;
    let w = (api.png_get_image_width)(png, info) as usize;
    let rb = (api.png_get_rowbytes)(png, info);
    let pd = (api.png_get_bit_depth)(png, info) as usize
        * (api.png_get_channels)(png, info) as usize;
    let tail_bits = (w * pd) % 8;
    log(format!(
        "rows: h={} rowbytes={} pixel_depth={} tail_bits={} owner={:?}",
        h, rb, pd, tail_bits, who
    ));
    for y in 0..h {
        let p = *rp.add(y);
        if p.is_null() {
            log(format!("row {}: <null>", y));
            continue;
        }
        let mut v = core::slice::from_raw_parts(p, rb).to_vec();
        if who == Rows::Lib && tail_bits != 0 && !v.is_empty() {
            let n = v.len();
            v[n - 1] = 0;
        }
        log(format!("row {}: {:02x?}", y, v));
        out.extend_from_slice(&v);
    }
}

/* ------------------------------------------------------------------ */
/* the two drivers                                                     */
/* ------------------------------------------------------------------ */

/// Rows the test owns.  `png_set_rows` stores the pointer, so this must stay
/// alive for as long as the `png_info` does.
struct OwnRows {
    _bufs: Vec<Vec<u8>>,
    ptrs: Vec<*mut u8>,
}

impl OwnRows {
    fn zeroed(h: usize, rowsize: usize) -> OwnRows {
        let mut bufs: Vec<Vec<u8>> = (0..h).map(|_| vec![0u8; rowsize]).collect();
        let ptrs = bufs.iter_mut().map(|b| b.as_mut_ptr()).collect();
        OwnRows { _bufs: bufs, ptrs }
    }
    fn filled(rows: &[Vec<u8>]) -> OwnRows {
        let mut bufs: Vec<Vec<u8>> = rows.to_vec();
        let ptrs = bufs.iter_mut().map(|b| b.as_mut_ptr()).collect();
        OwnRows { _bufs: bufs, ptrs }
    }
    fn as_ptr(&mut self) -> *mut *mut u8 {
        self.ptrs.as_mut_ptr()
    }
}

/// Any read transform can at most turn one pixel into 16-bit RGBA.
fn read_rowsize(w: u32) -> usize {
    w as usize * 8 + 16
}

/// Drive one complete `png_read_png` and record everything observable.
unsafe fn drive_read_png(
    api: &Api,
    data: &[u8],
    transforms: c_int,
    who: Rows,
    h: u32,
    w: u32,
    params_nonnull: bool,
) -> Outcome {
    let mut o = Outcome::default();
    tls().input = data.to_vec();
    tls().in_pos = 0;
    let (png, info) = new_read(api);
    (api.png_set_read_fn)(png, ptr::null_mut(), Some(read_cb));

    let mut own = if who == Rows::Own {
        Some(OwnRows::zeroed(h as usize, read_rowsize(w)))
    } else {
        None
    };
    let rows_ptr = own.as_mut().map(|r| r.as_ptr()).unwrap_or(ptr::null_mut());

    // `params` is documented as unused (`PNG_UNUSED(params)`, pngread.c:1068);
    // check it really is left alone.
    let mut params = [0xa5u8; 16];
    let pp = if params_nonnull {
        params.as_mut_ptr() as *mut c_void
    } else {
        ptr::null_mut()
    };

    let mut out: Vec<u8> = Vec::new();
    let g = guarded(api, png, &mut || {
        if who == Rows::Own {
            (api.png_set_rows)(png, info, rows_ptr);
            log(format!(
                "after set_rows: valid=0x{:x} rows_null={}",
                (api.png_get_valid)(png, info, 0xffffffff),
                (api.png_get_rows)(png, info).is_null()
            ));
        }
        (api.png_read_png)(png, info, transforms, pp);
        log_info(api, png, info, "after read_png");
        log_more(api, png, info, "after read_png");
        record_rows(api, png, info, who, &mut out);
    });
    log(format!("read_png guard={:?}", g));
    log(format!("params={:02x?}", params));
    destroy_read(api, png, info);
    drop(own);
    o.output = out;
    o
}

/// Everything needed to drive one `png_write_png`.
#[derive(Clone)]
struct WriteCase {
    w: u32,
    h: u32,
    ct: c_int,
    bd: c_int,
    il: c_int,
    palette: Vec<png_color>,
    rich: Option<Rich>,
    /// One buffer per row, each `read_rowsize(w)` bytes long so that no write
    /// transform (PACKING raises the user depth to 8, STRIP_FILLER raises the
    /// user channel count) can ever read past the end.
    rows: Vec<Vec<u8>>,
}

impl WriteCase {
    fn random(rng: &mut Rng, w: u32, h: u32, ct: c_int, bd: c_int, il: c_int, rich: bool) -> WriteCase {
        let palette = if ct == PNG_COLOR_TYPE_PALETTE {
            (0..(1usize << bd.min(8)))
                .map(|_| png_color {
                    red: rng.u8(),
                    green: rng.u8(),
                    blue: rng.u8(),
                })
                .collect()
        } else {
            Vec::new()
        };
        let r = if rich {
            Some(rich_for(rng, ct, bd, palette.len()))
        } else {
            None
        };
        let rowsize = read_rowsize(w);
        let rows = (0..h as usize).map(|_| rng.bytes(rowsize)).collect();
        WriteCase {
            w,
            h,
            ct,
            bd,
            il,
            palette,
            rich: r,
            rows,
        }
    }
}

/// Drive one complete `png_write_png` and record the produced file.
unsafe fn drive_write_png(
    api: &Api,
    wc: &WriteCase,
    transforms: c_int,
    set_rows: bool,
    params_nonnull: bool,
) -> Outcome {
    let mut o = Outcome::default();
    let (png, info) = new_write(api);
    (api.png_set_write_fn)(png, ptr::null_mut(), Some(write_cb), Some(flush_cb));

    let mut own = OwnRows::filled(&wc.rows);
    let rows_ptr = own.as_ptr();
    let mut params = [0x5au8; 16];
    let pp = if params_nonnull {
        params.as_mut_ptr() as *mut c_void
    } else {
        ptr::null_mut()
    };

    let g = guarded(api, png, &mut || {
        (api.png_set_IHDR)(
            png,
            info,
            wc.w,
            wc.h,
            wc.bd,
            wc.ct,
            wc.il,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
        if wc.ct == PNG_COLOR_TYPE_PALETTE && !wc.palette.is_empty() {
            (api.png_set_PLTE)(
                png,
                info,
                wc.palette.as_ptr(),
                wc.palette.len() as c_int,
            );
        }
        if let Some(r) = wc.rich.as_ref() {
            install_rich(api, png, info, r);
        }
        if set_rows {
            (api.png_set_rows)(png, info, rows_ptr);
        }
        log(format!(
            "before write_png: valid=0x{:x} rows_null={}",
            (api.png_get_valid)(png, info, 0xffffffff),
            (api.png_get_rows)(png, info).is_null()
        ));
        (api.png_write_png)(png, info, transforms, pp);
        log_info(api, png, info, "after write_png");
        log_more(api, png, info, "after write_png");
    });
    log(format!(
        "write_png guard={:?} flushes={}",
        g,
        tls().flushes
    ));
    log(format!("params={:02x?}", params));
    o.output = std::mem::take(&mut tls().output);
    destroy_write(api, png, info);
    drop(own);
    o
}

/* ------------------------------------------------------------------ */
/* source images                                                       */
/* ------------------------------------------------------------------ */

/// Build a PNG datastream with the *low level* writer, checking both libraries
/// produce the same bytes, and return the C library's version of it.
fn build_source(n: &mut usize, case: &str, img: &Img, rich: Option<&Rich>) -> Vec<u8> {
    let mut file: Vec<u8> = Vec::new();
    *n += 1;
    assert_same(case, |api| unsafe {
        let mut o = Outcome::default();
        let wr = write_image(api, img, &WriteOpts::default(), &mut |api, png, info| {
            if let Some(r) = rich {
                install_rich(api, png, info, r);
            }
        });
        o.push(format!("guard={:?}", wr.guard));
        o.output = wr.bytes.clone();
        if api.which == "C" {
            file = wr.bytes.clone();
        }
        o
    });
    assert!(!file.is_empty(), "{}: source writer produced nothing", case);
    file
}

/// One source file per (shape, interlace, size).
struct Source {
    img: Img,
    rich: Option<Rich>,
    bytes: Vec<u8>,
}

/// `(width, height, with sBIT+tRNS)`.
///
/// The widths deliberately mix both cases of the `png_combine_row` tail:
/// 8/16/32 divide every sub-byte pixel depth evenly (no padding bits, every
/// byte comparable even with `Rows::Lib`), while 1/7/13/23/37 leave a partial
/// last byte.  Both are paired with a plain and a rich source.
const SIZES: [(u32, u32, bool); 8] = [
    (13, 5, false),
    (8, 3, true),
    (1, 1, false),
    (7, 2, true),
    (16, 4, false),
    (23, 9, true),
    (32, 6, false),
    (37, 17, true),
];

fn sources(n: &mut usize) -> Vec<Source> {
    let mut v = Vec::new();
    for (ct, bd) in VALID_SHAPES {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for (si, &(w, h, rich)) in SIZES.iter().enumerate() {
                let mut rng = Rng::new(
                    0x131_0000
                        ^ ((ct as u64) << 40)
                        ^ ((bd as u64) << 32)
                        ^ ((il as u64) << 24)
                        ^ ((si as u64) << 16),
                );
                let mut img = Img::random(&mut rng, w, h, ct, bd);
                img.interlace = il;
                let r = if rich {
                    Some(rich_for(&mut rng, ct, bd, img.palette.len()))
                } else {
                    None
                };
                let case = format!(
                    "source ct={} bd={} il={} rich={} {}x{}",
                    ct, bd, il, rich, w, h
                );
                let bytes = build_source(n, &case, &img, r.as_ref());
                v.push(Source {
                    img,
                    rich: r,
                    bytes,
                });
            }
        }
    }
    v
}

/* ------------------------------------------------------------------ */
/* C-131: png_read_png                                                 */
/* ------------------------------------------------------------------ */

#[test]
fn read_png() {
    let mut n = 0usize;
    let mut nf = 0usize;
    let srcs = sources(&mut n);

    /* --- every read-legal flag on its own, over every shape ------------- */
    let mut single: Vec<c_int> = vec![PNG_TRANSFORM_IDENTITY];
    single.extend_from_slice(&READ_FLAGS);
    for s in &srcs {
        for &t in &single {
            for who in [Rows::Own, Rows::Lib] {
                let case = format!(
                    "read_png {} ct={} bd={} il={} rich={} rows={:?}",
                    tname(t),
                    s.img.color_type,
                    s.img.bit_depth,
                    s.img.interlace,
                    s.rich.is_some(),
                    who
                );
                n += 1;
                assert_same(&case, |api| unsafe {
                    drive_read_png(
                        api,
                        &s.bytes,
                        t,
                        who,
                        s.img.h,
                        s.img.w,
                        t & 1 != 0,
                    )
                });
            }
        }
    }

    /* --- all read flags at once, plus undefined bits -------------------- */
    let all_read: c_int = READ_FLAGS.iter().fold(0, |a, &f| a | f);
    for s in &srcs {
        for &t in &[all_read, 0xffff, 0x7fff_0000u32 as c_int | all_read, -1] {
            let case = format!(
                "read_png mass 0x{:x} ct={} bd={} il={} rich={}",
                t, s.img.color_type, s.img.bit_depth, s.img.interlace, s.rich.is_some()
            );
            n += 1;
            assert_same(&case, |api| unsafe {
                drive_read_png(api, &s.bytes, t, Rows::Own, s.img.h, s.img.w, true)
            });
        }
    }

    /* --- several hundred random 2..6 flag combinations ------------------ */
    let mut rng = Rng::new(0xc131_c001);
    for i in 0..1200 {
        let s = &srcs[rng.below(srcs.len())];
        let t = random_combo(&mut rng, &READ_FLAGS);
        let who = if rng.bool() { Rows::Own } else { Rows::Lib };
        let pn = rng.bool();
        let case = format!(
            "read_png combo#{} {} ct={} bd={} il={} rich={} rows={:?}",
            i,
            tname(t),
            s.img.color_type,
            s.img.bit_depth,
            s.img.interlace,
            s.rich.is_some(),
            who
        );
        n += 1;
        assert_same(&case, |api| unsafe {
            drive_read_png(api, &s.bytes, t, who, s.img.h, s.img.w, pn)
        });
    }

    /* --- the write-only STRIP_FILLER flags: png_read_png has no code for
     *     them, so they must be silently ignored --------------------------- */
    for s in &srcs {
        for &t in &WRITE_ONLY_FLAGS {
            for &extra in &[0, PNG_TRANSFORM_EXPAND, PNG_TRANSFORM_STRIP_ALPHA] {
                let case = format!(
                    "read_png write-only {}|{} ct={} bd={} il={}",
                    tname(t),
                    tname(extra),
                    s.img.color_type,
                    s.img.bit_depth,
                    s.img.interlace
                );
                n += 1;
                assert_same(&case, |api| unsafe {
                    drive_read_png(
                        api,
                        &s.bytes,
                        t | extra,
                        Rows::Own,
                        s.img.h,
                        s.img.w,
                        false,
                    )
                });
            }
        }
    }

    /* --- params NULL vs non-NULL, back to back ------------------------- */
    for s in srcs.iter().take(6) {
        for &t in &[PNG_TRANSFORM_IDENTITY, PNG_TRANSFORM_EXPAND] {
            for params_nonnull in [false, true] {
                let case = format!(
                    "read_png params={} {} ct={} bd={}",
                    params_nonnull,
                    tname(t),
                    s.img.color_type,
                    s.img.bit_depth
                );
                n += 1;
                assert_same(&case, |api| unsafe {
                    drive_read_png(
                        api,
                        &s.bytes,
                        t,
                        Rows::Own,
                        s.img.h,
                        s.img.w,
                        params_nonnull,
                    )
                });
            }
        }
    }

    /* --- NULL png_ptr / NULL info_ptr: both must just return ----------- */
    n += 1;
    assert_same("read_png NULL arguments", |api| unsafe {
        let mut o = Outcome::default();
        tls().input = srcs[0].bytes.clone();
        tls().in_pos = 0;
        let (png, info) = new_read(api);
        (api.png_set_read_fn)(png, ptr::null_mut(), Some(read_cb));
        let g = guarded(api, png, &mut || {
            (api.png_read_png)(ptr::null_mut(), info, PNG_TRANSFORM_EXPAND, ptr::null_mut());
            log("read_png(NULL, info) returned".to_string());
            (api.png_read_png)(png, ptr::null_mut(), PNG_TRANSFORM_EXPAND, ptr::null_mut());
            log("read_png(png, NULL) returned".to_string());
            (api.png_read_png)(ptr::null_mut(), ptr::null_mut(), 0, ptr::null_mut());
            log("read_png(NULL, NULL) returned".to_string());
            log_info(api, png, info, "untouched");
            log_more(api, png, info, "untouched");
        });
        log(format!("guard={:?}", g));
        destroy_read(api, png, info);
        o.push("done".to_string());
        o
    });

    /* --- "Image is too high to process with png_read_png()" ------------- */
    for (h, raise) in [
        (0x2000_0000u32, true),
        (0x7fff_ffffu32, true),
        (1_000_001u32, false),
        (0u32, false),
    ] {
        let case = format!("read_png height=0x{:x} raise_limits={}", h, raise);
        n += 1;
        assert_same(&case, |api| unsafe {
            let mut o = Outcome::default();
            tls().input = tall_png(h);
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, ptr::null_mut(), Some(read_cb));
            let g = guarded(api, png, &mut || {
                if raise {
                    (api.png_set_user_limits)(png, 0x7fff_ffff, 0x7fff_ffff);
                }
                log(format!(
                    "limits w={} h={}",
                    (api.png_get_user_width_max)(png),
                    (api.png_get_user_height_max)(png)
                ));
                (api.png_read_png)(png, info, PNG_TRANSFORM_IDENTITY, ptr::null_mut());
                log_info(api, png, info, "after read_png");
            });
            log(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o.push("done".to_string());
            o
        });
    }

    /* --- a truncated datastream ---------------------------------------- */
    for cut in [8usize, 20, 33, 40] {
        let s = &srcs[0];
        if cut >= s.bytes.len() {
            continue;
        }
        let short = s.bytes[..cut].to_vec();
        let case = format!("read_png truncated at {}", cut);
        n += 1;
        assert_same(&case, |api| unsafe {
            drive_read_png(api, &short, PNG_TRANSFORM_EXPAND, Rows::Own, s.img.h, s.img.w, false)
        });
    }

    /* --- the exact allocation sequence.  `png_read_png` mallocs the row
     *     pointer array (`height * sizeof(png_bytep)`) and then one buffer of
     *     `info_ptr->rowbytes` per row (pngread.c:1049..1059); a logging
     *     allocator compares every size and the order they come in. ---------- */
    for s in &srcs {
        for &t in &[
            PNG_TRANSFORM_IDENTITY,
            PNG_TRANSFORM_EXPAND,
            PNG_TRANSFORM_EXPAND_16,
            PNG_TRANSFORM_GRAY_TO_RGB,
            PNG_TRANSFORM_PACKING,
            PNG_TRANSFORM_STRIP_ALPHA,
        ] {
            let case = format!(
                "read_png allocs {} ct={} bd={} il={} {}x{}",
                tname(t),
                s.img.color_type,
                s.img.bit_depth,
                s.img.interlace,
                s.img.w,
                s.img.h
            );
            n += 1;
            assert_same(&case, |api| unsafe {
                let mut o = Outcome::default();
                tls().input = s.bytes.clone();
                tls().in_pos = 0;
                let (png, info) = new_read(api);
                (api.png_set_mem_fn)(png, ptr::null_mut(), Some(malloc_cb), Some(free_cb));
                (api.png_set_read_fn)(png, ptr::null_mut(), Some(read_cb));
                let mut out: Vec<u8> = Vec::new();
                let g = guarded(api, png, &mut || {
                    (api.png_read_png)(png, info, t, ptr::null_mut());
                    log_info(api, png, info, "allocs");
                    record_rows(api, png, info, Rows::Lib, &mut out);
                });
                log(format!(
                    "guard={:?} allocs={} frees={}",
                    g,
                    tls().allocs.len(),
                    tls().counter
                ));
                destroy_read(api, png, info);
                log(format!("after destroy frees={}", tls().counter));
                o.output = out;
                o
            });
        }
    }

    /* --- png_set_rows(rows) then png_set_rows(NULL): PNG_INFO_IDAT stays set
     *     but row_pointers is NULL again, so png_read_png takes the malloc
     *     branch with a non-empty `valid` mask (pngset.c:1785..1792) -------- */
    for s in srcs.iter().step_by(7) {
        let case = format!(
            "read_png set_rows then NULL ct={} bd={} il={} {}x{}",
            s.img.color_type, s.img.bit_depth, s.img.interlace, s.img.w, s.img.h
        );
        n += 1;
        assert_same(&case, |api| unsafe {
            let mut o = Outcome::default();
            tls().input = s.bytes.clone();
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, ptr::null_mut(), Some(read_cb));
            let mut rows = OwnRows::zeroed(s.img.h as usize, read_rowsize(s.img.w));
            let rp = rows.as_ptr();
            let mut out: Vec<u8> = Vec::new();
            let g = guarded(api, png, &mut || {
                (api.png_set_rows)(png, info, rp);
                log(format!(
                    "valid=0x{:x}",
                    (api.png_get_valid)(png, info, 0xffffffff)
                ));
                (api.png_set_rows)(png, info, ptr::null_mut());
                log(format!(
                    "valid=0x{:x} rows_null={}",
                    (api.png_get_valid)(png, info, 0xffffffff),
                    (api.png_get_rows)(png, info).is_null()
                ));
                (api.png_read_png)(png, info, PNG_TRANSFORM_EXPAND, ptr::null_mut());
                log_info(api, png, info, "after read_png");
                log_more(api, png, info, "after read_png");
                record_rows(api, png, info, Rows::Lib, &mut out);
            });
            log(format!("guard={:?}", g));
            destroy_read(api, png, info);
            drop(rows);
            o.output = out;
            o
        });
    }

    /* --- png_read_png twice on the same structs, with the input rewound.
     *     The second call re-enters png_read_info past IEND and must fail the
     *     same way in both libraries; it also exercises the
     *     png_free_data(PNG_FREE_ROWS) branch that really does free, because
     *     the first call left PNG_FREE_ROWS in info_ptr->free_me. ----------- */
    for s in srcs.iter().step_by(11) {
        let case = format!(
            "read_png twice ct={} bd={} il={} {}x{}",
            s.img.color_type, s.img.bit_depth, s.img.interlace, s.img.w, s.img.h
        );
        n += 1;
        assert_same(&case, |api| unsafe {
            let mut o = Outcome::default();
            tls().input = s.bytes.clone();
            tls().in_pos = 0;
            let (png, info) = new_read(api);
            (api.png_set_read_fn)(png, ptr::null_mut(), Some(read_cb));
            let mut out: Vec<u8> = Vec::new();
            let g = guarded(api, png, &mut || {
                (api.png_read_png)(png, info, PNG_TRANSFORM_IDENTITY, ptr::null_mut());
                log_info(api, png, info, "first");
                record_rows(api, png, info, Rows::Lib, &mut out);
                tls().in_pos = 0;
                (api.png_read_png)(png, info, PNG_TRANSFORM_EXPAND, ptr::null_mut());
                log_info(api, png, info, "second");
                log_more(api, png, info, "second");
                record_rows(api, png, info, Rows::Lib, &mut out);
            });
            log(format!("guard={:?}", g));
            destroy_read(api, png, info);
            o.output = out;
            o
        });
    }

    /* --- rows the app supplied are NULL: png_read_png keeps them and
     *     png_read_image dereferences them -------------------------------- */
    {
        let s = &srcs[0];
        let bytes = s.bytes.clone();
        let h = s.img.h;
        nf += 1;
        assert_same_forked("read_png with NULL row pointers", move |api| unsafe {
            tls().input = bytes.clone();
            tls().in_pos = 0;
            let mut rows: Vec<*mut u8> = vec![ptr::null_mut(); h as usize];
            let rp = rows.as_mut_ptr();
            guarded_in_child(api, false, &mut |api, png, info| {
                (api.png_set_read_fn)(png, ptr::null_mut(), Some(read_cb));
                (api.png_set_rows)(png, info, rp);
                (api.png_read_png)(png, info, PNG_TRANSFORM_IDENTITY, ptr::null_mut());
            })
        });
    }

    eprintln!(
        "[highlevel::read_png] {} assert_same + {} assert_same_forked comparisons",
        n, nf
    );
}

/// A datastream whose IHDR declares `h` rows; `png_read_info` stops at IDAT so
/// nothing is ever decoded.
fn tall_png(h: u32) -> Vec<u8> {
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&1u32.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
    let mut v = SIG.to_vec();
    v.extend_from_slice(&chunk(b"IHDR", &ihdr));
    v.extend_from_slice(&chunk(b"IDAT", &[0x78, 0x01, 0x00]));
    v.extend_from_slice(&chunk(b"IEND", &[]));
    v
}

/* ------------------------------------------------------------------ */
/* C-132: png_write_png                                                */
/* ------------------------------------------------------------------ */

#[test]
fn write_png() {
    let mut n = 0usize;
    let mut nf = 0usize;

    let mut cases: Vec<WriteCase> = Vec::new();
    for (ct, bd) in VALID_SHAPES {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            for (si, &(w, h, rich)) in SIZES.iter().enumerate() {
                let mut rng = Rng::new(
                    0x132_0000
                        ^ ((ct as u64) << 40)
                        ^ ((bd as u64) << 32)
                        ^ ((il as u64) << 24)
                        ^ ((si as u64) << 16),
                );
                cases.push(WriteCase::random(&mut rng, w, h, ct, bd, il, rich));
            }
        }
    }

    /* --- every write-legal flag on its own, over every shape ------------ */
    let mut single: Vec<c_int> = vec![PNG_TRANSFORM_IDENTITY];
    single.extend_from_slice(&WRITE_FLAGS);
    for wc in &cases {
        for &t in &single {
            let case = format!(
                "write_png {} ct={} bd={} il={} rich={}",
                tname(t),
                wc.ct,
                wc.bd,
                wc.il,
                wc.rich.is_some()
            );
            n += 1;
            assert_same(&case, |api| unsafe {
                drive_write_png(api, wc, t, true, t & 1 != 0)
            });
        }
    }

    /* --- BEFORE+AFTER together: png_app_error (pngwrite.c:1472) --------- */
    for wc in &cases {
        let t = PNG_TRANSFORM_STRIP_FILLER_BEFORE | PNG_TRANSFORM_STRIP_FILLER_AFTER;
        let case = format!(
            "write_png filler both ct={} bd={} il={} rich={}",
            wc.ct, wc.bd, wc.il, wc.rich.is_some()
        );
        n += 1;
        assert_same(&case, |api| unsafe {
            drive_write_png(api, wc, t, true, false)
        });
    }

    /* --- the read-only flags: png_write_png has no code for them -------- */
    for wc in &cases {
        for &t in &READ_ONLY_FLAGS {
            let case = format!(
                "write_png read-only {} ct={} bd={} il={} rich={}",
                tname(t),
                wc.ct,
                wc.bd,
                wc.il,
                wc.rich.is_some()
            );
            n += 1;
            assert_same(&case, |api| unsafe {
                drive_write_png(api, wc, t, true, false)
            });
        }
    }
    let all_read_only: c_int = READ_ONLY_FLAGS.iter().fold(0, |a, &f| a | f);
    for wc in &cases {
        for &t in &[all_read_only, 0xffff, -1] {
            let case = format!(
                "write_png mass 0x{:x} ct={} bd={} il={} rich={}",
                t, wc.ct, wc.bd, wc.il, wc.rich.is_some()
            );
            n += 1;
            assert_same(&case, |api| unsafe {
                drive_write_png(api, wc, t, true, true)
            });
        }
    }

    /* --- several hundred random 2..6 flag combinations ------------------ */
    let mut rng = Rng::new(0x132_c0de);
    for i in 0..800 {
        let wc = &cases[rng.below(cases.len())];
        let t = random_combo(&mut rng, &WRITE_FLAGS);
        let pn = rng.bool();
        let case = format!(
            "write_png combo#{} {} ct={} bd={} il={} rich={}",
            i,
            tname(t),
            wc.ct,
            wc.bd,
            wc.il,
            wc.rich.is_some()
        );
        n += 1;
        assert_same(&case, |api| unsafe { drive_write_png(api, wc, t, true, pn) });
    }
    // ... and combinations drawn from *all* 16 flags, legal or not.
    let every: Vec<c_int> = ALL_FLAGS.iter().map(|&(_, f)| f).collect();
    for i in 0..400 {
        let wc = &cases[rng.below(cases.len())];
        let t = random_combo(&mut rng, &every);
        let case = format!(
            "write_png anycombo#{} {} ct={} bd={} il={} rich={}",
            i,
            tname(t),
            wc.ct,
            wc.bd,
            wc.il,
            wc.rich.is_some()
        );
        n += 1;
        assert_same(&case, |api| unsafe {
            drive_write_png(api, wc, t, true, false)
        });
    }

    /* --- no png_set_rows at all: "no rows for png_write_image to write" - */
    for wc in cases.iter().take(8) {
        let case = format!("write_png without rows ct={} bd={}", wc.ct, wc.bd);
        n += 1;
        assert_same(&case, |api| unsafe {
            drive_write_png(api, wc, PNG_TRANSFORM_IDENTITY, false, false)
        });
    }

    /* --- the exact allocation sequence of a complete high-level write ---- */
    for wc in &cases {
        for &t in &[
            PNG_TRANSFORM_IDENTITY,
            PNG_TRANSFORM_PACKING,
            PNG_TRANSFORM_STRIP_FILLER_AFTER,
            PNG_TRANSFORM_BGR,
        ] {
            let case = format!(
                "write_png allocs {} ct={} bd={} il={} {}x{}",
                tname(t),
                wc.ct,
                wc.bd,
                wc.il,
                wc.w,
                wc.h
            );
            n += 1;
            assert_same(&case, |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_write(api);
                (api.png_set_mem_fn)(png, ptr::null_mut(), Some(malloc_cb), Some(free_cb));
                (api.png_set_write_fn)(png, ptr::null_mut(), Some(write_cb), Some(flush_cb));
                let mut own = OwnRows::filled(&wc.rows);
                let rp = own.as_ptr();
                let g = guarded(api, png, &mut || {
                    (api.png_set_IHDR)(
                        png,
                        info,
                        wc.w,
                        wc.h,
                        wc.bd,
                        wc.ct,
                        wc.il,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    if wc.ct == PNG_COLOR_TYPE_PALETTE && !wc.palette.is_empty() {
                        (api.png_set_PLTE)(
                            png,
                            info,
                            wc.palette.as_ptr(),
                            wc.palette.len() as c_int,
                        );
                    }
                    if let Some(r) = wc.rich.as_ref() {
                        install_rich(api, png, info, r);
                    }
                    (api.png_set_rows)(png, info, rp);
                    (api.png_write_png)(png, info, t, ptr::null_mut());
                });
                log(format!(
                    "guard={:?} allocs={} frees={}",
                    g,
                    tls().allocs.len(),
                    tls().counter
                ));
                o.output = std::mem::take(&mut tls().output);
                destroy_write(api, png, info);
                log(format!("after destroy frees={}", tls().counter));
                drop(own);
                o
            });
        }
    }

    /* --- png_write_info called by the application first, so png_write_png
     *     writes a second copy of the header (pngwrite.c:1422) ------------- */
    for wc in cases.iter().step_by(5) {
        for &t in &[PNG_TRANSFORM_IDENTITY, PNG_TRANSFORM_BGR] {
            let case = format!(
                "write_png after write_info {} ct={} bd={} il={}",
                tname(t),
                wc.ct,
                wc.bd,
                wc.il
            );
            n += 1;
            assert_same(&case, |api| unsafe {
                let mut o = Outcome::default();
                let (png, info) = new_write(api);
                (api.png_set_write_fn)(png, ptr::null_mut(), Some(write_cb), Some(flush_cb));
                let mut own = OwnRows::filled(&wc.rows);
                let rp = own.as_ptr();
                let g = guarded(api, png, &mut || {
                    (api.png_set_IHDR)(
                        png,
                        info,
                        wc.w,
                        wc.h,
                        wc.bd,
                        wc.ct,
                        wc.il,
                        PNG_COMPRESSION_TYPE_BASE,
                        PNG_FILTER_TYPE_BASE,
                    );
                    if wc.ct == PNG_COLOR_TYPE_PALETTE && !wc.palette.is_empty() {
                        (api.png_set_PLTE)(
                            png,
                            info,
                            wc.palette.as_ptr(),
                            wc.palette.len() as c_int,
                        );
                    }
                    (api.png_set_rows)(png, info, rp);
                    (api.png_write_info)(png, info);
                    log(format!("after write_info out={}", tls().output.len()));
                    (api.png_write_png)(png, info, t, ptr::null_mut());
                    log_info(api, png, info, "after write_png");
                });
                log(format!("guard={:?}", g));
                o.output = std::mem::take(&mut tls().output);
                destroy_write(api, png, info);
                drop(own);
                o
            });
        }
    }

    /* --- png_write_png twice on the same structs ------------------------- */
    for wc in cases.iter().step_by(9) {
        let case = format!("write_png twice ct={} bd={} il={}", wc.ct, wc.bd, wc.il);
        n += 1;
        assert_same(&case, |api| unsafe {
            let mut o = Outcome::default();
            let (png, info) = new_write(api);
            (api.png_set_write_fn)(png, ptr::null_mut(), Some(write_cb), Some(flush_cb));
            let mut own = OwnRows::filled(&wc.rows);
            let rp = own.as_ptr();
            let g = guarded(api, png, &mut || {
                (api.png_set_IHDR)(
                    png,
                    info,
                    wc.w,
                    wc.h,
                    wc.bd,
                    wc.ct,
                    wc.il,
                    PNG_COMPRESSION_TYPE_BASE,
                    PNG_FILTER_TYPE_BASE,
                );
                if wc.ct == PNG_COLOR_TYPE_PALETTE && !wc.palette.is_empty() {
                    (api.png_set_PLTE)(png, info, wc.palette.as_ptr(), wc.palette.len() as c_int);
                }
                (api.png_set_rows)(png, info, rp);
                (api.png_write_png)(png, info, PNG_TRANSFORM_IDENTITY, ptr::null_mut());
                log(format!("first write done out={}", tls().output.len()));
                (api.png_write_png)(png, info, PNG_TRANSFORM_IDENTITY, ptr::null_mut());
                log(format!("second write done out={}", tls().output.len()));
            });
            log(format!("guard={:?}", g));
            o.output = std::mem::take(&mut tls().output);
            destroy_write(api, png, info);
            drop(own);
            o
        });
    }

    /* --- NULL png_ptr / NULL info_ptr: both must just return ----------- */
    n += 1;
    assert_same("write_png NULL arguments", |api| unsafe {
        let mut o = Outcome::default();
        let (png, info) = new_write(api);
        (api.png_set_write_fn)(png, ptr::null_mut(), Some(write_cb), Some(flush_cb));
        let g = guarded(api, png, &mut || {
            (api.png_write_png)(ptr::null_mut(), info, 0, ptr::null_mut());
            log("write_png(NULL, info) returned".to_string());
            (api.png_write_png)(png, ptr::null_mut(), 0, ptr::null_mut());
            log("write_png(png, NULL) returned".to_string());
            (api.png_write_png)(ptr::null_mut(), ptr::null_mut(), 0, ptr::null_mut());
            log("write_png(NULL, NULL) returned".to_string());
        });
        log(format!("guard={:?} out={}", g, tls().output.len()));
        o.output = std::mem::take(&mut tls().output);
        destroy_write(api, png, info);
        o.push("done".to_string());
        o
    });

    /* --- PNG_INFO_IDAT set but row_pointers NULL: png_write_image
     *     dereferences NULL (pngwrite.c:1521) ------------------------------ */
    nf += 1;
    assert_same_forked("write_png with NULL row_pointers", |api| unsafe {
        guarded_in_child(api, true, &mut |api, png, info| {
            (api.png_set_write_fn)(png, ptr::null_mut(), Some(write_cb), Some(flush_cb));
            (api.png_set_IHDR)(
                png,
                info,
                4,
                4,
                8,
                PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            let mut rows: Vec<*mut u8> = vec![ptr::null_mut(); 4];
            // Sets PNG_INFO_IDAT ...
            (api.png_set_rows)(png, info, rows.as_mut_ptr());
            // ... which png_set_rows(NULL) does *not* clear (pngset.c:1789).
            (api.png_set_rows)(png, info, ptr::null_mut());
            (api.png_write_png)(png, info, 0, ptr::null_mut());
        })
    });

    /* --- rows present but every entry NULL ----------------------------- */
    nf += 1;
    assert_same_forked("write_png with NULL rows", |api| unsafe {
        let mut rows: Vec<*mut u8> = vec![ptr::null_mut(); 4];
        let rp = rows.as_mut_ptr();
        guarded_in_child(api, true, &mut |api, png, info| {
            (api.png_set_write_fn)(png, ptr::null_mut(), Some(write_cb), Some(flush_cb));
            (api.png_set_IHDR)(
                png,
                info,
                4,
                4,
                8,
                PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            (api.png_set_rows)(png, info, rp);
            (api.png_write_png)(png, info, 0, ptr::null_mut());
        })
    });

    eprintln!(
        "[highlevel::write_png] {} assert_same + {} assert_same_forked comparisons",
        n, nf
    );
}

/* ------------------------------------------------------------------ */
/* C-133: png_read_png + png_write_png round trip                      */
/* ------------------------------------------------------------------ */

/// `png_read_png` with `t_read`, then `png_write_png` with `t_write` on the
/// decoded rows.  The rewritten bytes are the comparison.
unsafe fn drive_round_trip(
    api: &Api,
    data: &[u8],
    t_read: c_int,
    t_write: c_int,
    il_out: c_int,
    w: u32,
    h: u32,
) -> Outcome {
    let mut o = Outcome::default();

    /* ---------------- read ---------------- */
    tls().input = data.to_vec();
    tls().in_pos = 0;
    let (rpng, rinfo) = new_read(api);
    (api.png_set_read_fn)(rpng, ptr::null_mut(), Some(read_cb));
    let rowsize = read_rowsize(w);
    let mut inrows = OwnRows::zeroed(h as usize, rowsize);
    let inptr = inrows.as_ptr();

    let mut shape: Option<(c_int, c_int, usize)> = None;
    let mut palette: Vec<png_color> = Vec::new();
    let mut sbit: Option<png_color_8> = None;
    let mut decoded: Vec<Vec<u8>> = Vec::new();

    let g1 = guarded(api, rpng, &mut || {
        (api.png_set_rows)(rpng, rinfo, inptr);
        (api.png_read_png)(rpng, rinfo, t_read, ptr::null_mut());
        log_info(api, rpng, rinfo, "rt read");
        log_more(api, rpng, rinfo, "rt read");
        let ct = (api.png_get_color_type)(rpng, rinfo) as c_int;
        let bd = (api.png_get_bit_depth)(rpng, rinfo) as c_int;
        let rb = (api.png_get_rowbytes)(rpng, rinfo);
        shape = Some((ct, bd, rb));
        let mut p: *mut png_color = ptr::null_mut();
        let mut np: c_int = 0;
        if (api.png_get_PLTE)(rpng, rinfo, &mut p, &mut np) != 0 && !p.is_null() && np > 0 {
            palette = core::slice::from_raw_parts(p, np as usize).to_vec();
        }
        let mut sb: *mut png_color_8 = ptr::null_mut();
        if (api.png_get_sBIT)(rpng, rinfo, &mut sb) != 0 && !sb.is_null() {
            sbit = Some(*sb);
        }
        let rp = (api.png_get_rows)(rpng, rinfo);
        if !rp.is_null() {
            for y in 0..h as usize {
                let src = *rp.add(y);
                let mut v = vec![0u8; rowsize];
                if !src.is_null() {
                    v[..rb].copy_from_slice(core::slice::from_raw_parts(src, rb));
                }
                log(format!("rt row {}: {:02x?}", y, &v[..rb]));
                decoded.push(v);
            }
        }
    });
    log(format!("rt read guard={:?}", g1));
    destroy_read(api, rpng, rinfo);
    drop(inrows);

    let Some((ct, bd, rb)) = shape else {
        o.push("rt: read failed, no write".to_string());
        return o;
    };
    if decoded.len() != h as usize {
        o.push(format!("rt: only {} rows decoded", decoded.len()));
        return o;
    }
    log(format!(
        "rt shape after read: ct={} bd={} rowbytes={} il_out={} palette={}",
        ct,
        bd,
        rb,
        il_out,
        palette.len()
    ));

    /* ---------------- write ---------------- */
    let wc = WriteCase {
        w,
        h,
        ct,
        bd,
        il: il_out,
        palette,
        rich: sbit.map(|s| Rich {
            sbit: s,
            trns: Trns::None,
        }),
        rows: decoded,
    };
    let wo = drive_write_png(api, &wc, t_write, true, false);
    o.output = wo.output;
    o.trace.extend(wo.trace);
    o
}

#[test]
fn round_trip() {
    let mut n = 0usize;
    let srcs = sources(&mut n);

    /* --- the same transform on the way in and out, over every shape ----- */
    let common_both: [c_int; 8] = [
        PNG_TRANSFORM_IDENTITY,
        PNG_TRANSFORM_PACKING,
        PNG_TRANSFORM_PACKSWAP,
        PNG_TRANSFORM_INVERT_MONO,
        PNG_TRANSFORM_SHIFT,
        PNG_TRANSFORM_BGR,
        PNG_TRANSFORM_SWAP_ALPHA,
        PNG_TRANSFORM_SWAP_ENDIAN,
    ];
    for s in &srcs {
        for &t in &common_both {
            let case = format!(
                "round_trip T=T'={} ct={} bd={} il={} rich={}",
                tname(t),
                s.img.color_type,
                s.img.bit_depth,
                s.img.interlace,
                s.rich.is_some()
            );
            n += 1;
            assert_same(&case, |api| unsafe {
                drive_round_trip(api, &s.bytes, t, t, s.img.interlace, s.img.w, s.img.h)
            });
        }
    }

    /* --- random (T, T') pairs over random shapes ------------------------ */
    let mut rng = Rng::new(0x133_5eed);
    for i in 0..900 {
        let s = &srcs[rng.below(srcs.len())];
        let t_read = if rng.bool() {
            READ_FLAGS[rng.below(READ_FLAGS.len())]
        } else {
            random_combo(&mut rng, &READ_FLAGS)
        };
        let t_write = if rng.bool() {
            WRITE_FLAGS[rng.below(WRITE_FLAGS.len())]
        } else {
            random_combo(&mut rng, &WRITE_FLAGS)
        };
        let il_out = if rng.bool() {
            s.img.interlace
        } else {
            1 - s.img.interlace
        };
        let case = format!(
            "round_trip#{} T={} T'={} ct={} bd={} il={}->{} rich={}",
            i,
            tname(t_read),
            tname(t_write),
            s.img.color_type,
            s.img.bit_depth,
            s.img.interlace,
            il_out,
            s.rich.is_some()
        );
        n += 1;
        assert_same(&case, |api| unsafe {
            drive_round_trip(api, &s.bytes, t_read, t_write, il_out, s.img.w, s.img.h)
        });
    }

    /* --- and the two "everything" masks -------------------------------- */
    let all_read: c_int = READ_FLAGS.iter().fold(0, |a, &f| a | f);
    let all_write: c_int = WRITE_FLAGS.iter().fold(0, |a, &f| a | f);
    for s in &srcs {
        for &(tr, tw) in &[
            (all_read, PNG_TRANSFORM_IDENTITY),
            (PNG_TRANSFORM_IDENTITY, all_write),
            (all_read, all_write),
            (0xffff, 0xffff),
        ] {
            let case = format!(
                "round_trip mass 0x{:x}/0x{:x} ct={} bd={} il={} rich={}",
                tr, tw, s.img.color_type, s.img.bit_depth, s.img.interlace, s.rich.is_some()
            );
            n += 1;
            assert_same(&case, |api| unsafe {
                drive_round_trip(api, &s.bytes, tr, tw, s.img.interlace, s.img.w, s.img.h)
            });
        }
    }

    eprintln!("[highlevel::round_trip] {} assert_same comparisons", n);
}

