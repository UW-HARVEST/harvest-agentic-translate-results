use std::{ffi::c_int, mem::size_of};

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
pub unsafe extern "C" fn premultiply(img: *mut CpImage) {
    let w = unsafe { (*img).w };
    let h = unsafe { (*img).h };
    let stride = w.wrapping_mul(size_of::<CpPixel>() as c_int);
    let data = unsafe { (*img).pix.cast::<u8>() };
    let end = stride.wrapping_mul(h);
    let mut i = 0;

    while i < end {
        let offset = i as usize;
        let a = f32::from(unsafe { *data.add(offset + 3) }) / 255.0;
        let mut r = f32::from(unsafe { *data.add(offset) }) / 255.0;
        let mut g = f32::from(unsafe { *data.add(offset + 1) }) / 255.0;
        let mut b = f32::from(unsafe { *data.add(offset + 2) }) / 255.0;
        r *= a;
        g *= a;
        b *= a;
        unsafe {
            *data.add(offset) = (r * 255.0) as u8;
            *data.add(offset + 1) = (g * 255.0) as u8;
            *data.add(offset + 2) = (b * 255.0) as u8;
        }
        i = i.wrapping_add(size_of::<CpPixel>() as c_int);
    }
}
