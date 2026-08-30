//! Translation of `lib/blake/src/thash_blake_simple.c`.

use crate::address::addr_bytes;
use crate::blake::SPX_BLAKE512;
use crate::blake::blake256::{SPX_BLAKE256_OUTPUT_BYTES, blake256};
use crate::blake::blake512::{SPX_BLAKE512_OUTPUT_BYTES, blake512};
use crate::context::SpxCtx;
use crate::params::*;

/// Takes an array of `inblocks` concatenated arrays of `SPX_N` bytes.
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    if SPX_BLAKE512 && inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let mut buf = [0u8; SPX_N + SPX_ADDR_BYTES + SPX_MAX_INBLOCKS * SPX_N];
    let n = inblocks * SPX_N;

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + n].copy_from_slice(&inp[..n]);

    let total = SPX_N + SPX_ADDR_BYTES + n;
    blake256(&mut outbuf, &buf[..total], total as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let mut buf = [0u8; SPX_N + SPX_ADDR_BYTES + SPX_MAX_INBLOCKS * SPX_N];
    let n = inblocks * SPX_N;

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + n].copy_from_slice(&inp[..n]);

    let total = SPX_N + SPX_ADDR_BYTES + n;
    blake512(&mut outbuf, &buf[..total], total as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
