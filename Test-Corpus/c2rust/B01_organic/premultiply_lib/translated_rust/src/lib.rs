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
    pub w: ::core::ffi::c_int,
    pub h: ::core::ffi::c_int,
    pub pix: *mut cp_pixel_t,
}
#[no_mangle]
pub unsafe extern "C" fn premultiply(mut img: *mut cp_image_t) {
    let mut w: ::core::ffi::c_int = (*img).w;
    let mut h: ::core::ffi::c_int = (*img).h;
    let mut stride: ::core::ffi::c_int = (w as usize)
        .wrapping_mul(::core::mem::size_of::<cp_pixel_t>() as usize)
        as ::core::ffi::c_int;
    let mut data: *mut uint8_t = (*img).pix as *mut uint8_t;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < stride * h {
        let mut a: ::core::ffi::c_float =
            *data.offset((i + 3 as ::core::ffi::c_int) as isize) as ::core::ffi::c_float / 255.0f32;
        let mut r: ::core::ffi::c_float =
            *data.offset((i + 0 as ::core::ffi::c_int) as isize) as ::core::ffi::c_float / 255.0f32;
        let mut g: ::core::ffi::c_float =
            *data.offset((i + 1 as ::core::ffi::c_int) as isize) as ::core::ffi::c_float / 255.0f32;
        let mut b: ::core::ffi::c_float =
            *data.offset((i + 2 as ::core::ffi::c_int) as isize) as ::core::ffi::c_float / 255.0f32;
        r *= a;
        g *= a;
        b *= a;
        *data.offset((i + 0 as ::core::ffi::c_int) as isize) = (r * 255.0f32) as uint8_t;
        *data.offset((i + 1 as ::core::ffi::c_int) as isize) = (g * 255.0f32) as uint8_t;
        *data.offset((i + 2 as ::core::ffi::c_int) as isize) = (b * 255.0f32) as uint8_t;
        i = (i as ::core::ffi::c_ulong)
            .wrapping_add(::core::mem::size_of::<cp_pixel_t>() as usize as ::core::ffi::c_ulong)
            as ::core::ffi::c_int as ::core::ffi::c_int;
    }
}
