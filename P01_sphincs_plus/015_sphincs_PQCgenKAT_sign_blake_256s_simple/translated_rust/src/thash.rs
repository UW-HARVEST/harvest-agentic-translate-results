use crate::address::*;
use crate::blake256::blake256;
use crate::blake512::blake512;
use crate::params::*;

// SPX_BLAKE512 = 1, so multi-block uses blake512
fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &Addr) {
    let mut outbuf = [0u8; SPX_BLAKE512_OUTPUT_BYTES];
    let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr);
    buf[SPX_N + SPX_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);
    blake512(&mut outbuf, &buf, buflen as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &Addr) {
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_BLAKE256_OUTPUT_BYTES];
    let buflen = SPX_N + SPX_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buflen];
    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_ADDR_BYTES].copy_from_slice(addr);
    buf[SPX_N + SPX_ADDR_BYTES..buflen].copy_from_slice(&inp[..inblocks * SPX_N]);
    blake256(&mut outbuf, &buf, buflen as u64);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
