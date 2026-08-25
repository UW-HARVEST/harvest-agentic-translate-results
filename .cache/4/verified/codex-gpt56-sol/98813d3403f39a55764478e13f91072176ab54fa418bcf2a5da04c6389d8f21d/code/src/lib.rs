use std::ffi::c_int;

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

#[inline]
fn premultiply_channel(channel: u8, alpha: f32) -> u8 {
    let mut value = f32::from(channel) / 255.0_f32;
    value *= alpha;
    (value * 255.0_f32) as u8
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn premultiply(img: *mut cp_image_t) {
    // Match the C implementation's signed-int stride and loop bound.
    let image = unsafe { &mut *img };
    let stride = image.w.wrapping_mul(size_of::<cp_pixel_t>() as c_int);
    let byte_len = stride.wrapping_mul(image.h);
    let data = image.pix.cast::<u8>();

    let mut i: c_int = 0;
    while i < byte_len {
        let pixel = unsafe { data.add(i as usize) };
        let alpha = f32::from(unsafe { *pixel.add(3) }) / 255.0_f32;

        for channel in 0..3 {
            let component = unsafe { *pixel.add(channel) };
            unsafe {
                *pixel.add(channel) = premultiply_channel(component, alpha);
            }
        }

        i = i.wrapping_add(size_of::<cp_pixel_t>() as c_int);
    }
}
