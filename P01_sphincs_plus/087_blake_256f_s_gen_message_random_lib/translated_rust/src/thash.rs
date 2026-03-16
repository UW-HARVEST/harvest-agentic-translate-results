use crate::params::*;
use crate::context::SpxCtx;
use crate::blake::{blake256, blake512};

// Simple thash variant: no bitmask/MGF1
// Uses blake256 for 1 block, blake512 for >1 blocks (SPX_BLAKE512=1)

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; total];
    let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..total].copy_from_slice(&inp[..inblocks * SPX_N]);

    blake512(&mut outbuf, &buf, total as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; total];
    let addr_bytes: &[u8; 32] = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..total].copy_from_slice(&inp[..inblocks * SPX_N]);

    blake256(&mut outbuf, &buf, total as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
