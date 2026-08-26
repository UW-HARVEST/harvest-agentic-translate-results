#![allow(non_camel_case_types)]

use std::os::raw::c_int;

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

/// Multiplies the RGB channels of each pixel by its alpha value (premultiplied alpha).
///
/// # Safety
///
/// `img` must be a valid pointer to a `cp_image_t`, and `img.pix` must point to
/// at least `img.w * img.h` valid `cp_pixel_t` elements.
#[no_mangle]
pub unsafe extern "C" fn premultiply(img: *mut cp_image_t) {
    if img.is_null() {
        return;
    }
    let img_ref = &mut *img;
    let w = img_ref.w as isize;
    let h = img_ref.h as isize;
    let pixel_size = std::mem::size_of::<cp_pixel_t>() as isize;
    let stride = w * pixel_size;
    let data = img_ref.pix as *mut u8;

    if data.is_null() {
        return;
    }

    let total = stride * h;
    let mut i: isize = 0;
    while i < total {
        let a = *data.offset(i + 3) as f32 / 255.0f32;
        let mut r = *data.offset(i) as f32 / 255.0f32;
        let mut g = *data.offset(i + 1) as f32 / 255.0f32;
        let mut b = *data.offset(i + 2) as f32 / 255.0f32;
        r *= a;
        g *= a;
        b *= a;
        *data.offset(i) = (r * 255.0f32) as u8;
        *data.offset(i + 1) = (g * 255.0f32) as u8;
        *data.offset(i + 2) = (b * 255.0f32) as u8;
        i += pixel_size;
    }
}
