use std::os::raw::{c_int, c_void};
use std::slice;

#[repr(C)]
pub struct CpPixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
pub struct CpImage {
    pub w: c_int,
    pub h: c_int,
    pub pix: *mut CpPixel,
}

#[unsafe(no_mangle)]
pub extern "C" fn premultiply(img: *mut CpImage) {
    unsafe {
        let img_ref = &mut *img;
        let w = img_ref.w as usize;
        let h = img_ref.h as usize;
        let len = w * h;
        let pixels = slice::from_raw_parts_mut(img_ref.pix, len);
        
        for pixel in pixels.iter_mut() {
            let a = pixel.a as f32 / 255.0;
            pixel.r = ((pixel.r as f32 / 255.0 * a) * 255.0) as u8;
            pixel.g = ((pixel.g as f32 / 255.0 * a) * 255.0) as u8;
            pixel.b = ((pixel.b as f32 / 255.0 * a) * 255.0) as u8;
        }
    }
}