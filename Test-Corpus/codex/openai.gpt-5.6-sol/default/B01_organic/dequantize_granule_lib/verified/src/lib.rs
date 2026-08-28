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
    let s = unsafe { (*bs).pos } & 7;
    let mut shl = n + s;
    let mut p = unsafe { (*bs).buf.offset(((*bs).pos >> 3) as isize) };

    unsafe { (*bs).pos += n };
    if unsafe { (*bs).pos > (*bs).limit } {
        return 0;
    }

    let mut next = u32::from(unsafe { *p }) & (255_u32 >> s);
    p = unsafe { p.add(1) };
    let mut cache = 0_u32;

    loop {
        shl -= 8;
        if shl <= 0 {
            break;
        }
        cache |= next << shl;
        next = u32::from(unsafe { *p });
        p = unsafe { p.add(1) };
    }

    cache | (next >> -shl)
}

/// # Safety
///
/// The pointers must be valid for the reads and writes performed by the C ABI,
/// according to the buffer sizes and fields supplied by the caller.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dequantize_granule(
    grbuf: *mut f32,
    bs: *mut Bs,
    sci: *mut L12ScaleInfo,
    group_size: c_int,
) -> c_int {
    let mut choff = 576_isize;

    for j in 0..4_isize {
        let mut dst = unsafe { grbuf.offset((group_size as isize) * j) };
        for i in 0..(2 * usize::from(unsafe { (*sci).total_bands })) {
            let bitalloc = unsafe { std::ptr::addr_of!((*sci).bitalloc).cast::<u8>() };
            let ba = c_int::from(unsafe { *bitalloc.add(i) });
            if ba != 0 {
                if ba < 17 {
                    let half = (1_i32 << (ba - 1)) - 1;
                    for k in 0..group_size as isize {
                        let sample = (unsafe { get_bits(bs, ba) } as i32) - half;
                        unsafe { *dst.offset(k) = sample as f32 };
                    }
                } else {
                    let modulus = (2_u32 << (ba - 17)) + 1;
                    let bits = modulus + 2 - (modulus >> 3);
                    let mut code = unsafe { get_bits(bs, bits as c_int) };
                    for k in 0..group_size as isize {
                        let sample = (code % modulus).wrapping_sub(modulus / 2) as i32;
                        unsafe { *dst.offset(k) = sample as f32 };
                        code /= modulus;
                    }
                }
            }
            dst = unsafe { dst.offset(choff) };
            choff = 18 - choff;
        }
    }

    group_size * 4
}
