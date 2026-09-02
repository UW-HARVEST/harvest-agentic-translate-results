//! Rust translation of the C library in `c_src/`.
//!
//! Public ABI (from `nm -D` on the C shared object):
//!   * `dequantize_granule`
//!
//! `get_bits` is `static` in the C source and therefore not exported; it is
//! kept private here as well.
//!
//! The translation is deliberately literal. In particular it reproduces the
//! C code's quirks rather than fixing them:
//!   * `sci->bitalloc[i]` is indexed with `i` up to `2 * total_bands - 1`
//!     (up to 509) even though `bitalloc` is only 64 bytes wide, so the read
//!     runs off the end of `bitalloc` and into the following `scfcod` field
//!     (and past the end of the struct). Raw pointer arithmetic on a `repr(C)`
//!     struct with the identical layout is used so the same bytes are read.
//!   * The grouped-quantization path computes `code % mod - mod / 2` in
//!     *unsigned* arithmetic and then converts to `int`, so small codes wrap
//!     around to huge unsigned values before becoming negative ints. This is
//!     reproduced with `wrapping_sub` + `as i32`.
//!   * Shift counts that C leaves undefined (`2 << (ba - 17)` for large `ba`,
//!     `next << shl` for large `n`) are reproduced with `wrapping_shl` /
//!     `wrapping_shr`, which mask the count to 5 bits exactly like the x86
//!     shift instructions that the C compiler emits.
//!   * Reads/writes past the ends of `grbuf` and `bs->buf` are performed as-is.
//!
//! All pointer arithmetic uses `wrapping_offset` rather than `offset`. The C
//! computes addresses far outside its objects (`grbuf + group_size * j` with a
//! negative or NULL `grbuf`, `bs->buf + (pos >> 3)` with a huge or negative
//! `pos`, `bitalloc[i]` for `i` up to 509), which `offset` forbids as a UB
//! precondition — a debug-profile build of this crate aborts on it even though
//! the computed address, and the release-profile behaviour, are identical.
//! `wrapping_offset` computes the same address without that precondition.

#![allow(non_camel_case_types)]

use std::ffi::c_int;

/// Mirrors:
/// ```c
/// typedef struct {
///     const uint8_t *buf;
///     int pos, limit;
/// } bs_t;
/// ```
#[repr(C)]
pub struct bs_t {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

/// Mirrors:
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

// Lock the ABI layout of the two structs against the C definitions
// (verified against the C compiler: sizeof(bs_t) == 16,
// sizeof(L12_scale_info) == 900, offsetof(bitalloc) == 770,
// offsetof(scfcod) == 834 on LP64).
const _: () = {
    assert!(std::mem::size_of::<bs_t>() == 16);
    assert!(std::mem::size_of::<L12_scale_info>() == 900);
    assert!(std::mem::offset_of!(L12_scale_info, total_bands) == 768);
    assert!(std::mem::offset_of!(L12_scale_info, stereo_bands) == 769);
    assert!(std::mem::offset_of!(L12_scale_info, bitalloc) == 770);
    assert!(std::mem::offset_of!(L12_scale_info, scfcod) == 834);
};

/// Translation of the `static` C helper:
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
/// Note the ordering: `s` and `p` are derived from the *old* `bs->pos`, and
/// `bs->pos` is advanced by `n` even on the early-out path that returns 0.
unsafe fn get_bits(bs: *mut bs_t, n: c_int) -> u32 {
    let mut cache: u32 = 0;
    // `uint32_t s = bs->pos & 7;`
    let s: u32 = ((*bs).pos & 7) as u32;
    // `int shl = n + s;`  (s converts to int here)
    let mut shl: c_int = n.wrapping_add(s as c_int);
    // `const uint8_t *p = bs->buf + (bs->pos >> 3);`
    let mut p: *const u8 = (*bs).buf.wrapping_offset(((*bs).pos >> 3) as isize);

    // `if ((bs->pos += n) > bs->limit) return 0;`
    (*bs).pos = (*bs).pos.wrapping_add(n);
    if (*bs).pos > (*bs).limit {
        return 0;
    }

    // `next = *p++ & (255 >> s);`
    let mut next: u32 = ((*p as c_int) & (255 >> s)) as u32;
    p = p.wrapping_offset(1);

    // `while ((shl -= 8) > 0) { cache |= next << shl; next = *p++; }`
    loop {
        shl = shl.wrapping_sub(8);
        if shl <= 0 {
            break;
        }
        cache |= next.wrapping_shl(shl as u32);
        next = *p as u32;
        p = p.wrapping_offset(1);
    }

    // `return cache | (next >> -shl);`
    cache | next.wrapping_shr(shl.wrapping_neg() as u32)
}

/// Translation of the single exported C function.
///
/// ```c
/// int dequantize_granule(float *grbuf, bs_t *bs, L12_scale_info *sci,
///                        int group_size);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut bs_t,
    sci: *mut L12_scale_info,
    group_size: c_int,
) -> c_int {
    // `int i, j, k, choff = 576;` -- `choff` lives outside the `j` loop and
    // keeps its value across iterations.
    let mut choff: c_int = 576;

    let mut j: c_int = 0;
    while j < 4 {
        // `float *dst = grbuf + group_size * j;`
        let mut dst: *mut f32 = grbuf.wrapping_offset(group_size.wrapping_mul(j) as isize);

        // `for (i = 0; i < 2 * sci->total_bands; i++)` -- the bound is re-read
        // from the struct on every iteration, as in the C source.
        let mut i: c_int = 0;
        while i < 2 * (*sci).total_bands as c_int {
            // `int ba = sci->bitalloc[i];`
            //
            // Unchecked, so that `i >= 64` reads past `bitalloc` into the
            // adjacent struct bytes exactly like the C does.
            let bitalloc: *const u8 = std::ptr::addr_of!((*sci).bitalloc) as *const u8;
            let ba: c_int = *bitalloc.wrapping_offset(i as isize) as c_int;

            if ba != 0 {
                if ba < 17 {
                    // `int half = (1 << (ba - 1)) - 1;`
                    let half: c_int = 1i32.wrapping_shl((ba - 1) as u32).wrapping_sub(1);
                    let mut k: c_int = 0;
                    while k < group_size {
                        // `dst[k] = (float)((int)get_bits(bs, ba) - half);`
                        let v = (get_bits(bs, ba) as c_int).wrapping_sub(half);
                        *dst.wrapping_offset(k as isize) = v as f32;
                        k += 1;
                    }
                } else {
                    // `unsigned mod = (2 << (ba - 17)) + 1;`
                    let m: u32 = 2i32.wrapping_shl((ba - 17) as u32).wrapping_add(1) as u32;
                    // `unsigned code = get_bits(bs, mod + 2 - (mod >> 3));`
                    let n: c_int = m.wrapping_add(2).wrapping_sub(m >> 3) as c_int;
                    let mut code: u32 = get_bits(bs, n);
                    let mut k: c_int = 0;
                    while k < group_size {
                        // `dst[k] = (float)((int)(code % mod - mod / 2));`
                        // Unsigned subtraction wraps before the cast to int.
                        let v = (code % m).wrapping_sub(m / 2) as c_int;
                        *dst.wrapping_offset(k as isize) = v as f32;
                        // `k++, code /= mod`
                        k += 1;
                        code /= m;
                    }
                }
            }

            // `dst += choff; choff = 18 - choff;`
            dst = dst.wrapping_offset(choff as isize);
            choff = 18 - choff;

            i += 1;
        }

        j += 1;
    }

    // `return group_size * 4;`
    group_size.wrapping_mul(4)
}
