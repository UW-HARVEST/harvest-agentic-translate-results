//! Rust translation of `c_src/` (minimp3-style layer-1/2 granule dequantizer).
//!
//! The C library globs `src/lib.c` into one shared object whose only exported
//! (non-static) symbol is `dequantize_granule`. `get_bits` is `static` in C, so
//! it stays private here.
//!
//! Semantics are reproduced bit-for-bit, including the quirks of the original:
//!   * `get_bits` advances `bs->pos` *even when* the limit check fails.
//!   * `choff` is initialised once, outside the `j` loop, and toggles between
//!     576 and -558 (`18 - 576`), so `dst` walks backwards on odd steps.
//!   * pointer arithmetic may run past the end of `grbuf` exactly as in C.
//!   * unsigned wrap-around in `code % mod - mod / 2` is preserved before the
//!     narrowing cast to `int`.
//!   * `sci->bitalloc[i]` is read for `i` up to `2*total_bands - 1` (509) even
//!     though `bitalloc` is only 64 bytes, so the read runs through `scfcod`
//!     and past the struct -- unchecked, just like C.
//!   * every shift uses the wrapping (`& 31`) form the C compiles to on x86,
//!     since `1 << (ba-1)`, `2 << (ba-17)`, `next << shl` and `next >> -shl`
//!     can all be handed counts >= 32.

#![allow(non_camel_case_types, non_snake_case)]

use std::ffi::c_int;

/// `typedef struct { const uint8_t *buf; int pos, limit; } bs_t;`
#[repr(C)]
pub struct bs_t {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

/// ```c
/// typedef struct {
///     float scf[3 * 64];
///     uint8_t total_bands, stereo_bands, bitalloc[64], scfcod[64];
/// } L12_scale_info;
/// ```
#[repr(C)]
pub struct L12_scale_info {
    pub scf: [f32; 3 * 64],
    pub total_bands: u8,
    pub stereo_bands: u8,
    pub bitalloc: [u8; 64],
    pub scfcod: [u8; 64],
}

/// Translation of the `static uint32_t get_bits(bs_t *bs, int n)` helper.
///
/// ```c
/// static uint32_t get_bits(bs_t *bs, int n) {
///     uint32_t next, cache = 0, s = bs->pos & 7;
///     int shl = n + s;
///     const uint8_t *p = bs->buf + (bs->pos >> 3);
///     if ((bs->pos += n) > bs->limit)
///         return 0;
///     next = *p++ & (255 >> s);
///     while ((shl -= 8) > 0) {
///         cache |= next << shl;
///         next = *p++;
///     }
///     return cache | (next >> -shl);
/// }
/// ```
///
/// # Safety
/// `bs` must be a valid pointer; `bs->buf` must be readable for the bits the
/// caller requests (the C code has the same, unchecked, requirement).
unsafe fn get_bits(bs: *mut bs_t, n: c_int) -> u32 {
    unsafe {
        let mut cache: u32 = 0;
        // `s = bs->pos & 7` is always in 0..=7, even for a negative `pos`.
        let s: u32 = ((*bs).pos & 7) as u32;
        // `int shl = n + s;` -- computed in `unsigned`, stored back into `int`.
        let mut shl: i32 = (n as u32).wrapping_add(s) as i32;
        let mut p: *const u8 = (*bs).buf.wrapping_offset(((*bs).pos >> 3) as isize);

        // NOTE: `pos` is advanced before (and regardless of) the limit test.
        (*bs).pos = (*bs).pos.wrapping_add(n);
        if (*bs).pos > (*bs).limit {
            return 0;
        }

        let mut next: u32 = (*p as u32) & (255u32 >> s);
        p = p.wrapping_add(1);

        loop {
            shl = shl.wrapping_sub(8);
            if shl <= 0 {
                break;
            }
            cache |= next.wrapping_shl(shl as u32);
            next = *p as u32;
            p = p.wrapping_add(1);
        }

        // `-shl` is in 0..=8 here; a shift of 32+ can only arise from an absurd
        // `n`, and `wrapping_shr` then mirrors what the C compiles to on x86.
        cache | next.wrapping_shr(shl.wrapping_neg() as u32)
    }
}

/// ```c
/// int dequantize_granule(float *grbuf, bs_t *bs, L12_scale_info *sci,
///                        int group_size);
/// ```
///
/// # Safety
/// All three pointers must be valid; `grbuf` must be large enough for the
/// (unchecked) strides the C code performs.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut bs_t,
    sci: *mut L12_scale_info,
    group_size: c_int,
) -> c_int {
    unsafe {
        // `choff` is deliberately initialised *outside* the `j` loop, matching C.
        let mut choff: c_int = 576;

        for j in 0..4i32 {
            let mut dst: *mut f32 = grbuf.wrapping_offset((group_size.wrapping_mul(j)) as isize);

            // `sci->bitalloc[i]` is an UNCHECKED C array access: the loop bound is
            // `2 * total_bands` (up to 510) while `bitalloc` is only 64 bytes, so C
            // happily reads into `scfcod` and past the end of the struct. Reproduce
            // that with raw pointer arithmetic -- a slice index would panic here.
            // Derived from the *struct* base so that the whole `L12_scale_info`
            // object (incl. `scfcod`) is in-provenance, exactly like C.
            let bitalloc: *const u8 = (sci as *const u8)
                .wrapping_add(core::mem::offset_of!(L12_scale_info, bitalloc));

            let mut i: c_int = 0;
            while i < 2 * (*sci).total_bands as c_int {
                let ba = *bitalloc.wrapping_offset(i as isize) as c_int;
                if ba != 0 {
                    if ba < 17 {
                        // `int half = (1 << (ba - 1)) - 1;`
                        let half: i32 = 1i32.wrapping_shl((ba - 1) as u32).wrapping_sub(1);
                        let mut k: c_int = 0;
                        while k < group_size {
                            let v = (get_bits(bs, ba) as i32).wrapping_sub(half);
                            *dst.wrapping_offset(k as isize) = v as f32;
                            k += 1;
                        }
                    } else {
                        // `unsigned mod = (2 << (ba - 17)) + 1;`  (always odd)
                        let m: u32 = 2u32.wrapping_shl((ba - 17) as u32).wrapping_add(1);
                        // `get_bits(bs, mod + 2 - (mod >> 3))` -- unsigned math,
                        // then narrowed to the `int` parameter.
                        let nbits = m.wrapping_add(2).wrapping_sub(m >> 3) as c_int;
                        let mut code: u32 = get_bits(bs, nbits);
                        let mut k: c_int = 0;
                        while k < group_size {
                            // Unsigned wrap-around, *then* the cast to `int`.
                            let v = (code % m).wrapping_sub(m / 2) as i32;
                            *dst.wrapping_offset(k as isize) = v as f32;
                            k += 1;
                            code /= m;
                        }
                    }
                }
                dst = dst.wrapping_offset(choff as isize);
                choff = 18 - choff;
                i += 1;
            }
        }

        group_size.wrapping_mul(4)
    }
}
