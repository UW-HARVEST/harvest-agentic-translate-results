//! Rust translation of the C library in `c_src/`.
//!
//! Mirrors `c_src/include/lib.h` (public types) and `c_src/src/lib.c`
//! (implementation).  The only symbol exported by the C shared library is
//! `dequantize_granule`; `get_bits` is `static` in the C translation unit and
//! therefore stays private here as well.
//!
//! All arithmetic reproduces the observable behaviour of the C code as compiled
//! by GCC on x86-64 (wrapping integer arithmetic, shift counts masked to 5
//! bits, arithmetic right shift of signed values, unsigned wrap-around before
//! the conversion to `int`).

#![allow(non_camel_case_types)]

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
unsafe fn get_bits(bs: *mut bs_t, n: c_int) -> u32 {
    let mut cache: u32 = 0;
    // `bs->pos & 7`: bitwise and on the two's complement representation, so the
    // result is always in 0..=7 even for a negative `pos`.
    let s: u32 = ((*bs).pos & 7) as u32;
    // `int shl = n + s;`  -- `n` is converted to `unsigned` for the addition,
    // the (possibly wrapped) result is converted back to `int`.
    let mut shl: c_int = (n as u32).wrapping_add(s) as c_int;
    // `bs->buf + (bs->pos >> 3)`: `>>` on a signed int is an arithmetic shift.
    let mut p: *const u8 = (*bs).buf.wrapping_offset(((*bs).pos >> 3) as isize);
    (*bs).pos = (*bs).pos.wrapping_add(n);
    if (*bs).pos > (*bs).limit {
        return 0;
    }
    let mut next: u32 = (*p as u32) & (255u32 >> s);
    p = p.wrapping_offset(1);
    loop {
        shl = shl.wrapping_sub(8);
        if shl <= 0 {
            break;
        }
        cache |= next.wrapping_shl(shl as u32);
        next = *p as u32;
        p = p.wrapping_offset(1);
    }
    cache | next.wrapping_shr(shl.wrapping_neg() as u32)
}

/// ```c
/// int dequantize_granule(float *grbuf, bs_t *bs, L12_scale_info *sci,
///                                   int group_size) {
///     int i, j, k, choff = 576;
///     for (j = 0; j < 4; j++) {
///         float *dst = grbuf + group_size * j;
///         for (i = 0; i < 2 * sci->total_bands; i++) {
///             int ba = sci->bitalloc[i];
///             if (ba != 0) {
///                 if (ba < 17) {
///                     int half = (1 << (ba - 1)) - 1;
///                     for (k = 0; k < group_size; k++) {
///                         dst[k] = (float)((int)get_bits(bs, ba) - half);
///                     }
///                 } else {
///                     unsigned mod = (2 << (ba - 17)) + 1;
///                     unsigned code = get_bits(bs, mod + 2 - (mod >> 3));
///                     for (k = 0; k < group_size; k++, code /= mod) {
///                         dst[k] = (float)((int)(code % mod - mod / 2));
///                     }
///                 }
///             }
///             dst += choff;
///             choff = 18 - choff;
///         }
///     }
///     return group_size * 4;
/// }
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut bs_t,
    sci: *mut L12_scale_info,
    group_size: c_int,
) -> c_int {
    // `choff` is initialised once, outside of the `j` loop, and keeps its value
    // across granule iterations.
    let mut choff: c_int = 576;
    let mut j: c_int = 0;
    while j < 4 {
        let mut dst: *mut f32 = grbuf.wrapping_offset(group_size.wrapping_mul(j) as isize);
        // Base of `sci->bitalloc`; indexed with a raw offset so that an
        // out-of-range `total_bands` reproduces the C code's out-of-bounds
        // reads into the following struct members.
        let bitalloc: *const u8 = std::ptr::addr_of!((*sci).bitalloc).cast::<u8>();
        let nbands: c_int = 2i32.wrapping_mul((*sci).total_bands as c_int);
        let mut i: c_int = 0;
        while i < nbands {
            let ba: c_int = *bitalloc.wrapping_offset(i as isize) as c_int;
            if ba != 0 {
                if ba < 17 {
                    let half: c_int = 1i32.wrapping_shl((ba - 1) as u32).wrapping_sub(1);
                    let mut k: c_int = 0;
                    while k < group_size {
                        let v = (get_bits(bs, ba) as c_int).wrapping_sub(half);
                        *dst.wrapping_offset(k as isize) = v as f32;
                        k += 1;
                    }
                } else {
                    // `2 << (ba - 17)`: signed shift whose count is masked to
                    // five bits on x86-64.  `mod` is always odd, hence never 0.
                    let m: u32 = (2i32.wrapping_shl((ba - 17) as u32) as u32).wrapping_add(1);
                    let mut code: u32 =
                        get_bits(bs, m.wrapping_add(2).wrapping_sub(m >> 3) as c_int);
                    let mut k: c_int = 0;
                    while k < group_size {
                        let v = (code % m).wrapping_sub(m / 2) as c_int;
                        *dst.wrapping_offset(k as isize) = v as f32;
                        k += 1;
                        code /= m;
                    }
                }
            }
            dst = dst.wrapping_offset(choff as isize);
            choff = 18i32.wrapping_sub(choff);
            i += 1;
        }
        j += 1;
    }
    group_size.wrapping_mul(4)
}
