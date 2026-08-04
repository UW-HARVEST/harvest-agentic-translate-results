// SHA-2 thash implementation
#![cfg(feature = "sha2")]
#![allow(dead_code)]

use crate::context::SpxCtx;
use crate::params::*;
use crate::sha2::*;

const SPX_SHA256_ADDR_BYTES: usize = 22;

#[cfg(feature = "robust")]
pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    {
        if inblocks > 1 {
            thash_512(out, input, inblocks, ctx, addr);
            return;
        }
    }
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_OUTPUT_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 40];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);

    mgf1_256(
        &mut bitmask,
        inblocks * SPX_N,
        &buf,
        SPX_N + SPX_SHA256_ADDR_BYTES,
    );

    sha2_state.copy_from_slice(&ctx.state_seeded);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }

    let buf_len = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    sha256_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf[SPX_N..SPX_N + buf_len],
        buf_len,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "robust", any(feature = "192f", feature = "192s", feature = "256f", feature = "256s")))]
fn thash_512(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut bitmask = vec![0u8; inblocks * SPX_N];
    let mut buf = vec![0u8; SPX_N + SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];
    let mut sha2_state = [0u8; 72];

    buf[..SPX_N].copy_from_slice(&ctx.pub_seed);
    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[SPX_N..SPX_N + SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);

    mgf1_512(
        &mut bitmask,
        inblocks * SPX_N,
        &buf,
        SPX_N + SPX_SHA256_ADDR_BYTES,
    );

    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    for i in 0..inblocks * SPX_N {
        buf[SPX_N + SPX_SHA256_ADDR_BYTES + i] = input[i] ^ bitmask[i];
    }

    let buf_len = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    sha512_inc_finalize(
        &mut outbuf,
        &mut sha2_state,
        &buf[SPX_N..SPX_N + buf_len],
        buf_len,
    );
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(feature = "simple")]
pub fn thash(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &mut [u32; 8]) {
    #[cfg(any(feature = "192f", feature = "192s", feature = "256f", feature = "256s"))]
    {
        if inblocks > 1 {
            thash_512(out, input, inblocks, ctx, addr);
            return;
        }
    }
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA256_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 40];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];

    sha2_state.copy_from_slice(&ctx.state_seeded);

    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&input[..inblocks * SPX_N]);

    let blen = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    sha256_inc_finalize(&mut outbuf, &mut sha2_state, &buf, blen);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}

#[cfg(all(feature = "simple", any(feature = "192f", feature = "192s", feature = "256f", feature = "256s")))]
fn thash_512(out: &mut [u8], input: &[u8], inblocks: u32, ctx: &SpxCtx, addr: &[u32; 8]) {
    let inblocks = inblocks as usize;
    let mut outbuf = [0u8; SPX_SHA512_OUTPUT_BYTES];
    let mut sha2_state = [0u8; 72];
    let mut buf = vec![0u8; SPX_SHA256_ADDR_BYTES + inblocks * SPX_N];

    sha2_state.copy_from_slice(&ctx.state_seeded_512);

    let addr_bytes: &[u8; 32] = unsafe { &*(addr.as_ptr() as *const [u8; 32]) };
    buf[..SPX_SHA256_ADDR_BYTES].copy_from_slice(&addr_bytes[..SPX_SHA256_ADDR_BYTES]);
    buf[SPX_SHA256_ADDR_BYTES..].copy_from_slice(&input[..inblocks * SPX_N]);

    let blen = SPX_SHA256_ADDR_BYTES + inblocks * SPX_N;
    sha512_inc_finalize(&mut outbuf, &mut sha2_state, &buf, blen);
    out[..SPX_N].copy_from_slice(&outbuf[..SPX_N]);
}
