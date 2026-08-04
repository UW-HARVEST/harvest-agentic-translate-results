#[repr(C)]
pub struct BsT {
    pub buf: *const u8,
    pub pos: i32,
    pub limit: i32,
}

#[repr(C)]
pub struct L12ScaleInfo {
    pub scf: [f32; 3 * 64],
    pub total_bands: u8,
    pub stereo_bands: u8,
    pub bitalloc: [u8; 64],
    pub scfcod: [u8; 64],
}

unsafe fn get_bits(bs: *mut BsT, n: i32) -> u32 {
    let mut cache: u32 = 0;
    let s: i32 = (*bs).pos & 7;
    let mut shl: i32 = n + s;
    let mut p: *const u8 = (*bs).buf.offset(((*bs).pos >> 3) as isize);
    (*bs).pos += n;
    if (*bs).pos > (*bs).limit {
        return 0;
    }
    let mut next: u32 = (*p as u32) & (255u32 >> s);
    p = p.add(1);
    shl -= 8;
    while shl > 0 {
        cache |= next << shl;
        next = *p as u32;
        p = p.add(1);
        shl -= 8;
    }
    // shl <= 0 here, so -shl >= 0
    cache | (next >> (-shl))
}

/// # Safety
/// `grbuf` must point to a buffer large enough for the writes performed,
/// `bs` must be a valid pointer to a `BsT`, and `sci` must be a valid pointer
/// to an `L12ScaleInfo`.
#[no_mangle]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut BsT,
    sci: *mut L12ScaleInfo,
    group_size: i32,
) -> i32 {
    let mut choff: i32 = 576;
    for j in 0..4i32 {
        let mut dst: *mut f32 = grbuf.offset((group_size * j) as isize);
        let total_bands = (*sci).total_bands as i32;
        for i in 0..(2 * total_bands) {
            let ba = (*sci).bitalloc[i as usize] as i32;
            if ba != 0 {
                if ba < 17 {
                    let half: i32 = (1i32 << (ba - 1)) - 1;
                    for k in 0..group_size {
                        let bits = get_bits(bs, ba) as i32;
                        *dst.offset(k as isize) = (bits - half) as f32;
                    }
                } else {
                    let mod_val: u32 = (2u32 << (ba - 17)) + 1;
                    let nbits: i32 = (mod_val + 2 - (mod_val >> 3)) as i32;
                    let mut code: u32 = get_bits(bs, nbits);
                    for k in 0..group_size {
                        let val = (code % mod_val) as i32 - (mod_val / 2) as i32;
                        *dst.offset(k as isize) = val as f32;
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
