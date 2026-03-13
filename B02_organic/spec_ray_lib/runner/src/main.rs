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
        cast: c2Raycast,
        mp_x: f32,
        mp_y: f32,
        c_p_x: f32,
        c_p_y: f32,
        c_r: f32,
        r_p_x: f32,
        r_p_y: f32,
        returns: i32
    },

    signature: unsafe extern "C" fn(*mut c2Raycast, f32, f32, f32, f32, f32, f32, f32) -> i32,

    fn run(&mut self) {
        unsafe {
            self.returns = (*SYMBOL)(
                &raw mut self.cast,
                self.mp_x,
                self.mp_y,
                self.c_p_x,
                self.c_p_y,
                self.c_r,
                self.r_p_x,
                self.r_p_y
            )
        };
    }
}
