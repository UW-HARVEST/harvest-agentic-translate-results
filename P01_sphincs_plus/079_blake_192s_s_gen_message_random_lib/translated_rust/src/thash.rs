use crate::params::*;
use crate::context::SpxCtx;

/// thash_blake_simple: uses blake256 for 1-block, blake512 for multi-block
pub fn thash(
    out: &mut [u8], inp: &[u8], inblocks: usize,
    ctx: &SpxCtx, addr: &mut [u32; 8],
) {
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);
    crate::blake256::blake256(&mut outbuf, &buf, buflen as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(
    out: &mut [u8], inp: &[u8], inblocks: usize,
    ctx: &SpxCtx, addr: &mut [u32; 8],
) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);
    crate::blake512::blake512(&mut outbuf, &buf, buflen as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
