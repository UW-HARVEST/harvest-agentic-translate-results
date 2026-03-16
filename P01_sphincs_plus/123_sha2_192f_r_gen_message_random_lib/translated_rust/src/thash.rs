use crate::params::*;
use crate::context::SpxCtx;
use crate::sha2::*;

fn thash_512(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut sha2_state = [0u8; 72];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES]
        .copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    mgf1_512(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    sha512_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf[SPX_N..],
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

pub fn thash(out: &mut [u8], inp: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &[u32; 8]) {
    if inblocks > 1 {
        thash_512(out, inp, inblocks, ctx, addr);
        return;
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_SHA256_OUTPUT_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut sha2_state = [0u8; 40];

    let addr_bytes =
        unsafe { core::slice::from_raw_parts(addr.as_ptr() as *const u8, 32) };

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES]
        .copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = inp[i] ^ bitmask[i];
    }

    sha256_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf[SPX_N..],
        SPX_SHA256_ADDR_BYTES + inblocks * SPX_N,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
