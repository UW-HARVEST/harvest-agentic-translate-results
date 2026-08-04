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

unsafe fn get_bits(bs: &mut bs_t, n: c_int) -> u32 {
    let s: u32 = (bs.pos & 7) as u32;
    let mut shl: i32 = n + s as i32;
    let mut p: *const u8 = bs.buf.offset((bs.pos >> 3) as isize);
    bs.pos += n;
    if bs.pos > bs.limit {
        return 0;
    }
    let mut next: u32 = (*p as u32) & (255u32 >> s);
    p = p.offset(1);
    let mut cache: u32 = 0;
    loop {
        shl -= 8;
        if shl <= 0 {
            break;
        }
        cache |= next << shl;
        next = *p as u32;
        p = p.offset(1);
    }
    cache | (next >> ((-shl) as u32))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut bs_t,
    sci: *mut L12_scale_info,
    group_size: c_int,
) -> c_int {
    let bs_ref = &mut *bs;
    let sci_ref = &mut *sci;
    let mut choff: c_int = 576;
    for j in 0..4i32 {
        let mut dst: *mut f32 = grbuf.offset((group_size * j) as isize);
        let total: c_int = 2 * sci_ref.total_bands as c_int;
        for i in 0..total {
            let ba: c_int = sci_ref.bitalloc[i as usize] as c_int;
            if ba != 0 {
                if ba < 17 {
                    let half: c_int = 1i32.wrapping_shl((ba - 1) as u32) - 1;
                    for k in 0..group_size {
                        let v: c_int = (get_bits(bs_ref, ba) as i32).wrapping_sub(half);
                        *dst.offset(k as isize) = v as f32;
                    }
                } else {
                    let modv: u32 = 2u32.wrapping_shl((ba - 17) as u32) + 1;
                    let mut code: u32 =
                        get_bits(bs_ref, (modv + 2 - (modv >> 3)) as c_int);
                    for k in 0..group_size {
                        let v: i32 =
                            (code % modv).wrapping_sub(modv / 2) as i32;
                        *dst.offset(k as isize) = v as f32;
                        code /= modv;
                    }
                }
            }
            dst = dst.offset(choff as isize);
            choff = 18 - choff;
        }
    }
    group_size * 4
}
