use core::ffi::c_int;

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
    // Preserve the C function's lack of validation and raw pointer access.
    let w = unsafe { (*img).w };
    let h = unsafe { (*img).h };
    let stride = w * core::mem::size_of::<cp_pixel_t>() as c_int;
    let data = unsafe { (*img).pix.cast::<u8>() };
    let total = stride * h;
    let step = core::mem::size_of::<cp_pixel_t>() as c_int;
    let mut i = 0;

    while i < total {
        let idx = i as isize;

        let a = unsafe { *data.offset(idx + 3) as f32 } / 255.0f32;
        let mut r = unsafe { *data.offset(idx) as f32 } / 255.0f32;
        let mut g = unsafe { *data.offset(idx + 1) as f32 } / 255.0f32;
        let mut b = unsafe { *data.offset(idx + 2) as f32 } / 255.0f32;

        r *= a;
        g *= a;
        b *= a;

        unsafe {
            *data.offset(idx) = (r * 255.0f32) as u8;
            *data.offset(idx + 1) = (g * 255.0f32) as u8;
            *data.offset(idx + 2) = (b * 255.0f32) as u8;
        }

        i += step;
    }
}
