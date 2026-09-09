//! Phase B rows 83..165 — the read pipeline (`R-LOW`, `R-ROWS`, `R-IMAGE`,
//! `R-PNG`, `R-PROG`, `R-SIMPLE`), every read transform, the A10 global-state
//! toggles, and the composed round trips.

use crate::api::Api;
use crate::p;
use crate::support::*;
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

pub fn run(api: &Api, case: &str, seed: u64) -> bool {
    let mut rng = Rng::new(seed);
    match case {
        "rd/low_all" => low_all(api, &mut rng),
        "rd/low_interlaced" => low_interlaced(api, &mut rng),
        "rd/low_interlaced_manual" => low_interlaced_manual(api, &mut rng),
        "rd/rows_api" => rows_api(api, &mut rng),
        "rd/image_api" => image_api(api, &mut rng),
        "rd/png_identity" => png_identity(api, &mut rng),
        "rd/png_transforms" => png_transforms(api, &mut rng),
        "rd/png_transform_combos" => png_transform_combos(api, &mut rng),
        "rd/prog_chunked" => prog_chunked(api, &mut rng),
        "rd/prog_interlaced" => prog_interlaced(api, &mut rng),
        "rd/prog_pause" => prog_pause(api, &mut rng),
        "rd/prog_partial_cbs" => prog_partial_cbs(api, &mut rng),
        "rd/simple_formats" => simple_formats(api, &mut rng),
        "rd/simple_colormap" => simple_colormap(api, &mut rng),
        "rd/simple_background" => simple_background(api, &mut rng),
        "rd/simple_stride" => simple_stride(api, &mut rng),
        "rd/simple_file" => simple_file(api, &mut rng),
        "rd/tr_palette_to_rgb" => tr_palette_to_rgb(api, &mut rng),
        "rd/tr_expand" => tr_expand(api, &mut rng),
        "rd/tr_expand_gray" => tr_expand_gray(api, &mut rng),
        "rd/tr_expand_16" => tr_expand_16(api, &mut rng),
        "rd/tr_gray_to_rgb" => tr_gray_to_rgb(api, &mut rng),
        "rd/tr_rgb_to_gray" => tr_rgb_to_gray(api, &mut rng),
        "rd/tr_strip_16" => tr_strip_16(api, &mut rng),
        "rd/tr_scale_16" => tr_scale_16(api, &mut rng),
        "rd/tr_strip_alpha" => tr_strip_alpha(api, &mut rng),
        "rd/tr_swap_alpha" => tr_swap_alpha(api, &mut rng),
        "rd/tr_invert_alpha" => tr_invert_alpha(api, &mut rng),
        "rd/tr_filler" => tr_filler(api, &mut rng),
        "rd/tr_add_alpha" => tr_add_alpha(api, &mut rng),
        "rd/tr_packing" => tr_packing(api, &mut rng),
        "rd/tr_packswap" => tr_packswap(api, &mut rng),
        "rd/tr_shift" => tr_shift(api, &mut rng),
        "rd/tr_swap" => tr_swap(api, &mut rng),
        "rd/tr_bgr" => tr_bgr(api, &mut rng),
        "rd/tr_invert_mono" => tr_invert_mono(api, &mut rng),
        "rd/tr_gamma_fixed" => tr_gamma_fixed(api, &mut rng),
        "rd/tr_gamma_float" => tr_gamma_float(api, &mut rng),
        "rd/tr_gamma_gama_chunk" => tr_gamma_gama_chunk(api, &mut rng),
        "rd/tr_gamma_srgb_chunk" => tr_gamma_srgb_chunk(api, &mut rng),
        "rd/tr_alpha_mode_png" => tr_alpha_mode(api, &mut rng, PNG_ALPHA_PNG),
        "rd/tr_alpha_mode_std" => tr_alpha_mode(api, &mut rng, PNG_ALPHA_STANDARD),
        "rd/tr_alpha_mode_assoc" => tr_alpha_mode(api, &mut rng, 2),
        "rd/tr_alpha_mode_opt" => tr_alpha_mode(api, &mut rng, PNG_ALPHA_OPTIMIZED),
        "rd/tr_alpha_mode_broken" => tr_alpha_mode(api, &mut rng, PNG_ALPHA_BROKEN),
        "rd/tr_bg_screen" => tr_bg_screen(api, &mut rng),
        "rd/tr_bg_file" => tr_bg_file(api, &mut rng),
        "rd/tr_bg_unique" => tr_bg_unique(api, &mut rng),
        "rd/tr_bg_palette" => tr_bg_palette(api, &mut rng),
        "rd/tr_bg_trns" => tr_bg_trns(api, &mut rng),
        "rd/tr_quantize_rgb" => tr_quantize_rgb(api, &mut rng),
        "rd/tr_quantize_palette" => tr_quantize_palette(api, &mut rng),
        "rd/tr_user" => tr_user(api, &mut rng),
        "rd/status_fn" => status_fn(api, &mut rng),
        "rd/tr_pipelines" => tr_pipelines(api, &mut rng),
        "rd/benign_errors" => benign_errors(api, &mut rng),
        "rd/crc_action_grid" => crc_action_grid(api, &mut rng),
        "rd/check_invalid_index" => check_invalid_index(api, &mut rng),
        "rd/user_limits" => user_limits(api, &mut rng),
        "rd/chunk_cache_max" => chunk_cache_max(api, &mut rng),
        "rd/chunk_malloc_max" => chunk_malloc_max(api, &mut rng),
        "rd/opt_inflate_window" => opt_inflate_window(api, &mut rng),
        "rd/opt_skip_srgb" => opt_skip_srgb(api, &mut rng),
        "rd/opt_ignore_adler" => opt_ignore_adler(api, &mut rng),
        "rd/keep_unknown" => keep_unknown(api, &mut rng),
        "rd/user_chunk_fn" => user_chunk_fn(api, &mut rng),
        "rd/mem_fn" => mem_fn(api, &mut rng),
        "rd/sig_bytes" => sig_bytes(api, &mut rng),
        "rd/anc_after_idat" => anc_after_idat(api, &mut rng),
        "rd/idat_split" => idat_split(api, &mut rng),
        "rd/update_info_report" => update_info_report(api, &mut rng),
        "rd/roundtrip_low" => roundtrip_low(api, &mut rng),
        "rd/roundtrip_png" => roundtrip_png(api, &mut rng),
        "rd/roundtrip_simple" => roundtrip_simple(api, &mut rng),
        "rd/io_state" => io_state(api, &mut rng),
        _ => return false,
    }
    true
}

/* ================================================================== */
/* shared helpers                                                     */
/* ================================================================== */

/// Report the post-`png_read_info` header the same way for every case.
unsafe fn report_hdr(api: &Api, rd: &Reader, tag: &str) {
    p!("{} ct={} bd={} {}x{} il={} rowbytes={} channels={}",
       tag,
       (api.png_get_color_type)(rd.png, rd.info),
       (api.png_get_bit_depth)(rd.png, rd.info),
       (api.png_get_image_width)(rd.png, rd.info),
       (api.png_get_image_height)(rd.png, rd.info),
       (api.png_get_interlace_type)(rd.png, rd.info),
       (api.png_get_rowbytes)(rd.png, rd.info),
       (api.png_get_channels)(rd.png, rd.info));
}

/// Report metadata after `png_read_update_info` (the values transforms mutate).
unsafe fn report_updated(api: &Api, rd: &Reader, tag: &str) {
    p!("{} upd ct={} bd={} ch={} rowbytes={}",
       tag,
       (api.png_get_color_type)(rd.png, rd.info),
       (api.png_get_bit_depth)(rd.png, rd.info),
       (api.png_get_channels)(rd.png, rd.info),
       (api.png_get_rowbytes)(rd.png, rd.info));
}

/// Encode a single image with the low-level write API and return the PNG bytes.
unsafe fn encode(
    api: &Api,
    rng: &mut Rng,
    w: u32,
    h: u32,
    depth: u8,
    color: u8,
    interlace: c_int,
) -> Vec<u8> {
    let mut wr = Writer::new(api);
    (api.png_set_IHDR)(
        wr.png,
        wr.info,
        w,
        h,
        depth as c_int,
        color as c_int,
        interlace,
        PNG_COMPRESSION_TYPE_BASE,
        PNG_FILTER_TYPE_BASE,
    );
    let pal: Vec<png_color>;
    if color == 3 {
        let n = 1usize << depth.min(8);
        pal = (0..n)
            .map(|_| png_color { red: rng.u8(), green: rng.u8(), blue: rng.u8() })
            .collect();
        (api.png_set_PLTE)(wr.png, wr.info, pal.as_ptr(), n as c_int);
    }
    (api.png_write_info)(wr.png, wr.info);
    let rb = rowbytes_of(w, depth, color);
    let mut rows: Vec<Vec<u8>> = (0..h as usize).map(|_| rng.image_bytes(rb)).collect();
    let npass = if interlace == PNG_INTERLACE_ADAM7 { 7 } else { 1 };
    for _ in 0..npass {
        for r in rows.iter_mut() {
            (api.png_write_row)(wr.png, r.as_ptr());
        }
    }
    (api.png_write_end)(wr.png, wr.info);
    let out = wbuf().clone();
    wr.destroy();
    out
}

/// Decode every row with the low-level sequential API and return the bytes.
unsafe fn read_all_rows(api: &Api, rd: &Reader, h: u32, passes: c_int) -> Vec<u8> {
    let rb = (api.png_get_rowbytes)(rd.png, rd.info);
    let mut all = Vec::new();
    for _ in 0..passes {
        for _ in 0..h {
            let mut row = vec![0u8; rb + 8];
            (api.png_read_row)(rd.png, row.as_mut_ptr(), null());
            all.extend_from_slice(&row[..rb]);
        }
    }
    all
}

const A1_16: [(u8, u8); 4] = [(0, 16), (2, 16), (4, 16), (6, 16)];
const A1_ALPHA: [(u8, u8); 4] = [(4, 8), (4, 16), (6, 8), (6, 16)];
const GRAY_SUB: [(u8, u8); 3] = [(0, 1), (0, 2), (0, 4)];
const PAL: [(u8, u8); 4] = [(3, 1), (3, 2), (3, 4), (3, 8)];
const SMALL: [(u32, u32); 4] = [(1, 1), (5, 5), (8, 8), (17, 3)];

/* ================================================================== */
/* B83..B90 — read drivers                                            */
/* ================================================================== */

/// B83 — every A1 combination, non-interlaced, no transforms.
fn low_all(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            for (w, h) in SHAPES {
                let data = build_png(rng, w, h, depth, color, &[], &[]);
                let mut rd = Reader::new(api, data);
                (api.png_read_info)(rd.png, rd.info);
                report_hdr(api, &rd, &format!("hdr ct={} bd={} {}x{}", color, depth, w, h));
                (api.png_read_update_info)(rd.png, rd.info);
                let all = read_all_rows(api, &rd, h, 1);
                (api.png_read_end)(rd.png, rd.info);
                emit_bytes(&format!("rows ct={} bd={} {}x{}", color, depth, w, h), &all);
                rd.destroy();
            }
        }
    }
}

/// B84 — interlaced input with `png_set_interlace_handling`; all 7 passes.
fn low_interlaced(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            for (w, h) in SMALL {
                let data = encode(api, rng, w, h, depth, color, PNG_INTERLACE_ADAM7);
                let mut rd = Reader::new(api, data);
                (api.png_read_info)(rd.png, rd.info);
                let passes = (api.png_set_interlace_handling)(rd.png);
                report_hdr(api, &rd, &format!("hdr ct={} bd={} {}x{} passes={}", color, depth, w, h, passes));
                (api.png_read_update_info)(rd.png, rd.info);
                let all = read_all_rows(api, &rd, h, passes);
                (api.png_read_end)(rd.png, rd.info);
                emit_bytes(&format!("rows ct={} bd={} {}x{}", color, depth, w, h), &all);
                rd.destroy();
            }
        }
    }
}

/// B85 — interlaced input WITHOUT `png_set_interlace_handling`. libpng then
/// returns the raw sparse pass data; the caller must know the pass geometry.
/// We simply read `height` rows once (pass 1) and record what comes back — the
/// point is that both libraries agree on the un-deinterlaced behaviour.
fn low_interlaced_manual(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            for (w, h) in SMALL {
                let data = encode(api, rng, w, h, depth, color, PNG_INTERLACE_ADAM7);
                let mut rd = Reader::new(api, data);
                (api.png_read_info)(rd.png, rd.info);
                report_hdr(api, &rd, &format!("hdr ct={} bd={} {}x{}", color, depth, w, h));
                (api.png_read_update_info)(rd.png, rd.info);
                // Manual Adam7: read each pass's rows using png_read_rows with a
                // display buffer so libpng expands the sparse pixels for us.
                let rb = (api.png_get_rowbytes)(rd.png, rd.info);
                let mut all = Vec::new();
                for _ in 0..7 {
                    for _ in 0..h {
                        let mut row = vec![0u8; rb + 8];
                        let mut disp = vec![0u8; rb + 8];
                        let mut rp = row.as_mut_ptr();
                        let mut dp = disp.as_mut_ptr();
                        (api.png_read_rows)(rd.png, &mut rp, &mut dp, 1);
                        all.extend_from_slice(&disp[..rb]);
                    }
                }
                (api.png_read_end)(rd.png, rd.info);
                emit_bytes(&format!("rows ct={} bd={} {}x{}", color, depth, w, h), &all);
                rd.destroy();
            }
        }
    }
}

/// B86 — `png_read_rows` with row_pointers only, display_row only, and both.
fn rows_api(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(0u8, 8u8), (2, 8), (3, 8), (6, 8)] {
            for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                for mode in 0..3 {
                    let (w, h) = (8u32, 8u32);
                    let data = encode(api, rng, w, h, depth, color, il);
                    let mut rd = Reader::new(api, data);
                    (api.png_read_info)(rd.png, rd.info);
                    let passes = (api.png_set_interlace_handling)(rd.png);
                    (api.png_read_update_info)(rd.png, rd.info);
                    let rb = (api.png_get_rowbytes)(rd.png, rd.info);
                    let mut all = Vec::new();
                    for _ in 0..passes {
                        for _ in 0..h {
                            let mut row = vec![0u8; rb + 8];
                            let mut disp = vec![0u8; rb + 8];
                            let mut rp = row.as_mut_ptr();
                            let mut dp = disp.as_mut_ptr();
                            match mode {
                                0 => (api.png_read_rows)(rd.png, &mut rp, null::<*mut u8>() as *mut *mut u8, 1),
                                1 => (api.png_read_rows)(rd.png, null::<*mut u8>() as *mut *mut u8, &mut dp, 1),
                                _ => (api.png_read_rows)(rd.png, &mut rp, &mut dp, 1),
                            }
                            let src = if mode == 1 { &disp } else { &row };
                            all.extend_from_slice(&src[..rb]);
                        }
                    }
                    (api.png_read_end)(rd.png, rd.info);
                    emit_bytes(&format!("rows ct={} il={} mode={}", color, il, mode), &all);
                    rd.destroy();
                }
            }
        }
    }
}

/// B87 — `png_read_image` on all A1 combos, interlaced and not.
fn image_api(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                let (w, h) = (8u32, 8u32);
                let data = encode(api, rng, w, h, depth, color, il);
                let mut rd = Reader::new(api, data);
                (api.png_read_info)(rd.png, rd.info);
                report_hdr(api, &rd, &format!("hdr ct={} bd={} il={}", color, depth, il));
                (api.png_read_update_info)(rd.png, rd.info);
                let rb = (api.png_get_rowbytes)(rd.png, rd.info);
                let mut storage: Vec<Vec<u8>> = (0..h as usize).map(|_| vec![0u8; rb + 8]).collect();
                let mut ptrs: Vec<*mut u8> = storage.iter_mut().map(|v| v.as_mut_ptr()).collect();
                (api.png_read_image)(rd.png, ptrs.as_mut_ptr());
                (api.png_read_end)(rd.png, rd.info);
                let mut all = Vec::new();
                for v in &storage {
                    all.extend_from_slice(&v[..rb]);
                }
                emit_bytes(&format!("rows ct={} bd={} il={}", color, depth, il), &all);
                rd.destroy();
            }
        }
    }
}

/// Read back with `png_read_png` for a given transform mask, emitting the
/// gathered row pointers.
unsafe fn read_png_with(api: &Api, data: Vec<u8>, transforms: c_int, tag: &str) {
    let mut rd = Reader::new(api, data);
    (api.png_read_png)(rd.png, rd.info, transforms, null());
    report_hdr(api, &rd, tag);
    let h = (api.png_get_image_height)(rd.png, rd.info);
    let rb = (api.png_get_rowbytes)(rd.png, rd.info);
    let rows = (api.png_get_rows)(rd.png, rd.info);
    let mut all = Vec::new();
    if !rows.is_null() && rb > 0 {
        for i in 0..h as isize {
            let rp = *rows.offset(i);
            if !rp.is_null() {
                all.extend_from_slice(std::slice::from_raw_parts(rp, rb));
            }
        }
    }
    emit_bytes(&format!("{} rows", tag), &all);
    rd.destroy();
}

/// B88 — `png_read_png` with PNG_TRANSFORM_IDENTITY on all A1 combos.
fn png_identity(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            let data = build_png(rng, 8, 8, depth, color, &[], &[]);
            read_png_with(api, data, PNG_TRANSFORM_IDENTITY,
                          &format!("identity ct={} bd={}", color, depth));
        }
    }
}

/// B89 — `png_read_png` with each transform bit individually.
fn png_transforms(api: &Api, rng: &mut Rng) {
    unsafe {
        let txs = [
            ("strip16", PNG_TRANSFORM_STRIP_16, 16u8),
            ("scale16", PNG_TRANSFORM_SCALE_16, 16),
            ("stripalpha", PNG_TRANSFORM_STRIP_ALPHA, 8),
            ("packing", PNG_TRANSFORM_PACKING, 1),
            ("packswap", PNG_TRANSFORM_PACKSWAP, 1),
            ("expand", PNG_TRANSFORM_EXPAND, 8),
            ("expand16", PNG_TRANSFORM_EXPAND_16, 8),
            ("invmono", PNG_TRANSFORM_INVERT_MONO, 1),
            ("shift", PNG_TRANSFORM_SHIFT, 8),
            ("bgr", PNG_TRANSFORM_BGR, 8),
            ("swapalpha", PNG_TRANSFORM_SWAP_ALPHA, 8),
            ("swapendian", PNG_TRANSFORM_SWAP_ENDIAN, 16),
            ("invalpha", PNG_TRANSFORM_INVERT_ALPHA, 8),
            ("graytorgb", PNG_TRANSFORM_GRAY_TO_RGB, 8),
        ];
        for (color, depth) in A1 {
            for (name, bit, want_depth) in txs {
                // Only apply where the depth is meaningful for the transform.
                if want_depth == 16 && depth != 16 {
                    continue;
                }
                if want_depth == 1 && depth > 8 {
                    continue;
                }
                let data = build_png(rng, 8, 6, depth, color, &[], &[]);
                read_png_with(api, data, bit,
                              &format!("{} ct={} bd={}", name, color, depth));
            }
        }
    }
}

/// B90 — pairwise / triple transform masks.
fn png_transform_combos(api: &Api, rng: &mut Rng) {
    unsafe {
        let combos = [
            ("expand|gray2rgb", PNG_TRANSFORM_EXPAND | PNG_TRANSFORM_GRAY_TO_RGB, 0u8, 8u8),
            ("expand16|swapendian", PNG_TRANSFORM_EXPAND_16 | PNG_TRANSFORM_SWAP_ENDIAN, 2, 16),
            ("strip16|stripalpha", PNG_TRANSFORM_STRIP_16 | PNG_TRANSFORM_STRIP_ALPHA, 6, 16),
            ("pack|packswap|invmono", PNG_TRANSFORM_PACKING | PNG_TRANSFORM_PACKSWAP | PNG_TRANSFORM_INVERT_MONO, 0, 2),
        ];
        for (name, mask, color, depth) in combos {
            let data = build_png(rng, 8, 6, depth, color, &[], &[]);
            read_png_with(api, data, mask, &format!("{} ct={} bd={}", name, color, depth));
        }
    }
}

/* ================================================================== */
/* B91..B94 — progressive read                                        */
/* ================================================================== */

unsafe fn feed_progressive(
    api: &Api,
    data: &[u8],
    chunk: usize,
    info_cb: bool,
    row_cb: bool,
    end_cb: bool,
) {
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
        if info_cb { Some(prog_info_fn) } else { None },
        if row_cb { Some(prog_row_fn) } else { None },
        if end_cb { Some(prog_end_fn) } else { None },
    );
    let mut off = 0usize;
    while off < data.len() {
        let n = std::cmp::min(chunk, data.len() - off);
        (api.png_process_data)(png, info, data[off..off + n].as_ptr() as *mut u8, n);
        off += n;
    }
    let mut pp = png;
    let mut ip = info;
    (api.png_destroy_read_struct)(&mut pp, &mut ip, null());
}

/// B91 — chunk sizes 1,2,3,7,13,64,1024,whole; all A1 combos, non-interlaced.
fn prog_chunked(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            let data = build_png(rng, 8, 5, depth, color, &[], &[]);
            for &c in &[1usize, 2, 3, 7, 13, 64, 1024, usize::MAX] {
                let cs = if c == usize::MAX { data.len().max(1) } else { c };
                emit(&format!("prog ct={} bd={} chunk={}", color, depth, cs));
                feed_progressive(api, &data, cs, true, true, true);
            }
        }
    }
}

/// B92 — same, interlaced (drives png_progressive_combine_row).
fn prog_interlaced(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            let data = encode(api, rng, 8, 5, depth, color, PNG_INTERLACE_ADAM7);
            for &c in &[1usize, 7, 64, usize::MAX] {
                let cs = if c == usize::MAX { data.len().max(1) } else { c };
                emit(&format!("prog ct={} bd={} chunk={}", color, depth, cs));
                feed_progressive(api, &data, cs, true, true, true);
            }
        }
    }
}

/// B93 — png_process_data_pause with save = 0 and 1.
fn prog_pause(api: &Api, rng: &mut Rng) {
    unsafe {
        for save in [0i32, 1] {
            let data = build_png(rng, 8, 5, 8, 2, &[], &[]);
            let png = (api.png_create_read_struct)(
                PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
                std::ptr::null_mut(),
                Some(err_fn),
                Some(warn_fn),
            );
            let info = (api.png_create_info_struct)(png);
            (api.png_set_progressive_read_fn)(
                png, std::ptr::null_mut(),
                Some(prog_info_fn), Some(prog_row_fn), Some(prog_end_fn),
            );
            let mut off = 0usize;
            while off < data.len() {
                let n = std::cmp::min(16, data.len() - off);
                (api.png_process_data)(png, info, data[off..off + n].as_ptr() as *mut u8, n);
                let buffered = (api.png_process_data_pause)(png, save);
                p!("pause save={} buffered={}", save, buffered);
                off += n;
            }
            let mut pp = png;
            let mut ip = info;
            (api.png_destroy_read_struct)(&mut pp, &mut ip, null());
        }
    }
}

/// B94 — only info_fn, only row_fn, only end_fn, all three NULL.
fn prog_partial_cbs(api: &Api, rng: &mut Rng) {
    unsafe {
        let combos = [
            (true, false, false),
            (false, true, false),
            (false, false, true),
            (false, false, false),
        ];
        for (i, o, e) in combos {
            let data = build_png(rng, 8, 4, 8, 2, &[], &[]);
            emit(&format!("prog cbs info={} row={} end={}", i, o, e));
            feed_progressive(api, &data, 32, i, o, e);
        }
    }
}

/* ================================================================== */
/* B95..B99 — simplified read API                                     */
/* ================================================================== */

/// Channels implied by a PNG_FORMAT_* flag word (mirrors PNG_IMAGE_*_CHANNELS).
fn format_channels(fmt: png_uint_32) -> u32 {
    let color = (fmt & PNG_FORMAT_FLAG_COLOR) != 0;
    let alpha = (fmt & PNG_FORMAT_FLAG_ALPHA) != 0;
    let colormap = (fmt & PNG_FORMAT_FLAG_COLORMAP) != 0;
    if colormap {
        return 1;
    }
    let base = if color { 3 } else { 1 };
    base + if alpha { 1 } else { 0 }
}

fn format_bytes_per_channel(fmt: png_uint_32) -> u32 {
    if (fmt & PNG_FORMAT_FLAG_LINEAR) != 0 { 2 } else { 1 }
}

/// Run the simplified reader over `data` for output format `fmt`, emitting the
/// return value, status, message and the decoded buffer + colormap.
unsafe fn simple_read(api: &Api, data: &[u8], fmt: png_uint_32, background: bool, tag: &str) {
    let mut image = png_image::default();
    image.version = PNG_IMAGE_VERSION;
    image.opaque = std::ptr::null_mut();
    let r = (api.png_image_begin_read_from_memory)(
        &mut image,
        data.as_ptr() as *const c_void,
        data.len(),
    );
    p!("{} begin={} woe={} msg={:?} {}x{}",
       tag, r, image.warning_or_error, image.msg(), image.width, image.height);
    if r == 0 {
        (api.png_image_free)(&mut image);
        return;
    }
    image.format = fmt;
    let ch = format_channels(fmt);
    let bpc = format_bytes_per_channel(fmt);
    let stride = image.width * ch;
    let buf_len = (stride * image.height * bpc) as usize + 16;
    let mut buf = vec![0u8; buf_len];
    // A colourmapped output needs a colormap of up to 256 entries * channels.
    let cmap_ch = if (fmt & PNG_FORMAT_FLAG_COLORMAP) != 0 { format_channels(fmt & !PNG_FORMAT_FLAG_COLORMAP) } else { 0 };
    let mut cmap = vec![0u8; 256 * 4 * bpc as usize + 16];
    let bg = png_color { red: 0x20, green: 0x40, blue: 0x60 };
    let bgp: *const png_color = if background { &bg } else { std::ptr::null() };
    let fr = (api.png_image_finish_read)(
        &mut image,
        bgp,
        buf.as_mut_ptr() as *mut c_void,
        0,
        cmap.as_mut_ptr() as *mut c_void,
    );
    p!("{} finish={} woe={} msg={:?} cmap_entries={} fmt={:#x} ch={}",
       tag, fr, image.warning_or_error, image.msg(), image.colormap_entries, image.format, ch);
    let out_len = (stride * image.height * bpc) as usize;
    emit_bytes(&format!("{} buf", tag), &buf[..out_len.min(buf.len())]);
    if cmap_ch > 0 {
        let cl = (image.colormap_entries * cmap_ch * bpc) as usize;
        emit_bytes(&format!("{} cmap", tag), &cmap[..cl.min(cmap.len())]);
    }
    (api.png_image_free)(&mut image);
}

/// B95 — all 16 non-colormap output formats.
fn simple_formats(api: &Api, rng: &mut Rng) {
    unsafe {
        let data = encode(api, rng, 8, 6, 8, 6, PNG_INTERLACE_NONE);
        for bits in 0u32..16 {
            let fmt = (bits & 1) * PNG_FORMAT_FLAG_ALPHA
                | ((bits >> 1) & 1) * PNG_FORMAT_FLAG_COLOR
                | ((bits >> 2) & 1) * PNG_FORMAT_FLAG_LINEAR
                | ((bits >> 3) & 1) * PNG_FORMAT_FLAG_BGR;
            simple_read(api, &data, fmt, false, &format!("fmt={:#x}", fmt));
        }
    }
}

/// B96 — colourmapped output formats.
fn simple_colormap(api: &Api, rng: &mut Rng) {
    unsafe {
        let data = encode(api, rng, 8, 6, 8, 6, PNG_INTERLACE_NONE);
        for bits in 0u32..16 {
            let fmt = PNG_FORMAT_FLAG_COLORMAP
                | (bits & 1) * PNG_FORMAT_FLAG_ALPHA
                | ((bits >> 1) & 1) * PNG_FORMAT_FLAG_COLOR
                | ((bits >> 2) & 1) * PNG_FORMAT_FLAG_BGR
                | ((bits >> 3) & 1) * PNG_FORMAT_FLAG_AFIRST;
            simple_read(api, &data, fmt, false, &format!("cmap fmt={:#x}", fmt));
        }
    }
}

/// B97 — background supplied vs NULL where alpha is removed.
fn simple_background(api: &Api, rng: &mut Rng) {
    unsafe {
        let data = encode(api, rng, 8, 6, 8, 6, PNG_INTERLACE_NONE);
        // Output format without alpha, so libpng must composite/remove it.
        let fmt = PNG_FORMAT_FLAG_COLOR;
        simple_read(api, &data, fmt, true, "bg=set");
        simple_read(api, &data, fmt, false, "bg=null");
    }
}

/// B98 — negative row_stride (bottom-up) and stride larger than minimum.
fn simple_stride(api: &Api, rng: &mut Rng) {
    unsafe {
        let data = encode(api, rng, 8, 6, 8, 6, PNG_INTERLACE_NONE);
        let fmt = PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA;
        for stride_mode in 0..3 {
            let mut image = png_image::default();
            image.version = PNG_IMAGE_VERSION;
            let r = (api.png_image_begin_read_from_memory)(
                &mut image, data.as_ptr() as *const c_void, data.len());
            if r == 0 {
                p!("stride begin fail woe={}", image.warning_or_error);
                (api.png_image_free)(&mut image);
                continue;
            }
            image.format = fmt;
            let ch = format_channels(fmt);
            let min_stride = (image.width * ch) as i32;
            let stride = match stride_mode {
                0 => -min_stride,
                1 => min_stride + 8,
                _ => min_stride,
            };
            // A negative stride means "bottom-most row first in the buffer".
            // libpng offsets to the last row itself
            // (pngread.c: `ptr += (height-1) * (-row_step)`), so the caller
            // passes the base of the allocation for either sign. Size the
            // buffer to exactly what the contract requires so that an
            // out-of-bounds access would be caught rather than absorbed by
            // slack.
            let span = stride.unsigned_abs() as usize * image.height as usize;
            let mut buf = vec![0u8; span];
            let base = buf.as_mut_ptr();
            let fr = (api.png_image_finish_read)(
                &mut image, std::ptr::null(), base as *mut c_void, stride, std::ptr::null_mut());
            p!("stride={} finish={} woe={} msg={:?}", stride, fr, image.warning_or_error, image.msg());
            emit_bytes(&format!("stride={} buf", stride), &buf);
            (api.png_image_free)(&mut image);
        }
    }
}

/// B99 — png_image_begin_read_from_file on a temp file.
fn simple_file(api: &Api, rng: &mut Rng) {
    unsafe {
        let data = encode(api, rng, 8, 6, 8, 6, PNG_INTERLACE_NONE);
        let path = format!("/tmp/rdpath_simple_{}.png", std::process::id());
        std::fs::write(&path, &data).unwrap();
        let cpath = cs(&path);
        let mut image = png_image::default();
        image.version = PNG_IMAGE_VERSION;
        let r = (api.png_image_begin_read_from_file)(&mut image, cpath.as_ptr());
        p!("file begin={} woe={} {}x{}", r, image.warning_or_error, image.width, image.height);
        if r != 0 {
            image.format = PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA;
            let ch = format_channels(image.format);
            let stride = image.width * ch;
            let mut buf = vec![0u8; (stride * image.height) as usize + 16];
            let fr = (api.png_image_finish_read)(
                &mut image, std::ptr::null(), buf.as_mut_ptr() as *mut c_void, 0, std::ptr::null_mut());
            p!("file finish={} woe={} msg={:?}", fr, image.warning_or_error, image.msg());
            emit_bytes("file buf", &buf[..(stride * image.height) as usize]);
        }
        (api.png_image_free)(&mut image);
        let _ = std::fs::remove_file(&path);
    }
}

/* ================================================================== */
/* B100..B118 — read transforms                                       */
/* ================================================================== */

/// Build a non-interlaced PNG, open a reader, run `set_transform` after
/// read_info, update_info, decode all rows and report.
unsafe fn run_transform<F: Fn(&Api, &Reader)>(
    api: &Api,
    rng: &mut Rng,
    color: u8,
    depth: u8,
    w: u32,
    h: u32,
    extra_before: &[Vec<u8>],
    tag: &str,
    set_transform: F,
) {
    let data = build_png(rng, w, h, depth, color, extra_before, &[]);
    let mut rd = Reader::new(api, data);
    (api.png_read_info)(rd.png, rd.info);
    set_transform(api, &rd);
    (api.png_read_update_info)(rd.png, rd.info);
    report_updated(api, &rd, tag);
    let all = read_all_rows(api, &rd, h, 1);
    (api.png_read_end)(rd.png, rd.info);
    emit_bytes(&format!("{} rows", tag), &all);
    rd.destroy();
}

/// A tRNS chunk appropriate for the colour type.
fn trns_chunk(color: u8, depth: u8, rng: &mut Rng) -> Vec<u8> {
    match color {
        3 => {
            let n = 1usize << depth.min(8);
            let bytes: Vec<u8> = (0..n).map(|_| rng.u8()).collect();
            chunk(b"tRNS", &bytes)
        }
        0 => chunk(b"tRNS", &[0, rng.u8() & (((1u16 << depth.min(8)) - 1) as u8)]),
        2 => {
            let mut d = Vec::new();
            for _ in 0..3 {
                d.push(0);
                d.push(rng.u8());
            }
            chunk(b"tRNS", &d)
        }
        _ => Vec::new(),
    }
}

/// B100 — png_set_palette_to_rgb on PALETTE, with and without tRNS.
fn tr_palette_to_rgb(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in PAL {
            for with_trns in [false, true] {
                let extra = if with_trns { vec![trns_chunk(3, depth, rng)] } else { vec![] };
                run_transform(api, rng, color, depth, 8, 4, &extra,
                    &format!("pal2rgb bd={} trns={}", depth, with_trns),
                    |a, rd| (a.png_set_palette_to_rgb)(rd.png));
            }
        }
    }
}

/// B101 — png_set_expand on PALETTE, GRAY×{1,2,4}, and images with tRNS.
fn tr_expand(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in PAL.iter().chain(GRAY_SUB.iter()).copied() {
            for with_trns in [false, true] {
                let extra = if with_trns { vec![trns_chunk(color, depth, rng)] } else { vec![] };
                run_transform(api, rng, color, depth, 8, 4, &extra,
                    &format!("expand ct={} bd={} trns={}", color, depth, with_trns),
                    |a, rd| (a.png_set_expand)(rd.png));
            }
        }
    }
}

/// B102 — png_set_expand_gray_1_2_4_to_8 on GRAY×1/2/4.
fn tr_expand_gray(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in GRAY_SUB {
            run_transform(api, rng, color, depth, 8, 4, &[],
                &format!("expandgray bd={}", depth),
                |a, rd| (a.png_set_expand_gray_1_2_4_to_8)(rd.png));
        }
    }
}

/// B103 — png_set_expand_16 on every A1 combo.
fn tr_expand_16(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            run_transform(api, rng, color, depth, 8, 4, &[],
                &format!("expand16 ct={} bd={}", color, depth),
                |a, rd| (a.png_set_expand_16)(rd.png));
        }
    }
}

/// B104 — png_set_gray_to_rgb on GRAY and GRAY_ALPHA.
fn tr_gray_to_rgb(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(0u8, 1u8), (0, 2), (0, 4), (0, 8), (0, 16), (4, 8), (4, 16)] {
            run_transform(api, rng, color, depth, 8, 4, &[],
                &format!("gray2rgb ct={} bd={}", color, depth),
                |a, rd| (a.png_set_gray_to_rgb)(rd.png));
        }
    }
}

/// B105 — png_set_rgb_to_gray_fixed error_action × coefficients.
fn tr_rgb_to_gray(api: &Api, rng: &mut Rng) {
    unsafe {
        let coeffs = [(-1i32, -1i32), (21000, 71000), (0, 0), (100000, 0)];
        for action in [1i32, 2, 3] {
            for (r, g) in coeffs {
                for (color, depth) in [(2u8, 8u8), (2, 16), (6, 8), (6, 16)] {
                    let data = build_png(rng, 8, 4, depth, color, &[], &[]);
                    let mut rd = Reader::new(api, data);
                    (api.png_read_info)(rd.png, rd.info);
                    (api.png_set_rgb_to_gray_fixed)(rd.png, action, r, g);
                    (api.png_read_update_info)(rd.png, rd.info);
                    report_updated(api, &rd, &format!("rgb2gray a={} c=({},{}) ct={} bd={}", action, r, g, color, depth));
                    let all = read_all_rows(api, &rd, 4, 1);
                    p!("  status={}", (api.png_get_rgb_to_gray_status)(rd.png));
                    (api.png_read_end)(rd.png, rd.info);
                    emit_bytes("rgb2gray rows", &all);
                    rd.destroy();
                }
            }
        }
    }
}

/// B106 — png_set_strip_16 on all 16-bit combos.
fn tr_strip_16(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, _depth) in A1_16 {
            run_transform(api, rng, color, 16, 8, 4, &[],
                &format!("strip16 ct={}", color),
                |a, rd| (a.png_set_strip_16)(rd.png));
        }
    }
}

/// B107 — png_set_scale_16 on all 16-bit combos.
fn tr_scale_16(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, _depth) in A1_16 {
            run_transform(api, rng, color, 16, 8, 4, &[],
                &format!("scale16 ct={}", color),
                |a, rd| (a.png_set_scale_16)(rd.png));
        }
    }
}

/// B108 — png_set_strip_alpha on alpha types.
fn tr_strip_alpha(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1_ALPHA {
            run_transform(api, rng, color, depth, 8, 4, &[],
                &format!("stripalpha ct={} bd={}", color, depth),
                |a, rd| (a.png_set_strip_alpha)(rd.png));
        }
    }
}

/// B109 — png_set_swap_alpha on alpha types.
fn tr_swap_alpha(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1_ALPHA {
            run_transform(api, rng, color, depth, 8, 4, &[],
                &format!("swapalpha ct={} bd={}", color, depth),
                |a, rd| (a.png_set_swap_alpha)(rd.png));
        }
    }
}

/// B110 — png_set_invert_alpha on alpha types.
fn tr_invert_alpha(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1_ALPHA {
            run_transform(api, rng, color, depth, 8, 4, &[],
                &format!("invalpha ct={} bd={}", color, depth),
                |a, rd| (a.png_set_invert_alpha)(rd.png));
        }
    }
}

/// B111 — png_set_filler BEFORE/AFTER on GRAY×8/16 and RGB×8/16.
fn tr_filler(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(0u8, 8u8), (0, 16), (2, 8), (2, 16)] {
            for loc in [PNG_FILLER_BEFORE, PNG_FILLER_AFTER] {
                for fill in [0u32, 0x7f, 0xffff] {
                    run_transform(api, rng, color, depth, 8, 4, &[],
                        &format!("filler ct={} bd={} loc={} v={}", color, depth, loc, fill),
                        move |a, rd| (a.png_set_filler)(rd.png, fill, loc));
                }
            }
        }
    }
}

/// B112 — png_set_add_alpha BEFORE/AFTER on GRAY×8/16 and RGB×8/16.
fn tr_add_alpha(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(0u8, 8u8), (0, 16), (2, 8), (2, 16)] {
            for loc in [PNG_FILLER_BEFORE, PNG_FILLER_AFTER] {
                for fill in [0u32, 0x7f, 0xffff] {
                    run_transform(api, rng, color, depth, 8, 4, &[],
                        &format!("addalpha ct={} bd={} loc={} v={}", color, depth, loc, fill),
                        move |a, rd| (a.png_set_add_alpha)(rd.png, fill, loc));
                }
            }
        }
    }
}

/// B113 — png_set_packing on GRAY×{1,2,4} and PALETTE×{1,2,4}.
fn tr_packing(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(0u8, 1u8), (0, 2), (0, 4), (3, 1), (3, 2), (3, 4)] {
            run_transform(api, rng, color, depth, 8, 4, &[],
                &format!("packing ct={} bd={}", color, depth),
                |a, rd| (a.png_set_packing)(rd.png));
        }
    }
}

/// B114 — png_set_packswap on GRAY×{1,2,4} and PALETTE×{1,2,4}.
fn tr_packswap(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(0u8, 1u8), (0, 2), (0, 4), (3, 1), (3, 2), (3, 4)] {
            run_transform(api, rng, color, depth, 8, 4, &[],
                &format!("packswap ct={} bd={}", color, depth),
                |a, rd| (a.png_set_packswap)(rd.png));
        }
    }
}

/// B115 — png_set_shift on GRAY×{8,16}, RGB×{8,16}, RGBA×8.
fn tr_shift(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(0u8, 8u8), (0, 16), (2, 8), (2, 16), (6, 8)] {
            let s = depth.min(8) - 1;
            let sb = png_color_8 { red: s, green: s, blue: s, gray: s, alpha: s };
            let data = build_png(rng, 8, 4, depth, color, &[], &[]);
            let mut rd = Reader::new(api, data);
            (api.png_read_info)(rd.png, rd.info);
            (api.png_set_shift)(rd.png, &sb);
            (api.png_read_update_info)(rd.png, rd.info);
            report_updated(api, &rd, &format!("shift ct={} bd={}", color, depth));
            let all = read_all_rows(api, &rd, 4, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit_bytes("shift rows", &all);
            rd.destroy();
        }
    }
}

/// B116 — png_set_swap on all 16-bit combos.
fn tr_swap(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, _depth) in A1_16 {
            run_transform(api, rng, color, 16, 8, 4, &[],
                &format!("swap ct={}", color),
                |a, rd| (a.png_set_swap)(rd.png));
        }
    }
}

/// B117 — png_set_bgr on RGB and RGBA.
fn tr_bgr(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(2u8, 8u8), (2, 16), (6, 8), (6, 16)] {
            run_transform(api, rng, color, depth, 8, 4, &[],
                &format!("bgr ct={} bd={}", color, depth),
                |a, rd| (a.png_set_bgr)(rd.png));
        }
    }
}

/// B118 — png_set_invert_mono on GRAY×{1,2,4,8,16}.
fn tr_invert_mono(api: &Api, rng: &mut Rng) {
    unsafe {
        for depth in [1u8, 2, 4, 8, 16] {
            run_transform(api, rng, 0, depth, 8, 4, &[],
                &format!("invmono bd={}", depth),
                |a, rd| (a.png_set_invert_mono)(rd.png));
        }
    }
}

/* ================================================================== */
/* B119..B137 — gamma, alpha_mode, background, quantize, user, pipes  */
/* ================================================================== */

/// B119 — png_set_gamma_fixed over a screen×file grid.
fn tr_gamma_fixed(api: &Api, rng: &mut Rng) {
    unsafe {
        let grid = [100000i32, 45455, 220000, 16, 625000000];
        for (color, depth) in [(0u8, 8u8), (2, 8), (2, 16), (3, 8)] {
            for &screen in &grid {
                for &file in &grid {
                    run_transform(api, rng, color, depth, 6, 3, &[],
                        &format!("gammafx ct={} bd={} s={} f={}", color, depth, screen, file),
                        move |a, rd| (a.png_set_gamma_fixed)(rd.png, screen, file));
                }
            }
        }
    }
}

/// B120 — png_set_gamma (float) over the same grid.
fn tr_gamma_float(api: &Api, rng: &mut Rng) {
    unsafe {
        let grid = [1.0f64, 0.45455, 2.2, 0.00016, 6250.0];
        for (color, depth) in [(0u8, 8u8), (2, 8), (2, 16), (3, 8)] {
            for &screen in &grid {
                for &file in &grid {
                    run_transform(api, rng, color, depth, 6, 3, &[],
                        &format!("gammaf ct={} bd={} s={} f={}", color, depth, screen, file),
                        move |a, rd| (a.png_set_gamma)(rd.png, screen, file));
                }
            }
        }
    }
}

/// B121 — image carries gAMA; gamma via DEFAULT_sRGB / MAC_18 / explicit.
fn tr_gamma_gama_chunk(api: &Api, rng: &mut Rng) {
    unsafe {
        for &screen in &[PNG_DEFAULT_sRGB, PNG_GAMMA_MAC_18, 100000] {
            let mut gama = Vec::new();
            gama.extend_from_slice(&45455u32.to_be_bytes());
            let extra = vec![chunk(b"gAMA", &gama)];
            run_transform(api, rng, 2, 8, 6, 3, &extra,
                &format!("gama_chunk s={}", screen),
                move |a, rd| (a.png_set_gamma_fixed)(rd.png, screen, 45455));
        }
    }
}

/// B122 — image carries sRGB; gamma handling via colorspace.
fn tr_gamma_srgb_chunk(api: &Api, rng: &mut Rng) {
    unsafe {
        for intent in [0u8, 1, 2, 3] {
            let extra = vec![chunk(b"sRGB", &[intent])];
            run_transform(api, rng, 2, 8, 6, 3, &extra,
                &format!("srgb_chunk intent={}", intent),
                |a, rd| (a.png_set_gamma_fixed)(rd.png, 45455, 45455));
        }
    }
}

/// B123..B127 — png_set_alpha_mode_fixed over a gamma grid on alpha types.
fn tr_alpha_mode(api: &Api, rng: &mut Rng, mode: c_int) {
    unsafe {
        let grid = [100000i32, 45455, 220000];
        for (color, depth) in A1_ALPHA {
            for &g in &grid {
                run_transform(api, rng, color, depth, 6, 3, &[],
                    &format!("alphamode m={} ct={} bd={} g={}", mode, color, depth, g),
                    move |a, rd| (a.png_set_alpha_mode_fixed)(rd.png, mode, g));
            }
        }
    }
}

/// Shared background helper: build image (optionally with tRNS), set background.
unsafe fn run_background(
    api: &Api,
    rng: &mut Rng,
    color: u8,
    depth: u8,
    extra: &[Vec<u8>],
    bg: png_color_16,
    gamma_code: c_int,
    need_expand: c_int,
    bg_gamma: i32,
    tag: &str,
) {
    let data = build_png(rng, 6, 3, depth, color, extra, &[]);
    let mut rd = Reader::new(api, data);
    (api.png_read_info)(rd.png, rd.info);
    if need_expand == 1 {
        (api.png_set_expand)(rd.png);
    }
    (api.png_set_background_fixed)(rd.png, &bg, gamma_code, need_expand, bg_gamma);
    (api.png_read_update_info)(rd.png, rd.info);
    report_updated(api, &rd, tag);
    let all = read_all_rows(api, &rd, 3, 1);
    (api.png_read_end)(rd.png, rd.info);
    emit_bytes(&format!("{} rows", tag), &all);
    rd.destroy();
}

/// B128 — background GAMMA_SCREEN × need_expand.
fn tr_bg_screen(api: &Api, rng: &mut Rng) {
    unsafe {
        let bg = png_color_16 { index: 0, red: 0x20, green: 0x40, blue: 0x60, gray: 0x30 };
        for ne in [0i32, 1] {
            run_background(api, rng, 2, 8, &[], bg, PNG_BACKGROUND_GAMMA_SCREEN, ne, 100000,
                &format!("bg_screen ne={}", ne));
        }
    }
}

/// B129 — background GAMMA_FILE × need_expand.
fn tr_bg_file(api: &Api, rng: &mut Rng) {
    unsafe {
        let bg = png_color_16 { index: 0, red: 0x20, green: 0x40, blue: 0x60, gray: 0x30 };
        for ne in [0i32, 1] {
            run_background(api, rng, 2, 8, &[], bg, PNG_BACKGROUND_GAMMA_FILE, ne, 100000,
                &format!("bg_file ne={}", ne));
        }
    }
}

/// B130 — background GAMMA_UNIQUE × need_expand × background gamma grid.
fn tr_bg_unique(api: &Api, rng: &mut Rng) {
    unsafe {
        let bg = png_color_16 { index: 0, red: 0x20, green: 0x40, blue: 0x60, gray: 0x30 };
        for ne in [0i32, 1] {
            for g in [100000i32, 45455, 220000] {
                run_background(api, rng, 2, 8, &[], bg, PNG_BACKGROUND_GAMMA_UNIQUE, ne, g,
                    &format!("bg_unique ne={} g={}", ne, g));
            }
        }
    }
}

/// B131 — background on a palette image with need_expand=1, palette-index bg.
fn tr_bg_palette(api: &Api, rng: &mut Rng) {
    unsafe {
        let bg = png_color_16 { index: 1, red: 0, green: 0, blue: 0, gray: 0 };
        run_background(api, rng, 3, 8, &[], bg, PNG_BACKGROUND_GAMMA_SCREEN, 1, 100000, "bg_palette");
    }
}

/// B132 — background combined with tRNS on GRAY, RGB, PALETTE.
fn tr_bg_trns(api: &Api, rng: &mut Rng) {
    unsafe {
        let bg = png_color_16 { index: 0, red: 0x10, green: 0x20, blue: 0x30, gray: 0x18 };
        for (color, depth) in [(0u8, 8u8), (2, 8), (3, 8)] {
            let extra = vec![trns_chunk(color, depth, rng)];
            let ne = if color == 3 { 1 } else { 0 };
            run_background(api, rng, color, depth, &extra, bg, PNG_BACKGROUND_GAMMA_SCREEN, ne, 100000,
                &format!("bg_trns ct={}", color));
        }
    }
}

/// B133 — png_set_quantize on RGB×8, maximum_colors grid, with/without hIST.
fn tr_quantize_rgb(api: &Api, rng: &mut Rng) {
    unsafe {
        for &maxc in &[2i32, 16, 64, 255, 256] {
            for full in [0i32, 1] {
                let data = build_png(rng, 8, 4, 8, 2, &[], &[]);
                let mut rd = Reader::new(api, data);
                (api.png_read_info)(rd.png, rd.info);
                (api.png_set_quantize)(rd.png, std::ptr::null_mut(), 0, maxc, std::ptr::null(), full);
                (api.png_read_update_info)(rd.png, rd.info);
                report_updated(api, &rd, &format!("quant_rgb maxc={} full={}", maxc, full));
                let all = read_all_rows(api, &rd, 4, 1);
                (api.png_read_end)(rd.png, rd.info);
                emit_bytes("quant_rgb rows", &all);
                rd.destroy();
            }
        }
    }
}

/// B134 — png_set_quantize on PALETTE×8 with num_palette 2/16/256, and hIST.
fn tr_quantize_palette(api: &Api, rng: &mut Rng) {
    unsafe {
        for np in [2usize, 16, 256] {
            let mut pal: Vec<png_color> = (0..np)
                .map(|_| png_color { red: rng.u8(), green: rng.u8(), blue: rng.u8() })
                .collect();
            let hist: Vec<u16> = (0..np).map(|_| (rng.u8() as u16) + 1).collect();
            let data = build_png(rng, 8, 4, 8, 3, &[], &[]);
            let mut rd = Reader::new(api, data);
            (api.png_read_info)(rd.png, rd.info);
            (api.png_set_quantize)(rd.png, pal.as_mut_ptr(), np as c_int, np as c_int, hist.as_ptr(), 1);
            (api.png_read_update_info)(rd.png, rd.info);
            report_updated(api, &rd, &format!("quant_pal np={}", np));
            let all = read_all_rows(api, &rd, 4, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit_bytes("quant_pal rows", &all);
            rd.destroy();
        }
    }
}

/// B135 — png_set_read_user_transform_fn + png_set_user_transform_info.
fn tr_user(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in [(0u8, 8u8), (2, 8), (6, 8)] {
            let data = build_png(rng, 8, 4, depth, color, &[], &[]);
            let mut rd = Reader::new(api, data);
            (api.png_read_info)(rd.png, rd.info);
            (api.png_set_read_user_transform_fn)(rd.png, Some(user_transform_fn));
            (api.png_set_user_transform_info)(rd.png, std::ptr::null_mut(), depth as c_int, channels_of(color) as c_int);
            (api.png_read_update_info)(rd.png, rd.info);
            report_updated(api, &rd, &format!("user ct={} bd={}", color, depth));
            let all = read_all_rows(api, &rd, 4, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit_bytes("user rows", &all);
            rd.destroy();
        }
    }
}

/// B136 — png_set_read_status_fn installed; callback sequence compared.
fn status_fn(api: &Api, rng: &mut Rng) {
    unsafe {
        for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
            let data = encode(api, rng, 8, 5, 8, 2, il);
            let mut rd = Reader::new(api, data);
            (api.png_set_read_status_fn)(rd.png, Some(read_status_fn));
            (api.png_read_info)(rd.png, rd.info);
            let passes = (api.png_set_interlace_handling)(rd.png);
            (api.png_read_update_info)(rd.png, rd.info);
            let _ = read_all_rows(api, &rd, 5, passes);
            (api.png_read_end)(rd.png, rd.info);
            emit(&format!("status done il={}", il));
            rd.destroy();
        }
    }
}

/// B137 — the 7 composed pipelines from CONFIGS.md row 137.
fn tr_pipelines(api: &Api, rng: &mut Rng) {
    unsafe {
        // 1: expand + gray_to_rgb + add_alpha  (GRAY×2)
        run_transform(api, rng, 0, 2, 8, 3, &[], "pipe1", |a, rd| {
            (a.png_set_expand)(rd.png);
            (a.png_set_gray_to_rgb)(rd.png);
            (a.png_set_add_alpha)(rd.png, 0xffff, PNG_FILLER_AFTER);
        });
        // 2: expand_16 + swap  (RGB×8 -> not 16, expand_16 promotes)
        run_transform(api, rng, 2, 8, 8, 3, &[], "pipe2", |a, rd| {
            (a.png_set_expand_16)(rd.png);
            (a.png_set_swap)(rd.png);
        });
        // 3: strip_16 + strip_alpha + gray_to_rgb  (GRAY_ALPHA×16)
        run_transform(api, rng, 4, 16, 8, 3, &[], "pipe3", |a, rd| {
            (a.png_set_strip_16)(rd.png);
            (a.png_set_strip_alpha)(rd.png);
            (a.png_set_gray_to_rgb)(rd.png);
        });
        // 4: palette_to_rgb + bgr + invert_alpha  (PALETTE×8 + tRNS)
        let trns = trns_chunk(3, 8, rng);
        run_transform(api, rng, 3, 8, 8, 3, &[trns], "pipe4", |a, rd| {
            (a.png_set_palette_to_rgb)(rd.png);
            (a.png_set_tRNS_to_alpha)(rd.png);
            (a.png_set_bgr)(rd.png);
            (a.png_set_invert_alpha)(rd.png);
        });
        // 5: gamma + background + expand  (GRAY×4)
        run_transform(api, rng, 0, 4, 8, 3, &[], "pipe5", |a, rd| {
            let bg = png_color_16 { index: 0, red: 0, green: 0, blue: 0, gray: 0x08 };
            (a.png_set_gamma_fixed)(rd.png, 100000, 45455);
            (a.png_set_expand)(rd.png);
            (a.png_set_background_fixed)(rd.png, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 100000);
        });
        // 6: alpha_mode + expand_16 + swap  (RGBA×8)
        run_transform(api, rng, 6, 8, 8, 3, &[], "pipe6", |a, rd| {
            (a.png_set_alpha_mode_fixed)(rd.png, PNG_ALPHA_STANDARD, 100000);
            (a.png_set_expand_16)(rd.png);
            (a.png_set_swap)(rd.png);
        });
        // 7: packing + shift + invert_mono  (GRAY×2)
        run_transform(api, rng, 0, 2, 8, 3, &[], "pipe7", |a, rd| {
            let sb = png_color_8 { red: 2, green: 2, blue: 2, gray: 2, alpha: 2 };
            (a.png_set_packing)(rd.png);
            (a.png_set_shift)(rd.png, &sb);
            (a.png_set_invert_mono)(rd.png);
        });
    }
}

/* ================================================================== */
/* B138..B155 — global state, limits, options, unknown chunks         */
/* ================================================================== */

/// B138 — png_set_benign_errors(0) and (1) on a stream with a recoverable
/// defect (a bad ancillary CRC, which is benign/recoverable).
fn benign_errors(api: &Api, rng: &mut Rng) {
    unsafe {
        // A bad-CRC ancillary tEXt chunk is recoverable; benign_errors controls
        // whether it warns or errors. Put it in its own variant so an error in
        // the (0) case does not mask the (1) case.
        let mode = pick(&[0i32, 1]);
        let text = chunk_badcrc(b"tEXt", b"Comment\0hello");
        let data = build_png(rng, 8, 4, 8, 2, &[text], &[]);
        let mut rd = Reader::new(api, data);
        (api.png_set_benign_errors)(rd.png, mode);
        (api.png_read_info)(rd.png, rd.info);
        report_hdr(api, &rd, &format!("benign mode={}", mode));
        (api.png_read_update_info)(rd.png, rd.info);
        let all = read_all_rows(api, &rd, 4, 1);
        (api.png_read_end)(rd.png, rd.info);
        emit_bytes("benign rows", &all);
        rd.destroy();
    }
}

/// B139 — png_set_crc_action over the 6×6 grid; bad ancillary CRC (in-process)
/// and bad critical CRC (each critical cell in its own variant, since a fatal
/// critical CRC terminates the process).
fn crc_action_grid(api: &Api, rng: &mut Rng) {
    unsafe {
        // Two families of variants: even variant => ancillary-only grid, run
        // fully in-process; odd variant => one critical grid cell per process.
        let which = variant() % 2;
        if which == 0 {
            // Bad ancillary CRC on a tEXt chunk; walk the full ancil action
            // range with crit_action fixed at USE (0) so nothing critical fires.
            for ancil in 0..6i32 {
                let text = chunk_badcrc(b"tEXt", b"Comment\0hi");
                let data = build_png(rng, 8, 4, 8, 2, &[text], &[]);
                let mut rd = Reader::new(api, data);
                (api.png_set_crc_action)(rd.png, 0, ancil);
                (api.png_read_info)(rd.png, rd.info);
                (api.png_read_update_info)(rd.png, rd.info);
                let all = read_all_rows(api, &rd, 4, 1);
                (api.png_read_end)(rd.png, rd.info);
                emit_bytes(&format!("crc ancil={} rows", ancil), &all);
                rd.destroy();
            }
        } else {
            // One critical-CRC cell per process. A corrupted IHDR CRC is the
            // canonical critical defect.
            let crit = pick(&[0i32, 1, 2, 3, 4, 5]);
            let mut v = Vec::new();
            v.extend_from_slice(&PNG_SIG);
            v.extend_from_slice(&chunk_badcrc(b"IHDR", &ihdr_data(8, 4, 8, 2, 0, 0, 0)));
            let raw = raw_rows(rng, 8, 4, 8, 2);
            v.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
            v.extend_from_slice(&chunk(b"IEND", &[]));
            let mut rd = Reader::new(api, v);
            (api.png_set_crc_action)(rd.png, crit, 0);
            emit(&format!("crc crit={}", crit));
            (api.png_read_info)(rd.png, rd.info);
            (api.png_read_update_info)(rd.png, rd.info);
            let all = read_all_rows(api, &rd, 4, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit_bytes("crc crit rows", &all);
            rd.destroy();
        }
    }
}

/// B140 / B165 — png_set_check_for_invalid_index and png_get_palette_max.
fn check_invalid_index(api: &Api, rng: &mut Rng) {
    unsafe {
        // Palette of 4 entries but pixels index up to 7 (out of range at bd=4).
        for on in [0i32, 1] {
            let mut v = Vec::new();
            v.extend_from_slice(&PNG_SIG);
            v.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 2, 4, 3, 0, 0, 0)));
            // 4-entry palette.
            let pal: Vec<u8> = (0..4 * 3).map(|_| rng.u8()).collect();
            v.extend_from_slice(&chunk(b"PLTE", &pal));
            // rows with indices 0..7 (some out of range).
            let rb = rowbytes_of(8, 4, 3);
            let mut raw = Vec::new();
            for _ in 0..2 {
                raw.push(0u8);
                for _ in 0..rb {
                    raw.push(((rng.u8() & 0x77) as u8).wrapping_add(0x01));
                }
            }
            v.extend_from_slice(&chunk(b"IDAT", &zlib_store(&raw)));
            v.extend_from_slice(&chunk(b"IEND", &[]));
            let mut rd = Reader::new(api, v);
            (api.png_set_check_for_invalid_index)(rd.png, on);
            (api.png_read_info)(rd.png, rd.info);
            (api.png_read_update_info)(rd.png, rd.info);
            let all = read_all_rows(api, &rd, 2, 1);
            (api.png_read_end)(rd.png, rd.info);
            p!("invalid_index on={} palette_max={}", on, (api.png_get_palette_max)(rd.png, rd.info));
            emit_bytes("invalid_index rows", &all);
            rd.destroy();
        }
    }
}

/// B141 — png_set_user_limits at/below/above image size.
fn user_limits(api: &Api, rng: &mut Rng) {
    unsafe {
        for (lw, lh) in [(8u32, 8u32), (4, 4), (100, 100)] {
            let data = build_png(rng, 8, 8, 8, 2, &[], &[]);
            let mut rd = Reader::new(api, data);
            (api.png_set_user_limits)(rd.png, lw, lh);
            p!("limits w={} h={} wmax={} hmax={}", lw, lh,
               (api.png_get_user_width_max)(rd.png), (api.png_get_user_height_max)(rd.png));
            (api.png_read_info)(rd.png, rd.info);
            report_hdr(api, &rd, &format!("limits w={} h={}", lw, lh));
            (api.png_read_update_info)(rd.png, rd.info);
            let all = read_all_rows(api, &rd, 8, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit_bytes("limits rows", &all);
            rd.destroy();
        }
    }
}

/// B142 — png_set_chunk_cache_max with 4 tEXt chunks present.
fn chunk_cache_max(api: &Api, rng: &mut Rng) {
    unsafe {
        for cache in [0u32, 1, 2, 3, 1000] {
            let mut extra = Vec::new();
            for i in 0..4u8 {
                let mut d = format!("Key{}", i).into_bytes();
                d.push(0);
                d.extend_from_slice(b"value");
                extra.push(chunk(b"tEXt", &d));
            }
            let data = build_png(rng, 8, 4, 8, 2, &extra, &[]);
            let mut rd = Reader::new(api, data);
            (api.png_set_chunk_cache_max)(rd.png, cache);
            p!("cache_max set={} get={}", cache, (api.png_get_chunk_cache_max)(rd.png));
            (api.png_read_info)(rd.png, rd.info);
            (api.png_read_update_info)(rd.png, rd.info);
            let all = read_all_rows(api, &rd, 4, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit_bytes(&format!("cache={} rows", cache), &all);
            rd.destroy();
        }
    }
}

/// B143 — png_set_chunk_malloc_max with a large tEXt.
fn chunk_malloc_max(api: &Api, rng: &mut Rng) {
    unsafe {
        for mm in [0u64, 1, 100, 8_000_000] {
            let mut d = b"Big\0".to_vec();
            d.extend(std::iter::repeat(b'x').take(2000));
            let extra = vec![chunk(b"tEXt", &d)];
            let data = build_png(rng, 8, 4, 8, 2, &extra, &[]);
            let mut rd = Reader::new(api, data);
            (api.png_set_chunk_malloc_max)(rd.png, mm);
            p!("malloc_max set={} get={}", mm, (api.png_get_chunk_malloc_max)(rd.png));
            (api.png_read_info)(rd.png, rd.info);
            (api.png_read_update_info)(rd.png, rd.info);
            let all = read_all_rows(api, &rd, 4, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit_bytes(&format!("mm={} rows", mm), &all);
            rd.destroy();
        }
    }
}

/// B144 — png_set_option(PNG_MAXIMUM_INFLATE_WINDOW, ON/OFF).
fn opt_inflate_window(api: &Api, rng: &mut Rng) {
    unsafe {
        for onoff in [0i32, 1] {
            let data = build_png(rng, 8, 4, 8, 2, &[], &[]);
            let mut rd = Reader::new(api, data);
            let r = (api.png_set_option)(rd.png, PNG_MAXIMUM_INFLATE_WINDOW, onoff);
            p!("inflate_window onoff={} prev={}", onoff, r);
            (api.png_read_info)(rd.png, rd.info);
            (api.png_read_update_info)(rd.png, rd.info);
            let all = read_all_rows(api, &rd, 4, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit_bytes("inflate_window rows", &all);
            rd.destroy();
        }
    }
}

/// B145 — png_set_option(PNG_SKIP_sRGB_CHECK_PROFILE, ON/OFF) with iCCP.
fn opt_skip_srgb(api: &Api, rng: &mut Rng) {
    unsafe {
        for onoff in [0i32, 1] {
            // Minimal structurally-valid iCCP: name\0 method zlib(profile>=132).
            let mut prof = Vec::new();
            prof.extend_from_slice(&132u32.to_be_bytes()); // profile size field
            prof.extend(std::iter::repeat(0u8).take(128));
            let mut d = b"icc\0".to_vec();
            d.push(0); // compression method
            d.extend_from_slice(&zlib_store(&prof));
            let extra = vec![chunk(b"iCCP", &d)];
            let data = build_png(rng, 8, 4, 8, 2, &extra, &[]);
            let mut rd = Reader::new(api, data);
            let r = (api.png_set_option)(rd.png, PNG_SKIP_sRGB_CHECK_PROFILE, onoff);
            p!("skip_srgb onoff={} prev={}", onoff, r);
            (api.png_read_info)(rd.png, rd.info);
            (api.png_read_update_info)(rd.png, rd.info);
            let all = read_all_rows(api, &rd, 4, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit_bytes("skip_srgb rows", &all);
            rd.destroy();
        }
    }
}

/// B146 — png_set_option(PNG_IGNORE_ADLER32, ON/OFF) on a bad-Adler stream.
fn opt_ignore_adler(api: &Api, rng: &mut Rng) {
    unsafe {
        // PNG_IGNORE_ADLER32 is option index 8 (may not be defined in this
        // build); use the numeric value from png.h. Corrupt the IDAT adler.
        const PNG_IGNORE_ADLER32: c_int = 8;
        let mode = pick(&[0i32, 1]);
        let raw = raw_rows(rng, 8, 4, 8, 2);
        let mut z = zlib_store(&raw);
        let n = z.len();
        z[n - 1] ^= 0xff; // corrupt adler-32
        let mut v = Vec::new();
        v.extend_from_slice(&PNG_SIG);
        v.extend_from_slice(&chunk(b"IHDR", &ihdr_data(8, 4, 8, 2, 0, 0, 0)));
        v.extend_from_slice(&chunk(b"IDAT", &z));
        v.extend_from_slice(&chunk(b"IEND", &[]));
        let mut rd = Reader::new(api, v);
        let r = (api.png_set_option)(rd.png, PNG_IGNORE_ADLER32, mode);
        p!("ignore_adler onoff={} prev={}", mode, r);
        (api.png_read_info)(rd.png, rd.info);
        (api.png_read_update_info)(rd.png, rd.info);
        let all = read_all_rows(api, &rd, 4, 1);
        (api.png_read_end)(rd.png, rd.info);
        emit_bytes("ignore_adler rows", &all);
        rd.destroy();
    }
}

/// B147 / B153 — png_set_keep_unknown_chunks for each handling mode ×
/// known-but-unhandled and truly unknown chunk types; then
/// png_get_unknown_chunks after reading with ALWAYS.
fn keep_unknown(api: &Api, rng: &mut Rng) {
    unsafe {
        for mode in [PNG_HANDLE_CHUNK_AS_DEFAULT, PNG_HANDLE_CHUNK_NEVER,
                     PNG_HANDLE_CHUNK_IF_SAFE, PNG_HANDLE_CHUNK_ALWAYS] {
            // one truly-unknown ancillary and one known-but-unhandled (oFFs).
            let unk = chunk(b"prVt", b"private-data");
            let offs = {
                let mut d = Vec::new();
                d.extend_from_slice(&10i32.to_be_bytes());
                d.extend_from_slice(&20i32.to_be_bytes());
                d.push(0);
                chunk(b"oFFs", &d)
            };
            let data = build_png(rng, 8, 4, 8, 2, &[unk, offs], &[]);
            let mut rd = Reader::new(api, data);
            (api.png_set_keep_unknown_chunks)(rd.png, mode, std::ptr::null(), 0);
            (api.png_read_info)(rd.png, rd.info);
            (api.png_read_update_info)(rd.png, rd.info);
            let _ = read_all_rows(api, &rd, 4, 1);
            (api.png_read_end)(rd.png, rd.info);
            let mut chunks: *mut png_unknown_chunk = std::ptr::null_mut();
            let n = (api.png_get_unknown_chunks)(rd.png, rd.info, &mut chunks);
            p!("keep mode={} unknown_count={}", mode, n);
            for i in 0..n as isize {
                if !chunks.is_null() {
                    let c = &*chunks.offset(i);
                    p!("  chunk name={:?} size={} loc={}", &c.name[..4], c.size, c.location);
                }
            }
            rd.destroy();
        }
    }
}

/// B148 — png_set_read_user_chunk_fn returning 0 / 1.
fn user_chunk_fn(api: &Api, rng: &mut Rng) {
    unsafe {
        for handled in [false, true] {
            // The third (reserved) byte of a chunk name must be uppercase:
            // check_chunk_name (pngrutil.c:153) clears bit 5 of bytes 0,1,3 but
            // not byte 2, so a lowercase byte 2 is rejected as
            // "bad header (invalid type)" before the callback is ever reached.
            let unk = chunk(b"unKn", b"payload-bytes");
            let data = build_png(rng, 8, 4, 8, 2, &[unk], &[]);
            let mut rd = Reader::new(api, data);
            // ALWAYS keep so the callback is invoked.
            (api.png_set_keep_unknown_chunks)(rd.png, PNG_HANDLE_CHUNK_ALWAYS, std::ptr::null(), 0);
            let cb: png_user_chunk_ptr = if handled { Some(user_chunk_ok) } else { Some(user_chunk_unhandled) };
            (api.png_set_read_user_chunk_fn)(rd.png, std::ptr::null_mut(), cb);
            (api.png_read_info)(rd.png, rd.info);
            (api.png_read_update_info)(rd.png, rd.info);
            let _ = read_all_rows(api, &rd, 4, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit(&format!("user_chunk handled={}", handled));
            rd.destroy();
        }
    }
}

/// B150 — png_set_mem_fn with custom malloc/free; the allocation call count
/// is compared (never any address).
fn mem_fn(api: &Api, rng: &mut Rng) {
    unsafe {
        MALLOC_CALLS = 0;
        FREE_CALLS = 0;
        let png = (api.png_create_read_struct)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            Some(err_fn),
            Some(warn_fn),
        );
        (api.png_set_mem_fn)(png, std::ptr::null_mut(), Some(count_malloc), Some(count_free));
        let info = (api.png_create_info_struct)(png);
        (api.png_set_read_fn)(png, std::ptr::null_mut(), Some(read_fn));
        set_rbuf(build_png(rng, 8, 4, 8, 2, &[], &[]));
        (api.png_read_info)(png, info);
        (api.png_read_update_info)(png, info);
        let rb = (api.png_get_rowbytes)(png, info);
        for _ in 0..4 {
            let mut row = vec![0u8; rb + 8];
            (api.png_read_row)(png, row.as_mut_ptr(), null());
        }
        (api.png_read_end)(png, info);
        let mut pp = png;
        let mut ip = info;
        (api.png_destroy_read_struct)(&mut pp, &mut ip, null());
        // Only the counts are deterministic across libraries at the same
        // granularity; emit the relationship, not raw totals which may differ
        // by internal allocation strategy. Report both individually.
        p!("mem malloc_calls={} free_calls={} balanced={}",
           MALLOC_CALLS, FREE_CALLS, MALLOC_CALLS == FREE_CALLS);
    }
}

/// B151 — png_set_sig_bytes(n) with n bytes pre-consumed by the caller.
fn sig_bytes(api: &Api, rng: &mut Rng) {
    unsafe {
        for n in 0usize..=8 {
            let data = build_png(rng, 8, 4, 8, 2, &[], &[]);
            // Pre-consume n bytes of signature, feed the remainder.
            let remainder = data[n..].to_vec();
            let mut rd = Reader::new(api, remainder);
            (api.png_set_sig_bytes)(rd.png, n as c_int);
            (api.png_read_info)(rd.png, rd.info);
            report_hdr(api, &rd, &format!("sigbytes n={}", n));
            (api.png_read_update_info)(rd.png, rd.info);
            let all = read_all_rows(api, &rd, 4, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit_bytes(&format!("sigbytes n={} rows", n), &all);
            rd.destroy();
        }
    }
}

/// B154 — ancillary chunks placed after IDAT (tEXt, tIME, eXIf).
fn anc_after_idat(api: &Api, rng: &mut Rng) {
    unsafe {
        let text = chunk(b"tEXt", b"Comment\0after-idat");
        let time = {
            let mut d = Vec::new();
            d.extend_from_slice(&2020u16.to_be_bytes());
            d.extend_from_slice(&[6, 15, 12, 30, 45]);
            chunk(b"tIME", &d)
        };
        let exif = chunk(b"eXIf", &[0x49, 0x49, 0x2a, 0x00]);
        let data = build_png(rng, 8, 4, 8, 2, &[], &[text, time, exif]);
        let mut rd = Reader::new(api, data);
        (api.png_read_info)(rd.png, rd.info);
        (api.png_read_update_info)(rd.png, rd.info);
        let all = read_all_rows(api, &rd, 4, 1);
        (api.png_read_end)(rd.png, rd.info);
        let mut tp: *mut png_time = std::ptr::null_mut();
        let has_time = (api.png_get_tIME)(rd.png, rd.info, &mut tp);
        p!("after_idat valid_time={}", has_time);
        if has_time != 0 && !tp.is_null() {
            let t = &*tp;
            p!("  tIME {}-{}-{} {}:{}:{}", t.year, t.month, t.day, t.hour, t.minute, t.second);
        }
        emit_bytes("after_idat rows", &all);
        rd.destroy();
    }
}

/// B155 — IDAT split across 1, 2, 5 chunks including a zero-length IDAT.
fn idat_split(api: &Api, rng: &mut Rng) {
    unsafe {
        let (w, h) = (8u32, 4u32);
        let raw = raw_rows(rng, w, h, 8, 2);
        let z = zlib_store(&raw);
        for splits in [1usize, 2, 5] {
            let mut v = Vec::new();
            v.extend_from_slice(&PNG_SIG);
            v.extend_from_slice(&chunk(b"IHDR", &ihdr_data(w, h, 8, 2, 0, 0, 0)));
            // A zero-length IDAT first, then the payload split into `splits`.
            v.extend_from_slice(&chunk(b"IDAT", &[]));
            let per = (z.len() + splits - 1) / splits;
            let mut off = 0usize;
            while off < z.len() {
                let end = (off + per).min(z.len());
                v.extend_from_slice(&chunk(b"IDAT", &z[off..end]));
                off = end;
            }
            v.extend_from_slice(&chunk(b"IEND", &[]));
            let mut rd = Reader::new(api, v);
            (api.png_read_info)(rd.png, rd.info);
            (api.png_read_update_info)(rd.png, rd.info);
            let all = read_all_rows(api, &rd, h, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit_bytes(&format!("idat_split n={} rows", splits), &all);
            rd.destroy();
        }
    }
}

/* ================================================================== */
/* mem_fn counting callbacks                                          */
/* ================================================================== */

extern "C" {
    fn malloc(n: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

static mut MALLOC_CALLS: u64 = 0;
static mut FREE_CALLS: u64 = 0;

unsafe extern "C" fn count_malloc(_p: png_structp, n: usize) -> *mut c_void {
    MALLOC_CALLS += 1;
    malloc(n)
}

unsafe extern "C" fn count_free(_p: png_structp, q: *mut c_void) {
    FREE_CALLS += 1;
    free(q);
}

/* ================================================================== */
/* B159 — update_info reported metadata per transform                 */
/* ================================================================== */

/// B159 — after each A5 transform, compare reported rowbytes/channels/bit_depth
/// /color_type. We drive a representative transform per applicable colour type.
fn update_info_report(api: &Api, rng: &mut Rng) {
    unsafe {
        let cases: &[(&str, u8, u8, fn(&Api, &Reader))] = &[
            ("expand", 3, 4, |a, rd| (a.png_set_expand)(rd.png)),
            ("expand16", 2, 8, |a, rd| (a.png_set_expand_16)(rd.png)),
            ("gray2rgb", 0, 8, |a, rd| (a.png_set_gray_to_rgb)(rd.png)),
            ("strip16", 6, 16, |a, rd| (a.png_set_strip_16)(rd.png)),
            ("stripalpha", 6, 8, |a, rd| (a.png_set_strip_alpha)(rd.png)),
            ("packing", 0, 4, |a, rd| (a.png_set_packing)(rd.png)),
            ("filler_after", 2, 8, |a, rd| (a.png_set_filler)(rd.png, 0xff, PNG_FILLER_AFTER)),
            ("addalpha", 0, 8, |a, rd| (a.png_set_add_alpha)(rd.png, 0xff, PNG_FILLER_AFTER)),
        ];
        for (name, color, depth, f) in cases {
            let data = build_png(rng, 8, 3, *depth, *color, &[], &[]);
            let mut rd = Reader::new(api, data);
            (api.png_read_info)(rd.png, rd.info);
            report_hdr(api, &rd, &format!("{} before", name));
            f(api, &rd);
            (api.png_read_update_info)(rd.png, rd.info);
            report_updated(api, &rd, name);
            let all = read_all_rows(api, &rd, 3, 1);
            (api.png_read_end)(rd.png, rd.info);
            emit_bytes(&format!("{} rows", name), &all);
            rd.destroy();
        }
    }
}

/* ================================================================== */
/* B156..B158 — round trips                                           */
/* ================================================================== */

/// B156 — write with W-LOW then read back with R-LOW, comparing encoded bytes
/// and decoded rows, over all A1 combos × interlace.
fn roundtrip_low(api: &Api, rng: &mut Rng) {
    unsafe {
        for (color, depth) in A1 {
            for il in [PNG_INTERLACE_NONE, PNG_INTERLACE_ADAM7] {
                let (w, h) = (8u32, 6u32);
                let encoded = encode(api, rng, w, h, depth, color, il);
                emit_bytes(&format!("enc ct={} bd={} il={}", color, depth, il), &encoded);
                let mut rd = Reader::new(api, encoded);
                (api.png_read_info)(rd.png, rd.info);
                let passes = (api.png_set_interlace_handling)(rd.png);
                report_hdr(api, &rd, &format!("rt ct={} bd={} il={}", color, depth, il));
                (api.png_read_update_info)(rd.png, rd.info);
                let all = read_all_rows(api, &rd, h, passes);
                (api.png_read_end)(rd.png, rd.info);
                emit_bytes("rt rows", &all);
                rd.destroy();
            }
        }
    }
}

/// B157 — write with W-PNG then read with R-PNG using an inverse-ish mask.
fn roundtrip_png(api: &Api, rng: &mut Rng) {
    unsafe {
        // Encode with the low-level writer (a plain valid stream), then read
        // back with png_read_png applying a transform and its logical inverse
        // pair so we exercise both W-PNG-style and R-PNG-style masks.
        let pairs = [
            ("bgr", PNG_TRANSFORM_BGR, PNG_TRANSFORM_BGR, 2u8, 8u8),
            ("swapendian", PNG_TRANSFORM_SWAP_ENDIAN, PNG_TRANSFORM_SWAP_ENDIAN, 2, 16),
            ("packing", PNG_TRANSFORM_PACKING, PNG_TRANSFORM_PACKING, 0, 2),
            ("invmono", PNG_TRANSFORM_INVERT_MONO, PNG_TRANSFORM_INVERT_MONO, 0, 4),
        ];
        for (name, wtx, rtx, color, depth) in pairs {
            // write side via png_write_png
            let mut wr = Writer::new(api);
            (api.png_set_IHDR)(wr.png, wr.info, 8, 4, depth as c_int, color as c_int,
                PNG_INTERLACE_NONE, PNG_COMPRESSION_TYPE_BASE, PNG_FILTER_TYPE_BASE);
            let pal: Vec<png_color>;
            if color == 3 {
                let n = 1usize << depth.min(8);
                pal = (0..n).map(|_| png_color { red: rng.u8(), green: rng.u8(), blue: rng.u8() }).collect();
                (api.png_set_PLTE)(wr.png, wr.info, pal.as_ptr(), n as c_int);
            }
            // The input row size for png_write_png depends on the transform:
            // PNG_TRANSFORM_PACKING expects one sample per byte, so the input
            // is `width*channels` bytes rather than the packed rowbytes. Every
            // other transform here consumes the packed native row.
            let native_rb = rowbytes_of(8, depth, color);
            let in_rb = if (wtx & PNG_TRANSFORM_PACKING) != 0 {
                8 * channels_of(color)
            } else {
                native_rb
            };
            // For packing, each input byte must be a valid `depth`-bit sample.
            let sample_mask: u8 = if (wtx & PNG_TRANSFORM_PACKING) != 0 && depth < 8 {
                ((1u16 << depth) - 1) as u8
            } else {
                0xff
            };
            let mut storage: Vec<Vec<u8>> = (0..4usize)
                .map(|_| rng.image_bytes(in_rb).iter().map(|b| b & sample_mask).collect())
                .collect();
            let mut ptrs: Vec<*mut u8> = storage.iter_mut().map(|v| v.as_mut_ptr()).collect();
            (api.png_set_rows)(wr.png, wr.info, ptrs.as_mut_ptr());
            (api.png_write_png)(wr.png, wr.info, wtx, null());
            let encoded = wbuf().clone();
            wr.destroy();
            emit_bytes(&format!("rtpng enc {} ct={}", name, color), &encoded);
            // read side via png_read_png with the matching mask
            read_png_with(api, encoded, rtx, &format!("rtpng {} ct={}", name, color));
        }
    }
}

/// B158 — W-SIMPLE then R-SIMPLE over the PNG_FORMAT_* cross-product.
fn roundtrip_simple(api: &Api, rng: &mut Rng) {
    unsafe {
        for bits in 0u32..8 {
            let fmt = (bits & 1) * PNG_FORMAT_FLAG_ALPHA
                | ((bits >> 1) & 1) * PNG_FORMAT_FLAG_COLOR
                | ((bits >> 2) & 1) * PNG_FORMAT_FLAG_BGR;
            // Build a source buffer and write it with the simplified writer.
            let (w, h) = (6u32, 4u32);
            let ch = format_channels(fmt);
            let stride = (w * ch) as usize;
            let src: Vec<u8> = (0..stride * h as usize).map(|_| rng.u8()).collect();
            let mut wimg = png_image::default();
            wimg.version = PNG_IMAGE_VERSION;
            wimg.width = w;
            wimg.height = h;
            wimg.format = fmt;
            let mut outbuf = vec![0u8; src.len() * 4 + 1024];
            let mut outlen: u64 = outbuf.len() as u64;
            let wr = (api.png_image_write_to_memory)(
                &mut wimg,
                outbuf.as_mut_ptr() as *mut c_void,
                &mut outlen,
                0,
                src.as_ptr() as *const c_void,
                (stride) as i32,
                std::ptr::null(),
            );
            p!("rtsimple fmt={:#x} write={} len={} woe={} msg={:?}",
               fmt, wr, outlen, wimg.warning_or_error, wimg.msg());
            (api.png_image_free)(&mut wimg);
            if wr == 0 {
                continue;
            }
            outbuf.truncate(outlen as usize);
            emit_bytes(&format!("rtsimple fmt={:#x} enc", fmt), &outbuf);
            // Read back with the same format.
            simple_read(api, &outbuf, fmt, false, &format!("rtsimple fmt={:#x}", fmt));
        }
    }
}

/* ================================================================== */
/* B163 — io_state / io_chunk_type sampled in the read callback       */
/* ================================================================== */

static mut IO_LOG: Option<Vec<(u32, u32)>> = None;
static mut IO_API: *const Api = std::ptr::null();

fn io_log() -> &'static mut Vec<(u32, u32)> {
    unsafe {
        let p = &raw mut IO_LOG;
        if (*p).is_none() {
            *p = Some(Vec::new());
        }
        (*p).as_mut().unwrap()
    }
}

/// Read callback that samples io_state/io_chunk_type on every invocation.
unsafe extern "C" fn io_state_read_fn(p: png_structp, data: *mut u8, len: usize) {
    let api = &*IO_API;
    let st = (api.png_get_io_state)(p);
    let ct = (api.png_get_io_chunk_type)(p);
    io_log().push((st, ct));
    // Delegate the actual byte movement to the standard harness reader.
    read_fn(p, data, len);
}

/// B163 — full io_state sequence across a read.
fn io_state(api: &Api, rng: &mut Rng) {
    unsafe {
        io_log().clear();
        IO_API = api as *const Api;
        set_rbuf(build_png(rng, 8, 4, 8, 2, &[], &[]));
        let png = (api.png_create_read_struct)(
            PNG_LIBPNG_VER_STRING.as_ptr() as *const c_char,
            std::ptr::null_mut(),
            Some(err_fn),
            Some(warn_fn),
        );
        let info = (api.png_create_info_struct)(png);
        (api.png_set_read_fn)(png, std::ptr::null_mut(), Some(io_state_read_fn));
        (api.png_read_info)(png, info);
        (api.png_read_update_info)(png, info);
        let rb = (api.png_get_rowbytes)(png, info);
        for _ in 0..4 {
            let mut row = vec![0u8; rb + 8];
            (api.png_read_row)(png, row.as_mut_ptr(), null());
        }
        (api.png_read_end)(png, info);
        let mut pp = png;
        let mut ip = info;
        (api.png_destroy_read_struct)(&mut pp, &mut ip, null());
        IO_API = std::ptr::null();
        for (st, ct) in io_log().iter() {
            let cb = ct.to_be_bytes();
            p!("io state={:#x} chunk={:?}", st, &cb);
        }
        p!("io events={}", io_log().len());
    }
}
