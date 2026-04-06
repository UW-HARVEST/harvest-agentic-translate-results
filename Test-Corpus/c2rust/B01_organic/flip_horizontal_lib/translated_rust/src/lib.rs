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
pub unsafe extern "C" fn flip_horizontal(mut img: *mut cp_image_t) {
    let mut pix: *mut cp_pixel_t = (*img).pix;
    let mut w: ::core::ffi::c_int = (*img).w;
    let mut h: ::core::ffi::c_int = (*img).h;
    let mut flips: ::core::ffi::c_int = h / 2 as ::core::ffi::c_int;
    let mut i: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    while i < flips {
        let mut a: *mut cp_pixel_t = pix.offset((w * i) as isize);
        let mut b: *mut cp_pixel_t = pix.offset((w * (h - i - 1 as ::core::ffi::c_int)) as isize);
        let mut j: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        while j < w {
            let mut t: cp_pixel_t = *a;
            *a = *b;
            *b = t;
            a = a.offset(1);
            b = b.offset(1);
            j += 1;
        }
        i += 1;
    }
}
