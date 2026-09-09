//! Phase B / write path — every case drives the *write* side of libpng and
//! records the produced PNG bytes plus any info-struct round trip, so the C and
//! Rust encoders are compared byte-for-byte. These are all valid-input cases:
//! none should ever trigger a `png_error`; if one does, that is a real finding.

use crate::api::Api;
#[allow(unused_imports)]
use crate::p;
use crate::support::*;
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

pub fn run(api: &Api, case: &str, seed: u64) -> bool {
    let mut rng = Rng::new(seed);
    match case {
        "wr/chunk_raw" => chunk_raw(api, &mut rng),
        "wr/low_all_shapes" => low_all_shapes(api, &mut rng),
        "wr/low_interlaced" => low_interlaced(api, &mut rng),
        "wr/rows_api" => rows_api(api, &mut rng),
        "wr/image_api" => image_api(api, &mut rng, false),
        "wr/image_api_interlaced" => image_api(api, &mut rng, true),
        "wr/filters_single" => filters_single(api, &mut rng),
        "wr/filters_all" => filters_all(api, &mut rng),
        "wr/filters_pairs" => filters_pairs(api, &mut rng),
        "wr/compression_level" => compression_level(api, &mut rng),
        "wr/compression_strategy" => compression_strategy(api, &mut rng),
        "wr/compression_memlevel" => compression_memlevel(api, &mut rng),
        "wr/compression_winbits" => compression_winbits(api, &mut rng),
        "wr/compression_bufsize" => compression_bufsize(api, &mut rng),
        "wr/flush" => flush(api, &mut rng),
        "wr/status_fn" => status_fn(api, &mut rng),
        "wr/tr_bgr" => tr_bgr(api, &mut rng),
        "wr/tr_swap" => tr_swap(api, &mut rng),
        "wr/tr_packing" => tr_packing(api, &mut rng),
        "wr/tr_packswap" => tr_packswap(api, &mut rng),
        "wr/tr_shift" => tr_shift(api, &mut rng),
        "wr/tr_invert_mono" => tr_invert_mono(api, &mut rng),
        "wr/tr_invert_alpha" => tr_invert_alpha(api, &mut rng),
        "wr/tr_swap_alpha" => tr_swap_alpha(api, &mut rng),
        "wr/tr_filler_after" => tr_filler(api, &mut rng, PNG_FILLER_AFTER),
        "wr/tr_filler_before" => tr_filler(api, &mut rng, PNG_FILLER_BEFORE),
        "wr/tr_user" => tr_user(api, &mut rng),
        "wr/tr_pairs" => tr_pairs(api, &mut rng),
        "wr/png_identity" => png_identity(api, &mut rng),
        "wr/png_transforms" => png_transforms(api, &mut rng),
        "wr/png_transform_pairs" => png_transform_pairs(api, &mut rng),
        "wr/anc_gama" => anc_gama(api, &mut rng),
        "wr/anc_chrm" => anc_chrm(api, &mut rng),
        "wr/anc_srgb" => anc_srgb(api, &mut rng),
        "wr/anc_iccp" => anc_iccp(api, &mut rng),
        "wr/anc_sbit" => anc_sbit(api, &mut rng),
        "wr/anc_trns" => anc_trns(api, &mut rng),
        "wr/anc_bkgd" => anc_bkgd(api, &mut rng),
        "wr/anc_hist" => anc_hist(api, &mut rng),
        "wr/anc_phys" => anc_phys(api, &mut rng),
        "wr/anc_offs" => anc_offs(api, &mut rng),
        "wr/anc_pcal" => anc_pcal(api, &mut rng),
        "wr/anc_scal" => anc_scal(api, &mut rng),
        "wr/anc_time" => anc_time(api, &mut rng),
        "wr/anc_splt" => anc_splt(api, &mut rng),
        "wr/anc_text" => anc_text(api, &mut rng),
        "wr/anc_ztxt" => anc_ztxt(api, &mut rng),
        "wr/anc_itxt" => anc_itxt(api, &mut rng),
        "wr/anc_exif" => anc_exif(api, &mut rng),
        "wr/anc_cicp" => anc_cicp(api, &mut rng),
        "wr/anc_clli" => anc_clli(api, &mut rng),
        "wr/anc_mdcv" => anc_mdcv(api, &mut rng),
        "wr/anc_unknown" => anc_unknown(api, &mut rng),
        "wr/anc_everything" => anc_everything(api, &mut rng),
        "wr/simple_formats" => simple_formats(api, &mut rng),
        "wr/simple_colormap" => simple_colormap(api, &mut rng),
        "wr/simple_convert8" => simple_convert8(api, &mut rng),
        "wr/simple_stride" => simple_stride(api, &mut rng),
        "wr/simple_sizing" => simple_sizing(api, &mut rng),
        "wr/simple_file" => simple_file(api, &mut rng),
        _ => return false,
    }
    true
}

/* ------------------------------------------------------------------ */
/* shared write helpers                                                */
/* ------------------------------------------------------------------ */

/// Fill and install a random PLTE of `1 << depth` entries (palette images).
unsafe fn set_random_plte(api: &Api, wr: &Writer, depth: u8, rng: &mut Rng) {
    let n = 1usize << depth.min(8);
    let pal: Vec<png_color> = (0..n)
        .map(|_| png_color { red: rng.u8(), green: rng.u8(), blue: rng.u8() })
        .collect();
    (api.png_set_PLTE)(wr.png, wr.info, pal.as_ptr(), n as c_int);
}

/// The full W-LOW drive for one image: IHDR (+PLTE) → write_info → rows →
/// write_end, then emit the produced bytes under `tag`.
unsafe fn write_low_image(
    api: &Api,
    rng: &mut Rng,
    color: u8,
    depth: u8,
    w: u32,
    h: u32,
    interlace: c_int,
    tag: &str,
) {
    let mut wr = Writer::new(api);
    (api.png_set_IHDR)(
        wr.png, wr.info, w, h, depth as c_int, color as c_int,
        interlace, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
    );
    if color == 3 {
        set_random_plte(api, &wr, depth, rng);
    }
    (api.png_write_info)(wr.png, wr.info);
    let passes = if interlace == PNG_INTERLACE_ADAM7 {
        (api.png_set_interlace_handling)(wr.png)
    } else {
        1
    };
    let rb = rowbytes_of(w, depth, color);
    // Pre-generate the image so every pass writes the same pixels.
    let rows: Vec<Vec<u8>> = (0..h).map(|_| rng.image_bytes(rb)).collect();
    for _ in 0..passes {
        for row in &rows {
            (api.png_write_row)(wr.png, row.as_ptr());
        }
    }
    (api.png_write_end)(wr.png, wr.info);
    emit_bytes(tag, wbuf());
    wr.destroy();
}

/* ------------------------------------------------------------------ */
/* B5 — raw chunk emission                                             */
/* ------------------------------------------------------------------ */

/// CONFIGS.md B5 — `png_write_sig` + `png_write_chunk_start`/`_data`/`_end`
/// and the one-shot `png_write_chunk`, over payload lengths that straddle the
/// internal compression/buffer boundaries.
fn chunk_raw(api: &Api, rng: &mut Rng) {
    unsafe {
        let lens: [usize; 8] = [0, 1, 2, 255, 256, 8191, 8192, 8193];
        for &len in &lens {
            let mut wr = Writer::new(api);
            (api.png_write_sig)(wr.png);
            // A random but ancillary+private-safe 4-byte type (lowercase 3rd
            // byte is reserved; keep letters so it is a legal chunk name).
            let name: [u8; 4] = [
                b'a' + (rng.u8() % 26),
                b'a' + (rng.u8() % 26),
                b'a' + (rng.u8() % 26),
                b'a' + (rng.u8() % 26),
            ];
            let payload = rng.image_bytes(len);
            // Split form: start / data (in two pieces) / end.
            (api.png_write_chunk_start)(wr.png, name.as_ptr(), len as u32);
            let half = len / 2;
            if half > 0 {
                (api.png_write_chunk_data)(wr.png, payload.as_ptr(), half);
            }
            if len - half > 0 {
                (api.png_write_chunk_data)(wr.png, payload.as_ptr().add(half), len - half);
            }
            (api.png_write_chunk_end)(wr.png);
            // One-shot form of the same chunk.
            (api.png_write_chunk)(wr.png, name.as_ptr(), payload.as_ptr(), len);
            emit_bytes(&format!("chunk len={} {:?}", len, name), wbuf());
            wr.destroy();
        }
    }
}

/* ------------------------------------------------------------------ */
/* B6..B20 — W-LOW over every colour/depth × shape                     */
/* ------------------------------------------------------------------ */

fn low_all_shapes(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            for (w, h) in SHAPES {
                write_low_image(
                    api, rng, color, depth, w, h, PNG_INTERLACE_NONE,
                    &format!("png ct={} bd={} {}x{}", color, depth, w, h),
                );
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* B21 — W-LOW, interlaced (Adam7)                                     */
/* ------------------------------------------------------------------ */

fn low_interlaced(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            for (w, h) in [(8u32, 8u32), (9, 4), (17, 3), (5, 5), (1, 1)] {
                write_low_image(
                    api, rng, color, depth, w, h, PNG_INTERLACE_ADAM7,
                    &format!("ipng ct={} bd={} {}x{}", color, depth, w, h),
                );
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* B22 — W-ROWS: png_write_rows in chunks of 1, 3, and all             */
/* ------------------------------------------------------------------ */

fn rows_api(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            for &chunk in &[1usize, 3, usize::MAX] {
                let (w, h) = (17u32, 6u32);
                let mut wr = Writer::new(api);
                (api.png_set_IHDR)(
                    wr.png, wr.info, w, h, depth as c_int, color as c_int,
                    PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                );
                if color == 3 {
                    set_random_plte(api, &wr, depth, rng);
                }
                (api.png_write_info)(wr.png, wr.info);
                let rb = rowbytes_of(w, depth, color);
                let mut rows: Vec<Vec<u8>> = (0..h).map(|_| rng.image_bytes(rb)).collect();
                let mut ptrs: Vec<*mut u8> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
                let mut i = 0usize;
                while i < ptrs.len() {
                    let n = chunk.min(ptrs.len() - i);
                    (api.png_write_rows)(wr.png, ptrs[i..].as_mut_ptr(), n as u32);
                    i += n;
                }
                (api.png_write_end)(wr.png, wr.info);
                emit_bytes(
                    &format!("rows ct={} bd={} chunk={}", color, depth,
                             if chunk == usize::MAX { h as usize } else { chunk }),
                    wbuf(),
                );
                wr.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* B23/B24 — W-IMAGE: png_write_image with a row_pointers array        */
/* ------------------------------------------------------------------ */

fn image_api(api: &Api, rng: &mut Rng, interlaced: bool) {
    unsafe {
        let il = if interlaced { PNG_INTERLACE_ADAM7 } else { PNG_INTERLACE_NONE };
        for (color, depth) in A1 {
            let (w, h) = (17u32, 6u32);
            let mut wr = Writer::new(api);
            (api.png_set_IHDR)(
                wr.png, wr.info, w, h, depth as c_int, color as c_int,
                il, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
            );
            if color == 3 {
                set_random_plte(api, &wr, depth, rng);
            }
            (api.png_write_info)(wr.png, wr.info);
            // png_write_image drives interlace passes internally, so it wants
            // one pointer per image row.
            let rb = rowbytes_of(w, depth, color);
            let mut rows: Vec<Vec<u8>> = (0..h).map(|_| rng.image_bytes(rb)).collect();
            let mut ptrs: Vec<*mut u8> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
            (api.png_write_image)(wr.png, ptrs.as_mut_ptr());
            (api.png_write_end)(wr.png, wr.info);
            emit_bytes(
                &format!("image ct={} bd={} il={}", color, depth, interlaced as u8),
                wbuf(),
            );
            wr.destroy();
        }
    }
}

/* ------------------------------------------------------------------ */
/* B25..B31 — filter masks on RGBA8                                    */
/* ------------------------------------------------------------------ */

/// Drive one 48×48 RGBA8 image with a given filter mask.
unsafe fn write_filtered(api: &Api, rng: &mut Rng, mask: c_int, tag: &str) {
    let (w, h) = (48u32, 48u32);
    let mut wr = Writer::new(api);
    (api.png_set_IHDR)(
        wr.png, wr.info, w, h, 8, PNG_COLOR_TYPE_RGBA,
        PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
    );
    (api.png_set_filter)(wr.png, PNG_FILTER_TYPE_BASE, mask);
    (api.png_write_info)(wr.png, wr.info);
    let rb = rowbytes_of(w, 8, PNG_COLOR_TYPE_RGBA as u8);
    for _ in 0..h {
        let row = rng.image_bytes(rb);
        (api.png_write_row)(wr.png, row.as_ptr());
    }
    (api.png_write_end)(wr.png, wr.info);
    emit_bytes(tag, wbuf());
    wr.destroy();
}

fn filters_single(api: &Api, rng: &mut Rng) {
    unsafe {
        for (name, mask) in [
            ("NONE", PNG_FILTER_NONE),
            ("SUB", PNG_FILTER_SUB),
            ("UP", PNG_FILTER_UP),
            ("AVG", PNG_FILTER_AVG),
            ("PAETH", PNG_FILTER_PAETH),
        ] {
            write_filtered(api, rng, mask, &format!("filter {}", name));
        }
    }
}

fn filters_all(api: &Api, rng: &mut Rng) {
    unsafe {
        write_filtered(api, rng, PNG_ALL_FILTERS, "filter ALL");
    }
}

fn filters_pairs(api: &Api, rng: &mut Rng) {
    unsafe {
        for (name, mask) in [
            ("NONE|SUB", PNG_FILTER_NONE | PNG_FILTER_SUB),
            ("SUB|UP", PNG_FILTER_SUB | PNG_FILTER_UP),
            ("UP|AVG", PNG_FILTER_UP | PNG_FILTER_AVG),
            ("AVG|PAETH", PNG_FILTER_AVG | PNG_FILTER_PAETH),
            ("NONE|PAETH", PNG_FILTER_NONE | PNG_FILTER_PAETH),
        ] {
            write_filtered(api, rng, mask, &format!("filter {}", name));
        }
    }
}

/* ------------------------------------------------------------------ */
/* B32..B36 — compression parameters                                   */
/* ------------------------------------------------------------------ */

/// Drive a 32×32 RGBA8 image, applying `setup` to the write struct before
/// `png_write_info`, and emit the produced bytes.
unsafe fn write_with_setup<F: Fn(png_structp)>(api: &Api, rng: &mut Rng, tag: &str, setup: F) {
    let (w, h) = (32u32, 32u32);
    let mut wr = Writer::new(api);
    (api.png_set_IHDR)(
        wr.png, wr.info, w, h, 8, PNG_COLOR_TYPE_RGBA,
        PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
    );
    setup(wr.png);
    (api.png_write_info)(wr.png, wr.info);
    let rb = rowbytes_of(w, 8, PNG_COLOR_TYPE_RGBA as u8);
    for _ in 0..h {
        let row = rng.image_bytes(rb);
        (api.png_write_row)(wr.png, row.as_ptr());
    }
    (api.png_write_end)(wr.png, wr.info);
    emit_bytes(tag, wbuf());
    wr.destroy();
}

fn compression_level(api: &Api, rng: &mut Rng) {
    unsafe {
        for lvl in [0i32, 1, 3, 6, 9] {
            write_with_setup(api, rng, &format!("clevel {}", lvl), |png| {
                (api.png_set_filter)(png, PNG_FILTER_TYPE_BASE, PNG_ALL_FILTERS);
                (api.png_set_compression_level)(png, lvl);
            });
        }
    }
}

fn compression_strategy(api: &Api, rng: &mut Rng) {
    unsafe {
        for s in [0i32, 1, 2, 3, 4] {
            write_with_setup(api, rng, &format!("cstrategy {}", s), |png| {
                (api.png_set_compression_strategy)(png, s);
            });
        }
    }
}

fn compression_memlevel(api: &Api, rng: &mut Rng) {
    unsafe {
        for m in [1i32, 4, 8, 9] {
            write_with_setup(api, rng, &format!("cmemlevel {}", m), |png| {
                (api.png_set_compression_mem_level)(png, m);
            });
        }
    }
}

fn compression_winbits(api: &Api, rng: &mut Rng) {
    unsafe {
        for wb in [8i32, 9, 11, 15] {
            write_with_setup(api, rng, &format!("cwinbits {}", wb), |png| {
                (api.png_set_compression_window_bits)(png, wb);
            });
        }
    }
}

fn compression_bufsize(api: &Api, rng: &mut Rng) {
    unsafe {
        for bs in [6usize, 64, 1024, 8192, 65536] {
            let (w, h) = (48u32, 48u32);
            let mut wr = Writer::new(api);
            (api.png_set_IHDR)(
                wr.png, wr.info, w, h, 8, PNG_COLOR_TYPE_RGBA,
                PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
            );
            (api.png_set_compression_buffer_size)(wr.png, bs);
            p!("bufsize set={} get={}", bs, (api.png_get_compression_buffer_size)(wr.png));
            (api.png_write_info)(wr.png, wr.info);
            let rb = rowbytes_of(w, 8, PNG_COLOR_TYPE_RGBA as u8);
            for _ in 0..h {
                let row = rng.image_bytes(rb);
                (api.png_write_row)(wr.png, row.as_ptr());
            }
            (api.png_write_end)(wr.png, wr.info);
            emit_bytes(&format!("bufsize {}", bs), wbuf());
            wr.destroy();
        }
    }
}

/* ------------------------------------------------------------------ */
/* B37 — flush; B38 — write status callback                            */
/* ------------------------------------------------------------------ */

fn flush(api: &Api, rng: &mut Rng) {
    unsafe {
        for n in [1i32, 2, 5] {
            let (w, h) = (16u32, 24u32);
            let mut wr = Writer::new(api);
            (api.png_set_IHDR)(
                wr.png, wr.info, w, h, 8, PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
            );
            (api.png_set_flush)(wr.png, n);
            (api.png_write_info)(wr.png, wr.info);
            let rb = rowbytes_of(w, 8, PNG_COLOR_TYPE_RGB as u8);
            for _ in 0..h {
                let row = rng.image_bytes(rb);
                (api.png_write_row)(wr.png, row.as_ptr());
                (api.png_write_flush)(wr.png);
            }
            (api.png_write_end)(wr.png, wr.info);
            emit_bytes(&format!("flush n={}", n), wbuf());
            wr.destroy();
        }
    }
}

fn status_fn(api: &Api, rng: &mut Rng) {
    unsafe {
        let (w, h) = (16u32, 12u32);
        let mut wr = Writer::new(api);
        (api.png_set_IHDR)(
            wr.png, wr.info, w, h, 8, PNG_COLOR_TYPE_RGB,
            PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        (api.png_set_write_status_fn)(wr.png, Some(write_status_fn));
        (api.png_write_info)(wr.png, wr.info);
        let rb = rowbytes_of(w, 8, PNG_COLOR_TYPE_RGB as u8);
        for _ in 0..h {
            let row = rng.image_bytes(rb);
            (api.png_write_row)(wr.png, row.as_ptr());
        }
        (api.png_write_end)(wr.png, wr.info);
        emit_bytes("status", wbuf());
        wr.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* B39..B50 — write transforms (A6)                                    */
/* ------------------------------------------------------------------ */

/// Drive a W-LOW image where the *declared* IHDR is (color, depth) but the
/// per-row input buffer is `in_rb` bytes wide (which may differ from the IHDR
/// rowbytes when a transform expands/contracts the row, e.g. filler). `setup`
/// installs the transform after IHDR and before write_info.
unsafe fn write_transform<F: Fn(png_structp, png_infop)>(
    api: &Api,
    rng: &mut Rng,
    color: u8,
    depth: u8,
    w: u32,
    h: u32,
    in_rb: usize,
    tag: &str,
    setup: F,
) {
    let mut wr = Writer::new(api);
    (api.png_set_IHDR)(
        wr.png, wr.info, w, h, depth as c_int, color as c_int,
        PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
    );
    if color == 3 {
        set_random_plte(api, &wr, depth, rng);
    }
    setup(wr.png, wr.info);
    (api.png_write_info)(wr.png, wr.info);
    for _ in 0..h {
        let row = rng.image_bytes(in_rb);
        (api.png_write_row)(wr.png, row.as_ptr());
    }
    (api.png_write_end)(wr.png, wr.info);
    emit_bytes(tag, wbuf());
    wr.destroy();
}

/// Same, but the transform is applied *after* `png_write_info`.
///
/// `png_set_shift` and `png_set_filler` validate against `png_ptr->bit_depth`
/// and `png_ptr->color_type` (pngtrans.c), and on a write struct those fields
/// are only populated by `png_write_IHDR`, which `png_write_info` calls. Setting
/// them earlier makes libpng compare against a bit depth of 0 and reject every
/// value with `png_app_error`, so the whole case would abort on its first call
/// instead of exercising the transform.
unsafe fn write_transform_after<F: Fn(png_structp, png_infop)>(
    api: &Api,
    rng: &mut Rng,
    color: u8,
    depth: u8,
    w: u32,
    h: u32,
    in_rb: usize,
    tag: &str,
    setup: F,
) {
    let mut wr = Writer::new(api);
    (api.png_set_IHDR)(
        wr.png, wr.info, w, h, depth as c_int, color as c_int,
        PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
    );
    if color == 3 {
        set_random_plte(api, &wr, depth, rng);
    }
    (api.png_write_info)(wr.png, wr.info);
    setup(wr.png, wr.info);
    for _ in 0..h {
        let row = rng.image_bytes(in_rb);
        (api.png_write_row)(wr.png, row.as_ptr());
    }
    (api.png_write_end)(wr.png, wr.info);
    emit_bytes(tag, wbuf());
    wr.destroy();
}

fn tr_bgr(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(2u8, 8u8), (2, 16), (6, 8), (6, 16)] {
            let (w, h) = (24u32, 8u32);
            let rb = rowbytes_of(w, depth, color);
            write_transform(api, rng, color, depth, w, h, rb,
                &format!("bgr ct={} bd={}", color, depth),
                |png, _| { (api.png_set_bgr)(png); });
        }
    }
}

fn tr_swap(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(0u8, 16u8), (2, 16), (4, 16), (6, 16)] {
            let (w, h) = (24u32, 8u32);
            let rb = rowbytes_of(w, depth, color);
            write_transform(api, rng, color, depth, w, h, rb,
                &format!("swap ct={} bd={}", color, depth),
                |png, _| { (api.png_set_swap)(png); });
        }
    }
}

fn tr_packing(api: &Api, rng: &mut Rng) {
    unsafe {
        // With png_set_packing the caller supplies one byte per pixel; libpng
        // packs to the declared sub-byte depth. Input rowbytes = 1 byte/pixel.
        for (color, depth) in [(0u8, 1u8), (0, 2), (0, 4), (3, 1), (3, 2), (3, 4)] {
            let (w, h) = (17u32, 6u32);
            let in_rb = w as usize; // 1 byte per pixel, 1 channel
            write_transform(api, rng, color, depth, w, h, in_rb,
                &format!("packing ct={} bd={}", color, depth),
                |png, _| { (api.png_set_packing)(png); });
        }
    }
}

fn tr_packswap(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(0u8, 1u8), (0, 2), (0, 4), (3, 1), (3, 2), (3, 4)] {
            let (w, h) = (17u32, 6u32);
            // packswap operates on the packed sub-byte row (no packing here),
            // so the input is the normal packed rowbytes.
            let rb = rowbytes_of(w, depth, color);
            write_transform(api, rng, color, depth, w, h, rb,
                &format!("packswap ct={} bd={}", color, depth),
                |png, _| { (api.png_set_packswap)(png); });
        }
    }
}

fn tr_shift(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(0u8, 8u8), (0, 16), (2, 8), (2, 16), (6, 8)] {
            let (w, h) = (16u32, 6u32);
            let rb = rowbytes_of(w, depth, color);
            let mask = depth.min(8);
            let sig = png_color_8 {
                red: mask, green: mask, blue: mask,
                gray: mask, alpha: mask,
            };
            write_transform_after(api, rng, color, depth, w, h, rb,
                &format!("shift ct={} bd={}", color, depth),
                |png, info| {
                    (api.png_set_sBIT)(png, info, &sig);
                    (api.png_set_shift)(png, &sig);
                });
        }
    }
}

fn tr_invert_mono(api: &Api, rng: &mut Rng) {
    unsafe {
        for depth in [1u8, 2, 4, 8, 16] {
            let (w, h) = (16u32, 6u32);
            let rb = rowbytes_of(w, depth, 0);
            write_transform(api, rng, 0, depth, w, h, rb,
                &format!("invmono bd={}", depth),
                |png, _| { (api.png_set_invert_mono)(png); });
        }
    }
}

fn tr_invert_alpha(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(4u8, 8u8), (4, 16), (6, 8), (6, 16)] {
            let (w, h) = (16u32, 6u32);
            let rb = rowbytes_of(w, depth, color);
            write_transform(api, rng, color, depth, w, h, rb,
                &format!("invalpha ct={} bd={}", color, depth),
                |png, _| { (api.png_set_invert_alpha)(png); });
        }
    }
}

fn tr_swap_alpha(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(4u8, 8u8), (4, 16), (6, 8), (6, 16)] {
            let (w, h) = (16u32, 6u32);
            let rb = rowbytes_of(w, depth, color);
            write_transform(api, rng, color, depth, w, h, rb,
                &format!("swapalpha ct={} bd={}", color, depth),
                |png, _| { (api.png_set_swap_alpha)(png); });
        }
    }
}

fn tr_filler(api: &Api, rng: &mut Rng, loc: c_int) {
    unsafe {
        // Declare RGB but feed RGBA rows; png_set_filler strips the filler
        // byte on write. Also GRAY (declared) from GRAY+filler input.
        let name = if loc == PNG_FILLER_AFTER { "after" } else { "before" };
        for (color, depth, chans_in) in [(2u8, 8u8, 4usize), (2, 16, 4), (0, 8, 2), (0, 16, 2)] {
            let (w, h) = (16u32, 6u32);
            let bytes_per_sample = (depth / 8) as usize;
            let in_rb = w as usize * chans_in * bytes_per_sample;
            write_transform_after(api, rng, color, depth, w, h, in_rb,
                &format!("filler_{} ct={} bd={}", name, color, depth),
                |png, _| { (api.png_set_filler)(png, 0, loc); });
        }
    }
}

fn tr_user(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(2u8, 8u8), (6, 8), (0, 8)] {
            let (w, h) = (16u32, 6u32);
            let rb = rowbytes_of(w, depth, color);
            write_transform(api, rng, color, depth, w, h, rb,
                &format!("user ct={} bd={}", color, depth),
                |png, _| {
                    (api.png_set_write_user_transform_fn)(png, Some(user_transform_fn));
                });
        }
    }
}

fn tr_pairs(api: &Api, rng: &mut Rng) {
    unsafe {
        // bgr+swap on RGB16
        {
            let (w, h) = (16u32, 6u32);
            let rb = rowbytes_of(w, 16, 2);
            write_transform(api, rng, 2, 16, w, h, rb, "pair bgr+swap",
                |png, _| { (api.png_set_bgr)(png); (api.png_set_swap)(png); });
        }
        // packing+packswap on GRAY×2
        {
            let (w, h) = (17u32, 6u32);
            let in_rb = w as usize;
            write_transform(api, rng, 0, 2, w, h, in_rb, "pair packing+packswap",
                |png, _| { (api.png_set_packing)(png); (api.png_set_packswap)(png); });
        }
        // invert_alpha+swap_alpha on RGBA8
        {
            let (w, h) = (16u32, 6u32);
            let rb = rowbytes_of(w, 8, 6);
            write_transform(api, rng, 6, 8, w, h, rb, "pair inv+swap alpha",
                |png, _| { (api.png_set_invert_alpha)(png); (api.png_set_swap_alpha)(png); });
        }
        // shift+swap on RGB16
        {
            let (w, h) = (16u32, 6u32);
            let rb = rowbytes_of(w, 16, 2);
            let sig = png_color_8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 };
            write_transform(api, rng, 2, 16, w, h, rb, "pair shift+swap",
                |png, info| {
                    (api.png_set_sBIT)(png, info, &sig);
                    (api.png_set_shift)(png, &sig);
                    (api.png_set_swap)(png);
                });
        }
    }
}

/* ------------------------------------------------------------------ */
/* B51..B53 — W-PNG one-shot via png_write_png + png_set_rows          */
/* ------------------------------------------------------------------ */

/// Drive png_write_png for a (color, depth) image with the given transform
/// mask. `in_chans`/`in_bps` describe the input row width (which differs from
/// the IHDR when the transform strips filler etc.).
unsafe fn write_png_masked(
    api: &Api,
    rng: &mut Rng,
    color: u8,
    depth: u8,
    transforms: c_int,
    in_rb: usize,
    tag: &str,
) {
    let (w, h) = (17u32, 6u32);
    let mut wr = Writer::new(api);
    (api.png_set_IHDR)(
        wr.png, wr.info, w, h, depth as c_int, color as c_int,
        PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
    );
    if color == 3 {
        set_random_plte(api, &wr, depth, rng);
    }
    let mut rows: Vec<Vec<u8>> = (0..h).map(|_| rng.image_bytes(in_rb)).collect();
    let mut ptrs: Vec<*mut u8> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
    (api.png_set_rows)(wr.png, wr.info, ptrs.as_mut_ptr());
    (api.png_write_png)(wr.png, wr.info, transforms, null());
    emit_bytes(tag, wbuf());
    wr.destroy();
}

fn png_identity(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            let rb = rowbytes_of(17, depth, color);
            write_png_masked(api, rng, color, depth, PNG_TRANSFORM_IDENTITY, rb,
                &format!("wpng ct={} bd={}", color, depth));
        }
    }
}

fn png_transforms(api: &Api, rng: &mut Rng) {
    unsafe {
        // (name, mask, color, depth, input rowbytes)
        // For most transforms the input row is the normal IHDR rowbytes; for
        // filler-stripping we feed an extra channel.
        let w = 17usize;
        let cases: &[(&str, c_int, u8, u8, usize)] = &[
            ("PACKING", PNG_TRANSFORM_PACKING, 0, 2, w /* 1 byte/px */),
            ("PACKSWAP", PNG_TRANSFORM_PACKSWAP, 0, 2, rowbytes_of(17, 2, 0)),
            ("INVERT_MONO", PNG_TRANSFORM_INVERT_MONO, 0, 8, rowbytes_of(17, 8, 0)),
            ("SHIFT", PNG_TRANSFORM_SHIFT, 2, 8, rowbytes_of(17, 8, 2)),
            ("BGR", PNG_TRANSFORM_BGR, 2, 8, rowbytes_of(17, 8, 2)),
            ("SWAP_ALPHA", PNG_TRANSFORM_SWAP_ALPHA, 6, 8, rowbytes_of(17, 8, 6)),
            ("SWAP_ENDIAN", PNG_TRANSFORM_SWAP_ENDIAN, 2, 16, rowbytes_of(17, 16, 2)),
            ("INVERT_ALPHA", PNG_TRANSFORM_INVERT_ALPHA, 6, 8, rowbytes_of(17, 8, 6)),
            ("STRIP_FILLER_BEFORE", PNG_TRANSFORM_STRIP_FILLER_BEFORE, 2, 8, w * 4),
            ("STRIP_FILLER_AFTER", PNG_TRANSFORM_STRIP_FILLER_AFTER, 2, 8, w * 4),
        ];
        for (name, mask, color, depth, in_rb) in cases {
            // SHIFT needs sBIT in the info struct; set it via a dedicated path.
            if *name == "SHIFT" {
                let (ww, hh) = (17u32, 6u32);
                let mut wr = Writer::new(api);
                (api.png_set_IHDR)(
                    wr.png, wr.info, ww, hh, *depth as c_int, *color as c_int,
                    PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
                );
                let sig = png_color_8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 };
                (api.png_set_sBIT)(wr.png, wr.info, &sig);
                let mut rows: Vec<Vec<u8>> =
                    (0..hh).map(|_| rng.image_bytes(*in_rb)).collect();
                let mut ptrs: Vec<*mut u8> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
                (api.png_set_rows)(wr.png, wr.info, ptrs.as_mut_ptr());
                (api.png_write_png)(wr.png, wr.info, *mask, null());
                emit_bytes(&format!("wpngt {}", name), wbuf());
                wr.destroy();
            } else {
                write_png_masked(api, rng, *color, *depth, *mask, *in_rb,
                    &format!("wpngt {}", name));
            }
        }
    }
}

fn png_transform_pairs(api: &Api, rng: &mut Rng) {
    unsafe {
        let w = 17usize;
        // BGR|SWAP_ENDIAN on RGB16
        write_png_masked(api, rng, 2, 16,
            PNG_TRANSFORM_BGR | PNG_TRANSFORM_SWAP_ENDIAN,
            rowbytes_of(17, 16, 2), "wpngp BGR|SWAP_ENDIAN");
        // PACKING|PACKSWAP on GRAY×2 (input 1 byte/px)
        write_png_masked(api, rng, 0, 2,
            PNG_TRANSFORM_PACKING | PNG_TRANSFORM_PACKSWAP,
            w, "wpngp PACKING|PACKSWAP");
        // INVERT_ALPHA|SWAP_ALPHA on RGBA8
        write_png_masked(api, rng, 6, 8,
            PNG_TRANSFORM_INVERT_ALPHA | PNG_TRANSFORM_SWAP_ALPHA,
            rowbytes_of(17, 8, 6), "wpngp INVERT_ALPHA|SWAP_ALPHA");
    }
}

/* ------------------------------------------------------------------ */
/* B54..B76 — ancillary chunks (A8)                                    */
/* ------------------------------------------------------------------ */

/// Set up a writer with IHDR (color, depth, w, h) and optional PLTE, run
/// `set` to install the chunk(s), write the image, emit the bytes, then run
/// `readback` to print the info-struct round trip.
unsafe fn anc_image<S, R>(
    api: &Api,
    rng: &mut Rng,
    color: u8,
    depth: u8,
    w: u32,
    h: u32,
    tag: &str,
    set: S,
    readback: R,
) where
    S: Fn(png_structp, png_infop),
    R: Fn(png_structp, png_infop),
{
    let mut wr = Writer::new(api);
    (api.png_set_IHDR)(
        wr.png, wr.info, w, h, depth as c_int, color as c_int,
        PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
    );
    if color == 3 {
        set_random_plte(api, &wr, depth, rng);
    }
    set(wr.png, wr.info);
    // Read the values straight back before writing, so the info-struct round
    // trip is compared.
    readback(wr.png, wr.info);
    (api.png_write_info)(wr.png, wr.info);
    let rb = rowbytes_of(w, depth, color);
    for _ in 0..h {
        let row = rng.image_bytes(rb);
        (api.png_write_row)(wr.png, row.as_ptr());
    }
    (api.png_write_end)(wr.png, wr.info);
    emit_bytes(tag, wbuf());
    wr.destroy();
}

fn anc_gama(api: &Api, rng: &mut Rng) {
    unsafe {
        for g in [1i32, 45455, 100000, 500000, 0x7fff_ffff] {
            anc_image(api, rng, 2, 8, 8, 8, &format!("gama_fixed {}", g),
                |png, info| { (api.png_set_gAMA_fixed)(png, info, g); },
                |png, info| {
                    let mut out = 0i32;
                    let r = (api.png_get_gAMA_fixed)(png, info, &mut out);
                    p!("  get_gAMA_fixed r={} v={}", r, out);
                });
        }
        for g in [0.45455f64, 1.0, 2.2] {
            anc_image(api, rng, 2, 8, 8, 8, &format!("gama_float {}", g),
                |png, info| { (api.png_set_gAMA)(png, info, g); },
                |png, info| {
                    let mut out = 0f64;
                    let r = (api.png_get_gAMA)(png, info, &mut out);
                    p!("  get_gAMA r={} v={:.5}", r, out);
                });
        }
    }
}

fn anc_chrm(api: &Api, rng: &mut Rng) {
    unsafe {
        // sensible sRGB-ish chromaticities
        let (wx, wy) = (0.3127f64, 0.3290);
        let (rx, ry) = (0.64, 0.33);
        let (gx, gy) = (0.30, 0.60);
        let (bx, by) = (0.15, 0.06);
        anc_image(api, rng, 2, 8, 8, 8, "chrm_float",
            |png, info| { (api.png_set_cHRM)(png, info, wx, wy, rx, ry, gx, gy, bx, by); },
            |png, info| {
                let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
                    (0f64, 0f64, 0f64, 0f64, 0f64, 0f64, 0f64, 0f64);
                let r = (api.png_get_cHRM)(png, info, &mut a, &mut b, &mut c, &mut d,
                                           &mut e, &mut f, &mut g, &mut h);
                p!("  get_cHRM r={} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} {:.4}",
                   r, a, b, c, d, e, f, g, h);
            });
        anc_image(api, rng, 2, 8, 8, 8, "chrm_fixed",
            |png, info| {
                (api.png_set_cHRM_fixed)(png, info, 31270, 32900, 64000, 33000,
                                         30000, 60000, 15000, 6000);
            },
            |png, info| {
                let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h) =
                    (0i32, 0i32, 0i32, 0i32, 0i32, 0i32, 0i32, 0i32);
                let r = (api.png_get_cHRM_fixed)(png, info, &mut a, &mut b, &mut c,
                                                 &mut d, &mut e, &mut f, &mut g, &mut h);
                p!("  get_cHRM_fixed r={} {} {} {} {} {} {} {} {}",
                   r, a, b, c, d, e, f, g, h);
            });
        // XYZ variant
        anc_image(api, rng, 2, 8, 8, 8, "chrm_xyz",
            |png, info| {
                (api.png_set_cHRM_XYZ)(png, info,
                    0.4124, 0.2126, 0.0193, 0.3576, 0.7152, 0.1192,
                    0.1805, 0.0722, 0.9505);
            },
            |png, info| {
                let mut v = [0f64; 9];
                let r = (api.png_get_cHRM_XYZ)(png, info,
                    &mut v[0], &mut v[1], &mut v[2], &mut v[3], &mut v[4],
                    &mut v[5], &mut v[6], &mut v[7], &mut v[8]);
                p!("  get_cHRM_XYZ r={} {:.4?}", r, v);
            });
    }
}

fn anc_srgb(api: &Api, rng: &mut Rng) {
    unsafe {
        for intent in [0i32, 1, 2, 3] {
            anc_image(api, rng, 2, 8, 8, 8, &format!("srgb {}", intent),
                |png, info| { (api.png_set_sRGB)(png, info, intent); },
                |png, info| {
                    let mut out = -1i32;
                    let r = (api.png_get_sRGB)(png, info, &mut out);
                    p!("  get_sRGB r={} intent={}", r, out);
                });
        }
        anc_image(api, rng, 2, 8, 8, 8, "srgb_gama_chrm",
            |png, info| { (api.png_set_sRGB_gAMA_and_cHRM)(png, info, 0); },
            |png, info| {
                let mut g = 0i32;
                let r = (api.png_get_gAMA_fixed)(png, info, &mut g);
                p!("  after srgb_gama_chrm gAMA r={} v={}", r, g);
            });
    }
}

fn anc_iccp(api: &Api, rng: &mut Rng) {
    unsafe {
        for len in [132usize, 136, 1024, 8192] {
            // A structurally valid-enough ICC profile: first 4 bytes are the
            // big-endian declared size; the rest is deterministic filler.
            let mut prof = rng.image_bytes(len);
            let sz = (len as u32).to_be_bytes();
            prof[0..4].copy_from_slice(&sz);
            let name = cs("ICC");
            anc_image(api, rng, 2, 8, 8, 8, &format!("iccp {}", len),
                |png, info| {
                    (api.png_set_iCCP)(png, info, name.as_ptr(), 0,
                                       prof.as_ptr(), len as u32);
                },
                |png, info| {
                    let mut nm: *mut c_char = null();
                    let mut comp = 0i32;
                    let mut pp: *mut u8 = null();
                    let mut plen = 0u32;
                    let r = (api.png_get_iCCP)(png, info, &mut nm, &mut comp,
                                               &mut pp, &mut plen);
                    p!("  get_iCCP r={} name={:?} comp={} len={}",
                       r, cstr(nm), comp, plen);
                });
        }
    }
}

fn anc_sbit(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            if color == 3 {
                continue; // sBIT for palette refers to source RGB; keep to true-colour/gray
            }
            let m = depth.min(8);
            let sig = png_color_8 { red: m, green: m, blue: m, gray: m, alpha: m };
            anc_image(api, rng, color, depth, 8, 8,
                &format!("sbit ct={} bd={}", color, depth),
                |png, info| { (api.png_set_sBIT)(png, info, &sig); },
                |png, info| {
                    let mut pp: *mut png_color_8 = null();
                    let r = (api.png_get_sBIT)(png, info, &mut pp);
                    if !pp.is_null() {
                        let s = *pp;
                        p!("  get_sBIT r={} r{} g{} b{} gray{} a{}",
                           r, s.red, s.green, s.blue, s.gray, s.alpha);
                    } else {
                        p!("  get_sBIT r={} <null>", r);
                    }
                });
        }
    }
}

fn anc_trns(api: &Api, rng: &mut Rng) {
    unsafe {
        // Palette tRNS
        for depth in [1u8, 2, 4, 8] {
            let n = 1usize << depth;
            let trns = rng.bytes(n);
            anc_image(api, rng, 3, depth, 8, 8, &format!("trns_pal bd={}", depth),
                |png, info| {
                    (api.png_set_tRNS)(png, info, trns.as_ptr(), n as c_int, null());
                },
                |png, info| {
                    let mut tp: *mut u8 = null();
                    let mut num = 0i32;
                    let mut vp: *mut png_color_16 = null();
                    let r = (api.png_get_tRNS)(png, info, &mut tp, &mut num, &mut vp);
                    p!("  get_tRNS(pal) r={} num={}", r, num);
                });
        }
        // Gray tRNS
        for depth in [1u8, 2, 4, 8, 16] {
            let v = png_color_16 {
                index: 0,
                red: 0, green: 0, blue: 0,
                gray: (rng.u32() as u16) & ((1u16 << depth.min(15)).wrapping_sub(1)).max(1),
            };
            anc_image(api, rng, 0, depth, 8, 8, &format!("trns_gray bd={}", depth),
                |png, info| { (api.png_set_tRNS)(png, info, null(), 0, &v); },
                |png, info| {
                    let mut tp: *mut u8 = null();
                    let mut num = 0i32;
                    let mut vp: *mut png_color_16 = null();
                    let r = (api.png_get_tRNS)(png, info, &mut tp, &mut num, &mut vp);
                    if !vp.is_null() {
                        p!("  get_tRNS(gray) r={} gray={}", r, (*vp).gray);
                    } else {
                        p!("  get_tRNS(gray) r={} <null>", r);
                    }
                });
        }
        // RGB tRNS
        for depth in [8u8, 16] {
            let v = png_color_16 {
                index: 0,
                red: rng.u32() as u16, green: rng.u32() as u16, blue: rng.u32() as u16,
                gray: 0,
            };
            anc_image(api, rng, 2, depth, 8, 8, &format!("trns_rgb bd={}", depth),
                |png, info| { (api.png_set_tRNS)(png, info, null(), 0, &v); },
                |png, info| {
                    let mut tp: *mut u8 = null();
                    let mut num = 0i32;
                    let mut vp: *mut png_color_16 = null();
                    let r = (api.png_get_tRNS)(png, info, &mut tp, &mut num, &mut vp);
                    if !vp.is_null() {
                        let c = *vp;
                        p!("  get_tRNS(rgb) r={} {} {} {}", r, c.red, c.green, c.blue);
                    } else {
                        p!("  get_tRNS(rgb) r={} <null>", r);
                    }
                });
        }
    }
}

fn anc_bkgd(api: &Api, rng: &mut Rng) {
    unsafe {
        // Palette index background
        anc_image(api, rng, 3, 8, 8, 8, "bkgd_pal",
            |png, info| {
                let b = png_color_16 { index: 3, red: 0, green: 0, blue: 0, gray: 0 };
                (api.png_set_bKGD)(png, info, &b);
            },
            |png, info| {
                let mut pp: *mut png_color_16 = null();
                let r = (api.png_get_bKGD)(png, info, &mut pp);
                if !pp.is_null() { p!("  get_bKGD(pal) r={} index={}", r, (*pp).index); }
            });
        // Gray / RGB backgrounds, 8 and 16 bit
        for (color, depth) in [(0u8, 8u8), (0, 16), (2, 8), (2, 16)] {
            let b = png_color_16 {
                index: 0,
                red: rng.u32() as u16, green: rng.u32() as u16, blue: rng.u32() as u16,
                gray: rng.u32() as u16,
            };
            anc_image(api, rng, color, depth, 8, 8,
                &format!("bkgd ct={} bd={}", color, depth),
                |png, info| { (api.png_set_bKGD)(png, info, &b); },
                |png, info| {
                    let mut pp: *mut png_color_16 = null();
                    let r = (api.png_get_bKGD)(png, info, &mut pp);
                    if !pp.is_null() {
                        let c = *pp;
                        p!("  get_bKGD r={} gray={} rgb={} {} {}",
                           r, c.gray, c.red, c.green, c.blue);
                    }
                });
        }
    }
}

fn anc_hist(api: &Api, rng: &mut Rng) {
    unsafe {
        for depth in [1u8, 4, 8] {
            let n = 1usize << depth; // 2, 16, 256
            let hist: Vec<u16> = (0..n).map(|_| rng.u32() as u16).collect();
            anc_image(api, rng, 3, depth, 8, 8, &format!("hist n={}", n),
                |png, info| { (api.png_set_hIST)(png, info, hist.as_ptr()); },
                |png, info| {
                    let mut hp: *mut u16 = null();
                    let r = (api.png_get_hIST)(png, info, &mut hp);
                    if !hp.is_null() {
                        let s = std::slice::from_raw_parts(hp, n);
                        p!("  get_hIST r={} first={} last={}", r, s[0], s[n - 1]);
                    } else {
                        p!("  get_hIST r={} <null>", r);
                    }
                });
        }
    }
}

fn anc_phys(api: &Api, rng: &mut Rng) {
    unsafe {
        for unit in [0i32, 1] {
            let (x, y) = (rng.range(1, 100000), rng.range(1, 100000));
            anc_image(api, rng, 2, 8, 8, 8, &format!("phys unit={}", unit),
                |png, info| { (api.png_set_pHYs)(png, info, x, y, unit); },
                |png, info| {
                    let (mut rx, mut ry, mut ru) = (0u32, 0u32, 0i32);
                    let r = (api.png_get_pHYs)(png, info, &mut rx, &mut ry, &mut ru);
                    p!("  get_pHYs r={} {} {} unit={}", r, rx, ry, ru);
                    // round-trip through the inch accessor as well
                    p!("  x_ppi={}", (api.png_get_x_pixels_per_inch)(png, info));
                });
        }
    }
}

fn anc_offs(api: &Api, rng: &mut Rng) {
    unsafe {
        for unit in [0i32, 1] {
            for (x, y) in [(0i32, 0i32), (-5, 7), (i32::MIN + 1, i32::MAX)] {
                anc_image(api, rng, 2, 8, 8, 8,
                    &format!("offs unit={} {} {}", unit, x, y),
                    |png, info| { (api.png_set_oFFs)(png, info, x, y, unit); },
                    |png, info| {
                        let (mut rx, mut ry, mut ru) = (0i32, 0i32, 0i32);
                        let r = (api.png_get_oFFs)(png, info, &mut rx, &mut ry, &mut ru);
                        p!("  get_oFFs r={} {} {} unit={}", r, rx, ry, ru);
                    });
            }
        }
    }
}

fn anc_pcal(api: &Api, rng: &mut Rng) {
    unsafe {
        // equation types 0..3 with nparams 0/1/2/3
        for (eqtype, nparams) in [(0i32, 0i32), (1, 1), (2, 2), (3, 3)] {
            let purpose = cs("calibration");
            let unit = cs("units");
            // params are C strings (ASCII numbers)
            let param_strs: Vec<Vec<c_char>> =
                (0..nparams).map(|i| cs(&format!("{}.0", i + 1))).collect();
            let mut param_ptrs: Vec<*mut c_char> =
                param_strs.iter().map(|p| p.as_ptr() as *mut c_char).collect();
            let params_ptr = param_ptrs.as_mut_ptr();
            let (x0, x1) = (0i32, 255i32);
            anc_image(api, rng, 0, 8, 8, 8,
                &format!("pcal eq={} n={}", eqtype, nparams),
                |png, info| {
                    (api.png_set_pCAL)(png, info, purpose.as_ptr(), x0, x1,
                                       eqtype, nparams, unit.as_ptr(),
                                       params_ptr);
                },
                |png, info| {
                    let mut pp: *mut c_char = null();
                    let (mut rx0, mut rx1) = (0i32, 0i32);
                    let (mut ret, mut rn) = (0i32, 0i32);
                    let mut ru: *mut c_char = null();
                    let mut rparams: *mut *mut c_char = null();
                    let r = (api.png_get_pCAL)(png, info, &mut pp, &mut rx0, &mut rx1,
                        &mut ret, &mut rn, &mut ru, &mut rparams);
                    p!("  get_pCAL r={} x0={} x1={} type={} nparams={} purpose={:?}",
                       r, rx0, rx1, ret, rn, cstr(pp));
                });
        }
    }
}

fn anc_scal(api: &Api, rng: &mut Rng) {
    unsafe {
        for unit in [1i32, 2] {
            // float form
            anc_image(api, rng, 2, 8, 8, 8, &format!("scal_f unit={}", unit),
                |png, info| { (api.png_set_sCAL)(png, info, unit, 2.5, 1.25); },
                |png, info| {
                    let (mut ru, mut w, mut h) = (0i32, 0f64, 0f64);
                    let r = (api.png_get_sCAL)(png, info, &mut ru, &mut w, &mut h);
                    p!("  get_sCAL r={} unit={} {:.4} {:.4}", r, ru, w, h);
                });
            // fixed form
            anc_image(api, rng, 2, 8, 8, 8, &format!("scal_x unit={}", unit),
                |png, info| { (api.png_set_sCAL_fixed)(png, info, unit, 250000, 125000); },
                |png, info| {
                    let (mut ru, mut w, mut h) = (0i32, 0i32, 0i32);
                    let r = (api.png_get_sCAL_fixed)(png, info, &mut ru, &mut w, &mut h);
                    p!("  get_sCAL_fixed r={} unit={} {} {}", r, ru, w, h);
                });
            // string form
            let ws = cs("2.5");
            let hs = cs("1.25");
            anc_image(api, rng, 2, 8, 8, 8, &format!("scal_s unit={}", unit),
                |png, info| {
                    (api.png_set_sCAL_s)(png, info, unit, ws.as_ptr(), hs.as_ptr());
                },
                |png, info| {
                    let (mut ru, mut wp, mut hp) =
                        (0i32, null::<c_char>() as *mut c_char, null::<c_char>() as *mut c_char);
                    let r = (api.png_get_sCAL_s)(png, info, &mut ru, &mut wp, &mut hp);
                    p!("  get_sCAL_s r={} unit={} w={:?} h={:?}",
                       r, ru, cstr(wp), cstr(hp));
                });
        }
    }
}

fn anc_time(api: &Api, rng: &mut Rng) {
    unsafe {
        let times = [
            png_time { year: 1970, month: 1, day: 1, hour: 0, minute: 0, second: 0 },
            png_time { year: 2000, month: 12, day: 31, hour: 23, minute: 59, second: 60 },
            png_time { year: 9999, month: 6, day: 15, hour: 12, minute: 30, second: 30 },
            png_time { year: 2023, month: 3, day: 9, hour: 8, minute: 5, second: 1 },
        ];
        for t in times {
            anc_image(api, rng, 2, 8, 8, 8,
                &format!("time {}-{}-{}", t.year, t.month, t.day),
                |png, info| { (api.png_set_tIME)(png, info, &t); },
                |png, info| {
                    let mut tp: *mut png_time = null();
                    let r = (api.png_get_tIME)(png, info, &mut tp);
                    if !tp.is_null() {
                        let x = *tp;
                        p!("  get_tIME r={} {}-{}-{} {}:{}:{}",
                           r, x.year, x.month, x.day, x.hour, x.minute, x.second);
                    }
                });
        }
    }
}

fn anc_splt(api: &Api, rng: &mut Rng) {
    unsafe {
        for (nentries, depth) in [(1i32, 8u8), (2, 8), (16, 8), (2, 16), (16, 16)] {
            let name = cs("suggested");
            let entries: Vec<png_sPLT_entry> = (0..nentries)
                .map(|_| png_sPLT_entry {
                    red: rng.u32() as u16, green: rng.u32() as u16,
                    blue: rng.u32() as u16, alpha: rng.u32() as u16,
                    frequency: rng.u32() as u16,
                })
                .collect();
            let splt = png_sPLT_t {
                name: name.as_ptr() as *mut c_char,
                depth,
                entries: entries.as_ptr() as *mut png_sPLT_entry,
                nentries,
            };
            let splt_ptr = &splt as *const png_sPLT_t;
            anc_image(api, rng, 2, 8, 8, 8,
                &format!("splt n={} d={}", nentries, depth),
                |png, info| { (api.png_set_sPLT)(png, info, splt_ptr, 1); },
                |png, info| {
                    let mut pp: *mut png_sPLT_t = null();
                    let r = (api.png_get_sPLT)(png, info, &mut pp);
                    if !pp.is_null() {
                        let s = &*pp;
                        p!("  get_sPLT r={} name={:?} depth={} nentries={}",
                           r, cstr(s.name), s.depth, s.nentries);
                    } else {
                        p!("  get_sPLT r={} <null>", r);
                    }
                });
        }
    }
}

/// Emit and read back a single png_text of the given compression type.
unsafe fn write_text(api: &Api, rng: &mut Rng, comp: c_int, key: &str, text_len: usize, tag: &str) {
    let k = cs(key);
    let txt: Vec<c_char> = {
        let mut v: Vec<c_char> =
            rng.image_bytes(text_len).iter().map(|b| ((*b & 0x7f) | 0x20) as c_char).collect();
        v.push(0);
        v
    };
    let t = png_text {
        compression: comp,
        key: k.as_ptr() as *mut c_char,
        text: txt.as_ptr() as *mut c_char,
        text_length: text_len,
        itxt_length: 0,
        lang: null(),
        lang_key: null(),
    };
    anc_image(api, rng, 2, 8, 8, 8, tag,
        |png, info| { (api.png_set_text)(png, info, &t, 1); },
        |png, info| {
            let mut tp: *mut png_text = null();
            let mut num = 0i32;
            let r = (api.png_get_text)(png, info, &mut tp, &mut num);
            p!("  get_text r={} num={}", r, num);
            if !tp.is_null() && num > 0 {
                let e = &*tp;
                p!("    key={:?} comp={} tlen={}", cstr(e.key), e.compression, e.text_length);
            }
        });
}

fn anc_text(api: &Api, rng: &mut Rng) {
    unsafe {
        // PNG_TEXT_COMPRESSION_NONE == -1
        for (key, tl) in [("K", 0usize), ("Title", 1), ("Comment", 8191)] {
            write_text(api, rng, -1, key, tl, &format!("text {} {}", key, tl));
        }
    }
}

fn anc_ztxt(api: &Api, rng: &mut Rng) {
    unsafe {
        // PNG_TEXT_COMPRESSION_zTXt == 0
        for lvl in [0i32, 1, 6, 9] {
            let key = "Comment";
            let tl = 512usize;
            let k = cs(key);
            let txt: Vec<c_char> = {
                let mut v: Vec<c_char> =
                    rng.image_bytes(tl).iter().map(|b| ((*b & 0x7f) | 0x20) as c_char).collect();
                v.push(0);
                v
            };
            let t = png_text {
                compression: 0,
                key: k.as_ptr() as *mut c_char,
                text: txt.as_ptr() as *mut c_char,
                text_length: tl,
                itxt_length: 0,
                lang: null(),
                lang_key: null(),
            };
            anc_image(api, rng, 2, 8, 8, 8, &format!("ztxt lvl={}", lvl),
                |png, info| {
                    (api.png_set_text_compression_level)(png, lvl);
                    (api.png_set_text)(png, info, &t, 1);
                },
                |png, info| {
                    let mut tp: *mut png_text = null();
                    let mut num = 0i32;
                    let r = (api.png_get_text)(png, info, &mut tp, &mut num);
                    p!("  get_text r={} num={}", r, num);
                });
        }
    }
}

fn anc_itxt(api: &Api, rng: &mut Rng) {
    unsafe {
        // PNG_TEXT_COMPRESSION_ITXT_COMPRESSION_NONE == 1, _zTXt(iTXt) == 2
        for (comp, with_lang) in [(1i32, false), (2, false), (1, true), (2, true)] {
            let key = cs("Comment");
            let tl = 200usize;
            let txt: Vec<c_char> = {
                let mut v: Vec<c_char> =
                    rng.image_bytes(tl).iter().map(|b| ((*b & 0x7f) | 0x20) as c_char).collect();
                v.push(0);
                v
            };
            let lang = cs("en");
            let langkey = cs("Comment");
            let t = png_text {
                compression: comp,
                key: key.as_ptr() as *mut c_char,
                text: txt.as_ptr() as *mut c_char,
                text_length: 0,
                itxt_length: tl,
                lang: if with_lang { lang.as_ptr() as *mut c_char } else { null() },
                lang_key: if with_lang { langkey.as_ptr() as *mut c_char } else { null() },
            };
            anc_image(api, rng, 2, 8, 8, 8,
                &format!("itxt comp={} lang={}", comp, with_lang as u8),
                |png, info| { (api.png_set_text)(png, info, &t, 1); },
                |png, info| {
                    let mut tp: *mut png_text = null();
                    let mut num = 0i32;
                    let r = (api.png_get_text)(png, info, &mut tp, &mut num);
                    p!("  get_text r={} num={}", r, num);
                });
        }
    }
}

fn anc_exif(api: &Api, rng: &mut Rng) {
    unsafe {
        // Both TIFF byte orders (II / MM), various lengths.
        for (order, len) in [(*b"II", 4usize), (*b"MM", 8), (*b"II", 1024)] {
            let mut exif = rng.image_bytes(len);
            if len >= 2 {
                exif[0] = order[0];
                exif[1] = order[1];
            }
            anc_image(api, rng, 2, 8, 8, 8,
                &format!("exif {:?} {}", order, len),
                |png, info| {
                    (api.png_set_eXIf_1)(png, info, len as u32, exif.as_ptr() as *mut u8);
                },
                |png, info| {
                    let mut n = 0u32;
                    let mut pp: *mut u8 = null();
                    let r = (api.png_get_eXIf_1)(png, info, &mut n, &mut pp);
                    p!("  get_eXIf_1 r={} len={}", r, n);
                });
        }
    }
}

fn anc_cicp(api: &Api, rng: &mut Rng) {
    unsafe {
        let prim = rng.u8();
        let trc = rng.u8();
        anc_image(api, rng, 2, 8, 8, 8, "cicp",
            |png, info| { (api.png_set_cICP)(png, info, prim, trc, 0, 1); },
            |png, info| {
                let (mut a, mut b, mut c, mut d) = (0u8, 0u8, 0u8, 0u8);
                let r = (api.png_get_cICP)(png, info, &mut a, &mut b, &mut c, &mut d);
                p!("  get_cICP r={} prim={} trc={} matrix={} range={}", r, a, b, c, d);
            });
    }
}

fn anc_clli(api: &Api, rng: &mut Rng) {
    unsafe {
        for (mx, avg) in [(0u32, 0u32), (1, 1), (0x7fff_ffff, 0x7fff_ffff)] {
            anc_image(api, rng, 2, 16, 8, 8, &format!("clli_fixed {} {}", mx, avg),
                |png, info| { (api.png_set_cLLI_fixed)(png, info, mx, avg); },
                |png, info| {
                    let (mut a, mut b) = (0u32, 0u32);
                    let r = (api.png_get_cLLI_fixed)(png, info, &mut a, &mut b);
                    p!("  get_cLLI_fixed r={} max={} avg={}", r, a, b);
                });
        }
        anc_image(api, rng, 2, 16, 8, 8, "clli_float",
            |png, info| { (api.png_set_cLLI)(png, info, 100.0, 50.0); },
            |png, info| {
                let (mut a, mut b) = (0f64, 0f64);
                let r = (api.png_get_cLLI)(png, info, &mut a, &mut b);
                p!("  get_cLLI r={} max={:.4} avg={:.4}", r, a, b);
            });
    }
}

fn anc_mdcv(api: &Api, rng: &mut Rng) {
    unsafe {
        anc_image(api, rng, 2, 16, 8, 8, "mdcv_float",
            |png, info| {
                (api.png_set_mDCV)(png, info,
                    0.64, 0.33, 0.30, 0.60, 0.15, 0.06, 0.3127, 0.3290,
                    1000.0, 0.005);
            },
            |png, info| {
                let mut v = [0f64; 8];
                let (mut mx, mut mn) = (0f64, 0f64);
                let r = (api.png_get_mDCV)(png, info,
                    &mut v[0], &mut v[1], &mut v[2], &mut v[3], &mut v[4],
                    &mut v[5], &mut v[6], &mut v[7], &mut mx, &mut mn);
                p!("  get_mDCV r={} max={:.4} min={:.4}", r, mx, mn);
            });
        anc_image(api, rng, 2, 16, 8, 8, "mdcv_fixed",
            |png, info| {
                (api.png_set_mDCV_fixed)(png, info,
                    32000, 16500, 15000, 30000, 7500, 3000, 15635, 16450,
                    10000000, 50);
            },
            |png, info| {
                let mut v = [0i32; 8];
                let (mut mx, mut mn) = (0u32, 0u32);
                let r = (api.png_get_mDCV_fixed)(png, info,
                    &mut v[0], &mut v[1], &mut v[2], &mut v[3], &mut v[4],
                    &mut v[5], &mut v[6], &mut v[7], &mut mx, &mut mn);
                p!("  get_mDCV_fixed r={} max={} min={}", r, mx, mn);
            });
    }
}

fn anc_unknown(api: &Api, rng: &mut Rng) {
    unsafe {
        // Write unknown chunks at each location. Locations: HAVE_IHDR (before
        // PLTE), HAVE_PLTE (between PLTE and IDAT), AFTER_IDAT.
        for &(loc, lname) in &[
            (PNG_HAVE_IHDR as u8, "ihdr"),
            (PNG_HAVE_PLTE as u8, "plte"),
            (PNG_AFTER_IDAT as u8, "afteridat"),
        ] {
            let data = rng.image_bytes(20);
            let mut chunk = png_unknown_chunk {
                name: *b"prVt\0",
                data: data.as_ptr() as *mut u8,
                size: data.len(),
                location: loc,
            };
            let mut wr = Writer::new(api);
            (api.png_set_IHDR)(
                wr.png, wr.info, 8, 8, 8, PNG_COLOR_TYPE_RGB,
                PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
            );
            // Ensure libpng keeps/writes unknown chunks.
            let keep = *b"prVt";
            (api.png_set_keep_unknown_chunks)(
                wr.png, PNG_HANDLE_CHUNK_ALWAYS, keep.as_ptr(), 1);
            (api.png_set_unknown_chunks)(wr.png, wr.info, &mut chunk, 1);
            (api.png_set_unknown_chunk_location)(wr.png, wr.info, 0, loc as c_int);
            (api.png_write_info)(wr.png, wr.info);
            let rb = rowbytes_of(8, 8, PNG_COLOR_TYPE_RGB as u8);
            for _ in 0..8 {
                let row = rng.image_bytes(rb);
                (api.png_write_row)(wr.png, row.as_ptr());
            }
            (api.png_write_end)(wr.png, wr.info);
            emit_bytes(&format!("unknown loc={}", lname), wbuf());
            wr.destroy();
        }
        // Exercise png_set_keep_unknown_chunks / png_handle_as_unknown modes.
        for mode in [0i32, 1, 2, 3, 4] {
            let png = (api.png_create_write_struct)(
                cptr(PNG_LIBPNG_VER_STRING), vnull(), Some(err_fn), Some(warn_fn));
            let names = *b"prVt";
            (api.png_set_keep_unknown_chunks)(png, mode, names.as_ptr(), 1);
            p!("keep mode={} handle_as={}", mode,
               (api.png_handle_as_unknown)(png, b"prVt".as_ptr()));
            let mut pp = png;
            (api.png_destroy_write_struct)(&mut pp, null());
        }
    }
}

fn anc_everything(api: &Api, rng: &mut Rng) {
    unsafe {
        // Maximal info struct: set as many ancillary chunks as legal on one
        // RGB8 image, write it, then read them all back.
        let mut wr = Writer::new(api);
        (api.png_set_IHDR)(
            wr.png, wr.info, 8, 8, 8, PNG_COLOR_TYPE_RGB,
            PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE,
        );
        (api.png_set_gAMA_fixed)(wr.png, wr.info, 45455);
        (api.png_set_cHRM_fixed)(wr.png, wr.info, 31270, 32900, 64000, 33000,
                                 30000, 60000, 15000, 6000);
        let sig = png_color_8 { red: 8, green: 8, blue: 8, gray: 8, alpha: 8 };
        (api.png_set_sBIT)(wr.png, wr.info, &sig);
        let bg = png_color_16 { index: 0, red: 100, green: 120, blue: 140, gray: 0 };
        (api.png_set_bKGD)(wr.png, wr.info, &bg);
        (api.png_set_pHYs)(wr.png, wr.info, 2835, 2835, 1);
        (api.png_set_oFFs)(wr.png, wr.info, 10, 20, 0);
        let t = png_time { year: 2024, month: 1, day: 2, hour: 3, minute: 4, second: 5 };
        (api.png_set_tIME)(wr.png, wr.info, &t);
        let key = cs("Comment");
        let txt = cs("everything");
        let text = png_text {
            compression: -1,
            key: key.as_ptr() as *mut c_char,
            text: txt.as_ptr() as *mut c_char,
            text_length: 10,
            itxt_length: 0,
            lang: null(),
            lang_key: null(),
        };
        (api.png_set_text)(wr.png, wr.info, &text, 1);

        (api.png_write_info)(wr.png, wr.info);
        let rb = rowbytes_of(8, 8, PNG_COLOR_TYPE_RGB as u8);
        for _ in 0..8 {
            let row = rng.image_bytes(rb);
            (api.png_write_row)(wr.png, row.as_ptr());
        }
        (api.png_write_end)(wr.png, wr.info);

        // Read every value back.
        p!("valid={:08x}", (api.png_get_valid)(wr.png, wr.info, 0xffff_ffff));
        let mut g = 0i32;
        p!("gAMA r={} v={}", (api.png_get_gAMA_fixed)(wr.png, wr.info, &mut g), g);
        let (mut rx, mut ry, mut ru) = (0u32, 0u32, 0i32);
        p!("pHYs r={} {} {} {}",
           (api.png_get_pHYs)(wr.png, wr.info, &mut rx, &mut ry, &mut ru), rx, ry, ru);
        emit_bytes("everything", wbuf());
        wr.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* B77..B82 — W-SIMPLE (png_image_write_*)                             */
/* ------------------------------------------------------------------ */

/// Number of channels implied by a simplified-API format's flag bits.
fn format_channels(format: png_uint_32) -> usize {
    let color = format & PNG_FORMAT_FLAG_COLOR != 0;
    let alpha = format & PNG_FORMAT_FLAG_ALPHA != 0;
    let colormap = format & PNG_FORMAT_FLAG_COLORMAP != 0;
    if colormap {
        return 1;
    }
    (if color { 3 } else { 1 }) + (if alpha { 1 } else { 0 })
}

/// Bytes per channel implied by the LINEAR flag.
fn format_bpc(format: png_uint_32) -> usize {
    if format & PNG_FORMAT_FLAG_LINEAR != 0 { 2 } else { 1 }
}

/// Build a png_image and its input buffer for a given format, then drive
/// png_image_write_to_memory and record the result.
unsafe fn simple_write(
    api: &Api,
    rng: &mut Rng,
    format: png_uint_32,
    w: u32,
    h: u32,
    convert8: c_int,
    colormap_entries: u32,
    tag: &str,
) {
    let chans = format_channels(format);
    let bpc = format_bpc(format);
    let row_stride = w as usize * chans; // in samples
    let sample_bytes = bpc;

    let mut image = png_image {
        opaque: std::ptr::null_mut(),
        version: PNG_IMAGE_VERSION,
        width: w,
        height: h,
        format,
        flags: 0,
        colormap_entries,
        warning_or_error: 0,
        message: [0; 64],
    };

    // Input pixel buffer: height * row_stride samples of `bpc` bytes each.
    let nsamples = h as usize * row_stride;
    let buf = rng.image_bytes(nsamples * sample_bytes);

    // Colormap (only used for colormapped formats): colormap_entries pixels in
    // the base (non-colormap) format channel count.
    let cmap_chans = format_channels(format & !PNG_FORMAT_FLAG_COLORMAP);
    let cmap = rng.image_bytes(colormap_entries as usize * cmap_chans * bpc);
    let cmap_ptr = if colormap_entries > 0 {
        cmap.as_ptr() as *const c_void
    } else {
        std::ptr::null()
    };

    // First get the needed size, then write into it.
    let mut size: u64 = 0;
    let r0 = (api.png_image_write_to_memory)(
        &mut image, std::ptr::null_mut(), &mut size, convert8,
        buf.as_ptr() as *const c_void, row_stride as i32, cmap_ptr);
    p!("{}: size r={} bytes={} woe={}", tag, r0, size, image.warning_or_error);
    if r0 == 0 {
        p!("  msg={:?}", image.msg());
        return;
    }
    let mut out = vec![0u8; size as usize];
    let mut size2 = size;
    let r1 = (api.png_image_write_to_memory)(
        &mut image, out.as_mut_ptr() as *mut c_void, &mut size2, convert8,
        buf.as_ptr() as *const c_void, row_stride as i32, cmap_ptr);
    out.truncate(size2 as usize);
    p!("  write r={} bytes={} woe={} msg={:?}", r1, size2, image.warning_or_error, image.msg());
    emit_bytes(tag, &out);
}

fn simple_formats(api: &Api, rng: &mut Rng) {
    unsafe {
        // 16 non-colormap combinations of ALPHA|COLOR|LINEAR|BGR|AFIRST.
        // BGR only meaningful with COLOR; AFIRST only with ALPHA — but the
        // simplified API accepts the full cross-product, so exercise 16 by
        // iterating the 4 primary bits (ALPHA, COLOR, LINEAR, BGR).
        for bits in 0u32..16 {
            let mut format = 0u32;
            if bits & 1 != 0 { format |= PNG_FORMAT_FLAG_ALPHA; }
            if bits & 2 != 0 { format |= PNG_FORMAT_FLAG_COLOR; }
            if bits & 4 != 0 { format |= PNG_FORMAT_FLAG_LINEAR; }
            if bits & 8 != 0 { format |= PNG_FORMAT_FLAG_BGR; }
            simple_write(api, rng, format, 12, 9, 1, 0,
                &format!("sformat {:#04x}", format));
        }
    }
}

fn simple_colormap(api: &Api, rng: &mut Rng) {
    unsafe {
        for bits in 0u32..8 {
            let mut format = PNG_FORMAT_FLAG_COLORMAP;
            if bits & 1 != 0 { format |= PNG_FORMAT_FLAG_ALPHA; }
            if bits & 2 != 0 { format |= PNG_FORMAT_FLAG_COLOR; }
            if bits & 4 != 0 { format |= PNG_FORMAT_FLAG_BGR; }
            simple_write(api, rng, format, 10, 8, 1, 64,
                &format!("scmap {:#04x}", format));
        }
    }
}

fn simple_convert8(api: &Api, rng: &mut Rng) {
    unsafe {
        // Linear (16-bit) source, converting to 8-bit and not.
        let format = PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_LINEAR;
        for c8 in [0i32, 1] {
            simple_write(api, rng, format, 12, 9, c8,
                0, &format!("sconvert8 {}", c8));
        }
    }
}

fn simple_stride(api: &Api, rng: &mut Rng) {
    unsafe {
        let format = PNG_FORMAT_FLAG_COLOR;
        let (w, h) = (12u32, 9u32);
        let chans = format_channels(format);
        let min_stride = w as usize * chans;
        // Positive minimum stride (0 lets libpng compute it), a larger stride,
        // and a negative (bottom-up) stride.
        for (name, stride) in [
            ("zero", 0i32),
            ("min", min_stride as i32),
            ("larger", (min_stride + chans) as i32),
            ("negative", -(min_stride as i32)),
        ] {
            let bpc = format_bpc(format);
            // Provide a buffer big enough for the larger stride; for negative
            // stride the simplified API reads bottom-up from the last row.
            let abs = stride.unsigned_abs() as usize;
            let per_row = if abs == 0 { min_stride } else { abs };
            let nsamples = h as usize * per_row;
            let buf = rng.image_bytes(nsamples * bpc);

            let mut image = png_image {
                opaque: std::ptr::null_mut(),
                version: PNG_IMAGE_VERSION,
                width: w, height: h, format,
                flags: 0, colormap_entries: 0, warning_or_error: 0,
                message: [0; 64],
            };
            let mut size: u64 = 0;
            // A negative stride means "bottom-most row first in the buffer".
            // libpng itself does `row += (height-1) * (-row_step)` (pngwrite.c
            // png_image_write_main), so the caller passes the base of the
            // allocation for both signs; offsetting here as well would read
            // past the end of the buffer.
            let base = buf.as_ptr();
            let r0 = (api.png_image_write_to_memory)(
                &mut image, std::ptr::null_mut(), &mut size, 1,
                base as *const c_void, stride, std::ptr::null());
            p!("sstride {}: size r={} bytes={} woe={}", name, r0, size, image.warning_or_error);
            if r0 == 0 { p!("  msg={:?}", image.msg()); continue; }
            let mut out = vec![0u8; size as usize];
            let mut size2 = size;
            let r1 = (api.png_image_write_to_memory)(
                &mut image, out.as_mut_ptr() as *mut c_void, &mut size2, 1,
                base as *const c_void, stride, std::ptr::null());
            out.truncate(size2 as usize);
            p!("  write r={} woe={} msg={:?}", r1, image.warning_or_error, image.msg());
            emit_bytes(&format!("sstride {}", name), &out);
        }
    }
}

fn simple_sizing(api: &Api, rng: &mut Rng) {
    unsafe {
        // Two-pass sizing: first NULL memory to get size, then fill. This is
        // exactly what simple_write does, so exercise a few formats here.
        for format in [
            PNG_FORMAT_FLAG_COLOR,
            PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA,
            0u32,
        ] {
            simple_write(api, rng, format, 16, 12, 1, 0,
                &format!("ssizing {:#04x}", format));
        }
    }
}

fn simple_file(api: &Api, rng: &mut Rng) {
    unsafe {
        let format = PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA;
        let (w, h) = (12u32, 9u32);
        let chans = format_channels(format);
        let row_stride = w as usize * chans;
        let buf = rng.image_bytes(h as usize * row_stride);

        // FIXED filename derived from the case so both workers agree; not pid-
        // or random-based.
        let mut path = std::env::temp_dir();
        path.push("wr_simple_file_case.png");
        let path_c = {
            let s = path.to_string_lossy();
            let mut v: Vec<c_char> = s.bytes().map(|b| b as c_char).collect();
            v.push(0);
            v
        };

        let mut image = png_image {
            opaque: std::ptr::null_mut(),
            version: PNG_IMAGE_VERSION,
            width: w, height: h, format,
            flags: 0, colormap_entries: 0, warning_or_error: 0,
            message: [0; 64],
        };
        let r = (api.png_image_write_to_file)(
            &mut image, path_c.as_ptr(), 1,
            buf.as_ptr() as *const c_void, row_stride as i32, std::ptr::null());
        p!("sfile write r={} woe={} msg={:?}", r, image.warning_or_error, image.msg());
        if r != 0 {
            match std::fs::read(&path) {
                Ok(bytes) => emit_bytes("sfile", &bytes),
                Err(e) => p!("sfile read err={}", e.kind() as i32),
            }
        }
        let _ = std::fs::remove_file(&path);
    }
}
