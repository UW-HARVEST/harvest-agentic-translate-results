//! Remaining declarations of `lib.c`. These are `static` (internal linkage) in
//! C and hence not part of the exported ABI, but they are translated here so
//! that the port covers the whole translation unit.

use core::ffi::{c_char, c_int};

/// ```c
/// struct cp_pixel_t { uint8_t r, g, b, a; };
/// ```
#[repr(C)]
#[derive(Copy, Clone)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// ```c
/// struct cp_image_t { int w; int h; cp_pixel_t *pix; };
/// ```
#[repr(C)]
#[allow(dead_code)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CpPixel,
}

/// ```c
/// static cp_pixel_t cp_make_pixel_a(uint8_t r, uint8_t g, uint8_t b, uint8_t a);
/// ```
#[allow(dead_code)]
pub fn cp_make_pixel_a(r: u8, g: u8, b: u8, a: u8) -> CpPixel {
    CpPixel { r, g, b, a }
}

/// ```c
/// static cp_pixel_t cp_make_pixel(uint8_t r, uint8_t g, uint8_t b);
/// ```
#[allow(dead_code)]
pub fn cp_make_pixel(r: u8, g: u8, b: u8) -> CpPixel {
    CpPixel { r, g, b, a: 0xFF }
}

/// ```c
/// typedef struct cp_raw_png_t { const uint8_t *p; const uint8_t *end; } cp_raw_png_t;
/// ```
#[repr(C)]
pub struct CpRawPng {
    pub p: *const u8,
    pub end: *const u8,
}

/// ```c
/// static uint32_t cp_make32(const uint8_t *s) {
///   return (s[0] << 24) | (s[1] << 16) | (s[2] << 8) | s[3];
/// }
/// ```
#[allow(dead_code)]
pub unsafe fn cp_make32(s: *const u8) -> u32 {
    (((*s.wrapping_offset(0) as c_int) << 24)
        | ((*s.wrapping_offset(1) as c_int) << 16)
        | ((*s.wrapping_offset(2) as c_int) << 8)
        | (*s.wrapping_offset(3) as c_int)) as u32
}

/// ```c
/// static const uint8_t *cp_chunk(cp_raw_png_t *png, const char *chunk, uint32_t minlen) { ... }
/// ```
#[allow(dead_code)]
pub unsafe fn cp_chunk(png: *mut CpRawPng, chunk: *const c_char, minlen: u32) -> *const u8 {
    let len = cp_make32((*png).p);
    let start = (*png).p;
    if memcmp4(start.wrapping_offset(4), chunk as *const u8) == 0 && len >= minlen {
        let offset = len.wrapping_add(12) as c_int;
        if (*png).p.wrapping_offset(offset as isize) as usize <= (*png).end as usize {
            (*png).p = (*png).p.wrapping_offset(offset as isize);
            return start.wrapping_offset(8);
        }
    }
    core::ptr::null()
}

/// ```c
/// static const uint8_t *cp_find(cp_raw_png_t *png, const char *chunk, uint32_t minlen) { ... }
/// ```
#[allow(dead_code)]
pub unsafe fn cp_find(png: *mut CpRawPng, chunk: *const c_char, minlen: u32) -> *const u8 {
    while ((*png).p as usize) < ((*png).end as usize) {
        let len = cp_make32((*png).p);
        let start = (*png).p;
        // `png->p += len + 12;` — the addend keeps type `uint32_t` here (unlike
        // `cp_chunk`, which first stores it in an `int`), so it is *zero*-extended
        // for the pointer arithmetic.
        (*png).p = (*png)
            .p
            .wrapping_add(len.wrapping_add(12) as usize);
        if memcmp4(start.wrapping_offset(4), chunk as *const u8) == 0
            && len >= minlen
            && (*png).p as usize <= (*png).end as usize
        {
            return start.wrapping_offset(8);
        }
    }
    core::ptr::null()
}

/// `memcmp(a, b, 4)`
#[allow(dead_code)]
unsafe fn memcmp4(a: *const u8, b: *const u8) -> c_int {
    let mut i = 0isize;
    while i < 4 {
        let av = *a.wrapping_offset(i);
        let bv = *b.wrapping_offset(i);
        if av != bv {
            return av as c_int - bv as c_int;
        }
        i += 1;
    }
    0
}
