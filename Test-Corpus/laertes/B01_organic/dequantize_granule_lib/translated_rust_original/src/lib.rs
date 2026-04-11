pub type __uint8_t = u8;
pub type __uint32_t = u32;
pub type uint8_t = __uint8_t;
pub type uint32_t = __uint32_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct bs_t {
    pub buf: *const uint8_t,
    pub pos: libc::c_int,
    pub limit: libc::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct L12_scale_info {
    pub scf: [libc::c_float; 192],
    pub total_bands: uint8_t,
    pub stereo_bands: uint8_t,
    pub bitalloc: [uint8_t; 64],
    pub scfcod: [uint8_t; 64],
}
unsafe extern "C" fn get_bits(mut bs: *mut bs_t, mut n: libc::c_int) -> uint32_t {
    let mut next: uint32_t = 0;
    let mut cache: uint32_t = 0 as uint32_t;
    let mut s: uint32_t = ((*bs).pos & 7 as libc::c_int) as uint32_t;
    let mut shl: libc::c_int = (n as uint32_t).wrapping_add(s) as libc::c_int;
    let mut p: *const uint8_t = (*bs)
        .buf
        .offset(((*bs).pos >> 3 as libc::c_int) as isize);
    (*bs).pos += n;
    if (*bs).pos > (*bs).limit {
        return 0 as uint32_t;
    }
    let fresh0 = p;
    p = p.offset(1);
    next = (*fresh0 as libc::c_int & 255 as libc::c_int >> s) as uint32_t;
    loop {
        shl -= 8 as libc::c_int;
        if !(shl > 0 as libc::c_int) {
            break;
        }
        cache = (cache as libc::c_uint | (next << shl) as libc::c_uint) as uint32_t;
        let fresh1 = p;
        p = p.offset(1);
        next = *fresh1 as uint32_t;
    }
    return cache | next >> -shl;
}
#[no_mangle]
pub unsafe extern "C" fn dequantize_granule(
    mut grbuf: *mut libc::c_float,
    mut bs: *mut bs_t,
    mut sci: *mut L12_scale_info,
    mut group_size: libc::c_int,
) -> libc::c_int {
    let mut i: libc::c_int = 0;
    let mut j: libc::c_int = 0;
    let mut k: libc::c_int = 0;
    let mut choff: libc::c_int = 576 as libc::c_int;
    j = 0 as libc::c_int;
    while j < 4 as libc::c_int {
        let mut dst: *mut libc::c_float = grbuf.offset((group_size * j) as isize);
        i = 0 as libc::c_int;
        while i < 2 as libc::c_int * (*sci).total_bands as libc::c_int {
            let mut ba: libc::c_int = (*sci).bitalloc[i as usize] as libc::c_int;
            if ba != 0 as libc::c_int {
                if ba < 17 as libc::c_int {
                    let mut half: libc::c_int = ((1 as libc::c_int)
                        << ba - 1 as libc::c_int)
                        - 1 as libc::c_int;
                    k = 0 as libc::c_int;
                    while k < group_size {
                        *dst.offset(k as isize) =
                            (get_bits(bs, ba) as libc::c_int - half) as libc::c_float;
                        k += 1;
                    }
                } else {
                    let mut mod_0: libc::c_uint = (((2 as libc::c_int)
                        << ba - 17 as libc::c_int)
                        + 1 as libc::c_int)
                        as libc::c_uint;
                    let mut code: libc::c_uint = get_bits(
                        bs,
                        mod_0
                            .wrapping_add(2 as libc::c_uint)
                            .wrapping_sub(mod_0 >> 3 as libc::c_int)
                            as libc::c_int,
                    )
                        as libc::c_uint;
                    k = 0 as libc::c_int;
                    while k < group_size {
                        *dst.offset(k as isize) = code
                            .wrapping_rem(mod_0)
                            .wrapping_sub(mod_0.wrapping_div(2 as libc::c_uint))
                            as libc::c_int
                            as libc::c_float;
                        k += 1;
                        code = code.wrapping_div(mod_0);
                    }
                }
            }
            dst = dst.offset(choff as isize);
            choff = 18 as libc::c_int - choff;
            i += 1;
        }
        j += 1;
    }
    return group_size * 4 as libc::c_int;
}
