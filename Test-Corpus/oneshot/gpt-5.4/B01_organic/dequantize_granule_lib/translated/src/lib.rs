use std::os::raw::c_int;

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
    let mut cache: u32 = 0;
    let s = (bs.pos & 7) as u32;
    let mut shl = n + s as c_int;
    let byte_offset = (bs.pos >> 3) as isize;
    let mut p = unsafe { bs.buf.offset(byte_offset) };
    bs.pos += n;
    if bs.pos > bs.limit {
        return 0;
    }
    let mut next = unsafe { *p } as u32 & (255u32 >> s);
    p = unsafe { p.offset(1) };
    shl -= 8;
    while shl > 0 {
        cache |= next << shl;
        next = unsafe { *p } as u32;
        p = unsafe { p.offset(1) };
        shl -= 8;
    }
    cache | (next >> (-shl) as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut bs_t,
    sci: *mut L12_scale_info,
    group_size: c_int,
) -> c_int {
    if grbuf.is_null() || bs.is_null() || sci.is_null() {
        return 0;
    }

    let bs = unsafe { &mut *bs };
    let sci = unsafe { &mut *sci };
    let group_size_usize = match usize::try_from(group_size) {
        Ok(v) => v,
        Err(_) => return 0,
    };

    let mut choff: isize = 576;
    for j in 0..4usize {
        let mut dst = unsafe { grbuf.add(group_size_usize * j) };
        let total = usize::from(sci.total_bands) * 2;
        for i in 0..total {
            let ba = sci.bitalloc[i];
            if ba != 0 {
                if ba < 17 {
                    let half = (1i32 << (ba - 1)) - 1;
                    for k in 0..group_size_usize {
                        let value = get_bits(bs, ba as c_int) as i32 - half;
                        unsafe {
                            *dst.add(k) = value as f32;
                        }
                    }
                } else {
                    let modv = (2u32 << (ba - 17)) + 1;
                    let mut code = get_bits(bs, (modv + 2 - (modv >> 3)) as c_int);
                    for k in 0..group_size_usize {
                        let value = (code % modv) as i32 - (modv / 2) as i32;
                        unsafe {
                            *dst.add(k) = value as f32;
                        }
                        code /= modv;
                    }
                }
            }
            dst = unsafe { dst.offset(choff) };
            choff = 18 - choff;
        }
    }
    group_size * 4
}
