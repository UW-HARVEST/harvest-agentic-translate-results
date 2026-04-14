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

#[unsafe(no_mangle)]
pub extern "C" fn flip_horizontal(img: *mut cp_image_t) {
    if img.is_null() {
        return;
    }

    let img = unsafe { &mut *img };
    if img.pix.is_null() || img.w <= 0 || img.h <= 0 {
        return;
    }

    let w = img.w as usize;
    let h = img.h as usize;
    let total = match w.checked_mul(h) {
        Some(v) => v,
        None => return,
    };

    let pix = unsafe { std::slice::from_raw_parts_mut(img.pix, total) };
    let flips = h / 2;

    for i in 0..flips {
        let top_start = w * i;
        let bottom_start = w * (h - i - 1);
        for j in 0..w {
            pix.swap(top_start + j, bottom_start + j);
        }
    }
}
