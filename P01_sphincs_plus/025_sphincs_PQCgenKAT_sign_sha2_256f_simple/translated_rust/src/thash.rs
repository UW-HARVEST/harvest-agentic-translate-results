use crate::params::*;
use crate::context::SpxCtx;
use crate::sha2;
use crate::address::addr_bytes;

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    if SPX_SHA512 && inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    sha2_state.copy_from_slice(&ctx.state_seeded);

    let ab = addr_bytes(addr);
    let buf_len = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..buf_len].copy_from_slice(&inp[..inblocks * SPX_N]);

    sha2::sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buf_len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    let ab = addr_bytes(addr);
    let buf_len = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&ab[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..buf_len].copy_from_slice(&inp[..inblocks * SPX_N]);

    sha2::sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf, buf_len);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
