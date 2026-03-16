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
        R: [[u8; SPX_N]; 4], // len >= 32
        sk_prf: [[u8; SPX_N]; 3],
        optrand: [u8; SPX_N],
        m: [u8; SPX_N], // len = mlen
        mlen: c_ulonglong,
        spx_ctx: Ctx,
    },

    library: "blake",

    symbol: "SPX_gen_message_random",

    signature: unsafe extern "C" fn(*mut u8, *const u8, *const u8, *const u8, c_ulonglong, *const Ctx),

    fn run(&mut self) {
        self.mlen %= SPX_N as u64;
        unsafe {
            (*SYMBOL)(
                &raw mut self.R as *mut u8,
                &raw const self.sk_prf as *const u8,
                &raw const self.optrand as *const u8,
                &raw const self.m as *const u8,
                self.mlen,
                &raw const self.spx_ctx,
            )
        };
    }
}
