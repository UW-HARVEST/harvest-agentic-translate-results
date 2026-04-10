use crate::params::*;
use crate::context::SpxCtx;
use crate::shake::fips202::shake256;

pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = unsafe { &*(addr as *const [u32; 8] as *const [u8; SPX_ADDR_BYTES]) };
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N]
        .copy_from_slice(&input[..inblocks * SPX_N]);

    shake256(out, SPX_N, &buf);
}
