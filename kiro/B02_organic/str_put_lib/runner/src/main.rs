#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        num: c_int
    },

    signature: unsafe extern "C" fn(c_int),

    fn run(&mut self) {
        unsafe {
            (*SYMBOL)(
                self.num
            )
        };
    }
}
