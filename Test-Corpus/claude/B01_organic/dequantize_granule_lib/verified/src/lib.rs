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
    let s: c_int = bs.pos & 7;
    let mut shl: c_int = n + s;
    let mut p = unsafe { bs.buf.offset((bs.pos >> 3) as isize) };
    bs.pos += n;
    if bs.pos > bs.limit {
        return 0;
    }
    let mut cache: u32 = 0;
    let mut next: u32 = (unsafe { *p } as u32) & (255u32.wrapping_shr(s as u32));
    p = unsafe { p.offset(1) };
    shl -= 8;
    while shl > 0 {
        cache |= next.wrapping_shl(shl as u32);
        next = unsafe { *p } as u32;
        p = unsafe { p.offset(1) };
        shl -= 8;
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
    let mut choff: c_int = 576;
    let bs_ref = unsafe { &mut *bs };
    let sci_ref = unsafe { &*sci };
    for j in 0..4 {
        let mut dst = unsafe { grbuf.offset((group_size * j) as isize) };
        let bands: c_int = 2 * sci_ref.total_bands as c_int;
        let mut i: c_int = 0;
        while i < bands {
            let ba = sci_ref.bitalloc[i as usize] as c_int;
            if ba != 0 {
                if ba < 17 {
                    let half: c_int = (1i32.wrapping_shl((ba - 1) as u32)) - 1;
                    for k in 0..group_size {
                        let v = unsafe { get_bits(bs_ref, ba) } as i32 - half;
                        unsafe {
                            *dst.offset(k as isize) = v as f32;
                        }
                    }
                } else {
                    let mod_: u32 = 2u32.wrapping_shl((ba - 17) as u32) + 1;
                    let mut code: u32 = unsafe {
                        get_bits(bs_ref, (mod_ + 2 - (mod_ >> 3)) as c_int)
                    };
                    let mut k: c_int = 0;
                    while k < group_size {
                        let v = ((code % mod_).wrapping_sub(mod_ / 2)) as i32;
                        unsafe {
                            *dst.offset(k as isize) = v as f32;
                        }
                        code /= mod_;
                        k += 1;
                    }
                }
            }
            dst = unsafe { dst.offset(choff as isize) };
            choff = 18 - choff;
            i += 1;
        }
    }
    group_size * 4
}
