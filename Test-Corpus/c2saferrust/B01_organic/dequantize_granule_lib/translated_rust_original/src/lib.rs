

pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct bs_t {
    pub buf: *const uint8_t,
    pub pos: ::core::ffi::c_int,
    pub limit: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct L12_scale_info {
    pub scf: [::core::ffi::c_float; 192],
    pub total_bands: uint8_t,
    pub stereo_bands: uint8_t,
    pub bitalloc: [uint8_t; 64],
    pub scfcod: [uint8_t; 64],
}
fn get_bits(bs: *mut bs_t, n: i32) -> u32 {
    let bs = match unsafe { bs.as_mut() } {
        Some(bs) => bs,
        None => return 0,
    };

    let s = (bs.pos & 7) as u32;
    let mut shl = n + s as i32;
    let byte_index = (bs.pos >> 3) as usize;

    bs.pos += n;
    if bs.pos > bs.limit {
        return 0;
    }

    let mut p = byte_index;
    let mut cache = 0u32;

    let mut next = unsafe { (*bs.buf.add(p)) as u32 } >> s;
    p += 1;

    loop {
        shl -= 8;
        if shl <= 0 {
            break;
        }
        cache |= next << shl;
        next = unsafe { (*bs.buf.add(p)) as u32 };
        p += 1;
    }

    cache | (next >> (-shl))
}

#[no_mangle]
pub fn dequantize_granule(
    grbuf: &mut [f32],
    bs: &mut bs_t,
    sci: &L12_scale_info,
    group_size: i32,
) -> i32 {
    let mut choff: i32 = 576;

    for j in 0..4 {
        let base = (group_size * j) as usize;

        for i in 0..(2 * sci.total_bands as i32) {
            let ba = sci.bitalloc[i as usize] as i32;
            let band_base = if choff == 576 { base } else { base + choff as usize };

            if ba != 0 {
                if ba < 17 {
                    let half = (1 << (ba - 1)) - 1;
                    for k in 0..group_size {
                        grbuf[band_base + k as usize] = (get_bits(bs, ba) as i32 - half) as f32;
                    }
                } else {
                    let modulus = (((2 << (ba - 17)) + 1) as u32);
                    let mut code = get_bits(
                        bs,
                        modulus
                            .wrapping_add(2)
                            .wrapping_sub(modulus >> 3) as i32,
                    ) as u32;

                    for k in 0..group_size {
                        grbuf[band_base + k as usize] = code
                            .wrapping_rem(modulus)
                            .wrapping_sub(modulus / 2) as i32 as f32;
                        code = code.wrapping_div(modulus);
                    }
                }
            }

            choff = 18 - choff;
        }
    }

    group_size * 4
}

