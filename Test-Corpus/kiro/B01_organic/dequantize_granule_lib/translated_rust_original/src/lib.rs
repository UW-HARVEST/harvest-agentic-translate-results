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
    let mut cache: u32 = 0;
    let s = (*bs).pos & 7;
    let mut shl = n + s;
    let p_start = (*bs).buf.offset(((*bs).pos >> 3) as isize);
    let mut p = p_start;

    (*bs).pos += n;
    if (*bs).pos > (*bs).limit {
        return 0;
    }

    let mut next: u32 = (*p as u32) & (255u32 >> s as u32);
    p = p.offset(1);

    shl -= 8;
    while shl > 0 {
        cache |= next << shl as u32;
        next = *p as u32;
        p = p.offset(1);
        shl -= 8;
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
    let mut choff: c_int;
    for j in 0..4 {
        let mut dst = grbuf.offset((group_size * j) as isize);
        choff = 576;
        for i in 0..2 * (*sci).total_bands as c_int {
            let ba = (*sci).bitalloc[i as usize] as c_int;
            if ba != 0 {
                if ba < 17 {
                    let half = (1i32 << (ba - 1)) - 1;
                    for k in 0..group_size {
                        *dst.offset(k as isize) = (get_bits(bs, ba) as i32 - half) as f32;
                    }
                } else {
                    let mod_val: u32 = (2u32 << (ba - 17) as u32).wrapping_add(1);
                    let mut code: u32 = get_bits(bs, (mod_val as c_int) + 2 - ((mod_val >> 3) as c_int));
                    for k in 0..group_size {
                        *dst.offset(k as isize) = (code % mod_val).wrapping_sub(mod_val / 2) as i32 as f32;
                        code /= mod_val;
                    }
                }
            }
            dst = dst.offset(choff as isize);
            choff = 18 - choff;
        }
    }
    group_size * 4
}
