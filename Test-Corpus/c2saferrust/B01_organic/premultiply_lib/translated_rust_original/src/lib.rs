
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
pub fn premultiply(img: &mut cp_image_t) {
    let pixel_count = (img.w as usize).saturating_mul(img.h as usize);
    let data = unsafe { ::core::slice::from_raw_parts_mut(img.pix, pixel_count) };

    for pixel in data.iter_mut() {
        let a = pixel.a as f32 / 255.0;
        pixel.r = (pixel.r as f32 * a) as uint8_t;
        pixel.g = (pixel.g as f32 * a) as uint8_t;
        pixel.b = (pixel.b as f32 * a) as uint8_t;
    }
}

