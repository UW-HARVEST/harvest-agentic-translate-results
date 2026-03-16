use crate::params::*;
use crate::context::SpxCtx;
use crate::blake256;

// thash_blake_simple: SPX_BLAKE512 = false, so only blake256 path

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize,
             ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let total = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; total];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes = crate::context::addr_bytes(addr);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr_bytes);
    buf[SPX_N + SPX_ADDR_BYTES..total].copy_from_slice(&inp[..inblocks * SPX_N]);

    blake256::blake256(&mut outbuf, &buf, total as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
