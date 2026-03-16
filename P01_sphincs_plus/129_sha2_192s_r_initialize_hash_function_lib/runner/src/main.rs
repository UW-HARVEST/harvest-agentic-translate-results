#![cfg_attr(fuzzing, no_main)]

use cando2::*;

const SPX_N: usize = 24;

state_member! {
    struct Ctx {
        pub_seed: [u8; SPX_N],
        sk_seed: [u8; SPX_N],
        state_seeded: [[u8; 20]; 2],
        state_seeded_512: [[u8; 24]; 3],
    }
}

harness! {
    state: {
        spx_ctx: Ctx,
    },

    library: "sha2",

    symbol: "SPX_initialize_hash_function",

    signature: unsafe extern "C" fn(*mut Ctx),

    fn run(&mut self) {
        unsafe {
            (*SYMBOL)(
                &raw mut self.spx_ctx,
            )
        };
    }
}
