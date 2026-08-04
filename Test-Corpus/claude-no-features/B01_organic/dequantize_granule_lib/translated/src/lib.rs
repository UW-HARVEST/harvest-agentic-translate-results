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

fn get_bits(bs: &mut bs_t, n: c_int) -> u32 {
    let s: u32 = (bs.pos & 7) as u32;
    let mut shl: i32 = (n as i32).wrapping_add(s as i32);
    let mut p_offset: isize = (bs.pos >> 3) as isize;
    bs.pos = bs.pos.wrapping_add(n);
    if bs.pos > bs.limit {
        return 0;
    }
    let mut next: u32 = (unsafe { *bs.buf.offset(p_offset) } as u32) & (255u32 >> s);
    p_offset += 1;
    let mut cache: u32 = 0;
    loop {
        shl = shl.wrapping_sub(8);
        if shl <= 0 {
            break;
        }
        cache |= next.wrapping_shl(shl as u32);
        next = unsafe { *bs.buf.offset(p_offset) } as u32;
        p_offset += 1;
    }
    cache | next.wrapping_shr((-shl) as u32)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut bs_t,
    sci: *mut L12_scale_info,
    group_size: c_int,
) -> c_int {
    let bs = &mut *bs;
    let sci = &mut *sci;
    let mut choff: isize = 576;
    for j in 0..4i32 {
        let mut dst_offset: isize = (group_size.wrapping_mul(j)) as isize;
        let n_iter: i32 = 2i32.wrapping_mul(sci.total_bands as i32);
        for i in 0..n_iter {
            let ba: i32 = sci.bitalloc[i as usize] as i32;
            if ba != 0 {
                if ba < 17 {
                    let half: i32 = (1i32.wrapping_shl((ba - 1) as u32)).wrapping_sub(1);
                    for k in 0..group_size {
                        let v = get_bits(bs, ba) as i32;
                        let val = (v.wrapping_sub(half)) as f32;
                        *grbuf.offset(dst_offset + k as isize) = val;
                    }
                } else {
                    let mod_: u32 = (2u32.wrapping_shl((ba - 17) as u32)).wrapping_add(1);
                    let n_bits: i32 = mod_.wrapping_add(2).wrapping_sub(mod_ >> 3) as i32;
                    let mut code: u32 = get_bits(bs, n_bits);
                    for k in 0..group_size {
                        let val = ((code % mod_).wrapping_sub(mod_ / 2)) as i32 as f32;
                        *grbuf.offset(dst_offset + k as isize) = val;
                        code /= mod_;
                    }
                }
            }
            dst_offset += choff;
            choff = 18 - choff;
        }
    }
    group_size.wrapping_mul(4)
}
