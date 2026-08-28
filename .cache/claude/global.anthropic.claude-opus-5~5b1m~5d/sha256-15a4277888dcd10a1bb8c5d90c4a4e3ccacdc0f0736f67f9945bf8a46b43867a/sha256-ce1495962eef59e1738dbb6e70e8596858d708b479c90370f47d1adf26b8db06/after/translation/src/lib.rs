//! Rust translation of c_src/src/lib.c
//!
//! Public ABI (matches `nm -D` of the C shared library exactly):
//!   - premultiply
//!
//! Types mirrored from c_src/include/lib.h:
//!   typedef struct cp_pixel_t { uint8_t r, g, b, a; } cp_pixel_t;      // 4 bytes, align 1
//!   typedef struct cp_image_t { int w; int h; cp_pixel_t *pix; } cp_image_t;

#![allow(non_camel_case_types)]

use core::ffi::c_int;

/// `cp_pixel_t` from lib.h — four `uint8_t` components, size 4, alignment 1.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// `cp_image_t` from lib.h.
#[repr(C)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

/// `sizeof(cp_pixel_t)` as used by the C code.
const PIXEL_SIZE: usize = core::mem::size_of::<cp_pixel_t>();

/// void premultiply(cp_image_t *img);
///
/// Faithful translation of the C loop, including its quirks:
///   * `int stride = w * sizeof(cp_pixel_t);`
///     `sizeof` is `size_t`, so `w` is converted to `size_t` (sign extended),
///     multiplied by 4, then truncated back to `int` -> `w.wrapping_mul(4)`.
///   * the loop bound is `(int)stride * h`, an `int` multiply counting *bytes*
///     (`stride` bytes per row times `h` rows), stepped 4 bytes at a time, i.e.
///     exactly `w * h` pixels. Overflow of that multiply wraps, as it does in
///     practice for the C code.
///   * `i += sizeof(cp_pixel_t)` promotes `i` to `size_t` and truncates back to
///     `int` -> `wrapping_add(4)`.
///   * channel writes are `(uint8_t)(x * 255.0f)`, a C float->integer
///     conversion that truncates toward zero.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn premultiply(img: *mut cp_image_t) {
    // Read the fields through the raw pointer rather than materialising a
    // `&mut cp_image_t`. A reference would assert non-nullness/dereferenceability
    // to the optimiser, whereas the C code simply loads through the pointer; for
    // `img == NULL` we want exactly the C behaviour (a faulting load).
    //
    // `read_unaligned` (not a plain `*` dereference) because the C code imposes
    // no alignment requirement on `cp_image_t *`: on x86-64 an unaligned
    // `img->w` load simply works, so a caller may legitimately pass a
    // misaligned struct pointer. A plain dereference would additionally trip
    // Rust's debug-only "misaligned pointer dereference" check and abort, which
    // the C never does.
    let w: c_int = core::ptr::read_unaligned(core::ptr::addr_of!((*img).w));
    let h: c_int = core::ptr::read_unaligned(core::ptr::addr_of!((*img).h));
    // int stride = w * sizeof(cp_pixel_t);
    let stride: c_int = w.wrapping_mul(PIXEL_SIZE as c_int);
    let data: *mut u8 = core::ptr::read_unaligned(core::ptr::addr_of!((*img).pix)) as *mut u8;

    // for (int i = 0; i < (int)stride * h; i += sizeof(cp_pixel_t))
    let limit: c_int = stride.wrapping_mul(h);
    let mut i: c_int = 0;
    while i < limit {
        // `wrapping_offset`, not `offset`: the C code performs plain address
        // arithmetic with no in-bounds guarantee, and `offset` would let the
        // optimiser assume the result stays inside one allocation.
        let base = data.wrapping_offset(i as isize);

        let a = f32::from(*base.add(3)) / 255.0f32;
        let mut r = f32::from(*base.add(0)) / 255.0f32;
        let mut g = f32::from(*base.add(1)) / 255.0f32;
        let mut b = f32::from(*base.add(2)) / 255.0f32;

        r *= a;
        g *= a;
        b *= a;

        *base.add(0) = c_float_to_u8(r * 255.0f32);
        *base.add(1) = c_float_to_u8(g * 255.0f32);
        *base.add(2) = c_float_to_u8(b * 255.0f32);

        i = i.wrapping_add(PIXEL_SIZE as c_int);
    }
}

/// C semantics for `(uint8_t)some_float`: truncate toward zero, then take the
/// value modulo 2^8. All values produced by `premultiply` are in `[0, 255]`, so
/// the wrapping path is never exercised, but it is spelled out to keep the
/// conversion faithful instead of relying on Rust's saturating `as` cast.
#[inline]
fn c_float_to_u8(v: f32) -> u8 {
    let t = v.trunc();
    if t >= 0.0 && t <= 255.0 {
        t as u8
    } else if t.is_finite() {
        // Emulate the usual two's-complement wrap of the underlying hardware.
        (t as i64 as u64 & 0xff) as u8
    } else {
        0
    }
}
