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
    let w = unsafe { (*img).w };
    let h = unsafe { (*img).h };
    let stride = w as usize * std::mem::size_of::<cp_pixel_t>();
    let data = unsafe { (*img).pix as *mut u8 };
    let mut i: usize = 0;
    while i < stride * h as usize {
        let a = unsafe { *data.add(i + 3) } as f32 / 255.0;
        let r = unsafe { *data.add(i) } as f32 / 255.0;
        let g = unsafe { *data.add(i + 1) } as f32 / 255.0;
        let b = unsafe { *data.add(i + 2) } as f32 / 255.0;
        let r = r * a;
        let g = g * a;
        let b = b * a;
        unsafe {
            *data.add(i) = (r * 255.0) as u8;
            *data.add(i + 1) = (g * 255.0) as u8;
            *data.add(i + 2) = (b * 255.0) as u8;
        }
        i += std::mem::size_of::<cp_pixel_t>();
    }
}
