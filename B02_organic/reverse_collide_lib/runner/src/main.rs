#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        x: f32,
        y: f32,
        r: f32,
        returns: i32,
    },

    signature: unsafe extern "C" fn(f32, f32, f32) -> i32,

    fn run(&mut self) {
        self.returns = unsafe {
            (*SYMBOL)(
                self.x,
                self.y,
                self.r
            )
        };
    }
}
