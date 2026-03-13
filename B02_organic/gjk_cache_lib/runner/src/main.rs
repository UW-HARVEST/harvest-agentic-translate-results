#![cfg_attr(fuzzing, no_main)]

use cando2::*;

state_member! {
    struct c2v {
        x: f32,
        y: f32
    }
}

harness! {
    state: {
        reverse: bool,
        a: c2v,
        b: c2v,
        a1: f32,
        a2: f32,
        a3: f32,
        a4: f32,
        b1: f32,
        b2: f32,
        b3: f32,
        b4: f32,
        b5: f32,
    },

    signature: unsafe extern "C" fn(bool, *mut c2v, *mut c2v, f32, f32, f32, f32, f32, f32, f32, f32, f32),

    fn run(&mut self) {
        unsafe {
            (*SYMBOL)(
                self.reverse,
                &raw mut self.a,
                &raw mut self.b,
                self.a1,
                self.a2,
                self.a3,
                self.a4,
                self.b1,
                self.b2,
                self.b3,
                self.b4,
                self.b5
            )
        };
    }
}
