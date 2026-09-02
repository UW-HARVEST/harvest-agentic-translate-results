//! All libpng compile-time constants (png.h, pngpriv.h, pnglibconf.h).
#![allow(non_upper_case_globals)]

use core::ffi::c_int;

use crate::types::{png_alloc_size_t, png_fixed_point, png_uint_32};

/* ---------------- version (png.h) ---------------- */

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
pub const PNG_LIBPNG_VER: png_uint_32 = 10659;

/// `PNG_RELEASE_BUILD` == (PNG_LIBPNG_BUILD_BASE_TYPE >= PNG_LIBPNG_BUILD_RC)
pub const PNG_RELEASE_BUILD: bool = PNG_LIBPNG_BUILD_BASE_TYPE >= PNG_LIBPNG_BUILD_RC;

/* ---------------- pnglibconf.h settings ---------------- */

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
pub const PNG_Z_DEFAULT_COMPRESSION: c_int = -1;
pub const PNG_Z_DEFAULT_NOFILTER_STRATEGY: c_int = 0;
pub const PNG_Z_DEFAULT_STRATEGY: c_int = 1;
pub const PNG_sCAL_PRECISION: usize = 5;
pub const PNG_sRGB_PROFILE_CHECKS: c_int = 2;
pub const PNG_ZLIB_VERNUM: c_int = 0;

/* ---------------- limits (png.h) ---------------- */

pub const PNG_UINT_31_MAX: png_uint_32 = 0x7fffffff;
pub const PNG_UINT_32_MAX: png_uint_32 = 0xffffffff;
pub const PNG_SIZE_MAX: usize = usize::MAX;

pub const PNG_FP_1: png_fixed_point = 100000;
pub const PNG_FP_HALF: png_fixed_point = 50000;
pub const PNG_FP_MAX: png_fixed_point = 0x7fffffff;
pub const PNG_FP_MIN: png_fixed_point = -PNG_FP_MAX;

/* ---------------- color types ---------------- */

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

pub const PNG_KEYWORD_MAX_LENGTH: png_uint_32 = 79;
pub const PNG_MAX_PALETTE_LENGTH: c_int = 256;

/* ---------------- info valid flags ---------------- */

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

/* ---------------- text compression ---------------- */

pub const PNG_TEXT_COMPRESSION_NONE_WR: c_int = -3;
pub const PNG_TEXT_COMPRESSION_zTXt_WR: c_int = -2;
pub const PNG_TEXT_COMPRESSION_NONE: c_int = -1;
pub const PNG_TEXT_COMPRESSION_zTXt: c_int = 0;
pub const PNG_ITXT_COMPRESSION_NONE: c_int = 1;
pub const PNG_ITXT_COMPRESSION_zTXt: c_int = 2;
pub const PNG_TEXT_COMPRESSION_LAST: c_int = 3;

/* ---------------- transform flags (public) ---------------- */

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

/* ---------------- MNG features ---------------- */

pub const PNG_FLAG_MNG_EMPTY_PLTE: png_uint_32 = 0x01;
pub const PNG_FLAG_MNG_FILTER_64: png_uint_32 = 0x04;
pub const PNG_ALL_MNG_FEATURES: png_uint_32 = 0x05;

/* ---------------- error actions / rgb to gray ---------------- */

pub const PNG_ERROR_ACTION_NONE: c_int = 1;
pub const PNG_ERROR_ACTION_WARN: c_int = 2;
pub const PNG_ERROR_ACTION_ERROR: c_int = 3;
pub const PNG_RGB_TO_GRAY_DEFAULT: c_int = -1;

/* ---------------- alpha modes / gamma ---------------- */

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

/* ---------------- filler ---------------- */

pub const PNG_FILLER_BEFORE: c_int = 0;
pub const PNG_FILLER_AFTER: c_int = 1;

/* ---------------- CRC actions ---------------- */

pub const PNG_CRC_DEFAULT: c_int = 0;
pub const PNG_CRC_ERROR_QUIT: c_int = 1;
pub const PNG_CRC_WARN_DISCARD: c_int = 2;
pub const PNG_CRC_WARN_USE: c_int = 3;
pub const PNG_CRC_QUIET_USE: c_int = 4;
pub const PNG_CRC_NO_CHANGE: c_int = 5;

/* ---------------- filters ---------------- */

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

/* ---------------- free flags ---------------- */

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

/* ---------------- io state (png.h) ---------------- */

pub const PNG_IO_NONE: png_uint_32 = 0x0000;
pub const PNG_IO_READING: png_uint_32 = 0x0001;
pub const PNG_IO_WRITING: png_uint_32 = 0x0002;
pub const PNG_IO_SIGNATURE: png_uint_32 = 0x0010;
pub const PNG_IO_CHUNK_HDR: png_uint_32 = 0x0020;
pub const PNG_IO_CHUNK_DATA: png_uint_32 = 0x0040;
pub const PNG_IO_CHUNK_CRC: png_uint_32 = 0x0080;
pub const PNG_IO_MASK_OP: png_uint_32 = 0x000f;
pub const PNG_IO_MASK_LOC: png_uint_32 = 0x00f0;

/* ---------------- options (png.h) ---------------- */

pub const PNG_ARM_NEON: c_int = 0;
pub const PNG_MAXIMUM_INFLATE_WINDOW: c_int = 2;
pub const PNG_SKIP_sRGB_CHECK_PROFILE: c_int = 4;
pub const PNG_MIPS_MSA: c_int = 6;
pub const PNG_IGNORE_ADLER32: c_int = 8;
pub const PNG_POWERPC_VSX: c_int = 10;
pub const PNG_MIPS_MMI: c_int = 12;
pub const PNG_RISCV_RVV: c_int = 14;
pub const PNG_OPTION_NEXT: c_int = 16;

pub const PNG_OPTION_UNSET: c_int = 0;
pub const PNG_OPTION_INVALID: c_int = 1;
pub const PNG_OPTION_OFF: c_int = 2;
pub const PNG_OPTION_ON: c_int = 3;

/* ---------------- simplified API (png.h) ---------------- */

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
pub const PNG_FORMAT_LINEAR_Y_ALPHA: png_uint_32 = PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_ALPHA;
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

/* ---------------- handle-as-unknown (png.h) ---------------- */

pub const PNG_HANDLE_CHUNK_AS_DEFAULT: c_int = 0;
pub const PNG_HANDLE_CHUNK_NEVER: c_int = 1;
pub const PNG_HANDLE_CHUNK_IF_SAFE: c_int = 2;
pub const PNG_HANDLE_CHUNK_ALWAYS: c_int = 3;
pub const PNG_HANDLE_CHUNK_LAST: c_int = 4;

/* ---------------- mode flags (png.h + pngpriv.h) ---------------- */

pub const PNG_HAVE_IHDR: png_uint_32 = 0x01;
pub const PNG_HAVE_PLTE: png_uint_32 = 0x02;
pub const PNG_HAVE_IDAT: png_uint_32 = 0x04;
pub const PNG_AFTER_IDAT: png_uint_32 = 0x08;
pub const PNG_HAVE_IEND: png_uint_32 = 0x10;
pub const PNG_HAVE_CHUNK_HEADER: png_uint_32 = 0x100;
pub const PNG_WROTE_tIME: png_uint_32 = 0x200;
pub const PNG_WROTE_INFO_BEFORE_PLTE: png_uint_32 = 0x400;
pub const PNG_BACKGROUND_IS_GRAY: png_uint_32 = 0x800;
pub const PNG_HAVE_PNG_SIGNATURE: png_uint_32 = 0x1000;
pub const PNG_HAVE_CHUNK_AFTER_IDAT: png_uint_32 = 0x2000;
pub const PNG_WROTE_eXIf: png_uint_32 = 0x4000;
pub const PNG_IS_READ_STRUCT: png_uint_32 = 0x8000;

/* ---------------- transformations (pngpriv.h) ---------------- */

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

/* ---------------- png_create_struct flags ---------------- */

pub const PNG_STRUCT_PNG: png_uint_32 = 0x0001;
pub const PNG_STRUCT_INFO: png_uint_32 = 0x0002;

/* ---------------- png_ptr->flags (pngpriv.h) ---------------- */

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
pub const PNG_FLAG_CRC_MASK: png_uint_32 = PNG_FLAG_CRC_ANCILLARY_MASK | PNG_FLAG_CRC_CRITICAL_MASK;

/* ---------------- gamma (pngpriv.h) ---------------- */

pub const PNG_GAMMA_MAC_OLD: png_fixed_point = 151724;
pub const PNG_GAMMA_MAC_INVERSE: png_fixed_point = 65909;
pub const PNG_GAMMA_sRGB_INVERSE: png_fixed_point = 45455;
pub const PNG_LIB_GAMMA_MIN: png_fixed_point = 1000;
pub const PNG_LIB_GAMMA_MAX: png_fixed_point = 10000000;

/* ---------------- chunk type constants (pngpriv.h) ---------------- */

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

/* ---------------- known chunk indices (PNG_KNOWN_CHUNKS) ---------------- */

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

/* ---------------- number formats (pngpriv.h) ---------------- */

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

/* ---------------- chunk report severity ---------------- */

pub const PNG_CHUNK_WARNING: c_int = 0;
pub const PNG_CHUNK_WRITE_ERROR: c_int = 1;
pub const PNG_CHUNK_ERROR: c_int = 2;

/* ---------------- fp parser states (pngpriv.h) ---------------- */

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

pub const PNG_sCAL_MAX_DIGITS: usize = PNG_sCAL_PRECISION + 1 + 1 + 10;

/* ---------------- misc ---------------- */

pub const PNG_UNEXPECTED_ZLIB_RETURN: c_int = -7;
pub const ZLIB_IO_MAX: crate::sys::uInt = crate::sys::uInt::MAX;
pub const PNG_USE_COMPILE_TIME_MASKS: c_int = 1;
pub const PNG_INFLATE_BUF_SIZE_U: usize = PNG_INFLATE_BUF_SIZE;
