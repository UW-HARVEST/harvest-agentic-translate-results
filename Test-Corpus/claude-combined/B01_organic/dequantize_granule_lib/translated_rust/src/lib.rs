#![allow(non_camel_case_types)]

use std::ffi::c_int;

#[repr(C)]
pub struct bs_t {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

#[repr(C)]
pub struct L12_scale_info {
    pub scf: [f32; 3 * 64],
    pub total_bands: u8,
    pub stereo_bands: u8,
    pub bitalloc: [u8; 64],
    pub scfcod: [u8; 64],
}

/// Mirrors `static uint32_t get_bits(bs_t *bs, int n)` from the C source.
///
/// Reads `n` bits from the bitstream. `bs->pos` is advanced by `n` regardless
/// of whether the read succeeds. If the new position exceeds `bs->limit`, the
/// function returns 0 (still leaving `pos` advanced).
///
/// # Safety
/// `bs` must point to a valid `bs_t`. `bs.buf` must point to at least
/// `(bs.limit + 7) / 8` bytes of valid memory.
unsafe fn get_bits(bs: *mut bs_t, n: c_int) -> u32 {
    let s: c_int = (*bs).pos & 7;
    let mut shl: c_int = n + s;
    let mut p: *const u8 = (*bs).buf.offset(((*bs).pos >> 3) as isize);
    (*bs).pos += n;
    if (*bs).pos > (*bs).limit {
        return 0;
    }
    let mut cache: u32 = 0;
    // next = *p++ & (255 >> s);
    let mut next: u32 = (*p as u32) & (255u32 >> s);
    p = p.add(1);
    // while ((shl -= 8) > 0) { cache |= next << shl; next = *p++; }
    loop {
        shl -= 8;
        if shl <= 0 {
            break;
        }
        cache |= next << shl;
        next = *p as u32;
        p = p.add(1);
    }
    // return cache | (next >> -shl);
    cache | (next >> (-shl))
}

/// Public C API. Mirrors `int dequantize_granule(float *grbuf, bs_t *bs,
/// L12_scale_info *sci, int group_size)` from the C source.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut bs_t,
    sci: *mut L12_scale_info,
    group_size: c_int,
) -> c_int {
    let mut choff: c_int = 576;
    for j in 0..4_i32 {
        let mut dst: *mut f32 = grbuf.offset((group_size * j) as isize);
        let total_bands = (*sci).total_bands as c_int;
        let band_count = 2 * total_bands;
        for i in 0..band_count {
            let ba = (*sci).bitalloc[i as usize] as c_int;
            if ba != 0 {
                if ba < 17 {
                    let half: c_int = (1 << (ba - 1)) - 1;
                    for k in 0..group_size {
                        let raw = get_bits(bs, ba);
                        let signed = (raw as i32).wrapping_sub(half);
                        *dst.offset(k as isize) = signed as f32;
                    }
                } else {
                    let mod_: u32 = (2u32 << (ba - 17)) + 1;
                    let bits_to_read = (mod_ + 2 - (mod_ >> 3)) as c_int;
                    let mut code: u32 = get_bits(bs, bits_to_read);
                    for k in 0..group_size {
                        // (int)(code % mod - mod / 2) — wrapping unsigned subtract,
                        // then bit-cast to signed before promoting to float.
                        let diff_u = (code % mod_).wrapping_sub(mod_ / 2);
                        let diff_i = diff_u as i32;
                        *dst.offset(k as isize) = diff_i as f32;
                        code /= mod_;
                    }
                }
            }
            dst = dst.offset(choff as isize);
            choff = 18 - choff;
        }
    }
    group_size * 4
}
