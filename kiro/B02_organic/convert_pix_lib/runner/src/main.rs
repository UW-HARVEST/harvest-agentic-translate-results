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

harness! {
    state: {
        bpp: c_int,
        w: c_int,
        h: c_int,
        src: Vec<u8>,
        dst: Vec<cp_pixel_t>,
    },

    signature: unsafe extern "C" fn(c_int, c_int, c_int, *const u8, *mut cp_pixel_t),

    fn run(&mut self) {
        self.dst.clear();
        self.dst.reserve((self.w * self.h) as usize);
        unsafe {
            (*SYMBOL)(
                self.bpp,
                self.w,
                self.h,
                self.src.as_ptr(),
                self.dst.as_mut_ptr()
            );
            self.dst.set_len((self.w * self.h) as usize);
        };
    }
}
