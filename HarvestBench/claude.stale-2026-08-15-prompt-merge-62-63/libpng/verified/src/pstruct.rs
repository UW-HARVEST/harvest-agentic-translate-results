//! Internal png_struct_def and png_info_def, mirroring pngstruct.h and
//! pnginfo.h for the enabled configuration (full libpng 1.6.59 feature set).
//!
//! These structures are opaque to applications, but libpng copies them by
//! value in a few places (e.g. png_destroy_png_struct), so we mirror the C
//! layout with `#[repr(C)]` and preserve field names/order.
#![allow(non_camel_case_types)]

use core::ffi::{c_int, c_uint};

use crate::cffi::z_stream;
use crate::ptypes::*;

/// glibc jmp_buf: struct __jmp_buf_tag[1], 200 bytes, 8-byte aligned.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct jmp_buf {
    pub bytes: [u64; 25],
}

pub const PNG_FILTER_VALUE_LAST_M1: usize = PNG_FILTER_VALUE_COUNT - 1;

#[repr(C)]
pub struct png_struct_def {
    // setjmp
    pub jmp_buf_local: jmp_buf,
    pub longjmp_fn: png_longjmp_ptr,
    pub jmp_buf_ptr: *mut jmp_buf,
    pub jmp_buf_size: usize,

    pub error_fn: png_error_ptr,
    pub warning_fn: png_error_ptr,
    pub error_ptr: png_voidp,
    pub write_data_fn: png_rw_ptr,
    pub read_data_fn: png_rw_ptr,
    pub io_ptr: png_voidp,

    pub read_user_transform_fn: png_user_transform_ptr,
    pub write_user_transform_fn: png_user_transform_ptr,

    pub user_transform_ptr: png_voidp,
    pub user_transform_depth: png_byte,
    pub user_transform_channels: png_byte,

    pub mode: png_uint_32,
    pub flags: png_uint_32,
    pub transformations: png_uint_32,

    pub zowner: png_uint_32,
    pub zstream: z_stream,

    // WRITE_SUPPORTED
    pub zbuffer_list: png_compression_bufferp,
    pub zbuffer_size: crate::cffi::uInt,
    pub zlib_level: c_int,
    pub zlib_method: c_int,
    pub zlib_window_bits: c_int,
    pub zlib_mem_level: c_int,
    pub zlib_strategy: c_int,

    // WRITE_CUSTOMIZE_ZTXT_COMPRESSION
    pub zlib_text_level: c_int,
    pub zlib_text_method: c_int,
    pub zlib_text_window_bits: c_int,
    pub zlib_text_mem_level: c_int,
    pub zlib_text_strategy: c_int,

    // WRITE_SUPPORTED (1.6.0)
    pub zlib_set_level: c_int,
    pub zlib_set_method: c_int,
    pub zlib_set_window_bits: c_int,
    pub zlib_set_mem_level: c_int,
    pub zlib_set_strategy: c_int,

    pub chunks: png_uint_32,

    pub width: png_uint_32,
    pub height: png_uint_32,
    pub num_rows: png_uint_32,
    pub usr_width: png_uint_32,
    pub rowbytes: usize,
    pub iwidth: png_uint_32,
    pub row_number: png_uint_32,
    pub chunk_name: png_uint_32,
    pub prev_row: png_bytep,
    pub row_buf: png_bytep,

    // WRITE_FILTER
    pub try_row: png_bytep,
    pub tst_row: png_bytep,

    pub info_rowbytes: usize,

    pub idat_size: png_uint_32,
    pub crc: png_uint_32,
    pub palette: png_colorp,
    pub num_palette: png_uint_16,

    // CHECK_FOR_INVALID_INDEX
    pub num_palette_max: c_int,

    pub num_trans: png_uint_16,
    pub compression: png_byte,
    pub filter: png_byte,
    pub interlaced: png_byte,
    pub pass: png_byte,
    pub do_filter: png_byte,
    pub color_type: png_byte,
    pub bit_depth: png_byte,
    pub usr_bit_depth: png_byte,
    pub pixel_depth: png_byte,
    pub channels: png_byte,
    pub usr_channels: png_byte,
    pub sig_bytes: png_byte,
    pub maximum_pixel_depth: png_byte,
    pub transformed_pixel_depth: png_byte,

    // ZLIB >= 1.2.4
    pub zstream_start: png_byte,

    // FILLER
    pub filler: png_uint_16,

    // bKGD / READ_BACKGROUND / READ_ALPHA_MODE
    pub background_gamma_type: png_byte,
    pub background_gamma: png_fixed_point,
    pub background: png_color_16,
    pub background_1: png_color_16, // READ_GAMMA

    // WRITE_FLUSH
    pub output_flush_fn: png_flush_ptr,
    pub flush_dist: png_uint_32,
    pub flush_rows: png_uint_32,

    // READ_RGB_TO_GRAY
    pub chromaticities: png_xy,

    // READ_GAMMA
    pub gamma_shift: c_int,
    pub screen_gamma: png_fixed_point,
    pub file_gamma: png_fixed_point,
    pub chunk_gamma: png_fixed_point,
    pub default_gamma: png_fixed_point,
    pub gamma_table: png_bytep,
    pub gamma_16_table: png_uint_16pp,
    pub gamma_from_1: png_bytep,
    pub gamma_to_1: png_bytep,
    pub gamma_16_from_1: png_uint_16pp,
    pub gamma_16_to_1: png_uint_16pp,

    // READ_GAMMA || sBIT
    pub sig_bit: png_color_8,

    // READ_SHIFT || WRITE_SHIFT
    pub shift: png_color_8,

    // tRNS ...
    pub trans_alpha: png_bytep,
    pub trans_color: png_color_16,

    pub read_row_fn: png_read_status_ptr,
    pub write_row_fn: png_write_status_ptr,

    // PROGRESSIVE_READ
    pub info_fn: png_progressive_info_ptr,
    pub row_fn: png_progressive_row_ptr,
    pub end_fn: png_progressive_end_ptr,
    pub save_buffer_ptr: png_bytep,
    pub save_buffer: png_bytep,
    pub current_buffer_ptr: png_bytep,
    pub current_buffer: png_bytep,
    pub push_length: png_uint_32,
    pub skip_length: png_uint_32,
    pub save_buffer_size: usize,
    pub save_buffer_max: usize,
    pub buffer_size: usize,
    pub current_buffer_size: usize,
    pub process_mode: c_int,
    pub cur_palette: c_int,

    // READ_QUANTIZE
    pub palette_lookup: png_bytep,
    pub quantize_index: png_bytep,

    // SET_OPTION
    pub options: png_uint_32,

    // LIBPNG_VER < 10700 && TIME_RFC1123
    pub time_buffer: [core::ffi::c_char; 29],

    pub free_me: png_uint_32,

    // USER_CHUNKS
    pub user_chunk_ptr: png_voidp,
    pub read_user_chunk_fn: png_user_chunk_ptr,

    // SET_UNKNOWN_CHUNKS
    pub unknown_default: c_int,
    pub num_chunk_list: c_uint,
    pub chunk_list: png_bytep,

    // READ_RGB_TO_GRAY (1.0.3)
    pub rgb_to_gray_status: png_byte,
    pub rgb_to_gray_coefficients_set: png_byte,
    pub rgb_to_gray_red_coeff: png_uint_16,
    pub rgb_to_gray_green_coeff: png_uint_16,

    // MNG_FEATURES
    pub mng_features_permitted: png_uint_32,
    pub filter_type: png_byte,

    // USER_MEM
    pub mem_ptr: png_voidp,
    pub malloc_fn: png_malloc_ptr,
    pub free_fn: png_free_ptr,

    pub big_row_buf: png_bytep,

    // READ_QUANTIZE
    pub index_to_palette: png_bytep,
    pub palette_to_index: png_bytep,

    pub compression_type: png_byte,

    // USER_LIMITS
    pub user_width_max: png_uint_32,
    pub user_height_max: png_uint_32,
    pub user_chunk_cache_max: png_uint_32,
    pub user_chunk_malloc_max: png_alloc_size_t,

    // READ_UNKNOWN_CHUNKS
    pub unknown_chunk: png_unknown_chunk,

    pub old_big_row_buf_size: usize,

    // READ_SUPPORTED
    pub read_buffer: png_bytep,
    pub read_buffer_size: png_alloc_size_t,

    // SEQUENTIAL_READ
    pub IDAT_read_size: crate::cffi::uInt,

    // IO_STATE
    pub io_state: png_uint_32,

    pub big_prev_row: png_bytep,

    // read_filter[PNG_FILTER_VALUE_LAST-1]
    pub read_filter: [png_read_filter_fn; PNG_FILTER_VALUE_LAST_M1],
}

#[repr(C)]
pub struct png_info_def {
    pub width: png_uint_32,
    pub height: png_uint_32,
    pub valid: png_uint_32,
    pub rowbytes: usize,
    pub palette: png_colorp,
    pub num_palette: png_uint_16,
    pub num_trans: png_uint_16,
    pub bit_depth: png_byte,
    pub color_type: png_byte,
    pub compression_type: png_byte,
    pub filter_type: png_byte,
    pub interlace_type: png_byte,
    pub channels: png_byte,
    pub pixel_depth: png_byte,
    pub spare_byte: png_byte,

    // READ_SUPPORTED
    pub signature: [png_byte; 8],

    // cICP
    pub cicp_colour_primaries: png_byte,
    pub cicp_transfer_function: png_byte,
    pub cicp_matrix_coefficients: png_byte,
    pub cicp_video_full_range_flag: png_byte,

    // iCCP
    pub iccp_name: png_charp,
    pub iccp_profile: png_bytep,
    pub iccp_proflen: png_uint_32,

    // cLLI
    pub maxCLL: png_uint_32,
    pub maxFALL: png_uint_32,

    // mDCV
    pub mastering_red_x: png_uint_16,
    pub mastering_red_y: png_uint_16,
    pub mastering_green_x: png_uint_16,
    pub mastering_green_y: png_uint_16,
    pub mastering_blue_x: png_uint_16,
    pub mastering_blue_y: png_uint_16,
    pub mastering_white_x: png_uint_16,
    pub mastering_white_y: png_uint_16,
    pub mastering_maxDL: png_uint_32,
    pub mastering_minDL: png_uint_32,

    // TEXT
    pub num_text: c_int,
    pub max_text: c_int,
    pub text: png_textp,

    // tIME
    pub mod_time: png_time,

    // sBIT
    pub sig_bit: png_color_8,

    // tRNS / READ_EXPAND / READ_BACKGROUND
    pub trans_alpha: png_bytep,
    pub trans_color: png_color_16,

    // bKGD / READ_BACKGROUND
    pub background: png_color_16,

    // oFFs
    pub x_offset: png_int_32,
    pub y_offset: png_int_32,
    pub offset_unit_type: png_byte,

    // pHYs
    pub x_pixels_per_unit: png_uint_32,
    pub y_pixels_per_unit: png_uint_32,
    pub phys_unit_type: png_byte,

    // eXIf
    pub num_exif: png_uint_32,
    pub exif: png_bytep,

    // hIST
    pub hist: png_uint_16p,

    // pCAL
    pub pcal_purpose: png_charp,
    pub pcal_X0: png_int_32,
    pub pcal_X1: png_int_32,
    pub pcal_units: png_charp,
    pub pcal_params: png_charpp,
    pub pcal_type: png_byte,
    pub pcal_nparams: png_byte,

    pub free_me: png_uint_32,

    // STORE_UNKNOWN_CHUNKS
    pub unknown_chunks: png_unknown_chunkp,
    pub unknown_chunks_num: c_int,

    // sPLT
    pub splt_palettes: png_sPLT_tp,
    pub splt_palettes_num: c_int,

    // sCAL
    pub scal_unit: png_byte,
    pub scal_s_width: png_charp,
    pub scal_s_height: png_charp,

    // INFO_IMAGE
    pub row_pointers: png_bytepp,

    // cHRM
    pub cHRM: png_xy,

    // gAMA
    pub gamma: png_fixed_point,

    // sRGB
    pub rendering_intent: c_int,
}
