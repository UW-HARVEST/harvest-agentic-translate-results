use crate::params::*;
use crate::context::SpxCtx;
use crate::address::addr_bytes;
use crate::blake256;
use crate::blake512;

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let bitmask_len = inblocks * SPX_N;
    let buf_len = SPX_N + SPX_ADDR_BYTES + bitmask_len;
    let mut bitmask = vec![0u8; bitmask_len];
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));

    blake256::blake256_mgf1(&mut bitmask, bitmask_len, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..bitmask_len {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake256::blake256(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + bitmask_len) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let bitmask_len = inblocks * SPX_N;
    let buf_len = SPX_N + SPX_ADDR_BYTES + bitmask_len;
    let mut bitmask = vec![0u8; bitmask_len];
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes(addr));

    blake512::blake512_mgf1(&mut bitmask, bitmask_len, &buf, SPX_N + SPX_ADDR_BYTES);

    for i in 0..bitmask_len {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    blake512::blake512(&mut outbuf, &buf[SPX_N..], (SPX_ADDR_BYTES + bitmask_len) as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
