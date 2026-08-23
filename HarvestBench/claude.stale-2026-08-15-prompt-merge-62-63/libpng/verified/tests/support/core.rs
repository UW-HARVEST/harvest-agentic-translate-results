//! Typed handles for the most-used libpng entry points, resolved from a `.so`.
#![allow(dead_code)]
#![allow(non_snake_case)]

use super::Lib;
use std::ffi::{c_char, c_int, c_uint, c_void};

pub type Png = *mut c_void;
pub type Info = *mut c_void;
/// Any callback pointer; passed through as an opaque address.
pub type Cb = *mut c_void;

macro_rules! core_struct {
    ( $( $field:ident : $ty:ty = $sym:literal ),* $(,)? ) => {
        pub struct Core {
            $( pub $field: $ty, )*
        }
        impl Core {
            pub fn new(lib: &Lib) -> Core {
                Core { $( $field: lib.f($sym), )* }
            }
        }
    };
}

core_struct! {
    // ---- lifecycle -------------------------------------------------------
    create_write: unsafe extern "C" fn(*const c_char, *mut c_void, Cb, Cb) -> Png
        = "png_create_write_struct",
    create_read: unsafe extern "C" fn(*const c_char, *mut c_void, Cb, Cb) -> Png
        = "png_create_read_struct",
    create_write_2: unsafe extern "C" fn(*const c_char, *mut c_void, Cb, Cb, *mut c_void, Cb, Cb) -> Png
        = "png_create_write_struct_2",
    create_read_2: unsafe extern "C" fn(*const c_char, *mut c_void, Cb, Cb, *mut c_void, Cb, Cb) -> Png
        = "png_create_read_struct_2",
    create_info: unsafe extern "C" fn(Png) -> Info = "png_create_info_struct",
    destroy_write: unsafe extern "C" fn(*mut Png, *mut Info) = "png_destroy_write_struct",
    destroy_read: unsafe extern "C" fn(*mut Png, *mut Info, *mut Info) = "png_destroy_read_struct",
    destroy_info: unsafe extern "C" fn(Png, *mut Info) = "png_destroy_info_struct",
    set_longjmp: unsafe extern "C" fn(Png, *const c_void, usize) -> *mut c_void
        = "png_set_longjmp_fn",
    set_error_fn: unsafe extern "C" fn(Png, *mut c_void, Cb, Cb) = "png_set_error_fn",
    set_mem_fn: unsafe extern "C" fn(Png, *mut c_void, Cb, Cb) = "png_set_mem_fn",
    set_benign_errors: unsafe extern "C" fn(Png, c_int) = "png_set_benign_errors",

    // ---- I/O -------------------------------------------------------------
    set_write_fn: unsafe extern "C" fn(Png, *mut c_void, Cb, Cb) = "png_set_write_fn",
    set_read_fn: unsafe extern "C" fn(Png, *mut c_void, Cb) = "png_set_read_fn",
    get_io_ptr: unsafe extern "C" fn(Png) -> *mut c_void = "png_get_io_ptr",
    get_io_state: unsafe extern "C" fn(Png) -> u32 = "png_get_io_state",
    get_io_chunk_type: unsafe extern "C" fn(Png) -> u32 = "png_get_io_chunk_type",
    set_read_status_fn: unsafe extern "C" fn(Png, Cb) = "png_set_read_status_fn",
    set_write_status_fn: unsafe extern "C" fn(Png, Cb) = "png_set_write_status_fn",

    // ---- header ----------------------------------------------------------
    set_IHDR: unsafe extern "C" fn(Png, Info, u32, u32, c_int, c_int, c_int, c_int, c_int)
        = "png_set_IHDR",
    get_IHDR: unsafe extern "C" fn(Png, Info, *mut u32, *mut u32, *mut c_int, *mut c_int,
        *mut c_int, *mut c_int, *mut c_int) -> u32 = "png_get_IHDR",
    get_rowbytes: unsafe extern "C" fn(Png, Info) -> usize = "png_get_rowbytes",
    get_channels: unsafe extern "C" fn(Png, Info) -> u8 = "png_get_channels",
    get_bit_depth: unsafe extern "C" fn(Png, Info) -> u8 = "png_get_bit_depth",
    get_color_type: unsafe extern "C" fn(Png, Info) -> u8 = "png_get_color_type",
    get_interlace_type: unsafe extern "C" fn(Png, Info) -> u8 = "png_get_interlace_type",
    get_compression_type: unsafe extern "C" fn(Png, Info) -> u8 = "png_get_compression_type",
    get_filter_type: unsafe extern "C" fn(Png, Info) -> u8 = "png_get_filter_type",
    get_image_width: unsafe extern "C" fn(Png, Info) -> u32 = "png_get_image_width",
    get_image_height: unsafe extern "C" fn(Png, Info) -> u32 = "png_get_image_height",
    get_valid: unsafe extern "C" fn(Png, Info, u32) -> u32 = "png_get_valid",
    set_invalid: unsafe extern "C" fn(Png, Info, c_int) = "png_set_invalid",

    // ---- palette / transparency -----------------------------------------
    set_PLTE: unsafe extern "C" fn(Png, Info, *const u8, c_int) = "png_set_PLTE",
    get_PLTE: unsafe extern "C" fn(Png, Info, *mut *mut u8, *mut c_int) -> u32 = "png_get_PLTE",
    set_tRNS: unsafe extern "C" fn(Png, Info, *const u8, c_int, *const u8) = "png_set_tRNS",
    get_tRNS: unsafe extern "C" fn(Png, Info, *mut *mut u8, *mut c_int, *mut *mut u8) -> u32
        = "png_get_tRNS",

    // ---- write -----------------------------------------------------------
    write_sig: unsafe extern "C" fn(Png) = "png_write_sig",
    write_info_before_PLTE: unsafe extern "C" fn(Png, Info) = "png_write_info_before_PLTE",
    write_info: unsafe extern "C" fn(Png, Info) = "png_write_info",
    write_row: unsafe extern "C" fn(Png, *const u8) = "png_write_row",
    write_rows: unsafe extern "C" fn(Png, *mut *mut u8, u32) = "png_write_rows",
    write_image: unsafe extern "C" fn(Png, *mut *mut u8) = "png_write_image",
    write_end: unsafe extern "C" fn(Png, Info) = "png_write_end",
    write_png: unsafe extern "C" fn(Png, Info, c_int, *mut c_void) = "png_write_png",
    write_flush: unsafe extern "C" fn(Png) = "png_write_flush",
    set_flush: unsafe extern "C" fn(Png, c_int) = "png_set_flush",
    write_chunk: unsafe extern "C" fn(Png, *const u8, *const u8, usize) = "png_write_chunk",
    write_chunk_start: unsafe extern "C" fn(Png, *const u8, u32) = "png_write_chunk_start",
    write_chunk_data: unsafe extern "C" fn(Png, *const u8, usize) = "png_write_chunk_data",
    write_chunk_end: unsafe extern "C" fn(Png) = "png_write_chunk_end",

    // ---- read ------------------------------------------------------------
    read_info: unsafe extern "C" fn(Png, Info) = "png_read_info",
    read_update_info: unsafe extern "C" fn(Png, Info) = "png_read_update_info",
    start_read_image: unsafe extern "C" fn(Png) = "png_start_read_image",
    read_row: unsafe extern "C" fn(Png, *mut u8, *mut u8) = "png_read_row",
    read_rows: unsafe extern "C" fn(Png, *mut *mut u8, *mut *mut u8, u32) = "png_read_rows",
    read_image: unsafe extern "C" fn(Png, *mut *mut u8) = "png_read_image",
    read_end: unsafe extern "C" fn(Png, Info) = "png_read_end",
    read_png: unsafe extern "C" fn(Png, Info, c_int, *mut c_void) = "png_read_png",
    set_sig_bytes: unsafe extern "C" fn(Png, c_int) = "png_set_sig_bytes",
    sig_cmp: unsafe extern "C" fn(*const u8, usize, usize) -> c_int = "png_sig_cmp",
    set_crc_action: unsafe extern "C" fn(Png, c_int, c_int) = "png_set_crc_action",
    set_interlace_handling: unsafe extern "C" fn(Png) -> c_int = "png_set_interlace_handling",
    get_current_pass_number: unsafe extern "C" fn(Png) -> u8 = "png_get_current_pass_number",
    get_current_row_number: unsafe extern "C" fn(Png) -> u32 = "png_get_current_row_number",

    // ---- progressive read ------------------------------------------------
    set_progressive_read_fn: unsafe extern "C" fn(Png, *mut c_void, Cb, Cb, Cb)
        = "png_set_progressive_read_fn",
    process_data: unsafe extern "C" fn(Png, Info, *mut u8, usize) = "png_process_data",
    process_data_pause: unsafe extern "C" fn(Png, c_int) -> usize = "png_process_data_pause",
    process_data_skip: unsafe extern "C" fn(Png) -> u32 = "png_process_data_skip",
    progressive_combine_row: unsafe extern "C" fn(Png, *mut u8, *const u8)
        = "png_progressive_combine_row",
    get_progressive_ptr: unsafe extern "C" fn(Png) -> *mut c_void = "png_get_progressive_ptr",

    // ---- compression settings -------------------------------------------
    set_compression_level: unsafe extern "C" fn(Png, c_int) = "png_set_compression_level",
    set_compression_mem_level: unsafe extern "C" fn(Png, c_int) = "png_set_compression_mem_level",
    set_compression_strategy: unsafe extern "C" fn(Png, c_int) = "png_set_compression_strategy",
    set_compression_window_bits: unsafe extern "C" fn(Png, c_int)
        = "png_set_compression_window_bits",
    set_compression_method: unsafe extern "C" fn(Png, c_int) = "png_set_compression_method",
    set_compression_buffer_size: unsafe extern "C" fn(Png, usize)
        = "png_set_compression_buffer_size",
    get_compression_buffer_size: unsafe extern "C" fn(Png) -> usize
        = "png_get_compression_buffer_size",
    set_text_compression_level: unsafe extern "C" fn(Png, c_int) = "png_set_text_compression_level",
    set_text_compression_mem_level: unsafe extern "C" fn(Png, c_int)
        = "png_set_text_compression_mem_level",
    set_text_compression_strategy: unsafe extern "C" fn(Png, c_int)
        = "png_set_text_compression_strategy",
    set_text_compression_window_bits: unsafe extern "C" fn(Png, c_int)
        = "png_set_text_compression_window_bits",
    set_text_compression_method: unsafe extern "C" fn(Png, c_int)
        = "png_set_text_compression_method",
    set_filter: unsafe extern "C" fn(Png, c_int, c_int) = "png_set_filter",

    // ---- read transforms -------------------------------------------------
    set_bgr: unsafe extern "C" fn(Png) = "png_set_bgr",
    set_swap: unsafe extern "C" fn(Png) = "png_set_swap",
    set_swap_alpha: unsafe extern "C" fn(Png) = "png_set_swap_alpha",
    set_packing: unsafe extern "C" fn(Png) = "png_set_packing",
    set_packswap: unsafe extern "C" fn(Png) = "png_set_packswap",
    set_invert_mono: unsafe extern "C" fn(Png) = "png_set_invert_mono",
    set_invert_alpha: unsafe extern "C" fn(Png) = "png_set_invert_alpha",
    set_strip_16: unsafe extern "C" fn(Png) = "png_set_strip_16",
    set_scale_16: unsafe extern "C" fn(Png) = "png_set_scale_16",
    set_strip_alpha: unsafe extern "C" fn(Png) = "png_set_strip_alpha",
    set_expand: unsafe extern "C" fn(Png) = "png_set_expand",
    set_expand_16: unsafe extern "C" fn(Png) = "png_set_expand_16",
    set_expand_gray_1_2_4_to_8: unsafe extern "C" fn(Png) = "png_set_expand_gray_1_2_4_to_8",
    set_palette_to_rgb: unsafe extern "C" fn(Png) = "png_set_palette_to_rgb",
    set_tRNS_to_alpha: unsafe extern "C" fn(Png) = "png_set_tRNS_to_alpha",
    set_gray_to_rgb: unsafe extern "C" fn(Png) = "png_set_gray_to_rgb",
    set_rgb_to_gray: unsafe extern "C" fn(Png, c_int, f64, f64) = "png_set_rgb_to_gray",
    set_rgb_to_gray_fixed: unsafe extern "C" fn(Png, c_int, i32, i32) = "png_set_rgb_to_gray_fixed",
    get_rgb_to_gray_status: unsafe extern "C" fn(Png) -> u8 = "png_get_rgb_to_gray_status",
    set_filler: unsafe extern "C" fn(Png, u32, c_int) = "png_set_filler",
    set_add_alpha: unsafe extern "C" fn(Png, u32, c_int) = "png_set_add_alpha",
    set_shift: unsafe extern "C" fn(Png, *const u8) = "png_set_shift",
    set_quantize: unsafe extern "C" fn(Png, *mut u8, c_int, c_int, *const u16, c_int)
        = "png_set_quantize",
    set_background: unsafe extern "C" fn(Png, *const u8, c_int, c_int, f64) = "png_set_background",
    set_background_fixed: unsafe extern "C" fn(Png, *const u8, c_int, c_int, i32)
        = "png_set_background_fixed",
    set_gamma: unsafe extern "C" fn(Png, f64, f64) = "png_set_gamma",
    set_gamma_fixed: unsafe extern "C" fn(Png, i32, i32) = "png_set_gamma_fixed",
    set_alpha_mode: unsafe extern "C" fn(Png, c_int, f64) = "png_set_alpha_mode",
    set_alpha_mode_fixed: unsafe extern "C" fn(Png, c_int, i32) = "png_set_alpha_mode_fixed",
    set_check_for_invalid_index: unsafe extern "C" fn(Png, c_int)
        = "png_set_check_for_invalid_index",
    get_palette_max: unsafe extern "C" fn(Png, Info) -> c_int = "png_get_palette_max",

    // ---- user callbacks / limits ----------------------------------------
    set_read_user_transform_fn: unsafe extern "C" fn(Png, Cb) = "png_set_read_user_transform_fn",
    set_write_user_transform_fn: unsafe extern "C" fn(Png, Cb) = "png_set_write_user_transform_fn",
    set_user_transform_info: unsafe extern "C" fn(Png, *mut c_void, c_int, c_int)
        = "png_set_user_transform_info",
    get_user_transform_ptr: unsafe extern "C" fn(Png) -> *mut c_void = "png_get_user_transform_ptr",
    set_read_user_chunk_fn: unsafe extern "C" fn(Png, *mut c_void, Cb) = "png_set_read_user_chunk_fn",
    get_user_chunk_ptr: unsafe extern "C" fn(Png) -> *mut c_void = "png_get_user_chunk_ptr",
    set_user_limits: unsafe extern "C" fn(Png, u32, u32) = "png_set_user_limits",
    get_user_width_max: unsafe extern "C" fn(Png) -> u32 = "png_get_user_width_max",
    get_user_height_max: unsafe extern "C" fn(Png) -> u32 = "png_get_user_height_max",
    set_chunk_cache_max: unsafe extern "C" fn(Png, u32) = "png_set_chunk_cache_max",
    get_chunk_cache_max: unsafe extern "C" fn(Png) -> u32 = "png_get_chunk_cache_max",
    set_chunk_malloc_max: unsafe extern "C" fn(Png, usize) = "png_set_chunk_malloc_max",
    get_chunk_malloc_max: unsafe extern "C" fn(Png) -> usize = "png_get_chunk_malloc_max",
    set_keep_unknown_chunks: unsafe extern "C" fn(Png, c_int, *const u8, c_int)
        = "png_set_keep_unknown_chunks",
    handle_as_unknown: unsafe extern "C" fn(Png, *const u8) -> c_int = "png_handle_as_unknown",
    set_unknown_chunks: unsafe extern "C" fn(Png, Info, *const c_void, c_int)
        = "png_set_unknown_chunks",
    get_unknown_chunks: unsafe extern "C" fn(Png, Info, *mut *mut c_void) -> c_int
        = "png_get_unknown_chunks",
    set_unknown_chunk_location: unsafe extern "C" fn(Png, Info, c_int, c_int)
        = "png_set_unknown_chunk_location",
    set_option: unsafe extern "C" fn(Png, c_int, c_int) -> c_int = "png_set_option",
    permit_mng_features: unsafe extern "C" fn(Png, u32) -> u32 = "png_permit_mng_features",
    set_rows: unsafe extern "C" fn(Png, Info, *mut *mut u8) = "png_set_rows",
    get_rows: unsafe extern "C" fn(Png, Info) -> *mut *mut u8 = "png_get_rows",
    free_data: unsafe extern "C" fn(Png, Info, u32, c_int) = "png_free_data",
    data_freer: unsafe extern "C" fn(Png, Info, c_int, u32) = "png_data_freer",

    // ---- memory ----------------------------------------------------------
    malloc: unsafe extern "C" fn(Png, u64) -> *mut c_void = "png_malloc",
    malloc_warn: unsafe extern "C" fn(Png, u64) -> *mut c_void = "png_malloc_warn",
    calloc: unsafe extern "C" fn(Png, u64) -> *mut c_void = "png_calloc",
    free: unsafe extern "C" fn(Png, *mut c_void) = "png_free",
    get_mem_ptr: unsafe extern "C" fn(Png) -> *mut c_void = "png_get_mem_ptr",
    get_error_ptr: unsafe extern "C" fn(Png) -> *mut c_void = "png_get_error_ptr",

    // ---- byte access helpers --------------------------------------------
    get_uint_32: unsafe extern "C" fn(*const u8) -> u32 = "png_get_uint_32",
    get_uint_16: unsafe extern "C" fn(*const u8) -> c_uint = "png_get_uint_16",
    get_int_32: unsafe extern "C" fn(*const u8) -> i32 = "png_get_int_32",
    get_uint_31: unsafe extern "C" fn(Png, *const u8) -> u32 = "png_get_uint_31",
    save_uint_32: unsafe extern "C" fn(*mut u8, u32) = "png_save_uint_32",
    save_int_32: unsafe extern "C" fn(*mut u8, i32) = "png_save_int_32",
    save_uint_16: unsafe extern "C" fn(*mut u8, c_uint) = "png_save_uint_16",

    // ---- info chunks (set/get) ------------------------------------------
    set_text: unsafe extern "C" fn(Png, Info, *const c_void, c_int) = "png_set_text",
    get_text: unsafe extern "C" fn(Png, Info, *mut *mut c_void, *mut c_int) -> c_int
        = "png_get_text",
    set_gAMA_fixed: unsafe extern "C" fn(Png, Info, i32) = "png_set_gAMA_fixed",
    get_gAMA_fixed: unsafe extern "C" fn(Png, Info, *mut i32) -> u32 = "png_get_gAMA_fixed",
    set_gAMA: unsafe extern "C" fn(Png, Info, f64) = "png_set_gAMA",
    get_gAMA: unsafe extern "C" fn(Png, Info, *mut f64) -> u32 = "png_get_gAMA",
    set_sRGB: unsafe extern "C" fn(Png, Info, c_int) = "png_set_sRGB",
    get_sRGB: unsafe extern "C" fn(Png, Info, *mut c_int) -> u32 = "png_get_sRGB",
    set_sRGB_gAMA_and_cHRM: unsafe extern "C" fn(Png, Info, c_int) = "png_set_sRGB_gAMA_and_cHRM",
    set_cHRM_fixed: unsafe extern "C" fn(Png, Info, i32, i32, i32, i32, i32, i32, i32, i32)
        = "png_set_cHRM_fixed",
    get_cHRM_fixed: unsafe extern "C" fn(Png, Info, *mut i32, *mut i32, *mut i32, *mut i32,
        *mut i32, *mut i32, *mut i32, *mut i32) -> u32 = "png_get_cHRM_fixed",
    set_cHRM_XYZ_fixed: unsafe extern "C" fn(Png, Info, i32, i32, i32, i32, i32, i32, i32, i32,
        i32) = "png_set_cHRM_XYZ_fixed",
    get_cHRM_XYZ_fixed: unsafe extern "C" fn(Png, Info, *mut i32, *mut i32, *mut i32, *mut i32,
        *mut i32, *mut i32, *mut i32, *mut i32, *mut i32) -> u32 = "png_get_cHRM_XYZ_fixed",
    set_iCCP: unsafe extern "C" fn(Png, Info, *const c_char, c_int, *const u8, u32) = "png_set_iCCP",
    get_iCCP: unsafe extern "C" fn(Png, Info, *mut *mut c_char, *mut c_int, *mut *mut u8,
        *mut u32) -> u32 = "png_get_iCCP",
    set_sBIT: unsafe extern "C" fn(Png, Info, *const u8) = "png_set_sBIT",
    get_sBIT: unsafe extern "C" fn(Png, Info, *mut *mut u8) -> u32 = "png_get_sBIT",
    set_bKGD: unsafe extern "C" fn(Png, Info, *const u8) = "png_set_bKGD",
    get_bKGD: unsafe extern "C" fn(Png, Info, *mut *mut u8) -> u32 = "png_get_bKGD",
    set_hIST: unsafe extern "C" fn(Png, Info, *const u16) = "png_set_hIST",
    get_hIST: unsafe extern "C" fn(Png, Info, *mut *mut u16) -> u32 = "png_get_hIST",
    set_pHYs: unsafe extern "C" fn(Png, Info, u32, u32, c_int) = "png_set_pHYs",
    get_pHYs: unsafe extern "C" fn(Png, Info, *mut u32, *mut u32, *mut c_int) -> u32
        = "png_get_pHYs",
    set_oFFs: unsafe extern "C" fn(Png, Info, i32, i32, c_int) = "png_set_oFFs",
    get_oFFs: unsafe extern "C" fn(Png, Info, *mut i32, *mut i32, *mut c_int) -> u32
        = "png_get_oFFs",
    set_tIME: unsafe extern "C" fn(Png, Info, *const u8) = "png_set_tIME",
    get_tIME: unsafe extern "C" fn(Png, Info, *mut *mut u8) -> u32 = "png_get_tIME",
    set_pCAL: unsafe extern "C" fn(Png, Info, *const c_char, i32, i32, c_int, c_int,
        *const c_char, *mut *mut c_char) = "png_set_pCAL",
    get_pCAL: unsafe extern "C" fn(Png, Info, *mut *mut c_char, *mut i32, *mut i32, *mut c_int,
        *mut c_int, *mut *mut c_char, *mut *mut *mut c_char) -> u32 = "png_get_pCAL",
    set_sCAL_s: unsafe extern "C" fn(Png, Info, c_int, *const c_char, *const c_char)
        = "png_set_sCAL_s",
    get_sCAL_s: unsafe extern "C" fn(Png, Info, *mut c_int, *mut *mut c_char, *mut *mut c_char)
        -> u32 = "png_get_sCAL_s",
    set_sCAL: unsafe extern "C" fn(Png, Info, c_int, f64, f64) = "png_set_sCAL",
    set_sCAL_fixed: unsafe extern "C" fn(Png, Info, c_int, i32, i32) = "png_set_sCAL_fixed",
    set_sPLT: unsafe extern "C" fn(Png, Info, *const c_void, c_int) = "png_set_sPLT",
    get_sPLT: unsafe extern "C" fn(Png, Info, *mut *mut c_void) -> u32 = "png_get_sPLT",
    set_eXIf_1: unsafe extern "C" fn(Png, Info, u32, *const u8) = "png_set_eXIf_1",
    get_eXIf_1: unsafe extern "C" fn(Png, Info, *mut u32, *mut *mut u8) -> u32 = "png_get_eXIf_1",
    set_cICP: unsafe extern "C" fn(Png, Info, u8, u8, u8, u8) = "png_set_cICP",
    get_cICP: unsafe extern "C" fn(Png, Info, *mut u8, *mut u8, *mut u8, *mut u8) -> u32
        = "png_get_cICP",
    set_cLLI_fixed: unsafe extern "C" fn(Png, Info, u32, u32) = "png_set_cLLI_fixed",
    get_cLLI_fixed: unsafe extern "C" fn(Png, Info, *mut u32, *mut u32) -> u32 = "png_get_cLLI_fixed",
    // png.h: 8 png_fixed_point chromaticities then 2 png_uint_32 luminances.
    set_mDCV_fixed: unsafe extern "C" fn(Png, Info, i32, i32, i32, i32, i32, i32, i32, i32,
        u32, u32) = "png_set_mDCV_fixed",
    get_mDCV_fixed: unsafe extern "C" fn(Png, Info, *mut i32, *mut i32, *mut i32, *mut i32,
        *mut i32, *mut i32, *mut i32, *mut i32, *mut u32, *mut u32) -> u32
        = "png_get_mDCV_fixed",

    // ---- misc ------------------------------------------------------------
    access_version_number: unsafe extern "C" fn() -> u32 = "png_access_version_number",
    get_copyright: unsafe extern "C" fn(Png) -> *const c_char = "png_get_copyright",
    get_libpng_ver: unsafe extern "C" fn(Png) -> *const c_char = "png_get_libpng_ver",
    get_header_ver: unsafe extern "C" fn(Png) -> *const c_char = "png_get_header_ver",
    get_header_version: unsafe extern "C" fn(Png) -> *const c_char = "png_get_header_version",
    convert_to_rfc1123_buffer: unsafe extern "C" fn(*mut c_char, *const u8) -> c_int
        = "png_convert_to_rfc1123_buffer",
    convert_from_time_t: unsafe extern "C" fn(*mut u8, i64) = "png_convert_from_time_t",
    build_grayscale_palette: unsafe extern "C" fn(c_int, *mut u8) = "png_build_grayscale_palette",
    reset_zstream: unsafe extern "C" fn(Png) -> c_int = "png_reset_zstream",
    info_init_3: unsafe extern "C" fn(*mut Info, usize) = "png_info_init_3",
    error: unsafe extern "C" fn(Png, *const c_char) = "png_error",
    warning: unsafe extern "C" fn(Png, *const c_char) = "png_warning",
    longjmp: unsafe extern "C" fn(Png, c_int) = "png_longjmp",
}

// ---------------------------------------------------------------------------
// libpng constants used by the tests (from png.h).
// ---------------------------------------------------------------------------

pub const PNG_COLOR_TYPE_GRAY: c_int = 0;
pub const PNG_COLOR_TYPE_RGB: c_int = 2;
pub const PNG_COLOR_TYPE_PALETTE: c_int = 3;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: c_int = 4;
pub const PNG_COLOR_TYPE_RGB_ALPHA: c_int = 6;

pub const PNG_INTERLACE_NONE: c_int = 0;
pub const PNG_INTERLACE_ADAM7: c_int = 1;

pub const PNG_FILTER_TYPE_BASE: c_int = 0;
pub const PNG_INTRAPIXEL_DIFFERENCING: c_int = 64;
pub const PNG_COMPRESSION_TYPE_BASE: c_int = 0;

pub const PNG_NO_FILTERS: c_int = 0x00;
pub const PNG_FILTER_NONE: c_int = 0x08;
pub const PNG_FILTER_SUB: c_int = 0x10;
pub const PNG_FILTER_UP: c_int = 0x20;
pub const PNG_FILTER_AVG: c_int = 0x40;
pub const PNG_FILTER_PAETH: c_int = 0x80;
pub const PNG_ALL_FILTERS: c_int = 0xF8;

pub const PNG_INFO_gAMA: u32 = 0x0001;
pub const PNG_INFO_sBIT: u32 = 0x0002;
pub const PNG_INFO_cHRM: u32 = 0x0004;
pub const PNG_INFO_PLTE: u32 = 0x0008;
pub const PNG_INFO_tRNS: u32 = 0x0010;
pub const PNG_INFO_bKGD: u32 = 0x0020;
pub const PNG_INFO_hIST: u32 = 0x0040;
pub const PNG_INFO_pHYs: u32 = 0x0080;
pub const PNG_INFO_oFFs: u32 = 0x0100;
pub const PNG_INFO_tIME: u32 = 0x0200;
pub const PNG_INFO_pCAL: u32 = 0x0400;
pub const PNG_INFO_sRGB: u32 = 0x0800;
pub const PNG_INFO_iCCP: u32 = 0x1000;
pub const PNG_INFO_sPLT: u32 = 0x2000;
pub const PNG_INFO_sCAL: u32 = 0x4000;
pub const PNG_INFO_IDAT: u32 = 0x8000;
pub const PNG_INFO_eXIf: u32 = 0x10000;
pub const PNG_INFO_cICP: u32 = 0x20000;
pub const PNG_INFO_cLLI: u32 = 0x40000;
pub const PNG_INFO_mDCV: u32 = 0x80000;

pub const PNG_TRANSFORM_IDENTITY: c_int = 0x0000;
pub const PNG_TRANSFORM_STRIP_16: c_int = 0x0001;
pub const PNG_TRANSFORM_STRIP_ALPHA: c_int = 0x0002;
pub const PNG_TRANSFORM_PACKING: c_int = 0x0004;
pub const PNG_TRANSFORM_PACKSWAP: c_int = 0x0008;
pub const PNG_TRANSFORM_EXPAND: c_int = 0x0010;
pub const PNG_TRANSFORM_INVERT_MONO: c_int = 0x0020;
pub const PNG_TRANSFORM_SHIFT: c_int = 0x0040;
pub const PNG_TRANSFORM_BGR: c_int = 0x0080;
pub const PNG_TRANSFORM_SWAP_ALPHA: c_int = 0x0100;
pub const PNG_TRANSFORM_SWAP_ENDIAN: c_int = 0x0200;
pub const PNG_TRANSFORM_INVERT_ALPHA: c_int = 0x0400;
pub const PNG_TRANSFORM_STRIP_FILLER: c_int = 0x0800;
pub const PNG_TRANSFORM_STRIP_FILLER_BEFORE: c_int = 0x0800;
pub const PNG_TRANSFORM_STRIP_FILLER_AFTER: c_int = 0x1000;
pub const PNG_TRANSFORM_GRAY_TO_RGB: c_int = 0x2000;
pub const PNG_TRANSFORM_EXPAND_16: c_int = 0x4000;
pub const PNG_TRANSFORM_SCALE_16: c_int = 0x8000;

pub const PNG_FILLER_BEFORE: c_int = 0;
pub const PNG_FILLER_AFTER: c_int = 1;

pub const PNG_BACKGROUND_GAMMA_UNKNOWN: c_int = 0;
pub const PNG_BACKGROUND_GAMMA_SCREEN: c_int = 1;
pub const PNG_BACKGROUND_GAMMA_FILE: c_int = 2;
pub const PNG_BACKGROUND_GAMMA_UNIQUE: c_int = 3;

pub const PNG_ALPHA_PNG: c_int = 0;
pub const PNG_ALPHA_STANDARD: c_int = 1;
pub const PNG_ALPHA_ASSOCIATED: c_int = 1;
pub const PNG_ALPHA_OPTIMIZED: c_int = 2;
pub const PNG_ALPHA_BROKEN: c_int = 3;

pub const PNG_ERROR_ACTION_NONE: c_int = 1;
pub const PNG_ERROR_ACTION_WARN: c_int = 2;
pub const PNG_ERROR_ACTION_ERROR: c_int = 3;

pub const PNG_CRC_DEFAULT: c_int = 0;
pub const PNG_CRC_ERROR_QUIT: c_int = 1;
pub const PNG_CRC_WARN_DISCARD: c_int = 2;
pub const PNG_CRC_WARN_USE: c_int = 3;
pub const PNG_CRC_QUIET_USE: c_int = 4;
pub const PNG_CRC_NO_CHANGE: c_int = 5;

pub const PNG_HANDLE_CHUNK_AS_DEFAULT: c_int = 0;
pub const PNG_HANDLE_CHUNK_NEVER: c_int = 1;
pub const PNG_HANDLE_CHUNK_IF_SAFE: c_int = 2;
pub const PNG_HANDLE_CHUNK_ALWAYS: c_int = 3;

pub const PNG_FREE_HIST: u32 = 0x0008;
pub const PNG_FREE_ICCP: u32 = 0x0010;
pub const PNG_FREE_SPLT: u32 = 0x0020;
pub const PNG_FREE_ROWS: u32 = 0x0040;
pub const PNG_FREE_PCAL: u32 = 0x0080;
pub const PNG_FREE_SCAL: u32 = 0x0100;
pub const PNG_FREE_UNKN: u32 = 0x0200;
pub const PNG_FREE_PLTE: u32 = 0x1000;
pub const PNG_FREE_TRNS: u32 = 0x2000;
pub const PNG_FREE_TEXT: u32 = 0x4000;
pub const PNG_FREE_EXIF: u32 = 0x8000;
pub const PNG_FREE_ALL: u32 = 0xFFFF;
pub const PNG_FREE_MUL: u32 = 0x4220;

pub const PNG_DESTROY_WILL_FREE_DATA: c_int = 1;
pub const PNG_SET_WILL_FREE_DATA: c_int = 2;
pub const PNG_USER_WILL_FREE_DATA: c_int = 3;

pub const PNG_TEXT_COMPRESSION_NONE: c_int = -1;
pub const PNG_TEXT_COMPRESSION_zTXt: c_int = 0;
pub const PNG_ITXT_COMPRESSION_NONE: c_int = 1;
pub const PNG_ITXT_COMPRESSION_zTXt: c_int = 2;

pub const PNG_RESOLUTION_UNKNOWN: c_int = 0;
pub const PNG_RESOLUTION_METER: c_int = 1;
pub const PNG_OFFSET_PIXEL: c_int = 0;
pub const PNG_OFFSET_MICROMETER: c_int = 1;
pub const PNG_SCALE_UNKNOWN: c_int = 0;
pub const PNG_SCALE_METER: c_int = 1;
pub const PNG_SCALE_RADIAN: c_int = 2;

pub const PNG_EQUATION_LINEAR: c_int = 0;
pub const PNG_EQUATION_BASE_E: c_int = 1;
pub const PNG_EQUATION_ARBITRARY: c_int = 2;
pub const PNG_EQUATION_HYPERBOLIC: c_int = 3;

pub const PNG_sRGB_INTENT_PERCEPTUAL: c_int = 0;
pub const PNG_sRGB_INTENT_RELATIVE: c_int = 1;
pub const PNG_sRGB_INTENT_SATURATION: c_int = 2;
pub const PNG_sRGB_INTENT_ABSOLUTE: c_int = 3;

/// `png_color` is 3 bytes; a palette is a flat `[u8]` of `3 * n`.
pub fn palette_bytes(entries: &[[u8; 3]]) -> Vec<u8> {
    entries.iter().flat_map(|e| e.iter().copied()).collect()
}

/// `png_color_16 { png_byte index; png_uint_16 red, green, blue, gray; }`
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct PngColor16 {
    pub index: u8,
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub gray: u16,
}

/// `png_color_8 { png_byte red, green, blue, gray, alpha; }`
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct PngColor8 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub gray: u8,
    pub alpha: u8,
}

/// `png_time { png_uint_16 year; png_byte month, day, hour, minute, second; }`
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct PngTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

/// `png_text`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PngText {
    pub compression: c_int,
    pub key: *mut c_char,
    pub text: *mut c_char,
    pub text_length: usize,
    pub itxt_length: usize,
    pub lang: *mut c_char,
    pub lang_key: *mut c_char,
}

impl Default for PngText {
    fn default() -> Self {
        PngText {
            compression: PNG_TEXT_COMPRESSION_NONE,
            key: std::ptr::null_mut(),
            text: std::ptr::null_mut(),
            text_length: 0,
            itxt_length: 0,
            lang: std::ptr::null_mut(),
            lang_key: std::ptr::null_mut(),
        }
    }
}

/// `png_unknown_chunk { png_byte name[5]; png_byte *data; size_t size; png_byte location; }`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PngUnknownChunk {
    pub name: [u8; 5],
    pub data: *mut u8,
    pub size: usize,
    pub location: u8,
}

/// `png_sPLT_t { png_charp name; png_byte depth; png_sPLT_entryp entries; png_int_32 nentries; }`
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PngSpltT {
    pub name: *mut c_char,
    pub depth: u8,
    pub entries: *mut PngSpltEntry,
    pub nentries: i32,
}

/// `png_sPLT_entry { png_uint_16 red, green, blue, alpha, frequency; }`
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct PngSpltEntry {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub alpha: u16,
    pub frequency: u16,
}

// ---------------------------------------------------------------------------
// Simplified API
// ---------------------------------------------------------------------------

/// `png_image` (png.h).  `opaque` must start as NULL and `version` as 1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct PngImage {
    pub opaque: *mut c_void,
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub flags: u32,
    pub colormap_entries: u32,
    pub warning_or_error: u32,
    pub message: [c_char; 64],
}

impl Default for PngImage {
    fn default() -> Self {
        PngImage {
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
}

impl PngImage {
    pub fn msg(&self) -> String {
        let bytes: Vec<u8> = self
            .message
            .iter()
            .take_while(|c| **c != 0)
            .map(|c| *c as u8)
            .collect();
        String::from_utf8_lossy(&bytes).into_owned()
    }
}

pub const PNG_IMAGE_VERSION: u32 = 1;
pub const PNG_IMAGE_WARNING: u32 = 1;
pub const PNG_IMAGE_ERROR: u32 = 2;

pub const PNG_FORMAT_FLAG_ALPHA: u32 = 0x01;
pub const PNG_FORMAT_FLAG_COLOR: u32 = 0x02;
pub const PNG_FORMAT_FLAG_LINEAR: u32 = 0x04;
pub const PNG_FORMAT_FLAG_COLORMAP: u32 = 0x08;
pub const PNG_FORMAT_FLAG_BGR: u32 = 0x10;
pub const PNG_FORMAT_FLAG_AFIRST: u32 = 0x20;
pub const PNG_FORMAT_FLAG_ASSOCIATED_ALPHA: u32 = 0x40;

pub const PNG_FORMAT_GRAY: u32 = 0;
pub const PNG_FORMAT_GA: u32 = PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_AG: u32 = PNG_FORMAT_GA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_RGB: u32 = PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_BGR: u32 = PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_BGR;
pub const PNG_FORMAT_RGBA: u32 = PNG_FORMAT_RGB | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_ARGB: u32 = PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_BGRA: u32 = PNG_FORMAT_BGR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_ABGR: u32 = PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_LINEAR_Y: u32 = PNG_FORMAT_FLAG_LINEAR;
pub const PNG_FORMAT_LINEAR_Y_ALPHA: u32 = PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_LINEAR_RGB: u32 = PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_LINEAR_RGB_ALPHA: u32 =
    PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_RGB_COLORMAP: u32 = PNG_FORMAT_RGB | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_BGR_COLORMAP: u32 = PNG_FORMAT_BGR | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_RGBA_COLORMAP: u32 = PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_ARGB_COLORMAP: u32 = PNG_FORMAT_ARGB | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_BGRA_COLORMAP: u32 = PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_ABGR_COLORMAP: u32 = PNG_FORMAT_ABGR | PNG_FORMAT_FLAG_COLORMAP;

/// `png_row_info` — argument of the internal row transform functions.
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct PngRowInfo {
    pub width: u32,
    pub rowbytes: usize,
    pub color_type: u8,
    pub bit_depth: u8,
    pub channels: u8,
    pub pixel_depth: u8,
}
