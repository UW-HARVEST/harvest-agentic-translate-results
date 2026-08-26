use std::ffi::c_int;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn flip_horizontal(img: *mut cp_image_t) {
    let img = &mut *img;
    let pix = img.pix;
    let w = img.w;
    let h = img.h;
    let flips = h / 2;
    for i in 0..flips {
        let mut a = pix.offset((w * i) as isize);
        let mut b = pix.offset((w * (h - i - 1)) as isize);
        for _ in 0..w {
            let t = *a;
            *a = *b;
            *b = t;
            a = a.offset(1);
            b = b.offset(1);
        }
    }
}
