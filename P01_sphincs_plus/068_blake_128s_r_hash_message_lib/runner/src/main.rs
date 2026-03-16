#![cfg_attr(fuzzing, no_main)]

use cando2::*;

const SPX_N: usize = 16;

state_member! {
    struct Ctx {
        pub_seed: [u8; SPX_N],
        sk_seed: [u8; SPX_N],
    }
}

harness! {
    state: {
        digest: [[u8; SPX_N]; 4], // len = (SPX_FORS_HEIGHT * SPX_FORS_TREES + 7) / 8
        tree: u64,
        leaf_idx: u32,
        R: [u8; SPX_N],
        pk: [[u8; SPX_N]; 3],
        m: [u8; SPX_N],
        mlen: c_ulonglong,
        spx_ctx: Ctx,
    },

    library: "blake",

    symbol: "SPX_hash_message",

    signature: unsafe extern "C" fn(*mut u8, *mut u64, *mut u32, *const u8, *const u8, *const u8, c_ulonglong, *const Ctx),

    fn run(&mut self) {
        self.mlen %= SPX_N as u64;
        unsafe {
            (*SYMBOL)(
                &raw mut self.digest as *mut u8,
                &raw mut self.tree,
                &raw mut self.leaf_idx,
                &raw const self.R as *const u8,
                &raw const self.pk as *const u8,
                &raw const self.m as *const u8,
                self.mlen,
                &raw const self.spx_ctx,
            )
        };
    }
}
