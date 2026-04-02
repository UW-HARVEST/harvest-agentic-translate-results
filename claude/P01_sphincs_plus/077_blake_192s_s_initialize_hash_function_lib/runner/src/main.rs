#![cfg_attr(fuzzing, no_main)]

use cando2::*;

const SPX_N: usize = 24;

state_member! {
    struct Ctx {
        pub_seed: [u8; SPX_N],
        sk_seed: [u8; SPX_N],
    }
}

harness! {
    state: {
        spx_ctx: Ctx,
    },

    library: "blake",

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
