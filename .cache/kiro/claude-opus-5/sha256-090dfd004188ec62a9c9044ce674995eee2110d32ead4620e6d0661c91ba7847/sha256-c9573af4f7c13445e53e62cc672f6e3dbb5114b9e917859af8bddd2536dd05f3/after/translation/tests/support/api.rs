//! Dynamic loader for the libpng C ABI.  Every entry point used by the tests is
//! fetched by *symbol name* from the `.so`, so the exact same code path drives
//! the C library and the Rust `cdylib`.

use super::pngdefs::*;
use std::ffi::{c_char, c_double, c_int, c_void};
use std::path::Path;

macro_rules! decl_api {
    ( $( $name:ident : unsafe extern "C" fn( $($at:ty),* $(,)? ) $(-> $rt:ty)? ; )* ) => {
        #[allow(non_snake_case)]
        pub struct Api {
            $( pub $name: unsafe extern "C" fn( $($at),* ) $(-> $rt)?, )*
        }

        impl Api {
            pub unsafe fn load(path: &Path) -> Api {
                // The C build (see c_src/CMakeLists.txt) links only zlib, so its
                // .so has undefined libm references (floor/pow/...).  Make libm
                // globally visible before dlopen'ing it.
                const RTLD_NOW: i32 = 2;
                const RTLD_GLOBAL: i32 = 0x100;
                for l in ["libm.so.6", "libz.so.1"] {
                    if let Ok(h) = libloading::os::unix::Library::open(
                        Some(l),
                        RTLD_NOW | RTLD_GLOBAL,
                    ) {
                        // must stay loaded and RTLD_GLOBAL for the whole process
                        std::mem::forget(h);
                    }
                }
                super::keep_libm_alive();
                let lib: &'static libloading::Library =
                    Box::leak(Box::new(libloading::Library::new(path)
                        .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", path))));
                Api {
                    $( $name: {
                        let s: libloading::Symbol<unsafe extern "C" fn( $($at),* ) $(-> $rt)?> =
                            lib.get(concat!(stringify!($name), "\0").as_bytes())
                               .unwrap_or_else(|e| panic!("dlsym {}: {e}", stringify!($name)));
                        *s
                    }, )*
                }
            }
        }
    };
}

decl_api! {
    /* --- version / misc --- */
    png_access_version_number: unsafe extern "C" fn() -> u32;
    png_get_copyright: unsafe extern "C" fn(PngPtr) -> *const c_char;
    png_get_header_ver: unsafe extern "C" fn(PngPtr) -> *const c_char;
    png_get_header_version: unsafe extern "C" fn(PngPtr) -> *const c_char;
    png_get_libpng_ver: unsafe extern "C" fn(PngPtr) -> *const c_char;

    /* --- struct lifecycle --- */
    png_create_read_struct: unsafe extern "C" fn(*const c_char, *mut c_void, png_error_ptr, png_error_ptr) -> PngPtr;
    png_create_write_struct: unsafe extern "C" fn(*const c_char, *mut c_void, png_error_ptr, png_error_ptr) -> PngPtr;
    png_create_read_struct_2: unsafe extern "C" fn(*const c_char, *mut c_void, png_error_ptr, png_error_ptr, *mut c_void, png_malloc_ptr, png_free_ptr) -> PngPtr;
    png_create_write_struct_2: unsafe extern "C" fn(*const c_char, *mut c_void, png_error_ptr, png_error_ptr, *mut c_void, png_malloc_ptr, png_free_ptr) -> PngPtr;
    png_create_info_struct: unsafe extern "C" fn(PngPtr) -> InfoPtr;
    png_destroy_info_struct: unsafe extern "C" fn(PngPtr, *mut InfoPtr);
    png_destroy_read_struct: unsafe extern "C" fn(*mut PngPtr, *mut InfoPtr, *mut InfoPtr);
    png_destroy_write_struct: unsafe extern "C" fn(*mut PngPtr, *mut InfoPtr);
    png_info_init_3: unsafe extern "C" fn(*mut InfoPtr, usize);

    /* --- error handling --- */
    png_set_error_fn: unsafe extern "C" fn(PngPtr, *mut c_void, png_error_ptr, png_error_ptr);
    png_get_error_ptr: unsafe extern "C" fn(PngPtr) -> *mut c_void;
    png_error: unsafe extern "C" fn(PngPtr, *const c_char);
    png_chunk_error: unsafe extern "C" fn(PngPtr, *const c_char);
    png_warning: unsafe extern "C" fn(PngPtr, *const c_char);
    png_chunk_warning: unsafe extern "C" fn(PngPtr, *const c_char);
    png_benign_error: unsafe extern "C" fn(PngPtr, *const c_char);
    png_chunk_benign_error: unsafe extern "C" fn(PngPtr, *const c_char);
    png_set_benign_errors: unsafe extern "C" fn(PngPtr, c_int);
    png_set_longjmp_fn: unsafe extern "C" fn(PngPtr, *mut c_void, usize) -> *mut c_void;

    /* --- io --- */
    png_set_read_fn: unsafe extern "C" fn(PngPtr, *mut c_void, png_rw_ptr);
    png_set_write_fn: unsafe extern "C" fn(PngPtr, *mut c_void, png_rw_ptr, png_flush_ptr);
    png_get_io_ptr: unsafe extern "C" fn(PngPtr) -> *mut c_void;
    png_set_read_status_fn: unsafe extern "C" fn(PngPtr, png_status_ptr);
    png_set_write_status_fn: unsafe extern "C" fn(PngPtr, png_status_ptr);
    png_get_io_state: unsafe extern "C" fn(PngPtr) -> u32;
    png_get_io_chunk_type: unsafe extern "C" fn(PngPtr) -> u32;
    png_set_mem_fn: unsafe extern "C" fn(PngPtr, *mut c_void, png_malloc_ptr, png_free_ptr);
    png_get_mem_ptr: unsafe extern "C" fn(PngPtr) -> *mut c_void;
    png_init_io: unsafe extern "C" fn(PngPtr, *mut c_void);

    /* --- signature --- */
    png_sig_cmp: unsafe extern "C" fn(*const u8, usize, usize) -> c_int;
    png_set_sig_bytes: unsafe extern "C" fn(PngPtr, c_int);
    png_get_signature: unsafe extern "C" fn(PngPtr, InfoPtr) -> *const u8;
    png_write_sig: unsafe extern "C" fn(PngPtr);

    /* --- integer helpers --- */
    png_get_uint_32: unsafe extern "C" fn(*const u8) -> u32;
    png_get_uint_16: unsafe extern "C" fn(*const u8) -> u16;
    png_get_int_32: unsafe extern "C" fn(*const u8) -> i32;
    png_get_uint_31: unsafe extern "C" fn(PngPtr, *const u8) -> u32;
    png_save_uint_32: unsafe extern "C" fn(*mut u8, u32);
    png_save_int_32: unsafe extern "C" fn(*mut u8, i32);
    png_save_uint_16: unsafe extern "C" fn(*mut u8, c_int);

    /* --- read --- */
    png_read_info: unsafe extern "C" fn(PngPtr, InfoPtr);
    png_read_update_info: unsafe extern "C" fn(PngPtr, InfoPtr);
    png_start_read_image: unsafe extern "C" fn(PngPtr);
    png_read_row: unsafe extern "C" fn(PngPtr, *mut u8, *mut u8);
    png_read_rows: unsafe extern "C" fn(PngPtr, *mut *mut u8, *mut *mut u8, u32);
    png_read_image: unsafe extern "C" fn(PngPtr, *mut *mut u8);
    png_read_end: unsafe extern "C" fn(PngPtr, InfoPtr);
    png_read_png: unsafe extern "C" fn(PngPtr, InfoPtr, c_int, *mut c_void);
    png_reset_zstream: unsafe extern "C" fn(PngPtr) -> c_int;

    /* --- write --- */
    png_write_info_before_PLTE: unsafe extern "C" fn(PngPtr, InfoPtr);
    png_write_info: unsafe extern "C" fn(PngPtr, InfoPtr);
    png_write_row: unsafe extern "C" fn(PngPtr, *const u8);
    png_write_rows: unsafe extern "C" fn(PngPtr, *mut *mut u8, u32);
    png_write_image: unsafe extern "C" fn(PngPtr, *mut *mut u8);
    png_write_end: unsafe extern "C" fn(PngPtr, InfoPtr);
    png_write_png: unsafe extern "C" fn(PngPtr, InfoPtr, c_int, *mut c_void);
    png_write_chunk: unsafe extern "C" fn(PngPtr, *const u8, *const u8, usize);
    png_write_chunk_start: unsafe extern "C" fn(PngPtr, *const u8, u32);
    png_write_chunk_data: unsafe extern "C" fn(PngPtr, *const u8, usize);
    png_write_chunk_end: unsafe extern "C" fn(PngPtr);
    png_write_flush: unsafe extern "C" fn(PngPtr);
    png_set_flush: unsafe extern "C" fn(PngPtr, c_int);

    /* --- IHDR / info --- */
    png_set_IHDR: unsafe extern "C" fn(PngPtr, InfoPtr, u32, u32, c_int, c_int, c_int, c_int, c_int);
    png_get_IHDR: unsafe extern "C" fn(PngPtr, InfoPtr, *mut u32, *mut u32, *mut c_int, *mut c_int, *mut c_int, *mut c_int, *mut c_int) -> u32;
    png_get_valid: unsafe extern "C" fn(PngPtr, InfoPtr, u32) -> u32;
    png_get_rowbytes: unsafe extern "C" fn(PngPtr, InfoPtr) -> usize;
    png_get_channels: unsafe extern "C" fn(PngPtr, InfoPtr) -> u8;
    png_get_image_width: unsafe extern "C" fn(PngPtr, InfoPtr) -> u32;
    png_get_image_height: unsafe extern "C" fn(PngPtr, InfoPtr) -> u32;
    png_get_bit_depth: unsafe extern "C" fn(PngPtr, InfoPtr) -> u8;
    png_get_color_type: unsafe extern "C" fn(PngPtr, InfoPtr) -> u8;
    png_get_filter_type: unsafe extern "C" fn(PngPtr, InfoPtr) -> u8;
    png_get_interlace_type: unsafe extern "C" fn(PngPtr, InfoPtr) -> u8;
    png_get_compression_type: unsafe extern "C" fn(PngPtr, InfoPtr) -> u8;
    png_set_invalid: unsafe extern "C" fn(PngPtr, InfoPtr, c_int);
    png_free_data: unsafe extern "C" fn(PngPtr, InfoPtr, u32, c_int);
    png_data_freer: unsafe extern "C" fn(PngPtr, InfoPtr, c_int, u32);
    png_get_rows: unsafe extern "C" fn(PngPtr, InfoPtr) -> *mut *mut u8;
    png_set_rows: unsafe extern "C" fn(PngPtr, InfoPtr, *mut *mut u8);

    /* --- memory --- */
    png_malloc: unsafe extern "C" fn(PngPtr, usize) -> *mut c_void;
    png_calloc: unsafe extern "C" fn(PngPtr, usize) -> *mut c_void;
    png_malloc_warn: unsafe extern "C" fn(PngPtr, usize) -> *mut c_void;
    png_malloc_default: unsafe extern "C" fn(PngPtr, usize) -> *mut c_void;
    png_free: unsafe extern "C" fn(PngPtr, *mut c_void);
    png_free_default: unsafe extern "C" fn(PngPtr, *mut c_void);

    /* --- chunk setters / getters --- */
    png_set_PLTE: unsafe extern "C" fn(PngPtr, InfoPtr, *const png_color, c_int);
    png_get_PLTE: unsafe extern "C" fn(PngPtr, InfoPtr, *mut *mut png_color, *mut c_int) -> u32;
    png_set_tRNS: unsafe extern "C" fn(PngPtr, InfoPtr, *const u8, c_int, *const png_color_16);
    png_get_tRNS: unsafe extern "C" fn(PngPtr, InfoPtr, *mut *mut u8, *mut c_int, *mut *mut png_color_16) -> u32;
    png_set_bKGD: unsafe extern "C" fn(PngPtr, InfoPtr, *const png_color_16);
    png_get_bKGD: unsafe extern "C" fn(PngPtr, InfoPtr, *mut *mut png_color_16) -> u32;
    png_set_sBIT: unsafe extern "C" fn(PngPtr, InfoPtr, *const png_color_8);
    png_get_sBIT: unsafe extern "C" fn(PngPtr, InfoPtr, *mut *mut png_color_8) -> u32;
    png_set_hIST: unsafe extern "C" fn(PngPtr, InfoPtr, *const u16);
    png_get_hIST: unsafe extern "C" fn(PngPtr, InfoPtr, *mut *mut u16) -> u32;
    png_set_gAMA: unsafe extern "C" fn(PngPtr, InfoPtr, c_double);
    png_set_gAMA_fixed: unsafe extern "C" fn(PngPtr, InfoPtr, i32);
    png_get_gAMA: unsafe extern "C" fn(PngPtr, InfoPtr, *mut c_double) -> u32;
    png_get_gAMA_fixed: unsafe extern "C" fn(PngPtr, InfoPtr, *mut i32) -> u32;
    png_set_sRGB: unsafe extern "C" fn(PngPtr, InfoPtr, c_int);
    png_set_sRGB_gAMA_and_cHRM: unsafe extern "C" fn(PngPtr, InfoPtr, c_int);
    png_get_sRGB: unsafe extern "C" fn(PngPtr, InfoPtr, *mut c_int) -> u32;
    png_set_cHRM: unsafe extern "C" fn(PngPtr, InfoPtr, c_double, c_double, c_double, c_double, c_double, c_double, c_double, c_double);
    png_set_cHRM_fixed: unsafe extern "C" fn(PngPtr, InfoPtr, i32, i32, i32, i32, i32, i32, i32, i32);
    png_set_cHRM_XYZ: unsafe extern "C" fn(PngPtr, InfoPtr, c_double, c_double, c_double, c_double, c_double, c_double, c_double, c_double, c_double);
    png_set_cHRM_XYZ_fixed: unsafe extern "C" fn(PngPtr, InfoPtr, i32, i32, i32, i32, i32, i32, i32, i32, i32);
    png_get_cHRM_fixed: unsafe extern "C" fn(PngPtr, InfoPtr, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32) -> u32;
    png_get_cHRM_XYZ_fixed: unsafe extern "C" fn(PngPtr, InfoPtr, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32) -> u32;
    png_set_iCCP: unsafe extern "C" fn(PngPtr, InfoPtr, *const c_char, c_int, *const u8, u32);
    png_get_iCCP: unsafe extern "C" fn(PngPtr, InfoPtr, *mut *mut c_char, *mut c_int, *mut *mut u8, *mut u32) -> u32;
    png_set_text: unsafe extern "C" fn(PngPtr, InfoPtr, *const png_text, c_int);
    png_get_text: unsafe extern "C" fn(PngPtr, InfoPtr, *mut *mut png_text, *mut c_int) -> c_int;
    png_set_tIME: unsafe extern "C" fn(PngPtr, InfoPtr, *const png_time);
    png_get_tIME: unsafe extern "C" fn(PngPtr, InfoPtr, *mut *mut png_time) -> u32;
    png_convert_to_rfc1123_buffer: unsafe extern "C" fn(*mut c_char, *const png_time) -> c_int;
    png_convert_to_rfc1123: unsafe extern "C" fn(PngPtr, *const png_time) -> *const c_char;
    png_convert_from_time_t: unsafe extern "C" fn(*mut png_time, i64);
    png_set_pHYs: unsafe extern "C" fn(PngPtr, InfoPtr, u32, u32, c_int);
    png_get_pHYs: unsafe extern "C" fn(PngPtr, InfoPtr, *mut u32, *mut u32, *mut c_int) -> u32;
    png_get_pHYs_dpi: unsafe extern "C" fn(PngPtr, InfoPtr, *mut u32, *mut u32, *mut c_int) -> u32;
    png_set_oFFs: unsafe extern "C" fn(PngPtr, InfoPtr, i32, i32, c_int);
    png_get_oFFs: unsafe extern "C" fn(PngPtr, InfoPtr, *mut i32, *mut i32, *mut c_int) -> u32;
    png_set_sCAL: unsafe extern "C" fn(PngPtr, InfoPtr, c_int, c_double, c_double);
    png_set_sCAL_fixed: unsafe extern "C" fn(PngPtr, InfoPtr, c_int, i32, i32);
    png_set_sCAL_s: unsafe extern "C" fn(PngPtr, InfoPtr, c_int, *const c_char, *const c_char);
    png_get_sCAL_s: unsafe extern "C" fn(PngPtr, InfoPtr, *mut c_int, *mut *mut c_char, *mut *mut c_char) -> u32;
    png_get_sCAL_fixed: unsafe extern "C" fn(PngPtr, InfoPtr, *mut c_int, *mut i32, *mut i32) -> u32;
    png_set_sPLT: unsafe extern "C" fn(PngPtr, InfoPtr, *const png_sPLT_t, c_int);
    png_get_sPLT: unsafe extern "C" fn(PngPtr, InfoPtr, *mut *mut png_sPLT_t) -> c_int;
    png_set_pCAL: unsafe extern "C" fn(PngPtr, InfoPtr, *const c_char, i32, i32, c_int, c_int, *const c_char, *mut *mut c_char);
    png_get_pCAL: unsafe extern "C" fn(PngPtr, InfoPtr, *mut *mut c_char, *mut i32, *mut i32, *mut c_int, *mut c_int, *mut *mut c_char, *mut *mut *mut c_char) -> u32;
    png_set_eXIf_1: unsafe extern "C" fn(PngPtr, InfoPtr, u32, *mut u8);
    png_get_eXIf_1: unsafe extern "C" fn(PngPtr, InfoPtr, *mut u32, *mut *mut u8) -> u32;
    png_set_cICP: unsafe extern "C" fn(PngPtr, InfoPtr, u8, u8, u8, u8);
    png_get_cICP: unsafe extern "C" fn(PngPtr, InfoPtr, *mut u8, *mut u8, *mut u8, *mut u8) -> u32;
    png_set_cLLI_fixed: unsafe extern "C" fn(PngPtr, InfoPtr, u32, u32);
    png_get_cLLI_fixed: unsafe extern "C" fn(PngPtr, InfoPtr, *mut u32, *mut u32) -> u32;
    png_set_mDCV_fixed: unsafe extern "C" fn(PngPtr, InfoPtr, i32, i32, i32, i32, i32, i32, i32, i32, u32, u32);
    png_get_mDCV_fixed: unsafe extern "C" fn(PngPtr, InfoPtr, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut i32, *mut u32, *mut u32) -> u32;

    /* --- unknown chunks --- */
    png_set_keep_unknown_chunks: unsafe extern "C" fn(PngPtr, c_int, *const u8, c_int);
    png_handle_as_unknown: unsafe extern "C" fn(PngPtr, *const u8) -> c_int;
    png_set_unknown_chunks: unsafe extern "C" fn(PngPtr, InfoPtr, *const png_unknown_chunk, c_int);
    png_set_unknown_chunk_location: unsafe extern "C" fn(PngPtr, InfoPtr, c_int, c_int);
    png_get_unknown_chunks: unsafe extern "C" fn(PngPtr, InfoPtr, *mut *mut png_unknown_chunk) -> c_int;
    png_set_read_user_chunk_fn: unsafe extern "C" fn(PngPtr, *mut c_void, png_user_chunk_ptr);
    png_get_user_chunk_ptr: unsafe extern "C" fn(PngPtr) -> *mut c_void;

    /* --- read transforms --- */
    png_set_expand: unsafe extern "C" fn(PngPtr);
    png_set_expand_gray_1_2_4_to_8: unsafe extern "C" fn(PngPtr);
    png_set_palette_to_rgb: unsafe extern "C" fn(PngPtr);
    png_set_tRNS_to_alpha: unsafe extern "C" fn(PngPtr);
    png_set_expand_16: unsafe extern "C" fn(PngPtr);
    png_set_bgr: unsafe extern "C" fn(PngPtr);
    png_set_gray_to_rgb: unsafe extern "C" fn(PngPtr);
    png_set_rgb_to_gray: unsafe extern "C" fn(PngPtr, c_int, c_double, c_double);
    png_set_rgb_to_gray_fixed: unsafe extern "C" fn(PngPtr, c_int, i32, i32);
    png_get_rgb_to_gray_status: unsafe extern "C" fn(PngPtr) -> u8;
    png_build_grayscale_palette: unsafe extern "C" fn(c_int, *mut png_color);
    png_set_alpha_mode: unsafe extern "C" fn(PngPtr, c_int, c_double);
    png_set_alpha_mode_fixed: unsafe extern "C" fn(PngPtr, c_int, i32);
    png_set_strip_alpha: unsafe extern "C" fn(PngPtr);
    png_set_swap_alpha: unsafe extern "C" fn(PngPtr);
    png_set_invert_alpha: unsafe extern "C" fn(PngPtr);
    png_set_filler: unsafe extern "C" fn(PngPtr, u32, c_int);
    png_set_add_alpha: unsafe extern "C" fn(PngPtr, u32, c_int);
    png_set_swap: unsafe extern "C" fn(PngPtr);
    png_set_packing: unsafe extern "C" fn(PngPtr);
    png_set_packswap: unsafe extern "C" fn(PngPtr);
    png_set_shift: unsafe extern "C" fn(PngPtr, *const png_color_8);
    png_set_interlace_handling: unsafe extern "C" fn(PngPtr) -> c_int;
    png_set_invert_mono: unsafe extern "C" fn(PngPtr);
    png_set_background: unsafe extern "C" fn(PngPtr, *const png_color_16, c_int, c_int, c_double);
    png_set_background_fixed: unsafe extern "C" fn(PngPtr, *const png_color_16, c_int, c_int, i32);
    png_set_scale_16: unsafe extern "C" fn(PngPtr);
    png_set_strip_16: unsafe extern "C" fn(PngPtr);
    png_set_quantize: unsafe extern "C" fn(PngPtr, *mut png_color, c_int, c_int, *const u16, c_int);
    png_set_gamma: unsafe extern "C" fn(PngPtr, c_double, c_double);
    png_set_gamma_fixed: unsafe extern "C" fn(PngPtr, i32, i32);
    png_set_crc_action: unsafe extern "C" fn(PngPtr, c_int, c_int);
    png_set_check_for_invalid_index: unsafe extern "C" fn(PngPtr, c_int);
    png_get_palette_max: unsafe extern "C" fn(PngPtr, InfoPtr) -> c_int;
    png_permit_mng_features: unsafe extern "C" fn(PngPtr, u32) -> u32;
    png_set_option: unsafe extern "C" fn(PngPtr, c_int, c_int) -> c_int;

    /* --- write options --- */
    png_set_filter: unsafe extern "C" fn(PngPtr, c_int, c_int);
    png_set_filter_heuristics: unsafe extern "C" fn(PngPtr, c_int, c_int, *const c_double, *const c_double);
    png_set_filter_heuristics_fixed: unsafe extern "C" fn(PngPtr, c_int, c_int, *const i32, *const i32);
    png_set_compression_level: unsafe extern "C" fn(PngPtr, c_int);
    png_set_compression_mem_level: unsafe extern "C" fn(PngPtr, c_int);
    png_set_compression_strategy: unsafe extern "C" fn(PngPtr, c_int);
    png_set_compression_window_bits: unsafe extern "C" fn(PngPtr, c_int);
    png_set_compression_method: unsafe extern "C" fn(PngPtr, c_int);
    png_set_text_compression_level: unsafe extern "C" fn(PngPtr, c_int);
    png_set_text_compression_mem_level: unsafe extern "C" fn(PngPtr, c_int);
    png_set_text_compression_strategy: unsafe extern "C" fn(PngPtr, c_int);
    png_set_text_compression_window_bits: unsafe extern "C" fn(PngPtr, c_int);
    png_set_text_compression_method: unsafe extern "C" fn(PngPtr, c_int);
    png_get_compression_buffer_size: unsafe extern "C" fn(PngPtr) -> usize;
    png_set_compression_buffer_size: unsafe extern "C" fn(PngPtr, usize);

    /* --- user limits --- */
    png_set_user_limits: unsafe extern "C" fn(PngPtr, u32, u32);
    png_get_user_width_max: unsafe extern "C" fn(PngPtr) -> u32;
    png_get_user_height_max: unsafe extern "C" fn(PngPtr) -> u32;
    png_set_chunk_cache_max: unsafe extern "C" fn(PngPtr, u32);
    png_get_chunk_cache_max: unsafe extern "C" fn(PngPtr) -> u32;
    png_set_chunk_malloc_max: unsafe extern "C" fn(PngPtr, usize);
    png_get_chunk_malloc_max: unsafe extern "C" fn(PngPtr) -> usize;

    /* --- user transforms --- */
    png_set_read_user_transform_fn: unsafe extern "C" fn(PngPtr, png_user_transform_ptr);
    png_set_write_user_transform_fn: unsafe extern "C" fn(PngPtr, png_user_transform_ptr);
    png_set_user_transform_info: unsafe extern "C" fn(PngPtr, *mut c_void, c_int, c_int);
    png_get_user_transform_ptr: unsafe extern "C" fn(PngPtr) -> *mut c_void;
    png_get_current_row_number: unsafe extern "C" fn(PngPtr) -> u32;
    png_get_current_pass_number: unsafe extern "C" fn(PngPtr) -> u8;

    /* --- progressive read --- */
    png_set_progressive_read_fn: unsafe extern "C" fn(PngPtr, *mut c_void, png_progressive_info_ptr, png_progressive_row_ptr, png_progressive_end_ptr);
    png_get_progressive_ptr: unsafe extern "C" fn(PngPtr) -> *mut c_void;
    png_process_data: unsafe extern "C" fn(PngPtr, InfoPtr, *mut u8, usize);
    png_process_data_pause: unsafe extern "C" fn(PngPtr, c_int) -> usize;
    png_process_data_skip: unsafe extern "C" fn(PngPtr) -> u32;
    png_progressive_combine_row: unsafe extern "C" fn(PngPtr, *mut u8, *const u8);

    /* --- easy access --- */
    png_get_pixels_per_meter: unsafe extern "C" fn(PngPtr, InfoPtr) -> u32;
    png_get_x_pixels_per_meter: unsafe extern "C" fn(PngPtr, InfoPtr) -> u32;
    png_get_y_pixels_per_meter: unsafe extern "C" fn(PngPtr, InfoPtr) -> u32;
    png_get_pixel_aspect_ratio_fixed: unsafe extern "C" fn(PngPtr, InfoPtr) -> i32;
    png_get_x_offset_pixels: unsafe extern "C" fn(PngPtr, InfoPtr) -> i32;
    png_get_y_offset_pixels: unsafe extern "C" fn(PngPtr, InfoPtr) -> i32;
    png_get_x_offset_microns: unsafe extern "C" fn(PngPtr, InfoPtr) -> i32;
    png_get_y_offset_microns: unsafe extern "C" fn(PngPtr, InfoPtr) -> i32;
    png_get_pixels_per_inch: unsafe extern "C" fn(PngPtr, InfoPtr) -> u32;
    png_get_x_pixels_per_inch: unsafe extern "C" fn(PngPtr, InfoPtr) -> u32;
    png_get_y_pixels_per_inch: unsafe extern "C" fn(PngPtr, InfoPtr) -> u32;
    png_get_x_offset_inches_fixed: unsafe extern "C" fn(PngPtr, InfoPtr) -> i32;
    png_get_y_offset_inches_fixed: unsafe extern "C" fn(PngPtr, InfoPtr) -> i32;

    /* --- simplified API --- */
    png_image_begin_read_from_memory: unsafe extern "C" fn(*mut png_image, *const c_void, usize) -> c_int;
    png_image_begin_read_from_file: unsafe extern "C" fn(*mut png_image, *const c_char) -> c_int;
    png_image_begin_read_from_stdio: unsafe extern "C" fn(*mut png_image, *mut c_void) -> c_int;
    png_image_finish_read: unsafe extern "C" fn(*mut png_image, *const png_color, *mut c_void, i32, *mut c_void) -> c_int;
    png_image_free: unsafe extern "C" fn(*mut png_image);
    png_image_write_to_memory: unsafe extern "C" fn(*mut png_image, *mut c_void, *mut usize, c_int, *const c_void, i32, *const c_void) -> c_int;
    png_image_write_to_file: unsafe extern "C" fn(*mut png_image, *const c_char, c_int, *const c_void, i32, *const c_void) -> c_int;
    png_image_write_to_stdio: unsafe extern "C" fn(*mut png_image, *mut c_void, c_int, *const c_void, i32, *const c_void) -> c_int;

    /* --- floating-point variants of the fixed-point APIs --- */
    png_set_cLLI: unsafe extern "C" fn(PngPtr, InfoPtr, c_double, c_double);
    png_get_cLLI: unsafe extern "C" fn(PngPtr, InfoPtr, *mut c_double, *mut c_double) -> u32;
    png_set_mDCV: unsafe extern "C" fn(PngPtr, InfoPtr, c_double, c_double, c_double, c_double, c_double, c_double, c_double, c_double, c_double, c_double);
    png_get_mDCV: unsafe extern "C" fn(PngPtr, InfoPtr, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double) -> u32;
    png_get_cHRM: unsafe extern "C" fn(PngPtr, InfoPtr, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double) -> u32;
    png_get_cHRM_XYZ: unsafe extern "C" fn(PngPtr, InfoPtr, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double, *mut c_double) -> u32;
    png_get_sCAL: unsafe extern "C" fn(PngPtr, InfoPtr, *mut c_int, *mut c_double, *mut c_double) -> u32;
    png_get_pixel_aspect_ratio: unsafe extern "C" fn(PngPtr, InfoPtr) -> f32;
    png_get_x_offset_inches: unsafe extern "C" fn(PngPtr, InfoPtr) -> f32;
    png_get_y_offset_inches: unsafe extern "C" fn(PngPtr, InfoPtr) -> f32;
    png_convert_from_struct_tm: unsafe extern "C" fn(*mut png_time, *const c_void);
}
