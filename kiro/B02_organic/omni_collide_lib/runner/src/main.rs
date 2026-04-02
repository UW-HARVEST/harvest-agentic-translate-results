#![cfg_attr(fuzzing, no_main)]

use cando2::*;

state_member! {
    #[derive(Copy)]
    enum C2Type {
        Capsule = 0,
        Circle = 1,
        Aabb = 2,
    }
}

harness! {
    state: {
        type_a: C2Type,
        a1: f32,
        a2: f32,
        a3: f32,
        a4: f32,
        a5: f32,
        type_b: C2Type,
        b1: f32,
        b2: f32,
        b3: f32,
        b4: f32,
        b5: f32,
        returns: i32,
    },

    signature: unsafe extern "C" fn(C2Type, f32, f32, f32, f32, f32, C2Type, f32, f32, f32, f32, f32) -> i32,

    fn run(&mut self) {
        self.returns = unsafe {
            (*SYMBOL)(
                self.type_a,
                self.a1,
                self.a2,
                self.a3,
                self.a4,
                self.a5,
                self.type_b,
                self.b1,
                self.b2,
                self.b3,
                self.b4,
                self.b5,
            )
        };
    }
}
