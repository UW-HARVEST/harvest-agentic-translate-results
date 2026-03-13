#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        w: c_int,
        h: c_int,
        bpp: c_int,
        raw: Vec<u8>,
    },

    signature: unsafe extern "C" fn(c_int, c_int, c_int, *mut u8),

    fn run(&mut self) {
        unsafe {
            (*SYMBOL)(
                self.w,
                self.h,
                self.bpp,
                self.raw.as_mut_ptr(),
            );
        };
    }
}
