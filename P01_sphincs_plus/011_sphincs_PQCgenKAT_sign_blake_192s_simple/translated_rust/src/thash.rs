use crate::params::*;
use crate::utils::SpxCtx;
use crate::address::addr_bytes;
use crate::blake256;
use crate::blake512;

/// thash_blake_simple.c
pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let ab = addr_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ab);
    buf[SPX_N + SPX_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);
    blake256::blake256(&mut outbuf, &buf, buflen);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let ab = addr_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(&ab);
    buf[SPX_N + SPX_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);
    blake512::blake512(&mut outbuf, &buf, buflen);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
