use std::ffi::c_int;

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn premultiply(img: *mut cp_image_t) {
    let w = unsafe { (*img).w };
    let h = unsafe { (*img).h };
    let stride = w.wrapping_mul(size_of::<cp_pixel_t>() as c_int);
    let data = unsafe { (*img).pix as *mut u8 };
    let total = stride.wrapping_mul(h);

    let mut i: c_int = 0;
    while i < total {
        let offset = i as isize;
        let a = unsafe { *data.offset(offset + 3) } as f32 / 255.0f32;
        let mut r = unsafe { *data.offset(offset) } as f32 / 255.0f32;
        let mut g = unsafe { *data.offset(offset + 1) } as f32 / 255.0f32;
        let mut b = unsafe { *data.offset(offset + 2) } as f32 / 255.0f32;

        r *= a;
        g *= a;
        b *= a;

        unsafe {
            *data.offset(offset) = (r * 255.0f32) as u8;
            *data.offset(offset + 1) = (g * 255.0f32) as u8;
            *data.offset(offset + 2) = (b * 255.0f32) as u8;
        }

        i = i.wrapping_add(size_of::<cp_pixel_t>() as c_int);
    }
}

