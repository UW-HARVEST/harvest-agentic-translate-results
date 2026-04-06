use std::os::raw::c_int;

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
    unsafe {
        let w = (*img).w;
        let h = (*img).h;
        let stride = w * std::mem::size_of::<cp_pixel_t>() as c_int;
        let data = (*img).pix as *mut u8;
        let mut i: c_int = 0;
        while i < stride * h {
            let a = *data.offset((i + 3) as isize) as f32 / 255.0;
            let mut r = *data.offset((i + 0) as isize) as f32 / 255.0;
            let mut g = *data.offset((i + 1) as isize) as f32 / 255.0;
            let mut b = *data.offset((i + 2) as isize) as f32 / 255.0;
            r *= a;
            g *= a;
            b *= a;
            *data.offset((i + 0) as isize) = (r * 255.0) as u8;
            *data.offset((i + 1) as isize) = (g * 255.0) as u8;
            *data.offset((i + 2) as isize) = (b * 255.0) as u8;
            i += std::mem::size_of::<cp_pixel_t>() as c_int;
        }
    }
}
