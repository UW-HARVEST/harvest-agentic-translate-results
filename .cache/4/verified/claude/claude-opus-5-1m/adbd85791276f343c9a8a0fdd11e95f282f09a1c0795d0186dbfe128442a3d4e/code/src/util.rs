//! The libpng preprocessor macros, translated into `#[inline]` functions,
//! plus a handful of C library helpers.

#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]

use crate::ffi::*;
use crate::pngtypes::*;
use core::ffi::{c_char, c_int, c_uint, c_void};

/* ================================================================== */
/* <string.h> replacements                                             */
/* ================================================================== */

#[inline]
pub unsafe fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    if n != 0 {
        core::ptr::copy_nonoverlapping(src as *const u8, dst as *mut u8, n);
    }
    dst
}

#[inline]
pub unsafe fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void {
    if n != 0 {
        core::ptr::copy(src as *const u8, dst as *mut u8, n);
    }
    dst
}

#[inline]
pub unsafe fn memset(dst: *mut c_void, v: c_int, n: usize) -> *mut c_void {
    if n != 0 {
        core::ptr::write_bytes(dst as *mut u8, v as u8, n);
    }
    dst
}

extern "C" {
    /// The C library `memcmp`.  `png_sig_cmp()` *returns* the value of `memcmp`
    /// straight to its caller, so the magnitude — not just the sign — is part of
    /// libpng's observable behaviour; glibc returns the difference of the first
    /// differing bytes.  Calling the very same routine the C build calls is the
    /// only way to guarantee an identical result.
    #[link_name = "memcmp"]
    fn c_memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
}

/* ================================================================== */
/* C integer division                                                  */
/* ================================================================== */

/// `a / b` with C's behaviour when `b == 0`.
///
/// `png_image_write_main` divides by `png_row_stride`, which is zero when the
/// application asks for a zero-width image (`pngwrite.c:2045`); the C library
/// therefore executes a `div` by zero and the process dies from **SIGFPE**.
/// Rust's `/` would instead panic ("attempt to divide by zero"), which under
/// `panic = "abort"` terminates with SIGABRT — a different observable outcome for
/// the very same input.  Perform the division with the same instruction the C
/// compiler emits so that the same hardware trap is raised.
#[inline]
pub unsafe fn c_div_u32(a: png_uint_32, b: png_uint_32) -> png_uint_32 {
    #[cfg(target_arch = "x86_64")]
    {
        let mut quotient: u32 = a;
        core::arch::asm!(
            "xor edx, edx",
            "div {divisor:e}",
            divisor = in(reg) b,
            inout("eax") quotient,
            out("edx") _,
            options(nostack, nomem)
        );
        quotient
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        if b == 0 {
            // Deliver the very signal the hardware trap would.
            extern "C" {
                fn raise(sig: c_int) -> c_int;
            }
            raise(8 /* SIGFPE */);
            crate::PNG_ABORT()
        } else {
            a / b
        }
    }
}

#[inline]
pub unsafe fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int {
    c_memcmp(a, b, n)
}

#[inline]
pub unsafe fn strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

#[inline]
pub unsafe fn strcmp(a: *const c_char, b: *const c_char) -> c_int {
    let mut i = 0usize;
    loop {
        let x = *a.add(i) as u8;
        let y = *b.add(i) as u8;
        if x != y {
            return if x < y { -1 } else { 1 };
        }
        if x == 0 {
            return 0;
        }
        i += 1;
    }
}

#[inline]
pub unsafe fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int {
    let mut i = 0usize;
    while i < n {
        let x = *a.add(i) as u8;
        let y = *b.add(i) as u8;
        if x != y {
            return if x < y { -1 } else { 1 };
        }
        if x == 0 {
            return 0;
        }
        i += 1;
    }
    0
}

/// `abs()` for `int`.
#[inline]
pub fn abs(v: c_int) -> c_int {
    v.wrapping_abs()
}

/* ================================================================== */
/* png.h read/write integer macros                                     */
/* ================================================================== */

/// `PNG_get_uint_32` — the read macro used inside the library.
#[inline]
pub unsafe fn PNG_get_uint_32(buf: png_const_bytep) -> png_uint_32 {
    ((*buf as png_uint_32) << 24)
        .wrapping_add((*buf.add(1) as png_uint_32) << 16)
        .wrapping_add((*buf.add(2) as png_uint_32) << 8)
        .wrapping_add(*buf.add(3) as png_uint_32)
}

/// `PNG_get_uint_16`
#[inline]
pub unsafe fn PNG_get_uint_16(buf: png_const_bytep) -> png_uint_16 {
    (((*buf as c_uint) << 8) + (*buf.add(1) as c_uint)) as png_uint_16
}

/// `PNG_get_int_32`
#[inline]
pub unsafe fn PNG_get_int_32(buf: png_const_bytep) -> png_int_32 {
    if (*buf & 0x80) != 0 {
        -(((((PNG_get_uint_32(buf) ^ 0xffffffffu32).wrapping_add(1)) & 0x7fffffff) as png_int_32))
    } else {
        PNG_get_uint_32(buf) as png_int_32
    }
}

/* ================================================================== */
/* pngpriv.h utility macros                                            */
/* ================================================================== */

/// `PNG_ROWBYTES(pixel_bits, width)`
#[inline]
pub fn PNG_ROWBYTES(pixel_bits: usize, width: usize) -> usize {
    if pixel_bits >= 8 {
        width.wrapping_mul(pixel_bits >> 3)
    } else {
        (width.wrapping_mul(pixel_bits).wrapping_add(7)) >> 3
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

/// `PNG_OUT_OF_RANGE(value, ideal, delta)`
#[inline]
pub fn PNG_OUT_OF_RANGE(value: png_fixed_point, ideal: png_fixed_point, delta: png_fixed_point) -> bool {
    value < ideal - delta || value > ideal + delta
}

/// `PNG_COLOR_DIST(c1, c2)`
#[inline]
pub fn PNG_COLOR_DIST(c1: png_color, c2: png_color) -> c_int {
    abs(c1.red as c_int - c2.red as c_int)
        + abs(c1.green as c_int - c2.green as c_int)
        + abs(c1.blue as c_int - c2.blue as c_int)
}

/// `png_float(png_ptr, fixed, s)`
#[inline]
pub fn png_float(_png_ptr: png_const_structrp, fixed: png_fixed_point, _s: png_const_charp) -> f64 {
    0.00001 * (fixed as f64)
}

pub const PNG_GAMMA_THRESHOLD: f64 = (PNG_GAMMA_THRESHOLD_FIXED as f64) * 0.00001;

/* Chunk name helpers ----------------------------------------------- */

#[inline]
pub fn PNG_32to8(cn: png_uint_32, s: c_int) -> png_uint_32 {
    (cn >> s) & 0xff
}

#[inline]
pub fn PNG_CHUNK_ANCILLARY(c: png_uint_32) -> png_uint_32 {
    1 & (c >> 29)
}
#[inline]
pub fn PNG_CHUNK_CRITICAL(c: png_uint_32) -> bool {
    PNG_CHUNK_ANCILLARY(c) == 0
}
#[inline]
pub fn PNG_CHUNK_PRIVATE(c: png_uint_32) -> png_uint_32 {
    1 & (c >> 21)
}
#[inline]
pub fn PNG_CHUNK_RESERVED(c: png_uint_32) -> png_uint_32 {
    1 & (c >> 13)
}
#[inline]
pub fn PNG_CHUNK_SAFE_TO_COPY(c: png_uint_32) -> png_uint_32 {
    1 & (c >> 5)
}

#[inline]
pub fn PNG_CN_VALID_UPPER(b: png_uint_32) -> bool {
    b >= 65 && b <= 90
}
#[inline]
pub fn PNG_CN_VALID_ASCII(b: png_uint_32) -> bool {
    PNG_CN_VALID_UPPER(b & !32u32)
}
#[inline]
pub fn PNG_CHUNK_NAME_VALID(cn: png_uint_32) -> bool {
    PNG_CN_VALID_ASCII(PNG_32to8(cn, 24))
        && PNG_CN_VALID_ASCII(PNG_32to8(cn, 16))
        && PNG_CN_VALID_UPPER(PNG_32to8(cn, 8))
        && PNG_CN_VALID_ASCII(PNG_32to8(cn, 0))
}

/// `PNG_CHUNK_FROM_STRING(s)`
///
/// The reads are `read_volatile` so that they happen *before* the enclosing call,
/// exactly as in C.  `png_write_chunk_start(png_ptr, NULL, len)` and
/// `png_write_chunk(png_ptr, NULL, ..)` expand this macro in the argument list of
/// `png_write_chunk_header`, so the C library dereferences the NULL *before* it
/// reaches that function's `png_ptr == NULL` guard and dies.  With plain
/// dereferences LLVM is free to sink the loads past the callee's null check (a
/// load that would trap is UB in Rust), which made the Rust build survive a call
/// the C build does not.
#[inline]
pub unsafe fn PNG_CHUNK_FROM_STRING(s: png_const_bytep) -> png_uint_32 {
    PNG_U32(
        0xff & core::ptr::read_volatile(s) as png_uint_32,
        0xff & core::ptr::read_volatile(s.add(1)) as png_uint_32,
        0xff & core::ptr::read_volatile(s.add(2)) as png_uint_32,
        0xff & core::ptr::read_volatile(s.add(3)) as png_uint_32,
    )
}

/// `PNG_STRING_FROM_CHUNK(s, c)`
#[inline]
pub unsafe fn PNG_STRING_FROM_CHUNK(s: png_bytep, c: png_uint_32) {
    *s = ((c >> 24) & 0xff) as png_byte;
    *s.add(1) = ((c >> 16) & 0xff) as png_byte;
    *s.add(2) = ((c >> 8) & 0xff) as png_byte;
    *s.add(3) = (c & 0xff) as png_byte;
}

/// `PNG_CSTRING_FROM_CHUNK(s, c)`
#[inline]
pub unsafe fn PNG_CSTRING_FROM_CHUNK(s: png_bytep, c: png_uint_32) {
    PNG_STRING_FROM_CHUNK(s, c);
    *s.add(4) = 0;
}

/* png_struct::chunks bookkeeping ----------------------------------- */

#[inline]
pub fn png_chunk_flag_from_index(i: c_int) -> png_uint_32 {
    0x80000000u32 >> (31 - i)
}

#[inline]
pub unsafe fn png_file_has_chunk(png_ptr: png_const_structrp, i: c_int) -> bool {
    ((*png_ptr).chunks & png_chunk_flag_from_index(i)) != 0
}

#[inline]
pub unsafe fn png_file_add_chunk(png_ptr: png_structrp, i: c_int) {
    (*png_ptr).chunks |= png_chunk_flag_from_index(i);
}

/// `png_chunk_max(png_ptr)` — PNG_SET_USER_LIMITS_SUPPORTED is enabled.
#[inline]
pub unsafe fn png_chunk_max(png_ptr: png_const_structrp) -> png_alloc_size_t {
    (*png_ptr).user_chunk_malloc_max
}

/* Interlace helpers ------------------------------------------------ */

#[inline]
pub fn PNG_PASS_START_ROW(pass: c_int) -> c_int {
    ((1 & !pass) << (3 - (pass >> 1))) & 7
}
#[inline]
pub fn PNG_PASS_START_COL(pass: c_int) -> c_int {
    ((1 & pass) << (3 - ((pass + 1) >> 1))) & 7
}
#[inline]
pub fn PNG_PASS_ROW_OFFSET(pass: c_int) -> c_int {
    if pass > 2 {
        8 >> ((pass - 1) >> 1)
    } else {
        8
    }
}
#[inline]
pub fn PNG_PASS_COL_OFFSET(pass: c_int) -> c_int {
    1 << ((7 - pass) >> 1)
}
#[inline]
pub fn PNG_PASS_ROW_SHIFT(pass: c_int) -> c_int {
    if pass > 2 {
        (8 - pass) >> 1
    } else {
        3
    }
}
#[inline]
pub fn PNG_PASS_COL_SHIFT(pass: c_int) -> c_int {
    if pass > 1 {
        (7 - pass) >> 1
    } else {
        3
    }
}
#[inline]
pub fn PNG_PASS_ROWS(height: png_uint_32, pass: c_int) -> png_uint_32 {
    (height.wrapping_add(
        ((1u32 << PNG_PASS_ROW_SHIFT(pass)) - 1).wrapping_sub(PNG_PASS_START_ROW(pass) as png_uint_32),
    )) >> PNG_PASS_ROW_SHIFT(pass)
}
#[inline]
pub fn PNG_PASS_COLS(width: png_uint_32, pass: c_int) -> png_uint_32 {
    (width.wrapping_add(
        ((1u32 << PNG_PASS_COL_SHIFT(pass)) - 1).wrapping_sub(PNG_PASS_START_COL(pass) as png_uint_32),
    )) >> PNG_PASS_COL_SHIFT(pass)
}
#[inline]
pub fn PNG_ROW_FROM_PASS_ROW(y_in: png_uint_32, pass: c_int) -> png_uint_32 {
    (y_in << PNG_PASS_ROW_SHIFT(pass)).wrapping_add(PNG_PASS_START_ROW(pass) as png_uint_32)
}
#[inline]
pub fn PNG_COL_FROM_PASS_COL(x_in: png_uint_32, pass: c_int) -> png_uint_32 {
    (x_in << PNG_PASS_COL_SHIFT(pass)).wrapping_add(PNG_PASS_START_COL(pass) as png_uint_32)
}
#[inline]
pub fn PNG_PASS_MASK(pass: c_int, off: c_int) -> png_uint_32 {
    ((0x110145AFu32 >> (((7 - off) - pass) << 2)) & 0xF)
        | ((0x01145AF0u32 >> (((7 - off) - pass) << 2)) & 0xF0)
}
#[inline]
pub fn PNG_ROW_IN_INTERLACE_PASS(y: png_uint_32, pass: c_int) -> png_uint_32 {
    (PNG_PASS_MASK(pass, 0) >> (y & 7)) & 1
}
#[inline]
pub fn PNG_COL_IN_INTERLACE_PASS(x: png_uint_32, pass: c_int) -> png_uint_32 {
    (PNG_PASS_MASK(pass, 1) >> (x & 7)) & 1
}

/* Alpha composition (PNG_READ_COMPOSITE_NODIV_SUPPORTED) ----------- */

/// `png_composite(composite, fg, alpha, bg)`
///
/// ```c
/// png_uint_16 temp = (png_uint_16)((png_uint_16)(fg)
///     * (png_uint_16)(alpha)
///     + (png_uint_16)(bg)*(png_uint_16)(255
///     - (png_uint_16)(alpha)) + 128);
/// (composite) = (png_byte)(((temp + (temp >> 8)) >> 8) & 0xff);
/// ```
#[inline]
pub fn png_composite(fg: png_uint_16, alpha: png_uint_16, bg: png_uint_16) -> png_byte {
    let temp: png_uint_16 = ((fg as c_int).wrapping_mul(alpha as c_int))
        .wrapping_add(
            (bg as c_int).wrapping_mul(((255 - (alpha as c_int)) as png_uint_16) as c_int),
        )
        .wrapping_add(128) as png_uint_16;
    ((((temp as c_int).wrapping_add((temp as c_int) >> 8)) >> 8) & 0xff) as png_byte
}

/// `png_composite_16(composite, fg, alpha, bg)`
///
/// ```c
/// png_uint_32 temp = (png_uint_32)((png_uint_32)(fg)
///     * (png_uint_32)(alpha)
///     + (png_uint_32)(bg)*(65535
///     - (png_uint_32)(alpha)) + 32768);
/// (composite) = (png_uint_16)(0xffff & ((temp + (temp >> 16)) >> 16));
/// ```
#[inline]
pub fn png_composite_16(fg: png_uint_32, alpha: png_uint_32, bg: png_uint_32) -> png_uint_16 {
    let temp: png_uint_32 = fg
        .wrapping_mul(alpha)
        .wrapping_add(bg.wrapping_mul(65535u32.wrapping_sub(alpha)))
        .wrapping_add(32768);
    (0xffff & ((temp.wrapping_add(temp >> 16)) >> 16)) as png_uint_16
}

/* fp state predicates --------------------------------------------- */

#[inline]
pub fn PNG_FP_IS_ZERO(state: c_int) -> bool {
    (state & PNG_FP_Z_MASK) == PNG_FP_SAW_DIGIT
}
#[inline]
pub fn PNG_FP_IS_POSITIVE(state: c_int) -> bool {
    (state & PNG_FP_NZ_MASK) == PNG_FP_Z_MASK
}
#[inline]
pub fn PNG_FP_IS_NEGATIVE(state: c_int) -> bool {
    (state & PNG_FP_NZ_MASK) == PNG_FP_NZ_MASK
}

/* ================================================================== */
/* Simplified API size macros                                          */
/* ================================================================== */

#[inline]
pub fn PNG_IMAGE_SAMPLE_CHANNELS(fmt: png_uint_32) -> png_uint_32 {
    (fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1
}
#[inline]
pub fn PNG_IMAGE_SAMPLE_COMPONENT_SIZE(fmt: png_uint_32) -> png_uint_32 {
    ((fmt & PNG_FORMAT_FLAG_LINEAR) >> 2) + 1
}
#[inline]
pub fn PNG_IMAGE_SAMPLE_SIZE(fmt: png_uint_32) -> png_uint_32 {
    PNG_IMAGE_SAMPLE_CHANNELS(fmt) * PNG_IMAGE_SAMPLE_COMPONENT_SIZE(fmt)
}
#[inline]
pub fn PNG_IMAGE_MAXIMUM_COLORMAP_COMPONENTS(fmt: png_uint_32) -> png_uint_32 {
    PNG_IMAGE_SAMPLE_CHANNELS(fmt) * 256
}
#[inline]
pub fn PNG_IMAGE_PIXEL_CHANNELS(fmt: png_uint_32) -> png_uint_32 {
    if (fmt & PNG_FORMAT_FLAG_COLORMAP) != 0 {
        1
    } else {
        PNG_IMAGE_SAMPLE_CHANNELS(fmt)
    }
}
#[inline]
pub fn PNG_IMAGE_PIXEL_COMPONENT_SIZE(fmt: png_uint_32) -> png_uint_32 {
    if (fmt & PNG_FORMAT_FLAG_COLORMAP) != 0 {
        1
    } else {
        PNG_IMAGE_SAMPLE_COMPONENT_SIZE(fmt)
    }
}
#[inline]
pub fn PNG_IMAGE_PIXEL_SIZE(fmt: png_uint_32) -> png_uint_32 {
    if (fmt & PNG_FORMAT_FLAG_COLORMAP) != 0 {
        1
    } else {
        PNG_IMAGE_SAMPLE_SIZE(fmt)
    }
}
#[inline]
pub unsafe fn PNG_IMAGE_ROW_STRIDE(image: *const png_image) -> png_uint_32 {
    PNG_IMAGE_PIXEL_CHANNELS((*image).format).wrapping_mul((*image).width)
}
#[inline]
pub unsafe fn PNG_IMAGE_BUFFER_SIZE(image: *const png_image, row_stride: png_uint_32) -> png_uint_32 {
    PNG_IMAGE_PIXEL_COMPONENT_SIZE((*image).format)
        .wrapping_mul((*image).height)
        .wrapping_mul(row_stride)
}
#[inline]
pub unsafe fn PNG_IMAGE_SIZE(image: *const png_image) -> png_uint_32 {
    PNG_IMAGE_BUFFER_SIZE(image, PNG_IMAGE_ROW_STRIDE(image))
}
#[inline]
pub unsafe fn PNG_IMAGE_COLORMAP_SIZE(image: *const png_image) -> png_uint_32 {
    PNG_IMAGE_SAMPLE_SIZE((*image).format).wrapping_mul((*image).colormap_entries)
}
#[inline]
pub unsafe fn PNG_IMAGE_DATA_SIZE(image: *const png_image) -> png_uint_32 {
    PNG_IMAGE_SIZE(image).wrapping_add((*image).height)
}
#[inline]
pub fn PNG_ZLIB_MAX_SIZE(b: png_alloc_size_t) -> png_alloc_size_t {
    b + ((b + 7) >> 3) + ((b + 63) >> 6) + 11
}
#[inline]
pub unsafe fn PNG_IMAGE_COMPRESSED_SIZE_MAX(image: *const png_image) -> png_alloc_size_t {
    PNG_ZLIB_MAX_SIZE(PNG_IMAGE_DATA_SIZE(image) as png_alloc_size_t)
}
#[inline]
pub unsafe fn PNG_IMAGE_PNG_SIZE_MAX_(image: *const png_image, image_size: png_alloc_size_t) -> png_alloc_size_t {
    (8usize
        + 25
        + 16
        + 44
        + 12
        + (if ((*image).format & PNG_FORMAT_FLAG_COLORMAP) != 0 {
            12 + 3 * (*image).colormap_entries as usize
                + (if ((*image).format & PNG_FORMAT_FLAG_ALPHA) != 0 {
                    12 + (*image).colormap_entries as usize
                } else {
                    0
                })
        } else {
            0
        })
        + 12)
        + (12 * (image_size / PNG_ZBUF_SIZE))
        + image_size
}
#[inline]
pub unsafe fn PNG_IMAGE_PNG_SIZE_MAX(image: *const png_image) -> png_alloc_size_t {
    PNG_IMAGE_PNG_SIZE_MAX_(image, PNG_IMAGE_COMPRESSED_SIZE_MAX(image))
}
#[inline]
pub unsafe fn PNG_IMAGE_FAILED(image: *const png_image) -> bool {
    ((*image).warning_or_error & 0x03) > 1
}
