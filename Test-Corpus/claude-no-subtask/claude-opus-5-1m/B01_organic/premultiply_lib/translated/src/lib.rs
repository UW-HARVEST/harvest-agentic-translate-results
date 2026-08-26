use std::ffi::c_int;

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct cp_pixel_t {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
#[allow(non_camel_case_types)]
pub struct cp_image_t {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut cp_pixel_t,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn premultiply(img: *mut cp_image_t) {
    let img_ref = &mut *img;
    let w = img_ref.w;
    let h = img_ref.h;
    let pixel_size = std::mem::size_of::<cp_pixel_t>() as c_int;
    let stride = w * pixel_size;
    let data = img_ref.pix as *mut u8;

    let total = stride * h;
    let mut i: c_int = 0;
    while i < total {
        let idx = i as isize;
        let a = *data.offset(idx + 3) as f32 / 255.0f32;
        let mut r = *data.offset(idx) as f32 / 255.0f32;
        let mut g = *data.offset(idx + 1) as f32 / 255.0f32;
        let mut b = *data.offset(idx + 2) as f32 / 255.0f32;
        r *= a;
        g *= a;
        b *= a;
        *data.offset(idx) = (r * 255.0f32) as u8;
        *data.offset(idx + 1) = (g * 255.0f32) as u8;
        *data.offset(idx + 2) = (b * 255.0f32) as u8;
        i += pixel_size;
    }
}
