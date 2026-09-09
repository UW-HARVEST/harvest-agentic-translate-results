//! The libpng public API, loaded from a shared object at run time.
//!
//! Both the C reference `.so` and the Rust `.so` are loaded exclusively through
//! this table, so every call in every test crosses the real `#[no_mangle]`
//! `extern "C"` boundary exactly as an external consumer's would.
#![allow(non_snake_case, dead_code)]

use crate::types::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

macro_rules! api {
    ( $( fn $name:ident ( $($at:ty),* $(,)? ) $(-> $ret:ty)? ; )* ) => {
        pub struct Api {
            #[allow(dead_code)]
            lib: libloading::Library,
            $( pub $name: unsafe extern "C" fn($($at),*) $(-> $ret)?, )*
        }

        impl Api {
            /// Load every symbol by name from `path`. A missing symbol is a hard
            /// failure: it means the `.so` does not export the C API.
            pub fn load(path: &str) -> Result<Api, String> {
                unsafe {
                    let lib = libloading::Library::new(path)
                        .map_err(|e| format!("dlopen {}: {}", path, e))?;
                    $(
                        let $name = {
                            type F = unsafe extern "C" fn($($at),*) $(-> $ret)?;
                            let sym: libloading::Symbol<F> = lib
                                .get(concat!(stringify!($name), "\0").as_bytes())
                                .map_err(|e| format!(
                                    "{} missing {}: {}", path, stringify!($name), e))?;
                            *sym
                        };
                    )*
                    Ok(Api { $( $name, )* lib })
                }
            }
        }
    }
}

api! {
    /* ---- version / signature / primitives ---- */
    fn png_access_version_number() -> u32;
    fn png_get_copyright(*const c_void) -> *const c_char;
    fn png_get_header_ver(*const c_void) -> *const c_char;
    fn png_get_header_version(*const c_void) -> *const c_char;
    fn png_get_libpng_ver(*const c_void) -> *const c_char;
    fn png_sig_cmp(*const u8, usize, usize) -> c_int;
    fn png_set_sig_bytes(png_structp, c_int);
    fn png_get_uint_32(*const u8) -> u32;
    fn png_get_uint_16(*const u8) -> u16;
    fn png_get_int_32(*const u8) -> i32;
    fn png_get_uint_31(png_structp, *const u8) -> u32;
    fn png_save_uint_32(*mut u8, u32);
    fn png_save_uint_16(*mut u8, c_int);
    fn png_save_int_32(*mut u8, i32);
    fn png_build_grayscale_palette(c_int, *mut png_color);

    /* ---- struct lifecycle ---- */
    fn png_create_read_struct(*const c_char, *mut c_void, png_error_ptr, png_error_ptr) -> png_structp;
    fn png_create_write_struct(*const c_char, *mut c_void, png_error_ptr, png_error_ptr) -> png_structp;
    fn png_create_read_struct_2(*const c_char, *mut c_void, png_error_ptr, png_error_ptr, *mut c_void, png_malloc_ptr, png_free_ptr) -> png_structp;
    fn png_create_write_struct_2(*const c_char, *mut c_void, png_error_ptr, png_error_ptr, *mut c_void, png_malloc_ptr, png_free_ptr) -> png_structp;
    fn png_create_info_struct(png_structp) -> png_infop;
    fn png_destroy_info_struct(png_structp, *mut png_infop);
    fn png_destroy_read_struct(*mut png_structp, *mut png_infop, *mut png_infop);
    fn png_destroy_write_struct(*mut png_structp, *mut png_infop);
    fn png_info_init_3(*mut png_infop, usize);
    fn png_free_data(png_structp, png_infop, u32, c_int);
    fn png_data_freer(png_structp, png_infop, c_int, u32);
    fn png_set_invalid(png_structp, png_infop, c_int);
    fn png_malloc(png_structp, u64) -> *mut c_void;
    fn png_calloc(png_structp, u64) -> *mut c_void;
    fn png_malloc_warn(png_structp, u64) -> *mut c_void;
    fn png_free(png_structp, *mut c_void);
    fn png_get_mem_ptr(png_structp) -> *mut c_void;
    fn png_set_mem_fn(png_structp, *mut c_void, png_malloc_ptr, png_free_ptr);

    /* ---- error handling ---- */
    fn png_set_error_fn(png_structp, *mut c_void, png_error_ptr, png_error_ptr);
    fn png_get_error_ptr(png_structp) -> *mut c_void;
    fn png_set_benign_errors(png_structp, c_int);
    fn png_set_longjmp_fn(png_structp, *mut c_void, usize) -> *mut c_void;
    fn png_error(png_structp, *const c_char);
    fn png_warning(png_structp, *const c_char);
    fn png_benign_error(png_structp, *const c_char);
    fn png_chunk_error(png_structp, *const c_char);
    fn png_chunk_warning(png_structp, *const c_char);
    fn png_chunk_benign_error(png_structp, *const c_char);

    /* ---- io ---- */
    fn png_set_read_fn(png_structp, *mut c_void, png_rw_ptr);
    fn png_set_write_fn(png_structp, *mut c_void, png_rw_ptr, png_flush_ptr);
    fn png_get_io_ptr(png_structp) -> *mut c_void;
    fn png_get_io_state(png_structp) -> u32;
    fn png_get_io_chunk_type(png_structp) -> u32;
    fn png_init_io(png_structp, *mut c_void);
    fn png_set_read_status_fn(png_structp, png_read_status_ptr);
    fn png_set_write_status_fn(png_structp, png_write_status_ptr);
    fn png_set_flush(png_structp, c_int);
    fn png_write_flush(png_structp);

    /* ---- read pipeline ---- */
    fn png_read_info(png_structp, png_infop);
    fn png_read_update_info(png_structp, png_infop);
    fn png_start_read_image(png_structp);
    fn png_read_row(png_structp, *mut u8, *mut u8);
    fn png_read_rows(png_structp, *mut *mut u8, *mut *mut u8, u32);
    fn png_read_image(png_structp, *mut *mut u8);
    fn png_read_end(png_structp, png_infop);
    fn png_read_png(png_structp, png_infop, c_int, *mut c_void);
    fn png_reset_zstream(png_structp) -> c_int;

    /* ---- progressive read ---- */
    fn png_set_progressive_read_fn(png_structp, *mut c_void, png_progressive_info_ptr, png_progressive_row_ptr, png_progressive_end_ptr);
    fn png_get_progressive_ptr(png_structp) -> *mut c_void;
    fn png_process_data(png_structp, png_infop, *mut u8, usize);
    fn png_process_data_pause(png_structp, c_int) -> usize;
    fn png_process_data_skip(png_structp) -> u32;
    fn png_progressive_combine_row(png_structp, *mut u8, *const u8);

    /* ---- write pipeline ---- */
    fn png_write_sig(png_structp);
    fn png_write_info(png_structp, png_infop);
    fn png_write_info_before_PLTE(png_structp, png_infop);
    fn png_write_row(png_structp, *const u8);
    fn png_write_rows(png_structp, *mut *mut u8, u32);
    fn png_write_image(png_structp, *mut *mut u8);
    fn png_write_end(png_structp, png_infop);
    fn png_write_png(png_structp, png_infop, c_int, *mut c_void);
    fn png_write_chunk(png_structp, *const u8, *const u8, usize);
    fn png_write_chunk_start(png_structp, *const u8, u32);
    fn png_write_chunk_data(png_structp, *const u8, usize);
    fn png_write_chunk_end(png_structp);

    /* ---- compression / filter options ---- */
    fn png_set_filter(png_structp, c_int, c_int);
    fn png_set_compression_level(png_structp, c_int);
    fn png_set_compression_mem_level(png_structp, c_int);
    fn png_set_compression_strategy(png_structp, c_int);
    fn png_set_compression_window_bits(png_structp, c_int);
    fn png_set_compression_method(png_structp, c_int);
    fn png_set_compression_buffer_size(png_structp, usize);
    fn png_get_compression_buffer_size(png_structp) -> usize;
    fn png_set_text_compression_level(png_structp, c_int);
    fn png_set_text_compression_mem_level(png_structp, c_int);
    fn png_set_text_compression_strategy(png_structp, c_int);
    fn png_set_text_compression_window_bits(png_structp, c_int);
    fn png_set_text_compression_method(png_structp, c_int);

    /* ---- read transforms ---- */
    fn png_set_expand(png_structp);
    fn png_set_expand_gray_1_2_4_to_8(png_structp);
    fn png_set_expand_16(png_structp);
    fn png_set_palette_to_rgb(png_structp);
    fn png_set_tRNS_to_alpha(png_structp);
    fn png_set_gray_to_rgb(png_structp);
    fn png_set_rgb_to_gray(png_structp, c_int, f64, f64);
    fn png_set_rgb_to_gray_fixed(png_structp, c_int, i32, i32);
    fn png_get_rgb_to_gray_status(png_structp) -> u8;
    fn png_set_strip_16(png_structp);
    fn png_set_scale_16(png_structp);
    fn png_set_strip_alpha(png_structp);
    fn png_set_swap_alpha(png_structp);
    fn png_set_invert_alpha(png_structp);
    fn png_set_filler(png_structp, u32, c_int);
    fn png_set_add_alpha(png_structp, u32, c_int);
    fn png_set_swap(png_structp);
    fn png_set_packing(png_structp);
    fn png_set_packswap(png_structp);
    fn png_set_shift(png_structp, *const png_color_8);
    fn png_set_interlace_handling(png_structp) -> c_int;
    fn png_set_invert_mono(png_structp);
    fn png_set_bgr(png_structp);
    fn png_set_background(png_structp, *const png_color_16, c_int, c_int, f64);
    fn png_set_background_fixed(png_structp, *const png_color_16, c_int, c_int, i32);
    fn png_set_alpha_mode(png_structp, c_int, f64);
    fn png_set_alpha_mode_fixed(png_structp, c_int, i32);
    fn png_set_gamma(png_structp, f64, f64);
    fn png_set_gamma_fixed(png_structp, i32, i32);
    fn png_set_quantize(png_structp, *mut png_color, c_int, c_int, *const u16, c_int);
    fn png_set_crc_action(png_structp, c_int, c_int);
    fn png_set_check_for_invalid_index(png_structp, c_int);
    fn png_get_palette_max(png_structp, png_infop) -> c_int;
    fn png_set_read_user_transform_fn(png_structp, png_user_transform_ptr);
    fn png_set_write_user_transform_fn(png_structp, png_user_transform_ptr);
    fn png_set_user_transform_info(png_structp, *mut c_void, c_int, c_int);
    fn png_get_user_transform_ptr(png_structp) -> *mut c_void;
    fn png_get_current_row_number(png_structp) -> u32;
    fn png_get_current_pass_number(png_structp) -> u8;

    /* ---- limits / options ---- */
    fn png_set_user_limits(png_structp, u32, u32);
    fn png_get_user_width_max(png_structp) -> u32;
    fn png_get_user_height_max(png_structp) -> u32;
    fn png_set_chunk_cache_max(png_structp, u32);
    fn png_get_chunk_cache_max(png_structp) -> u32;
    fn png_set_chunk_malloc_max(png_structp, u64);
    fn png_get_chunk_malloc_max(png_structp) -> u64;
    fn png_set_option(png_structp, c_int, c_int) -> c_int;
    fn png_permit_mng_features(png_structp, u32) -> u32;

    /* ---- unknown chunks ---- */
    fn png_set_keep_unknown_chunks(png_structp, c_int, *const u8, c_int);
    fn png_handle_as_unknown(png_structp, *const u8) -> c_int;
    fn png_set_unknown_chunks(png_structp, png_infop, *const png_unknown_chunk, c_int);
    fn png_set_unknown_chunk_location(png_structp, png_infop, c_int, c_int);
    fn png_get_unknown_chunks(png_structp, png_infop, *mut *mut png_unknown_chunk) -> c_int;
    fn png_set_read_user_chunk_fn(png_structp, *mut c_void, png_user_chunk_ptr);
    fn png_get_user_chunk_ptr(png_structp) -> *mut c_void;

    /* ---- info accessors ---- */
    fn png_get_IHDR(png_structp, png_infop, *mut u32, *mut u32, *mut c_int, *mut c_int, *mut c_int, *mut c_int, *mut c_int) -> u32;
    fn png_set_IHDR(png_structp, png_infop, u32, u32, c_int, c_int, c_int, c_int, c_int);
    fn png_get_valid(png_structp, png_infop, u32) -> u32;
    fn png_get_rowbytes(png_structp, png_infop) -> usize;
    fn png_get_channels(png_structp, png_infop) -> u8;
    fn png_get_image_width(png_structp, png_infop) -> u32;
    fn png_get_image_height(png_structp, png_infop) -> u32;
    fn png_get_bit_depth(png_structp, png_infop) -> u8;
    fn png_get_color_type(png_structp, png_infop) -> u8;
    fn png_get_filter_type(png_structp, png_infop) -> u8;
    fn png_get_interlace_type(png_structp, png_infop) -> u8;
    fn png_get_compression_type(png_structp, png_infop) -> u8;
    fn png_get_signature(png_structp, png_infop) -> *const u8;
    fn png_get_rows(png_structp, png_infop) -> *mut *mut u8;
    fn png_set_rows(png_structp, png_infop, *mut *mut u8);

    fn png_get_PLTE(png_structp, png_infop, *mut *mut png_color, *mut c_int) -> u32;
    fn png_set_PLTE(png_structp, png_infop, *const png_color, c_int);
    fn png_get_tRNS(png_structp, png_infop, *mut *mut u8, *mut c_int, *mut *mut png_color_16) -> u32;
    fn png_set_tRNS(png_structp, png_infop, *const u8, c_int, *const png_color_16);
    fn png_get_bKGD(png_structp, png_infop, *mut *mut png_color_16) -> u32;
    fn png_set_bKGD(png_structp, png_infop, *const png_color_16);
    fn png_get_gAMA(png_structp, png_infop, *mut f64) -> u32;
    fn png_get_gAMA_fixed(png_structp, png_infop, *mut i32) -> u32;
    fn png_set_gAMA(png_structp, png_infop, f64);
    fn png_set_gAMA_fixed(png_structp, png_infop, i32);
    fn png_get_sBIT(png_structp, png_infop, *mut *mut png_color_8) -> u32;
    fn png_set_sBIT(png_structp, png_infop, *const png_color_8);
    fn png_get_sRGB(png_structp, png_infop, *mut c_int) -> u32;
    fn png_set_sRGB(png_structp, png_infop, c_int);
    fn png_set_sRGB_gAMA_and_cHRM(png_structp, png_infop, c_int);
    fn png_get_iCCP(png_structp, png_infop, *mut *mut c_char, *mut c_int, *mut *mut u8, *mut u32) -> u32;
    fn png_set_iCCP(png_structp, png_infop, *const c_char, c_int, *const u8, u32);
    fn png_get_hIST(png_structp, png_infop, *mut *mut u16) -> u32;
    fn png_set_hIST(png_structp, png_infop, *const u16);
    fn png_get_pHYs(png_structp, png_infop, *mut u32, *mut u32, *mut c_int) -> u32;
    fn png_set_pHYs(png_structp, png_infop, u32, u32, c_int);
    fn png_get_pHYs_dpi(png_structp, png_infop, *mut u32, *mut u32, *mut c_int) -> u32;
    fn png_get_oFFs(png_structp, png_infop, *mut i32, *mut i32, *mut c_int) -> u32;
    fn png_set_oFFs(png_structp, png_infop, i32, i32, c_int);
    fn png_get_pCAL(png_structp, png_infop, *mut *mut c_char, *mut i32, *mut i32, *mut c_int, *mut c_int, *mut *mut c_char, *mut *mut *mut c_char) -> u32;
    fn png_set_pCAL(png_structp, png_infop, *const c_char, i32, i32, c_int, c_int, *const c_char, *mut *mut c_char);
    fn png_get_sCAL(png_structp, png_infop, *mut c_int, *mut f64, *mut f64) -> u32;
    fn png_get_sCAL_fixed(png_structp, png_infop, *mut c_int, *mut i32, *mut i32) -> u32;
    fn png_get_sCAL_s(png_structp, png_infop, *mut c_int, *mut *mut c_char, *mut *mut c_char) -> u32;
    fn png_set_sCAL(png_structp, png_infop, c_int, f64, f64);
    fn png_set_sCAL_fixed(png_structp, png_infop, c_int, i32, i32);
    fn png_set_sCAL_s(png_structp, png_infop, c_int, *const c_char, *const c_char);
    fn png_get_tIME(png_structp, png_infop, *mut *mut png_time) -> u32;
    fn png_set_tIME(png_structp, png_infop, *const png_time);
    fn png_get_text(png_structp, png_infop, *mut *mut png_text, *mut c_int) -> c_int;
    fn png_set_text(png_structp, png_infop, *const png_text, c_int);
    fn png_get_sPLT(png_structp, png_infop, *mut *mut png_sPLT_t) -> c_int;
    fn png_set_sPLT(png_structp, png_infop, *const png_sPLT_t, c_int);
    fn png_get_eXIf(png_structp, png_infop, *mut *mut u8) -> u32;
    fn png_set_eXIf(png_structp, png_infop, *mut u8);
    fn png_get_eXIf_1(png_structp, png_infop, *mut u32, *mut *mut u8) -> u32;
    fn png_set_eXIf_1(png_structp, png_infop, u32, *mut u8);
    fn png_get_cHRM(png_structp, png_infop, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64) -> u32;
    fn png_get_cHRM_fixed(png_structp, png_infop, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32) -> u32;
    fn png_get_cHRM_XYZ(png_structp, png_infop, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64) -> u32;
    fn png_get_cHRM_XYZ_fixed(png_structp, png_infop, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32) -> u32;
    fn png_set_cHRM(png_structp, png_infop, f64, f64, f64, f64, f64, f64, f64, f64);
    fn png_set_cHRM_fixed(png_structp, png_infop, i32, i32, i32, i32, i32, i32, i32, i32);
    fn png_set_cHRM_XYZ(png_structp, png_infop, f64, f64, f64, f64, f64, f64, f64, f64, f64);
    fn png_set_cHRM_XYZ_fixed(png_structp, png_infop, i32, i32, i32, i32, i32, i32, i32, i32, i32);
    fn png_get_cICP(png_structp, png_infop, *mut u8, *mut u8, *mut u8, *mut u8) -> u32;
    fn png_set_cICP(png_structp, png_infop, u8, u8, u8, u8);
    fn png_get_cLLI(png_structp, png_infop, *mut f64, *mut f64) -> u32;
    fn png_get_cLLI_fixed(png_structp, png_infop, *mut u32, *mut u32) -> u32;
    fn png_set_cLLI(png_structp, png_infop, f64, f64);
    fn png_set_cLLI_fixed(png_structp, png_infop, u32, u32);
    fn png_get_mDCV(png_structp, png_infop, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64, *mut f64) -> u32;
    fn png_get_mDCV_fixed(png_structp, png_infop, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut u32, *mut u32) -> u32;
    fn png_set_mDCV(png_structp, png_infop, f64, f64, f64, f64, f64, f64, f64, f64, f64, f64);
    fn png_set_mDCV_fixed(png_structp, png_infop, i32, i32, i32, i32, i32, i32, i32, i32, u32, u32);

    /* ---- easy access / conversions ---- */
    fn png_get_pixels_per_meter(png_structp, png_infop) -> u32;
    fn png_get_x_pixels_per_meter(png_structp, png_infop) -> u32;
    fn png_get_y_pixels_per_meter(png_structp, png_infop) -> u32;
    fn png_get_pixels_per_inch(png_structp, png_infop) -> u32;
    fn png_get_x_pixels_per_inch(png_structp, png_infop) -> u32;
    fn png_get_y_pixels_per_inch(png_structp, png_infop) -> u32;
    fn png_get_pixel_aspect_ratio(png_structp, png_infop) -> f32;
    fn png_get_pixel_aspect_ratio_fixed(png_structp, png_infop) -> i32;
    fn png_get_x_offset_pixels(png_structp, png_infop) -> i32;
    fn png_get_y_offset_pixels(png_structp, png_infop) -> i32;
    fn png_get_x_offset_microns(png_structp, png_infop) -> i32;
    fn png_get_y_offset_microns(png_structp, png_infop) -> i32;
    fn png_get_x_offset_inches(png_structp, png_infop) -> f32;
    fn png_get_y_offset_inches(png_structp, png_infop) -> f32;
    fn png_get_x_offset_inches_fixed(png_structp, png_infop) -> i32;
    fn png_get_y_offset_inches_fixed(png_structp, png_infop) -> i32;
    fn png_convert_to_rfc1123_buffer(*mut c_char, *const png_time) -> c_int;
    fn png_convert_to_rfc1123(png_structp, *const png_time) -> *const c_char;
    fn png_convert_from_struct_tm(*mut png_time, *const c_void);
    fn png_convert_from_time_t(*mut png_time, i64);

    /* ---- simplified API ---- */
    fn png_image_begin_read_from_memory(*mut png_image, *const c_void, usize) -> c_int;
    fn png_image_begin_read_from_file(*mut png_image, *const c_char) -> c_int;
    fn png_image_begin_read_from_stdio(*mut png_image, *mut c_void) -> c_int;
    fn png_image_finish_read(*mut png_image, *const png_color, *mut c_void, i32, *mut c_void) -> c_int;
    fn png_image_free(*mut png_image);
    fn png_image_write_to_memory(*mut png_image, *mut c_void, *mut u64, c_int, *const c_void, i32, *const c_void) -> c_int;
    fn png_image_write_to_file(*mut png_image, *const c_char, c_int, *const c_void, i32, *const c_void) -> c_int;
    fn png_image_write_to_stdio(*mut png_image, *mut c_void, c_int, *const c_void, i32, *const c_void) -> c_int;

    /* ------------------------------------------------------------------ *
     * Internal (`PNG_INTERNAL_FUNCTION` / `PNG_INTERNAL_DATA`) symbols.
     *
     * This build has no symbol-visibility script, so libpng's own private
     * helpers are exported from the `.so` too and are therefore part of the
     * ABI the Rust `.so` must reproduce. They are the *lowest-level* entry
     * points in the library, so they are driven directly rather than only
     * through the convenience wrappers. Signatures come from
     * `c_src/include/pngpriv.h`.
     * ------------------------------------------------------------------ */

    /* fixed-point and gamma arithmetic */
    fn png_muldiv(*mut i32, i32, i32, i32) -> c_int;
    fn png_reciprocal(i32) -> i32;
    fn png_reciprocal2(i32, i32) -> i32;
    fn png_fixed(png_structp, f64, *const c_char) -> i32;
    fn png_fixed_ITU(png_structp, f64, *const c_char) -> u32;
    fn png_gamma_significant(i32) -> c_int;
    fn png_gamma_8bit_correct(c_uint, i32) -> u8;
    fn png_gamma_16bit_correct(c_uint, i32) -> u16;
    fn png_gamma_correct(png_structp, c_uint, i32) -> u16;

    /* colour-space conversion */
    fn png_XYZ_from_xy(*mut png_XYZ, *const png_xy) -> c_int;
    fn png_xy_from_XYZ(*mut png_xy, *const png_XYZ) -> c_int;

    /* number / string formatting */
    fn png_check_fp_number(*const c_char, usize, *mut c_int, *mut usize) -> c_int;
    fn png_check_fp_string(*const c_char, usize) -> c_int;
    fn png_ascii_from_fp(png_structp, *mut c_char, usize, f64, c_uint);
    fn png_ascii_from_fixed(png_structp, *mut c_char, usize, i32);
    fn png_safecat(*mut c_char, usize, usize, *const c_char) -> usize;
    fn png_format_number(*const c_char, *mut c_char, c_int, u64) -> *mut c_char;
    fn png_check_keyword(png_structp, *const c_char, *mut u8) -> u32;

    /* row transforms operating on a caller-supplied png_row_info + buffer */
    fn png_do_bgr(*mut png_row_info, *mut u8);
    fn png_do_invert(*mut png_row_info, *mut u8);
    fn png_do_swap(*mut png_row_info, *mut u8);
    fn png_do_packswap(*mut png_row_info, *mut u8);
    fn png_do_strip_channel(*mut png_row_info, *mut u8, c_int);
    fn png_do_read_interlace(*mut png_row_info, *mut u8, c_int, u32);
    fn png_do_write_interlace(*mut png_row_info, *mut u8, c_int);

    /* misc internal entry points */
    fn png_check_IHDR(png_structp, u32, u32, c_int, c_int, c_int, c_int, c_int);
    fn png_chunk_unknown_handling(png_structp, u32) -> c_int;
    fn png_user_version_check(png_structp, *const c_char) -> c_int;
    fn png_zalloc(*mut c_void, c_uint, c_uint) -> *mut c_void;
    fn png_zfree(*mut c_void, *mut c_void);
    fn png_malloc_base(png_structp, u64) -> *mut c_void;
    fn png_malloc_default(png_structp, u64) -> *mut c_void;
    fn png_free_default(png_structp, *mut c_void);
    fn png_set_text_2(png_structp, png_infop, *const png_text, c_int) -> c_int;
    fn png_app_error(png_structp, *const c_char);
    fn png_app_warning(png_structp, *const c_char);
    fn png_chunk_report(png_structp, *const c_char, c_int);
    fn png_fixed_error(png_structp, *const c_char);
    fn png_warning_parameter(*mut c_char, c_int, *const c_char);
    fn png_warning_parameter_unsigned(*mut c_char, c_int, c_int, u64);
    fn png_warning_parameter_signed(*mut c_char, c_int, c_int, i32);
    fn png_formatted_warning(png_structp, *mut c_char, *const c_char);
    fn png_set_filter_heuristics(png_structp, c_int, c_int, *const f64, *const f64);
    fn png_set_filter_heuristics_fixed(png_structp, c_int, c_int, *const i32, *const i32);
    fn png_create_png_struct(*const c_char, *mut c_void, png_error_ptr, png_error_ptr, *mut c_void, png_malloc_ptr, png_free_ptr) -> png_structp;
    fn png_destroy_png_struct(png_structp);

    /* internal write-side chunk emitters, driven on a fresh write struct */
    fn png_write_IHDR(png_structp, u32, u32, c_int, c_int, c_int, c_int, c_int);
    fn png_write_PLTE(png_structp, *const png_color, u32);
    fn png_write_IEND(png_structp);
    fn png_write_gAMA_fixed(png_structp, i32);
    fn png_write_cHRM_fixed(png_structp, *const png_xy);
    fn png_write_sRGB(png_structp, c_int);
    fn png_write_sBIT(png_structp, *const png_color_8, c_int);
    fn png_write_tRNS(png_structp, *const u8, *const png_color_16, c_int, c_int);
    fn png_write_bKGD(png_structp, *const png_color_16, c_int);
    fn png_write_hIST(png_structp, *const u16, c_int);
    fn png_write_pHYs(png_structp, u32, u32, c_int);
    fn png_write_oFFs(png_structp, i32, i32, c_int);
    fn png_write_tIME(png_structp, *const png_time);
    fn png_write_tEXt(png_structp, *const c_char, *const c_char, usize);
    fn png_write_zTXt(png_structp, *const c_char, *const c_char, c_int);
    fn png_write_iTXt(png_structp, c_int, *const c_char, *const c_char, *const c_char, *const c_char);
    fn png_write_sPLT(png_structp, *const png_sPLT_t);
    fn png_write_pCAL(png_structp, *mut c_char, i32, i32, c_int, c_int, *const c_char, *mut *mut c_char);
    fn png_write_sCAL_s(png_structp, c_int, *const c_char, *const c_char);
    fn png_write_iCCP(png_structp, *const c_char, *const u8, u32);
    fn png_write_eXIf(png_structp, *mut u8, c_int);
    fn png_write_cICP(png_structp, u8, u8, u8, u8);
    fn png_write_cLLI_fixed(png_structp, u32, u32);
    fn png_write_mDCV_fixed(png_structp, u16, u16, u16, u16, u16, u16, u16, u16, u32, u32);

    /* internal data tables */
    fn png_reset_crc(png_structp);
    fn png_calculate_crc(png_structp, *const u8, usize);

    /* Core filter reconstruction. Safe to call directly on a fresh read struct:
     * `png_read_filter_row` lazily initialises `pp->read_filter[]` itself
     * (pngrutil.c:4163 `if (pp->read_filter[0] == NULL) png_init_filter_functions(pp)`). */
    fn png_read_filter_row(png_structp, *mut png_row_info, *mut u8, *const u8, c_int);

    /* io plumbing */
    fn png_read_data(png_structp, *mut u8, usize);
    fn png_write_data(png_structp, *const u8, usize);
    fn png_flush(png_structp);
    fn png_default_read_data(png_structp, *mut u8, usize);
    fn png_default_write_data(png_structp, *mut u8, usize);
    fn png_default_flush(png_structp);

    /* array allocation helpers */
    fn png_malloc_array(png_structp, c_int, usize) -> *mut c_void;
    fn png_realloc_array(png_structp, *const c_void, c_int, c_int, usize) -> *mut c_void;
}

/// `PNG_INTERNAL_DATA` symbols, which are objects rather than functions.
pub struct DataTables {
    pub png_sRGB_table: *const u16,
    pub png_sRGB_base: *const u16,
    pub png_sRGB_delta: *const u8,
}

impl Api {
    /// Resolve the exported data tables. These are `const` arrays in the C, so
    /// a translation error in any entry is a real ABI/behaviour difference and
    /// they are compared element by element.
    pub fn data_tables(&self) -> Result<DataTables, String> {
        unsafe {
            let g = |n: &[u8]| -> Result<*const u8, String> {
                let s: libloading::Symbol<*const u8> = self
                    .lib
                    .get(n)
                    .map_err(|e| format!("{}: {}", String::from_utf8_lossy(n), e))?;
                Ok(*s)
            };
            Ok(DataTables {
                png_sRGB_table: g(b"png_sRGB_table\0")? as *const u16,
                png_sRGB_base: g(b"png_sRGB_base\0")? as *const u16,
                png_sRGB_delta: g(b"png_sRGB_delta\0")?,
            })
        }
    }
}
