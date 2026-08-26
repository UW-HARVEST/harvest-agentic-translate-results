use std::os::raw::{c_float, c_int};

#[repr(C)]
pub struct bs_t {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

#[repr(C)]
pub struct L12_scale_info {
    pub scf: [c_float; 3 * 64],
    pub total_bands: u8,
    pub stereo_bands: u8,
    pub bitalloc: [u8; 64],
    pub scfcod: [u8; 64],
}

unsafe fn get_bits(bs: *mut bs_t, n: c_int) -> u32 {
    let mut cache: u32 = 0;
    let s = (*bs).pos & 7;
    let mut shl = n + s;
    let mut p = (*bs).buf.add(((*bs).pos >> 3) as usize);
    
    (*bs).pos += n;
    if (*bs).pos > (*bs).limit {
        return 0;
    }
    
    let mut next = (*p as u32) & (255_u32 >> (s as u32));
    p = p.add(1);
    
    shl -= 8;
    while shl > 0 {
        cache |= next << (shl as u32);
        next = *p as u32;
        p = p.add(1);
        shl -= 8;
    }
    
    cache | (next >> ((-shl) as u32))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut c_float,
    bs: *mut bs_t,
    sci: *mut L12_scale_info,
    group_size: c_int,
) -> c_int {
    let mut choff: isize = 576;
    for j in 0..4 {
        let mut dst = grbuf.add((group_size * j) as usize);
        for i in 0..(2 * (*sci).total_bands as usize) {
            let ba = (*sci).bitalloc[i] as c_int;
            if ba != 0 {
                if ba < 17 {
                    let half = (1_i32 << ((ba - 1) as u32)) - 1;
                    for k in 0..group_size {
                        *dst.add(k as usize) = (get_bits(bs, ba) as i32 - half) as c_float;
                    }
                } else {
                    let mod_val = (2_u32 << ((ba - 17) as u32)) + 1;
                    let mut code = get_bits(bs, (mod_val + 2 - (mod_val >> 3)) as c_int);
                    for k in 0..group_size {
                        *dst.add(k as usize) = ((code % mod_val) as i32 - (mod_val / 2) as i32) as c_float;
                        code /= mod_val;
                    }
                }
            }
            dst = dst.offset(choff);
            choff = 18 - choff;
        }
    }
    group_size * 4
}
