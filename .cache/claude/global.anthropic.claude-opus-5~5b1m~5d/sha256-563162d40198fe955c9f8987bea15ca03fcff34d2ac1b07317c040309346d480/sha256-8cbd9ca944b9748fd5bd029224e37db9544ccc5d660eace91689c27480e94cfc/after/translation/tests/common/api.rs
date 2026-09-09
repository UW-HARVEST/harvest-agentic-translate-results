//! Every libpng entry point used by the tests, declared once.  Each wrapper
//! takes the `Library` to call as its first argument and resolves the symbol
//! with `dlsym`, so the identical test body drives the C `.so` and the Rust
//! `.so`.
#![allow(dead_code)]
#![allow(non_snake_case)]

use super::*;
use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

pub type PngRw = unsafe extern "C-unwind" fn(png_structp, png_bytep, usize);
pub type PngFlush = unsafe extern "C-unwind" fn(png_structp);
pub type PngErr = unsafe extern "C-unwind" fn(png_structp, png_const_charp);
pub type PngRowCb = unsafe extern "C-unwind" fn(png_structp, png_uint_32, c_int);
pub type PngInfoCb = unsafe extern "C-unwind" fn(png_structp, png_infop);
pub type PngProgRowCb = unsafe extern "C-unwind" fn(png_structp, png_bytep, png_uint_32, c_int);
pub type PngEndCb = unsafe extern "C-unwind" fn(png_structp, png_infop);
pub type PngUserChunkCb =
    unsafe extern "C-unwind" fn(png_structp, *mut png_unknown_chunk) -> c_int;
pub type PngUserTransformCb =
    unsafe extern "C-unwind" fn(png_structp, *mut png_row_info, png_bytep);
pub type PngMalloc = unsafe extern "C-unwind" fn(png_structp, usize) -> png_voidp;
pub type PngFreeFn = unsafe extern "C-unwind" fn(png_structp, png_voidp);

crate::decl_api! {
    // ---- version / misc
    fn png_access_version_number() -> png_uint_32;
    fn png_get_libpng_ver(pp: png_structp) -> png_const_charp;
    fn png_get_header_ver(pp: png_structp) -> png_const_charp;
    fn png_get_header_version(pp: png_structp) -> png_const_charp;
    fn png_get_copyright(pp: png_structp) -> png_const_charp;
    fn png_sig_cmp(sig: png_const_bytep, start: usize, num: usize) -> c_int;

    // ---- byte order helpers
    fn png_save_uint_32(buf: png_bytep, i: png_uint_32);
    fn png_save_int_32(buf: png_bytep, i: png_int_32);
    fn png_save_uint_16(buf: png_bytep, i: c_uint);
    fn png_get_uint_32(buf: png_const_bytep) -> png_uint_32;
    fn png_get_uint_16(buf: png_const_bytep) -> png_uint_16;
    fn png_get_int_32(buf: png_const_bytep) -> png_int_32;
    fn png_get_uint_31(pp: png_structp, buf: png_const_bytep) -> png_uint_32;

    // ---- arithmetic helpers
    fn png_muldiv(res: *mut png_fixed_point, a: png_fixed_point, times: png_int_32,
                  divisor: png_int_32) -> c_int;
    fn png_reciprocal(a: png_fixed_point) -> png_fixed_point;
    fn png_reciprocal2(a: png_fixed_point, b: png_fixed_point) -> png_fixed_point;
    fn png_fixed(pp: png_structp, fp: c_double, text: png_const_charp) -> png_fixed_point;
    fn png_fixed_ITU(pp: png_structp, fp: c_double, text: png_const_charp) -> png_fixed_point;
    fn png_gamma_significant(g: png_fixed_point) -> c_int;
    fn png_gamma_8bit_correct(value: c_uint, g: png_fixed_point) -> png_byte;
    fn png_gamma_16bit_correct(value: c_uint, g: png_fixed_point) -> png_uint_16;
    fn png_gamma_correct(pp: png_structp, value: c_uint, g: png_fixed_point) -> png_uint_16;
    fn png_XYZ_from_xy(xyz: *mut c_void, xy: *const c_void) -> c_int;
    fn png_xy_from_XYZ(xy: *mut c_void, xyz: *const c_void) -> c_int;
    fn png_check_fp_number(s: png_const_charp, size: usize, statep: *mut c_int,
                           whereami: *mut usize) -> c_int;
    fn png_check_fp_string(s: png_const_charp, size: usize) -> c_int;
    fn png_ascii_from_fp(pp: png_structp, ascii: png_charp, size: usize, fp: c_double,
                         precision: c_uint);
    fn png_ascii_from_fixed(pp: png_structp, ascii: png_charp, size: usize,
                            fp: png_fixed_point);
    fn png_safecat(buffer: png_charp, bufsize: usize, pos: usize, string: png_const_charp)
                  -> usize;
    fn png_format_number(start: png_const_charp, end: png_charp, format: c_int,
                         number: c_ulong) -> png_charp;
    fn png_warning_parameter(p: *mut c_char, number: c_int, string: png_const_charp);
    fn png_warning_parameter_unsigned(p: *mut c_char, number: c_int, format: c_int,
                                      value: png_uint_32);
    fn png_warning_parameter_signed(p: *mut c_char, number: c_int, format: c_int,
                                    value: png_int_32);
    fn png_formatted_warning(pp: png_structp, p: *mut c_char, message: png_const_charp);
    fn png_check_IHDR(pp: png_structp, width: png_uint_32, height: png_uint_32,
                      bit_depth: c_int, color_type: c_int, interlace_type: c_int,
                      compression_type: c_int, filter_type: c_int);
    fn png_build_grayscale_palette(bit_depth: c_int, palette: *mut png_color);
    fn png_convert_to_rfc1123_buffer(out: png_charp, t: *const png_time) -> c_int;
    fn png_convert_to_rfc1123(pp: png_structp, t: *const png_time) -> png_charp;
    fn png_convert_from_struct_tm(ptime: *mut png_time, tm: *const c_void);
    fn png_convert_from_time_t(ptime: *mut png_time, t: c_long);

    // ---- create / destroy / memory
    fn png_create_read_struct(ver: png_const_charp, ep: png_voidp,
                              ef: Option<PngErr>, wf: Option<PngErr>) -> png_structp;
    fn png_create_write_struct(ver: png_const_charp, ep: png_voidp,
                               ef: Option<PngErr>, wf: Option<PngErr>) -> png_structp;
    fn png_create_read_struct_2(ver: png_const_charp, ep: png_voidp, ef: Option<PngErr>,
                                wf: Option<PngErr>, mp: png_voidp,
                                mf: Option<PngMalloc>, ff: Option<PngFreeFn>) -> png_structp;
    fn png_create_write_struct_2(ver: png_const_charp, ep: png_voidp, ef: Option<PngErr>,
                                 wf: Option<PngErr>, mp: png_voidp,
                                 mf: Option<PngMalloc>, ff: Option<PngFreeFn>) -> png_structp;
    fn png_create_info_struct(pp: png_structp) -> png_infop;
    fn png_destroy_info_struct(pp: png_structp, ipp: *mut png_infop);
    fn png_info_init_3(ipp: *mut png_infop, size: usize);
    fn png_destroy_read_struct(ppp: *mut png_structp, ipp: *mut png_infop,
                               eipp: *mut png_infop);
    fn png_destroy_write_struct(ppp: *mut png_structp, ipp: *mut png_infop);
    fn png_free_data(pp: png_structp, ip: png_infop, mask: png_uint_32, num: c_int);
    fn png_data_freer(pp: png_structp, ip: png_infop, freer: c_int, mask: png_uint_32);
    fn png_set_error_fn(pp: png_structp, ep: png_voidp, ef: Option<PngErr>,
                        wf: Option<PngErr>);
    fn png_get_error_ptr(pp: png_structp) -> png_voidp;
    fn png_set_mem_fn(pp: png_structp, mp: png_voidp, mf: Option<PngMalloc>,
                      ff: Option<PngFreeFn>);
    fn png_get_mem_ptr(pp: png_structp) -> png_voidp;
    fn png_malloc(pp: png_structp, size: usize) -> png_voidp;
    fn png_malloc_warn(pp: png_structp, size: usize) -> png_voidp;
    fn png_malloc_default(pp: png_structp, size: usize) -> png_voidp;
    fn png_calloc(pp: png_structp, size: usize) -> png_voidp;
    fn png_free(pp: png_structp, p: png_voidp);
    fn png_free_default(pp: png_structp, p: png_voidp);

    // ---- I/O
    fn png_set_write_fn(pp: png_structp, iop: png_voidp, wf: Option<PngRw>,
                        ff: Option<PngFlush>);
    fn png_set_read_fn(pp: png_structp, iop: png_voidp, rf: Option<PngRw>);
    fn png_get_io_ptr(pp: png_structp) -> png_voidp;
    fn png_get_io_state(pp: png_structp) -> png_uint_32;
    fn png_get_io_chunk_type(pp: png_structp) -> png_uint_32;
    fn png_init_io(pp: png_structp, f: *mut c_void);
    fn png_set_sig_bytes(pp: png_structp, n: c_int);
    fn png_set_flush(pp: png_structp, rows: c_int);
    fn png_write_flush(pp: png_structp);

    // ---- info setters / getters
    fn png_set_IHDR(pp: png_structp, ip: png_infop, w: png_uint_32, h: png_uint_32,
                    bd: c_int, ct: c_int, il: c_int, cm: c_int, ft: c_int);
    fn png_get_IHDR(pp: png_structp, ip: png_infop, w: *mut png_uint_32,
                    h: *mut png_uint_32, bd: *mut c_int, ct: *mut c_int, il: *mut c_int,
                    cm: *mut c_int, ft: *mut c_int) -> png_uint_32;
    fn png_set_PLTE(pp: png_structp, ip: png_infop, pal: *const png_color, num: c_int);
    fn png_get_PLTE(pp: png_structp, ip: png_infop, pal: *mut *mut png_color,
                    num: *mut c_int) -> png_uint_32;
    fn png_set_gAMA_fixed(pp: png_structp, ip: png_infop, g: png_fixed_point);
    fn png_set_gAMA(pp: png_structp, ip: png_infop, g: c_double);
    fn png_get_gAMA_fixed(pp: png_structp, ip: png_infop, g: *mut png_fixed_point)
                         -> png_uint_32;
    fn png_get_gAMA(pp: png_structp, ip: png_infop, g: *mut c_double) -> png_uint_32;
    fn png_set_sRGB(pp: png_structp, ip: png_infop, intent: c_int);
    fn png_set_sRGB_gAMA_and_cHRM(pp: png_structp, ip: png_infop, intent: c_int);
    fn png_get_sRGB(pp: png_structp, ip: png_infop, intent: *mut c_int) -> png_uint_32;
    fn png_set_cHRM_fixed(pp: png_structp, ip: png_infop, wx: png_fixed_point,
                          wy: png_fixed_point, rx: png_fixed_point, ry: png_fixed_point,
                          gx: png_fixed_point, gy: png_fixed_point, bx: png_fixed_point,
                          by: png_fixed_point);
    fn png_set_cHRM(pp: png_structp, ip: png_infop, wx: c_double, wy: c_double,
                    rx: c_double, ry: c_double, gx: c_double, gy: c_double,
                    bx: c_double, by: c_double);
    fn png_get_cHRM_fixed(pp: png_structp, ip: png_infop, wx: *mut png_fixed_point,
                          wy: *mut png_fixed_point, rx: *mut png_fixed_point,
                          ry: *mut png_fixed_point, gx: *mut png_fixed_point,
                          gy: *mut png_fixed_point, bx: *mut png_fixed_point,
                          by: *mut png_fixed_point) -> png_uint_32;
    fn png_set_cHRM_XYZ_fixed(pp: png_structp, ip: png_infop, rX: png_fixed_point,
                              rY: png_fixed_point, rZ: png_fixed_point,
                              gX: png_fixed_point, gY: png_fixed_point,
                              gZ: png_fixed_point, bX: png_fixed_point,
                              bY: png_fixed_point, bZ: png_fixed_point);
    fn png_get_cHRM_XYZ_fixed(pp: png_structp, ip: png_infop, rX: *mut png_fixed_point,
                              rY: *mut png_fixed_point, rZ: *mut png_fixed_point,
                              gX: *mut png_fixed_point, gY: *mut png_fixed_point,
                              gZ: *mut png_fixed_point, bX: *mut png_fixed_point,
                              bY: *mut png_fixed_point, bZ: *mut png_fixed_point)
                             -> png_uint_32;
    fn png_set_sBIT(pp: png_structp, ip: png_infop, sbit: *const png_color_8);
    fn png_get_sBIT(pp: png_structp, ip: png_infop, sbit: *mut *mut png_color_8)
                   -> png_uint_32;
    fn png_set_bKGD(pp: png_structp, ip: png_infop, bg: *const png_color_16);
    fn png_get_bKGD(pp: png_structp, ip: png_infop, bg: *mut *mut png_color_16)
                   -> png_uint_32;
    fn png_set_hIST(pp: png_structp, ip: png_infop, hist: *const png_uint_16);
    fn png_get_hIST(pp: png_structp, ip: png_infop, hist: *mut *mut png_uint_16)
                   -> png_uint_32;
    fn png_set_tRNS(pp: png_structp, ip: png_infop, trans: *const png_byte, num: c_int,
                    tc: *const png_color_16);
    fn png_get_tRNS(pp: png_structp, ip: png_infop, trans: *mut *mut png_byte,
                    num: *mut c_int, tc: *mut *mut png_color_16) -> png_uint_32;
    fn png_set_oFFs(pp: png_structp, ip: png_infop, x: png_int_32, y: png_int_32,
                    unit: c_int);
    fn png_get_oFFs(pp: png_structp, ip: png_infop, x: *mut png_int_32,
                    y: *mut png_int_32, unit: *mut c_int) -> png_uint_32;
    fn png_set_pHYs(pp: png_structp, ip: png_infop, x: png_uint_32, y: png_uint_32,
                    unit: c_int);
    fn png_get_pHYs(pp: png_structp, ip: png_infop, x: *mut png_uint_32,
                    y: *mut png_uint_32, unit: *mut c_int) -> png_uint_32;
    fn png_set_pCAL(pp: png_structp, ip: png_infop, purpose: png_const_charp,
                    x0: png_int_32, x1: png_int_32, typ: c_int, nparams: c_int,
                    units: png_const_charp, params: *mut png_charp);
    fn png_get_pCAL(pp: png_structp, ip: png_infop, purpose: *mut png_charp,
                    x0: *mut png_int_32, x1: *mut png_int_32, typ: *mut c_int,
                    nparams: *mut c_int, units: *mut png_charp,
                    params: *mut *mut png_charp) -> png_uint_32;
    fn png_set_sCAL_fixed(pp: png_structp, ip: png_infop, unit: c_int,
                          w: png_fixed_point, h: png_fixed_point);
    fn png_set_sCAL(pp: png_structp, ip: png_infop, unit: c_int, w: c_double,
                    h: c_double);
    fn png_set_sCAL_s(pp: png_structp, ip: png_infop, unit: c_int, w: png_const_charp,
                      h: png_const_charp);
    fn png_get_sCAL_s(pp: png_structp, ip: png_infop, unit: *mut c_int,
                      w: *mut png_charp, h: *mut png_charp) -> png_uint_32;
    fn png_get_sCAL_fixed(pp: png_structp, ip: png_infop, unit: *mut c_int,
                          w: *mut png_fixed_point, h: *mut png_fixed_point) -> png_uint_32;
    fn png_get_sCAL(pp: png_structp, ip: png_infop, unit: *mut c_int, w: *mut c_double,
                    h: *mut c_double) -> png_uint_32;
    fn png_set_tIME(pp: png_structp, ip: png_infop, t: *const png_time);
    fn png_get_tIME(pp: png_structp, ip: png_infop, t: *mut *mut png_time) -> png_uint_32;
    fn png_set_iCCP(pp: png_structp, ip: png_infop, name: png_const_charp,
                    ctype: c_int, profile: png_const_bytep, proflen: png_uint_32);
    fn png_get_iCCP(pp: png_structp, ip: png_infop, name: *mut png_charp,
                    ctype: *mut c_int, profile: *mut png_bytep,
                    proflen: *mut png_uint_32) -> png_uint_32;
    fn png_set_sPLT(pp: png_structp, ip: png_infop, e: *const png_sPLT_t, num: c_int);
    fn png_get_sPLT(pp: png_structp, ip: png_infop, e: *mut *mut png_sPLT_t) -> c_int;
    fn png_set_text(pp: png_structp, ip: png_infop, t: *const png_text, num: c_int);
    fn png_set_text_2(pp: png_structp, ip: png_infop, t: *const png_text, num: c_int)
                     -> c_int;
    fn png_get_text(pp: png_structp, ip: png_infop, t: *mut *mut png_text,
                    num: *mut c_int) -> c_int;
    fn png_set_eXIf_1(pp: png_structp, ip: png_infop, num: png_uint_32,
                      exif: png_bytep);
    fn png_get_eXIf_1(pp: png_structp, ip: png_infop, num: *mut png_uint_32,
                      exif: *mut png_bytep) -> png_uint_32;
    fn png_set_eXIf(pp: png_structp, ip: png_infop, exif: png_bytep);
    fn png_get_eXIf(pp: png_structp, ip: png_infop, exif: *mut png_bytep) -> png_uint_32;
    fn png_set_cICP(pp: png_structp, ip: png_infop, cp: png_byte, tf: png_byte,
                    mc: png_byte, vfrf: png_byte);
    fn png_get_cICP(pp: png_structp, ip: png_infop, cp: *mut png_byte,
                    tf: *mut png_byte, mc: *mut png_byte, vfrf: *mut png_byte)
                   -> png_uint_32;
    fn png_set_cLLI_fixed(pp: png_structp, ip: png_infop, maxCLL: png_uint_32,
                          maxFALL: png_uint_32);
    fn png_set_cLLI(pp: png_structp, ip: png_infop, maxCLL: c_double, maxFALL: c_double);
    fn png_get_cLLI_fixed(pp: png_structp, ip: png_infop, maxCLL: *mut png_uint_32,
                          maxFALL: *mut png_uint_32) -> png_uint_32;
    fn png_get_cLLI(pp: png_structp, ip: png_infop, maxCLL: *mut c_double,
                    maxFALL: *mut c_double) -> png_uint_32;
    fn png_set_mDCV_fixed(pp: png_structp, ip: png_infop, rx: png_uint_32,
                          ry: png_uint_32, gx: png_uint_32, gy: png_uint_32,
                          bx: png_uint_32, by: png_uint_32, wx: png_uint_32,
                          wy: png_uint_32, maxl: png_uint_32, minl: png_uint_32);
    fn png_get_mDCV_fixed(pp: png_structp, ip: png_infop, rx: *mut png_uint_32,
                          ry: *mut png_uint_32, gx: *mut png_uint_32, gy: *mut png_uint_32,
                          bx: *mut png_uint_32, by: *mut png_uint_32, wx: *mut png_uint_32,
                          wy: *mut png_uint_32, maxl: *mut png_uint_32,
                          minl: *mut png_uint_32) -> png_uint_32;
    fn png_set_rows(pp: png_structp, ip: png_infop, rows: *mut png_bytep);
    fn png_get_rows(pp: png_structp, ip: png_infop) -> *mut png_bytep;
    fn png_set_invalid(pp: png_structp, ip: png_infop, mask: c_int);
    fn png_get_valid(pp: png_structp, ip: png_infop, flag: png_uint_32) -> png_uint_32;
    fn png_get_rowbytes(pp: png_structp, ip: png_infop) -> usize;
    fn png_get_channels(pp: png_structp, ip: png_infop) -> png_byte;
    fn png_get_bit_depth(pp: png_structp, ip: png_infop) -> png_byte;
    fn png_get_color_type(pp: png_structp, ip: png_infop) -> png_byte;
    fn png_get_interlace_type(pp: png_structp, ip: png_infop) -> png_byte;
    fn png_get_compression_type(pp: png_structp, ip: png_infop) -> png_byte;
    fn png_get_filter_type(pp: png_structp, ip: png_infop) -> png_byte;
    fn png_get_image_width(pp: png_structp, ip: png_infop) -> png_uint_32;
    fn png_get_image_height(pp: png_structp, ip: png_infop) -> png_uint_32;
    fn png_get_signature(pp: png_structp, ip: png_infop) -> png_bytep;
    fn png_get_palette_max(pp: png_structp, ip: png_infop) -> c_int;
    fn png_get_x_pixels_per_meter(pp: png_structp, ip: png_infop) -> png_uint_32;
    fn png_get_y_pixels_per_meter(pp: png_structp, ip: png_infop) -> png_uint_32;
    fn png_get_pixels_per_meter(pp: png_structp, ip: png_infop) -> png_uint_32;
    fn png_get_x_pixels_per_inch(pp: png_structp, ip: png_infop) -> png_uint_32;
    fn png_get_y_pixels_per_inch(pp: png_structp, ip: png_infop) -> png_uint_32;
    fn png_get_pixels_per_inch(pp: png_structp, ip: png_infop) -> png_uint_32;
    fn png_get_pixel_aspect_ratio(pp: png_structp, ip: png_infop) -> f32;
    fn png_get_pixel_aspect_ratio_fixed(pp: png_structp, ip: png_infop)
                                       -> png_fixed_point;
    fn png_get_x_offset_pixels(pp: png_structp, ip: png_infop) -> png_int_32;
    fn png_get_y_offset_pixels(pp: png_structp, ip: png_infop) -> png_int_32;
    fn png_get_x_offset_microns(pp: png_structp, ip: png_infop) -> png_int_32;
    fn png_get_y_offset_microns(pp: png_structp, ip: png_infop) -> png_int_32;
    fn png_get_x_offset_inches(pp: png_structp, ip: png_infop) -> f32;
    fn png_get_y_offset_inches(pp: png_structp, ip: png_infop) -> f32;
    fn png_get_x_offset_inches_fixed(pp: png_structp, ip: png_infop) -> png_fixed_point;
    fn png_get_y_offset_inches_fixed(pp: png_structp, ip: png_infop) -> png_fixed_point;
    fn png_get_pHYs_dpi(pp: png_structp, ip: png_infop, x: *mut png_uint_32,
                        y: *mut png_uint_32, unit: *mut c_int) -> png_uint_32;

    // ---- write pipeline
    fn png_write_info(pp: png_structp, ip: png_infop);
    fn png_write_info_before_PLTE(pp: png_structp, ip: png_infop);
    fn png_write_row(pp: png_structp, row: png_const_bytep);
    fn png_write_rows(pp: png_structp, rows: *mut png_bytep, num: png_uint_32);
    fn png_write_image(pp: png_structp, rows: *mut png_bytep);
    fn png_write_end(pp: png_structp, ip: png_infop);
    fn png_write_png(pp: png_structp, ip: png_infop, transforms: c_int, params: png_voidp);
    fn png_set_filter(pp: png_structp, method: c_int, filters: c_int);
    fn png_set_compression_level(pp: png_structp, level: c_int);
    fn png_set_compression_mem_level(pp: png_structp, level: c_int);
    fn png_set_compression_strategy(pp: png_structp, s: c_int);
    fn png_set_compression_window_bits(pp: png_structp, b: c_int);
    fn png_set_compression_method(pp: png_structp, m: c_int);
    fn png_set_compression_buffer_size(pp: png_structp, size: usize);
    fn png_get_compression_buffer_size(pp: png_structp) -> usize;
    fn png_set_text_compression_level(pp: png_structp, level: c_int);
    fn png_set_text_compression_mem_level(pp: png_structp, level: c_int);
    fn png_set_text_compression_strategy(pp: png_structp, s: c_int);
    fn png_set_text_compression_window_bits(pp: png_structp, b: c_int);
    fn png_set_text_compression_method(pp: png_structp, m: c_int);
    fn png_set_write_status_fn(pp: png_structp, cb: Option<PngRowCb>);
    fn png_set_write_user_transform_fn(pp: png_structp, cb: Option<PngUserTransformCb>);
    fn png_set_filter_heuristics(pp: png_structp, h: c_int, n: c_int, w: *mut c_double,
                                 c: *mut c_double);
    fn png_set_filter_heuristics_fixed(pp: png_structp, h: c_int, n: c_int,
                                       w: *mut png_fixed_point, c: *mut png_fixed_point);

    // ---- read pipeline
    fn png_read_info(pp: png_structp, ip: png_infop);
    fn png_read_update_info(pp: png_structp, ip: png_infop);
    fn png_start_read_image(pp: png_structp);
    fn png_read_row(pp: png_structp, row: png_bytep, dsp: png_bytep);
    fn png_read_rows(pp: png_structp, rows: *mut png_bytep, dsp: *mut png_bytep,
                     num: png_uint_32);
    fn png_read_image(pp: png_structp, rows: *mut png_bytep);
    fn png_read_end(pp: png_structp, ip: png_infop);
    fn png_read_png(pp: png_structp, ip: png_infop, transforms: c_int, params: png_voidp);
    fn png_set_read_status_fn(pp: png_structp, cb: Option<PngRowCb>);
    fn png_set_read_user_transform_fn(pp: png_structp, cb: Option<PngUserTransformCb>);
    fn png_set_user_transform_info(pp: png_structp, p: png_voidp, bd: c_int, ch: c_int);
    fn png_get_user_transform_ptr(pp: png_structp) -> png_voidp;
    fn png_set_crc_action(pp: png_structp, crit: c_int, ancil: c_int);
    fn png_set_benign_errors(pp: png_structp, allowed: c_int);
    fn png_set_user_limits(pp: png_structp, w: png_uint_32, h: png_uint_32);
    fn png_get_user_width_max(pp: png_structp) -> png_uint_32;
    fn png_get_user_height_max(pp: png_structp) -> png_uint_32;
    fn png_set_chunk_cache_max(pp: png_structp, n: png_uint_32);
    fn png_get_chunk_cache_max(pp: png_structp) -> png_uint_32;
    fn png_set_chunk_malloc_max(pp: png_structp, n: usize);
    fn png_get_chunk_malloc_max(pp: png_structp) -> usize;
    fn png_set_keep_unknown_chunks(pp: png_structp, keep: c_int, list: png_const_bytep,
                                   num: c_int);
    fn png_handle_as_unknown(pp: png_structp, name: png_const_bytep) -> c_int;
    fn png_set_unknown_chunks(pp: png_structp, ip: png_infop,
                              chunks: *const png_unknown_chunk, num: c_int);
    fn png_set_unknown_chunk_location(pp: png_structp, ip: png_infop, chunk: c_int,
                                      location: c_int);
    fn png_get_unknown_chunks(pp: png_structp, ip: png_infop,
                              chunks: *mut *mut png_unknown_chunk) -> c_int;
    fn png_set_read_user_chunk_fn(pp: png_structp, p: png_voidp,
                                  cb: Option<PngUserChunkCb>);
    fn png_get_user_chunk_ptr(pp: png_structp) -> png_voidp;
    fn png_set_option(pp: png_structp, option: c_int, onoff: c_int) -> c_int;
    fn png_permit_mng_features(pp: png_structp, f: png_uint_32) -> png_uint_32;
    fn png_set_check_for_invalid_index(pp: png_structp, allowed: c_int);
    fn png_get_current_row_number(pp: png_structp) -> png_uint_32;
    fn png_get_current_pass_number(pp: png_structp) -> png_byte;
    fn png_get_rgb_to_gray_status(pp: png_structp) -> png_byte;

    // ---- read transforms
    fn png_set_expand(pp: png_structp);
    fn png_set_expand_16(pp: png_structp);
    fn png_set_expand_gray_1_2_4_to_8(pp: png_structp);
    fn png_set_palette_to_rgb(pp: png_structp);
    fn png_set_tRNS_to_alpha(pp: png_structp);
    fn png_set_gray_to_rgb(pp: png_structp);
    fn png_set_rgb_to_gray_fixed(pp: png_structp, action: c_int, r: png_fixed_point,
                                 g: png_fixed_point);
    fn png_set_rgb_to_gray(pp: png_structp, action: c_int, r: c_double, g: c_double);
    fn png_set_background_fixed(pp: png_structp, bg: *const png_color_16,
                                bg_gamma_code: c_int, need_expand: c_int,
                                bg_gamma: png_fixed_point);
    fn png_set_background(pp: png_structp, bg: *const png_color_16,
                          bg_gamma_code: c_int, need_expand: c_int, bg_gamma: c_double);
    fn png_set_alpha_mode_fixed(pp: png_structp, mode: c_int, g: png_fixed_point);
    fn png_set_alpha_mode(pp: png_structp, mode: c_int, g: c_double);
    fn png_set_gamma_fixed(pp: png_structp, screen: png_fixed_point,
                           file: png_fixed_point);
    fn png_set_gamma(pp: png_structp, screen: c_double, file: c_double);
    fn png_set_quantize(pp: png_structp, pal: *mut png_color, num: c_int,
                        max: c_int, hist: *const png_uint_16, full: c_int);
    fn png_set_strip_16(pp: png_structp);
    fn png_set_scale_16(pp: png_structp);
    fn png_set_strip_alpha(pp: png_structp);
    fn png_set_packing(pp: png_structp);
    fn png_set_packswap(pp: png_structp);
    fn png_set_shift(pp: png_structp, sig: *const png_color_8);
    fn png_set_swap(pp: png_structp);
    fn png_set_swap_alpha(pp: png_structp);
    fn png_set_invert_alpha(pp: png_structp);
    fn png_set_invert_mono(pp: png_structp);
    fn png_set_bgr(pp: png_structp);
    fn png_set_filler(pp: png_structp, filler: png_uint_32, flags: c_int);
    fn png_set_add_alpha(pp: png_structp, filler: png_uint_32, flags: c_int);
    fn png_set_interlace_handling(pp: png_structp) -> c_int;

    // ---- progressive read
    fn png_set_progressive_read_fn(pp: png_structp, p: png_voidp,
                                   info_cb: Option<PngInfoCb>,
                                   row_cb: Option<PngProgRowCb>,
                                   end_cb: Option<PngEndCb>);
    fn png_get_progressive_ptr(pp: png_structp) -> png_voidp;
    fn png_process_data(pp: png_structp, ip: png_infop, buf: png_bytep, size: usize);
    fn png_process_data_pause(pp: png_structp, save: c_int) -> usize;
    fn png_process_data_skip(pp: png_structp) -> png_uint_32;
    fn png_progressive_combine_row(pp: png_structp, old_row: png_bytep,
                                   new_row: png_const_bytep);

    // ---- low-level row operations
    fn png_do_bgr(ri: *mut png_row_info, row: png_bytep);
    fn png_do_invert(ri: *mut png_row_info, row: png_bytep);
    fn png_do_swap(ri: *mut png_row_info, row: png_bytep);
    fn png_do_packswap(ri: *mut png_row_info, row: png_bytep);
    fn png_do_strip_channel(ri: *mut png_row_info, row: png_bytep, at_start: c_int);
    fn png_do_check_palette_indexes(pp: png_structp, ri: *mut png_row_info);
    fn png_do_read_interlace(ri: *mut png_row_info, row: png_bytep, pass: c_int,
                             transformations: png_uint_32);
    fn png_do_write_interlace(ri: *mut png_row_info, row: png_bytep, pass: c_int);
    fn png_read_filter_row(pp: png_structp, ri: *mut png_row_info, row: png_bytep,
                           prev: png_const_bytep, filter: c_int);
    fn png_combine_row(pp: png_structp, dp: png_bytep, display: c_int);
    fn png_build_gamma_table(pp: png_structp, bit_depth: c_int);
    fn png_destroy_gamma_table(pp: png_structp);
    fn png_reset_crc(pp: png_structp);
    fn png_calculate_crc(pp: png_structp, ptr: png_const_bytep, len: usize);
    fn png_reset_zstream(pp: png_structp) -> c_int;
    fn png_zalloc(pp: png_voidp, items: c_uint, size: c_uint) -> png_voidp;
    fn png_zfree(pp: png_voidp, ptr: png_voidp);

    // ---- chunk writers (low level)
    fn png_write_sig(pp: png_structp);
    fn png_write_IHDR(pp: png_structp, w: png_uint_32, h: png_uint_32, bd: c_int,
                      ct: c_int, cm: c_int, ft: c_int, il: c_int);
    fn png_write_PLTE(pp: png_structp, pal: *const png_color, num: png_uint_32);
    fn png_write_IEND(pp: png_structp);
    fn png_write_gAMA_fixed(pp: png_structp, g: png_fixed_point);
    fn png_write_sRGB(pp: png_structp, intent: c_int);
    fn png_write_sBIT(pp: png_structp, sbit: *const png_color_8, ct: c_int);
    fn png_write_cHRM_fixed(pp: png_structp, xy: *const c_void);
    fn png_write_cICP(pp: png_structp, colour_primaries: png_byte, transfer_function: png_byte,
                      matrix_coefficients: png_byte, video_full_range_flag: png_byte);
    fn png_write_cLLI_fixed(pp: png_structp, maxCLL: png_uint_32, maxFALL: png_uint_32);
    fn png_write_mDCV_fixed(pp: png_structp, rx: png_uint_16, ry: png_uint_16,
                            gx: png_uint_16, gy: png_uint_16, bx: png_uint_16,
                            by: png_uint_16, wx: png_uint_16, wy: png_uint_16,
                            maxl: png_uint_32, minl: png_uint_32);
    fn png_write_tRNS(pp: png_structp, trans: png_const_bytep,
                      tc: *const png_color_16, num: c_int, ct: c_int);
    fn png_write_bKGD(pp: png_structp, bg: *const png_color_16, ct: c_int);
    fn png_write_hIST(pp: png_structp, hist: *const png_uint_16, num: c_int);
    fn png_write_oFFs(pp: png_structp, x: png_int_32, y: png_int_32, unit: c_int);
    fn png_write_pHYs(pp: png_structp, x: png_uint_32, y: png_uint_32, unit: c_int);
    fn png_write_tIME(pp: png_structp, t: *const png_time);
    fn png_write_eXIf(pp: png_structp, exif: png_bytep, num: png_int_32);
    fn png_write_tEXt(pp: png_structp, key: png_const_charp, text: png_const_charp,
                      len: usize);
    fn png_write_zTXt(pp: png_structp, key: png_const_charp, text: png_const_charp,
                      compression: c_int);
    fn png_write_iTXt(pp: png_structp, compression: c_int, key: png_const_charp,
                      lang: png_const_charp, lang_key: png_const_charp,
                      text: png_const_charp);
    fn png_write_sCAL_s(pp: png_structp, unit: c_int, w: png_const_charp,
                        h: png_const_charp);
    fn png_write_pCAL(pp: png_structp, purpose: png_charp, x0: png_int_32,
                      x1: png_int_32, typ: c_int, nparams: c_int, units: png_const_charp,
                      params: *mut png_charp);
    fn png_write_iCCP(pp: png_structp, name: png_const_charp, profile: png_const_bytep,
                      proflen: png_uint_32);
    fn png_write_sPLT(pp: png_structp, p: *const png_sPLT_t);
    fn png_write_chunk(pp: png_structp, name: png_const_bytep, data: png_const_bytep,
                       len: usize);
    fn png_write_chunk_start(pp: png_structp, name: png_const_bytep, len: png_uint_32);
    fn png_write_chunk_data(pp: png_structp, data: png_const_bytep, len: usize);
    fn png_write_chunk_end(pp: png_structp);

    // ---- simplified API
    fn png_image_begin_read_from_memory(img: *mut png_image, mem: *const c_void,
                                        size: usize) -> c_int;
    fn png_image_begin_read_from_file(img: *mut png_image, name: png_const_charp)
                                     -> c_int;
    fn png_image_begin_read_from_stdio(img: *mut png_image, f: *mut c_void) -> c_int;
    fn png_image_finish_read(img: *mut png_image, bg: *const png_color,
                             buffer: *mut c_void, row_stride: png_int_32,
                             colormap: *mut c_void) -> c_int;
    fn png_image_free(img: *mut png_image);
    fn png_image_write_to_memory(img: *mut png_image, mem: *mut c_void,
                                 size: *mut usize, convert8: c_int,
                                 buffer: *const c_void, row_stride: png_int_32,
                                 colormap: *const c_void) -> c_int;
    fn png_image_write_to_file(img: *mut png_image, name: png_const_charp,
                               convert8: c_int, buffer: *const c_void,
                               row_stride: png_int_32, colormap: *const c_void)
                              -> c_int;
    fn png_image_write_to_stdio(img: *mut png_image, f: *mut c_void, convert8: c_int,
                                buffer: *const c_void, row_stride: png_int_32,
                                colormap: *const c_void) -> c_int;

    // ---- error entry points
    fn png_error(pp: png_structp, msg: png_const_charp);
    fn png_warning(pp: png_structp, msg: png_const_charp);
    fn png_benign_error(pp: png_structp, msg: png_const_charp);
    fn png_app_error(pp: png_structp, msg: png_const_charp);
    fn png_app_warning(pp: png_structp, msg: png_const_charp);
    fn png_chunk_error(pp: png_structp, msg: png_const_charp);
    fn png_chunk_warning(pp: png_structp, msg: png_const_charp);
    fn png_chunk_benign_error(pp: png_structp, msg: png_const_charp);
    fn png_chunk_report(pp: png_structp, msg: png_const_charp, error: c_int);
    fn png_fixed_error(pp: png_structp, msg: png_const_charp);
    fn png_set_longjmp_fn(pp: png_structp, f: png_voidp, size: usize) -> png_voidp;
    fn png_free_jmpbuf(pp: png_structp);
    // Prototypes verified against c_src/include/pngpriv.h:1886-1900 (there is
    // no `png_colorspace` parameter in libpng 1.6.59).
    fn png_icc_check_header(pp: png_structp, name: png_const_charp,
                            proflen: png_uint_32, profile: png_const_bytep,
                            ct: c_int) -> c_int;
    fn png_icc_check_length(pp: png_structp, name: png_const_charp,
                            proflen: png_uint_32) -> c_int;
    fn png_icc_check_tag_table(pp: png_structp, name: png_const_charp,
                               proflen: png_uint_32, profile: png_const_bytep)
                              -> c_int;
    fn png_resolve_file_gamma(pp: png_structp) -> png_fixed_point;
    fn png_chunk_unknown_handling(pp: png_structp, chunk_name: png_uint_32) -> c_int;
    fn png_user_version_check(pp: png_structp, ver: png_const_charp) -> c_int;
}

// ------------------------------------------------------------ convenience

/// Create a write struct with the recording error/warning handlers and the
/// harness write sink installed.
pub unsafe fn new_writer(l: &::libloading::Library) -> png_structp {
    let pp = png_create_write_struct(
        l,
        PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
        1usize as png_voidp,
        Some(rec_error),
        Some(rec_warning),
    );
    assert!(!pp.is_null());
    png_set_write_fn(l, pp, 1usize as png_voidp, Some(write_cb), Some(flush_cb));
    pp
}

/// Create a read struct with the recording handlers and the harness read
/// source installed (call [`src_set`] first).
pub unsafe fn new_reader(l: &::libloading::Library) -> png_structp {
    let pp = png_create_read_struct(
        l,
        PNG_LIBPNG_VER_STRING.as_ptr() as png_const_charp,
        1usize as png_voidp,
        Some(rec_error),
        Some(rec_warning),
    );
    assert!(!pp.is_null());
    png_set_read_fn(l, pp, 1usize as png_voidp, Some(read_cb));
    pp
}
