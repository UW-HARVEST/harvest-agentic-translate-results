//! Rust equivalents of the libpng utility macros (pngpriv.h / png.h).
#![allow(non_snake_case)]

use core::ffi::{c_char, c_int, c_uint};

use crate::consts::*;
use crate::types::*;

/* --------------------------------------------------------------- */
/* PNG_ROWBYTES / trailing bits                                     */
/* --------------------------------------------------------------- */

/// `PNG_ROWBYTES(pixel_bits, width)`
#[inline]
pub fn PNG_ROWBYTES(pixel_bits: usize, width: usize) -> usize {
    if pixel_bits >= 8 {
        width * (pixel_bits >> 3)
    } else {
        ((width * pixel_bits) + 7) >> 3
    }
}

/// `PNG_TRAILBITS(pixel_bits, width)`
#[inline]
pub fn PNG_TRAILBITS(pixel_bits: png_uint_32, width: png_uint_32) -> png_uint_32 {
    (pixel_bits.wrapping_mul(width % 8)) % 8
}

/// `PNG_PADBITS(pixel_bits, width)`
#[inline]
pub fn PNG_PADBITS(pixel_bits: png_uint_32, width: png_uint_32) -> png_uint_32 {
    (8 - PNG_TRAILBITS(pixel_bits, width)) % 8
}

/// `PNG_OUT_OF_RANGE(value, ideal, delta)`
#[inline]
pub fn PNG_OUT_OF_RANGE(
    value: png_fixed_point,
    ideal: png_fixed_point,
    delta: png_fixed_point,
) -> bool {
    value < ideal - delta || value > ideal + delta
}

/// `PNG_DIV65535(v24)`
#[inline]
pub fn PNG_DIV65535(v24: png_uint_32) -> png_uint_32 {
    (v24.wrapping_add(32895)) >> 16
}

/// `PNG_DIV257(v16)`
#[inline]
pub fn PNG_DIV257(v16: png_uint_32) -> png_uint_32 {
    PNG_DIV65535(v16.wrapping_mul(255))
}

/// `png_float(png_ptr, fixed, s)`
#[inline]
pub fn png_float(_png_ptr: png_const_structrp, fixed: png_fixed_point, _s: *const c_char) -> f64 {
    0.00001f64 * (fixed as f64)
}

/* --------------------------------------------------------------- */
/* chunk name helpers                                               */
/* --------------------------------------------------------------- */

/// `PNG_CHUNK_ANCILLARY(c)`
#[inline]
pub fn PNG_CHUNK_ANCILLARY(c: png_uint_32) -> c_int {
    (1 & (c >> 29)) as c_int
}
/// `PNG_CHUNK_CRITICAL(c)`
#[inline]
pub fn PNG_CHUNK_CRITICAL(c: png_uint_32) -> c_int {
    (PNG_CHUNK_ANCILLARY(c) == 0) as c_int
}
/// `PNG_CHUNK_PRIVATE(c)`
#[inline]
pub fn PNG_CHUNK_PRIVATE(c: png_uint_32) -> c_int {
    (1 & (c >> 21)) as c_int
}
/// `PNG_CHUNK_RESERVED(c)`
#[inline]
pub fn PNG_CHUNK_RESERVED(c: png_uint_32) -> c_int {
    (1 & (c >> 13)) as c_int
}
/// `PNG_CHUNK_SAFE_TO_COPY(c)`
#[inline]
pub fn PNG_CHUNK_SAFE_TO_COPY(c: png_uint_32) -> c_int {
    (1 & (c >> 5)) as c_int
}

/// `PNG_CHUNK_FROM_STRING(s)`
#[inline]
pub unsafe fn PNG_CHUNK_FROM_STRING(s: *const c_char) -> png_uint_32 {
    PNG_U32(
        (0xff & *s.add(0) as u32) as u32,
        (0xff & *s.add(1) as u32) as u32,
        (0xff & *s.add(2) as u32) as u32,
        (0xff & *s.add(3) as u32) as u32,
    )
}

/// `PNG_STRING_FROM_CHUNK(s, c)`
#[inline]
pub unsafe fn PNG_STRING_FROM_CHUNK(s: *mut c_char, c: png_uint_32) {
    *s.add(0) = ((c >> 24) & 0xff) as c_char;
    *s.add(1) = ((c >> 16) & 0xff) as c_char;
    *s.add(2) = ((c >> 8) & 0xff) as c_char;
    *s.add(3) = (c & 0xff) as c_char;
}

/// `PNG_CSTRING_FROM_CHUNK(s, c)`
#[inline]
pub unsafe fn PNG_CSTRING_FROM_CHUNK(s: *mut c_char, c: png_uint_32) {
    PNG_STRING_FROM_CHUNK(s, c);
    *s.add(4) = 0;
}

/// `PNG_32to8(cn, s)`
#[inline]
pub fn PNG_32to8(cn: png_uint_32, s: c_int) -> png_uint_32 {
    (cn >> s) & 0xff
}

/// `PNG_CN_VALID_UPPER(b)`
#[inline]
pub fn PNG_CN_VALID_UPPER(b: png_uint_32) -> bool {
    b >= 65 && b <= 90
}

/// `PNG_CN_VALID_ASCII(b)`
#[inline]
pub fn PNG_CN_VALID_ASCII(b: png_uint_32) -> bool {
    PNG_CN_VALID_UPPER(b & !32u32)
}

/// `PNG_CHUNK_NAME_VALID(cn)`
#[inline]
pub fn PNG_CHUNK_NAME_VALID(cn: png_uint_32) -> bool {
    PNG_CN_VALID_ASCII(PNG_32to8(cn, 24))
        && PNG_CN_VALID_ASCII(PNG_32to8(cn, 16))
        && PNG_CN_VALID_UPPER(PNG_32to8(cn, 8))
        && PNG_CN_VALID_ASCII(PNG_32to8(cn, 0))
}

/* --------------------------------------------------------------- */
/* png_struct::chunks bit set                                       */
/* --------------------------------------------------------------- */

/// `png_chunk_flag_from_index(i)`
#[inline]
pub fn png_chunk_flag_from_index(i: c_int) -> png_uint_32 {
    0x80000000u32 >> (31 - i)
}

/// `png_file_has_chunk(png_ptr, i)`
#[inline]
pub unsafe fn png_file_has_chunk(png_ptr: png_const_structrp, i: c_int) -> bool {
    ((*png_ptr).chunks & png_chunk_flag_from_index(i)) != 0
}

/// `png_file_add_chunk(png_ptr, i)`
#[inline]
pub unsafe fn png_file_add_chunk(png_ptr: png_structrp, i: c_int) {
    (*png_ptr).chunks |= png_chunk_flag_from_index(i);
}

/* --------------------------------------------------------------- */
/* interlace macros (png.h)                                         */
/* --------------------------------------------------------------- */

/// `PNG_PASS_START_ROW(pass)`
#[inline]
pub fn PNG_PASS_START_ROW(pass: c_int) -> c_int {
    ((1 & !pass) << (3 - (pass >> 1))) & 7
}
/// `PNG_PASS_START_COL(pass)`
#[inline]
pub fn PNG_PASS_START_COL(pass: c_int) -> c_int {
    ((1 & pass) << (3 - ((pass + 1) >> 1))) & 7
}
/// `PNG_PASS_ROW_OFFSET(pass)`
#[inline]
pub fn PNG_PASS_ROW_OFFSET(pass: c_int) -> c_int {
    if pass > 2 {
        8 >> ((pass - 1) >> 1)
    } else {
        8
    }
}
/// `PNG_PASS_COL_OFFSET(pass)`
#[inline]
pub fn PNG_PASS_COL_OFFSET(pass: c_int) -> c_int {
    1 << ((7 - pass) >> 1)
}
/// `PNG_PASS_ROW_SHIFT(pass)`
#[inline]
pub fn PNG_PASS_ROW_SHIFT(pass: c_int) -> c_int {
    if pass > 2 {
        (8 - pass) >> 1
    } else {
        3
    }
}
/// `PNG_PASS_COL_SHIFT(pass)`
#[inline]
pub fn PNG_PASS_COL_SHIFT(pass: c_int) -> c_int {
    if pass > 1 {
        (7 - pass) >> 1
    } else {
        3
    }
}
/// `PNG_PASS_ROWS(height, pass)`
#[inline]
pub fn PNG_PASS_ROWS(height: png_uint_32, pass: c_int) -> png_uint_32 {
    (height.wrapping_add(
        (((1 << PNG_PASS_ROW_SHIFT(pass)) - 1) - PNG_PASS_START_ROW(pass)) as png_uint_32,
    )) >> PNG_PASS_ROW_SHIFT(pass)
}
/// `PNG_PASS_COLS(width, pass)`
#[inline]
pub fn PNG_PASS_COLS(width: png_uint_32, pass: c_int) -> png_uint_32 {
    (width.wrapping_add(
        (((1 << PNG_PASS_COL_SHIFT(pass)) - 1) - PNG_PASS_START_COL(pass)) as png_uint_32,
    )) >> PNG_PASS_COL_SHIFT(pass)
}
/// `PNG_ROW_FROM_PASS_ROW(y_in, pass)`
#[inline]
pub fn PNG_ROW_FROM_PASS_ROW(y_in: png_uint_32, pass: c_int) -> png_uint_32 {
    (y_in << PNG_PASS_ROW_SHIFT(pass)).wrapping_add(PNG_PASS_START_ROW(pass) as png_uint_32)
}
/// `PNG_COL_FROM_PASS_COL(x_in, pass)`
#[inline]
pub fn PNG_COL_FROM_PASS_COL(x_in: png_uint_32, pass: c_int) -> png_uint_32 {
    (x_in << PNG_PASS_COL_SHIFT(pass)).wrapping_add(PNG_PASS_START_COL(pass) as png_uint_32)
}
/// `PNG_PASS_MASK(pass, off)`
#[inline]
pub fn PNG_PASS_MASK(pass: c_int, off: c_int) -> png_uint_32 {
    ((0x110145AFu32 >> (((7 - off) - pass) << 2)) & 0xF)
        | ((0x01145AF0u32 >> (((7 - off) - pass) << 2)) & 0xF0)
}
/// `PNG_ROW_IN_INTERLACE_PASS(y, pass)`
#[inline]
pub fn PNG_ROW_IN_INTERLACE_PASS(y: png_uint_32, pass: c_int) -> c_int {
    ((PNG_PASS_MASK(pass, 0) >> (y & 7)) & 1) as c_int
}
/// `PNG_COL_IN_INTERLACE_PASS(x, pass)`
#[inline]
pub fn PNG_COL_IN_INTERLACE_PASS(x: png_uint_32, pass: c_int) -> c_int {
    ((PNG_PASS_MASK(pass, 1) >> (x & 7)) & 1) as c_int
}

/* --------------------------------------------------------------- */
/* alpha compositing (PNG_READ_COMPOSITE_NODIV_SUPPORTED variant)   */
/* --------------------------------------------------------------- */

/// `png_composite(composite, fg, alpha, bg)` - NODIV variant.
#[inline]
pub fn png_composite(fg: png_uint_16, alpha: png_uint_16, bg: png_uint_16) -> png_byte {
    let temp: png_uint_16 = (fg)
        .wrapping_mul(alpha)
        .wrapping_add(bg.wrapping_mul(255u16.wrapping_sub(alpha)))
        .wrapping_add(128);
    ((temp.wrapping_add(temp >> 8) >> 8) & 0xff) as png_byte
}

/// `png_composite_16(composite, fg, alpha, bg)` - NODIV variant.
#[inline]
pub fn png_composite_16(fg: png_uint_32, alpha: png_uint_32, bg: png_uint_32) -> png_uint_16 {
    let temp: png_uint_32 = fg
        .wrapping_mul(alpha)
        .wrapping_add(bg.wrapping_mul(65535u32.wrapping_sub(alpha)))
        .wrapping_add(32768);
    (0xffff & (temp.wrapping_add(temp >> 16) >> 16)) as png_uint_16
}

/* --------------------------------------------------------------- */
/* alignment                                                        */
/* --------------------------------------------------------------- */

/// `png_isaligned(ptr, type)` with `PNG_ALIGN_TYPE == PNG_ALIGN_SIZE`.
#[inline]
pub fn png_isaligned<T>(ptr: *const u8) -> bool {
    (ptr as usize & (core::mem::size_of::<T>() - 1)) == 0
}

/* --------------------------------------------------------------- */
/* misc                                                             */
/* --------------------------------------------------------------- */

/// `PNG_COLOR_DIST(c1, c2)`
#[inline]
pub fn PNG_COLOR_DIST(c1: png_color, c2: png_color) -> c_int {
    (c1.red as c_int - c2.red as c_int).abs()
        + (c1.green as c_int - c2.green as c_int).abs()
        + (c1.blue as c_int - c2.blue as c_int).abs()
}

/// `png_chunk_max(png_ptr)` (PNG_SET_USER_LIMITS_SUPPORTED variant)
#[inline]
pub unsafe fn png_chunk_max(png_ptr: png_const_structrp) -> png_alloc_size_t {
    (*png_ptr).user_chunk_malloc_max
}

/// `PNG_sRGB_FROM_LINEAR(linear)`
#[inline]
pub fn PNG_sRGB_FROM_LINEAR(linear: png_uint_32) -> png_byte {
    let base = crate::srgb::png_sRGB_base[(linear >> 15) as usize] as png_uint_32;
    let delta = crate::srgb::png_sRGB_delta[(linear >> 15) as usize] as png_uint_32;
    (0xff & ((base.wrapping_add(((linear & 0x7fff).wrapping_mul(delta)) >> 12)) >> 8)) as png_byte
}

/// C string literal helper: `b"...\0"` as `*const c_char`.
#[inline]
pub const fn cstr(s: &'static [u8]) -> *const c_char {
    s.as_ptr() as *const c_char
}

/// Number of `unsigned int` bits used by `num_chunk_list`.
pub type png_num_chunk_list_t = c_uint;
