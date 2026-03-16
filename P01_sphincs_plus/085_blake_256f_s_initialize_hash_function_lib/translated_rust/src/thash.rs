use crate::params::*;
use crate::context::*;
use crate::blake256;
use crate::blake512;

// thash_blake_simple: SPX_BLAKE512 is true
fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; total];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_as_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..total].copy_from_slice(&inp[..inblocks * SPX_N]);

    blake512::blake512(&mut outbuf, &buf, total as u64);
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

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_as_bytes(addr));
    buf[SPX_N + SPX_ADDR_BYTES..total].copy_from_slice(&inp[..inblocks * SPX_N]);

    blake256::blake256(&mut outbuf, &buf, total as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
