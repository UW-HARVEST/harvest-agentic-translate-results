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
pub extern "C" fn premultiply(img: *mut cp_image_t) {
    // Safety: caller passes a valid pointer to a cp_image_t whose `pix`
    // points to at least w*h pixels.
    let img_ref = unsafe { &mut *img };
    let w = img_ref.w;
    let h = img_ref.h;
    let stride = w * core::mem::size_of::<cp_pixel_t>() as c_int;
    let data = img_ref.pix as *mut u8;

    let total = stride * h;
    let pixel_size = core::mem::size_of::<cp_pixel_t>() as c_int;

    let mut i: c_int = 0;
    while i < total {
        unsafe {
            let a = *data.offset((i + 3) as isize) as f32 / 255.0f32;
            let mut r = *data.offset((i + 0) as isize) as f32 / 255.0f32;
            let mut g = *data.offset((i + 1) as isize) as f32 / 255.0f32;
            let mut b = *data.offset((i + 2) as isize) as f32 / 255.0f32;
            r *= a;
            g *= a;
            b *= a;
            *data.offset((i + 0) as isize) = (r * 255.0f32) as u8;
            *data.offset((i + 1) as isize) = (g * 255.0f32) as u8;
            *data.offset((i + 2) as isize) = (b * 255.0f32) as u8;
        }
        i += pixel_size;
    }
}
