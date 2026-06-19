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

unsafe fn get_bits(bs: *mut bs_t, n: c_int) -> u32 {
    let bs_ref = unsafe { &mut *bs };
    let mut cache = 0u32;
    let s = bs_ref.pos & 7;
    let mut shl = n + s;
    let mut p = unsafe { bs_ref.buf.add((bs_ref.pos >> 3) as usize) };

    bs_ref.pos += n;
    if bs_ref.pos > bs_ref.limit {
        return 0;
    }

    let mut next = (unsafe { *p } & (255u8 >> s)) as u32;
    p = unsafe { p.add(1) };

    loop {
        shl -= 8;
        if shl <= 0 {
            break;
        }
        cache |= next << shl;
        next = unsafe { *p } as u32;
        p = unsafe { p.add(1) };
    }

    cache | (next >> (-shl))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut bs_t,
    sci: *mut L12_scale_info,
    group_size: c_int,
) -> c_int {
    let sci_ref = unsafe { &mut *sci };
    let mut choff: c_int = 576;

    let mut j = 0;
    while j < 4 {
        let mut dst = unsafe { grbuf.offset((group_size * j) as isize) };
        let mut i = 0;
        while i < 2 * sci_ref.total_bands as c_int {
            let ba = sci_ref.bitalloc[i as usize] as c_int;
            if ba != 0 {
                if ba < 17 {
                    let half = (1 << (ba - 1)) - 1;
                    let mut k = 0;
                    while k < group_size {
                        let value = unsafe { get_bits(bs, ba) } as c_int - half;
                        unsafe {
                            *dst.offset(k as isize) = value as f32;
                        }
                        k += 1;
                    }
                } else {
                    let mod_ = (2u32 << ((ba - 17) as u32)) + 1;
                    let mut code =
                        unsafe { get_bits(bs, (mod_ + 2 - (mod_ >> 3)) as c_int) };
                    let mut k = 0;
                    while k < group_size {
                        let value = (code % mod_) as c_int - (mod_ / 2) as c_int;
                        unsafe {
                            *dst.offset(k as isize) = value as f32;
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

        j += 1;
    }

    group_size * 4
}
