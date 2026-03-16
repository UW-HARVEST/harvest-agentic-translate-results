use crate::blake256;
use crate::blake512;
use crate::context::SpxCtx;
use crate::params::*;

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8] =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..buf_len].copy_from_slice(&inp[..inblocks * SPX_N]);

    blake512::blake512(&mut outbuf, &buf, buf_len as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    if SPX_BLAKE512 && inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8] =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, SPX_ADDR_BYTES) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..buf_len].copy_from_slice(&inp[..inblocks * SPX_N]);

    blake256::blake256(&mut outbuf, &buf, buf_len as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
