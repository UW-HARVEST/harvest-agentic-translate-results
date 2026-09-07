//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `premultiply`
//!
//! The header `include/lib.h` declares no namespace-renaming macros, so the
//! linker symbol equals the source-level name.

#![allow(non_camel_case_types)]

use std::os::raw::c_int;

/// Mirrors `cp_pixel_t` from `include/lib.h` (4 bytes, no padding).
#[repr(C)]
#[derive(Copy, Clone)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Mirrors `cp_image_t` from `include/lib.h`.
#[repr(C)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

/// Size of `cp_pixel_t`, i.e. `sizeof(cp_pixel_t)` in the C source.
const PIXEL_SIZE: c_int = 4;

/// Byte offsets of the `cp_image_t` fields, matching the C ABI layout
/// `{ int w; int h; cp_pixel_t *pix; }` on LP64 (verified by offsetof assertions
/// in `tests/feature_matrix.rs`).
const OFF_W: usize = 0;
const OFF_H: usize = 4;
const OFF_PIX: usize = 8;

/// The C library is built for the same LP64 target, so `cp_pixel_t *` is 8 bytes.
const _: () = assert!(core::mem::size_of::<*mut cp_pixel_t>() == 8);
const _: () = assert!(core::mem::size_of::<c_int>() == 4);

/// Load `N` bytes starting at `p` one byte at a time.
///
/// This exists to reproduce the C's *unchecked* struct loads. gcc compiles
/// `img->w` to a plain `mov` with no validity or alignment check, so a null or
/// misaligned `img` must fault (or not) exactly as the hardware dictates.
/// Rust's alternatives all impose a debug-mode `ub_checks` precondition that the
/// C does not have:
///
/// * `(*img).w` — checks non-null **and** alignment, aborting with SIGABRT where
///   the C raises SIGSEGV.
/// * `ptr::read_volatile` — checks alignment, aborting on a misaligned `img`
///   that the C reads happily on x86-64.
/// * `ptr::read_unaligned` — checks non-null.
///
/// A `u8` volatile read has no such precondition: its alignment requirement of 1
/// is satisfied by every address, and it performs no null check, so a null `img`
/// faults on the real load just like the C.
#[inline]
unsafe fn load_bytes<const N: usize>(p: *const u8) -> [u8; N] {
    let mut out = [0u8; N];
    let mut k = 0usize;
    while k < N {
        out[k] = p.wrapping_add(k).read_volatile();
        k += 1;
    }
    out
}

/// Alpha-premultiply every pixel of `img` in place.
///
/// Faithful translation of `premultiply` in `c_src/src/lib.c`, including its
/// quirks, which are deliberately preserved:
///
/// * `int stride = w * sizeof(cp_pixel_t);` promotes `w` to `size_t`, performs
///   the multiply in 64-bit unsigned arithmetic and then truncates back to
///   `int`. Truncation to 32 bits makes this indistinguishable from a wrapping
///   32-bit multiply, which is what is used here. (gcc emits `cltq; shl $2,%eax`,
///   i.e. exactly a wrapping 32-bit `w * 4`.)
/// * The loop bound is `(int)stride * h`, a 32-bit signed `imul` compared with
///   `jl`, reproduced here with wrapping semantics and a signed compare.
/// * The iteration walks raw bytes and covers `stride * h` bytes, so a negative
///   or zero bound performs no work at all. Two negative dimensions therefore
///   yield a POSITIVE bound and the loop runs.
/// * The alpha channel (`data[i + 3]`) is read but never written back, so alpha
///   is left untouched. This is not corrected.
/// * The float pipeline is single precision throughout: divide by `255.0f`,
///   multiply by the alpha factor, then scale by `255.0f` and truncate toward
///   zero on the way back to a byte (`cvttss2si` + low byte store).
/// * The `cp_image_t` fields are read with unchecked byte-wise loads (see
///   `load_bytes`) so that a null or misaligned `img` behaves exactly as it does
///   in C rather than tripping a Rust-only debug assertion.
///
/// The pixel bytes themselves are read and written through plain `u8` derefs:
/// `u8` has an alignment of 1, so the only `ub_checks` precondition that applies
/// is the null check, and it is unreachable — the first access of each iteration
/// is `data[i + 3]`, which is never address 0 for a null `pix`, so a null buffer
/// faults on address 3 in both implementations.
///
/// # Safety
///
/// `img` must point to a valid `cp_image_t` whose `pix` buffer holds at least
/// `w * h` pixels, exactly as the C function requires. A null or undersized
/// pointer faults here just as it does in C.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn premultiply(img: *mut cp_image_t) {
    // int w = img->w;  int h = img->h;  ... = img->pix;  -> three unchecked loads
    let base = img as *const u8;
    let w = c_int::from_ne_bytes(load_bytes::<4>(base.wrapping_add(OFF_W)));
    let h = c_int::from_ne_bytes(load_bytes::<4>(base.wrapping_add(OFF_H)));
    // int stride = w * sizeof(cp_pixel_t);  -> 64-bit multiply truncated to int
    let stride: c_int = w.wrapping_mul(PIXEL_SIZE);
    let data: *mut u8 =
        usize::from_ne_bytes(load_bytes::<8>(base.wrapping_add(OFF_PIX))) as *mut u8;

    // for (int i = 0; i < (int)stride * h; i += sizeof(cp_pixel_t))
    let end: c_int = stride.wrapping_mul(h);
    let mut i: c_int = 0;
    while i < end {
        let base = i as isize;

        let a = f32::from(*data.offset(base + 3)) / 255.0f32;
        let mut r = f32::from(*data.offset(base)) / 255.0f32;
        let mut g = f32::from(*data.offset(base + 1)) / 255.0f32;
        let mut b = f32::from(*data.offset(base + 2)) / 255.0f32;

        r *= a;
        g *= a;
        b *= a;

        // (uint8_t)(x * 255.0f): truncate toward zero, keep the low byte.
        *data.offset(base) = (r * 255.0f32) as i32 as u8;
        *data.offset(base + 1) = (g * 255.0f32) as i32 as u8;
        *data.offset(base + 2) = (b * 255.0f32) as i32 as u8;
        // data[i + 3] (alpha) is intentionally left as-is, matching the C.

        i = i.wrapping_add(PIXEL_SIZE);
    }
}
