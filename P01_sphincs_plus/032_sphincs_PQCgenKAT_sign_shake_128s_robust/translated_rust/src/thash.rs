use crate::params::*;
use crate::context::SpxCtx;
use crate::address;
use crate::fips202;

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let buf_len = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut bitmask = vec![0u8; inblocks * SPX_N];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(address::addr_bytes(addr));

    fips202::shake256(&mut bitmask, inblocks * SPX_N, &buf[..SPX_N + SPX_ADDR_BYTES]);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    fips202::shake256(out, SPX_N, &buf);
}
