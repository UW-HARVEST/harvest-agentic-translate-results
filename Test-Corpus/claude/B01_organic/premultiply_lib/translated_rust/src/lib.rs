use std::ffi::c_int;

#[repr(C)]
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
pub unsafe extern "C" fn premultiply(img: *mut cp_image_t) {
    let img = &mut *img;
    let w: c_int = img.w;
    let h: c_int = img.h;
    let pixel_size: c_int = core::mem::size_of::<cp_pixel_t>() as c_int;
    let stride: c_int = w * pixel_size;
    let data: *mut u8 = img.pix as *mut u8;

    let total: c_int = stride * h;
    let mut i: c_int = 0;
    while i < total {
        let idx = i as isize;
        let a = (*data.offset(idx + 3)) as f32 / 255.0f32;
        let mut r = (*data.offset(idx + 0)) as f32 / 255.0f32;
        let mut g = (*data.offset(idx + 1)) as f32 / 255.0f32;
        let mut b = (*data.offset(idx + 2)) as f32 / 255.0f32;
        r *= a;
        g *= a;
        b *= a;
        *data.offset(idx + 0) = (r * 255.0f32) as u8;
        *data.offset(idx + 1) = (g * 255.0f32) as u8;
        *data.offset(idx + 2) = (b * 255.0f32) as u8;
        i += pixel_size;
    }
}
