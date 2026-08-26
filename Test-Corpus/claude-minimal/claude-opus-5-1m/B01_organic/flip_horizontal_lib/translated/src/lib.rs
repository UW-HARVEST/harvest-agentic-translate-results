use std::os::raw::c_int;

#[repr(C)]
#[derive(Clone, Copy)]
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

/// # Safety
///
/// `img` must be a valid pointer to a `cp_image_t`. The `pix` field of `*img`
/// must point to a buffer of at least `w * h` `cp_pixel_t` elements.
#[no_mangle]
pub unsafe extern "C" fn flip_horizontal(img: *mut cp_image_t) {
    if img.is_null() {
        return;
    }
    let img_ref = &mut *img;
    let pix = img_ref.pix;
    let w = img_ref.w;
    let h = img_ref.h;
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
