#![cfg_attr(fuzzing, no_main)]

use cando2::*;

state_member! {
    struct cp_pixel_t {
        r: u8,
        g: u8,
        b: u8,
        a: u8
    }
}

#[repr(C)]
struct cp_image_t {
    w: c_int,
    h: c_int,
    pix: *const cp_pixel_t,
}

harness! {
    state: {
        png_data: Vec<u8>,
        w: c_int,
        h: c_int,
        pix: Vec<cp_pixel_t>,
    },

    signature: unsafe extern "C" fn(*const u8, c_int) -> cp_image_t,

    fn run(&mut self) {
        let returns = unsafe {
            (*SYMBOL)(
                self.png_data.as_ptr(),
                self.png_data.len() as c_int,
            )
        };
        self.w = returns.w;
        self.h = returns.h;
        let pix_slice = unsafe { std::slice::from_raw_parts(returns.pix, (returns.w * returns.h) as usize) };
        self.pix = pix_slice.to_vec();
    }
}
