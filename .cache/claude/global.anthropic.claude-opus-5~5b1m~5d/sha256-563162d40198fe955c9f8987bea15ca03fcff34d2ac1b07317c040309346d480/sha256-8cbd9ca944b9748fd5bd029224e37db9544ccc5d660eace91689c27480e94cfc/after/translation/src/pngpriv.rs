//! Private constants and helper functions (pngpriv.h).
#![allow(non_upper_case_globals)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use core::ffi::c_int;

use crate::types::*;

// --- pnglibconf.h settings -------------------------------------------------
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

pub const PNG_LITERAL_LEFT_SQUARE_BRACKET: u8 = 0x5b;
pub const PNG_LITERAL_RIGHT_SQUARE_BRACKET: u8 = 0x5d;

pub const PNG_ALIGN_NONE: c_int = 0;
pub const PNG_ALIGN_ALWAYS: c_int = 1;
pub const PNG_ALIGN_OFFSET: c_int = 2;
pub const PNG_ALIGN_SIZE: c_int = 3;

// --- Modes of operation ----------------------------------------------------
pub const PNG_HAVE_IDAT: png_uint_32 = 0x04;
pub const PNG_HAVE_IEND: png_uint_32 = 0x10;
pub const PNG_HAVE_CHUNK_HEADER: png_uint_32 = 0x100;
pub const PNG_WROTE_tIME: png_uint_32 = 0x200;
pub const PNG_WROTE_INFO_BEFORE_PLTE: png_uint_32 = 0x400;
pub const PNG_BACKGROUND_IS_GRAY: png_uint_32 = 0x800;
pub const PNG_HAVE_PNG_SIGNATURE: png_uint_32 = 0x1000;
pub const PNG_HAVE_CHUNK_AFTER_IDAT: png_uint_32 = 0x2000;
pub const PNG_WROTE_eXIf: png_uint_32 = 0x4000;
pub const PNG_IS_READ_STRUCT: png_uint_32 = 0x8000;

// --- Transformation flags --------------------------------------------------
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

// --- png_ptr->flags --------------------------------------------------------
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

// --- Chunk type constants --------------------------------------------------
#[inline]
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

#[inline]
pub const fn PNG_32to8(cn: png_uint_32, s: u32) -> u32 {
    (cn >> s) & 0xff
}
#[inline]
pub const fn PNG_CN_VALID_UPPER(b: u32) -> bool {
    b >= 65 && b <= 90
}
#[inline]
pub const fn PNG_CN_VALID_ASCII(b: u32) -> bool {
    PNG_CN_VALID_UPPER(b & !32u32)
}
#[inline]
pub const fn PNG_CHUNK_NAME_VALID(cn: png_uint_32) -> bool {
    PNG_CN_VALID_ASCII(PNG_32to8(cn, 24))
        && PNG_CN_VALID_ASCII(PNG_32to8(cn, 16))
        && PNG_CN_VALID_UPPER(PNG_32to8(cn, 8))
        && PNG_CN_VALID_ASCII(PNG_32to8(cn, 0))
}

/// `PNG_CHUNK_FROM_STRING(s)`
#[inline]
pub unsafe fn PNG_CHUNK_FROM_STRING(s: *const png_byte) -> png_uint_32 {
    PNG_U32(
        (*s.add(0)) as u32,
        (*s.add(1)) as u32,
        (*s.add(2)) as u32,
        (*s.add(3)) as u32,
    )
}

/// `PNG_STRING_FROM_CHUNK(s, c)`
#[inline]
pub unsafe fn PNG_STRING_FROM_CHUNK(s: *mut png_byte, c: png_uint_32) {
    *s.add(0) = ((c >> 24) & 0xff) as png_byte;
    *s.add(1) = ((c >> 16) & 0xff) as png_byte;
    *s.add(2) = ((c >> 8) & 0xff) as png_byte;
    *s.add(3) = (c & 0xff) as png_byte;
}

/// `PNG_CSTRING_FROM_CHUNK(s, c)`
#[inline]
pub unsafe fn PNG_CSTRING_FROM_CHUNK(s: *mut png_byte, c: png_uint_32) {
    PNG_STRING_FROM_CHUNK(s, c);
    *s.add(4) = 0;
}

#[inline]
pub const fn PNG_CHUNK_ANCILLARY(c: png_uint_32) -> u32 {
    1 & (c >> 29)
}
#[inline]
pub const fn PNG_CHUNK_CRITICAL(c: png_uint_32) -> bool {
    PNG_CHUNK_ANCILLARY(c) == 0
}
#[inline]
pub const fn PNG_CHUNK_PRIVATE(c: png_uint_32) -> u32 {
    1 & (c >> 21)
}
#[inline]
pub const fn PNG_CHUNK_RESERVED(c: png_uint_32) -> u32 {
    1 & (c >> 13)
}
#[inline]
pub const fn PNG_CHUNK_SAFE_TO_COPY(c: png_uint_32) -> u32 {
    1 & (c >> 5)
}

// --- Gamma constants -------------------------------------------------------
pub const PNG_GAMMA_MAC_OLD: png_fixed_point = 151724;
pub const PNG_GAMMA_MAC_INVERSE: png_fixed_point = 65909;
pub const PNG_GAMMA_sRGB_INVERSE: png_fixed_point = 45455;
pub const PNG_LIB_GAMMA_MIN: png_fixed_point = 1000;
pub const PNG_LIB_GAMMA_MAX: png_fixed_point = 10000000;

// --- Utility macros --------------------------------------------------------
#[inline]
pub const fn PNG_DIV65535(v24: png_uint_32) -> png_uint_32 {
    (v24.wrapping_add(32895)) >> 16
}
#[inline]
pub const fn PNG_DIV257(v16: png_uint_32) -> png_uint_32 {
    PNG_DIV65535(v16.wrapping_mul(255))
}

/// `PNG_ROWBYTES(pixel_bits, width)`
#[inline]
pub const fn PNG_ROWBYTES(pixel_bits: u32, width: png_uint_32) -> usize {
    if pixel_bits >= 8 {
        (width as usize).wrapping_mul((pixel_bits as usize) >> 3)
    } else {
        ((width as usize)
            .wrapping_mul(pixel_bits as usize)
            .wrapping_add(7))
            >> 3
    }
}

#[inline]
pub const fn PNG_TRAILBITS(pixel_bits: u32, width: png_uint_32) -> png_uint_32 {
    (pixel_bits.wrapping_mul(width % 8)) % 8
}

#[inline]
pub const fn PNG_PADBITS(pixel_bits: u32, width: png_uint_32) -> png_uint_32 {
    (8 - PNG_TRAILBITS(pixel_bits, width)) % 8
}

#[inline]
pub fn PNG_OUT_OF_RANGE(value: png_fixed_point, ideal: png_fixed_point, delta: png_fixed_point) -> bool {
    value < ideal - delta || value > ideal + delta
}

#[inline]
pub fn PNG_COLOR_DIST(c1: png_color, c2: png_color) -> c_int {
    ((c1.red as c_int) - (c2.red as c_int)).abs()
        + ((c1.green as c_int) - (c2.green as c_int)).abs()
        + ((c1.blue as c_int) - (c2.blue as c_int)).abs()
}

/// `png_float(png_ptr, fixed, s)`
#[inline]
pub fn png_float_of(fixed: png_fixed_point) -> f64 {
    0.00001f64 * (fixed as f64)
}

// --- warning / number formatting -------------------------------------------
pub const PNG_NUMBER_BUFFER_SIZE: usize = 24;
pub const PNG_NUMBER_FORMAT_u: c_int = 1;
pub const PNG_NUMBER_FORMAT_02u: c_int = 2;
pub const PNG_NUMBER_FORMAT_d: c_int = 1;
pub const PNG_NUMBER_FORMAT_02d: c_int = 2;
pub const PNG_NUMBER_FORMAT_x: c_int = 3;
pub const PNG_NUMBER_FORMAT_02x: c_int = 4;
pub const PNG_NUMBER_FORMAT_fixed: c_int = 5;

pub const PNG_WARNING_PARAMETER_SIZE: usize = 32;
pub const PNG_WARNING_PARAMETER_COUNT: usize = 8;
pub type png_warning_parameters = [[core::ffi::c_char; PNG_WARNING_PARAMETER_SIZE];
    PNG_WARNING_PARAMETER_COUNT];

pub const PNG_CHUNK_WARNING: c_int = 0;
pub const PNG_CHUNK_WRITE_ERROR: c_int = 1;
pub const PNG_CHUNK_ERROR: c_int = 2;

pub const PNG_UNEXPECTED_ZLIB_RETURN: c_int = -7;

// --- ASCII <-> FP parser states -------------------------------------------
pub const PNG_sCAL_MAX_DIGITS: usize = PNG_sCAL_PRECISION + 1 + 1 + 10;

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

#[inline]
pub const fn PNG_FP_IS_ZERO(state: c_int) -> bool {
    (state & PNG_FP_Z_MASK) == PNG_FP_SAW_DIGIT
}
#[inline]
pub const fn PNG_FP_IS_POSITIVE(state: c_int) -> bool {
    (state & PNG_FP_NZ_MASK) == PNG_FP_Z_MASK
}
#[inline]
pub const fn PNG_FP_IS_NEGATIVE(state: c_int) -> bool {
    (state & PNG_FP_NZ_MASK) == PNG_FP_NZ_MASK
}

// --- png_handle_result_code ------------------------------------------------
pub const handled_error: c_int = 0;
pub const handled_discarded: c_int = 1;
pub const handled_saved: c_int = 2;
pub const handled_ok: c_int = 3;

pub const PNG_USE_COMPILE_TIME_MASKS: c_int = 1;

/// `PNG_sRGB_FROM_LINEAR(linear)`
#[inline]
pub fn PNG_sRGB_FROM_LINEAR(linear: png_uint_32) -> png_byte {
    let hi = (linear >> 15) as usize;
    (0xff & ((crate::srgb_tables::png_sRGB_base[hi] as u32
        + (((linear & 0x7fff) * crate::srgb_tables::png_sRGB_delta[hi] as u32) >> 12))
        >> 8)) as png_byte
}
