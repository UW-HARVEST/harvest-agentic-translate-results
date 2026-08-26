use std::ffi::{c_float, c_int, c_uint, c_void};
use std::slice;

#[repr(C)]
pub struct bs_t {
    pub buf: *const u8,
    pub pos: c_int,
    pub limit: c_int,
}

#[repr(C)]
pub struct L12_scale_info {
    pub scf: [c_float; 192],
    pub total_bands: u8,
    pub stereo_bands: u8,
    pub bitalloc: [u8; 64],
    pub scfcod: [u8; 64],
}

fn get_bits(bs: &mut bs_t, n: c_int) -> u32 {
    let s = (bs.pos & 7) as u32;
    let shl = (n + bs.pos as c_int) as i32;
    let mut p = unsafe { bs.buf.add((bs.pos >> 3) as usize) };
    bs.pos += n;
    if bs.pos > bs.limit {
        return 0;
    }
    let mut cache: u32 = 0;
    let mut next = unsafe { (*p & (255 >> s)) as u32 };
    p = unsafe { p.add(1) };
    let mut remaining = shl - 8;
    while remaining > 0 {
        cache |= next << remaining;
        next = unsafe { *p } as u32;
        p = unsafe { p.add(1) };
        remaining -= 8;
    }
    cache | (next >> (-remaining) as u32)
}

#[unsafe(no_mangle)]
pub extern "C" fn dequantize_granule(
    grbuf: *mut c_float,
    bs: *mut bs_t,
    sci: *mut L12_scale_info,
    group_size: c_int,
) -> c_int {
    let bs = unsafe { &mut *bs };
    let sci = unsafe { &*sci };
    let grbuf = unsafe { slice::from_raw_parts_mut(grbuf, 576 * 2) };
    let group_size = group_size as usize;
    let total_bands = sci.total_bands as usize;
    let mut choff: usize = 576;
    for j in 0..4 {
        let base = group_size * j;
        for i in 0..(2 * total_bands) {
            let ba = sci.bitalloc[i] as c_int;
            if ba != 0 {
                let dst_idx = base;
                if ba < 17 {
                    let half = ((1 << (ba - 1)) - 1) as i32;
                    for k in 0..group_size {
                        let val = get_bits(bs, ba) as i32 - half;
                        grbuf[dst_idx + k + choff] = val as c_float;
                    }
                } else {
                    let mod_val = ((2 << (ba - 17)) + 1) as u32;
                    let bits = (mod_val + 2 - (mod_val >> 3)) as c_int;
                    let mut code = get_bits(bs, bits);
                    for k in 0..group_size {
                        let val = (code % mod_val) as i32 - (mod_val / 2) as i32;
                        grbuf[dst_idx + k + choff] = val as c_float;
                        code /= mod_val;
                    }
                }
            }
            choff = 18 - choff;
        }
    }
    (group_size * 4) as c_int
}
