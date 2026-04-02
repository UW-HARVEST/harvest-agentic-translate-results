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
        out: [u8; SPX_N],
        spx_ctx: Ctx,
        addr: [u32; 8],
    },

    library: "sha2",

    symbol: "SPX_prf_addr",

    signature: unsafe extern "C" fn(*mut u8, *const Ctx, *const [u32; 8]),

    fn run(&mut self) {
        unsafe {
            (*SYMBOL)(
                &raw mut self.out as *mut u8,
                &raw const self.spx_ctx,
                &raw const self.addr
            )
        };
    }
}
