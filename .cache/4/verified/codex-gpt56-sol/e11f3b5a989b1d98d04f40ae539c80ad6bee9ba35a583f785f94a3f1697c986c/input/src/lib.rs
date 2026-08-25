use std::ffi::c_int;

#[repr(C)]
pub struct Bs {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

#[repr(C)]
pub struct L12ScaleInfo {
    pub scf: [f32; 3 * 64],
    pub total_bands: u8,
    pub stereo_bands: u8,
    pub bitalloc: [u8; 64],
    pub scfcod: [u8; 64],
}

unsafe fn get_bits(bs: *mut Bs, n: c_int) -> u32 {
    let bs = unsafe { &mut *bs };
    let mut cache = 0_u32;
    let s = bs.pos & 7;
    let mut shl = n.wrapping_add(s);
    let mut p = unsafe { bs.buf.offset((bs.pos >> 3) as isize) };

    bs.pos = bs.pos.wrapping_add(n);
    if bs.pos > bs.limit {
        return 0;
    }

    let mut next = u32::from(unsafe { *p }) & (255_u32 >> s);
    p = unsafe { p.add(1) };
    loop {
        shl = shl.wrapping_sub(8);
        if shl <= 0 {
            break;
        }
        cache |= next.wrapping_shl(shl as u32);
        next = u32::from(unsafe { *p });
        p = unsafe { p.add(1) };
    }
    cache | (next >> shl.wrapping_neg())
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut Bs,
    sci: *mut L12ScaleInfo,
    group_size: c_int,
) -> c_int {
    let sci = unsafe { &*sci };
    let mut choff = 576_i32;

    for j in 0..4_i32 {
        let mut dst = unsafe { grbuf.offset(group_size.wrapping_mul(j) as isize) };
        for i in 0..(2 * c_int::from(sci.total_bands)) {
            let ba = c_int::from(sci.bitalloc[i as usize]);
            if ba != 0 {
                if ba < 17 {
                    let half = (1_i32 << (ba - 1)) - 1;
                    for k in 0..group_size {
                        let sample = unsafe { get_bits(bs, ba) } as i32 - half;
                        unsafe { *dst.offset(k as isize) = sample as f32 };
                    }
                } else {
                    let modulus = 2_u32.wrapping_shl((ba - 17) as u32) + 1;
                    let width = modulus + 2 - (modulus >> 3);
                    let mut code = unsafe { get_bits(bs, width as c_int) };
                    for k in 0..group_size {
                        let sample = (code % modulus).wrapping_sub(modulus / 2) as i32;
                        unsafe { *dst.offset(k as isize) = sample as f32 };
                        code /= modulus;
                    }
                }
            }
            dst = unsafe { dst.offset(choff as isize) };
            choff = 18 - choff;
        }
    }

    group_size.wrapping_mul(4)
}
