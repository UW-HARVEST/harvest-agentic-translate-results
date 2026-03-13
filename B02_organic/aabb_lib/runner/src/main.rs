#![cfg_attr(fuzzing, no_main)]

use cando2::*;

harness! {
    state: {
        min_x: f32,
        min_y: f32,
        max_x: f32,
        max_y: f32,
        returns: i32,
    },

    signature: unsafe extern "C" fn(f32, f32, f32, f32) -> i32,

    fn run(&mut self) {
        self.returns = unsafe {
            (*SYMBOL)(
                self.min_x,
                self.min_y,
                self.max_x,
                self.max_y,
            )
        };
    }
}
