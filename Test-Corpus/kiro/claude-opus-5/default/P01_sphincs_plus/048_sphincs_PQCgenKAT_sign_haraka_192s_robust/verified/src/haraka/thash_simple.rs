//! Translation of `lib/haraka/src/thash_haraka_simple.c`.

use crate::address::addr_bytes;
use crate::context::SpxCtx;
use crate::haraka::haraka::{haraka512, haraka_s};
use crate::params::*;

/// Takes an array of `inblocks` concatenated arrays of `SPX_N` bytes.
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut buf = [0u8; SPX_ADDR_BYTES + SPX_MAX_INBLOCKS * SPX_N];
    let mut outbuf = [0u8; 32];
    let mut buf_tmp = [0u8; 64];

    if inblocks == 1 {
        /* F function */
        /* Since SPX_N may be smaller than 32, we need a temporary buffer. */
        buf_tmp.fill(0);
        buf_tmp[..32].copy_from_slice(addr_bytes(addr));
        buf_tmp[SPX_ADDR_BYTES..SPX_ADDR_BYTES + SPX_N].copy_from_slice(&inp[..SPX_N]);

        haraka512(&mut outbuf, &buf_tmp, ctx);
        out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
    } else {
        /* All other tweakable hashes */
        let n = inblocks * SPX_N;
        buf[..32].copy_from_slice(addr_bytes(addr));
        buf[SPX_ADDR_BYTES..SPX_ADDR_BYTES + n].copy_from_slice(&inp[..n]);

        let total = SPX_ADDR_BYTES + n;
        haraka_s(
            &mut out[..SPX_N],
            SPX_N as u64,
            &buf[..total],
            total as u64,
            ctx,
        );
    }
}
