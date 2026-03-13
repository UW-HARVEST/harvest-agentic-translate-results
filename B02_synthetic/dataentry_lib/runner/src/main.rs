#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        a: i32,
        b: i32,
        c: i32,
        d: i32,
        returns: i32
    },

    signature: unsafe extern "C" fn(i32, i32, i32, i32) -> i32,

    fn run(&mut self) {
        self.returns = unsafe {
            (*SYMBOL)(
                self.a,
                self.b,
                self.c,
                self.d
            )
        }
    }
}
