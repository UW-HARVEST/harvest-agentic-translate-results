pub type __uint8_t = u8;
pub type uint8_t = __uint8_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cp_pixel_t {
    pub r: uint8_t,
    pub g: uint8_t,
    pub b: uint8_t,
    pub a: uint8_t,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct cp_image_t {
    pub w: libc::c_int,
    pub h: libc::c_int,
    pub pix: *mut cp_pixel_t,
}
#[no_mangle]
pub unsafe extern "C" fn premultiply(mut img: *mut cp_image_t) {
    let mut w: libc::c_int = (*img).w;
    let mut h: libc::c_int = (*img).h;
    let mut stride: libc::c_int = (w as usize)
        .wrapping_mul(std::mem::size_of::<cp_pixel_t>() as usize)
        as libc::c_int;
    let mut data: *mut uint8_t = (*img).pix as *mut uint8_t;
    let mut i: libc::c_int = 0 as libc::c_int;
    while i < stride * h {
        let mut a: libc::c_float =
            *data.offset((i + 3 as libc::c_int) as isize) as libc::c_float / 255.0f32;
        let mut r: libc::c_float =
            *data.offset((i + 0 as libc::c_int) as isize) as libc::c_float / 255.0f32;
        let mut g: libc::c_float =
            *data.offset((i + 1 as libc::c_int) as isize) as libc::c_float / 255.0f32;
        let mut b: libc::c_float =
            *data.offset((i + 2 as libc::c_int) as isize) as libc::c_float / 255.0f32;
        r *= a;
        g *= a;
        b *= a;
        *data.offset((i + 0 as libc::c_int) as isize) = (r * 255.0f32) as uint8_t;
        *data.offset((i + 1 as libc::c_int) as isize) = (g * 255.0f32) as uint8_t;
        *data.offset((i + 2 as libc::c_int) as isize) = (b * 255.0f32) as uint8_t;
        i = (i as libc::c_ulong)
            .wrapping_add(std::mem::size_of::<cp_pixel_t>() as usize as libc::c_ulong)
            as libc::c_int as libc::c_int;
    }
}
