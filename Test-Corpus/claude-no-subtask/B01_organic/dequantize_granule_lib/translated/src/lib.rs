#![allow(non_camel_case_types)]

use std::ffi::c_int;
use std::ptr;

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

unsafe fn get_bits(bs: *mut bs_t, n: c_int) -> u32 {
    unsafe {
        let s: u32 = ((*bs).pos & 7) as u32;
        let mut shl: c_int = n + s as c_int;
        let mut p: *const u8 = (*bs).buf.wrapping_add(((*bs).pos >> 3) as usize);
        (*bs).pos += n;
        if (*bs).pos > (*bs).limit {
            return 0;
        }
        let mut next: u32 = (*p as u32) & (255u32 >> s);
        p = p.wrapping_add(1);
        let mut cache: u32 = 0;
        loop {
            shl -= 8;
            if shl <= 0 {
                break;
            }
            cache |= next << (shl as u32);
            next = *p as u32;
            p = p.wrapping_add(1);
        }
        cache | (next >> ((0i32.wrapping_sub(shl)) as u32))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut bs_t,
    sci: *mut L12_scale_info,
    group_size: c_int,
) -> c_int {
    unsafe {
        let mut choff: c_int = 576;
        let total_bands = (*sci).total_bands as c_int;
        let bitalloc_ptr = ptr::addr_of!((*sci).bitalloc) as *const u8;
        for j in 0..4i32 {
            let mut dst: *mut f32 = grbuf.wrapping_offset((group_size * j) as isize);
            for i in 0..(2 * total_bands) {
                let ba = *bitalloc_ptr.wrapping_offset(i as isize) as c_int;
                if ba != 0 {
                    if ba < 17 {
                        let half: c_int = (1i32 << (ba - 1)) - 1;
                        for k in 0..group_size {
                            let bits = get_bits(bs, ba) as i32;
                            *dst.wrapping_offset(k as isize) = (bits - half) as f32;
                        }
                    } else {
                        let mod_val: u32 = (2u32 << (ba - 17)) + 1;
                        let mut code: u32 =
                            get_bits(bs, (mod_val + 2 - (mod_val >> 3)) as c_int);
                        for k in 0..group_size {
                            let val = (code % mod_val).wrapping_sub(mod_val / 2);
                            *dst.wrapping_offset(k as isize) = (val as i32) as f32;
                            code /= mod_val;
                        }
                    }
                }
                dst = dst.wrapping_offset(choff as isize);
                choff = 18 - choff;
            }
        }
        group_size * 4
    }
}
