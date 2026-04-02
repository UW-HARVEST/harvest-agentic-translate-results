#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        pfcn: i32,
        returns: i32,
    },

    signature: unsafe extern "C" fn(i32) -> i32,

    fn run(&mut self) {
        self.returns = unsafe {
            (*SYMBOL)(
                self.pfcn,
            )
        };
    }
}
