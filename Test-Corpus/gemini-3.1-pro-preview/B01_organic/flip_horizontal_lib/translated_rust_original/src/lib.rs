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
    unsafe {
        let w = (*img).w;
        let h = (*img).h;
        if w <= 0 || h <= 0 {
            return;
        }
        let w = w as usize;
        let h = h as usize;
        let pix = (*img).pix;
        let flips = h / 2;
        let slice = std::slice::from_raw_parts_mut(pix, w * h);
        for i in 0..flips {
            let a_idx = w * i;
            let b_idx = w * (h - i - 1);
            let (left, right) = slice.split_at_mut(b_idx);
            left[a_idx..a_idx + w].swap_with_slice(&mut right[0..w]);
        }
    }
}
