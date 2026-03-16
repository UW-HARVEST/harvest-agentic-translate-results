use crate::address::Addr;
use crate::context::SpxCtx;
use crate::fips202::shake256;
use crate::params::*;

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &Addr) {
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut bitmask = vec![0u8; inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr);

    shake256(&mut bitmask, &buf[..SPX_N + SPX_ADDR_BYTES]);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    let mut result = vec![0u8; SPX_N];
    shake256(&mut result, &buf);
    out[..SPX_N].copy_from_slice(&result);
}
