//! Small helper functions/macros used across modules (translations of
//! function-like macros in pngpriv.h).
#![allow(dead_code)]

use crate::ptypes::{png_byte, png_uint_32};

/// PNG_ROWBYTES(pixel_bits, width)
#[inline]
pub fn png_rowbytes(pixel_bits: u32, width: u32) -> usize {
    let width = width as usize;
    let pixel_bits = pixel_bits as usize;
    if pixel_bits >= 8 {
        width * (pixel_bits >> 3)
    } else {
        (width * pixel_bits + 7) >> 3
    }
}

/// PNG_DIV65535
#[inline]
pub fn png_div65535(v24: u32) -> u32 {
    (v24 + 32895) >> 16
}

/// PNG_DIV257
#[inline]
pub fn png_div257(v16: u32) -> u32 {
    png_div65535(v16 * 255)
}

/// PNG_PIXEL_DEPTH-ish helpers on chunk names
#[inline]
pub fn png_chunk_ancillary(c: png_uint_32) -> u32 {
    1 & (c >> 29)
}
#[inline]
pub fn png_chunk_critical(c: png_uint_32) -> u32 {
    (png_chunk_ancillary(c) == 0) as u32
}
#[inline]
pub fn png_chunk_private(c: png_uint_32) -> u32 {
    1 & (c >> 21)
}
#[inline]
pub fn png_chunk_reserved(c: png_uint_32) -> u32 {
    1 & (c >> 13)
}
#[inline]
pub fn png_chunk_safe_to_copy(c: png_uint_32) -> u32 {
    1 & (c >> 5)
}

/// PNG_CSTRING_FROM_CHUNK: fill s[0..5] with the 4 chunk bytes + NUL.
#[inline]
pub unsafe fn png_cstring_from_chunk(s: *mut core::ffi::c_char, c: png_uint_32) {
    *s.offset(0) = ((c >> 24) & 0xff) as core::ffi::c_char;
    *s.offset(1) = ((c >> 16) & 0xff) as core::ffi::c_char;
    *s.offset(2) = ((c >> 8) & 0xff) as core::ffi::c_char;
    *s.offset(3) = (c & 0xff) as core::ffi::c_char;
    *s.offset(4) = 0;
}

/// PNG_STRING_FROM_CHUNK (no NUL terminator)
#[inline]
pub unsafe fn png_string_from_chunk(s: *mut core::ffi::c_char, c: png_uint_32) {
    *s.offset(0) = ((c >> 24) & 0xff) as core::ffi::c_char;
    *s.offset(1) = ((c >> 16) & 0xff) as core::ffi::c_char;
    *s.offset(2) = ((c >> 8) & 0xff) as core::ffi::c_char;
    *s.offset(3) = (c & 0xff) as core::ffi::c_char;
}

/// PNG_CHUNK_FROM_STRING
#[inline]
pub unsafe fn png_chunk_from_string(s: *const png_byte) -> png_uint_32 {
    ((*s.offset(0) as png_uint_32) << 24)
        | ((*s.offset(1) as png_uint_32) << 16)
        | ((*s.offset(2) as png_uint_32) << 8)
        | (*s.offset(3) as png_uint_32)
}

/// PNG_OUT_OF_RANGE
#[inline]
pub fn png_out_of_range(value: i32, ideal: i32, delta: i32) -> bool {
    value < ideal - delta || value > ideal + delta
}
