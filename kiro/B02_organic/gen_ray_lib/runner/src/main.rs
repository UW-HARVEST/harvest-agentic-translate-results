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
        cast3: c2Raycast,
        mp_x: f32,
        mp_y: f32,
        r_p_x: f32,
        r_p_y: f32,
        c_p_x: f32,
        c_p_y: f32,
        c_r: f32,
        cap_a_x: f32,
        cap_a_y: f32,
        cap_b_x: f32,
        cap_b_y: f32,
        cap_r: f32,
        bb_min_x: f32,
        bb_min_y: f32,
        bb_max_x: f32,
        bb_max_y: f32,
        returns: i32
    },

    signature: unsafe extern "C" fn(*mut c2Raycast, *mut c2Raycast, *mut c2Raycast, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32, f32) -> i32,

    fn run(&mut self) {
        unsafe {
            self.returns = (*SYMBOL)(
                &raw mut self.cast1,
                &raw mut self.cast2,
                &raw mut self.cast3,
                self.mp_x,
                self.mp_y,
                self.r_p_x,
                self.r_p_y,
                self.c_p_x,
                self.c_p_y,
                self.c_r,
                self.cap_a_x,
                self.cap_a_y,
                self.cap_b_x,
                self.cap_b_y,
                self.cap_r,
                self.bb_min_x,
                self.bb_min_y,
                self.bb_max_x,
                self.bb_max_y
            )
        };
    }
}
