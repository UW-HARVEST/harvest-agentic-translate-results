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
pub extern "C" fn premultiply(img: *mut cp_image_t) {
    if img.is_null() {
        return;
    }

    let img_ref = unsafe { &mut *img };
    if img_ref.pix.is_null() {
        return;
    }

    let w = img_ref.w;
    let h = img_ref.h;
    if w <= 0 || h <= 0 {
        return;
    }

    let len = match (w as usize).checked_mul(h as usize) {
        Some(v) => v,
        None => return,
    };

    let pixels = unsafe { std::slice::from_raw_parts_mut(img_ref.pix, len) };

    for px in pixels.iter_mut() {
        let a = px.a as f32 / 255.0;
        let r = px.r as f32 / 255.0;
        let g = px.g as f32 / 255.0;
        let b = px.b as f32 / 255.0;

        px.r = (r * a * 255.0) as u8;
        px.g = (g * a * 255.0) as u8;
        px.b = (b * a * 255.0) as u8;
    }
}
