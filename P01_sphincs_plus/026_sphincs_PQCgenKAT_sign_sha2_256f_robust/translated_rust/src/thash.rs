use crate::params::*;
use crate::context::SpxCtx;
use crate::sha2::*;
use crate::address;

/// thash - robust variant using MGF1 bitmask
/// For inblocks > 1, uses SHA-512; for inblocks == 1, uses SHA-256
pub fn thash(out: &mut [u8], in_: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    if inblocks > 1 {
        thash_512(out, in_, inblocks, ctx, addr);
        return;
    }
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut sha2_state = [0u8; 40];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&address::addr_as_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = in_[i] ^ bitmask[i];
    }

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf[SPX_N..], SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

fn thash_512(out: &mut [u8], in_: &[u8], inblocks: usize, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut sha2_state = [0u8; 72];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&address::addr_as_bytes(addr)[..SPX_SHA256_ADDR_BYTES]);
    mgf1_512(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = in_[i] ^ bitmask[i];
    }

    sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf[SPX_N..], SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

/// Helper: thash on current[0..SPX_N] || current[SPX_N..2*SPX_N], result into current[SPX_N..]
pub fn thash_inplace_right(current: &mut [u8], ctx: &SpxCtx, addr: &mut [u32; 8]) {
    let mut tmp = vec![0u8; 2 * SPX_N];
    tmp.copy_from_slice(&current[..2 * SPX_N]);
    thash(&mut current[SPX_N..], &tmp, 2, ctx, addr);
}
