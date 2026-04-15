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
    let w = img_ref.w;
    let h = img_ref.h;
    
    if w <= 0 || h <= 0 || img_ref.pix.is_null() {
        return;
    }
    
    let len = (w * h) as usize;
    let pixels = unsafe { std::slice::from_raw_parts_mut(img_ref.pix, len) };
    
    for pixel in pixels.iter_mut() {
        let a = pixel.a as f32 / 255.0;
        let r = pixel.r as f32 / 255.0;
        let g = pixel.g as f32 / 255.0;
        let b = pixel.b as f32 / 255.0;
        
        pixel.r = (r * a * 255.0) as u8;
        pixel.g = (g * a * 255.0) as u8;
        pixel.b = (b * a * 255.0) as u8;
    }
}
