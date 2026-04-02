#![cfg_attr(fuzzing, no_main)]

use cando2::*;

state_member! {
    struct c2v {
        x: f32,
        y: f32
    }
}

state_member! {
    struct c2Raycast {
        t: f32,
        n: c2v
    }
}

harness! {
    state: {
        cast1: c2Raycast,
        cast2: c2Raycast,
        returns: i32
    },

    signature: unsafe extern "C" fn(*mut c2Raycast, *mut c2Raycast) -> i32,

    fn run(&mut self) {
        unsafe {
            self.returns = (*SYMBOL)(
                &raw mut self.cast1,
                &raw mut self.cast2,
            )
        };
    }
}
