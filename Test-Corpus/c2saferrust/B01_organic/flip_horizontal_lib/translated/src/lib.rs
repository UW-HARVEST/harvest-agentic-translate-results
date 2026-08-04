
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
pub fn flip_horizontal(img: &mut cp_image_t) {
    let w = img.w as isize;
    let h = img.h as isize;
    let flips = h / 2;

    for i in 0..flips {
        let mut a = img.pix.wrapping_offset(w * i);
        let mut b = img.pix.wrapping_offset(w * (h - i - 1));

        for _ in 0..w {
            unsafe {
                core::ptr::swap(a, b);
                a = a.wrapping_offset(1);
                b = b.wrapping_offset(1);
            }
        }
    }
}

