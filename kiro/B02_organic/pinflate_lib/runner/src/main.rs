#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        in_bytes: c_int,
        out_bytes: c_int,
        b_in: Vec<u8>,
        b_out: Vec<u8>,
        returns: c_int
    },

    signature: unsafe extern "C" fn(*mut u8, c_int, *mut u8, c_int) -> c_int,

    fn run(&mut self) {
        self.b_out.clear();
        self.b_out.reserve(self.out_bytes as usize);
        unsafe {
            self.returns = (*SYMBOL)(
                self.b_in.as_mut_ptr(),
                self.in_bytes,
                self.b_out.as_mut_ptr(),
                self.out_bytes
            );
        };
    }
}
