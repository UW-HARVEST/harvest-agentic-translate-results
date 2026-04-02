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

state_member! {
    struct c2v {
        x: f32,
        y: f32
    }
}

state_member! {
    struct c2Manifold {
        count: i32,
        depths: [f32; 2],
        contact_points: [c2v; 2],
        n: c2v
    }
}

harness! {
    state: {
        m: c2Manifold,
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
    },

    signature: unsafe extern "C" fn(*mut c2Manifold, C2Type, f32, f32, f32, f32, f32, C2Type, f32, f32, f32, f32, f32),

    fn run(&mut self) {
        unsafe {
            (*SYMBOL)(
                &raw mut self.m,
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
