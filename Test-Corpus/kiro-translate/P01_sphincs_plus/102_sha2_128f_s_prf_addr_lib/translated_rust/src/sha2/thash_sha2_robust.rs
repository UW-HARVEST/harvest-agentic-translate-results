use crate::params::*;
use crate::context::SpxCtx;
use crate::sha2::sha2_impl::*;

pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let addr_bytes = unsafe { &*(addr as *const [u32; 8] as *const [u8; 32]) };

    if SPX_SHA512 && inblocks > 1 {
        thash_512(out, input, inblocks, ctx, addr_bytes);
        return;
    }

    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut sha2_state = [0u8; 40];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    mgf1_256(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }

    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf[SPX_N..], SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(not(any(feature = "sphincs-sha2-128s", feature = "sphincs-sha2-128f")))]
fn thash_512(out: &mut [u8], input: &[u8], inblocks: usize, ctx: &SpxCtx, addr_bytes: &[u8]) {
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let buf_len = SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    let mut buf = vec![0u8; buf_len];
    let mut sha2_state = [0u8; 72];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    mgf1_512(&mut bitmask, inblocks * SPX_N, &buf, SPX_N + SPX_SHA256_ADDR_BYTES);

    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }

    sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf[SPX_N..], SPX_SHA256_ADDR_BYTES + inblocks * SPX_N);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(any(feature = "sphincs-sha2-128s", feature = "sphincs-sha2-128f"))]
fn thash_512(_out: &mut [u8], _input: &[u8], _inblocks: usize, _ctx: &SpxCtx, _addr_bytes: &[u8]) {
    // SPX_SHA512 is false for 128-bit variants, so this is unreachable
    unreachable!()
}
