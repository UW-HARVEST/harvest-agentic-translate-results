//! Rust translation of `c_src/src/lib.c` (MP3-style layer-1/2 granule dequantizer).
//!
//! The translation is deliberately literal: pointer arithmetic that walks outside
//! the nominal bounds of the C arrays, the wrapping integer arithmetic and the
//! post-increment bitstream position update are all reproduced exactly, including
//! the quirks (e.g. `bs->pos` is advanced even when the read is rejected, and the
//! `choff` toggle persists across the outer `j` loop).

use std::ffi::c_int;

/// Mirrors the C `bs_t` bit-reader.
#[repr(C)]
pub struct bs_t {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

/// Mirrors the C `L12_scale_info`.
#[repr(C)]
pub struct L12_scale_info {
    pub scf: [f32; 3 * 64],
    pub total_bands: u8,
    pub stereo_bands: u8,
    pub bitalloc: [u8; 64],
    pub scfcod: [u8; 64],
}

/// Reads `n` bits MSB-first from `bs`.
///
/// Faithful port of the C `static uint32_t get_bits(bs_t *bs, int n)`. Note that
/// `bs->pos` is updated before the limit check, so an over-long read still moves
/// the cursor while returning 0.
unsafe fn get_bits(bs: *mut bs_t, n: c_int) -> u32 {
    let s: u32 = (unsafe { (*bs).pos } & 7) as u32;
    let mut shl: c_int = n.wrapping_add(s as c_int);
    // `p` is derived from the *pre-increment* position, exactly as in C.
    let mut p: *const u8 = unsafe { (*bs).buf.offset((((*bs).pos) >> 3) as isize) };

    unsafe {
        (*bs).pos = (*bs).pos.wrapping_add(n);
        if (*bs).pos > (*bs).limit {
            return 0;
        }
    }

    let mut cache: u32 = 0;
    let mut next: u32 = (unsafe { *p } as u32) & (255u32 >> s);
    p = unsafe { p.add(1) };

    loop {
        shl = shl.wrapping_sub(8);
        if shl <= 0 {
            break;
        }
        // C: `cache |= next << shl` on a uint32_t; emulate the hardware shift.
        cache |= next.wrapping_shl(shl as u32);
        next = unsafe { *p } as u32;
        p = unsafe { p.add(1) };
    }

    cache | next.wrapping_shr(shl.wrapping_neg() as u32)
}

/// Faithful port of the C `dequantize_granule`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut bs_t,
    sci: *mut L12_scale_info,
    group_size: c_int,
) -> c_int {
    let mut choff: c_int = 576;

    let total_bands = unsafe { (*sci).total_bands } as c_int;
    let bitalloc: *const u8 = unsafe { (*sci).bitalloc.as_ptr() };

    let mut j: c_int = 0;
    while j < 4 {
        let mut dst: *mut f32 = unsafe { grbuf.offset(group_size.wrapping_mul(j) as isize) };

        let mut i: c_int = 0;
        while i < 2 * total_bands {
            // May read past `bitalloc[64]` when total_bands > 32, just like the C.
            let ba: c_int = unsafe { *bitalloc.offset(i as isize) } as c_int;
            if ba != 0 {
                if ba < 17 {
                    let half: c_int = (1i32.wrapping_shl((ba - 1) as u32)).wrapping_sub(1);
                    let mut k: c_int = 0;
                    while k < group_size {
                        let v = (unsafe { get_bits(bs, ba) } as c_int).wrapping_sub(half);
                        unsafe { *dst.offset(k as isize) = v as f32 };
                        k += 1;
                    }
                } else {
                    let m: u32 = (2u32.wrapping_shl((ba - 17) as u32)).wrapping_add(1);
                    let nbits: c_int = m.wrapping_add(2).wrapping_sub(m >> 3) as c_int;
                    let mut code: u32 = unsafe { get_bits(bs, nbits) };
                    let mut k: c_int = 0;
                    while k < group_size {
                        let v = (code % m).wrapping_sub(m / 2) as c_int;
                        unsafe { *dst.offset(k as isize) = v as f32 };
                        k += 1;
                        code /= m;
                    }
                }
            }
            dst = unsafe { dst.offset(choff as isize) };
            choff = 18 - choff;
            i += 1;
        }
        j += 1;
    }

    group_size.wrapping_mul(4)
}
