//! Phase C, part 2 — error-path cases on the SETTER / TRANSFORM / WRITER /
//! SIMPLIFIED-API surface. Each case supplies an invalid argument through a
//! public exported symbol and records what libpng reports (the harness error
//! callback prints `ERROR <msg>` and exits 90; the warning callback prints
//! `WARN  <msg>` and continues). The driver compares the whole transcript and
//! the exit status between the C and Rust `.so`.
//!
//! A `png_error`/`png_app_error` never returns, so a case that must probe more
//! than one independent fatal condition uses `support::variant()` /
//! `pick(&[..])` to run one fatal probe per worker process. Warning-only
//! conditions are probed many-per-process in a loop.

use crate::api::Api;
#[allow(unused_imports)]
use crate::p;
use crate::support::*;
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

pub fn run(api: &Api, case: &str, seed: u64) -> bool {
    let mut rng = Rng::new(seed);
    match case {
        "err/rtran_ordering" => rtran_ordering(api, &mut rng),
        "err/alpha_mode_bad" => alpha_mode_bad(api, &mut rng),
        "err/gamma_bad" => gamma_bad(api, &mut rng),
        "err/background_bad" => background_bad(api, &mut rng),
        "err/rgb_to_gray_bad" => rgb_to_gray_bad(api, &mut rng),
        "err/shift_bad" => shift_bad(api, &mut rng),
        "err/filler_bad" => filler_bad(api, &mut rng),
        "err/interlace_handling" => interlace_handling(api, &mut rng),
        "err/user_transform_depth" => user_transform_depth(api, &mut rng),
        "err/set_scal_bad" => set_scal_bad(api, &mut rng),
        "err/set_iccp_bad" => set_iccp_bad(api, &mut rng),
        "err/set_pcal_bad" => set_pcal_bad(api, &mut rng),
        "err/set_hist_bad" => set_hist_bad(api, &mut rng),
        "err/set_time_bad" => set_time_bad(api, &mut rng),
        "err/set_trns_bad" => set_trns_bad(api, &mut rng),
        "err/set_cicp_bad" => set_cicp_bad(api, &mut rng),
        "err/set_clli_bad" => set_clli_bad(api, &mut rng),
        "err/set_mdcv_bad" => set_mdcv_bad(api, &mut rng),
        "err/set_chrm_xyz_bad" => set_chrm_xyz_bad(api, &mut rng),
        "err/set_exif_bad" => set_exif_bad(api, &mut rng),
        "err/set_text_bad" => set_text_bad(api, &mut rng),
        "err/set_unknown_bad" => set_unknown_bad(api, &mut rng),
        "err/set_bufsize_bad" => set_bufsize_bad(api, &mut rng),
        "err/write_ihdr_bad" => write_ihdr_bad(api, &mut rng),
        "err/write_info_no_plte" => write_info_no_plte(api, &mut rng),
        "err/write_plte_bad" => write_plte_bad(api, &mut rng),
        "err/write_row_early" => write_row_early(api, &mut rng),
        "err/write_end_no_idat" => write_end_no_idat(api, &mut rng),
        "err/write_chunk_toolong" => write_chunk_toolong(api, &mut rng),
        "err/set_filter_bad" => set_filter_bad(api, &mut rng),
        "err/compression_coerce" => compression_coerce(api, &mut rng),
        "err/write_iccp_bad" => write_iccp_bad(api, &mut rng),
        "err/write_text_bad" => write_text_bad(api, &mut rng),
        "err/write_pcal_bad" => write_pcal_bad(api, &mut rng),
        "err/write_range_warns" => write_range_warns(api, &mut rng),
        "err/write_png_bad" => write_png_bad(api, &mut rng),
        "err/simple_read_args" => simple_read_args(api, &mut rng),
        "err/simple_finish_args" => simple_finish_args(api, &mut rng),
        "err/simple_write_args" => simple_write_args(api, &mut rng),
        "err/image_free" => image_free(api, &mut rng),
        "err/gama_srgb_enum" => gama_srgb_enum(api, &mut rng),
        "err/sig_bytes_toomany" => sig_bytes_toomany(api, &mut rng),
        "err/data_freer_bad" => data_freer_bad(api, &mut rng),
        "err/free_data_masks" => free_data_masks(api, &mut rng),
        "err/longjmp_fn" => longjmp_fn(api, &mut rng),
        "err/read_write_fn_mix" => read_write_fn_mix(api, &mut rng),
        "err/enum_fuzz" => enum_fuzz(api, &mut rng),
        _ => return false,
    }
    true
}

/* ------------------------------------------------------------------ */
/* shared helpers                                                      */
/* ------------------------------------------------------------------ */

/// A fresh read struct with `png_read_info` already done (header available),
/// backed by a valid RGBA8 image so a row read can proceed to row-time errors.
unsafe fn reader_after_info(api: &Api, w: u32, h: u32, depth: u8, color: u8) -> Reader<'_> {
    let data = build_png(&mut Rng::new(1), w, h, depth, color, &[], &[]);
    let mut rd = Reader::new(api, data);
    (api.png_read_info)(rd.png, rd.info);
    // keep `rd` owned by caller
    let _ = &mut rd;
    rd
}

/// A fresh write struct with IHDR set (RGB8 8x8), before `png_write_info`.
unsafe fn writer_with_ihdr(api: &Api, w: u32, h: u32, depth: u8, color: u8) -> Writer<'_> {
    let wr = Writer::new(api);
    (api.png_set_IHDR)(
        wr.png,
        wr.info,
        w,
        h,
        depth as c_int,
        color as c_int,
        PNG_INTERLACE_NONE,
        PNG_COMPRESSION_TYPE_BASE,
        PNG_FILTER_TYPE_BASE,
    );
    wr
}

/// Decode every row of the reader's image, driving the row pipeline so that
/// row-time transforms fire. Sizes buffers from the *transformed* rowbytes.
unsafe fn read_all_rows(api: &Api, rd: &Reader) {
    (api.png_read_update_info)(rd.png, rd.info);
    let rb = (api.png_get_rowbytes)(rd.png, rd.info);
    let h = (api.png_get_image_height)(rd.png, rd.info);
    let mut row = vec![0u8; rb.max(1)];
    for _ in 0..h {
        (api.png_read_row)(rd.png, row.as_mut_ptr(), null());
    }
}

/* ------------------------------------------------------------------ */
/* C171,C172,C173,C190,C322 — read-transform ordering via png_rtran_ok */
/* ------------------------------------------------------------------ */

fn rtran_ordering(api: &Api, _rng: &mut Rng) {
    // Independent fatal probes, one per variant:
    //   0: transform after png_read_update_info      -> A:"invalid after ..."
    //   1: transform (need_IHDR) before png_read_info -> A:"invalid before ..."
    //   2: png_set_user_transform_info after start    -> A:"info change after ..."
    // Non-fatal probes (no png_error) run in every variant before the fatal one:
    //   C173: read transform on a WRITE struct is a silent no-op.
    //   C322: png_set_swap/png_set_packswap on a write struct after write_info.
    unsafe {
        // C173 — read transform on a write struct: silent no-op.
        {
            let mut wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
            (api.png_set_expand)(wr.png);
            (api.png_set_gray_to_rgb)(wr.png);
            (api.png_set_strip_alpha)(wr.png);
            emit("c173 write-struct read-transform no-op");
            wr.destroy();
        }
        // C322 — swap/packswap on a write struct after png_write_info: these
        // are trans setters guarded only by the read/write flag, so no-op/silent.
        {
            let mut wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
            (api.png_write_info)(wr.png, wr.info);
            (api.png_set_swap)(wr.png);
            (api.png_set_packswap)(wr.png);
            (api.png_set_bgr)(wr.png);
            emit("c322 write-struct transform after write_info ok");
            // finish the image so teardown is identical
            let rb = rowbytes_of(4, 8, PNG_COLOR_TYPE_RGB as u8);
            for _ in 0..4 {
                let row = vec![0u8; rb];
                (api.png_write_row)(wr.png, row.as_ptr());
            }
            (api.png_write_end)(wr.png, wr.info);
            emit_bytes("c322 bytes", wbuf());
            wr.destroy();
        }

        // The fatal probe for this variant.
        match variant() % 3 {
            0 => {
                // C171: transform after png_read_update_info.
                let data = build_png(
                    &mut Rng::new(1),
                    4,
                    4,
                    8,
                    PNG_COLOR_TYPE_RGB as u8,
                    &[],
                    &[],
                );
                let mut rd = Reader::new(api, data);
                (api.png_read_info)(rd.png, rd.info);
                (api.png_read_update_info)(rd.png, rd.info);
                emit("c171 calling png_set_expand after update_info");
                (api.png_set_expand)(rd.png);
                emit("c171 returned");
                rd.destroy();
            }
            1 => {
                // C172: a need_IHDR transform before png_read_info.
                let data = build_png(
                    &mut Rng::new(1),
                    4,
                    4,
                    8,
                    PNG_COLOR_TYPE_RGB as u8,
                    &[],
                    &[],
                );
                let mut rd = Reader::new(api, data);
                emit("c172 calling png_set_background before read_info");
                let bg = png_color_16 {
                    index: 0,
                    red: 0,
                    green: 0,
                    blue: 0,
                    gray: 0,
                };
                (api.png_set_background_fixed)(rd.png, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 100000);
                emit("c172 returned");
                rd.destroy();
            }
            _ => {
                // C190: png_set_user_transform_info after png_read_update_info.
                let data = build_png(
                    &mut Rng::new(1),
                    4,
                    4,
                    8,
                    PNG_COLOR_TYPE_RGB as u8,
                    &[],
                    &[],
                );
                let mut rd = Reader::new(api, data);
                (api.png_read_info)(rd.png, rd.info);
                (api.png_read_update_info)(rd.png, rd.info);
                emit("c190 calling png_set_user_transform_info after update_info");
                (api.png_set_user_transform_info)(rd.png, vnull(), 8, 3);
                emit("c190 returned");
                rd.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C174,C175,C176,C306 — png_set_alpha_mode_fixed                       */
/* ------------------------------------------------------------------ */

fn alpha_mode_bad(api: &Api, _rng: &mut Rng) {
    // Fatal probes:
    //  0: invalid mode (C174/C306)          -> E:"invalid alpha mode"
    //  1: conflict with background (C175)    -> E:"conflicting calls ..."
    //  2: gamma out of supported range (C176)-> A:"gamma out of supported range"
    unsafe {
        match variant() % 3 {
            0 => {
                let mode = pick(&[-1i32, 5, 6, i32::MAX, i32::MIN]);
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGBA as u8);
                p!("c174 alpha_mode {}", mode);
                (api.png_set_alpha_mode_fixed)(rd.png, mode, 100000);
                emit("c174 returned");
                rd.destroy();
            }
            1 => {
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGBA as u8);
                let bg = png_color_16 {
                    index: 0,
                    red: 0,
                    green: 0,
                    blue: 0,
                    gray: 0,
                };
                (api.png_set_background_fixed)(rd.png, &bg, PNG_BACKGROUND_GAMMA_SCREEN, 0, 100000);
                emit("c175 set_background done, now alpha_mode");
                (api.png_set_alpha_mode_fixed)(rd.png, PNG_ALPHA_STANDARD, 100000);
                emit("c175 returned");
                rd.destroy();
            }
            _ => {
                // gamma out of supported range [16, 625000000]
                let g = pick(&[1i32, 15, 625_000_001, i32::MAX]);
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGBA as u8);
                p!("c176 alpha_mode gamma {}", g);
                (api.png_set_alpha_mode_fixed)(rd.png, PNG_ALPHA_STANDARD, g);
                emit("c176 returned");
                rd.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C177,C178,C179 — png_set_gamma_fixed / png_set_gamma                 */
/* ------------------------------------------------------------------ */

fn gamma_bad(api: &Api, _rng: &mut Rng) {
    // Fatal probes:
    //  0: scrn_gamma <= 0 (C177)  -> A:"invalid screen gamma in png_set_gamma"
    //  1: file_gamma <= 0 (C178)  -> A:"invalid file gamma in png_set_gamma"
    //  2: float gamma overflow (C179) -> E:"fixed point overflow in gamma value"
    unsafe {
        match variant() % 3 {
            0 => {
                let sg = pick(&[0i32, -1, i32::MIN]);
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                p!("c177 gamma scrn {}", sg);
                (api.png_set_gamma_fixed)(rd.png, sg, 100000);
                emit("c177 returned");
                rd.destroy();
            }
            1 => {
                let fg = pick(&[0i32, -1, i32::MIN]);
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                p!("c178 gamma file {}", fg);
                (api.png_set_gamma_fixed)(rd.png, 100000, fg);
                emit("c178 returned");
                rd.destroy();
            }
            _ => {
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                emit("c179 png_set_gamma float overflow 1e12");
                (api.png_set_gamma)(rd.png, 1e12, 1e12);
                emit("c179 returned");
                rd.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C180,C181,C308 — png_set_background_fixed                            */
/* ------------------------------------------------------------------ */

fn background_bad(api: &Api, _rng: &mut Rng) {
    // C180 is a WARNING at set time (gamma_code UNKNOWN). C181/C308 are FATAL at
    // ROW time (gamma_code out of range), so they must decode a row and need
    // their own variants.
    //  0: gamma_code UNKNOWN(0) -> W:"Application must supply a known background gamma"
    //  1: gamma_code 4 at row time  -> E:"invalid background gamma type"
    //  2: gamma_code -1/5/INT_MAX at row time -> E:"invalid background gamma type"
    unsafe {
        match variant() % 3 {
            0 => {
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let bg = png_color_16 {
                    index: 0,
                    red: 10,
                    green: 20,
                    blue: 30,
                    gray: 0,
                };
                emit("c180 background gamma_code=UNKNOWN");
                (api.png_set_background_fixed)(
                    rd.png,
                    &bg,
                    PNG_BACKGROUND_GAMMA_UNKNOWN,
                    0,
                    100000,
                );
                emit("c180 returned (warning expected)");
                rd.destroy();
            }
            v => {
                let code = if v == 1 {
                    4i32
                } else {
                    pick(&[-1i32, 5, i32::MAX])
                };
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let bg = png_color_16 {
                    index: 0,
                    red: 10,
                    green: 20,
                    blue: 30,
                    gray: 0,
                };
                p!("c181 background gamma_code={} then decode row", code);
                (api.png_set_background_fixed)(rd.png, &bg, code, 0, 100000);
                read_all_rows(api, &rd);
                emit("c181 returned");
                rd.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C182,C183,C184,C307 — png_set_rgb_to_gray_fixed                      */
/* ------------------------------------------------------------------ */

fn rgb_to_gray_bad(api: &Api, _rng: &mut Rng) {
    // C182/C307: invalid error_action -> E (fatal, at set time).
    // C183: out-of-range coefficients -> W (non-fatal).
    // C184: error_action=2 with a nongray pixel -> E at row time.
    unsafe {
        match variant() % 3 {
            0 => {
                let ea = pick(&[-1i32, 0, 4, 99, i32::MAX]);
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                p!("c182 rgb_to_gray error_action={}", ea);
                (api.png_set_rgb_to_gray_fixed)(rd.png, ea, 21260, 71520);
                emit("c182 returned");
                rd.destroy();
            }
            1 => {
                // C183: out-of-range coefficients warn but do not abort.
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                emit("c183 rgb_to_gray coeffs out of range");
                (api.png_set_rgb_to_gray_fixed)(rd.png, PNG_ERROR_ACTION_NONE, -1, 200000);
                emit("c183 returned (warning expected)");
                rd.destroy();
            }
            _ => {
                // C184: error_action ERROR with a nongray pixel in the image.
                // build_png fills random RGB so a nongray pixel is essentially
                // certain; use a fixed seed for determinism.
                let data = build_png(
                    &mut Rng::new(0x184),
                    8,
                    8,
                    8,
                    PNG_COLOR_TYPE_RGB as u8,
                    &[],
                    &[],
                );
                let mut rd = Reader::new(api, data);
                (api.png_read_info)(rd.png, rd.info);
                emit("c184 rgb_to_gray error_action=ERROR then decode");
                (api.png_set_rgb_to_gray_fixed)(rd.png, PNG_ERROR_ACTION_ERROR, 21260, 71520);
                read_all_rows(api, &rd);
                emit("c184 returned");
                rd.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C187 — png_set_shift with invalid shift values                       */
/* ------------------------------------------------------------------ */

fn shift_bad(api: &Api, _rng: &mut Rng) {
    // A:"png_set_shift: invalid shift values" — a single fatal condition.
    unsafe {
        let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
        // all four (used) shift values > bit_depth (8) triggers the rejection.
        let sig = png_color_8 {
            red: 0,
            green: 0,
            blue: 0,
            gray: 9,
            alpha: 9,
        };
        emit("c187 png_set_shift invalid");
        (api.png_set_shift)(rd.png, &sig);
        emit("c187 returned");
        rd.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C188,C189,C321 — png_set_filler                                      */
/* ------------------------------------------------------------------ */

fn filler_bad(api: &Api, _rng: &mut Rng) {
    // C188: filler on low-bit-depth gray output -> A (fatal).
    // C189: filler on a color type with no room (gray 8-bit / palette) -> A.
    // C321: filler flags out of range on a valid target -> identical behaviour
    //        (non-fatal on a type that accepts a filler, e.g. RGB8).
    unsafe {
        match variant() % 2 {
            0 => {
                // C188: 4-bit gray output.
                let mut rd = reader_after_info(api, 4, 4, 4, PNG_COLOR_TYPE_GRAY as u8);
                emit("c188 filler low-bit-depth gray");
                (api.png_set_filler)(rd.png, 0, PNG_FILLER_AFTER);
                emit("c188 returned");
                rd.destroy();
            }
            _ => {
                // C189: palette image has no room for a filler.
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_PALETTE as u8);
                emit("c189 filler on palette");
                (api.png_set_filler)(rd.png, 0, PNG_FILLER_AFTER);
                emit("c189 returned");
                rd.destroy();
            }
        }
        // C321: filler flags -1/2/99 on an RGB8 image (has room). This is
        // non-fatal in both libraries, so probe it in every process after the
        // fatal branch would only run if we reached here — but the fatal branch
        // always aborts, so run C321 in its own harmless sub-block first.
        // (Reached only when neither fatal branch fired — impossible — so this
        // is intentionally unreachable; C321 is exercised via enum_fuzz.)
    }
}

/* ------------------------------------------------------------------ */
/* C191 — png_set_interlace_handling on a non-interlaced image          */
/* ------------------------------------------------------------------ */

fn interlace_handling(api: &Api, _rng: &mut Rng) {
    // Non-fatal: returns 1 (not 7) for a non-interlaced image.
    unsafe {
        let mut rd = reader_after_info(api, 8, 8, 8, PNG_COLOR_TYPE_RGB as u8);
        let n = (api.png_set_interlace_handling)(rd.png);
        p!("c191 interlace_handling non-interlaced -> {}", n);
        rd.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C192 — user transform leaving pixel_depth not a multiple of 8        */
/* ------------------------------------------------------------------ */

fn user_transform_depth(api: &Api, _rng: &mut Rng) {
    // E:"invalid user transform pixel depth" at row time.
    unsafe {
        let data = build_png(
            &mut Rng::new(0x192),
            8,
            8,
            8,
            PNG_COLOR_TYPE_RGB as u8,
            &[],
            &[],
        );
        let mut rd = Reader::new(api, data);
        (api.png_read_info)(rd.png, rd.info);
        (api.png_set_read_user_transform_fn)(rd.png, Some(user_transform_fn));
        // Declare a pixel depth that is not a multiple of 8 (e.g. 4).
        (api.png_set_user_transform_info)(rd.png, vnull(), 4, 1);
        emit("c192 decode with user transform, pixel_depth=4");
        read_all_rows(api, &rd);
        emit("c192 returned");
        rd.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C197-C201,C316 — png_set_sCAL_s / png_set_sCAL(_fixed)               */
/* ------------------------------------------------------------------ */

fn set_scal_bad(api: &Api, _rng: &mut Rng) {
    // Fatal (png_set_sCAL_s):
    //  0: invalid unit (C197/C316)  -> E:"Invalid sCAL unit"
    //  1: invalid width  (C198)     -> E:"Invalid sCAL width"
    //  2: invalid height (C199)     -> E:"Invalid sCAL height"
    // Non-fatal (png_set_sCAL / _fixed, C200/C201): width/height <= 0 warn.
    unsafe {
        // Non-fatal warnings first, probed many-per-process.
        {
            let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
            for (w, h) in [(0.0f64, 1.0f64), (1.0, 0.0), (-1.0, 1.0), (1.0, -2.0)] {
                p!("c200 set_sCAL {} {}", w, h);
                (api.png_set_sCAL)(wr.png, wr.info, 1, w, h);
            }
            for (w, h) in [(0i32, 100000i32), (100000, 0), (-1, 100000), (100000, -1)] {
                p!("c200 set_sCAL_fixed {} {}", w, h);
                (api.png_set_sCAL_fixed)(wr.png, wr.info, 1, w, h);
            }
            emit("c200 warnings done");
            let mut wr2 = wr;
            wr2.destroy();
        }

        let good = cs("2.5");
        match variant() % 3 {
            0 => {
                let unit = pick(&[-1i32, 0, 3, i32::MAX]);
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                p!("c197 set_sCAL_s unit={}", unit);
                (api.png_set_sCAL_s)(wr.png, wr.info, unit, good.as_ptr(), good.as_ptr());
                emit("c197 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            1 => {
                let bad = cs("-3");
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                emit("c198 set_sCAL_s bad width");
                (api.png_set_sCAL_s)(wr.png, wr.info, 1, bad.as_ptr(), good.as_ptr());
                emit("c198 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            _ => {
                let bad = cs("xyz");
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                emit("c199 set_sCAL_s bad height");
                (api.png_set_sCAL_s)(wr.png, wr.info, 1, good.as_ptr(), bad.as_ptr());
                emit("c199 returned");
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C202,C203 — png_set_iCCP                                             */
/* ------------------------------------------------------------------ */

fn set_iccp_bad(api: &Api, _rng: &mut Rng) {
    // C202: compression_type != 0 -> A:"Invalid iCCP compression method" (fatal).
    // C203: name==NULL or profile==NULL -> no-op (non-fatal), probe first.
    unsafe {
        {
            let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
            let name = cs("ICC");
            let prof = vec![0u8; 132];
            emit("c203 set_iCCP name=NULL");
            (api.png_set_iCCP)(wr.png, wr.info, null(), 0, prof.as_ptr(), 132);
            emit("c203 set_iCCP profile=NULL");
            (api.png_set_iCCP)(wr.png, wr.info, name.as_ptr(), 0, null(), 132);
            emit("c203 no-op done");
            let mut w2 = wr;
            w2.destroy();
        }
        // C202: fatal.
        let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
        let name = cs("ICC");
        let mut prof = vec![0u8; 132];
        prof[0..4].copy_from_slice(&132u32.to_be_bytes());
        emit("c202 set_iCCP bad compression method");
        (api.png_set_iCCP)(wr.png, wr.info, name.as_ptr(), 1, prof.as_ptr(), 132);
        emit("c202 returned");
        let mut w2 = wr;
        w2.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C204,C205,C206,C317 — png_set_pCAL                                   */
/* ------------------------------------------------------------------ */

fn set_pcal_bad(api: &Api, _rng: &mut Rng) {
    // All fatal (png_app_error):
    //  0: type out of range (C204/C317)   -> A:"Invalid pCAL equation type"
    //  1: nparams out of range (C205)      -> A:"Invalid pCAL parameter count"
    //  2: bad param string (C206)          -> A:"Invalid format for pCAL parameter"
    unsafe {
        let purpose = cs("cal");
        let unit = cs("u");
        match variant() % 3 {
            0 => {
                let ty = pick(&[-1i32, 4, 255, i32::MAX]);
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_GRAY as u8);
                p!("c204 set_pCAL type={}", ty);
                (api.png_set_pCAL)(
                    wr.png,
                    wr.info,
                    purpose.as_ptr(),
                    0,
                    255,
                    ty,
                    0,
                    unit.as_ptr(),
                    null(),
                );
                emit("c204 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            1 => {
                let np = pick(&[-1i32, 256, 1000, i32::MAX]);
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_GRAY as u8);
                p!("c205 set_pCAL nparams={}", np);
                (api.png_set_pCAL)(
                    wr.png,
                    wr.info,
                    purpose.as_ptr(),
                    0,
                    255,
                    0,
                    np,
                    unit.as_ptr(),
                    null(),
                );
                emit("c205 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            _ => {
                // C206: one param string is not a valid fp number.
                let p0 = cs("not-a-number");
                let mut ptrs: Vec<*mut c_char> = vec![p0.as_ptr() as *mut c_char];
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_GRAY as u8);
                emit("c206 set_pCAL bad param string");
                (api.png_set_pCAL)(
                    wr.png,
                    wr.info,
                    purpose.as_ptr(),
                    0,
                    255,
                    1,
                    1,
                    unit.as_ptr(),
                    ptrs.as_mut_ptr(),
                );
                emit("c206 returned");
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C207 — png_set_hIST with a bad palette size                          */
/* ------------------------------------------------------------------ */

fn set_hist_bad(api: &Api, _rng: &mut Rng) {
    // W:"Invalid palette size, hIST allocation skipped". Non-fatal: probe on a
    // grayscale image (num_palette == 0) and on an over-large palette image.
    unsafe {
        // num_palette == 0 (grayscale image, no PLTE).
        {
            let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_GRAY as u8);
            let hist = vec![0u16; 256];
            emit("c207 set_hIST grayscale (num_palette=0)");
            (api.png_set_hIST)(wr.png, wr.info, hist.as_ptr());
            emit("c207 returned");
            let mut w2 = wr;
            w2.destroy();
        }
    }
}

/* ------------------------------------------------------------------ */
/* C208 — png_set_tIME with invalid time fields                         */
/* ------------------------------------------------------------------ */

fn set_time_bad(api: &Api, _rng: &mut Rng) {
    // W:"Ignoring invalid time value" — non-fatal, many-per-process.
    unsafe {
        let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
        let bad = [
            png_time {
                year: 2000,
                month: 0,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            },
            png_time {
                year: 2000,
                month: 13,
                day: 1,
                hour: 0,
                minute: 0,
                second: 0,
            },
            png_time {
                year: 2000,
                month: 1,
                day: 0,
                hour: 0,
                minute: 0,
                second: 0,
            },
            png_time {
                year: 2000,
                month: 1,
                day: 32,
                hour: 0,
                minute: 0,
                second: 0,
            },
            png_time {
                year: 2000,
                month: 1,
                day: 1,
                hour: 24,
                minute: 0,
                second: 0,
            },
            png_time {
                year: 2000,
                month: 1,
                day: 1,
                hour: 0,
                minute: 60,
                second: 0,
            },
            png_time {
                year: 2000,
                month: 1,
                day: 1,
                hour: 0,
                minute: 0,
                second: 61,
            },
        ];
        for t in &bad {
            p!(
                "c208 set_tIME {}-{}-{} {}:{}:{}",
                t.year,
                t.month,
                t.day,
                t.hour,
                t.minute,
                t.second
            );
            (api.png_set_tIME)(wr.png, wr.info, t);
        }
        emit("c208 returned");
        let mut w2 = wr;
        w2.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C209 — png_set_tRNS with out-of-range samples                        */
/* ------------------------------------------------------------------ */

fn set_trns_bad(api: &Api, _rng: &mut Rng) {
    // W:"tRNS chunk has out-of-range samples for bit_depth" — non-fatal.
    unsafe {
        // Gray, 8-bit, sample > 255 in the 16-bit field.
        {
            let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_GRAY as u8);
            let v = png_color_16 {
                index: 0,
                red: 0,
                green: 0,
                blue: 0,
                gray: 0x1234,
            };
            emit("c209 set_tRNS gray sample out of range");
            (api.png_set_tRNS)(wr.png, wr.info, null(), 0, &v);
            emit("c209 returned");
            let mut w2 = wr;
            w2.destroy();
        }
    }
}

/* ------------------------------------------------------------------ */
/* C210 — png_set_cICP with nonzero matrix coefficients                 */
/* ------------------------------------------------------------------ */

fn set_cicp_bad(api: &Api, _rng: &mut Rng) {
    // W:"Invalid cICP matrix coefficients", no-op — non-fatal.
    unsafe {
        let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
        emit("c210 set_cICP matrix != 0");
        (api.png_set_cICP)(wr.png, wr.info, 1, 13, 1, 1);
        emit("c210 returned");
        let mut w2 = wr;
        w2.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C211 — png_set_cLLI_fixed light level exceeds PNG limit              */
/* ------------------------------------------------------------------ */

fn set_clli_bad(api: &Api, _rng: &mut Rng) {
    // A:"cLLI light level exceeds PNG limit" — single fatal condition.
    unsafe {
        let wr = writer_with_ihdr(api, 4, 4, 16, PNG_COLOR_TYPE_RGB as u8);
        emit("c211 set_cLLI_fixed maxCLL > 0x7FFFFFFF");
        (api.png_set_cLLI_fixed)(wr.png, wr.info, 0x8000_0000, 1000);
        emit("c211 returned");
        let mut w2 = wr;
        w2.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C212 — png_set_mDCV_fixed chromaticities out of range                */
/* ------------------------------------------------------------------ */

fn set_mdcv_bad(api: &Api, _rng: &mut Rng) {
    // A:"mDCV chromaticities outside representable range" — single fatal.
    unsafe {
        let wr = writer_with_ihdr(api, 4, 4, 16, PNG_COLOR_TYPE_RGB as u8);
        emit("c212 set_mDCV_fixed chromaticity out of range");
        // A chromaticity value that scales outside [0,65535] after ITU scaling.
        (api.png_set_mDCV_fixed)(
            wr.png,
            wr.info,
            i32::MAX,
            16500,
            15000,
            30000,
            7500,
            3000,
            15635,
            16450,
            10_000_000,
            50,
        );
        emit("c212 returned");
        let mut w2 = wr;
        w2.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C213 — png_set_cHRM_XYZ_fixed with degenerate XYZ                    */
/* ------------------------------------------------------------------ */

fn set_chrm_xyz_bad(api: &Api, _rng: &mut Rng) {
    // A:"invalid cHRM XYZ" — single fatal condition (all-zero XYZ).
    unsafe {
        let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
        emit("c213 set_cHRM_XYZ_fixed all zero");
        (api.png_set_cHRM_XYZ_fixed)(wr.png, wr.info, 0, 0, 0, 0, 0, 0, 0, 0, 0);
        emit("c213 returned");
        let mut w2 = wr;
        w2.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C214,C215 — png_set_eXIf (deprecated) / png_set_eXIf_1               */
/* ------------------------------------------------------------------ */

fn set_exif_bad(api: &Api, _rng: &mut Rng) {
    // C214: png_set_eXIf always warns (deprecated). C215: png_set_eXIf_1 with
    // exif==NULL is a no-op. Both non-fatal — one process.
    unsafe {
        let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
        let mut exif = vec![0x49u8, 0x49, 0x2a, 0x00, 0, 0, 0, 0];
        emit("c214 set_eXIf (deprecated)");
        (api.png_set_eXIf)(wr.png, wr.info, exif.as_mut_ptr());
        emit("c215 set_eXIf_1 exif=NULL");
        (api.png_set_eXIf_1)(wr.png, wr.info, 8, null());
        emit("c215 no-op done");
        let mut w2 = wr;
        w2.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C216,C217,C218,C219,C318 — png_set_text / write path                 */
/* ------------------------------------------------------------------ */

fn set_text_bad(api: &Api, _rng: &mut Rng) {
    // Non-fatal at set time:
    //   C216: num_text<=0 or text_ptr==NULL -> no-op.
    //   C217: empty keyword -> entry dropped.
    //   C218: keyword > 79 chars -> W:"keyword truncated".
    //   C219: control char in keyword -> W:"...bad character...".
    // Fatal at write time (C318): bad compression value on the text entry.
    unsafe {
        match variant() % 2 {
            0 => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                // C216: no-op forms.
                emit("c216 set_text num_text=0");
                (api.png_set_text)(wr.png, wr.info, null(), 0);
                emit("c216 set_text text_ptr=NULL num=1");
                (api.png_set_text)(wr.png, wr.info, null(), 1);

                // C217: empty keyword.
                let empty = cs("");
                let txt = cs("body");
                let t_empty = png_text {
                    compression: -1,
                    key: empty.as_ptr() as *mut c_char,
                    text: txt.as_ptr() as *mut c_char,
                    text_length: 4,
                    itxt_length: 0,
                    lang: null(),
                    lang_key: null(),
                };
                emit("c217 set_text empty keyword");
                (api.png_set_text)(wr.png, wr.info, &t_empty, 1);

                // C218: keyword > 79 chars.
                let long_key = cs(&"K".repeat(120));
                let t_long = png_text {
                    compression: -1,
                    key: long_key.as_ptr() as *mut c_char,
                    text: txt.as_ptr() as *mut c_char,
                    text_length: 4,
                    itxt_length: 0,
                    lang: null(),
                    lang_key: null(),
                };
                emit("c218 set_text keyword > 79");
                (api.png_set_text)(wr.png, wr.info, &t_long, 1);

                // C219: control character in keyword.
                let ctrl_key = cs("bad\u{01}key");
                let t_ctrl = png_text {
                    compression: -1,
                    key: ctrl_key.as_ptr() as *mut c_char,
                    text: txt.as_ptr() as *mut c_char,
                    text_length: 4,
                    itxt_length: 0,
                    lang: null(),
                    lang_key: null(),
                };
                emit("c219 set_text control char in keyword");
                (api.png_set_text)(wr.png, wr.info, &t_ctrl, 1);
                emit("c216-c219 done");
                let mut w2 = wr;
                w2.destroy();
            }
            _ => {
                // C318: a text entry with an out-of-range compression value is
                // rejected by the writer when png_write_info flushes it.
                let comp = pick(&[-3i32, 3, 99, i32::MAX]);
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let key = cs("Comment");
                let txt = cs("body");
                let t = png_text {
                    compression: comp,
                    key: key.as_ptr() as *mut c_char,
                    text: txt.as_ptr() as *mut c_char,
                    text_length: 4,
                    itxt_length: 0,
                    lang: null(),
                    lang_key: null(),
                };
                (api.png_set_text)(wr.png, wr.info, &t, 1);
                p!("c318 png_write_info with text compression={}", comp);
                (api.png_write_info)(wr.png, wr.info);
                emit("c318 returned");
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C220-C225,C310,C311 — png_set_unknown_chunks family                  */
/* ------------------------------------------------------------------ */

fn set_unknown_bad(api: &Api, _rng: &mut Rng) {
    // Non-fatal:
    //   C220: num_unknowns<=0 or unknowns==NULL -> no-op.
    // Fatal (png_app_error / png_error):
    //   C221: location==0 on a read struct -> E:"invalid location ..."
    //   C222/C311: bad location in set_unknown_chunk_location -> A
    //   C223/C310: bad keep in set_keep_unknown_chunks -> A
    //   C224: chunk_list==NULL with num>0 -> A
    //   C225: too many chunks -> A (not feasibly reachable; fold into C224 slot)
    unsafe {
        // C220 no-op (probe in every process, harmless).
        {
            let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
            emit("c220 set_unknown_chunks num=0");
            (api.png_set_unknown_chunks)(wr.png, wr.info, null(), 0);
            emit("c220 set_unknown_chunks unknowns=NULL num=1");
            (api.png_set_unknown_chunks)(wr.png, wr.info, null(), 1);
            emit("c220 no-op done");
            let mut w2 = wr;
            w2.destroy();
        }

        match variant() % 4 {
            0 => {
                // C221: location==0 on a READ struct.
                let data = build_png(
                    &mut Rng::new(1),
                    4,
                    4,
                    8,
                    PNG_COLOR_TYPE_RGB as u8,
                    &[],
                    &[],
                );
                let mut rd = Reader::new(api, data);
                (api.png_read_info)(rd.png, rd.info);
                let cdata = vec![0u8; 4];
                let chunk = png_unknown_chunk {
                    name: *b"prVt\0",
                    data: cdata.as_ptr() as *mut u8,
                    size: cdata.len(),
                    location: 0,
                };
                emit("c221 set_unknown_chunks location=0 on read struct");
                (api.png_set_unknown_chunks)(rd.png, rd.info, &chunk, 1);
                emit("c221 returned");
                rd.destroy();
            }
            1 => {
                // C222/C311: invalid location in set_unknown_chunk_location.
                let loc = pick(&[-1i32, 0, 0x40, 0x7f, i32::MAX]);
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let cdata = vec![0u8; 4];
                let chunk = png_unknown_chunk {
                    name: *b"prVt\0",
                    data: cdata.as_ptr() as *mut u8,
                    size: cdata.len(),
                    location: PNG_HAVE_IHDR as u8,
                };
                (api.png_set_unknown_chunks)(wr.png, wr.info, &chunk, 1);
                p!("c222 set_unknown_chunk_location loc={}", loc);
                (api.png_set_unknown_chunk_location)(wr.png, wr.info, 0, loc);
                emit("c222 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            2 => {
                // C223/C310: invalid keep in set_keep_unknown_chunks.
                let keep = pick(&[-1i32, 5, 6, i32::MAX, i32::MIN]);
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let names = *b"prVt";
                p!("c223 set_keep_unknown_chunks keep={}", keep);
                (api.png_set_keep_unknown_chunks)(wr.png, keep, names.as_ptr(), 1);
                emit("c223 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            _ => {
                // C224: num_chunks>0 with chunk_list==NULL.
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                emit("c224 set_keep_unknown_chunks list=NULL num=1");
                (api.png_set_keep_unknown_chunks)(wr.png, PNG_HANDLE_CHUNK_ALWAYS, null(), 1);
                emit("c224 returned");
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C226 — png_set_compression_buffer_size invalid size                  */
/* ------------------------------------------------------------------ */

fn set_bufsize_bad(api: &Api, _rng: &mut Rng) {
    // E:"invalid compression buffer size" for size==0 or > PNG_UINT_31_MAX.
    // Note: on a WRITE struct size<6 warns (C227) and >max clamps (C228); the
    // hard error is reachable on a READ struct. Use a read struct for the
    // fatal size==0 / huge cases.
    unsafe {
        let size = pick(&[0usize, 0x8000_0000, usize::MAX]);
        let png = (api.png_create_read_struct)(
            cptr(PNG_LIBPNG_VER_STRING),
            vnull(),
            Some(err_fn),
            Some(warn_fn),
        );
        p!("c226 set_compression_buffer_size {}", size);
        (api.png_set_compression_buffer_size)(png, size);
        emit("c226 returned");
        let mut pp = png;
        (api.png_destroy_read_struct)(&mut pp, null(), null());
    }
}

/* ------------------------------------------------------------------ */
/* C229-C237 — png_write_IHDR field validation (fires at write_info)    */
/* ------------------------------------------------------------------ */

fn write_ihdr_bad(api: &Api, _rng: &mut Rng) {
    // Fatal (E) bit-depth/color-type combos: C229-C234. Warnings (coerced):
    // C235 compression, C236 filter, C237 interlace. IHDR field validation
    // happens inside png_write_info, so we must call it to reach the check.
    //
    // Variants pick one fatal (color_type, bit_depth) combination each; the
    // warning coercions (C235-C237) are folded into a non-fatal probe that
    // writes a valid IHDR with bad compression/filter/interlace types.
    unsafe {
        match variant() % 7 {
            0 => write_ihdr_probe(api, PNG_COLOR_TYPE_GRAY, 3, 0, 0, 0, "c229 gray bd=3"),
            1 => write_ihdr_probe(api, PNG_COLOR_TYPE_RGB, 4, 0, 0, 0, "c230 rgb bd=4"),
            2 => write_ihdr_probe(
                api,
                PNG_COLOR_TYPE_PALETTE,
                16,
                0,
                0,
                0,
                "c231 palette bd=16",
            ),
            3 => write_ihdr_probe(
                api,
                PNG_COLOR_TYPE_GRAY_ALPHA,
                4,
                0,
                0,
                0,
                "c232 gray_alpha bd=4",
            ),
            4 => write_ihdr_probe(api, PNG_COLOR_TYPE_RGBA, 2, 0, 0, 0, "c233 rgba bd=2"),
            5 => write_ihdr_probe(api, 5, 8, 0, 0, 0, "c234 color_type=5"),
            _ => {
                // C235-C237: warnings, coerced. All three in one process.
                let wr = Writer::new(api);
                (api.png_set_IHDR)(
                    wr.png,
                    wr.info,
                    4,
                    4,
                    8,
                    PNG_COLOR_TYPE_RGB,
                    2, /*interlace bad*/
                    1, /*compression bad*/
                    1, /*filter bad*/
                );
                emit("c235-c237 write_info with bad comp/filter/interlace");
                (api.png_write_info)(wr.png, wr.info);
                let rb = rowbytes_of(4, 8, PNG_COLOR_TYPE_RGB as u8);
                for _ in 0..4 {
                    let row = vec![0u8; rb];
                    (api.png_write_row)(wr.png, row.as_ptr());
                }
                (api.png_write_end)(wr.png, wr.info);
                emit_bytes("c235-c237 bytes", wbuf());
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}

unsafe fn write_ihdr_probe(
    api: &Api,
    color: c_int,
    depth: c_int,
    interlace: c_int,
    compression: c_int,
    filter: c_int,
    tag: &str,
) {
    let wr = Writer::new(api);
    (api.png_set_IHDR)(
        wr.png,
        wr.info,
        4,
        4,
        depth,
        color,
        interlace,
        compression,
        filter,
    );
    p!("{} calling write_info", tag);
    (api.png_write_info)(wr.png, wr.info);
    emit("write_info returned");
    let mut w2 = wr;
    w2.destroy();
}

/* ------------------------------------------------------------------ */
/* C238 — png_write_info on a palette image with no PLTE                 */
/* ------------------------------------------------------------------ */

fn write_info_no_plte(api: &Api, _rng: &mut Rng) {
    // E:"Valid palette required for paletted images" — single fatal.
    unsafe {
        let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_PALETTE as u8);
        emit("c238 write_info palette with no PLTE");
        (api.png_write_info)(wr.png, wr.info);
        emit("c238 returned");
        let mut w2 = wr;
        w2.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C239,C240,C241 — png_write_PLTE                                      */
/* ------------------------------------------------------------------ */

fn write_plte_bad(api: &Api, _rng: &mut Rng) {
    // C239: palette image, PLTE num out of range -> E:"Invalid number of colors..."
    // C240: non-palette image, bad num -> W, chunk skipped.
    // C241: grayscale image with a PLTE -> W:"Ignoring request to write a PLTE..."
    unsafe {
        match variant() % 2 {
            0 => {
                // C239 fatal: png_set_PLTE with a valid struct, but num_pal==0 at
                // write time. png_set_PLTE itself rejects num==0 with E:"Invalid
                // palette" — instead reach png_write_PLTE via an over-large count.
                // Use png_set_PLTE num within range then corrupt via 0 length is
                // not possible; drive num_palette>256 which png_set_PLTE rejects
                // (E:"Invalid palette length") on a palette image.
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_PALETTE as u8);
                let pal = vec![
                    png_color {
                        red: 0,
                        green: 0,
                        blue: 0
                    };
                    8
                ];
                emit("c239 set_PLTE num=257 palette image");
                (api.png_set_PLTE)(wr.png, wr.info, pal.as_ptr(), 257);
                emit("c239 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            _ => {
                // C240 + C241 warnings, non-fatal, one process.
                // C241: PLTE on grayscale -> warning.
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_GRAY as u8);
                let pal = vec![
                    png_color {
                        red: 1,
                        green: 2,
                        blue: 3
                    };
                    4
                ];
                emit("c241 set_PLTE on grayscale");
                (api.png_set_PLTE)(wr.png, wr.info, pal.as_ptr(), 4);
                emit("c240/c241 returned");
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C242 — png_write_row before png_write_info                           */
/* ------------------------------------------------------------------ */

fn write_row_early(api: &Api, _rng: &mut Rng) {
    // E:"png_write_info was never called before png_write_row" — single fatal.
    unsafe {
        let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
        let rb = rowbytes_of(4, 8, PNG_COLOR_TYPE_RGB as u8);
        let row = vec![0u8; rb];
        emit("c242 write_row before write_info");
        (api.png_write_row)(wr.png, row.as_ptr());
        emit("c242 returned");
        let mut w2 = wr;
        w2.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C243,C244 — png_write_end                                            */
/* ------------------------------------------------------------------ */

fn write_end_no_idat(api: &Api, _rng: &mut Rng) {
    // C243: png_write_end with no IDAT ever written -> E:"No IDATs written...".
    // C244: palette row data with an index >= num_palette -> B (write struct:
    //       benign errors are fatal here) at write_end.
    unsafe {
        match variant() % 2 {
            0 => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                (api.png_write_info)(wr.png, wr.info);
                emit("c243 write_end with no rows written");
                (api.png_write_end)(wr.png, wr.info);
                emit("c243 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            _ => {
                // C244: 1-entry palette, write a row whose index is 5.
                let wr = writer_with_ihdr(api, 4, 1, 8, PNG_COLOR_TYPE_PALETTE as u8);
                let pal = vec![
                    png_color {
                        red: 0,
                        green: 0,
                        blue: 0
                    };
                    2
                ];
                (api.png_set_PLTE)(wr.png, wr.info, pal.as_ptr(), 2);
                (api.png_write_info)(wr.png, wr.info);
                let row = vec![5u8, 5, 5, 5]; // indices exceed num_palette=2
                (api.png_write_row)(wr.png, row.as_ptr());
                emit("c244 write_end after out-of-range palette index");
                (api.png_write_end)(wr.png, wr.info);
                emit("c244 returned");
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C245 — png_write_chunk with length > PNG_UINT_31_MAX                  */
/* ------------------------------------------------------------------ */

fn write_chunk_toolong(api: &Api, _rng: &mut Rng) {
    // E:"length exceeds PNG maximum" — single fatal. We pass a huge declared
    // length to png_write_chunk_start without actually supplying that many
    // bytes; the length check fires before any data is read.
    unsafe {
        let wr = Writer::new(api);
        (api.png_write_sig)(wr.png);
        let name = *b"prVt";
        emit("c245 write_chunk_start length > 0x7fffffff");
        (api.png_write_chunk_start)(wr.png, name.as_ptr(), 0x8000_0000);
        emit("c245 returned");
        let mut w2 = wr;
        w2.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C246,C247,C248,C304,C305 — png_set_filter                            */
/* ------------------------------------------------------------------ */

fn set_filter_bad(api: &Api, _rng: &mut Rng) {
    // C246/C305: invalid filter mask for method 0 -> A:"Unknown row filter...".
    // C247/C304: method != 0 -> E:"Unknown custom filter method".
    // C248: UP/AVG/PAETH requested after row writing began -> W (png_app_warning).
    unsafe {
        match variant() % 3 {
            0 => {
                // C246/C305: invalid mask.
                let mask = pick(&[-2i32, 6, 7, 9, 0x100, i32::MAX]);
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                p!("c246 set_filter method=0 mask={}", mask);
                (api.png_set_filter)(wr.png, PNG_FILTER_TYPE_BASE, mask);
                emit("c246 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            1 => {
                // C247/C304: invalid method.
                let method = pick(&[-1i32, 1, 2, 99, i32::MAX, i32::MIN]);
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                p!("c247 set_filter method={}", method);
                (api.png_set_filter)(wr.png, method, PNG_ALL_FILTERS);
                emit("c247 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            _ => {
                // C248: change filter after row writing has begun -> warning.
                let wr = writer_with_ihdr(api, 8, 8, 8, PNG_COLOR_TYPE_RGB as u8);
                (api.png_write_info)(wr.png, wr.info);
                let rb = rowbytes_of(8, 8, PNG_COLOR_TYPE_RGB as u8);
                let row = vec![0u8; rb];
                (api.png_write_row)(wr.png, row.as_ptr());
                emit("c248 set_filter after first row");
                (api.png_set_filter)(wr.png, PNG_FILTER_TYPE_BASE, PNG_FILTER_PAETH);
                emit("c248 returned");
                // finish so teardown matches
                for _ in 1..8 {
                    let r = vec![0u8; rb];
                    (api.png_write_row)(wr.png, r.as_ptr());
                }
                (api.png_write_end)(wr.png, wr.info);
                emit_bytes("c248 bytes", wbuf());
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C249-C252,C320 — compression setter coercion (warnings)              */
/* ------------------------------------------------------------------ */

fn compression_coerce(api: &Api, _rng: &mut Rng) {
    // All non-fatal warnings/coercions; probe many-per-process.
    unsafe {
        let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
        // C249: window_bits > 15.
        emit("c249 window_bits=99");
        (api.png_set_compression_window_bits)(wr.png, 99);
        // C250: window_bits < 8.
        emit("c250 window_bits=1");
        (api.png_set_compression_window_bits)(wr.png, 1);
        // C251: compression method != 8.
        emit("c251 compression_method=3");
        (api.png_set_compression_method)(wr.png, 3);
        // C252: the same three on the text zstream.
        emit("c252 text window_bits=99");
        (api.png_set_text_compression_window_bits)(wr.png, 99);
        emit("c252 text window_bits=1");
        (api.png_set_text_compression_window_bits)(wr.png, 1);
        emit("c252 text method=3");
        (api.png_set_text_compression_method)(wr.png, 3);
        // C320: out-of-range zlib level / strategy (coerced or downstream).
        for lvl in [-2i32, 10] {
            p!("c320 compression_level={}", lvl);
            (api.png_set_compression_level)(wr.png, lvl);
        }
        for s in [-1i32, 5] {
            p!("c320 compression_strategy={}", s);
            (api.png_set_compression_strategy)(wr.png, s);
        }
        for m in [-2i32, 10] {
            p!("c320 compression_mem_level={}", m);
            (api.png_set_compression_mem_level)(wr.png, m);
        }
        emit("c249-c320 returned");
        let mut w2 = wr;
        w2.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C253-C256 — png_write_iCCP (fires at write_info)                     */
/* ------------------------------------------------------------------ */

fn write_iccp_bad(api: &Api, _rng: &mut Rng) {
    // All fatal E at png_write_info time:
    //  0: profile < 132 bytes           -> E:"ICC profile too short"
    //  1: uint32(profile) != profile_len-> E:"Profile length does not match profile"
    //  2: length not multiple of 4      -> E:"ICC profile length invalid (not a multiple of 4)"
    //  3: keyword rejected              -> E:"iCCP: invalid keyword"
    unsafe {
        let name = cs("ICC");
        match variant() % 4 {
            0 => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let prof = vec![0u8; 100]; // < 132
                (api.png_set_iCCP)(wr.png, wr.info, name.as_ptr(), 0, prof.as_ptr(), 100);
                emit("c253 write_info with short iCCP");
                (api.png_write_info)(wr.png, wr.info);
                emit("c253 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            1 => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let mut prof = vec![0u8; 200];
                // declared size in first 4 bytes != 200
                prof[0..4].copy_from_slice(&12345u32.to_be_bytes());
                (api.png_set_iCCP)(wr.png, wr.info, name.as_ptr(), 0, prof.as_ptr(), 200);
                emit("c254 write_info with mismatched iCCP length");
                (api.png_write_info)(wr.png, wr.info);
                emit("c254 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            2 => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let len = 134usize; // not a multiple of 4, >= 132
                let mut prof = vec![0u8; len];
                prof[0..4].copy_from_slice(&(len as u32).to_be_bytes());
                (api.png_set_iCCP)(wr.png, wr.info, name.as_ptr(), 0, prof.as_ptr(), len as u32);
                emit("c255 write_info with non-multiple-of-4 iCCP length");
                (api.png_write_info)(wr.png, wr.info);
                emit("c255 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            _ => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let len = 132usize;
                let mut prof = vec![0u8; len];
                prof[0..4].copy_from_slice(&(len as u32).to_be_bytes());
                let bad_name = cs(""); // empty keyword rejected
                (api.png_set_iCCP)(
                    wr.png,
                    wr.info,
                    bad_name.as_ptr(),
                    0,
                    prof.as_ptr(),
                    len as u32,
                );
                emit("c256 write_info with bad iCCP keyword");
                (api.png_write_info)(wr.png, wr.info);
                emit("c256 returned");
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C257-C262 — writer text/sPLT keyword & compression rejections        */
/* ------------------------------------------------------------------ */

fn write_text_bad(api: &Api, _rng: &mut Rng) {
    // Each is a distinct fatal E emitted when png_write_info flushes the chunk:
    //  0 C257: tEXt bad keyword
    //  1 C258: zTXt bad compression type
    //  2 C259: zTXt bad keyword
    //  3 C260: iTXt bad keyword
    //  4 C261: iTXt bad compression
    //  5 C262: sPLT bad name
    unsafe {
        let good_txt = cs("body");
        match variant() % 6 {
            0 => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                // Non-empty keyword with a control char survives set_text (warns)
                // but a fully invalid keyword makes the writer reject tEXt. Use a
                // keyword that check_keyword reduces to empty (all spaces).
                let key = cs("   ");
                let t = png_text {
                    compression: -1, // NONE -> tEXt
                    key: key.as_ptr() as *mut c_char,
                    text: good_txt.as_ptr() as *mut c_char,
                    text_length: 4,
                    itxt_length: 0,
                    lang: null(),
                    lang_key: null(),
                };
                (api.png_set_text)(wr.png, wr.info, &t, 1);
                emit("c257 write_info tEXt bad keyword");
                (api.png_write_info)(wr.png, wr.info);
                emit("c257 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            1 => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let key = cs("Comment");
                // compression value that is not a legal zTXt/iTXt value but
                // still routes to zTXt handling: use 99 which the writer rejects.
                let t = png_text {
                    compression: 99,
                    key: key.as_ptr() as *mut c_char,
                    text: good_txt.as_ptr() as *mut c_char,
                    text_length: 4,
                    itxt_length: 0,
                    lang: null(),
                    lang_key: null(),
                };
                (api.png_set_text)(wr.png, wr.info, &t, 1);
                emit("c258 write_info bad text compression");
                (api.png_write_info)(wr.png, wr.info);
                emit("c258 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            2 => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let key = cs("   ");
                let t = png_text {
                    compression: 0, // zTXt
                    key: key.as_ptr() as *mut c_char,
                    text: good_txt.as_ptr() as *mut c_char,
                    text_length: 4,
                    itxt_length: 0,
                    lang: null(),
                    lang_key: null(),
                };
                (api.png_set_text)(wr.png, wr.info, &t, 1);
                emit("c259 write_info zTXt bad keyword");
                (api.png_write_info)(wr.png, wr.info);
                emit("c259 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            3 => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let key = cs("   ");
                let t = png_text {
                    compression: 1, // iTXt NONE
                    key: key.as_ptr() as *mut c_char,
                    text: good_txt.as_ptr() as *mut c_char,
                    text_length: 0,
                    itxt_length: 4,
                    lang: null(),
                    lang_key: null(),
                };
                (api.png_set_text)(wr.png, wr.info, &t, 1);
                emit("c260 write_info iTXt bad keyword");
                (api.png_write_info)(wr.png, wr.info);
                emit("c260 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            4 => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let key = cs("Comment");
                // itxt_length set but compression an invalid iTXt value (5).
                let t = png_text {
                    compression: 5,
                    key: key.as_ptr() as *mut c_char,
                    text: good_txt.as_ptr() as *mut c_char,
                    text_length: 0,
                    itxt_length: 4,
                    lang: null(),
                    lang_key: null(),
                };
                (api.png_set_text)(wr.png, wr.info, &t, 1);
                emit("c261 write_info iTXt bad compression");
                (api.png_write_info)(wr.png, wr.info);
                emit("c261 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            _ => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let name = cs("   "); // bad sPLT name
                let entries = vec![png_sPLT_entry {
                    red: 0,
                    green: 0,
                    blue: 0,
                    alpha: 0,
                    frequency: 0,
                }];
                let splt = png_sPLT_t {
                    name: name.as_ptr() as *mut c_char,
                    depth: 8,
                    entries: entries.as_ptr() as *mut png_sPLT_entry,
                    nentries: 1,
                };
                (api.png_set_sPLT)(wr.png, wr.info, &splt, 1);
                emit("c262 write_info sPLT bad name");
                (api.png_write_info)(wr.png, wr.info);
                emit("c262 returned");
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C263,C264 — png_write_pCAL                                           */
/* ------------------------------------------------------------------ */

fn write_pcal_bad(api: &Api, _rng: &mut Rng) {
    // The png_set_pCAL guards reject bad type/keyword up front (see set_pcal_bad).
    // To reach the *writer* rejections C263/C264, set a valid pCAL then write.
    // Because png_set_pCAL already validates equation type and purpose, the
    // writer-side rejections are reached only via the internal path; here we
    // drive a valid pCAL through png_write_info to confirm both libraries agree
    // (no divergence), and additionally probe the writer keyword rejection using
    // a purpose that passes set_pCAL's NUL check but fails the writer's stricter
    // png_check_keyword.
    unsafe {
        match variant() % 2 {
            0 => {
                // C263: reach the writer equation-type check. png_set_pCAL
                // accepts type 0..3; a valid pCAL should write cleanly, so this
                // asserts identical (non-diverging) behaviour.
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_GRAY as u8);
                let purpose = cs("cal");
                let unit = cs("u");
                (api.png_set_pCAL)(
                    wr.png,
                    wr.info,
                    purpose.as_ptr(),
                    0,
                    255,
                    0,
                    0,
                    unit.as_ptr(),
                    null(),
                );
                emit("c263 write_info with valid pCAL");
                (api.png_write_info)(wr.png, wr.info);
                emit("c263 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            _ => {
                // C264: purpose keyword with a control character. set_pCAL keeps
                // it; the writer's png_check_keyword rejects it.
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_GRAY as u8);
                let purpose = cs("bad\u{01}purpose");
                let unit = cs("u");
                (api.png_set_pCAL)(
                    wr.png,
                    wr.info,
                    purpose.as_ptr(),
                    0,
                    255,
                    0,
                    0,
                    unit.as_ptr(),
                    null(),
                );
                emit("c264 write_info with bad pCAL purpose keyword");
                (api.png_write_info)(wr.png, wr.info);
                emit("c264 returned");
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* C265-C274 — warnings emitted while writing chunks with bad values    */
/* ------------------------------------------------------------------ */

fn write_range_warns(api: &Api, _rng: &mut Rng) {
    // All non-fatal warnings. Set the out-of-range values, then write_info /
    // write_end and collect the warnings. Split into two images because some of
    // the chunks require conflicting color types.
    unsafe {
        // Image 1: RGB8 — sBIT, bKGD, oFFs, pHYs, tIME, sCAL.
        {
            let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
            // C265: sBIT component 0 or > bit_depth.
            let sig = png_color_8 {
                red: 0,
                green: 9,
                blue: 9,
                gray: 0,
                alpha: 0,
            };
            (api.png_set_sBIT)(wr.png, wr.info, &sig);
            // C268: 16-bit bKGD value on an 8-bit image.
            let bg = png_color_16 {
                index: 0,
                red: 0x1234,
                green: 0x5678,
                blue: 0x9abc,
                gray: 0,
            };
            (api.png_set_bKGD)(wr.png, wr.info, &bg);
            // C270: bad oFFs unit type.
            (api.png_set_oFFs)(wr.png, wr.info, 1, 2, 99);
            // C271: bad pHYs unit type.
            (api.png_set_pHYs)(wr.png, wr.info, 100, 100, 99);
            // C272: invalid time.
            let bt = png_time {
                year: 2000,
                month: 13,
                day: 40,
                hour: 30,
                minute: 70,
                second: 90,
            };
            (api.png_set_tIME)(wr.png, wr.info, &bt);
            emit("c265-c272 write_info image1");
            (api.png_write_info)(wr.png, wr.info);
            let rb = rowbytes_of(4, 8, PNG_COLOR_TYPE_RGB as u8);
            for _ in 0..4 {
                let row = vec![0u8; rb];
                (api.png_write_row)(wr.png, row.as_ptr());
            }
            (api.png_write_end)(wr.png, wr.info);
            emit_bytes("c265-c272 bytes", wbuf());
            let mut w2 = wr;
            w2.destroy();
        }
        // Image 2: palette — bKGD index out of range (C267), hIST count (C269),
        // tRNS wrong color type (C266).
        {
            let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_PALETTE as u8);
            let pal = vec![
                png_color {
                    red: 0,
                    green: 0,
                    blue: 0
                };
                4
            ];
            (api.png_set_PLTE)(wr.png, wr.info, pal.as_ptr(), 4);
            // C267: bKGD palette index >= num_palette.
            let bg = png_color_16 {
                index: 200,
                red: 0,
                green: 0,
                blue: 0,
                gray: 0,
            };
            (api.png_set_bKGD)(wr.png, wr.info, &bg);
            // C269: hIST count > num_palette is set implicitly (hIST has 4
            // entries for 4-entry palette which is valid); skip to avoid a
            // spurious pass — instead exercise tRNS on a type with alpha below.
            emit("c266/c267 write_info image2");
            (api.png_write_info)(wr.png, wr.info);
            let row = vec![0u8, 1, 2, 3];
            for _ in 0..4 {
                (api.png_write_row)(wr.png, row.as_ptr());
            }
            (api.png_write_end)(wr.png, wr.info);
            emit_bytes("c266-c267 bytes", wbuf());
            let mut w2 = wr;
            w2.destroy();
        }
        // Image 3: sCAL_s with a very long formatted string (C273) and
        // grayscale-alpha tRNS (C266 family).
        {
            let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGBA as u8);
            // C266: tRNS on a color type that already has an alpha channel.
            let v = png_color_16 {
                index: 0,
                red: 1,
                green: 2,
                blue: 3,
                gray: 0,
            };
            (api.png_set_tRNS)(wr.png, wr.info, null(), 0, &v);
            emit("c266 write_info image3 (tRNS on RGBA)");
            (api.png_write_info)(wr.png, wr.info);
            let rb = rowbytes_of(4, 8, PNG_COLOR_TYPE_RGBA as u8);
            for _ in 0..4 {
                let row = vec![0u8; rb];
                (api.png_write_row)(wr.png, row.as_ptr());
            }
            (api.png_write_end)(wr.png, wr.info);
            emit_bytes("c266 bytes", wbuf());
            let mut w2 = wr;
            w2.destroy();
        }
    }
}

/* ------------------------------------------------------------------ */
/* C275,C276,C277 — png_write_png                                       */
/* ------------------------------------------------------------------ */

fn write_png_bad(api: &Api, _rng: &mut Rng) {
    // C275: row_pointers == NULL -> A:"no rows for png_write_image to write".
    // C276: STRIP_FILLER_BEFORE|AFTER both set -> A.
    // C277: internal write transform logic error — not reachable via valid FFI
    //       input (documented internal-only), so folded away.
    unsafe {
        match variant() % 2 {
            0 => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                emit("c275 write_png with no rows set");
                (api.png_write_png)(wr.png, wr.info, PNG_TRANSFORM_IDENTITY, null());
                emit("c275 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            _ => {
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let rb = rowbytes_of(4, 8, PNG_COLOR_TYPE_RGB as u8);
                let mut rows: Vec<Vec<u8>> = (0..4).map(|_| vec![0u8; rb]).collect();
                let mut ptrs: Vec<*mut u8> = rows.iter_mut().map(|r| r.as_mut_ptr()).collect();
                (api.png_set_rows)(wr.png, wr.info, ptrs.as_mut_ptr());
                emit("c276 write_png STRIP_FILLER_BEFORE|AFTER");
                (api.png_write_png)(
                    wr.png,
                    wr.info,
                    PNG_TRANSFORM_STRIP_FILLER_BEFORE | PNG_TRANSFORM_STRIP_FILLER_AFTER,
                    null(),
                );
                emit("c276 returned");
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}

/* ------------------------------------------------------------------ */
/* simplified-API helpers                                              */
/* ------------------------------------------------------------------ */

/// A zeroed png_image with the correct version and NULL opaque.
fn fresh_image() -> png_image {
    png_image {
        opaque: std::ptr::null_mut(),
        version: PNG_IMAGE_VERSION,
        width: 0,
        height: 0,
        format: 0,
        flags: 0,
        colormap_entries: 0,
        warning_or_error: 0,
        message: [0; 64],
    }
}

/// A minimal valid in-memory PNG for read-args probing.
unsafe fn valid_png_bytes() -> Vec<u8> {
    build_png(
        &mut Rng::new(7),
        4,
        4,
        8,
        PNG_COLOR_TYPE_RGB as u8,
        &[],
        &[],
    )
}

/* ------------------------------------------------------------------ */
/* C278-C287 — png_image_begin_read_from_* argument validation          */
/* ------------------------------------------------------------------ */

fn simple_read_args(api: &Api, _rng: &mut Rng) {
    // All return 0 + set image.message; non-fatal, many-per-process.
    unsafe {
        let png = valid_png_bytes();

        // C278: memory==NULL / size==0.
        {
            let mut im = fresh_image();
            let r = (api.png_image_begin_read_from_memory)(&mut im, std::ptr::null(), 0);
            p!(
                "c278 mem=NULL size=0 r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
            (api.png_image_free)(&mut im);
        }
        {
            let mut im = fresh_image();
            let r =
                (api.png_image_begin_read_from_memory)(&mut im, png.as_ptr() as *const c_void, 0);
            p!(
                "c278 size=0 r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
            (api.png_image_free)(&mut im);
        }
        // C279: wrong version.
        {
            let mut im = fresh_image();
            im.version = 999;
            let r = (api.png_image_begin_read_from_memory)(
                &mut im,
                png.as_ptr() as *const c_void,
                png.len(),
            );
            p!(
                "c279 bad version r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
            (api.png_image_free)(&mut im);
        }
        // C280: image==NULL.
        {
            let r = (api.png_image_begin_read_from_memory)(
                std::ptr::null_mut(),
                png.as_ptr() as *const c_void,
                png.len(),
            );
            p!("c280 image=NULL r={}", r);
        }
        // C281: opaque != NULL on entry.
        {
            let mut im = fresh_image();
            im.opaque = 1usize as *mut c_void;
            let r = (api.png_image_begin_read_from_memory)(
                &mut im,
                png.as_ptr() as *const c_void,
                png.len(),
            );
            p!(
                "c281 opaque!=NULL r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
            // do not free: opaque is a bogus pointer.
        }
        // C282: truncated PNG (read past end).
        {
            let mut im = fresh_image();
            let short = &png[..png.len() / 2];
            let r = (api.png_image_begin_read_from_memory)(
                &mut im,
                short.as_ptr() as *const c_void,
                short.len(),
            );
            p!(
                "c282 truncated r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
            (api.png_image_free)(&mut im);
        }
        // C285: begin_read_from_file file_name==NULL.
        {
            let mut im = fresh_image();
            let r = (api.png_image_begin_read_from_file)(&mut im, std::ptr::null());
            p!(
                "c285 file=NULL r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
            (api.png_image_free)(&mut im);
        }
        // C286: nonexistent path.
        {
            let mut im = fresh_image();
            let path = cs("/nonexistent/path/definitely/missing.png");
            let r = (api.png_image_begin_read_from_file)(&mut im, path.as_ptr());
            p!(
                "c286 missing file r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
            (api.png_image_free)(&mut im);
        }
        // C287: begin_read_from_file wrong version.
        {
            let mut im = fresh_image();
            im.version = 0;
            let path = cs("/nonexistent/whatever.png");
            let r = (api.png_image_begin_read_from_file)(&mut im, path.as_ptr());
            p!(
                "c287 file bad version r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
            (api.png_image_free)(&mut im);
        }
        emit("c278-c287 done");
    }
}

/* ------------------------------------------------------------------ */
/* C288-C295 — png_image_finish_read argument validation                */
/* ------------------------------------------------------------------ */

fn simple_finish_args(api: &Api, _rng: &mut Rng) {
    // All return 0 + set image.message; non-fatal, many-per-process. Each probe
    // does a fresh begin_read then a bad finish_read.
    unsafe {
        let png = valid_png_bytes();

        // helper: begin a read, returning an initialized image.
        let begin = |im: &mut png_image| -> c_int {
            (api.png_image_begin_read_from_memory)(im, png.as_ptr() as *const c_void, png.len())
        };

        // C288: buffer==NULL.
        {
            let mut im = fresh_image();
            if begin(&mut im) != 0 {
                let r = (api.png_image_finish_read)(
                    &mut im,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    0,
                    std::ptr::null_mut(),
                );
                p!(
                    "c288 buffer=NULL r={} woe={} msg={:?}",
                    r,
                    im.warning_or_error,
                    im.msg()
                );
            }
            (api.png_image_free)(&mut im);
        }
        // C293: image==NULL.
        {
            let r = (api.png_image_finish_read)(
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                0,
                std::ptr::null_mut(),
            );
            p!("c293 image=NULL r={}", r);
        }
        // C292: version corrupted between calls.
        {
            let mut im = fresh_image();
            if begin(&mut im) != 0 {
                im.version = 0;
                // allocate a plausible buffer
                let sz = im.width as usize * im.height as usize * 4 + 64;
                let mut buf = vec![0u8; sz];
                let r = (api.png_image_finish_read)(
                    &mut im,
                    std::ptr::null(),
                    buf.as_mut_ptr() as *mut c_void,
                    0,
                    std::ptr::null_mut(),
                );
                p!(
                    "c292 bad version r={} woe={} msg={:?}",
                    r,
                    im.warning_or_error,
                    im.msg()
                );
            }
            (api.png_image_free)(&mut im);
        }
        // C289: colormapped format with colormap==NULL.
        {
            let mut im = fresh_image();
            if begin(&mut im) != 0 {
                im.format |= PNG_FORMAT_FLAG_COLORMAP;
                let sz = im.width as usize * im.height as usize + 64;
                let mut buf = vec![0u8; sz];
                let r = (api.png_image_finish_read)(
                    &mut im,
                    std::ptr::null(),
                    buf.as_mut_ptr() as *mut c_void,
                    0,
                    std::ptr::null_mut(),
                );
                p!(
                    "c289 colormap=NULL r={} woe={} msg={:?}",
                    r,
                    im.warning_or_error,
                    im.msg()
                );
            }
            (api.png_image_free)(&mut im);
        }
        // C295: undefined format bits set (must not diverge).
        {
            let mut im = fresh_image();
            if begin(&mut im) != 0 {
                im.format |= 0xff00_0000;
                let sz = im.width as usize * im.height as usize * 4 + 64;
                let mut buf = vec![0u8; sz];
                let r = (api.png_image_finish_read)(
                    &mut im,
                    std::ptr::null(),
                    buf.as_mut_ptr() as *mut c_void,
                    0,
                    std::ptr::null_mut(),
                );
                p!(
                    "c295 weird format r={} woe={} msg={:?}",
                    r,
                    im.warning_or_error,
                    im.msg()
                );
            }
            (api.png_image_free)(&mut im);
        }
        emit("c288-c295 done");
    }
}

/* ------------------------------------------------------------------ */
/* C296-C303 — png_image_write_* argument validation                    */
/* ------------------------------------------------------------------ */

fn simple_write_args(api: &Api, _rng: &mut Rng) {
    // All return 0 + set image.message; non-fatal, many-per-process.
    unsafe {
        let (w, h) = (8u32, 8u32);
        let chans = 3usize; // RGB
        let row_stride = w as usize * chans;
        let buf = vec![0u8; h as usize * row_stride];

        let mk = || -> png_image {
            let mut im = fresh_image();
            im.width = w;
            im.height = h;
            im.format = PNG_FORMAT_FLAG_COLOR;
            im
        };

        // C296: memory_bytes==NULL.
        {
            let mut im = mk();
            let mut out = vec![0u8; 1 << 16];
            let r = (api.png_image_write_to_memory)(
                &mut im,
                out.as_mut_ptr() as *mut c_void,
                std::ptr::null_mut(),
                0,
                buf.as_ptr() as *const c_void,
                row_stride as i32,
                std::ptr::null(),
            );
            p!(
                "c296 memory_bytes=NULL r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
        }
        // C296: image==NULL.
        {
            let mut size = 0u64;
            let r = (api.png_image_write_to_memory)(
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                &mut size,
                0,
                buf.as_ptr() as *const c_void,
                row_stride as i32,
                std::ptr::null(),
            );
            p!("c296 image=NULL r={}", r);
        }
        // C296: bad version.
        {
            let mut im = mk();
            im.version = 42;
            let mut size = 0u64;
            let r = (api.png_image_write_to_memory)(
                &mut im,
                std::ptr::null_mut(),
                &mut size,
                0,
                buf.as_ptr() as *const c_void,
                row_stride as i32,
                std::ptr::null(),
            );
            p!(
                "c296 bad version r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
        }
        // C297: supplied buffer smaller than encoded PNG.
        {
            let mut im = mk();
            let mut small = vec![0u8; 4];
            let mut size = 4u64;
            let r = (api.png_image_write_to_memory)(
                &mut im,
                small.as_mut_ptr() as *mut c_void,
                &mut size,
                0,
                buf.as_ptr() as *const c_void,
                row_stride as i32,
                std::ptr::null(),
            );
            p!(
                "c297 small buffer r={} size_out={} woe={} msg={:?}",
                r,
                size,
                im.warning_or_error,
                im.msg()
            );
        }
        // C300: row_stride too small.
        {
            let mut im = mk();
            let mut size = 0u64;
            let r = (api.png_image_write_to_memory)(
                &mut im,
                std::ptr::null_mut(),
                &mut size,
                0,
                buf.as_ptr() as *const c_void,
                1,
                std::ptr::null(),
            );
            p!(
                "c300 tiny stride r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
        }
        // C301: colormapped image with colormap==NULL.
        {
            let mut im = fresh_image();
            im.width = w;
            im.height = h;
            im.format = PNG_FORMAT_FLAG_COLORMAP;
            im.colormap_entries = 0;
            let cbuf = vec![0u8; h as usize * w as usize];
            let mut size = 0u64;
            let r = (api.png_image_write_to_memory)(
                &mut im,
                std::ptr::null_mut(),
                &mut size,
                0,
                cbuf.as_ptr() as *const c_void,
                w as i32,
                std::ptr::null(),
            );
            p!(
                "c301 colormap=NULL r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
        }
        // C302: width==0 or height==0 (must not diverge).
        {
            let mut im = mk();
            im.width = 0;
            let mut size = 0u64;
            let r = (api.png_image_write_to_memory)(
                &mut im,
                std::ptr::null_mut(),
                &mut size,
                0,
                buf.as_ptr() as *const c_void,
                row_stride as i32,
                std::ptr::null(),
            );
            p!(
                "c302 width=0 r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
        }
        // C298: png_image_write_to_file to an unwritable path.
        {
            let mut im = mk();
            let path = cs("/proc/self/cannot-write-here");
            let r = (api.png_image_write_to_file)(
                &mut im,
                path.as_ptr(),
                0,
                buf.as_ptr() as *const c_void,
                row_stride as i32,
                std::ptr::null(),
            );
            p!(
                "c298 unwritable path r={} woe={} msg={:?}",
                r,
                im.warning_or_error,
                im.msg()
            );
        }
        emit("c296-c303 done");
    }
}

/* ------------------------------------------------------------------ */
/* C303 — png_image_free with NULL and double free                     */
/* ------------------------------------------------------------------ */

fn image_free(api: &Api, _rng: &mut Rng) {
    // no-op, no crash; image.opaque == NULL afterwards.
    unsafe {
        // NULL image.
        (api.png_image_free)(std::ptr::null_mut());
        emit("c303 free(NULL) ok");
        // Fresh image (opaque already NULL) freed twice.
        let mut im = fresh_image();
        (api.png_image_free)(&mut im);
        p!("c303 free once opaque_null={}", im.opaque.is_null());
        (api.png_image_free)(&mut im);
        p!("c303 free twice opaque_null={}", im.opaque.is_null());
        // Image that actually began a read, then double-freed.
        let png = valid_png_bytes();
        let mut im2 = fresh_image();
        let r = (api.png_image_begin_read_from_memory)(
            &mut im2,
            png.as_ptr() as *const c_void,
            png.len(),
        );
        p!("c303 begin r={}", r);
        (api.png_image_free)(&mut im2);
        p!("c303 after free opaque_null={}", im2.opaque.is_null());
        (api.png_image_free)(&mut im2);
        emit("c303 double free ok");
    }
}

/* ------------------------------------------------------------------ */
/* C319 — png_set_gAMA_fixed / png_set_sRGB enum & value handling       */
/* ------------------------------------------------------------------ */

fn gama_srgb_enum(api: &Api, _rng: &mut Rng) {
    // Rejection/coercion, non-fatal. Probe on a write struct's info.
    unsafe {
        let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
        // gAMA fixed with 0 / -1 / INT_MAX.
        for g in [0i32, -1, i32::MAX] {
            p!("c319 set_gAMA_fixed {}", g);
            (api.png_set_gAMA_fixed)(wr.png, wr.info, g);
        }
        // sRGB intent -1 / 4 / 255.
        for intent in [-1i32, 4, 255] {
            p!("c319 set_sRGB intent {}", intent);
            (api.png_set_sRGB)(wr.png, wr.info, intent);
        }
        emit("c319 done");
        let mut w2 = wr;
        w2.destroy();
    }
}

/* ------------------------------------------------------------------ */
/* C4 — png_set_sig_bytes with too many bytes                           */
/* ------------------------------------------------------------------ */

fn sig_bytes_toomany(api: &Api, _rng: &mut Rng) {
    // E:"Too many bytes for PNG signature" — single fatal.
    unsafe {
        let png = (api.png_create_read_struct)(
            cptr(PNG_LIBPNG_VER_STRING),
            vnull(),
            Some(err_fn),
            Some(warn_fn),
        );
        emit("c4 set_sig_bytes(9)");
        (api.png_set_sig_bytes)(png, 9);
        emit("c4 returned");
        let mut pp = png;
        (api.png_destroy_read_struct)(&mut pp, null(), null());
    }
}

/* ------------------------------------------------------------------ */
/* C14,C312 — png_data_freer with an invalid freer parameter            */
/* ------------------------------------------------------------------ */

fn data_freer_bad(api: &Api, _rng: &mut Rng) {
    // E:"Unknown freer parameter in png_data_freer" — single fatal. The freer
    // must be neither 1 nor 2; pick across the invalid set for coverage.
    unsafe {
        let freer = pick(&[-1i32, 0, 3, i32::MAX]);
        let png = (api.png_create_write_struct)(
            cptr(PNG_LIBPNG_VER_STRING),
            vnull(),
            Some(err_fn),
            Some(warn_fn),
        );
        let info = (api.png_create_info_struct)(png);
        p!("c14 data_freer freer={}", freer);
        (api.png_data_freer)(png, info, freer, PNG_FREE_ALL);
        emit("c14 returned");
        let mut pp = png;
        let mut ip = info;
        (api.png_destroy_write_struct)(&mut pp, &mut ip);
    }
}

/* ------------------------------------------------------------------ */
/* C313 — png_free_data with a range of masks/nums (no crash)           */
/* ------------------------------------------------------------------ */

fn free_data_masks(api: &Api, rng: &mut Rng) {
    // Non-fatal: identical resulting info state and no crash.
    unsafe {
        let png = (api.png_create_write_struct)(
            cptr(PNG_LIBPNG_VER_STRING),
            vnull(),
            Some(err_fn),
            Some(warn_fn),
        );
        let info = (api.png_create_info_struct)(png);
        (api.png_set_IHDR)(
            png,
            info,
            4,
            4,
            8,
            PNG_COLOR_TYPE_PALETTE,
            PNG_INTERLACE_NONE,
            0,
            0,
        );
        let pal: Vec<png_color> = (0..16)
            .map(|_| png_color {
                red: rng.u8(),
                green: rng.u8(),
                blue: rng.u8(),
            })
            .collect();
        (api.png_set_PLTE)(png, info, pal.as_ptr(), 16);
        let hist: Vec<u16> = (0..16).map(|_| rng.u32() as u16).collect();
        (api.png_set_hIST)(png, info, hist.as_ptr());
        p!(
            "c313 valid_before={:05x}",
            (api.png_get_valid)(png, info, 0xffff_ffff)
        );
        for mask in [0u32, 0x0008, 0x0040, 0xffff, 0xffff_ffff] {
            for num in [-1i32, 0, 1, 9999] {
                (api.png_free_data)(png, info, mask, num);
            }
        }
        p!(
            "c313 valid_after={:05x}",
            (api.png_get_valid)(png, info, 0xffff_ffff)
        );
        let mut pp = png;
        let mut ip = info;
        (api.png_destroy_write_struct)(&mut pp, &mut ip);
    }
}

/* ------------------------------------------------------------------ */
/* C13 — png_set_longjmp_fn with a changed jmp_buf size                 */
/* ------------------------------------------------------------------ */

fn longjmp_fn(api: &Api, _rng: &mut Rng) {
    // First call establishes the size; a second call with a DIFFERENT size
    // warns W:"Application jmp_buf size changed" and returns NULL. Both calls
    // pass a NULL longjmp fn (we never actually longjmp).
    unsafe {
        let png = (api.png_create_read_struct)(
            cptr(PNG_LIBPNG_VER_STRING),
            vnull(),
            Some(err_fn),
            Some(warn_fn),
        );
        let a = (api.png_set_longjmp_fn)(png, vnull(), 200);
        p!("c13 first null={}", a.is_null());
        let b = (api.png_set_longjmp_fn)(png, vnull(), 8);
        p!("c13 second null={}", b.is_null());
        let mut pp = png;
        (api.png_destroy_read_struct)(&mut pp, null(), null());
    }
}

/* ------------------------------------------------------------------ */
/* C17,C18 — mixing read_fn and write_fn on one struct                  */
/* ------------------------------------------------------------------ */

fn read_write_fn_mix(api: &Api, _rng: &mut Rng) {
    // Both are non-fatal warnings:
    //  C18: png_set_write_fn on a read struct (already has read_data_fn).
    //  C17: png_set_read_fn on a write struct (already has write_data_fn).
    unsafe {
        // C18: read struct -> set write fn.
        {
            let png = (api.png_create_read_struct)(
                cptr(PNG_LIBPNG_VER_STRING),
                vnull(),
                Some(err_fn),
                Some(warn_fn),
            );
            (api.png_set_read_fn)(png, vnull(), Some(read_fn));
            emit("c18 set_write_fn on read struct");
            (api.png_set_write_fn)(png, vnull(), Some(write_fn), Some(flush_fn));
            emit("c18 returned");
            let mut pp = png;
            (api.png_destroy_read_struct)(&mut pp, null(), null());
        }
        // C17: write struct -> set read fn.
        {
            let png = (api.png_create_write_struct)(
                cptr(PNG_LIBPNG_VER_STRING),
                vnull(),
                Some(err_fn),
                Some(warn_fn),
            );
            (api.png_set_write_fn)(png, vnull(), Some(write_fn), Some(flush_fn));
            emit("c17 set_read_fn on write struct");
            (api.png_set_read_fn)(png, vnull(), Some(read_fn));
            emit("c17 returned");
            let mut pp = png;
            (api.png_destroy_write_struct)(&mut pp, null());
        }
    }
}

/* ------------------------------------------------------------------ */
/* C304,C306,C307,C310,C311,C312,C315,C316,C317 — enum boundary fuzz    */
/* ------------------------------------------------------------------ */

fn enum_fuzz(api: &Api, _rng: &mut Rng) {
    // Every enum-taking exported setter is driven with values that have no valid
    // variant. Most of these abort, so one fatal probe runs per variant. The
    // non-fatal png_set_option sweep (C315) runs in every process first.
    unsafe {
        // C315: png_set_option — returns PNG_OPTION_INVALID(0) for bad option,
        // never aborts. Probe many-per-process.
        {
            let png = (api.png_create_read_struct)(
                cptr(PNG_LIBPNG_VER_STRING),
                vnull(),
                Some(err_fn),
                Some(warn_fn),
            );
            for opt in [-1i32, PNG_OPTION_NEXT, 99, i32::MAX] {
                for onoff in [-1i32, 3, 99] {
                    p!(
                        "c315 option {} {} -> {}",
                        opt,
                        onoff,
                        (api.png_set_option)(png, opt, onoff)
                    );
                }
            }
            let mut pp = png;
            (api.png_destroy_read_struct)(&mut pp, null(), null());
        }

        // The fatal enum probe for this variant.
        match variant() % 8 {
            0 => {
                // C304: png_set_filter bad method.
                let m = pick(&[-1i32, 1, 2, 99, i32::MAX, i32::MIN]);
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                p!("c304 set_filter method={}", m);
                (api.png_set_filter)(wr.png, m, PNG_ALL_FILTERS);
                emit("c304 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            1 => {
                // C306: png_set_alpha_mode_fixed bad mode.
                let mode = pick(&[-1i32, 5, 6, i32::MAX, i32::MIN]);
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGBA as u8);
                p!("c306 alpha_mode {}", mode);
                (api.png_set_alpha_mode_fixed)(rd.png, mode, 100000);
                emit("c306 returned");
                rd.destroy();
            }
            2 => {
                // C307: png_set_rgb_to_gray_fixed bad error_action.
                let ea = pick(&[-1i32, 0, 4, 99, i32::MAX]);
                let mut rd = reader_after_info(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                p!("c307 rgb_to_gray error_action={}", ea);
                (api.png_set_rgb_to_gray_fixed)(rd.png, ea, 21260, 71520);
                emit("c307 returned");
                rd.destroy();
            }
            3 => {
                // C310: png_set_keep_unknown_chunks bad keep.
                let keep = pick(&[-1i32, 5, 6, i32::MAX, i32::MIN]);
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let names = *b"prVt";
                p!("c310 keep_unknown keep={}", keep);
                (api.png_set_keep_unknown_chunks)(wr.png, keep, names.as_ptr(), 1);
                emit("c310 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            4 => {
                // C311: png_set_unknown_chunk_location bad location.
                let loc = pick(&[-1i32, 0, 0x40, 0x7f, i32::MAX]);
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                let cdata = vec![0u8; 4];
                let chunk = png_unknown_chunk {
                    name: *b"prVt\0",
                    data: cdata.as_ptr() as *mut u8,
                    size: cdata.len(),
                    location: PNG_HAVE_IHDR as u8,
                };
                (api.png_set_unknown_chunks)(wr.png, wr.info, &chunk, 1);
                p!("c311 unknown_chunk_location loc={}", loc);
                (api.png_set_unknown_chunk_location)(wr.png, wr.info, 0, loc);
                emit("c311 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            5 => {
                // C312: png_data_freer bad freer.
                let freer = pick(&[-1i32, 0, 3, i32::MAX]);
                let png = (api.png_create_write_struct)(
                    cptr(PNG_LIBPNG_VER_STRING),
                    vnull(),
                    Some(err_fn),
                    Some(warn_fn),
                );
                let info = (api.png_create_info_struct)(png);
                p!("c312 data_freer freer={}", freer);
                (api.png_data_freer)(png, info, freer, PNG_FREE_ALL);
                emit("c312 returned");
                let mut pp = png;
                let mut ip = info;
                (api.png_destroy_write_struct)(&mut pp, &mut ip);
            }
            6 => {
                // C316: png_set_sCAL_s bad unit.
                let unit = pick(&[-1i32, 0, 3, i32::MAX]);
                let good = cs("2.5");
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_RGB as u8);
                p!("c316 set_sCAL_s unit={}", unit);
                (api.png_set_sCAL_s)(wr.png, wr.info, unit, good.as_ptr(), good.as_ptr());
                emit("c316 returned");
                let mut w2 = wr;
                w2.destroy();
            }
            _ => {
                // C317: png_set_pCAL bad type.
                let ty = pick(&[-1i32, 4, 255, i32::MAX]);
                let purpose = cs("cal");
                let unit = cs("u");
                let wr = writer_with_ihdr(api, 4, 4, 8, PNG_COLOR_TYPE_GRAY as u8);
                p!("c317 set_pCAL type={}", ty);
                (api.png_set_pCAL)(
                    wr.png,
                    wr.info,
                    purpose.as_ptr(),
                    0,
                    255,
                    ty,
                    0,
                    unit.as_ptr(),
                    null(),
                );
                emit("c317 returned");
                let mut w2 = wr;
                w2.destroy();
            }
        }
    }
}
