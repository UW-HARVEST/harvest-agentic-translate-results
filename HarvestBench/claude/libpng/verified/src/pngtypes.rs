//! Translation of png.h / pngconf.h / pngstruct.h / pnginfo.h / pngpriv.h
//! type and constant definitions.

#![allow(non_camel_case_types, non_upper_case_globals, dead_code)]

use crate::ffi::*;
pub use core::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void};

/* ================================================================== */
/* pngconf.h base types                                                */
/* ================================================================== */

pub type png_byte = u8;
pub type png_int_16 = i16;
pub type png_uint_16 = u16;
pub type png_int_32 = i32;
pub type png_uint_32 = u32;
pub type png_size_t = usize;
pub type png_ptrdiff_t = isize;
pub type png_alloc_size_t = usize;
pub type png_fixed_point = i32;

pub type png_voidp = *mut c_void;
pub type png_const_voidp = *const c_void;
pub type png_bytep = *mut png_byte;
pub type png_const_bytep = *const png_byte;
pub type png_uint_32p = *mut png_uint_32;
pub type png_const_uint_32p = *const png_uint_32;
pub type png_int_32p = *mut png_int_32;
pub type png_const_int_32p = *const png_int_32;
pub type png_uint_16p = *mut png_uint_16;
pub type png_const_uint_16p = *const png_uint_16;
pub type png_int_16p = *mut png_int_16;
pub type png_const_int_16p = *const png_int_16;
pub type png_charp = *mut c_char;
pub type png_const_charp = *const c_char;
pub type png_fixed_point_p = *mut png_fixed_point;
pub type png_const_fixed_point_p = *const png_fixed_point;
pub type png_size_tp = *mut usize;
pub type png_const_size_tp = *const usize;
pub type png_doublep = *mut f64;
pub type png_const_doublep = *const f64;
pub type png_bytepp = *mut png_bytep;
pub type png_uint_32pp = *mut png_uint_32p;
pub type png_int_32pp = *mut png_int_32p;
pub type png_uint_16pp = *mut png_uint_16p;
pub type png_int_16pp = *mut png_int_16p;
pub type png_const_charpp = *mut png_const_charp;
pub type png_charpp = *mut png_charp;
pub type png_fixed_point_pp = *mut png_fixed_point_p;
pub type png_doublepp = *mut png_doublep;
pub type png_charppp = *mut png_charpp;
pub type png_FILE_p = *mut FILE;

/// `const png_uint_16p *` (pngpriv.h)
pub type png_const_uint_16pp = *const png_uint_16p;

pub const PNG_UINT_31_MAX: png_uint_32 = 0x7fffffff;
pub const PNG_UINT_32_MAX: png_uint_32 = u32::MAX;
pub const PNG_SIZE_MAX: usize = usize::MAX;

pub const INT_MAX: c_int = i32::MAX;
pub const INT_MIN: c_int = i32::MIN;

/* ================================================================== */
/* Version information                                                 */
/* ================================================================== */

pub const PNG_LIBPNG_VER_STRING: &[u8] = b"1.6.59.git\0";
pub const PNG_HEADER_VERSION_STRING: &[u8] = b" libpng version 1.6.59.git\n\0";
pub const PNG_LIBPNG_VER_SHAREDLIB: c_int = 16;
pub const PNG_LIBPNG_VER_MAJOR: c_int = 1;
pub const PNG_LIBPNG_VER_MINOR: c_int = 6;
pub const PNG_LIBPNG_VER_RELEASE: c_int = 59;
pub const PNG_LIBPNG_VER_BUILD: c_int = 1;
pub const PNG_LIBPNG_BUILD_ALPHA: c_int = 1;
pub const PNG_LIBPNG_BUILD_BETA: c_int = 2;
pub const PNG_LIBPNG_BUILD_RC: c_int = 3;
pub const PNG_LIBPNG_BUILD_STABLE: c_int = 4;
pub const PNG_LIBPNG_BUILD_RELEASE_STATUS_MASK: c_int = 7;
pub const PNG_LIBPNG_BUILD_PATCH: c_int = 8;
pub const PNG_LIBPNG_BUILD_PRIVATE: c_int = 16;
pub const PNG_LIBPNG_BUILD_SPECIAL: c_int = 32;
pub const PNG_LIBPNG_BUILD_BASE_TYPE: c_int = PNG_LIBPNG_BUILD_BETA;
pub const PNG_LIBPNG_BUILD_TYPE: c_int = PNG_LIBPNG_BUILD_BASE_TYPE;
pub const PNG_LIBPNG_VER: png_uint_32 = 10659;
pub const PNG_RELEASE_BUILD: bool = PNG_LIBPNG_BUILD_BASE_TYPE >= PNG_LIBPNG_BUILD_RC;

/* ================================================================== */
/* pnglibconf.h settings                                               */
/* ================================================================== */

pub const PNG_API_RULE: c_int = 0;
pub const PNG_DEFAULT_READ_MACROS: c_int = 1;
pub const PNG_GAMMA_THRESHOLD_FIXED: png_fixed_point = 5000;
pub const PNG_ZBUF_SIZE: usize = 8192;
pub const PNG_IDAT_READ_SIZE: usize = PNG_ZBUF_SIZE;
pub const PNG_INFLATE_BUF_SIZE: usize = 1024;
pub const PNG_MAX_GAMMA_8: c_int = 11;
pub const PNG_QUANTIZE_BLUE_BITS: c_int = 5;
pub const PNG_QUANTIZE_GREEN_BITS: c_int = 5;
pub const PNG_QUANTIZE_RED_BITS: c_int = 5;
pub const PNG_TEXT_Z_DEFAULT_COMPRESSION: c_int = -1;
pub const PNG_TEXT_Z_DEFAULT_STRATEGY: c_int = 0;
pub const PNG_USER_CHUNK_CACHE_MAX: png_uint_32 = 1000;
pub const PNG_USER_CHUNK_MALLOC_MAX: png_alloc_size_t = 8000000;
pub const PNG_USER_HEIGHT_MAX: png_uint_32 = 1000000;
pub const PNG_USER_WIDTH_MAX: png_uint_32 = 1000000;
pub const PNG_ZLIB_VERNUM: c_int = 0;
pub const PNG_Z_DEFAULT_COMPRESSION: c_int = -1;
pub const PNG_Z_DEFAULT_NOFILTER_STRATEGY: c_int = 0;
pub const PNG_Z_DEFAULT_STRATEGY: c_int = 1;
pub const PNG_sCAL_PRECISION: c_int = 5;
pub const PNG_sRGB_PROFILE_CHECKS: c_int = 2;

/* ================================================================== */
/* Simple structures                                                   */
/* ================================================================== */

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct png_color {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
}
pub type png_colorp = *mut png_color;
pub type png_const_colorp = *const png_color;
pub type png_colorpp = *mut png_colorp;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct png_color_16 {
    pub index: png_byte,
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub gray: png_uint_16,
}
pub type png_color_16p = *mut png_color_16;
pub type png_const_color_16p = *const png_color_16;
pub type png_color_16pp = *mut png_color_16p;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct png_color_8 {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
    pub gray: png_byte,
    pub alpha: png_byte,
}
pub type png_color_8p = *mut png_color_8;
pub type png_const_color_8p = *const png_color_8;
pub type png_color_8pp = *mut png_color_8p;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct png_sPLT_entry {
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub alpha: png_uint_16,
    pub frequency: png_uint_16,
}
pub type png_sPLT_entryp = *mut png_sPLT_entry;
pub type png_const_sPLT_entryp = *const png_sPLT_entry;
pub type png_sPLT_entrypp = *mut png_sPLT_entryp;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_sPLT_t {
    pub name: png_charp,
    pub depth: png_byte,
    pub entries: png_sPLT_entryp,
    pub nentries: png_int_32,
}
pub type png_sPLT_tp = *mut png_sPLT_t;
pub type png_const_sPLT_tp = *const png_sPLT_t;
pub type png_sPLT_tpp = *mut png_sPLT_tp;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_text {
    pub compression: c_int,
    pub key: png_charp,
    pub text: png_charp,
    pub text_length: usize,
    pub itxt_length: usize,
    pub lang: png_charp,
    pub lang_key: png_charp,
}
pub type png_textp = *mut png_text;
pub type png_const_textp = *const png_text;
pub type png_textpp = *mut png_textp;

pub const PNG_TEXT_COMPRESSION_NONE_WR: c_int = -3;
pub const PNG_TEXT_COMPRESSION_zTXt_WR: c_int = -2;
pub const PNG_TEXT_COMPRESSION_NONE: c_int = -1;
pub const PNG_TEXT_COMPRESSION_zTXt: c_int = 0;
pub const PNG_ITXT_COMPRESSION_NONE: c_int = 1;
pub const PNG_ITXT_COMPRESSION_zTXt: c_int = 2;
pub const PNG_TEXT_COMPRESSION_LAST: c_int = 3;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct png_time {
    pub year: png_uint_16,
    pub month: png_byte,
    pub day: png_byte,
    pub hour: png_byte,
    pub minute: png_byte,
    pub second: png_byte,
}
pub type png_timep = *mut png_time;
pub type png_const_timep = *const png_time;
pub type png_timepp = *mut png_timep;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_unknown_chunk {
    pub name: [png_byte; 5],
    pub data: *mut png_byte,
    pub size: usize,
    pub location: png_byte,
}
pub type png_unknown_chunkp = *mut png_unknown_chunk;
pub type png_const_unknown_chunkp = *const png_unknown_chunk;
pub type png_unknown_chunkpp = *mut png_unknown_chunkp;

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct png_row_info {
    pub width: png_uint_32,
    pub rowbytes: usize,
    pub color_type: png_byte,
    pub bit_depth: png_byte,
    pub channels: png_byte,
    pub pixel_depth: png_byte,
}
pub type png_row_infop = *mut png_row_info;
pub type png_row_infopp = *mut png_row_infop;

/* ================================================================== */
/* Callback types                                                      */
/* ================================================================== */

pub type png_error_ptr = Option<unsafe extern "C" fn(png_structp, png_const_charp)>;
pub type png_rw_ptr = Option<unsafe extern "C" fn(png_structp, png_bytep, usize)>;
pub type png_flush_ptr = Option<unsafe extern "C" fn(png_structp)>;
pub type png_read_status_ptr = Option<unsafe extern "C" fn(png_structp, png_uint_32, c_int)>;
pub type png_write_status_ptr = Option<unsafe extern "C" fn(png_structp, png_uint_32, c_int)>;
pub type png_progressive_info_ptr = Option<unsafe extern "C" fn(png_structp, png_infop)>;
pub type png_progressive_end_ptr = Option<unsafe extern "C" fn(png_structp, png_infop)>;
pub type png_progressive_row_ptr =
    Option<unsafe extern "C" fn(png_structp, png_bytep, png_uint_32, c_int)>;
pub type png_user_transform_ptr =
    Option<unsafe extern "C" fn(png_structp, png_row_infop, png_bytep)>;
pub type png_user_chunk_ptr = Option<unsafe extern "C" fn(png_structp, png_unknown_chunkp) -> c_int>;
pub type png_longjmp_ptr = Option<unsafe extern "C" fn(*mut jmp_buf, c_int)>;
pub type png_malloc_ptr = Option<unsafe extern "C" fn(png_structp, png_alloc_size_t) -> png_voidp>;
pub type png_free_ptr = Option<unsafe extern "C" fn(png_structp, png_voidp)>;

/// The type of the unfiltering routines stored in `png_struct::read_filter`.
pub type png_read_filter_fn =
    Option<unsafe extern "C" fn(png_row_infop, png_bytep, png_const_bytep)>;

/* ================================================================== */
/* png.h constants                                                     */
/* ================================================================== */

pub const PNG_HAVE_IHDR: png_uint_32 = 0x01;
pub const PNG_HAVE_PLTE: png_uint_32 = 0x02;
pub const PNG_AFTER_IDAT: png_uint_32 = 0x08;

pub const PNG_FP_1: c_int = 100000;
pub const PNG_FP_HALF: c_int = 50000;
pub const PNG_FP_MAX: png_fixed_point = 0x7fffffff;
pub const PNG_FP_MIN: png_fixed_point = -PNG_FP_MAX;

pub const PNG_COLOR_MASK_PALETTE: c_int = 1;
pub const PNG_COLOR_MASK_COLOR: c_int = 2;
pub const PNG_COLOR_MASK_ALPHA: c_int = 4;

pub const PNG_COLOR_TYPE_GRAY: c_int = 0;
pub const PNG_COLOR_TYPE_PALETTE: c_int = PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
pub const PNG_COLOR_TYPE_RGB: c_int = PNG_COLOR_MASK_COLOR;
pub const PNG_COLOR_TYPE_RGB_ALPHA: c_int = PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_ALPHA;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: c_int = PNG_COLOR_MASK_ALPHA;
pub const PNG_COLOR_TYPE_RGBA: c_int = PNG_COLOR_TYPE_RGB_ALPHA;
pub const PNG_COLOR_TYPE_GA: c_int = PNG_COLOR_TYPE_GRAY_ALPHA;

pub const PNG_COMPRESSION_TYPE_BASE: c_int = 0;
pub const PNG_COMPRESSION_TYPE_DEFAULT: c_int = PNG_COMPRESSION_TYPE_BASE;

pub const PNG_FILTER_TYPE_BASE: c_int = 0;
pub const PNG_INTRAPIXEL_DIFFERENCING: c_int = 64;
pub const PNG_FILTER_TYPE_DEFAULT: c_int = PNG_FILTER_TYPE_BASE;

pub const PNG_INTERLACE_NONE: c_int = 0;
pub const PNG_INTERLACE_ADAM7: c_int = 1;
pub const PNG_INTERLACE_LAST: c_int = 2;

pub const PNG_OFFSET_PIXEL: c_int = 0;
pub const PNG_OFFSET_MICROMETER: c_int = 1;
pub const PNG_OFFSET_LAST: c_int = 2;

pub const PNG_EQUATION_LINEAR: c_int = 0;
pub const PNG_EQUATION_BASE_E: c_int = 1;
pub const PNG_EQUATION_ARBITRARY: c_int = 2;
pub const PNG_EQUATION_HYPERBOLIC: c_int = 3;
pub const PNG_EQUATION_LAST: c_int = 4;

pub const PNG_SCALE_UNKNOWN: c_int = 0;
pub const PNG_SCALE_METER: c_int = 1;
pub const PNG_SCALE_RADIAN: c_int = 2;
pub const PNG_SCALE_LAST: c_int = 3;

pub const PNG_RESOLUTION_UNKNOWN: c_int = 0;
pub const PNG_RESOLUTION_METER: c_int = 1;
pub const PNG_RESOLUTION_LAST: c_int = 2;

pub const PNG_sRGB_INTENT_PERCEPTUAL: c_int = 0;
pub const PNG_sRGB_INTENT_RELATIVE: c_int = 1;
pub const PNG_sRGB_INTENT_SATURATION: c_int = 2;
pub const PNG_sRGB_INTENT_ABSOLUTE: c_int = 3;
pub const PNG_sRGB_INTENT_LAST: c_int = 4;

pub const PNG_KEYWORD_MAX_LENGTH: c_int = 79;
pub const PNG_MAX_PALETTE_LENGTH: c_int = 256;

pub const PNG_INFO_gAMA: png_uint_32 = 0x0001;
pub const PNG_INFO_sBIT: png_uint_32 = 0x0002;
pub const PNG_INFO_cHRM: png_uint_32 = 0x0004;
pub const PNG_INFO_PLTE: png_uint_32 = 0x0008;
pub const PNG_INFO_tRNS: png_uint_32 = 0x0010;
pub const PNG_INFO_bKGD: png_uint_32 = 0x0020;
pub const PNG_INFO_hIST: png_uint_32 = 0x0040;
pub const PNG_INFO_pHYs: png_uint_32 = 0x0080;
pub const PNG_INFO_oFFs: png_uint_32 = 0x0100;
pub const PNG_INFO_tIME: png_uint_32 = 0x0200;
pub const PNG_INFO_pCAL: png_uint_32 = 0x0400;
pub const PNG_INFO_sRGB: png_uint_32 = 0x0800;
pub const PNG_INFO_iCCP: png_uint_32 = 0x1000;
pub const PNG_INFO_sPLT: png_uint_32 = 0x2000;
pub const PNG_INFO_sCAL: png_uint_32 = 0x4000;
pub const PNG_INFO_IDAT: png_uint_32 = 0x8000;
pub const PNG_INFO_eXIf: png_uint_32 = 0x10000;
pub const PNG_INFO_cICP: png_uint_32 = 0x20000;
pub const PNG_INFO_cLLI: png_uint_32 = 0x40000;
pub const PNG_INFO_mDCV: png_uint_32 = 0x80000;
pub const PNG_INFO_acTL: png_uint_32 = 0x100000;
pub const PNG_INFO_fcTL: png_uint_32 = 0x200000;
pub const PNG_INFO_fdAT: png_uint_32 = 0x400000;

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
pub const PNG_TRANSFORM_STRIP_FILLER_BEFORE: c_int = PNG_TRANSFORM_STRIP_FILLER;
pub const PNG_TRANSFORM_STRIP_FILLER_AFTER: c_int = 0x1000;
pub const PNG_TRANSFORM_GRAY_TO_RGB: c_int = 0x2000;
pub const PNG_TRANSFORM_EXPAND_16: c_int = 0x4000;
pub const PNG_TRANSFORM_SCALE_16: c_int = 0x8000;

pub const PNG_FLAG_MNG_EMPTY_PLTE: png_uint_32 = 0x01;
pub const PNG_FLAG_MNG_FILTER_64: png_uint_32 = 0x04;
pub const PNG_ALL_MNG_FEATURES: png_uint_32 = 0x05;

pub const PNG_ERROR_ACTION_NONE: c_int = 1;
pub const PNG_ERROR_ACTION_WARN: c_int = 2;
pub const PNG_ERROR_ACTION_ERROR: c_int = 3;
pub const PNG_RGB_TO_GRAY_DEFAULT: c_int = -1;

pub const PNG_ALPHA_PNG: c_int = 0;
pub const PNG_ALPHA_STANDARD: c_int = 1;
pub const PNG_ALPHA_ASSOCIATED: c_int = 1;
pub const PNG_ALPHA_PREMULTIPLIED: c_int = 1;
pub const PNG_ALPHA_OPTIMIZED: c_int = 2;
pub const PNG_ALPHA_BROKEN: c_int = 3;

pub const PNG_DEFAULT_sRGB: png_fixed_point = -1;
pub const PNG_GAMMA_MAC_18: png_fixed_point = -2;
pub const PNG_GAMMA_sRGB: png_fixed_point = 220000;
pub const PNG_GAMMA_LINEAR: png_fixed_point = PNG_FP_1;

pub const PNG_FILLER_BEFORE: c_int = 0;
pub const PNG_FILLER_AFTER: c_int = 1;

pub const PNG_BACKGROUND_GAMMA_UNKNOWN: c_int = 0;
pub const PNG_BACKGROUND_GAMMA_SCREEN: c_int = 1;
pub const PNG_BACKGROUND_GAMMA_FILE: c_int = 2;
pub const PNG_BACKGROUND_GAMMA_UNIQUE: c_int = 3;

pub const PNG_CRC_DEFAULT: c_int = 0;
pub const PNG_CRC_ERROR_QUIT: c_int = 1;
pub const PNG_CRC_WARN_DISCARD: c_int = 2;
pub const PNG_CRC_WARN_USE: c_int = 3;
pub const PNG_CRC_QUIET_USE: c_int = 4;
pub const PNG_CRC_NO_CHANGE: c_int = 5;

pub const PNG_NO_FILTERS: c_int = 0x00;
pub const PNG_FILTER_NONE: c_int = 0x08;
pub const PNG_FILTER_SUB: c_int = 0x10;
pub const PNG_FILTER_UP: c_int = 0x20;
pub const PNG_FILTER_AVG: c_int = 0x40;
pub const PNG_FILTER_PAETH: c_int = 0x80;
pub const PNG_FAST_FILTERS: c_int = PNG_FILTER_NONE | PNG_FILTER_SUB | PNG_FILTER_UP;
pub const PNG_ALL_FILTERS: c_int = PNG_FAST_FILTERS | PNG_FILTER_AVG | PNG_FILTER_PAETH;

pub const PNG_FILTER_VALUE_NONE: c_int = 0;
pub const PNG_FILTER_VALUE_SUB: c_int = 1;
pub const PNG_FILTER_VALUE_UP: c_int = 2;
pub const PNG_FILTER_VALUE_AVG: c_int = 3;
pub const PNG_FILTER_VALUE_PAETH: c_int = 4;
pub const PNG_FILTER_VALUE_LAST: c_int = 5;

pub const PNG_FILTER_HEURISTIC_DEFAULT: c_int = 0;
pub const PNG_FILTER_HEURISTIC_UNWEIGHTED: c_int = 1;
pub const PNG_FILTER_HEURISTIC_WEIGHTED: c_int = 2;
pub const PNG_FILTER_HEURISTIC_LAST: c_int = 3;

pub const PNG_DESTROY_WILL_FREE_DATA: c_int = 1;
pub const PNG_SET_WILL_FREE_DATA: c_int = 1;
pub const PNG_USER_WILL_FREE_DATA: c_int = 2;

pub const PNG_FREE_HIST: png_uint_32 = 0x0008;
pub const PNG_FREE_ICCP: png_uint_32 = 0x0010;
pub const PNG_FREE_SPLT: png_uint_32 = 0x0020;
pub const PNG_FREE_ROWS: png_uint_32 = 0x0040;
pub const PNG_FREE_PCAL: png_uint_32 = 0x0080;
pub const PNG_FREE_SCAL: png_uint_32 = 0x0100;
pub const PNG_FREE_UNKN: png_uint_32 = 0x0200;
pub const PNG_FREE_PLTE: png_uint_32 = 0x1000;
pub const PNG_FREE_TRNS: png_uint_32 = 0x2000;
pub const PNG_FREE_TEXT: png_uint_32 = 0x4000;
pub const PNG_FREE_EXIF: png_uint_32 = 0x8000;
pub const PNG_FREE_ALL: png_uint_32 = 0xffff;
pub const PNG_FREE_MUL: png_uint_32 = 0x4220;

pub const PNG_HANDLE_CHUNK_AS_DEFAULT: c_int = 0;
pub const PNG_HANDLE_CHUNK_NEVER: c_int = 1;
pub const PNG_HANDLE_CHUNK_IF_SAFE: c_int = 2;
pub const PNG_HANDLE_CHUNK_ALWAYS: c_int = 3;
pub const PNG_HANDLE_CHUNK_LAST: c_int = 4;

pub const PNG_IO_NONE: png_uint_32 = 0x0000;
pub const PNG_IO_READING: png_uint_32 = 0x0001;
pub const PNG_IO_WRITING: png_uint_32 = 0x0002;
pub const PNG_IO_SIGNATURE: png_uint_32 = 0x0010;
pub const PNG_IO_CHUNK_HDR: png_uint_32 = 0x0020;
pub const PNG_IO_CHUNK_DATA: png_uint_32 = 0x0040;
pub const PNG_IO_CHUNK_CRC: png_uint_32 = 0x0080;
pub const PNG_IO_MASK_OP: png_uint_32 = 0x000f;
pub const PNG_IO_MASK_LOC: png_uint_32 = 0x00f0;

pub const PNG_INTERLACE_ADAM7_PASSES: c_int = 7;

pub const PNG_IMAGE_VERSION: png_uint_32 = 1;
pub const PNG_IMAGE_WARNING: png_uint_32 = 1;
pub const PNG_IMAGE_ERROR: png_uint_32 = 2;

pub const PNG_FORMAT_FLAG_ALPHA: png_uint_32 = 0x01;
pub const PNG_FORMAT_FLAG_COLOR: png_uint_32 = 0x02;
pub const PNG_FORMAT_FLAG_LINEAR: png_uint_32 = 0x04;
pub const PNG_FORMAT_FLAG_COLORMAP: png_uint_32 = 0x08;
pub const PNG_FORMAT_FLAG_BGR: png_uint_32 = 0x10;
pub const PNG_FORMAT_FLAG_AFIRST: png_uint_32 = 0x20;
pub const PNG_FORMAT_FLAG_ASSOCIATED_ALPHA: png_uint_32 = 0x40;

pub const PNG_FORMAT_GRAY: png_uint_32 = 0;
pub const PNG_FORMAT_GA: png_uint_32 = PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_AG: png_uint_32 = PNG_FORMAT_GA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_RGB: png_uint_32 = PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_BGR: png_uint_32 = PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_BGR;
pub const PNG_FORMAT_RGBA: png_uint_32 = PNG_FORMAT_RGB | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_ARGB: png_uint_32 = PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_BGRA: png_uint_32 = PNG_FORMAT_BGR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_ABGR: png_uint_32 = PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_LINEAR_Y: png_uint_32 = PNG_FORMAT_FLAG_LINEAR;
pub const PNG_FORMAT_LINEAR_Y_ALPHA: png_uint_32 =
    PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_LINEAR_RGB: png_uint_32 = PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_LINEAR_RGB_ALPHA: png_uint_32 =
    PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_RGB_COLORMAP: png_uint_32 = PNG_FORMAT_RGB | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_BGR_COLORMAP: png_uint_32 = PNG_FORMAT_BGR | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_RGBA_COLORMAP: png_uint_32 = PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_ARGB_COLORMAP: png_uint_32 = PNG_FORMAT_ARGB | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_BGRA_COLORMAP: png_uint_32 = PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_ABGR_COLORMAP: png_uint_32 = PNG_FORMAT_ABGR | PNG_FORMAT_FLAG_COLORMAP;

pub const PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB: png_uint_32 = 0x01;
pub const PNG_IMAGE_FLAG_FAST: png_uint_32 = 0x02;
pub const PNG_IMAGE_FLAG_16BIT_sRGB: png_uint_32 = 0x04;

pub const PNG_MAXIMUM_INFLATE_WINDOW: c_int = 2;
pub const PNG_SKIP_sRGB_CHECK_PROFILE: c_int = 4;
pub const PNG_OPTION_NEXT: c_int = 16;
pub const PNG_OPTION_UNSET: c_int = 0;
pub const PNG_OPTION_INVALID: c_int = 1;
pub const PNG_OPTION_OFF: c_int = 2;
pub const PNG_OPTION_ON: c_int = 3;

/* ================================================================== */
/* pngpriv.h constants                                                 */
/* ================================================================== */

pub const PNG_HAVE_IDAT: png_uint_32 = 0x004;
pub const PNG_HAVE_IEND: png_uint_32 = 0x010;
pub const PNG_HAVE_CHUNK_HEADER: png_uint_32 = 0x100;
pub const PNG_WROTE_tIME: png_uint_32 = 0x200;
pub const PNG_WROTE_INFO_BEFORE_PLTE: png_uint_32 = 0x400;
pub const PNG_BACKGROUND_IS_GRAY: png_uint_32 = 0x800;
pub const PNG_HAVE_PNG_SIGNATURE: png_uint_32 = 0x1000;
pub const PNG_HAVE_CHUNK_AFTER_IDAT: png_uint_32 = 0x2000;
pub const PNG_WROTE_eXIf: png_uint_32 = 0x4000;
pub const PNG_IS_READ_STRUCT: png_uint_32 = 0x8000;

pub const PNG_BGR: png_uint_32 = 0x0001;
pub const PNG_INTERLACE: png_uint_32 = 0x0002;
pub const PNG_PACK: png_uint_32 = 0x0004;
pub const PNG_SHIFT: png_uint_32 = 0x0008;
pub const PNG_SWAP_BYTES: png_uint_32 = 0x0010;
pub const PNG_INVERT_MONO: png_uint_32 = 0x0020;
pub const PNG_QUANTIZE: png_uint_32 = 0x0040;
pub const PNG_COMPOSE: png_uint_32 = 0x0080;
pub const PNG_BACKGROUND_EXPAND: png_uint_32 = 0x0100;
pub const PNG_EXPAND_16: png_uint_32 = 0x0200;
pub const PNG_16_TO_8: png_uint_32 = 0x0400;
pub const PNG_RGBA: png_uint_32 = 0x0800;
pub const PNG_EXPAND: png_uint_32 = 0x1000;
pub const PNG_GAMMA: png_uint_32 = 0x2000;
pub const PNG_GRAY_TO_RGB: png_uint_32 = 0x4000;
pub const PNG_FILLER: png_uint_32 = 0x8000;
pub const PNG_PACKSWAP: png_uint_32 = 0x10000;
pub const PNG_SWAP_ALPHA: png_uint_32 = 0x20000;
pub const PNG_STRIP_ALPHA: png_uint_32 = 0x40000;
pub const PNG_INVERT_ALPHA: png_uint_32 = 0x80000;
pub const PNG_USER_TRANSFORM: png_uint_32 = 0x100000;
pub const PNG_RGB_TO_GRAY_ERR: png_uint_32 = 0x200000;
pub const PNG_RGB_TO_GRAY_WARN: png_uint_32 = 0x400000;
pub const PNG_RGB_TO_GRAY: png_uint_32 = 0x600000;
pub const PNG_ENCODE_ALPHA: png_uint_32 = 0x800000;
pub const PNG_ADD_ALPHA: png_uint_32 = 0x1000000;
pub const PNG_EXPAND_tRNS: png_uint_32 = 0x2000000;
pub const PNG_SCALE_16_TO_8: png_uint_32 = 0x4000000;

pub const PNG_STRUCT_PNG: png_uint_32 = 0x0001;
pub const PNG_STRUCT_INFO: png_uint_32 = 0x0002;

pub const PNG_FLAG_ZLIB_CUSTOM_STRATEGY: png_uint_32 = 0x0001;
pub const PNG_FLAG_ZSTREAM_INITIALIZED: png_uint_32 = 0x0002;
pub const PNG_FLAG_ZSTREAM_ENDED: png_uint_32 = 0x0008;
pub const PNG_FLAG_ROW_INIT: png_uint_32 = 0x0040;
pub const PNG_FLAG_FILLER_AFTER: png_uint_32 = 0x0080;
pub const PNG_FLAG_CRC_ANCILLARY_USE: png_uint_32 = 0x0100;
pub const PNG_FLAG_CRC_ANCILLARY_NOWARN: png_uint_32 = 0x0200;
pub const PNG_FLAG_CRC_CRITICAL_USE: png_uint_32 = 0x0400;
pub const PNG_FLAG_CRC_CRITICAL_IGNORE: png_uint_32 = 0x0800;
pub const PNG_FLAG_OPTIMIZE_ALPHA: png_uint_32 = 0x2000;
pub const PNG_FLAG_DETECT_UNINITIALIZED: png_uint_32 = 0x4000;
pub const PNG_FLAG_LIBRARY_MISMATCH: png_uint_32 = 0x20000;
pub const PNG_FLAG_STRIP_ERROR_TEXT: png_uint_32 = 0x80000;
pub const PNG_FLAG_BENIGN_ERRORS_WARN: png_uint_32 = 0x100000;
pub const PNG_FLAG_APP_WARNINGS_WARN: png_uint_32 = 0x200000;
pub const PNG_FLAG_APP_ERRORS_WARN: png_uint_32 = 0x400000;

pub const PNG_FLAG_CRC_ANCILLARY_MASK: png_uint_32 =
    PNG_FLAG_CRC_ANCILLARY_USE | PNG_FLAG_CRC_ANCILLARY_NOWARN;
pub const PNG_FLAG_CRC_CRITICAL_MASK: png_uint_32 =
    PNG_FLAG_CRC_CRITICAL_USE | PNG_FLAG_CRC_CRITICAL_IGNORE;
pub const PNG_FLAG_CRC_MASK: png_uint_32 =
    PNG_FLAG_CRC_ANCILLARY_MASK | PNG_FLAG_CRC_CRITICAL_MASK;

pub const PNG_GAMMA_MAC_OLD: png_fixed_point = 151724;
pub const PNG_GAMMA_MAC_INVERSE: png_fixed_point = 65909;
pub const PNG_GAMMA_sRGB_INVERSE: png_fixed_point = 45455;
pub const PNG_LIB_GAMMA_MIN: png_fixed_point = 1000;
pub const PNG_LIB_GAMMA_MAX: png_fixed_point = 10000000;

pub const PNG_UNEXPECTED_ZLIB_RETURN: c_int = -7;

pub const PNG_CHUNK_WARNING: c_int = 0;
pub const PNG_CHUNK_WRITE_ERROR: c_int = 1;
pub const PNG_CHUNK_ERROR: c_int = 2;

pub const PNG_NUMBER_FORMAT_u: c_int = 1;
pub const PNG_NUMBER_FORMAT_02u: c_int = 2;
pub const PNG_NUMBER_FORMAT_d: c_int = 1;
pub const PNG_NUMBER_FORMAT_02d: c_int = 2;
pub const PNG_NUMBER_FORMAT_x: c_int = 3;
pub const PNG_NUMBER_FORMAT_02x: c_int = 4;
pub const PNG_NUMBER_FORMAT_fixed: c_int = 5;
pub const PNG_NUMBER_BUFFER_SIZE: usize = 24;

pub const PNG_WARNING_PARAMETER_SIZE: usize = 32;
pub const PNG_WARNING_PARAMETER_COUNT: usize = 8;
/// `char [8][32]`
pub type png_warning_parameters = [[c_char; PNG_WARNING_PARAMETER_SIZE]; PNG_WARNING_PARAMETER_COUNT];

pub const PNG_FP_INTEGER: c_int = 0;
pub const PNG_FP_FRACTION: c_int = 1;
pub const PNG_FP_EXPONENT: c_int = 2;
pub const PNG_FP_STATE: c_int = 3;
pub const PNG_FP_SAW_SIGN: c_int = 4;
pub const PNG_FP_SAW_DIGIT: c_int = 8;
pub const PNG_FP_SAW_DOT: c_int = 16;
pub const PNG_FP_SAW_E: c_int = 32;
pub const PNG_FP_SAW_ANY: c_int = 60;
pub const PNG_FP_WAS_VALID: c_int = 64;
pub const PNG_FP_NEGATIVE: c_int = 128;
pub const PNG_FP_NONZERO: c_int = 256;
pub const PNG_FP_STICKY: c_int = 448;
pub const PNG_FP_INVALID: c_int = 512;
pub const PNG_FP_MAYBE: c_int = 0;
pub const PNG_FP_OK: c_int = 1;
pub const PNG_FP_NZ_MASK: c_int = PNG_FP_SAW_DIGIT | PNG_FP_NEGATIVE | PNG_FP_NONZERO;
pub const PNG_FP_Z_MASK: c_int = PNG_FP_SAW_DIGIT | PNG_FP_NONZERO;

pub const PNG_sCAL_MAX_DIGITS: usize = (PNG_sCAL_PRECISION as usize) + 1 + 1 + 10;

pub const PNG_ALIGN_NONE: c_int = 0;
pub const PNG_ALIGN_ALWAYS: c_int = 1;
pub const PNG_ALIGN_OFFSET: c_int = 2;
pub const PNG_ALIGN_SIZE: c_int = 3;
pub const PNG_ALIGN_TYPE: c_int = PNG_ALIGN_SIZE;

pub const PNG_USE_COMPILE_TIME_MASKS: c_int = 1;

/* Chunk type constants -------------------------------------------- */

#[inline(always)]
pub const fn PNG_U32(b1: u32, b2: u32, b3: u32, b4: u32) -> png_uint_32 {
    (b1 << 24) | (b2 << 16) | (b3 << 8) | b4
}

pub const png_IDAT: png_uint_32 = PNG_U32(73, 68, 65, 84);
pub const png_IEND: png_uint_32 = PNG_U32(73, 69, 78, 68);
pub const png_IHDR: png_uint_32 = PNG_U32(73, 72, 68, 82);
pub const png_PLTE: png_uint_32 = PNG_U32(80, 76, 84, 69);
pub const png_acTL: png_uint_32 = PNG_U32(97, 99, 84, 76);
pub const png_bKGD: png_uint_32 = PNG_U32(98, 75, 71, 68);
pub const png_cHRM: png_uint_32 = PNG_U32(99, 72, 82, 77);
pub const png_cICP: png_uint_32 = PNG_U32(99, 73, 67, 80);
pub const png_cLLI: png_uint_32 = PNG_U32(99, 76, 76, 73);
pub const png_eXIf: png_uint_32 = PNG_U32(101, 88, 73, 102);
pub const png_fcTL: png_uint_32 = PNG_U32(102, 99, 84, 76);
pub const png_fdAT: png_uint_32 = PNG_U32(102, 100, 65, 84);
pub const png_fRAc: png_uint_32 = PNG_U32(102, 82, 65, 99);
pub const png_gAMA: png_uint_32 = PNG_U32(103, 65, 77, 65);
pub const png_gIFg: png_uint_32 = PNG_U32(103, 73, 70, 103);
pub const png_gIFt: png_uint_32 = PNG_U32(103, 73, 70, 116);
pub const png_gIFx: png_uint_32 = PNG_U32(103, 73, 70, 120);
pub const png_hIST: png_uint_32 = PNG_U32(104, 73, 83, 84);
pub const png_iCCP: png_uint_32 = PNG_U32(105, 67, 67, 80);
pub const png_iTXt: png_uint_32 = PNG_U32(105, 84, 88, 116);
pub const png_mDCV: png_uint_32 = PNG_U32(109, 68, 67, 86);
pub const png_oFFs: png_uint_32 = PNG_U32(111, 70, 70, 115);
pub const png_pCAL: png_uint_32 = PNG_U32(112, 67, 65, 76);
pub const png_pHYs: png_uint_32 = PNG_U32(112, 72, 89, 115);
pub const png_sBIT: png_uint_32 = PNG_U32(115, 66, 73, 84);
pub const png_sCAL: png_uint_32 = PNG_U32(115, 67, 65, 76);
pub const png_sPLT: png_uint_32 = PNG_U32(115, 80, 76, 84);
pub const png_sRGB: png_uint_32 = PNG_U32(115, 82, 71, 66);
pub const png_sTER: png_uint_32 = PNG_U32(115, 84, 69, 82);
pub const png_tEXt: png_uint_32 = PNG_U32(116, 69, 88, 116);
pub const png_tIME: png_uint_32 = PNG_U32(116, 73, 77, 69);
pub const png_tRNS: png_uint_32 = PNG_U32(116, 82, 78, 83);
pub const png_zTXt: png_uint_32 = PNG_U32(122, 84, 88, 116);

/* Chunk indices (pngstruct.h `png_index`) -------------------------- */

pub const PNG_INDEX_IHDR: c_int = 0;
pub const PNG_INDEX_PLTE: c_int = 1;
pub const PNG_INDEX_IDAT: c_int = 2;
pub const PNG_INDEX_IEND: c_int = 3;
pub const PNG_INDEX_acTL: c_int = 4;
pub const PNG_INDEX_bKGD: c_int = 5;
pub const PNG_INDEX_cHRM: c_int = 6;
pub const PNG_INDEX_cICP: c_int = 7;
pub const PNG_INDEX_cLLI: c_int = 8;
pub const PNG_INDEX_eXIf: c_int = 9;
pub const PNG_INDEX_fcTL: c_int = 10;
pub const PNG_INDEX_fdAT: c_int = 11;
pub const PNG_INDEX_gAMA: c_int = 12;
pub const PNG_INDEX_hIST: c_int = 13;
pub const PNG_INDEX_iCCP: c_int = 14;
pub const PNG_INDEX_iTXt: c_int = 15;
pub const PNG_INDEX_mDCV: c_int = 16;
pub const PNG_INDEX_oFFs: c_int = 17;
pub const PNG_INDEX_pCAL: c_int = 18;
pub const PNG_INDEX_pHYs: c_int = 19;
pub const PNG_INDEX_sBIT: c_int = 20;
pub const PNG_INDEX_sCAL: c_int = 21;
pub const PNG_INDEX_sPLT: c_int = 22;
pub const PNG_INDEX_sRGB: c_int = 23;
pub const PNG_INDEX_tEXt: c_int = 24;
pub const PNG_INDEX_tIME: c_int = 25;
pub const PNG_INDEX_tRNS: c_int = 26;
pub const PNG_INDEX_zTXt: c_int = 27;
pub const PNG_INDEX_unknown: c_int = 28;

/// `png_handle_result_code`
pub const handled_error: c_int = 0;
pub const handled_discarded: c_int = 1;
pub const handled_saved: c_int = 2;
pub const handled_ok: c_int = 3;
pub type png_handle_result_code = c_int;

/* ================================================================== */
/* png_xy / png_XYZ / compression buffer                               */
/* ================================================================== */

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct png_xy {
    pub redx: png_fixed_point,
    pub redy: png_fixed_point,
    pub greenx: png_fixed_point,
    pub greeny: png_fixed_point,
    pub bluex: png_fixed_point,
    pub bluey: png_fixed_point,
    pub whitex: png_fixed_point,
    pub whitey: png_fixed_point,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct png_XYZ {
    pub red_X: png_fixed_point,
    pub red_Y: png_fixed_point,
    pub red_Z: png_fixed_point,
    pub green_X: png_fixed_point,
    pub green_Y: png_fixed_point,
    pub green_Z: png_fixed_point,
    pub blue_X: png_fixed_point,
    pub blue_Y: png_fixed_point,
    pub blue_Z: png_fixed_point,
}

#[repr(C)]
pub struct png_compression_buffer {
    pub next: *mut png_compression_buffer,
    pub output: [png_byte; 1],
}
pub type png_compression_bufferp = *mut png_compression_buffer;

/// `offsetof(png_compression_buffer, output) + (pp)->zbuffer_size`
#[inline(always)]
pub unsafe fn PNG_COMPRESSION_BUFFER_SIZE(pp: png_const_structrp) -> usize {
    core::mem::offset_of!(png_compression_buffer, output) + (*pp).zbuffer_size as usize
}

/* ================================================================== */
/* png_struct                                                          */
/* ================================================================== */

pub type png_struct = png_struct_def;
pub type png_structp = *mut png_struct;
pub type png_structrp = *mut png_struct;
/// libpng's `const png_struct *`; represented as a mutable pointer because
/// `const` has no effect on the C ABI and libpng itself casts the `const`
/// away in a number of places.
pub type png_const_structp = *mut png_struct;
pub type png_const_structrp = *mut png_struct;
pub type png_structpp = *mut png_structp;

pub type png_info = png_info_def;
pub type png_infop = *mut png_info;
pub type png_inforp = *mut png_info;
pub type png_const_infop = *mut png_info;
pub type png_const_inforp = *mut png_info;
pub type png_infopp = *mut png_infop;

#[repr(C)]
pub struct png_struct_def {
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

    pub zbuffer_list: png_compression_bufferp,
    pub zbuffer_size: uInt,

    pub zlib_level: c_int,
    pub zlib_method: c_int,
    pub zlib_window_bits: c_int,
    pub zlib_mem_level: c_int,
    pub zlib_strategy: c_int,

    pub zlib_text_level: c_int,
    pub zlib_text_method: c_int,
    pub zlib_text_window_bits: c_int,
    pub zlib_text_mem_level: c_int,
    pub zlib_text_strategy: c_int,

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
    pub try_row: png_bytep,
    pub tst_row: png_bytep,
    pub info_rowbytes: usize,

    pub idat_size: png_uint_32,
    pub crc: png_uint_32,
    pub palette: png_colorp,
    pub num_palette: png_uint_16,

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
    pub zstream_start: png_byte,
    pub filler: png_uint_16,

    pub background_gamma_type: png_byte,
    pub background_gamma: png_fixed_point,
    pub background: png_color_16,
    pub background_1: png_color_16,

    pub output_flush_fn: png_flush_ptr,
    pub flush_dist: png_uint_32,
    pub flush_rows: png_uint_32,

    pub chromaticities: png_xy,

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

    pub sig_bit: png_color_8,
    pub shift: png_color_8,

    pub trans_alpha: png_bytep,
    pub trans_color: png_color_16,

    pub read_row_fn: png_read_status_ptr,
    pub write_row_fn: png_write_status_ptr,

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

    pub palette_lookup: png_bytep,
    pub quantize_index: png_bytep,

    pub options: png_uint_32,

    pub time_buffer: [c_char; 29],

    pub free_me: png_uint_32,

    pub user_chunk_ptr: png_voidp,
    pub read_user_chunk_fn: png_user_chunk_ptr,

    pub unknown_default: c_int,
    pub num_chunk_list: c_uint,
    pub chunk_list: png_bytep,

    pub rgb_to_gray_status: png_byte,
    pub rgb_to_gray_coefficients_set: png_byte,
    pub rgb_to_gray_red_coeff: png_uint_16,
    pub rgb_to_gray_green_coeff: png_uint_16,

    /* New member added in libpng-1.6.36.  Present whenever
     * PNG_READ_EXPAND_SUPPORTED is defined and PNG_ARM_NEON_IMPLEMENTATION or
     * PNG_RISCV_RVV_IMPLEMENTATION is *defined* -- pngpriv.h always defines
     * PNG_ARM_NEON_IMPLEMENTATION (as 0 here), so the field exists even though
     * the code that uses it is compiled out. */
    pub riffled_palette: png_bytep,

    pub mng_features_permitted: png_uint_32,
    pub filter_type: png_byte,

    pub mem_ptr: png_voidp,
    pub malloc_fn: png_malloc_ptr,
    pub free_fn: png_free_ptr,

    pub big_row_buf: png_bytep,

    pub index_to_palette: png_bytep,
    pub palette_to_index: png_bytep,

    pub compression_type: png_byte,

    pub user_width_max: png_uint_32,
    pub user_height_max: png_uint_32,
    pub user_chunk_cache_max: png_uint_32,
    pub user_chunk_malloc_max: png_alloc_size_t,

    pub unknown_chunk: png_unknown_chunk,

    pub old_big_row_buf_size: usize,

    pub read_buffer: png_bytep,
    pub read_buffer_size: png_alloc_size_t,
    pub IDAT_read_size: uInt,

    pub io_state: png_uint_32,

    pub big_prev_row: png_bytep,

    pub read_filter: [png_read_filter_fn; (PNG_FILTER_VALUE_LAST - 1) as usize],
}

/* ================================================================== */
/* png_info                                                            */
/* ================================================================== */

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

    pub signature: [png_byte; 8],

    pub cicp_colour_primaries: png_byte,
    pub cicp_transfer_function: png_byte,
    pub cicp_matrix_coefficients: png_byte,
    pub cicp_video_full_range_flag: png_byte,

    pub iccp_name: png_charp,
    pub iccp_profile: png_bytep,
    pub iccp_proflen: png_uint_32,

    pub maxCLL: png_uint_32,
    pub maxFALL: png_uint_32,

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

    pub num_text: c_int,
    pub max_text: c_int,
    pub text: png_textp,

    pub mod_time: png_time,

    pub sig_bit: png_color_8,

    pub trans_alpha: png_bytep,
    pub trans_color: png_color_16,

    pub background: png_color_16,

    pub x_offset: png_int_32,
    pub y_offset: png_int_32,
    pub offset_unit_type: png_byte,

    pub x_pixels_per_unit: png_uint_32,
    pub y_pixels_per_unit: png_uint_32,
    pub phys_unit_type: png_byte,

    pub num_exif: png_uint_32,
    pub exif: png_bytep,

    pub hist: png_uint_16p,

    pub pcal_purpose: png_charp,
    pub pcal_X0: png_int_32,
    pub pcal_X1: png_int_32,
    pub pcal_units: png_charp,
    pub pcal_params: png_charpp,
    pub pcal_type: png_byte,
    pub pcal_nparams: png_byte,

    pub free_me: png_uint_32,

    pub unknown_chunks: png_unknown_chunkp,
    pub unknown_chunks_num: c_int,

    pub splt_palettes: png_sPLT_tp,
    pub splt_palettes_num: c_int,

    pub scal_unit: png_byte,
    pub scal_s_width: png_charp,
    pub scal_s_height: png_charp,

    pub row_pointers: png_bytepp,

    pub cHRM: png_xy,

    pub gamma: png_fixed_point,

    pub rendering_intent: c_int,
}

/* ================================================================== */
/* Simplified API                                                      */
/* ================================================================== */

#[repr(C)]
pub struct png_control {
    pub png_ptr: png_structp,
    pub info_ptr: png_infop,
    pub error_buf: png_voidp,
    pub memory: png_const_bytep,
    pub size: usize,
    /// bitfields: `for_write:1`, `owned_file:1`
    pub bitfields: c_uint,
}
pub type png_controlp = *mut png_control;

impl png_control {
    #[inline]
    pub fn for_write(&self) -> c_uint {
        self.bitfields & 1
    }
    #[inline]
    pub fn set_for_write(&mut self, v: c_uint) {
        self.bitfields = (self.bitfields & !1) | (v & 1);
    }
    #[inline]
    pub fn owned_file(&self) -> c_uint {
        (self.bitfields >> 1) & 1
    }
    #[inline]
    pub fn set_owned_file(&mut self, v: c_uint) {
        self.bitfields = (self.bitfields & !2) | ((v & 1) << 1);
    }
}

#[repr(C)]
pub struct png_image {
    pub opaque: png_controlp,
    pub version: png_uint_32,
    pub width: png_uint_32,
    pub height: png_uint_32,
    pub format: png_uint_32,
    pub flags: png_uint_32,
    pub colormap_entries: png_uint_32,
    pub warning_or_error: png_uint_32,
    pub message: [c_char; 64],
}
pub type png_imagep = *mut png_image;
